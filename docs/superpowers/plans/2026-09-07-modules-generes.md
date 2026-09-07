# Plan d'implémentation — les modules de `rbs add` sous `src/modules/`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ranger sous `src/modules/` les dix répertoires que `rbs add` dépose dans `src/`, en n'exceptant qu'`auth`, pour que `src/` ne montre plus que le code du développeur.

**Architecture:** Une quatorzième ancre, `<rbs:modules>`, vit dans `src/modules/mod.rs` ; `add` crée ce fichier à la volée au premier fragment qui la vise et inscrit `pub mod modules;` dans `<rbs:features>`. Les manifestes des fragments disent leur nouvelle destination en toutes lettres — aucun préfixe implicite. Les projets existants gardent leur disposition ; `doctor` signale un mélange sans jamais y toucher.

**Tech Stack:** Rust 2024, `minijinja` (délimiteurs alternatifs), `include_dir`, `assert_cmd`, `testcontainers` (PostgreSQL).

**Spec:** `docs/superpowers/specs/2026-09-07-modules-generes-design.md`

## Contraintes globales

- **Branche** : `feat/modules-generes`, déjà créée, portant le commit de la spec. Jamais sur `main`.
- **Commits** : Conventional Commits, sujet en français à l'impératif, sans majuscule ni point final. Aucun identifiant de tâche, aucun renvoi à ce plan ou à un backlog, **jamais de ligne `Co-Authored-By` ni `Claude-Session`**. Corps portant le *pourquoi*, puis un intertitre `Vérifications :` avec les commandes lancées et leur résultat réel.
- **Le dossier s'appelle `modules`**, le module Rust `crate::modules`, l'ancre `<rbs:modules>`. Écrit en toutes lettres partout ; aucune constante ne le calcule à partir d'autre chose.
- **`auth` ne bouge pas.** `src/auth/`, `crate::auth`, `anchor = "features"` : le fragment `auth` est hors du périmètre de chaque tâche.
- **`ci` et `docker` ne bougent pas.** Ils n'écrivent rien dans `src/`.
- **Un commentaire explique le *pourquoi*, jamais le *quoi*.** Noms de tests en anglais, commentaires et messages en français.
- **Les blancs de minijinja sont un piège** : `-%}` mange l'indentation, et un blanc perdu n'est vu que par `integration_examples`. Ne pas reformater une template ; ne changer que les chemins.
- **Bloquant en CI** à chaque commit : `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`.
- **Toolchain** : lancer `rustup update` avant la passe finale — un clippy vert en local ne dit rien de la version que prend `@stable` en CI.

## Structure des fichiers

| Fichier | Responsabilité après ce plan |
|---|---|
| `crates/rbs-cli/src/anchors.rs` | Déclare `MODULES` ; `ANCRES` passe à 14 ; `JOBS.file` suit son fragment |
| `crates/rbs-cli/src/add/installation.rs` | Ouvre le point de montage quand un fragment vise `modules` |
| `crates/rbs-cli/templates/features/*/feature.toml` | Destinations et contenus d'ancres, en toutes lettres |
| `crates/rbs-cli/templates/features/{scheduler,webhooks,rate-limit}/*.jinja` | Chemins de crate du code rendu |
| `crates/rbs-cli/templates/feature/service.rs.jinja` | Le gabarit de `generate crud` importe `crate::modules::storage` |
| `crates/rbs-cli/src/doctor/agents.rs` | `HORS_FEATURES` accueille `modules` |
| `crates/rbs-cli/src/doctor/disposition.rs` | **Neuf** — signale une disposition mixte, sans correctif |
| `crates/rbs-cli/src/doctor/mod.rs` | Enregistre le contrôle de disposition |
| `examples/{blog-auth,file-drop,newsletter-queue,hello-crud}/` | Régénérés par diff |
| `docs/docs/**`, `docs/i18n/fr/**` | 9 pages × 2 langues, plus un paragraphe neuf sur la disposition |
| `CHANGELOG.md`, `CHANGELOG.fr.md`, `Cargo.toml` | Note de version, 1.2.0 → 1.3.0 |

---

### Task 1 : l'ancre `modules` au registre

**Files:**
- Modify: `crates/rbs-cli/src/anchors.rs` (constantes du registre, ~ligne 289)
- Test: `crates/rbs-cli/src/anchors.rs` (module `tests`), `crates/rbs-cli/src/doctor/anchors.rs` (module `tests`)

**Interfaces:**
- Produces: `anchors::MODULES` — une `Anchor` de nom `"modules"`, fichier `"src/modules/mod.rs"`, `sorted: true`, `optional: true`. `anchors::ANCRES` devient `[Anchor; 14]`.

- [ ] **Step 1 : écrire le test qui échoue**

Dans le module `tests` de `crates/rbs-cli/src/anchors.rs` :

```rust
/// L'ancre des modules vit dans un fichier que le squelette ne pose pas : la déclarer
/// obligatoire ferait passer pour incomplet tout projet sans fragment.
#[test]
fn the_modules_anchor_is_optional_and_sorted() {
    let modules = ANCRES
        .into_iter()
        .find(|anchor| anchor.name == "modules")
        .expect("le registre porte l'ancre des modules");

    assert_eq!(modules.file, "src/modules/mod.rs");
    assert!(modules.optional, "son fichier n'existe pas sans fragment");
    assert!(modules.sorted, "rustfmt trie les `pub mod` du point de montage");
}
```

- [ ] **Step 2 : lancer le test pour le voir échouer**

Run : `cargo test -p rbs-cli --lib anchors::tests::the_modules_anchor_is_optional_and_sorted`
Expected : FAIL — panique `le registre porte l'ancre des modules`.

- [ ] **Step 3 : déclarer l'ancre**

Dans `crates/rbs-cli/src/anchors.rs`, après la constante `FEATURES` :

```rust
/// Déclaration des modules que `rbs add` installe, dans leur point de montage.
///
/// `src/` ne mêle plus le code du développeur et celui du CLI : les fragments s'y
/// déclarent, et `<rbs:features>` ne reçoit d'eux que le `pub mod modules;` qui ouvre ce
/// fichier. `auth` fait exception et reste une feature comme les siennes.
pub(crate) const MODULES: Anchor = Anchor {
    name: Cow::Borrowed("modules"),
    file: Cow::Borrowed("src/modules/mod.rs"),
    comment: "//",
    // Même raison que `FEATURES` : rustfmt trie les `pub mod`, et un fragment intercalé
    // entre deux autres ferait échouer le `cargo fmt --check` du développeur.
    sorted: true,
    // `add` pose ce fichier au premier fragment qui l'y vise, et pas avant : un projet
    // sans fragment n'a pas de répertoire vide à porter.
    optional: true,
    after: "",
};
```

