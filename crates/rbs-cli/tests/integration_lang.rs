//! Un projet `--lang en` répond à ses clients en anglais, de bout en bout.
//!
//! `cargo test -p rbs-cli --lib` prouve qu'un rendu `en` de chaque gabarit ne contient
//! plus les chaînes françaises visibles du client, et `cargo test -p rbs-core` prouve que
//! `Error::parts` choisit la bonne langue pour chaque statut. Aucun des deux ne prouve
//! qu'un serveur réellement lancé les sert : `Error::parts` lit `lang::current()`, un
//! global de processus posé par `Config::load()`, jamais exercé par un test qui monte le
//! `Router` en mémoire sans passer par le binaire.
//!
//! `webhooks` (qui entraîne `jobs`, `auth`, `rate-limit` et `mail`) et un CRUD réunis dans
//! le même projet couvrent à eux deux les cinq statuts d'erreur qui portent un message :
//! 409 sur un gabarit d'`auth`, 422 sur la validation, 404 et 409 sur deux gabarits du
//! CRUD, 429 sur `rate-limit` — plus la résolution paresseuse qu'expose `cargo run --bin
//! openapi`, qui ne charge jamais la configuration complète.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use assert_cmd::Command;
use rand::TryRng;
use rand::rngs::SysRng;
use serde_json::Value;
use tempfile::TempDir;

mod common;

/// Le mot de passe du compte ouvert par ce test : au-dessus des douze caractères que
/// `RegisterRequest`/`LoginRequest` exigent.
const MOT_DE_PASSE: &str = "un mot de passe assez long";

/// Un second mot de passe, tout aussi valide, mais faux pour le compte enregistré : c'est
/// lui qui use la limite de `/auth/login` jusqu'au premier 429.
const MOT_DE_PASSE_INCORRECT: &str = "ce mot de passe est faux";

