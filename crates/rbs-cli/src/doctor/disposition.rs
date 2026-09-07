//! Contrôle de la disposition des modules installés.
//!
//! Un projet engendré avant que `rbs add` ne range ses modules porte les siens à la
//! racine de `src/`. Le CLI ne les déplace pas — réécrire ses `use` reviendrait à
//! toucher à l'AST du développeur — mais un projet qui porte les deux dispositions à la
//! fois a droit de le savoir.

use std::path::Path;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "disposition";

/// Les répertoires que `rbs add` déposait à la racine de `src/`.
///
/// `auth` n'y figure pas : il y vit toujours. `cache` est le répertoire du fragment
/// `redis`, qui ne porte pas son nom.
const ANCIENS: [&str; 10] = [
    "audit",
    "cache",
    "cors",
    "jobs",
    "mail",
    "observability",
    "rate_limit",
    "scheduler",
    "storage",
    "webhooks",
];

/// Signale un projet qui porte les deux dispositions à la fois.
///
/// Le remède est manuel et le reste : déplacer `src/mail/` demanderait de réécrire les
/// `use crate::mail::` du développeur, c'est-à-dire de toucher à un AST que le CLI
/// s'interdit — et de risquer du code qu'il n'a pas écrit.
pub(crate) fn check(root: &Path) -> Check {
    let src = root.join("src");

    if !src.join("modules").is_dir() {
        return Check::ok(TITRE, "aucun module ne mélange les deux dispositions");
    }

    let restes: Vec<String> = ANCIENS
        .into_iter()
        .filter(|ancien| src.join(ancien).is_dir())
        .map(|ancien| format!("src/{ancien}"))
        .collect();

    if restes.is_empty() {
        return Check::ok(TITRE, "aucun module ne mélange les deux dispositions");
    }

    Check::warned(
        TITRE,
        format!("hors de src/modules/ : {}", restes.join(", ")),
        "posés par une version antérieure ; rbs ne les déplacera pas — déplacez-les et \
         corrigez leurs `use` si vous voulez une disposition unique",
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Un projet dont `src/` porte les répertoires nommés, et `modules/` si demandé.
    fn projet(anciens: &[&str], avec_modules: bool) -> TempDir {
        let racine = TempDir::new().expect("répertoire temporaire créable");
        for ancien in anciens {
            fs::create_dir_all(racine.path().join("src").join(ancien))
                .expect("le répertoire se crée");
        }
        if avec_modules {
            fs::create_dir_all(racine.path().join("src/modules/audit"))
                .expect("le répertoire se crée");
        }
        racine
    }

    /// Un projet antérieur est cohérent : il n'a rien à lire à ce sujet tant qu'il n'a
    /// pas commencé à recevoir des modules rangés.
    #[test]
    fn a_project_entirely_in_the_old_layout_is_not_warned() {
        let racine = projet(&["mail", "storage"], false);

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{check:?}");
    }

    #[test]
    fn a_project_carrying_both_layouts_is_warned_and_names_the_directories() {
        let racine = projet(&["mail", "cache"], true);

        let check = check(racine.path());

        assert_eq!(check.state, State::Avertissement, "{check:?}");
        assert!(check.detail.contains("src/mail"), "{}", check.detail);
        assert!(check.detail.contains("src/cache"), "{}", check.detail);
    }

    /// `auth` vit à la racine par décision, non par ancienneté : le signaler ferait
    /// avertir tout projet authentifié.
    #[test]
    fn auth_at_the_root_is_never_a_mixed_layout() {
        let racine = projet(&["auth"], true);

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{check:?}");
    }

    #[test]
    fn a_project_only_in_the_new_layout_is_good() {
        let racine = projet(&[], true);

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{check:?}");
    }
}
