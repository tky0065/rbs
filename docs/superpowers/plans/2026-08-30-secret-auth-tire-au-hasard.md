# Secret d'`auth` tiré au hasard dans le `.env`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs add auth` (et `rbs new --with auth`) écrit dans le `.env` du projet un secret HS256 tiré au hasard, au lieu de conseiller de recopier celui que la crate publie.

**Architecture:** le manifeste de fragment gagne un champ `secret` sur ses `[[env]]`. Une variable ainsi marquée continue d'aller dans `.env.example` avec son placeholder — le fichier versionné reste la référence de `doctor` — et reçoit **en plus** une valeur tirée au hasard dans `.env`, qui est gitignoré. `installation.rs` ne connaît toujours aucune feature par son nom.

**Tech Stack:** Rust, `rand` 0.10 (déjà épinglé au workspace, API `SysRng` + `TryRng` telle que `rbs-core/src/token.rs:23` l'emploie), `serde`/`toml_edit` pour le manifeste.

**Spec:** `IMPROVE.md` tâche 1 (P0, Sécu). Design validé en chat le 2026-08-30 ; options retenues : « étendre le manifeste » et non un cas spécial `auth`.

## Global Constraints

- Commits en Conventional Commits, sujet français à l'impératif, **aucun** identifiant de tâche, aucun renvoi à `IMPROVE.md`, aucune ligne `Co-Authored-By` ni mention d'un assistant (`CLAUDE.md`, section Commits).
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `#![warn(missing_docs)]` est actif sur `rbs-core` mais pas sur `rbs-cli` ; les items `pub(crate)` de `rbs-cli` portent tout de même un `///`, par convention du dépôt.
- Documentation bilingue : toute page modifiée sous `docs/docs/` l'est aussi sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/`, **dans le même commit**.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont bloquants.
- Ne pas toucher `docs/build/` : c'est un artefact gitignoré.

---

### Task 1: le tirage du secret

**Files:**
- Create: `crates/rbs-cli/src/secret.rs`
- Modify: `crates/rbs-cli/Cargo.toml` (bloc `[dependencies]`), `crates/rbs-cli/src/lib.rs` (déclaration du module, à ranger dans l'ordre alphabétique des `mod`)

**Interfaces:**
- Produces: `pub(crate) fn secret::tire_au_hasard() -> String` — 32 octets tirés du générateur système, rendus en 64 caractères hexadécimaux minuscules.

- [x] **Step 1: écrire le test qui échoue**

Dans `crates/rbs-cli/src/secret.rs`, sous le module d'implémentation :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_secret_is_sixty_four_hexadecimal_characters() {
        let secret = tire_au_hasard();

        assert_eq!(secret.len(), 64, "{secret}");
        assert!(
            secret.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
            "{secret}"
        );
    }

    /// Le critère de la tâche : deux installations ne partagent pas leur secret.
    #[test]
    fn two_draws_do_not_collide() {
        assert_ne!(tire_au_hasard(), tire_au_hasard());
    }
}
```

- [x] **Step 2: lancer le test et le voir échouer**

Run: `cargo test -p rbs-cli secret::`
Expected: FAIL — le module `secret` n'existe pas encore / `tire_au_hasard` introuvable.

- [x] **Step 3: ajouter la dépendance**

Dans `crates/rbs-cli/Cargo.toml`, `[dependencies]`, en gardant l'ordre alphabétique existant (`minijinja`, puis `rand`, puis `serde`) :

```toml
rand.workspace = true
```

- [x] **Step 4: écrire l'implémentation minimale**

`crates/rbs-cli/src/secret.rs` :

```rust
//! Tirage des secrets que `rbs` dépose dans le `.env` d'un projet.
//!
//! L'hexadécimal plutôt que le base64 de `rbs-core` : la valeur traverse un fichier
//! d'environnement, où seul un alphabet sans `+`, `/` ni `=` échappe à la question du
//! guillemet.

use rand::TryRng;
use rand::rngs::SysRng;

