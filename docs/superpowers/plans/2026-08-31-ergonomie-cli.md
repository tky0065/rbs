# Ergonomie du CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Faire descendre `--yes` et `--template-dir` sur les seules sous-commandes qui les honorent, donner à `rbs doctor` une sortie `--json`, et lui faire annoncer la compilation de la crate `migration` avant qu'elle ne bloque.

**Architecture:** `doctor::run` cesse d'assembler un rapport pour l'afficher d'un bloc : il construit d'abord le plan des contrôles à jouer — leurs titres suffisent à fixer la largeur de la colonne — puis remet chaque constat à un puits `Sortie` au moment où il est fait. Le puits texte écrit et vide la sortie au fil de l'eau, ce qui permet à `doctor/base.rs` d'annoncer sa compilation juste avant de la lancer ; le puits du mode `--json` ne dit rien, et le document est sérialisé à la fin depuis le `Report`.

**Tech Stack:** Rust 2024, clap 4 (derive), serde / serde_json, `console` pour les couleurs, `assert_cmd` + `testcontainers` pour l'intégration.

**Spec:** `docs/superpowers/specs/2026-08-31-ergonomie-cli-design.md`

## Global Constraints

- Worktree : `/Users/yacoubakone/dev/rs-wt/ergonomie`, branche `fix/p2-ergonomie-cli`. Tous les chemins visent ce répertoire.
- MSRV `1.94` : les chaînes `let` sont disponibles, et clippy les **exige** — `collapsible_if` échoue sous `-D warnings` sur des `if` imbriqués.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont bloquants.
- Un commentaire explique le *pourquoi*, jamais le *quoi*. Un commentaire qui paraphrase la ligne suivante se supprime.
- Commits : Conventional Commits, sujet en français à l'impératif, sans majuscule initiale ni point final, **sans identifiant de tâche**, sans renvoi à un fichier de suivi, **sans ligne `Co-Authored-By` ni `Claude-Session`**. Corps : le *pourquoi* technique, puis un intertitre `Vérifications :` portant les commandes lancées et leur résultat réel.
- Le rendu texte de `rbs doctor` ne change pas d'un caractère, hors la ligne d'annonce que la tâche 3 ajoute.
- Ne rien cocher ni modifier dans `IMPROVE.md`.
- Documentation bilingue : toute page modifiée en anglais l'est aussi en français, dans le même commit.
- Vocabulaire JSON figé par la spec : clés `sain`, `checks`, `name`, `status`, `detail`, `remede` ; statuts `ok`, `avertissement`, `echec` ; `remede` omis quand il n'existe pas.

---

### Task 1: `--yes` et `--template-dir` descendent sur leurs sous-commandes

**Files:**
- Modify: `crates/rbs-cli/src/cli.rs` (struct `Cli`, variantes `New` et `Add`, module `tests`)
- Modify: `crates/rbs-cli/src/lib.rs` (`run`, branches `Commands::New` et `Commands::Add`)

**Interfaces:**
- Consumes: rien.
- Produces: `Commands::New { name, database_url, database, with, core_path, lang, template_dir, yes }` et `Commands::Add { feature, force, dry_run, template_dir }`. `Cli` ne porte plus que `command`.

- [ ] **Step 1: Write the failing tests**

Dans `crates/rbs-cli/src/cli.rs`, module `tests`, à la suite des tests existants :

```rust
    /// Le drapeau ne descend que sur les deux commandes qui le lisent. Ailleurs, clap
    /// doit le refuser : `rbs generate crud --template-dir ./mes-templates` l'acceptait
    /// et rendait le projet depuis les templates embarquées, sans un mot.
    #[test]
    fn template_dir_is_refused_by_the_commands_that_ignore_it() {
        for commande in [
            vec!["rbs", "generate", "crud", "users", "--template-dir", "/tmp/t"],
            vec!["rbs", "migrate", "up", "--template-dir", "/tmp/t"],
            vec!["rbs", "seed", "--template-dir", "/tmp/t"],
            vec!["rbs", "dev", "--template-dir", "/tmp/t"],
            vec!["rbs", "doctor", "--template-dir", "/tmp/t"],
            vec!["rbs", "upgrade", "--template-dir", "/tmp/t"],
        ] {
            assert!(
                Cli::try_parse_from(&commande).is_err(),
                "{commande:?} doit être refusée : le drapeau n'y ferait rien"
            );
        }
    }

    /// `new` rend le projet depuis ce répertoire, `add` y prend ses fragments : les deux
    /// gardent le drapeau.
    #[test]
    fn template_dir_stays_on_the_two_commands_that_honour_it() {
        let creation = Cli::try_parse_from(["rbs", "new", "blog", "--template-dir", "/tmp/t"])
            .expect("commande valide");
        let Commands::New { template_dir, .. } = creation.command else {
            panic!("`new` attendue");
        };
        assert_eq!(template_dir, Some(PathBuf::from("/tmp/t")));

        let ajout = Cli::try_parse_from(["rbs", "add", "cors", "--template-dir", "/tmp/t"])
            .expect("commande valide");
        let Commands::Add { template_dir, .. } = ajout.command else {
            panic!("`add` attendue");
        };
        assert_eq!(template_dir, Some(PathBuf::from("/tmp/t")));
    }

    /// `prompts.rs` est le seul module qui pose des questions, et `rbs new` la seule
    /// commande qui l'appelle : `--yes` n'a rien à faire ailleurs.
    #[test]
    fn yes_is_accepted_only_by_new() {
        let creation =
            Cli::try_parse_from(["rbs", "new", "blog", "--yes"]).expect("commande valide");
        let Commands::New { yes, .. } = creation.command else {
            panic!("`new` attendue");
        };
        assert!(yes);

        for commande in [
            vec!["rbs", "add", "cors", "--yes"],
            vec!["rbs", "generate", "crud", "users", "--yes"],
            vec!["rbs", "migrate", "up", "--yes"],
            vec!["rbs", "seed", "--yes"],
            vec!["rbs", "dev", "--yes"],
            vec!["rbs", "doctor", "--yes"],
            vec!["rbs", "upgrade", "--yes"],
        ] {
            assert!(
                Cli::try_parse_from(&commande).is_err(),
                "{commande:?} doit être refusée : le drapeau n'y ferait rien"
            );
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rbs-cli --lib cli::tests`
Expected: FAIL — les trois nouveaux tests échouent (les commandes acceptent encore les drapeaux, et `Commands::New` n'a pas de champ `template_dir`, donc la compilation échoue d'abord).