/// Les mots français qu'aucune des cinq réponses d'erreur ne doit plus porter — un seul
/// suffirait à trahir une branche `lang` non prise, dans `rbs-core` ou dans un gabarit.
const MOTS_FRANCAIS: [&str; 6] = [
    "introuvable",
    "déjà",
    "requête",
    "réessayez",
    "échouée",
    "interne",
];

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn an_english_project_answers_its_clients_in_english() {
    const EMAIL: &str = "lang-en@exemple.test";

    let postgres = common::start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_lang_en(&common::url_of(&postgres), &parent);

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'à la fin du test.
    let _cible = common::verrou(&common::cible());

    let config = fs::read_to_string(racine.join("config/default.toml")).expect("config lisible");
    assert!(
        config.contains("lang = \"en\""),
        "le projet doit porter sa langue dans `[server]` :\n{config}"
    );

    rbs(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["migrate", "up"])
        .assert()
        .success();

    // `-p demo-api` et non `--all` : les fragments (`add`) ne passent pas par rustfmt, la
    // branche `en` est donc écrite à la main sous la forme qu'il produirait, et c'est ce
    // que ce `--check` éprouve. `migration` en est exclu à dessein — il porte un ordre de
    // modules qui varie déjà selon la seconde où plusieurs migrations sont écrites dans la
    // même exécution, ce qu'`add webhooks` fait en écrire trois : un écart préexistant,
    // sans rapport avec la langue des réponses.
    let fmt = std::process::Command::new("cargo")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["fmt", "--check", "-p", "demo-api"])
        .output()
        .expect("cargo fmt doit se lancer");
    assert!(
        fmt.status.success(),
        "`cargo fmt --check -p demo-api` a échoué :\n{}{}",
        String::from_utf8_lossy(&fmt.stdout),
        String::from_utf8_lossy(&fmt.stderr)
    );

    // Un seul passage avec `--include-ignored` : il rejoue à la fois ce que le projet
    // teste sans base et ce qu'il teste contre PostgreSQL. Sans les lignes qui suivent, un
    // filtre qui ne retiendrait plus aucun de ces tests sortirait quand même en 0.
    let (abouti, journal) =
        cargo_test_brut(&racine, &common::cible(), &["--", "--include-ignored"]);
    assert!(
        abouti,
        "`cargo test --workspace -- --include-ignored` a échoué :\n{journal}"
    );
    for test in [
        "articles::tests::a_replayed_unique_value_returns_409",
        "articles::tests::an_unknown_id_returns_404",
        "modules::webhooks::tests::blocked::an_admin_subscribing_a_private_url_gets_400",
    ] {
        assert!(
            journal.contains(&format!("test {test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{journal}"
        );
    }

    // `cargo run --bin openapi` ne passe jamais par `Config::load()`, donc jamais par
    // `.env` : c'est la résolution paresseuse de `lang::current()`, sur la seule cascade
    // de fichiers, qui doit choisir l'anglais ici.
    let sortie = std::process::Command::new("cargo")
        .current_dir(&racine)
        .env("CARGO_TARGET_DIR", common::cible())
        .args(["run", "--quiet", "--bin", "openapi"])
        .output()
        .expect("cargo run doit se lancer");
    assert!(
        sortie.status.success(),
        "`cargo run --bin openapi` a échoué :\n{}",
        String::from_utf8_lossy(&sortie.stderr)
    );
    let document: Value =
        serde_json::from_slice(&sortie.stdout).expect("le document OpenAPI doit être du JSON");
    assert_eq!(
        document["components"]["responses"]["NotFound"]["description"], "resource not found",
        "{document:#}"
    );
    assert_eq!(
        document["paths"]["/articles"]["get"]["responses"]["500"]["description"], "internal error",
        "{document:#}"
    );

    compile(&racine, &common::cible());

    let serveur = Serveur::lancer(&racine, "demo-api", "info");
    let port = serveur.port();

    let (statut, corps) = request(
        port,
        "POST",
        "/auth/register",
        None,
        Some(&credentials(EMAIL, MOT_DE_PASSE)),
    );
    assert_eq!(statut, 202, "l'inscription doit aboutir : {corps}");

    // Une adresse prise rend le même 202, sans message à traduire : le conflit que ce
    // parcours éprouve en anglais est celui du CRUD, plus bas.
    let (statut, corps) = request(
        port,
        "POST",
        "/auth/register",
        None,
        Some(&credentials(EMAIL, MOT_DE_PASSE)),
    );
    assert_eq!(
        statut, 202,
        "la même adresse doit rendre le même 202 : {corps}"
    );

    let (statut, corps) = request(
        port,
        "POST",
        "/auth/register",
        None,
        Some(&credentials("pas-un-email", MOT_DE_PASSE)),
    );
    assert_eq!(statut, 422, "une adresse mal formée doit échouer : {corps}");
    assert_eq!(corps["title"], "Validation failed");
    assert_anglais(&corps);

    let (statut, paire) = request(
        port,
        "POST",
        "/auth/login",
        None,
        Some(&credentials(EMAIL, MOT_DE_PASSE)),
    );
    assert_eq!(statut, 200, "la connexion doit rendre une paire : {paire}");
    let jeton = access(&paire);

    let (statut, corps) = request(
        port,
        "GET",
        &format!("/articles/{}", uuid_v4_aleatoire()),
        Some(&jeton),
        None,
    );
    assert_eq!(
        statut, 404,
        "un identifiant inconnu doit rendre 404 : {corps}"
    );
    assert_eq!(corps["title"], "Not Found");
    assert_eq!(corps["detail"], "article not found");
    assert_anglais(&corps);

    const ARTICLE: &str =
        r#"{"title":"un article du parcours","body":"un corps.","published":true}"#;

    let (statut, corps) = request(port, "POST", "/articles", Some(&jeton), Some(ARTICLE));
    assert_eq!(statut, 201, "la première création doit aboutir : {corps}");

    let (statut, corps) = request(port, "POST", "/articles", Some(&jeton), Some(ARTICLE));
    assert_eq!(
        statut, 409,
        "le même `title` doit heurter la contrainte `unique` : {corps}"
    );
    assert_eq!(corps["title"], "Conflict");
    assert_eq!(corps["detail"], "this value is already taken");
    assert_anglais(&corps);

    // `/auth/login` tolère cinq requêtes par soixante secondes ; la connexion réussie
    // ci-dessus en a déjà compté une. Dix tentatives ratées suffisent largement à
    // atteindre la sixième requête qui bascule en 429, sans dépendre d'une fenêtre qui
    // s'écoule pendant que le test tourne.
    let mut refus = None;
    for _ in 0..10 {
        let (statut, corps) = request(
            port,
            "POST",
            "/auth/login",
            None,
            Some(&credentials(EMAIL, MOT_DE_PASSE_INCORRECT)),
        );
        if statut == 429 {
            refus = Some(corps);
            break;
        }
    }
    let corps = refus.unwrap_or_else(|| panic!("aucune 429 reçue en dix tentatives"));
    assert_eq!(corps["title"], "too_many_requests");
    assert_eq!(corps["detail"], "too many requests: try again later");
    assert_anglais(&corps);
}

/// Un projet `--lang en` portant `webhooks` (donc `jobs`, `auth`, `rate-limit` et `mail`)
/// et un CRUD `articles`, sa base pointée sur `url`.
fn project_lang_en(url: &str, parent: &TempDir) -> PathBuf {
    let racine = parent.path().join("demo-api");

    rbs(parent.path())
        .args([
            "new",
            "demo-api",
            "--lang",
            "en",
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

    common::commiter(&racine, "projet neuf");

    rbs(&racine).args(["add", "webhooks"]).assert().success();

    rbs(&racine)
        .args([
            "generate",
            "crud",
            "articles",
            "--fields",
            "title:string:unique,body:text,published:bool",
            "--force",
        ])
        .assert()
        .success();

    racine
}

/// Joue `cargo test` dans le projet et rend son issue et ses deux flux réunis.
fn cargo_test_brut(racine: &Path, cible: &Path, arguments: &[&str]) -> (bool, String) {
    let output = std::process::Command::new("cargo")
        .current_dir(racine)
        .env("CARGO_TARGET_DIR", cible)
        .arg("test")
        .arg("--workspace")
        .args(arguments)
        .output()
        .expect("cargo doit se lancer");

    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    (output.status.success(), journal)
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}

/// Un UUID v4 syntaxiquement valide, sans dépendre de la crate `uuid` : seuls les nibbles
/// de version et de variante sont fixés, le reste vient du générateur du système — le même
/// que `secret::tire_au_hasard`.
fn uuid_v4_aleatoire() -> String {
    let mut octets = [0u8; 16];
    SysRng
        .try_fill_bytes(&mut octets)
        .expect("le générateur du système doit être disponible");

    octets[6] = (octets[6] & 0x0f) | 0x40;
    octets[8] = (octets[8] & 0x3f) | 0x80;

    let hex: String = octets.iter().map(|octet| format!("{octet:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Le corps d'inscription et de connexion d'un compte.
fn credentials(email: &str, password: &str) -> String {
    format!(r#"{{"email":"{email}","password":"{password}"}}"#)
}

/// Le jeton d'accès d'une paire.
fn access(paire: &Value) -> String {
    paire["access_token"]
        .as_str()
        .unwrap_or_else(|| panic!("la paire doit porter `access_token` : {paire}"))
        .to_owned()
}

/// Un corps d'erreur entièrement anglais : de l'ASCII, et aucun des mots français que la
/// branche `fr` des mêmes gabarits aurait laissés passer.
fn assert_anglais(corps: &Value) {
    let texte = corps.to_string();
    assert!(texte.is_ascii(), "le corps n'est pas ASCII : {texte}");
    for mot in MOTS_FRANCAIS {
        assert!(
            !texte.contains(mot),
            "le corps porte le mot français « {mot} » : {texte}"
        );
    }
}

/// Compile le projet, binaire compris.
fn compile(racine: &Path, cible: &Path) {
    Command::new("cargo")
        .current_dir(racine)
        .env("CARGO_TARGET_DIR", cible)
        .arg("build")
        .assert()
        .success();
}

/// Un port que personne n'écoute au moment de l'appel.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("l'hôte doit pouvoir prêter un port")
        .local_addr()
        .expect("adresse locale lisible")
        .port()
}

/// Attend que le serveur accepte les connexions sur `port`.
fn wait_for_listening(port: u16) {
    let limite = Instant::now() + Duration::from_secs(60);

    while Instant::now() < limite {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    panic!("le serveur n'écoute toujours pas sur {port} après 60 s");
}

/// Le binaire d'un projet, lancé sur un port libre, arrêté quand ce garde tombe.
///
/// `Drop` plutôt qu'un `kill` en fin de test : une assertion qui échoue au milieu d'un
/// parcours déroule la pile sans jamais l'atteindre, et laisse derrière elle un serveur
/// qui écoute et un conteneur qu'il tient ouvert.
struct Serveur {
    processus: Option<std::process::Child>,
    port: u16,
}

impl Serveur {
    fn lancer(racine: &Path, binaire: &str, journal: &str) -> Self {
        let port = free_port();

        let processus = std::process::Command::new(common::cible().join("debug").join(binaire))
            .current_dir(racine)
            .env("RBS_SERVER__PORT", port.to_string())
            .env("RUST_LOG", journal)
            // Ce parcours éprouve la langue des réponses et non la preuve d'adresse, que
            // `integration_auth` couvre : il se connecte dès l'inscription.
            .env("RBS_AUTH__LOGIN_REQUIRES_VERIFICATION", "false")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("le binaire du projet doit être lançable");

        wait_for_listening(port);

        Self {
            processus: Some(processus),
            port,
        }
    }

    fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for Serveur {
    fn drop(&mut self) {
        if let Some(processus) = self.processus.as_mut() {
            let _ = processus.kill();
            let _ = processus.wait();
        }
    }
}

/// Joue une requête sur le serveur local et rend son statut avec son corps décodé.
///
/// La requête est écrite à la main plutôt que par un client HTTP : ce parcours n'a besoin
/// que d'un statut et de deux champs de JSON, et la dépendance se paierait sur toute la CI.
fn request(
    port: u16,
    methode: &str,
    chemin: &str,
    jeton: Option<&str>,
    corps: Option<&str>,
) -> (u16, Value) {
    let mut flux = TcpStream::connect(("127.0.0.1", port)).expect("le serveur doit répondre");

    let corps = corps.unwrap_or_default();
    let mut entete =
        format!("{methode} {chemin} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");

    if let Some(jeton) = jeton {
        entete.push_str(&format!("Authorization: Bearer {jeton}\r\n"));
    }

    if !corps.is_empty() {
        entete.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            corps.len()
        ));
    }

    entete.push_str("\r\n");
    entete.push_str(corps);

    flux.write_all(entete.as_bytes())
        .expect("la requête doit partir");

    let mut reponse = String::new();
    flux.read_to_string(&mut reponse)
        .expect("la réponse doit être lisible");

    decode(&reponse)
}

/// Sépare le statut du corps d'une réponse HTTP brute.
fn decode(reponse: &str) -> (u16, Value) {
    let statut = reponse
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("réponse sans ligne de statut lisible :\n{reponse}"));

    let corps = reponse
        .split_once("\r\n\r\n")
        .map(|(_, corps)| corps)
        .unwrap_or_default()
        .trim();

    // Le message d'un serveur qui refuse la requête avant de la router n'est pas
    // nécessairement du JSON : rendre le texte brut plutôt que d'échouer ici laisse
    // l'assertion appelante afficher ce que le serveur a réellement dit.
    let corps = serde_json::from_str(corps).unwrap_or_else(|_| Value::String(corps.to_string()));

    (statut, corps)
}
