//! Le seul test qui prouve que rbs fonctionne : il invoque le binaire livré, pas
//! `new::creer`, et compile ce que ce binaire a produit.

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Les tests qu'un CRUD engendré livre au projet, et qui joignent tous la base.
const TESTS_DU_CRUD: [&str; 4] = [
    "articles::tests::the_full_lifecycle_goes_through_the_api",
    "articles::tests::two_creations_in_a_row_carry_increasing_ids",
    "articles::tests::an_unknown_id_returns_404",
    "articles::tests::an_unreadable_body_returns_400",
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
    for racine_de_modules in ["src/main.rs", "src/seeds/main.rs", "migration/src/lib.rs"] {
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

        Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(&projet)
            .args([
                "generate",
                "crud",
                "articles",
                "--fields",
                "title:string,body:text,published:bool",
                "--yes",
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

        let sortie = Command::new("cargo")
            .current_dir(&projet)
            .env("CARGO_TARGET_DIR", &cible)
            .args(["test", "--workspace"])
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
