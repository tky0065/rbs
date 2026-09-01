# Plan — `rbs completions` et `rbs doctor --fix`

Deux tâches d'`IMPROVE.md` : **#51** puis **#52**. Elles touchent toutes deux `cli.rs` et
`lib.rs` : elles s'exécutent dans cet ordre, jamais en parallèle.

Design validé le 2026-09-01. Un commit par tâche, Conventional Commits, sur cette branche.

---

## Tâche #51 — `rbs completions <bash|zsh|fish|powershell>`

**Décision de design.** La `Command` servant à la génération est enrichie des noms de
fragments, mais le parseur réel ne l'est pas : poser un `PossibleValuesParser` sur
`Add { feature }` ferait refuser `rbs add ma-feature --template-dir ./mes-templates`,
qu'aucun binaire ne connaît.

1. Relever la dernière version stable de `clap_complete` par
   `cargo add --dry-run clap_complete -p rbs-cli` (ne pas la deviner), l'ajouter au
   workspace comme les autres dépendances, et la rendre disponible à `rbs-cli`.
2. Test d'abord, dans un nouveau `crates/rbs-cli/src/completions.rs` : `render` écrit,
   pour chacun des quatre shells, une sortie non vide contenant `doctor` et `generate`.
   Le voir échouer à la compilation avant d'écrire `render`.
3. Écrire `render(shell, buffer)` : `Cli::command()`, puis `clap_complete::generate`.
4. Test ensuite : la sortie contient les neuf noms de fragments rendus par
   `templates::embedded_names()`. Le voir échouer.
5. Enrichir la `Command` locale à `render` par `mut_subcommand("add", …)` /
   `mut_arg("feature", …)` avec un `PossibleValuesParser` construit sur
   `embedded_names()`.
6. Test de non-régression : `Cli::try_parse_from(["rbs", "add", "un-nom-inconnu"])`
   réussit toujours — le parseur réel n'a pas été touché.
7. Brancher la variante `Commands::Completions { shell }` dans `cli.rs` et son bras dans
   `lib.rs::run`. La sortie va sur `stdout` et sur rien d'autre : c'est un flux destiné à
   un `eval`.
8. Documentation bilingue sous `docs/docs/cli/` — une page en anglais, sa jumelle
   française, dans le même commit, avec l'invocation par shell.
9. `cargo test -p rbs-cli --lib`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo fmt --all --check`. Lire les sorties avant toute affirmation.

---

## Tâche #52 — `rbs doctor --fix`

**Décision de design.** `Anchor` ne porte aucune position ; elle en gagne une, déclarée.

1. Ajouter à `Anchor` (`crates/rbs-cli/src/anchors.rs`) un champ `after: &'static str` :
   le motif de la ligne après laquelle le bloc se repose. Le renseigner pour les onze
   ancres du registre — `.merge(docs)` pour `layers`, etc. — en vérifiant chaque motif
   contre la template qui le porte, sous `templates/project/`.
2. Test d'abord, dans `doctor/` : sur un projet de fixture amputé d'une ancre, la
   réparation la repose entre les bonnes lignes, à l'indentation de la ligne d'accroche,
   et un `doctor` relancé repasse au vert. Le voir échouer.
3. Écrire la réparation : elle part de la liste des absentes que `doctor/anchors.rs`
   calcule déjà, et passe par `plan::Builder` — c'est lui qui porte la restauration en
   cas d'échec partiel, et le rituel afficher → appliquer.
4. Test : motif introuvable, ou trouvé deux fois → l'ancre n'est pas reposée, elle est
   nommée, et son bloc reste affiché comme aujourd'hui. Une réparation qui vise à côté
   coûte plus cher qu'une réparation qui s'abstient : `<rbs:layers>` posée au mauvais
   endroit ne verrait plus le `request_id`.
5. Ajouter `--fix` et `--force` à `Commands::Doctor`. Sans `--force`, `--fix` refuse
   d'écrire sur un arbre Git sale, comme `add` et `generate` : `git::garde`.
6. `--fix` avec `--json` : le rapport dit ce qui a été reposé et ce qui ne l'a pas été.
7. Test : `--fix` sur un projet sain n'écrit rien et le dit.
8. Documentation bilingue de `docs/docs/cli/doctor.md` et de sa jumelle.
9. `cargo test -p rbs-cli`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo fmt --all --check`. Les tests de `doctor/` compilent une crate `migration` :
   compter environ deux minutes.
