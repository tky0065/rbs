use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{CreateIncident, IncidentResponse, UpdateIncident};
use super::filter::IncidentFilter;
use super::repository::{self, ActiveModel};

pub async fn list(
    db: &DatabaseConnection,
    pagination: &Pagination,
) -> Result<Page<IncidentResponse>> {
    let (incidents, total) = repository::list(db, pagination).await?;

    Ok(Page::new(
        incidents.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn filter(
    db: &DatabaseConnection,
    filtre: &IncidentFilter,
    pagination: &Pagination,
) -> Result<Page<IncidentResponse>> {
    let (incidents, total) = repository::filter(db, filtre, pagination).await?;

    Ok(Page::new(
        incidents.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<IncidentResponse> {
    let incident = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("incident"))?;

    Ok(incident.into())
}

pub async fn create(db: &DatabaseConnection, input: CreateIncident) -> Result<IncidentResponse> {
    let incident = ActiveModel {
        reference: Set(input.reference),
        sujet: Set(input.sujet),
        detail: Set(input.detail),
        gravite: Set(input.gravite),
        ouvert: Set(input.ouvert),
        duree_minutes: Set(input.duree_minutes),
        echeance: Set(input.echeance),
        constate_le: Set(input.constate_le),
        ..Default::default()
    };

    Ok(repository::create(db, incident).await?.into())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateIncident,
) -> Result<IncidentResponse> {
    let mut incident: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("incident"))?
        .into();

    // `Option` ne distingue pas un champ absent d'un `null` explicite : cette route ne
    // peut donc pas remettre un champ optionnel à NULL. Ajoutez-y le cas si votre API en
    // a besoin.
    if let Some(reference) = input.reference {
        incident.reference = Set(reference);
    }
    if let Some(sujet) = input.sujet {
        incident.sujet = Set(sujet);
    }
    if let Some(detail) = input.detail {
        incident.detail = Set(detail);
    }
    if let Some(gravite) = input.gravite {
        incident.gravite = Set(gravite);
    }
    if let Some(ouvert) = input.ouvert {
        incident.ouvert = Set(ouvert);
    }
    if let Some(duree_minutes) = input.duree_minutes {
        incident.duree_minutes = Set(Some(duree_minutes));
    }
    if let Some(echeance) = input.echeance {
        incident.echeance = Set(Some(echeance));
    }
    if let Some(constate_le) = input.constate_le {
        incident.constate_le = Set(constate_le);
    }
    incident.updated_at = Set(chrono::Utc::now().into());

    Ok(repository::update(db, incident).await?.into())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("incident"));
    }

    Ok(())
}
