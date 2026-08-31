//! Lecture et mise à jour de `[package.metadata.rbs]`.
//!
//! Le `Cargo.toml` du projet est le seul endroit où rbs garde son état : version qui a
//! généré le projet, features installées. Un fichier de plus, non versionné par
//! réflexe, se serait désynchronisé du dépôt.

use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

use crate::database::Database;

/// Ce qui peut empêcher de désigner la racine d'un projet rbs.
#[derive(Debug, thiserror::Error)]
pub enum RootError {
    /// Aucun manifeste rbs entre le point de départ et la racine du système.
    #[error("aucun projet rbs ici : aucun `Cargo.toml` portant `[package.metadata.rbs]`")]
    Absent,

    /// Un manifeste a été trouvé, mais n'a pas pu être lu.
    #[error(transparent)]
    Illisible(#[from] Error),
}

/// Remonte de `start` jusqu'au projet rbs qui le contient.
///
/// Le manifeste seul ne suffit pas à trancher : la crate `migration` en porte un, et une
/// commande lancée depuis `migration/src` viserait sinon la mauvaise racine.
///
/// Une faute autre que l'absence arrête la remontée : un manifeste que le développeur
/// vient de casser désigne le projet qu'il visait, et le taire ferait viser un projet
/// englobant — celui du dépôt, voire celui d'un répertoire parent sans rapport.
pub fn project_root(start: &Path) -> Result<PathBuf, RootError> {
    for candidat in start.ancestors() {
        match read(&candidat.join("Cargo.toml")) {
            Ok(_) => return Ok(candidat.to_path_buf()),
            // Ces deux-là sont le régime ordinaire d'un ancêtre traversé : un répertoire
            // sans manifeste, ou celui d'une crate étrangère à rbs comme `migration`.
            Err(Error::PasUnProjet { .. }) => continue,
            Err(Error::Acces { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                continue;
            }
            Err(faute) => return Err(RootError::Illisible(faute)),
        }
    }

    Err(RootError::Absent)
}

/// Métadonnées rbs d'un projet, telles que portées par son `Cargo.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    /// Version de rbs qui a généré le projet.
    pub version: String,
    /// Features installées, dans l'ordre où elles ont été ajoutées.
    pub features: Vec<String>,
    /// Moteur de base sur lequel le projet a été créé.
    pub database: Database,
    /// Langue du guide `AGENTS.md` du projet.
    pub lang: crate::lang::Lang,
    /// Nom du paquet que le manifeste déclare.
    ///
    /// Optionnel parce que la faute ne se lève qu'à l'usage : `upgrade` n'a besoin du nom
    /// que pour recréer un guide absent, et un `[package] name` illisible n'a pas à faire
    /// échouer une mise à niveau qui s'en passe.
    pub package: Option<String>,
}

impl Metadata {
    /// Le nom du paquet, ou la faute qui le dit absent.
    ///
    /// C'est le nom du binaire du projet, et la racine de celui de sa base : les fragments
    /// de feature en ont besoin là où `rbs new` disposait encore du nom saisi.
    /// `cargo_toml` ne sert qu'à nommer le fichier fautif — rien n'est relu ici.
    pub fn package_name(&self, cargo_toml: &Path) -> Result<String, Error> {
        self.package.clone().ok_or_else(|| Error::Field {
            path: name_of(cargo_toml),
            key: "name",
        })
    }
}

/// Une dépendance telle qu'un patch de manifeste la réclame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    /// Nom du paquet, tel qu'il sera écrit en clé de `[dependencies]`.
    pub name: String,
    /// Version demandée, recopiée telle quelle dans le manifeste.
    pub version: String,
    /// Features à activer sur ce paquet.
    pub features: Vec<String>,
    /// Les défauts du paquet, laissés actifs sauf mention contraire.
    pub default_features: bool,
}

/// Ce qui peut empêcher de lire ou de mettre à jour les métadonnées d'un projet.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Le manifeste n'a pas pu être lu ou réécrit.
    #[error("{path} est inaccessible : {source}")]
    Acces {
        /// Chemin du manifeste.
        path: String,
        /// Cause système.
        source: std::io::Error,
    },

    /// Le manifeste n'est pas du TOML valide.
    #[error("{path} n'est pas un TOML valide : {source}")]
    Syntaxe {
        /// Chemin du manifeste.
        path: String,
        /// Cause de l'analyse.
        source: toml_edit::TomlError,
    },

    /// Le manifeste ne porte pas de section `[package.metadata.rbs]`.
    #[error(
        "{path} ne porte pas de section `[package.metadata.rbs]` : ce répertoire n'est pas un projet rbs"
    )]
    PasUnProjet {
        /// Chemin du manifeste.
        path: String,
    },

    /// Une clé attendue est absente ou porte le mauvais type.
    #[error("`package.metadata.rbs.{key}` est absente ou mal typée dans {path}")]
    Field {
        /// Chemin du manifeste.
        path: String,
        /// Clé fautive.
        key: &'static str,
    },

    /// La clé `database` porte une valeur qui n'est pas un moteur rbs.
    #[error(
        "`package.metadata.rbs.database` vaut `{database}` dans {path} : \
         moteurs connus — {known}"
    )]
    MoteurInconnu {
        /// Chemin du manifeste.
        path: String,
        /// Valeur refusée.
        database: String,
        /// Moteurs que rbs connaît.
        known: String,
    },

    /// Une déclaration du manifeste n'a pas la forme qu'un patch sait manipuler.
    #[error("`{key}` n'a pas la forme attendue dans {path}")]
    Declaration {
        /// Chemin du manifeste.
        path: String,
        /// Clé fautive, telle qu'elle apparaît dans le manifeste.
        key: String,
    },

    /// La dépendance est déjà déclarée, dans une version que le patch ne peut pas
    /// remplacer sans décider à la place du développeur.
    #[error("{path} déclare déjà `{dependency}` en version {present}, et non {demandee}")]
    VersionIncompatible {
        /// Chemin du manifeste.
        path: String,
        /// Dépendance en cause.
        dependency: String,
        /// Version que le manifeste porte.
        present: String,
        /// Version que le patch réclame.
        demandee: String,
    },

    /// Une feature est réclamée sur une dépendance que le manifeste ne déclare pas.
    #[error("{path} ne déclare pas la dépendance `{dependency}`")]
    DependanceAbsente {
        /// Chemin du manifeste.
        path: String,
        /// Dépendance introuvable.
        dependency: String,
    },
}