Puis, dans `ANCRES`, porter le tableau à 14 et y ajouter `MODULES` juste après `FEATURES` :

```rust
pub(crate) const ANCRES: [Anchor; 14] = [
    FEATURES,
    MODULES,
    ROUTES,
    // … le reste inchangé
];
```

- [ ] **Step 4 : lancer le test pour le voir passer**

Run : `cargo test -p rbs-cli --lib anchors::tests::the_modules_anchor_is_optional_and_sorted`
Expected : PASS.

- [ ] **Step 5 : rattraper les deux tests qui comptent les ancres applicables**

Le registre gagne une troisième ancre optionnelle : les compteurs relatifs de `doctor/anchors.rs` glissent d'un cran. Dans `crates/rbs-cli/src/doctor/anchors.rs`, remplacer le commentaire et l'assertion de `a_fresh_project_carries_every_anchor_that_applies_to_it` :

```rust
    /// Un projet frais ne porte pas *toutes* les ancres du registre : `jobs` vit dans
    /// `src/modules/jobs/mod.rs` et `modules` dans `src/modules/mod.rs`, que seul
    /// `rbs add` dépose — contrairement au compose, que `new` écrit déjà. Deux des trois
    /// ancres optionnelles sont donc inapplicables ici.
    #[test]
    fn a_fresh_project_carries_every_anchor_that_applies_to_it() {
        let (_parent, root) = project();

        let check = check(&root);

        assert_eq!(check.state, State::Bon);
        assert!(
            check.detail.contains(&(ANCRES.len() - 2).to_string()),
            "{}",
            check.detail
        );
        assert!(check.remedy.is_none());
    }
```

et, dans `an_optional_anchor_whose_file_is_absent_is_not_missing` :

```rust
        // Le compose retiré à la main, `jobs` et `modules` déjà absents par défaut
        // (v. le test précédent) : les trois ancres optionnelles sont inapplicables.
        assert!(
            check.detail.contains(&(ANCRES.len() - 3).to_string()),
            "ni le compose, ni le registre de la file, ni le point de montage ne comptent \
             parmi les applicables : {}",
            check.detail
        );
```

- [ ] **Step 6 : lancer la suite rapide**

Run : `cargo test -p rbs-cli --lib`
Expected : PASS, aucun test en échec.

- [ ] **Step 7 : clippy et fmt**

Run : `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected : silence.

- [ ] **Step 8 : commit**

```bash
git add crates/rbs-cli/src/anchors.rs crates/rbs-cli/src/doctor/anchors.rs
git commit -F - <<'MSG'
feat(anchors): déclare le point d'insertion des modules installés

Le registre gagne <rbs:modules>, dans src/modules/mod.rs. Elle est
optionnelle — son fichier n'est posé qu'au premier fragment qui l'y vise —
et triée, pour la même raison que <rbs:features> : rustfmt ordonne les
`pub mod`, et un fragment intercalé entre deux autres ferait échouer le
`cargo fmt --check` du projet sur une ligne que personne n'y a écrite.

Aucun fragment ne la vise encore ; ce commit ne change rien à ce qu'un
projet reçoit.

Vérifications :

- cargo test -p rbs-cli --lib : vert
- cargo clippy --workspace --all-targets -- -D warnings : silence
- cargo fmt --all --check : silence
MSG
```

---

### Task 2 : `add` ouvre le point de montage

**Files:**
- Modify: `crates/rbs-cli/src/add/installation.rs` (constantes en tête, fonction `actions`)
- Test: `crates/rbs-cli/src/add/mod.rs` (module `tests`)

**Interfaces:**
- Consumes: `anchors::MODULES` (Task 1), `plan::Builder::{exists, create, insert}`, le helper de test `fragment_options(root, fragments)` qui installe la feature `essai` depuis un `--template-dir`.
- Produces: `installation::actions` crée `src/modules/mod.rs` et insère `pub mod modules;` dans l'ancre `features` dès qu'un manifeste déclare `anchor = "modules"`.

- [ ] **Step 1 : écrire les deux tests qui échouent**

Dans le module `tests` de `crates/rbs-cli/src/add/mod.rs`, à côté des autres tests de fragments jetables :

```rust
/// Écrit dans `fragments` un fragment nommé `nom` qui se déclare dans les modules.
fn fragment_module(fragments: &TempDir, nom: &str) {
    fs::create_dir(fragments.path().join(nom)).expect("le fragment se crée");
    fs::write(
        fragments.path().join(nom).join("feature.toml"),
        format!(
            "[feature]\ndescription = \"{nom}\"\n\n\
             [[anchors]]\nanchor = \"modules\"\ncontent = \"pub mod {nom};\"\n"
        ),
    )
    .expect("le manifeste s'écrit");
}

/// Le squelette ne pose pas `src/modules/mod.rs` : c'est le premier fragment qui s'y
/// déclare qui l'ouvre, et qui inscrit le module dans la bibliothèque du projet.
#[test]
fn the_first_fragment_that_targets_modules_opens_the_mount_point() {
    let (_parent, root) = project();
    let fragments = TempDir::new().expect("répertoire temporaire créable");
    fragment_module(&fragments, "essai");

    run(&fragment_options(&root, &fragments)).expect("l'installation doit aboutir");

    let montage = fs::read_to_string(root.join("src/modules/mod.rs"))
        .expect("le point de montage doit être posé");
    assert!(montage.contains("// <rbs:modules>"), "{montage}");
    assert!(montage.contains("pub mod essai;"), "{montage}");

    let lib = fs::read_to_string(root.join("src/lib.rs")).expect("lib.rs lisible");
    assert!(lib.contains("pub mod modules;"), "{lib}");
    assert!(
        !lib.contains("pub mod essai;"),
        "le fragment se déclare dans le point de montage, pas dans la bibliothèque : {lib}"
    );
}

