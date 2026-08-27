//! Contrôle de la feature `auth`.
//!
//! `add auth` n'écrit `RBS_AUTH__SECRET` que dans `.env.example` : ce fichier est
//! versionné, et un secret réel n'a rien à y faire. Un projet fraîchement doté d'auth ne
//! démarre donc pas tant que l'utilisateur ne l'a pas recopié — et le message qu'il lit
//! alors vient du noyau, au boot. Ce contrôle le lui dit avant.

use std::path::Path;

use crate::dotenv;

use super::Controle;

const TITRE: &str = "auth";
const SECRET: &str = "RBS_AUTH__SECRET";
const FICHIER: &str = ".env";
const EXEMPLE: &str = ".env.example";
const CONFIG: &str = "config/default.toml";

/// Longueur minimale du secret, en octets.
///
/// Duplique `SECRET_MINIMUM` de `rbs-core`, que `rbs-cli` ne peut pas lire : les deux
/// crates sont indépendantes par construction, le CLI ne fait qu'inscrire le noyau dans
/// les manifestes qu'il génère.
const MINIMUM: usize = 32;

/// Vérifie ce dont la feature `auth` a besoin pour démarrer.
pub(crate) fn controler(racine: &Path) -> Controle {
    controler_avec(racine, |cle| std::env::var(cle).ok())
}

/// Le contrôle, l'environnement passé en paramètre.
///
/// L'environnement l'emporte sur le `.env`, comme dans `migrate::variables_du_projet` :
/// un diagnostic qui crierait au secret manquant alors qu'il est exporté serait faux.
fn controler_avec(racine: &Path, env: impl Fn(&str) -> Option<String>) -> Controle {
    let du_fichier = dotenv::lire(&racine.join(FICHIER)).unwrap_or_default();
    let de_l_exemple = dotenv::lire(&racine.join(EXEMPLE)).unwrap_or_default();

    let secret = env(SECRET).or_else(|| dotenv::valeur(&du_fichier, SECRET).map(str::to_owned));

    let mut defauts = Vec::new();
    let mut remedes = Vec::new();

    match secret {
        None => {
            defauts.push(format!(
                "{SECRET} n'est renseignée ni dans le {FICHIER} ni dans l'environnement"
            ));
            remedes.push(format!(
                "ajoutez au {FICHIER} une valeur tirée au hasard :\n      {SECRET}=$(openssl rand -hex 32)"
            ));
        }
        Some(valeur) => {
            if valeur.len() < MINIMUM {
                defauts.push(format!(
                    "{SECRET} porte {} octets, il en faut {MINIMUM}",
                    valeur.len()
                ));
                remedes.push(format!(
                    "allongez {SECRET} :\n      {SECRET}=$(openssl rand -hex 32)"
                ));
            }

            // Comparé à `.env.example` plutôt qu'à une chaîne écrite ici : ce fichier est
            // la référence, et une reformulation d'`add auth` n'a alors rien à
            // resynchroniser.
            if dotenv::valeur(&de_l_exemple, SECRET) == Some(valeur.as_str()) {
                defauts.push(format!(
                    "{SECRET} est resté à la valeur d'exemple, publiée dans Git"
                ));
                remedes.push(format!(
                    "remplacez-la par une valeur tirée au hasard :\n      {SECRET}=$(openssl rand -hex 32)"
                ));
            }
        }
    }

    if !section_auth(racine) {
        defauts.push(format!("{CONFIG} ne porte pas de section `[auth]`"));
        remedes.push(format!(
            "ajoutez à {CONFIG} :\n      [auth]\n      access_ttl_secs = 900\n      refresh_ttl_secs = 2592000"
        ));
    }

    if defauts.is_empty() {
        return Controle::bon(TITRE, "le secret et la configuration sont en place");
    }

    Controle::echec(TITRE, defauts.join(" ; "), remedes.join("\n    "))
}

