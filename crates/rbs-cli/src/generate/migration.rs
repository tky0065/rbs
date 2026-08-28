//! Rendu de la migration SeaORM correspondant aux champs d'une feature.

use chrono::Utc;

use crate::template::Renderer;

use super::feature::Feature;

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/feature/migration.rs.jinja"
));

/// Une migration rendue, et le module sous lequel elle doit être écrite.
#[derive(Debug)]
pub(crate) struct Migration {
    /// Nom du module et du fichier, sans suffixe : `m20260826_143000_create_users`.
    ///
    /// `DeriveMigrationName` en tire le nom de la migration en base ; il n'est donc pas
    /// libre, et le renommer après application déclencherait une seconde exécution.
    pub module: String,
    /// Source Rust du fichier de migration.
    pub content: String,
}

/// Horodatage UTC au format qu'attend `DeriveMigrationName`.
pub(crate) fn current_timestamp() -> String {
    Utc::now().format("%Y%m%d_%H%M%S").to_string()
}

/// Rend la migration de `feature`, datée de `timestamp`.
///
/// L'horodatage est reçu et non lu de l'horloge : un rendu doit être reproductible.
pub(crate) fn render(feature: &Feature, timestamp: &str) -> Result<Migration, minijinja::Error> {
    let module = format!("m{timestamp}_create_{}", feature.module());
    let content = Renderer::new().render(TEMPLATE, feature)?;

    Ok(Migration { module, content })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{bench, fields};

    const HORODATAGE: &str = "20260826_143000";

    fn migration(name: &str, fields: &str) -> Migration {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields), HORODATAGE).expect("la migration doit se rendre")
    }

    #[test]
    fn the_module_carries_the_timestamp_and_the_table() {
        assert_eq!(
            migration("blog_posts", "title:string").module,
            "m20260826_143000_create_blog_posts"
        );
    }

    #[test]
    fn the_current_timestamp_has_the_shape_seaorm_expects() {
        let timestamp = current_timestamp();

        assert_eq!(timestamp.len(), 15, "« {timestamp} »");
        assert_eq!(&timestamp[8..9], "_", "« {timestamp} »");
        assert!(
            timestamp
                .chars()
                .enumerate()
                .all(|(rang, c)| rang == 8 || c.is_ascii_digit()),
            "« {timestamp} »"
        );
    }

    #[test]
    fn the_primary_key_carries_the_uuidv7_default() {
        let rendered = migration("users", "nom:string").content;

        assert!(
            rendered.contains(r#".default(Expr::cust("uuidv7()"))"#),
            "défaut uuidv7 absent :\n{rendered}"
        );
        assert!(rendered.contains(".primary_key()"), "{rendered}");
    }

    #[test]
    fn each_type_projects_to_its_column_method() {
        let rendered = migration(
            "samples",
            "title:string,quantity:int,price:float,active:bool,owner:uuid,\
             published_at:datetime,body:text",
        )
        .content;

        for expected in [
            "ColumnDef::new(Samples::Title).string()",
            "ColumnDef::new(Samples::Quantity).integer()",
            "ColumnDef::new(Samples::Price).double()",
            "ColumnDef::new(Samples::Active).boolean()",
            "ColumnDef::new(Samples::Owner).uuid()",
            "ColumnDef::new(Samples::PublishedAt).timestamp_with_time_zone()",
            "ColumnDef::new(Samples::Body).text()",
        ] {
            assert!(
                rendered.contains(expected),
                "« {expected} » absent de :\n{rendered}"
            );
        }
    }

    #[test]
    fn an_optional_field_is_nullable() {
        let rendered = migration("users", "bio:text:optional").content;

        assert!(
            rendered.contains("ColumnDef::new(Users::Bio).text().null()"),
            "colonne optionnelle non nullable :\n{rendered}"
        );
    }

    #[test]
    fn a_required_field_is_not_null() {
        let rendered = migration("users", "nom:string").content;

        assert!(
            rendered.contains("ColumnDef::new(Users::Nom).string().not_null()"),
            "colonne obligatoire non contrainte :\n{rendered}"
        );
    }

    #[test]
    fn a_unique_field_carries_its_constraint() {
        let rendered = migration("users", "email:string:unique").content;

        assert!(
            rendered.contains("ColumnDef::new(Users::Email).string().not_null().unique_key()"),
            "contrainte d'unicité absente :\n{rendered}"
        );
    }

    #[test]
    fn an_indexed_field_receives_its_named_index() {
        let rendered = migration("articles", "slug:string:index").content;

        assert!(
            rendered.contains(r#".name("idx_articles_slug")"#),
            "index nommé absent :\n{rendered}"
        );
        assert!(rendered.contains(".col(Articles::Slug)"), "{rendered}");
    }

    #[test]
    fn a_field_without_a_modifier_creates_no_index() {
        let rendered = migration("users", "nom:string").content;

        assert!(
            !rendered.contains("create_index"),
            "index créé sans avoir été demandé :\n{rendered}"
        );
    }

    #[test]
    fn the_timestamps_are_set_with_their_default() {
        let rendered = migration("users", "nom:string").content;

        let compact: String = rendered.split_whitespace().collect::<Vec<_>>().join(" ");

        for column in ["CreatedAt", "UpdatedAt"] {
            assert!(
                compact.contains(&format!(
                    "ColumnDef::new(Users::{column}) .timestamp_with_time_zone() .not_null() \
                     .default(Expr::current_timestamp())"
                )),
                "colonne {column} mal définie :\n{rendered}"
            );
        }
    }

    #[test]
    fn the_down_migration_drops_the_table() {
        let rendered = migration("users", "nom:string").content;

        assert!(
            rendered.contains("Table::drop().table(Users::Table)"),
            "descente absente :\n{rendered}"
        );
    }

    #[test]
    fn the_iden_enum_declares_the_table_and_all_its_columns() {
        let rendered = migration("blog_posts", "title:string,published:bool").content;

        assert!(rendered.contains("enum BlogPosts {"), "{rendered}");
        for variante in [
            "Table,",
            "Id,",
            "Title,",
            "Published,",
            "CreatedAt,",
            "UpdatedAt,",
        ] {
            assert!(
                rendered.contains(variante),
                "variante {variante} absente :\n{rendered}"
            );
        }
    }

    /// Test posé dans le projet généré : seule la base tranche ce que vaut la migration.
    const REVERSIBILITE: &str = r#"use migration::{Migrator, MigratorTrait};
use sea_orm_migration::sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};

async fn scalar(db: &DatabaseConnection, sql: &str) -> String {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .expect("la requête doit aboutir")
        .expect("une ligne était attendue")
        .try_get_by_index::<String>(0)
        .expect("colonne textuelle")
}

async fn milliseconds(db: &DatabaseConnection) -> i64 {
    scalar(db, "SELECT floor(EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::bigint::text")
        .await
        .parse()
        .expect("horloge de la base")
}

#[tokio::test]
async fn la_migration_monte_insere_et_redescend() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL doit être fournie");
    let db = Database::connect(&url).await.expect("connexion à la base");

    Migrator::up(&db, None).await.expect("montée de la migration");

    let defaut = scalar(
        &db,
        "SELECT column_default FROM information_schema.columns \
         WHERE table_name = 'articles' AND column_name = 'id'",
    )
    .await;
    assert!(defaut.contains("uuidv7()"), "défaut de la colonne id : {defaut}");

    let before = milliseconds(&db).await;
    db.execute_unprepared("INSERT INTO articles (title, slug) VALUES ('essai', 'essai')")
        .await
        .expect("insertion sans identifiant");
    let after = milliseconds(&db).await;

    let id = scalar(&db, "SELECT id::text FROM articles").await;
    assert_eq!(id.chars().nth(14), Some('7'), "version de l'UUID : {id}");

    let tete: String = id.chars().filter(|c| *c != '-').take(12).collect();
    let timestamp = i64::from_str_radix(&tete, 16).expect("tête hexadécimale de l'UUID");
    assert!(
        (before..=after).contains(&timestamp),
        "horodatage {timestamp} hors de l'intervalle d'insertion [{before}, {after}]"
    );

    Migrator::down(&db, None).await.expect("descente de la migration");

    let tables = scalar(
        &db,
        "SELECT count(*)::text FROM information_schema.tables WHERE table_name = 'articles'",
    )
    .await;
    assert_eq!(tables, "0", "la table survit à la descente");

    let suivies = scalar(&db, "SELECT count(*)::text FROM seaql_migrations").await;
    assert_eq!(suivies, "0", "la migration reste inscrite après sa descente");

    Migrator::up(&db, None).await.expect("remontée après descente");
    Migrator::down(&db, None).await.expect("seconde descente");
}
"#;

    #[test]
    #[ignore = "démarre PostgreSQL 18 en conteneur et compile la crate migration"]
    fn the_generated_migration_is_reversible_and_sets_a_uuidv7() {
        let fields = fields::parse("title:string,slug:string:unique,summary:text:optional")
            .expect("champs valides");
        let rendue = render(&Feature::fresh("articles", fields), HORODATAGE)
            .expect("la migration doit se rendre");

        let project = bench::Project::fresh();
        project.write_migration(&rendue.module, &rendue.content);
        project.write_migration_test("reversibilite", REVERSIBILITE);

        let base = bench::TestDatabase::start();
        project.test_migration(base.url());
    }

    /// `uuidv7()` tronque l'horodatage qu'il inscrit ; un cast direct en `bigint`, lui,
    /// arrondit au plus proche. Dès que la partie décimale de la borne dépasse la demie,
    /// la borne basse passe une milliseconde au-dessus de ce que l'UUID portera, et le
    /// test posé échoue — 222 fois sur 500 mesurées contre PostgreSQL 18.
    ///
    /// Encore faut-il que l'insertion tombe dans la même milliseconde que la lecture :
    /// vert sur une machine lente, rouge sur un runner. Cette garde ne dépend d'aucune
    /// horloge, et c'est tout son intérêt — elle tient là où le test qu'elle protège
    /// n'échoue qu'une fois sur deux.
    #[test]
    fn the_bounds_of_the_reversibility_test_truncate_like_uuidv7() {
        let borne = REVERSIBILITE
            .lines()
            .find(|line| line.contains("EXTRACT(EPOCH"))
            .expect("le test posé doit lire l'horloge de la base");

        assert!(
            borne.contains("floor("),
            "la borne arrondit au lieu de tronquer : elle passera au-dessus de \
             l'timestamp de uuidv7 une fois sur deux :\n{borne}"
        );
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn preview() {
        println!(
            "{}",
            migration(
                "articles",
                "title:string,slug:string:unique,summary:text:optional,views:int,\
                 published_at:datetime,auteur:uuid:index"
            )
            .content
        );
    }
}
