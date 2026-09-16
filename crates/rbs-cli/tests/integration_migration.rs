//! Une migration d'évolution appliquée, puis défaite, contre chacun des trois moteurs.
//!
//! Ce que le rendu ne peut pas prouver : qu'`ALTER TABLE ADD COLUMN` passe là où chaque
//! moteur a ses interdits, et que le `CHECK` d'une énumération mord une fois la colonne
//! ajoutée. SQLite accepte l'un et l'autre — éprouvé ici plutôt que supposé —, MySQL ne
//! tient un `CHECK` que depuis la 8.0.16, et PostgreSQL le porte de longue date.
//!
//! **La descente est jouée, et son effet mesuré.** Une `down` que rien n'exécute est une
//! `down` que rien ne dit juste : chaque banc la lance, puis demande à la base si la table
//! est restée et si la colonne ajoutée s'en est allée — sans quoi un `migrate down` qui
//! sortirait en zéro sans rien défaire passerait pour une preuve. Le cas qui le mérite est
//! MySQL, seul des trois à matérialiser le `CHECK` en contrainte nommée au niveau de la
//! table, et donc seul à pouvoir refuser le retrait de la colonne qu'elle nomme.
//!
//! Chaque banc pose une table et une migration de noms qui lui sont propres : les projets
//! d'essai partagent leur répertoire de compilation, où deux modules de migration de même
//! nom se sont montrés capables d'échanger leur code compilé.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;
use testcontainers::core::ExecCommand;
use testcontainers::{Container, GenericImage};

mod common;

/// L'image MySQL ne crée que `root` : `MYSQL_USER` n'est pas passé par le démarreur commun.
const UTILISATEUR_MYSQL: &str = "root";

/// Les colonnes que chaque banc ajoute : une énumération, que son `CHECK` borne, et un
/// entier, qui prouve qu'une colonne ordinaire arrive aussi.
const AJOUTS: &str = "statut:enum(draft,published):optional,note:int:optional";

fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}

/// Lance `rbs migrate <action>` sur le projet, et exige qu'il aboutisse.
fn migre(projet: &Path, cible: &Path, action: &str) {
    rbs(projet)
        .env("CARGO_TARGET_DIR", cible)
        .args(["migrate", action])
        .assert()
        .success();
}

/// Crée un projet, y engendre un CRUD, puis la migration qui l'enrichit, et l'applique.
fn projet_migre(
    parent: &Path,
    moteur: &str,
    url: &str,
    table: &str,
    migration: &str,
    cible: &Path,
) -> PathBuf {
    rbs(parent)
        .args([
            "new",
            "demo-api",
            "--database",
            moteur,
            "--database-url",
            url,
            "--core-path",
            common::noyau()
                .to_str()
                .expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    let projet = parent.join("demo-api");

    rbs(&projet)
        .args(["generate", "crud", table, "--fields", "titre:string"])
        .assert()
        .success();

    rbs(&projet)
        .args([
            "generate",
            "migration",
            migration,
            "--add-column",
            table,
            "--fields",
            AJOUTS,
        ])
        .assert()
        .success();

    migre(&projet, cible, "up");

    projet
}

/// Joue `sql` dans le conteneur, et rend ce que le client en a dit — les deux flux réunis.
///
/// Le message qu'imprime le moteur est le critère : un insert accepté n'imprime rien, et
/// l'assertion qui attend un refus tombe alors d'elle-même.
fn dans_le_conteneur(conteneur: &Container<GenericImage>, commande: [&str; 9]) -> String {
    let mut sortie = conteneur
        .exec(ExecCommand::new(commande))
        .expect("le client doit pouvoir s'exécuter dans le conteneur");

    format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout_to_vec().expect("la sortie du client se lit")),
        String::from_utf8_lossy(&sortie.stderr_to_vec().expect("l'erreur du client se lit")),
    )
}

fn psql(postgres: &Container<GenericImage>, sql: &str) -> String {
    dans_le_conteneur(
        postgres,
        [
            "psql",
            "-U",
            common::UTILISATEUR,
            "-d",
            common::BASE,
            "-t",
            "-A",
            "-c",
            sql,
        ],
    )
}

