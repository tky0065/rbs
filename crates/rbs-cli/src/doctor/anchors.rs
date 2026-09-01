//! Contrôle des points d'insertion du projet.
//!
//! Une ancre disparue ne casse rien tant qu'aucune génération n'a lieu : c'est
//! précisément pourquoi `doctor` la cherche avant que `rbs generate` ne bute dessus.

use std::fs;
use std::path::Path;

use crate::anchors::{self, Anchor};

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "ancres";

/// Vérifie que le projet porte toutes ses ancres, et dit comment recoller les absentes.
pub(crate) fn check(root: &Path) -> Check {
    let (applicables, absentes) = inventaire(root);

    if absentes.is_empty() {
        return Check::ok(
            TITRE,
            format!("les {applicables} points d'insertion sont en place"),
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

/// Les ancres que le projet devrait porter, comptées, et celles qui lui manquent.
///
/// Une ancre optionnelle dont le fichier n'existe pas n'est pas applicable : la réclamer
/// ferait passer pour incomplet un projet qui ne l'est pas.
fn inventaire(root: &Path) -> (usize, Vec<Anchor>) {
    let applicables: Vec<Anchor> = anchors::resolved(root)
        .into_iter()
        .filter(|anchor| !anchor.optional || root.join(anchor.file.as_ref()).exists())
        .collect();

    let absentes = applicables
        .iter()
        .filter(|anchor| !present(root, anchor))
        .cloned()
        .collect();

    (applicables.len(), absentes)
}

/// Une ancre que la réparation n'a pas reposée, et la raison qu'elle en donne.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(crate) struct Laissee {
    /// Nom de l'ancre, tel qu'il paraît entre les chevrons.
    #[serde(rename = "ancre")]
    pub anchor: String,
    /// Pourquoi elle n'a pas été reposée.
    pub raison: String,
}

/// Ce qu'une réparation fera au projet, et ce qu'elle n'y fera pas.
#[derive(Debug)]
pub(crate) struct Repair {
    /// Les écritures, calculées et rien d'écrit.
    pub plan: crate::plan::Plan,
    /// Les ancres que le plan repose, dans l'ordre du registre.
    pub reposees: Vec<String>,
    /// Les ancres qu'il laisse absentes, et pourquoi.
    pub laissees: Vec<Laissee>,
}

/// Planifie la remise en place des ancres absentes du projet.
///
/// Rien n'est écrit ici : le plan s'affiche avant de s'appliquer, comme celui de toute
/// commande qui touche un projet existant.
pub(crate) fn repair(root: &Path) -> Result<Repair, crate::plan::Error> {
    let (_, absentes) = inventaire(root);
    let mut builder = crate::plan::Builder::new(root);
    let mut reposees = Vec::new();
    let mut laissees = Vec::new();

    for anchor in absentes {
        match builder.repose(anchor.clone())? {
            crate::plan::Repose::Reposee => reposees.push(anchor.name.to_string()),
            crate::plan::Repose::Laissee(cause) => laissees.push(Laissee {
                anchor: anchor.name.to_string(),
                raison: cause.raison(&anchor),
            }),
        }
    }

    Ok(Repair {
        plan: builder.finir(),
        reposees,
        laissees,
    })
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
    use crate::anchors::{self, ANCRES};
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

    /// L'indentation de la ligne portant `balise`, dans `source`.
    fn indentation(source: &str, balise: &str) -> String {
        let ligne = source
            .lines()
            .find(|ligne| ligne.trim() == balise)
            .unwrap_or_else(|| panic!("`{balise}` absente :\n{source}"));

        ligne[..ligne.len() - ligne.trim_start().len()].to_string()
    }

    /// La réparation repose l'ancre là où la template l'avait posée, et le diagnostic
    /// relancé repasse au vert.
    #[test]
    fn a_deleted_anchor_is_put_back_and_turns_the_diagnosis_green() {
        let (_parent, root) = project();
        let avant = fs::read_to_string(root.join("src/router.rs")).expect("le routeur est lisible");
        remove(&root, "src/router.rs", "<rbs:routes>");
        remove(&root, "src/router.rs", "</rbs:routes>");
        assert_eq!(
            check(&root).state,
            State::Echec,
            "le test ne prouverait rien"
        );

        let repair = repair(&root).expect("la réparation se planifie");
        crate::plan::application::apply(&repair.plan, false).expect("le plan s'applique");

        assert_eq!(repair.reposees, vec!["routes".to_string()]);
        assert!(repair.laissees.is_empty(), "{:?}", repair.laissees);
        assert_eq!(check(&root).state, State::Bon);

        let apres = fs::read_to_string(root.join("src/router.rs")).expect("le routeur est lisible");
        assert_eq!(
            indentation(&apres, "// <rbs:routes>"),
            indentation(&avant, "// <rbs:routes>")
        );
    }

    /// Chaque ancre du registre déclare une accroche, et cette accroche doit reposer le
    /// bloc à l'indentation qu'il avait : une ancre YAML remise deux colonnes à côté
    /// ferait insérer un service hors de `services:`, et le compose ne s'analyserait plus.
    #[test]
    fn every_anchor_of_the_registry_is_put_back_at_its_own_indentation() {
        let (_parent, root) = project();

        for anchor in anchors::resolved(&root) {
            let path = root.join(anchor.file.as_ref());
            let avant = fs::read_to_string(&path).expect("le fichier porteur est lisible");

            remove(&root, &anchor.file, &anchor.opening());
            remove(&root, &anchor.file, &anchor.closing());

            let repair = repair(&root).expect("la réparation se planifie");
            crate::plan::application::apply(&repair.plan, false).expect("le plan s'applique");

            assert_eq!(
                repair.reposees,
                vec![anchor.name.to_string()],
                "{} n'a pas été reposée : {:?}",
                anchor.name,
                repair.laissees
            );

            let apres = fs::read_to_string(&path).expect("le fichier porteur est lisible");
            assert_eq!(
                indentation(&apres, &anchor.opening()),
                indentation(&avant, &anchor.opening()),
                "{} est reposée à une autre colonne",
                anchor.name
            );

            fs::write(&path, &avant).expect("le fichier se rétablit");
        }
    }

    /// Une accroche que le fichier ne porte pas, ou qu'il porte deux fois, ne dit plus où
    /// reposer le bloc : une ancre posée au hasard coûte plus cher qu'une ancre absente,
    /// et `<rbs:layers>` mise au mauvais endroit ne verrait plus le `request_id`.
    #[test]
    fn an_ambiguous_or_missing_hook_leaves_the_anchor_alone() {
        // L'accroche de `layers` est `.merge(docs)` : réécrite, plus rien ne la porte ;
        // doublée, rien ne désigne celle des deux qui va recevoir le bloc.
        for reecriture in [
            ".merge(openapi::routes(state.core().config()))",
            ".merge(docs)\n        .merge(docs)",
        ] {
            let (_parent, root) = project();
            let path = root.join("src/router.rs");
            remove(&root, "src/router.rs", "<rbs:layers>");
            remove(&root, "src/router.rs", "</rbs:layers>");

            let source = fs::read_to_string(&path).expect("le routeur est lisible");
            fs::write(&path, source.replace(".merge(docs)", reecriture))
                .expect("le routeur est réécrivable");

            let repair = repair(&root).expect("la réparation se planifie");

            assert!(repair.reposees.is_empty(), "{:?}", repair.reposees);
            assert_eq!(repair.laissees.len(), 1, "{:?}", repair.laissees);
            assert_eq!(repair.laissees[0].anchor, "layers");
            assert!(
                repair.laissees[0].raison.contains(".merge(docs)"),
                "la raison nomme l'accroche en cause : {}",
                repair.laissees[0].raison
            );
            assert!(
                repair.plan.files().is_empty(),
                "une abstention n'écrit rien : {:?}",
                repair.plan.files()
            );
        }
    }

    /// Une ancre à demi effacée ne se répare pas : reposer le bloc entier doublerait la
    /// balise restante, et l'endroit de celle qui manque ne se déduit pas de l'autre —
    /// entre les deux, il y a tout ce que l'ancre portait.
    #[test]
    fn a_half_deleted_anchor_is_named_rather_than_doubled() {
        let (_parent, root) = project();
        remove(&root, "src/router.rs", "</rbs:routes>");

        let repair = repair(&root).expect("la réparation se planifie");

        assert!(repair.reposees.is_empty(), "{:?}", repair.reposees);
        assert_eq!(repair.laissees.len(), 1, "{:?}", repair.laissees);
        assert_eq!(repair.laissees[0].anchor, "routes");
        assert!(
            repair.laissees[0].raison.contains("</rbs:routes>"),
            "la raison nomme la balise restée : {}",
            repair.laissees[0].raison
        );
        assert!(repair.plan.files().is_empty());
    }

    /// Sur un projet sain, la réparation n'a rien à reposer et rien à écrire.
    #[test]
    fn a_healthy_project_gets_an_empty_repair() {
        let (_parent, root) = project();

        let repair = repair(&root).expect("la réparation se planifie");

        assert!(repair.reposees.is_empty());
        assert!(repair.laissees.is_empty());
        assert!(repair.plan.files().is_empty(), "{:?}", repair.plan.files());
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
