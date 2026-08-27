use serde::Deserialize;

/// Réglages du transport, section `[mail]` de la configuration.
///
/// Les défauts vivent ici et non dans le noyau, qui n'en oppose aucun à une section qu'il
/// ne connaît pas : c'est donc dans ce fichier qu'ils se lisent et se changent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct MailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    /// Vient de `RBS_MAIL__SMTP_PASSWORD` : aucun fichier versionné ne le porte.
    pub smtp_password: String,
    pub tls: Tls,
    /// Expéditeur, sous la forme `adresse` ou `Nom <adresse>`.
    pub from: String,
    pub timeout_secs: u64,
    /// Répertoire des gabarits de messages, relatif à la racine du projet.
    pub gabarits: String,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_user: String::new(),
            smtp_password: String::new(),
            tls: Tls::Aucun,
            from: "no-reply@localhost".to_string(),
            timeout_secs: 10,
            gabarits: "templates/mail".to_string(),
        }
    }
}

/// Chiffrement de la connexion SMTP.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tls {
    /// En clair, ce qui ne convient qu'à un serveur de développement local.
    #[default]
    #[serde(rename = "none")]
    Aucun,
    /// Connexion en clair puis `STARTTLS` : ce qu'attend le port 587.
    Starttls,
    /// Connexion chiffrée dès son ouverture : ce qu'attend le port 465.
    Wrapper,
}
