//! Contrôle de la feature `storage`.
//!
//! Le backend `fs` marche sans rien renseigner : c'est le défaut du fragment, et un
//! projet qui s'y tient n'a aucun réglage à fournir. Tout ce qui suit ne concerne donc
//! que `backend = "s3"`, où le bucket et les identifiants deviennent nécessaires — et où
//! aucun d'eux ne figure dans `[[config]]`, le fragment les laissant à l'environnement.

use std::path::Path;

use crate::dotenv;

use super::{Check, Config};

const TITRE: &str = "storage";
const FICHIER: &str = ".env";
const EXEMPLE: &str = ".env.example";
const CONFIG: &str = "config/default.toml";
const SECTION: &str = "storage";
const BACKEND: &str = "backend";
const S3: &str = "s3";
const BUCKET: &str = "RBS_STORAGE__BUCKET";

/// Identifiants du backend S3, dont `.env.example` porte une valeur à remplacer.
const IDENTIFIANTS: [&str; 2] = [
    "RBS_STORAGE__ACCESS_KEY_ID",
    "RBS_STORAGE__SECRET_ACCESS_KEY",
];

/// Vérifie ce dont la feature `storage` a besoin pour déposer.
pub(crate) fn check(root: &Path, config: &Config) -> Check {
    check_with(root, config, |key| std::env::var(key).ok())
}

