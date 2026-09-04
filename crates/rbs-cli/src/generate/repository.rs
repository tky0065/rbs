//! Rendu de `<name>/repository.rs` : le seul fichier qui parle à la base.

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/repository.rs.jinja"
));

/// Rend le repository de `feature`.
pub(crate) fn render(feature: &Feature) -> Result<String, minijinja::Error> {
    Renderer::new().render(TEMPLATE, feature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::{bench, fields};

    fn repository(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields)).expect("le repository doit se rendre")
    }

    /// Un seul chemin de tri : `list` est le filtre vide. Deux `order_by` en dur
    /// divergeraient au premier changement, et la liste non filtrée est celle que personne
    /// ne pense à rejouer.
    ///
    /// L'ordre déterministe qu'éprouvait `the_ordering_follows_the_descending_id` est
    /// désormais celui de `filter.rs`, où
    /// `generate::filter::the_default_order_stays_the_descending_id` le garde.
    #[test]
    fn the_list_is_the_empty_filter() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains("filter(db, &ArticleFilter::default(), pagination).await"),
            "`list` doit déléguer à `filter` :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("order_by_desc").count(),
            0,
            "le tri appartient désormais à `filter.rs` :\n{rendered}"
        );
    }

    /// Le total compte ce que le filtre retient, et non toute la table : sans cela, la
    /// dernière page d'une liste filtrée serait vide.
    #[test]
    fn the_total_counts_the_filtered_rows() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains("requete.count(db)"),
            "le total doit porter sur la requête filtrée :\n{rendered}"
        );
        assert!(
            !rendered.contains("Entity::find().count(db)"),
            "le total ne doit plus compter toute la table :\n{rendered}"
        );
    }

    #[test]
    fn the_repository_exposes_the_five_crud_operations() {
        let rendered = repository("articles", "title:string");

        for signature in [
            "pub async fn list(",
            "pub async fn find(",
            "pub async fn create(",
            "pub async fn update(",
            "pub async fn delete(",
        ] {
            assert!(
                rendered.contains(signature),
                "« {signature} » absente :\n{rendered}"
            );
        }
    }

    #[test]
    fn no_axum_import_appears() {
        let rendered = repository("articles", "title:string,views:int");

        assert!(
            !rendered.contains("axum"),
            "le repository ignore la couche HTTP :\n{rendered}"
        );
    }

    #[test]
    fn the_repository_ignores_the_dtos_and_the_rendered_pagination() {
        let rendered = repository("articles", "title:string");

        assert!(
            !rendered.contains("super::dto"),
            "le repository ne connaît que model.rs :\n{rendered}"
        );
        assert!(
            !rendered.contains("Page<"),
            "assembler la page revient au service :\n{rendered}"
        );
    }

    #[test]
    fn the_list_returns_the_page_and_its_total() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains("pub async fn list(db: &DatabaseConnection, pagination: &Pagination) -> Result<(Vec<Model>, u64)> {"),
            "signature de list inattendue :\n{rendered}"
        );
    }

    #[test]
    fn the_list_bounds_the_query_with_the_window_it_receives() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains(".offset(pagination.offset())")
                && rendered.contains(".limit(pagination.per_page())"),
            "la fenêtre de pagination n'est pas appliquée :\n{rendered}"
        );
    }

    /// Le total est un `COUNT` sur toute la table : l'attendre avant la page faisait deux
    /// allers-retours en série à chaque `GET` de collection.
    #[test]
    fn the_page_and_its_total_leave_together() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains("tokio::try_join!("),
            "les deux requêtes doivent partir ensemble :\n{rendered}"
        );
        assert!(
            !rendered.contains("let total = Entity::find().count(db).await?;"),
            "le total ne doit plus précéder la page :\n{rendered}"
        );
    }

    #[test]
    fn the_model_is_the_services_door_to_the_entity() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains("pub use super::model::{ActiveModel, Model};"),
            "le service ne pourra pas atteindre l'entité sans nommer model.rs :\n{rendered}"
        );
    }

    #[test]
    fn deletion_reports_whether_a_row_disappeared() {
        let rendered = repository("articles", "title:string");

        assert!(
            rendered.contains(
                "pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {"
            ),
            "signature de delete inattendue :\n{rendered}"
        );
        assert!(
            rendered.contains("rows_affected"),
            "la suppression doit constater son effet :\n{rendered}"
        );
    }

    /// Sans cette traduction, un second POST sur une colonne `unique` rend 500 : une
    /// faute du client servie comme une panne du serveur.
    #[test]
    fn a_unique_violation_becomes_a_conflict_rather_than_a_500() {
        let rendered = repository("articles", "email:string:unique");

        assert!(
            rendered.contains("Some(SqlErr::UniqueConstraintViolation(_)) => {"),
            "la violation d'unicité doit être reconnue :\n{rendered}"
        );
        assert!(
            rendered.contains(r#"Error::Conflict("cette valeur est déjà prise".to_owned())"#),
            "elle doit devenir un conflit :\n{rendered}"
        );
        assert!(
            rendered.contains("use sea_orm::error::SqlErr;"),
            "`SqlErr` doit être importé :\n{rendered}"
        );
    }

    /// Le gabarit ne sait pas quelle colonne a fauté : le message ne peut pas la nommer,
    /// et surtout pas recopier celui, propre à l'email, du fragment `auth`.
    #[test]
    fn the_conflict_message_stays_generic() {
        let rendered = repository("articles", "email:string:unique");

        assert!(!rendered.contains("adresse"), "{rendered}");
        assert!(!rendered.contains("inscrite"), "{rendered}");
    }

    /// Les deux écritures passent par la même porte : sans cela, `PATCH` rendrait 500 là où
    /// `POST` rend 409, pour la même contrainte.
    #[test]
    fn the_creation_and_the_update_share_the_same_translation() {
        let rendered = repository("articles", "email:string:unique");

        assert!(
            rendered.contains(".insert(db).await.map_err(conflict_on_duplicate)"),
            "la création doit traduire :\n{rendered}"
        );
        assert!(
            rendered.contains(".update(db).await.map_err(conflict_on_duplicate)"),
            "la mise à jour doit traduire :\n{rendered}"
        );
    }

    #[test]
    fn the_render_depends_only_on_the_feature_name() {
        let sans_champ = repository("articles", "");
        let avec_champs = repository("articles", "title:string,views:int,summary:text:optional");

        assert_eq!(
            sans_champ, avec_champs,
            "le CRUD est le même quels que soient les champs"
        );
    }

    /// Deux formes de ce fichier suivent le nom de l'entité, et chacune bascule à sa propre
    /// longueur : les chaînes `…insert(db).await.map_err(…)` aux 60 colonnes de
    /// `chain_width`, dès treize caractères de singulier ; les signatures de `create` et
    /// `update` aux 100 de `max_width`, dès vingt-trois. Cinq noms encadrent chacun des
    /// deux seuils — en deçà de treize, entre treize et vingt-trois où la chaîne est déjà
    /// repliée mais la signature encore entière, à vingt-trois et au-delà où les deux le
    /// sont — pour que la combinaison des deux macros du gabarit soit éprouvée sur chaque
    /// forme qu'elle peut produire.
    ///
    /// Les noms montent jusqu'à `organizational_structures`, dont le singulier fait
    /// vingt-quatre caractères. Au-delà de vingt-six pour l'entité, rustfmt éventaille les
    /// arguments de l'appel `filter(db, &…Filter::default(), pagination).await` que rend
    /// `list` — un arbitrage dont la constante ne porte le nom d'aucun réglage, que le
    /// gabarit ne devine pas et que `format::format_batch` rattrape à l'écriture.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(|name| {
            repository(name, "title:string,email:string:unique")
        });

        assert_eq!(
            divergentes,
            (27..=40).collect::<Vec<usize>>(),
            "la plage où le repository diverge de rustfmt a bougé"
        );

        // Sous soft-delete, `filter::apply(Entity::find().filter(Column::DeletedAt.is_null()),
        // filtre)?` ne dépend d'aucun nom : ses arguments valent cinquante-huit colonnes contre
        // soixante pour `fn_call_width`, une marge que le nom de l'entité, absent de cette
        // ligne, ne peut pas entamer.
        let divergentes_soft_delete = bench::longueurs_divergentes(|name| {
            let champs = fields::parse("title:string,email:string:unique").expect("champs");
            render(&Feature::fresh(name, champs).soft_deleting())
                .expect("le repository doit se rendre")
        });

        assert_eq!(
            divergentes_soft_delete,
            (27..=40).collect::<Vec<usize>>(),
            "la plage où le repository sous soft-delete diverge de rustfmt a bougé"
        );
    }

    #[test]
    fn the_delete_marks_the_row_instead_of_removing_it() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"))
            .soft_deleting();
        let rendered = render(&feature).expect("le repository doit se rendre");

        assert!(
            !rendered.contains("delete_by_id"),
            "la ligne ne doit plus partir :\n{rendered}"
        );
        assert!(
            rendered.contains("Entity::update_many()") && rendered.contains("Column::DeletedAt"),
            "la suppression doit dater la colonne :\n{rendered}"
        );
    }

    #[test]
    fn a_second_delete_still_answers_404() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"))
            .soft_deleting();
        let rendered = render(&feature).expect("le repository doit se rendre");

        assert_eq!(
            rendered.matches("Column::DeletedAt.is_null()").count(),
            3,
            "les deux lectures et le delete portent la condition ; sans elle sur le \
             delete, une seconde suppression rendrait 204 :\n{rendered}"
        );
    }

    #[test]
    fn every_read_hides_the_deleted_rows() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"))
            .soft_deleting();
        let rendered = render(&feature).expect("le repository doit se rendre");

        assert!(
            rendered.contains("Entity::find().filter(Column::DeletedAt.is_null())"),
            "`filter`, dont `list` dépend, doit écarter les lignes supprimées :\n{rendered}"
        );
        assert!(
            rendered.contains("Entity::find_by_id(id)") && rendered.contains("QueryFilter"),
            "`find` doit les écarter aussi :\n{rendered}"
        );
    }

    #[test]
    fn an_ordinary_repository_imports_only_what_it_uses() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"));
        let rendered = render(&feature).expect("le repository doit se rendre");

        assert!(rendered.contains("delete_by_id"), "témoin :\n{rendered}");
        assert!(
            !rendered.contains("QueryFilter") && !rendered.contains("ColumnTrait"),
            "un import inutilisé ferait échouer clippy sur le projet engendré :\n{rendered}"
        );
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_generated_repository_compiles_in_a_fresh_project() {
        let fields =
            fields::parse("title:string,views:int,summary:text:optional").expect("champs valides");
        let feature = Feature::fresh("articles", fields);

        let project = bench::Project::fresh();
        project.write_feature(
            "articles",
            &bench::retenus(&feature, false, &["model.rs", "filter.rs", "repository.rs"]),
        );
        project.compile();
    }
}
