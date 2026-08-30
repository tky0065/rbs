//! Les deux ancres qu'un modèle de feature doit porter pour recevoir une relation.
//!
//! Hors du registre statique des ancres : leur fichier dépend des features du projet,
//! qui ne se connaissent qu'en le parcourant.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::anchors::{RELATED, RELATIONS};
use crate::generate::entities;

use super::Check;

const TITLE: &str = "relations";

/// Vérifie que chaque modèle du projet porte ses deux ancres de relation.
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

            [&RELATIONS, &RELATED].iter().any(|anchor| {
                !source.contains(&anchor.opening()) || !source.contains(&anchor.closing())
            })
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

    const MODEL: &str = r#"
#[sea_orm(table_name = "posts")]
pub struct Model { pub id: Uuid }

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations>
    // </rbs:relations>
}
// <rbs:related>
// </rbs:related>
"#;

    fn project(source: &str) -> TempDir {
        let root = TempDir::new().expect("le répertoire se crée");
        let directory = root.path().join("src/posts");
        fs::create_dir_all(&directory).expect("le répertoire se crée");
        fs::write(directory.join("model.rs"), source).expect("l'écriture aboutit");
        root
    }

    #[test]
    fn a_model_carrying_both_anchors_passes() {
        assert_eq!(check(project(MODEL).path()).state, State::Bon);
    }

    #[test]
    fn a_model_missing_one_anchor_fails_by_naming_its_file() {
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

    // Un projet engendré avant ce jalon n'a aucune des deux ancres : le contrôle doit
    // le dire une fois par fichier, non deux fois par fichier.
    #[test]
    fn a_model_missing_both_anchors_is_reported_once() {
        let without_anchors = MODEL
            .replace("    // <rbs:relations>\n", "")
            .replace("    // </rbs:relations>\n", "")
            .replace("// <rbs:related>\n", "")
            .replace("// </rbs:related>\n", "");
        let result = check(project(&without_anchors).path());

        assert_eq!(
            result.detail.matches("src/posts/model.rs").count(),
            1,
            "{result:?}"
        );
    }

    #[test]
    fn a_project_without_any_entity_has_nothing_to_report() {
        let root = TempDir::new().expect("le répertoire se crée");

        assert_eq!(check(root.path()).state, State::Bon);
    }
}
