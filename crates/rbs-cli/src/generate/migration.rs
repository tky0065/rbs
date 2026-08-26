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
    pub contenu: String,
}

/// Horodatage UTC au format qu'attend `DeriveMigrationName`.
pub(crate) fn horodatage_courant() -> String {
    Utc::now().format("%Y%m%d_%H%M%S").to_string()
}

/// Rend la migration de `feature`, datée de `horodatage`.
///
/// L'horodatage est reçu et non lu de l'horloge : un rendu doit être reproductible.
pub(crate) fn rendre(feature: &Feature, horodatage: &str) -> Result<Migration, minijinja::Error> {
    let module = format!("m{horodatage}_create_{}", feature.module());
    let contenu = Renderer::new().rendre(TEMPLATE, feature)?;

    Ok(Migration { module, contenu })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::{banc, champs};

    const HORODATAGE: &str = "20260826_143000";

    fn migration(nom: &str, fields: &str) -> Migration {
        let champs = champs::analyser(fields).expect("les champs du test doivent être valides");
        rendre(&Feature::nouvelle(nom, champs), HORODATAGE).expect("la migration doit se rendre")
    }

    #[test]
    fn le_module_porte_l_horodatage_et_la_table() {
        assert_eq!(
            migration("blog_posts", "titre:string").module,
            "m20260826_143000_create_blog_posts"
        );
    }

    #[test]
    fn l_horodatage_courant_a_la_forme_attendue_par_seaorm() {
        let horodatage = horodatage_courant();

        assert_eq!(horodatage.len(), 15, "« {horodatage} »");
        assert_eq!(&horodatage[8..9], "_", "« {horodatage} »");
        assert!(
            horodatage
                .chars()
                .enumerate()
                .all(|(rang, c)| rang == 8 || c.is_ascii_digit()),
            "« {horodatage} »"
        );
    }

    #[test]
    fn la_cle_primaire_porte_le_defaut_uuidv7() {
        let rendu = migration("users", "nom:string").contenu;

        assert!(
            rendu.contains(r#".default(Expr::cust("uuidv7()"))"#),
            "défaut uuidv7 absent :\n{rendu}"
        );
        assert!(rendu.contains(".primary_key()"), "{rendu}");
    }

    #[test]
    fn chaque_type_se_projette_vers_sa_methode_de_colonne() {
        let rendu = migration(
            "echantillons",
            "titre:string,quantite:int,prix:float,actif:bool,proprietaire:uuid,\
             publie_le:datetime,corps:text",
        )
        .contenu;

        for attendu in [
            "ColumnDef::new(Echantillons::Titre).string()",
            "ColumnDef::new(Echantillons::Quantite).integer()",
            "ColumnDef::new(Echantillons::Prix).double()",
            "ColumnDef::new(Echantillons::Actif).boolean()",
            "ColumnDef::new(Echantillons::Proprietaire).uuid()",
            "ColumnDef::new(Echantillons::PublieLe).timestamp_with_time_zone()",
            "ColumnDef::new(Echantillons::Corps).text()",
        ] {
            assert!(
                rendu.contains(attendu),
                "« {attendu} » absent de :\n{rendu}"
            );
        }
    }

    #[test]
    fn un_champ_optionnel_est_nullable() {
        let rendu = migration("users", "bio:text:optional").contenu;

        assert!(
            rendu.contains("ColumnDef::new(Users::Bio).text().null()"),
            "colonne optionnelle non nullable :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_obligatoire_est_not_null() {
        let rendu = migration("users", "nom:string").contenu;

        assert!(
            rendu.contains("ColumnDef::new(Users::Nom).string().not_null()"),
            "colonne obligatoire non contrainte :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_unique_porte_sa_contrainte() {
        let rendu = migration("users", "email:string:unique").contenu;

        assert!(
            rendu.contains("ColumnDef::new(Users::Email).string().not_null().unique_key()"),
            "contrainte d'unicité absente :\n{rendu}"
        );
    }

    #[test]
    fn un_champ_indexe_recoit_son_index_nomme() {
        let rendu = migration("articles", "slug:string:index").contenu;

        assert!(
            rendu.contains(r#".name("idx_articles_slug")"#),
            "index nommé absent :\n{rendu}"
        );
        assert!(rendu.contains(".col(Articles::Slug)"), "{rendu}");
    }

    #[test]
    fn un_champ_sans_modificateur_ne_cree_aucun_index() {
        let rendu = migration("users", "nom:string").contenu;

        assert!(
            !rendu.contains("create_index"),
            "index créé sans avoir été demandé :\n{rendu}"
        );
    }

    #[test]
    fn les_horodatages_sont_poses_avec_leur_defaut() {
        let rendu = migration("users", "nom:string").contenu;

        let compact: String = rendu.split_whitespace().collect::<Vec<_>>().join(" ");

        for colonne in ["CreatedAt", "UpdatedAt"] {
            assert!(
                compact.contains(&format!(
                    "ColumnDef::new(Users::{colonne}) .timestamp_with_time_zone() .not_null() \
                     .default(Expr::current_timestamp())"
                )),
                "colonne {colonne} mal définie :\n{rendu}"
            );
        }
    }

    #[test]
    fn la_descente_supprime_la_table() {
        let rendu = migration("users", "nom:string").contenu;

        assert!(
            rendu.contains("Table::drop().table(Users::Table)"),
            "descente absente :\n{rendu}"
        );
    }

    #[test]
    fn l_enum_iden_declare_la_table_et_toutes_ses_colonnes() {
        let rendu = migration("blog_posts", "titre:string,publie:bool").contenu;

        assert!(rendu.contains("enum BlogPosts {"), "{rendu}");
        for variante in [
            "Table,",
            "Id,",
            "Titre,",
            "Publie,",
            "CreatedAt,",
            "UpdatedAt,",
        ] {
            assert!(
                rendu.contains(variante),
                "variante {variante} absente :\n{rendu}"
            );
        }
    }

    /// Test posé dans le projet généré : seule la base tranche ce que vaut la migration.
    const REVERSIBILITE: &str = r#"use migration::{Migrator, MigratorTrait};
use sea_orm_migration::sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};

async fn scalaire(db: &DatabaseConnection, sql: &str) -> String {
    db.query_one_raw(Statement::from_string(DatabaseBackend::Postgres, sql))
        .await
        .expect("la requête doit aboutir")
        .expect("une ligne était attendue")
        .try_get_by_index::<String>(0)
        .expect("colonne textuelle")
}

async fn millisecondes(db: &DatabaseConnection) -> i64 {
    scalaire(db, "SELECT floor(EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::bigint::text")
        .await
        .parse()
        .expect("horloge de la base")
}

#[tokio::test]
async fn la_migration_monte_insere_et_redescend() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL doit être fournie");
    let db = Database::connect(&url).await.expect("connexion à la base");

    Migrator::up(&db, None).await.expect("montée de la migration");

    let defaut = scalaire(
        &db,
        "SELECT column_default FROM information_schema.columns \
         WHERE table_name = 'articles' AND column_name = 'id'",
    )
    .await;
    assert!(defaut.contains("uuidv7()"), "défaut de la colonne id : {defaut}");

    let avant = millisecondes(&db).await;
    db.execute_unprepared("INSERT INTO articles (titre, slug) VALUES ('essai', 'essai')")
        .await
        .expect("insertion sans identifiant");
    let apres = millisecondes(&db).await;

    let id = scalaire(&db, "SELECT id::text FROM articles").await;
    assert_eq!(id.chars().nth(14), Some('7'), "version de l'UUID : {id}");

    let tete: String = id.chars().filter(|c| *c != '-').take(12).collect();
    let horodatage = i64::from_str_radix(&tete, 16).expect("tête hexadécimale de l'UUID");
    assert!(
        (avant..=apres).contains(&horodatage),
        "horodatage {horodatage} hors de l'intervalle d'insertion [{avant}, {apres}]"
    );

    Migrator::down(&db, None).await.expect("descente de la migration");

    let tables = scalaire(
        &db,
        "SELECT count(*)::text FROM information_schema.tables WHERE table_name = 'articles'",
    )
    .await;
    assert_eq!(tables, "0", "la table survit à la descente");

    let suivies = scalaire(&db, "SELECT count(*)::text FROM seaql_migrations").await;
    assert_eq!(suivies, "0", "la migration reste inscrite après sa descente");

    Migrator::up(&db, None).await.expect("remontée après descente");
    Migrator::down(&db, None).await.expect("seconde descente");
}
"#;

    #[test]
    #[ignore = "démarre PostgreSQL 18 en conteneur et compile la crate migration"]
    fn la_migration_generee_est_reversible_et_pose_un_uuidv7() {
        let champs = champs::analyser("titre:string,slug:string:unique,resume:text:optional")
            .expect("champs valides");
        let rendue = rendre(&Feature::nouvelle("articles", champs), HORODATAGE)
            .expect("la migration doit se rendre");

        let projet = banc::Projet::neuf();
        projet.poser_migration(&rendue.module, &rendue.contenu);
        projet.poser_test_de_migration("reversibilite", REVERSIBILITE);

        let base = banc::BaseDeTest::demarrer();
        projet.tester_migration(base.url());
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
    fn les_bornes_du_test_d_inversibilite_tronquent_comme_uuidv7() {
        let borne = REVERSIBILITE
            .lines()
            .find(|ligne| ligne.contains("EXTRACT(EPOCH"))
            .expect("le test posé doit lire l'horloge de la base");

        assert!(
            borne.contains("floor("),
            "la borne arrondit au lieu de tronquer : elle passera au-dessus de \
             l'horodatage de uuidv7 une fois sur deux :\n{borne}"
        );
    }

    /// Rendu complet imprimé pour la revue de lecture qu'exige le lot.
    #[test]
    #[ignore = "affichage pour revue humaine"]
    fn apercu() {
        println!(
            "{}",
            migration(
                "articles",
                "titre:string,slug:string:unique,resume:text:optional,vues:int,\
                 publie_le:datetime,auteur:uuid:index"
            )
            .contenu
        );
    }
}
