//! Contrôle de l'`AGENTS.md` du projet, et de la règle qu'il énonce.
//!
//! Un guide périmé n'induit pas un développeur en erreur : il induit en erreur tout agent
//! qui travaille sur le projet, et sans qu'aucun d'eux ait de quoi s'en apercevoir. C'est
//! ce que ce contrôle regarde, avec le seul constat que le CLI puisse faire de la règle
//! « le CLI d'abord » : nommer le code qui n'est pas passé par lui.

use std::path::Path;

use crate::agents;
use crate::metadata;

use super::Check;

const TITRE: &str = "agents";

/// Répertoires de `src/` qui ne sont pas des features engendrées.
///
/// `health` est le module du squelette et `seeds` le binaire des données de démonstration ;
/// les autres sont les répertoires que les fragments déposent sous un nom qui n'est pas le
/// leur — `redis` s'installe en `src/cache/`. Les compter comme écrits à la main ferait
/// avertir sur chaque projet qui les porte.
const HORS_FEATURES: [&str; 3] = ["health", "seeds", "cache"];

/// Contrôle l'`AGENTS.md` du projet, et nomme le code qui n'est pas passé par le CLI.
pub(crate) fn check(root: &Path) -> Check {
    let Ok(metadonnees) = metadata::read(&root.join("Cargo.toml")) else {
        return Check::failed(
            TITRE,
            "le manifeste du projet est illisible",
            "vérifiez [package.metadata.rbs] dans Cargo.toml",
        );
    };

    let Ok(present) = std::fs::read_to_string(root.join(agents::FICHIER)) else {
        return Check::failed(
            TITRE,
            format!("{} est absent", agents::FICHIER),
            "rbs upgrade le recrée",
        );
    };

    for zone in [agents::GUIDE, agents::INVENTORY] {
        if agents::body(&present, zone).is_none() {
            let manquante = agents::MissingZone {
                zone: zone.to_string(),
            };

            return Check::failed(
                TITRE,
                format!("la zone `rbs:{zone}` manque à {}", agents::FICHIER),
                format!(
                    "collez ce bloc dans {} :\n{}",
                    agents::FICHIER,
                    manquante.block()
                ),
            );
        }
    }

    match agents::version(&present, agents::GUIDE) {
        Some(version) if version != agents::VERSION => {
            return Check::failed(
                TITRE,
                format!("le guide est en {version}, le CLI en {}", agents::VERSION),
                "rbs upgrade réécrit le guide",
            );
        }
        None => {
            return Check::failed(
                TITRE,
                "le guide ne porte pas de version",
                "rbs upgrade réécrit le guide",
            );
        }
        Some(_) => {}
    }

    // Ces deux constats sont indépendants — une feature ajoutée à la main au manifeste
    // dérègle l'inventaire *et* n'a pas de répertoire — et se cumulent donc dans le même
    // échec au lieu de se masquer l'un l'autre : le premier trouvé tairait la seconde
    // cause à un développeur qui vient de corriger la première.
    let mut echecs: Vec<(String, String)> = Vec::new();

    if let Ok(attendu) = agents::inventory(root, metadonnees.lang)
        && agents::body(&present, agents::INVENTORY) != Some(attendu.as_str())
    {
        echecs.push((
            "l'inventaire ne décrit plus le projet".to_string(),
            "rbs upgrade le recalcule".to_string(),
        ));
    }

    if let Some(declaree) = declared_without_directory(root, &metadonnees.features) {
        echecs.push((
            format!("`{declaree}` est déclarée sans que src/{declaree}/ existe"),
            format!("rbs add {declaree}, ou retirez la ligne de [package.metadata.rbs]"),
        ));
    }

    if !echecs.is_empty() {
        let detail = echecs
            .iter()
            .map(|(detail, _)| detail.as_str())
            .collect::<Vec<_>>()
            .join(" ; ");
        let remedy = echecs
            .iter()
            .map(|(_, remedy)| remedy.as_str())
            .collect::<Vec<_>>()
            .join(" ; ");

        return Check::failed(TITRE, detail, remedy);
    }

    let hors_cli = written_by_hand(root, &metadonnees.features);
    if !hors_cli.is_empty() {
        return Check::warned(
            TITRE,
            format!("écrit hors du CLI : {}", hors_cli.join(", ")),
            "légitime si rbs ne couvre pas ce code ; sinon, rbs generate le reprend",
        );
    }

    Check::ok(TITRE, "guide et inventaire à jour")
}

/// Une feature déclarée dont le répertoire manque, s'il y en a une.
///
/// Un fragment peut écrire ailleurs que dans `src/<nom>/` — `redis` va dans `src/cache/` —
/// et n'est donc pas jugé ici : ce contrôle vise les entités engendrées, dont le
/// répertoire porte toujours le nom.
fn declared_without_directory(root: &Path, features: &[String]) -> Option<String> {
    let catalogue = crate::templates::feature_names(None);

    features
        .iter()
        .filter(|feature| !catalogue.contains(*feature))
        .filter(|feature| !HORS_FEATURES.contains(&feature.as_str()))
        .find(|feature| !root.join("src").join(feature.as_str()).is_dir())
        .cloned()
}