/// Longueur du secret tiré, en octets.
///
/// Le double du minimum qu'exige `rbs-core` : la marge coûte 32 octets dans un fichier
/// et dispense d'y revenir.
const OCTETS: usize = 32;

/// Tire un secret de 32 octets, rendu en hexadécimal minuscule.
///
/// # Panics
///
/// Panique si le générateur du système est indisponible. Aucun appelant ne saurait
/// traiter cet échec : sans source d'aléa, il n'y a pas de secret à écrire, et en
/// inventer un serait précisément le défaut que cette fonction corrige.
pub(crate) fn tire_au_hasard() -> String {
    let mut octets = [0u8; OCTETS];
    SysRng
        .try_fill_bytes(&mut octets)
        .expect("le générateur du système doit être disponible");

    octets.iter().map(|octet| format!("{octet:02x}")).collect()
}
```

Puis dans `crates/rbs-cli/src/lib.rs`, déclarer `mod secret;` à sa place alphabétique parmi les autres `mod`.

- [x] **Step 5: lancer le test et le voir passer**

Run: `cargo test -p rbs-cli secret::`
Expected: PASS, 2 tests.

- [x] **Step 6: commit**

```bash
git add crates/rbs-cli/src/secret.rs crates/rbs-cli/src/lib.rs crates/rbs-cli/Cargo.toml Cargo.lock
git commit -m "feat(cli): tire un secret hexadécimal du générateur du système"
```

---

### Task 2: le champ `secret` du manifeste et l'écriture dans `.env`

**Files:**
- Modify: `crates/rbs-cli/src/manifest.rs:109-113` (`DeclaredVariable`)
- Modify: `crates/rbs-cli/src/add/installation.rs:19-23` (les constantes) et `:155-162` (la boucle `env`)
- Test: `crates/rbs-cli/src/add/installation.rs`, module `tests` existant (constante `PATCHS` en `:374`, helpers `avec` et `projected`)

**Interfaces:**
- Consumes: `secret::tire_au_hasard()` de la Task 1.
- Produces: `manifest::DeclaredVariable.secret: bool` — `false` par défaut, donc les six autres fragments sont inchangés.

- [x] **Step 1: écrire les trois tests qui échouent**

Dans le module `tests` d'`installation.rs`, ajouter à côté de `PATCHS` un second manifeste, puis les tests. Noter que `plan_for(root, manifeste, templates)` et les helpers `avec` / `projected` existent déjà dans ce module.

```rust
/// Le même fragment, sa variable marquée comme portant un secret.
const PATCHS_SECRET: &str = "[feature]\ndescription = \"auth\"\n\n\
     [[env]]\nkey = \"RBS_AUTH__SECRET\"\nvalue = \"changez-moi\"\nsecret = true\n\
     comment = \"Secret de signature HS256, au moins 32 octets\"\n";

/// Le critère de la tâche : l'exemple versionné garde son placeholder, le `.env` reçoit
/// une valeur qui n'est pas celle-là.
#[test]
fn a_secret_variable_reaches_the_env_with_a_drawn_value() {
    let project = TempDir::new().expect("répertoire temporaire créable");
    avec(
        project.path(),
        &[
            (".env", "RBS_ENV=development\n"),
            (".env.example", "RBS_ENV=development\n"),
        ],
    );

    let (_, plan) =
        plan_for(project.path(), PATCHS_SECRET, &[]).expect("le plan doit se calculer");

    let exemple = projected(&plan, ".env.example");
    assert!(
        exemple.contains("RBS_AUTH__SECRET=changez-moi"),
        "l'exemple versionné doit garder son placeholder :\n{exemple}"
    );

    let env = projected(&plan, ".env");
    let tire = crate::dotenv::value(&crate::dotenv::parse(env), "RBS_AUTH__SECRET")
        .expect("le .env doit porter la variable");
    assert_eq!(tire.len(), 64, "{env}");
    assert_ne!(tire, "changez-moi", "{env}");
}

