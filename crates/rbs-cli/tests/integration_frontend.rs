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
///
/// Le socle porte le module qui instancie le client d'API : il importe donc le **client
/// engendré**, et la vérification des types ne veut rien dire tant qu'il n'est pas là.
/// C'est ce qui fait compiler le projet ici, sur un socle sans shell — et c'est aussi le
/// seul endroit qui prouve que la commande écrit dans l'arbre du client sans qu'on le lui
/// dise.
#[test]
#[ignore = "compile le projet engendré, puis installe les dépendances du client : lent et en ligne"]
fn the_client_installs_typechecks_builds_and_proxies_the_api() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet).args(["add", "frontend"]).assert().success();

    {
        // La cible est partagée par tous les binaires de `tests/` : elle se prend avant le
        // cargo qu'engendre la lecture du contrat, et se rend avant npm, qui n'en a que
        // faire.
        let _cible = common::verrou(&common::cible());

        // Sans `--out` : le répertoire par défaut est celui où le socle importe son
        // client dès que le fragment `frontend` est posé.
        rbs(&projet)
            .env("CARGO_TARGET_DIR", common::cible())
            .args(["generate", "client", "--lang", "ts"])
            .assert()
            .success();
    }

    let engendre = projet.join("frontend/src/api/client.ts");
    assert!(
        engendre.exists(),
        "le client n'est pas tombé dans l'arbre du socle : {}",
        engendre.display()
    );

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
    let lit = |suffixe: &str| {
        std::fs::read_dir(client.join("dist/assets"))
            .expect("le build écrit ses assets")
            .filter_map(Result::ok)
            .filter(|entree| entree.file_name().to_string_lossy().ends_with(suffixe))
            .map(|entree| std::fs::read_to_string(entree.path()).unwrap_or_default())
            .collect::<String>()
    };
    let scripts = lit(".js");
    let feuilles = lit(".css");
    assert!(
        scripts.contains("demo-api"),
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

    // La variante sombre s'allume ici aussi, sans le shell d'administration : c'est le
    // socle qui lit la préférence du système et pose la classe, et le second jeu de
    // valeurs part au navigateur avec le premier. Sans ces quatre témoins, la moitié de la
    // charte serait du code mort partout où le shell n'est pas posé.
    //
    // Le sélecteur et la valeur sont cherchés dans la feuille construite, la lecture de la
    // préférence et la pose de la classe dans les scripts : un témoin cherché dans tout le
    // build passerait au vert sur la seule présence du CSS.
    for temoin in [".sombre", "#16150f"] {
        assert!(
            feuilles.contains(temoin),
            "`{temoin}` ne part pas au navigateur : le second jeu de valeurs n'existe pas"
        );
    }
    // La chaîne du guillemet ne s'écrit pas : l'empaqueteur récrit les littéraux en
    // gradins inverses, et un test qui figerait le guillemet figerait son minificateur.
    for temoin in [
        "(prefers-color-scheme: dark)",
        "classList.toggle(",
        "sombre",
    ] {
        assert!(
            scripts.contains(temoin),
            "`{temoin}` ne part pas au navigateur : rien n'allume la variante sombre"
        );
    }

    proxy_atteint_l_api(&client);
}

