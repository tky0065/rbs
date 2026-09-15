use std::time::Duration;

use chrono::{TimeDelta, Utc};
use sea_orm::prelude::Expr;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::super::Config;
use super::super::model::{Column, Entity, Model, Status};
use super::a_la_seconde;

/// Ce que la reprise des réservations abandonnées a fait de chacune.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Reprise {
    /// Lignes rendues à la file, de nouveau `pending`.
    pub rendus: u64,
    /// Lignes condamnées, `failed`, faute de tentative restante.
    pub condamnes: u64,
}

/// Le message inscrit dans `last_error` d'un job condamné sans avoir jamais répondu.
const ABANDON: &str = "réservation abandonnée : le worker n'a jamais inscrit le sort du job";

/// Reprend les jobs qu'un worker a réservés sans jamais en inscrire le sort.
///
/// Un processus tué entre la réservation et `mark_done` laisse sa ligne en `running`, et
/// rien d'autre ne la dépilerait plus : chaque redémarrage perdrait en silence le job en
/// cours. Passé `lease_secs`, la réservation est tenue pour abandonnée et la ligne redevient
/// `pending`. Un job encore en cours au-delà du bail serait donc rejoué — c'est pourquoi
/// le bail se règle au-dessus du plus long job.
///
/// `attempts` n'est pas rendu : incrémenté à la réservation, il compte le crash comme une
/// tentative. Et c'est ici que `max_attempts` le borne, non dans `retry_or_fail` : un job
/// qui tue son worker à chaque fois n'atteint jamais `retry_or_fail`, et sans cette borne
/// il serait rendu à la file, puis réservé, sans fin. Au-delà, la ligne passe `failed`
/// avec un `last_error` qui dit que personne n'a répondu.
///
/// La borne du bail est liée en paramètre, comme dans les trois requêtes de réservation :
/// un `now()` du moteur comparerait l'horloge du serveur à un `updated_at` posé par
/// l'application — et, sur SQLite, du texte à du texte d'un autre format.
pub async fn requeue_stale(db: &DatabaseConnection, config: &Config) -> anyhow::Result<Reprise> {
    let maintenant = Utc::now().fixed_offset();
    let limite = maintenant - TimeDelta::from_std(Duration::from_secs(config.lease_secs))?;

    let condamnes = Entity::update_many()
        .col_expr(Column::Status, Expr::value(Status::Failed))
        .col_expr(Column::LastError, Expr::value(Some(ABANDON.to_string())))
        .col_expr(Column::UpdatedAt, Expr::value(maintenant))
        .filter(Column::Status.eq(Status::Running))
        .filter(Column::UpdatedAt.lt(limite))
        .filter(Column::Attempts.gte(config.max_attempts))
        .exec(db)
        .await?;

    let rendus = Entity::update_many()
        .col_expr(Column::Status, Expr::value(Status::Pending))
        .col_expr(Column::UpdatedAt, Expr::value(maintenant))
        .filter(Column::Status.eq(Status::Running))
        .filter(Column::UpdatedAt.lt(limite))
        .filter(Column::Attempts.lt(config.max_attempts))
        .exec(db)
        .await?;

    Ok(Reprise {
        rendus: rendus.rows_affected,
        condamnes: condamnes.rows_affected,
    })
}

/// Marque un job réussi.
///
/// Un `UPDATE` ciblé plutôt que `ActiveModel::update` : celui-ci relit la ligne entière,
/// payload compris — par `RETURNING` sur PostgreSQL et SQLite, par un `SELECT` de plus sur
/// MySQL — pour rendre un modèle que personne ne lit.
pub async fn mark_done(db: &DatabaseConnection, job: &Model) -> anyhow::Result<()> {
    Entity::update_many()
        .col_expr(Column::Status, Expr::value(Status::Done))
        .col_expr(Column::LastError, Expr::value(Option::<String>::None))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now().fixed_offset()))
        .filter(Column::Id.eq(job.id))
        .exec(db)
        .await?;

    Ok(())
}

/// Le délai avant qu'un job raté redevienne dépilable : `retry_delay_secs`, doublé à
/// chaque tentative, sous `retry_max_delay_secs`.
///
/// `attempts` est celui de la ligne, déjà incrémenté à la réservation : la première
/// tentative ratée attend le délai de base. Un receveur en panne est ainsi sollicité de
/// moins en moins souvent, au lieu de l'être à cadence fixe jusqu'à l'échec définitif.
/// La multiplication sature avant d'être bornée : un `max_attempts` élevé ne fait pas
/// déborder le calcul.
pub(in crate::modules::jobs) fn retry_delay(config: &Config, attempts: i32) -> Duration {
    let doublements = u32::try_from(attempts.saturating_sub(1)).unwrap_or(0);
    let facteur = 2u64.checked_pow(doublements).unwrap_or(u64::MAX);
    let secondes = config
        .retry_delay_secs
        .saturating_mul(facteur)
        .min(config.retry_max_delay_secs);

    Duration::from_secs(secondes)
}

/// Replace un job raté dans la file, ou le condamne s'il a épuisé ses tentatives, et rend
/// le statut retenu.
///
/// `attempts` a déjà été incrémenté par la réservation : le compteur lu ici est celui de
/// la tentative qui vient d'échouer.
pub async fn retry_or_fail(
    db: &DatabaseConnection,
    job: &Model,
    config: &Config,
    error: &anyhow::Error,
) -> anyhow::Result<Status> {
    let status = if job.attempts >= config.max_attempts {
        Status::Failed
    } else {
        Status::Pending
    };

    let attente = TimeDelta::from_std(retry_delay(config, job.attempts))
        .unwrap_or_else(|_| TimeDelta::seconds(0));

    Entity::update_many()
        .col_expr(Column::Status, Expr::value(status))
        .col_expr(Column::LastError, Expr::value(Some(format!("{error:#}"))))
        .col_expr(
            Column::AvailableAt,
            Expr::value(a_la_seconde((Utc::now() + attente).fixed_offset())),
        )
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now().fixed_offset()))
        .filter(Column::Id.eq(job.id))
        .exec(db)
        .await?;

    Ok(status)
}