- [ ] **Step 3: Déplacer les deux drapeaux**

Dans `crates/rbs-cli/src/cli.rs`, la struct racine perd ses deux champs :

```rust
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
```

`Commands::New` les reçoit, après `lang` :

```rust
        /// Répertoire de templates remplaçant celles embarquées dans le binaire.
        #[arg(long, value_name = "CHEMIN")]
        template_dir: Option<PathBuf>,

        /// Prend les valeurs par défaut sans rien demander : le CLI reste scriptable.
        #[arg(long, short = 'y')]
        yes: bool,
```

`Commands::Add` reçoit le sien, après `dry_run` :

```rust
        /// Répertoire de templates remplaçant celles embarquées dans le binaire.
        #[arg(long, value_name = "CHEMIN")]
        template_dir: Option<PathBuf>,
```

Dans `crates/rbs-cli/src/lib.rs`, les deux branches se destructurent :

```rust
        Commands::New {
            name,
            database_url,
            database,
            with,
            core_path,
            lang,
            template_dir,
            yes,
        } => {
            let resultat = create_project(
                name,
                database_url,
                database,
                with,
                core_path,
                template_dir,
                yes,
                lang,
            );
```

```rust
        Commands::Add {
            feature,
            force,
            dry_run,
            template_dir,
        } => {
            if let Err(error) = add(feature, force, dry_run, template_dir) {
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rbs-cli --lib`
Expected: PASS, y compris `the_clap_declaration_is_consistent`.

- [ ] **Step 5: Vérifier l'aide sur le binaire**

Run :
```bash
cargo run -q -p rbs-cli --bin rbs -- --help
cargo run -q -p rbs-cli --bin rbs -- generate --help
cargo run -q -p rbs-cli --bin rbs -- new --help
```
Expected : `rbs --help` n'affiche plus de section d'options globales autre que `-h/-V` ; `rbs generate --help` ne nomme ni `--template-dir` ni `--yes` ; `rbs new --help` les nomme tous les deux. Coller les sorties réelles dans le corps du commit.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/src/cli.rs crates/rbs-cli/src/lib.rs
git commit
```

Sujet : `fix(cli): fait descendre --yes et --template-dir sur les commandes qui les lisent`

---

### Task 2: le rapport de diagnostic se sérialise

**Files:**
- Modify: `crates/rbs-cli/Cargo.toml` (`serde_json` passe en `[dependencies]`)
- Modify: `crates/rbs-cli/src/doctor/mod.rs` (`State`, `Check`, `pub mod json;`)
- Create: `crates/rbs-cli/src/doctor/json.rs`

**Interfaces:**
- Consumes: `doctor::{Check, Report, State}` et `Report::succeeded()`.
- Produces: `doctor::json::report(&Report) -> String`.

- [ ] **Step 1: Write the failing tests**

Créer `crates/rbs-cli/src/doctor/json.rs` avec, pour l'instant, ses seuls tests :

```rust
//! Rendu machine du rapport de diagnostic.
//!
//! Le code de sortie dit qu'il y a quelque chose ; il ne dit pas quoi. Un script qui veut
//! le savoir n'a pas à lire des glyphes colorés.

use super::Report;

#[cfg(test)]
mod tests {
    use super::super::Check;
    use super::*;

    /// Le document, analysé comme un script l'analyserait.
    fn document(checks: Vec<Check>) -> serde_json::Value {
        let rendu = report(&Report { checks });

        serde_json::from_str(&rendu)
            .unwrap_or_else(|faute| panic!("le rendu doit être un JSON valide ({faute}) : {rendu}"))
    }

    /// Les trois verdicts doivent se distinguer : un script qui ne voit que « pas ok »
    /// ne sait pas s'il doit arrêter sa chaîne.
    #[test]
    fn the_three_verdicts_carry_distinct_statuses() {
        let document = document(vec![
            Check::ok("ancres", "les 11 sont en place"),
            Check::warned("agents", "écrit hors du CLI : webhooks", "rien à faire"),
            Check::failed(".env", "RBS_ENV absente", "ajoutez RBS_ENV=development"),
        ]);

        let statuts: Vec<&str> = document["checks"]
            .as_array()
            .expect("checks est un tableau")
            .iter()
            .map(|check| check["status"].as_str().expect("un statut textuel"))
            .collect();

        assert_eq!(statuts, vec!["ok", "avertissement", "echec"]);
    }

    /// Le nom et le détail sont ce qui permet de savoir *quel* contrôle a échoué.
    #[test]
    fn each_check_names_itself_and_what_it_found() {
        let document = document(vec![Check::failed(
            "base",
            "rien ne répond sur localhost:5432",
            "lancez `docker compose up -d`",
        )]);
        let check = &document["checks"][0];

        assert_eq!(check["name"], "base");
        assert_eq!(check["detail"], "rien ne répond sur localhost:5432");
        assert_eq!(check["remede"], "lancez `docker compose up -d`");
    }

    /// Un remède absent ne se rend pas en `null` : chaque lecteur aurait à le filtrer.
    #[test]
    fn a_check_without_a_remedy_carries_no_remedy_field() {
        let document = document(vec![Check::ok("ancres", "les 11 sont en place")]);

        assert!(
            document["checks"][0].get("remede").is_none(),
            "{}",
            document["checks"][0]
        );
    }

