//! Le seul test qui prouve que rbs fonctionne : il invoque le binaire livré, pas
//! `new::creer`, et compile ce que ce binaire a produit.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Les tests qu'un CRUD engendré livre au projet, et qui joignent tous la base.
const TESTS_DU_CRUD: [&str; 4] = [
    "articles::tests::lifecycle::the_full_lifecycle_goes_through_the_api",
    "articles::tests::lifecycle::two_creations_in_a_row_carry_increasing_ids",
    "articles::tests::errors::an_unknown_id_returns_404",
    "articles::tests::errors::an_unreadable_body_returns_400",
];

#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_generated_project_compiles_and_passes_its_tests() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let noyau = common::noyau();

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent.path())
        .args([
            "new",
            "demo-api",
            "--database-url",
            "postgres://rbs:rbs@localhost:5432/demo_api",
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    let projet = parent.path().join("demo-api");
    assert!(projet.join("Cargo.toml").is_file(), "projet non créé");

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    for action in ["build", "test"] {
        Command::new("cargo")
            .current_dir(&projet)
            .env("CARGO_TARGET_DIR", common::cible())
            .args([action, "--workspace"])
            .assert()
            .success();
    }

    // Le niveau qu'exige la CI que `rbs add ci` pose dans le projet : un squelette qui
    // laisse un warning derrière lui rendrait rouge, dès le premier push, du code que
    // l'utilisateur n'a pas écrit.
    Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .assert()
        .success();

    // `rustfmt` sur les racines de modules et non `cargo fmt` : lancé sous `cargo test`,
    // celui-ci retrouve le workspace de rbs lui-même et signalerait ses fichiers.
    // `src/lib.rs` porte la bibliothèque — `health`, `openapi`, `router`, `state` — et
    // `src/main.rs` n'est plus qu'une feuille sans enfant à elle.
    for racine_de_modules in [
        "src/lib.rs",
        "src/main.rs",
        "src/seeds/main.rs",
        "migration/src/lib.rs",
    ] {
        Command::new("rustfmt")
            .args(["--edition", "2024", "--check"])
            .arg(projet.join(racine_de_modules))
            .assert()
            .success();
    }
}

/// Le second critère : une valeur inconnue est refusée en nommant les trois admises.
///
/// Le contrôle appartient à clap, qui énumère déjà les valeurs d'un `ValueEnum` : ce test
/// constate que rbs ne s'est pas mis en travers, non qu'un message maison existe.
#[test]
fn an_unknown_engine_is_refused_naming_the_three_admitted() {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    let sortie = Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent.path())
        .args(["new", "demo-api", "--database", "oracle", "--yes"])
        .assert()
        .failure()
        .get_output()
        .clone();

    let message = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    for admise in ["postgres", "mysql", "sqlite"] {
        assert!(
            message.contains(admise),
            "le refus ne nomme pas `{admise}` :\n{message}"
        );
    }
    assert!(
        !parent.path().join("demo-api").exists(),
        "un projet a été créé malgré le refus"
    );
}

