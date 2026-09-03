use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, ConnectionTrait};

use super::Entry;
use super::model::ActiveModel;

/// Inscrit une entrée au journal et rend l'identifiant de sa ligne.
///
/// `db` est un `ConnectionTrait` et non une connexion, et c'est toute la raison d'avoir
/// mis le journal en base : une transaction en est un. Passez-lui celle du métier, et la
/// trace naît si et seulement si le changement qu'elle décrit est committé.
pub async fn record<C>(db: &C, entry: Entry) -> anyhow::Result<Uuid>
where
    C: ConnectionTrait,
{
    // L'identifiant et la date viennent des défauts : ce que l'appelant n'a pas à
    // choisir, il n'a pas à l'écrire.
    let ligne = ActiveModel {
        actor_id: Set(entry.actor_id),
        action: Set(entry.action),
        entity: Set(entry.entity),
        entity_id: Set(entry.entity_id),
        changes: Set(entry.changes),
        ..Default::default()
    };

    Ok(ligne.insert(db).await?.id)
}