/// Lit les métadonnées rbs du manifeste désigné.
pub fn read(cargo_toml: &Path) -> Result<Metadata, Error> {
    let document = load(cargo_toml)?;

    let rbs = document
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("rbs"))
        .ok_or_else(|| Error::PasUnProjet {
            path: name_of(cargo_toml),
        })?;

    Ok(Metadata {
        version: version(rbs, cargo_toml)?,
        features: features(rbs, cargo_toml)?,
        database: database(rbs, cargo_toml)?,
        lang: lang(rbs),
        package: document
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(Item::as_str)
            .map(str::to_owned),
    })
}

/// Rend le manifeste avec `feature` inscrite, ou `None` si elle y est déjà.
///
/// `name` ne désigne le fichier que dans les messages d'erreur : rien n'est lu ni écrit ici.
pub fn record_feature(text: &str, feature: &str, name: &str) -> Result<Option<String>, Error> {
    let mut document = parse(text, name)?;

    // `get_mut` sur une clé absente la crée à `Item::None` pour permettre l'écriture :
    // vérifier l'existence de la section avant de la traverser en mutable évite qu'une
    // absence s'y déguise en `Item::None` au lieu de déclencher `PasUnProjet`.
    if document
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("rbs"))
        .is_none()
    {
        return Err(Error::PasUnProjet {
            path: name.to_string(),
        });
    }

    let rbs = document
        .get_mut("package")
        .and_then(|package| package.get_mut("metadata"))
        .and_then(|metadata| metadata.get_mut("rbs"))
        .expect("la section a été vérifiée juste au-dessus");

    let installees = rbs
        .get_mut("features")
        .and_then(Item::as_array_mut)
        .ok_or_else(|| Error::Field {
            path: name.to_string(),
            key: "features",
        })?;

    if installees
        .iter()
        .any(|value| value.as_str() == Some(feature))
    {
        return Ok(None);
    }

    installees.push(feature);

    Ok(Some(document.to_string()))
}

/// Rend le manifeste avec `dep` déclarée dans `[dependencies]`, ou `None` s'il la porte
/// déjà avec au moins ce qui est demandé.
///
/// Une version déjà déclarée qui diffère est un conflit, pas un silence : la remplacer
/// casserait un choix du développeur, la taire installerait une feature contre une version
/// qui ne la porte pas.
pub fn add_dependency(text: &str, dep: &Dependency, name: &str) -> Result<Option<String>, Error> {
    let mut document = parse(text, name)?;

    let dependencies = document
        .entry("dependencies")
        .or_insert_with(toml_edit::table)
        .as_table_like_mut()
        .ok_or_else(|| Error::Declaration {
            path: name.to_string(),
            key: "dependencies".to_string(),
        })?;

    let Some(declared) = dependencies.get_mut(&dep.name) else {
        dependencies.insert(&dep.name, declaration(dep));
        return Ok(Some(document.to_string()));
    };

    if let Some(present) = declared_version(declared)
        && present != dep.version
    {
        return Err(Error::VersionIncompatible {
            path: name.to_string(),
            dependency: dep.name.clone(),
            present: present.to_string(),
            demandee: dep.version.clone(),
        });
    }

    let malformed = || Error::Declaration {
        path: name.to_string(),
        key: dep.name.clone(),
    };

    let mut modifie = false;
    if !dep.default_features {
        modifie |= strip_defaults(declared).ok_or_else(malformed)?;
    }
    for feature in &dep.features {
        modifie |= enable_feature(declared, feature).ok_or_else(malformed)?;
    }

    Ok(modifie.then(|| document.to_string()))
}

/// Rend le manifeste avec `feature` activée sur la dépendance `dep`, ou `None` si elle
/// l'est déjà.
///
/// Une dépendance absente est une erreur de l'appelant : activer une feature suppose de
/// savoir dans quelle version, ce que seul l'appelant sait.
pub fn add_feature_to_dependency(
    text: &str,
    dep: &str,
    feature: &str,
    name: &str,
) -> Result<Option<String>, Error> {
    let mut document = parse(text, name)?;

    let absente = || Error::DependanceAbsente {
        path: name.to_string(),
        dependency: dep.to_string(),
    };

    let modifie = {
        let dependencies = document
            .get_mut("dependencies")
            .ok_or_else(absente)?
            .as_table_like_mut()
            .ok_or_else(|| Error::Declaration {
                path: name.to_string(),
                key: "dependencies".to_string(),
            })?;

        let declared = dependencies.get_mut(dep).ok_or_else(absente)?;

        enable_feature(declared, feature).ok_or_else(|| Error::Declaration {
            path: name.to_string(),
            key: dep.to_string(),
        })?
    };

    Ok(modifie.then(|| document.to_string()))
}