/// Les trois moteurs produisent-ils un projet dont la suite passe ?
///
/// `cargo build` ne prouvait que la compilation, et une compilation ne demande aucune
/// base : les requêtes que SeaORM engendre pour un moteur ne sont exercées qu'à
/// l'exécution. C'est le critère de sortie du jalon qui a fait monter ce test d'un cran.
///
/// Une cible de compilation par moteur, comme au jour de l'arbitrage : les trois activent
/// des features `sea-orm` différentes, et une cible commune ferait recompiler `sea-orm` et
/// `sqlx` à chaque bascule.
///
/// Un CRUD est engendré avant de lancer la suite, et ses quatre tests sont **exigés
/// nommément** en `... ok`. Sans cela le test ne prouverait rien : un projet vierge n'a
/// aucun test qui touche la base, et `cargo test` y rend « 0 passed » sur les trois
/// moteurs — y compris sur un moteur dont pas une requête ne fonctionnerait.
#[test]
#[ignore = "démarre PostgreSQL et MySQL, puis compile et joue trois projets complets : plusieurs minutes"]
fn each_engine_produces_a_project_whose_tests_pass() {
    let noyau = common::noyau();
    let postgres = common::start_postgres();
    let mysql = common::start_mysql();

    // SQLite n'a pas de serveur : sa base est un fichier, que l'URL crée au besoin. Il
    // vit dans le répertoire du projet, où `migrate` et `cargo test` sont tous deux
    // lancés — une URL relative n'a de sens que rapportée au même répertoire courant.
    let moteurs = [
        ("postgres", common::url_of(&postgres)),
        ("mysql", common::url_of_mysql(&mysql)),
        ("sqlite", "sqlite://demo_api.db?mode=rwc".to_string()),
    ];

    for (moteur, url) in moteurs {
        let parent = TempDir::new().expect("répertoire temporaire créable");

        Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(parent.path())
            .args([
                "new",
                "demo-api",
                "--database",
                moteur,
                "--database-url",
                &url,
                "--core-path",
                noyau.to_str().expect("chemin du noyau représentable"),
                "--yes",
            ])
            .assert()
            .success();

        let projet = parent.path().join("demo-api");
        let cible = common::cible_pour(moteur);
        // Une cible par moteur, donc un verrou par moteur : les trois branches restent
        // libres de tourner de front avec celles d'un autre binaire de test.
        let _verrou = common::verrou(&cible);

        Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(&projet)
            .args([
                "generate",
                "crud",
                "articles",
                "--fields",
                "title:string,body:text,published:bool",
            ])
            .assert()
            .success();

        // Les tests livrés au projet supposent les migrations appliquées : ils montent
        // l'application sur la base décrite par le `.env`, et ne créent aucun schéma.
        Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(&projet)
            .env("CARGO_TARGET_DIR", &cible)
            .args(["migrate", "up"])
            .assert()
            .success();

        // `--include-ignored` : les tests engendrés joignent la base du projet et sont
        // `#[ignore]` pour cette raison. La migration vient d'être appliquée ci-dessus,
        // ils ont donc de quoi tourner.
        let sortie = Command::new("cargo")
            .current_dir(&projet)
            .env("CARGO_TARGET_DIR", &cible)
            .args(["test", "--workspace", "--", "--include-ignored"])
            .output()
            .expect("cargo doit être lançable");

        let rendu = format!(
            "{}{}",
            String::from_utf8_lossy(&sortie.stdout),
            String::from_utf8_lossy(&sortie.stderr)
        );

        assert!(
            sortie.status.success(),
            "la suite du projet engendré échoue sur {moteur} :\n{rendu}"
        );

        for test in TESTS_DU_CRUD {
            assert!(
                rendu.contains(&format!("{test} ... ok")),
                "`{test}` n'a pas tourné sur {moteur} — un gabarit qui cesserait de livrer \
                 ses tests laisserait ce test au vert, `cargo test` sortant en 0 même \
                 quand il ne joue rien :\n{rendu}"
            );
        }
    }
}

/// Le parcours que la documentation enseigne, joué en entier : créer, monter la base par
/// le seul compose engendré, migrer, compiler. Les tests ci-dessus prouvent la forme du
/// compose ; celui-ci prouve qu'il sert — aucune valeur n'est recopiée d'un fichier à
/// l'autre.
///
/// Ce qu'il attraperait : un port publié qui ne suit pas l'URL du `.env` (`migrate` et le
/// binaire compilé chercheraient la base ailleurs que là où le compose l'a montée), un
/// service `db` mal formé ou une ancre `<rbs:services>` produisant un YAML que `docker
/// compose` refuserait, ou une variable d'environnement que le squelette engendré
/// attendrait à la main — puisqu'aucune n'est passée ici, ni à `compose`, ni à `migrate`.
#[test]
#[ignore = "démarre PostgreSQL par le compose engendré et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_generated_compose_serves_the_project_it_was_generated_for() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let noyau = common::noyau();

    // Tiré au sort à chaque exécution : deux lancements concurrents ne doivent pas se
    // disputer 5432, que cette machine héberge déjà pour d'autres PostgreSQL locaux. Le
    // retrouver dans l'URL une fois la base migrée est aussi ce qui prouve que le port
    // publié par le compose suit l'URL du projet plutôt qu'une valeur figée dans la
    // template.
    let port = free_port();
    let url = format!("postgres://rbs:secret@localhost:{port}/demo");

    // Le nom du projet Docker Compose est global, indépendant du répertoire courant : un
    // `name: demo` figé se disputerait avec toute autre exécution concurrente de ce même
    // test, ou avec un conteneur resté d'un lancement tué avant que `ComposeGuard` n'ait
    // pu démonter. Dérivé du port déjà tiré au sort, il est unique pour la même raison.
    let name = format!("demo-{port}");

    rbs(parent.path())
        .args(["new", &name, "--yes", "--database-url", &url])
        .args([
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
        ])
        .assert()
        .success();

    let root = parent.path().join(&name);

    // C'est ce `name:` que Docker emploie comme identifiant de projet — celui qui isole
    // les ressources d'une exécution de celles d'une autre. Le vérifier ici, c'est
    // vérifier que la protection porte bien sur ce que Docker regarde, pas seulement sur
    // le nom du répertoire.
    let compose_yml = fs::read_to_string(root.join("docker-compose.yml")).expect("compose lisible");
    assert!(
        compose_yml.contains(&format!("name: {name}")),
        "le compose ne porte pas le nom unique du projet :\n{compose_yml}"
    );

    // Démonte les conteneurs et le volume même si une assertion plus bas échoue — y
    // compris celle du `up` lui-même : construite après, une panique de cette ligne aurait
    // laissé les conteneurs tourner sans que rien ne les démonte.
    let _garde = ComposeGuard { root: root.clone() };

    // Le compose engendré, et lui seul : aucun `docker run` ni variable d'environnement
    // passée à la main — précisément ce que ce test doit prouver.
    compose(&root, &["up", "-d", "--wait"]).assert().success();

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    rbs(&root)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();

    Command::new("cargo")
        .current_dir(&root)
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("build")
        .assert()
        .success();
}