/// Les répertoires de `src/` que rien ne déclare : du code écrit à la main.
fn written_by_hand(root: &Path, features: &[String]) -> Vec<String> {
    let Ok(entrees) = std::fs::read_dir(root.join("src")) else {
        return Vec::new();
    };

    let mut noms: Vec<String> = entrees
        .flatten()
        .filter(|entree| entree.path().is_dir())
        .filter_map(|entree| entree.file_name().into_string().ok())
        .filter(|nom| !HORS_FEATURES.contains(&nom.as_str()))
        .filter(|nom| !features.iter().any(|feature| feature == nom))
        .collect();

    noms.sort();
    noms
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::doctor::State;
    use crate::fixtures::project;

    #[test]
    fn a_freshly_created_project_passes() {
        let (_parent, root) = project();

        assert_eq!(check(&root).state, State::Bon);
    }

    #[test]
    fn a_missing_file_is_a_failure_pointing_at_upgrade() {
        let (_parent, root) = project();
        fs::remove_file(root.join("AGENTS.md")).expect("le fichier existe");

        let constat = check(&root);

        assert_eq!(constat.state, State::Echec);
        assert!(
            constat
                .remedy
                .as_deref()
                .is_some_and(|r| r.contains("rbs upgrade")),
            "{constat:?}"
        );
    }

    #[test]
    fn a_missing_zone_is_a_failure_carrying_the_block_to_paste() {
        let (_parent, root) = project();
        let agents = root.join("AGENTS.md");
        let sans_zone = fs::read_to_string(&agents)
            .expect("AGENTS.md est écrit")
            .replace("<!-- rbs:inventory -->", "")
            .replace("<!-- /rbs:inventory -->", "");
        fs::write(&agents, sans_zone).expect("l'écriture aboutit");

        let constat = check(&root);

        assert_eq!(constat.state, State::Echec);
        assert!(
            constat
                .remedy
                .as_deref()
                .is_some_and(|r| r.contains("<!-- rbs:inventory -->")),
            "{constat:?}"
        );
    }

    #[test]
    fn an_outdated_guide_version_is_a_failure() {
        let (_parent, root) = project();
        let agents = root.join("AGENTS.md");
        let vieilli = fs::read_to_string(&agents)
            .expect("AGENTS.md est écrit")
            .replace(
                &crate::agents::opening(crate::agents::GUIDE, Some(crate::agents::VERSION)),
                &crate::agents::opening(crate::agents::GUIDE, Some("0.9.0")),
            );
        fs::write(&agents, vieilli).expect("l'écriture aboutit");

        let constat = check(&root);

        assert_eq!(constat.state, State::Echec);
        assert!(constat.detail.contains("0.9.0"), "{constat:?}");
    }

    #[test]
    fn an_outdated_inventory_is_a_failure() {
        let (_parent, root) = project();
        let agents = root.join("AGENTS.md");
        let fausse = crate::agents::replace(
            &fs::read_to_string(&agents).expect("AGENTS.md est écrit"),
            crate::agents::INVENTORY,
            "- rbs 0.0.1 · base mysql",
        )
        .expect("la zone est présente");
        fs::write(&agents, fausse).expect("l'écriture aboutit");

        assert_eq!(check(&root).state, State::Echec);
    }

    /// Le cas symétrique : aucun contrôle existant ne le couvre, et un projet qui déclare
    /// une feature sans porter ses fichiers ne compile pas.
    #[test]
    fn a_declared_feature_without_its_directory_is_a_failure() {
        let (_parent, root) = project();
        let manifest = root.join("Cargo.toml");
        let source = fs::read_to_string(&manifest).expect("manifeste lisible");
        let patched = crate::metadata::record_feature(&source, "articles", "Cargo.toml")
            .expect("le manifeste accepte la feature")
            .expect("la feature n'y est pas encore");
        fs::write(&manifest, patched).expect("manifeste réécrit");

        let constat = check(&root);

        assert_eq!(constat.state, State::Echec);
        assert!(constat.detail.contains("articles"), "{constat:?}");
    }

    /// Le contrôle « CLI first » proprement dit. Ce n'est pas une erreur : le guide
    /// autorise d'écrire à la main ce que rbs ne couvre pas, et un échec ici ferait
    /// rougir la CI d'un projet sain.
    #[test]
    fn a_module_written_by_hand_is_a_warning_not_a_failure() {
        let (_parent, root) = project();
        fs::create_dir_all(root.join("src/webhooks")).expect("répertoire créable");
        fs::write(root.join("src/webhooks/mod.rs"), "// à la main\n").expect("l'écriture aboutit");

        let constat = check(&root);

        assert_eq!(constat.state, State::Avertissement);
        assert!(constat.detail.contains("webhooks"), "{constat:?}");
    }

    /// `redis` s'installe en `src/cache/` : compter ce répertoire comme écrit à la main
    /// ferait avertir sur chaque projet qui installe la feature.
    #[test]
    fn a_directory_deposited_by_a_fragment_under_another_name_is_not_a_warning() {
        let (_parent, root) = project();
        crate::add::plan_for(&crate::add::Options {
            feature: "redis".to_string(),
            directory: root.clone(),
            force: true,
            template_dir: None,
        })
        .and_then(|planned| {
            crate::plan::application::apply(&planned.plan, true)?;
            Ok(planned)
        })
        .expect("redis doit s'installer");

        let constat = check(&root);

        assert_ne!(constat.state, State::Avertissement, "{constat:?}");
    }

    /// Ni le module du squelette ni le binaire des seeds ne sont des features.
    #[test]
    fn the_skeleton_directories_are_not_warned_about() {
        let (_parent, root) = project();

        let constat = check(&root);

        assert!(!constat.detail.contains("health"), "{constat:?}");
        assert!(!constat.detail.contains("seeds"), "{constat:?}");
    }
}
