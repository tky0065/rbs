use async_trait::async_trait;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_sdk_s3::primitives::ByteStream;

use super::{Storage, StorageConfig, StorageError, normalize};

/// Stockage sur S3, ou sur toute API qui en suit le protocole.
#[derive(Debug, Clone)]
pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    /// Dérive un client d'une configuration déjà résolue, sans joindre le réseau.
    ///
    /// Les identifiants viennent de la configuration et non de la chaîne de fournisseurs
    /// par défaut du SDK : celle-ci est asynchrone et interroge le service de métadonnées
    /// de l'instance, ce qu'un `AppState::new` synchrone ne peut ni lancer ni attendre.
    pub fn new(config: &StorageConfig) -> Self {
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

fn unavailable<E: std::error::Error + Send + Sync + 'static>(error: E) -> StorageError {
    StorageError::Unavailable(Box::new(error))
}

#[async_trait]
impl Storage for S3Storage {
    async fn put(&self, key: &str, content: Vec<u8>) -> Result<(), StorageError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(normalize(key)?)
            .body(ByteStream::from(content))
            .send()
            .await
            .map_err(unavailable)?;

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(normalize(key)?)
            .send()
            .await
            .map_err(|error| match error.as_service_error() {
                Some(service) if service.is_no_such_key() => StorageError::NotFound(key.to_owned()),
                _ => unavailable(error),
            })?;

        Ok(object.body.collect().await.map_err(unavailable)?.to_vec())
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(normalize(key)?)
            .send()
            .await
            .map_err(unavailable)?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(normalize(key)?)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(error)
                if error
                    .as_service_error()
                    .is_some_and(HeadObjectError::is_not_found) =>
            {
                Ok(false)
            }
            Err(error) => Err(unavailable(error)),
        }
    }

    // `head_bucket` plutôt qu'une lecture d'objet : la requête ne transporte rien, et
    // elle éprouve à la fois le réseau, les identifiants et l'existence du bucket.
    async fn available(&self) -> bool {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .is_ok()
    }
}
