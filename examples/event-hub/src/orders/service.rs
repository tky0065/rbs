use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;

use super::dto::{CreateOrder, OrderResponse, UpdateOrder};
use super::filter::OrderFilter;
use super::repository::{self, ActiveModel};
use crate::modules::audit::{self, Entry};
use crate::modules::webhooks;

pub async fn list(db: &DatabaseConnection, pagination: &Pagination) -> Result<Page<OrderResponse>> {
    let (orders, total) = repository::list(db, pagination).await?;

    Ok(Page::new(
        orders.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn filter(
    db: &DatabaseConnection,
    filtre: &OrderFilter,
    pagination: &Pagination,
) -> Result<Page<OrderResponse>> {
    let (orders, total) = repository::filter(db, filtre, pagination).await?;

    Ok(Page::new(
        orders.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<OrderResponse> {
    let order = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("order"))?;

    Ok(order.into())
}

// region: create
/// Crée une commande, sa trace et son événement, ensemble ou pas du tout.
///
/// Les trois écritures partagent `transaction` : un rollback emporte la commande, l'entrée
/// du journal et les livraisons d'un même geste. Sur `db`, un `order.created` partirait pour
/// une commande que la base a refusée, et le journal tracerait une écriture qui n'a pas eu
/// lieu.
pub async fn create(
    db: &DatabaseConnection,
    input: CreateOrder,
    actor: &str,
) -> Result<OrderResponse> {
    let transaction = db.begin().await?;

    let order = ActiveModel {
        reference: Set(input.reference),
        amount: Set(input.amount),
        ..Default::default()
    };
    let order: OrderResponse = repository::create(&transaction, order).await?.into();

    audit::record(
        &transaction,
        Entry::new(audit::CREATE, "orders", order.id.to_string())
            .actor(actor)
            .changes(json!({ "reference": order.reference, "amount": order.amount })),
    )
    .await?;

    // Le DTO et non l'entité : le corps d'un webhook est un contrat public, et le coupler à
    // la table ferait de chaque colonne renommée une rupture chez tous les abonnés.
    webhooks::emit(&transaction, "order.created", &order).await?;

    transaction.commit().await?;

    Ok(order)
}
// endregion: create

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateOrder,
) -> Result<OrderResponse> {
    let mut order: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("order"))?
        .into();

    // `Option` ne distingue pas un champ absent d'un `null` explicite : cette route ne
    // peut donc pas remettre un champ optionnel à NULL. Ajoutez-y le cas si votre API en
    // a besoin.
    if let Some(reference) = input.reference {
        order.reference = Set(reference);
    }
    if let Some(amount) = input.amount {
        order.amount = Set(amount);
    }
    order.updated_at = Set(chrono::Utc::now().into());

    Ok(repository::update(db, order).await?.into())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("order"));
    }

    Ok(())
}