/// Rend le manifeste aligné sur `version` — celle de la dépendance `dep` et celle que
/// `[package.metadata.rbs]` garde de la génération — ou `None` s'il l'est déjà.
///
/// Un noyau déclaré par un chemin n'a pas de version à changer : le mode de développement
/// de rbs lui-même reste en place, et seule la métadonnée suit.
pub fn align_version(
    text: &str,
    dep: &str,
    version: &str,
    name: &str,
) -> Result<Option<String>, Error> {
    let mut document = parse(text, name)?;

    if document
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("rbs"))
        .is_none()
    {
        return Err(Error::PasUnProjet {
            path: name.to_string(),
        });
    }

    let rbs = document
        .get_mut("package")
        .and_then(|package| package.get_mut("metadata"))
        .and_then(|metadata| metadata.get_mut("rbs"))
        .expect("la section a été vérifiée juste au-dessus")
        .as_table_like_mut()
        .ok_or_else(|| Error::Declaration {
            path: name.to_string(),
            key: "package.metadata.rbs".to_string(),
        })?;

    if rbs.get("version").and_then(Item::as_str).is_none() {
        return Err(Error::Field {
            path: name.to_string(),
            key: "version",
        });
    }

    let mut modifie = replace_version(rbs, version);

    let absente = || Error::DependanceAbsente {
        path: name.to_string(),
        dependency: dep.to_string(),
    };

    let declared = document
        .get_mut("dependencies")
        .ok_or_else(absente)?
        .as_table_like_mut()
        .ok_or_else(|| Error::Declaration {
            path: name.to_string(),
            key: "dependencies".to_string(),
        })?
        .get_mut(dep)
        .ok_or_else(absente)?;

    modifie |= set_version(declared, version).ok_or_else(|| Error::Declaration {
        path: name.to_string(),
        key: dep.to_string(),
    })?;

    Ok(modifie.then(|| document.to_string()))
}

/// Donne à `version` la version d'une déclaration de dépendance, en rendant `false` si
/// elle la portait déjà ou n'en portait aucune — cas d'un `path` ou d'un `git` — et `None`
/// si la déclaration n'a pas une forme manipulable.
fn set_version(declared: &mut Item, version: &str) -> Option<bool> {
    if let Some(present) = declared.as_str() {
        if present == version {
            return Some(false);
        }

        let decor = declared.as_value()?.decor().clone();
        let mut value = Value::from(version);
        *value.decor_mut() = decor;
        *declared = Item::Value(value);

        return Some(true);
    }

    let table = declared.as_table_like_mut()?;

    if table.get("version").and_then(Item::as_str).is_none() {
        return Some(false);
    }

    Some(replace_version(table, version))
}

/// Écrit `version` dans la clé `version` d'une table, en rendant `false` si elle y était
/// déjà.
///
/// Le décor de l'ancienne valeur est repris : sans lui, une table inline perdrait les
/// espaces qui l'entourent, et le manifeste changerait plus que son numéro.
fn replace_version(table: &mut dyn toml_edit::TableLike, version: &str) -> bool {
    if table.get("version").and_then(Item::as_str) == Some(version) {
        return false;
    }

    let decor = table
        .get("version")
        .and_then(Item::as_value)
        .map(|present| present.decor().clone());

    let mut value = Value::from(version);
    if let Some(decor) = decor {
        *value.decor_mut() = decor;
    }
    table.insert("version", Item::Value(value));

    true
}

/// La déclaration à écrire pour une dépendance encore absente.
///
/// Une version nue tant qu'il n'y a rien d'autre à dire : c'est la forme qu'un développeur
/// aurait écrite, et le manifeste n'a pas à s'alourdir d'une table pour une seule clé.
fn declaration(dep: &Dependency) -> Item {
    if dep.features.is_empty() && dep.default_features {
        return Item::Value(Value::from(dep.version.as_str()));
    }

    let mut table = InlineTable::new();
    table.insert("version", Value::from(dep.version.as_str()));
    if !dep.default_features {
        table.insert("default-features", Value::from(false));
    }
    if !dep.features.is_empty() {
        table.insert(
            "features",
            Value::Array(dep.features.iter().map(String::as_str).collect::<Array>()),
        );
    }

    Item::Value(Value::InlineTable(table))
}

/// La version que la déclaration porte, ou `None` si elle n'en porte pas — cas d'une
/// dépendance en `path` ou en `git`, que le patch laisse alors telle quelle.
fn declared_version(declared: &Item) -> Option<&str> {
    declared
        .as_str()
        .or_else(|| declared.get("version").and_then(Item::as_str))
}

/// Coupe les défauts d'une déclaration, en rendant `false` s'ils l'étaient déjà et `None`
/// si la déclaration n'a pas une forme manipulable.
fn strip_defaults(declared: &mut Item) -> Option<bool> {
    spread(declared)?;

    let table = declared.as_table_like_mut()?;
    if table.get("default-features").and_then(Item::as_bool) == Some(false) {
        return Some(false);
    }

    table.insert("default-features", Item::Value(Value::from(false)));

    Some(true)
}

