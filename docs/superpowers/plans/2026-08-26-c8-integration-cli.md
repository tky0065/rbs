# Test d'intégration du CLI — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** prouver, en invoquant le binaire livré, qu'un projet créé par `rbs new` compile et passe ses tests — ce qu'aucun test unitaire de `new::` ne peut établir.

**Architecture:** un test d'intégration unique dans `crates/rbs-cli/tests/`, marqué `#[ignore]`, qui déroule `rbs new` dans un `TempDir` puis lance `cargo build` et `cargo test` sur le projet obtenu. Trois décisions le séparent des tests unitaires existants :

- **Aucun `--template-dir`.** Les 13 tests de `new::` pointent vers `templates/project` du dépôt pour porter sur le squelette réel. Celui-ci fait l'inverse et consomme les templates embarquées par `include_dir` : la question posée n'est pas « les fichiers du dépôt sont-ils bons » mais « le binaire distribué produit-il un projet qui compile ».
- **`--core-path` absolu vers `crates/rbs-core`.** Le noyau n'est pas publié ; sans ce drapeau le projet généré porte une dépendance introuvable et le test échouerait pour une raison étrangère à ce qu'il mesure.
- **`CARGO_TARGET_DIR` redirigé vers `target/rbs-integration`.** Le `TempDir` disparaît à chaque exécution : sans cible partagée, chaque lancement recompile axum, sea-orm et utoipa-swagger-ui depuis zéro. Répertoire distinct de `target/` pour qu'aucun verrou Cargo ne croise celui du test qui l'invoque, et capté par `Swatinem/rust-cache` en CI.

**Tech Stack:** assert_cmd, tempfile, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §4.1, §5.6

## Global Constraints

- Le test porte `#[ignore]` : `cargo test --workspace` reste rapide en local. La CI le lance par une étape dédiée.
- Aucun Docker, aucun PostgreSQL. Le squelette ne génère pas encore de test ; c'est D8 qui lui en donne et D13 qui branche `testcontainers`.
- `cargo build` et `cargo test` du projet généré passent `--workspace` : la crate `migration` est membre du projet sans être une dépendance de sa racine, donc elle échapperait autrement à la compilation.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` restent propres — `--all-targets` compile le nouveau test sans l'exécuter.

---

### Task 1 : le test bout-en-bout

**Files:**
- Create: `crates/rbs-cli/tests/integration_new.rs`
- Modify: `crates/rbs-cli/Cargo.toml` (dev-dependency `assert_cmd`)

**Interfaces:**
- Consomme : le binaire `rbs`, via `assert_cmd::Command::cargo_bin("rbs")`, et ses drapeaux `--database-url`, `--core-path`, `--yes` définis dans `cli.rs`.
- Produit : rien que D13 importe. D13 étendra ce fichier avec un second test, d'où le nom générique du module.

- [x] **Step 1 : déclarer la dépendance**

```toml
[dev-dependencies]
assert_cmd.workspace = true
tempfile.workspace = true
```

- [x] **Step 2 : écrire le test**

```rust
//! Le seul test qui prouve que rbs fonctionne : il invoque le binaire livré, pas
//! `new::creer`, et compile ce que ce binaire a produit.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

/// Racine du dépôt, d'où se déduisent le noyau local et la cible de compilation.
fn depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("la racine du dépôt doit être résoluble")
}

#[test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn le_projet_genere_compile_et_passe_ses_tests() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let noyau = depot().join("crates/rbs-core");

    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent.path())
        .args([
            "new",
            "demo-api",
            "--database-url",
            "postgres://rbs:rbs@localhost:5432/demo_api",
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    let projet = parent.path().join("demo-api");
    assert!(projet.join("Cargo.toml").is_file(), "projet non créé");

    for action in ["build", "test"] {
        Command::new("cargo")
            .current_dir(&projet)
            .env("CARGO_TARGET_DIR", depot().join("target/rbs-integration"))
            .args([action, "--workspace"])
            .assert()
            .success();
    }
}
```

- [x] **Step 3 : le voir passer**

Run: `cargo test -p rbs-cli --test integration_new -- --ignored --nocapture`
Expected: `1 passed`. Le premier lancement compile tout le projet généré ; les suivants réutilisent `target/rbs-integration`.

### Task 2 : la preuve du rouge

**Files:** aucun de façon durable — une modification temporaire de `templates/project/src/main.rs.jinja`, restaurée par `git checkout`.

Le critère dit « échoue si le projet généré ne compile pas ». Un test vert ne le démontre pas : il faut voir le rouge, et le voir sur l'étape `cargo build`, pas sur `rbs new`.

- [x] **Step 1 : casser une template**

Insérer une ligne invalide (`let _: u32 = "pas un entier";`) dans le corps de `main` de `templates/project/src/main.rs.jinja`.

- [x] **Step 2 : recompiler le binaire, puis lancer**

Les templates sont embarquées par `include_dir` : sans recompilation, le binaire sert encore l'ancienne version.

Run: `cargo test -p rbs-cli --test integration_new -- --ignored`
Expected: FAIL, sur l'assertion `success()` de `cargo build`, avec `E0308 mismatched types` dans la sortie.

- [x] **Step 3 : restaurer**

Run: `git checkout templates/project/src/main.rs.jinja && cargo test -p rbs-cli --test integration_new -- --ignored`
Expected: vert de nouveau.

### Task 3 : la CI

**Files:**
- Modify: `.github/workflows/ci.yml`

- [x] **Step 1 : ajouter l'étape**

Après `cargo test`, `--workspace` plutôt qu'un nom de test précis pour que D13 s'y greffe sans retoucher le workflow :

```yaml
      - name: cargo test (intégration)
        run: cargo test --workspace -- --ignored
```

- [x] **Step 2 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning, et `cargo test --workspace` toujours rapide — le test lourd est ignoré.

- [x] **Step 3 : cocher C8 et commiter**

Message : `test(cli): compile le projet généré dans un test d'intégration`, corps portant le *pourquoi* et un intertitre `Vérifications :` avec les sorties rouge et verte.