/// Deux fragments posés à la création cohabitent, et le projet qui en résulte compile.
///
/// Ce qu'il attraperait : un ordre d'insertion où la seconde feature écraserait l'ancre
/// de la première plutôt que de s'y ajouter, une dépendance Cargo dupliquée ou en conflit
/// entre les deux fragments, ou une ancre `<rbs:services>` produisant, une fois le service
/// redis ajouté à côté du `db` déjà présent, un compose que `docker compose config`
/// refuserait.
#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn a_project_created_with_two_features_compiles() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let noyau = common::noyau();

    // Distinct de celui du test précédent : la confusion entre les deux compose
    // engendrés n'a pas besoin d'être empêchée par un tirage au sort ici, ce test
    // n'appelant jamais `compose up` — seulement `docker compose config`, qui ne touche
    // aucune ressource nommée globalement.
    let sortie = rbs(parent.path())
        .args(["new", "demo-with-features", "--yes", "--with", "auth,redis"])
        .args([
            "--database-url",
            "postgres://rbs:secret@localhost:5432/demo",
        ])
        .args([
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&sortie.stdout);
    for pose in ["+ auth", "+ redis"] {
        assert!(
            stdout.contains(pose),
            "la sortie ne rapporte pas `{pose}` posée :\n{stdout}"
        );
    }

    let root = parent.path().join("demo-with-features");

    assert!(root.join("src/auth/service/mod.rs").is_file());
    assert!(root.join("src/modules/cache/mod.rs").is_file());

    let compose_yml = fs::read_to_string(root.join("docker-compose.yml")).expect("compose lisible");
    assert!(compose_yml.contains("redis:8-alpine"), "{compose_yml}");

    Command::new("docker")
        .current_dir(&root)
        .args(["compose", "config"])
        .assert()
        .success();

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    Command::new("cargo")
        .current_dir(&root)
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("build")
        .assert()
        .success();
}