/// Vrai si `config/default.toml` porte une section `[auth]`.
///
/// Lu par `toml_edit` et non par recherche de texte : un `[auth]` en commentaire n'est
/// pas une section.
fn section_auth(racine: &Path) -> bool {
    std::fs::read_to_string(racine.join(CONFIG))
        .ok()
        .and_then(|source| source.parse::<toml_edit::DocumentMut>().ok())
        .is_some_and(|document| document.get("auth").is_some())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::Etat;
    use super::*;

    /// Un projet neuf, doté à la main de ce que `add auth` y dépose.
    ///
    /// La commande elle-même n'est pas appelée : ce contrôle ne lit que trois fichiers,
    /// et les poser directement garde le test à la seconde plutôt qu'à la minute.
    fn projet_avec_auth() -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let projet = crate::new::creer(
            &crate::new::Options {
                nom: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let racine = projet.racine;

        ajouter(&racine, EXEMPLE, &format!("{SECRET}={EXEMPLE_DU_SECRET}\n"));
        ajouter(
            &racine,
            CONFIG,
            "\n[auth]\naccess_ttl_secs = 900\nrefresh_ttl_secs = 2592000\n",
        );

        (parent, racine)
    }

    /// La valeur que `add auth` écrit dans `.env.example`.
    const EXEMPLE_DU_SECRET: &str =
        "changez-moi-par-un-secret-tire-au-hasard-de-32-octets-au-moins";

    /// Un secret acceptable : tiré au hasard et assez long.
    const SECRET_VALIDE: &str = "1f3c9a7e5b2d8064af1e3c5970b2d846e1c3a597f0b2d8461f3c9a7e5b2d8064";

    fn ajouter(racine: &Path, fichier: &str, ligne: &str) {
        let chemin = racine.join(fichier);
        let source = fs::read_to_string(&chemin).unwrap_or_default();
        fs::write(&chemin, format!("{source}{ligne}")).expect("fichier inscriptible");
    }

    /// Sans environnement : ce que voit un utilisateur qui n'a rien exporté.
    fn nu(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn sans_secret_le_diagnostic_nomme_la_variable() {
        let (_parent, racine) = projet_avec_auth();

        let controle = controler_avec(&racine, nu);

        assert_eq!(controle.etat, Etat::Echec, "{}", controle.detail);
        assert!(
            controle.detail.contains(SECRET),
            "le détail doit nommer la variable : {}",
            controle.detail
        );
    }

    #[test]
    fn un_secret_trop_court_est_refuse() {
        let (_parent, racine) = projet_avec_auth();
        let court = "a".repeat(MINIMUM - 1);
        ajouter(&racine, FICHIER, &format!("{SECRET}={court}\n"));

        let controle = controler_avec(&racine, nu);

        assert_eq!(controle.etat, Etat::Echec, "{}", controle.detail);
        assert!(
            controle.detail.contains(&format!("{}", MINIMUM - 1)),
            "le détail doit donner les octets fournis : {}",
            controle.detail
        );
    }

    #[test]
    fn un_secret_reste_a_la_valeur_d_exemple_est_signale() {
        let (_parent, racine) = projet_avec_auth();
        ajouter(&racine, FICHIER, &format!("{SECRET}={EXEMPLE_DU_SECRET}\n"));

        let controle = controler_avec(&racine, nu);

        assert_eq!(
            controle.etat,
            Etat::Echec,
            "un secret publié dans Git ne vaut pas mieux qu'aucun : {}",
            controle.detail
        );
        assert!(
            controle.detail.contains("exemple"),
            "le détail doit dire d'où vient la valeur : {}",
            controle.detail
        );
    }

    #[test]
    fn sans_section_auth_le_diagnostic_le_dit() {
        let (_parent, racine) = projet_avec_auth();
        ajouter(&racine, FICHIER, &format!("{SECRET}={SECRET_VALIDE}\n"));
        let config = racine.join(CONFIG);
        let source = fs::read_to_string(&config).expect("config lisible");
        fs::write(
            &config,
            source.replace("[auth]", "# section retirée par le test"),
        )
        .expect("config inscriptible");

        let controle = controler_avec(&racine, nu);

        assert_eq!(controle.etat, Etat::Echec, "{}", controle.detail);
        assert!(
            controle.detail.contains("[auth]"),
            "le détail doit nommer la section : {}",
            controle.detail
        );
    }

    #[test]
    fn un_projet_correctement_dote_ne_signale_rien() {
        let (_parent, racine) = projet_avec_auth();
        ajouter(&racine, FICHIER, &format!("{SECRET}={SECRET_VALIDE}\n"));

        let controle = controler_avec(&racine, nu);

        assert_eq!(controle.etat, Etat::Bon, "{}", controle.detail);
    }

    #[test]
    fn le_secret_de_l_environnement_dispense_du_fichier() {
        let (_parent, racine) = projet_avec_auth();

        // Le `.env` ne porte rien : seul l'environnement répond.
        let controle = controler_avec(&racine, |cle| {
            (cle == SECRET).then(|| SECRET_VALIDE.to_string())
        });

        assert_eq!(
            controle.etat,
            Etat::Bon,
            "un secret exporté vaut un secret écrit : {}",
            controle.detail
        );
    }

    /// Une section en commentaire n'est pas une section.
    #[test]
    fn un_auth_en_commentaire_ne_compte_pas_pour_une_section() {
        let (_parent, racine) = projet_avec_auth();
        ajouter(&racine, FICHIER, &format!("{SECRET}={SECRET_VALIDE}\n"));
        let config = racine.join(CONFIG);
        let source = fs::read_to_string(&config).expect("config lisible");
        fs::write(&config, source.replace("[auth]", "# [auth]")).expect("config inscriptible");

        let controle = controler_avec(&racine, nu);

        assert_eq!(controle.etat, Etat::Echec, "{}", controle.detail);
    }
}
