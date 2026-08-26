//! Provenance et lecture des templates du squelette de projet.
//!
//! Le binaire porte l'arborescence en lui, pour qu'une installation depuis crates.io
//! n'ait besoin d'aucun fichier externe ; `--template-dir` lui substitue un répertoire du
//! disque, ce dont le développement de rbs a besoin à chaque retouche d'une template.

// Les templates précèdent leurs appelants : aucune commande du CLI n'est encore
// implémentée.
#![allow(dead_code)]

use std::io;
use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

/// Suffixe que porte toute template, et que ne porte aucune destination.
const SUFFIXE: &str = "jinja";

/// Le squelette de projet, embarqué au moment de la compilation du binaire.
static EMBARQUEES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/project");

/// Provenance des templates du squelette.
pub enum Source {
    /// L'arborescence embarquée dans le binaire.
    Embarquees,
    /// Un répertoire du disque, donné par `--template-dir`.
    Repertoire(PathBuf),
}

/// Une template et le chemin auquel son rendu sera écrit.
#[derive(Debug)]
pub struct Fichier {
    /// Chemin de sortie relatif à la racine du projet, suffixe `.jinja` retiré.
    pub destination: PathBuf,
    /// Source de la template, telle quelle : le rendu est l'affaire de l'appelant.
    pub source: String,
}

impl Source {
    /// Retient le répertoire donné par `--template-dir`, ou l'embarqué à défaut.
    pub fn nouvelle(repertoire: Option<&Path>) -> Self {
        match repertoire {
            Some(chemin) => Self::Repertoire(chemin.to_path_buf()),
            None => Self::Embarquees,
        }
    }

    /// Lit toutes les templates, triées par destination.
    ///
    /// Le tri n'est pas cosmétique : `include_dir` et `fs::read_dir` ne rendent pas leurs
    /// entrées dans le même ordre, et le second n'en garantit aucun.
    pub fn fichiers(&self) -> io::Result<Vec<Fichier>> {
        let mut fichiers = Vec::new();

        match self {
            Self::Embarquees => lire_embarquees(&EMBARQUEES, &mut fichiers)?,
            Self::Repertoire(racine) => lire_repertoire(racine, racine, &mut fichiers)?,
        }

        fichiers.sort_by(|gauche, droite| gauche.destination.cmp(&droite.destination));

        Ok(fichiers)
    }
}

fn lire_embarquees(repertoire: &Dir<'static>, fichiers: &mut Vec<Fichier>) -> io::Result<()> {
    for sous_repertoire in repertoire.dirs() {
        lire_embarquees(sous_repertoire, fichiers)?;
    }

    for fichier in repertoire.files() {
        // Une template non-UTF-8 est une template qu'aucun rendu ne traversera : la
        // laisser passer déplacerait l'échec dans l'écriture du projet.
        let source = fichier.contents_utf8().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} n'est pas de l'UTF-8", fichier.path().display()),
            )
        })?;

        fichiers.push(Fichier {
            destination: destination(fichier.path()),
            source: source.to_owned(),
        });
    }

    Ok(())
}

fn lire_repertoire(
    racine: &Path,
    repertoire: &Path,
    fichiers: &mut Vec<Fichier>,
) -> io::Result<()> {
    let entrees = std::fs::read_dir(repertoire).map_err(|erreur| nommer(repertoire, erreur))?;

    for entree in entrees {
        let chemin = entree.map_err(|erreur| nommer(repertoire, erreur))?.path();

        if chemin.is_dir() {
            lire_repertoire(racine, &chemin, fichiers)?;
            continue;
        }

        let source = std::fs::read_to_string(&chemin).map_err(|erreur| nommer(&chemin, erreur))?;
        let relatif = chemin.strip_prefix(racine).unwrap_or(&chemin);

        fichiers.push(Fichier {
            destination: destination(relatif),
            source,
        });
    }

    Ok(())
}

