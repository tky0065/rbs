//! Rendu de `<name>/dto.rs` : les trois formes que la feature expose en HTTP.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/dto.rs.jinja"
));

/// Rend les DTO de `feature`.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().render(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{bench, entity, fields};

    fn dto(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields)).expect("les DTO doivent se rendre")
    }

    /// Les contraintes d'un champ tiennent en un seul attribut : `validator` n'en lit
    /// qu'un par champ, et deux attributs empilés engendreraient un projet qui ne
    /// compile pas.
    #[test]
    fn an_email_field_produces_an_email_validation_constraint() {
        let rendered = dto("users", "email:string,nom:string");

        assert!(
            rendered.contains("#[validate(email, length(max = 255))]\n    pub email: String,"),
            "contrainte d'email absente de Create :\n{rendered}"
        );
        assert!(
            rendered
                .contains("#[validate(email, length(max = 255))]\n    pub email: Option<String>,"),
            "contrainte d'email absente d'Update :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("#[validate(").count(),
            4,
            "un champ porte plus d'un attribut de validation :\n{rendered}"
        );
    }

    /// La borne par défaut : sans elle, chaque route publique accepte une chaîne de
    /// longueur arbitraire, que la colonne n'arrête pas non plus.
    #[test]
    fn a_string_field_is_bounded_in_both_dtos() {
        let rendered = dto("users", "nom:string");

        assert!(
            rendered.contains("#[validate(length(max = 255))]\n    pub nom: String,"),
            "borne absente de Create :\n{rendered}"
        );
        assert!(
            rendered.contains("#[validate(length(max = 255))]\n    pub nom: Option<String>,"),
            "borne absente d'Update :\n{rendered}"
        );
    }

    #[test]
    fn the_max_modifier_is_carried_into_the_dtos() {
        let rendered = dto("articles", "titre:string:max=200,resume:text:max=5000");

        assert!(
            rendered.contains("#[validate(length(max = 200))]\n    pub titre: String,"),
            "borne écrite non reprise :\n{rendered}"
        );
        assert!(
            rendered.contains("#[validate(length(max = 5000))]\n    pub resume: String,"),
            "borne écrite non reprise sur un text :\n{rendered}"
        );
    }

    /// `text` porte du texte long : c'est le type qu'on choisit pour dépasser la borne,
    /// et il ne la reçoit que si on la lui écrit.
    #[test]
    fn a_field_that_bounds_nothing_carries_no_constraint() {
        let rendered = dto("articles", "resume:text,vues:int,publie:bool");

        assert!(
            !rendered.contains("#[validate("),
            "contrainte posée à tort :\n{rendered}"
        );
    }

    #[test]
    fn a_datetime_field_declares_its_openapi_format() {
        let rendered = dto(
            "articles",
            "published_at:datetime,archive_le:datetime:optional",
        );

        let creation = extract(&rendered, "pub struct CreateArticle {");
        assert!(
            creation.contains(
                "#[schema(value_type = String, format = DateTime)]\n    pub published_at:"
            ),
            "format OpenAPI absent sur un datetime obligatoire :\n{creation}"
        );
        assert!(
            creation.contains(
                "#[schema(value_type = Option<String>, format = DateTime)]\n    pub archive_le:"
            ),
            "format OpenAPI absent sur un datetime optionnel :\n{creation}"
        );

        let maj = extract(&rendered, "pub struct UpdateArticle {");
        assert!(
            !maj.contains("value_type = String,"),
            "dans Update, tout champ est optionnel, le schéma aussi :\n{maj}"
        );
    }

    #[test]
    fn the_response_timestamps_declare_their_format() {
        let rendered = dto("users", "nom:string");
        let response = extract(&rendered, "pub struct UserResponse {");

        assert_eq!(
            response
                .matches("#[schema(value_type = String, format = DateTime)]")
                .count(),
            2,
            "les deux horodatages doivent déclarer leur format :\n{response}"
        );
    }

    #[test]
    fn the_three_dtos_carry_the_singular_name_of_the_entity() {
        let rendered = dto("blog_posts", "title:string");

        for expected in [
            "pub struct CreateBlogPost {",
            "pub struct UpdateBlogPost {",
            "pub struct BlogPostResponse {",
        ] {
            assert!(
                rendered.contains(expected),
                "« {expected} » absent de :\n{rendered}"
            );
        }
    }

    #[test]
    fn the_creation_dto_takes_the_declared_fields() {
        let rendered = dto("users", "nom:string,age:int,bio:text:optional");
        let creation = extract(&rendered, "pub struct CreateUser {");

        assert!(creation.contains("pub nom: String,"), "{creation}");
        assert!(creation.contains("pub age: i32,"), "{creation}");
        assert!(creation.contains("pub bio: Option<String>,"), "{creation}");
        assert!(
            !creation.contains("pub id:"),
            "l'identifiant est posé par la base, pas par le client :\n{creation}"
        );
    }

    #[test]
    fn the_update_dto_makes_all_its_fields_optional() {
        let rendered = dto("users", "nom:string,age:int,bio:text:optional");
        let maj = extract(&rendered, "pub struct UpdateUser {");

        assert!(maj.contains("pub nom: Option<String>,"), "{maj}");
        assert!(maj.contains("pub age: Option<i32>,"), "{maj}");
        assert!(
            maj.contains("pub bio: Option<String>,") && !maj.contains("Option<Option<"),
            "un champ déjà optionnel ne se double pas :\n{maj}"
        );
    }

    #[test]
    fn the_response_dto_adds_the_id_and_the_timestamps() {
        let rendered = dto("users", "nom:string");
        let response = extract(&rendered, "pub struct UserResponse {");

        assert!(response.contains("pub id: Uuid,"), "{response}");
        assert!(response.contains("pub nom: String,"), "{response}");
        assert!(
            response.contains("pub created_at: DateTimeWithTimeZone,"),
            "{response}"
        );
        assert!(
            response.contains("pub updated_at: DateTimeWithTimeZone,"),
            "{response}"
        );
    }

    #[test]
    fn the_response_is_built_from_the_entity() {
        let rendered = dto("users", "nom:string");

        assert!(
            rendered.contains("impl From<Model> for UserResponse {"),
            "conversion depuis l'entité absente :\n{rendered}"
        );
    }

    #[test]
    fn incoming_dtos_derive_deserialisation_and_validation() {
        let rendered = dto("users", "nom:string");

        assert_eq!(
            rendered
                .matches("#[derive(Debug, Deserialize, ToSchema, Validate)]")
                .count(),
            2,
            "les deux DTO entrants doivent dériver Deserialize, ToSchema et Validate :\n{rendered}"
        );
        assert!(
            rendered.contains("#[derive(Debug, Serialize, ToSchema)]"),
            "le DTO sortant doit dériver Serialize et ToSchema :\n{rendered}"
        );
    }

    #[test]
    fn a_field_less_feature_renders_three_valid_dtos() {
        let rendered = dto("tokens", "");

        assert!(rendered.contains("pub struct CreateToken {}"), "{rendered}");
        assert!(rendered.contains("pub struct UpdateToken {}"), "{rendered}");
        assert!(rendered.contains("pub id: Uuid,"), "{rendered}");
    }

    /// Aucune ligne de ce fichier ne suit le nom de l'entité d'assez près pour franchir un
    /// seuil de rustfmt : la plus longue, `impl From<Model> for …Response {`, vaut trente
    /// et un caractères de plus que lui, et le balayage de deux à quarante ne trouve pas
    /// une divergence. Les quatre noms n'y cherchent donc pas un seuil — ils tiennent la
    /// propriété pour la prochaine retouche du gabarit.
    ///
    /// L'axe qui, lui, finit par bouger est le nom d'un champ, pas celui de l'entité : à
    /// quarante caractères, `<champ>: model.<champ>,` atteint 101 colonnes dans
    /// `From<Model>`. `format::format_batch` le rattrape à l'écriture.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(|name| {
            dto(
                name,
                "title:string,summary:text:optional,published_at:datetime",
            )
        });

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu du DTO diverge de rustfmt à ces longueurs de nom"
        );
    }

    /// L'axe qui finit par bouger n'est pas le nom de l'entité mais celui d'un champ :
    /// dans `impl From<Model>`, `<champ>: model.<champ>,` croît deux fois plus vite que
    /// lui. Aucun garde ne balayait cet axe-là.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write_whatever_the_field_length() {
        let divergentes =
            bench::longueurs_divergentes(|champ| dto("article", &format!("{champ}:string")));

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu du DTO diverge de rustfmt à ces longueurs de champ"
        );
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_generated_dtos_compile_in_a_fresh_project() {
        let fields = "title:string,email:string:unique,summary:text:optional,views:int,\
                      published:bool,auteur_id:uuid,published_at:datetime";
        let fields = fields::parse(fields).expect("champs valides");
        let feature = Feature::fresh("articles", fields);

        let project = bench::Project::fresh();
        project.write_feature(
            "articles",
            &[
                (
                    "model.rs",
                    &entity::render(&feature).expect("entité rendue"),
                ),
                ("dto.rs", &render(&feature).expect("DTO rendus")),
            ],
        );
        project.compile();
    }

    /// Isole une struct du rendu, de son en-tête à son accolade fermante.
    fn extract<'a>(rendered: &'a str, entete: &str) -> &'a str {
        let debut = rendered
            .find(entete)
            .unwrap_or_else(|| panic!("« {entete} » absent :\n{rendered}"));
        let reste = &rendered[debut..];
        let fin = reste.find("\n}").map_or(reste.len(), |offset| offset + 2);

        &reste[..fin]
    }
}