/// Le contrôle, l'environnement passé en paramètre.
///
/// L'environnement l'emporte sur le `.env`, comme dans `auth::check_with`.
fn check_with(root: &Path, config: &Config, env: impl Fn(&str) -> Option<String>) -> Check {
    let du_fichier = dotenv::read(&root.join(FICHIER)).unwrap_or_default();
    let de_l_exemple = dotenv::read(&root.join(EXEMPLE)).unwrap_or_default();

    let lire = |key: &str| {
        env(key)
            .or_else(|| dotenv::value(&du_fichier, key).map(str::to_owned))
            .filter(|value| !value.is_empty())
    };

    let mut defauts = Vec::new();
    let mut remedes = Vec::new();

    if !config.section(SECTION) {
        defauts.push(format!("{CONFIG} ne porte pas de section `[{SECTION}]`"));
        remedes.push(format!(
            "ajoutez à {CONFIG} :\n[{SECTION}]\nbackend = \"fs\"\nroot = \"./storage\""
        ));
    }

    // Le backend fichiers se passe de tout réglage : rien de ce qui suit ne le concerne.
    if config.field(SECTION, BACKEND).as_deref() == Some(S3) {
        if config
            .field(SECTION, "bucket")
            .filter(|value| !value.is_empty())
            .is_none()
            && lire(BUCKET).is_none()
        {
            defauts.push(format!(
                "backend = \"{S3}\" sans bucket : ni {CONFIG} ni {BUCKET} n'en nomment un"
            ));
            remedes.push(format!("nommez le bucket dans le {FICHIER} :\n{BUCKET}=…"));
        }

        // Comparés à `.env.example` plutôt qu'à une chaîne écrite ici : ce fichier est la
        // référence, et une reformulation d'`add storage` n'a alors rien à resynchroniser.
        let inchanges: Vec<&str> = IDENTIFIANTS
            .into_iter()
            .filter(|cle| {
                lire(cle)
                    .is_some_and(|value| dotenv::value(&de_l_exemple, cle) == Some(value.as_str()))
            })
            .collect();

        if !inchanges.is_empty() {
            defauts.push(format!(
                "{} {} restée{s} à la valeur d'exemple, publiée dans Git",
                inchanges.join(" et "),
                if inchanges.len() > 1 { "sont" } else { "est" },
                s = if inchanges.len() > 1 { "s" } else { "" }
            ));
            remedes.push(format!(
                "remplacez dans le {FICHIER} :\n{}",
                inchanges
                    .iter()
                    .map(|cle| format!("{cle}=…"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    }

    if defauts.is_empty() {
        return Check::ok(TITRE, "le stockage est configuré");
    }

    Check::failed(TITRE, defauts.join(" ; "), remedes.join("\n"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::{Config, State};
    use super::*;

    /// La valeur que `add storage` écrit dans `.env.example` pour les deux identifiants.
    const EXEMPLE_DES_CLES: &str = "changez-moi";

    /// Un projet neuf, doté à la main de ce que `add storage` y dépose.
    fn project_with_storage() -> (TempDir, PathBuf) {
        let (parent, root) = crate::fixtures::project();

        add(
            &root,
            CONFIG,
            "\n[storage]\nbackend = \"fs\"\nroot = \"./storage\"\n",
        );
        for fichier in [EXEMPLE, FICHIER] {
            add(&root, fichier, &format!("{BUCKET}=demo\n"));
            for cle in IDENTIFIANTS {
                add(&root, fichier, &format!("{cle}={EXEMPLE_DES_CLES}\n"));
            }
        }

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

    /// Passe le projet en S3, identifiants renseignés : la configuration attendue de
    /// quiconque sort du backend fichiers.
    fn switch_to_s3(root: &Path) {
        rewrite(root, CONFIG, "backend = \"fs\"", "backend = \"s3\"");
        for cle in IDENTIFIANTS {
            rewrite(
                root,
                FICHIER,
                &format!("{cle}={EXEMPLE_DES_CLES}"),
                &format!("{cle}=une-vraie-valeur"),
            );
        }
    }

    /// Sans environnement : ce que voit un utilisateur qui n'a rien exporté.
    fn bare(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn s3_without_a_bucket_is_reported() {
        let (_parent, root) = project_with_storage();
        switch_to_s3(&root);
        rewrite(&root, FICHIER, &format!("{BUCKET}=demo\n"), "");

        let check = check_with(&root, &Config::read(&root), bare);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("bucket"),
            "le détail doit nommer ce qui manque : {}",
            check.detail
        );
    }

    /// Le bucket a deux sources : la section et l'environnement.
    #[test]
    fn a_bucket_from_the_environment_is_enough() {
        let (_parent, root) = project_with_storage();
        switch_to_s3(&root);
        rewrite(&root, FICHIER, &format!("{BUCKET}=demo\n"), "");

        let check = check_with(&root, &Config::read(&root), |key| {
            (key == BUCKET).then(|| "depots".to_string())
        });

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// Le backend fichiers n'a que faire d'un bucket.
    #[test]
    fn the_file_backend_needs_no_bucket() {
        let (_parent, root) = project_with_storage();
        rewrite(&root, FICHIER, &format!("{BUCKET}=demo\n"), "");

        let check = check_with(&root, &Config::read(&root), bare);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    #[test]
    fn s3_credentials_left_at_the_example_value_are_reported() {
        let (_parent, root) = project_with_storage();
        rewrite(&root, CONFIG, "backend = \"fs\"", "backend = \"s3\"");

        let check = check_with(&root, &Config::read(&root), bare);

        assert_eq!(
            check.state,
            State::Echec,
            "des identifiants publiés dans Git ne valent pas mieux qu'aucun : {}",
            check.detail
        );
        assert!(
            check.detail.contains("exemple"),
            "le détail doit dire d'où vient la valeur : {}",
            check.detail
        );
    }

    #[test]
    fn without_a_storage_section_the_diagnosis_says_so() {
        let (_parent, root) = project_with_storage();
        rewrite(&root, CONFIG, "[storage]", "# section retirée par le test");

        let check = check_with(&root, &Config::read(&root), bare);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("[storage]"),
            "le détail doit nommer la section : {}",
            check.detail
        );
    }

    #[test]
    fn a_properly_configured_s3_project_reports_nothing() {
        let (_parent, root) = project_with_storage();
        switch_to_s3(&root);

        let check = check_with(&root, &Config::read(&root), bare);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }
}
