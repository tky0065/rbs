//! Mise en forme d'un plan, avant qu'il ne soit appliqué.
//!
//! Un fichier par ligne, et non une action par ligne : deux insertions de la même ligne
//! sont deux actions mais un seul changement, et c'est le fichier qui porte le statut
//! agrégé qui dit la vérité.

use super::{CauseSautee, File, Plan, Sautee, Status};
use crate::anchors::Anchor;
use crate::ui;

/// Rend le plan : la racine du projet en tête, un fichier par ligne, puis ce que le plan
/// a renoncé à écrire.
///
/// La puce et le libellé se suffisent à eux-mêmes : la couleur ne porte jamais seule une
/// information, pour que la sortie reste lisible dans un `less`, un log ou une CI.
pub(crate) fn plan(plan: &Plan) -> String {
    let entete = format!("plan pour {}", plan.root().display());
    let files = plan.files();

    let corps = if files.is_empty() {
        "  rien à faire".to_string()
    } else {
        let width = files
            .iter()
            .map(|file| file.path.chars().count())
            .max()
            .unwrap_or(0);

        let lines: Vec<String> = files.iter().map(|file| line(file, width)).collect();

        format!("{}\n\n  {}", lines.join("\n"), footer(files))
    };

    match sautees(plan) {
        Some(sautees) => format!("{entete}\n\n{corps}\n\n{sautees}"),
        None => format!("{entete}\n\n{corps}"),
    }
}

/// Les insertions que le plan a sautées, chacune avec son bloc à reporter.
///
/// Sous le tableau plutôt que dedans : le tableau dit ce qui s'écrira, et une ligne qui
/// n'écrit rien y serait comptée. Exposée pour `rbs new`, qui n'affiche pas de plan mais
/// doit dire la même chose de la feature qu'il vient de poser.
pub(crate) fn sautees(plan: &Plan) -> Option<String> {
    if plan.sautees().is_empty() {
        return None;
    }

    let blocs: Vec<String> = plan.sautees().iter().map(sautee).collect();

    Some(blocs.join("\n\n"))
}

/// Une insertion sautée : ce qui manquait, l'ancre visée, le bloc — indenté d'un cran
/// sous son annonce, indentation propre conservée, pour être collé tel quel.
fn sautee(sautee: &Sautee) -> String {
    let (fichier, balise) = (&sautee.anchor.file, sautee.anchor.opening());

    let annonce = ui::yellow(&match &sautee.cause {
        CauseSautee::FichierAbsent => {
            format!("{fichier} absent : le bloc destiné à `{balise}` est à reporter vous-même")
        }
        CauseSautee::AncreAbsente { obstacle: None } => format!(
            "{fichier} ne porte pas `{balise}` : lancez `rbs doctor --fix`, puis relancez la \
             commande — ou reportez vous-même le bloc qui lui était destiné"
        ),
        CauseSautee::AncreAbsente {
            obstacle: Some(obstacle),
        } => format!(
            "{fichier} ne porte pas `{balise}`, et `rbs doctor --fix` ne peut pas la reposer : \
             {} — {}, puis relancez la commande",
            obstacle.raison(&sautee.anchor),
            geste(&sautee.anchor)
        ),
        CauseSautee::Entrainee { par } => format!(
            "{fichier} : le bloc destiné à `{balise}` suit celui de `{}`, sauté — il nomme ce \
             que ce dernier devait poser, et ne s'écrit pas sans lui",
            par.opening()
        ),
    });
    let bloc: Vec<String> = sautee
        .lines
        .iter()
        .map(|ligne| format!("    {ligne}"))
        .collect();

    format!("  {annonce}\n{}", bloc.join("\n"))
}

/// Le geste manuel qui pose une ancre que `rbs doctor --fix` ne sait pas reposer.
///
/// `schedules` a le sien : un calendrier d'avant 1.5.0 est un `vec![]`, où une ancre ne
/// survit pas à rustfmt — et où le bloc, un `calendrier.push(…)`, n'aurait pas de variable
/// à qui s'adresser. La fonction se réécrit en instructions d'abord.
fn geste(anchor: &Anchor) -> String {
    if anchor.name == crate::anchors::SCHEDULES.name {
        "réécrivez `schedules()` en instructions et posez-y la balise à la main (voir le \
         guide scheduler)"
            .to_string()
    } else {
        format!(
            "posez `{}` et `{}` à la main",
            anchor.opening(),
            anchor.closing()
        )
    }
}

