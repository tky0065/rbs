//! Contrôle du `.env` du projet.
//!
//! `.env.example` sert de référence : il est versionné, généré par le squelette et mis à
//! jour en même temps que lui. Comparer à une liste tenue dans le CLI aurait fait deux
//! vérités à synchroniser.

use std::path::Path;

use crate::dotenv;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = ".env";
const FICHIER: &str = ".env";
const EXEMPLE: &str = ".env.example";

/// Vérifie que le `.env` porte tout ce que `.env.example` déclare avec une valeur.
pub(crate) fn check(root: &Path) -> Check {
    let attendues = match dotenv::read(&root.join(EXEMPLE)) {
        Ok(paires) => paires,
        Err(error) => {
            return Check::failed(
                TITRE,
                error.to_string(),
                format!("{EXEMPLE} est la référence du diagnostic : restaurez-le depuis Git"),
            );
        }
    };

    let presentes = match dotenv::read(&root.join(FICHIER)) {
        Ok(paires) => paires,
        Err(error) => {
            return Check::failed(
                TITRE,
                error.to_string(),
                format!("cp {EXEMPLE} {FICHIER}, puis renseignez l'URL de votre base"),
            );
        }
    };

    // Une variable propre au projet est légitime : seule l'absence est un défaut. Et une
    // clé que l'exemple laisse vide — le mot de passe SMTP de `mail`, inutile tant que le
    // serveur local n'authentifie personne — n'a rien à apprendre du `.env` : l'exiger
    // ferait échouer `doctor` sur un projet qui sort de `rbs new`.
    let mut manquantes = Vec::new();
    let mut videes = Vec::new();
    for (key, exemple) in &attendues {
        if dotenv::value(&presentes, key).is_some() {
            continue;
        }
        if exemple.is_empty() {
            videes.push(key.as_str());
        } else {
            manquantes.push(key.as_str());
        }
    }

    if manquantes.is_empty() && videes.is_empty() {
        return Check::ok(
            TITRE,
            format!(
                "les {} variables de {EXEMPLE} sont renseignées",
                attendues.len()
            ),
        );
    }

    if manquantes.is_empty() {
        let (s, peut) = if videes.len() > 1 {
            ("s", "peuvent")
        } else {
            ("", "peut")
        };
        return Check::ok(
            TITRE,
            format!(
                "{} des {} variables de {EXEMPLE} sont renseignées ; {}, vide{s} dans l'exemple, {peut} manquer",
                attendues.len() - videes.len(),
                attendues.len(),
                videes.join(", ")
            ),
        );
    }

    Check::failed(
        TITRE,
        format!(
            "{} absente{} du {FICHIER}",
            manquantes.join(", "),
            if manquantes.len() > 1 { "s" } else { "" }
        ),
        format!(
            "ajoutez au {FICHIER} :\n{}",
            manquantes
                .iter()
                .map(|key| format!("{key}={}", dotenv::value(&attendues, key).unwrap_or("")))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )
}

/// Le contrôle `.env` a-t-il déjà nommé `cle`, avec son remède ?
///
/// Il le fait dès que `.env.example` la déclare avec une valeur et que le `.env` ne la
/// porte pas — ou ne se lit pas. Un contrôle de feature qui la nommerait aussi compterait
/// deux fautes pour une ligne à écrire, avec deux remèdes différents.
pub(crate) fn signalee(root: &Path, cle: &str) -> bool {
    let Ok(attendues) = dotenv::read(&root.join(EXEMPLE)) else {
        return false;
    };
    if dotenv::value(&attendues, cle).is_none_or(str::is_empty) {
        return false;
    }
    match dotenv::read(&root.join(FICHIER)) {
        Ok(presentes) => dotenv::value(&presentes, cle).is_none(),
        Err(_) => true,
    }
}

/// `.env.example` déclare-t-il `cle` sans valeur ?
///
/// Le contrôle `.env` tolère alors son absence : un contrôle de feature doit la lire
/// comme vide, et non comme oubliée.
pub(crate) fn laissee_vide(root: &Path, cle: &str) -> bool {
    dotenv::read(&root.join(EXEMPLE))
        .is_ok_and(|attendues| dotenv::value(&attendues, cle) == Some(""))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::State;
    use super::*;
    use crate::fixtures::project;

    /// Retire du `.env` la ligne portant `key`.
    fn remove(root: &Path, key: &str) {
        let path = root.join(FICHIER);
        let source = fs::read_to_string(&path).expect("le .env est lisible");
        let ampute: Vec<_> = source
            .lines()
            .filter(|line| !line.starts_with(key))
            .collect();
        fs::write(&path, ampute.join("\n")).expect("le .env est réécrivable");
    }

    #[test]
    fn a_fresh_project_has_a_complete_env() {
        let (_parent, root) = project();

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
        assert!(check.remedy.is_none());
    }

    #[test]
    fn a_variable_from_the_example_missing_from_env_is_named() {
        let (_parent, root) = project();
        remove(&root, "RBS_LOG_FORMAT");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("RBS_LOG_FORMAT"));
        assert!(
            check
                .remedy
                .expect("un échec porte son remède")
                .contains("RBS_LOG_FORMAT")
        );
    }

    #[test]
    fn the_finding_agrees_with_the_number_of_missing_variables() {
        let (_parent, root) = project();
        remove(&root, "RBS_LOG_FORMAT");

        assert!(check(&root).detail.contains("absente du"));

        remove(&root, "RUST_LOG");

        assert!(check(&root).detail.contains("absentes du"));
    }

    #[test]
    fn a_project_specific_variable_does_not_get_in_the_way() {
        let (_parent, root) = project();
        let path = root.join(FICHIER);
        let source = fs::read_to_string(&path).expect("le .env est lisible");
        fs::write(&path, format!("{source}\nSTRIPE_KEY=sk_test\n")).expect("écriture");

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    #[test]
    fn a_missing_env_points_to_the_example_that_rebuilds_it() {
        let (_parent, root) = project();
        fs::remove_file(root.join(FICHIER)).expect("le .env existe");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(
            check
                .remedy
                .expect("un échec porte son remède")
                .contains(EXEMPLE)
        );
    }

    #[test]
    fn a_declared_key_missing_from_env_is_already_reported() {
        let (_parent, root) = project();
        remove(&root, "RBS_LOG_FORMAT");

        assert!(signalee(&root, "RBS_LOG_FORMAT"));
    }

    #[test]
    fn every_declared_key_is_already_reported_when_env_is_missing() {
        let (_parent, root) = project();
        fs::remove_file(root.join(FICHIER)).expect("le .env existe");

        assert!(signalee(&root, "RBS_LOG_FORMAT"));
    }

    #[test]
    fn a_key_the_example_does_not_declare_is_not_reported() {
        let (_parent, root) = project();

        assert!(!signalee(&root, "STRIPE_KEY"));
    }

    #[test]
    fn a_key_the_env_carries_is_not_reported() {
        let (_parent, root) = project();

        assert!(!signalee(&root, "RBS_LOG_FORMAT"));
    }

    /// Ajoute `ligne` à `fichier`.
    fn append(root: &Path, fichier: &str, ligne: &str) {
        let path = root.join(fichier);
        let mut source = fs::read_to_string(&path).expect("le fichier est lisible");
        if !source.ends_with('\n') {
            source.push('\n');
        }
        fs::write(&path, format!("{source}{ligne}\n")).expect("le fichier est réécrivable");
    }

    /// Ce que dépose le `[[env]]` non secret de `mail` : la clé, vide, dans l'exemple seul.
    #[test]
    fn a_key_the_example_leaves_empty_may_be_missing_from_env() {
        for declaration in [
            "SMTP_PASSWORD=",
            "SMTP_PASSWORD=   ",
            "SMTP_PASSWORD=  # vide tant que smtp_user l'est",
            "SMTP_PASSWORD=\"\"",
            "export SMTP_PASSWORD=''",
        ] {
            let (_parent, root) = project();
            append(&root, EXEMPLE, declaration);

            let check = check(&root);

            assert_eq!(check.state, State::Bon, "{declaration} : {}", check.detail);
            assert!(
                check.detail.contains("SMTP_PASSWORD"),
                "le constat doit nommer la clé laissée vide : {}",
                check.detail
            );
            assert!(!signalee(&root, "SMTP_PASSWORD"), "{declaration}");
        }
    }

    /// Le constat ne compte comme renseignées que les clés que le `.env` porte.
    #[test]
    fn the_finding_counts_only_the_keys_the_env_carries() {
        let (_parent, root) = project();
        let declarees = dotenv::read(&root.join(EXEMPLE))
            .expect("exemple lisible")
            .len();
        append(&root, EXEMPLE, "SMTP_PASSWORD=");
        append(&root, EXEMPLE, "SMTP_TOKEN=");

        let detail = check(&root).detail;

        assert!(
            detail.contains(&format!("{declarees} des {} variables", declarees + 2)),
            "{detail}"
        );
        assert!(detail.contains("SMTP_PASSWORD, SMTP_TOKEN"), "{detail}");
        assert!(detail.contains("vides dans"), "{detail}");
    }

    #[test]
    fn a_key_the_example_leaves_empty_and_the_env_carries_is_simply_counted() {
        let (_parent, root) = project();
        let declarees = dotenv::read(&root.join(EXEMPLE))
            .expect("exemple lisible")
            .len();
        append(&root, EXEMPLE, "SMTP_PASSWORD=");
        append(&root, FICHIER, "SMTP_PASSWORD=");

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
        assert_eq!(
            check.detail,
            format!(
                "les {} variables de {EXEMPLE} sont renseignées",
                declarees + 1
            )
        );
    }

    /// Une valeur d'exemple non vide — fût-ce une espace entre guillemets — est une
    /// valeur que le projet attend : son absence reste une faute.
    #[test]
    fn a_key_the_example_gives_a_value_is_still_required() {
        for declaration in ["API_TOKEN=changeme", "API_TOKEN=\" \""] {
            let (_parent, root) = project();
            append(&root, EXEMPLE, declaration);

            let check = check(&root);

            assert_eq!(check.state, State::Echec, "{declaration}");
            assert!(
                check.detail.contains("API_TOKEN absente du"),
                "{}",
                check.detail
            );
            assert!(signalee(&root, "API_TOKEN"), "{declaration}");
        }
    }

    /// Le contrôle vérifie la présence, non le contenu : une ligne vide dans le `.env`
    /// est un choix du développeur.
    #[test]
    fn a_key_present_but_empty_in_env_is_accepted() {
        let (_parent, root) = project();
        append(&root, EXEMPLE, "API_TOKEN=changeme");
        append(&root, FICHIER, "API_TOKEN=");

        assert_eq!(check(&root).state, State::Bon);
        assert!(!signalee(&root, "API_TOKEN"));
    }

    #[test]
    fn without_the_example_file_the_check_says_so_rather_than_concluding_green() {
        let (_parent, root) = project();
        fs::remove_file(root.join(EXEMPLE)).expect("l'exemple existe");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains(EXEMPLE));
    }
}
