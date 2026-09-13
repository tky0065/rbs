use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{CreateOrder, OrderResponse, UpdateOrder};
use super::filter::OrderFilter;
use super::repository::{self, ActiveModel};

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

pub async fn create(db: &DatabaseConnection, input: CreateOrder) -> Result<OrderResponse> {
    let order = ActiveModel {
        reference: Set(input.reference),
        amount: Set(input.amount),
        ..Default::default()
    };

    Ok(repository::create(db, order).await?.into())
}

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
