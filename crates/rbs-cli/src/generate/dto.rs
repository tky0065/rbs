//! Rendu de `features/<nom>/dto.rs` : les trois formes que la feature expose en HTTP.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../templates/feature/dto.rs.jinja"
));

/// Rend les DTO de `feature`.
pub(crate) fn rendre(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().rendre(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{banc, champs, entite};

    fn dto(nom: &str, fields: &str) -> String {
        let champs = champs::analyser(fields).expect("les champs du test doivent être valides");
        rendre(&Feature::nouvelle(nom, champs)).expect("les DTO doivent se rendre")
    }

    #[test]
    fn un_champ_email_produit_une_contrainte_de_validation_d_email() {
        let rendu = dto("users", "email:string,nom:string");

        assert!(
            rendu.contains("#[validate(email)]\n    pub email: String,"),
            "contrainte d'email absente de Create :\n{rendu}"
        );
        assert!(
            rendu.contains("#[validate(email)]\n    pub email: Option<String>,"),
            "contrainte d'email absente d'Update :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_ordinaire_ne_porte_aucune_contrainte() {
        let rendu = dto("users", "nom:string");

        assert!(
            !rendu.contains("#[validate(email)]"),
            "contrainte posée à tort :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_datetime_declare_son_format_openapi() {
        let rendu = dto(
            "articles",
            "publie_le:datetime,archive_le:datetime:optional",
        );

        let creation = extraire(&rendu, "pub struct CreateArticle {");
        assert!(
            creation
                .contains("#[schema(value_type = String, format = DateTime)]\n    pub publie_le:"),
            "format OpenAPI absent sur un datetime obligatoire :\n{creation}"
        );
        assert!(
            creation.contains(
                "#[schema(value_type = Option<String>, format = DateTime)]\n    pub archive_le:"
            ),
            "format OpenAPI absent sur un datetime optionnel :\n{creation}"
        );

        let maj = extraire(&rendu, "pub struct UpdateArticle {");
        assert!(
            !maj.contains("value_type = String,"),
            "dans Update, tout champ est optionnel, le schéma aussi :\n{maj}"
        );
    }

    #[test]
    fn les_horodatages_de_la_reponse_declarent_leur_format() {
        let rendu = dto("users", "nom:string");
        let reponse = extraire(&rendu, "pub struct UserResponse {");

        assert_eq!(
            reponse
                .matches("#[schema(value_type = String, format = DateTime)]")
                .count(),
            2,
            "les deux horodatages doivent déclarer leur format :\n{reponse}"
        );
    }

    #[test]
    fn les_trois_dto_portent_le_nom_singulier_de_l_entite() {
        let rendu = dto("blog_posts", "titre:string");

        for attendu in [
            "pub struct CreateBlogPost {",
            "pub struct UpdateBlogPost {",
            "pub struct BlogPostResponse {",
        ] {
            assert!(
                rendu.contains(attendu),
                "« {attendu} » absent de :\n{rendu}"
            );
        }
    }

    #[test]
    fn le_dto_de_creation_reprend_les_champs_declares() {
        let rendu = dto("users", "nom:string,age:int,bio:text:optional");
        let creation = extraire(&rendu, "pub struct CreateUser {");

        assert!(creation.contains("pub nom: String,"), "{creation}");
        assert!(creation.contains("pub age: i32,"), "{creation}");
        assert!(creation.contains("pub bio: Option<String>,"), "{creation}");
        assert!(
            !creation.contains("pub id:"),
            "l'identifiant est posé par la base, pas par le client :\n{creation}"
        );
    }

    #[test]
    fn le_dto_de_mise_a_jour_rend_tous_ses_champs_optionnels() {
        let rendu = dto("users", "nom:string,age:int,bio:text:optional");
        let maj = extraire(&rendu, "pub struct UpdateUser {");

        assert!(maj.contains("pub nom: Option<String>,"), "{maj}");
        assert!(maj.contains("pub age: Option<i32>,"), "{maj}");
        assert!(
            maj.contains("pub bio: Option<String>,") && !maj.contains("Option<Option<"),
            "un champ déjà optionnel ne se double pas :\n{maj}"
        );
    }

    #[test]
    fn le_dto_de_reponse_ajoute_l_identifiant_et_les_horodatages() {
        let rendu = dto("users", "nom:string");
        let reponse = extraire(&rendu, "pub struct UserResponse {");

        assert!(reponse.contains("pub id: Uuid,"), "{reponse}");
        assert!(reponse.contains("pub nom: String,"), "{reponse}");
        assert!(
            reponse.contains("pub created_at: DateTimeWithTimeZone,"),
            "{reponse}"
        );
        assert!(
            reponse.contains("pub updated_at: DateTimeWithTimeZone,"),
            "{reponse}"
        );
    }

    #[test]
    fn la_reponse_se_construit_depuis_l_entite() {
        let rendu = dto("users", "nom:string");

        assert!(
            rendu.contains("impl From<Model> for UserResponse {"),
            "conversion depuis l'entité absente :\n{rendu}"
        );
    }

    #[test]
    fn les_dto_entrants_derivent_la_deserialisation_et_la_validation() {
        let rendu = dto("users", "nom:string");

        assert_eq!(
            rendu
                .matches("#[derive(Debug, Deserialize, ToSchema, Validate)]")
                .count(),
            2,
            "les deux DTO entrants doivent dériver Deserialize, ToSchema et Validate :\n{rendu}"
        );
        assert!(
            rendu.contains("#[derive(Debug, Serialize, ToSchema)]"),
            "le DTO sortant doit dériver Serialize et ToSchema :\n{rendu}"
        );
    }

    #[test]
    fn une_feature_sans_champ_rend_trois_dto_valides() {
        let rendu = dto("tokens", "");

        assert!(rendu.contains("pub struct CreateToken {}"), "{rendu}");
        assert!(rendu.contains("pub struct UpdateToken {}"), "{rendu}");
        assert!(rendu.contains("pub id: Uuid,"), "{rendu}");
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn les_dto_generes_compilent_dans_un_projet_neuf() {
        let fields = "titre:string,email:string:unique,resume:text:optional,vues:int,\
                      publie:bool,auteur_id:uuid,publie_le:datetime";
        let champs = champs::analyser(fields).expect("champs valides");
        let feature = Feature::nouvelle("articles", champs);

        let projet = banc::Projet::neuf();
        projet.poser_feature(
            "articles",
            &[
                (
                    "model.rs",
                    &entite::rendre(&feature).expect("entité rendue"),
                ),
                ("dto.rs", &rendre(&feature).expect("DTO rendus")),
            ],
        );
        projet.compiler();
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn apercu() {
        println!(
            "{}",
            dto(
                "articles",
                "titre:string,email:string,resume:text:optional,vues:int"
            )
        );
    }

    /// Isole une struct du rendu, de son en-tête à son accolade fermante.
    fn extraire<'a>(rendu: &'a str, entete: &str) -> &'a str {
        let debut = rendu
            .find(entete)
            .unwrap_or_else(|| panic!("« {entete} » absent :\n{rendu}"));
        let reste = &rendu[debut..];
        let fin = reste.find("\n}").map_or(reste.len(), |offset| offset + 2);

        &reste[..fin]
    }
}
