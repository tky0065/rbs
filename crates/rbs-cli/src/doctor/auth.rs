//! Contrôle de la feature `auth`.
//!
//! `add auth` n'écrit `RBS_AUTH__SECRET` que dans `.env.example` : ce fichier est
//! versionné, et un secret réel n'a rien à y faire. Un projet fraîchement doté d'auth ne
//! démarre donc pas tant que l'utilisateur ne l'a pas recopié — et le message qu'il lit
//! alors vient du noyau, au boot. Ce contrôle le lui dit avant.

use std::path::Path;

use crate::dotenv;

use super::Check;

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
pub(crate) fn check(root: &Path) -> Check {
    check_with(root, |key| std::env::var(key).ok())
}

/// Le contrôle, l'environnement passé en paramètre.
///
/// L'environnement l'emporte sur le `.env`, comme dans `migrate::project_variables` :
/// un diagnostic qui crierait au secret manquant alors qu'il est exporté serait faux.
fn check_with(root: &Path, env: impl Fn(&str) -> Option<String>) -> Check {
    let du_fichier = dotenv::read(&root.join(FICHIER)).unwrap_or_default();
    let de_l_exemple = dotenv::read(&root.join(EXEMPLE)).unwrap_or_default();

    let secret = env(SECRET).or_else(|| dotenv::value(&du_fichier, SECRET).map(str::to_owned));

    let mut defauts = Vec::new();
    let mut remedes = Vec::new();

    match secret {
        None => {
            defauts.push(format!(
                "{SECRET} n'est renseignée ni dans le {FICHIER} ni dans l'environnement"
            ));
            remedes.push(format!(
                "ajoutez au {FICHIER} une valeur tirée au hasard :\n{SECRET}=$(openssl rand -hex 32)"
            ));
        }
        Some(value) => {
            if value.len() < MINIMUM {
                defauts.push(format!(
                    "{SECRET} porte {} octets, il en faut {MINIMUM}",
                    value.len()
                ));
                remedes.push(format!(
                    "allongez {SECRET} :\n{SECRET}=$(openssl rand -hex 32)"
                ));
            }

            // Comparé à `.env.example` plutôt qu'à une chaîne écrite ici : ce fichier est
            // la référence, et une reformulation d'`add auth` n'a alors rien à
            // resynchroniser.
            if dotenv::value(&de_l_exemple, SECRET) == Some(value.as_str()) {
                defauts.push(format!(
                    "{SECRET} est resté à la valeur d'exemple, publiée dans Git"
                ));
                remedes.push(format!(
                    "remplacez-la par une valeur tirée au hasard :\n{SECRET}=$(openssl rand -hex 32)"
                ));
            }
        }
    }

    if !auth_section(root) {
        defauts.push(format!("{CONFIG} ne porte pas de section `[auth]`"));
        remedes.push(format!(
            "ajoutez à {CONFIG} :\n[auth]\naccess_ttl_secs = 900\nrefresh_ttl_secs = 2592000"
        ));
    }

    if defauts.is_empty() {
        return Check::ok(TITRE, "le secret et la configuration sont en place");
    }

    Check::failed(TITRE, defauts.join(" ; "), remedes.join("\n"))
}

