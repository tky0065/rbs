# `rbs generate job <nom> [--every "<cron>"]` — plan d'implémentation

> **Pour les agents :** sous-skill requis : `superpowers:subagent-driven-development`. Les
> étapes se cochent (`- [ ]`).

**But :** une commande qui écrit un job de la file — son fichier, sa déclaration, son
inscription au registre — et, sous `--every`, son échéance au calendrier.

**Architecture :** deux ancres optionnelles nouvelles, `<rbs:job_modules>` (jobs) et
`<rbs:schedules>` (scheduler), portent 14 → 16 le registre `ANCRES`. Un module
`generate/job.rs` suit la séquence lire → planifier → vérifier → afficher → appliquer de
`generate/command.rs`. Les lignes insérées passent par rustfmt, enveloppées dans une fonction
d'emprunt, puisque leur longueur dépend du nom. L'expression de `--every` passe par
`crate::cron::valider` (livré avec les contrôles `doctor`) avant tout plan.

**Pile :** Rust 2024, clap, minijinja (délimiteurs `{@ @}`), rustfmt, `assert_cmd` et
testcontainers pour le test lent.

**Spec :** `docs/superpowers/specs/2026-09-13-lot-features-p2-design.md`, section « 36 ».

## Contraintes globales

- `rbs generate job <nom> [--every "<cron>"] [--dry-run] [--force]`.
- `<nom>` en snake_case, ni mot-clé Rust, ni module de la file (`config`, `demo`, `model`,
  `queue`, `worker`, `tests`).
- Exige `jobs` au manifeste ; `--every` exige `scheduler`. Refus sinon, remède
  `rbs add jobs` / `rbs add scheduler`.
