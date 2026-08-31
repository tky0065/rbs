# Factorisations de la dette et de la performance du CLI — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** supprimer sept duplications et relectures inutiles de `crates/rbs-cli/src/`, à comportement observable strictement inchangé.

**Architecture :** quatre corrections locales (résolution des ancres, contrôle de section de `doctor`, lectures répétées de `doctor`, nom du paquet déjà parsé) puis trois factorisations transverses (fixture de test partagée, module d'erreurs communes, préambule et rituel des commandes qui modifient un projet). Rien n'est ajouté à `rbs-core`, rien n'est touché sous `templates/`.

**Tech Stack :** Rust 2024, `thiserror`, `toml_edit`, `tempfile`, `assert_cmd`, `testcontainers`.

**Spec :** `docs/superpowers/specs/2026-08-31-factorisations-cli-design.md` (tâches 37, 38, 39 ; les tâches 35, 36, 40, 41 sont locales et décrites intégralement ici).

## Global Constraints

- **Comportement observable strictement inchangé.** Aucun message d'erreur, aucune ligne
  de sortie, aucun code de sortie ne bouge d'un caractère.
- **Aucun test supprimé ni affaibli.** Le nombre de tests de `cargo test --workspace` ne
  doit pas diminuer. Baseline mesurée avant le premier commit, reportée à la fin.
- **Ne pas toucher `crates/rbs-cli/src/cli.rs` ni la signature des sous-commandes.**
- **Ne pas toucher `templates/` ni `examples/`** : `git status` doit rester muet sous
  `examples/`.
- **Ne pas toucher `IMPROVE.md`.**
- Commits en Conventional Commits, sujet en français à l'impératif, sans identifiant de
  tâche, sans renvoi à un fichier de suivi, sans `Co-Authored-By` ni `Claude-Session`.
  Corps portant le *pourquoi* technique et un intertitre `Vérifications :`.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- Bloquant après chaque tâche : `cargo test --workspace`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo fmt --all --check`.

**Le cycle de test de ce lot n'est pas TDD.** Un refactoring à comportement inchangé n'a
pas de test rouge à écrire : la suite existante *est* le test, et elle est verte avant
comme après. Un test n'est ajouté que là où une fonction nouvelle porte une règle qui
n'était vérifiée nulle part (tâches 40 et 41). Toute rougeur en cours de route est une
régression, jamais une étape.

---

### Task 0 : mesurer la baseline

**Files:** aucun.

- [ ] **Step 1 : compter les tests avant toute modification**

```bash
cd /Users/yacoubakone/dev/rs-wt/dette-cli
cargo test --workspace 2>&1 | grep -E "^test result:" 
```

Noter la somme des `N passed` de chaque binaire. C'est le nombre à retrouver, au moins, à
la fin du lot.

- [ ] **Step 2 : vérifier que l'arbre est propre**

```bash
git status --porcelain
```

Attendu : seuls le design doc et ce plan apparaissent.

---

### Task 1 : une seule résolution des ancres (tâche 40)

**Files:**
- Modify: `crates/rbs-cli/src/anchors.rs` (autour de `resolve_features`, l. 247-258)
- Modify: `crates/rbs-cli/src/agents.rs` (`anchor_list`, `present_anchors`)
- Modify: `crates/rbs-cli/src/doctor/anchors.rs` (`check`)
- Modify: `crates/rbs-cli/src/templates.rs` (test `each_anchor_is_opened_then_closed_in_its_file`)
- Test: `crates/rbs-cli/src/anchors.rs` (module `tests` du même fichier)

**Interfaces:**
- Consomme : `anchors::ANCRES`, `anchors::FEATURES`, `Anchor::in_file`.
- Produit :
  - `pub(crate) fn resolve(anchor: Anchor, with_library: bool) -> Anchor`
  - `pub(crate) fn resolved(root: &Path) -> Vec<Anchor>`
  - `pub(crate) fn resolve_features(root: &Path) -> Anchor` (signature inchangée)

- [ ] **Step 1 : écrire les deux fonctions dans `anchors.rs`**

Juste après `resolve_features`, en remplaçant son corps :

```rust
/// La même ancre, celle des features visant la bibliothèque quand le projet en a une.
///
/// Séparée de [`resolve_features`] pour que la règle n'existe qu'ici : quatre appelants
/// la réécrivaient, et une ancre ajoutée au registre demandait de les visiter tous.
pub(crate) fn resolve(anchor: Anchor, with_library: bool) -> Anchor {
    if with_library && anchor.name == FEATURES.name {
        anchor.in_file("src/lib.rs")
    } else {
        anchor
    }
}

/// Les ancres du registre, celle des features résolue pour `root`.
///
/// Le disque n'est interrogé qu'une fois pour les onze, et non une fois par ancre.
pub(crate) fn resolved(root: &Path) -> Vec<Anchor> {
    let with_library = root.join("src/lib.rs").exists();

    ANCRES
        .into_iter()
        .map(|anchor| resolve(anchor, with_library))
        .collect()
}
```

et `resolve_features` devient :

```rust
pub(crate) fn resolve_features(root: &Path) -> Anchor {
    resolve(FEATURES, root.join("src/lib.rs").exists())
}
```

Le doc-commentaire existant de `resolve_features` (le repli `src/lib.rs` / `src/main.rs`
et la raison du parc existant) reste sur `resolve_features`, sans être recopié.

- [ ] **Step 2 : un test qui verrouille la nouvelle fonction**

Dans le module `tests` de `anchors.rs` :

```rust
/// Une ancre ajoutée au registre doit paraître dans la liste résolue sans que personne
/// n'ait à toucher les appelants.
#[test]
fn the_resolved_registry_carries_every_anchor() {
    let project = TempDir::new().expect("répertoire temporaire créable");

    let resolues = resolved(project.path());

    assert_eq!(resolues.len(), ANCRES.len());
    for anchor in ANCRES {
        assert!(
            resolues.iter().any(|autre| autre.name == anchor.name),
            "`{}` manque à la liste résolue",
            anchor.name
        );
    }
}

/// Sans bibliothèque, l'ancre des features reste dans `src/main.rs` : c'est le parc des
/// projets engendrés avant que le squelette n'en porte une.
#[test]
fn without_a_library_the_features_anchor_stays_in_the_binary() {
    let project = TempDir::new().expect("répertoire temporaire créable");

    let features = resolved(project.path())
        .into_iter()
        .find(|anchor| anchor.name == FEATURES.name)
        .expect("l'ancre des features est au registre");

    assert_eq!(features.file, "src/main.rs");
}
```

(`TempDir` est déjà importé par le module de tests de `anchors.rs` ; vérifier et ajouter
`use tempfile::TempDir;` si nécessaire.)

- [ ] **Step 3 : lancer ces deux tests**

```bash
cargo test -p rbs-cli --lib anchors::tests
```

Attendu : PASS.

- [ ] **Step 4 : remplacer les quatre copies**

`agents.rs`, dans `anchor_list` — le `.map(…)` de résolution disparaît :

```rust
    let registre = anchors::resolved(root)
        .iter()
        .map(relie)
        .collect::<Vec<_>>()
        .join("\n");
```

`agents.rs`, dans `present_anchors` :

```rust
    anchors::resolved(root)
        .into_iter()
        .filter(|anchor| !anchor.optional || porte(anchor.file.as_ref()))
        .map(|anchor| format!("{} ({})", anchor.name, anchor.file))
        .collect()
```

`doctor/anchors.rs`, dans `check` :

```rust
    // L'ancre des features se résout par repli : `src/lib.rs` sur un projet engendré
    // depuis ce jalon, `src/main.rs` sur un projet plus ancien, dépourvu de bibliothèque.
    let anchors = anchors::resolved(root);
```

`templates.rs`, dans le test — le `if` disparaît, la raison reste :

```rust
            // L'ancre des features vise `src/lib.rs` depuis ce jalon : c'est là que le
            // squelette la rend, `src/main.rs` n'étant qu'un repli pour un projet plus
            // ancien, sans bibliothèque.
            let anchor = crate::anchors::resolve(anchor, true);
```

Ajuster les `use` devenus inutiles (`ANCRES`, `Anchor` dans `doctor/anchors.rs`) — clippy
les signalera.

- [ ] **Step 5 : vérifier**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Attendu : suite verte, aucun warning.

- [ ] **Step 6 : commit**

```bash
git add -A
git commit -m "refactor(cli): rassemble la résolution des ancres en une seule fonction"
```

---

### Task 2 : un contrôle de section partagé pour `doctor` (tâche 41)

**Files:**
- Modify: `crates/rbs-cli/src/doctor/mod.rs` (ajout de `section_check`)
- Modify: `crates/rbs-cli/src/doctor/redis.rs`
- Modify: `crates/rbs-cli/src/doctor/jobs.rs`
- Test: modules `tests` de `redis.rs` et `jobs.rs` (inchangés dans leur substance)

**Interfaces:**
- Consomme : `doctor::Check`, `doctor::section`.
- Produit : `pub(crate) fn section_check(titre: &'static str, section: &str, present: &str, reglages: &str) -> Check`.

**Attention aux messages** : le détail du succès diffère entre les deux modules — « la
configuration du cache est en place » pour `redis`, « la configuration de la file est en
place » pour `jobs`. Il est donc paramètre, pas constante.

- [ ] **Step 1 : écrire `section_check` dans `doctor/mod.rs`**

À côté de `section` :

```rust
/// Le contrôle d'une feature dont tout le diagnostic tient à sa section de configuration.
///
/// Seul `config/default.toml` est lu : le CLI ne sait pas quel `RBS_ENV` l'utilisateur
/// emploiera, et une section posée dans le seul `config/production.toml` échapperait donc
/// au diagnostic comme elle échappe au défaut du projet.
fn section_check(
    root: &Path,
    titre: &'static str,
    section: &str,
    present: &str,
    reglages: &str,
) -> Check {
    if self::section(root, section) {
        return Check::ok(titre, present);
    }

    Check::failed(
        titre,
        format!("{CONFIG} ne porte pas de section `[{section}]`"),
        format!("ajoutez à {CONFIG} :\n[{section}]\n{reglages}"),
    )
}
```

avec, en tête du module, `const CONFIG: &str = "config/default.toml";` — la constante
existe déjà, à l'identique, dans `redis.rs`, `jobs.rs`, `mail.rs` et `storage.rs` ; celles
de `redis.rs` et `jobs.rs` disparaissent, les deux autres restent (leurs modules ne sont
pas touchés par cette tâche).

- [ ] **Step 2 : réduire `redis.rs` à son appel**

```rust
pub(crate) fn check(root: &Path) -> Check {
    super::section_check(
        root,
        TITRE,
        SECTION,
        "la configuration du cache est en place",
        "url = \"redis://127.0.0.1:6379\"\nttl_secs = 300",
    )
}
```

Le doc-commentaire de `check` qui répétait la règle du `RBS_ENV` est retiré : il vit
désormais sur `section_check`. Le doc-module de `redis.rs`, qui dit pourquoi la feature
`redis` se configure sous `[cache]`, reste — il n'est pas partagé.

- [ ] **Step 3 : réduire `jobs.rs` à son appel**

```rust
pub(crate) fn check(root: &Path) -> Check {
    super::section_check(
        root,
        TITRE,
        SECTION,
        "la configuration de la file est en place",
        "max_attempts = 5\nretry_delay_secs = 30\npoll_interval_secs = 1",
    )
}
```

- [ ] **Step 4 : vérifier octet à octet que les remèdes n'ont pas bougé**

Les tests existants `the_remedy_carries_the_three_settings_of_the_fragment` et
`without_a_{cache,jobs}_section_the_diagnosis_says_so` couvrent déjà détail et remède.
Les lancer :

```bash
cargo test -p rbs-cli --lib doctor::redis doctor::jobs
```

Attendu : PASS, sans qu'une seule assertion ait été modifiée.

- [ ] **Step 5 : vérifier**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

- [ ] **Step 6 : commit**

```bash
git add -A
git commit -m "refactor(doctor): partage le contrôle des features tenant à une section"
```

**Ce qui n'entre pas dans cette factorisation** : `doctor/mail.rs` et `doctor/storage.rs`
accumulent plusieurs défauts, lisent le `.env` en plus de la configuration, et leur
section n'est qu'un défaut parmi d'autres — ils ne rendent pas un `Check` sur la seule
présence d'une section. Ils restent tels quels ; le rapport final le dit.

---

### Task 3 : `doctor` lit chaque fichier une fois (tâche 35)

**Files:**
- Modify: `crates/rbs-cli/src/doctor/mod.rs` (`run`, `section`, `field`, `FEATURE_CHECKS`, `installed_feature`)
- Modify: `crates/rbs-cli/src/doctor/auth.rs`, `redis.rs`, `jobs.rs`, `mail.rs`, `storage.rs`

**Interfaces:**
- Produit :
  - `pub(crate) struct Config` avec `Config::read(root: &Path) -> Config`,
    `fn section(&self, name: &str) -> bool`, `fn field(&self, section: &str, key: &str) -> Option<String>`
  - `type FeatureCheck = (&'static str, fn(&Path, &Config) -> Check)`
  - `fn section_check(config: &Config, titre, section, present, reglages) -> Check`

- [ ] **Step 1 : écrire `Config` dans `doctor/mod.rs`, en remplacement de `section` et `field`**

```rust
/// `config/default.toml` du projet, lu et analysé une seule fois.
///
/// Un diagnostic complet interrogeait ce fichier jusqu'à huit fois, chaque contrôle le
/// relisant et le réanalysant pour une question d'une ligne — `storage` en enchaîne trois
/// d'affilée.
pub(crate) struct Config(Option<toml_edit::DocumentMut>);

impl Config {
    /// Lit la configuration du projet. Un fichier absent ou illisible se comporte comme
    /// un fichier vide : ce qui intéresse un contrôle est de disposer ou non de la
    /// valeur, jamais laquelle des deux couches manque.
    pub(crate) fn read(root: &Path) -> Self {
        Self(
            std::fs::read_to_string(root.join(CONFIG))
                .ok()
                .and_then(|source| source.parse::<toml_edit::DocumentMut>().ok()),
        )
    }

    /// Vrai si la configuration porte une section `[name]`.
    ///
    /// Lu par `toml_edit` et non par recherche de texte : une section en commentaire
    /// n'est pas une section.
    pub(crate) fn section(&self, name: &str) -> bool {
        self.0
            .as_ref()
            .is_some_and(|document| document.get(name).is_some())
    }

    /// Valeur d'un champ, s'il est renseigné.
    pub(crate) fn field(&self, section: &str, key: &str) -> Option<String> {
        self.0.as_ref().and_then(|document| {
            document
                .get(section)
                .and_then(|table| table.get(key))
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
    }
}
```

Les fonctions libres `section` et `field` sont supprimées.

- [ ] **Step 2 : lire le manifeste et la configuration une seule fois dans `run`**

```rust
    // Une seule lecture pour toute la boucle : `installed_feature` refaisait une analyse
    // complète du manifeste par entrée du tableau, et chaque contrôle relisait la
    // configuration pour une question d'une ligne.
    let installees = metadata::read(&root.join("Cargo.toml"))
        .map(|metadonnees| metadonnees.features)
        .unwrap_or_default();
    let config = Config::read(&root);

    // Un projet qui n'a pas installé une feature n'a pas à lire une ligne à son sujet :
    // le rapport ne porte que des contrôles dont le verdict le concerne.
    for (feature, check) in FEATURE_CHECKS {
        if installees.iter().any(|installee| installee == feature) {
            checks.push(check(&root, &config));
        }
    }
```

`installed_feature` est supprimée. Le repli `unwrap_or_default()` reproduit exactement
l'ancien `is_ok_and(...)` : un manifeste illisible ne fait paraître aucun contrôle de
feature — et `project_root`, deux lignes plus haut, a de toute façon déjà refusé un
manifeste cassé.

- [ ] **Step 3 : adapter le tableau**

```rust
type FeatureCheck = (&'static str, fn(&Path, &Config) -> Check);

const FEATURE_CHECKS: [FeatureCheck; 6] = [
    ("auth", auth::check),
    ("auth", |root, _| guards::check(root)),
    ("redis", redis::check),
    ("mail", mail::check),
    ("storage", storage::check),
    ("jobs", jobs::check),
];
```

`guards::check` ne lit pas la configuration : la fermeture sans capture, qui se convertit
en pointeur de fonction, lui évite un paramètre qu'elle n'emploierait pas.

- [ ] **Step 4 : propager `&Config` dans les cinq contrôles**

Signatures : `pub(crate) fn check(root: &Path, config: &Config) -> Check` pour `auth`,
`redis`, `jobs`, `mail`, `storage`. Dans les corps, `super::section(root, X)` devient
`config.section(X)` et `super::field(root, S, K)` devient `config.field(S, K)`.

`redis::check` et `jobs::check` passent `config` à `section_check`, dont la signature
perd `root` au profit de `config: &Config`.

`mail::check_with` et `storage::check_with` prennent un `config: &Config` de plus, à
côté de leur `env` — leur signature interne, appelée par leurs tests.

- [ ] **Step 5 : adapter les tests des cinq modules**

Chaque appel `check(&root)` d'un test devient `check(&root, &Config::read(&root))`, avec
`use super::super::Config;` en tête du module de tests. **Aucune assertion ne change.**
Les tests qui réécrivent la configuration entre deux appels doivent relire la `Config`
après réécriture — c'est déjà ce que fait `rewrite` puis `check(&root)`.

- [ ] **Step 6 : vérifier**

```bash
cargo test -p rbs-cli --lib doctor
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Attendu : le même nombre de tests `doctor` qu'avant, tous verts.

- [ ] **Step 7 : commit**

```bash
git add -A
git commit -m "perf(doctor): lit le manifeste et la configuration une seule fois"
```

---

### Task 4 : le nom du paquet vient du document déjà analysé (tâche 36)

**Files:**
- Modify: `crates/rbs-cli/src/metadata.rs` (`Metadata`, `read`, `package_name`)
- Modify: `crates/rbs-cli/src/generate/command.rs` (l. ~284 et le test l. ~1792)
- Modify: `crates/rbs-cli/src/add/mod.rs` (l. ~212)
- Modify: `crates/rbs-cli/src/upgrade.rs` (l. ~194)

**Interfaces:**
- Produit :
  - champ `pub package: Option<String>` sur `metadata::Metadata`
  - `impl Metadata { pub fn package_name(&self, cargo_toml: &Path) -> Result<String, Error> }`
- Retire : la fonction libre `metadata::package_name`.

- [ ] **Step 1 : porter le nom du paquet sur `Metadata`**

```rust
pub struct Metadata {
    …
    /// Nom du paquet que le manifeste déclare.
    ///
    /// Optionnel parce que la faute ne se lève qu'à l'usage : `upgrade` n'a besoin du nom
    /// que pour recréer un guide absent, et un `[package] name` illisible ne doit pas
    /// faire échouer une mise à niveau qui s'en passe.
    pub package: Option<String>,
}
```

Dans `read`, à côté des quatre autres champs :

```rust
        package: document
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(Item::as_str)
            .map(str::to_owned),
```

- [ ] **Step 2 : déplacer `package_name` en méthode**

```rust
impl Metadata {
    /// Le nom du paquet, ou la faute qui le nomme absent.
    ///
    /// C'est le nom du binaire du projet, et la racine de celui de sa base : les
    /// fragments de feature en ont besoin là où `rbs new` disposait encore du nom saisi.
    /// `cargo_toml` ne sert qu'à nommer le fichier fautif : rien n'est relu ici.
    pub fn package_name(&self, cargo_toml: &Path) -> Result<String, Error> {
        self.package.clone().ok_or_else(|| Error::Field {
            path: name_of(cargo_toml),
            key: "name",
        })
    }
}
```

La fonction libre `pub fn package_name(cargo_toml: &Path)` est supprimée.

- [ ] **Step 3 : les trois appelants passent par les métadonnées déjà lues**

`generate/command.rs` :

```rust
    let crate_name = root
        .join("src/lib.rs")
        .exists()
        .then(|| {
            metadonnees
                .package_name(&root.join("Cargo.toml"))
                .map(|name| name.replace('-', "_"))
        })
        .transpose()?;
```

`add/mod.rs` :

```rust
    let nom_projet = metadonnees.package_name(&root.join("Cargo.toml"))?;
```

`upgrade.rs`, dans la branche qui recrée le guide — le commentaire dit maintenant vrai :

```rust
        // La faute ne se lève qu'ici : le nom du paquet ne sert qu'au titre du document
        // recréé, et un `[package] name` illisible n'a pas à faire échouer une mise à
        // niveau qui n'en a pas besoin.
        let package = metadonnees.package_name(&root.join("Cargo.toml"))?;
```

Le test `generate/command.rs:~1792` devient :

```rust
        let crate_name = crate::metadata::read(&root.join("Cargo.toml"))
            .expect("métadonnées lisibles")
            .package_name(&root.join("Cargo.toml"))
            .expect("le manifeste nomme son paquet")
            .replace('-', "_");
```

- [ ] **Step 4 : le commentaire « une seule lecture » devient vrai**

Dans `generate/command.rs:~226` et `add/mod.rs:~170`, le commentaire est laissé tel quel :
il énonce le *pourquoi* (l'erreur se propage une fois, `agents::refresh` reçoit ces
métadonnées) et il est désormais exact. Vérifier par lecture qu'il ne reste aucun
`metadata::read` ni `metadata::package_name` dans ces deux fonctions :

```bash
grep -n "metadata::read\|package_name" crates/rbs-cli/src/generate/command.rs crates/rbs-cli/src/add/mod.rs
```

- [ ] **Step 5 : vérifier**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

- [ ] **Step 6 : commit**

```bash
git add -A
git commit -m "perf(cli): tire le nom du paquet du manifeste déjà analysé"
```

---

### Task 5 : une fixture de projet partagée (tâche 37)

**Files:**
- Create: `crates/rbs-cli/src/fixtures.rs`
- Modify: `crates/rbs-cli/src/lib.rs` (déclaration du module)
- Modify: les 18 modules de tests portant une copie : `add/mod.rs`, `agents.rs`,
  `dev/mod.rs`, `doctor/{agents,anchors,auth,base,env,jobs,mail,mod,redis,storage,versions}.rs`,
  `generate/command.rs` (×2), `migrate/fresh.rs`, `migrate/mod.rs`, `seed.rs`,
  `upgrade.rs`, `lib.rs`

**Interfaces:**
- Produit :

```rust
pub(crate) struct Project;
impl Project {
    pub(crate) fn new() -> Self;
    pub(crate) fn database(self, database: Database) -> Self;
    pub(crate) fn url(self, url: &str) -> Self;
    pub(crate) fn features(self, features: &[&str]) -> Self;
    pub(crate) fn core_path(self, core_path: Option<PathBuf>) -> Self;
    pub(crate) fn create(self) -> (TempDir, PathBuf);
}
pub(crate) fn project() -> (TempDir, PathBuf);
```

- [ ] **Step 1 : écrire `fixtures.rs`**

```rust
//! Le projet neuf sur lequel presque tous les tests de la crate s'appuient.
//!
//! Dix-huit modules en portaient leur copie : une option ajoutée à `rbs new` demandait de
//! les visiter tous, et deux copies avaient déjà divergé de ce que `rbs new` produit.
//!
//! Le constructeur est en chaîne pour que chaque appelant ne nomme que ce qui le
//! concerne : un test qui choisit son moteur n'a pas à répéter l'URL, et un test qui
//! choisit son URL n'a pas à répéter le moteur.

use std::path::PathBuf;

use tempfile::TempDir;

use crate::database::Database;
use crate::lang::Lang;
use crate::new;

/// Un projet à créer, et ce qui le distingue du projet par défaut.
pub(crate) struct Project {
    options: new::Options,
}

impl Project {
    /// Le projet que la plupart des tests attendent : `demo-api`, PostgreSQL, sans
    /// feature.
    pub(crate) fn new() -> Self {
        Self {
            options: new::Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Database::default(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
                lang: Lang::Fr,
            },
        }
    }

    /// Le moteur du projet. L'URL ne suit pas : les deux se choisissent séparément.
    pub(crate) fn database(mut self, database: Database) -> Self {
        self.options.database = database;
        self
    }

    /// L'URL que le `.env` du projet portera.
    pub(crate) fn url(mut self, url: &str) -> Self {
        self.options.database_url = url.to_string();
        self
    }

    /// Les features à installer à la création.
    pub(crate) fn features(mut self, features: &[&str]) -> Self {
        self.options.features = features.iter().map(|f| (*f).to_string()).collect();
        self
    }

    /// Le chemin du noyau, quand le test a besoin d'une dépendance locale.
    pub(crate) fn core_path(mut self, core_path: Option<PathBuf>) -> Self {
        self.options.core_path = core_path;
        self
    }

    /// Crée le projet dans un répertoire temporaire, rendu avec lui : le laisser tomber
    /// efface le projet.
    pub(crate) fn create(self) -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = new::create(&self.options, parent.path()).expect("le projet doit se créer");

        (parent, project.root)
    }
}

/// Le projet par défaut, sans rien à préciser.
pub(crate) fn project() -> (TempDir, PathBuf) {
    Project::new().create()
}
```

Dans `lib.rs`, en gardant l'ordre alphabétique des déclarations :

```rust
#[cfg(test)]
mod fixtures;
```

- [ ] **Step 2 : compiler la fixture seule**

```bash
cargo test -p rbs-cli --lib fixtures
```

Attendu : compile, 0 test — la fixture n'a pas de test à elle, ce sont ses dix-huit
appelants qui l'exercent.

- [ ] **Step 3 : migrer les fixtures sans variante**

Les dix modules dont la copie est identique au caractère — `doctor/agents.rs`,
`doctor/anchors.rs`, `doctor/env.rs`, `doctor/versions.rs`, `generate/command.rs`
(`project`), `migrate/fresh.rs`, `migrate/mod.rs`, `seed.rs`, `lib.rs` (`projet`) —
perdent leur `fn project()` et importent `use crate::fixtures::project;`. Quand le module
a besoin du nom local (`projet` dans `lib.rs`), l'importer sous ce nom :
`use crate::fixtures::project as projet;`.

- [ ] **Step 4 : migrer les fixtures à variante**

Chaque module garde sa fonction et son doc-commentaire, et n'en délègue que la création :

```rust
// add/mod.rs
fn project_with(database: Database, database_url: &str) -> (TempDir, PathBuf) {
    Project::new().database(database).url(database_url).create()
}

// dev/mod.rs
fn project_on(database: Database, features: &[&str], url: &str) -> (TempDir, PathBuf) {
    Project::new().database(database).features(features).url(url).create()
}

// doctor/base.rs
fn project_on(database: Database, url: &str) -> (TempDir, PathBuf) {
    Project::new().database(database).url(url).create()
}

// agents.rs — la signature locale prend un Vec<String>, elle ne change pas
fn project(features: Vec<String>) -> (TempDir, PathBuf) {
    let noms: Vec<&str> = features.iter().map(String::as_str).collect();
    Project::new().features(&noms).create()
}

// upgrade.rs
fn project(core_path: Option<PathBuf>) -> (TempDir, PathBuf) {
    Project::new().core_path(core_path).create()
}

// generate/command.rs — le second projet, l'authentification installée
fn project_with_auth() -> (TempDir, PathBuf) {
    Project::new().features(&["auth"]).create()
}

// doctor/{auth,mail,redis,jobs,storage}.rs — le corps qui pose les fichiers reste
fn project_with_redis() -> (TempDir, PathBuf) {
    let (parent, root) = crate::fixtures::project();
    …   // inchangé
    (parent, root)
}

// doctor/mod.rs — la réécriture du manifeste reste, seule la création délègue
pub(super) fn project(features: &[&str]) -> (TempDir, PathBuf) {
    let (parent, root) = crate::fixtures::project();
    …   // la réécriture de `features = ["health"]` reste inchangée
    (parent, root)
}
```

**Ne changer aucune valeur au passage.** En particulier : `add::project_on` continue de
dériver son URL du moteur (`database.default_url("demo_api")`), `doctor/base::project`
continue de passer l'URL par défaut du module, et `doctor/mod::project` continue de créer
le projet **sans** feature puis de réécrire le manifeste — un test qui change de fixture
change de sens.

- [ ] **Step 5 : vérifier**

```bash
cargo test --workspace
```

Attendu : le nombre de tests est **exactement** celui de la Task 0.

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

- [ ] **Step 6 : commit**

```bash
git add -A
git commit -m "test(cli): rassemble la fixture du projet neuf en un seul module"
```

---

### Task 6 : un module d'erreurs communes (tâche 38)

**Files:**
- Create: `crates/rbs-cli/src/errors.rs`
- Modify: `crates/rbs-cli/src/lib.rs` (déclaration du module, 4 `map_err`)
- Modify: `crates/rbs-cli/src/{metadata,dotenv,seed,upgrade}.rs`, `plan/mod.rs`,
  `add/mod.rs`, `generate/command.rs`, `migrate/mod.rs`, `dev/mod.rs`, `doctor/mod.rs`

**Interfaces:**
- Produit :

```rust
pub(crate) struct Acces { pub path: String, pub source: io::Error }
impl Acces { pub(crate) fn new(path: &Path, source: io::Error) -> Self }

pub(crate) struct WorkingTreeSale { pub files: String }

pub(crate) const PAS_UN_PROJET: &str = "cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici";

macro_rules! depuis_la_racine { ($erreur:ty) => { … } }
```

- [ ] **Step 1 : écrire `errors.rs`**

```rust
//! Les fautes que plusieurs commandes rendent au même mot près.
//!
//! Rust ne partage pas une variante entre deux énumérations : chaque commande garde donc
//! la sienne, et n'en porte plus le texte ni le constructeur. Ce qui diffère d'une
//! commande à l'autre — le message qui nomme `rbs add` ou `rbs generate` — reste chez
//! elle : deux textes voisins restent deux textes.

use std::io;
use std::path::Path;

/// Un fichier du projet ou d'une template n'a pu être lu ou écrit.
#[derive(Debug, thiserror::Error)]
#[error("{path} est inaccessible : {source}")]
pub(crate) struct Acces {
    /// Chemin fautif.
    pub path: String,
    /// Cause système.
    pub source: io::Error,
}

impl Acces {
    /// La faute, le chemin rendu tel qu'il s'affiche.
    pub(crate) fn new(path: &Path, source: io::Error) -> Self {
        Self {
            path: path.display().to_string(),
            source,
        }
    }
}

/// Le projet porte des modifications non commitées, qu'une commande rendrait
/// indiscernables des siennes.
#[derive(Debug, thiserror::Error)]
#[error("le working tree n'est pas propre : {files} — commitez, ou relancez avec --force")]
pub(crate) struct WorkingTreeSale {
    /// Fichiers suivis modifiés, énumérés.
    pub files: String,
}

/// Le message des commandes qui ne nomment pas la commande fautive.
pub(crate) const PAS_UN_PROJET: &str =
    "cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici";

/// Déclare, pour une énumération portant `PasUnProjet` et `Metadata`, la conversion
/// depuis la faute de remontée : une faute du manifeste se nomme, seule son absence vaut
/// « pas un projet rbs ».
macro_rules! depuis_la_racine {
    ($erreur:ty) => {
        impl From<$crate::metadata::RootError> for $erreur {
            fn from(faute: $crate::metadata::RootError) -> Self {
                match faute {
                    $crate::metadata::RootError::Absent => Self::PasUnProjet,
                    $crate::metadata::RootError::Illisible(faute) => Self::Metadata(faute),
                }
            }
        }
    };
}

pub(crate) use depuis_la_racine;
```

Dans `lib.rs` : `mod errors;` (dans l'ordre alphabétique, entre `dotenv` et `generate`).

- [ ] **Step 2 : adopter `Acces` dans les sept énumérations**

Pour chacune de `metadata::Error`, `dotenv::Error`, `plan::Error`, `seed::Error`,
`upgrade::Error`, `add::Error`, `generate::command::Error` :

```rust
    /// Un fichier du projet n'a pu être lu ou écrit.
    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),