/// Le second fragment trouve le point de montage ouvert : il s'y ajoute sans redéclarer
/// `pub mod modules;`, qu'un doublon ferait refuser à la compilation.
#[test]
fn a_second_fragment_does_not_declare_the_mount_point_twice() {
    let (_parent, root) = project();
    let fragments = TempDir::new().expect("répertoire temporaire créable");
    fragment_module(&fragments, "essai");
    fragment_module(&fragments, "autre");
    run(&fragment_options(&root, &fragments)).expect("la première installation aboutit");

    let mut options = fragment_options(&root, &fragments);
    options.feature = "autre".to_string();
    options.force = true;
    run(&options).expect("la seconde installation doit aboutir");

    let lib = fs::read_to_string(root.join("src/lib.rs")).expect("lib.rs lisible");
    assert_eq!(
        lib.matches("pub mod modules;").count(),
        1,
        "le point de montage ne se déclare qu'une fois : {lib}"
    );
    let montage = fs::read_to_string(root.join("src/modules/mod.rs"))
        .expect("le point de montage existe");
    assert!(montage.contains("pub mod essai;"), "{montage}");
    assert!(montage.contains("pub mod autre;"), "{montage}");
}
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib add::tests::the_first_fragment_that_targets_modules_opens_the_mount_point add::tests::a_second_fragment_does_not_declare_the_mount_point_twice`
Expected : FAIL — le premier sur « le point de montage doit être posé » (`insert` échoue en amont avec `FichierAbsent` pour `src/modules/mod.rs`).

- [ ] **Step 3 : ouvrir le point de montage**

Dans `crates/rbs-cli/src/add/installation.rs`, après la constante `FICHIER_ENV` :

```rust
/// Le fichier qui déclare les modules installés, et qui porte leur ancre.
const POINT_DE_MONTAGE: &str = "src/modules/mod.rs";

/// Ce que `add` y dépose en l'ouvrant.
const MONTAGE_INITIAL: &str = "\
//! Les modules d'infrastructure que `rbs add` installe.