/// Active `feature` sur une déclaration, en rendant `false` si elle y était déjà et `None`
/// si la déclaration n'a pas une forme manipulable.
fn enable_feature(declared: &mut Item, feature: &str) -> Option<bool> {
    spread(declared)?;

    let table = declared.as_table_like_mut()?;
    let features = table
        .entry("features")
        .or_insert(Item::Value(Value::Array(Array::new())))
        .as_array_mut()?;

    if features.iter().any(|value| value.as_str() == Some(feature)) {
        return Some(false);
    }

    features.push(feature);

    Some(true)
}

/// Donne à une déclaration la forme d'une table inline, seule à pouvoir porter plus que
/// la version.
///
/// Le décor d'une déclaration en chaîne, qui porte l'espacement et le commentaire de fin
/// de ligne, est reporté sur la table.
fn spread(declared: &mut Item) -> Option<()> {
    let Some(version) = declared.as_str().map(str::to_owned) else {
        return Some(());
    };

    let decor = declared.as_value()?.decor().clone();

    let mut table = InlineTable::new();
    table.insert("version", Value::from(version));

    let mut value = Value::InlineTable(table);
    *value.decor_mut() = decor;
    *declared = Item::Value(value);

    Some(())
}

/// Analyse un manifeste en préservant sa mise en forme et ses commentaires.
fn parse(text: &str, name: &str) -> Result<DocumentMut, Error> {
    text.parse::<DocumentMut>()
        .map_err(|source| Error::Syntaxe {
            path: name.to_string(),
            source,
        })
}

/// Analyse le manifeste en préservant sa mise en forme et ses commentaires.
fn load(cargo_toml: &Path) -> Result<DocumentMut, Error> {
    let source = fs::read_to_string(cargo_toml).map_err(|source| Error::Acces {
        path: name_of(cargo_toml),
        source,
    })?;

    parse(&source, &name_of(cargo_toml))
}

fn version(rbs: &Item, cargo_toml: &Path) -> Result<String, Error> {
    rbs.get("version")
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::Field {
            path: name_of(cargo_toml),
            key: "version",
        })
}

/// Moteur déclaré par le manifeste.
///
/// Son absence n'est pas une erreur : les projets créés avant que le moteur soit un
/// choix n'ont pas la clé, et sont des projets PostgreSQL.
fn database(rbs: &Item, cargo_toml: &Path) -> Result<Database, Error> {
    let Some(declared) = rbs.get("database") else {
        return Ok(Database::default());
    };

    let name = declared.as_str().ok_or_else(|| Error::Field {
        path: name_of(cargo_toml),
        key: "database",
    })?;

    Database::from_name(name).ok_or_else(|| Error::MoteurInconnu {
        path: name_of(cargo_toml),
        database: name.to_owned(),
        known: Database::TOUS.map(Database::name).join(", "),
    })
}

/// Langue du guide déclarée par le manifeste, le français à défaut.
///
/// Ni l'absence de la clé — les projets antérieurs à ce jalon n'en portent pas — ni une
/// valeur inconnue ne sont des erreurs : elles immobiliseraient toutes les commandes d'un
/// projet par ailleurs sain.
fn lang(rbs: &Item) -> crate::lang::Lang {
    rbs.get("lang")
        .and_then(Item::as_str)
        .and_then(crate::lang::Lang::parse)
        .unwrap_or_default()
}