/// Le shell d'administration, du fragment posé au morceau construit.
///
/// La chaîne entière en une suite, parce qu'elle n'a qu'un enchaînement : le shell importe
/// un client que le fragment ne livre pas — il sort du document OpenAPI de ce projet-ci —
/// et la vérification des types ne veut rien dire tant qu'il n'est pas là. Engendrer ce
/// client demande de compiler le projet, ce qui est aussi la seule preuve que le contrat
/// lu est celui que le binaire publie.
///
/// Le client n'est engendré à la main qu'une fois, et **avant** la table : c'est le
/// terrain sans lequel `generate crud` n'a rien à refaire, comme elle saute les ancres du
/// frontend plutôt que de les exiger. Aucune commande ne le retouche ensuite, et il porte
/// pourtant les méthodes de la table — c'est la génération d'entité qui les y a écrites,
/// depuis le contrat. Un `rbs generate client` de plus entre la table et la vérification
/// des types rendrait la suite verte sans que rien ne l'ait prouvé.
#[test]
#[ignore = "compile deux fois le projet engendré, puis installe les dépendances du client : lent et en ligne"]
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
        // La cible est partagée par tous les binaires de `tests/`, et tout ce bloc lance
        // cargo — la génération de client comme celle de la table, qui relit le contrat :
        // le verrou se prend avant le premier, et se rend avant npm, qui n'en a que faire.
        let _cible = common::verrou(&common::cible());

        // Sans `--out` : le défaut de la commande est le répertoire où le socle importe
        // son client, dès que le fragment `frontend` est posé.
        rbs(&projet)
            .env("CARGO_TARGET_DIR", common::cible())
            .args(["generate", "client", "--lang", "ts"])
            .assert()
            .success();

        // Ce client-là sort d'un contrat où la table n'existe pas encore. Le relire ici
        // est ce qui donne son sens à tout ce qui suit : les méthodes de `bordereaux`
        // n'ont, à cet instant, aucune raison d'y être.
        let amorce = std::fs::read_to_string(&client).expect("le client doit être engendré");
        assert!(
            !amorce.contains("bordereauxFilter("),
            "le client d'amorce porte déjà la table : la suite ne prouverait plus rien\n{amorce}"
        );

        // Une table réelle, et ses écrans engendrés : c'est le second producteur de l'écran
        // patron, et le seul endroit du dépôt où son rendu passe par le compilateur. Les
        // types couverts sont ceux dont chaque contrôle du formulaire dépend — chaîne,
        // texte long, entier, décimal, booléen, date, instant, énumération, colonne
        // facultative.
        rbs(&projet)
            .env("CARGO_TARGET_DIR", common::cible())
            .args([
                "generate",
                "crud",
                "bordereaux",
                "--fields",
                "titre:string,corps:text,vues:int,prix:decimal,publie:bool,paru:date,vu:datetime,\
                 statut:enum(draft,published),note:string:optional",
            ])
            .assert()
            .success();

        // Et une table qui les refuse : le drapeau ne doit rien laisser derrière lui. Sans
        // écran, pas de client à refaire non plus — celui-ci ne compile rien.
        rbs(&projet)
            .env("CARGO_TARGET_DIR", common::cible())
            .args([
                "generate",
                "crud",
                "jetons",
                "--fields",
                "valeur:string",
                "--no-admin",
            ])
            .assert()
            .success();
    }

    assert!(
        !projet.join("frontend/src/admin/vues/Jetons.vue").exists(),
        "`--no-admin` a laissé un écran"
    );

    let engendre = std::fs::read_to_string(&client).expect("le client doit être engendré");
    for methode in [
        "authLogin(",
        "authRefresh(",
        "authLogout(",
        "authMe(",
        "authListSessions(",
        "authRevokeSession(",
        "authRevokeSessions(",
        "authChangePassword(",
        "authUpdateMe(",
        "authRegister(",
        "authRegistrationStatus(",
        "authForgotPassword(",
        "authResetPassword(",
        "authVerifyEmail(",
        "authResendVerification(",
        "health(",
        "bordereauxFilter(",
        "bordereauxFind(",
        "bordereauxCreate(",
        "bordereauxUpdate(",
        "bordereauxDelete(",
    ] {
        assert!(
            engendre.contains(methode),
            "`{methode}` manque au client : le shell ne compilera pas\n{engendre}"
        );
    }

    // Et l'inverse : chaque appel que le shell écrit correspond à une méthode que le
    // client publie. La liste ci-dessus est un plancher, que l'ajout d'un écran ne
    // relèverait pas — celle-ci se relit toute seule, et c'est elle qui attrape un appel
    // à une route que le contrat n'expose pas.
    let appels = appels_du_shell(&projet.join("frontend/src"));
    // Un relevé vide passerait la boucle sans rien prouver : le shell appelle au moins ce
    // par quoi il ouvre une session.
    assert!(
        appels.iter().any(|appel| appel == "authLogin"),
        "le relevé des appels du shell n'a rien trouvé : {appels:?}"
    );
    for appel in appels {
        assert!(
            engendre.contains(&format!("{appel}(")),
            "le shell appelle `api.{appel}()`, que le contrat n'expose pas\n{engendre}"
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
    for ecran in [
        "Shell-",
        "Connexion-",
        "Inscription-",
        "Reinitialisation-",
        "Verification-",
        "TableauDeBord-",
        "Sessions-",
        "Profil-",
        "Demonstration-",
        "Bordereaux-",
    ] {
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
    // Le témoin est le *nom de méthode* du client, et non le chemin `/forgot-password` :
    // depuis que les liens des courriels sont portés en `alias` par les écrans publics, ce
    // chemin est une chaîne de la table de routage, que le montage met de toute façon dans
    // le morceau d'entrée. Seul le client engendré écrit `authResetPassword`.
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
        !entree.contains("authResetPassword"),
        "le client engendré part dans le morceau d'entrée, que télécharge le visiteur de \
         l'accueil"
    );

    // L'écran de démonstration non plus : c'est le patron dont sortiront tous les écrans
    // engendrés, et une importation statique le long de sa chaîne les ferait tous
    // descendre chez le visiteur de l'accueil, table par table.
    assert!(
        !entree.contains("DEM-001"),
        "l'écran de démonstration part dans le morceau d'entrée"
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

    // Et l'écran patron a bien été compilé pour de bon, colonnes et lignes comprises : un
    // morceau nommé `Demonstration-` sortirait même d'un composant vide, ce que ces deux
    // témoins-ci ne feraient pas. La référence d'une ligne, et le tri d'une colonne.
    for temoin in ["DEM-001", "admin-demonstration"] {
        assert!(
            bundles.contains(temoin),
            "`{temoin}` ne part pas au navigateur : l'écran patron n'a pas compilé"
        );
    }

    // Le rail tient dans une seule liste, et l'écran s'y est inscrit par son ancre : le
    // voir dans le build est ce qui prouve que la coquille la parcourt vraiment.
    let rail = std::fs::read_to_string(repertoire.join("src/admin/rail.ts"))
        .expect("le rail du shell se lit");
    assert!(
        rail.contains("route: 'admin-demonstration'"),
        "l'écran ne s'est pas inscrit au rail :\n{rail}"
    );
}

/// Les méthodes du client que le shell appelle, relevées sous `racine`.
///
/// Le relevé porte sur `api.<methode>(`, la seule forme par laquelle le shell atteint le
/// contrat : `api` est l'instance unique que `src/api/index.ts` construit, et rien
/// d'autre ne parle HTTP — un test de plan le tient.
fn appels_du_shell(racine: &Path) -> Vec<String> {
    let mut appels = Vec::new();

    for source in sources(racine) {
        let lu = std::fs::read_to_string(&source).unwrap_or_default();

        for (debut, _) in lu.match_indices("api.") {
            // `openapi.json` porte les mêmes quatre caractères : ce qui précède décide,
            // et un identifiant ne se poursuit pas par la variable qu'on cherche.
            if lu[..debut]
                .chars()
                .next_back()
                .is_some_and(|avant| avant.is_ascii_alphanumeric() || avant == '_')
            {
                continue;
            }

            let apres = &lu[debut + "api.".len()..];
            let methode: String = apres
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();

            if !methode.is_empty() && apres[methode.len()..].starts_with('(') {
                appels.push(methode);
            }
        }
    }

    appels.sort();
    appels.dedup();
    appels
}

/// Les fichiers `.ts` et `.vue` sous `racine`, le client engendré mis à part.
fn sources(racine: &Path) -> Vec<std::path::PathBuf> {
    let mut trouves = Vec::new();

    for entree in std::fs::read_dir(racine).into_iter().flatten().flatten() {
        let chemin = entree.path();

        if chemin.is_dir() {
            trouves.extend(sources(&chemin));
        } else if chemin.ends_with("client.ts") {
            continue;
        } else if matches!(
            chemin.extension().and_then(|suffixe| suffixe.to_str()),
            Some("ts" | "vue")
        ) {
            trouves.push(chemin);
        }
    }

    trouves
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