// <rbs:modules>
// </rbs:modules>
";
```

Puis, toujours dans ce fichier, la fonction qui l'ouvre :

```rust
/// Ouvre `src/modules/` si le fragment s'y déclare et que le projet ne l'a pas encore.
///
/// Le squelette ne pose pas ce fichier : un projet qui n'installe rien n'a pas de
/// répertoire vide à porter. C'est donc le premier fragment qui vise l'ancre `modules`
/// qui l'ouvre, et qui inscrit `pub mod modules;` là où les features se déclarent.
fn ouvre_le_point_de_montage(
    fragment: &Fragment,
    builder: &mut plan::Builder,
) -> Result<(), Error> {
    let s_y_declare = fragment
        .manifest
        .anchors
        .iter()
        .any(|insertion| insertion.anchor == anchors::MODULES.name);

    if !s_y_declare {
        return Ok(());
    }

    if !builder.exists(POINT_DE_MONTAGE)? {
        builder.create(POINT_DE_MONTAGE, MONTAGE_INITIAL)?;
    }

    // Inconditionnelle plutôt que réservée à l'ouverture : l'insertion est idempotente,
    // et un projet dont la ligne a été retirée à la main la retrouve, au lieu de porter
    // un répertoire que rien ne compile.
    let features = anchor(fragment, anchors::FEATURES.name.as_ref(), builder)?;
    builder.insert(features, &["pub mod modules;".to_string()])?;

    Ok(())
}
```

Enfin, dans `actions`, l'appeler juste avant la boucle qui interprète les ancres du manifeste — le point de montage doit exister quand `insert` y écrit :

```rust
    ouvre_le_point_de_montage(fragment, builder)?;

    for insertion in &fragment.manifest.anchors {
        let anchor = anchor(fragment, &insertion.anchor, builder)?;
```

- [ ] **Step 4 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib add::tests::the_first_fragment_that_targets_modules_opens_the_mount_point add::tests::a_second_fragment_does_not_declare_the_mount_point_twice`
Expected : PASS pour les deux.

- [ ] **Step 5 : vérifier qu'aucun fragment réel n'a changé de comportement**

Run : `cargo test -p rbs-cli --lib`
Expected : PASS — aucun manifeste livré ne vise encore `modules`, la suite est donc inchangée.

- [ ] **Step 6 : clippy et fmt**

Run : `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected : silence.

- [ ] **Step 7 : commit**

```bash
git add crates/rbs-cli/src/add/installation.rs crates/rbs-cli/src/add/mod.rs
git commit -F - <<'MSG'
feat(add): ouvre le point de montage des modules à la première pose

Un fragment qui se déclare dans l'ancre `modules` a besoin d'un fichier
qui la porte. Le squelette ne le pose pas — un projet qui n'installe rien
n'a pas de répertoire vide à porter — donc c'est la première installation
qui l'ouvre, et qui inscrit `pub mod modules;` dans la bibliothèque.

L'inscription est rejouée à chaque pose plutôt que réservée à l'ouverture :
elle est idempotente, et un projet dont la ligne a été retirée à la main la
retrouve, au lieu de porter un répertoire que rien ne compile.

Aucun manifeste livré ne vise encore cette ancre.

Vérifications :

- cargo test -p rbs-cli --lib : vert, dont les deux tests neufs sur
  l'ouverture et sur la non-duplication de la déclaration
- cargo clippy --workspace --all-targets -- -D warnings : silence
- cargo fmt --all --check : silence
MSG
```

---

### Task 3 : les sept fragments à manifeste seul

Les sept fragments dont aucun `.jinja` ne cite un chemin de crate : `audit`, `cors`, `jobs`, `mail`, `observability`, `redis`, `storage`. L'ancre `JOBS` suit son fragment dans le même mouvement, sans quoi `webhooks` viserait un fichier disparu.

**Files:**
- Modify: `crates/rbs-cli/templates/features/{audit,cors,jobs,mail,observability,redis,storage}/feature.toml`
- Modify: `crates/rbs-cli/src/anchors.rs` (`JOBS.file`)
- Modify: `crates/rbs-cli/src/lib.rs:483,488` (conseils affichés après installation)
- Test: `crates/rbs-cli/src/add/mod.rs` (module `tests`), `crates/rbs-cli/src/lib.rs` (module `tests`, ligne ~1092)

**Interfaces:**
- Consumes: `anchors::MODULES` (Task 1), l'ouverture du point de montage (Task 2).
- Produces: `rbs add storage` dépose `src/modules/storage/` et `crate::modules::storage::from_config()` ; `rbs add redis` dépose `src/modules/cache/` et `crate::modules::cache::Cache`.

- [ ] **Step 1 : déplacer les destinations et les chemins de crate**

```bash
cd /Users/yacoubakone/dev/rs
for f in audit cors jobs mail observability redis storage; do
  m="crates/rbs-cli/templates/features/$f/feature.toml"
  sed -i '' \
    -e 's|"src/\(audit\|cache\|cors\|jobs\|mail\|observability\|storage\)/|"src/modules/\1/|g' \
    -e 's|`src/\(audit\|cache\|cors\|jobs\|mail\|observability\|storage\)/|`src/modules/\1/|g' \
    -e 's|crate::\(audit\|cache\|cors\|jobs\|mail\|observability\|storage\)::|crate::modules::\1::|g' \
    "$m"
done
```

- [ ] **Step 2 : rediriger leur déclaration vers l'ancre des modules**

Ces sept manifestes déclarent `anchor = "features"` avec un `pub mod <x>;`. Seule cette ligne d'ancre change ; le `content` reste le même.

```bash
for f in audit cors jobs mail observability redis storage; do
  perl -0pi -e 's/anchor  = "features"\ncontent = "pub mod/anchor  = "modules"\ncontent = "pub mod/g' \
    "crates/rbs-cli/templates/features/$f/feature.toml"
done
grep -n 'anchor  = "' crates/rbs-cli/templates/features/{audit,cors,jobs,mail,observability,redis,storage}/feature.toml
```

Expected : aucune occurrence de `"features"` parmi les sept ; sept occurrences de `"modules"`.

- [ ] **Step 3 : faire suivre l'ancre `jobs` à son fragment**

Dans `crates/rbs-cli/src/anchors.rs`, la constante `JOBS` :

```rust
    file: Cow::Borrowed("src/modules/jobs/mod.rs"),
```

- [ ] **Step 4 : corriger les deux conseils qui citent un chemin**

Dans `crates/rbs-cli/src/lib.rs`, autour des lignes 483 et 488 :

```rust
        "jobs" => Some("rbs migrate up, puis inscrivez vos jobs dans src/modules/jobs/mod.rs"),
```

```rust
            "rbs migrate up, puis déclarez vos échéances dans src/modules/scheduler/mod.rs — \
```

et, dans le test de `lib.rs` (ligne ~1092) :

```rust
        assert!(conseil.contains("src/modules/scheduler/mod.rs"), "{conseil}");
```

- [ ] **Step 5 : rattraper les tests unitaires de `add` qui citent ces chemins**

```bash
grep -rn 'src/\(audit\|cache\|cors\|jobs\|mail\|observability\|storage\)/\|crate::\(audit\|cache\|cors\|jobs\|mail\|observability\|storage\)::' \
  crates/rbs-cli/src/add/mod.rs crates/rbs-cli/src/doctor/
```

Chaque occurrence trouvée dans un test ou une assertion prend le préfixe `modules/` — ce sont les chaînes figées qui vérifient qu'une sonde atterrit dans `health_probes` ou qu'un layer atterrit dans `layers`. Ne pas toucher aux occurrences de `doctor/agents.rs`, qui relèvent de la Task 5, ni à celles qui citent `auth`.

- [ ] **Step 6 : lancer la suite rapide**

Run : `cargo test -p rbs-cli --lib`
Expected : PASS.

- [ ] **Step 7 : prouver que le code posé compile**

Le code d'un fragment n'est compilé par aucun test rapide. On le compile ici, sur un projet jetable, avant la passe lente :

```bash
SCRATCH=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/f383d35c-bc6f-4366-8018-7228bde636bb/scratchpad
rm -rf "$SCRATCH/t3" && mkdir -p "$SCRATCH/t3"
cargo run -q -p rbs-cli --bin rbs -- new "$SCRATCH/t3/probe" --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/probe' --lang fr
cd "$SCRATCH/t3/probe"
for f in audit cors jobs mail observability redis storage; do
  git add -A && git commit -q -m "before $f"
  cargo run -q --manifest-path /Users/yacoubakone/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- add "$f"
done
cargo check 2>&1 | tail -20
cd /Users/yacoubakone/dev/rs
```

Expected : `cargo check` termine sans erreur. Vérifier de l'œil que `src/modules/` porte les sept répertoires (`cache` et non `redis`) et que `src/` n'en porte aucun.

- [ ] **Step 8 : clippy et fmt**

Run : `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected : silence.

- [ ] **Step 9 : commit**

```bash
git add crates/rbs-cli/templates/features crates/rbs-cli/src/anchors.rs crates/rbs-cli/src/lib.rs crates/rbs-cli/src/add/mod.rs
git commit -F - <<'MSG'
feat(add): range sept modules installés sous src/modules

audit, cors, jobs, mail, observability, redis et storage se déposent
désormais sous src/modules/, et s'y déclarent. src/ cesse d'y mêler le code
du développeur : ce qu'il ouvre ne montre plus que ses propres features.

Les manifestes disent leur destination en toutes lettres, sans préfixe que
le CLI appliquerait dans son dos : lire un feature.toml continue de dire
exactement où chaque fichier atterrit.

L'ancre du registre des jobs suit son fragment vers
src/modules/jobs/mod.rs, sans quoi webhooks viserait un fichier disparu.

Vérifications :

- cargo test -p rbs-cli --lib : vert
- projet jetable engendré puis pourvu des sept fragments, cargo check :
  compile ; src/modules/ porte les sept répertoires, src/ aucun
- cargo clippy --workspace --all-targets -- -D warnings : silence
- cargo fmt --all --check : silence
MSG
```

---

### Task 4 : les trois fragments à code rendu, et le gabarit du CRUD

`rate-limit`, `scheduler` et `webhooks` citent leurs propres chemins dans le code qu'ils rendent. S'y ajoute `templates/feature/service.rs.jinja`, le gabarit de `generate crud`, qui importe `crate::storage`.

**Files:**
- Modify: `crates/rbs-cli/templates/features/{rate-limit,scheduler,webhooks}/feature.toml`
- Modify: `crates/rbs-cli/templates/features/scheduler/{mod,tests}.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/webhooks/{delivery,service,tests}.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/rate-limit/counter.rs.jinja`
- Modify: `crates/rbs-cli/templates/feature/service.rs.jinja:54`
- Test: `crates/rbs-cli/src/add/mod.rs`, `crates/rbs-cli/src/generate/command.rs:1817`

**Interfaces:**
- Consumes: tout ce que produit la Task 3.
- Produces: `crate::modules::{rate_limit,scheduler,webhooks}` ; le service d'une feature engendrée par `generate crud --with-upload` importe `crate::modules::storage::{Storage, StorageError}`.

- [ ] **Step 1 : déplacer manifestes et code rendu**

```bash
cd /Users/yacoubakone/dev/rs
for f in rate-limit scheduler webhooks; do
  find "crates/rbs-cli/templates/features/$f" -type f \
    \( -name '*.toml' -o -name '*.jinja' \) -exec sed -i '' \
    -e 's|"src/\(rate_limit\|scheduler\|webhooks\)/|"src/modules/\1/|g' \
    -e 's|`src/\(rate_limit\|scheduler\|webhooks\)/|`src/modules/\1/|g' \
    -e 's|crate::\(rate_limit\|scheduler\|webhooks\|jobs\|storage\|mail\|cache\)::|crate::modules::\1::|g' \
    {} +
done
sed -i '' 's|use crate::storage::{Storage, StorageError};|use crate::modules::storage::{Storage, StorageError};|' \
  crates/rbs-cli/templates/feature/service.rs.jinja
```

- [ ] **Step 2 : rediriger leur déclaration vers l'ancre des modules**

```bash
for f in rate-limit scheduler webhooks; do
  perl -0pi -e 's/anchor  = "features"\ncontent = "pub mod/anchor  = "modules"\ncontent = "pub mod/g' \
    "crates/rbs-cli/templates/features/$f/feature.toml"
done
grep -rn 'anchor  = "features"' crates/rbs-cli/templates/features/
```

Expected : une seule occurrence restante, dans `auth/feature.toml`.

- [ ] **Step 3 : vérifier qu'aucun chemin n'a été préfixé deux fois**

```bash
grep -rn 'crate::modules::modules\|src/modules/modules' crates/rbs-cli/templates/ || echo "aucun doublon"
grep -rn 'crate::\(rate_limit\|scheduler\|webhooks\|jobs\|storage\|mail\|cache\|audit\|cors\|observability\)::' \
  crates/rbs-cli/templates/ | grep -v 'crate::modules::' || echo "aucun chemin resté à plat"
```

Expected : les deux lignes de repli s'affichent.

- [ ] **Step 4 : rattraper les tests unitaires qui citent ces chemins**

Dans `crates/rbs-cli/src/generate/command.rs:1817` :

```rust
            main.contains("crate::modules::jobs::worker::spawn(state.clone());"),
```

Puis les occurrences restantes dans `crates/rbs-cli/src/add/mod.rs` :

```bash
grep -n 'src/\(rate_limit\|scheduler\|webhooks\)/\|crate::\(rate_limit\|scheduler\|webhooks\)::' \
  crates/rbs-cli/src/add/mod.rs
```

Chacune prend le préfixe `modules/` ou `modules::`.

- [ ] **Step 5 : lancer la suite rapide**

Run : `cargo test -p rbs-cli --lib`
Expected : PASS.

- [ ] **Step 6 : prouver que le code posé compile**

`webhooks` exige `jobs` ; `scheduler` pose une migration. On installe la totalité et on ajoute une feature CRUD avec dépôt de fichier, seule à exercer le gabarit corrigé :

```bash
SCRATCH=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/f383d35c-bc6f-4366-8018-7228bde636bb/scratchpad
rm -rf "$SCRATCH/t4" && mkdir -p "$SCRATCH/t4"
cargo run -q -p rbs-cli --bin rbs -- new "$SCRATCH/t4/probe" --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/probe' --lang fr
cd "$SCRATCH/t4/probe"
for f in redis storage mail jobs rate-limit scheduler webhooks observability audit cors; do
  git add -A && git commit -q -m "before $f"
  cargo run -q --manifest-path /Users/yacoubakone/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- add "$f"
done
git add -A && git commit -q -m "before crud"
cargo run -q --manifest-path /Users/yacoubakone/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud uploads --fields 'title:string,size:int' --with-upload --force
cargo check 2>&1 | tail -20
cargo fmt --check 2>&1 | tail -20
cd /Users/yacoubakone/dev/rs
```

Expected : `cargo check` compile, `cargo fmt --check` reste silencieux — un blanc perdu par un `-%}` de minijinja se voit ici avant `integration_examples`.

- [ ] **Step 7 : clippy et fmt du dépôt**

Run : `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected : silence.

