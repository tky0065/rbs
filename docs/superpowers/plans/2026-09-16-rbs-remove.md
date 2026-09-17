# `rbs remove <fragment>` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Désinstaller un fragment posé par `rbs add` — fichiers, ancres, migration, dépendances, sections de configuration, métadonnées — en refusant plutôt qu'en devinant.

**Architecture:** La décision structurante est la bascule de `plan::File.after` de `String` vers `Option<String>`, qui fait entrer l'absence au cœur du plan : la porte des conflits, la restauration tout-ou-rien, l'affichage et l'idempotence la traversent sans être élargis. Par-dessus, `src/remove/{mod,desinstallation}.rs` est le miroir d'`add/{mod,installation}.rs` — `mod` lit, refuse et rend un `Planned` ; `desinstallation` parcourt le manifeste du fragment à l'envers.

**Tech Stack:** Rust 1.98, `toml_edit`, `minijinja`, `clap`, `thiserror`, `assert_cmd`, `tempfile`, `testcontainers`.

**Spec:** `docs/superpowers/specs/2026-09-16-rbs-remove-design.md` (commit `62399e4`). Le plan argumente depuis elle ; les deux se lisent ensemble.

## Global Constraints

- **Branche** : `remove-fragment`. Jamais de commit sur `main`.
- **Commits** : Conventional Commits, sujet en français, verbe en tête au présent, sans
  majuscule ni point final. Le `CLAUDE.md` dit « à l'impératif », mais la pratique du dépôt
  est l'indicatif de troisième personne — le sujet décrit *ce que le commit fait au dépôt*.
  Sur tous les verbes irréguliers du corpus : `rend` (41), `fait` (26), `dit` (20),
  `inscrit` (15), `lit` (14), `écrit` (10), `met` (7), et jamais `rends`, `fais` ni `dis`.
  Suivre le corpus, pas la lettre. Aucun identifiant de tâche, aucun renvoi à un fichier de suivi, aucune ligne `Co-Authored-By` ni `Claude-Session`. Le corps porte le *pourquoi* et un intertitre `Vérifications :` avec les commandes réellement lancées et leurs chiffres.
- **Noms de tests en anglais**, `snake_case`, phrase complète — `a_conflict_rejects_the_plan_before_the_first_write`. Commentaires et documentation en français.
- **Commentaires** : le *pourquoi*, jamais le *quoi*. `#![warn(missing_docs)]` ne concerne que `rbs-core`, pas `rbs-cli`.
- **Bloquant en CI**, à relancer à chaque tâche : `cargo fmt --all --check` et `cargo clippy --workspace --all-targets -- -D warnings`.
- **Aucune conversation avec la base** dans tout le code de `remove`.
- **Aucun scan des références** depuis le code de l'utilisateur.
- **Aucune template modifiée** : `integration_examples` doit rester vert sans régénération d'`examples/`.
- Toolchain locale possiblement en retard sur la CI : `rustup update` avant la passe finale.
- **Preuve que les tests discriminent.** Un rouge obtenu par une fonction absente — « la
  fonction n'existe pas encore », donc une erreur de compilation — ne prouve rien de la
  qualité des assertions. Pour chaque comportement neuf, le rapport doit porter une
  **mutation délibérée** : casser l'implémentation d'une manière qui **laisse le code
  compiler**, lancer le test couvrant, coller sa sortie rouge réelle, restaurer. C'est le
  standard du dépôt — plusieurs entrées de son backlog portent un « rouge prouvé » obtenu
  ainsi. Une mutation qui casse la compilation ne compte pas.
- **Les sorties de commandes se collent, ne s'affirment pas.** `cargo fmt --all --check` et
  `cargo clippy --workspace --all-targets -- -D warnings` déclarés verts en prose, sans
  trace, sont un constat de revue. **Une commande qui n'émet rien quand tout va bien —
  `cargo fmt --all --check` en premier lieu — n'a pas de sortie à coller : prouve-la par son
  code de retour**, `cargo fmt --all --check; echo "exit: $?"`, et colle ces deux lignes.
  « Silencieux, code de sortie 0 » écrit en prose n'est pas une trace, c'est une affirmation.
- **Un wrapper de `Builder` reçoit ses propres tests, au niveau `Builder`.** Tester la
  fonction pure qu'il appelle ne suffit pas : le wrapper calcule un statut, propage des
  erreurs et projette sur `files()`, et rien de tout cela n'est exercé par un test de la
  couche du dessous. Le précédent du dépôt est établi — `Builder::supprimer` en a trois,
  `Builder::retirer_lignes` en a cinq. Au minimum : un retrait effectif à `Status::AFaire`,
  un retrait sans effet à `Status::DejaFait`, et la propagation de l'erreur spécifique.
- **Une mutation par comportement neuf**, pas une par tâche. Le chemin de succès, le
  branchement qui ne fait rien, et le chemin d'erreur sont trois comportements distincts.

---

### Task 1: L'absence entre dans le plan

**Files:**
- Modify: `crates/rbs-cli/src/plan/mod.rs:23-35` (`struct File`), `:625`, `:650-663` (`project_onto`), `:688-694` (`combined_status`)
- Modify: `crates/rbs-cli/src/plan/application.rs:90-104` (`Log::write`)
- Modify: `crates/rbs-cli/src/plan/json.rs:621,627,633`, `crates/rbs-cli/src/plan/render.rs:171-172`
- Test: `crates/rbs-cli/src/plan/application.rs` (module `tests`)

**Interfaces:**
- Consomme : rien.
- Produit : `File.after: Option<String>` — `None` signifie *ce fichier ne doit plus exister*. `combined_status(origin: Option<&str>, after: Option<&str>) -> Status`. `Builder::project_onto(path, before, after: Option<String>, statut)`.

Refactor pur : aucun comportement existant ne change, tous les appelants rendent `Some(...)`. Les vingt-huit assertions `plan.files()[0].after` de `plan/mod.rs` deviennent `.after.as_deref()`.

- [ ] **Step 1: Écrire le test qui échoue**

Dans le module `tests` de `crates/rbs-cli/src/plan/application.rs`, étendre le helper `file` puis ajouter :

```rust
/// Un fichier que le plan projette absent est retiré du disque.
#[test]
fn a_file_projected_absent_is_removed_from_disk() {
    let projet = project();
    let racine = projet.path();
    fs::write(racine.join("src/parti.rs"), "// à retirer\n").expect("le fichier s'écrit");

    let plan = plan_of(
        racine,
        vec![File {
            path: "src/parti.rs".to_string(),
            before: Some("// à retirer\n".to_string()),
            after: None,
            statut: Status::AFaire,
        }],
    );

    apply(&plan, false).expect("le plan s'applique");

    assert!(
        !racine.join("src/parti.rs").exists(),
        "le fichier devait disparaître"
    );
}

/// Une suppression défaite rend le fichier tel qu'il était.
#[test]
fn a_removal_rolled_back_restores_the_file() {
    let projet = project();
    let racine = projet.path();
    fs::write(racine.join("src/parti.rs"), "// à retirer\n").expect("le fichier s'écrit");

    let plan = plan_of(
        racine,
        vec![
            File {
                path: "src/parti.rs".to_string(),
                before: Some("// à retirer\n".to_string()),
                after: None,
                statut: Status::AFaire,
            },
            // Un répertoire en guise de chemin : l'écriture échoue, et la restauration
            // doit rendre le fichier que l'action précédente a supprimé.
            file("src", None, "peu importe", Status::AFaire),
        ],
    );

    apply(&plan, false).expect_err("l'écriture sur un répertoire doit échouer");

    assert_eq!(
        fs::read_to_string(racine.join("src/parti.rs")).expect("le fichier est revenu"),
        "// à retirer\n"
    );
}
```

- [ ] **Step 2: Lancer le test et le voir échouer**

Run: `cargo test -p rbs-cli --lib -- plan::application::tests::a_file_projected_absent_is_removed_from_disk`
Expected: FAIL à la compilation — `expected String, found Option<_>` sur le champ `after`.

- [ ] **Step 3: Basculer le type**

