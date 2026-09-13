use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

use super::Schedule;
use super::model::{ActiveModel, Column, Entity};

/// Aligne la table sur le calendrier déclaré en code.
///
/// Toute expression est compilée **avant** le premier écrit : une seule illisible arrête
/// le démarrage plutôt que de laisser un service qui paraît sain et dont une tâche ne
/// tournera jamais — le mode de panne le plus coûteux à diagnostiquer.
pub async fn reconcilier(db: &DatabaseConnection, schedules: &[Schedule]) -> anyhow::Result<()> {
    let mut prochaines = Vec::with_capacity(schedules.len());
    for schedule in schedules {
        let horaire = schedule.compiler()?;
        let prochaine = horaire
            .after(&Utc::now().fixed_offset())
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "`{}` n'a plus aucune occurrence à venir",
                    schedule.expression
                )
            })?;

        prochaines.push((schedule.kind, super::a_la_seconde(prochaine)));
    }

    let declares: Vec<&str> = prochaines.iter().map(|(kind, _)| *kind).collect();

    // Une échéance retirée du code resterait sinon due pour toujours, sans que rien ne la
    // réserve ni ne la fasse avancer.
    let mut suppression = Entity::delete_many();
    if !declares.is_empty() {
        suppression = suppression.filter(Column::Kind.is_not_in(declares.iter().copied()));
    }
    suppression.exec(db).await?;

    let maintenant = super::a_la_seconde(Utc::now().fixed_offset());

    for (kind, prochaine) in prochaines {
        if let Some(connue) = Entity::find_by_id(kind).one(db).await? {
            // Une échéance échue appartient au tick : la déplacer effacerait une occurrence
            // que le processus a déjà gagnée, alors que la réservation la jouera puis
            // l'avancera d'elle-même selon l'expression nouvelle. Une échéance à venir qui
            // tombe sur l'occurrence recalculée est un redéploiement sans changement, et
            // rien ne doit bouger. Ne reste que l'échéance à venir qui diverge : c'est la
            // seule trace qu'une expression a changé, la table ne la stockant pas, et sans
            // ce recalcul le code ne ferait foi qu'après l'occurrence de l'ancienne — un
            // mois, pour un `0 3 1 * *` devenu `*/5 * * * *`.
            if connue.next_run_at <= maintenant || connue.next_run_at == prochaine {
                continue;
            }

            // L'`UPDATE` ne touche que ces deux colonnes et exige l'échéance relue : un
            // ticker qui l'aurait réservée entre la lecture et l'écriture a déjà avancé la
            // ligne, et la réécrire en entier effacerait son `last_run_at`.
            Entity::update_many()
                .col_expr(Column::NextRunAt, prochaine.into())
                .col_expr(Column::UpdatedAt, maintenant.into())
                .filter(Column::Kind.eq(kind))
                .filter(Column::NextRunAt.eq(connue.next_run_at))
                .exec(db)
                .await?;

            tracing::info!(
                kind,
                ancienne = %connue.next_run_at,
                nouvelle = %prochaine,
                "échéance recalculée : l'expression a changé"
            );

            continue;
        }

        // Les deux horodatages sont posés plutôt que laissés aux défauts de la table :
        // la clé n'étant pas auto-incrémentée, `ActiveModelBehavior` ne remplit rien, et
        // SeaORM refuse un `NotSet` sur une colonne `not null`.
        let ligne = ActiveModel {
            kind: Set(kind.to_string()),
            next_run_at: Set(prochaine),
            last_run_at: Set(None),
            created_at: Set(maintenant),
            updated_at: Set(maintenant),
        };

        // Un déploiement lance ses réplicas ensemble : tous lisent une table où l'échéance
        // manque encore, et tous l'insèrent. Sans cette tolérance, le démarrage de tous
        // sauf un échouerait sur la clé primaire — sur le fragment dont l'argument est
        // précisément de tenir le multi-réplica.
        //
        // La ligne du réplica qui a devancé reste intacte, ce qui est la règle voulue :
        // la première échéance posée fait foi.
        //
        // `do_nothing_on` et non `do_nothing` : les deux disent la même chose, mais le
        // second laisse vide la liste de colonnes, et le générateur MySQL écrit alors
        // `ON DUPLICATE KEY IGNORE`, qui n'est pas du SQL — l'erreur 1064 est levée à
        // l'exécution, sur ce seul moteur. Nommer la colonne lui fait écrire
        // `ON DUPLICATE KEY UPDATE kind = kind`, pendant que PostgreSQL et SQLite
        // reçoivent le `DO NOTHING` attendu.
        let pose = Entity::insert(ligne)
            .on_conflict(
                OnConflict::column(Column::Kind)
                    .do_nothing_on([Column::Kind])
                    .to_owned(),
            )
            .exec(db)
            .await;

        match pose {
            Ok(_) => {}
            // Ce que SeaORM rend quand le `do_nothing` a mordu : la ligne était déjà là.
            Err(DbErr::RecordNotInserted) => {}
            Err(autre) => return Err(autre.into()),
        }
    }

    Ok(())
}
