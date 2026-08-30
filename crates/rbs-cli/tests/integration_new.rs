//! Le seul test qui prouve que rbs fonctionne : il invoque le binaire livré, pas
//! `new::creer`, et compile ce que ce binaire a produit.

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

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

    assert!(root.join("src/auth/service.rs").is_file());
    assert!(root.join("src/cache/mod.rs").is_file());

    let compose_yml = fs::read_to_string(root.join("docker-compose.yml")).expect("compose lisible");
    assert!(compose_yml.contains("redis:8-alpine"), "{compose_yml}");

    Command::new("docker")
        .current_dir(&root)
        .args(["compose", "config"])
        .assert()
        .success();

    Command::new("cargo")
        .current_dir(&root)
        .env("CARGO_TARGET_DIR", common::cible())
        .arg("build")
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
