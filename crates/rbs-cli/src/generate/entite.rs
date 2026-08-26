//! Rendu de `features/<nom>/model.rs` : l'entité SeaORM d'une feature.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../templates/feature/model.rs.jinja"
));

/// Rend l'entité SeaORM de `feature`.
pub(crate) fn rendre(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().rendre(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{banc, champs};

    fn entite(nom: &str, fields: &str) -> String {
        let champs = champs::analyser(fields).expect("les champs du test doivent être valides");
        rendre(&Feature::nouvelle(nom, champs)).expect("l'entité doit se rendre")
    }

    #[test]
    fn la_cle_primaire_est_un_uuid_sans_auto_increment() {
        let rendu = entite("users", "name:string");

        assert!(
            rendu.contains("#[sea_orm(primary_key, auto_increment = false)]\n    pub id: Uuid,"),
            "clé primaire attendue en Uuid non auto-incrémenté :\n{rendu}"
        );
    }

    #[test]
    fn la_table_porte_le_nom_pluriel_de_la_feature() {
        let rendu = entite("blog_posts", "title:string");

        assert!(
            rendu.contains(r#"#[sea_orm(table_name = "blog_posts")]"#),
            "nom de table absent :\n{rendu}"
        );
    }

    #[test]
    fn chaque_type_de_la_grammaire_se_projette_dans_l_entite() {
        let rendu = entite(
            "echantillons",
            "titre:string,quantite:int,prix:float,actif:bool,proprietaire:uuid,\
             publie_le:datetime,corps:text",
        );

        for attendu in [
            "pub titre: String,",
            "pub quantite: i32,",
            "pub prix: f64,",
            "pub actif: bool,",
            "pub proprietaire: Uuid,",
            "pub publie_le: DateTimeWithTimeZone,",
            "pub corps: String,",
        ] {
            assert!(rendu.contains(attendu), "« {attendu} » absent de :\n{rendu}");
        }
    }

    #[test]
    fn un_champ_optionnel_devient_une_option() {
        let rendu = entite("users", "bio:string:optional");

        assert!(
            rendu.contains("pub bio: Option<String>,"),
            "champ optionnel non rendu en Option :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_unique_porte_l_attribut_correspondant() {
        let rendu = entite("users", "email:string:unique");

        assert!(
            rendu.contains("#[sea_orm(unique)]\n    pub email: String,"),
            "attribut unique absent :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_indexe_porte_l_attribut_correspondant() {
        let rendu = entite("users", "slug:string:index");

        assert!(
            rendu.contains("#[sea_orm(indexed)]\n    pub slug: String,"),
            "attribut indexed absent :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_text_force_son_type_de_colonne() {
        let rendu = entite("articles", "corps:text");

        assert!(
            rendu.contains(r#"#[sea_orm(column_type = "Text")]"#),
            "type de colonne Text non forcé :\n{rendu}"
        );
    }

    #[test]
    fn les_modificateurs_cumules_tiennent_dans_un_seul_attribut() {
        let rendu = entite("articles", "resume:text:index");

        assert!(
            rendu.contains(r#"#[sea_orm(column_type = "Text", indexed)]"#),
            "modificateurs non cumulés :\n{rendu}"
        );
    }

    #[test]
    fn les_horodatages_sont_poses_sans_avoir_ete_declares() {
        let rendu = entite("users", "name:string");

        assert!(rendu.contains("pub created_at: DateTimeWithTimeZone,"), "{rendu}");
        assert!(rendu.contains("pub updated_at: DateTimeWithTimeZone,"), "{rendu}");
    }

    #[test]
    fn une_feature_sans_champ_rend_une_entite_complete() {
        let rendu = entite("tokens", "");

        assert!(rendu.contains("pub struct Model {"), "{rendu}");
        assert!(rendu.contains("pub enum Relation {}"), "{rendu}");
        assert!(rendu.contains("impl ActiveModelBehavior for ActiveModel {}"), "{rendu}");
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn l_entite_generee_compile_dans_un_projet_neuf() {
        let projet = banc::Projet::neuf();
        let rendu = entite(
            "articles",
            "titre:string,slug:string:unique,resume:text:optional,vues:int,publie:bool,\
             auteur_id:uuid,publie_le:datetime",
        );

        projet.poser_feature("articles", &[("model.rs", &rendu)]);
        projet.compiler();
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn apercu() {
        println!(
            "{}",
            entite(
                "articles",
                "titre:string,slug:string:unique,resume:text:optional,vues:int,publie:bool"
            )
        );
    }

    #[test]
    fn le_rendu_se_termine_par_un_retour_a_la_ligne_unique() {
        let rendu = entite("users", "name:string");

        assert!(rendu.ends_with("}\n"), "fin de fichier inattendue :\n{rendu}");
        assert!(!rendu.ends_with("\n\n"), "ligne vide finale :\n{rendu}");
    }
}
