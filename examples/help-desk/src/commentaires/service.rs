use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{CommentaireResponse, CreateCommentaire, UpdateCommentaire};
use super::filter::CommentaireFilter;
use super::repository::{self, ActiveModel};

pub async fn list(
    db: &DatabaseConnection,
    pagination: &Pagination,
) -> Result<Page<CommentaireResponse>> {
    let (commentaires, total) = repository::list(db, pagination).await?;

    Ok(Page::new(
        commentaires.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn filter(
    db: &DatabaseConnection,
    filtre: &CommentaireFilter,
    pagination: &Pagination,
) -> Result<Page<CommentaireResponse>> {
    let (commentaires, total) = repository::filter(db, filtre, pagination).await?;

    Ok(Page::new(
        commentaires.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<CommentaireResponse> {
    let commentaire = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("commentaire"))?;

    Ok(commentaire.into())
}

pub async fn create(
    db: &DatabaseConnection,
    auteur: Uuid,
    input: CreateCommentaire,
) -> Result<CommentaireResponse> {
    let commentaire = ActiveModel {
        corps: Set(input.corps),
        ticket_id: Set(input.ticket_id),
        auteur_id: Set(auteur),
        ..Default::default()
    };

    Ok(repository::create(db, commentaire).await?.into())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateCommentaire,
) -> Result<CommentaireResponse> {
    let mut commentaire: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("commentaire"))?
        .into();

    // `Option` ne distingue pas un champ absent d'un `null` explicite : cette route ne
    // peut donc pas remettre un champ optionnel à NULL. Ajoutez-y le cas si votre API en
    // a besoin.
    if let Some(corps) = input.corps {
        commentaire.corps = Set(corps);
    }
    if let Some(ticket_id) = input.ticket_id {
        commentaire.ticket_id = Set(ticket_id);
    }
    commentaire.updated_at = Set(chrono::Utc::now().into());

    Ok(repository::update(db, commentaire).await?.into())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("commentaire"));
    }

    Ok(())
}
