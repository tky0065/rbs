//! Rendu de `<nom>/service.rs` : les décisions métier de la feature.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/service.rs.jinja"
));

/// Rend le service de `feature`.
pub(crate) fn rendre(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().rendre(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::{banc, champs, dto, entite, repository};

    fn service(nom: &str, fields: &str) -> String {
        let champs = champs::analyser(fields).expect("les champs du test doivent être valides");
        rendre(&Feature::nouvelle(nom, champs)).expect("le service doit se rendre")
    }

    #[test]
    fn le_service_expose_les_cinq_operations_du_crud() {
        let rendu = service("articles", "titre:string");

        for signature in [
            "pub async fn list(\n    db: &DatabaseConnection,\n    pagination: &Pagination,\n) -> Result<Page<ArticleResponse>> {",
            "pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<ArticleResponse> {",
            "pub async fn create(db: &DatabaseConnection, entree: CreateArticle) -> Result<ArticleResponse> {",
            "pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {",
        ] {
            assert!(
                rendu.contains(signature),
                "« {signature} » absente :\n{rendu}"
            );
        }
        assert!(
            rendu.contains("pub async fn update(") && rendu.contains("entree: UpdateArticle"),
            "signature d'update inattendue :\n{rendu}"
        );
    }

    #[test]
    fn aucune_requete_seaorm_n_est_construite_ici() {
        let rendu = service("articles", "titre:string,vues:int");

        assert!(
            !rendu.contains("EntityTrait"),
            "le service ne parle pas à la base :\n{rendu}"
        );
        for interdit in ["Entity::find", "Entity::delete", ".all(db)", ".one(db)"] {
            assert!(
                !rendu.contains(interdit),
                "« {interdit} » n'a rien à faire ici :\n{rendu}"
            );
        }
    }

    #[test]
    fn l_entite_n_est_atteinte_qu_a_travers_le_repository() {
        let rendu = service("articles", "titre:string");

        assert!(
            !rendu.contains("super::model"),
            "le service ne connaît que repository.rs et dto.rs :\n{rendu}"
        );
        assert!(
            rendu.contains("use super::repository::{self, ActiveModel};"),
            "l'ActiveModel doit venir du repository :\n{rendu}"
        );
    }

    #[test]
    fn l_absence_devient_une_erreur_nommee() {
        let rendu = service("blog_posts", "titre:string");

        assert_eq!(
            rendu.matches(r#"Error::NotFound("blog_post")"#).count(),
            3,
            "find, update et delete doivent chacun signaler l'absence :\n{rendu}"
        );
    }

    #[test]
    fn la_liste_assemble_la_page_du_noyau() {
        let rendu = service("articles", "titre:string");

        assert!(
            rendu.contains("let (articles, total) = repository::list(db, pagination).await?;"),
            "la liste doit venir du repository :\n{rendu}"
        );
        assert!(
            rendu.contains("Ok(Page::new("),
            "l'assemblage de la page revient au service :\n{rendu}"
        );
    }

    #[test]
    fn la_creation_pose_chaque_champ_declare() {
        let rendu = service("articles", "titre:string,resume:text:optional");

        assert!(rendu.contains("titre: Set(entree.titre),"), "{rendu}");
        assert!(rendu.contains("resume: Set(entree.resume),"), "{rendu}");
        assert!(
            rendu.contains("..Default::default()"),
            "id et horodatages sont posés par la base :\n{rendu}"
        );
    }

    #[test]
    fn la_mise_a_jour_n_ecrit_que_les_champs_recus() {
        let rendu = service("articles", "titre:string,vues:int");

        assert!(
            rendu.contains(
                "if let Some(titre) = entree.titre {\n        article.titre = Set(titre);\n    }"
            ),
            "champ obligatoire mal appliqué :\n{rendu}"
        );
        assert!(
            rendu.contains(
                "if let Some(vues) = entree.vues {\n        article.vues = Set(vues);\n    }"
            ),
            "champ obligatoire mal appliqué :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_optionnel_reste_optionnel_dans_la_colonne() {
        let rendu = service("articles", "resume:text:optional");

        assert!(
            rendu.contains("article.resume = Set(Some(resume));"),
            "la colonne est nullable : la valeur reçue doit y être réemballée :\n{rendu}"
        );
    }

    #[test]
    fn la_mise_a_jour_horodate_le_changement() {
        let rendu = service("articles", "titre:string");

        assert!(
            rendu.contains("article.updated_at = Set(chrono::Utc::now().into());"),
            "`updated_at` doit être posé par le service :\n{rendu}"
        );
    }

    #[test]
    fn une_feature_sans_champ_rend_un_service_valide() {
        let rendu = service("tokens", "");

        assert!(rendu.contains("let token = ActiveModel {"), "{rendu}");
        assert!(
            !rendu.contains("if let Some("),
            "rien à appliquer :\n{rendu}"
        );
    }

    #[test]
    fn le_rendu_traverse_rustfmt_sans_diff() {
        let rendu = service("articles", "titre:string,resume:text:optional,vues:int");

        assert_eq!(
            banc::formate(&rendu),
            rendu,
            "un `cargo fmt` chez l'utilisateur reformaterait le fichier généré"
        );
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn le_service_genere_compile_dans_un_projet_neuf() {
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
                ("dto.rs", &dto::rendre(&feature).expect("DTO rendus")),
                (
                    "repository.rs",
                    &repository::rendre(&feature).expect("repository rendu"),
                ),
                ("service.rs", &rendre(&feature).expect("service rendu")),
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
            service("articles", "titre:string,resume:text:optional,vues:int")
        );
    }
}
