//! `rbs test` : les tests du projet, dans l'état où la CI engendrée les lance.
//!
//! Les étapes sont celles de `rbs dev` sans le serveur — la base montée et migrée est
//! celle que les tests joignent — puis la commande même de la CI.

use std::path::Path;

use crate::dev::{self, Skip, Step};
use crate::metadata;

/// Les arguments de `cargo`, alignés sur ceux de la CI engendrée.
///
/// `--include-ignored` : les tests qui joignent la base sont `#[ignore]` pour qu'un
/// `cargo test` nu reste rapide ; ici la base vient d'être montée, ils ont tout lieu de
/// tourner.
pub(crate) fn arguments(filtre: Option<&str>, libtest: &[String]) -> Vec<String> {
    let mut arguments: Vec<String> = ["test", "--workspace", "--no-fail-fast"]
        .map(String::from)
        .into();
    arguments.extend(filtre.map(String::from));
    arguments.extend(["--".to_string(), "--include-ignored".to_string()]);
    arguments.extend(libtest.iter().cloned());
    arguments
}

/// Le plan partagé de `rbs dev`, complété de l'étape des tests.
pub(crate) fn plan(
    root: &Path,
    skip: Skip,
    filtre: Option<&str>,
    libtest: &[String],
) -> Result<Vec<Step>, dev::Error> {
    let mut steps = dev::plan(root, skip)?;
    steps.push(Step::Tests(arguments(filtre, libtest)));
    Ok(steps)
}

/// Monte la base du projet qui contient `directory`, la migre, puis lance ses tests.
pub(crate) fn run(
    directory: &Path,
    skip: Skip,
    filtre: Option<&str>,
    libtest: &[String],
) -> Result<(), dev::Error> {
    let root = metadata::project_root(directory)?;
    let steps = plan(&root, skip, filtre, libtest)?;
    crate::ui::info(&dev::render(&steps));
    dev::start(&root, &steps, dev::patience(&steps))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[test]
    fn without_a_filter_the_arguments_are_those_of_the_generated_ci() {
        assert_eq!(
            arguments(None, &[]),
            [
                "test",
                "--workspace",
                "--no-fail-fast",
                "--",
                "--include-ignored"
            ]
        );
    }

    #[test]
    fn the_filter_goes_before_the_separator_and_libtest_arguments_after() {
        let libtest = vec!["--nocapture".to_string(), "--test-threads=1".to_string()];
        assert_eq!(
            arguments(Some("articles"), &libtest),
            [
                "test",
                "--workspace",
                "--no-fail-fast",
                "articles",
                "--",
                "--include-ignored",
                "--nocapture",
                "--test-threads=1",
            ]
        );
    }

    #[test]
    fn the_plan_runs_the_tests_after_the_migrations_and_starts_no_server() {
        let (_parent, root) = crate::fixtures::Project::new()
            .url("postgres://rbs:rbs@localhost:5432/demo_api")
            .create();
        let steps = plan(&root, Skip::default(), None, &[]).expect("le plan se calcule");
        assert!(
            !steps.iter().any(|s| matches!(s, Step::Server(_))),
            "{steps:?}"
        );
        let position = |cible: fn(&Step) -> bool| steps.iter().position(cible);
        assert!(
            position(|s| matches!(s, Step::Migrations)) < position(|s| matches!(s, Step::Tests(_)))
        );
        assert!(matches!(steps.last(), Some(Step::Tests(_))), "{steps:?}");
    }

    #[test]
    fn a_sqlite_project_runs_its_tests_without_waiting_for_a_database() {
        let (_parent, root) = crate::fixtures::Project::new()
            .database(Database::Sqlite)
            .url("sqlite://demo_api.db?mode=rwc")
            .create();
        let steps = plan(&root, Skip::default(), None, &[]).expect("le plan se calcule");
        assert!(
            !steps.iter().any(|s| matches!(s, Step::Database { .. })),
            "{steps:?}"
        );
        assert!(matches!(steps.last(), Some(Step::Tests(_))), "{steps:?}");
    }

    #[test]
    fn skipping_the_migrations_still_runs_the_tests() {
        let (_parent, root) = crate::fixtures::Project::new()
            .url("postgres://rbs:rbs@localhost:5432/demo_api")
            .create();
        let skip = Skip {
            migrations: true,
            ..Skip::default()
        };
        let steps = plan(&root, skip, None, &[]).expect("le plan se calcule");
        assert!(
            !steps.iter().any(|s| matches!(s, Step::Migrations)),
            "{steps:?}"
        );
        assert!(matches!(steps.last(), Some(Step::Tests(_))), "{steps:?}");
    }

    #[test]
    fn outside_an_rbs_project_no_test_is_run() {
        let ailleurs = tempfile::TempDir::new().expect("répertoire temporaire créable");
        let error =
            run(ailleurs.path(), Skip::default(), None, &[]).expect_err("ce n'est pas un projet");
        assert!(matches!(error, crate::dev::Error::PasUnProjet));
    }
}
