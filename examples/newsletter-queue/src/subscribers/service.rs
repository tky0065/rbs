use rbs_core::{Error, Page, Pagination, Result};
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::{DatabaseConnection, TransactionTrait};

use super::dto::{Broadcast, CreateSubscriber, SubscriberResponse, UpdateSubscriber};
use super::repository::{self, ActiveModel};
use crate::jobs;
use crate::jobs::newsletter::SendNewsletter;

// region: broadcast
/// Enfile une lettre par abonné confirmé, et rend leur nombre.
///
/// La lecture et les enfilages partagent une transaction : ou bien la campagne est
/// entière, ou bien elle n'existe pas. C'est ce qu'une file en mémoire ne saurait pas
/// tenir — un job poussé dans Redis survivrait au rollback qui l'a annulé, et c'est la
/// raison pour laquelle la file de ce projet est une table.
///
/// Rien n'est envoyé ici. Le worker dépile ensuite, et un SMTP en panne fait réessayer la
/// lettre au lieu de la perdre : la réponse HTTP est partie depuis longtemps.
pub async fn broadcast(db: &DatabaseConnection, input: Broadcast) -> Result<usize> {
    let transaction = db.begin().await?;

    let subscribers = repository::confirmed(&transaction).await?;

    for subscriber in &subscribers {
        jobs::enqueue(
            &transaction,
            &SendNewsletter {
                subscriber: subscriber.id,
                subject: input.subject.clone(),
                body: input.body.clone(),
            },
        )
        .await?;
    }

    transaction.commit().await?;

    Ok(subscribers.len())
}
// endregion: broadcast

pub async fn list(
    db: &DatabaseConnection,
    pagination: &Pagination,
) -> Result<Page<SubscriberResponse>> {
    let (subscribers, total) = repository::list(db, pagination).await?;

    Ok(Page::new(
        subscribers.into_iter().map(Into::into).collect(),
        pagination,
        total,
    ))
}

pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<SubscriberResponse> {
    let subscriber = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("subscriber"))?;

    Ok(subscriber.into())
}

pub async fn create(
    db: &DatabaseConnection,
    input: CreateSubscriber,
) -> Result<SubscriberResponse> {
    let subscriber = ActiveModel {
        email: Set(input.email),
        name: Set(input.name),
        confirmed: Set(input.confirmed),
        ..Default::default()
    };

    Ok(repository::create(db, subscriber).await?.into())
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateSubscriber,
) -> Result<SubscriberResponse> {
    let mut subscriber: ActiveModel = repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("subscriber"))?
        .into();

    // `Option` ne distingue pas un champ absent d'un `null` explicite : cette route ne
    // peut donc pas remettre un champ optionnel à NULL. Ajoutez-y le cas si votre API en
    // a besoin.
    if let Some(email) = input.email {
        subscriber.email = Set(email);
    }
    if let Some(name) = input.name {
        subscriber.name = Set(name);
    }
    if let Some(confirmed) = input.confirmed {
        subscriber.confirmed = Set(confirmed);
    }
    subscriber.updated_at = Set(chrono::Utc::now().into());

    Ok(repository::update(db, subscriber).await?.into())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("subscriber"));
    }

    Ok(())
}