- [ ] **Step 8 : commit**

```bash
git add crates/rbs-cli/templates crates/rbs-cli/src
git commit -F - <<'MSG'
feat(add): range les trois derniers modules et le gabarit du CRUD

rate-limit, scheduler et webhooks rejoignent src/modules/, et avec eux les
chemins que leur propre code cite. auth reste seul à la racine de src/ :
il pose l'entité User, que le développeur étend comme les siennes.

Le service d'une feature engendrée avec --with-upload importe désormais
crate::modules::storage : le renommage traverse la frontière dans l'autre
sens, une feature du développeur consommant un module installé. C'est
légitime, et seul le chemin s'allonge.

Vérifications :

- cargo test -p rbs-cli --lib : vert
- projet jetable pourvu des dix fragments puis d'un CRUD --with-upload,
  cargo check : compile ; cargo fmt --check dans le projet : silence
- grep : plus aucun crate::<module> resté à plat dans les templates, et
  aucun chemin préfixé deux fois
- cargo clippy --workspace --all-targets -- -D warnings : silence
MSG
```

---

### Task 5 : `doctor` connaît la nouvelle disposition

**Files:**
- Modify: `crates/rbs-cli/src/doctor/agents.rs:24` (`HORS_FEATURES`), commentaires des lignes 21-23 et 131-133, tests des lignes ~282 et ~291
- Create: `crates/rbs-cli/src/doctor/disposition.rs`
- Modify: `crates/rbs-cli/src/doctor/mod.rs` (déclaration du module, enregistrement du contrôle)
- Test: `crates/rbs-cli/src/doctor/disposition.rs` (module `tests`)

**Interfaces:**
- Consumes: `doctor::Check::{ok, warned}`, `doctor::Projet`.
- Produces: `disposition::{TITRE, check}` — `check(root: &Path) -> Check`.

- [ ] **Step 1 : écrire les tests qui échouent**

Créer `crates/rbs-cli/src/doctor/disposition.rs` avec son seul module de tests, la fonction restant à écrire :