fn features(rbs: &Item, cargo_toml: &Path) -> Result<Vec<String>, Error> {
    let manquant = || Error::Field {
        path: name_of(cargo_toml),
        key: "features",
    };

    rbs.get("features")
        .and_then(Item::as_array)
        .ok_or_else(manquant)?
        .iter()
        // Une entrée qui n'est pas une chaîne est une erreur, pas une entrée à ignorer :
        // silencieusement écartée, elle serait réinstallée au prochain `rbs add`.
        .map(|value| value.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(manquant)
}

/// Le chemin tel qu'il apparaîtra dans un message d'erreur.
fn name_of(cargo_toml: &Path) -> String {
    cargo_toml.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use minijinja::context;
    use tempfile::TempDir;

    use super::*;
    use crate::template::Renderer;

    /// Manifeste du squelette, résolu depuis la crate plutôt que depuis le répertoire
    /// courant, que `cargo test` ne garantit pas.
    const MANIFESTE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/templates/project/Cargo.toml.jinja"
    );

    /// Déroule le manifeste du squelette dans un répertoire temporaire. C'est le plus
    /// proche d'un projet fraîchement généré tant que `rbs new` n'existe pas.
    fn generated_project() -> (TempDir, PathBuf) {
        let source = fs::read_to_string(MANIFESTE).expect("le manifeste du squelette est lisible");

        let rendered = Renderer::new()
            .render(
                &source,
                context! {
                    project_name => "mon-api",
                    crate_name => "mon_api",
                    rbs_core_dep => "\"0.1\"",
                    rbs_version => "0.1.0",
                    database => Database::default().name(),
                    sea_orm_feature => Database::default().sea_orm_feature(),
                    lang => crate::lang::Lang::default().name(),
                },
            )
            .expect("le manifeste doit se rendre");

        write(rendered)
    }

    fn write(content: impl AsRef<[u8]>) -> (TempDir, PathBuf) {
        let directory = TempDir::new().expect("répertoire temporaire créable");
        let path = directory.path().join("Cargo.toml");
        fs::write(&path, content).expect("manifeste écrit");

        (directory, path)
    }

    /// Un manifeste minimal, dont on veut voir survivre la mise en forme.
    const ALIGNABLE: &str = "[package]\nname = \"demo\"\n\n\
        [package.metadata.rbs]\n\
        version = \"0.1.0\"\n\
        features = []\n\n\
        [dependencies]\n\
        # le noyau, et rien d'autre\n\
        rbs-core = { version = \"0.1.0\", default-features = false }\n\
        anyhow = \"1.0\"\n";

    #[test]
    fn aligning_moves_both_numbers_and_leaves_the_rest_alone() {
        let rendu = align_version(ALIGNABLE, "rbs-core", "1.0.0", "Cargo.toml")
            .expect("l'alignement doit aboutir")
            .expect("un manifeste en retard change");

        assert_eq!(rendu, ALIGNABLE.replace("0.1.0", "1.0.0"));
    }

    #[test]
    fn aligning_an_already_aligned_manifest_changes_nothing() {
        let aligne = ALIGNABLE.replace("0.1.0", "1.0.0");

        assert_eq!(
            align_version(&aligne, "rbs-core", "1.0.0", "Cargo.toml")
                .expect("l'alignement doit aboutir"),
            None
        );
    }

    /// Le noyau pris d'un chemin n'a pas de version à changer, et le mode de
    /// développement de rbs ne doit pas se faire écraser par une mise à niveau.
    #[test]
    fn a_core_declared_by_a_path_keeps_it_while_the_metadata_follows() {
        let source = ALIGNABLE.replace(
            "rbs-core = { version = \"0.1.0\", default-features = false }",
            "rbs-core = { path = \"../rbs-core\", default-features = false }",
        );

        let rendu = align_version(&source, "rbs-core", "1.0.0", "Cargo.toml")
            .expect("l'alignement doit aboutir")
            .expect("la métadonnée, elle, change");

        assert!(rendu.contains("path = \"../rbs-core\""), "{rendu}");
        assert!(rendu.contains("version = \"1.0.0\""), "{rendu}");
    }

    #[test]
    fn aligning_a_manifest_without_the_core_names_the_missing_dependency() {
        let source = ALIGNABLE.replace(
            "rbs-core = { version = \"0.1.0\", default-features = false }\n",
            "",
        );

        let error = align_version(&source, "rbs-core", "1.0.0", "Cargo.toml")
            .expect_err("une dépendance absente est une erreur");

        assert!(error.to_string().contains("rbs-core"), "{error}");
    }

    #[test]
    fn the_metadata_of_a_generated_project_reads_back() {
        let (_repertoire, path) = generated_project();

        let metadonnees = read(&path).expect("le manifeste généré porte ses métadonnées");

        assert_eq!(metadonnees.version, "0.1.0");
        assert_eq!(metadonnees.features, vec!["health".to_string()]);
        assert_eq!(metadonnees.database, Database::Postgres);
        assert_eq!(metadonnees.lang, crate::lang::Lang::Fr);
    }

    // Le critère de S1 : aucun projet existant ne change. Les manifestes créés avant que
    // le moteur soit un choix n'ont pas la clé, et doivent rester des projets PostgreSQL.
    #[test]
    fn a_manifest_without_a_database_key_reads_back_as_postgres() {
        let (_repertoire, path) = write(
            "[package]\nname = \"demo\"\n\n\
             [package.metadata.rbs]\nversion = \"0.1.0\"\nfeatures = []\n",
        );

        let metadonnees = read(&path).expect("un manifeste sans moteur reste lisible");

        assert_eq!(metadonnees.database, Database::Postgres);
    }

    #[test]
    fn a_declared_database_reads_back_as_itself() {
        for engine in Database::TOUS {
            let (_repertoire, path) = write(format!(
                "[package]\nname = \"demo\"\n\n\
                 [package.metadata.rbs]\nversion = \"0.1.0\"\nfeatures = []\n\
                 database = \"{}\"\n",
                engine.name()
            ));

            let metadonnees = read(&path).expect("le manifeste porte un moteur connu");

            assert_eq!(metadonnees.database, engine);
        }
    }

    // Une valeur inconnue est une faute de frappe dans un fichier que l'utilisateur
    // édite : la corriger en silence vers `postgres` produirait un projet dont le
    // manifeste et le comportement divergent.
    #[test]
    fn an_unknown_database_is_rejected_naming_the_key() {
        let (_repertoire, path) = write(
            "[package]\nname = \"demo\"\n\n\
             [package.metadata.rbs]\nversion = \"0.1.0\"\nfeatures = []\n\
             database = \"oracle\"\n",
        );

        let error = read(&path).expect_err("`oracle` n'est pas un moteur rbs");

        let message = error.to_string();
        assert!(
            message.contains("database"),
            "le message ne nomme pas la clé fautive : {message}"
        );
    }

    #[test]
    fn a_manifest_without_an_rbs_section_is_rejected_naming_the_file() {
        let (_repertoire, path) = write("[package]\nname = \"mon-api\"\n");

        let error = read(&path).expect_err("un manifeste sans section rbs n'est pas un projet");

        let message = error.to_string();
        assert!(
            message.contains(&path.display().to_string()),
            "le message ne nomme pas le fichier : {message}"
        );
        assert!(
            message.contains("projet rbs"),
            "le message ne dit pas ce qui manque : {message}"
        );
    }

    const MANIFESTE_MINIMAL: &str = r#"[package]
name = "demo"

# les features installées
[package.metadata.rbs]
version = "0.1.0"
features = ["health"]
"#;

    #[test]
    fn a_missing_feature_is_recorded_without_touching_the_rest_of_the_manifest() {
        let rendered = record_feature(MANIFESTE_MINIMAL, "docker", "Cargo.toml")
            .expect("le manifeste est valide")
            .expect("la feature est absente, le texte change");

        assert!(rendered.contains(r#"features = ["health", "docker"]"#));
        assert!(rendered.contains("# les features installées"));
        assert!(rendered.starts_with("[package]\nname = \"demo\"\n"));
    }

    #[test]
    fn an_already_recorded_feature_produces_no_text() {
        let rendered = record_feature(MANIFESTE_MINIMAL, "health", "Cargo.toml")
            .expect("le manifeste est valide");

        assert_eq!(rendered, None);
    }

    const MANIFESTE_DEPS: &str = r#"[package]
name = "demo"
version = "0.1.0"

# les dépendances du projet
[dependencies]
axum = "0.9"       # le serveur
tokio = { version = "1", features = ["macros"] }
"#;

    #[test]
    fn a_missing_dependency_is_added_without_moving_the_rest() {
        let rendered = add_dependency(
            MANIFESTE_DEPS,
            &Dependency {
                name: "redis".into(),
                version: "0.32".into(),
                features: vec![],
                default_features: true,
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la dépendance est absente");

        assert!(rendered.contains(r#"redis = "0.32""#), "{rendered}");
        assert!(
            rendered.contains("# les dépendances du projet"),
            "{rendered}"
        );
        assert!(
            rendered.contains(r#"axum = "0.9"       # le serveur"#),
            "{rendered}"
        );
    }

    #[test]
    fn an_already_declared_dependency_produces_no_text() {
        let rendered = add_dependency(
            MANIFESTE_DEPS,
            &Dependency {
                name: "axum".into(),
                version: "0.9".into(),
                features: vec![],
                default_features: true,
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide");

        assert_eq!(rendered, None);
    }

    #[test]
    fn a_dependency_declared_at_another_version_is_a_conflict() {
        let error = add_dependency(
            MANIFESTE_DEPS,
            &Dependency {
                name: "axum".into(),
                version: "0.8".into(),
                features: vec![],
                default_features: true,
            },
            "Cargo.toml",
        )
        .expect_err("les deux versions ne se réconcilient pas");

        assert!(
            matches!(error, Error::VersionIncompatible { .. }),
            "{error}"
        );
        let message = error.to_string();
        assert!(message.contains("axum"), "{message}");
        assert!(
            message.contains("0.8") && message.contains("0.9"),
            "{message}"
        );
    }

    #[test]
    fn a_dependency_declared_without_all_its_features_receives_them() {
        let rendered = add_dependency(
            MANIFESTE_DEPS,
            &Dependency {
                name: "tokio".into(),
                version: "1".into(),
                features: vec!["rt-multi-thread".into()],
                default_features: true,
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la feature manque");

        assert!(
            rendered
                .contains(r#"tokio = { version = "1", features = ["macros", "rt-multi-thread"] }"#),
            "{rendered}"
        );
    }

    /// Une crate que le projet porte déjà avec ses défauts, et qu'un fragment réclame sans
    /// eux : c'est le seul chemin par lequel `default-features` arrive sur une déclaration
    /// existante, et le commentaire du développeur doit y survivre comme ailleurs.
    #[test]
    fn an_already_declared_dependency_receives_its_default_features_cut() {
        let rendered = add_dependency(
            MANIFESTE_DEPS,
            &Dependency {
                name: "axum".into(),
                version: "0.9".into(),
                features: vec![],
                default_features: false,
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("les défauts sont encore actifs");

        let line = line_of(&rendered, "axum");
        assert!(
            line.contains(r#"axum = { version = "0.9", default-features = false }"#),
            "{line}"
        );
        assert!(line.contains("# le serveur"), "{line}");
    }

    #[test]
    fn an_already_written_default_features_cut_produces_no_text() {
        let cut = add_dependency(
            MANIFESTE_DEPS,
            &Dependency {
                name: "axum".into(),
                version: "0.9".into(),
                features: vec![],
                default_features: false,
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("les défauts sont encore actifs");

        let rendered = add_dependency(
            &cut,
            &Dependency {
                name: "axum".into(),
                version: "0.9".into(),
                features: vec![],
                default_features: false,
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide");

        assert_eq!(rendered, None);
    }

    #[test]
    fn a_dependency_with_features_is_declared_as_an_inline_table() {
        let rendered = add_dependency(
            MANIFESTE_DEPS,
            &Dependency {
                name: "redis".into(),
                version: "0.32".into(),
                features: vec!["tokio-comp".into()],
                default_features: true,
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la dépendance est absente");

        assert!(
            rendered.contains(r#"redis = { version = "0.32", features = ["tokio-comp"] }"#),
            "{rendered}"
        );
    }

    #[test]
    fn a_manifest_without_a_dependencies_table_receives_one() {
        let rendered = add_dependency(
            MANIFESTE_MINIMAL,
            &Dependency {
                name: "redis".into(),
                version: "0.32".into(),
                features: vec![],
                default_features: true,
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la dépendance est absente");

        assert!(rendered.contains("[dependencies]"), "{rendered}");
        assert!(rendered.contains(r#"redis = "0.32""#), "{rendered}");
        assert!(rendered.contains("# les features installées"), "{rendered}");
        assert_eq!(
            read_dependency(&rendered, "redis"),
            Some("0.32".to_string()),
            "la dépendance n'a pas atterri dans `[dependencies]` :\n{rendered}"
        );
    }

    /// La version que le manifeste rendu déclare pour `dependency`, table inline ou chaîne.
    fn read_dependency(text: &str, dependency: &str) -> Option<String> {
        let document = text.parse::<DocumentMut>().expect("TOML valide");
        let item = document.get("dependencies")?.get(dependency)?;

        item.as_str()
            .or_else(|| item.get("version").and_then(|v| v.as_str()))
            .map(str::to_owned)
    }

    /// La ligne du manifeste rendu qui déclare `dependency`.
    fn line_of(text: &str, dependency: &str) -> String {
        text.lines()
            .find(|line| line.starts_with(dependency))
            .unwrap_or_else(|| panic!("`{dependency}` n'est plus déclarée :\n{text}"))
            .to_string()
    }

    #[test]
    fn a_feature_is_added_to_a_dependency_already_in_an_inline_table() {
        let rendered =
            add_feature_to_dependency(MANIFESTE_DEPS, "tokio", "rt-multi-thread", "Cargo.toml")
                .expect("le manifeste est valide")
                .expect("la feature manque");

        assert_eq!(
            line_of(&rendered, "tokio"),
            r#"tokio = { version = "1", features = ["macros", "rt-multi-thread"] }"#
        );
    }

    #[test]
    fn a_string_dependency_becomes_an_inline_table_keeping_its_comment() {
        let rendered = add_feature_to_dependency(MANIFESTE_DEPS, "axum", "macros", "Cargo.toml")
            .expect("le manifeste est valide")
            .expect("la feature manque");

        let line = line_of(&rendered, "axum");
        assert!(
            line.contains(r#"axum = { version = "0.9", features = ["macros"] }"#),
            "la déclaration n'est pas devenue une table inline : {line}"
        );
        assert!(
            line.contains("# le serveur"),
            "le commentaire de fin de ligne a été perdu : {line}"
        );
    }

    #[test]
    fn an_already_enabled_feature_produces_no_text() {
        let rendered = add_feature_to_dependency(MANIFESTE_DEPS, "tokio", "macros", "Cargo.toml")
            .expect("le manifeste est valide");

        assert_eq!(rendered, None);
    }

    #[test]
    fn a_missing_dependency_is_rejected() {
        let error = add_feature_to_dependency(MANIFESTE_DEPS, "redis", "tokio-comp", "Cargo.toml")
            .expect_err("la dépendance manque");

        assert!(matches!(error, Error::DependanceAbsente { .. }), "{error}");
        assert!(error.to_string().contains("redis"), "{error}");
    }

    #[test]
    fn a_feature_on_a_manifest_without_dependencies_is_rejected() {
        let error = add_feature_to_dependency(MANIFESTE_MINIMAL, "axum", "macros", "Cargo.toml")
            .expect_err("il n'y a pas de table `[dependencies]`");

        assert!(matches!(error, Error::DependanceAbsente { .. }), "{error}");
    }

    /// Manifeste témoin : commentaires de tête et de fin de ligne, lignes vides,
    /// alignements irréguliers, tables voisines que le patch doit ignorer.
    const TEMOIN: &str = r#"# Manifest écrit à la main.
[package]
name    = "demo"           # aligné exprès
version = "0.1.0"
edition = "2024"

# les dépendances du projet
[dependencies]
axum       = "0.9"     # le serveur
sea-orm    = { version = "1.1", features = ["runtime-tokio-rustls"] }
tokio      = { version = "1", features = ["macros"] }

[dev-dependencies]
tempfile = "3"   # rien à voir avec le patch

# l'état de rbs dans ce project
[package.metadata.rbs]
version = "0.1.0"
features = ["health"]
"#;

    /// Les lignes que le rendu a perdues, puis celles qu'il a gagnées.
    ///
    /// Diff naïf par appariement : il suffit ici, où l'on veut seulement établir que rien
    /// d'autre que la zone patchée n'a bougé.
    fn modified_lines(before: &str, after: &str) -> (Vec<String>, Vec<String>) {
        let mut restantes: Vec<&str> = after.lines().collect();
        let mut perdues = Vec::new();

        for line in before.lines() {
            match restantes.iter().position(|candidate| *candidate == line) {
                Some(index) => {
                    restantes.remove(index);
                }
                None => perdues.push(line.to_string()),
            }
        }

        (perdues, restantes.into_iter().map(str::to_owned).collect())
    }

    /// Les lignes du texte, privées de celles qu'un patch était censé toucher.
    fn out_of_range(text: &str, touchees: &[String]) -> Vec<String> {
        text.lines()
            .filter(|line| !touchees.iter().any(|touchee| touchee == line))
            .map(str::to_owned)
            .collect()
    }

    /// Établit qu'entre `TEMOIN` et `rendered`, seules les lignes annoncées ont changé — et
    /// que le reste est resté dans le même ordre.
    fn only_these_lines_changed(rendered: &str, perdues: &[&str], gagnees: &[&str]) {
        let expected = (
            perdues.iter().map(|l| l.to_string()).collect::<Vec<_>>(),
            gagnees.iter().map(|l| l.to_string()).collect::<Vec<_>>(),
        );

        assert_eq!(
            modified_lines(TEMOIN, rendered),
            expected,
            "le patch a débordé de sa ligne :\n{rendered}"
        );
        assert_eq!(
            out_of_range(TEMOIN, &expected.0),
            out_of_range(rendered, &expected.1),
            "le patch a réordonné le manifeste :\n{rendered}"
        );
    }

    #[test]
    fn adding_a_dependency_only_touches_its_own_line() {
        let rendered = add_dependency(
            TEMOIN,
            &Dependency {
                name: "redis".into(),
                version: "0.32".into(),
                features: vec!["tokio-comp".into()],
                default_features: true,
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la dépendance est absente");

        only_these_lines_changed(
            &rendered,
            &[],
            &[r#"redis = { version = "0.32", features = ["tokio-comp"] }"#],
        );
    }

    #[test]
    fn adding_a_feature_to_a_dependency_only_touches_its_own_line() {
        let rendered = add_feature_to_dependency(TEMOIN, "sea-orm", "with-uuid", "Cargo.toml")
            .expect("le manifeste est valide")
            .expect("la feature manque");

        only_these_lines_changed(
            &rendered,
            &[r#"sea-orm    = { version = "1.1", features = ["runtime-tokio-rustls"] }"#],
            &[
                r#"sea-orm    = { version = "1.1", features = ["runtime-tokio-rustls", "with-uuid"] }"#,
            ],
        );
    }

    #[test]
    fn recording_an_rbs_feature_only_touches_its_own_line() {
        let rendered = record_feature(TEMOIN, "docker", "Cargo.toml")
            .expect("le manifeste est valide")
            .expect("la feature est absente");

        only_these_lines_changed(
            &rendered,
            &[r#"features = ["health"]"#],
            &[r#"features = ["health", "docker"]"#],
        );
    }

    #[test]
    fn a_manifest_without_an_rbs_section_is_rejected() {
        let error = record_feature("[package]\nname = \"demo\"\n", "docker", "Cargo.toml")
            .expect_err("la section manque");

        assert!(matches!(error, Error::PasUnProjet { .. }));
    }

    /// La clé porte la langue du guide : sans elle, `add` et `upgrade` réécriraient un
    /// guide dans la langue de celui qui les lance, non dans celle du projet.
    #[test]
    fn the_language_is_read_from_the_manifest() {
        let directory = tempfile::tempdir().expect("répertoire temporaire");
        let manifest = directory.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            "[package]\nname = \"blog\"\n\n[package.metadata.rbs]\nversion = \"1.1.0\"\n\
             features = [\"health\"]\ndatabase = \"postgres\"\nlang = \"en\"\n",
        )
        .expect("manifeste écrit");

        let metadonnees = read(&manifest).expect("le manifeste est lisible");

        assert_eq!(metadonnees.lang, crate::lang::Lang::En);
    }

    /// Tout projet engendré avant ce jalon est dépourvu de la clé. Le refuser rendrait
    /// `doctor` et `upgrade` inutilisables sur le parc existant.
    #[test]
    fn a_manifest_without_the_key_is_read_as_french() {
        let directory = tempfile::tempdir().expect("répertoire temporaire");
        let manifest = directory.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            "[package]\nname = \"blog\"\n\n[package.metadata.rbs]\nversion = \"1.0.0\"\n\
             features = [\"health\"]\ndatabase = \"postgres\"\n",
        )
        .expect("manifeste écrit");

        let metadonnees = read(&manifest).expect("le manifeste est lisible");

        assert_eq!(metadonnees.lang, crate::lang::Lang::Fr);
    }

    /// Une valeur inconnue vient forcément d'une édition à la main : la traiter comme le
    /// défaut vaut mieux que d'immobiliser toutes les commandes du projet.
    #[test]
    fn an_unknown_language_is_read_as_french() {
        let directory = tempfile::tempdir().expect("répertoire temporaire");
        let manifest = directory.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            "[package]\nname = \"blog\"\n\n[package.metadata.rbs]\nversion = \"1.1.0\"\n\
             features = [\"health\"]\ndatabase = \"postgres\"\nlang = \"kl\"\n",
        )
        .expect("manifeste écrit");

        assert_eq!(
            read(&manifest).expect("le manifeste est lisible").lang,
            crate::lang::Lang::Fr
        );
    }

    /// Une virgule oubliée dans le manifeste du projet doit se dire, et non faire viser
    /// un projet situé plus haut dans l'arborescence.
    #[test]
    fn an_invalid_manifest_names_the_fault_instead_of_climbing_past_it() {
        let dir = TempDir::new().expect("répertoire temporaire");
        let projet = dir.path().join("api");
        fs::create_dir_all(projet.join("src")).expect("arborescence");
        fs::write(
            projet.join("Cargo.toml"),
            "[package]\nname = \"api\"\n[package.metadata.rbs]\nversion = \"1.1.0\",\n",
        )
        .expect("manifeste cassé");

        let error = project_root(&projet.join("src")).expect_err("la faute doit se dire");

        assert!(matches!(error, RootError::Illisible(Error::Syntaxe { .. })));
    }

    /// Le manifeste de la crate `migration` ne porte pas de section rbs : la remontée le
    /// traverse, comme avant.
    #[test]
    fn the_migration_crate_manifest_does_not_stop_the_climb() {
        let dir = TempDir::new().expect("répertoire temporaire");
        let projet = dir.path().join("api");
        fs::create_dir_all(projet.join("migration/src")).expect("arborescence");
        fs::write(
            projet.join("Cargo.toml"),
            "[package]\nname = \"api\"\n\n[package.metadata.rbs]\nversion = \"1.1.0\"\nfeatures = []\ndatabase = \"postgres\"\n",
        )
        .expect("manifeste du projet");
        fs::write(
            projet.join("migration/Cargo.toml"),
            "[package]\nname = \"migration\"\n",
        )
        .expect("manifeste de migration");

        let root = project_root(&projet.join("migration/src")).expect("la racine est trouvée");

        assert_eq!(root, projet);
    }
}
