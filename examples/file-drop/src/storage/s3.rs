use async_trait::async_trait;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_sdk_s3::primitives::ByteStream;

use super::{Storage, StorageConfig, StorageError, normaliser};

/// Stockage sur S3, ou sur toute API qui en suit le protocole.
#[derive(Debug, Clone)]
pub struct StockageS3 {
    client: Client,
    bucket: String,
}

impl StockageS3 {
    /// Dérive un client d'une configuration déjà résolue, sans joindre le réseau.
    ///
    /// Les identifiants viennent de la configuration et non de la chaîne de fournisseurs
    /// par défaut du SDK : celle-ci est asynchrone et interroge le service de métadonnées
    /// de l'instance, ce qu'un `AppState::new` synchrone ne peut ni lancer ni attendre.
    pub fn nouveau(config: &StorageConfig) -> Self {
        let identifiants = Credentials::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            None,
            None,
            "rbs-storage",
        );

        let mut client = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .credentials_provider(identifiants)
            .force_path_style(config.force_path_style);

        if let Some(endpoint) = &config.endpoint {
            client = client.endpoint_url(endpoint);
        }

        Self {
            client: Client::from_conf(client.build()),
            bucket: config.bucket.clone(),
        }
    }
}

fn indisponible<E: std::error::Error + Send + Sync + 'static>(erreur: E) -> StorageError {
    StorageError::Indisponible(Box::new(erreur))
}

#[async_trait]
impl Storage for StockageS3 {
    async fn deposer(&self, cle: &str, contenu: Vec<u8>) -> Result<(), StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(normaliser(cle)?)
            .body(ByteStream::from(contenu))
            .send()
            .await
            .map_err(indisponible)?;

        Ok(())
    }

    async fn lire(&self, cle: &str) -> Result<Vec<u8>, StorageError> {
        let objet = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(normaliser(cle)?)
            .send()
            .await
            .map_err(|erreur| match erreur.as_service_error() {
                Some(service) if service.is_no_such_key() => {
                    StorageError::Introuvable(cle.to_owned())
                }
                _ => indisponible(erreur),
            })?;

        Ok(objet.body.collect().await.map_err(indisponible)?.to_vec())
    }

    async fn supprimer(&self, cle: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(normaliser(cle)?)
            .send()
            .await
            .map_err(indisponible)?;

        Ok(())
    }

    async fn existe(&self, cle: &str) -> Result<bool, StorageError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(normaliser(cle)?)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(erreur)
                if erreur
                    .as_service_error()
                    .is_some_and(HeadObjectError::is_not_found) =>
            {
                Ok(false)
            }
            Err(erreur) => Err(indisponible(erreur)),
        }
    }
}