/// Retire le suffixe `.jinja` du chemin d'une template.
///
/// C'est l'unique endroit du CLI où la convention du §1 du design du squelette
/// s'applique : tout le reste du code ne voit que des destinations. Un chemin sans
/// suffixe traverse intact — le refuser transformerait la faute de frappe d'un
/// `--template-dir` en erreur incompréhensible.
fn destination(template: &Path) -> PathBuf {
    if template
        .extension()
        .is_some_and(|suffixe| suffixe == SUFFIXE)
    {
        template.with_extension("")
    } else {
        template.to_path_buf()
    }
}

/// Rejoue une erreur d'entrée-sortie en nommant le chemin en cause.
///
/// Un `--template-dir` mal saisi est l'erreur la plus probable de ce flag, et
/// « No such file or directory » seul ne la corrige pas.
fn nommer(chemin: &Path, erreur: io::Error) -> io::Error {
    io::Error::new(erreur.kind(), format!("{} : {erreur}", chemin.display()))
}

#[cfg(test)]
mod tests {
    //! Ces templates sont une interface : les commandes de génération écrivent dans leurs
    //! ancres, et un projet déjà déroulé ne bénéficie d'aucune correction faite après
    //! coup. On vérifie donc en permanence ce qui ne dépend pas d'un rendu complet — la
    //! convention de nommage, les quatre ancres, et l'absence de variable non déclarée.

    use std::fs;
    use std::path::{Path, PathBuf};

    use minijinja::{Value, context};

    use super::*;
    use crate::template::Renderer;

    /// Racine des templates du squelette, résolue depuis la crate plutôt que depuis le
    /// répertoire courant, que `cargo test` ne garantit pas.
    const RACINE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../templates/project");

    /// Les quatre points d'insertion, chacun avec le fichier qui le porte.
    const ANCRES: [(&str, &str); 4] = [
        ("features", "src/features/mod.rs.jinja"),
        ("routes", "src/router.rs.jinja"),
        ("openapi", "src/openapi.rs.jinja"),
        ("migrations", "migration/src/lib.rs.jinja"),
    ];

    /// Les chemins de sortie attendus du squelette, tels que `rbs new` les écrira.
    const DESTINATIONS: [&str; 14] = [
        ".env.example",
        ".gitignore",
        "Cargo.toml",
        "config/default.toml",
        "config/development.toml",
        "migration/Cargo.toml",
        "migration/src/lib.rs",
        "src/features/health/controller.rs",
        "src/features/health/mod.rs",
        "src/features/mod.rs",
        "src/main.rs",
        "src/openapi.rs",
        "src/router.rs",
        "src/state.rs",
    ];

    /// Contexte de rendu minimal : les trois variables que `rbs new` fournira.
    fn contexte() -> Value {
        context! {
            nom_projet => "mon-api",
            nom_crate => "mon_api",
            rbs_core_dep => "\"0.1\"",
        }
    }

    /// Toutes les templates du squelette, répertoires imbriqués compris.
    fn templates() -> Vec<PathBuf> {
        let mut trouvees = Vec::new();
        parcourir(Path::new(RACINE), &mut trouvees);

        assert!(
            !trouvees.is_empty(),
            "aucune template trouvée sous {RACINE}"
        );

        trouvees
    }

    fn parcourir(repertoire: &Path, trouvees: &mut Vec<PathBuf>) {
        let entrees = fs::read_dir(repertoire).unwrap_or_else(|erreur| {
            panic!("{} illisible : {erreur}", repertoire.display());
        });

        for entree in entrees {
            let chemin = entree.expect("entrée de répertoire lisible").path();
            if chemin.is_dir() {
                parcourir(&chemin, trouvees);
            } else {
                trouvees.push(chemin);
            }
        }
    }

    fn lire(chemin: &Path) -> String {
        fs::read_to_string(chemin).unwrap_or_else(|erreur| {
            panic!("{} illisible : {erreur}", chemin.display());
        })
    }

    #[test]
    fn chaque_template_porte_le_suffixe_jinja() {
        for chemin in templates() {
            assert_eq!(
                chemin.extension().and_then(|suffixe| suffixe.to_str()),
                Some("jinja"),
                "{} ne porte pas le suffixe `.jinja`",
                chemin.display()
            );
        }
    }