/// Le critère de la tâche : un secret déjà en place n'est jamais écrasé.
#[test]
fn an_existing_secret_is_left_untouched() {
    let project = TempDir::new().expect("répertoire temporaire créable");
    avec(
        project.path(),
        &[
            (".env", "RBS_AUTH__SECRET=le-mien\n"),
            (".env.example", "RBS_ENV=development\n"),
        ],
    );

    let (_, plan) =
        plan_for(project.path(), PATCHS_SECRET, &[]).expect("le plan doit se calculer");

    let env = projected(&plan, ".env");
    assert_eq!(
        crate::dotenv::value(&crate::dotenv::parse(env), "RBS_AUTH__SECRET"),
        Some("le-mien"),
        "{env}"
    );
}

/// Sans le marqueur, rien ne change pour les six autres fragments.
#[test]
fn a_plain_variable_never_reaches_the_env() {
    let project = TempDir::new().expect("répertoire temporaire créable");
    avec(
        project.path(),
        &[
            ("Cargo.toml", CARGO),
            ("config/default.toml", "[server]\nport = 8080\n"),
            (".env", "RBS_ENV=development\n"),
            (".env.example", "RBS_DATABASE__URL=postgres://\n"),
        ],
    );

    let (_, plan) = plan_for(project.path(), PATCHS, &[]).expect("le plan doit se calculer");

    assert!(
        !plan.files().iter().any(|file| file.path == ".env"),
        "le .env n'a pas à être touché : {:?}",
        plan.files().iter().map(|f| &f.path).collect::<Vec<_>>()
    );
}
```

- [x] **Step 2: lancer les tests et les voir échouer**

Run: `cargo test -p rbs-cli add::installation::tests`
Expected: FAIL — `unknown field 'secret'` refusé par `#[serde(deny_unknown_fields)]` sur `DeclaredVariable` (les deux premiers tests), le troisième passant déjà.

- [x] **Step 3: ajouter le champ au manifeste**

`crates/rbs-cli/src/manifest.rs`, dans `DeclaredVariable` :

```rust
pub(crate) struct DeclaredVariable {
    pub key: String,
    pub value: String,
    pub comment: Option<String>,
    /// La variable porte un secret propre à chaque installation.
    ///
    /// `value` reste l'exemple versionné, que `doctor` compare au `.env` pour dire si le
    /// développeur l'a remplacé ; c'est le `.env`, gitignoré, qui reçoit la valeur tirée.
    #[serde(default)]
    pub secret: bool,
}
```

- [x] **Step 4: écrire dans le `.env`**

`crates/rbs-cli/src/add/installation.rs`. Remplacer les constantes `:19-23` :

```rust
/// Où les variables d'environnement d'un fragment sont déclarées.
///
/// Versionné, il ne porte que des valeurs à remplacer : c'est la référence à laquelle
/// `doctor` compare le `.env` du développeur.
const FICHIER_EXEMPLE: &str = ".env.example";

/// Le fichier d'environnement du projet, gitignoré.
///
/// Seule une variable déclarée `secret` y descend, et avec une valeur tirée au hasard :
/// un secret qu'un exemple publié suffirait à deviner n'en est pas un.
const FICHIER_ENV: &str = ".env";
```

Puis la boucle `:155-162` :

```rust
for variable in &fragment.manifest.env {
    builder.add_variable(
        FICHIER_EXEMPLE,
        &variable.key,
        &variable.value,
        variable.comment.as_deref(),
    )?;

    if !variable.secret {
        continue;
    }

    // Un projet dont le `.env` a été supprimé n'a pas à voir `add` échouer : le
    // fichier est gitignoré, le reposer ne coûte rien et ne perd rien.
    if !builder.exists(FICHIER_ENV)? {
        builder.create(FICHIER_ENV, "")?;
    }

    builder.add_variable(
        FICHIER_ENV,
        &variable.key,
        &secret::tire_au_hasard(),
        variable.comment.as_deref(),
    )?;
}
```

