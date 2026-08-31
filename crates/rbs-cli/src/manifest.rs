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
pub(crate) struct Manifest {
    pub feature: Description,
    #[serde(default)]
    pub files: Vec<DeclaredFile>,
    #[serde(default)]
    pub anchors: Vec<DeclaredInsertion>,
    pub migration: Option<DeclaredMigration>,
    /// Les crates tierces que le fragment déclare, dans l'ordre où elles seront patchées.
    #[serde(default)]
    pub dependencies: Vec<DeclaredDependency>,
    /// Une entrée par crate à patcher. `BTreeMap` et non `HashMap` : l'ordre des patchs
    /// se retrouve dans l'affichage du plan, qui ne doit pas varier d'une exécution à
    /// l'autre.
    #[serde(default)]
    pub cargo: BTreeMap<String, PatchCrate>,
    #[serde(default)]
    pub config: Vec<DeclaredSection>,
    #[serde(default)]
    pub env: Vec<DeclaredVariable>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Description {
    pub description: String,
    /// Les fragments sans lesquels celui-ci n'installe qu'une moitié de ce qu'il promet.
    ///
    /// `auth` exige `rate-limit` : sa route de connexion hache un Argon2 même pour un
    /// email inconnu, et sans limite de débit cette protection contre l'énumération
    /// devient un déni de service à la portée de n'importe qui.
    #[serde(default)]
    pub requires: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclaredFile {
    pub source: String,
    pub destination: String,
    /// Le fichier n'est déposé que si le projet ne le porte pas déjà.
    ///
    /// Réservé aux fichiers qu'un fragment étend d'ordinaire par une ancre : sans ce
    /// repli, un projet antérieur à l'ancre n'aurait aucun moyen d'obtenir le fichier.
    #[serde(default)]
    pub if_absent: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclaredInsertion {
    pub anchor: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclaredMigration {
    pub source: String,
    pub name: String,
}

/// Une crate tierce que le fragment apporte au projet.
///
/// La version est figée par le fragment et jamais déduite : un projet généré doit compiler
/// dans six mois avec les versions que le fragment a validées.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclaredDependency {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
    /// Les défauts de la crate, laissés actifs sauf mention contraire.
    ///
    /// Ce n'est pas une symétrie avec `cargo add` : `lettre` active `native-tls` par
    /// défaut, qui réclamerait OpenSSL sur les trois plateformes d'une CI générée.
    #[serde(default = "truthy")]
    pub default_features: bool,
}

/// Le défaut de `default_features`, serde n'acceptant qu'une fonction.
fn truthy() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PatchCrate {
    pub features: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclaredSection {
    pub file: String,
    pub section: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclaredVariable {
    pub key: String,
    /// Ce que reçoit `.env.example`, versionné : une valeur de démonstration, jamais une
    /// vraie. Rendue dans le contexte du fragment, comme `project_value`.
    pub value: String,
    pub comment: Option<String>,
    /// La variable porte un secret propre à chaque installation.
    ///
    /// `value` reste l'exemple versionné, que `doctor` compare au `.env` pour dire si le
    /// développeur l'a remplacé ; c'est le `.env`, gitignoré, qui reçoit la valeur tirée.
    #[serde(default)]
    pub secret: bool,
    /// Ce que reçoit le `.env` du projet, quand la valeur se déduit du projet lui-même.
    ///
    /// Le service `db` d'un compose interpole les identifiants de la base : ce sont ceux
    /// de l'URL du projet, et non un tirage. `secret` répond à l'autre besoin — une valeur
    /// qu'aucun exemple publié ne doit permettre de deviner ; déclarer les deux n'a pas de
    /// sens, et `project_value` l'emporte alors.
    pub project_value: Option<String>,
    /// L'expression Jinja sous laquelle la variable est déclarée, ou rien pour toujours.
    ///
    /// Les clés du service `db` dépendent du moteur, et `MYSQL_USER` du compte : une image
    /// MySQL à qui l'on redemande `root` refuse de s'initialiser.
    pub when: Option<String>,
}

/// Ce qui peut empêcher de lire un manifeste.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Le manifeste ne se désérialise pas.
    #[error("{file} est invalide : {source}")]
    Invalide {
        /// Chemin du manifeste fautif.
        file: String,
        /// Cause de la désérialisation.
        source: toml_edit::de::Error,
    },
}

/// Lit le manifeste d'un fragment. `name` ne sert qu'aux messages d'erreur.
pub(crate) fn read(text: &str, name: &str) -> Result<Manifest, Error> {
    toml_edit::de::from_str(text).map_err(|source| Error::Invalide {
        file: name.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPLET: &str = r#"
[feature]
description = "JWT, Argon2, rôles"

[[files]]
source = "model.rs.jinja"
destination  = "src/features/auth/model.rs"

[[anchors]]
anchor   = "features"
content = "mod auth;"

[migration]
source = "users.rs.jinja"
name    = "create_users"

[[dependencies]]
name              = "lettre"
version          = "0.11"
default_features = false
features         = ["smtp-transport", "builder"]

[[dependencies]]
name     = "minijinja"
version = "2.24"

[cargo.rbs-core]
features = ["auth"]

[[config]]
file = "config/default.toml"
section = "auth"
content = """
access_ttl_secs = 900
refresh_ttl_secs = 2592000
"""

[[env]]
key         = "RBS_AUTH__SECRET"
value      = "changez-moi"
comment = "Secret de signature HS256, au moins 32 octets"
"#;

    /// Un fragment qui n'installe qu'une moitié de ce qu'il promet sans un autre le dit
    /// dans son `[feature]`, et le CLI n'a pas à connaître la paire par son nom.
    #[test]
    fn a_fragment_can_declare_what_it_requires() {
        let manifest = read(
            "[feature]\ndescription = \"auth\"\nrequires = [\"rate-limit\"]\n",
            "features/auth/feature.toml",
        )
        .expect("le manifeste est valide");

        assert_eq!(manifest.feature.requires, ["rate-limit"]);
    }

    #[test]
    fn a_valid_manifest_deserialises() {
        let manifest =
            read(COMPLET, "features/auth/feature.toml").expect("le manifeste est valide");

        assert_eq!(manifest.feature.description, "JWT, Argon2, rôles");
        // Le défaut, qui vaut pour tous les fragments sauf `auth` : rien n'est entraîné.
        assert!(manifest.feature.requires.is_empty());

        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].source, "model.rs.jinja");
        assert_eq!(manifest.files[0].destination, "src/features/auth/model.rs");

        assert_eq!(manifest.anchors.len(), 1);
        assert_eq!(manifest.anchors[0].anchor, "features");
        assert_eq!(manifest.anchors[0].content, "mod auth;");

        let migration = manifest.migration.expect("la migration est déclarée");
        assert_eq!(migration.source, "users.rs.jinja");
        assert_eq!(migration.name, "create_users");

        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.dependencies[0].name, "lettre");
        assert_eq!(manifest.dependencies[0].version, "0.11");
        assert!(!manifest.dependencies[0].default_features);
        assert_eq!(
            manifest.dependencies[0].features,
            ["smtp-transport", "builder"]
        );

        // Ce qu'un fragment silencieux obtient : les défauts de la crate, et aucune
        // feature de plus.
        assert!(manifest.dependencies[1].default_features);
        assert!(manifest.dependencies[1].features.is_empty());

        assert_eq!(manifest.cargo.len(), 1);
        assert_eq!(manifest.cargo["rbs-core"].features, ["auth"]);

        assert_eq!(manifest.config.len(), 1);
        assert_eq!(manifest.config[0].file, "config/default.toml");
        assert_eq!(manifest.config[0].section, "auth");
        assert_eq!(
            manifest.config[0].content,
            "access_ttl_secs = 900\nrefresh_ttl_secs = 2592000\n"
        );

        assert_eq!(manifest.env.len(), 1);
        assert_eq!(manifest.env[0].key, "RBS_AUTH__SECRET");
        assert_eq!(manifest.env[0].value, "changez-moi");
        assert_eq!(
            manifest.env[0].comment.as_deref(),
            Some("Secret de signature HS256, au moins 32 octets")
        );
        assert_eq!(manifest.env[0].project_value, None);
        assert_eq!(manifest.env[0].when, None);
    }

    /// Une variable dont la valeur se déduit du projet, sous la condition qui la déclare.
    #[test]
    fn a_variable_can_declare_a_project_value_under_a_condition() {
        let manifest = read(
            "[feature]\ndescription = \"docker\"\n\n\
             [[env]]\nkey = \"POSTGRES_PASSWORD\"\nvalue = \"postgres\"\n\
             project_value = \"{@ database_password @}\"\n\
             when = \"database == 'postgres'\"\n",
            "docker/feature.toml",
        )
        .expect("le manifeste doit être valide");

        assert_eq!(
            manifest.env[0].project_value.as_deref(),
            Some("{@ database_password @}")
        );
        assert_eq!(
            manifest.env[0].when.as_deref(),
            Some("database == 'postgres'")
        );
    }

    #[test]
    fn an_unknown_field_names_the_field_and_the_file() {
        let error = read(
            "[feature]\ndescription = \"x\"\ninconnu = 1\n",
            "features/auth/feature.toml",
        )
        .unwrap_err();

        let message = error.to_string();
        assert!(message.contains("inconnu"), "{message}");
        assert!(message.contains("features/auth/feature.toml"), "{message}");
    }

    #[test]
    fn a_minimal_manifest_declares_only_its_description() {
        let manifest = read(
            "[feature]\ndescription = \"docker\"\n",
            "docker/feature.toml",
        )
        .expect("un manifeste réduit à sa description est valide");

        assert_eq!(manifest.feature.description, "docker");
        assert!(manifest.files.is_empty());
        assert!(manifest.anchors.is_empty());
        assert!(manifest.migration.is_none());
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.cargo.is_empty());
        assert!(manifest.config.is_empty());
        assert!(manifest.env.is_empty());
    }
}
