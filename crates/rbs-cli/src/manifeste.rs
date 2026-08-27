//! Ce qu'un fragment de feature déclare installer, et sa lecture.
//!
//! Le schéma est décrit ici en entier ; son interprétation appartient à `add`.

use std::collections::BTreeMap;

use serde::Deserialize;

/// Ce qu'un fragment installe dans le projet.
///
/// Chaque section est refusée sur un champ inconnu : une clé mal orthographiée dans un
/// manifeste s'installerait autrement en silence, en n'installant rien.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifeste {
    pub feature: Description,
    #[serde(default)]
    pub fichiers: Vec<FichierDeclare>,
    #[serde(default)]
    pub ancres: Vec<InsertionDeclaree>,
    pub migration: Option<MigrationDeclaree>,
    /// Les crates tierces que le fragment déclare, dans l'ordre où elles seront patchées.
    #[serde(default)]
    pub dependances: Vec<DependanceDeclaree>,
    /// Une entrée par crate à patcher. `BTreeMap` et non `HashMap` : l'ordre des patchs
    /// se retrouve dans l'affichage du plan, qui ne doit pas varier d'une exécution à
    /// l'autre.
    #[serde(default)]
    pub cargo: BTreeMap<String, PatchCrate>,
    #[serde(default)]
    pub config: Vec<SectionDeclaree>,
    #[serde(default)]
    pub env: Vec<VariableDeclaree>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Description {
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FichierDeclare {
    pub source: String,
    pub cible: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InsertionDeclaree {
    pub ancre: String,
    pub contenu: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MigrationDeclaree {
    pub source: String,
    pub nom: String,
}

/// Une crate tierce que le fragment apporte au projet.
///
/// La version est figée par le fragment et jamais déduite : un projet généré doit compiler
/// dans six mois avec les versions que le fragment a validées.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DependanceDeclaree {
    pub nom: String,
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
    /// Les défauts de la crate, laissés actifs sauf mention contraire.
    ///
    /// Ce n'est pas une symétrie avec `cargo add` : `lettre` active `native-tls` par
    /// défaut, qui réclamerait OpenSSL sur les trois plateformes d'une CI générée.
    #[serde(default = "vrai")]
    pub default_features: bool,
}

/// Le défaut de `default_features`, serde n'acceptant qu'une fonction.
fn vrai() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PatchCrate {
    pub features: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SectionDeclaree {
    pub fichier: String,
    pub section: String,
    pub contenu: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VariableDeclaree {
    pub cle: String,
    pub valeur: String,
    pub commentaire: Option<String>,
}

/// Ce qui peut empêcher de lire un manifeste.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// Le manifeste ne se désérialise pas.
    #[error("{fichier} est invalide : {source}")]
    Invalide {
        /// Chemin du manifeste fautif.
        fichier: String,
        /// Cause de la désérialisation.
        source: toml_edit::de::Error,
    },
}

/// Lit le manifeste d'un fragment. `nom` ne sert qu'aux messages d'erreur.
pub(crate) fn lire(texte: &str, nom: &str) -> Result<Manifeste, Erreur> {
    toml_edit::de::from_str(texte).map_err(|source| Erreur::Invalide {
        fichier: nom.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPLET: &str = r#"
[feature]
description = "JWT, Argon2, rôles"

[[fichiers]]
source = "model.rs.jinja"
cible  = "src/features/auth/model.rs"

[[ancres]]
ancre   = "features"
contenu = "mod auth;"

[migration]
source = "users.rs.jinja"
nom    = "create_users"

[[dependances]]
nom              = "lettre"
version          = "0.11"
default_features = false
features         = ["smtp-transport", "builder"]

[[dependances]]
nom     = "minijinja"
version = "2.24"

[cargo.rbs-core]
features = ["auth"]

[[config]]
fichier = "config/default.toml"
section = "auth"
contenu = """
access_ttl_secs = 900
refresh_ttl_secs = 2592000
"""

[[env]]
cle         = "RBS_AUTH__SECRET"
valeur      = "changez-moi"
commentaire = "Secret de signature HS256, au moins 32 octets"
"#;

    #[test]
    fn un_manifeste_valide_se_deserialise() {
        let manifeste =
            lire(COMPLET, "features/auth/feature.toml").expect("le manifeste est valide");

        assert_eq!(manifeste.feature.description, "JWT, Argon2, rôles");

        assert_eq!(manifeste.fichiers.len(), 1);
        assert_eq!(manifeste.fichiers[0].source, "model.rs.jinja");
        assert_eq!(manifeste.fichiers[0].cible, "src/features/auth/model.rs");

        assert_eq!(manifeste.ancres.len(), 1);
        assert_eq!(manifeste.ancres[0].ancre, "features");
        assert_eq!(manifeste.ancres[0].contenu, "mod auth;");

        let migration = manifeste.migration.expect("la migration est déclarée");
        assert_eq!(migration.source, "users.rs.jinja");
        assert_eq!(migration.nom, "create_users");

        assert_eq!(manifeste.dependances.len(), 2);
        assert_eq!(manifeste.dependances[0].nom, "lettre");
        assert_eq!(manifeste.dependances[0].version, "0.11");
        assert!(!manifeste.dependances[0].default_features);
        assert_eq!(
            manifeste.dependances[0].features,
            ["smtp-transport", "builder"]
        );

        // Ce qu'un fragment silencieux obtient : les défauts de la crate, et aucune
        // feature de plus.
        assert!(manifeste.dependances[1].default_features);
        assert!(manifeste.dependances[1].features.is_empty());

        assert_eq!(manifeste.cargo.len(), 1);
        assert_eq!(manifeste.cargo["rbs-core"].features, ["auth"]);

        assert_eq!(manifeste.config.len(), 1);
        assert_eq!(manifeste.config[0].fichier, "config/default.toml");
        assert_eq!(manifeste.config[0].section, "auth");
        assert_eq!(
            manifeste.config[0].contenu,
            "access_ttl_secs = 900\nrefresh_ttl_secs = 2592000\n"
        );

        assert_eq!(manifeste.env.len(), 1);
        assert_eq!(manifeste.env[0].cle, "RBS_AUTH__SECRET");
        assert_eq!(manifeste.env[0].valeur, "changez-moi");
        assert_eq!(
            manifeste.env[0].commentaire.as_deref(),
            Some("Secret de signature HS256, au moins 32 octets")
        );
    }

    #[test]
    fn un_champ_inconnu_nomme_le_champ_et_le_fichier() {
        let erreur = lire(
            "[feature]\ndescription = \"x\"\ninconnu = 1\n",
            "features/auth/feature.toml",
        )
        .unwrap_err();

        let message = erreur.to_string();
        assert!(message.contains("inconnu"), "{message}");
        assert!(message.contains("features/auth/feature.toml"), "{message}");
    }

    #[test]
    fn un_manifeste_minimal_ne_declare_que_sa_description() {
        let manifeste = lire(
            "[feature]\ndescription = \"docker\"\n",
            "docker/feature.toml",
        )
        .expect("un manifeste réduit à sa description est valide");

        assert_eq!(manifeste.feature.description, "docker");
        assert!(manifeste.fichiers.is_empty());
        assert!(manifeste.ancres.is_empty());
        assert!(manifeste.migration.is_none());
        assert!(manifeste.dependances.is_empty());
        assert!(manifeste.cargo.is_empty());
        assert!(manifeste.config.is_empty());
        assert!(manifeste.env.is_empty());
    }
}