Ajouter `use crate::secret;` aux `use` du fichier, à sa place alphabétique.

- [x] **Step 5: lancer les tests et les voir passer**

Run: `cargo test -p rbs-cli add::installation::tests`
Expected: PASS — y compris les tests préexistants `the_configuration_section_and_the_environment_variable_are_added` et `the_three_patches_are_no_ops_the_second_time`, que la constante `FICHIER_EXEMPLE` renommée ne doit pas avoir cassés.

- [x] **Step 6: commit**

```bash
git add crates/rbs-cli/src/manifest.rs crates/rbs-cli/src/add/installation.rs
git commit -m "feat(add): dépose la valeur des variables marquées secrètes dans le .env"
```

---

### Task 3: marquer `auth` et corriger ce que le CLI dit ensuite

**Files:**
- Modify: `crates/rbs-cli/templates/features/auth/feature.toml` (bloc `[[env]]`, vers `:69-72`)
- Modify: `crates/rbs-cli/src/lib.rs:326-331` (le bras `"auth"` de `fn suite`)
- Modify: `crates/rbs-cli/src/lib.rs:608-612` (le commentaire du test qui cite l'ancien conseil)
- Modify: `crates/rbs-cli/src/doctor/auth.rs:1-6` (doc de module)
- Modify: `crates/rbs-cli/src/add/mod.rs:698-713` (le test `.env.example`)

**Interfaces:**
- Consumes: le champ `secret` de la Task 2.

- [x] **Step 1: marquer la variable du fragment**

`crates/rbs-cli/templates/features/auth/feature.toml` :

```toml
[[env]]
key     = "RBS_AUTH__SECRET"
value   = "changez-moi-par-un-secret-tire-au-hasard-de-32-octets-au-moins"
secret  = true
comment = "Secret de signature HS256. 32 octets au moins, sans quoi le démarrage échoue."
```

- [x] **Step 2: adapter le conseil de fin d'installation**

`crates/rbs-cli/src/lib.rs`, bras `"auth"` de `fn suite` — l'ancien commentaire et l'ancien texte sont tous deux faux désormais :

```rust
// Le secret est tiré à l'installation et déposé dans le `.env` : il ne reste que la
// migration, sans quoi les tables d'authentification manqueraient au premier login.
"auth" => Some("rbs migrate up"),
```

- [x] **Step 3: relire les deux commentaires devenus faux**

`crates/rbs-cli/src/lib.rs:608-612` et l'en-tête de module de `crates/rbs-cli/src/doctor/auth.rs:1-6` affirment tous deux qu'`add auth` n'écrit que dans `.env.example`. Les réécrire pour dire ce qui est vrai : le `.env` reçoit un secret tiré, l'exemple versionné garde son placeholder, et le contrôle de `doctor` sert désormais aux projets antérieurs à ce changement et aux `.env` recopiés à la main.

**Ne pas toucher à la logique de `doctor/auth.rs`** : sa comparaison `.env` vs `.env.example` (`:68`) reste exactement le bon test, elle passe simplement au vert sur un projet neuf.

- [x] **Step 4: étendre le test d'`add/mod.rs`**

`crates/rbs-cli/src/add/mod.rs:698-713` vérifie que le secret est bien déclaré dans `.env.example`. Ce test reste vrai : le garder tel quel, et ajouter à sa suite l'assertion symétrique — le `.env` projeté porte `RBS_AUTH__SECRET` avec une valeur de 64 caractères différente de celle de l'exemple. Reprendre le helper `projected` du module.

- [x] **Step 5: lancer les tests**

Run: `cargo test -p rbs-cli`
Expected: PASS. Si un test d'intégration `assert_cmd` capture la ligne « recopiez RBS_AUTH__SECRET… », mettre à jour son attendu — c'est la sortie réelle qui a changé, pas le test qui a tort.

- [x] **Step 6: commit**

```bash
git add crates/rbs-cli/templates/features/auth/feature.toml crates/rbs-cli/src/lib.rs crates/rbs-cli/src/doctor/auth.rs crates/rbs-cli/src/add/mod.rs
git commit -m "fix(auth): tire le secret de signature à l'installation au lieu de publier le sien"
```

---

### Task 4: la documentation, dans les deux langues

**Files:**
- Modify: `docs/docs/cli/new.md:264,281`, `docs/docs/cli/add.md:159,163`, `docs/docs/guides/auth.md:47,65`
- Modify: les trois mêmes sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/`

- [x] **Step 1: recenser ce qui ment**

Run: `rg -n "RBS_AUTH__SECRET|recopiez RBS_AUTH__SECRET" docs/docs docs/i18n`
Deux familles à corriger :
1. les blocs de sortie capturés, qui affichent encore la ligne `recopiez RBS_AUTH__SECRET de .env.example vers votre .env, puis rbs migrate up` ;
2. le paragraphe de `guides/auth.md:65` — « The fragment writes `RBS_AUTH__SECRET` into `.env.example` — never into your `.env` » et son équivalent français — qui affirme maintenant l'inverse du binaire, ainsi que l'extrait de `.env` qui le suit.

- [x] **Step 2: régénérer la sortie plutôt que l'inventer**

Les blocs annoncent être capturés en lançant la commande. Bâtir le binaire puis lancer réellement `rbs new` / `rbs add auth` dans un répertoire temporaire, et recopier la sortie obtenue :

```bash
cargo build -p rbs-cli
cd "$(mktemp -d)" && /chemin/vers/rs/target/debug/rbs new blog --with auth --yes
```

- [x] **Step 3: réécrire le paragraphe du guide**

Dans `docs/docs/guides/auth.md` et sa version française, dire ce qui se passe : `add auth` tire le secret et l'écrit dans le `.env`, gitignoré ; `.env.example` garde un placeholder pour que le fichier versionné documente la variable sans la livrer ; un déploiement fournit sa propre valeur par l'environnement, et `rbs doctor` refuse un `.env` resté sur le placeholder. Garder la section « rotating the secret » (`:187`), qui reste juste.

- [x] **Step 4: vérifier la parité**

Run: `node docs/parite.mjs` si le script existe, sinon comparer à la main que chaque fichier anglais modifié a son pendant français modifié.
Expected: aucun écart nouveau.

- [x] **Step 5: commit**

```bash
git add docs/docs docs/i18n
git commit -m "docs: décrit le secret d'auth tiré à l'installation, en anglais et en français"
```

---

### Vérification finale (avant de rendre la main)

- [x] `cargo test --workspace` — lire la sortie, pas la supposer
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo fmt --all --check`
- [x] `rg -n "recopiez RBS_AUTH__SECRET" crates docs/docs docs/i18n` → aucun résultat

---

### Écarts relevés à l'exécution

- **Task 2, Step 1.** Le test `a_secret_variable_reaches_the_env_with_a_drawn_value` tel
  qu'écrit ici ne compile pas : `dotenv::value` emprunte le `Vec` que `dotenv::parse`
  rend, et le temporaire meurt en fin d'instruction (E0716). La paire est liée à un `let`
  avant l'appel.
- **Task 3, Step 4.** `add/mod.rs:698-713` est le test du mot de passe SMTP de `mail`, non
  celui du secret d'`auth` — aucun test d'`add/mod.rs` ne portait sur `auth`. L'assertion
  demandée a donc été écrite comme un test neuf,
  `adding_auth_draws_the_signing_secret_into_the_env`.
- **Une cinquième tâche, non prévue.** `tests/integration_examples.rs` compare
  `examples/blog-auth` à une génération fraîche, fichier par fichier : le secret étant
  tiré, aucun `.env` versionné ne pouvait plus correspondre. La comparaison masque
  désormais la seule forme tirée — soixante-quatre hexadécimaux — et laisse le placeholder
  de `.env.example` comparé caractère par caractère.
