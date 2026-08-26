// region: imports
use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{ArticleResponse, CreateArticle, UpdateArticle};
use super::repository::{self, ActiveModel};
// endregion: imports

pub async fn list(
    db: &DatabaseConnection,
    pagination: &Pagination,
) -> Result<Page<ArticleResponse>> {
    let (articles, total) = repository::list(db, pagination).await?;

    Ok(Page::new(
        articles.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

// region: find
pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<ArticleResponse> {
    let article = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("article"))?;

    Ok(article.into())
}
// endregion: find

pub async fn create(db: &DatabaseConnection, entree: CreateArticle) -> Result<ArticleResponse> {
    let article = ActiveModel {
        title: Set(entree.title),
        body: Set(entree.body),
        published: Set(entree.published),
        ..Default::default()
    };

    Ok(repository::create(db, article).await?.into())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    entree: UpdateArticle,
) -> Result<ArticleResponse> {
    let mut article: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("article"))?
        .into();

    // Un champ absent du corps garde sa valeur : cette route ne peut donc pas remettre un
    // champ optionnel à NULL. Ajoutez-y le cas si votre API en a besoin.
    if let Some(title) = entree.title {
        article.title = Set(title);
    }
    if let Some(body) = entree.body {
        article.body = Set(body);
    }
    if let Some(published) = entree.published {
        article.published = Set(published);
    }
    article.updated_at = Set(chrono::Utc::now().into());

    Ok(repository::update(db, article).await?.into())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("article"));
    }

    Ok(())
}
