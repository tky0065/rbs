//! `rbs add frontend` sur un projet neuf, puis sa compilation et celle de son client.
//!
//! Deux suites, parce que les deux chaînes n'ont rien en commun. La première compile le
//! Rust : elle prouve que le module sert la page d'amorçage par le routeur réel du projet,
//! avec la documentation OpenAPI et la sonde de santé à côté d'elle. La seconde installe
//! les dépendances du client, vérifie ses types et le construit.
//!
//! La première vaut aussi pour ce qu'elle ne fait pas : `cargo` ne lance ni `npm`, ni
//! `node`, ni aucun outil du client. Un projet qui a installé le frontend compile sur une
//! machine qui n'a pas Node — c'est la contrepartie du mécanisme purement déclaratif des
//! fragments.
//!
//! Pas de conteneur : le repli ne joint aucun service, et l'état se monte sur une
//! connexion non établie. Le `#[ignore]` de la première ne tient qu'à la compilation d'un
//! projet Axum + SeaORM complet ; celui de la seconde au registre npm, qu'il faut
//! joindre.
//!
//! La seconde est le seul endroit du dépôt où du TypeScript est compilé. Sans elle, le
//! fragment livrerait des fichiers que rien n'a jamais vérifiés.
//!
//! Une troisième suit la même chaîne pour le shell d'administration. Elle est la moitié
//! manquante d'une paire : la seconde prouve que le socle seul se vérifie — son routeur
//! cherche un montage et n'en trouve aucun —, celle-ci qu'il se vérifie encore une fois
//! le montage posé. Aucun autre endroit ne regarde les deux côtés de cette découverte.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command as Processus, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Les tests que le fragment livre et qui portent ses deux promesses : la page d'amorçage
/// servie par le routeur réel, et le service qui ne masque rien.
const PROMESSES: [&str; 4] = [
    "the_project_router_serves_the_bootstrap_page_at_its_root",
    "the_project_router_keeps_its_probe_and_its_openapi_document",
    "a_mounted_route_is_never_reached_by_the_fallback",
    "the_build_erases_the_bootstrap_page_as_soon_as_it_exists",
];

#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_fragment_compiles_and_serves_its_bootstrap_page() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet).args(["add", "frontend"]).assert().success();

    // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
    // premier cargo et se tient jusqu'au dernier.
    let _cible = common::verrou(&common::cible());

    let output = Processus::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        // Filtré sur le module : les tests de santé du squelette exigeraient une base de
        // données, que ce fragment-ci n'a aucune raison de faire monter.
        .args(["test", "--lib", "modules::frontend"])
        .output()
        .expect("cargo doit se lancer");

    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "les tests du frontend ont échoué :\n{journal}"
    );

    // Un filtre qui ne retient aucun test sort en 0 : sans ces lignes, un fragment qui
    // cesserait de livrer ses tests laisserait celui-ci au vert sans que rien n'ait servi
    // la moindre page.
    for test in PROMESSES {
        assert!(
            journal.contains(&format!("test modules::frontend::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{journal}"
        );
    }
}

/// La chaîne Node, du registre au build servi.
///
/// Une seule suite pour les quatre gestes : l'installation seule coûte la moitié du temps
/// du test, et la découper en autant de `#[test]` la paierait autant de fois.
#[test]
#[ignore = "installe les dépendances du client depuis le registre npm : lent et en ligne"]
fn the_client_installs_typechecks_builds_and_proxies_the_api() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet).args(["add", "frontend"]).assert().success();

    let client = projet.join("frontend");
    // `npm ci` demanderait un fichier de verrouillage, qu'un fragment ne peut pas livrer :
    // il est le produit d'une installation, et le versionner figerait chez l'utilisateur
    // l'arbre résolu le jour où la template a été écrite.
    npm(&client, &["install", "--no-audit", "--no-fund"]);
    npm(&client, &["run", "typecheck"]);
    npm(&client, &["run", "build"]);

    // Le build sort là où la section `[frontend]` de la configuration dit au binaire de
    // regarder, et le chemin se lit dans cette configuration plutôt qu'il ne se recopie
    // ici : c'est cette égalité-là qui fait disparaître la page d'amorçage, et elle est le
    // seul point où les deux moitiés du fragment se touchent.
    let index = projet
        .join(configure(&projet, "dir"))
        .join(configure(&projet, "index"));
    let rendu = std::fs::read_to_string(&index)
        .unwrap_or_else(|_| panic!("{} doit exister après le build", index.display()));
    assert!(
        rendu.contains("<div id=\"app\">"),
        "l'index construit ne monte pas l'application :\n{rendu}"
    );

    // Le nom du projet a traversé toute la chaîne — la génération, le moteur de template,
    // le compilateur Vue, l'empaqueteur — et se lit dans ce qui part au navigateur.
    let bundles = std::fs::read_dir(client.join("dist/assets"))
        .expect("le build écrit ses assets")
        .filter_map(Result::ok)
        .map(|entree| std::fs::read_to_string(entree.path()).unwrap_or_default())
        .collect::<String>();
    assert!(
        bundles.contains("demo-api"),
        "le nom du projet n'a pas atteint le bundle"
    );

    // La galerie part dans son propre morceau, parce que la route la charge paresseusement.
    // Le vérifier ici est ce qui prouve qu'elle a compilé : le reste de la suite passerait
    // aussi bien sur un socle qui ne l'aurait jamais montée.
    let morceaux = std::fs::read_dir(client.join("dist/assets"))
        .expect("le build écrit ses assets")
        .filter_map(Result::ok)
        .map(|entree| entree.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(
        morceaux.iter().any(|nom| nom.starts_with("Galerie-")),
        "la galerie n'est pas dans le build :\n{morceaux:?}"
    );

    proxy_atteint_l_api(&client);
}