fn mysql(conteneur: &Container<GenericImage>, sql: &str) -> String {
    dans_le_conteneur(
        conteneur,
        [
            "mysql",
            &format!("-u{UTILISATEUR_MYSQL}"),
            &format!("-p{}", common::MOT_DE_PASSE),
            common::BASE,
            "-N",
            "-e",
            sql,
            "--batch",
            "--silent",
        ],
    )
}

/// Le compte qu'une requête `select count(*)` a rendu.
///
/// Les deux flux du client sont réunis — c'est l'erreur du moteur qui prouve un refus —, et
/// le client MySQL écrit sur la sienne un avertissement dès qu'un mot de passe passe par la
/// ligne de commande. La valeur est donc la première ligne qui n'en est pas un : sans ce
/// tri, le compte se lisait « 1\nmysql: [Warning]… » et l'assertion tombait sur la forme du
/// message plutôt que sur ce que la base avait fait.
fn compte(sortie: &str) -> String {
    sortie
        .lines()
        .map(str::trim)
        .find(|ligne| !ligne.is_empty() && !ligne.contains("[Warning]"))
        .unwrap_or_default()
        .to_string()
}

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn an_added_column_migrates_and_its_check_bites_on_postgresql() {
    let postgres = common::start_postgres();
    let url = common::url_of(&postgres);
    let parent = TempDir::new().expect("répertoire temporaire créable");

    let cible = common::cible();
    let _verrou = common::verrou(&cible);

    let projet = projet_migre(
        parent.path(),
        "postgres",
        &url,
        "notices",
        "ajoute_statut_pg",
        &cible,
    );

    let refus = psql(
        &postgres,
        "insert into notices (id, titre, statut) values \
         ('0199c0de-0000-7000-8000-000000000001', 'essai', 'archived');",
    );
    assert!(
        refus.to_lowercase().contains("check constraint"),
        "PostgreSQL a accepté une valeur hors de l'énumération : le CHECK ne tient pas \
         sur une colonne ajoutée :\n{refus}"
    );

    psql(
        &postgres,
        "insert into notices (id, titre, statut, note) values \
         ('0199c0de-0000-7000-8000-000000000002', 'essai', 'draft', 7);",
    );
    let relu = psql(
        &postgres,
        "select statut || '|' || note from notices where note is not null;",
    );
    assert!(
        relu.contains("draft|7"),
        "les deux colonnes ajoutées ne se relisent pas :\n{relu}"
    );

    const TABLE: &str =
        "select count(*) from information_schema.tables where table_name = 'notices';";
    const COLONNE: &str = "select count(*) from information_schema.columns where \
                           table_name = 'notices' and column_name = 'statut';";

    migre(&projet, &cible, "down");
    assert_eq!(
        compte(&psql(&postgres, TABLE)),
        "1",
        "la descente a emporté la table entière, non la seule colonne ajoutée"
    );
    assert_eq!(
        compte(&psql(&postgres, COLONNE)),
        "0",
        "la colonne ajoutée survit à la descente"
    );

    migre(&projet, &cible, "up");
    assert_eq!(
        compte(&psql(&postgres, COLONNE)),
        "1",
        "la remontée ne repose pas la colonne"
    );
}

