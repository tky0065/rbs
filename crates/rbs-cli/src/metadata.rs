//! Lecture et mise à jour de `[package.metadata.rbs]`.
//!
//! Le `Cargo.toml` du projet est le seul endroit où rbs garde son état : version qui a
//! généré le projet, features installées. Un fichier de plus, non versionné par
//! réflexe, se serait désynchronisé du dépôt.

use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{Array, DocumentMut, InlineTable, Item, Value};

/// Remonte de `depart` jusqu'au projet rbs qui le contient.
///
/// Le manifeste seul ne suffit pas à trancher : la crate `migration` en porte un, et une
/// commande lancée depuis `migration/src` viserait sinon la mauvaise racine.
pub fn racine_du_projet(depart: &Path) -> Option<PathBuf> {
    depart
        .ancestors()
        .find(|candidat| lire(&candidat.join("Cargo.toml")).is_ok())
        .map(Path::to_path_buf)
}

/// Métadonnées rbs d'un projet, telles que portées par son `Cargo.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadonnees {
    /// Version de rbs qui a généré le projet.
    pub version: String,
    /// Features installées, dans l'ordre où elles ont été ajoutées.
    pub features: Vec<String>,
}

/// Une dépendance telle qu'un patch de manifeste la réclame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependance {
    /// Nom du paquet, tel qu'il sera écrit en clé de `[dependencies]`.
    pub nom: String,
    /// Version demandée, recopiée telle quelle dans le manifeste.
    pub version: String,
    /// Features à activer sur ce paquet.
    pub features: Vec<String>,
}

/// Ce qui peut empêcher de lire ou de mettre à jour les métadonnées d'un projet.
#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    /// Le manifeste n'a pas pu être lu ou réécrit.
    #[error("{chemin} est inaccessible : {source}")]
    Acces {
        /// Chemin du manifeste.
        chemin: String,
        /// Cause système.
        source: std::io::Error,
    },

    /// Le manifeste n'est pas du TOML valide.
    #[error("{chemin} n'est pas un TOML valide : {source}")]
    Syntaxe {
        /// Chemin du manifeste.
        chemin: String,
        /// Cause de l'analyse.
        source: toml_edit::TomlError,
    },

    /// Le manifeste ne porte pas de section `[package.metadata.rbs]`.
    #[error(
        "{chemin} ne porte pas de section `[package.metadata.rbs]` : ce répertoire n'est pas un projet rbs"
    )]
    PasUnProjet {
        /// Chemin du manifeste.
        chemin: String,
    },

    /// Une clé attendue est absente ou porte le mauvais type.
    #[error("`package.metadata.rbs.{cle}` est absente ou mal typée dans {chemin}")]
    Champ {
        /// Chemin du manifeste.
        chemin: String,
        /// Clé fautive.
        cle: &'static str,
    },

    /// Une déclaration du manifeste n'a pas la forme qu'un patch sait manipuler.
    #[error("`{cle}` n'a pas la forme attendue dans {chemin}")]
    Declaration {
        /// Chemin du manifeste.
        chemin: String,
        /// Clé fautive, telle qu'elle apparaît dans le manifeste.
        cle: String,
    },

    /// La dépendance est déjà déclarée, dans une version que le patch ne peut pas
    /// remplacer sans décider à la place du développeur.
    #[error("{chemin} déclare déjà `{dependance}` en version {presente}, et non {demandee}")]
    VersionIncompatible {
        /// Chemin du manifeste.
        chemin: String,
        /// Dépendance en cause.
        dependance: String,
        /// Version que le manifeste porte.
        presente: String,
        /// Version que le patch réclame.
        demandee: String,
    },

    /// Une feature est réclamée sur une dépendance que le manifeste ne déclare pas.
    #[error("{chemin} ne déclare pas la dépendance `{dependance}`")]
    DependanceAbsente {
        /// Chemin du manifeste.
        chemin: String,
        /// Dépendance introuvable.
        dependance: String,
    },
}