```rust
//! Contrôle de la disposition des modules installés.
//!
//! Un projet engendré avant que `rbs add` ne range ses modules porte les siens à la
//! racine de `src/`. Le CLI ne les déplace pas — réécrire ses `use` reviendrait à
//! toucher à l'AST du développeur — mais un projet qui porte les deux dispositions à la
//! fois a droit de le savoir.

use std::path::Path;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "disposition";

/// Les répertoires que `rbs add` déposait à la racine de `src/`.
///
/// `auth` n'y figure pas : il y vit toujours. `cache` est le répertoire du fragment
/// `redis`, qui ne porte pas son nom.
const ANCIENS: [&str; 10] = [
    "audit",
    "cache",
    "cors",
    "jobs",
    "mail",
    "observability",
    "rate_limit",
    "scheduler",
    "storage",
    "webhooks",
];

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Un projet dont `src/` porte les répertoires nommés, et `modules/` si demandé.
    fn projet(anciens: &[&str], avec_modules: bool) -> TempDir {
        let racine = TempDir::new().expect("répertoire temporaire créable");
        for ancien in anciens {
            fs::create_dir_all(racine.path().join("src").join(ancien))
                .expect("le répertoire se crée");
        }
        if avec_modules {
            fs::create_dir_all(racine.path().join("src/modules/audit"))
                .expect("le répertoire se crée");
        }
        racine
    }

    /// Un projet antérieur est cohérent : il n'a rien à lire à ce sujet tant qu'il n'a
    /// pas commencé à recevoir des modules rangés.
    #[test]
    fn a_project_entirely_in_the_old_layout_is_not_warned() {
        let racine = projet(&["mail", "storage"], false);

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{check:?}");
    }

    #[test]
    fn a_project_carrying_both_layouts_is_warned_and_names_the_directories() {
        let racine = projet(&["mail", "cache"], true);

        let check = check(racine.path());

        assert_eq!(check.state, State::Avertissement, "{check:?}");
        assert!(check.detail.contains("src/mail"), "{}", check.detail);
        assert!(check.detail.contains("src/cache"), "{}", check.detail);
    }

    /// `auth` vit à la racine par décision, non par ancienneté : le signaler ferait
    /// avertir tout projet authentifié.
    #[test]
    fn auth_at_the_root_is_never_a_mixed_layout() {
        let racine = projet(&["auth"], true);

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{check:?}");
    }

    #[test]
    fn a_project_only_in_the_new_layout_is_good() {
        let racine = projet(&[], true);

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{check:?}");
    }
}
```

Déclarer le module dans `crates/rbs-cli/src/doctor/mod.rs`, à sa place alphabétique parmi les autres `mod` :

```rust
mod disposition;
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib doctor::disposition`
Expected : FAIL à la compilation — `cannot find function 'check' in this scope`.

- [ ] **Step 3 : écrire le contrôle**

Dans `crates/rbs-cli/src/doctor/disposition.rs`, entre `ANCIENS` et le module de tests :

```rust
/// Signale un projet qui porte les deux dispositions à la fois.
///
/// Le remède est manuel et le reste : déplacer `src/mail/` demanderait de réécrire les
/// `use crate::mail::` du développeur, c'est-à-dire de toucher à un AST que le CLI
/// s'interdit — et de risquer du code qu'il n'a pas écrit.
pub(crate) fn check(root: &Path) -> Check {
    let src = root.join("src");

    if !src.join("modules").is_dir() {
        return Check::ok(TITRE, "les modules installés sont là où le CLI les pose");
    }

    let restes: Vec<String> = ANCIENS
        .into_iter()
        .filter(|ancien| src.join(ancien).is_dir())
        .map(|ancien| format!("src/{ancien}"))
        .collect();

    if restes.is_empty() {
        return Check::ok(TITRE, "les modules installés sont là où le CLI les pose");
    }

    Check::warned(
        TITRE,
        format!("hors de src/modules/ : {}", restes.join(", ")),
        "posés par une version antérieure ; rbs ne les déplacera pas — déplacez-les et \
         corrigez leurs `use` si vous voulez une disposition unique",
    )
}
```

- [ ] **Step 4 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib doctor::disposition`
Expected : PASS, les quatre.

- [ ] **Step 5 : enregistrer le contrôle au rapport**

Dans `crates/rbs-cli/src/doctor/mod.rs`, fonction `plan`, ajouter à la fin du `vec!` initial — après `base` — un contrôle de plus :

```rust
        Controle {
            titre: disposition::TITRE,
            executer: |projet, _| disposition::check(&projet.root),
        },
```

- [ ] **Step 6 : apprendre à `agents.rs` le nouveau répertoire**

Dans `crates/rbs-cli/src/doctor/agents.rs`, la constante et son commentaire :

```rust
/// Répertoires de `src/` qui ne sont pas des features engendrées.
///
/// `health` est le module du squelette, `seeds` le binaire des données de démonstration,
/// `bin` celui qui imprime le document OpenAPI, et `modules` le point de montage de tout
/// ce que `rbs add` installe. `cache` n'y est plus déposé, mais le parc engendré avant ce
/// rangement en porte un : le compter comme écrit à la main ferait avertir sur chacun de
/// ces projets.
const HORS_FEATURES: [&str; 5] = ["health", "seeds", "cache", "bin", "modules"];
```

Corriger ensuite le commentaire de `declared_without_directory` (ligne ~131), qui cite l'ancienne disposition :

```rust
/// Une feature déclarée dont le répertoire manque, s'il y en a une.
///
/// Un fragment installé ne vit plus sous `src/<nom>/` mais sous `src/modules/`, et n'est
/// donc pas jugé ici : ce contrôle vise les entités engendrées, dont le répertoire porte
/// toujours le nom.
```

Enfin, les deux tests du fichier qui citent un chemin de fragment (lignes ~282 et ~291) : `src/webhooks` y devient `src/modules/webhooks`, et le commentaire sur `redis`/`src/cache` prend la nouvelle forme `src/modules/cache`.

- [ ] **Step 7 : lancer la suite rapide**

Run : `cargo test -p rbs-cli --lib`
Expected : PASS.

- [ ] **Step 8 : voir le rapport sur les deux dispositions**

```bash
SCRATCH=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/f383d35c-bc6f-4366-8018-7228bde636bb/scratchpad
cd "$SCRATCH/t4/probe" && cargo run -q --manifest-path /Users/yacoubakone/dev/rs/Cargo.toml \
  -p rbs-cli --bin rbs -- doctor 2>&1 | tail -25
mkdir -p src/mail && touch src/mail/mod.rs
cargo run -q --manifest-path /Users/yacoubakone/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- doctor 2>&1 | tail -25
rm -rf src/mail
cd /Users/yacoubakone/dev/rs
```

Expected : le premier rapport annonce la disposition en règle et les ancres au complet ; le second porte un avertissement nommant `src/mail`, sans faire échouer le diagnostic.

- [ ] **Step 9 : clippy, fmt et commit**

```bash
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
git add crates/rbs-cli/src/doctor
git commit -F - <<'MSG'
feat(doctor): signale un projet qui porte les deux dispositions

Un projet engendré avant ce rangement garde ses modules à la racine de
src/. Le CLI ne les déplacera pas : réécrire ses `use` reviendrait à
toucher à un AST qu'il s'interdit, et à risquer du code qu'il n'a pas
écrit. Mais un projet qui reçoit un module rangé à côté de modules qui ne
le sont pas a droit de le savoir, et de lire ce qu'il peut en faire.

