use chrono::Utc;
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, Statement, TransactionTrait,
};

use super::super::model::{Entity, Model, Status};

/// Les colonnes que la réservation doit rendre, dans l'ordre de `Model`.
const COLONNES: &str =
    "id, kind, payload, status, attempts, available_at, last_error, created_at, updated_at";

/// Le dépilage PostgreSQL, et le seul SQL écrit à la main du fragment.
///
/// Réserver la ligne et incrémenter son compteur sont un seul `UPDATE` : deux workers ne
/// peuvent donc pas se voir attribuer la même, quel que soit leur entrelacement. Un
/// `SELECT` suivi d'un `UPDATE` les leur donnerait tous les deux.
///
/// `FOR UPDATE SKIP LOCKED` fait passer le second worker à la ligne suivante au lieu de
/// l'y faire attendre.
///
/// L'instant est lié en paramètre, et non lu par un `now()` de la base : `available_at`
/// est posé par l'application — à l'enfilage comme au réessai — et le comparer à
/// l'horloge du serveur rend un job indépilable dès que les deux horloges divergent d'une
/// milliseconde. C'est aussi ce qui débarrasse les trois requêtes de toute fonction de
/// date propre à un moteur.
const RESERVATION_POSTGRES: &str = "\
UPDATE jobs
SET status = $1, attempts = attempts + 1, updated_at = $3
WHERE id = (
    SELECT id FROM jobs
    WHERE status = $2 AND available_at <= $3
    ORDER BY available_at, id
    FOR UPDATE SKIP LOCKED
    LIMIT 1
)
RETURNING id, kind, payload, status, attempts, available_at, last_error, created_at, updated_at";

/// Le dépilage SQLite : la même requête, sans le verrou.
///
/// SQLite ne laisse écrire qu'un processus à la fois, et un `UPDATE` isolé y est sa
/// propre transaction immédiate. Le `busy_timeout` du pilote fait attendre le worker
/// bloqué plutôt que de le faire échouer — `SKIP LOCKED` n'aurait rien à sauter.
const RESERVATION_SQLITE: &str = "\
UPDATE jobs
SET status = ?, attempts = attempts + 1, updated_at = ?
WHERE id = (
    SELECT id FROM jobs
    WHERE status = ? AND available_at <= ?
    ORDER BY available_at, id
    LIMIT 1
)
RETURNING id, kind, payload, status, attempts, available_at, last_error, created_at, updated_at";

/// L'élection de la ligne, en MySQL 8.
///
/// MySQL connaît `SKIP LOCKED`, mais son erreur 1093 interdit de viser `jobs` dans le
/// sous-`SELECT` d'un `UPDATE` sur `jobs`, et il n'a pas d'`UPDATE … RETURNING` : la
/// requête unique y est impossible. Le verrou que ce `SELECT` pose est tenu jusqu'au
/// commit, ce qui interdit la même ligne à deux workers — le rôle que `SKIP LOCKED` joue
/// ailleurs dans une seule requête.
const ELECTION_MYSQL: &str = "\
SELECT id FROM jobs
WHERE status = ? AND available_at <= ?
ORDER BY available_at, id
LIMIT 1
FOR UPDATE SKIP LOCKED";

/// La réservation de la ligne élue, dans la même transaction.
const RESERVATION_MYSQL: &str = "\
UPDATE jobs SET status = ?, attempts = attempts + 1, updated_at = ? WHERE id = ?";

/// Réserve le prochain job dépilable, ou rend `None` si la file n'a rien à donner.
///
/// **C'est le seul endroit du fragment où la file est dépilée**, et c'est délibéré : le
/// jour où un autre moteur devra la porter, il n'y a que ce corps de fonction à écrire.
pub async fn reserver_prochain_job(db: &DatabaseConnection) -> anyhow::Result<Option<Model>> {
    let backend = db.get_database_backend();

    // Une seule lecture de l'horloge pour toute la réservation : la borne de
    // `available_at` et l'`updated_at` posé doivent être le même instant.
    let maintenant = Utc::now().fixed_offset();

    match backend {
        DatabaseBackend::MySql => reserver_en_deux_temps(db, maintenant).await,
        DatabaseBackend::Sqlite => {
            // SQLite n'a pas de paramètre nommé ici : l'instant est lié deux fois, une
            // par `?` de la requête.
            reserver_en_un_coup(
                db,
                backend,
                RESERVATION_SQLITE,
                [
                    Status::Running.as_str().into(),
                    maintenant.into(),
                    Status::Pending.as_str().into(),
                    maintenant.into(),
                ],
            )
            .await
        }
        DatabaseBackend::Postgres => {
            reserver_en_un_coup(
                db,
                backend,
                RESERVATION_POSTGRES,
                [
                    Status::Running.as_str().into(),
                    Status::Pending.as_str().into(),
                    maintenant.into(),
                ],
            )
            .await
        }
        // `DatabaseBackend` est `non_exhaustive` : un moteur ajouté par SeaORM arriverait
        // ici. Le dépilage n'est pas portable par défaut — mieux vaut le dire que rejouer
        // au hasard la requête d'un autre moteur.
        autre => anyhow::bail!("le dépilage des jobs n'est pas écrit pour {autre:?}"),
    }
}

/// Réserve par un `UPDATE … RETURNING` unique, ce que PostgreSQL et SQLite savent faire.
async fn reserver_en_un_coup<const N: usize>(
    db: &DatabaseConnection,
    backend: DatabaseBackend,
    requete: &str,
    valeurs: [sea_orm::Value; N],
) -> anyhow::Result<Option<Model>> {
    let statement = Statement::from_sql_and_values(backend, requete, valeurs);

    Ok(Entity::find().from_raw_sql(statement).one(db).await?)
}

/// Réserve en élisant puis en marquant, dans une transaction, ce qu'exige MySQL 8.
///
/// La transaction n'est pas un confort : c'est elle qui tient le verrou posé par
/// l'élection jusqu'à ce que la ligne soit marquée. Sortir l'`UPDATE` de la transaction
/// rendrait la ligne réservable deux fois.
async fn reserver_en_deux_temps(
    db: &DatabaseConnection,
    maintenant: DateTimeWithTimeZone,
) -> anyhow::Result<Option<Model>> {
    let transaction = db.begin().await?;
    let backend = DatabaseBackend::MySql;

    let elue = transaction
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            ELECTION_MYSQL,
            [Status::Pending.as_str().into(), maintenant.into()],
        ))
        .await?;

    let Some(ligne) = elue else {
        transaction.rollback().await?;
        return Ok(None);
    };

    let id: Uuid = ligne.try_get_by_index(0)?;

    transaction
        .execute_raw(Statement::from_sql_and_values(
            backend,
            RESERVATION_MYSQL,
            [
                Status::Running.as_str().into(),
                maintenant.into(),
                id.into(),
            ],
        ))
        .await?;

    let reserve = Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            backend,
            format!("SELECT {COLONNES} FROM jobs WHERE id = ?"),
            [id.into()],
        ))
        .one(&transaction)
        .await?;

    transaction.commit().await?;

    Ok(reserve)
}