    #[test]
    fn chaque_ancre_est_ouverte_puis_refermee_dans_son_fichier() {
        for (nom, relatif) in ANCRES {
            let chemin = Path::new(RACINE).join(relatif);
            let source = lire(&chemin);

            let ouverture = format!("// <rbs:{nom}>");
            let fermeture = format!("// </rbs:{nom}>");

            assert_eq!(
                source.matches(&ouverture).count(),
                1,
                "{relatif} doit porter une fois `{ouverture}`"
            );
            assert_eq!(
                source.matches(&fermeture).count(),
                1,
                "{relatif} doit porter une fois `{fermeture}`"
            );
            assert!(
                source.find(&ouverture) < source.find(&fermeture),
                "{relatif} referme `{nom}` avant de l'ouvrir"
            );
        }
    }

    #[test]
    fn chaque_template_se_rend_avec_les_trois_variables() {
        let renderer = Renderer::new();

        for chemin in templates() {
            let source = lire(&chemin);
            renderer
                .rendre(&source, contexte())
                .unwrap_or_else(|erreur| {
                    panic!("{} ne se rend pas : {erreur}", chemin.display());
                });
        }
    }

    #[test]
    fn le_manifeste_rendu_porte_le_nom_du_projet_et_la_dependance_au_noyau() {
        let source = lire(&Path::new(RACINE).join("Cargo.toml.jinja"));

        let rendu = Renderer::new()
            .rendre(&source, contexte())
            .expect("le manifeste doit se rendre");

        assert!(
            rendu.contains("name = \"mon-api\""),
            "nom du paquet absent du manifeste rendu :\n{rendu}"
        );
        assert!(
            rendu.contains("rbs-core = \"0.1\""),
            "dépendance au noyau absente du manifeste rendu :\n{rendu}"
        );
    }

    #[test]
    fn la_source_embarquee_restitue_le_squelette_avec_ses_chemins_de_sortie() {
        let fichiers = Source::nouvelle(None)
            .fichiers()
            .expect("les templates embarquées doivent se lire");

        let destinations: Vec<String> = fichiers
            .iter()
            .map(|fichier| fichier.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, DESTINATIONS);

        for fichier in &fichiers {
            assert!(
                !fichier.source.is_empty(),
                "{} est embarquée vide",
                fichier.destination.display()
            );
        }
    }

    #[test]
    fn aucune_destination_ne_porte_le_suffixe_jinja() {
        let fichiers = Source::nouvelle(None)
            .fichiers()
            .expect("les templates embarquées doivent se lire");

        for fichier in fichiers {
            assert_ne!(
                fichier.destination.extension(),
                Some("jinja".as_ref()),
                "{} garde le suffixe `.jinja`",
                fichier.destination.display()
            );
        }
    }

    #[test]
    fn un_repertoire_de_templates_prend_le_pas_sur_l_embarque() {
        let repertoire = tempfile::tempdir().expect("répertoire temporaire créable");
        fs::create_dir(repertoire.path().join("config")).expect("sous-répertoire créable");
        fs::write(
            repertoire.path().join("Cargo.toml.jinja"),
            "name = \"surcharge\"",
        )
        .expect("template écrivable");
        fs::write(
            repertoire.path().join("config/default.toml.jinja"),
            "port = 1",
        )
        .expect("template écrivable");

        let fichiers = Source::nouvelle(Some(repertoire.path()))
            .fichiers()
            .expect("le répertoire doit se lire");

        let destinations: Vec<String> = fichiers
            .iter()
            .map(|fichier| fichier.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, ["Cargo.toml", "config/default.toml"]);
        assert_eq!(fichiers[0].source, "name = \"surcharge\"");
    }

    #[test]
    fn un_repertoire_de_templates_inexistant_echoue_en_nommant_le_chemin() {
        let absent = Path::new("/introuvable/templates/rbs");

        let erreur = Source::nouvelle(Some(absent))
            .fichiers()
            .expect_err("un répertoire absent ne doit pas rendre une liste vide");

        assert!(
            erreur.to_string().contains("/introuvable/templates/rbs"),
            "le message ne nomme pas le chemin : {erreur}"
        );
    }
}