```rust
// plan/mod.rs
pub(crate) struct File {
    pub path: String,
    pub before: Option<String>,
    /// Contenu que l'application écrira, ou `None` si le fichier ne doit plus exister.
    ///
    /// L'absence est dans le type et non à côté : la porte des conflits, la restauration
    /// et l'affichage travaillent tous sur `File`, et n'ont donc rien à apprendre.
    pub after: Option<String>,
    pub statut: Status,
}

fn combined_status(origin: Option<&str>, after: Option<&str>) -> Status {
    if origin == after {
        Status::DejaFait
    } else {
        Status::AFaire
    }
}
```

`project_onto` prend `after: Option<String>`. `states()` (`:625`) devient `courant: file.after.clone()`.

```rust
// plan/application.rs
fn write(&mut self, root: &Path, file: &File) -> io::Result<()> {
    let path = root.join(&file.path);

    match &file.after {
        Some(content) => {
            if let Some(parent) = path.parent() {
                self.create_directories(parent)?;
            }
            crate::secret::write(&path, content.as_bytes())?;
        }
        // Un fichier déjà absent n'est pas une faute : le plan projette un état, pas une
        // opération, et l'état visé est déjà celui-là.
        None => match fs::remove_file(&path) {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(source),
        },
    }

    self.ecrits.push(file.path.clone());
    self.origines.push(file.before.clone());

    Ok(())
}
```

`Log::undo` ne bouge pas : il restaure déjà `Some` par écriture et `None` par `remove_file`.

Tous les autres appelants — `Builder::create`, `insert`, `patch`, les helpers de test de `json.rs` et `render.rs` — passent `Some(...)`. Les assertions `.after` de `plan/mod.rs` deviennent `.after.as_deref()`.

- [ ] **Step 4: Lancer les tests et les voir passer**

