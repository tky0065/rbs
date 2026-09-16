use chrono::{Timelike, Utc};
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::Job;
use super::model::ActiveModel;

mod outcome;
mod reserve;

pub use outcome::{Reprise, mark_done, requeue_stale, retry_or_fail};
// Réexporté pour que les tests du fragment écrivent `queue::retry_delay` : ils sont le
// seul appelant de ce chemin, et ne compilent que sous `#[cfg(test)]`, ce qui rend le
// réexport inutile aux yeux du compilateur en dehors des tests.
#[allow(unused_imports)]
pub(super) use outcome::retry_delay;
pub use reserve::reserver_prochain_job;

/// Enfile un job, exécutable dès maintenant, et rend l'identifiant de sa ligne.
///
/// `db` est un `ConnectionTrait` et non une connexion, et c'est toute la raison d'avoir
/// mis la file en base : une transaction en est un. Passez-lui celle du métier, et le job
/// naît si et seulement si elle est committée.
// region: enqueue
pub async fn enqueue<C, J>(db: &C, job: &J) -> anyhow::Result<Uuid>
where
    C: ConnectionTrait,
    J: Job,
{
    enqueue_at(db, job, Utc::now().fixed_offset()).await
}
// endregion: enqueue

/// Enfile un job qui ne deviendra dépilable qu'à `available_at`.
pub async fn enqueue_at<C, J>(
    db: &C,
    job: &J,
    available_at: DateTimeWithTimeZone,
) -> anyhow::Result<Uuid>
where
    C: ConnectionTrait,
    J: Job,
{
    // L'identifiant, le statut et le compteur viennent des défauts de la table : ce que
    // l'appelant n'a pas à choisir, il n'a pas à l'écrire.
    let ligne = ActiveModel {
        kind: Set(J::KIND.to_string()),
        payload: Set(serde_json::to_value(job)?),
        available_at: Set(a_la_seconde(available_at)),
        ..Default::default()
    };

    Ok(ligne.insert(db).await?.id)
}

/// Tronque un instant à la seconde, tel qu'il sera stocké.
///
/// MySQL rend `timestamp` sans précision fractionnaire et **arrondit** ce qu'on y écrit :
/// un `available_at` à `…34,6 s` y devient `…35 s`, soit un job que sa propre échéance
/// place dans le futur et que le dépilage ne verra pas. Tronquer à l'écriture rend la
/// valeur exactement représentable sur les trois moteurs — et une file scrutée à la
/// seconde n'a que faire des microsecondes.
fn a_la_seconde(instant: DateTimeWithTimeZone) -> DateTimeWithTimeZone {
    instant.with_nanosecond(0).unwrap_or(instant)
}