/// La sonde qu'un fragment inscrit dans `<rbs:health_probes>` doit compiler.
///
/// Aucun test de rendu ne le prouve : l'ancre reçoit une ligne de texte, et seul le
/// compilateur dit si elle nomme des types qui existent, si le futur qu'elle construit est
/// `Send`, et si les deux emprunts de `state` — celui de la base et celui de la
/// dépendance — tiennent ensemble le temps de la réponse.
#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_probes_installed_by_two_fragments_compile_into_the_health_route() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let noyau = common::noyau();

    rbs(parent.path())
        .args([
            "new",
            "demo-with-probes",
            "--yes",
            "--with",
            "redis,storage",
        ])
        .args([
            "--database-url",
            "postgres://rbs:secret@localhost:5432/demo",
        ])
        .args([
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
        ])
        .assert()
        .success();

    let root = parent.path().join("demo-with-probes");

    let controleur =
        fs::read_to_string(root.join("src/health/controller.rs")).expect("contrôleur lisible");
    // La sonde de `storage` dépasse la largeur de ligne de rustfmt une fois préfixée par
    // `modules::` : elle se reformate sur trois lignes, à la différence de celle de `cache`.
    for sonde in [
        r#"rbs_core::health::Probe::new("cache", state.cache().ping()),"#,
        "rbs_core::health::Probe::new(\n                \"storage\",\n                \
         crate::modules::storage::probe(state.storage()),\n            ),",
    ] {
        assert!(
            controleur.contains(sonde),
            "la sonde `{sonde}` manque au contrôleur :\n{controleur}"
        );
    }

    // `clippy -D warnings` plutôt que `build` : la sonde du stockage passe par une
    // fonction que rien d'autre n'appelle, et un `#[allow(dead_code)]` mal placé la
    // laisserait passer pour morte.
    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    Command::new("cargo")
        .current_dir(&root)
        .env("CARGO_TARGET_DIR", common::cible())
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .assert()
        .success();
}

/// Ce test n'ignore rien et ne compile pas le projet : `rbs new` écrit des fichiers, et
/// c'est tout ce qu'il y a à regarder ici.
#[test]
fn the_manifest_records_the_language_asked_for() {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent.path())
        .args([
            "new",
            "demo-api",
            "--database-url",
            "postgres://rbs:rbs@localhost:5432/demo_api",
            "--lang",
            "en",
            "--yes",
        ])
        .assert()
        .success();

    let manifeste = fs::read_to_string(parent.path().join("demo-api/Cargo.toml"))
        .expect("le manifeste est écrit");

    assert!(
        manifeste.contains(r#"lang = "en""#),
        "le manifeste ne garde pas la langue demandée :\n{manifeste}"
    );
}

/// À défaut de `--lang`, la langue vient de la locale de l'environnement.
///
/// `locale_from` et `Lang::from_locale` sont éprouvées chacune de leur côté ; ce qui ne
/// l'était pas, c'est le câblage — le `unwrap_or_else` qui les enchaîne, et qu'une
/// signature changée aurait pu court-circuiter sans qu'aucun test ne bouge. D'où le
/// binaire, seul endroit où l'environnement du processus se pose sans être partagé avec
/// les autres tests.
#[test]
fn without_the_flag_the_language_is_taken_from_the_environment() {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent.path())
        .env("LC_ALL", "fr_FR.UTF-8")
        .args([
            "new",
            "demo-api",
            "--database-url",
            "postgres://rbs:rbs@localhost:5432/demo_api",
            "--yes",
        ])
        .assert()
        .success();

    let manifeste = fs::read_to_string(parent.path().join("demo-api/Cargo.toml"))
        .expect("le manifeste est écrit");

    assert!(
        manifeste.contains(r#"lang = "fr""#),
        "la locale de l'environnement n'a pas été suivie :\n{manifeste}"
    );
}

/// Une URL dont rbs ne tire aucun hôte laisse le projet sans compose, et c'est le seul
/// cas où l'absence du fichier ne se lit pas dans l'URL : elle doit donc s'annoncer.
///
/// Le binaire et non `url_opaque` : ce qui est en jeu ici, c'est le câblage entre la
/// décomposition de l'URL et l'avertissement, qu'une autorité sans hôte traversait
/// jusqu'ici en silence. Rien n'est compilé, `rbs new` ne fait qu'écrire des fichiers.
#[test]
fn a_url_without_a_host_says_why_no_compose_was_written() {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    let sortie = Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent.path())
        .args([
            "new",
            "demo-api",
            "--database-url",
            "postgres://:5432/demo_api",
            "--yes",
        ])
        .assert()
        .success();

    let rendu = String::from_utf8_lossy(&sortie.get_output().stderr).into_owned();

    assert!(
        rendu.contains("aucun hôte"),
        "l'URL sans hôte n'est pas annoncée :\n{rendu}"
    );
    // La phrase qui explique l'avertissement doit le suivre sur le même flux : redirigée
    // seule, elle ne dirait ni ce qu'elle explique ni ce qu'il faut en faire.
    assert!(
        rendu.contains("socket Unix"),
        "la suite de l'avertissement n'est pas sur la sortie d'erreur :\n{rendu}"
    );
    assert!(
        !String::from_utf8_lossy(&sortie.get_output().stdout).contains("socket Unix"),
        "la suite de l'avertissement est restée sur la sortie standard"
    );
    assert!(
        !parent.path().join("demo-api/docker-compose.yml").exists(),
        "un compose a été écrit pour une URL sans hôte :\n{rendu}"
    );
}

