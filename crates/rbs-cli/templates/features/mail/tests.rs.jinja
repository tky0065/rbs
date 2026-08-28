use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use minijinja::context;

use super::config::{MailConfig, Tls};
use super::gabarit::Gabarits;
use super::service::Mailer;

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
async fn les_trois_modes_de_chiffrement_batissent_un_transport() {
    for tls in [Tls::Aucun, Tls::Starttls, Tls::Wrapper] {
        Mailer::nouveau(&MailConfig { tls, ..config() })
            .unwrap_or_else(|erreur| panic!("le transport {tls:?} doit se bâtir : {erreur}"));
    }
}

/// L'expéditeur est analysé au démarrage, pas au premier message : une faute de frappe
/// dans `config/default.toml` doit arrêter le serveur, non une requête au hasard.
#[tokio::test]
async fn un_expediteur_invalide_arrete_la_construction_en_le_nommant() {
    let erreur = Mailer::nouveau(&MailConfig {
        from: "pas une adresse".to_string(),
        ..config()
    })
    .expect_err("« pas une adresse » n'est pas un expéditeur");

    assert!(
        format!("{erreur:#}").contains("pas une adresse"),
        "l'erreur ne nomme pas l'expéditeur fautif : {erreur:#}"
    );
}

#[tokio::test]
async fn le_message_porte_l_expediteur_configure_et_son_destinataire() {
    let mailer = Mailer::nouveau(&config()).expect("le transport doit se bâtir");

    let message = mailer
        .message(
            "client@example.test",
            "Bienvenue",
            "<p>Bonjour</p>".to_string(),
        )
        .expect("le message doit se construire");

    let rendu = String::from_utf8(message.formatted()).expect("un message est de l'UTF-8");

    assert!(
        rendu.contains("From: Facteur <no-reply@example.test>"),
        "{rendu}"
    );
    assert!(rendu.contains("To: client@example.test"), "{rendu}");
    assert!(rendu.contains("Subject: Bienvenue"), "{rendu}");
}

/// Le gabarit livré par le fragment, tel que le projet le trouve sur son disque.
fn gabarits() -> Gabarits {
    Gabarits::nouveaux(MailConfig::default().gabarits)
}

#[test]
fn le_gabarit_rendu_porte_les_variables_qui_lui_sont_passees() {
    let rendu = gabarits()
        .rendre(
            "bienvenue.html",
            context! { nom => "Ada & Lovelace", lien => "https://example.test/compte" },
        )
        .expect("le gabarit livré doit se rendre");

    // L'esperluette ressort échappée : le gabarit porte l'extension `.html`, dont
    // minijinja tire l'échappement. Un nom venu de la base n'y injecte pas de balise.
    assert!(
        rendu.contains("Ada &amp; Lovelace"),
        "le nom n'est pas rendu :\n{rendu}"
    );
    assert!(
        rendu.contains("example.test"),
        "le lien n'est pas rendu :\n{rendu}"
    );
}

/// L'erreur nomme le fichier, que minijinja ne connaît que par son nom de gabarit :
/// « absent.html » seul n'aide personne à trouver le répertoire qui le manque.
#[test]
fn un_gabarit_introuvable_nomme_le_fichier_sans_paniquer() {
    let erreur = gabarits()
        .rendre("absent.html", context! {})
        .expect_err("« absent.html » n'existe pas");

    assert!(
        erreur.to_string().contains("templates/mail/absent.html"),
        "l'erreur ne nomme pas le fichier : {erreur}"
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
async fn envoyer_detache_rend_la_main_sans_attendre_l_envoi() {
    let ecoute = TcpListener::bind("127.0.0.1:0").expect("un port libre de la boucle locale");
    let port = ecoute.local_addr().expect("l'écoute est liée").port();
    let (annonce, connexions) = mpsc::channel();

    std::thread::spawn(move || {
        let connexion = ecoute.accept();
        let _ = annonce.send(());
        std::thread::sleep(Duration::from_secs(1));
        drop(connexion);
    });

    let mailer = Mailer::nouveau(&MailConfig {
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
    mailer.envoyer_detache(message);
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
async fn un_message_a_gabarit_part_vers_le_serveur_smtp() {
    let mailer = Mailer::depuis_config().expect("la section [mail] doit être lisible");

    mailer
        .envoyer_gabarit(
            "ada@example.org",
            "Bienvenue chez nous",
            "bienvenue.html",
            context! { nom => "Ada", lien => "https://example.org/compte" },
        )
        .await
        .expect("l'envoi doit aboutir");
}
