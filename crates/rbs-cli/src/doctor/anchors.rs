//! Contrôle des points d'insertion du projet.
//!
//! Une ancre disparue ne casse rien tant qu'aucune génération n'a lieu : c'est
//! précisément pourquoi `doctor` la cherche avant que `rbs generate` ne bute dessus.

use std::fs;
use std::path::Path;

use crate::anchors::{ANCRES, Anchor};

use super::Check;

const TITRE: &str = "ancres";

/// Vérifie que le projet porte toutes ses ancres, et dit comment recoller les absentes.
pub(crate) fn check(root: &Path) -> Check {
    // Une ancre optionnelle dont le fichier n'existe pas n'est pas applicable : la
    // réclamer ferait passer pour incomplet un projet qui ne l'est pas.
    let applicables: Vec<&Anchor> = ANCRES
        .iter()
        .filter(|anchor| !anchor.optional || root.join(anchor.file).exists())
        .collect();

    let absentes: Vec<&&Anchor> = applicables.iter().filter(|a| !present(root, a)).collect();

    if absentes.is_empty() {
        return Check::ok(
            TITRE,
            format!("les {} points d'insertion sont en place", applicables.len()),
        );
    }

    let detail = absentes
        .iter()
        .map(|a| format!("{} manque dans {}", a.name, a.file))
        .collect::<Vec<_>>()
        .join(", ");

    let remedy = absentes
        .iter()
        .map(|a| format!("dans {} :\n{}", a.file, a.block()))
        .collect::<Vec<_>>()
        .join("\n\n");

    Check::failed(TITRE, detail, remedy)
}

/// Vrai si le fichier porteur existe et contient les deux balises de l'ancre.
///
/// Un fichier illisible vaut ancre absente : le diagnostic le signale par le nom du
/// fichier plutôt que de s'interrompre.
fn present(root: &Path, anchor: &Anchor) -> bool {
    fs::read_to_string(root.join(anchor.file)).is_ok_and(|source| {
        source.contains(&anchor.opening()) && source.contains(&anchor.closing())
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    fn project() -> (TempDir, PathBuf) {
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

        (parent, project.root)
    }

    /// Retire du projet la ligne portant `motif`.
    fn remove(root: &Path, file: &str, motif: &str) {
        let path = root.join(file);
        let source = fs::read_to_string(&path).expect("le fichier est lisible");
        let ampute: Vec<_> = source.lines().filter(|l| !l.contains(motif)).collect();
        fs::write(&path, ampute.join("\n")).expect("le fichier est réécrivable");
    }

    #[test]
    fn a_fresh_project_carries_all_its_anchors() {
        let (_parent, root) = project();

        let check = check(&root);

        assert_eq!(check.state, State::Bon);
        // Le squelette n'écrit pas encore de compose : seules les neuf ancres non
        // optionnelles sont applicables. `ANCRES.len()` vaudra de nouveau ce compte une
        // fois `docker-compose.yml` généré.
        assert!(
            check.detail.contains(&(ANCRES.len() - 1).to_string()),
            "{}",
            check.detail
        );
        assert!(check.remedy.is_none());
    }

    #[test]
    fn a_deleted_anchor_is_reported_with_the_block_to_paste() {
        let (_parent, root) = project();
        remove(&root, "src/router.rs", "<rbs:routes>");
        remove(&root, "src/router.rs", "</rbs:routes>");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("routes"));
        assert!(check.detail.contains("src/router.rs"));

        let remedy = check.remedy.expect("un échec porte son remède");
        assert!(remedy.contains("// <rbs:routes>"));
        assert!(remedy.contains("// </rbs:routes>"));
        assert!(
            remedy.contains("src/router.rs"),
            "le remède dit où coller le bloc"
        );
    }

    /// Le huitième point d'insertion vit dans un second binaire, hors de `src/main.rs` :
    /// sans ce test, l'oublier dans la liste ne se verrait nulle part.
    #[test]
    fn the_seeds_anchor_is_one_of_those_counted() {
        let (_parent, root) = project();
        remove(&root, "src/seeds/main.rs", "<rbs:seeds>");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("seeds"), "{}", check.detail);
        assert!(
            check.detail.contains("src/seeds/main.rs"),
            "{}",
            check.detail
        );
    }

    #[test]
    fn an_anchor_missing_its_closing_counts_as_absent() {
        let (_parent, root) = project();
        remove(&root, "src/router.rs", "</rbs:routes>");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("routes"));
    }

    #[test]
    fn both_anchors_of_one_file_are_checked_separately() {
        let (_parent, root) = project();
        remove(&root, "migration/src/lib.rs", "<rbs:migrations>");
        remove(&root, "migration/src/lib.rs", "</rbs:migrations>");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("migrations"));
        assert!(
            !check.detail.contains("migration_modules"),
            "l'autre ancre du fichier est intacte"
        );
    }

    #[test]
    fn a_vanished_file_is_reported_rather_than_panicking_the_diagnosis() {
        let (_parent, root) = project();
        fs::remove_file(root.join("src/openapi.rs")).expect("le fichier existe");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("src/openapi.rs"));
    }

    #[test]
    #[ignore = "le squelette n'écrit pas encore de compose"]
    fn an_optional_anchor_whose_file_is_absent_is_not_missing() {
        let (_parent, root) = project();
        fs::remove_file(root.join("docker-compose.yml")).expect("le compose doit exister");

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{check:?}");
        assert!(
            check.detail.contains('9'),
            "seules les neuf ancres applicables comptent : {}",
            check.detail
        );
    }

    #[test]
    #[ignore = "le squelette n'écrit pas encore de compose"]
    fn an_optional_anchor_removed_from_a_present_file_is_missing() {
        let (_parent, root) = project();
        remove(&root, "docker-compose.yml", "<rbs:services>");

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{check:?}");
        assert!(
            check
                .detail
                .contains("services manque dans docker-compose.yml"),
            "{}",
            check.detail
        );
    }
}
