use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{CreateTicket, TicketResponse, UpdateTicket};
use super::filter::TicketFilter;
use super::repository::{self, ActiveModel};

pub async fn list(
    db: &DatabaseConnection,
    pagination: &Pagination,
) -> Result<Page<TicketResponse>> {
    let (tickets, total) = repository::list(db, pagination).await?;

    Ok(Page::new(
        tickets.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn filter(
    db: &DatabaseConnection,
    filtre: &TicketFilter,
    pagination: &Pagination,
) -> Result<Page<TicketResponse>> {
    let (tickets, total) = repository::filter(db, filtre, pagination).await?;

    Ok(Page::new(
        tickets.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<TicketResponse> {
    let ticket = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("ticket"))?;

    Ok(ticket.into())
}

pub async fn create(db: &DatabaseConnection, input: CreateTicket) -> Result<TicketResponse> {
    let ticket = ActiveModel {
        sujet: Set(input.sujet),
        detail: Set(input.detail),
        statut: Set(input.statut),
        priorite: Set(input.priorite),
        auteur_id: Set(input.auteur_id),
        ..Default::default()
    };

    Ok(repository::create(db, ticket).await?.into())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateTicket,
) -> Result<TicketResponse> {
    let mut ticket: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("ticket"))?
        .into();

    // `Option` ne distingue pas un champ absent d'un `null` explicite : cette route ne
    // peut donc pas remettre un champ optionnel à NULL. Ajoutez-y le cas si votre API en
    // a besoin.
    if let Some(sujet) = input.sujet {
        ticket.sujet = Set(sujet);
    }
    if let Some(detail) = input.detail {
        ticket.detail = Set(detail);
    }
    if let Some(statut) = input.statut {
        ticket.statut = Set(statut);
    }
    if let Some(priorite) = input.priorite {
        ticket.priorite = Set(priorite);
    }
    if let Some(auteur_id) = input.auteur_id {
        ticket.auteur_id = Set(auteur_id);
    }
    ticket.updated_at = Set(chrono::Utc::now().into());

    Ok(repository::update(db, ticket).await?.into())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("ticket"));
    }

    Ok(())
}