/// Le même sur MySQL, qui ne tient un `CHECK` que depuis la 8.0.16 — avant, il l'analysait
/// puis l'ignorait en silence.
///
/// C'est ici que la descente se joue vraiment : MySQL matérialise le `CHECK` en contrainte
/// nommée au niveau de la table, et refuse le retrait d'une colonne qu'une contrainte
/// nomme encore. Si la `down` engendrée devait échouer quelque part, c'est sur ce moteur.
#[test]
#[ignore = "démarre MySQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn an_added_column_migrates_and_its_check_bites_on_mysql() {
    let conteneur = common::start_mysql();
    let url = common::url_of_mysql(&conteneur);
    let parent = TempDir::new().expect("répertoire temporaire créable");

    let cible = common::cible_pour("mysql");
    let _verrou = common::verrou(&cible);

    let projet = projet_migre(
        parent.path(),
        "mysql",
        &url,
        "bulletins",
        "ajoute_statut_my",
        &cible,
    );

    let refus = mysql(
        &conteneur,
        "insert into bulletins (id, titre, statut) values \
         (x'00000000000040008000000000000001', 'essai', 'archived');",
    );
    assert!(
        refus.to_lowercase().contains("check constraint"),
        "MySQL a accepté une valeur hors de l'énumération : le CHECK ne tient pas sur \
         une colonne ajoutée :\n{refus}"
    );

    let relu = mysql(
        &conteneur,
        "insert into bulletins (id, titre, statut, note) values \
         (x'00000000000040008000000000000002', 'essai', 'draft', 7); \
         select concat(statut, '|', note) from bulletins where note is not null;",
    );
    assert!(
        relu.contains("draft|7"),
        "les deux colonnes ajoutées ne se relisent pas :\n{relu}"
    );

    const TABLE: &str = "select count(*) from information_schema.tables where \
                         table_schema = 'demo' and table_name = 'bulletins';";
    const COLONNE: &str = "select count(*) from information_schema.columns where \
                           table_schema = 'demo' and table_name = 'bulletins' and \
                           column_name = 'statut';";

    migre(&projet, &cible, "down");
    assert_eq!(
        compte(&mysql(&conteneur, TABLE)),
        "1",
        "la descente a emporté la table entière, non la seule colonne ajoutée"
    );
    assert_eq!(
        compte(&mysql(&conteneur, COLONNE)),
        "0",
        "la colonne ajoutée survit à la descente : MySQL a-t-il refusé de la retirer \
         pendant que la contrainte la nommait encore ?"
    );

    migre(&projet, &cible, "up");
    assert_eq!(
        compte(&mysql(&conteneur, COLONNE)),
        "1",
        "la remontée ne repose pas la colonne"
    );
}

/// Le même sur SQLite, qui accepte `ADD COLUMN … CHECK` et l'applique — c'est ce que ce
/// banc établit, et c'est ce qui autorise la commande à rendre le `CHECK` pour les trois
/// moteurs plutôt que pour deux. Aucun conteneur : la base est un fichier du projet.
#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn an_added_column_migrates_and_its_check_bites_on_sqlite() {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    let cible = common::cible_pour("types-sqlite");
    let _verrou = common::verrou(&cible);

    let projet = projet_migre(
        parent.path(),
        "sqlite",
        "sqlite://demo_api.db?mode=rwc",
        "memos",
        "ajoute_statut_sq",
        &cible,
    );

    let sqlite3 = |sql: &str| {
        let sortie = std::process::Command::new("sqlite3")
            .arg(projet.join("demo_api.db"))
            .arg(sql)
            .output()
            .expect("sqlite3 doit être lançable");

        format!(
            "{}{}",
            String::from_utf8_lossy(&sortie.stdout),
            String::from_utf8_lossy(&sortie.stderr),
        )
    };

    // SeaORM range l'`uuid` en BLOB de seize octets sur SQLite, jamais en texte : d'où les
    // littéraux `x'…'`.
    let refus = sqlite3(
        "insert into memos (id, titre, statut) values \
         (x'00000000000040008000000000000001', 'essai', 'archived');",
    );
    assert!(
        refus.to_lowercase().contains("check constraint"),
        "SQLite a accepté une valeur hors de l'énumération : le CHECK ne tient pas sur \
         une colonne ajoutée :\n{refus}"
    );

    let relu = sqlite3(
        "insert into memos (id, titre, statut, note) values \
         (x'00000000000040008000000000000002', 'essai', 'draft', 7); \
         select statut || '|' || note from memos where note is not null;",
    );
    assert!(
        relu.contains("draft|7"),
        "les deux colonnes ajoutées ne se relisent pas :\n{relu}"
    );

    const TABLE: &str = "select count(*) from sqlite_master where type = 'table' and \
                         name = 'memos';";
    const COLONNE: &str = "select count(*) from pragma_table_info('memos') where \
                           name = 'statut';";

    migre(&projet, &cible, "down");
    assert_eq!(
        compte(&sqlite3(TABLE)),
        "1",
        "la descente a emporté la table entière, non la seule colonne ajoutée"
    );
    assert_eq!(
        compte(&sqlite3(COLONNE)),
        "0",
        "la colonne ajoutée survit à la descente"
    );

    migre(&projet, &cible, "up");
    assert_eq!(
        compte(&sqlite3(COLONNE)),
        "1",
        "la remontée ne repose pas la colonne"
    );
}
