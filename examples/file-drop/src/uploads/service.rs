use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{CreateUpload, UpdateUpload, UploadResponse};
use super::repository::{self, ActiveModel};

pub async fn list(
    db: &DatabaseConnection,
    pagination: &Pagination,
) -> Result<Page<UploadResponse>> {
    let (uploads, total) = repository::list(db, pagination).await?;

    Ok(Page::new(
        uploads.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<UploadResponse> {
    let upload = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("upload"))?;

    Ok(upload.into())
}

pub async fn create(db: &DatabaseConnection, entree: CreateUpload) -> Result<UploadResponse> {
    let upload = ActiveModel {
        title: Set(entree.title),
        owner_email: Set(entree.owner_email),
        content_type: Set(entree.content_type),
        size: Set(entree.size),
        ..Default::default()
    };

    Ok(repository::create(db, upload).await?.into())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    entree: UpdateUpload,
) -> Result<UploadResponse> {
    let mut upload: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("upload"))?
        .into();

    // Un champ absent du corps garde sa valeur : cette route ne peut donc pas remettre un
    // champ optionnel à NULL. Ajoutez-y le cas si votre API en a besoin.
    if let Some(title) = entree.title {
        upload.title = Set(title);
    }
    if let Some(owner_email) = entree.owner_email {
        upload.owner_email = Set(owner_email);
    }
    if let Some(content_type) = entree.content_type {
        upload.content_type = Set(content_type);
    }
    if let Some(size) = entree.size {
        upload.size = Set(size);
    }
    upload.updated_at = Set(chrono::Utc::now().into());

    Ok(repository::update(db, upload).await?.into())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("upload"));
    }

    Ok(())
}
