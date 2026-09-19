use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use super::Config;
use super::feuille::{FERMETURE, FEUILLE, OUVERTURE};

/// Le nom du projet, tel que `rbs new` l'a fixé.
const PROJET: &str = "admin-console";

const TITRE: &str = "Le client n'est pas encore construit";
const SOUS_TITRE: &str =
    "L'API tourne. Ce qui suit est ce qui manque au serveur pour vous rendre une page.";
const ETAT: &str = "État du service";
const API: &str = "API";
const API_VALEUR: &str = "répond — cette page en vient";
const BASE: &str = "Base de données";
const BASE_OK: &str = "joignable";
const BASE_KO: &str = "injoignable";
const CLIENT: &str = "Build du client";
const CLIENT_KO: &str = "absent";
const ATTENDU: &str = "Attendu sous";
const GESTES: &str = "Ce qu'il reste à taper";
const NOTE: &str = "Cette page disparaît d'elle-même dès que le build existe. \
                    Il n'y a aucun interrupteur à basculer.";
const SONDE: &str = "Sonde de santé";
/// La page servie tant que le client n'est pas construit.
///
/// Autonome par nécessité : elle est rendue avant qu'aucune dépendance du client ne soit
/// installée, donc sans police, sans feuille de style et sans image distantes. Tout ce
/// qu'elle affiche est vrai à l'instant du rendu — le nom du projet vient de la
/// génération, l'état de la base d'un ping, le répertoire attendu de la configuration.
pub fn page(config: &Config, base_joignable: bool) -> Response {
    let index = config.index().display().to_string();
    let mut corps = String::with_capacity(OUVERTURE.len() + FEUILLE.len() + 2048);

    corps.push_str(OUVERTURE);
    corps.push_str(&echappe(PROJET));
    corps.push_str(FEUILLE);

    corps.push_str("<header class=\"bande bande--titre\"><p class=\"projet\">");
    corps.push_str(&echappe(PROJET));
    corps.push_str("</p><h1>");
    corps.push_str(&echappe(TITRE));
    corps.push_str("</h1><p class=\"tenue\">");
    corps.push_str(&echappe(SOUS_TITRE));
    corps.push_str("</p></header>");

    corps.push_str("<section class=\"bande\"><h2>");
    corps.push_str(&echappe(ETAT));
    corps.push_str("</h2><dl class=\"releve\">");
    releve(&mut corps, API, API_VALEUR, true);
    releve(
        &mut corps,
        BASE,
        if base_joignable { BASE_OK } else { BASE_KO },
        base_joignable,
    );
    releve(&mut corps, CLIENT, CLIENT_KO, false);
    corps.push_str("</dl><p class=\"tenue\">");
    corps.push_str(&echappe(ATTENDU));
    corps.push_str(" <code>");
    corps.push_str(&echappe(&index));
    corps.push_str("</code></p></section>");

    corps.push_str("<section class=\"bande bande--paire\"><h2>");
    corps.push_str(&echappe(GESTES));
    corps.push_str("</h2><ol class=\"gestes\">");
    // Le `cd` n'est imprimé que s'il y a quelque part où se rendre : un build qui sort à
    // la racine du projet n'en demande aucun, et une commande fausse sur une page de
    // secours coûte plus cher qu'une commande absente.
    let gestes = config
        .projet()
        .map(|projet| format!("cd {projet}"))
        .into_iter()
        .chain(["npm install".to_string(), "npm run build".to_string()]);
    for geste in gestes {
        corps.push_str("<li><code>");
        corps.push_str(&echappe(&geste));
        corps.push_str("</code></li>");
    }
    corps.push_str("</ol><p class=\"tenue\">");
    corps.push_str(&echappe(NOTE));
    corps.push_str("</p></section>");

    corps.push_str("<footer class=\"bande\"><p class=\"tenue\">");
    corps.push_str(&echappe(SONDE));
    corps.push_str(" <a href=\"/health\">/health</a></p></footer>");

    corps.push_str(FERMETURE);

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        corps,
    )
        .into_response()
}

/// Une ligne du relevé d'état. Le verdict passe par une classe : un mot de couleur écrit
/// dans le balisage ne se remplacerait pas en réécrivant les seules variables de la feuille.
fn releve(corps: &mut String, terme: &str, valeur: &str, bon: bool) {
    corps.push_str("<dt>");
    corps.push_str(&echappe(terme));
    corps.push_str("</dt><dd class=\"");
    corps.push_str(if bon { "bon" } else { "manque" });
    corps.push_str("\">");
    corps.push_str(&echappe(valeur));
    corps.push_str("</dd>");
}

/// Échappe ce qui part dans le document.
///
/// Le nom du projet est contraint par `rbs new`, mais le répertoire de build vient de la
/// configuration, qu'un développeur écrit à la main : une page de secours qui injecterait
/// du balisage serait une faille née d'un message d'aide.
fn echappe(texte: &str) -> String {
    texte
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
