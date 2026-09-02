//! Rendu de `<name>/model.rs` : l'entité SeaORM d'une feature.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/model.rs.jinja"
));

/// Rend l'entité SeaORM de `feature`.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().render(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{bench, fields};

    fn entity(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields)).expect("l'entité doit se rendre")
    }

    fn entity_with(
        name: &str,
        fields: &str,
        entities: &[crate::generate::entities::Entity],
    ) -> String {
        let mut parsed = fields::parse(fields).expect("les champs du test doivent être valides");
        crate::generate::relations::resolve(&mut parsed, entities, name)
            .expect("les cibles du test doivent se résoudre");
        render(&Feature::fresh(name, parsed)).expect("l'entité doit se rendre")
    }

    fn users_entity() -> Vec<crate::generate::entities::Entity> {
        vec![crate::generate::entities::Entity {
            table: "users".to_string(),
            module_path: "crate::auth::model::user".to_string(),
            file: "src/auth/model.rs".to_string(),
        }]
    }

    // `tags` s'ajoute à `users` : un test d'ambiguïté doit aussi prouver qu'une relation
    // ordinaire, vers une autre cible, continue de recevoir son `impl Related`.
    fn users_and_tags_entities() -> Vec<crate::generate::entities::Entity> {
        let mut entities = users_entity();
        entities.push(crate::generate::entities::Entity {
            table: "tags".to_string(),
            module_path: "crate::tags::model".to_string(),
            file: "src/tags/model.rs".to_string(),
        });
        entities
    }

    #[test]
    fn the_primary_key_is_a_uuid_without_auto_increment() {
        let rendered = entity("users", "name:string");

        assert!(
            rendered.contains("#[sea_orm(primary_key, auto_increment = false)]\n    pub id: Uuid,"),
            "clé primaire attendue en Uuid non auto-incrémenté :\n{rendered}"
        );
    }

    // Tous les chemins d'insertion du projet passent par `..Default::default()`, que la
    // macro fait déléguer à `ActiveModelBehavior::new()` : c'est le seul point à écrire.
    #[test]
    fn the_model_lays_the_v7_identifier_itself() {
        let rendered = entity("users", "name:string");

        assert!(
            rendered.contains("Uuid::now_v7()"),
            "le modèle ne pose pas l'identifiant :\n{rendered}"
        );
        assert!(
            rendered.contains("fn new() -> Self"),
            "l'identifiant n'est pas posé par `new()`, seul point que `Default` appelle :\n{rendered}"
        );
    }

    #[test]
    fn the_table_carries_the_plural_name_of_the_feature() {
        let rendered = entity("blog_posts", "title:string");

        assert!(
            rendered.contains(r#"#[sea_orm(table_name = "blog_posts")]"#),
            "nom de table absent :\n{rendered}"
        );
    }

    #[test]
    fn each_type_of_the_grammar_projects_into_the_entity() {
        let rendered = entity(
            "samples",
            "title:string,quantity:int,price:float,active:bool,owner:uuid,\
             published_at:datetime,body:text",
        );

        for expected in [
            "pub title: String,",
            "pub quantity: i32,",
            "pub price: f64,",
            "pub active: bool,",
            "pub owner: Uuid,",
            "pub published_at: DateTimeWithTimeZone,",
            "pub body: String,",
        ] {
            assert!(
                rendered.contains(expected),
                "« {expected} » absent de :\n{rendered}"
            );
        }
    }

    #[test]
    fn an_optional_field_becomes_an_option() {
        let rendered = entity("users", "bio:string:optional");

        assert!(
            rendered.contains("pub bio: Option<String>,"),
            "champ optionnel non rendu en Option :\n{rendered}"
        );
    }

    #[test]
    fn a_unique_field_carries_the_matching_attribute() {
        let rendered = entity("users", "email:string:unique");

        assert!(
            rendered.contains("#[sea_orm(unique)]\n    pub email: String,"),
            "attribut unique absent :\n{rendered}"
        );
    }

    #[test]
    fn an_indexed_field_carries_the_matching_attribute() {
        let rendered = entity("users", "slug:string:index");

        assert!(
            rendered.contains("#[sea_orm(indexed)]\n    pub slug: String,"),
            "attribut indexed absent :\n{rendered}"
        );
    }

    #[test]
    fn a_text_field_forces_its_column_type() {
        let rendered = entity("articles", "corps:text");

        assert!(
            rendered.contains(r#"#[sea_orm(column_type = "Text")]"#),
            "type de colonne Text non forcé :\n{rendered}"
        );
    }

    // `text` était le seul type à cumuler un attribut de colonne avec un `indexed` ;
    // maintenant qu'il refuse « unique » et « index », plus aucune ligne de `--fields`
    // n'en pose deux. Ce qui reste à prouver, c'est qu'aucun ne s'y glisse.
    #[test]
    fn a_text_field_carries_its_column_type_alone() {
        let rendered = entity("articles", "summary:text:optional");

        assert!(
            rendered
                .contains("#[sea_orm(column_type = \"Text\")]\n    pub summary: Option<String>,"),
            "attribut inattendu sur la colonne :\n{rendered}"
        );
    }

    /// Le corps de `struct Model`, sans la clé primaire ni les horodatages qui l'encadrent.
    ///
    /// Compter les attributs sur le rendu entier confondrait `table_name` et les
    /// `belongs_to` des relations avec ceux que porte une colonne.
    fn model_block(rendered: &str) -> &str {
        rendered
            .split_once("pub struct Model {")
            .and_then(|(_, reste)| reste.split_once("\n}"))
            .map_or_else(
                || panic!("le bloc du modèle doit se délimiter :\n{rendered}"),
                |(corps, _)| corps,
            )
    }

    /// Le gabarit n'écrit qu'un attribut par colonne, en `if`/`elif`, parce qu'aucune
    /// ligne de `--fields` acceptée n'en produit deux : `column_type` ne sort que sur
    /// `text`, qui refuse `unique` comme `index` ; sur les autres scalaires, les cumuler
    /// est refusé comme redondant ; et une référence n'indexe que ce qu'`unique`
    /// n'indexe pas déjà. Ce test exerce les six cas où un attribut sort, plus le cas nu.
    #[test]
    fn each_accepted_field_carries_the_single_attribute_it_earns() {
        for (spec, expected) in [
            (
                "body:text",
                "#[sea_orm(column_type = \"Text\")]\n    pub body: String,",
            ),
            (
                "body:text:optional",
                "#[sea_orm(column_type = \"Text\")]\n    pub body: Option<String>,",
            ),
            (
                "email:string:unique",
                "#[sea_orm(unique)]\n    pub email: String,",
            ),
            (
                "slug:string:index",
                "#[sea_orm(indexed)]\n    pub slug: String,",
            ),
            (
                "author:references:users:unique",
                "#[sea_orm(unique)]\n    pub author_id: Uuid,",
            ),
            (
                "author:references:users",
                "#[sea_orm(indexed)]\n    pub author_id: Uuid,",
            ),
            ("title:string", "pub title: String,"),
        ] {
            let rendered = entity_with("posts", spec, &users_entity());
            let model = model_block(&rendered);

            assert!(
                model.contains(expected),
                "« {spec} » doit rendre :\n{expected}\n--- rendu :\n{model}"
            );
            assert_eq!(
                model.matches("#[sea_orm(").count(),
                1 + usize::from(expected.contains("#[sea_orm(")),
                "« {spec} » doit porter la clé primaire et son seul attribut :\n{model}"
            );
        }
    }

    /// Les attributs se distribuent par champ : une colonne ne doit pas hériter de celui
    /// de sa voisine, ce qu'une déclaration isolée ne peut pas montrer.
    #[test]
    fn on_a_full_line_of_fields_each_attribute_stays_on_its_own_column() {
        let rendered = entity_with(
            "posts",
            "title:string,email:string:unique,slug:string:index,body:text,author:references:users",
            &users_entity(),
        );
        let model = model_block(&rendered);

        assert_eq!(
            model,
            "\n    #[sea_orm(primary_key, auto_increment = false)]\n    \
             pub id: Uuid,\n    \
             pub title: String,\n    \
             #[sea_orm(unique)]\n    pub email: String,\n    \
             #[sea_orm(indexed)]\n    pub slug: String,\n    \
             #[sea_orm(column_type = \"Text\")]\n    pub body: String,\n    \
             #[sea_orm(indexed)]\n    pub author_id: Uuid,\n    \
             pub created_at: DateTimeWithTimeZone,\n    \
             pub updated_at: DateTimeWithTimeZone,",
            "le modèle rendu :\n{rendered}"
        );
    }

    #[test]
    fn the_timestamps_are_set_without_having_been_declared() {
        let rendered = entity("users", "name:string");

        assert!(
            rendered.contains("pub created_at: DateTimeWithTimeZone,"),
            "{rendered}"
        );
        assert!(
            rendered.contains("pub updated_at: DateTimeWithTimeZone,"),
            "{rendered}"
        );
    }

    #[test]
    fn a_field_less_feature_renders_a_complete_entity() {
        let rendered = entity("tokens", "");

        assert!(rendered.contains("pub struct Model {"), "{rendered}");
        assert!(rendered.contains("pub enum Relation {"), "{rendered}");
        assert!(
            rendered.contains("impl ActiveModelBehavior for ActiveModel {"),
            "{rendered}"
        );
    }

    #[test]
    fn a_reference_becomes_a_uuid_column_named_after_the_relation() {
        let rendered = entity_with("posts", "author:references:users", &users_entity());

        assert!(rendered.contains("pub author_id: Uuid,"), "{rendered}");
    }

    #[test]
    fn a_reference_declares_its_variant_and_its_on_delete() {
        let rendered = entity_with("posts", "author:references:users", &users_entity());

        assert!(
            rendered.contains(r#"belongs_to = "crate::auth::model::user::Entity""#),
            "{rendered}"
        );
        assert!(
            rendered.contains(r#"from = "Column::AuthorId""#),
            "{rendered}"
        );
        assert!(
            rendered.contains(r#"to = "crate::auth::model::user::Column::Id""#),
            "{rendered}"
        );
        assert!(rendered.contains(r#"on_delete = "Restrict""#), "{rendered}");
        assert!(rendered.contains("    Author,"), "{rendered}");
    }

    #[test]
    fn a_reference_implements_related_towards_its_target() {
        let rendered = entity_with("posts", "author:references:users", &users_entity());

        assert!(
            rendered.contains("impl Related<crate::auth::model::user::Entity> for Entity {"),
            "{rendered}"
        );
        assert!(
            rendered.contains("fn to() -> RelationDef {\n        Relation::Author.def()"),
            "{rendered}"
        );
    }

    // `Related<T>` prend le type cible pour seule clé : deux relations vers `users`
    // implémenteraient toutes deux `Related<crate::auth::model::user::Entity> for
    // Entity`, qu'`rustc` refuse (E0119). Aucune des deux n'a de meilleure prétention à
    // l'implémentation que l'autre, donc aucune ne s'écrit.
    #[test]
    fn two_relations_to_the_same_target_yield_no_related_impl_and_a_comment_naming_both() {
        let rendered = entity_with(
            "posts",
            "author:references:users,reviewer:references:users",
            &users_entity(),
        );

        assert!(
            !rendered.contains("impl Related<crate::auth::model::user::Entity> for Entity {"),
            "un `impl Related` a été émis malgré l'ambiguïté :\n{rendered}"
        );
        assert!(rendered.contains("`users`"), "{rendered}");
        assert!(rendered.contains("`Author`"), "{rendered}");
        assert!(rendered.contains("`Reviewer`"), "{rendered}");
    }

    #[test]
    fn two_relations_to_different_targets_each_get_their_own_related_impl() {
        let rendered = entity_with(
            "posts",
            "author:references:users,editor:references:tags",
            &users_and_tags_entities(),
        );

        assert!(
            rendered.contains("impl Related<crate::auth::model::user::Entity> for Entity {"),
            "{rendered}"
        );
        assert!(
            rendered.contains("impl Related<crate::tags::model::Entity> for Entity {"),
            "{rendered}"
        );
        assert_eq!(
            rendered.matches("impl Related<").count(),
            2,
            "deux cibles distinctes, deux implémentations attendues :\n{rendered}"
        );
    }

    #[test]
    fn a_cascade_reference_carries_its_action() {
        let rendered = entity_with("posts", "author:references:users:cascade", &users_entity());

        assert!(rendered.contains(r#"on_delete = "Cascade""#), "{rendered}");
    }

    // Les variantes vivent dans les accolades de l'énumération, les `impl Related` ne le
    // peuvent pas : il faut donc deux ancres, et non une.
    #[test]
    fn the_model_carries_both_anchors_even_without_a_relation() {
        let rendered = entity("posts", "title:string");

        assert!(
            rendered.contains("    // <rbs:relations:posts>\n    // </rbs:relations:posts>"),
            "les ancres portent le nom de l'entité :\n{rendered}"
        );
        assert!(
            rendered.contains("// <rbs:related:posts>\n// </rbs:related:posts>"),
            "{rendered}"
        );
    }

    #[test]
    fn a_self_reference_points_at_the_local_entity() {
        let rendered = entity_with("posts", "parent:references:posts:optional", &[]);

        assert!(rendered.contains(r#"belongs_to = "Entity""#), "{rendered}");
        assert!(
            rendered.contains("pub parent_id: Option<Uuid>,"),
            "{rendered}"
        );
    }

    // La garde de `command.rs` (`the_render_goes_through_rustfmt_without_a_diff_…`) ne
    // porte aucun champ `:references:` : rien n'y prouve que les blocs `impl Related` et
    // le commentaire d'ambiguïté sortent déjà mis en forme. Ce test-ci les couvre au
    // niveau du rendu, en passant par le même `resolve` qu'un vrai `generate crud`.
    #[test]
    fn the_render_with_relations_conforms_to_rustfmt_unique_and_ambiguous_alike() {
        let rendered = entity_with(
            "posts",
            "author:references:users,reviewer:references:users,editor:references:tags",
            &users_and_tags_entities(),
        );

        assert_eq!(
            bench::formatted(&rendered),
            rendered,
            "un `cargo fmt` reformaterait le rendu portant des relations :\n{rendered}"
        );
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_generated_entity_compiles_in_a_fresh_project() {
        let project = bench::Project::fresh();
        let rendered = entity(
            "articles",
            "title:string,slug:string:unique,summary:text:optional,views:int,published:bool,\
             auteur_id:uuid,published_at:datetime",
        );

        project.write_feature("articles", &[("model.rs", &rendered)]);
        project.compile();
    }

    #[test]
    fn the_render_ends_with_a_single_newline() {
        let rendered = entity("users", "name:string");

        assert!(
            rendered.ends_with("}\n"),
            "fin de fichier inattendue :\n{rendered}"
        );
        assert!(
            !rendered.ends_with("\n\n"),
            "ligne vide finale :\n{rendered}"
        );
    }
}
