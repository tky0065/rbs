use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use minijinja::context;

use super::config::{MailConfig, Tls};
use super::service::Mailer;
use super::template::Templates;

/// Une configuration de test : le serveur local en clair, sans identifiants.
///
/// Les tests sont `async` bien que rien n'y soit attendu : le pool de `lettre` inscrit sa
/// tâche d'entretien au runtime dès la construction du transport, et panique sans lui.
fn config() -> MailConfig {
    MailConfig {
        from: "Facteur <no-reply@example.test>".to_string(),
        ..MailConfig::default()
    }
}

#[tokio::test]
async fn the_three_encryption_modes_build_a_transport() {
    for tls in [Tls::Aucun, Tls::Starttls, Tls::Wrapper] {
        Mailer::new(&MailConfig { tls, ..config() })
            .unwrap_or_else(|error| panic!("le transport {tls:?} doit se bâtir : {error}"));
    }
}

/// L'expéditeur est analysé au démarrage, pas au premier message : une faute de frappe
/// dans `config/default.toml` doit arrêter le serveur, non une requête au hasard.
#[tokio::test]
async fn an_invalid_sender_stops_the_build_naming_it() {
    let error = Mailer::new(&MailConfig {
        from: "pas une adresse".to_string(),
        ..config()
    })
    .expect_err("« pas une adresse » n'est pas un expéditeur");

    assert!(
        format!("{error:#}").contains("pas une adresse"),
        "l'erreur ne nomme pas l'expéditeur fautif : {error:#}"
    );
}

#[tokio::test]
async fn the_message_carries_the_configured_sender_and_its_recipient() {
    let mailer = Mailer::new(&config()).expect("le transport doit se bâtir");

    let message = mailer
        .message(
            "client@example.test",
            "Bienvenue",
            "<p>Bonjour</p>".to_string(),
        )
        .expect("le message doit se construire");

    let rendered = String::from_utf8(message.formatted()).expect("un message est de l'UTF-8");

    assert!(
        rendered.contains("From: Facteur <no-reply@example.test>"),
        "{rendered}"
    );
    assert!(rendered.contains("To: client@example.test"), "{rendered}");
    assert!(rendered.contains("Subject: Bienvenue"), "{rendered}");
}

/// Le gabarit livré par le fragment, tel que le projet le trouve sur son disque.
fn templates() -> Templates {
    Templates::new(MailConfig::default().templates)
}

#[test]
fn the_rendered_template_carries_the_variables_passed_to_it() {
    let rendered = templates()
        .render(
            "bienvenue.html",
            context! { name => "Ada & Lovelace", link => "https://example.test/compte" },
        )
        .expect("le gabarit livré doit se rendre");

    // L'esperluette ressort échappée : le gabarit porte l'extension `.html`, dont
    // minijinja tire l'échappement. Un nom venu de la base n'y injecte pas de balise.
    assert!(
        rendered.contains("Ada &amp; Lovelace"),
        "le nom n'est pas rendu :\n{rendered}"
    );
    assert!(
        rendered.contains("example.test"),
        "le lien n'est pas rendu :\n{rendered}"
    );
    // Le lien est attendu dans l'attribut, non seulement dans le texte : une variable
    // mal nommée dans un `href` rend un lien vide sans que le corps le montre.
    //
    // L'hôte seul, et non l'URL entière : le gabarit porte l'extension `.html`, dont
    // minijinja tire l'échappement, et ses `/` ressortent en `&#x2f;`.
    let href = rendered
        .split_once(r#"<a href=""#)
        .and_then(|(_, reste)| reste.split_once('"'))
        .map(|(href, _)| href)
        .expect("le gabarit doit porter un lien");
    assert!(
        href.contains("example.test"),
        "le href ne porte pas le lien :\n{rendered}"
    );
}

/// L'erreur nomme le fichier, que minijinja ne connaît que par son nom de gabarit :
/// « absent.html » seul n'aide personne à trouver le répertoire qui le manque.
#[test]
fn a_missing_template_names_the_file_without_panicking() {
    let error = templates()
        .render("absent.html", context! {})
        .expect_err("« absent.html » n'existe pas");

    assert!(
        error.to_string().contains("templates/mail/absent.html"),
        "l'erreur ne nomme pas le fichier : {error}"
    );
}

/// Le critère de la tâche, prouvé sans serveur SMTP.
///
/// Le faux serveur accepte la connexion et ne répond jamais : `lettre` y reste suspendu
/// sur la bannière SMTP, aussi longtemps que son délai le permet. Un envoi attendu
/// bloquerait donc ce test — les deux assertions disent ensemble que l'appel rend la main
/// et que l'envoi part quand même, ce qu'un corps vide ne tiendrait pas.
///
/// `multi_thread` : sur le runtime à fil unique, la tâche détachée n'aurait aucun fil pour
/// tourner pendant que le test attend la connexion.
#[tokio::test(flavor = "multi_thread")]
async fn send_detached_returns_without_awaiting_the_send() {
    let ecoute = TcpListener::bind("127.0.0.1:0").expect("un port libre de la boucle locale");
    let port = ecoute.local_addr().expect("l'écoute est liée").port();
    let (annonce, connexions) = mpsc::channel();

    std::thread::spawn(move || {
        let connection = ecoute.accept();
        let _ = annonce.send(());
        std::thread::sleep(Duration::from_secs(1));
        drop(connection);
    });

    let mailer = Mailer::new(&MailConfig {
        smtp_host: "127.0.0.1".to_string(),
        smtp_port: port,
        timeout_secs: 30,
        ..config()
    })
    .expect("le transport doit se bâtir");

    let message = mailer
        .message("client@example.test", "Bonjour", "<p>Salut</p>".to_string())
        .expect("le message doit se construire");

    let debut = Instant::now();
    mailer.send_detached(message);
    let rendue = debut.elapsed();

    assert!(
        rendue < Duration::from_millis(200),
        "envoyer_detache a attendu {rendue:?}"
    );
    connexions
        .recv_timeout(Duration::from_secs(10))
        .expect("l'envoi n'a pas été lancé en arrière-plan");
}

/// L'envoi joué contre un vrai serveur SMTP, gabarit compris.
///
/// `#[ignore]` : il lui faut le serveur que décrit la section `[mail]` — Mailpit ou
/// MailHog en développement. `cargo test -- --ignored` le lance, `RBS_MAIL__SMTP_PORT`
/// en surchargeant le port au besoin.
///
/// Ce que le test ne peut pas voir d'ici, c'est ce qui est *arrivé* : il envoie, et c'est
/// la boîte du serveur qui porte la preuve.
#[tokio::test]
#[ignore = "joint le serveur SMTP de la section [mail]"]
async fn a_templated_message_goes_out_to_the_smtp_server() {
    let mailer = Mailer::from_config().expect("la section [mail] doit être lisible");

    mailer
        .send_template(
            "ada@example.org",
            "Bienvenue chez nous",
            "bienvenue.html",
            context! { name => "Ada", link => "https://example.org/compte" },
        )
        .await
        .expect("l'envoi doit aboutir");
}
