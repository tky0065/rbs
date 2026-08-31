//! Contrôle des points d'insertion du projet.
//!
//! Une ancre disparue ne casse rien tant qu'aucune génération n'a lieu : c'est
//! précisément pourquoi `doctor` la cherche avant que `rbs generate` ne bute dessus.

use std::fs;
use std::path::Path;

use crate::anchors::{self, Anchor};

use super::Check;

const TITRE: &str = "ancres";

/// Vérifie que le projet porte toutes ses ancres, et dit comment recoller les absentes.
pub(crate) fn check(root: &Path) -> Check {
    // L'ancre des features se résout par repli : `src/lib.rs` sur un projet engendré
    // depuis ce jalon, `src/main.rs` sur un projet plus ancien, dépourvu de bibliothèque.
    let anchors = anchors::resolved(root);

    // Une ancre optionnelle dont le fichier n'existe pas n'est pas applicable : la
    // réclamer ferait passer pour incomplet un projet qui ne l'est pas.
    let applicables: Vec<&Anchor> = anchors
        .iter()
        .filter(|anchor| !anchor.optional || root.join(anchor.file.as_ref()).exists())
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
    fs::read_to_string(root.join(anchor.file.as_ref())).is_ok_and(|source| {
        anchors::marks(&source, &anchor.opening()) && anchors::marks(&source, &anchor.closing())
    })
}

#[cfg(test)]
mod tests {
    use crate::anchors::ANCRES;
    use crate::fixtures::project;

    use super::super::State;
    use super::*;

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
        assert!(
            check.detail.contains(&ANCRES.len().to_string()),
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

    /// Les deux ancres du routeur se contrôlent séparément : une couche insérée dans
    /// `routes` n'envelopperait rien, et le diagnostic doit nommer celle qui manque.
    #[test]
    fn the_layers_anchor_is_claimed_on_its_own() {
        let (_parent, root) = project();
        remove(&root, "src/router.rs", "<rbs:layers>");
        remove(&root, "src/router.rs", "</rbs:layers>");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(
            check.detail.contains("layers manque dans src/router.rs"),
            "{}",
            check.detail
        );
        assert!(
            !check.detail.contains("routes manque"),
            "l'autre ancre du fichier est intacte : {}",
            check.detail
        );

        let remedy = check.remedy.expect("un échec porte son remède");
        assert!(remedy.contains("// <rbs:layers>"), "{remedy}");
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
    fn an_optional_anchor_whose_file_is_absent_is_not_missing() {
        let (_parent, root) = project();
        fs::remove_file(root.join("docker-compose.yml")).expect("le compose doit exister");

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{check:?}");
        assert!(
            check.detail.contains(&(ANCRES.len() - 1).to_string()),
            "l'ancre du compose ne compte pas parmi les applicables : {}",
            check.detail
        );
    }

    /// Une balise citée dans une chaîne n'est pas un point d'insertion : `doctor` doit
    /// la voir absente, exactement comme `generate`, faute de quoi il annonce sain un
    /// projet où l'insertion échouera.
    #[test]
    fn an_anchor_quoted_inside_a_string_does_not_count_as_present() {
        let (_parent, root) = project();
        let router = root.join("src/router.rs");
        let source = fs::read_to_string(&router).expect("le routeur est lisible");
        let cite = source
            .replace("// <rbs:routes>", "let doc = \"// <rbs:routes>\";")
            .replace("// </rbs:routes>", "let fin = \"// </rbs:routes>\";");
        fs::write(&router, cite).expect("routeur réécrivable");

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{check:?}");
        assert!(
            check.detail.contains("routes manque dans src/router.rs"),
            "{}",
            check.detail
        );
    }

    #[test]
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
