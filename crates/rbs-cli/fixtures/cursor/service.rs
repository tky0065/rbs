use rbs_core::{Cursor, CursorPage, Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{ArticleResponse, CreateArticle, UpdateArticle};
use super::filter::ArticleFilter;
use super::repository::{self, ActiveModel};

pub async fn list(db: &DatabaseConnection, cursor: &Cursor) -> Result<CursorPage<ArticleResponse>> {
    let articles = repository::list(db, cursor).await?;
    let dernier = articles.last().map(|ligne| ligne.id);

    Ok(CursorPage::new(
        articles.into_iter().map(Into::into).collect(),
        cursor,
        dernier,
    ))
}

pub async fn filter(
    db: &DatabaseConnection,
    filtre: &ArticleFilter,
    pagination: &Pagination,
) -> Result<Page<ArticleResponse>> {
    let (articles, total) = repository::filter(db, filtre, pagination).await?;

    Ok(Page::new(
        articles.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<ArticleResponse> {
    let article = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("article"))?;

    Ok(article.into())
}

pub async fn create(db: &DatabaseConnection, input: CreateArticle) -> Result<ArticleResponse> {
    let article = ActiveModel {
        title: Set(input.title),
        ..Default::default()
    };

    Ok(repository::create(db, article).await?.into())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateArticle,
) -> Result<ArticleResponse> {
    let mut article: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("article"))?
        .into();

    // `Option` ne distingue pas un champ absent d'un `null` explicite : cette route ne
    // peut donc pas remettre un champ optionnel à NULL. Ajoutez-y le cas si votre API en
    // a besoin.
    if let Some(title) = input.title {
        article.title = Set(title);
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
