//! Cohérence entre le projet, son noyau et le CLI qui le diagnostique.
//!
//! Un projet généré par une version de rbs et manipulé par une autre n'est pas fautif en
//! soi — mais c'est la première chose à savoir quand une génération se comporte
//! autrement qu'attendu.
//!
//! Deux choses s'y jouent : d'où le projet tire `rbs-core`, et si les trois numéros
//! concordent. La première prime — des numéros alignés sur une dépendance que `cargo` ne
//! résout pas n'apprennent rien à qui est bloqué.

use std::fs;
use std::path::Path;

use super::Check;

const TITRE: &str = "versions";

/// Version du CLI en train de diagnostiquer.
const CLI: &str = env!("CARGO_PKG_VERSION");

/// Vrai depuis la publication de `rbs-core` sur crates.io : un projet qui l'y déclare
/// résout désormais, et ne se diagnostique plus en échec pour cette seule raison. Le CLI
/// ne peut pas le vérifier sans requête réseau, dans un diagnostic qui doit rester local —
/// d'où une constante, que la première publication a fait basculer.
const NOYAU_PUBLIE: bool = true;

/// Compare la version qui a généré le projet, celle de son noyau et celle du CLI.
pub(crate) fn check(root: &Path) -> Check {
    check_with(root, NOYAU_PUBLIE, CLI)
}

/// Le verdict, la publication du noyau et la version du CLI étant données en paramètres.
///
/// Les deux chemins restent ainsi exerçables par les tests de part et d'autre de la
/// bascule de `NOYAU_PUBLIE`, et de part et d'autre du numéro que porte le binaire — un
/// écart de version ne s'observe pas autrement depuis le dépôt qui produit ce numéro.
fn check_with(root: &Path, noyau_publie: bool, cli: &str) -> Check {
    let manifest = root.join("Cargo.toml");

    let metadonnees = match crate::metadata::read(&manifest) {
        Ok(metadonnees) => metadonnees,
        Err(error) => {
            return Check::failed(TITRE, error.to_string(), "restaurez le manifeste du projet");
        }
    };

    let mut ecarts = Vec::new();

    if metadonnees.version != cli {
        ecarts.push(format!(
            "projet généré par rbs {}, CLI {cli}",
            metadonnees.version
        ));
    }

    let core = match core(&manifest) {
        Ok(core) => core,
        Err(detail) => {
            return Check::failed(
                TITRE,
                detail,
                format!("déclarez rbs-core = \"{cli}\" dans [dependencies]"),
            );
        }
    };

    let core = match core {
        Core::Local => "rbs-core pris d'un chemin local".to_string(),
        Core::Version(version) if !noyau_publie => {
            return Check::failed(
                TITRE,
                format!(
                    "rbs-core {version} déclaré depuis crates.io, où rbs n'est pas encore publié"
                ),
                "clonez https://github.com/tky0065/rbs, puis dans Cargo.toml :\n\
                 rbs-core = { path = \"<clone>/crates/rbs-core\" }",
            );
        }
        Core::Version(version) if version == cli => format!("rbs-core {version}"),
        Core::Version(version) => {
            ecarts.push(format!("rbs-core {version}, CLI {cli}"));
            String::new()
        }
    };

    if ecarts.is_empty() {
        return Check::ok(TITRE, format!("projet et {core} alignés sur le CLI {cli}"));
    }

    Check::failed(TITRE, ecarts.join(" ; "), remede(&metadonnees.version, cli))
}

/// Le geste qui referme l'écart, selon le sens dans lequel il se creuse.
///
/// `rbs upgrade` ne redescend pas un projet : le nommer à qui est en avance sur son CLI
/// enverrait vers une commande qui refuse. De ce côté-là, ce n'est pas le projet qu'il
/// faut bouger, mais le binaire.
fn remede(projet: &str, cli: &str) -> String {
    if crate::upgrade::posterieure(projet, cli) {
        return "relancez la commande avec le CLI qui a généré le projet".to_string();
    }

    format!("lancez `rbs upgrade` : il aligne le manifeste du projet sur rbs {cli}")
}

/// D'où le projet tire `rbs-core`.
enum Core {
    /// Une version publiée.
    Version(String),
    /// Un chemin du disque : le mode de développement de rbs lui-même.
    Local,
}

