//! Rendu de `<nom>/repository.rs` : le seul fichier qui parle à la base.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/repository.rs.jinja"
));

/// Rend le repository de `feature`.
pub(crate) fn rendre(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().rendre(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::{banc, champs, entite};

    fn repository(nom: &str, fields: &str) -> String {
        let champs = champs::analyser(fields).expect("les champs du test doivent être valides");
        rendre(&Feature::nouvelle(nom, champs)).expect("le repository doit se rendre")
    }

    #[test]
    fn le_repository_expose_les_cinq_operations_du_crud() {
        let rendu = repository("articles", "titre:string");

        for signature in [
            "pub async fn list(",
            "pub async fn find(",
            "pub async fn create(",
            "pub async fn update(",
            "pub async fn delete(",
        ] {
            assert!(
                rendu.contains(signature),
                "« {signature} » absente :\n{rendu}"
            );
        }
    }

    #[test]
    fn aucun_import_d_axum_n_apparait() {
        let rendu = repository("articles", "titre:string,vues:int");

        assert!(
            !rendu.contains("axum"),
            "le repository ignore la couche HTTP :\n{rendu}"
        );
    }

    #[test]
    fn le_repository_ignore_les_dto_et_la_pagination_rendue() {
        let rendu = repository("articles", "titre:string");

        assert!(
            !rendu.contains("super::dto"),
            "le repository ne connaît que model.rs :\n{rendu}"
        );
        assert!(
            !rendu.contains("Page<"),
            "assembler la page revient au service :\n{rendu}"
        );
    }

    #[test]
    fn la_liste_rend_la_page_et_son_total() {
        let rendu = repository("articles", "titre:string");

        assert!(
            rendu.contains("pub async fn list(db: &DatabaseConnection, pagination: &Pagination) -> Result<(Vec<Model>, u64)> {"),
            "signature de list inattendue :\n{rendu}"
        );
    }

    #[test]
    fn la_liste_borne_la_requete_avec_la_fenetre_recue() {
        let rendu = repository("articles", "titre:string");

        assert!(
            rendu.contains(".offset(pagination.offset())")
                && rendu.contains(".limit(pagination.per_page())"),
            "la fenêtre de pagination n'est pas appliquée :\n{rendu}"
        );
    }

    #[test]
    fn le_tri_suit_l_identifiant_decroissant() {
        let rendu = repository("articles", "titre:string");

        assert!(
            rendu.contains(".order_by_desc(Column::Id)"),
            "l'ordre de la liste n'est pas déterministe :\n{rendu}"
        );
    }

    #[test]
    fn le_modele_est_la_porte_du_service_vers_l_entite() {
        let rendu = repository("articles", "titre:string");

        assert!(
            rendu.contains("pub use super::model::{ActiveModel, Model};"),
            "le service ne pourra pas atteindre l'entité sans nommer model.rs :\n{rendu}"
        );
    }

    #[test]
    fn la_suppression_rapporte_si_une_ligne_a_disparu() {
        let rendu = repository("articles", "titre:string");

        assert!(
            rendu.contains(
                "pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {"
            ),
            "signature de delete inattendue :\n{rendu}"
        );
        assert!(
            rendu.contains("rows_affected"),
            "la suppression doit constater son effet :\n{rendu}"
        );
    }

    #[test]
    fn le_rendu_ne_depend_que_du_nom_de_la_feature() {
        let sans_champ = repository("articles", "");
        let avec_champs = repository("articles", "titre:string,vues:int,resume:text:optional");

        assert_eq!(
            sans_champ, avec_champs,
            "le CRUD est le même quels que soient les champs"
        );
    }

    #[test]
    fn le_rendu_traverse_rustfmt_sans_diff() {
        let rendu = repository("articles", "titre:string");

        assert_eq!(
            banc::formate(&rendu),
            rendu,
            "un `cargo fmt` chez l'utilisateur reformaterait le fichier généré"
        );
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn le_repository_genere_compile_dans_un_projet_neuf() {
        let champs =
            champs::analyser("titre:string,vues:int,resume:text:optional").expect("champs valides");
        let feature = Feature::nouvelle("articles", champs);

        let projet = banc::Projet::neuf();
        projet.poser_feature(
            "articles",
            &[
                (
                    "model.rs",
                    &entite::rendre(&feature).expect("entité rendue"),
                ),
                (
                    "repository.rs",
                    &rendre(&feature).expect("repository rendu"),
                ),
            ],
        );
        projet.compiler();
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn apercu() {
        println!("{}", repository("articles", "titre:string,vues:int"));
    }
}
