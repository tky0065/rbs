//! Contrôle de la feature `mail`.
//!
//! `add mail` pose `RBS_MAIL__SMTP_PASSWORD` vide dans `.env.example`, et le commentaire
//! qui l'accompagne dit pourquoi : « Reste vide tant que `smtp_user` l'est ». Un serveur
//! de développement — Mailpit, MailHog — n'authentifie personne. C'est donc le couple qui
//! se diagnostique, et non la variable seule : vide avec un compte renseigné, l'envoi
//! échouera à l'authentification, ce qu'`env::check` ne peut pas voir puisque la clé est
//! bien là.

use std::path::Path;

use crate::dotenv;

use super::Check;

const TITRE: &str = "mail";
const CLE: &str = "RBS_MAIL__SMTP_PASSWORD";
const FICHIER: &str = ".env";
const CONFIG: &str = "config/default.toml";
const SECTION: &str = "mail";
const UTILISATEUR: &str = "smtp_user";

/// Vérifie ce dont la feature `mail` a besoin pour envoyer.
pub(crate) fn check(root: &Path) -> Check {
    check_with(root, |key| std::env::var(key).ok())
}

/// Le contrôle, l'environnement passé en paramètre.
///
/// L'environnement l'emporte sur le `.env`, comme dans `auth::check_with` : crier au mot
/// de passe manquant alors qu'il est exporté serait faux.
fn check_with(root: &Path, env: impl Fn(&str) -> Option<String>) -> Check {
    let du_fichier = dotenv::read(&root.join(FICHIER)).unwrap_or_default();

    let mot_de_passe = env(CLE).or_else(|| dotenv::value(&du_fichier, CLE).map(str::to_owned));
    let compte = super::field(root, SECTION, UTILISATEUR).unwrap_or_default();

    let mut defauts = Vec::new();
    let mut remedes = Vec::new();

    match mot_de_passe.as_deref() {
        None => {
            defauts.push(format!(
                "{CLE} n'est renseignée ni dans le {FICHIER} ni dans l'environnement"
            ));
            remedes.push(format!(
                "ajoutez au {FICHIER} la ligne que {SECTION} y attend, vide tant que \
                 {UTILISATEUR} l'est :\n{CLE}="
            ));
        }
        // Une valeur vide n'est un défaut que si un compte est nommé : le serveur de
        // développement du fragment n'authentifie personne.
        Some("") if !compte.is_empty() => {
            defauts.push(format!(
                "{UTILISATEUR} vaut `{compte}` et {CLE} est vide : le serveur refusera l'authentification"
            ));
            remedes.push(format!(
                "renseignez le mot de passe du compte dans le {FICHIER} :\n{CLE}=…\nou laissez \
                 {UTILISATEUR} vide, ce que n'authentifie aucun serveur local"
            ));
        }
        Some(_) => {}
    }

    if !super::section(root, SECTION) {
        defauts.push(format!("{CONFIG} ne porte pas de section `[{SECTION}]`"));
        remedes.push(format!(
            "ajoutez à {CONFIG} :\n[{SECTION}]\nsmtp_host = \"localhost\"\nsmtp_port = 1025\n\
             smtp_user = \"\"\ntls = \"none\"\nfrom = \"no-reply@localhost\"\ntimeout_secs = 10\n\
             templates = \"templates/mail\""
        ));
    }

    if defauts.is_empty() {
        return Check::ok(TITRE, "le transport SMTP est configuré");
    }

    Check::failed(TITRE, defauts.join(" ; "), remedes.join("\n"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Un projet neuf, doté à la main de ce que `add mail` y dépose.
    fn project_with_mail() -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Default::default(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let root = project.root;

        // Ce que le `[[config]]` du fragment inscrit, `smtp_user` vide compris.
        add(
            &root,
            CONFIG,
            "\n[mail]\nsmtp_host = \"localhost\"\nsmtp_port = 1025\nsmtp_user = \"\"\ntls = \"none\"\nfrom = \"no-reply@localhost\"\ntimeout_secs = 10\ntemplates = \"templates/mail\"\n",
        );
        // Ce que le `[[env]]` du fragment inscrit : la clé, vide.
        add(&root, ".env.example", &format!("{CLE}=\n"));
        add(&root, FICHIER, &format!("{CLE}=\n"));

        (parent, root)
    }

    fn add(root: &Path, file: &str, line: &str) {
        let path = root.join(file);
        let source = fs::read_to_string(&path).unwrap_or_default();
        fs::write(&path, format!("{source}{line}")).expect("fichier inscriptible");
    }

    fn rewrite(root: &Path, file: &str, from: &str, to: &str) {
        let path = root.join(file);
        let source = fs::read_to_string(&path).expect("fichier lisible");
        fs::write(&path, source.replace(from, to)).expect("fichier inscriptible");
    }

    /// Sans environnement : ce que voit un utilisateur qui n'a rien exporté.
    fn bare(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn without_the_password_variable_the_diagnosis_names_it() {
        let (_parent, root) = project_with_mail();
        rewrite(&root, FICHIER, &format!("{CLE}=\n"), "");

        let check = check_with(&root, bare);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains(CLE),
            "le détail doit nommer la variable : {}",
            check.detail
        );
    }

    /// Le défaut qu'`env::check` ne peut pas voir : la clé est là, mais vide, et le
    /// compte SMTP est renseigné.
    #[test]
    fn a_named_account_without_a_password_is_reported() {
        let (_parent, root) = project_with_mail();
        rewrite(
            &root,
            CONFIG,
            "smtp_user = \"\"",
            "smtp_user = \"envoi@exemple.fr\"",
        );

        let check = check_with(&root, bare);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains(UTILISATEUR),
            "le détail doit dire ce qui rend le mot de passe nécessaire : {}",
            check.detail
        );
    }

    /// Mailpit n'authentifie personne : un mot de passe vide y est la configuration
    /// juste, et le fragment le déclare ainsi.
    #[test]
    fn a_local_server_without_authentication_reports_nothing() {
        let (_parent, root) = project_with_mail();

        let check = check_with(&root, bare);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    #[test]
    fn without_a_mail_section_the_diagnosis_says_so() {
        let (_parent, root) = project_with_mail();
        rewrite(&root, CONFIG, "[mail]", "# section retirée par le test");

        let check = check_with(&root, bare);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("[mail]"),
            "le détail doit nommer la section : {}",
            check.detail
        );
    }

    #[test]
    fn a_password_from_the_environment_makes_the_file_unnecessary() {
        let (_parent, root) = project_with_mail();
        rewrite(&root, FICHIER, &format!("{CLE}=\n"), "");
        rewrite(
            &root,
            CONFIG,
            "smtp_user = \"\"",
            "smtp_user = \"envoi@exemple.fr\"",
        );

        let check = check_with(&root, |key| {
            (key == CLE).then(|| "un-mot-de-passe".to_string())
        });

        assert_eq!(
            check.state,
            State::Bon,
            "un mot de passe exporté vaut un mot de passe écrit : {}",
            check.detail
        );
    }
}
