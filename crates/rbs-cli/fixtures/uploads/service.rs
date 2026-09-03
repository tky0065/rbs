use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{CreateUpload, UpdateUpload, UploadResponse};
use super::filter::UploadFilter;
use super::repository::{self, ActiveModel};
use crate::storage::{Storage, StorageError};

/// Clé du contenu déposé pour `id`.
///
/// Le stockage est un magasin plat : c'est ce préfixe qui range les objets de cette
/// ressource, et rien d'autre ne les distingue.
fn content_key(id: Uuid) -> String {
    format!("uploads/{id}")
}

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

pub async fn filter(
    db: &DatabaseConnection,
    filtre: &UploadFilter,
    pagination: &Pagination,
) -> Result<Page<UploadResponse>> {
    let (uploads, total) = repository::filter(db, filtre, pagination).await?;

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

pub async fn create(db: &DatabaseConnection, input: CreateUpload) -> Result<UploadResponse> {
    let upload = ActiveModel {
        title: Set(input.title),
        owner_email: Set(input.owner_email),
        content_type: Set(input.content_type),
        size: Set(input.size),
        ..Default::default()
    };

    Ok(repository::create(db, upload).await?.into())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateUpload,
) -> Result<UploadResponse> {
    let mut upload: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("upload"))?
        .into();

    // `Option` ne distingue pas un champ absent d'un `null` explicite : cette route ne
    // peut donc pas remettre un champ optionnel à NULL. Ajoutez-y le cas si votre API en
    // a besoin.
    if let Some(title) = input.title {
        upload.title = Set(title);
    }
    if let Some(owner_email) = input.owner_email {
        upload.owner_email = Set(owner_email);
    }
    if let Some(content_type) = input.content_type {
        upload.content_type = Set(content_type);
    }
    if let Some(size) = input.size {
        upload.size = Set(size);
    }
    upload.updated_at = Set(chrono::Utc::now().into());

    Ok(repository::update(db, upload).await?.into())
}

pub async fn delete(db: &DatabaseConnection, storage: &dyn Storage, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("upload"));
    }

    // Le contenu part avec la ligne. `delete` est idempotent des deux côtés du trait :
    // une ressource créée sans contenu ne fait donc pas échouer sa suppression.
    storage
        .delete(&content_key(id))
        .await
        .map_err(|error| Error::Internal(anyhow::anyhow!("{error}")))
}

/// Dépose le contenu de `id`, la ressource devant exister.
pub async fn put_content(
    db: &DatabaseConnection,
    storage: &dyn Storage,
    id: Uuid,
    content: Vec<u8>,
) -> Result<()> {
    // La ligne est lue avant le dépôt : sans elle, le magasin accumulerait des objets
    // qu'aucune ressource ne réclame.
    repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("upload"))?;

    storage
        .put(&content_key(id), content)
        .await
        .map_err(|error| Error::Internal(anyhow::anyhow!("{error}")))
}

/// Un contenu est-il déposé pour `id` ?
///
/// `exists` plutôt qu'un `get` dont on jetterait le corps : la question ne demande pas de
/// transférer l'objet, et les deux backends savent y répondre sans le lire.
pub async fn has_content(storage: &dyn Storage, id: Uuid) -> Result<bool> {
    storage
        .exists(&content_key(id))
        .await
        .map_err(|error| Error::Internal(anyhow::anyhow!("{error}")))
}

/// Rend le contenu déposé pour `id`.
pub async fn get_content(storage: &dyn Storage, id: Uuid) -> Result<Vec<u8>> {
    storage
        .get(&content_key(id))
        .await
        // `NotFound` est le seul cas qui vienne du client : les autres sont des pannes.
        .map_err(|error| match error {
            StorageError::NotFound(_) => Error::NotFound("contenu"),
            autre => Error::Internal(anyhow::anyhow!("{autre}")),
        })
}
