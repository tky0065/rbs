use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{CreatePost, PostResponse, UpdatePost};
use super::repository::{self, ActiveModel};

pub async fn list(
    db: &DatabaseConnection,
    pagination: &Pagination,
) -> Result<Page<PostResponse>> {
    let (posts, total) = repository::list(db, pagination).await?;

    Ok(Page::new(
        posts.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<PostResponse> {
    let post = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("post"))?;

    Ok(post.into())
}

pub async fn create(db: &DatabaseConnection, entree: CreatePost) -> Result<PostResponse> {
    let post = ActiveModel {
        title: Set(entree.title),
        body: Set(entree.body),
        published: Set(entree.published),
        ..Default::default()
    };

    Ok(repository::create(db, post).await?.into())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    entree: UpdatePost,
) -> Result<PostResponse> {
    let mut post: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("post"))?
        .into();

    // Un champ absent du corps garde sa valeur : cette route ne peut donc pas remettre un
    // champ optionnel à NULL. Ajoutez-y le cas si votre API en a besoin.
    if let Some(title) = entree.title {
        post.title = Set(title);
    }
    if let Some(body) = entree.body {
        post.body = Set(body);
    }
    if let Some(published) = entree.published {
        post.published = Set(published);
    }
    post.updated_at = Set(chrono::Utc::now().into());

    Ok(repository::update(db, post).await?.into())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("post"));
    }

    Ok(())
}