    /// `sain` vaut exactement ce que vaut le code de sortie : un avertissement n'y fait
    /// pas obstacle, un échec si.
    #[test]
    fn the_summary_follows_the_exit_status() {
        let avertissement = document(vec![Check::warned(
            "agents",
            "1 module hors CLI",
            "rien à faire",
        )]);
        assert_eq!(avertissement["sain"], true);

        let echec = document(vec![Check::failed(
            ".env",
            "RBS_ENV absente",
            "ajoutez RBS_ENV=development",
        )]);
        assert_eq!(echec["sain"], false);
    }
}
```

Déclarer le module dans `crates/rbs-cli/src/doctor/mod.rs`, à sa place alphabétique entre `guards` et `jobs` :

```rust
pub mod json;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rbs-cli --lib doctor::json`
Expected: FAIL à la compilation — `report` n'existe pas, et `serde_json` n'est pas une dépendance de la bibliothèque.

- [ ] **Step 3: Sortir `serde_json` des dev-dependencies**

Dans `crates/rbs-cli/Cargo.toml`, ajouter `serde_json.workspace = true` à `[dependencies]` (ordre alphabétique, après `serde.workspace = true`) et retirer la ligne homonyme de `[dev-dependencies]` : une dépendance déclarée aux deux endroits n'ajoute rien.

- [ ] **Step 4: Rendre `State` et `Check` sérialisables**

Dans `crates/rbs-cli/src/doctor/mod.rs`, ajouter `use serde::Serialize;` aux imports, puis :

```rust
/// Verdict d'un contrôle.
///
/// Les noms rendus en JSON sont ceux du dépôt, en ASCII : `ok` est déjà celui du
/// constructeur `Check::ok`, et les deux autres ceux des variantes ci-dessous. Un
/// troisième vocabulaire serait un de plus à tenir à jour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum State {
    /// Rien à signaler.
    #[serde(rename = "ok")]
    Bon,
    /// Ce qui mérite d'être su sans empêcher le projet de fonctionner.
    Avertissement,
    /// Ce qui empêche le projet de fonctionner.
    Echec,
}

/// Ce qu'un contrôle a constaté.
///
/// Les noms des champs en JSON suivent la seule autre sortie structurée du dépôt, le
/// corps de `GET /health` de `rbs-core` : `status` y désigne déjà un verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Check {
    /// Ce qui est vérifié, en un mot : `anchors`, `.env`, `versions`, `base`.
    #[serde(rename = "name")]
    pub title: &'static str,
    /// Verdict.
    #[serde(rename = "status")]
    pub state: State,
    /// Ce qui a été constaté, en une ligne.
    pub detail: String,
    /// Quoi faire, quand il y a quelque chose à faire.
    #[serde(rename = "remede", skip_serializing_if = "Option::is_none")]
    pub remedy: Option<String>,
}
```

- [ ] **Step 5: Écrire le rendu**

Dans `crates/rbs-cli/src/doctor/json.rs`, au-dessus du module `tests` :

```rust
use serde::Serialize;

use super::{Check, Report};

/// Le rapport tel qu'un script le lit.
#[derive(Serialize)]
struct Document<'a> {
    /// Le verdict d'ensemble, celui-là même que porte le code de sortie.
    ///
    /// Sans lui, un lecteur devrait le recalculer sur le tableau, en sachant qu'un
    /// avertissement n'y fait pas obstacle — règle qu'aucun champ du document n'énonce.
    sain: bool,
    /// Les constats, dans l'ordre où ils ont été faits.
    checks: &'a [Check],
}

/// Rend le rapport en JSON, seul document de la sortie standard.
pub(crate) fn report(report: &Report) -> String {
    let document = Document {
        sain: report.succeeded(),
        checks: &report.checks,
    };

    // Ni carte à clés non textuelles ni flottant : la sérialisation ne peut échouer que
    // sur un défaut de programmation, qu'il vaut mieux voir tomber ici.
    serde_json::to_string_pretty(&document).expect("le rapport se sérialise")
}
```

Remplacer l'import `use super::Report;` posé à l'étape 1 par les deux lignes ci-dessus.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p rbs-cli --lib doctor::`
Expected: PASS — quatre tests dans `doctor::json::tests`, et les tests existants de `doctor` inchangés.

- [ ] **Step 7: Commit**

```bash
git add crates/rbs-cli/Cargo.toml crates/rbs-cli/src/doctor/mod.rs crates/rbs-cli/src/doctor/json.rs
git commit
```

Sujet : `feat(doctor): sérialise le rapport de diagnostic en JSON`

---

### Task 3: le rendu texte s'écrit au fil des contrôles

**Files:**
- Modify: `crates/rbs-cli/src/doctor/mod.rs` (trait `Sortie`)
- Modify: `crates/rbs-cli/src/doctor/render.rs` (réécriture complète)
- Modify: `crates/rbs-cli/src/doctor/json.rs` (puits `Muette`)

**Interfaces:**
- Consumes: `doctor::{Check, Report, State}`.
- Produces:
  - `pub(crate) trait Sortie { fn debut(&mut self, titres: &[&'static str]); fn annonce(&mut self, titre: &'static str, raison: &str); fn constat(&mut self, check: &Check); }`
  - `pub(crate) struct render::Texte<W: std::io::Write>` avec `Texte::new(sortie: W) -> Self`, qui implémente `Sortie`.
  - `pub(crate) struct json::Muette`, qui implémente `Sortie` sans rien écrire.
- `render::report(&Report) -> String` **disparaît** : `Texte` est le seul rendu texte.

- [ ] **Step 1: Déclarer le trait et le puits muet**

Dans `crates/rbs-cli/src/doctor/mod.rs`, après la définition de `Report` :

```rust
/// Ce qui reçoit le rapport au fil des contrôles.
///
/// Le diagnostic ne s'assemble plus avant de s'afficher : un contrôle qui va bloquer une
/// minute doit pouvoir le dire pendant que les précédents sont déjà à l'écran.
pub(crate) trait Sortie {
    /// Les titres de tous les contrôles prévus, avant que le premier ne s'exécute.
    ///
    /// La colonne des détails s'aligne sur le plus long d'entre eux, largeur qu'un rendu
    /// au fil de l'eau ne peut plus découvrir après coup.
    fn debut(&mut self, titres: &[&'static str]);

    /// Ce qu'un contrôle s'apprête à faire, quand cela va prendre du temps.
    fn annonce(&mut self, titre: &'static str, raison: &str);

    /// Le constat qui vient d'être fait.
    fn constat(&mut self, check: &Check);
}
```

Dans `crates/rbs-cli/src/doctor/json.rs`, sous `report` :

```rust
/// Le puits du mode `--json`, qui ne dit rien pendant le diagnostic.
///
/// La sortie standard ne doit porter que le document : une ligne de rapport ou une
/// annonce d'attente y ferait échouer le premier `jq` venu.
pub(crate) struct Muette;

impl super::Sortie for Muette {
    fn debut(&mut self, _titres: &[&'static str]) {}

    fn annonce(&mut self, _titre: &'static str, _raison: &str) {}

    fn constat(&mut self, _check: &Check) {}
}
```

