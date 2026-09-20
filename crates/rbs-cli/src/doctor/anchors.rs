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

    // Une ancre optionnelle dont la ligne d'accroche manque vit dans un fichier écrit avant
    // elle — `schedules()` encore en `vec![]` : le projet est sain, et un échec ferait
    // tomber sa CI pour une ancre que seul `rbs generate job` emploie. Une ancre à demi
    // effacée ou une accroche ambiguë est, elle, une faute du projet ; elle garde l'échec,
    // comme une seule absence réparable à côté.
    let irreparables: Vec<String> = absentes
        .iter()
        .filter(|a| a.optional)
        .filter_map(|a| match obstacle(root, a) {
            Some(cause @ anchors::Cause::Introuvable) => Some((a, cause)),
            _ => None,
        })
        .map(|(a, cause)| {
            format!(
                "{} : {} — `rbs doctor --fix` ne peut donc pas la reposer ; donnez d'abord à {} \
                 la forme que le fragment pose aujourd'hui (la note de `rbs upgrade` la montre)",
                a.name,
                cause.raison(a),
                a.file
            )
        })
        .collect();

    if irreparables.len() == absentes.len() {
        return Check::warned(
            TITRE,
            detail,
            format!("{}\n\n{remedy}", irreparables.join("\n")),
        );
    }

    Check::failed(TITRE, detail, remedy)
}