```

Le doc-commentaire propre à chaque module (« Un fichier du projet ou une template n'a pu
être lu » pour `add`) est conservé.

Chaque construction `Error::Acces { path: …, source }` devient
`crate::errors::Acces::new(path, source).into()` — ou, sous un `map_err`,
`.map_err(|source| crate::errors::Acces::new(path, source).into())`. Là où le chemin est
déjà une chaîne (`lib.rs`, `path: ".".to_string()`), passer `Path::new(".")` : `Path::new(".").display()`
rend exactement `.`.

Les deux `fn access(path, source) -> Error` de `generate/command.rs` et `add/mod.rs` sont
supprimées ; leurs appelants passent par `errors::Acces::new(...).into()`.

- [ ] **Step 3 : adapter les trois filtrages**

```rust
// metadata.rs:41
    Err(Error::Acces(faute)) if faute.source.kind() == std::io::ErrorKind::NotFound => {

// add/mod.rs:226
    Err(migrate::Error::Env(dotenv::Error::Acces(faute)))
        if faute.source.kind() == io::ErrorKind::NotFound =>

// plan/mod.rs:666 (test)
    assert!(matches!(error, Error::Acces(_)), "{error:?}");
```

- [ ] **Step 4 : adopter `WorkingTreeSale` dans les trois énumérations**

```rust
    /// Le projet porte des modifications non commitées, qu'une génération rendrait
    /// indiscernables des siennes.
    #[error(transparent)]
    WorkingTreeSale(#[from] crate::errors::WorkingTreeSale),
```

Les trois constructions `Error::WorkingTreeSale { files: git::enumerate(&modifies) }` sont
remplacées à la Task 7, qui pose le garde ; jusque-là, écrire
`crate::errors::WorkingTreeSale { files: git::enumerate(&modifies) }.into()`.

Vérifier les filtrages de tests :

```bash
grep -rn "WorkingTreeSale" crates/rbs-cli/src/
```

et convertir chaque `Error::WorkingTreeSale { .. }` en `Error::WorkingTreeSale(_)`, chaque
`Error::WorkingTreeSale { files }` en `Error::WorkingTreeSale(faute)` puis `faute.files`.

- [ ] **Step 5 : mettre le message de `PasUnProjet` en commun là où il est identique**

Dans `seed.rs`, `doctor/mod.rs`, `migrate/mod.rs`, `dev/mod.rs` seulement :

```rust
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error("{}", crate::errors::PAS_UN_PROJET)]
    PasUnProjet,
```

**Ne pas toucher** aux trois messages de `add`, `generate` et `upgrade`, qui nomment leur
commande : les aligner changerait le texte rendu à l'utilisateur.

- [ ] **Step 6 : remplacer les sept `impl From<metadata::RootError>`**

Dans chacun des sept modules, le bloc de huit lignes devient :

```rust
/// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);
```

- [ ] **Step 7 : vérifier que pas un message n'a bougé**

```bash
cargo test --workspace
```

Les tests d'intégration `assert_cmd` comparent des sorties entières : ce sont eux qui
prouvent que le texte est intact. Lancer aussi :

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

- [ ] **Step 8 : commit**

```bash
git add -A
git commit -m "refactor(cli): rassemble les fautes communes aux commandes en un module"
```

---

### Task 7 : le préambule et le rituel des commandes (tâche 39)

**Files:**
- Modify: `crates/rbs-cli/src/metadata.rs` (ajout de `Cible` et `cible`)
- Modify: `crates/rbs-cli/src/git.rs` (ajout de `garde`)
- Modify: `crates/rbs-cli/src/generate/command.rs` (`plan_for`), `add/mod.rs` (`plan_for`),
  `upgrade.rs` (`plan_for_with`)
- Modify: `crates/rbs-cli/src/lib.rs` (`add_in`, `generate_in`, `upgrade_in`)

**Interfaces:**
- Consomme : `errors::Acces`, `errors::WorkingTreeSale` (Task 6).
- Produit :

```rust
// metadata.rs
pub struct Cible { pub root: PathBuf, pub metadonnees: Metadata }
pub fn cible<E>(directory: &Path) -> Result<Cible, E>
where E: From<crate::errors::Acces> + From<RootError> + From<Error>;

// git.rs
pub(crate) fn garde(root: &Path) -> Result<(), crate::errors::WorkingTreeSale>;

// lib.rs
fn appliquer(plan: &plan::Plan, force: bool, dry_run: bool) -> Result<bool, plan::application::Error>;
```

- [ ] **Step 1 : écrire `cible` dans `metadata.rs`**

```rust
/// Le projet visé depuis un répertoire de lancement, et son manifeste.
pub struct Cible {
    /// Racine du projet.
    pub root: PathBuf,
    /// Métadonnées rbs, lues une seule fois.
    pub metadonnees: Metadata,
}

/// Désigne le projet que `directory` habite, et lit son manifeste.
///
/// Le préambule des trois commandes qui modifient un projet existant. Générique sur
/// l'erreur de l'appelant : chacune garde son énumération et ses messages, qui nomment la
/// commande — c'est `?` qui convertit, et rien du texte rendu ne dépend d'ici.
pub fn cible<E>(directory: &Path) -> Result<Cible, E>
where
    E: From<crate::errors::Acces> + From<RootError> + From<Error>,
{
    let start = directory
        .canonicalize()
        .map_err(|source| crate::errors::Acces::new(directory, source))?;
    let root = project_root(&start)?;
    let metadonnees = read(&root.join("Cargo.toml"))?;

    Ok(Cible { root, metadonnees })
}
```

- [ ] **Step 2 : écrire `garde` dans `git.rs`**

```rust
/// Refuse d'écrire dans un working tree qui porte des modifications non commitées.
///
/// Ce qu'une commande écrirait s'y mêlerait à ce que le développeur n'a pas encore
/// enregistré, et `git diff` ne les distinguerait plus.
pub(crate) fn garde(root: &Path) -> Result<(), crate::errors::WorkingTreeSale> {
    let modifies = modified_files(root);

    if modifies.is_empty() {
        return Ok(());
    }

    Err(crate::errors::WorkingTreeSale {
        files: enumerate(&modifies),
    })
}
```

- [ ] **Step 3 : `generate::command::plan_for` ouvre sur le préambule**

```rust
pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error> {
    // Une seule lecture pour toute la fonction : son erreur se propage par `?` plutôt que
    // d'être ré-tentée, et `agents::refresh` reçoit ces métadonnées au lieu de les relire
    // elle-même.
    let metadata::Cible { root, metadonnees } = metadata::cible(&options.directory)?;

    if !options.force {
        git::garde(&root)?;
    }
    …
```

- [ ] **Step 4 : `add::plan_for` de même**

```rust
pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error> {
    // Une seule lecture pour toute la fonction : son erreur se propage par `?` plutôt
    // que d'être ré-tentée, et `agents::refresh` reçoit ces métadonnées au lieu de les
    // relire elle-même.
    let metadata::Cible { root, metadonnees } = metadata::cible(&options.directory)?;

    // L'idempotence se juge sur `[package.metadata.rbs]` … (bloc inchangé)

    if !options.force {
        git::garde(&root)?;
    }
    …
```

L'ordre est **conservé** : `add` juge son idempotence entre la lecture et le garde, un
projet qui porte déjà la feature ne se voyant pas refuser pour un working tree sale.

- [ ] **Step 5 : `upgrade::plan_for_with` de même**

```rust
    let metadata::Cible { root, metadonnees } = metadata::cible(&options.directory)?;
    let depuis = metadonnees.version.clone();
    …
    if !deja_a_jour && !options.force {
        git::garde(&root)?;
    }
```

`upgrade` garde sa condition à deux termes : il ne réclame un working tree propre que
s'il a quelque chose à écrire.

- [ ] **Step 6 : le rituel de `lib.rs`**

```rust
/// Applique le plan, ou dit que `--dry-run` l'a laissé sur le papier.
///
/// Rend `false` quand rien n'a été écrit : l'appelant sort alors sans annoncer une
/// écriture qui n'a pas eu lieu.
fn appliquer(
    plan: &plan::Plan,
    force: bool,
    dry_run: bool,
) -> Result<bool, plan::application::Error> {
    if dry_run {
        ui::info("\n  rien n'a été écrit (--dry-run)");
        return Ok(false);
    }

    plan::application::apply(plan, force)?;

    Ok(true)
}
```

Les trois blocs de `add_in`, `generate_in` et `upgrade_in` deviennent :

```rust
    if !appliquer(&planned.plan, force, dry_run)? {
        return Ok(());
    }
```

- [ ] **Step 7 : vérifier**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Les tests `add_dry_run_leaves_the_project_untouched_while_the_real_run_writes` et
`upgrade_dry_run_leaves_the_project_untouched_while_the_real_run_writes` de `lib.rs`
comparent le projet octet à octet avant et après : ce sont eux qui prouvent que le rituel
n'a pas changé de sens.

- [ ] **Step 8 : commit**

```bash
git add -A
git commit -m "refactor(cli): partage le préambule et le rituel des commandes qui écrivent"
```

---

### Task 8 : preuves finales

- [ ] **Step 1 : la suite complète, et son compte**

```bash
cargo test --workspace 2>&1 | grep -E "^test result:"
```

Attendu : la somme des tests est **égale ou supérieure** à celle de la Task 0.

- [ ] **Step 2 : les deux bloquants de CI**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

- [ ] **Step 3 : `examples/` n'a pas bougé**

```bash
git status --porcelain examples/
```

Attendu : aucune ligne.

- [ ] **Step 4 : la suite Docker, seule preuve de bout en bout**

```bash
cargo test --workspace --no-fail-fast -- --ignored
```

`--no-fail-fast` est obligatoire : sans lui, la suite s'arrête au premier binaire et
masque les échecs des suivants. Plusieurs minutes sont attendues. Lire la sortie réelle
avant toute affirmation de succès.
