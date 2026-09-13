use serde::Deserialize;

/// Ce que les parcours de réinitialisation et de vérification lisent dans `[auth]`.
///
/// Ces trois clés vivent ici et non dans `rbs_core::config::AuthConfig`, qui les
/// ignorerait : le noyau porte ce qui ne varie pas d'un projet à l'autre, et l'adresse de
/// votre application n'entre pas dans cette catégorie. C'est donc ce fichier que vous
/// ouvrirez pour changer les durées ou l'URL des liens.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct FlowConfig {
    /// Durée de vie du lien de réinitialisation, en secondes.
    pub reset_ttl_secs: u64,
    /// Durée de vie du lien de vérification, en secondes.
    pub verification_ttl_secs: u64,
    /// Racine des liens envoyés par courriel, sans barre finale.
    pub app_url: String,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            reset_ttl_secs: 3600,
            verification_ttl_secs: 86_400,
            app_url: "http://localhost:3000".to_string(),
        }
    }
}

impl FlowConfig {
    /// Lit la section `[auth]`, dont elle ne retient que ses trois clés.
    pub fn from_config() -> anyhow::Result<Self> {
        Ok(rbs_core::config::section::<Self>("auth")?)
    }

    /// Le lien à mettre dans un courriel, `app_url` et la barre finale réconciliées.
    ///
    /// La barre est retirée plutôt que supposée absente : `http://exemple.test/` dans un
    /// fichier de configuration est aussi naturel que sans, et donnerait sinon un lien à
    /// double barre que certains clients de messagerie coupent.
    pub fn link(&self, path: &str, token: &str) -> String {
        format!(
            "{}/{path}?token={token}",
            self.app_url.trim_end_matches('/')
        )
    }
}
