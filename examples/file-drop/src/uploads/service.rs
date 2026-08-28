// region: imports
use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::dto::{CreateUpload, UpdateUpload, UploadResponse};
use super::repository::{self, ActiveModel};
use crate::cache::Cache;
use crate::mail::Mailer;
use crate::storage::Storage;
use minijinja::context;
// endregion: imports

/// Préfixe des clés de cache de la ressource.
///
/// Toutes les pages le partagent : une écriture les invalide d'un seul appel, sans
/// savoir combien de pages ont été servies.
const CACHE: &str = "uploads:";

/// Clé du contenu déposé pour `id`.
///
/// Le stockage est un magasin plat : c'est ce préfixe qui range les objets de cette
/// ressource, et rien d'autre ne les distingue.
fn content_key(id: Uuid) -> String {
    format!("uploads/{id}")
}

// region: list
/// La page vient de la base ; son total vient du cache.
///
/// C'est le `COUNT(*)` que l'on met en cache, non la page : il parcourt toute la table à
/// chaque appel, quand la page n'en lit que `per_page` lignes. Et `Page` ne se
/// désérialise pas — elle n'est que `Serialize`, ce qui suffit à la rendre et interdit de
/// la relire du cache sans toucher au noyau.
pub async fn list(
    db: &DatabaseConnection,
    cache: &Cache,
    pagination: &Pagination,
) -> Result<Page<UploadResponse>> {
    let key = format!("{CACHE}total");

    let (uploads, total) = match cache.get::<u64>(&key).await? {
        Some(total) => (repository::page(db, pagination).await?, total),
        None => {
            let (uploads, total) = repository::list(db, pagination).await?;
            cache.set(&key, &total).await?;
            (uploads, total)
        }
    };

    Ok(Page::new(
        uploads.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}
// endregion: list

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<UploadResponse> {
    let upload = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("upload"))?;

    Ok(upload.into())
}

// region: create
pub async fn create(
    db: &DatabaseConnection,
    cache: &Cache,
    mailer: &Mailer,
    input: CreateUpload,
) -> Result<UploadResponse> {
    let recipient = input.owner_email.clone();
    let upload = ActiveModel {
        title: Set(input.title),
        owner_email: Set(input.owner_email),
        content_type: Set(input.content_type),
        size: Set(input.size),
        ..Default::default()
    };

    let upload: UploadResponse = repository::create(db, upload).await?.into();

    cache.invalidate_prefix(CACHE).await?;
    notify(mailer, &recipient, &upload);

    Ok(upload)
}
// endregion: create

// region: notify
/// Prévient le déposant, sans retenir la réponse HTTP.
///
/// L'envoi part dans sa propre tâche : un SMTP lent ferait autrement attendre le client
/// pour un message qui ne conditionne pas la création. Ni file ni réessai — un message
/// perdu l'est pour de bon, et seul le journal en garde trace. C'est le compromis que le
/// fragment documente pour `send_detached`, et il vaut aussi pour un gabarit.
fn notify(mailer: &Mailer, recipient: &str, upload: &UploadResponse) {
    let mailer = mailer.clone();
    let recipient = recipient.to_owned();
    let context = context! {
        title => upload.title.clone(),
        link => format!("/uploads/{}/content", upload.id),
    };

    tokio::spawn(async move {
        let envoi = mailer
            .send_template(
                &recipient,
                "Votre dépôt est en ligne",
                "depot.html",
                context,
            )
            .await;

        // L'échec ne remonte nulle part : la réponse est déjà partie, et la ligne est en
        // base. Le journal est le seul endroit où il puisse être vu.
        if let Err(error) = envoi {
            tracing::error!(%error, "courriel de dépôt non envoyé");
        }
    });
}
// endregion: notify

pub async fn update(
    db: &DatabaseConnection,
    cache: &Cache,
    id: Uuid,
    input: UpdateUpload,
) -> Result<UploadResponse> {
    let mut upload: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("upload"))?
        .into();

    // Un champ absent du corps garde sa valeur : cette route ne peut donc pas remettre un
    // champ optionnel à NULL. Ajoutez-y le cas si votre API en a besoin.
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

    let upload = repository::update(db, upload).await?;
    cache.invalidate_prefix(CACHE).await?;

    Ok(upload.into())
}

// region: delete
pub async fn delete(
    db: &DatabaseConnection,
    cache: &Cache,
    storage: &dyn Storage,
    id: Uuid,
) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("upload"));
    }

    // Le contenu part avec la ligne : `delete` est idempotent des deux côtés du trait,
    // une ressource créée sans contenu ne fait donc pas échouer sa suppression.
    storage
        .delete(&content_key(id))
        .await
        .map_err(|error| Error::Internal(anyhow::anyhow!("{error}")))?;
    cache.invalidate_prefix(CACHE).await?;

    Ok(())
}
// endregion: delete

// region: contenu
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
/// `exists` plutôt qu'un `get` dont on jetterait le corps : la question ne demande pas
/// de transférer l'objet, et les deux backends savent y répondre sans le lire.
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
        // `Introuvable` est le seul cas qui vient du client : les autres sont des pannes.
        .map_err(|error| match error {
            crate::storage::StorageError::NotFound(_) => Error::NotFound("contenu"),
            autre => Error::Internal(anyhow::anyhow!("{autre}")),
        })
}
// endregion: contenu