- Écrit : `src/modules/jobs/<nom>.rs` (`#[derive(Debug, Serialize, Deserialize)] pub struct
  <Nom> {}`, `impl Job` avec `KIND = "<nom>"`, `run` qui journalise et rend `Ok(())`, un seul
  commentaire : le point d'extension) ; `pub mod <nom>;` dans `<rbs:job_modules>` ;
  `registre = registre.register::<<nom>::<Nom>>();` dans `<rbs:jobs>` ; sous `--every`,
  `calendrier.push(Schedule::every::<crate::modules::jobs::<nom>::<Nom>>("<expr>", || …));`
  dans `<rbs:schedules>`.
- Ancres `optional: true`, chacune avec une ligne d'accroche unique vérifiée contre sa
  template. `schedules()` réécrite en instructions.
- Projet antérieur sans ces ancres : insertion sautée, bloc à coller affiché ; `doctor` les
  réclame, `doctor --fix` les repose.
- Fichier du job présent → le plan le dit, rien n'est réécrit. Rien dans
  `[package.metadata.rbs].features`.
- Commits : Conventional Commits en français, sans identifiant de tâche, sans mention du
  backlog ni d'un outil, sans `Co-Authored-By` ni `Claude-Session`.
- Bloquants : fmt, clippy `-D warnings`, `cargo test --workspace`, `integration_examples`
  (templates touchées), `npm run build` (exemples ou doc touchés).

## Décisions prises en rédigeant ce plan

1. **`#[allow(clippy::vec_init_then_push)]` sur `schedules()`.** Mesuré sur une crate
   jetable : `Vec::new()` suivi de `push` déclenche ce lint (groupe `perf`) ; `vec![démo]`
   suivi de l'ancre vide déclenche `unused_mut`. La permission, justifiée en commentaire, est
   la seule forme qui passe clippy dans tous les états — ancre vide, remplie, démo retirée.
2. **Accroche de `schedules` = `let mut calendrier = Vec::new();`**, l'ancre juste dessous et
   l'échéance de démonstration après elle : l'accroche survit au retrait de la démo, ce que
   celle de `jobs` (la ligne de `demo::Log`) ne fait pas.
3. **Accroche de `job_modules` = `pub mod worker;`**, ancre triée : rustfmt ne réordonne pas
   les `pub mod` à travers une ligne de commentaire (mesuré), mais ordonne ceux du bloc.
4. **Ancre absente d'un fichier présent → insertion sautée** (nouvelle cause de `Sautee`) :
   c'est ce que la spec demande pour « un projet antérieur sans ces ancres ». Le mécanisme
   existant ne couvrait que le fichier absent ; la cause est distinguée à l'affichage.
5. **Refus supplémentaires, conservateurs**, parce qu'ils rendraient sinon un projet qui ne
   compile pas ou un calendrier faux : `pub mod <nom>;` déjà déclaré hors de l'ancre ; un
   fichier `<nom>.rs` qui ne définit pas `impl Job for <Nom>` ; un `KIND = "<nom>"` déjà pris
   dans `src/` ; une seconde échéance pour un job déjà planifié avec une autre expression
   (le calendrier se clé par `kind`, `sync.rs.jinja:44`).

---

## Fichiers

| Fichier | Rôle |
|---|---|
| `crates/rbs-cli/src/anchors.rs` | `JOB_MODULES`, `SCHEDULES`, `ANCRES: [Anchor; 16]`, tests |
| `crates/rbs-cli/templates/features/jobs/mod.rs.jinja` | ancre `job_modules` sous `pub mod worker;` |
| `crates/rbs-cli/templates/features/scheduler/mod.rs.jinja` | `schedules()` en instructions, ancre `schedules` |
| `crates/rbs-cli/src/doctor/anchors.rs` | comptes des tests, réparation des deux ancres |
| `crates/rbs-cli/src/plan/mod.rs`, `plan/render.rs` | `Sautee::cause`, `Builder::insert_ou_sauter` |
| `crates/rbs-cli/src/generate/format.rs` | `instructions(source) -> Result<Vec<String>, Avertissement>` |
| `crates/rbs-cli/src/generate/name.rs` | `validate_identifier` (le nom sans la règle des modules du squelette) |
| `crates/rbs-cli/templates/job/job.rs.jinja` | le fichier du job |
| `crates/rbs-cli/src/generate/job.rs` | `Options`, `Planned`, `Error`, `plan_for` |
| `crates/rbs-cli/src/cli.rs`, `src/lib.rs` | `GenerateCommands::Job`, `generate_job` |
| `crates/rbs-cli/tests/integration_scheduler.rs` | test lent : `generate job purge --every` compile et passe |
| `examples/event-hub`, `examples/newsletter-queue` | ancres reportées (régions restaurées) |
| `CLAUDE.md`, `docs/…` | quatorze → seize, section `generate job`, guides jobs et scheduler |

---

### Tâche 1 : les deux ancres, leurs templates et les exemples

**Interfaces :** produit `anchors::JOB_MODULES` et `anchors::SCHEDULES` (`Anchor`, noms
`job_modules` et `schedules`), inscrites dans `ANCRES`.

- [ ] **Étape 1 : tests (rouges).** Dans `anchors.rs` :
  `only_the_anchors_of_a_fragment_deposited_file_are_optional` attend
  `["modules", "services", "jobs", "job_modules", "schedules"]` ; le test d'accroche de
  `JOBS` devient une boucle sur `(JOBS, "jobs/mod.rs.jinja")`, `(JOB_MODULES,
  "jobs/mod.rs.jinja")`, `(SCHEDULES, "scheduler/mod.rs.jinja")` : balises présentes une
  fois, accroche présente une fois, **la ligne qui suit l'accroche est la balise ouvrante**
  (c'est ce qui rend `--fix` exact à l'octet). Dans `doctor/anchors.rs` :
  `ANCRES.len() - 2` → `- 4`, `- 3` → `- 5`, commentaires suivis ; nouveau test :
  sur `fixtures::Project::new().features(&["jobs", "scheduler"])`, chacune des deux ancres
  retirée puis `repair` → `reposees == [nom]`, fichier revenu à l'octet.

- [ ] **Étape 2 : constantes.**

```rust
pub(crate) const JOB_MODULES: Anchor = Anchor {
    name: Cow::Borrowed("job_modules"),
    file: Cow::Borrowed("src/modules/jobs/mod.rs"),
    comment: "//",
    sorted: true,
    optional: true,
    after: "pub mod worker;",
};

pub(crate) const SCHEDULES: Anchor = Anchor {
    name: Cow::Borrowed("schedules"),
    file: Cow::Borrowed("src/modules/scheduler/mod.rs"),
    comment: "//",
    sorted: false,
    optional: true,
    after: "let mut calendrier = Vec::new();",
};
```

`ANCRES: [Anchor; 16]`, les deux en fin. Le commentaire « pour les treize » de `resolved`
perd son nombre.

- [ ] **Étape 3 : templates.** `jobs/mod.rs.jinja` : `// <rbs:job_modules>` /
  `// </rbs:job_modules>` sous `pub mod worker;`. `scheduler/mod.rs.jinja` :

```rust
/// Les échéances de ce projet. Déclarez les vôtres ici.
///
/// Les expressions sont évaluées **en UTC** : `0 3 * * *` est 3 h UTC.
///
/// En instructions et non en `vec![]`, pour la raison que `jobs::registry` donne : une ancre
/// posée dans un `vec![]` ne survit pas à rustfmt dès qu'un second élément s'y ajoute. Clippy
/// préférerait le `vec![]`, seule forme que l'ancre ne peut pas porter.
#[allow(clippy::vec_init_then_push)]
pub fn schedules() -> Vec<Schedule> {
    let mut calendrier = Vec::new();
    // <rbs:schedules>
    // </rbs:schedules>
    calendrier.push(Schedule::every::<crate::modules::jobs::demo::Log>(
        "0 3 * * *",
        || crate::modules::jobs::demo::Log {
            message: "échéance quotidienne".to_string(),
        },
    ));
    calendrier
}
```

La forme exacte du `push` est celle que rustfmt rend : la garde
`templates::tests::each_rust_template_of_each_fragment_conforms_to_rustfmt` tranche.

- [ ] **Étape 4 : littéraux figés.** `grep -rn 'schedules()\|vec!\[Schedule\|register::<demo' crates/rbs-cli/src crates/rbs-cli/tests`
  et adapter ce qui fige l'ancienne forme.

- [ ] **Étape 5 : exemples.** `event-hub` : `src/modules/jobs/mod.rs` reçoit l'ancre ;
  `src/modules/scheduler/mod.rs` reçoit la nouvelle `schedules()`, entre
  `// region: schedules` et `// endregion: schedules`. `newsletter-queue` (fichier édité à la
  main) : l'ancre sous `pub mod worker;`. Puis `cargo check --all-targets` et
  `cargo clippy --all-targets -- -D warnings` dans chacun des deux, et
  `cargo test -p rbs-cli --test integration_examples -- --include-ignored`.

- [ ] **Étape 6 : vérifier** : `cargo test -p rbs-cli --lib` **entier** (la garde rustfmt
  des fragments vit dans `templates::tests`), fmt, clippy. **Commit**
  `feat(anchors): ouvre deux ancres aux jobs engendrés et à leurs échéances`.

### Tâche 2 : insertion sautée sur ancre absente

**Interfaces :** produit `plan::CauseSautee::{FichierAbsent, AncreAbsente}`, champ
`Sautee::cause`, et `Builder::insert_ou_sauter(&mut self, anchor: Anchor, lines: &[String]) -> Result<(), Error>`.

- [ ] **Étape 1 : tests (rouges)** dans `plan/mod.rs` : ancre optionnelle, fichier présent
  sans l'ancre → `Ok`, une `Sautee` de cause `AncreAbsente`, fichier non touché ; ancre
  obligatoire dans le même cas → `Err(Error::Anchor)` ; fichier absent → cause
  `FichierAbsent`. Dans `plan/render.rs` : l'annonce d'une `AncreAbsente` nomme la balise et
  `rbs doctor --fix`.

- [ ] **Étape 2 : implémentation.** `insert_ou_sauter` délègue à `insert` ; sur
  `Err(Error::Anchor(_))` pour une ancre `optional`, pousse `Sautee { anchor, lines, cause:
  CauseSautee::AncreAbsente }`. `insert` renseigne `cause: FichierAbsent`. Rendu :
  « {fichier} ne porte pas `{balise}` : le bloc qui lui était destiné est à reporter
  vous-même — `rbs doctor --fix` repose l'ancre ».

- [ ] **Étape 3 : vérifier, commit** `feat(plan): saute l'insertion dans une ancre optionnelle disparue`.

### Tâche 3 : rustfmt sur des instructions isolées

**Interfaces :** produit `generate::format::instructions(source: &str) -> Result<Vec<String>, Avertissement>`.

- [ ] **Étape 1 : tests (rouges).** Une ligne courte ressort telle quelle ; un appel de 110
  colonnes ressort sur plusieurs lignes, sans l'indentation de l'enveloppe, la continuation
  indentée de quatre ; une source refusée par rustfmt → `Err`.

- [ ] **Étape 2 : implémentation.** Envelopper dans `fn f() {\n{source}\n}\n`, passer par
  `formatted`, retirer la première et la dernière ligne, ôter quatre espaces de tête. Le
  corps de l'enveloppe a l'indentation de `registry()` et de `schedules()`.

- [ ] **Étape 3 : vérifier, commit** avec la tâche 4 (sans appelant, la fonction serait du
  code mort).

### Tâche 4 : `generate/job.rs`

**Interfaces :**
- Consomme : `crate::cron::valider`, `anchors::{JOBS, JOB_MODULES, SCHEDULES}`,
  `plan::Builder::insert_ou_sauter`, `format::{instructions, format_batch}`,
  `name::validate_identifier`, `fields::to_pascal_case`.
- Produit : `job::Options { name: String, every: Option<String>, directory: PathBuf, force: bool }`,
  `job::Planned { plan: plan::Plan, fichier: String, avertissement: Option<Avertissement>, echeance: Option<String> }`,
  `job::plan_for(&Options) -> Result<Planned, job::Error>`, `job::Error::remedy(&self) -> Option<String>`.

- [ ] **Étape 1 : `name::validate_identifier`** : vide, snake_case, mot-clé ; `validate`
  l'appelle puis ajoute la règle des modules du squelette. Tests existants inchangés.

- [ ] **Étape 2 : tests de `job.rs` (rouges)**, sur `fixtures::Project::new().features(&["jobs", "scheduler"])` :
  nom en PascalCase refusé ; `demo` refusé ; `match` refusé ; projet sans `jobs` → message
  portant `rbs add jobs` ; `--every` sans `scheduler` → `rbs add scheduler` ; `--every "0 99 * * *"`
  refusé, rien d'écrit ; plan attendu (fichier créé, trois fichiers modifiés, lignes exactes
  dans les ancres, rien dans `features`) ; appliqué deux fois → second plan tout « inchangé » ;
  job déjà édité à la main → « inchangé », contenu intact ; ancre `job_modules` retirée →
  `Sautee` `AncreAbsente` et fichier du job créé ; `pub mod log;` hors ancre → refus ;
  `KIND = "log"` de `demo.rs` → `generate job log` refusé ; seconde `--every` différente →
  refus ; le fichier rendu est déjà ce que rustfmt écrirait.

