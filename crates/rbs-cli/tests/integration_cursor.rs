//! `generate crud --cursor`, jusqu'à la base et jusqu'au client.
//!
//! Les gabarits touchés par `--cursor` ne se lisent pas seuls : `CursorPage` doit encore
//! passer `utoipa` et Axum, le repository doit encore produire du SQL valide contre une
//! vraie base, et le document OpenAPI qu'il porte doit encore s'analyser par
//! `rbs generate client`. Le drapeau se promet compatible avec `--role`, `--soft-delete`
//! et `--with-upload` : ce projet-ci l'engendre avec les trois à la fois, pour que chacune
//! de ces compatibilités soit compilée et jouée contre une vraie base.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Le binaire livré, lancé dans `racine`.
fn rbs(racine: &Path) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(racine);
    commande
}

/// La chaîne complète : un projet sous `auth` et `storage`, une entité paginée par curseur
/// et protégée par un rôle, ses migrations appliquées, sa suite jouée contre PostgreSQL, et
/// son client TypeScript engendré.
///
/// `--with auth,storage` fait exercer, sous `--cursor`, la garde `require_role` et
/// l'entrée `security` du contrôleur (`--role admin`) comme les trois routes de contenu
/// binaire (`--with-upload`) — la seule combinaison qu'aucun autre banc ne joue. Le
/// `--soft-delete` maintient `deleted_at IS NULL` sur la liste par curseur elle-même.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet, avec auth, storage \
            et un client TypeScript : plusieurs minutes"]
fn a_cursor_paginated_crud_with_auth_storage_and_a_role_passes_and_renders_its_client() {
    let postgres = common::start_postgres();
    let url = common::url_of(&postgres);

    let parent = TempDir::new().expect("répertoire temporaire créable");

    // Nom distinct de `demo-api` et `test-api`, déjà pris par d'autres suites dans la même
    // cible partagée : deux projets du même nom s'y gêneraient.
    rbs(parent.path())
        .args([
            "new",
            "cursor-api",
            "--database-url",
            &url,
            "--core-path",
            common::noyau()
                .to_str()
                .expect("chemin du noyau représentable"),
            "--yes",
            "--with",
            "auth,storage",
        ])
        .assert()
        .success();

    let projet = parent.path().join("cursor-api");

    // Le squelette écrit un compose réel pour une base locale : le laisser monterait un
    // second PostgreSQL par-dessus le conteneur déjà démarré ci-dessus.
    fs::remove_file(projet.join("docker-compose.yml")).expect("le compose doit exister");

    rbs(&projet)
        .args([
            "generate",
            "crud",
            "articles",
            "--fields",
            "title:string",
            "--cursor",
            "--soft-delete",
            "--with-upload",
            "--role",
            "admin",
        ])
        .assert()
        .success();

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    rbs(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();

    // Les tests générés joignent la base décrite par le `.env` du projet et sont
    // `#[ignore]` : sans `--include-ignored`, cette étape ne lancerait plus rien.
    // `--no-fail-fast` avant le `--` : sans lui la suite s'arrêterait au premier binaire de
    // test en échec et masquerait les suivants.
    //
    // Les deux `--skip` excluent les tests du fragment `storage` qui exigent un vrai S3 :
    // sans MinIO démarré, `GetObjectEndpointParamsInterceptor` refuse faute de `bucket`.
    // `--with-upload` n'en a pas besoin — ses trois routes de contenu passent par le
    // backend `file` — mais `storage` embarque ces deux scénarios dès qu'il est installé.
    // `crates/rbs-cli/tests/integration_storage.rs` les possède déjà et les prouve contre
    // un MinIO qu'il démarre lui-même.
    let sortie = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args([
            "test",
            "--no-fail-fast",
            "--",
            "--include-ignored",
            "--skip",
            "an_object_put_by_the_trait_reads_back_through_the_s3_client",
            "--skip",
            "the_s3_backend_passes_the_same_round_as_the_file_backend",
        ])
        .output()
        .expect("cargo doit être lançable");

    let rendu = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    assert!(
        sortie.status.success(),
        "la suite du projet engendré échoue :\n{rendu}"
    );

    // Nommés plutôt que comptés : `cargo test` sort en 0 sur une suite amputée, et c'est
    // précisément une suite amputée qu'une template cassée livrerait.
    for scenario in [
        // La marche par curseur : chaque ligne une fois, jusqu'à l'extinction de `next`.
        "articles::tests::lifecycle::the_cursor_walks_every_page_without_duplicates ... ok",
        // Le cycle de vie ordinaire, et les identifiants croissants d'un curseur sur `id`.
        "articles::tests::lifecycle::the_full_lifecycle_goes_through_the_api ... ok",
        "articles::tests::lifecycle::two_creations_in_a_row_carry_increasing_ids ... ok",
        // La route de filtre reste en `Page`/`Pagination` : `--cursor` ne l'a pas déplacée.
        "articles::tests::filter::the_filter_narrows_the_list ... ok",
        "articles::tests::errors::an_unknown_sort_column_returns_400 ... ok",
        // `--role admin` fait franchir les deux seuils : la garde et la lecture anonyme.
        "articles::tests::access::an_anonymous_request_returns_401 ... ok",
        "articles::tests::access::an_anonymous_read_returns_401 ... ok",
        // `--with-upload`, sous `auth` : les routes de contenu, protégées elles aussi.
        "articles::tests::content::the_content_round_trips_through_put_get_and_head ... ok",
        "articles::tests::content::an_anonymous_content_request_returns_401 ... ok",
    ] {
        assert!(
            rendu.contains(scenario),
            "`{scenario}` n'a pas été joué :\n{rendu}"
        );
    }

    rbs(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    let ts = fs::read_to_string(projet.join("clients/ts/client.ts"))
        .expect("le client TypeScript doit être écrit");

    // Le nom exact dépend de la convention de repli des génériques utoipa
    // (`Page_PostResponse` devient `PagePostResponse`) : seul le préfixe est garanti.
    assert!(
        ts.contains("export interface CursorPage"),
        "aucune interface CursorPage… dans le client :\n{ts}"
    );
    assert!(
        ts.contains("articlesList("),
        "la méthode articlesList est absente du client :\n{ts}"
    );
}
