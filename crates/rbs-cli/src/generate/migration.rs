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

    /// Le rendu débarrassé de ses blancs.
    ///
    /// Une colonne dont les arguments franchissent les soixante colonnes de
    /// `fn_call_width` est écrite éclatée : ce qui s'y vérifie est la projection du type
    /// sur sa méthode, et non la ligne où elle tombe.
    fn sans_blancs(rendu: &str) -> String {
        rendu.split_whitespace().collect()
    }

    /// Deux régimes se croisent dans ce fichier, tous deux régis par les soixante colonnes
    /// de `fn_call_width` : la colonne `Id`, que rustfmt compacte tant que ses arguments
    /// tiennent, et les colonnes de champs, qu'il éclate dès qu'elles débordent. Le gabarit
    /// écrivait chacune dans un seul de ces deux régimes.
    ///
    /// `CreatedAt` et `UpdatedAt` n'y figurent pas : leurs arguments valent quatre-vingt-dix-
    /// neuf caractères de plus que l'iden, donc elles restent éclatées à toute longueur.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        let divergentes =
            bench::longueurs_divergentes(|name| migration(name, "title:string,views:int").content);

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu de la migration diverge de rustfmt à ces longueurs de nom"
        );
    }

    fn users_entity() -> Vec<crate::generate::entities::Entity> {
        vec![crate::generate::entities::Entity {
            table: "users".to_string(),
            module_path: "crate::auth::model::user".to_string(),
            file: "src/auth/model.rs".to_string(),
        }]
    }

    fn migration_with(
        name: &str,
        fields: &str,
        entities: &[crate::generate::entities::Entity],
    ) -> Migration {
        let mut parsed = fields::parse(fields).expect("les champs du test doivent être valides");
        crate::generate::relations::resolve(&mut parsed, entities, name)
            .expect("les cibles du test doivent se résoudre");
        render(&Feature::fresh(name, parsed), HORODATAGE).expect("la migration doit se rendre")
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

    // Le défaut est parti dans le modèle : `uuidv7()` n'a d'équivalent à écrire ni en
    // MySQL ni en SQLite, et la migration doit se rendre pour les trois moteurs.
    #[test]
    fn the_primary_key_leaves_its_default_to_the_application() {
        let rendered = migration("users", "nom:string").content;

        assert!(
            !rendered.contains("uuidv7"),
            "la migration pose encore un défaut de base :\n{rendered}"
        );
        assert!(rendered.contains(".primary_key()"), "{rendered}");
        assert!(rendered.contains(".uuid()"), "{rendered}");
    }

    #[test]
    fn each_type_projects_to_its_column_method() {
        let rendered = migration(
            "samples",
            "title:string,quantity:int,price:float,active:bool,owner:uuid,\
             published_at:datetime,body:text",
        )
        .content;

        let compact = sans_blancs(&rendered);

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
                compact.contains(expected),
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
            sans_blancs(&rendered)
                .contains("ColumnDef::new(Users::Email).string().not_null().unique_key()"),
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

    #[test]
    fn a_reference_creates_its_foreign_key_named_after_table_and_column() {
        let rendered = migration_with("posts", "author:references:users", &users_entity()).content;

        assert!(
            rendered.contains(r#".name("fk_posts_author_id")"#),
            "{rendered}"
        );
        assert!(
            rendered.contains(".from(Posts::Table, Posts::AuthorId)"),
            "{rendered}"
        );
        assert!(
            rendered.contains(".to(Users::Table, Users::Id)"),
            "{rendered}"
        );
        assert!(
            rendered.contains(".on_delete(ForeignKeyAction::Restrict)"),
            "{rendered}"
        );
    }

    #[test]
    fn the_referencing_column_is_a_not_null_uuid() {
        let rendered = migration_with("posts", "author:references:users", &users_entity()).content;

        assert!(
            rendered.contains("ColumnDef::new(Posts::AuthorId).uuid().not_null()"),
            "{rendered}"
        );
    }

    #[test]
    fn an_optional_reference_is_nullable_and_can_be_set_null() {
        let rendered = migration_with(
            "posts",
            "author:references:users:optional:nullify",
            &users_entity(),
        )
        .content;

        assert!(
            rendered.contains("ColumnDef::new(Posts::AuthorId).uuid().null()"),
            "{rendered}"
        );
        assert!(
            rendered.contains(".on_delete(ForeignKeyAction::SetNull)"),
            "{rendered}"
        );
    }

    // Sans index, la vérification de la contrainte au `DELETE` de la cible parcourt la
    // table portante en entier.
    #[test]
    fn a_reference_gets_its_index() {
        let rendered = migration_with("posts", "author:references:users", &users_entity()).content;

        assert!(
            rendered.contains(r#".name("idx_posts_author_id")"#),
            "{rendered}"
        );
    }

    #[test]
    fn the_target_iden_is_declared_once_even_for_two_relations_to_the_same_table() {
        let rendered = migration_with(
            "posts",
            "author:references:users,reviewer:references:users",
            &users_entity(),
        )
        .content;

        assert_eq!(
            rendered.matches("enum Users {").count(),
            1,
            "l'identifiant de la table cible est déclaré deux fois :\n{rendered}"
        );
        assert!(
            rendered.contains("enum Users {\n    Table,\n    Id,\n}"),
            "{rendered}"
        );
    }

    #[test]
    fn a_self_reference_does_not_redeclare_its_own_iden() {
        let rendered = migration_with("posts", "parent:references:posts:optional", &[]).content;

        assert_eq!(rendered.matches("enum Posts {").count(), 1, "{rendered}");
        assert!(
            rendered.contains(".to(Posts::Table, Posts::Id)"),
            "{rendered}"
        );
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

#[tokio::test]
async fn la_migration_monte_insere_et_redescend() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL doit être fournie");
    let db = Database::connect(&url).await.expect("connexion à la base");

    Migrator::up(&db, None).await.expect("montée de la migration");

    // La colonne ne porte aucun défaut : l'identifiant vient du modèle, qui le pose pour
    // les trois moteurs. Un défaut qui reparaîtrait ici rendrait la migration injouable
    // partout ailleurs que sur PostgreSQL.
    let defauts = scalar(
        &db,
        "SELECT count(*)::text FROM information_schema.columns \
         WHERE table_name = 'articles' AND column_name = 'id' \
         AND column_default IS NOT NULL",
    )
    .await;
    assert_eq!(defauts, "0", "la colonne id porte encore un défaut de base");

    db.execute_unprepared(
        "INSERT INTO articles (id, title, slug) \
         VALUES ('0199c0de-0000-7000-8000-000000000001', 'essai', 'essai')",
    )
    .await
    .expect("insertion avec l'identifiant que le modèle poserait");

    let id = scalar(&db, "SELECT id::text FROM articles").await;
    assert_eq!(id.chars().nth(14), Some('7'), "version de l'UUID : {id}");

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
    #[ignore = "démarre PostgreSQL en conteneur et compile la crate migration"]
    fn the_generated_migration_is_reversible_without_a_column_default() {
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

    /// Le défaut de colonne parti, la migration doit se rendre pour les trois moteurs.
    ///
    /// La garde est ici plutôt que dans le seul test sous conteneur : celui-ci ne tourne
    /// que contre PostgreSQL, où un `uuidv7()` réintroduit passerait au vert.
    #[test]
    fn the_posted_reversibility_test_expects_no_column_default() {
        assert!(
            !REVERSIBILITE.contains("uuidv7"),
            "le test posé attend encore un défaut de base :\n{REVERSIBILITE}"
        );
        assert!(
            REVERSIBILITE.contains("column_default IS NOT NULL"),
            "le test posé ne vérifie plus l'absence de défaut :\n{REVERSIBILITE}"
        );
    }

    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_generated_migration_compiles_with_its_foreign_key() {
        let project = bench::Project::fresh();

        // Le fichier de migration redéclare localement l'énumération `Users` — la
        // contrainte n'est donc pas un lien de compilation vers le fichier de sa
        // cible, et l'ordre d'écriture ci-dessous n'a aucune incidence sur ce que ce
        // test prouve. `users` est écrite quand même : elle donne un décor réaliste,
        // dont un futur test d'exécution (qui applique réellement les migrations,
        // dans l'ordre, contre une base) aura besoin.
        //
        // Ce que ce test prouve : la chaîne `.foreign_key(ForeignKey::create()…)`,
        // son `ForeignKeyAction` et la déclaration du `DeriveIden` cible passent le
        // typage de `sea_orm_migration`, et le fichier engendré est du Rust que
        // `rustc` accepte. Ce qu'il ne prouve pas : que les migrations s'appliquent
        // dans le bon ordre — une propriété d'exécution, qui ne s'éprouve que contre
        // une vraie base.
        let users = migration("users", "email:string:unique");
        project.write_migration(&users.module, &users.content);

        let posts = migration_with(
            "posts",
            "title:string,author:references:users",
            &users_entity(),
        );
        project.write_migration(&posts.module, &posts.content);

        project.compile();
    }

    #[test]
    fn soft_delete_creates_the_column_and_its_index() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"))
            .soft_deleting();
        let rendered = render(&feature, "20260902_000000")
            .expect("la migration doit se rendre")
            .content;

        assert!(
            rendered.contains("ColumnDef::new(Articles::DeletedAt)"),
            "la colonne manque :\n{rendered}"
        );
        assert!(
            rendered.contains("idx_articles_deleted_at"),
            "toute lecture filtre sur cette colonne, elle doit être indexée :\n{rendered}"
        );
        assert!(
            rendered.contains("    DeletedAt,"),
            "l'enum DeriveIden doit porter la variante :\n{rendered}"
        );
    }

    #[test]
    fn the_unique_constraint_moves_to_a_partial_index() {
        let feature = Feature::fresh(
            "articles",
            fields::parse("title:string:unique").expect("champs"),
        )
        .soft_deleting();
        let rendered = render(&feature, "20260902_000000")
            .expect("la migration doit se rendre")
            .content;

        assert!(
            !rendered.contains(".unique_key()"),
            "la contrainte de colonne serait inconditionnelle, l'index partiel n'y \
             changerait rien :\n{rendered}"
        );
        assert!(
            rendered.contains("uq_articles_title") && rendered.contains("Articles::DeletedAt)"),
            "l'unicité doit passer par un index restreint aux lignes vivantes :\n{rendered}"
        );
    }

    #[test]
    fn mysql_keeps_a_global_uniqueness() {
        let feature = Feature::fresh(
            "articles",
            fields::parse("title:string:unique").expect("champs"),
        )
        .soft_deleting();
        let rendered = render(&feature, "20260902_000000")
            .expect("la migration doit se rendre")
            .content;

        assert!(
            rendered.contains("sea_orm::DbBackend::MySql"),
            "MySQL n'a pas d'index partiel : la migration doit brancher, faute de quoi \
             elle ne s'y applique pas du tout :\n{rendered}"
        );
    }

    #[test]
    fn an_ordinary_migration_keeps_its_unique_key() {
        let feature = Feature::fresh(
            "articles",
            fields::parse("title:string:unique").expect("champs"),
        );
        let rendered = render(&feature, "20260902_000000")
            .expect("la migration doit se rendre")
            .content;

        assert!(rendered.contains(".unique_key()"), "témoin :\n{rendered}");
        assert!(!rendered.contains("deleted_at"), "témoin :\n{rendered}");
    }
}