- [ ] **Étape 3 : template** `templates/job/job.rs.jinja` (variables `nom`, `type`) :

```rust
use serde::{Deserialize, Serialize};

use super::Job;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct {@ type @} {}

#[async_trait::async_trait]
impl Job for {@ type @} {
    const KIND: &'static str = "{@ nom @}";

    // Le point d'extension : ce que fait le job. Une erreur rendue ici vaut réessai.
    async fn run(&self, _state: &AppState) -> anyhow::Result<()> {
        tracing::info!("job `{@ nom @}`");

        Ok(())
    }
}
```

- [ ] **Étape 4 : `job.rs`.** Ordre : cible, garde Git (sauf `--force`), nom, modules de la
  file, `jobs`, `scheduler` si `--every`, `cron::valider`, `pub mod` hors ancre, fichier
  étranger, `KIND` pris, échéance existante ; puis rendu, `format_batch`, plan :
  `create(chemin, contenu ou existant)`, `insert_ou_sauter(JOB_MODULES, ["pub mod <nom>;"])`,
  `insert_ou_sauter(JOBS, instructions("registre = registre.register::<<nom>::<Nom>>();"))`,
  et sous `--every`, `insert_ou_sauter(SCHEDULES, instructions("calendrier.push(Schedule::every::<crate::modules::jobs::<nom>::<Nom>>(<expr en Debug>, || crate::modules::jobs::<nom>::<Nom> {}));"))`.
  rustfmt absent → lignes brutes et avertissement, comme `format_batch`.