/// Le shell d'administration, du fragment posé au morceau construit.
///
/// La chaîne entière en une suite, parce qu'elle n'a qu'un enchaînement : le shell importe
/// un client que le fragment ne livre pas — il sort du document OpenAPI de ce projet-ci —
/// et la vérification des types ne veut rien dire tant qu'il n'est pas là. Engendrer ce
/// client demande de compiler le projet, ce qui est aussi la seule preuve que le contrat
/// lu est celui que le binaire publie.
#[test]
#[ignore = "compile le projet engendré, puis installe les dépendances du client : lent et en ligne"]
fn the_admin_shell_generates_its_client_typechecks_and_builds() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet)
        .args(["add", "frontend-admin"])
        .assert()
        .success();

    let client = projet.join("frontend/src/api/client.ts");
    assert!(
        !client.exists(),
        "le fragment livre un client figé : il mentirait dès la première route ajoutée"
    );

    {
        // La cible est partagée par tous les binaires de `tests/`, et `rbs generate
        // client` lance cargo : le verrou se prend avant, et se rend avant npm, qui n'en
        // a que faire.
        let _cible = common::verrou(&common::cible());

        rbs(&projet)
            .env("CARGO_TARGET_DIR", common::cible())
            .args([
                "generate",
                "client",
                "--lang",
                "ts",
                "--out",
                "frontend/src/api",
            ])
            .assert()
            .success();
    }

    let engendre = std::fs::read_to_string(&client).expect("le client doit être engendré");
    for methode in ["authLogin(", "authRefresh(", "authLogout(", "authMe("] {
        assert!(
            engendre.contains(methode),
            "`{methode}` manque au client : le shell ne compilera pas\n{engendre}"
        );
    }

    let repertoire = projet.join("frontend");
    npm(&repertoire, &["install", "--no-audit", "--no-fund"]);
    npm(&repertoire, &["run", "typecheck"]);
    npm(&repertoire, &["run", "build"]);

    // Le shell part dans ses propres morceaux : le visiteur de l'accueil ne télécharge
    // jamais l'administration, et les voir sortir prouve qu'ils ont compilé — un montage
    // que le routeur du socle n'aurait pas trouvé passerait la vérification des types
    // sans laisser une ligne dans le build.
    let morceaux: Vec<String> = std::fs::read_dir(repertoire.join("dist/assets"))
        .expect("le build écrit ses assets")
        .filter_map(Result::ok)
        .map(|entree| entree.file_name().to_string_lossy().into_owned())
        .collect();
    for ecran in ["Shell-", "Connexion-"] {
        assert!(
            morceaux.iter().any(|nom| nom.starts_with(ecran)),
            "`{ecran}` n'est pas dans le build :\n{morceaux:?}"
        );
    }

    // Un morceau séparé par écran ne suffit pas : le routeur du socle lit le montage sans
    // attendre, et une importation statique le long de cette chaîne — la garde, puis la
    // couche d'état, puis le client engendré — reviendrait à livrer toute l'administration
    // dans le morceau d'entrée sans qu'aucun nom de fichier ne le dise.
    //
    // La route de réinitialisation est le témoin : le client l'expose, et seul
    // l'espace d'administration l'appelle.
    let entree = std::fs::read_dir(repertoire.join("dist/assets"))
        .expect("le build écrit ses assets")
        .filter_map(Result::ok)
        .find(|entree| {
            let nom = entree.file_name().to_string_lossy().into_owned();
            nom.starts_with("index-") && nom.ends_with(".js")
        })
        .expect("le build écrit un morceau d'entrée");
    let entree = std::fs::read_to_string(entree.path()).expect("le morceau d'entrée se lit");
    assert!(
        !entree.contains("forgot-password"),
        "le client engendré part dans le morceau d'entrée, que télécharge le visiteur de \
         l'accueil"
    );

    // Et les textes du shell ont traversé la chaîne entière : la génération, le moteur de
    // template, le compilateur Vue et l'empaqueteur.
    //
    // Le libellé est relu dans le projet plutôt que recopié ici : le fragment en porte un
    // par langue, et la langue du projet est celle de sa création. Une chaîne écrite en
    // dur ne vaudrait que d'un côté, et passerait pour une régression de l'autre.
    let bundles = std::fs::read_dir(repertoire.join("dist/assets"))
        .expect("le build écrit ses assets")
        .filter_map(Result::ok)
        .map(|entree| std::fs::read_to_string(entree.path()).unwrap_or_default())
        .collect::<String>();
    let refus = libelle(&repertoire.join("src/admin/textes.ts"), "refuse");
    assert!(
        bundles.contains(&refus),
        "`{refus}` ne part pas au navigateur"
    );
}

