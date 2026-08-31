use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{CreatePost, PostResponse, UpdatePost};
use super::repository::{self, ActiveModel};

pub async fn list(db: &DatabaseConnection, pagination: &Pagination) -> Result<Page<PostResponse>> {
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

pub async fn create(db: &DatabaseConnection, input: CreatePost) -> Result<PostResponse> {
    let post = ActiveModel {
        title: Set(input.title),
        body: Set(input.body),
        published: Set(input.published),
        ..Default::default()
    };

    Ok(repository::create(db, post).await?.into())
}

pub async fn update(db: &DatabaseConnection, id: Uuid, input: UpdatePost) -> Result<PostResponse> {
    let mut post: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("post"))?
        .into();

    // `Option` ne distingue pas un champ absent d'un `null` explicite : cette route ne
    // peut donc pas remettre un champ optionnel à NULL. Ajoutez-y le cas si votre API en
    // a besoin.
    if let Some(title) = input.title {
        post.title = Set(title);
    }
    if let Some(body) = input.body {
        post.body = Set(body);
    }
    if let Some(published) = input.published {
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