- [ ] **Step 2: Write the failing tests**

Réécrire le module `tests` de `crates/rbs-cli/src/doctor/render.rs` : le rendu se lit désormais dans ce que le puits a écrit.

```rust
#[cfg(test)]
mod tests {
    use super::super::Check;
    use super::*;

    /// Le rendu texte d'un rapport, écrit par le puits contrôle par contrôle.
    fn rendu(checks: Vec<Check>) -> String {
        let mut octets = Vec::new();
        let titres: Vec<&'static str> = checks.iter().map(|check| check.title).collect();

        {
            let mut texte = Texte::new(&mut octets);
            texte.debut(&titres);
            for check in &checks {
                texte.constat(check);
            }
        }

        String::from_utf8(octets).expect("le rendu est de l'UTF-8")
    }

    #[test]
    fn the_two_verdicts_carry_distinct_markers_without_colour() {
        let rendered = rendu(vec![
            Check::ok("ancres", "les 5 sont en place"),
            Check::failed(".env", "RBS_ENV manque", "ajoutez RBS_ENV=development"),
        ]);
        let mut lines = rendered.lines();

        let ok = lines.next().expect("le premier contrôle est rendu");
        let failed = lines.next().expect("le second contrôle est rendu");

        assert!(ok.contains('✓') && ok.contains("ancres"));
        assert!(failed.contains('✗') && failed.contains(".env"));
        assert!(!ok.contains('✗'));
    }

    /// Le rapport se lit aussi sans couleur — journaux de CI, terminaux monochromes.
    #[test]
    fn the_three_verdicts_carry_distinct_markers_without_colour() {
        let rendered = rendu(vec![
            Check::ok("ancres", "les 10 sont en place"),
            Check::warned("cli", "1 module hors CLI", "rbs generate, ou rien"),
            Check::failed(".env", "RBS_ENV manque", "ajoutez RBS_ENV=development"),
        ]);
        let mut lines = rendered.lines();

        let ok = lines.next().expect("le premier contrôle est rendu");
        let warned = lines.next().expect("le deuxième contrôle est rendu");

        assert!(ok.contains('✓'));
        assert!(warned.contains('!') && warned.contains("cli"));
        assert!(!warned.contains('✓') && !warned.contains('✗'));
    }

    #[test]
    fn the_remedy_follows_its_finding_indented() {
        let rendered = rendu(vec![Check::failed(
            ".env",
            "RBS_ENV manque",
            "ajoutez RBS_ENV=development",
        )]);

        let remedy = rendered
            .lines()
            .find(|line| line.contains("ajoutez RBS_ENV=development"))
            .expect("le remède est rendu");

        assert!(
            remedy.starts_with("      "),
            "le remède est en retrait du constat : « {remedy} »"
        );
    }

    #[test]
    fn a_multi_line_remedy_is_indented_throughout() {
        let rendered = rendu(vec![Check::failed(
            "ancres",
            "routes manque",
            "dans src/router.rs :\n// <rbs:routes>\n// </rbs:routes>",
        )]);

        for line in rendered.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            assert!(
                line.starts_with("      "),
                "chaque ligne du remède est en retrait : « {line} »"
            );
        }
    }

    #[test]
    fn a_spotless_check_adds_no_line() {
        let rendered = rendu(vec![Check::ok("ancres", "les 5 sont là")]);

        assert_eq!(rendered.lines().count(), 1);
    }

    #[test]
    fn the_details_align_on_the_longest_title() {
        let rendered = rendu(vec![
            Check::ok("base", "PostgreSQL 18.1"),
            Check::ok("versions", "alignées"),
        ]);

        let mut lines = rendered.lines();
        let premiere = lines.next().expect("première ligne");
        let seconde = lines.next().expect("seconde ligne");

        assert_eq!(
            colonne(premiere, "PostgreSQL 18.1"),
            colonne(seconde, "alignées")
        );
    }

    /// La colonne, en caractères, où `detail` commence dans `line`.
    fn colonne(line: &str, detail: &str) -> usize {
        let octets = line.find(detail).expect("le détail est présent");

        line[..octets].chars().count()
    }

    /// L'annonce n'a de sens que devant le constat du contrôle qu'elle annonce : c'est
    /// tout son objet, et le rendu en bloc l'interdisait.
    #[test]
    fn the_announcement_precedes_the_finding_of_its_own_check() {
        let mut octets = Vec::new();

        {
            let mut texte = Texte::new(&mut octets);
            texte.debut(&["ancres", "base"]);
            texte.constat(&Check::ok("ancres", "les 11 sont en place"));
            texte.annonce("base", "compilation de la crate migration");
            texte.constat(&Check::ok("base", "postgres 18.6 répond"));
        }

        let rendered = String::from_utf8(octets).expect("le rendu est de l'UTF-8");
        let lignes: Vec<&str> = rendered.lines().collect();
        let annonce = lignes
            .iter()
            .position(|line| line.contains("compilation de la crate migration"))
            .expect("l'annonce est rendue");
        let constat = lignes
            .iter()
            .position(|line| line.contains("postgres 18.6 répond"))
            .expect("le constat est rendu");

        assert!(annonce < constat, "{rendered}");
        assert!(lignes[annonce].contains("base"), "{rendered}");
    }

    /// La suite d'une annonce se lit sous son propre début, et non sous le marqueur.
    #[test]
    fn a_multi_line_announcement_aligns_on_the_detail_column() {
        let mut octets = Vec::new();

        {
            let mut texte = Texte::new(&mut octets);
            texte.debut(&["ancres", "base"]);
            texte.annonce("base", "compilation de la crate migration,\nune minute…");
        }

        let rendered = String::from_utf8(octets).expect("le rendu est de l'UTF-8");
        let mut lignes = rendered.lines();
        let premiere = lignes.next().expect("la première ligne de l'annonce");
        let suite = lignes.next().expect("la suite de l'annonce");

        assert_eq!(
            colonne(premiere, "compilation de la crate migration,"),
            colonne(suite, "une minute…")
        );
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p rbs-cli --lib doctor::render`
Expected: FAIL à la compilation — `Texte` n'existe pas.

- [ ] **Step 4: Réécrire le rendu**

Remplacer tout ce qui précède le module `tests` dans `crates/rbs-cli/src/doctor/render.rs` :