Run: `cargo test -p rbs-cli --lib`
Expected: PASS — 1569 passés, 0 échoué (les 1567 d'avant plus les deux neufs).

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/plan/
git commit -m "refactor(plan): fait porter l'absence d'un fichier par le plan lui-même"
```

---

### Task 2: `Builder::supprimer` et la divergence

**Files:**
- Modify: `crates/rbs-cli/src/plan/action.rs` (`Effect`), `crates/rbs-cli/src/plan/mod.rs` (après `create`, `:298-330`)
- Test: `crates/rbs-cli/src/plan/mod.rs` (module `tests`)

**Interfaces:**
- Consomme : `File.after: Option<String>` de la tâche 1.
- Produit : `Effect::Supprimer` ; `Builder::supprimer(&mut self, path: &str, rendu_attendu: &str) -> Result<(), Error>`.

C'est ici que naît le contrat central de la spec (§3.1, refus 4) : `Conflit` quand le fichier existe **et diffère** du rendu neuf.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
/// Un fichier conforme au rendu se supprime sans réclamer de forçage.
#[test]
fn a_file_matching_its_render_is_removed_without_force() {
    let projet = projet_avec(&[("src/pose.rs", "// posé par le fragment\n")]);
    let mut builder = Builder::new(projet.path());

    builder
        .supprimer("src/pose.rs", "// posé par le fragment\n")
        .expect("la suppression se planifie");

    let plan = builder.finir();
    assert_eq!(plan.files()[0].statut, Status::AFaire);
    assert_eq!(plan.files()[0].after, None);
}

/// Un fichier que le développeur a modifié entre en conflit.
#[test]
fn a_file_the_developer_changed_is_a_conflict() {
    let projet = projet_avec(&[("src/pose.rs", "// posé, puis retouché\n")]);
    let mut builder = Builder::new(projet.path());

    builder
        .supprimer("src/pose.rs", "// posé par le fragment\n")
        .expect("la suppression se planifie");

    assert_eq!(builder.finir().files()[0].statut, Status::Conflit);
}

/// Un fichier déjà absent rend la suppression sans effet, et non fautive.
#[test]
fn an_already_absent_file_makes_the_removal_a_no_op() {
    let projet = projet_avec(&[]);
    let mut builder = Builder::new(projet.path());

    builder
        .supprimer("src/pose.rs", "// posé par le fragment\n")
        .expect("la suppression se planifie");

    assert_eq!(builder.finir().files()[0].statut, Status::DejaFait);
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --lib -- plan::tests::a_file_matching_its_render_is_removed_without_force`
Expected: FAIL — `no method named 'supprimer' found for struct 'Builder'`.

- [ ] **Step 3: Implémenter**

```rust
// plan/action.rs, dans enum Effect
/// Retire un fichier que le projet ne doit plus porter.
Supprimer,
```

```rust
// plan/mod.rs, juste après create
/// Planifie le retrait de `path`, dont le contenu attendu est `rendu_attendu`.
///
/// Le statut est l'inverse exact de celui de [`Builder::create`] : là où créer est un
/// conflit quand le fichier existe, supprimer n'en est un que s'il existe **et diffère**
/// de ce qu'une installation neuve produirait. C'est ce test, et lui seul, qui distingue
/// un fichier que le CLI a posé d'un fichier que le développeur a fait sien.
pub fn supprimer(&mut self, path: &str, rendu_attendu: &str) -> Result<(), Error> {
    if self.projected(path) {
        return Err(Error::DejaProjete {
            path: path.to_string(),
        });
    }

    let origin = self.read(path)?;

    let statut = match origin.as_deref() {
        None => Status::DejaFait,
        Some(actuel) if actuel == rendu_attendu => Status::AFaire,
        Some(_) => Status::Conflit,
    };

    self.project_onto(path, origin, None, statut);
    self.actions.push(Action {
        path: path.to_string(),
        effet: Effect::Supprimer,
        statut,
    });

    Ok(())
}
```

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --lib -- plan::tests`
Expected: PASS.

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/plan/
git commit -m "feat(plan): planifie le retrait d'un fichier conforme à son rendu"
```

---

### Task 3: `anchors::retire`

**Files:**
- Modify: `crates/rbs-cli/src/anchors.rs` (après `insert`, `:637-694`)
- Test: `crates/rbs-cli/src/anchors.rs` (module `tests`)

**Interfaces:**
- Consomme : `line_of`, `Missing`, `Anchor` — existants.
- Produit : `anchors::retire(source: &str, anchor: &Anchor, lines: &[String]) -> Result<String, Missing>`.

Symétrique d'`insert`, mais sans la branche `sorted` : retirer une ligne d'un bloc trié le laisse trié.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
/// La ligne visée s'en va, ses voisines restent.
#[test]
fn the_named_line_leaves_and_its_neighbours_stay() {
    let source = "// <rbs:routes>\n    .merge(a::routes())\n    .merge(b::routes())\n// </rbs:routes>\n";

    let apres = retire(source, &ROUTES, &[".merge(a::routes())".to_string()])
        .expect("l'ancre est là");

    assert!(!apres.contains("a::routes"));
    assert!(apres.contains("    .merge(b::routes())\n"));
}

/// Retirer une ligne absente ne change rien : le retrait est idempotent.
#[test]
fn removing_an_absent_line_changes_nothing() {
    let source = "// <rbs:routes>\n    .merge(b::routes())\n// </rbs:routes>\n";

    let apres = retire(source, &ROUTES, &[".merge(a::routes())".to_string()])
        .expect("l'ancre est là");

    assert_eq!(apres, source);
}

/// Rien hors de l'ancre n'est touché, fût-ce une ligne identique.
#[test]
fn an_identical_line_outside_the_anchor_survives() {
    let source = "    .merge(a::routes())\n// <rbs:routes>\n    .merge(a::routes())\n// </rbs:routes>\n";

    let apres = retire(source, &ROUTES, &[".merge(a::routes())".to_string()])
        .expect("l'ancre est là");

    assert_eq!(apres.matches("a::routes").count(), 1);
}

/// Une ancre absente est une faute, comme pour l'insertion.
#[test]
fn a_missing_anchor_is_reported() {
    retire("pub fn router() {}\n", &ROUTES, &[".merge(a::routes())".to_string()])
        .expect_err("l'ancre manque");
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --lib -- anchors::tests::the_named_line_leaves_and_its_neighbours_stay`
Expected: FAIL — `cannot find function 'retire' in this scope`.

- [ ] **Step 3: Implémenter**

```rust
/// Rend `source` privé des `lines` que porte le bloc de `anchor`.
///
/// Pas de branche `sorted`, à la différence d'[`insert`] : retirer une ligne d'un bloc
/// trié le laisse trié. La comparaison se fait sur la ligne ébarbée, comme dans
/// [`contains`] — l'indentation appartient au fichier, pas à la déclaration du fragment.
pub(crate) fn retire(
    source: &str,
    anchor: &Anchor,
    lines: &[String],
) -> Result<String, Missing> {
    let absente = || Missing {
        anchor: anchor.clone(),
    };

    let (opening, _) = line_of(source, &anchor.opening()).ok_or_else(absente)?;
    let (closing, _) = line_of(source, &anchor.closing()).ok_or_else(absente)?;

    if closing < opening {
        return Err(absente());
    }

    // `opening` est le début de la ligne de balise : le corps commence après son saut.
    let debut = source[opening..closing]
        .find('\n')
        .map_or(closing, |fin| opening + fin + 1);

    let a_retirer: Vec<&str> = lines.iter().map(|line| line.trim()).collect();
    let corps: String = source[debut..closing]
        .split_inclusive('\n')
        .filter(|line| !a_retirer.contains(&line.trim()))
        .collect();

    Ok(format!("{}{corps}{}", &source[..debut], &source[closing..]))
}
```

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --lib -- anchors::tests`
Expected: PASS.

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/anchors.rs
git commit -m "feat(anchors): retire d'un bloc les lignes qu'un fragment y avait posées"
```

---

### Task 4: `Builder::retirer_lignes`

> **Correction du contrôleur (R5) — effet de bord obligatoire.** Ajouter une variante à
> `Effect` ou à `PatchToml` casse aussitôt le `match` exhaustif de
> `From<&Effect> for EffectJson` (`crates/rbs-cli/src/plan/json.rs`), et le projet ne
> compile plus. Pose donc dans le même commit le **bras minimal** de sérialisation, sans
> test JSON : le test appartient à la tâche 7. Ce n'est pas un débordement de périmètre,
> c'est la condition pour que ta tâche compile.

**Files:**
- Modify: `crates/rbs-cli/src/plan/action.rs` (`Effect`), `crates/rbs-cli/src/plan/mod.rs` (après `insert`)
- Test: `crates/rbs-cli/src/plan/mod.rs` (module `tests`)

**Interfaces:**
- Consomme : `anchors::retire` (tâche 3), `File.after: Option<String>` (tâche 1).
- Produit : `Effect::RetirerLignes { anchor: Anchor, lines: Vec<String> }` ; `Builder::retirer_lignes(&mut self, anchor: Anchor, lines: &[String]) -> Result<(), Error>`.

Une ancre optionnelle dont le fichier porteur manque n'est pas une faute : il n'y a rien à retirer, et c'est un succès silencieux — non une `Sautee`, qui sert à annoncer un bloc *à coller*.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
/// Le retrait passe par le plan, et le fichier projeté perd la ligne.
#[test]
fn a_planned_removal_drops_the_line_from_the_projected_file() {
    let projet = projet_avec(&[(
        "src/router.rs",
        "// <rbs:routes>\n        .merge(a::routes())\n        // </rbs:routes>\n",
    )]);
    let mut builder = Builder::new(projet.path());

    builder
        .retirer_lignes(ROUTES, &[".merge(a::routes())".to_string()])
        .expect("le retrait se planifie");

    let plan = builder.finir();
    assert_eq!(plan.files()[0].statut, Status::AFaire);
    assert!(!plan.files()[0].after.as_deref().expect("le fichier reste").contains("a::routes"));
}

/// Une ancre optionnelle dont le fichier manque ne fait rien, et ne faute pas.
#[test]
fn an_optional_anchor_without_its_file_removes_nothing() {
    let projet = projet_avec(&[]);
    let mut builder = Builder::new(projet.path());

    builder
        .retirer_lignes(SERVICES, &["  mailpit:".to_string()])
        .expect("l'absence du compose n'est pas une faute");

    assert!(builder.finir().files().is_empty());
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --lib -- plan::tests::a_planned_removal_drops_the_line_from_the_projected_file`
Expected: FAIL — `no method named 'retirer_lignes'`.

- [ ] **Step 3: Implémenter**

```rust
// plan/action.rs, dans enum Effect
/// Retire d'une ancre les lignes qu'un fragment y avait posées.
RetirerLignes {
    /// L'ancre visée.
    anchor: Anchor,
    /// Les lignes à retirer, telles que le manifeste du fragment les déclare.
    lines: Vec<String>,
},
```

```rust
// plan/mod.rs
/// Planifie le retrait de `lines` du bloc de `anchor`.
///
/// Un fichier porteur absent n'est une faute que si l'ancre ne l'est pas : le compose
/// d'un projet SQLite n'existe pas, et le service qu'un fragment y aurait posé n'a rien
/// à y perdre. Rien n'est consigné en `Sautee` — celle-ci annonce un bloc *à coller*,
/// et il n'y a ici rien à coller.
pub fn retirer_lignes(&mut self, anchor: Anchor, lines: &[String]) -> Result<(), Error> {
    let path = anchor.file.to_string();

    let states = self.states(&path)?;
    let Some(courant) = states.courant else {
        if anchor.optional {
            return Ok(());
        }
        return Err(Error::FichierAbsent { path });
    };

    let after = crate::anchors::retire(&courant, &anchor, lines).map_err(Error::Anchor)?;
    let statut = combined_status(states.origin.as_deref(), Some(after.as_str()));

    self.project_onto(&path, states.origin, Some(after), statut);
    self.actions.push(Action {
        path,
        effet: Effect::RetirerLignes {
            anchor,
            lines: lines.to_vec(),
        },
        statut,
    });

    Ok(())
}
```

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --lib -- plan::tests`
Expected: PASS.

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/plan/ crates/rbs-cli/src/anchors.rs
git commit -m "feat(plan): planifie le retrait de lignes d'une ancre"
```

---

### Task 5: Les trois inverses du manifeste Cargo

> **Correction du contrôleur (R5) — effet de bord obligatoire.** Ajouter une variante à
> `Effect` ou à `PatchToml` casse aussitôt le `match` exhaustif de
> `From<&Effect> for EffectJson` (`crates/rbs-cli/src/plan/json.rs`), et le projet ne
> compile plus. Pose donc dans le même commit le **bras minimal** de sérialisation, sans
> test JSON : le test appartient à la tâche 7. Ce n'est pas un débordement de périmètre,
> c'est la condition pour que ta tâche compile.

**Files:**
- Modify: `crates/rbs-cli/src/metadata.rs` (après `record_feature:293`, `add_dependency:342`, `add_feature_to_dependency:392`)
- Modify: `crates/rbs-cli/src/plan/action.rs` (`PatchToml`), `crates/rbs-cli/src/plan/mod.rs` (`Builder::patch:498`)
- Test: `crates/rbs-cli/src/metadata.rs` (module `tests`)

**Interfaces:**
- Consomme : `parse`, `enable_feature` — existants.
- Produit :
  - `metadata::remove_feature(text: &str, feature: &str, name: &str) -> Result<Option<String>, Error>`
  - `metadata::remove_dependency(text: &str, dep: &str, name: &str) -> Result<Option<String>, Error>`
  - `metadata::remove_feature_from_dependency(text: &str, dep: &str, feature: &str, name: &str) -> Result<Option<String>, Error>`
  - `PatchToml::{RetirerFeature(String), RetirerDependance(String), RetirerFeatureADependance { dependency: String, feature: String }}`

Chacune rend `None` quand il n'y a rien à faire, comme sa jumelle.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
/// La feature quitte le tableau, les autres gardent leur ordre.
#[test]
fn the_feature_leaves_the_array_and_the_others_keep_their_order() {
    let manifeste = MANIFESTE_AVEC_FEATURES; // ["health", "mail", "auth"]

    let apres = remove_feature(manifeste, "mail", "Cargo.toml")
        .expect("le manifeste se lit")
        .expect("le tableau change");

    assert!(apres.contains(r#"features = ["health", "auth"]"#));
}

/// Une feature absente ne fait rien écrire.
#[test]
fn an_absent_feature_rewrites_nothing() {
    assert_eq!(
        remove_feature(MANIFESTE_AVEC_FEATURES, "storage", "Cargo.toml")
            .expect("le manifeste se lit"),
        None
    );
}

/// La dépendance quitte la table, le reste du manifeste est intact.
#[test]
fn the_dependency_leaves_the_table_and_the_rest_survives() {
    let apres = remove_dependency(MANIFESTE_AVEC_LETTRE, "lettre", "Cargo.toml")
        .expect("le manifeste se lit")
        .expect("la table change");

    assert!(!apres.contains("lettre"));
    assert!(apres.contains("# un commentaire du développeur"));
}

/// Une feature quitte une dépendance, ses sœurs restent.
#[test]
fn a_feature_leaves_a_dependency_and_its_siblings_stay() {
    let apres = remove_feature_from_dependency(
        MANIFESTE_AVEC_TOKIO, // features = ["macros", "time", "sync"]
        "tokio",
        "sync",
        "Cargo.toml",
    )
    .expect("le manifeste se lit")
    .expect("la déclaration change");

    assert!(apres.contains(r#"features = ["macros", "time"]"#));
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --lib -- metadata::tests::the_feature_leaves_the_array_and_the_others_keep_their_order`
Expected: FAIL — `cannot find function 'remove_feature'`.

- [ ] **Step 3: Implémenter**

```rust
/// Rend le manifeste privé de `feature` dans `[package.metadata.rbs]`, ou `None` s'il ne
/// l'inscrit pas.
pub fn remove_feature(text: &str, feature: &str, name: &str) -> Result<Option<String>, Error> {
    let mut document = parse(text, name)?;

    let Some(installees) = document
        .get_mut("package")
        .and_then(|package| package.get_mut("metadata"))
        .and_then(|metadata| metadata.get_mut("rbs"))
        .and_then(|rbs| rbs.get_mut("features"))
        .and_then(Item::as_array_mut)
    else {
        return Err(Error::PasUnProjet {
            path: name.to_string(),
        });
    };

    let avant = installees.len();
    installees.retain(|value| value.as_str() != Some(feature));

    Ok((installees.len() != avant).then(|| document.to_string()))
}

/// Rend le manifeste privé de la dépendance `dep`, ou `None` s'il ne la déclare pas.
///
/// L'appelant a déjà tranché que personne d'autre ne la réclame : la règle d'union
/// appartient à `remove::desinstallation`, qui seul connaît les fragments installés.
pub fn remove_dependency(text: &str, dep: &str, name: &str) -> Result<Option<String>, Error> {
    let mut document = parse(text, name)?;

    let Some(dependencies) = document
        .get_mut("dependencies")
        .and_then(Item::as_table_like_mut)
    else {
        return Ok(None);
    };

    Ok(dependencies.remove(dep).is_some().then(|| document.to_string()))
}

/// Rend le manifeste avec `feature` désactivée sur `dep`, ou `None` si elle ne l'est pas.
///
/// Une dépendance absente n'est pas une faute ici, à la différence de
/// [`add_feature_to_dependency`] : un fragment parti a pu emporter la dépendance dans le
/// même plan.
pub fn remove_feature_from_dependency(
    text: &str,
    dep: &str,
    feature: &str,
    name: &str,
) -> Result<Option<String>, Error> {
    let mut document = parse(text, name)?;

    let Some(declared) = document
        .get_mut("dependencies")
        .and_then(Item::as_table_like_mut)
        .and_then(|dependencies| dependencies.get_mut(dep))
    else {
        return Ok(None);
    };

    let modifie = disable_feature(declared, feature).ok_or_else(|| Error::Declaration {
        path: name.to_string(),
        key: dep.to_string(),
    })?;

    Ok(modifie.then(|| document.to_string()))
}
```

`disable_feature` se pose à côté d'`enable_feature`, même forme : rend `None` sur une déclaration malformée, `Some(false)` si la feature n'y était pas, `Some(true)` si elle en est retirée.

Les trois variantes de `PatchToml` et leurs bras dans `Builder::patch` suivent la forme des variantes existantes.

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --lib -- metadata::`
Expected: PASS.

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/metadata.rs crates/rbs-cli/src/plan/
git commit -m "feat(metadata): inverse l'inscription d'une feature, d'une dépendance et de ses flags"
```

---

### Task 6: Le retrait d'une section de configuration

> **Correction du contrôleur (R5) — effet de bord obligatoire.** Ajouter une variante à
> `Effect` ou à `PatchToml` casse aussitôt le `match` exhaustif de
> `From<&Effect> for EffectJson` (`crates/rbs-cli/src/plan/json.rs`), et le projet ne
> compile plus. Pose donc dans le même commit le **bras minimal** de sérialisation, sans
> test JSON : le test appartient à la tâche 7. Ce n'est pas un débordement de périmètre,
> c'est la condition pour que ta tâche compile.

**Files:**
- Modify: `crates/rbs-cli/src/plan/text.rs` (après `add_section:16`), `crates/rbs-cli/src/plan/action.rs`, `crates/rbs-cli/src/plan/mod.rs` (après `add_section:538`)
- Test: `crates/rbs-cli/src/plan/text.rs` (module `tests`)

**Interfaces:**
- Produit : `plan::text::remove_section(text: &str, section: &str) -> Result<Option<String>, TomlError>` ; `Effect::RetirerSection { section: String }` ; `Builder::retirer_section(&mut self, path: &str, section: &str) -> Result<(), Error>`.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
/// La section s'en va, le reste du document la traverse intact.
#[test]
fn the_section_leaves_and_the_rest_of_the_document_survives() {
    let source = "[server]\nport = 3000\n\n[mail]\nfrom = \"a@b.c\"\n";

    let apres = remove_section(source, "mail")
        .expect("le document se lit")
        .expect("la section part");

    assert!(!apres.contains("[mail]"));
    assert!(apres.contains("port = 3000"));
}

/// Une section absente ne fait rien réécrire.
#[test]
fn an_absent_section_rewrites_nothing() {
    assert_eq!(
        remove_section("[server]\nport = 3000\n", "mail").expect("le document se lit"),
        None
    );
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --lib -- plan::text::tests::the_section_leaves_and_the_rest_of_the_document_survives`
Expected: FAIL — `cannot find function 'remove_section'`.

- [ ] **Step 3: Implémenter**

```rust
/// Rend le document privé de `section`, ou `None` s'il ne la porte pas.
///
/// Comme [`add_section`], le document n'est pas re-sérialisé au-delà de ce que `toml_edit`
/// préserve : l'ordre, les commentaires et la mise en forme du développeur traversent le
/// retrait.
pub(crate) fn remove_section(text: &str, section: &str) -> Result<Option<String>, TomlError> {
    let mut document: DocumentMut = text.parse()?;

    Ok(document
        .remove(section)
        .is_some()
        .then(|| document.to_string()))
}
```

`Builder::retirer_section` suit `Builder::add_section` trait pour trait, avec `Effect::RetirerSection`.

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --lib -- plan::text::tests`
Expected: PASS.

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/plan/
git commit -m "feat(plan): retire d'un document de configuration la section d'un fragment"
```

---

### Task 7: Les nouvelles variantes en JSON

> **Correction du contrôleur (R5).** Les bras de sérialisation des variantes ont déjà été
> posés, au minimum, par les tâches qui les ont introduites — sans quoi elles n'auraient pas
> compilé. Ton travail ici est donc : **les tests** qui les gardent, les variantes de
> `PatchTomlJson` qui manqueraient encore, et la vérification que chaque bras rend bien le
> nom attendu. Ne t'étonne pas de trouver le code déjà en place ; vérifie-le.

> **Correction du contrôleur (R3) — prime sur le code de l'étape 1 ci-dessous.**
> Le helper `json_of(&[Action…])` **n'existe pas**. Le module de tests de
> `crates/rbs-cli/src/plan/json.rs` construit par `minimal_plan(effect, statut)` puis
> `document(&plan)`, qui rend un `serde_json::Value`. Suis cet idiome : assertions sur
> `document(&plan)["actions"][0]["effet"]["type"]`, et non par recherche de sous-chaîne.
> Même remarque pour `PatchTomlJson`, que le plan ne décrit qu'en prose : ses trois
> variantes suivent la forme des existantes.

**Files:**
- Modify: `crates/rbs-cli/src/plan/json.rs:64-91` (`EffectJson`), `:93-125` (`From<&Effect>`), et `PatchTomlJson`
- Test: `crates/rbs-cli/src/plan/json.rs` (module `tests`)

**Interfaces:**
- Consomme : les variantes des tâches 2, 4, 5, 6.
- Produit : la sérialisation de `Supprimer`, `RetirerLignes`, `RetirerSection`, et des trois `PatchToml` neuves.

- [ ] **Step 1: Écrire le test qui échoue**

```rust
/// Chaque effet de retrait porte son nom dans le document.
#[test]
fn each_removal_effect_carries_its_name_in_the_document() {
    let document = json_of(&[
        Action { path: "src/parti.rs".to_string(), effet: Effect::Supprimer, statut: Status::AFaire },
        Action {
            path: "src/router.rs".to_string(),
            effet: Effect::RetirerLignes { anchor: ROUTES, lines: vec![".merge(a::routes())".to_string()] },
            statut: Status::AFaire,
        },
    ]);

    assert!(document.contains(r#""type":"supprimer""#));
    assert!(document.contains(r#""type":"retirer_lignes""#));
    assert!(document.contains(r#""ancre":"routes""#));
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --lib -- plan::json::tests::each_removal_effect_carries_its_name_in_the_document`
Expected: FAIL — `non-exhaustive patterns` sur `From<&Effect> for EffectJson`, le `match` ne couvrant pas les variantes neuves.

- [ ] **Step 3: Implémenter**

```rust
// dans enum EffectJson<'a>
Supprimer,
RetirerLignes {
    ancre: &'a str,
    lignes: &'a [String],
},
RetirerSection {
    section: &'a str,
},
```

```rust
// dans From<&Effect> for EffectJson
Effect::Supprimer => EffectJson::Supprimer,
Effect::RetirerLignes { anchor, lines } => EffectJson::RetirerLignes {
    ancre: anchor.name.as_ref(),
    lignes: lines,
},
Effect::RetirerSection { section } => EffectJson::RetirerSection { section },
```

`PatchTomlJson` reçoit les trois variantes symétriques.

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --lib -- plan::json::tests`
Expected: PASS.

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/plan/json.rs
git commit -m "feat(plan): sérialise les effets de retrait dans le document JSON"
```

---

### Task 8: `remove::desinstallation` — le parcours inverse du manifeste

> **Correction du contrôleur (R10) — un fichier `if_absent` ne se supprime JAMAIS.**
> Mon texte disait qu'il « est traité comme les autres : il a été posé ou il ne l'a pas
> été ». C'est faux, et une revue l'a prouvé : `docker/config/production.toml.jinja` et
> `project/config/production.toml.jinja` sont identiques octet pour octet, donc le rendu
> coïncide avec le disque, donc `Status::AFaire`, donc `rbs remove docker` effacerait sans
> `--force` un fichier que le squelette écrit sur tout projet. Pire, `docker-compose.yml`
> est `if_absent` lui aussi, et `mail` comme `redis` y insèrent leurs services : le
> supprimer emporterait le travail d'autres fragments installés.
> `if_absent` veut dire « ne le pose que s'il manque » — le fragment y désavoue la
> paternité du fichier quand il préexiste, et le retrait ne peut pas savoir lequel des deux
> cas s'est produit. **Ces fichiers sont signalés dans `laissees`, comme les variables
> d'environnement, jamais supprimés.**

> **Correction du contrôleur (R8) — le point de montage ne se supprime jamais.**
> `installation::ouvre_le_point_de_montage` crée `src/modules/mod.rs` s'il manque, puis y
> insère `pub mod <feature>;`. L'inverse s'arrête à la ligne : **retire le `pub mod`, ne
> supprime jamais le fichier**, même s'il devient vide. D'autres fragments et les CRUD
> engendrés y vivent aussi, et `lib.rs` porte un `mod modules;` qu'un fichier supprimé
> casserait, là où un fichier vide ne gêne personne. Corollaire pour l'ordre : aucune ancre
> que tu vides ne vit dans un fichier que le même plan supprime — les ancres sont dans le
> squelette, les fichiers supprimés appartiennent au fragment. Si tu rencontres un contre
> -exemple, arrête-toi et signale-le plutôt que de deviner.

**Files:**
- Create: `crates/rbs-cli/src/remove/mod.rs` (déclaration du module seulement, à cette tâche), `crates/rbs-cli/src/remove/desinstallation.rs`
- Modify: `crates/rbs-cli/src/lib.rs` (`mod remove;`)
- Test: `crates/rbs-cli/src/remove/desinstallation.rs` (module `tests`)

**Interfaces:**
- Consomme : `Builder::{supprimer, retirer_lignes, retirer_section, patch}` (tâches 2, 4, 5, 6), `manifest::Manifest`, `templates`, `Renderer`, `mount::for_migration`.
- Produit :
  ```rust
  pub(crate) struct Fragment<'a> {
      pub name: &'a str,
      pub manifest: &'a manifest::Manifest,
      pub templates: &'a [templates::File],
      pub context: minijinja::Value,
      /// Ce que les autres fragments installés réclament encore.
      pub reclamees: &'a Reclamees,
  }

  pub(crate) struct Reclamees {
      pub dependencies: BTreeSet<String>,
      pub cargo: BTreeMap<String, BTreeSet<String>>,
  }

  pub(crate) fn actions(fragment: &Fragment, builder: &mut plan::Builder) -> Result<Retires, Error>;

  pub(crate) struct Retires {
      pub fichiers: Vec<String>,
      pub migration: Option<String>,
      /// Ce qui a été sciemment laissé : variables d'environnement, dépendances partagées.
      pub laissees: Vec<String>,
  }

  pub(crate) fn reclamees_ailleurs(
      template_dir: Option<&Path>,
      partant: &str,
      installees: &[String],
  ) -> Result<Reclamees, Error>;

  pub(crate) fn migration_de(
      builder: &plan::Builder,
      declared: &manifest::DeclaredMigration,
  ) -> Result<Option<(String, String)>, Error>;
  ```

`reclamees_ailleurs` lit le manifeste de chaque fragment installé **sauf le partant**. Un nom de `[package.metadata.rbs] features` sans `feature.toml` embarqué est un CRUD engendré : il est ignoré, non fautif.

`migration_de` cherche dans `migration/src/` les fichiers `m*_{declared.name}.rs`. Zéro correspondance → `None`. Plusieurs → `Error::MigrationAmbigue`, qui les nomme : `add` étant idempotent, deux fichiers signalent une réécriture manuelle que la commande ne doit pas trancher.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
/// Chaque fichier déclaré est planifié en suppression, comparé à son rendu.
#[test]
fn every_declared_file_is_planned_for_removal() {
    let (projet, fragment) = fragment_pose("cors");
    let mut builder = plan::Builder::new(projet.path());

    let retires = actions(&fragment, &mut builder).expect("le retrait se planifie");

    assert_eq!(retires.fichiers.len(), 3);
    assert!(builder.finir().files().iter().all(|file| file.after.is_none()
        || file.path.ends_with("Cargo.toml")
        || file.path.ends_with("default.toml")
        || file.path.ends_with("router.rs")));
}

/// Une dépendance qu'un autre fragment installé déclare encore reste en place.
#[test]
fn a_dependency_another_installed_fragment_still_declares_stays() {
    let (projet, fragment) = fragment_pose_avec("storage", &["jobs"]); // async-trait partagée
    let mut builder = plan::Builder::new(projet.path());

    let retires = actions(&fragment, &mut builder).expect("le retrait se planifie");

    assert!(
        retires.laissees.iter().any(|laissee| laissee.contains("async-trait")),
        "la dépendance partagée doit être nommée comme laissée : {:?}",
        retires.laissees
    );
    let cargo = builder.finir();
    let manifeste = cargo
        .files()
        .iter()
        .find(|file| file.path == "Cargo.toml")
        .expect("le manifeste est visé");
    assert!(manifeste.after.as_deref().expect("il reste").contains("async-trait"));
}

/// Une feature cargo qu'un autre fragment installé demande encore reste active.
#[test]
fn a_cargo_feature_another_installed_fragment_still_asks_for_stays() {
    // `scheduler` et `jobs` demandent tous deux des features sur `tokio`.
    let (projet, fragment) = fragment_pose_avec("scheduler", &["jobs"]);
    let mut builder = plan::Builder::new(projet.path());

    actions(&fragment, &mut builder).expect("le retrait se planifie");

    let manifeste = builder.finir();
    let cargo = manifeste
        .files()
        .iter()
        .find(|file| file.path == "Cargo.toml")
        .expect("le manifeste est visé");
    let apres = cargo.after.as_deref().expect("le manifeste reste");
    assert!(apres.contains("\"time\""), "la feature partagée devait rester : {apres}");
}

/// Une dépendance du squelette n'est jamais retirée, fût-elle vidée de ses features.
#[test]
fn a_skeleton_dependency_is_never_removed() {
    let (projet, fragment) = fragment_pose("rate-limit");
    let mut builder = plan::Builder::new(projet.path());

    actions(&fragment, &mut builder).expect("le retrait se planifie");

    let manifeste = builder.finir();
    let cargo = manifeste
        .files()
        .iter()
        .find(|file| file.path == "Cargo.toml")
        .expect("le manifeste est visé");
    assert!(
        cargo.after.as_deref().expect("le manifeste reste").contains("tokio"),
        "tokio appartient au squelette : un fragment lui ajoute des flags, il ne l'apporte pas"
    );
}

/// La variable d'environnement reste, et le retrait la nomme.
#[test]
fn the_environment_variable_stays_and_is_named() {
    let (projet, fragment) = fragment_pose("mail");
    let mut builder = plan::Builder::new(projet.path());

    let retires = actions(&fragment, &mut builder).expect("le retrait se planifie");

    assert!(retires
        .laissees
        .iter()
        .any(|laissee| laissee.contains("RBS_MAIL__SMTP_PASSWORD")));
}

/// La migration est retrouvée par son suffixe, son horodatage n'étant nulle part gardé.
#[test]
fn the_migration_is_found_by_its_suffix() {
    let projet = projet_avec(&[
        ("migration/src/m20260101_000000_create_auth_tables.rs", "// la migration\n"),
    ]);
    let builder = plan::Builder::new(projet.path());

    let trouvee = migration_de(
        &builder,
        &manifest::DeclaredMigration {
            source: "migration.rs.jinja".to_string(),
            name: "create_auth_tables".to_string(),
        },
    )
    .expect("la recherche aboutit");

    assert_eq!(
        trouvee,
        Some((
            "m20260101_000000_create_auth_tables".to_string(),
            "migration/src/m20260101_000000_create_auth_tables.rs".to_string()
        ))
    );
}

/// Deux migrations de même suffixe arrêtent la commande plutôt que d'en choisir une.
#[test]
fn two_migrations_of_the_same_suffix_stop_the_command() {
    let projet = projet_avec(&[
        ("migration/src/m20260101_000000_create_auth_tables.rs", "// une\n"),
        ("migration/src/m20260202_000000_create_auth_tables.rs", "// deux\n"),
    ]);
    let builder = plan::Builder::new(projet.path());

    migration_de(&builder, &declared("create_auth_tables"))
        .expect_err("l'ambiguïté doit être refusée");
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --lib -- remove::desinstallation`
Expected: FAIL — le module n'existe pas.

- [ ] **Step 3: Implémenter**

`actions` parcourt le manifeste dans cet ordre, miroir d'`installation::actions` :

1. **Les fichiers** — `a_deposer` rendu public à `crate` depuis `add::installation`, ou son jumeau local. Chaque destination est rendue puis `builder.supprimer(&destination, &rendu)`. Un fichier `if_absent` est traité comme les autres : il a été posé ou il ne l'a pas été, et `supprimer` rend `DejaFait` dans le second cas.
2. **La migration** — `migration_de` ; si trouvée, `builder.supprimer(&path, &rendu)` puis, pour chaque `mount::for_migration(&module)`, `builder.retirer_lignes(mount.anchor, &mount.lines)`.
3. **Les ancres** — pour chaque `DeclaredInsertion`, la ligne rendue puis `builder.retirer_lignes`. `before` n'est pas vérifié : il garde une insertion, pas un retrait.
4. **Les dépendances** — pour chaque `DeclaredDependency` absente de `reclamees.dependencies`, `PatchToml::RetirerDependance`. Sinon, une ligne dans `laissees`.
5. **Les features cargo** — pour chaque `(crate, PatchCrate)`, chaque feature absente de `reclamees.cargo[crate]` donne `PatchToml::RetirerFeatureADependance`. La crate elle-même n'est jamais retirée par ce chemin.
6. **Les sections de configuration** — `builder.retirer_section(&section.file, &section.section)`.
7. **Les variables d'environnement** — **rien**, une ligne dans `laissees` par variable.
8. **La métadonnée** — `PatchToml::RetirerFeature(name)`.

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --lib -- remove::`
Expected: PASS.

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/remove/ crates/rbs-cli/src/lib.rs
git commit -m "feat(remove): parcourt à l'envers ce que le manifeste d'un fragment déclare"
```

---

### Task 9: `remove::mod` — les quatre refus

> **Correction du contrôleur (R2).** `Retires.migration` (tâche 8) est un
> `Option<String>` — le chemin du fichier retrouvé ; `Planned.migration` ci-dessous est un
> `bool` — le rapport n'a besoin que de savoir s'il faut avertir. `plan_for` convertit par
> `.is_some()`. Les deux types divergent à dessein.

**Files:**
- Modify: `crates/rbs-cli/src/remove/mod.rs`
- Test: `crates/rbs-cli/src/remove/mod.rs` (module `tests`)

**Interfaces:**
- Consomme : `desinstallation::{actions, reclamees_ailleurs}` (tâche 8), `metadata::cible`, `templates`, `git::garde`.
- Produit :
  ```rust
  pub(crate) struct Options {
      pub feature: String,
      pub directory: PathBuf,
      pub force: bool,
      pub template_dir: Option<PathBuf>,
  }

  pub(crate) struct Planned {
      pub plan: plan::Plan,
      pub description: String,
      pub migration: bool,
      pub laissees: Vec<String>,
      pub deja_absente: bool,
  }

  pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error>;
  ```
  `Error` porte `PasInstallee { feature }`, `PasUnFragment { feature, known }`, `Exigee { feature, dependants }`, plus les `#[from]` d'`add::Error` qui s'appliquent : `Acces`, `Manifest`, `Metadata`, `WorkingTreeSale`, `Unknown`, `Desinstallation`.

Ordre des contrôles : fragment connu → inscrit → dépendants → garde git → plan. La divergence n'est pas un contrôle de cette fonction : elle naît du `Status::Conflit` que `Builder::supprimer` pose, et c'est `plan::application::apply` qui refuse, comme pour toute autre commande.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
/// Un fragment que le projet n'inscrit pas n'a rien à retirer.
#[test]
fn a_fragment_the_project_does_not_record_has_nothing_to_remove() {
    let projet = projet_avec_features(&["health"]);

    let planned = plan_for(&options(projet.path(), "mail")).expect("la commande aboutit");

    assert!(planned.deja_absente);
    assert!(planned.plan.files().is_empty());
}

/// Un CRUD engendré n'est pas un fragment, et la commande le dit.
#[test]
fn a_generated_crud_is_not_a_fragment() {
    let projet = projet_avec_features(&["health", "posts"]);

    let faute = plan_for(&options(projet.path(), "posts")).expect_err("le retrait est refusé");

    assert!(matches!(faute, Error::PasUnFragment { .. }));
}

/// Un fragment qu'un autre exige est refusé, et le dépendant est nommé.
#[test]
fn a_fragment_another_requires_is_refused_and_the_dependant_is_named() {
    let projet = projet_avec_features(&["health", "rate-limit", "mail", "auth"]);

    let faute = plan_for(&options(projet.path(), "mail")).expect_err("le retrait est refusé");

    let Error::Exigee { dependants, .. } = &faute else {
        panic!("attendu Exigee, reçu {faute:?}");
    };
    assert_eq!(dependants, "auth");
}

/// Le dépendant transitif est nommé lui aussi.
#[test]
fn a_transitive_dependant_is_named_too() {
    let projet = projet_avec_features(&["health", "jobs", "rate-limit", "mail", "auth", "webhooks"]);

    let faute = plan_for(&options(projet.path(), "mail")).expect_err("le retrait est refusé");

    let Error::Exigee { dependants, .. } = &faute else {
        panic!("attendu Exigee, reçu {faute:?}");
    };
    assert_eq!(dependants, "auth, webhooks");
}

/// Un second retrait ne fait rien : l'idempotence se juge sur le manifeste.
#[test]
fn a_second_removal_does_nothing() {
    let projet = projet_avec_features(&["health"]);

    assert!(plan_for(&options(projet.path(), "cors"))
        .expect("la commande aboutit")
        .deja_absente);
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --lib -- remove::tests`
Expected: FAIL — `cannot find function 'plan_for' in module 'remove'`.

- [ ] **Step 3: Implémenter**

Les dépendants se calculent en lisant le `feature.toml` de chaque fragment installé et en gardant ceux dont `feature.requires` contient le partant, **puis en propageant jusqu'au point fixe** : `webhooks` exige `auth`, qui exige `mail`, donc retirer `mail` nomme les deux. La fermeture transitive et non le seul cercle direct — nommer `auth` sans nommer `webhooks` enverrait l'utilisateur retirer `auth`, pour se heurter aussitôt à un second refus qu'on savait venir.

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --lib -- remove::`
Expected: PASS.

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/remove/
git commit -m "feat(remove): refuse avant d'écrire ce qu'un dépendant exige encore"
```

---

### Task 10: Le branchement de la commande

> **Correction du contrôleur (R11) — `remove::Error` n'est pas encore classée.**
> La tâche 9 a signalé, sans l'implémenter, que `remove::Error` ne porte ni `Codee` ni
> `Classee` : ni son brief ni celui-ci ne les mentionnaient. **Tu en as besoin** — c'est toi
> qui câbles `--json` et l'appel `echec(&error, error.remedy(), json)` de `lib.rs`, qui
> exigent un code stable en `snake_case` et une famille de sortie. Implémente-les sur le
> modèle d'`add::Error`, et vérifie que chaque variante reçoit le bon code de sortie :
> `Faute` (1) pour ce que le projet porte, `Usage` (2) pour un mauvais appel, `Environnement`
> (3) pour un fichier illisible ou un service injoignable.

> **Correction du contrôleur (R9) — un commentaire qui ment, trouvé par une revue.**
> Le doc-commentaire de `Builder::retirer_section` (`crates/rbs-cli/src/plan/mod.rs`, vers
> la ligne 650) affirme que « les tests de `text::remove_section` en prouvent déjà le
> contrat ». C'est faux et cela a été établi : le wrapper a depuis ses trois tests propres,
> et la couche texte ne prouve ni le statut, ni la propagation d'erreur, ni la projection.
> Tu édites déjà ce voisinage pour retirer les attributs : **corrige ce commentaire au
> passage**, et vérifie qu'aucun autre doc-commentaire de wrapper ne répète la même
> affirmation. Ce dépôt a déjà dû nettoyer une fois des commentaires devenus faux ; ils ne
> doivent pas revenir par cette porte.

> **Correction du contrôleur (R7) — trouvé par une revue, hors de son diff.**
> `Plan::bilan` (`crates/rbs-cli/src/plan/mod.rs`) classe chaque fichier écrit en *créé* ou
> *modifié* selon `before.is_some()`, sans connaître la suppression. Le bilan que tu
> affiches annoncerait donc un fichier **supprimé** comme « modifié ». Corrige `bilan` pour
> compter les suppressions à part, et dis-les dans le bilan de `remove` — un utilisateur à
> qui l'on annonce « 7 fichiers modifiés » après une désinstallation est mal renseigné.

> **Correction du contrôleur (R6).** La tâche 2 a posé deux `#[allow(dead_code)]`
> — sur `Builder::supprimer` (`plan/mod.rs`) et `Effect::Supprimer` (`plan/action.rs`) —
> parce que rien ne les appelait encore. **C'est ta tâche qui câble l'appel : retire-les.**
> Recense-les par `grep -rn 'allow(dead_code)' crates/rbs-cli/src --include='*.rs'` et
> retire-les tous. **Piège à éviter** : ce grep remonte aussi `anchors.rs` vers la ligne
> 1346, où un test préexistant manipule et assertionne la *chaîne littérale*
> `"#[allow(dead_code)]"` — il n'a rien à voir avec notre séquence et **ne doit pas être
> touché**. Au dernier relevé (fin de la tâche 4) les attributs réels étaient au nombre de
> cinq : `plan/mod.rs` sur `supprimer` et `retirer_lignes`, `plan/action.rs` sur
> `Effect::Supprimer` et `Effect::RetirerLignes`, `anchors.rs` sur `retire`. Les tâches 5 à
> 9 en ajouteront d'autres. Un
> `#[allow(dead_code)]` oublié avec un commentaire devenu faux est un défaut que ce dépôt a
> déjà dû nettoyer une fois ; il ne doit pas revenir par cette porte.

> **Correction du contrôleur (R1) — prime sur l'étape 1 ci-dessous.**
> Le test `a_dry_run_writes_nothing` **ne va pas ici** : il appelle `empreinte` et
> `assert_intact`, qui vivent dans `crates/rbs-cli/tests/common/mod.rs` et ne sont
> atteignables que depuis un test d'intégration — `src/lib.rs` n'en a aucun équivalent.
> Il est déplacé en tâche 11. Cette tâche ne garde que le test de parsing clap, et sa
> ligne `Test:` se lit `crates/rbs-cli/src/cli.rs` seul.

**Files:**
- Modify: `crates/rbs-cli/src/cli.rs` (après `Add`, `:92-113`)
- Modify: `crates/rbs-cli/src/lib.rs` (`Commands::Remove` vers `:84`, `fn remove`/`fn remove_in` vers `:551`)
- Test: `crates/rbs-cli/src/cli.rs` (module `tests`), `crates/rbs-cli/src/lib.rs` (module `tests`)

**Interfaces:**
- Consomme : `remove::{plan_for, Options, Planned}` (tâche 9), `appliquer`, `ui`, `plan::{render, json}`.
- Produit : `Commands::Remove { feature, force, dry_run, json, template_dir }` ; `fn remove(...) -> Result<(), remove::Error>` et `fn remove_in(directory, ...)`.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
/// La commande prend les mêmes drapeaux qu'`add`.
#[test]
fn the_removal_takes_the_same_flags_as_the_installation() {
    let parsed = Cli::try_parse_from(["rbs", "remove", "mail", "--force", "--dry-run", "--json"])
        .expect("la commande se parse");

    let Commands::Remove { feature, force, dry_run, json, .. } = parsed.command else {
        panic!("attendu Remove");
    };
    assert_eq!(feature, "mail");
    assert!(force && dry_run && json);
}

/// `--dry-run` n'écrit rien : le projet est intact après la commande.
#[test]
fn a_dry_run_writes_nothing() {
    let projet = projet_avec_fragment("cors");
    let avant = empreinte(projet.path());

    remove_in(projet.path().to_path_buf(), "cors".to_string(), false, true, false, None)
        .expect("la commande aboutit");

    assert_intact(&avant, projet.path(), "un --dry-run ne doit rien écrire");
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --lib -- cli::tests::the_removal_takes_the_same_flags_as_the_installation`
Expected: FAIL — `no variant named 'Remove'`.

- [ ] **Step 3: Implémenter**

```rust
// cli.rs, après Add
/// Retire une feature installée : ses fichiers, ses ancres, sa migration et ses dépendances.
Remove {
    /// Feature à retirer.
    feature: String,

    /// Retire même si un fichier a été modifié, ou si le working tree Git est sale.
    #[arg(long)]
    force: bool,

    /// Affiche le plan sans rien écrire.
    #[arg(long)]
    dry_run: bool,

    /// Rend le plan, ou l'erreur, en un document JSON sur la sortie standard.
    #[arg(long)]
    json: bool,

    /// Répertoire de templates remplaçant celles embarquées.
    #[arg(long)]
    template_dir: Option<PathBuf>,
},
```

`remove_in` suit `add_in` : plan, `deja_absente` court-circuite, affichage du plan, `appliquer`, bilan. Après le bilan, le rapport nomme ce qui reste :

```rust
if planned.migration {
    ui::info(
        "\n  la migration est retirée du projet, mais le schéma garde ses tables : \
         `rbs migrate down` devait passer avant",
    );
}

for laissee in &planned.laissees {
    ui::info(&format!("  {laissee}"));
}

ui::info("\n  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature");
```

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --lib`
Expected: PASS, aucun test en échec.

- [ ] **Step 5: Lint puis commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/cli.rs crates/rbs-cli/src/lib.rs
git commit -m "feat(cli): branche la commande de retrait d'une feature"
```

---

### Task 11: La preuve — un projet qui compile après le retrait

> **Correction du contrôleur (R12) — le test doit prouver plus que la compilation.**
> Une revue a établi qu'un retrait pourtant propre faisait **échouer `rbs doctor`** : la
> feature quittait le manifeste sans que l'inventaire d'`AGENTS.md` soit rafraîchi, et
> `doctor/agents.rs` compare les deux. La tâche 10 l'a corrigé — mais ce test-ci est le seul
> témoin permanent possible, et il ne lançait pas `doctor`.
> **Ajoute-le** : après le retrait, lance `rbs doctor` sur le projet engendré et exige un
> succès. Un projet qui vient de subir un retrait propre doit se diagnostiquer sans faute,
> pas seulement compiler. Sans ce témoin, la garde qu'on vient de poser se redégraderait en
> silence — c'est le défaut que cette séquence a déjà rencontré trois fois.

> **Correction du contrôleur (R1).** Cette tâche reçoit en plus le test
> `a_dry_run_writes_nothing` que la tâche 10 ne pouvait pas porter, **sans `#[ignore]`** :
> il ne compile pas le projet engendré, donc il tourne sur chaque PR. Il pose un projet,
> lance `rbs remove cors --dry-run`, et vérifie par `common::empreinte` puis
> `common::assert_intact` que rien n'a été écrit — même forme que son précédent dans
> `integration_add.rs`.

**Files:**
- Create: `crates/rbs-cli/tests/integration_remove.rs`

**Interfaces:**
- Consomme : la commande entière ; `common::{projet, commiter, empreinte, assert_intact, cible}`.

C'est le seul test qui prouve que `remove` fait son travail : un test unitaire prouve que le moteur sait faire, celui-ci prouve que le projet survit.

- [ ] **Step 1: Écrire le test qui échoue**

```rust
//! Ce que `rbs remove` garantit, éprouvé par la commande telle que l'utilisateur la lance.
//!
//! `#[ignore]` : ce fichier compile le projet engendré, ce qu'aucune PR ne peut se payer
//! sur chaque poussée. C'est pourtant le seul test qui prouve qu'un projet survit au
//! retrait — un plan peut être juste et le projet ne plus compiler.

use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Un projet neuf, `cors` posée, puis retirée : il doit compiler aux deux bouts.
#[test]
#[ignore = "compile le projet engendré"]
fn a_project_still_compiles_once_the_fragment_is_removed() {
    let parent = TempDir::new().expect("le répertoire temporaire se crée");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "projet neuf");

    rbs(&racine).args(["add", "cors"]).assert().success();
    common::commiter(&racine, "cors posée");

    rbs(&racine).args(["remove", "cors"]).assert().success();

    assert!(!racine.join("src/modules/cors").exists(), "les fichiers doivent partir");
    assert!(
        !std::fs::read_to_string(racine.join("src/router.rs"))
            .expect("le routeur se lit")
            .contains("cors::layer"),
        "la ligne d'ancre doit partir"
    );

    cargo_check(&racine).assert().success();
}

/// Un fichier que le développeur a modifié arrête la commande, et rien n'est écrit.
#[test]
#[ignore = "compile le projet engendré"]
fn a_file_the_developer_changed_stops_the_command() {
    let parent = TempDir::new().expect("le répertoire temporaire se crée");
    let racine = common::projet(parent.path());
    rbs(&racine).args(["add", "cors"]).assert().success();
    common::commiter(&racine, "cors posée");

    let touche = racine.join("src/modules/cors/config.rs");
    let source = std::fs::read_to_string(&touche).expect("le fichier se lit");
    std::fs::write(&touche, format!("{source}// une ligne à moi\n")).expect("le fichier s'écrit");
    common::commiter(&racine, "retouche du développeur");

    let avant = common::empreinte(&racine);
    let sortie = rbs(&racine).args(["remove", "cors"]).output().expect("le binaire tourne");

    assert!(!sortie.status.success(), "la divergence doit arrêter la commande");
    common::assert_intact(&avant, &racine, "un refus ne doit rien écrire");

    rbs(&racine).args(["remove", "cors", "--force"]).assert().success();
}

/// L'aller-retour rend un projet qui compile encore.
#[test]
#[ignore = "compile le projet engendré"]
fn the_round_trip_leaves_a_project_that_still_compiles() {
    let parent = TempDir::new().expect("le répertoire temporaire se crée");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "projet neuf");

    rbs(&racine).args(["add", "cors"]).assert().success();
    rbs(&racine).args(["remove", "cors"]).assert().success();
    rbs(&racine).args(["add", "cors"]).assert().success();

    cargo_check(&racine).assert().success();
}
```

- [ ] **Step 2: Lancer et voir échouer**

Run: `cargo test -p rbs-cli --test integration_remove -- --ignored --no-fail-fast`
Expected: FAIL avant l'implémentation complète ; une fois les tâches 1 à 10 faites, ce doit être le premier vert.

- [ ] **Step 3: Corriger ce que le test révèle**

Aucun code à écrire d'avance : ce test est l'oracle. Tout échec ici désigne une tâche précédente à reprendre, et non un correctif local à improviser ici.

- [ ] **Step 4: Lancer et voir passer**

Run: `cargo test -p rbs-cli --test integration_remove -- --ignored --no-fail-fast`
Expected: PASS — 3 passés, 0 échoué. Compter les secondes : ces tests compilent, ils dureront des minutes.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/tests/integration_remove.rs
git commit -m "test(remove): prouve qu'un projet compile encore après le retrait d'une feature"
```

---

### Task 12: La documentation

**Files:**
- Create: `docs/docs/cli/remove.md`, `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/remove.md`
- Modify: `docs/sidebars.ts`, `crates/rbs-cli/templates/agents/{en,fr}.md.jinja`, `docs/docs/cli/completions.md` et sa jumelle, `CHANGELOG.md`, `docs/superpowers/specs/2026-08-25-rbs-design.md` (§4.4)

**Interfaces:**
- Consomme : la commande livrée par les tâches 1 à 11.

- [ ] **Step 1: Écrire les deux pages, avec une transcription gardée**

Le bloc d'aide porte un marqueur, comme ses voisins :

```
{/* rbs:transcript cmd="rbs remove --help" */}
```

Contrainte à connaître : `attribut` (`integration_docs.rs:119`) s'arrête au **premier guillemet double**, un `cmd=` ne peut donc pas en porter.

Les deux pages disent, en toutes lettres : les quatre refus ; que le schéma garde ses tables ; que la variable d'environnement reste ; que `cargo build` est l'oracle des références.

- [ ] **Step 2: Vérifier la parité et le rendu**

```bash
cd docs && npm ci && npm run parite && npm run build
```
Expected: 0 écart structurel ; SUCCESS en `en` et en `fr`.

- [ ] **Step 3: Vérifier que la transcription est réellement gardée**

Run: `cargo test -p rbs-cli --test integration_docs -- --ignored --no-fail-fast`
Expected: PASS. Puis **muter une ligne du bloc** et relancer : la suite doit échouer en nommant le fichier et la ligne. Sans cette preuve rouge, le vert ne prouve qu'une garde décorative.

- [ ] **Step 4: Mettre à jour les guides d'agents, `completions`, le changelog et la spec**

Les deux `AGENTS.md` engendrés listent les commandes : `remove` y rejoint `add`. `completions` énumère les fragments, inchangés. `CHANGELOG.md` reçoit son entrée. §4.4 de la spec de référence reçoit une ligne : `git checkout` n'est plus le seul chemin de retour.

- [ ] **Step 5: Commit**

```bash
git add docs/ crates/rbs-cli/templates/agents/ CHANGELOG.md
git commit -m "docs(cli): documente le retrait d'une feature dans les deux langues"
```

---

## Passe finale

- [ ] `rustup update` — la toolchain locale peut être en retard sur la CI, et un clippy vert en 1.96 ne dit rien de la 1.98.
- [ ] `cargo fmt --all --check` et `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] `cargo test --workspace` puis `cargo test -p rbs-core --all-features` — `--workspace` ne compile pas les `#[cfg(feature = "auth")]` du noyau.
- [ ] `cargo test -p rbs-cli --test integration_examples` — doit passer **sans régénération** : aucune template n'a changé.
- [ ] Les suites lentes, **un job par suite** : une seule commande dépasse les 600 s du shell.
- [ ] Cocher la tâche dans `IMPROVE.md` avec la date et les chiffres réels, puis `superpowers:finishing-a-development-branch`.