- [ ] **Étape 5 : vérifier, commit** `feat(generate): écrit un job de la file et son échéance`.

### Tâche 5 : la commande

- [ ] **Étape 1 : tests (rouges)** dans `cli.rs` : `rbs generate job purge --every "0 4 * * *" --dry-run`
  se lit ; l'aide de `generate` nomme `job`.
- [ ] **Étape 2 : `GenerateCommands::Job { name, every, force, dry_run }`** (« Génère un job
  de la file, et son échéance sous --every ; exige la feature jobs ») et `generate_job` dans
  `lib.rs` : plan affiché, avertissement, `appliquer`, succès « job <nom> écrit — N fichiers »,
  puis sous `--every` « l'échéance « <expr> » est relue au prochain démarrage, en UTC ».
- [ ] **Étape 3 : vérifier, commit** `feat(cli): expose rbs generate job`.

### Tâche 6 : test lent

- [ ] Dans `tests/integration_scheduler.rs`, `#[ignore]` :
  `a_generated_job_with_its_schedule_compiles_and_passes_the_project_tests` — PostgreSQL,
  `project_with_scheduler_on("postgres", …)`, commit, `rbs generate job purge --every "0 4 * * *"`,
  `migrate up`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace -- --include-ignored` dans le projet : trois succès.
- [ ] Lancé seul, en arrière-plan, sortie vers `…/scratchpad/g1-<nom>.txt`, `--no-fail-fast`.
- [ ] Commit `test(generate): compile un projet portant un job engendré et son échéance`.

### Tâche 7 : documentation

- [ ] `CLAUDE.md` : « quatorze » → « seize », deux lignes de table, `generate crud` six
  ancres et `generate job` trois, cinq optionnelles.
- [ ] EN+FR : `compatibility.md` (seize, quinze en Rust), `cli/doctor.md` (liste des ancres,
  cinq optionnelles, « les 15 points » là où `jobs` est installé), `cli/generate.md`
  (transcript de `--help`, section `rbs generate job`, compte des ancres), `cli/add.md`,
  `cli/new.md`, `getting-started.md` (prose des ancres qui sortent du compte),
  `guides/jobs.md` et `guides/scheduler.md` (la commande, la forme en instructions, le cas
  d'un projet antérieur).
- [ ] `integration_docs` rapide puis `--ignored` (le transcript `doctor` sous `jobs` est
  `base="oui"`), `npm run build`.
- [ ] Commit `docs: décrit rbs generate job et les deux ancres qu'il emploie`.

## Auto-relecture

- Couverture : commande (T4-T5), validation du nom et des features (T4), `--every` validé
  avant tout plan (T4), fichier du job (T4), trois insertions (T4), ancres 14 → 16 et
  accroches vérifiées (T1), ancre absente → bloc sauté (T2, T4), `doctor --fix` (T1),
  idempotence (T4), rien dans `features` (T4), test lent (T6), exemples et docs (T1, T7).
- Noms constants d'une tâche à l'autre : `insert_ou_sauter`, `CauseSautee`, `instructions`,
  `validate_identifier`, `JOB_MODULES`, `SCHEDULES`.
