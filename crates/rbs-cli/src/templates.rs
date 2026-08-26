//! Validation des templates du squelette de projet.
//!
//! Ces templates sont une interface : les commandes de génération écrivent dans leurs
//! ancres, et un projet déjà déroulé ne bénéficie d'aucune correction faite après coup.
//! Ce module vérifie donc en permanence ce qui ne dépend pas d'un rendu complet — la
//! convention de nommage, les quatre ancres, et l'absence de variable non déclarée.

use std::fs;
use std::path::{Path, PathBuf};

use minijinja::{Value, context};

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
