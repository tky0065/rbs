//! Ce que les générateurs du lot partagent : le nom de la feature et ses champs.
//!
//! Une feature se nomme au pluriel — `users` — mais son entité, ses DTO et son service se
//! nomment au singulier — `User`, `CreateUser`. La dérivation est faite ici, une fois,
//! plutôt que dans chaque template.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

use super::fields::{Field, RelationView, to_pascal_case};

/// Une feature à générer, telle que la voient l'entité, les DTO et la migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Feature {
    /// Nom au pluriel, en snake_case : le module et la table le portent tel quel.
    pub name: String,
    /// Champs déclarés dans `--fields`, sans `id` ni les horodatages.
    pub fields: Vec<Field>,
}

impl Feature {
    pub(crate) fn fresh(name: &str, fields: Vec<Field>) -> Self {
        Self {
            name: name.to_string(),
            fields,
        }
    }

    /// Nom du module et de la table : le nom donné, inchangé.
    pub(crate) fn module(&self) -> &str {
        &self.name
    }

    /// Nom de l'entité en PascalCase singulier : `blog_posts` donne `BlogPost`.
    pub(crate) fn entity(&self) -> String {
        to_pascal_case(&self.singular())
    }

    /// Nom de l'enum `DeriveIden` de la migration : `blog_posts` donne `BlogPosts`.
    ///
    /// SeaORM tire le nom de la table de celui de l'enum, pas de sa variante `Table` :
    /// l'enum se nomme donc au pluriel, contrairement à l'entité.
    pub(crate) fn iden(&self) -> String {
        to_pascal_case(&self.name)
    }

    /// Nom singulier en snake_case : `blog_posts` donne `blog_post`.
    pub(crate) fn singular(&self) -> String {
        to_singular(&self.name)
    }

    /// Relations dont la cible n'est visée qu'une fois : `impl Related` y a une réponse
    /// juste. Deux relations vers la même table s'excluent l'une l'autre, quel que soit
    /// leur nombre — `Related` prend un type pour clé, pas une paire (type, relation).
    pub(crate) fn unique_relations(&self) -> Vec<&RelationView> {
        let counts = self.target_counts();

        self.fields
            .iter()
            .filter_map(|field| field.relation())
            .filter(|relation| counts.get(&relation.target) == Some(&1))
            .collect()
    }

    /// Tables visées par plus d'une relation : sans réponse juste pour `impl Related`, une
    /// par table plutôt qu'une par relation, dans l'ordre où la première des concurrentes
    /// est déclarée.
    pub(crate) fn ambiguous_targets(&self) -> Vec<AmbiguousTarget> {
        let counts = self.target_counts();
        let mut deja_vues = HashSet::new();

        self.fields
            .iter()
            .filter_map(|field| field.relation())
            .filter(|relation| counts.get(&relation.target) != Some(&1))
            .filter(|relation| deja_vues.insert(relation.target.clone()))
            .map(|relation| {
                AmbiguousTarget::new(&relation.target, self.variants_towards(&relation.target))
            })
            .collect()
    }

    /// Nombre de relations déclarées vers chaque table.
    fn target_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for relation in self.fields.iter().filter_map(|field| field.relation()) {
            *counts.entry(relation.target.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Variantes `Relation` qui visent `target`, dans l'ordre de déclaration des champs.
    fn variants_towards(&self, target: &str) -> Vec<String> {
        self.fields
            .iter()
            .filter_map(|field| field.relation())
            .filter(|relation| relation.target == target)
            .map(|relation| relation.variant.clone())
            .collect()
    }
}

/// Une table visée par plus d'une relation de la feature.
///
/// `Related<T>` prend le type cible pour seule clé : deux relations vers la même table
/// s'implémenteraient toutes deux `Related<T> for Entity`, ce que `rustc` refuse
/// (`E0119`). Aucune des deux n'a de meilleure prétention à l'implémentation que l'autre,
/// donc aucune n'est écrite — un commentaire explique comment joindre à la place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AmbiguousTarget {
    /// Commentaire prêt à écrire, à la place de l'`impl Related` qu'on ne peut pas poser
    /// sans arbitrairement préférer une des relations concurrentes.
    pub comment: String,
}

impl AmbiguousTarget {
    fn new(target: &str, variants: Vec<String>) -> Self {
        let nommees = variants
            .iter()
            .map(|variant| format!("`{variant}`"))
            .collect::<Vec<_>>()
            .join(", ");
        // La première variante déclarée sert d'exemple : n'importe laquelle joint la même
        // table, le choix n'a donc pas besoin d'être significatif.
        let exemple = variants.first().cloned().unwrap_or_default();

        Self {
            comment: format!(
                "// `{target}` est visée par {compte} relations ({nommees}) : `Related` \
                 serait ambigu. Joindre explicitement, par exemple\n\
                 // `Entity::find().join(JoinType::LeftJoin, Relation::{exemple}.def())`.",
                compte = variants.len(),
            ),
        }
    }
}

/// Sérialisé à la main, comme `Field` : minijinja ne voit pas les méthodes Rust, et les
/// templates lisent `entity` comme elles lisent `module`.
impl Serialize for Feature {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Feature", 8)?;
        state.serialize_field("module", self.module())?;
        state.serialize_field("table", self.module())?;
        state.serialize_field("entity", &self.entity())?;
        state.serialize_field("iden", &self.iden())?;
        state.serialize_field("singular", &self.singular())?;
        state.serialize_field("fields", &self.fields)?;
        state.serialize_field("unique_relations", &self.unique_relations())?;
        state.serialize_field("ambiguous_targets", &self.ambiguous_targets())?;
        state.end()
    }
}