```rust
//! Mise en forme du rapport de diagnostic, écrite au fil des contrôles.
//!
//! Un remède se lit sous le constat qui l'appelle, indenté : un diagnostic qui renvoie
//! ses remèdes en bas de page oblige à faire l'aller-retour.
//!
//! Le rapport n'est pas assemblé puis affiché d'un bloc : il s'écrit constat par constat,
//! ce qui est la condition pour qu'un contrôle sur le point de bloquer une minute puisse
//! le dire avant, et non après.

use std::io::Write;

use crate::ui;

use super::{Check, Sortie, State};

/// Retrait des remèdes, sous le constat qui les appelle.
const RETRAIT: &str = "      ";

/// Ce qui sépare le marqueur, le titre et le détail : `  ✓ `, puis `   `.
const ENTOUR: usize = 7;

/// Rendu texte, écrit dans `sortie` à mesure que les contrôles rendent leur verdict.
pub(crate) struct Texte<W: Write> {
    sortie: W,
    /// Largeur de la colonne des titres, fixée par [`Sortie::debut`].
    width: usize,
}

impl<W: Write> Texte<W> {
    /// Un rendu qui écrit dans `sortie`.
    pub(crate) fn new(sortie: W) -> Self {
        Self { sortie, width: 0 }
    }

    /// Écrit une ligne, l'échec d'écriture laissé de côté.
    ///
    /// Une sortie fermée — `rbs doctor | head -3` — n'est pas une faute du projet
    /// diagnostiqué, et s'interrompre pour le dire perdrait les contrôles restants.
    fn ligne(&mut self, ligne: &str) {
        let _ = writeln!(self.sortie, "{ligne}");
    }

    /// Le début d'une ligne de constat : marqueur, titre, et l'espace jusqu'au détail.
    fn tete(&self, marqueur: &str, titre: &str) -> String {
        let width = self.width;

        format!("  {marqueur} {titre:width$}   ")
    }
}

impl<W: Write> Sortie for Texte<W> {
    fn debut(&mut self, titres: &[&'static str]) {
        self.width = titres
            .iter()
            .map(|titre| titre.chars().count())
            .max()
            .unwrap_or(0);
    }

    fn annonce(&mut self, titre: &'static str, raison: &str) {
        let mut raisons = raison.lines();
        let premiere = raisons.next().unwrap_or_default();
        let tete = self.tete(&ui::dimmed("…"), titre);
        self.ligne(&format!("{tete}{}", ui::dimmed(premiere)));

        let colonne = " ".repeat(self.width + ENTOUR);
        for suite in raisons {
            self.ligne(&format!("{colonne}{}", ui::dimmed(suite)));
        }

        // Une annonce qui arriverait après le travail qu'elle annonce n'annoncerait
        // rien : elle ne doit pas attendre le tampon d'un appelant.
        let _ = self.sortie.flush();
    }

    fn constat(&mut self, check: &Check) {
        let marqueur = match check.state {
            State::Bon => ui::green("✓"),
            State::Avertissement => ui::yellow("!"),
            State::Echec => ui::red("✗"),
        };
        let tete = self.tete(&marqueur, check.title);
        self.ligne(&format!("{tete}{}", check.detail));

        let Some(remedy) = &check.remedy else {
            return;
        };

        for ligne in remedy.lines().map(|line| format!("{RETRAIT}{}", ui::dimmed(line))) {
            self.ligne(&ligne);
        }
    }
}
```

Note : `tete` produit exactement le préfixe d'aujourd'hui — deux espaces, le marqueur,
une espace, le titre complété à `width`, trois espaces —, et `ENTOUR` en est le compte
visible hors titre. Le rendu ne change donc pas d'un caractère.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p rbs-cli --lib doctor::`
Expected: PASS. `doctor::mod` ne compile pas encore si `run` appelle toujours les anciens contrôles — c'est la tâche 4 ; si la compilation casse ici, s'arrêter à la tâche 4 avant de commiter.

- [ ] **Step 6: Commit**

Committer tâches 3 et 4 ensemble (voir tâche 4, étape 6) : le trait n'a pas d'appelant tant que `run` ne l'alimente pas.

---

### Task 4: `doctor::run` planifie ses contrôles puis les joue