/// La valeur du libellé `cle`, tronquée à son premier caractère non ASCII.
///
/// L'empaqueteur échappe ce qui sort de l'ASCII, et la comparaison porterait alors sur
/// deux écritures du même mot. Le préfixe suffit à distinguer les deux langues.
fn libelle(textes: &Path, cle: &str) -> String {
    let source = std::fs::read_to_string(textes).expect("les libellés du shell se lisent");
    let valeur = source
        .split(&format!("{cle}: '"))
        .nth(1)
        .unwrap_or_else(|| panic!("`{cle}` absent de {} :\n{source}", textes.display()))
        .split('\'')
        .next()
        .expect("le littéral se referme");

    valeur
        .chars()
        .take_while(char::is_ascii)
        .collect::<String>()
}

/// Le développement sur un port distinct atteint l'API par le relais.
///
/// Sans lui, le client appellerait `/health` sur le port de Vite et recevrait
/// l'application en retour ; en visant le port du binaire, il se heurterait à l'origine.
/// Une API postiche suffit à le prouver : ce qui est en cause est le relais, non ce qu'il
/// relaie.
fn proxy_atteint_l_api(client: &Path) {
    let api = TcpListener::bind("127.0.0.1:0").expect("l'API postiche doit s'ouvrir");
    let port_api = api.local_addr().expect("l'adresse est connue").port();
    let postiche = std::thread::spawn(move || repond_une_fois(&api, "{\"status\":\"ok\"}"));

    // Un port libre, pris puis rendu : `--strictPort` fait échouer Vite plutôt que glisser
    // sur le suivant, ce qui rendrait la suite muette sur ce qu'elle a réellement joint.
    let port_client = TcpListener::bind("127.0.0.1:0")
        .and_then(|prise| prise.local_addr())
        .expect("un port libre doit se trouver")
        .port();

    // `node` sur le binaire de Vite, et non `npm run dev` : `npm` n'est qu'un lanceur, et
    // le tuer laisserait le serveur derrière lui.
    let mut serveur = Vite(
        Processus::new("node")
            .current_dir(client)
            .env("RBS_API_URL", format!("http://127.0.0.1:{port_api}"))
            // `--host` explicite : par défaut Vite écoute `localhost`, que cette machine
            // résout d'abord en IPv6, et le test viserait une adresse où rien n'écoute.
            .args([
                "node_modules/vite/bin/vite.js",
                "--host",
                "127.0.0.1",
                "--port",
                &port_client.to_string(),
                "--strictPort",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("le serveur de développement doit se lancer"),
    );

    // La racine d'abord, pour savoir que Vite écoute : l'API postiche ne répond qu'une
    // fois, et une attente qui passerait par le relais la consommerait avant la mesure.
    attendre(port_client, "/").expect("le serveur de développement doit démarrer");
    let corps = interroge(port_client, "/health").expect("le relais doit répondre");
    serveur.0.kill().expect("le serveur s'arrête");

    assert!(
        corps.contains("{\"status\":\"ok\"}"),
        "le relais n'a pas atteint l'API :\n{corps}"
    );
    postiche.join().expect("l'API postiche se referme");
}

/// Un serveur de développement qu'on abat quel que soit le chemin de sortie du test.
///
/// Sans ce garde, une assertion rompue entre le lancement et l'arrêt laisserait un Vite
/// vivant, sur un port que la suite suivante croirait libre.
struct Vite(Child);

impl Drop for Vite {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Interroge `uri` jusqu'à ce que le serveur de développement rende un 200.
///
/// Le premier appel arrive avant que Vite n'écoute, et les suivants pendant qu'il résout
/// ses dépendances : c'est une attente, pas une tentative.
fn attendre(port: u16, uri: &str) -> Option<String> {
    let limite = Instant::now() + Duration::from_secs(60);

    while Instant::now() < limite {
        match interroge(port, uri) {
            Some(reponse) if reponse.starts_with("HTTP/1.1 200") => return Some(reponse),
            _ => std::thread::sleep(Duration::from_millis(250)),
        }
    }

    None
}

/// Un `GET uri` en HTTP/1.1, réponse entière rendue telle quelle, statut compris.
fn interroge(port: u16, uri: &str) -> Option<String> {
    let requete = format!("GET {uri} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    let mut prise = TcpStream::connect(("127.0.0.1", port)).ok()?;
    prise
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("la borne de lecture se pose");
    prise.write_all(requete.as_bytes()).ok()?;

    let mut reponse = String::new();
    prise.read_to_string(&mut reponse).ok()?;

    Some(reponse)
}

/// Répond une fois, puis se tait : le relais n'est interrogé qu'une fois.
fn repond_une_fois(prise: &TcpListener, corps: &str) {
    let (mut flux, _) = prise.accept().expect("le relais doit se connecter");

    // La requête se lit jusqu'à sa ligne vide : sans cela, la réponse partirait pendant
    // que le relais écrit encore, et certains le prennent pour une connexion rompue.
    let mut entete = BufReader::new(flux.try_clone().expect("le flux se dédouble"));
    let mut ligne = String::new();
    while entete.read_line(&mut ligne).unwrap_or(0) > 0 {
        if ligne == "\r\n" || ligne == "\n" {
            break;
        }
        ligne.clear();
    }

    let _ = flux.write_all(
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{corps}",
            corps.len()
        )
        .as_bytes(),
    );
}

/// Une clé de la section `[frontend]` de la configuration du projet engendré.
///
/// Lue plutôt que recopiée : le test prouve alors que le build sort là où le binaire
/// regarde, et non que deux chaînes écrites côte à côte se ressemblent.
fn configure(projet: &Path, cle: &str) -> String {
    let configuration = std::fs::read_to_string(projet.join("config/default.toml"))
        .expect("la configuration du projet se lit");
    let section = configuration
        .split("[frontend]")
        .nth(1)
        .expect("le fragment a posé sa section");

    section
        .lines()
        .take_while(|ligne| !ligne.starts_with('['))
        .find_map(|ligne| ligne.strip_prefix(&format!("{cle} = ")))
        .unwrap_or_else(|| panic!("`{cle}` manque à la section [frontend] :\n{section}"))
        .trim_matches('"')
        .to_string()
}

/// `npm` dans le répertoire du client, sa sortie rendue au test quand il échoue.
fn npm(client: &Path, arguments: &[&str]) {
    let output = Processus::new("npm")
        .current_dir(client)
        .args(arguments)
        .output()
        .unwrap_or_else(|faute| panic!("npm {} doit se lancer : {faute}", arguments.join(" ")));

    assert!(
        output.status.success(),
        "npm {} a échoué :\n{}{}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}
