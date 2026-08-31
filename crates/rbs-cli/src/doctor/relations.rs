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

use crate::anchors::{self, Anchor, RELATED, RELATIONS};
use crate::generate::entities::{self, Entity};

use super::Check;

const TITLE: &str = "relations";

/// Vérifie qu'aucun modèle ne porte déjà une relation sans porter ses deux ancres.
pub(crate) fn check(root: &Path) -> Check {
    // Un fichier sans relation n'a rien à prouver : ses ancres se réclameront le jour où
    // `rbs generate` voudra y écrire. Le contrôle est donc lu par fichier, et les entités
    // qu'il porte n'y sont examinées que si l'une d'elles a déjà une relation.
    let mut incomplete: Vec<Entity> = Vec::new();

    for entity in entities::scan(root) {
        let Ok(source) = fs::read_to_string(root.join(&entity.file)) else {
            continue;
        };

        if !(source.contains("belongs_to") || source.contains("has_many")) {
            continue;
        }

        let both_anchors_present = [&RELATIONS, &RELATED]
            .iter()
            .map(|anchor| anchor.for_entity(&entity.file, &entity.table))
            .all(|anchor| carries(&source, &anchor));

        if !both_anchors_present {
            incomplete.push(entity);
        }
    }

    if incomplete.is_empty() {
        return Check::ok(TITLE, "les modèles portent leurs ancres de relation");
    }

    // Les fichiers dédoublonnés et triés : `src/auth/model.rs` porte deux entités, et
    // nommer deux fois le même fichier dans le résumé n'apprendrait rien de plus.
    let files: BTreeSet<&str> = incomplete
        .iter()
        .map(|entity| entity.file.as_str())
        .collect();
    let detail = files
        .iter()
        .map(|file| format!("relations manquent dans {file}"))
        .collect::<Vec<_>>()
        .join(", ");

    // Le remède, lui, se donne par entité : les deux blocs à coller portent son nom, et
    // un fichier à deux entités en attend deux paires distinctes.
    let remedy = incomplete
        .iter()
        .map(|entity| {
            format!(
                "dans {}, pour `{}` :\n{}\n\n{}",
                entity.file,
                entity.table,
                RELATIONS.for_entity(&entity.file, &entity.table).block(),
                RELATED.for_entity(&entity.file, &entity.table).block()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    Check::failed(TITLE, detail, remedy)
}

/// Le fichier porte-t-il les deux balises de `anchor` ?
fn carries(source: &str, anchor: &Anchor) -> bool {
    anchors::marks(source, &anchor.opening()) && anchors::marks(source, &anchor.closing())
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
    // <rbs:relations:posts>
    #[sea_orm(has_many = "crate::comments::model::Entity")]
    Comments,
    // </rbs:relations:posts>
}
// <rbs:related:posts>
// </rbs:related:posts>
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
        project_of("posts", source)
    }

    fn project_of(module: &str, source: &str) -> TempDir {
        let root = TempDir::new().expect("le répertoire se crée");
        let directory = root.path().join("src").join(module);
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
        let amputated = MODEL.replace("    // </rbs:relations:posts>\n", "");
        let result = check(project(&amputated).path());

        assert_eq!(result.state, State::Echec);
        assert!(result.detail.contains("src/posts/model.rs"), "{result:?}");
        assert!(result.detail.contains("relations"), "{result:?}");
        assert!(
            result
                .remedy
                .expect("un remède")
                .contains("<rbs:relations:posts>"),
            "le bloc à coller doit être donné"
        );
    }

    // Toujours la ligne 2, dédoublonnée : un même fichier incohérent ne se nomme qu'une
    // fois, même retiré de ses deux ancres.
    #[test]
    fn a_model_carrying_a_relation_but_missing_both_anchors_is_reported_once() {
        let without_anchors = MODEL
            .replace("    // <rbs:relations:posts>\n", "")
            .replace("    // </rbs:relations:posts>\n", "")
            .replace("// <rbs:related:posts>\n", "")
            .replace("// </rbs:related:posts>\n", "");
        let result = check(project(&without_anchors).path());

        assert_eq!(result.state, State::Echec, "{result:?}");
        assert_eq!(
            result.detail.matches("src/posts/model.rs").count(),
            1,
            "{result:?}"
        );
    }

    // Une balise citée dans une chaîne n'est pas un point d'insertion : le contrôle doit
    // la voir absente, comme l'insertion elle-même.
    #[test]
    fn a_relation_anchor_quoted_inside_a_string_does_not_count_as_present() {
        let quoted = MODEL.replace(
            "// <rbs:related:posts>",
            "let aide = \"// <rbs:related:posts>\";",
        );
        let result = check(project(&quoted).path());

        assert_eq!(result.state, State::Echec, "{result:?}");
        assert!(result.detail.contains("src/posts/model.rs"), "{result:?}");
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

    // Deux entités dans un même fichier : les ancres de la première ne valent pas pour
    // la seconde, qui ne peut pas recevoir de relation sans les siennes.
    const TWO_ENTITIES: &str = r#"
pub mod user {
    #[sea_orm(table_name = "users")]
    pub struct Model { pub id: Uuid }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        // <rbs:relations:users>
        #[sea_orm(has_many = "crate::posts::model::Entity")]
        Posts,
        // </rbs:relations:users>
    }

    // <rbs:related:users>
    // </rbs:related:users>
}

pub mod refresh_token {
    #[sea_orm(table_name = "refresh_tokens")]
    pub struct Model { pub id: Uuid }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        // <rbs:relations:refresh_tokens>
        // </rbs:relations:refresh_tokens>
    }

    // <rbs:related:refresh_tokens>
    // </rbs:related:refresh_tokens>
}
"#;

    #[test]
    fn a_file_carrying_two_entities_passes_when_each_has_its_own_pair() {
        assert_eq!(
            check(project_of("auth", TWO_ENTITIES).path()).state,
            State::Bon
        );
    }

    #[test]
    fn a_second_entity_deprived_of_its_own_pair_is_reported_by_name() {
        let amputated = TWO_ENTITIES
            .replace("        // <rbs:relations:refresh_tokens>\n", "")
            .replace("        // </rbs:relations:refresh_tokens>\n", "");
        let result = check(project_of("auth", &amputated).path());

        assert_eq!(result.state, State::Echec, "{result:?}");
        let remedy = result.remedy.expect("un remède");
        assert!(remedy.contains("refresh_tokens"), "{remedy}");
        assert!(
            !remedy.contains("pour `users`"),
            "l'entité complète n'a rien à recoller : {remedy}"
        );
    }
}