L'avertissement ne se déclenche que sur le mélange : un projet
entièrement à l'ancienne est cohérent, simplement antérieur, et ne lit
rien à ce sujet. auth n'y compte jamais — il vit à la racine par
décision, non par ancienneté.

L'inventaire d'AGENTS.md apprend au passage le point de montage, faute de
quoi il compterait src/modules/ comme du code écrit à la main.

Vérifications :

- cargo test -p rbs-cli --lib : vert, dont les quatre tests du contrôle
- rbs doctor sur un projet neuf pourvu des dix fragments : disposition en
  règle ; le même avec un src/mail/ ajouté à la main : avertissement
  nommant src/mail, sans échec du diagnostic
- cargo clippy --workspace --all-targets -- -D warnings : silence
MSG
```

---

### Task 6 : les quatre exemples régénérés

**Files:**
- Modify: `examples/{blog-auth,file-drop,newsletter-queue,hello-crud}/**`
- Modify: `examples/README.md`, `examples/README.fr.md`

**Interfaces:**
- Consumes: le CLI tel que les Tasks 1 à 5 le laissent.
- Produces: quatre exemples que `integration_examples` reconnaît octet à octet.

- [ ] **Step 1 : régénérer chaque exemple hors du dépôt**

`file-drop` et `newsletter-queue` portent des éditions manuelles : on ne remplace jamais un exemple par écrasement, on l'amène par diff. Les commandes exactes sont dans `examples/README.fr.md` — les suivre à la lettre, y compris l'ordre d'installation et les retouches que le CLI ne produit pas (suppression du `.git`, `rbs-core` réécrit en `{ path = "../../crates/rbs-core" }`).

```bash
SCRATCH=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/f383d35c-bc6f-4366-8018-7228bde636bb/scratchpad
rm -rf "$SCRATCH/exemples" && mkdir -p "$SCRATCH/exemples"
```

Régénérer les quatre projets dans `$SCRATCH/exemples/`, puis, pour chacun :

```bash
diff -ru examples/<nom> "$SCRATCH/exemples/<nom>" | head -100
```

- [ ] **Step 2 : reporter le diff, et lui seul**

Pour chaque exemple, n'appliquer au dépôt que les différences dues au rangement : chemins de fichiers déplacés, `use`, `pub mod`, la carte des ancres d'`AGENTS.md`. Toute autre différence est soit une édition manuelle qu'il faut conserver, soit une régression à comprendre avant d'aller plus loin — ne pas la reporter en bloc.

```bash
git mv examples/file-drop/src/mail examples/file-drop/src/modules/mail
```

est la forme attendue du déplacement ; `git status` doit montrer des renommages, non des paires suppression/création.

- [ ] **Step 3 : lancer le test de non-dérive**

Run : `cargo test -p rbs-cli --test integration_examples -- --no-fail-fast`
Expected : PASS. Ce test régénère les quatre projets et les compare octet à octet ; il est l'oracle de cette tâche.

- [ ] **Step 4 : mettre à jour les deux README d'exemples**

`examples/README.md` et `examples/README.fr.md` expliquent, pour `file-drop` et `newsletter-queue`, comment le nom de la ressource garde l'ancre `features` triée, et notent que `rbs add redis` écrit `mod cache;`. Ces deux explications visent désormais l'ancre `modules` : les corriger dans les deux langues, dans ce commit.

- [ ] **Step 5 : compiler les quatre exemples**

Run : `cargo check --workspace --all-targets`
Expected : PASS — les exemples sont des membres du workspace.

- [ ] **Step 6 : commit**

```bash
git add examples
git commit -F - <<'MSG'
feat(examples): range les modules des quatre projets sous src/modules

Les exemples sont la seule documentation dont aucune ligne n'est écrite à
la main : un exemple périmé fait mentir les guides qui le citent. Les
quatre suivent donc le rangement, par report du diff entre deux
générations — file-drop et newsletter-queue portent des retouches que le
CLI ne produit pas, et qu'un écrasement perdrait.

Les README d'exemples visent l'ancre modules là où ils visaient features.

Vérifications :

- cargo test -p rbs-cli --test integration_examples -- --no-fail-fast :
  vert, les quatre projets reconnus octet à octet
- cargo check --workspace --all-targets : compile
- git status : des renommages, non des paires suppression/création
MSG
```

---

### Task 7 : les tests d'intégration, et la passe lente

**Files:**
- Modify: `crates/rbs-cli/tests/integration_{webhooks,scheduler,doctor,add,new}.rs`
- Test: la suite complète, Docker démarré

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces: une suite verte, seule preuve que rbs fonctionne.

- [ ] **Step 1 : relever les chaînes figées côté tests**

```bash
grep -rn 'src/\(audit\|cache\|cors\|jobs\|mail\|observability\|rate_limit\|scheduler\|storage\|webhooks\)/\|crate::\(audit\|cache\|cors\|jobs\|mail\|observability\|rate_limit\|scheduler\|storage\|webhooks\)::' \
  crates/rbs-cli/tests/
```

Une chaîne déposée par un fragment est figée des deux côtés de la frontière Docker : chacune de ces occurrences prend le préfixe. Ne pas toucher à celles qui citent `auth`.

- [ ] **Step 2 : vérifier que la suite compile**

Run : `cargo test --workspace --no-run`
Expected : compile sans erreur.

- [ ] **Step 3 : mettre la toolchain à jour**

Run : `rustup update && cargo clippy --workspace --all-targets -- -D warnings`
Expected : silence. Un clippy vert sur une toolchain en retard ne dit rien de celle que prend `@stable` en CI.

- [ ] **Step 4 : lancer la passe lente, Docker démarré**

```bash
SCRATCH=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/f383d35c-bc6f-4366-8018-7228bde636bb/scratchpad
cargo test --workspace --no-fail-fast > "$SCRATCH/passe-lente-t7.txt" 2>&1; tail -40 "$SCRATCH/passe-lente-t7.txt"
```

`--no-fail-fast` est obligatoire : sans lui la suite s'arrête au premier binaire en échec et masque tous les suivants. La sortie va dans un fichier, faute de quoi les chiffres d'une suite longue sont rognés.

Expected : `test result: ok` sur chaque binaire. Consigner les chiffres réels dans le commit.

- [ ] **Step 5 : commit**

```bash
git add crates/rbs-cli/tests
git commit -F - <<'MSG'
test(cli): vise src/modules dans les assertions des tests lents

Une chaîne déposée par un fragment est figée des deux côtés de la
frontière Docker : les tests d'intégration citaient les chemins de la
disposition précédente, et échouaient sur du code pourtant juste.

Vérifications :

- cargo test --workspace --no-fail-fast, Docker démarré : <chiffres réels>
- rustup update puis cargo clippy --workspace --all-targets
  -D warnings : silence
MSG
```

---

### Task 8 : documentation, note de version, 1.3.0

**Files:**
- Modify: `docs/docs/cli/add.md`, `docs/docs/guides/{audit,cache,jobs,mail,observability,scheduler,storage,webhooks}.md`
- Modify: les huit mêmes sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/`, plus `cli/add.md`
- Modify: la page de structure du projet, dans les deux langues
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md`, `Cargo.toml`
- Modify: `CLAUDE.md` (tableau des ancres)

**Interfaces:**
- Consumes: la disposition telle que les tasks précédentes la livrent.

- [ ] **Step 1 : corriger les chemins des neuf pages, dans les deux langues**

```bash
cd /Users/yacoubakone/dev/rs
for d in docs/docs docs/i18n/fr/docusaurus-plugin-content-docs/current; do
  find "$d" -name '*.md' -exec sed -i '' \
    -e 's|src/\(audit\|cache\|cors\|jobs\|mail\|observability\|rate_limit\|scheduler\|storage\|webhooks\)/|src/modules/\1/|g' \
    -e 's|crate::\(audit\|cache\|cors\|jobs\|mail\|observability\|rate_limit\|scheduler\|storage\|webhooks\)::|crate::modules::\1::|g' \
    {} +
done
grep -rn 'src/modules/modules\|crate::modules::modules' docs/docs docs/i18n || echo "aucun doublon"
```

Relire ensuite chaque page modifiée : un `sed` corrige les chemins, pas les phrases qui les entourent. Là où une page dit « le fragment dépose un répertoire à la racine de `src/` », c'est la phrase qu'il faut réécrire.

- [ ] **Step 2 : écrire le paragraphe sur la disposition**

Dans la page de structure du projet, en anglais et en français, un paragraphe qui dit la règle, l'exception et le sort des projets existants. Contenu à porter, en français :

> `src/` porte les features que vous engendrez, `src/modules/` les modules que `rbs add`
> installe : `mail`, `storage`, `jobs`, `cache`… La séparation existe pour qu'ouvrir
> `src/` montre votre code, et lui seul. `auth` fait exception et reste à la racine : il
> pose l'entité `User`, que vous étendez comme n'importe laquelle des vôtres.
>
> Un projet engendré avant rbs 1.3 garde ses modules là où il les a reçus. `rbs` ne les
> déplacera pas — réécrire vos `use` reviendrait à toucher à votre code — mais s'il
> reçoit ensuite un module rangé, `rbs doctor` vous signale que le projet porte les deux
> dispositions.

- [ ] **Step 3 : mettre à jour le tableau des ancres de `CLAUDE.md`**

Ajouter la ligne `| \`// <rbs:modules>\` | \`src/modules/mod.rs\` — le point de montage des fragments, la troisième optionnelle |`, corriger le fichier de `<rbs:jobs>` en `src/modules/jobs/mod.rs`, et porter « treize au total » à « quatorze au total ».

- [ ] **Step 4 : porter la version et écrire la note**

Dans `Cargo.toml` : `version = "1.3.0"`.

Dans `CHANGELOG.md` et `CHANGELOG.fr.md`, une entrée `1.3.0` disant les trois choses qu'un lecteur doit savoir : ce que `rbs add` pose désormais, qu'`auth` fait exception, et que les projets existants gardent leur disposition, `doctor` se bornant à signaler un mélange.

- [ ] **Step 5 : vérifier que le site se construit**

```bash
cd docs && npm run build 2>&1 | tail -20; cd ..
```

Expected : build réussi, aucun lien mort.

- [ ] **Step 6 : la passe complète, une dernière fois**

```bash
SCRATCH=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/f383d35c-bc6f-4366-8018-7228bde636bb/scratchpad
cargo test --workspace --no-fail-fast > "$SCRATCH/passe-lente-t8.txt" 2>&1; tail -40 "$SCRATCH/passe-lente-t8.txt"
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```

Expected : `test result: ok` partout, clippy et fmt silencieux.

- [ ] **Step 7 : commit**

```bash
git add docs CHANGELOG.md CHANGELOG.fr.md Cargo.toml CLAUDE.md
git commit -F - <<'MSG'
docs: décrit la disposition src/modules et publie la 1.3.0

Les neuf pages qui citaient un chemin de module le citent rangé, dans les
deux langues. La page de structure gagne le paragraphe qui manque : ce que
src/ porte, ce que src/modules/ porte, pourquoi auth fait exception, et ce
qu'un projet antérieur doit attendre.

La version passe en mineure : aucune API publique ne change, seul change ce
que le CLI écrit — une rupture réelle pour qui lit son projet,
inobservable pour le compilateur.

Vérifications :

- npm run build dans docs/ : site construit, aucun lien mort
- cargo test --workspace --no-fail-fast : <chiffres réels>
- cargo clippy --workspace --all-targets -- -D warnings : silence
- cargo fmt --all --check : silence
MSG
```

---

## Auto-revue

**Couverture de la spec.** Chaque section trouve sa tâche : la quatorzième ancre → Task 1 ; le point de montage → Task 2 ; les templates → Tasks 3 et 4 ; `doctor` (ancre, `HORS_FEATURES`, `written_by_hand`, contrôle neuf) → Tasks 1 et 5 ; les projets existants → Task 5, et `AGENTS.md` par `agents::resolved`, qui lit le registre et n'a donc rien à modifier ; les exemples → Task 6 ; les tests → Tasks 3, 4 et 7 ; la documentation et le versionnage → Task 8. Le hors-périmètre de la spec n'a aucune tâche, ce qui est voulu.

**Cohérence des noms.** `anchors::MODULES` (Task 1) est consommée sous ce nom en Tasks 2 et 5. `POINT_DE_MONTAGE` et `MONTAGE_INITIAL` ne vivent qu'en Task 2. `disposition::{TITRE, check}` est défini et enregistré dans la même tâche. `HORS_FEATURES` passe de 4 à 5 entrées, annoncé comme tel.

**Un point de vigilance pour l'exécutant.** Les `sed` des Tasks 3, 4 et 8 sont écrits pour macOS (`sed -i ''`). Chacun est suivi d'un `grep` de contrôle : ne pas passer à l'étape suivante si le contrôle ne rend pas ce qui est annoncé. Un préfixe appliqué deux fois donne `crate::modules::modules::`, que le compilateur ne rattrapera qu'en Task 4.