/// Lit les métadonnées rbs du manifeste désigné.
pub fn lire(cargo_toml: &Path) -> Result<Metadonnees, Erreur> {
    let document = charger(cargo_toml)?;

    let rbs = document
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("rbs"))
        .ok_or_else(|| Erreur::PasUnProjet {
            chemin: nommer(cargo_toml),
        })?;

    Ok(Metadonnees {
        version: version(rbs, cargo_toml)?,
        features: features(rbs, cargo_toml)?,
    })
}

/// Rend le manifeste avec `feature` inscrite, ou `None` si elle y est déjà.
///
/// `nom` ne désigne le fichier que dans les messages d'erreur : rien n'est lu ni écrit ici.
pub fn inscrire_feature(texte: &str, feature: &str, nom: &str) -> Result<Option<String>, Erreur> {
    let mut document = analyser(texte, nom)?;

    // `get_mut` sur une clé absente la crée à `Item::None` pour permettre l'écriture :
    // vérifier l'existence de la section avant de la traverser en mutable évite qu'une
    // absence s'y déguise en `Item::None` au lieu de déclencher `PasUnProjet`.
    if document
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("rbs"))
        .is_none()
    {
        return Err(Erreur::PasUnProjet {
            chemin: nom.to_string(),
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
        .ok_or_else(|| Erreur::Champ {
            chemin: nom.to_string(),
            cle: "features",
        })?;

    if installees
        .iter()
        .any(|valeur| valeur.as_str() == Some(feature))
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
pub fn ajouter_dependance(
    texte: &str,
    dep: &Dependance,
    nom: &str,
) -> Result<Option<String>, Erreur> {
    let mut document = analyser(texte, nom)?;

    let dependances = document
        .entry("dependencies")
        .or_insert_with(toml_edit::table)
        .as_table_like_mut()
        .ok_or_else(|| Erreur::Declaration {
            chemin: nom.to_string(),
            cle: "dependencies".to_string(),
        })?;

    let Some(declaree) = dependances.get_mut(&dep.nom) else {
        dependances.insert(&dep.nom, declaration(dep));
        return Ok(Some(document.to_string()));
    };

    if let Some(presente) = version_declaree(declaree)
        && presente != dep.version
    {
        return Err(Erreur::VersionIncompatible {
            chemin: nom.to_string(),
            dependance: dep.nom.clone(),
            presente: presente.to_string(),
            demandee: dep.version.clone(),
        });
    }

    let mut modifie = false;
    for feature in &dep.features {
        modifie |= activer_feature(declaree, feature).ok_or_else(|| Erreur::Declaration {
            chemin: nom.to_string(),
            cle: dep.nom.clone(),
        })?;
    }

    Ok(modifie.then(|| document.to_string()))
}

/// Rend le manifeste avec `feature` activée sur la dépendance `dep`, ou `None` si elle
/// l'est déjà.
///
/// Une dépendance absente est une erreur de l'appelant : activer une feature suppose de
/// savoir dans quelle version, ce que seul l'appelant sait.
pub fn ajouter_feature_a_dependance(
    texte: &str,
    dep: &str,
    feature: &str,
    nom: &str,
) -> Result<Option<String>, Erreur> {
    let mut document = analyser(texte, nom)?;

    let absente = || Erreur::DependanceAbsente {
        chemin: nom.to_string(),
        dependance: dep.to_string(),
    };

    let modifie = {
        let dependances = document
            .get_mut("dependencies")
            .ok_or_else(absente)?
            .as_table_like_mut()
            .ok_or_else(|| Erreur::Declaration {
                chemin: nom.to_string(),
                cle: "dependencies".to_string(),
            })?;

        let declaree = dependances.get_mut(dep).ok_or_else(absente)?;

        activer_feature(declaree, feature).ok_or_else(|| Erreur::Declaration {
            chemin: nom.to_string(),
            cle: dep.to_string(),
        })?
    };

    Ok(modifie.then(|| document.to_string()))
}

/// Inscrit `feature` dans les features installées, sans effet si elle y est déjà.
///
/// Ne réécrit pas le manifeste dans ce cas : une commande relancée ne doit pas salir le
/// working tree.
pub fn ajouter_feature(cargo_toml: &Path, feature: &str) -> Result<(), Erreur> {
    let nom = nommer(cargo_toml);

    let texte = fs::read_to_string(cargo_toml).map_err(|source| Erreur::Acces {
        chemin: nom.clone(),
        source,
    })?;

    let Some(rendu) = inscrire_feature(&texte, feature, &nom)? else {
        return Ok(());
    };

    fs::write(cargo_toml, rendu).map_err(|source| Erreur::Acces {
        chemin: nom,
        source,
    })
}

/// La déclaration à écrire pour une dépendance encore absente.
fn declaration(dep: &Dependance) -> Item {
    if dep.features.is_empty() {
        return Item::Value(Value::from(dep.version.as_str()));
    }

    let mut table = InlineTable::new();
    table.insert("version", Value::from(dep.version.as_str()));
    table.insert(
        "features",
        Value::Array(dep.features.iter().map(String::as_str).collect::<Array>()),
    );

    Item::Value(Value::InlineTable(table))
}

/// La version que la déclaration porte, ou `None` si elle n'en porte pas — cas d'une
/// dépendance en `path` ou en `git`, que le patch laisse alors telle quelle.
fn version_declaree(declaree: &Item) -> Option<&str> {
    declaree
        .as_str()
        .or_else(|| declaree.get("version").and_then(Item::as_str))
}

/// Active `feature` sur une déclaration, en rendant `false` si elle y était déjà et `None`
/// si la déclaration n'a pas une forme manipulable.
///
/// Une déclaration en chaîne devient une table inline : son décor, qui porte l'espacement
/// et le commentaire de fin de ligne, est reporté sur la table.
fn activer_feature(declaree: &mut Item, feature: &str) -> Option<bool> {
    if let Some(version) = declaree.as_str().map(str::to_owned) {
        let decor = declaree.as_value()?.decor().clone();

        let mut table = InlineTable::new();
        table.insert("version", Value::from(version));

        let mut valeur = Value::InlineTable(table);
        *valeur.decor_mut() = decor;
        *declaree = Item::Value(valeur);
    }

    let table = declaree.as_table_like_mut()?;
    let features = table
        .entry("features")
        .or_insert(Item::Value(Value::Array(Array::new())))
        .as_array_mut()?;

    if features
        .iter()
        .any(|valeur| valeur.as_str() == Some(feature))
    {
        return Some(false);
    }

    features.push(feature);

    Some(true)
}

/// Analyse un manifeste en préservant sa mise en forme et ses commentaires.
fn analyser(texte: &str, nom: &str) -> Result<DocumentMut, Erreur> {
    texte
        .parse::<DocumentMut>()
        .map_err(|source| Erreur::Syntaxe {
            chemin: nom.to_string(),
            source,
        })
}

/// Analyse le manifeste en préservant sa mise en forme et ses commentaires.
fn charger(cargo_toml: &Path) -> Result<DocumentMut, Erreur> {
    let source = fs::read_to_string(cargo_toml).map_err(|source| Erreur::Acces {
        chemin: nommer(cargo_toml),
        source,
    })?;

    analyser(&source, &nommer(cargo_toml))
}

fn version(rbs: &Item, cargo_toml: &Path) -> Result<String, Erreur> {
    rbs.get("version")
        .and_then(Item::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Erreur::Champ {
            chemin: nommer(cargo_toml),
            cle: "version",
        })
}

fn features(rbs: &Item, cargo_toml: &Path) -> Result<Vec<String>, Erreur> {
    let manquant = || Erreur::Champ {
        chemin: nommer(cargo_toml),
        cle: "features",
    };

    rbs.get("features")
        .and_then(Item::as_array)
        .ok_or_else(manquant)?
        .iter()
        // Une entrée qui n'est pas une chaîne est une erreur, pas une entrée à ignorer :
        // silencieusement écartée, elle serait réinstallée au prochain `rbs add`.
        .map(|valeur| valeur.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(manquant)
}

/// Le chemin tel qu'il apparaîtra dans un message d'erreur.
fn nommer(cargo_toml: &Path) -> String {
    cargo_toml.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use minijinja::context;
    use tempfile::TempDir;

    use super::*;
    use crate::template::Renderer;

    /// Manifeste du squelette, résolu depuis la crate plutôt que depuis le répertoire
    /// courant, que `cargo test` ne garantit pas.
    const MANIFESTE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../templates/project/Cargo.toml.jinja"
    );

    /// Déroule le manifeste du squelette dans un répertoire temporaire. C'est le plus
    /// proche d'un projet fraîchement généré tant que `rbs new` n'existe pas.
    fn projet_genere() -> (TempDir, PathBuf) {
        let source = fs::read_to_string(MANIFESTE).expect("le manifeste du squelette est lisible");

        let rendu = Renderer::new()
            .rendre(
                &source,
                context! {
                    nom_projet => "mon-api",
                    nom_crate => "mon_api",
                    rbs_core_dep => "\"0.1\"",
                    rbs_version => "0.1.0",
                },
            )
            .expect("le manifeste doit se rendre");

        ecrire(rendu)
    }

    fn ecrire(contenu: impl AsRef<[u8]>) -> (TempDir, PathBuf) {
        let repertoire = TempDir::new().expect("répertoire temporaire créable");
        let chemin = repertoire.path().join("Cargo.toml");
        fs::write(&chemin, contenu).expect("manifeste écrit");

        (repertoire, chemin)
    }

    fn lu(chemin: &Path) -> String {
        fs::read_to_string(chemin).expect("manifeste relisible")
    }

    #[test]
    fn les_metadonnees_d_un_projet_genere_se_relisent() {
        let (_repertoire, chemin) = projet_genere();

        let metadonnees = lire(&chemin).expect("le manifeste généré porte ses métadonnées");

        assert_eq!(metadonnees.version, "0.1.0");
        assert_eq!(metadonnees.features, vec!["health".to_string()]);
    }

    #[test]
    fn ajouter_deux_fois_la_meme_feature_ne_produit_qu_une_entree() {
        let (_repertoire, chemin) = projet_genere();

        ajouter_feature(&chemin, "auth").expect("premier ajout");
        let apres_le_premier = lu(&chemin);
        ajouter_feature(&chemin, "auth").expect("second ajout");

        assert_eq!(
            lu(&chemin),
            apres_le_premier,
            "le second ajout a réécrit le manifeste"
        );
        assert_eq!(
            lire(&chemin).expect("relecture").features,
            vec!["health".to_string(), "auth".to_string()]
        );
    }

    #[test]
    fn un_manifeste_sans_section_rbs_est_refuse_en_nommant_le_fichier() {
        let (_repertoire, chemin) = ecrire("[package]\nname = \"mon-api\"\n");

        let erreur = lire(&chemin).expect_err("un manifeste sans section rbs n'est pas un projet");

        let message = erreur.to_string();
        assert!(
            message.contains(&chemin.display().to_string()),
            "le message ne nomme pas le fichier : {message}"
        );
        assert!(
            message.contains("projet rbs"),
            "le message ne dit pas ce qui manque : {message}"
        );
    }

    #[test]
    fn l_ecriture_preserve_les_commentaires_et_l_ordre_des_sections() {
        let original = r#"# Manifeste écrit à la main.
[package]
name    = "mon-api"          # aligné exprès
version = "0.1.0"

[package.metadata.rbs]
version = "0.1.0"
features = ["health"]

[dependencies]
anyhow = "1.0"  # gardé tel quel
"#;
        let (_repertoire, chemin) = ecrire(original);

        ajouter_feature(&chemin, "redis").expect("ajout");

        let apres = lu(&chemin);
        assert!(
            apres.contains("# Manifeste écrit à la main."),
            "commentaire de tête perdu :\n{apres}"
        );
        assert!(
            apres.contains("name    = \"mon-api\"          # aligné exprès"),
            "formatage et commentaire de `name` perdus :\n{apres}"
        );
        assert!(
            apres.contains("anyhow = \"1.0\"  # gardé tel quel"),
            "commentaire de dépendance perdu :\n{apres}"
        );
        assert!(
            apres.contains("features = [\"health\", \"redis\"]"),
            "feature ajoutée hors du tableau ou mal formatée :\n{apres}"
        );

        let sections: Vec<&str> = apres
            .lines()
            .filter(|ligne| ligne.starts_with('['))
            .collect();
        assert_eq!(
            sections,
            vec!["[package]", "[package.metadata.rbs]", "[dependencies]"],
            "ordre des sections modifié :\n{apres}"
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
    fn une_feature_absente_est_inscrite_sans_toucher_au_reste_du_manifeste() {
        let rendu = inscrire_feature(MANIFESTE_MINIMAL, "docker", "Cargo.toml")
            .expect("le manifeste est valide")
            .expect("la feature est absente, le texte change");

        assert!(rendu.contains(r#"features = ["health", "docker"]"#));
        assert!(rendu.contains("# les features installées"));
        assert!(rendu.starts_with("[package]\nname = \"demo\"\n"));
    }

    #[test]
    fn une_feature_deja_inscrite_ne_produit_aucun_texte() {
        let rendu = inscrire_feature(MANIFESTE_MINIMAL, "health", "Cargo.toml")
            .expect("le manifeste est valide");

        assert_eq!(rendu, None);
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
    fn une_dependance_absente_s_ajoute_sans_deplacer_le_reste() {
        let rendu = ajouter_dependance(
            MANIFESTE_DEPS,
            &Dependance {
                nom: "redis".into(),
                version: "0.32".into(),
                features: vec![],
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la dépendance est absente");

        assert!(rendu.contains(r#"redis = "0.32""#), "{rendu}");
        assert!(rendu.contains("# les dépendances du projet"), "{rendu}");
        assert!(
            rendu.contains(r#"axum = "0.9"       # le serveur"#),
            "{rendu}"
        );
    }

    #[test]
    fn une_dependance_deja_declaree_ne_produit_aucun_texte() {
        let rendu = ajouter_dependance(
            MANIFESTE_DEPS,
            &Dependance {
                nom: "axum".into(),
                version: "0.9".into(),
                features: vec![],
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide");

        assert_eq!(rendu, None);
    }

    #[test]
    fn une_dependance_declaree_dans_une_autre_version_est_un_conflit() {
        let erreur = ajouter_dependance(
            MANIFESTE_DEPS,
            &Dependance {
                nom: "axum".into(),
                version: "0.8".into(),
                features: vec![],
            },
            "Cargo.toml",
        )
        .expect_err("les deux versions ne se réconcilient pas");

        assert!(
            matches!(erreur, Erreur::VersionIncompatible { .. }),
            "{erreur}"
        );
        let message = erreur.to_string();
        assert!(message.contains("axum"), "{message}");
        assert!(
            message.contains("0.8") && message.contains("0.9"),
            "{message}"
        );
    }

    #[test]
    fn une_dependance_declaree_sans_toutes_ses_features_les_recoit() {
        let rendu = ajouter_dependance(
            MANIFESTE_DEPS,
            &Dependance {
                nom: "tokio".into(),
                version: "1".into(),
                features: vec!["rt-multi-thread".into()],
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la feature manque");

        assert!(
            rendu
                .contains(r#"tokio = { version = "1", features = ["macros", "rt-multi-thread"] }"#),
            "{rendu}"
        );
    }

    #[test]
    fn une_dependance_avec_features_se_declare_en_table_inline() {
        let rendu = ajouter_dependance(
            MANIFESTE_DEPS,
            &Dependance {
                nom: "redis".into(),
                version: "0.32".into(),
                features: vec!["tokio-comp".into()],
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la dépendance est absente");

        assert!(
            rendu.contains(r#"redis = { version = "0.32", features = ["tokio-comp"] }"#),
            "{rendu}"
        );
    }

    #[test]
    fn un_manifeste_sans_table_dependencies_en_recoit_une() {
        let rendu = ajouter_dependance(
            MANIFESTE_MINIMAL,
            &Dependance {
                nom: "redis".into(),
                version: "0.32".into(),
                features: vec![],
            },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la dépendance est absente");

        assert!(rendu.contains("[dependencies]"), "{rendu}");
        assert!(rendu.contains(r#"redis = "0.32""#), "{rendu}");
        assert!(rendu.contains("# les features installées"), "{rendu}");
        assert_eq!(
            lire_dependance(&rendu, "redis"),
            Some("0.32".to_string()),
            "la dépendance n'a pas atterri dans `[dependencies]` :\n{rendu}"
        );
    }

    /// La version que le manifeste rendu déclare pour `dependance`, table inline ou chaîne.
    fn lire_dependance(texte: &str, dependance: &str) -> Option<String> {
        let document = texte.parse::<DocumentMut>().expect("TOML valide");
        let item = document.get("dependencies")?.get(dependance)?;

        item.as_str()
            .or_else(|| item.get("version").and_then(|v| v.as_str()))
            .map(str::to_owned)
    }

    /// La ligne du manifeste rendu qui déclare `dependance`.
    fn ligne_de(texte: &str, dependance: &str) -> String {
        texte
            .lines()
            .find(|ligne| ligne.starts_with(dependance))
            .unwrap_or_else(|| panic!("`{dependance}` n'est plus déclarée :\n{texte}"))
            .to_string()
    }

    #[test]
    fn une_feature_s_ajoute_a_une_dependance_deja_en_table_inline() {
        let rendu =
            ajouter_feature_a_dependance(MANIFESTE_DEPS, "tokio", "rt-multi-thread", "Cargo.toml")
                .expect("le manifeste est valide")
                .expect("la feature manque");

        assert_eq!(
            ligne_de(&rendu, "tokio"),
            r#"tokio = { version = "1", features = ["macros", "rt-multi-thread"] }"#
        );
    }

    #[test]
    fn une_dependance_en_chaine_devient_une_table_inline_en_gardant_son_commentaire() {
        let rendu = ajouter_feature_a_dependance(MANIFESTE_DEPS, "axum", "macros", "Cargo.toml")
            .expect("le manifeste est valide")
            .expect("la feature manque");

        let ligne = ligne_de(&rendu, "axum");
        assert!(
            ligne.contains(r#"axum = { version = "0.9", features = ["macros"] }"#),
            "la déclaration n'est pas devenue une table inline : {ligne}"
        );
        assert!(
            ligne.contains("# le serveur"),
            "le commentaire de fin de ligne a été perdu : {ligne}"
        );
    }

    #[test]
    fn une_feature_deja_active_ne_produit_aucun_texte() {
        let rendu = ajouter_feature_a_dependance(MANIFESTE_DEPS, "tokio", "macros", "Cargo.toml")
            .expect("le manifeste est valide");

        assert_eq!(rendu, None);
    }

    #[test]
    fn une_dependance_absente_est_refusee() {
        let erreur =
            ajouter_feature_a_dependance(MANIFESTE_DEPS, "redis", "tokio-comp", "Cargo.toml")
                .expect_err("la dépendance manque");

        assert!(
            matches!(erreur, Erreur::DependanceAbsente { .. }),
            "{erreur}"
        );
        assert!(erreur.to_string().contains("redis"), "{erreur}");
    }

    #[test]
    fn une_feature_sur_un_manifeste_sans_dependances_est_refusee() {
        let erreur =
            ajouter_feature_a_dependance(MANIFESTE_MINIMAL, "axum", "macros", "Cargo.toml")
                .expect_err("il n'y a pas de table `[dependencies]`");

        assert!(
            matches!(erreur, Erreur::DependanceAbsente { .. }),
            "{erreur}"
        );
    }

    #[test]
    fn un_manifeste_sans_section_rbs_est_refuse() {
        let erreur = inscrire_feature("[package]\nname = \"demo\"\n", "docker", "Cargo.toml")
            .expect_err("la section manque");

        assert!(matches!(erreur, Erreur::PasUnProjet { .. }));
    }
}
