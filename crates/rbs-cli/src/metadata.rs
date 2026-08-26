//! Lecture et mise à jour de `[package.metadata.rbs]`.
//!
//! Le `Cargo.toml` du projet est le seul endroit où rbs garde son état : version qui a
//! généré le projet, features installées. Un fichier de plus, non versionné par
//! réflexe, se serait désynchronisé du dépôt.

use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item};

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
    let mut document = texte
        .parse::<DocumentMut>()
        .map_err(|source| Erreur::Syntaxe {
            chemin: nom.to_string(),
            source,
        })?;

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

/// Analyse le manifeste en préservant sa mise en forme et ses commentaires.
fn charger(cargo_toml: &Path) -> Result<DocumentMut, Erreur> {
    let source = fs::read_to_string(cargo_toml).map_err(|source| Erreur::Acces {
        chemin: nommer(cargo_toml),
        source,
    })?;

    source
        .parse::<DocumentMut>()
        .map_err(|source| Erreur::Syntaxe {
            chemin: nommer(cargo_toml),
            source,
        })
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

    #[test]
    fn un_manifeste_sans_section_rbs_est_refuse() {
        let erreur = inscrire_feature("[package]\nname = \"demo\"\n", "docker", "Cargo.toml")
            .expect_err("la section manque");

        assert!(matches!(erreur, Erreur::PasUnProjet { .. }));
    }
}
