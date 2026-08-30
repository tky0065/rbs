//! Les deux ancres qu'un modèle de feature doit porter pour recevoir une relation.
//!
//! Hors du registre statique des ancres : leur fichier dépend des features du projet,
//! qui ne se connaissent qu'en le parcourant.
//!
//! Contrairement aux ancres statiques, sollicitées à chaque génération quel que soit le
//! projet, celles-ci ne servent qu'à un usage précis — écrire une relation — que beaucoup
//! de projets CRUD ne feront jamais. Réclamer leur présence à tout modèle rougirait en
//! permanence tout projet engendré avant ce jalon, pour une capacité jamais employée : le
//! contrôle ne s'inquiète donc que d'un modèle qui porte déjà une relation (`belongs_to`
//! ou `has_many`) sans avoir les ancres qui permettraient d'en écrire une seconde — un état
//! incohérent, vraisemblablement issu d'une retouche à la main.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::anchors::{RELATED, RELATIONS};
use crate::generate::entities;

use super::Check;

const TITLE: &str = "relations";

/// Vérifie qu'aucun modèle ne porte déjà une relation sans porter ses deux ancres.
pub(crate) fn check(root: &Path) -> Check {
    // Un même fichier peut porter plusieurs entités — `auth` en porte deux : le
    // dédoublonner évite de nommer deux fois le même modèle incomplet.
    let files: BTreeSet<String> = entities::scan(root)
        .into_iter()
        .map(|entity| entity.file)
        .collect();

    let incomplete: Vec<String> = files
        .into_iter()
        .filter(|file| {
            let Ok(source) = fs::read_to_string(root.join(file)) else {
                return false;
            };

            let both_anchors_present = [&RELATIONS, &RELATED].iter().all(|anchor| {
                source.contains(&anchor.opening()) && source.contains(&anchor.closing())
            });

            // Un modèle sans relation n'a rien à perdre à ne pas avoir ses ancres : le
            // CLI les réclamera le jour où `rbs generate` en écrira une. Ce qui justifie
            // le rouge est un modèle qui porte déjà une relation sans pouvoir en recevoir
            // une seconde.
            !both_anchors_present && (source.contains("belongs_to") || source.contains("has_many"))
        })
        .collect();

    if incomplete.is_empty() {
        return Check::ok(TITLE, "les modèles portent leurs ancres de relation");
    }

    let detail = incomplete
        .iter()
        .map(|file| format!("relations manquent dans {file}"))
        .collect::<Vec<_>>()
        .join(", ");

    let remedy = incomplete
        .iter()
        .map(|file| {
            format!(
                "dans {file} :\n{}\n\n{}",
                RELATIONS.block(),
                RELATED.block()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    Check::failed(TITLE, detail, remedy)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    // Porte déjà une relation (`has_many`) entre ses deux ancres — le cas que le
    // contrôle doit surveiller une fois ces ancres retirées.
    const MODEL: &str = r#"
#[sea_orm(table_name = "posts")]
pub struct Model { pub id: Uuid }

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations>
    #[sea_orm(has_many = "crate::comments::model::Entity")]
    Comments,
    // </rbs:relations>
}
// <rbs:related>
// </rbs:related>
"#;

    // Ni ancres, ni relation : le cas de la grande majorité des projets CRUD, qui
    // n'écriront peut-être jamais de relation.
    const MODEL_WITHOUT_A_RELATION: &str = r#"
#[sea_orm(table_name = "posts")]
pub struct Model { pub id: Uuid }

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
"#;

    fn project(source: &str) -> TempDir {
        let root = TempDir::new().expect("le répertoire se crée");
        let directory = root.path().join("src/posts");
        fs::create_dir_all(&directory).expect("le répertoire se crée");
        fs::write(directory.join("model.rs"), source).expect("l'écriture aboutit");
        root
    }

    // Ligne 1 du tableau : les deux ancres sont là, la relation n'a pas d'importance.
    #[test]
    fn a_model_carrying_both_anchors_passes() {
        assert_eq!(check(project(MODEL).path()).state, State::Bon);
    }

    // Ligne 2 : la relation est déjà écrite, une de ses deux ancres a disparu — état
    // incohérent, vraisemblablement issu d'une retouche à la main.
    #[test]
    fn a_model_carrying_a_relation_but_missing_one_anchor_fails_by_naming_its_file() {
        let amputated = MODEL.replace("    // </rbs:relations>\n", "");
        let result = check(project(&amputated).path());

        assert_eq!(result.state, State::Echec);
        assert!(result.detail.contains("src/posts/model.rs"), "{result:?}");
        assert!(result.detail.contains("relations"), "{result:?}");
        assert!(
            result
                .remedy
                .expect("un remède")
                .contains("<rbs:relations>"),
            "le bloc à coller doit être donné"
        );
    }

    // Toujours la ligne 2, dédoublonnée : un même fichier incohérent ne se nomme qu'une
    // fois, même retiré de ses deux ancres.
    #[test]
    fn a_model_carrying_a_relation_but_missing_both_anchors_is_reported_once() {
        let without_anchors = MODEL
            .replace("    // <rbs:relations>\n", "")
            .replace("    // </rbs:relations>\n", "")
            .replace("// <rbs:related>\n", "")
            .replace("// </rbs:related>\n", "");
        let result = check(project(&without_anchors).path());

        assert_eq!(result.state, State::Echec, "{result:?}");
        assert_eq!(
            result.detail.matches("src/posts/model.rs").count(),
            1,
            "{result:?}"
        );
    }

    // Ligne 3 : aucune relation, aucune ancre — rien n'est cassé, le CLI préviendra le
    // jour utile.
    #[test]
    fn a_model_without_any_relation_or_anchor_stays_green() {
        assert_eq!(
            check(project(MODEL_WITHOUT_A_RELATION).path()).state,
            State::Bon
        );
    }

    #[test]
    fn a_project_without_any_entity_has_nothing_to_report() {
        let root = TempDir::new().expect("le répertoire se crée");

        assert_eq!(check(root.path()).state, State::Bon);
    }
}