/// Singularise le dernier mot d'un nom en snake_case.
///
/// Quinze lignes d'anglais approximatif plutôt qu'une crate d'inflexion : ce qu'on
/// cherche est un nom de type Rust, que l'utilisateur relira et pourra corriger, pas une
/// forme grammaticale juste. Les cas irréguliers — `people`, `children` — sortent
/// inchangés, ce qui donne une entité `People` : lisible, et rectifiable à la main.
pub(crate) fn to_singular(name: &str) -> String {
    let (prefixe, dernier) = match name.rfind('_') {
        Some(coupe) => name.split_at(coupe + 1),
        None => ("", name),
    };

    let singular = if let Some(root) = dernier.strip_suffix("ies") {
        // `ies` sur un mot d'une syllabe — `ties`, `pies` — ne vient pas d'un `y`.
        if root.is_empty() {
            dernier.to_string()
        } else {
            format!("{root}y")
        }
    } else if SIFFLANTES
        .iter()
        .any(|sifflante| dernier.ends_with(&format!("{sifflante}es")))
    {
        dernier[..dernier.len() - 2].to_string()
    } else if dernier.ends_with('s') && !FINALES_NON_PLURIELLES.iter().any(|f| dernier.ends_with(f))
    {
        dernier[..dernier.len() - 1].to_string()
    } else {
        dernier.to_string()
    };

    format!("{prefixe}{singular}")
}

/// Consonnes après lesquelles le pluriel anglais s'écrit `es` et non `s`.
const SIFFLANTES: [&str; 5] = ["s", "x", "z", "ch", "sh"];

/// Terminaisons en `s` qui ne sont pas des marques de pluriel : `status`, `class`.
const FINALES_NON_PLURIELLES: [&str; 3] = ["ss", "us", "is"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_regular_plural_loses_its_s() {
        assert_eq!(to_singular("users"), "user");
        assert_eq!(to_singular("articles"), "article");
        assert_eq!(to_singular("comments"), "comment");
    }

    #[test]
    fn a_plural_in_ies_becomes_y_again() {
        assert_eq!(to_singular("categories"), "category");
        assert_eq!(to_singular("companies"), "company");
    }

    #[test]
    fn a_plural_in_es_after_a_sibilant_loses_both_letters() {
        assert_eq!(to_singular("addresses"), "address");
        assert_eq!(to_singular("boxes"), "box");
        assert_eq!(to_singular("branches"), "branch");
        assert_eq!(to_singular("dishes"), "dish");
    }

    #[test]
    fn an_already_singular_name_passes_through_intact() {
        assert_eq!(to_singular("status"), "status");
        assert_eq!(to_singular("person"), "person");
        assert_eq!(to_singular("data"), "data");
    }

    #[test]
    fn only_the_last_word_is_singularised() {
        assert_eq!(to_singular("blog_posts"), "blog_post");
        assert_eq!(to_singular("users_categories"), "users_category");
    }

    #[test]
    fn the_entity_is_the_singular_in_pascal_case() {
        let feature = Feature::fresh("blog_posts", Vec::new());

        assert_eq!(feature.entity(), "BlogPost");
        assert_eq!(feature.singular(), "blog_post");
        assert_eq!(feature.module(), "blog_posts");
    }

    #[test]
    fn serialisation_exposes_the_derived_names_to_the_templates() {
        let feature = Feature::fresh("users", Vec::new());
        let vue = serde_json::to_value(&feature).expect("la feature doit se sérialiser");

        assert_eq!(vue["module"], "users");
        assert_eq!(vue["table"], "users");
        assert_eq!(vue["entity"], "User");
        assert_eq!(vue["iden"], "Users");
        assert_eq!(vue["singular"], "user");
        assert!(vue["fields"].is_array(), "les champs doivent être exposés");
    }
}
