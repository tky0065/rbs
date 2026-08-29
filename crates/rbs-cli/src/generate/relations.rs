//! Confrontation des cibles déclarées dans `--fields` à ce que le projet contient.
//!
//! Séparé du parseur, qui reste pur : une chaîne s'analyse sans projet, une cible ne se
//! juge que contre un inventaire.

use std::fmt;

use super::entities::{self, Entity};
use super::feature::to_singular;
use super::fields::{Field, RelationView, to_pascal_case};

/// Une cible qu'aucune entité du projet ne porte.
// Rien n'appelle encore `resolve` hors des tests : la commande qui l'invoquera, avec
// l'inventaire réellement scanné, arrive à une tâche suivante.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UnknownTarget {
    /// Nom de la relation fautive : `author`.
    pub relation: String,
    /// Cible écrite : `writers`.
    pub target: String,
    /// Tables connues, triées.
    pub known: Vec<String>,
}

impl fmt::Display for UnknownTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "erreur : relation « {} » — « {} » est introuvable dans ce projet\n        \
             → entités connues : {}",
            self.relation,
            self.target,
            self.known.join(", ")
        )
    }
}

impl std::error::Error for UnknownTarget {}

/// Résout chaque référence contre l'inventaire, et pose sa vue pour les templates.
///
/// `generated_table` rejoint les cibles admises : elle n'est pas encore sur le disque,
/// et une entité qui se référence elle-même — un arbre — est légitime.
#[allow(dead_code)]
pub(crate) fn resolve(
    fields: &mut [Field],
    entities: &[Entity],
    generated_table: &str,
) -> Result<(), Vec<UnknownTarget>> {
    let mut known = entities::tables(entities);
    if !known.iter().any(|table| table == generated_table) {
        known.push(generated_table.to_string());
        known.sort();
    }

    let mut errors = Vec::new();

    for field in fields.iter_mut() {
        let Some(reference) = field.reference().cloned() else {
            continue;
        };

        let entity_path = if reference.target == generated_table {
            // L'entité se référence elle-même : son module n'existe pas encore, et
            // `Entity` la désigne depuis son propre fichier.
            "Entity".to_string()
        } else {
            match entities::find(entities, &reference.target) {
                Some(entity) => format!("{}::Entity", entity.module_path),
                None => {
                    errors.push(UnknownTarget {
                        relation: field.relation_name().to_string(),
                        target: reference.target.clone(),
                        known: known.clone(),
                    });
                    continue;
                }
            }
        };

        // `Entity` désigne l'entité locale : sa colonne est `Column::Id`, sans chemin.
        let target_column_path = if entity_path == "Entity" {
            "Column::Id".to_string()
        } else {
            entity_path.replace("::Entity", "::Column::Id")
        };

        field.set_relation(RelationView {
            name: field.relation_name().to_string(),
            variant: to_pascal_case(&to_singular(field.relation_name())),
            target: reference.target.clone(),
            entity_path,
            target_column_path,
            target_iden: to_pascal_case(&reference.target),
            on_delete: reference.on_delete.action().to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::entities::Entity;
    use crate::generate::fields;

    fn inventory() -> Vec<Entity> {
        vec![
            Entity {
                table: "users".to_string(),
                module_path: "crate::auth::model::user".to_string(),
                file: "src/auth/model.rs".to_string(),
            },
            Entity {
                table: "tags".to_string(),
                module_path: "crate::tags::model".to_string(),
                file: "src/tags/model.rs".to_string(),
            },
        ]
    }

    fn resolved(input: &str, generated: &str) -> Vec<fields::Field> {
        let mut parsed = fields::parse(input).expect("la chaîne doit être acceptée");
        resolve(&mut parsed, &inventory(), generated).expect("les cibles doivent se résoudre");
        parsed
    }

    #[test]
    fn a_target_in_a_nested_module_resolves_to_its_full_path() {
        let fields = resolved("author:references:users", "posts");
        let view = fields[0].relation().expect("la vue de relation est posée");

        assert_eq!(view.entity_path, "crate::auth::model::user::Entity");
        assert_eq!(
            view.target_column_path,
            "crate::auth::model::user::Column::Id"
        );
        assert_eq!(view.variant, "Author");
        assert_eq!(view.target_iden, "Users");
        assert_eq!(view.on_delete, "Restrict");
    }

    // Un arbre : l'entité en cours de génération n'est pas encore sur le disque, et
    // doit pourtant être une cible valable.
    #[test]
    fn the_entity_being_generated_is_a_valid_target() {
        let fields = resolved("parent:references:posts:optional", "posts");
        let view = fields[0].relation().expect("la vue de relation est posée");

        assert_eq!(view.entity_path, "Entity");
        assert_eq!(view.target_column_path, "Column::Id");
        assert_eq!(view.target_iden, "Posts");
    }

    #[test]
    fn an_unknown_target_is_rejected_and_names_the_known_tables() {
        let mut parsed = fields::parse("author:references:writers").expect("acceptée");
        let errors = resolve(&mut parsed, &inventory(), "posts")
            .expect_err("une cible inconnue doit être refusée");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].target, "writers");
        assert_eq!(errors[0].relation, "author");
        assert_eq!(errors[0].known, ["posts", "tags", "users"]);
    }

    #[test]
    fn every_unknown_target_is_collected_in_one_pass() {
        let mut parsed = fields::parse("a:references:x,b:references:y").expect("acceptée");
        let errors = resolve(&mut parsed, &inventory(), "posts").expect_err("refusée");

        assert_eq!(errors.len(), 2, "{errors:?}");
    }

    #[test]
    fn the_message_names_the_relation_the_target_and_the_known_tables() {
        let error = UnknownTarget {
            relation: "author".to_string(),
            target: "writers".to_string(),
            known: vec!["posts".to_string(), "users".to_string()],
        };
        let text = error.to_string();

        assert!(text.contains("« writers » est introuvable"), "{text}");
        assert!(text.contains("author"), "{text}");
        assert!(text.contains("posts, users"), "{text}");
    }

    #[test]
    fn a_scalar_field_is_left_untouched() {
        let fields = resolved("title:string", "posts");

        assert!(fields[0].relation().is_none());
    }
}