**Files:**
- Modify: `crates/rbs-cli/src/doctor/mod.rs` (`run`, `Controle`, `plan`, `FEATURE_CHECKS`, tests)
- Modify: `crates/rbs-cli/src/doctor/base.rs` (`TITRE` public, `check` reçoit l'annonce, constante `ANNONCE`, tests)
- Modify: les onze autres `crates/rbs-cli/src/doctor/*.rs` : `const TITRE` devient `pub(crate) const TITRE`
- Modify: `crates/rbs-cli/src/lib.rs` (`diagnose`)
- Modify: `crates/rbs-cli/src/cli.rs` (`Commands::Doctor { json }`)

**Interfaces:**
- Consumes: `Sortie`, `render::Texte`, `json::{Muette, report}` de la tâche 3, `Commands::Doctor` de la tâche 1.
- Produces:
  - `doctor::run(directory: &Path, sortie: &mut dyn Sortie) -> Result<Report, Error>`
  - `doctor::base::check(root: &Path, annonce: &mut dyn FnMut(&str)) -> Check`
  - `pub(crate) const TITRE: &str` dans chacun des douze modules de contrôle
  - `Commands::Doctor { json: bool }`

- [ ] **Step 1: Write the failing test**

Dans `crates/rbs-cli/src/doctor/mod.rs`, module `tests`, ajouter un puits d'essai et le test qui prouve que les constats arrivent au fil de l'eau :

```rust
    /// Un puits qui note ce qu'il reçoit, dans l'ordre où il le reçoit.
    struct Journal {
        titres: Vec<&'static str>,
        constats: Vec<&'static str>,
    }

    impl Sortie for Journal {
        fn debut(&mut self, titres: &[&'static str]) {
            self.titres = titres.to_vec();
        }

        fn annonce(&mut self, _titre: &'static str, _raison: &str) {}

        fn constat(&mut self, check: &Check) {
            self.constats.push(check.title);
        }
    }

    /// Les titres sont connus avant le premier verdict — c'est ce qui fixe la largeur de
    /// la colonne sans attendre le dernier — et chaque constat est remis au fil de l'eau.
    #[test]
    fn the_sink_learns_every_title_before_the_first_finding() {
        let (_parent, root) = project(&["health", "jobs"]);
        let mut journal = Journal {
            titres: Vec::new(),
            constats: Vec::new(),
        };

        let report = run_with(&root, &mut journal).expect("c'est un projet rbs");

        assert_eq!(journal.titres, titles(&report));
        assert_eq!(journal.constats, titles(&report));
    }
```

Adapter les tests existants du module : `run(&root)` devient `run_with(&root, &mut Muet)` où `Muet` est un puits de test sans état. Pour ne pas répéter, ajouter au module `tests` :

```rust
    /// Le diagnostic, sans rien afficher : ces tests jugent le rapport, pas son rendu.
    fn run_with(root: &std::path::Path, sortie: &mut dyn Sortie) -> Result<Report, Error> {
        super::run(root, sortie)
    }

    /// Un puits qui laisse tomber ce qu'il reçoit.
    struct Muet;

    impl Sortie for Muet {
        fn debut(&mut self, _titres: &[&'static str]) {}

        fn annonce(&mut self, _titre: &'static str, _raison: &str) {}

        fn constat(&mut self, _check: &Check) {}
    }
```

et remplacer chaque `run(&root)` des tests existants par `run_with(&root, &mut Muet)`, `run(ailleurs.path())` par `run_with(ailleurs.path(), &mut Muet)`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rbs-cli --lib doctor::tests`
Expected: FAIL à la compilation — `run` ne prend qu'un argument.

- [ ] **Step 3: Rendre les titres publics et annoncer dans `base`**

Dans chacun des douze modules de contrôle, `const TITRE: &str = …;` devient
`pub(crate) const TITRE: &str = …;` — et `relations.rs`, seul à l'avoir nommé `TITLE`,
prend le nom des onze autres (renommer la constante et ses trois usages dans le fichier).

Ajouter la documentation de la constante là où elle manque, sur le modèle :

```rust
/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "base";
```

Dans `crates/rbs-cli/src/doctor/base.rs` :

```rust
/// Ce que le contrôle s'apprête à faire, dit avant de le faire.
///
/// La version se demande au binaire de la crate `migration`, que cargo bâtit au premier
/// appel : un diagnostic présumé instantané bloquait alors le temps d'une compilation,
/// sans que rien ne l'annonce.
const ANNONCE: &str =
    "compilation de la crate migration, peut prendre\nune minute au premier lancement…";
```

La signature devient :

```rust
pub(crate) fn check(root: &Path, annonce: &mut dyn FnMut(&str)) -> Check {
```

et l'annonce est émise juste avant la seule ligne qui lance cargo, c'est-à-dire
immédiatement avant `match version(root, &variables)` :

```rust
    // Les cinq sorties ci-dessus n'ont rien compilé : annoncer plus haut promettrait une
    // attente qui n'a pas lieu sur un projet dont la base est arrêtée.
    annonce(ANNONCE);

    match version(root, &variables) {
```

Dans le module `tests` de `base.rs`, tout appel à `check(&root)` devient
`check(&root, &mut |_: &str| {})`.

- [ ] **Step 4: Planifier puis jouer les contrôles**

Dans `crates/rbs-cli/src/doctor/mod.rs`, remplacer `run`, `FeatureCheck` et
`FEATURE_CHECKS` par :

```rust
/// Diagnostique le projet qui contient `directory`, en remettant chaque constat à
/// `sortie` au moment où il est fait.
pub(crate) fn run(directory: &Path, sortie: &mut dyn Sortie) -> Result<Report, Error> {
    let root = metadata::project_root(directory)?;

    let controles = plan(&root);
    let titres: Vec<&'static str> = controles.iter().map(|controle| controle.titre).collect();
    sortie.debut(&titres);

    // Une seule lecture pour toute la boucle : la configuration était relue et
    // réanalysée par chaque contrôle pour une question d'une ligne.
    let config = Config::read(&root);
    let mut checks = Vec::with_capacity(controles.len());

    for controle in controles {
        let check = {
            let mut annonce = |raison: &str| sortie.annonce(controle.titre, raison);
            (controle.executer)(&root, &config, &mut annonce)
        };

        sortie.constat(&check);
        checks.push(check);
    }

    Ok(Report { checks })
}

/// Un contrôle du diagnostic : son titre, connu avant qu'il ne s'exécute, et son
/// exécution.
///
/// Un contrôle qui n'interroge ni la configuration ni l'annonce les ignore par une
/// fermeture : lui imposer un paramètre qu'il n'emploie pas se lirait comme une
/// dépendance qu'il n'a pas.
#[derive(Clone, Copy)]
struct Controle {
    /// Ce qui est vérifié, tel qu'il paraîtra au rapport.
    titre: &'static str,
    /// Le contrôle lui-même.
    executer: fn(&Path, &Config, &mut dyn FnMut(&str)) -> Check,
}

/// Les contrôles à jouer sur ce projet, dans l'ordre du rapport.
///
/// Le plan se construit avant le premier verdict : c'est ce qui permet à un rendu écrit
/// au fil de l'eau de connaître la largeur de sa colonne de titres.
fn plan(root: &Path) -> Vec<Controle> {
    let mut controles = vec![
        Controle {
            titre: anchors::TITRE,
            executer: |root, _, _| anchors::check(root),
        },
        Controle {
            titre: agents::TITRE,
            executer: |root, _, _| agents::check(root),
        },
        Controle {
            titre: relations::TITRE,
            executer: |root, _, _| relations::check(root),
        },
        Controle {
            titre: env::TITRE,
            executer: |root, _, _| env::check(root),
        },
        Controle {
            titre: versions::TITRE,
            executer: |root, _, _| versions::check(root),
        },
        Controle {
            titre: base::TITRE,
            executer: |root, _, annonce| base::check(root, annonce),
        },
    ];

    let installees = metadata::read(&root.join("Cargo.toml"))
        .map(|metadonnees| metadonnees.features)
        .unwrap_or_default();

    // Un projet qui n'a pas installé une feature n'a pas à lire une ligne à son sujet :
    // le rapport ne porte que des contrôles dont le verdict le concerne.
    for (feature, controle) in FEATURE_CHECKS {
        if installees.iter().any(|installee| installee == feature) {
            controles.push(controle);
        }
    }

    controles
}

/// Le contrôle propre à chaque feature, sous le nom qu'elle porte dans le manifeste.
///
/// `redis` s'installe en `src/cache/` sous une section `[cache]` : c'est le nom de la
/// crate d'un côté, celui du service rendu de l'autre. Le tableau porte le nom déclaré,
/// seul commun aux quatre.
///
/// Une feature peut y figurer deux fois : `auth` amène de quoi vérifier son secret, et de
/// quoi juger les routes que les rôles qu'elle installe pourraient protéger.
const FEATURE_CHECKS: [(&str, Controle); 6] = [
    (
        "auth",
        Controle {
            titre: auth::TITRE,
            executer: |root, config, _| auth::check(root, config),
        },
    ),
    (
        "auth",
        Controle {
            titre: guards::TITRE,
            executer: |root, _, _| guards::check(root),
        },
    ),
    (
        "redis",
        Controle {
            titre: redis::TITRE,
            executer: |_, config, _| redis::check(config),
        },
    ),
    (
        "mail",
        Controle {
            titre: mail::TITRE,
            executer: |root, config, _| mail::check(root, config),
        },
    ),
    (
        "storage",
        Controle {
            titre: storage::TITRE,
            executer: |root, config, _| storage::check(root, config),
        },
    ),
    (
        "jobs",
        Controle {
            titre: jobs::TITRE,
            executer: |_, config, _| jobs::check(config),
        },
    ),
];
```

- [ ] **Step 5: Câbler la commande**

Dans `crates/rbs-cli/src/cli.rs` :

```rust
    /// Diagnostique le projet : ancres, .env, base joignable, versions.
    Doctor {
        /// Rend le rapport en JSON sur la sortie standard, pour un script ou une CI.
        #[arg(long)]
        json: bool,
    },
```

Dans `crates/rbs-cli/src/lib.rs` :

```rust
        Commands::Doctor { json } => match diagnose(json) {
```

et :

```rust
/// Rend le rapport et dit si le projet est sain.
fn diagnose(json: bool) -> Result<bool, Box<dyn Error>> {
    let directory = std::env::current_dir()?;

    // En JSON, la sortie standard ne porte que le document : ni les deux lignes de
    // conclusion, que `sain` remplace, ni l'annonce d'attente du contrôle `base`.
    if json {
        let report = doctor::run(&directory, &mut doctor::json::Muette)?;
        println!("{}", doctor::json::report(&report));

        return Ok(report.succeeded());
    }

    let report = doctor::run(&directory, &mut doctor::render::Texte::new(std::io::stdout()))?;

    if report.succeeded() {
        ui::success("le projet est sain");
    } else {
        ui::warn("le projet demande votre attention");
    }

    Ok(report.succeeded())
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p rbs-cli --lib`
Expected: PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: aucun avertissement.

Run: `cargo fmt --all --check`
Expected: aucun écart.

- [ ] **Step 7: Vérifier sur le binaire**

```bash
cd /private/tmp && rm -rf rbs-ergonomie && mkdir rbs-ergonomie && cd rbs-ergonomie
cargo run -q --manifest-path /Users/yacoubakone/dev/rs-wt/ergonomie/Cargo.toml -p rbs-cli --bin rbs -- \
  new demo-api --core-path /Users/yacoubakone/dev/rs-wt/ergonomie/crates/rbs-core --yes
cd demo-api
cargo run -q --manifest-path /Users/yacoubakone/dev/rs-wt/ergonomie/Cargo.toml -p rbs-cli --bin rbs -- doctor --json | jq .
echo "code de sortie du jq : $?"
```
Expected : `jq` sort en 0 et rend le document ; la base étant absente, `sain` vaut `false`
et le contrôle `base` porte `"status": "echec"`.

- [ ] **Step 8: Commit**

```bash
git add crates/rbs-cli/src
git commit
```

Sujet : `feat(doctor): rend le diagnostic au fil des contrôles et l'offre en JSON`

Corps : le rendu en bloc interdisait toute annonce pendant le travail ; le puits la rend
possible et sert aussi le mode `--json`, où la sortie standard ne porte que le document.

---

### Task 5: les tests d'intégration prouvent les deux sorties

**Files:**
- Modify: `crates/rbs-cli/tests/integration_doctor.rs`

**Interfaces:**
- Consumes: le binaire `rbs` livré, `common::{projet, start_postgres, url_of}`.
- Produces: rien pour les autres tâches.

- [ ] **Step 1: Write the failing tests**

Ajouter à `crates/rbs-cli/tests/integration_doctor.rs` :

```rust
/// Un script ne peut pas lire des glyphes colorés : `--json` doit rendre un document,
/// seul et valide sur la sortie standard, et nommer le contrôle qui a échoué.
#[test]
fn the_json_report_is_the_only_thing_on_stdout() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    viser(&projet, INJOIGNABLE);

    let sortie = rbs(&projet)
        .args(["doctor", "--json"])
        .assert()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&sortie.stdout).into_owned();

    let document: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|faute| panic!("stdout doit être un JSON valide ({faute}) :\n{stdout}"));

    assert_eq!(document["sain"], false, "{stdout}");
    let base = document["checks"]
        .as_array()
        .expect("checks est un tableau")
        .iter()
        .find(|check| check["name"] == "base")
        .expect("le contrôle base figure au rapport");
    assert_eq!(base["status"], "echec", "{stdout}");
    assert!(base["remede"].is_string(), "{stdout}");

    // Ni glyphe du rendu texte, ni séquence ANSI : ce sont elles qui feraient échouer
    // l'analyse d'un script.
    for parasite in ['✓', '✗', '!', '…', '\u{1b}'] {
        assert!(
            !stdout.contains(parasite),
            "`{parasite}` sur la sortie standard :\n{stdout}"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rbs-cli --test integration_doctor the_json_report_is_the_only_thing_on_stdout`
Expected : PASS si les tâches 2 à 4 sont faites — le lancer d'abord sur `git stash` n'a pas
de sens ici ; ce test garde l'acquis. S'il échoue, lire la sortie et corriger.

- [ ] **Step 3: Écrire le test de l'annonce**

L'annonce n'est émise que si la base répond : sur une base injoignable, le contrôle sort
avant de compiler quoi que ce soit. Le test demande donc un vrai PostgreSQL.

```rust
/// L'annonce n'a de valeur que si elle atteint le terminal *avant* la compilation
/// qu'elle annonce : sa ligne doit précéder le constat du même contrôle.
#[test]
#[ignore = "démarre PostgreSQL et compile la crate migration d'un projet complet : plusieurs minutes"]
fn the_slow_check_announces_itself_before_the_finding() {
    let postgres = common::start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());
    viser(&projet, &common::url_of(&postgres));

    let rendu = diagnostic(&projet);
    let lignes: Vec<&str> = rendu.lines().collect();

    let annonce = lignes
        .iter()
        .position(|ligne| ligne.contains("compilation de la crate migration"))
        .unwrap_or_else(|| panic!("aucune annonce dans :\n{rendu}"));
    let constat = lignes
        .iter()
        .position(|ligne| ligne.contains("répond sur"))
        .unwrap_or_else(|| panic!("aucun constat de base dans :\n{rendu}"));

    assert!(annonce < constat, "{rendu}");
}
```

- [ ] **Step 4: Run the ignored test**

Run: `cargo test -p rbs-cli --test integration_doctor -- --ignored the_slow_check_announces_itself_before_the_finding`
Expected: PASS (plusieurs minutes : la crate `migration` se compile).

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-cli/tests/integration_doctor.rs
git commit
```

Sujet : `test(doctor): prouve la sortie JSON et l'ordre de l'annonce sur le binaire`

---

### Task 6: la documentation dit ce que le CLI fait

**Files:**
- Modify: `docs/docs/cli/{new,add,generate,migrate,seed,dev,doctor,upgrade}.md`
- Modify: `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/{new,add,generate,migrate,seed,dev,doctor,upgrade}.md`
- Modify: `crates/rbs-cli/README.md`

**Interfaces:**
- Consumes: le binaire construit à la tâche 4.
- Produces: rien.

- [ ] **Step 1: Regénérer les blocs d'aide**

Les blocs `--help` des pages sont recopiés verbatim. Ils ne se corrigent pas à la main :
retirer deux lignes change l'alignement des lignes restantes, clap alignant sur l'option
la plus longue. Pour chaque page, relancer la commande et coller la sortie :

```bash
cd /Users/yacoubakone/dev/rs-wt/ergonomie
for commande in new add generate migrate seed dev doctor upgrade; do
  echo "=== $commande ==="
  cargo run -q -p rbs-cli --bin rbs -- "$commande" --help
done
```

Reporter chaque bloc dans la page anglaise **et** dans la page française
correspondante — les blocs de terminal sont identiques dans les deux langues, seule la
prose autour est traduite.

- [ ] **Step 2: Corriger la prose qui les disait globaux**

Deux passages les présentent comme globaux, à récrire dans les deux langues :

- `docs/docs/cli/new.md` (« `--template-dir` and `--yes` are global — every command
  accepts them — but … ») : dire désormais que `--yes` n'existe que sur `new`, seule
  commande qui pose des questions, et `--template-dir` sur `new` et `add`, les deux qui
  les rendent.
- `docs/docs/cli/doctor.md` (« No flag of its own. The two global options are accepted
  because clap propagates them, and neither does anything here. ») : la commande porte
  désormais `--json`, et n'accepte plus les deux autres.

Puis relire chaque page pour toute autre phrase qui suppose la portée globale :

```bash
grep -rn "global\|template-dir\|--yes" docs/docs/cli docs/i18n/fr/docusaurus-plugin-content-docs/current/cli crates/rbs-cli/README.md
```

- [ ] **Step 3: Documenter `--json` et l'annonce**

Dans `docs/docs/cli/doctor.md` et sa traduction, ajouter deux sections après « A project
with problems » :

- **`--json`** : le contrat du document — `sain`, `checks[]`, les clés `name`, `status`,
  `detail`, `remede` ; les trois statuts `ok`, `avertissement`, `echec` ; `remede` omis
  quand il n'y a rien à faire ; le code de sortie inchangé. Un bloc de terminal capturé
  réellement, avec `rbs doctor --json | jq '.checks[] | select(.status != "ok")'`.
- **L'annonce du contrôle `base`** : pourquoi le diagnostic peut prendre une minute — la
  version du serveur se demande au binaire de la crate `migration`, que cargo bâtit — et
  le bloc capturé montrant la ligne `…` puis le constat.

Mettre à jour le bloc « A healthy project » de la page pour qu'il porte l'annonce, capture
réelle à l'appui.

- [ ] **Step 4: Vérifier la parité**

Run: `cd docs && npm run parite`
Expected: aucun écart signalé sur les pages touchées. (L'instrument ne voit ni les tableaux
ni le dernier commit des paires racine : relire soi-même les paires modifiées.)

- [ ] **Step 5: Commit**

```bash
git add docs crates/rbs-cli/README.md
git commit
```

Sujet : `docs(cli): aligne les pages sur la portée des drapeaux et documente --json`

---

### Task 7: vérification d'ensemble

**Files:** aucun, sauf correction.

- [ ] **Step 1: La suite complète**

Run: `cargo test --workspace`
Expected: PASS. Noter le nombre de tests, et le comparer au décompte pris avant la tâche 1.

- [ ] **Step 2: Les deux gardes bloquantes**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Run: `cargo fmt --all --check`
Expected: silence pour les deux.

- [ ] **Step 3: Les preuves par le binaire**

Sur le projet engendré dans `/private/tmp/rbs-ergonomie/demo-api` :

```bash
rbs generate crud users --fields name:string --template-dir ./mes-templates   # refusé par clap
rbs doctor --json | jq .                                                      # exit 0
rbs doctor --json | head -1                                                   # aucune ligne parasite
rbs doctor 2>&1 | while IFS= read -r l; do printf '%s %s\n' "$(date +%T)" "$l"; done
```
Expected : le premier sort en 2 avec `error: unexpected argument '--template-dir'` ; le
deuxième en 0 ; le dernier horodate la ligne `… base` avant la ligne `✓ base`, l'écart
mesurant la compilation.

- [ ] **Step 4: Les tests Docker**

Run: `cargo test --workspace --no-fail-fast -- --ignored`
Expected: PASS. Le `--no-fail-fast` est obligatoire : sans lui la suite s'arrête au premier
binaire et masque les échecs suivants.

- [ ] **Step 5: Commit d'éventuelles corrections**

Si les étapes précédentes ont demandé une correction, la committer sous le type qui
convient (`fix`, `docs`, `test`), sujet en français à l'impératif.