#[test]
fn the_created_project_carries_an_agents_file_naming_the_cli() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    let agents = fs::read_to_string(projet.join("AGENTS.md")).expect("AGENTS.md est écrit");

    assert!(agents.contains("rbs generate crud"), "{agents}");
    assert!(agents.contains("<!-- rbs:inventory -->"), "{agents}");
    assert!(agents.contains("postgres"), "{agents}");
}

/// `/health/live` répond tant que le processus tourne, base ou non ; `/health` passe au
/// 503 dès qu'elle manque. C'est la distinction qu'un orchestrateur lit : sonder la vie
/// sur la base ferait redémarrer l'API en boucle le jour où c'est la base qui tombe.
///
/// La base est présente au démarrage — `main` s'y connecte avant d'écouter — puis arrêtée
/// sous le serveur qui tourne.
#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_liveness_route_outlives_the_database() {
    let postgres = common::start_postgres();
    let url = common::url_of(&postgres);
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let noyau = common::noyau();

    // Un nom qu'aucune autre suite n'emploie : deux projets homonymes compilés dans la
    // cible partagée se disputent leurs artefacts.
    rbs(parent.path())
        .args(["new", "sonde-vie", "--yes", "--database-url", &url])
        .args([
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
        ])
        .assert()
        .success();

    let root = parent.path().join("sonde-vie");

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    rbs(&root)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();

    Command::new("cargo")
        .current_dir(&root)
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("build")
        .assert()
        .success();

    let serveur = Serveur::lancer(&root, "sonde-vie");

    assert_eq!(get(serveur.port, "/health/live").0, 200);
    assert_eq!(get(serveur.port, "/health").0, 200);

    let (code, document) = get(serveur.port, "/api-docs/openapi.json");
    assert_eq!(code, 200, "{document}");
    for chemin in ["\"/health/live\"", "\"/health\""] {
        assert!(
            document.contains(chemin),
            "{chemin} absent du document OpenAPI :\n{document}"
        );
    }

    postgres.stop().expect("PostgreSQL doit s'arrêter");

    let (code, corps) = get(serveur.port, "/health");
    assert_eq!(code, 503, "la readiness ignore la base arrêtée :\n{corps}");

    let (code, corps) = get(serveur.port, "/health/live");
    assert_eq!(code, 200, "la liveness a suivi la base :\n{corps}");
}

/// Ce que `make dev` lance à la place du binaire et de Vite : une boucle qui annonce son
/// pid dans le fichier qu'on lui nomme, puis ne s'arrête plus d'elle-même.
#[cfg(unix)]
const FAUX_PROCESSUS: &str = "#!/bin/sh\necho $$ > \"$1\"\nwhile : ; do sleep 0.2 ; done\n";

/// La recette `dev` du squelette, éprouvée pour de bon : deux processus lancés ensemble,
/// un SIGINT au groupe, et plus rien qui survive.
///
/// `cargo run` et le serveur du client sont remplacés par deux boucles de shell, `DEV`
/// étant surchargé sur la ligne de commande, où make donne le dernier mot. Ce test dit
/// donc ce que fait la recette — son piège, son groupe, son `wait` — et non ce que font
/// `cargo` et `npm` sous elle : les deux processus réels coûteraient une compilation et
/// un `npm install` sans rien apprendre de plus sur le piège, qui ne les distingue pas.
///
/// Le groupe à part n'est pas un raffinement de test : `kill 0` emporte le groupe entier
/// du shell qui a lancé make, donc celui de `cargo test` si on ne l'en sépare pas.
#[cfg(unix)]
#[test]
fn a_ctrl_c_on_the_dev_shortcut_leaves_none_of_its_processes_behind() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    let faux = projet.join("faux.sh");
    fs::write(&faux, FAUX_PROCESSUS).expect("le faux processus doit s'écrire");
    rendre_executable(&faux);

    let mut dev = Dev::lancer(&projet, "./faux.sh back.pid & ./faux.sh front.pid &");

    // Les deux doivent tourner avant le signal : un Ctrl-C reçu par une recette qui n'a
    // encore rien lancé ne prouverait rien.
    let pids = ["back.pid", "front.pid"].map(|nom| pid_annonce(&projet.join(nom)));

    assert!(
        signaler(dev.groupe(), "INT"),
        "le SIGINT doit partir vers le groupe de la recette"
    );

    let statut = dev.attendre(Duration::from_secs(30));

    let survivants: Vec<u32> = pids
        .into_iter()
        .filter(|pid| survit(*pid, Duration::from_secs(10)))
        .collect();

    // Abattus avant l'assertion : un test qui échoue ne doit pas laisser derrière lui ce
    // dont il vient de constater la survie.
    for pid in &survivants {
        signaler(*pid, "KILL");
    }

    assert!(
        survivants.is_empty(),
        "un Ctrl-C sur `make dev` a laissé {survivants:?} derrière lui ; make est sorti \
         sur {statut}"
    );
}