/// Ce qu'une ligne dit d'un fichier : sa puce, son chemin, ce qu'il adviendra de lui.
fn line(file: &File, width: usize) -> String {
    let path = format!("{:width$}", file.path);

    let (puce, libelle) = match (file.statut, &file.before) {
        (Status::AFaire, None) => (ui::green("+"), ui::green("créé")),
        (Status::AFaire, Some(_)) => (ui::green("~"), ui::green("modifié")),
        (Status::DejaFait, _) => (ui::dimmed("·"), ui::dimmed("inchangé")),
        (Status::Conflit, _) => (ui::red("!"), ui::red("conflit — relancer avec --force")),
    };

    format!("  {puce} {path}   {libelle}")
}

/// Le compte, par ce qui adviendra des fichiers.
///
/// Les conflits se comptent à part : sans `--force`, ils ne seront pas écrits, et les
/// ranger avec le reste ferait annoncer une écriture qui n'aura pas lieu. Créés et
/// modifiés se distinguent comme sur les lignes au-dessus, et comme dans le bilan que la
/// commande affiche une fois le plan appliqué.
fn footer(files: &[File]) -> String {
    let compter = |statut: Status| files.iter().filter(|f| f.statut == statut).count();
    let a_faire = |existant: bool| {
        files
            .iter()
            .filter(|f| f.statut == Status::AFaire && f.before.is_some() == existant)
            .count()
    };

    let (a_creer, a_modifier, inchanges, conflits) = (
        a_faire(false),
        a_faire(true),
        compter(Status::DejaFait),
        compter(Status::Conflit),
    );

    let mut segments = Vec::new();
    if a_creer > 0 {
        segments.push(format!("{a_creer} à créer"));
    }
    if a_modifier > 0 {
        segments.push(format!("{a_modifier} à modifier"));
    }
    if inchanges > 0 {
        let pluriel = if inchanges > 1 { "s" } else { "" };
        segments.push(format!("{inchanges} inchangé{pluriel}"));
    }
    if conflits > 0 {
        segments.push(format!("{conflits} en conflit"));
    }

    segments.join(", ")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::{CauseSautee, File, Sautee, Status};
    use super::*;

    fn file(path: &str, before: Option<&str>, statut: Status) -> File {
        File {
            path: path.to_string(),
            before: before.map(str::to_string),
            after: "peu importe".to_string(),
            statut,
        }
    }

    fn plan_of(files: Vec<File>) -> Plan {
        Plan {
            root: PathBuf::from("/projets/demo-api"),
            actions: Vec::new(),
            files,
            sautees: Vec::new(),
        }
    }

    /// Colonne d'un libellé, comptée en caractères : `find` rend des octets, et les puces
    /// n'en occupent pas le même nombre.
    fn column(line: &str, libelle: &str) -> usize {
        let octets = line.find(libelle).expect("le libellé est présent");
        line[..octets].chars().count()
    }

    fn line_of<'a>(rendered: &'a str, path: &str) -> &'a str {
        rendered
            .lines()
            .find(|line| line.contains(path))
            .unwrap_or_else(|| panic!("aucune ligne pour `{path}` dans :\n{rendered}"))
    }

    #[test]
    fn the_header_carries_the_project_root_only_once() {
        let rendered = plan(&plan_of(vec![file("Dockerfile", None, Status::AFaire)]));

        assert!(
            rendered.starts_with("plan pour /projets/demo-api\n"),
            "{rendered}"
        );
        assert_eq!(
            rendered.matches("/projets/demo-api").count(),
            1,
            "{rendered}"
        );
    }

    #[test]
    fn a_missing_file_is_announced_created_and_a_present_one_modified() {
        let rendered = plan(&plan_of(vec![
            file("Dockerfile", None, Status::AFaire),
            file("Cargo.toml", Some("[package]\n"), Status::AFaire),
        ]));

        assert!(
            line_of(&rendered, "Dockerfile").contains("créé"),
            "{rendered}"
        );
        assert!(
            line_of(&rendered, "Cargo.toml").contains("modifié"),
            "{rendered}"
        );
    }

    #[test]
    fn an_already_conforming_file_is_announced_unchanged() {
        let rendered = plan(&plan_of(vec![file(
            "src/router.rs",
            Some("déjà monté"),
            Status::DejaFait,
        )]));

        assert!(
            line_of(&rendered, "src/router.rs").contains("inchangé"),
            "{rendered}"
        );
    }

    #[test]
    fn a_conflict_carries_its_remedy_on_its_line() {
        let rendered = plan(&plan_of(vec![file(
            "src/main.rs",
            Some("écrit à la main"),
            Status::Conflit,
        )]));

        let line = line_of(&rendered, "src/main.rs");
        assert!(line.contains("conflit"), "{line}");
        assert!(line.contains("--force"), "{line}");
    }

    #[test]
    fn the_labels_align_on_the_longest_path() {
        let rendered = plan(&plan_of(vec![
            file("Dockerfile", None, Status::AFaire),
            file("docker-compose.yml", None, Status::AFaire),
            file("src/router.rs", Some("x"), Status::DejaFait),
        ]));

        assert_eq!(
            column(line_of(&rendered, "Dockerfile"), "créé"),
            column(line_of(&rendered, "docker-compose.yml"), "créé"),
            "{rendered}"
        );
        assert_eq!(
            column(line_of(&rendered, "Dockerfile"), "créé"),
            column(line_of(&rendered, "src/router.rs"), "inchangé"),
            "{rendered}"
        );
    }

    /// Le pied compte comme les lignes au-dessus : un total « à écrire » annonçait neuf
    /// fichiers là où le bilan en disait trois, les seuls créés.
    #[test]
    fn the_footer_separates_files_to_create_from_files_to_modify() {
        let un = plan(&plan_of(vec![file("Dockerfile", None, Status::AFaire)]));
        assert!(un.ends_with("1 à créer"), "{un}");

        let plusieurs = plan(&plan_of(vec![
            file("Dockerfile", None, Status::AFaire),
            file("Cargo.toml", Some("x"), Status::AFaire),
            file("src/router.rs", Some("x"), Status::DejaFait),
        ]));
        assert!(
            plusieurs.ends_with("1 à créer, 1 à modifier, 1 inchangé"),
            "{plusieurs}"
        );

        let tout = plan(&plan_of(vec![
            file("Dockerfile", None, Status::AFaire),
            file("src/notes/mod.rs", None, Status::AFaire),
            file("src/notes/dto.rs", None, Status::AFaire),
            file("Cargo.toml", Some("x"), Status::AFaire),
            file("src/router.rs", Some("x"), Status::AFaire),
            file("src/lib.rs", Some("x"), Status::DejaFait),
            file("src/main.rs", Some("x"), Status::Conflit),
        ]));
        assert!(
            tout.ends_with("3 à créer, 2 à modifier, 1 inchangé, 1 en conflit"),
            "{tout}"
        );
    }

    #[test]
    fn a_conflict_is_not_counted_among_the_files_to_write() {
        let rendered = plan(&plan_of(vec![
            file("Dockerfile", None, Status::AFaire),
            file("src/main.rs", Some("x"), Status::Conflit),
        ]));

        assert!(rendered.ends_with("1 à créer, 1 en conflit"), "{rendered}");
    }

    fn mailpit() -> Sautee {
        Sautee {
            anchor: crate::anchors::SERVICES,
            lines: vec![
                "mailpit:".to_string(),
                "  image: axllent/mailpit".to_string(),
            ],
            cause: CauseSautee::FichierAbsent,
        }
    }

    /// Ce que le plan ne fera pas se lit sous ce qu'il fera : le fichier qui manque,
    /// l'ancre visée, et le bloc à reporter — nommé pour que l'utilisateur sache quel
    /// service monter.
    #[test]
    fn a_skipped_insertion_names_its_file_its_anchor_and_carries_its_block() {
        let mut plan = plan_of(vec![file("src/state.rs", Some("x"), Status::AFaire)]);
        plan.sautees.push(mailpit());

        let rendered = super::plan(&plan);

        let (avant, apres) = rendered
            .split_once("1 à modifier")
            .expect("le pied du tableau est là");
        assert!(!avant.contains("docker-compose.yml"), "{rendered}");
        assert!(apres.contains("docker-compose.yml absent"), "{rendered}");
        assert!(apres.contains("# <rbs:services>"), "{rendered}");
        assert!(apres.contains("mailpit:"), "{rendered}");
        assert!(
            apres.contains("  image: axllent/mailpit"),
            "le bloc doit garder son indentation :\n{rendered}"
        );
    }

    /// Un plan qui n'écrit rien mais saute une insertion n'est pas un plan sans effet :
    /// « rien à faire » seul ferait croire que le service est en place.
    #[test]
    fn a_plan_with_nothing_but_a_skipped_insertion_still_announces_it() {
        let mut plan = plan_of(Vec::new());
        plan.sautees.push(mailpit());

        let rendered = super::plan(&plan);

        assert!(rendered.contains("rien à faire"), "{rendered}");
        assert!(rendered.contains("docker-compose.yml absent"), "{rendered}");
    }

    /// Le fichier est là, c'est l'ancre qui manque : l'annonce doit le dire, nommer la
    /// balise et la commande qui la repose — « absent » enverrait chercher un fichier qui
    /// existe.
    #[test]
    fn a_skipped_insertion_into_a_vanished_anchor_names_the_tag_and_the_fix() {
        let mut plan = plan_of(Vec::new());
        plan.sautees.push(Sautee {
            anchor: crate::anchors::JOB_MODULES,
            lines: vec!["pub mod purge;".to_string()],
            cause: CauseSautee::AncreAbsente { obstacle: None },
        });

        let rendered = super::plan(&plan);

        assert!(
            rendered.contains("src/modules/jobs/mod.rs ne porte pas `// <rbs:job_modules>`"),
            "{rendered}"
        );
        assert!(
            rendered.contains("lancez `rbs doctor --fix`, puis relancez la commande"),
            "{rendered}"
        );
        assert!(!rendered.contains("absent"), "{rendered}");
        assert!(rendered.contains("\n    pub mod purge;"), "{rendered}");
    }

    /// Une ancre que `--fix` ne peut pas reposer ne lui est pas confiée : l'annonce dit
    /// pourquoi et renvoie à la main. Une insertion entraînée nomme celle qu'elle attend.
    #[test]
    fn an_anchor_doctor_cannot_repose_and_a_dependent_block_say_so() {
        let mut plan = plan_of(Vec::new());
        plan.sautees.push(Sautee {
            anchor: crate::anchors::JOB_MODULES,
            lines: vec!["pub mod purge;".to_string()],
            cause: CauseSautee::AncreAbsente {
                obstacle: Some(crate::anchors::Cause::Introuvable),
            },
        });
        plan.sautees.push(Sautee {
            anchor: crate::anchors::JOBS,
            lines: vec!["registre = registre.register::<purge::Purge>();".to_string()],
            cause: CauseSautee::Entrainee {
                par: crate::anchors::JOB_MODULES,
            },
        });

        let rendered = super::plan(&plan);

        assert!(
            !rendered.contains("lancez `rbs doctor --fix`"),
            "{rendered}"
        );
        assert!(
            rendered.contains("la ligne d'accroche `pub mod worker;` est introuvable"),
            "{rendered}"
        );
        assert!(
            rendered.contains("posez `// <rbs:job_modules>` et `// </rbs:job_modules>` à la main"),
            "{rendered}"
        );
        assert!(
            rendered.contains("`// <rbs:jobs>` suit celui de `// <rbs:job_modules>`"),
            "{rendered}"
        );
    }

    #[test]
    fn an_empty_plan_does_not_lie() {
        let rendered = plan(&plan_of(Vec::new()));

        assert!(rendered.contains("rien à faire"), "{rendered}");
        assert!(!rendered.contains("à créer"), "{rendered}");
        assert!(!rendered.contains("à modifier"), "{rendered}");
    }

    #[test]
    fn each_state_is_distinguishable_without_colour() {
        let rendered = plan(&plan_of(vec![
            file("cree.txt", None, Status::AFaire),
            file("modifie.txt", Some("x"), Status::AFaire),
            file("inchange.txt", Some("x"), Status::DejaFait),
            file("conflit.txt", Some("x"), Status::Conflit),
        ]));

        assert!(
            !rendered.contains('\u{1b}'),
            "aucun code ANSI hors TTY :\n{rendered}"
        );

        let puces: Vec<char> = ["cree.txt", "modifie.txt", "inchange.txt", "conflit.txt"]
            .iter()
            .map(|path| {
                line_of(&rendered, path)
                    .trim_start()
                    .chars()
                    .next()
                    .expect("la ligne porte une puce")
            })
            .collect();

        let mut distinctes = puces.clone();
        distinctes.sort_unstable();
        distinctes.dedup();
        assert_eq!(distinctes.len(), puces.len(), "puces : {puces:?}");
    }
}
