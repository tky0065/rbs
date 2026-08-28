//! L'envoi d'un courriel joué contre un vrai serveur SMTP, et relu par son API.
//!
//! Les tests du fragment prouvent le rendu d'un gabarit et le détachement de l'envoi.
//! Aucun ne montre qu'un message part réellement, ni que ce qui arrive à l'autre bout
//! porte le corps rendu — un transport en mémoire ne le prouverait pas : ce corps
//! n'existe qu'après la sérialisation MIME, l'échange SMTP et le décodage par le serveur.
//!
//! Mailpit est retenu pour son API HTTP, qui permet de relire le message reçu.

use std::io::{Read, Write};
use std::net::TcpStream;

use assert_cmd::Command;
use tempfile::TempDir;
use testcontainers::GenericImage;
use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;

mod common;

const IMAGE: (&str, &str) = ("axllent/mailpit", "latest");

const ENVOI: &str = "a_templated_message_goes_out_to_the_smtp_server";

#[test]
#[ignore = "démarre Mailpit et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn a_templated_email_reaches_its_destination_and_reads_back_through_the_api() {
    let mailpit = GenericImage::new(IMAGE.0, IMAGE.1)
        .with_wait_for(WaitFor::log(LogWaitStrategy::stdout_or_stderr(
            "accessible via http",
        )))
        .start()
        .expect("Mailpit doit démarrer — Docker est-il lancé ?");

    let smtp = mailpit
        .get_host_port_ipv4(1025.tcp())
        .expect("le port SMTP de Mailpit doit être publié");
    let api = mailpit
        .get_host_port_ipv4(8025.tcp())
        .expect("le port HTTP de Mailpit doit être publié");

    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet).args(["add", "mail"]).assert().success();

    // Le conteneur reçoit ses ports au démarrage : `config/default.toml` ne peut pas les
    // connaître, et c'est la surcharge par l'environnement qui les lui apprend.
    let output = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .env("RBS_MAIL__SMTP_HOST", "127.0.0.1")
        .env("RBS_MAIL__SMTP_PORT", smtp.to_string())
        .args(["test", "--workspace", "--", "--ignored"])
        .output()
        .expect("cargo doit se lancer");

    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "l'envoi depuis le projet a échoué :\n{journal}"
    );

    // `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun** test : sans
    // cette ligne, un fragment qui cesserait de livrer son test d'envoi laisserait
    // celui-ci au vert.
    assert!(
        journal.contains(&format!("test mail::tests::{ENVOI} ... ok")),
        "`{ENVOI}` n'a pas été exécuté :\n{journal}"
    );

    let boite = json(api, "/api/v1/messages");

    // Mailpit démarre vide : exiger le compte exact, et non la seule présence d'un
    // message, ferme le cas d'une boîte relue trop tôt comme celui d'un reliquat.
    assert_eq!(
        boite["messages_count"], 1,
        "un message et un seul devait être arrivé : {boite}"
    );

    let identifiant = boite["messages"][0]["ID"]
        .as_str()
        .expect("Mailpit nomme chaque message par un identifiant")
        .to_string();

    let message = json(api, &format!("/api/v1/message/{identifiant}"));

    assert_eq!(message["To"][0]["Address"], "ada@example.org");
    assert_eq!(message["Subject"], "Bienvenue chez nous");

    // Le corps est ce que le critère vise : ces deux chaînes n'existent qu'une fois le
    // gabarit rendu avec son contexte, et ont traversé MIME et SMTP pour arriver ici.
    //
    // L'URL n'y est pas cherchée entière : l'autoéchappement de minijinja rend ses `/` en
    // `&#x2f;`, et c'est bien ce qu'on veut d'un gabarit HTML — un lien qui vient d'une
    // entrée utilisateur ne doit pas pouvoir en sortir. L'hôte suffit à établir que le
    // contexte a traversé, et il n'apparaît nulle part ailleurs dans le corps.
    let corps = message["HTML"]
        .as_str()
        .expect("le message est envoyé en HTML");
    for attendu in ["Bonjour Ada,", "example.org"] {
        assert!(
            corps.contains(attendu),
            "le corps reçu ne porte pas `{attendu}` :\n{corps}"
        );
    }
}

/// Le corps JSON que l'API de Mailpit rend pour `chemin`.
///
/// La requête est en **HTTP/1.0** : le serveur Go de Mailpit répondrait sinon en
/// *chunked*, qu'il faudrait décoder. En 1.0 il annonce la fin par la fermeture de la
/// connexion, et la réponse se lit jusqu'au bout sans rien interpréter.
fn json(port: u16, chemin: &str) -> serde_json::Value {
    let mut flux = TcpStream::connect(("127.0.0.1", port)).expect("l'API de Mailpit doit répondre");

    flux.write_all(format!("GET {chemin} HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n").as_bytes())
        .expect("la requête doit partir");

    let mut reponse = String::new();
    flux.read_to_string(&mut reponse)
        .expect("la réponse doit se lire");

    let corps = reponse
        .split_once("\r\n\r\n")
        .map(|(_, corps)| corps)
        .unwrap_or_else(|| panic!("réponse HTTP sans corps pour `{chemin}` :\n{reponse}"));

    serde_json::from_str(corps)
        .unwrap_or_else(|erreur| panic!("`{chemin}` n'a pas rendu du JSON : {erreur}\n{corps}"))
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<std::path::Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}