/// Un `make dev` dont tout ce qu'il a lancé se termine de soi-même rend zéro.
///
/// C'est ce qu'un piège sur `EXIT` interdisait, et il en portait un : `wait` ne rend la
/// main qu'une fois les processus finis, si bien que le `kill 0` d'`EXIT` n'avait plus
/// rien à arrêter et emportait le groupe entier — make compris, et le shell d'un script
/// qui aurait appelé la recette. Mesuré avant le correctif : make mourait de son propre
/// SIGTERM, sur un projet où rien n'avait échoué.
#[cfg(unix)]
#[test]
fn the_dev_shortcut_exits_zero_when_what_it_started_ends_on_its_own() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    let mut dev = Dev::lancer(&projet, "true &");
    let statut = dev.attendre(Duration::from_secs(30));

    assert!(
        statut.success(),
        "`make dev` doit rendre zéro quand ce qu'il mène s'arrête seul, il a rendu {statut}"
    );
}

/// `make dev` lancé dans son propre groupe de processus, ce groupe abattu si le garde
/// tombe avant que make se soit arrêté.
#[cfg(unix)]
struct Dev {
    processus: Child,
}

#[cfg(unix)]
impl Dev {
    /// Lance la recette, `DEV` surchargé par `dev`.
    fn lancer(projet: &Path, dev: &str) -> Self {
        use std::os::unix::process::CommandExt;

        let processus = std::process::Command::new("make")
            .current_dir(projet)
            .arg("dev")
            .arg(format!("DEV={dev}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("make doit être lançable");

        Self { processus }
    }

    /// Le groupe de la recette : `process_group(0)` fait de make son chef, et le groupe
    /// porte donc son pid.
    fn groupe(&self) -> u32 {
        self.processus.id()
    }

    /// Attend la fin de make, et échoue au bout de `limite`.
    fn attendre(&mut self, limite: Duration) -> std::process::ExitStatus {
        let fin = Instant::now() + limite;

        loop {
            match self
                .processus
                .try_wait()
                .expect("l'état de make est lisible")
            {
                Some(statut) => return statut,
                None => assert!(
                    Instant::now() < fin,
                    "`make dev` tourne encore {limite:?} après le signal"
                ),
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

#[cfg(unix)]
impl Drop for Dev {
    fn drop(&mut self) {
        if matches!(self.processus.try_wait(), Ok(None)) {
            signaler(self.groupe(), "KILL");
            let _ = self.processus.wait();
        }
    }
}

/// Le pid qu'un faux processus a écrit dans `fichier`, attendu jusqu'à trente secondes.
///
/// Le fichier est tronqué avant d'être écrit : une lecture peut le trouver vide, et c'est
/// une relecture qu'il faut alors, non un échec.
#[cfg(unix)]
fn pid_annonce(fichier: &Path) -> u32 {
    let fin = Instant::now() + Duration::from_secs(30);

    loop {
        if let Some(pid) = fs::read_to_string(fichier)
            .ok()
            .and_then(|lu| lu.trim().parse().ok())
        {
            return pid;
        }

        assert!(
            Instant::now() < fin,
            "{} n'annonce toujours aucun pid",
            fichier.display()
        );

        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Envoie `signal` au groupe `groupe`, par le `kill` du shell.
///
/// Celui-ci, et non un appel système : `libc` ne serait une dépendance que d'ici, et le
/// pid négatif qui désigne un groupe est du POSIX que tout shell sait écrire.
#[cfg(unix)]
fn signaler(groupe: u32, signal: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("kill -{signal} -{groupe}"))
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|statut| statut.success())
}

/// `pid` vit-il encore au bout de `limite` ? Le signal nul ne fait que poser la question.
#[cfg(unix)]
fn survit(pid: u32, limite: Duration) -> bool {
    let fin = Instant::now() + limite;

    loop {
        let vivant = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("kill -0 {pid}"))
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|statut| statut.success());

        if !vivant {
            return false;
        }

        if Instant::now() >= fin {
            return true;
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Rend `chemin` exécutable : un faux processus que la recette lance par son chemin.
#[cfg(unix)]
fn rendre_executable(chemin: &Path) {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(chemin, fs::Permissions::from_mode(0o755))
        .expect("le faux processus doit être exécutable");
}

/// Le binaire d'un projet, lancé sur un port libre, arrêté quand ce garde tombe.
///
/// `Drop` plutôt qu'un `kill` en fin de test : une assertion qui échoue déroule la pile
/// sans jamais l'atteindre, et laisserait derrière elle un serveur qui écoute.
struct Serveur {
    processus: Child,
    port: u16,
}

impl Serveur {
    fn lancer(racine: &Path, binaire: &str) -> Self {
        let port = free_port();
        let processus = std::process::Command::new(common::cible().join("debug").join(binaire))
            .current_dir(racine)
            .env("RBS_SERVER__PORT", port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("le binaire du projet doit être lançable");

        // Construit avant l'attente : un serveur qui ne se met jamais à écouter est tué
        // par le `Drop` de la panique, au lieu de survivre au test.
        let mut serveur = Self { processus, port };
        let limite = Instant::now() + Duration::from_secs(60);

        while TcpStream::connect(("127.0.0.1", port)).is_err() {
            if let Ok(Some(sortie)) = serveur.processus.try_wait() {
                panic!("le serveur s'est arrêté avant d'écouter : {sortie}");
            }
            assert!(
                Instant::now() < limite,
                "le serveur n'écoute toujours pas sur {port} après 60 s"
            );
            std::thread::sleep(Duration::from_millis(100));
        }

        serveur
    }
}

impl Drop for Serveur {
    fn drop(&mut self) {
        let _ = self.processus.kill();
        let _ = self.processus.wait();
    }
}

/// Joue `GET chemin` sur le serveur local, et rend le statut avec le corps brut.
///
/// Écrite à la main plutôt que par un client HTTP : un statut et un corps suffisent ici,
/// et la dépendance se paierait sur toute la CI.
fn get(port: u16, chemin: &str) -> (u16, String) {
    let mut flux = TcpStream::connect(("127.0.0.1", port)).expect("le serveur doit répondre");
    write!(
        flux,
        "GET {chemin} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .expect("la requête doit partir");

    let mut reponse = String::new();
    flux.read_to_string(&mut reponse)
        .expect("la réponse doit être lisible");

    let code = reponse
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("réponse sans ligne de statut lisible :\n{reponse}"));
    let corps = reponse
        .split_once("\r\n\r\n")
        .map(|(_, corps)| corps.to_string())
        .unwrap_or_default();

    (code, corps)
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}

/// Le compose du projet, invoqué comme `rbs dev` l'invoque en interne : jamais un
/// `docker run` qui contournerait ce que ces tests doivent prouver.
fn compose(root: &Path, args: &[&str]) -> Command {
    let mut commande = Command::new("docker");
    commande.current_dir(root).arg("compose").args(args);
    commande
}

/// Un port TCP libre, relâché aussitôt : lié puis rendu avant que le compose ne s'en
/// serve, il reste disponible le temps que `docker compose up` le publie.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("l'hôte doit pouvoir prêter un port")
        .local_addr()
        .expect("adresse locale lisible")
        .port()
}

/// Démonte le compose de `root` quand ce garde tombe, succès ou panique confondus.
///
/// `.output()` et non `.assert()` : un `Drop` qui panique pendant qu'un autre panique se
/// déroule déjà ferait avorter le processus de test plutôt que de rapporter l'échec
/// d'origine.
struct ComposeGuard {
    root: PathBuf,
}

impl Drop for ComposeGuard {
    fn drop(&mut self) {
        let _ = compose(&self.root, &["down", "-v"]).output();
    }
}
