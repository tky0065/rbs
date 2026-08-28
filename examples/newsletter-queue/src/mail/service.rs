use std::time::Duration;

use anyhow::Context;
use lettre::message::Mailbox;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use rbs_core::{Error, Result};
use serde::Serialize;

use super::config::{MailConfig, Tls};
use super::template::Templates;

/// Le transport SMTP, l'expéditeur et les gabarits du projet, clonés avec l'état.
#[derive(Debug, Clone)]
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    sender: Mailbox,
    templates: Templates,
}

impl Mailer {
    /// Bâtit le transport depuis la section `[mail]`.
    // region: construction
    pub fn from_config() -> anyhow::Result<Self> {
        let config = rbs_core::config::section::<MailConfig>("mail")
            .context("section [mail] de la configuration")?;

        Self::new(&config)
    }

    /// Faillible mais synchrone : rien n'est ouvert ici, la première connexion attend le
    /// premier message. L'expéditeur est analysé maintenant pour qu'une faute de frappe
    /// arrête le démarrage plutôt que le premier envoi.
    ///
    /// À appeler depuis un runtime Tokio — celui de `main` — sans quoi le pool de `lettre`
    /// panique en y inscrivant sa tâche d'entretien.
    pub fn new(config: &MailConfig) -> anyhow::Result<Self> {
        let hote = config.smtp_host.as_str();
        let batisseur = match config.tls {
            Tls::Aucun => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(hote),
            Tls::Starttls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(hote)?,
            Tls::Wrapper => AsyncSmtpTransport::<Tokio1Executor>::relay(hote)?,
        }
        .port(config.smtp_port)
        .timeout(Some(Duration::from_secs(config.timeout_secs)));

        let batisseur = if config.smtp_user.is_empty() {
            batisseur
        } else {
            batisseur.credentials(Credentials::new(
                config.smtp_user.clone(),
                config.smtp_password.clone(),
            ))
        };

        Ok(Self {
            transport: batisseur.build(),
            sender: config
                .from
                .parse()
                .with_context(|| format!("expéditeur « {} »", config.from))?,
            templates: Templates::new(&config.templates),
        })
    }
    // endregion: construction

    /// Prépare un message HTML de l'expéditeur configuré vers `recipient`.
    pub fn message(&self, recipient: &str, subject: &str, body: String) -> Result<Message> {
        let recipient: Mailbox = recipient.parse().map_err(internal)?;

        Message::builder()
            .from(self.sender.clone())
            .to(recipient)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body)
            .map_err(internal)
    }

    pub async fn send(&self, message: Message) -> Result<()> {
        self.transport.send(message).await.map_err(internal)?;

        Ok(())
    }

    /// Rend `template` avec `context`, et envoie le résultat.
    // region: send_template
    pub async fn send_template<S: Serialize>(
        &self,
        recipient: &str,
        subject: &str,
        template: &str,
        context: S,
    ) -> Result<()> {
        let body = self.templates.render(template, context)?;

        self.send(self.message(recipient, subject, body)?).await
    }
    // endregion: send_template

    // region: send_detached
    /// Lance l'envoi et rend la main sans l'attendre.
    ///
    /// Ni file ni réessai : un message perdu l'est pour de bon, et seul le journal en
    /// garde trace. C'est le prix d'un envoi qui ne retient pas la réponse HTTP.
    ///
    /// Ce projet passe par un job plutôt que par cette porte, et la garde ouverte : un
    /// message dont la perte est sans conséquence n'a pas besoin d'une ligne en base.
    #[allow(dead_code)]
    pub fn send_detached(&self, message: Message) {
        let transport = self.transport.clone();

        tokio::spawn(async move {
            if let Err(error) = transport.send(message).await {
                tracing::error!(%error, "envoi de courriel échoué");
            }
        });
    }
    // endregion: send_detached
}

/// Une panne du transport n'apprend rien au client : elle reste au journal du serveur.
fn internal(source: impl std::error::Error + Send + Sync + 'static) -> Error {
    Error::Internal(source.into())
}