/// Ce qui empêche `--fix` de reposer `anchor` dans son fichier tel qu'il est, s'il y a
/// quelque chose : le même jugement que la réparation, rendu sans rien planifier.
fn obstacle(root: &Path, anchor: &Anchor) -> Option<anchors::Cause> {
    let source = fs::read_to_string(root.join(anchor.file.as_ref())).ok()?;
    anchors::repose(&source, anchor).err()
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
    use crate::anchors::{self, ANCRES, JOB_MODULES, SCHEDULES};
    use crate::fixtures::{Project, project};

    use super::super::State;
    use super::*;

    /// Retire du projet la ligne portant `motif`.
    ///
    /// Conserve le saut de ligne final du fichier : `lines()` ne le rend pas, et le perdre
    /// ferait échouer toute comparaison à l'octet entre le fichier reposé et l'original,
    /// sans rapport avec la réparation elle-même.
    fn remove(root: &Path, file: &str, motif: &str) {
        let path = root.join(file);
        let source = fs::read_to_string(&path).expect("le fichier est lisible");
        let ampute: Vec<_> = source.lines().filter(|l| !l.contains(motif)).collect();
        let mut resultat = ampute.join("\n");
        if source.ends_with('\n') {
            resultat.push('\n');
        }
        fs::write(&path, resultat).expect("le fichier est réécrivable");
    }

    /// Un projet frais ne porte pas *toutes* les ancres du registre : `jobs` et
    /// `job_modules` vivent dans `src/modules/jobs/mod.rs`, `schedules` dans
    /// `src/modules/scheduler/mod.rs`, `modules` dans `src/modules/mod.rs` — que seul
    /// `rbs add` dépose, contrairement au compose que `new` écrit déjà —, `auth_impl`
    /// dans `src/auth/mod.rs`, que seul le fragment `auth` pose, `vite_proxy` dans
    /// `frontend/vite.config.ts`, que seul `frontend` dépose, et `admin_routes` comme
    /// `admin_rail` sous `frontend/src/admin/`, que seul `frontend-admin` dépose. Huit
    /// des dix ancres optionnelles sont donc inapplicables ici ; le compose et le fichier
    /// d'exclusions, que `new` écrit tous deux, comptent parmi les applicables.
    #[test]
    fn a_fresh_project_carries_every_anchor_that_applies_to_it() {
        let (_parent, root) = project();

        let check = check(&root);

        assert_eq!(check.state, State::Bon);
        assert!(
            check.detail.contains(&(ANCRES.len() - 8).to_string()),
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
        // Le compose retiré à la main, `jobs`, `job_modules`, `schedules`, `modules`,
        // `auth_impl`, `vite_proxy`, `admin_routes` et `admin_rail` déjà absents par
        // défaut (v. le test précédent) : neuf des dix ancres optionnelles sont
        // inapplicables, `ignore` restant portée par le `.gitignore`.
        assert!(
            check.detail.contains(&(ANCRES.len() - 9).to_string()),
            "ni le compose, ni le registre de la file, ni ses modules, ni le calendrier, \
             ni le point de montage, ni l'implémentation d'authentification, ni le relais \
             du client, ni les deux ancres de l'administration ne comptent parmi les \
             applicables : {}",
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

    /// Les trois ancres du client entrent dans le parcours dès que les fragments qui les
    /// déposent sont posés, et `--fix` les repose sous leur accroche.
    ///
    /// Le test voisin ne les voit pas : un projet frais n'a ni client ni espace
    /// d'administration, et elles y sont — à juste titre — inapplicables. Sans celui-ci,
    /// une accroche fausse ne se verrait que le jour où un développeur efface une ancre.
    #[test]
    fn the_three_client_anchors_are_walked_once_their_fragments_are_there() {
        let (_parent, root) = Project::new()
            .features(&["frontend", "auth", "frontend-admin"])
            .create();

        assert_eq!(check(&root).state, State::Bon);

        for anchor in [
            anchors::VITE_PROXY,
            anchors::ADMIN_ROUTES,
            anchors::ADMIN_RAIL,
        ] {
            let path = root.join(anchor.file.as_ref());
            let avant = fs::read_to_string(&path).expect("le fichier porteur est lisible");

            remove(&root, &anchor.file, &anchor.opening());
            remove(&root, &anchor.file, &anchor.closing());

            let manquante = check(&root);
            assert_eq!(manquante.state, State::Echec, "{manquante:?}");
            assert!(
                manquante.detail.contains(anchor.name.as_ref()),
                "`{}` n'est pas réclamée : {}",
                anchor.name,
                manquante.detail
            );

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

        assert_eq!(check(&root).state, State::Bon);
    }

    /// Chaque ancre du registre déclare une accroche, et cette accroche doit reposer le
    /// bloc à l'indentation qu'il avait : une ancre YAML remise deux colonnes à côté
    /// ferait insérer un service hors de `services:`, et le compose ne s'analyserait plus.
    #[test]
    fn every_anchor_of_the_registry_is_put_back_at_its_own_indentation() {
        let (_parent, root) = project();

        // Même filtre que celui de `inventaire()` : une ancre optionnelle dont le fichier
        // est absent n'a pas de fichier où lire une indentation, et n'est pas de celles
        // que la réparation d'un projet frais doit reposer.
        let applicables = anchors::resolved(&root)
            .into_iter()
            .filter(|anchor| !anchor.optional || root.join(anchor.file.as_ref()).exists());

        for anchor in applicables {
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

    /// Un calendrier écrit avant `<rbs:schedules>` est encore un `vec![]`, sans la ligne
    /// sous laquelle `--fix` reposerait l'ancre : le projet est sain, et `doctor` ne doit
    /// pas faire échouer sa CI. Il avertit, avec le geste.
    #[test]
    fn an_optional_anchor_that_fix_cannot_put_back_only_warns() {
        let (_parent, root) = project();
        calendrier_d_avant_l_ancre(&root);

        let check = check(&root);

        assert_eq!(check.state, State::Avertissement, "{check:?}");
        assert!(
            check
                .detail
                .contains("schedules manque dans src/modules/scheduler/mod.rs"),
            "{}",
            check.detail
        );
    }

    /// L'avertissement ne couvre que l'ancre que `--fix` ne peut pas reposer : une absence
    /// réparable à côté d'elle garde le contrôle en échec, et les deux sont nommées.
    #[test]
    fn a_repairable_absence_beside_it_keeps_the_check_failed() {
        let (_parent, root) = project();
        calendrier_d_avant_l_ancre(&root);
        remove(&root, "src/router.rs", "<rbs:routes>");

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{check:?}");
        assert!(check.detail.contains("routes manque"), "{}", check.detail);
        assert!(
            check.detail.contains("schedules manque"),
            "{}",
            check.detail
        );
    }

    /// Le calendrier tel que `scheduler` l'écrivait avant 1.5.0 : un littéral, sans ancre.
    fn calendrier_d_avant_l_ancre(root: &Path) {
        let dossier = root.join("src/modules/scheduler");
        fs::create_dir_all(&dossier).expect("le dossier est créable");
        fs::write(
            dossier.join("mod.rs"),
            "pub fn schedules() -> Vec<Schedule> {\n    vec![]\n}\n",
        )
        .expect("le calendrier est écrivable");
    }

    /// L'ancre des exclusions est parcourue comme les autres : effacée d'un fichier qui
    /// existe, elle manque, et le bloc à coller sort avec le marqueur de commentaire de
    /// Git — un `//` collé dans un `.gitignore` deviendrait un motif d'exclusion.
    #[test]
    fn the_exclusions_anchor_is_walked_like_the_others() {
        let (_parent, root) = project();
        remove(&root, ".gitignore", "<rbs:ignore>");
        remove(&root, ".gitignore", "</rbs:ignore>");

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{check:?}");
        assert!(
            check.detail.contains("ignore manque dans .gitignore"),
            "{}",
            check.detail
        );
        let remedy = check.remedy.expect("un échec porte son remède");
        assert!(remedy.contains("# <rbs:ignore>"), "{remedy}");
        assert!(remedy.contains("# </rbs:ignore>"), "{remedy}");
    }

    /// Un projet dont le développeur a supprimé le fichier d'exclusions n'est pas un projet
    /// incomplet : l'ancre est optionnelle, et son fichier absent la rend inapplicable.
    #[test]
    fn a_project_without_exclusions_is_not_reported_incomplete() {
        let (_parent, root) = project();
        fs::remove_file(root.join(".gitignore")).expect("le squelette pose un .gitignore");

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{check:?}");
    }

    /// Un projet qui installe `jobs` et `scheduler` porte les deux ancres que
    /// `rbs generate job` visera : effacée puis reposée, chacune retrouve le fichier
    /// qu'elle avait, à l'octet près.
    #[test]
    fn a_generated_job_anchor_and_its_schedule_anchor_are_put_back_at_the_byte() {
        let (_parent, root) = Project::new().features(&["jobs", "scheduler"]).create();

        for anchor in [JOB_MODULES, SCHEDULES] {
            let path = root.join(anchor.file.as_ref());
            assert!(
                path.exists(),
                "{} : le fragment doit avoir déposé {}",
                anchor.name,
                anchor.file
            );

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
                apres, avant,
                "{} : le fichier ne revient pas à l'octet",
                anchor.name
            );

            fs::write(&path, &avant).expect("le fichier se rétablit");
        }
    }
}