/// Lit la dépendance `rbs-core` du manifeste.
fn core(manifest: &Path) -> Result<Core, String> {
    let source = fs::read_to_string(manifest).map_err(|error| error.to_string())?;
    let document: toml_edit::DocumentMut = source
        .parse()
        .map_err(|error: toml_edit::TomlError| error.to_string())?;

    let Some(dependency) = document
        .get("dependencies")
        .and_then(|table| table.get("rbs-core"))
    else {
        return Err("rbs-core n'est pas une dépendance du projet".to_string());
    };

    if let Some(version) = dependency.as_str() {
        return Ok(Core::Version(version.to_string()));
    }

    if dependency.get("path").is_some() {
        return Ok(Core::Local);
    }

    match dependency.get("version").and_then(|v| v.as_str()) {
        Some(version) => Ok(Core::Version(version.to_string())),
        None => Err("la dépendance rbs-core ne porte ni version ni chemin".to_string()),
    }
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

    /// La dépendance au noyau telle que `rbs new` l'écrit, pilote compris.
    ///
    /// Elle sert d'ancre aux réécritures : `version = "…"` seul figure aussi dans
    /// `[package.metadata.rbs]`, et le viser y toucherait aussi.
    fn noyau() -> String {
        format!(
            "rbs-core = {{ version = \"{CLI}\", default-features = false, features = [\"postgres\"] }}"
        )
    }

    /// Remplace un fragment du manifeste du projet.
    fn rewrite(root: &Path, before: &str, after: &str) {
        let path = root.join("Cargo.toml");
        let source = fs::read_to_string(&path).expect("le manifeste est lisible");
        assert!(source.contains(before), "« {before} » absent du manifeste");
        fs::write(&path, source.replace(before, after)).expect("le manifeste est réécrivable");
    }

    /// Bascule le noyau du projet sur un chemin local, pour isoler ce qui ne dépend pas
    /// de la publication.
    fn local_core(root: &Path) {
        rewrite(
            root,
            &noyau(),
            "rbs-core = { path = \"../../crates/rbs-core\", default-features = false, \
             features = [\"postgres\"] }",
        );
    }

    #[test]
    fn a_registry_core_is_reported_while_rbs_is_unpublished() {
        let (_parent, root) = project();

        let check = check_with(&root, false, CLI);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("crates.io"), "{}", check.detail);
        assert!(check.detail.contains(CLI), "{}", check.detail);
    }

    #[test]
    fn the_remedy_gives_the_local_path_to_declare() {
        let (_parent, root) = project();

        let check = check_with(&root, false, CLI);
        let remedy = check.remedy.expect("un échec porte son remède");

        assert!(remedy.contains("path"), "{remedy}");
        assert!(remedy.contains("crates/rbs-core"), "{remedy}");
    }

    #[test]
    fn not_being_published_outweighs_the_version_gap() {
        let (_parent, root) = project();
        rewrite(&root, &noyau(), "rbs-core = \"0.0.1\"");

        let check = check_with(&root, false, CLI);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("crates.io"), "{}", check.detail);
        assert!(check.detail.contains("0.0.1"), "{}", check.detail);
    }

    #[test]
    fn check_decides_from_the_publication_constant() {
        let (_parent, root) = project();

        assert_eq!(check(&root), check_with(&root, NOYAU_PUBLIE, CLI));
    }

    #[test]
    fn once_the_core_is_published_a_fresh_project_is_consistent() {
        let (_parent, root) = project();

        let check = check_with(&root, true, CLI);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
        assert!(check.detail.contains(CLI));
    }

    #[test]
    fn once_the_core_is_published_a_version_gap_is_still_reported() {
        let (_parent, root) = project();
        rewrite(&root, &noyau(), "rbs-core = \"0.0.1\"");

        let check = check_with(&root, true, CLI);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("rbs-core"));
        assert!(check.detail.contains("0.0.1"));
    }

    #[test]
    fn a_project_generated_by_another_version_is_reported_with_both_numbers() {
        let (_parent, root) = project();
        local_core(&root);
        rewrite(
            &root,
            &format!("version = \"{CLI}\"\nfeatures"),
            "version = \"0.0.1\"\nfeatures",
        );

        let check = check_with(&root, false, CLI);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("0.0.1"));
        assert!(check.detail.contains(CLI));
    }

    /// Une version strictement postérieure à celle du workspace, **dérivée d'elle** : un
    /// numéro figé ici finit rattrapé, et le projet neuf cesse alors d'être en retard sur
    /// un futur devenu présent — ce qui est arrivé à `"1.0.0"` le jour de sa publication.
    fn futur() -> &'static str {
        static FUTUR: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
            let [majeur, ..] = crate::upgrade::nombres(CLI).expect("la version du CLI se lit");
            format!("{}.0.0", majeur + 1)
        });

        &FUTUR
    }

    #[test]
    fn a_project_behind_the_cli_is_told_which_command_closes_the_gap() {
        let (_parent, root) = project();

        let check = check_with(&root, true, futur());
        let remedy = check.remedy.expect("un échec porte son remède");

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains(CLI), "{}", check.detail);
        assert!(check.detail.contains(futur()), "{}", check.detail);
        assert!(remedy.contains("rbs upgrade"), "{remedy}");
    }

    #[test]
    fn an_aligned_project_keeps_the_line_it_has_always_had() {
        let (_parent, root) = project();

        let check = check_with(&root, true, CLI);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
        assert_eq!(
            check.detail,
            format!("projet et rbs-core {CLI} alignés sur le CLI {CLI}")
        );
        assert_eq!(check.remedy, None, "une ligne ✓ ne porte pas de remède");
    }

    #[test]
    fn a_project_ahead_of_the_cli_is_not_sent_to_a_command_that_refuses_it() {
        let (_parent, root) = project();

        let check = check_with(&root, true, "0.0.1");
        let remedy = check.remedy.expect("un échec porte son remède");

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(!remedy.contains("upgrade"), "{remedy}");
        assert!(remedy.contains("CLI"), "{remedy}");
    }

    #[test]
    fn a_core_taken_from_a_local_path_is_stated_without_being_held_at_fault() {
        let (_parent, root) = project();
        local_core(&root);

        let check = check_with(&root, false, CLI);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
        assert!(
            check.detail.contains("chemin local"),
            "le mode développement doit rester visible : {}",
            check.detail
        );
    }

    #[test]
    fn a_manifest_without_a_core_dependency_is_reported() {
        let (_parent, root) = project();
        rewrite(&root, &format!("{}\n", noyau()), "");

        let check = check_with(&root, false, CLI);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("rbs-core"));
    }
}