/// Vrai si `config/default.toml` porte une section `[auth]`.
///
/// Lu par `toml_edit` et non par recherche de texte : un `[auth]` en commentaire n'est
/// pas une section.
fn auth_section(root: &Path) -> bool {
    std::fs::read_to_string(root.join(CONFIG))
        .ok()
        .and_then(|source| source.parse::<toml_edit::DocumentMut>().ok())
        .is_some_and(|document| document.get("auth").is_some())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Un projet neuf, doté à la main de ce que `add auth` y dépose.
    ///
    /// La commande elle-même n'est pas appelée : ce contrôle ne lit que trois fichiers,
    /// et les poser directement garde le test à la seconde plutôt qu'à la minute.
    fn project_with_auth() -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let root = project.root;

        add(&root, EXEMPLE, &format!("{SECRET}={EXEMPLE_DU_SECRET}\n"));
        add(
            &root,
            CONFIG,
            "\n[auth]\naccess_ttl_secs = 900\nrefresh_ttl_secs = 2592000\n",
        );

        (parent, root)
    }

    /// La valeur que `add auth` écrit dans `.env.example`.
    const EXEMPLE_DU_SECRET: &str =
        "changez-moi-par-un-secret-tire-au-hasard-de-32-octets-au-moins";

    /// Un secret acceptable : tiré au hasard et assez long.
    const SECRET_VALIDE: &str = "1f3c9a7e5b2d8064af1e3c5970b2d846e1c3a597f0b2d8461f3c9a7e5b2d8064";

    fn add(root: &Path, file: &str, line: &str) {
        let path = root.join(file);
        let source = fs::read_to_string(&path).unwrap_or_default();
        fs::write(&path, format!("{source}{line}")).expect("fichier inscriptible");
    }

    /// Sans environnement : ce que voit un utilisateur qui n'a rien exporté.
    fn bare(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn without_a_secret_the_diagnosis_names_the_variable() {
        let (_parent, root) = project_with_auth();

        let check = check_with(&root, bare);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains(SECRET),
            "le détail doit nommer la variable : {}",
            check.detail
        );
    }

    #[test]
    fn a_too_short_secret_is_rejected() {
        let (_parent, root) = project_with_auth();
        let court = "a".repeat(MINIMUM - 1);
        add(&root, FICHIER, &format!("{SECRET}={court}\n"));

        let check = check_with(&root, bare);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains(&format!("{}", MINIMUM - 1)),
            "le détail doit donner les octets fournis : {}",
            check.detail
        );
    }

    #[test]
    fn a_secret_left_at_the_example_value_is_reported() {
        let (_parent, root) = project_with_auth();
        add(&root, FICHIER, &format!("{SECRET}={EXEMPLE_DU_SECRET}\n"));

        let check = check_with(&root, bare);

        assert_eq!(
            check.state,
            State::Echec,
            "un secret publié dans Git ne vaut pas mieux qu'aucun : {}",
            check.detail
        );
        assert!(
            check.detail.contains("exemple"),
            "le détail doit dire d'où vient la valeur : {}",
            check.detail
        );
    }

    #[test]
    fn without_an_auth_section_the_diagnosis_says_so() {
        let (_parent, root) = project_with_auth();
        add(&root, FICHIER, &format!("{SECRET}={SECRET_VALIDE}\n"));
        let config = root.join(CONFIG);
        let source = fs::read_to_string(&config).expect("config lisible");
        fs::write(
            &config,
            source.replace("[auth]", "# section retirée par le test"),
        )
        .expect("config inscriptible");

        let check = check_with(&root, bare);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("[auth]"),
            "le détail doit nommer la section : {}",
            check.detail
        );
    }

    #[test]
    fn a_properly_equipped_project_reports_nothing() {
        let (_parent, root) = project_with_auth();
        add(&root, FICHIER, &format!("{SECRET}={SECRET_VALIDE}\n"));

        let check = check_with(&root, bare);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    #[test]
    fn a_secret_from_the_environment_makes_the_file_unnecessary() {
        let (_parent, root) = project_with_auth();

        // Le `.env` ne porte rien : seul l'environnement répond.
        let check = check_with(&root, |key| {
            (key == SECRET).then(|| SECRET_VALIDE.to_string())
        });

        assert_eq!(
            check.state,
            State::Bon,
            "un secret exporté vaut un secret écrit : {}",
            check.detail
        );
    }

    /// Une section en commentaire n'est pas une section.
    #[test]
    fn a_commented_out_auth_does_not_count_as_a_section() {
        let (_parent, root) = project_with_auth();
        add(&root, FICHIER, &format!("{SECRET}={SECRET_VALIDE}\n"));
        let config = root.join(CONFIG);
        let source = fs::read_to_string(&config).expect("config lisible");
        fs::write(&config, source.replace("[auth]", "# [auth]")).expect("config inscriptible");

        let check = check_with(&root, bare);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
    }
}
