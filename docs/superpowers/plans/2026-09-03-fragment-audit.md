# Fragment `audit` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs add audit` installe un journal des écritures — table, entrée, fonction — dont la trace est committée avec le changement qu'elle décrit.

**Architecture:** Un onzième fragment de `crates/rbs-cli/templates/features/`, moulé sur `jobs`. Il dépose six fichiers sous `src/audit/` du projet engendré, une migration, et une seule ancre (`features`). L'API tient en `record(&impl ConnectionTrait, Entry)`, sur le contrat exact d'`enqueue`. Aucune dépendance à `auth`, aucune route, aucune configuration.

**Tech Stack:** Rust, minijinja (**délimiteurs alternatifs** : `{@ … @}` pour les expressions, `{% … %}` pour les blocs), SeaORM, `include_dir`, `assert_cmd` + `testcontainers` pour l'intégration.

**Spec:** `docs/superpowers/specs/2026-09-03-fragment-audit-design.md` — à lire en entier avant la tâche 1.

## Global Constraints

- **Worktree** : tout se fait dans `.claude/worktrees/audit`, branche `feature/fragment-audit`. Ne jamais commiter sur `main`.
- **Commits** : Conventional Commits, sujet **en français**, à l'impératif, sans majuscule initiale ni point final. **Aucune** ligne `Co-Authored-By`, aucune mention d'un assistant. Corps = le pourquoi technique + un intertitre `Vérifications :` portant les commandes lancées et leur résultat réel.
- **Commentaires** : expliquent le *pourquoi*, jamais le *quoi*. Un commentaire qui paraphrase la ligne suivante se supprime.
- **Frontière des couches** : `repository.rs` est le seul fichier du fragment qui construit une requête SeaORM.
- **Lint bloquant** : `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester sans sortie.
- **Point fixe rustfmt** : le rendu d'une template doit être exactement ce que `rustfmt` écrirait. Un blanc perdu par un `-%}` ne se voit qu'ici.
- **Documentation bilingue** : toute page anglaise modifiée l'est aussi en français, `docs/i18n/fr/docusaurus-plugin-content-docs/current/…`, **dans le même commit**.
- **Interdit** : ajouter une tâche au backlog pour un problème rencontré en chemin. Un défaut trouvé se corrige dans le lot, et se dit dans le rapport.

---

### Task 1 : le fragment nu — table, entrée, écriture

**Files:**
- Create: `crates/rbs-cli/templates/features/audit/feature.toml`
- Create: `crates/rbs-cli/templates/features/audit/model.rs.jinja`
- Create: `crates/rbs-cli/templates/features/audit/migration.rs.jinja`
- Create: `crates/rbs-cli/templates/features/audit/mod.rs.jinja`
- Create: `crates/rbs-cli/templates/features/audit/repository.rs.jinja`
- Test: `crates/rbs-cli/src/templates.rs` (module `tests` en fin de fichier)

**Interfaces:**
- Consumes: rien.
- Produces: le module `audit` du projet engendré, dont les tâches 2 et 4 dépendent —
  - `pub const CREATE: &str = "create";`, `UPDATE = "update"`, `DELETE = "delete"`
  - `pub struct Entry { action: String, entity: String, entity_id: String, actor_id: Option<String>, changes: serde_json::Value }`
  - `impl Entry { pub fn new(action: impl Into<String>, entity: impl Into<String>, entity_id: impl Into<String>) -> Self; pub fn actor(self, actor_id: impl Into<String>) -> Self; pub fn changes(self, changes: serde_json::Value) -> Self }`
  - `pub async fn record<C: ConnectionTrait>(db: &C, entry: Entry) -> anyhow::Result<Uuid>` (réexporté depuis `repository`)
  - `model::Entity`, `model::Model`, `model::ActiveModel`

- [ ] **Step 1 : lire le moule avant d'écrire une ligne**

Lire intégralement, dans cet ordre : `templates/features/jobs/feature.toml`, `jobs/model.rs.jinja`, `jobs/migration.rs.jinja`, `jobs/queue.rs.jinja` (seulement `enqueue` et `enqueue_at`, lignes 80-115). Le fragment `audit` est ce moule avec une table plus simple et sans dépilage.

- [ ] **Step 2 : écrire le test CLI qui échoue**

Dans le module `tests` de `crates/rbs-cli/src/templates.rs`, à côté de `the_jobs_fragment_carries_both_anchors` :

```rust
#[test]
fn the_audit_fragment_declares_its_migration_and_its_single_anchor() {
    let source = read(&Path::new(RACINE_FEATURES).join("audit/feature.toml"));
    let manifest = crate::manifest::read(&source, "audit/feature.toml")
        .expect("le manifeste du fragment audit doit se lire");

    let migration = manifest
        .migration
        .as_ref()
        .expect("le fragment pose une table, il doit donc porter une migration");
    assert_eq!(migration.name, "create_audit_log");

    // Le fragment n'expose ni route, ni layer, ni tâche de fond : une ancre de plus
    // serait le signe qu'il en fait plus que ce que la spec lui donne à faire.
    let ancres: Vec<&str> = manifest
        .anchors
        .iter()
        .map(|ancre| ancre.anchor.as_str())
        .collect();
    assert_eq!(ancres, ["features"]);

    assert!(
        manifest.feature.requires.is_empty(),
        "le fragment ne dépend pas de `auth` — c'est ce qui le rend installable sans JWT"
    );
}
```

Adapter les noms de champs (`manifest.migration`, `manifest.anchors`) à ce que `crates/rbs-cli/src/manifest.rs` déclare réellement : **le lire avant d'écrire ce test**, et corriger l'appel plutôt que la structure.

- [ ] **Step 3 : voir le test échouer**

Run: `cargo test -p rbs-cli --lib the_audit_fragment`
Expected: FAIL — le fichier `audit/feature.toml` n'existe pas encore.

- [ ] **Step 4 : écrire `feature.toml`**

```toml
[feature]
description = "journal des écritures : qui a modifié quoi, quand, dans la transaction du changement"

[[files]]
source      = "mod.rs.jinja"
destination = "src/audit/mod.rs"

[[files]]
source      = "model.rs.jinja"
destination = "src/audit/model.rs"

[[files]]
source      = "repository.rs.jinja"
destination = "src/audit/repository.rs"

[[files]]
source      = "tests.rs.jinja"
destination = "src/audit/tests.rs"

[[anchors]]
anchor  = "features"
content = "pub mod audit;"

[migration]
source = "migration.rs.jinja"
name   = "create_audit_log"

# `changes` est une valeur JSON, que sea-orm ne sait lire que sous cette feature.
[cargo.sea-orm]
features = ["with-json"]
```

`tests.rs.jinja` est déclaré ici mais écrit en tâche 2 : le déclarer maintenant évite un second passage dans le manifeste. **La tâche 1 doit donc créer un `tests.rs.jinja` au moins vide** (un fichier absent fait échouer l'installation), qui sera rempli en tâche 2.

- [ ] **Step 5 : écrire `migration.rs.jinja`**

Sur le modèle exact de `jobs/migration.rs.jinja`, table `audit_log`, avec ces colonnes et rien d'autre :

| Colonne | Déclaration sea-query |
|---|---|
| `Id` | `.uuid().not_null().primary_key()` |
| `ActorId` | `.text().null()` |
| `Action` | `.text().not_null()` |
| `Entity` | `.text().not_null()` |
| `EntityId` | `.text().not_null()` |
| `Changes` | `.json().not_null()` |
| `CreatedAt` | `.timestamp_with_time_zone().not_null().default(Expr::current_timestamp())` |

Puis deux index, `if_not_exists`, nommés `idx_audit_log_entity` sur `(Entity, EntityId)` et `idx_audit_log_created_at` sur `(CreatedAt)`. Le `down` fait un `drop_table`.

Attention : `Entity` est un nom de variante du `#[derive(DeriveIden)] enum AuditLog`, il n'entre pas en conflit avec `model::Entity` — les deux vivent dans des fichiers différents.

Un commentaire, un seul, sur les index : sans le premier, le coût d'une lecture croît avec le journal entier, et un journal est fait pour grossir.

- [ ] **Step 6 : écrire `model.rs.jinja`**

Sur le modèle de `jobs/model.rs.jinja`, sans enum de statut :

```rust
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "audit_log")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// L'auteur de l'écriture, ou rien : un job, un seed et une commande n'ont pas
    /// d'identité HTTP, et ce sont précisément les écritures qu'on cherche à expliquer.
    pub actor_id: Option<String>,
    pub action: String,
    /// La table visée.
    pub entity: String,
    /// La clé de la ligne visée, telle qu'elle s'écrit — une clé entière ou composite
    /// doit rester citable.
    pub entity_id: String,
    /// Ce qui a changé. Le fragment n'impose aucun schéma.
    pub changes: Json,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:audit_log>
    // </rbs:relations:audit_log>
}

// <rbs:related:audit_log>
// </rbs:related:audit_log>

/// L'identifiant est posé ici, et non par un défaut de colonne : `uuidv7()` n'a
/// d'équivalent à écrire ni en MySQL ni en SQLite.
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::now_v7()),
            ..ActiveModelTrait::default()
        }
    }
}
```

Vérifier dans `jobs/model.rs.jinja` la forme exacte des ancres `relations:`/`related:` et le nom qu'elles portent (`jobs` y est le nom du module, pas de la table) — **reprendre la convention observée**, pas celle écrite ci-dessus si elle diverge.

- [ ] **Step 7 : écrire `repository.rs.jinja`**

```rust
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, ConnectionTrait};

use super::Entry;
use super::model::ActiveModel;

/// Inscrit une entrée au journal et rend l'identifiant de sa ligne.
///
/// `db` est un `ConnectionTrait` et non une connexion, et c'est toute la raison d'avoir
/// mis le journal en base : une transaction en est un. Passez-lui celle du métier, et la
/// trace naît si et seulement si le changement qu'elle décrit est committé.
pub async fn record<C>(db: &C, entry: Entry) -> anyhow::Result<Uuid>
where
    C: ConnectionTrait,
{
    // L'identifiant et la date viennent des défauts : ce que l'appelant n'a pas à
    // choisir, il n'a pas à l'écrire.
    let ligne = ActiveModel {
        actor_id: Set(entry.actor_id),
        action: Set(entry.action),
        entity: Set(entry.entity),
        entity_id: Set(entry.entity_id),
        changes: Set(entry.changes),
        ..Default::default()
    };

    Ok(ligne.insert(db).await?.id)
}
```

- [ ] **Step 8 : écrire `mod.rs.jinja`**

```rust
pub mod model;
pub mod repository;

#[cfg(test)]
mod tests;

// Réexportée pour que le projet écrive `audit::record(&transaction, entry)` : tant
// qu'aucun service ne le fait, le compilateur la tient pour inutile.
#[allow(unused_imports)]
pub use repository::record;

/// Les trois actions du CRUD, nommées une fois pour ne pas les réécrire à chaque appel.
///
/// Ce sont des constantes et non un enum : l'ensemble est ouvert, et un `login` ou un
/// `export` sont des actions légitimes qu'un enum fermé forcerait à contourner.
pub const CREATE: &str = "create";
pub const UPDATE: &str = "update";
pub const DELETE: &str = "delete";

/// Une écriture à inscrire au journal.
#[derive(Debug, Clone)]
pub struct Entry {
    pub action: String,
    pub entity: String,
    pub entity_id: String,
    pub actor_id: Option<String>,
    pub changes: serde_json::Value,
}

impl Entry {
    /// Les trois champs sans lesquels une ligne de journal ne veut rien dire.
    pub fn new(
        action: impl Into<String>,
        entity: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        Self {
            action: action.into(),
            entity: entity.into(),
            entity_id: entity_id.into(),
            actor_id: None,
            changes: serde_json::Value::Null,
        }
    }

    /// L'auteur de l'écriture. Sous `auth`, c'est `identity.user_id`.
    pub fn actor(mut self, actor_id: impl Into<String>) -> Self {
        self.actor_id = Some(actor_id.into());
        self
    }

    /// Ce qui a changé, sous la forme que le projet décide.
    pub fn changes(mut self, changes: serde_json::Value) -> Self {
        self.changes = changes;
        self
    }
}
```

- [ ] **Step 9 : voir le test passer**

Run: `cargo test -p rbs-cli --lib the_audit_fragment`
Expected: PASS.

- [ ] **Step 10 : vérifier que le rendu est un point fixe de rustfmt**

Run: `cargo test -p rbs-cli --lib`
Expected: PASS, y compris le balayage existant qui compare le rendu de chaque fragment à ce que `rustfmt` écrirait. S'il n'existe pas de balayage couvrant les fichiers d'un fragment, le vérifier à la main :

```bash
cargo run -p rbs-cli --bin rbs -- new /tmp/probe-audit --database postgres --yes
cd /tmp/probe-audit && cargo run -p rbs-cli --bin rbs -- add audit && cargo fmt --check
```

Toute divergence est un blanc de template, pas un défaut de `rustfmt` : la corriger dans le `.jinja`.

- [ ] **Step 11 : commit**

```bash
git add crates/rbs-cli/templates/features/audit crates/rbs-cli/src/templates.rs
git commit
```

Sujet : `feat(audit): pose la table du journal et l'écriture qui la remplit`.

---

### Task 2 : les tests livrés au projet

**Files:**
- Modify: `crates/rbs-cli/templates/features/audit/tests.rs.jinja` (créé vide en tâche 1)

**Interfaces:**
- Consumes: `Entry`, `record`, `model::Entity` de la tâche 1.
- Produces: quatre noms de tests que la tâche 4 exigera **nommément** —
  `an_entry_written_in_a_rolled_back_transaction_does_not_exist`,
  `an_entry_reads_back_with_every_field_it_was_given`,
  `an_entry_without_an_actor_stores_null_rather_than_an_empty_string`,
  `the_entries_of_one_row_read_back_in_order`.

- [ ] **Step 1 : lire le modèle**

Lire `templates/features/jobs/tests.rs.jinja` en entier. Deux choses à en reprendre : la fonction `table_a_soi()` qui prend un verrou et vide la table (les tests partagent une base), et l'attribut `#[ignore = "joint la base du projet"]` sur tout test qui l'interroge.

- [ ] **Step 2 : écrire les quatre tests**

```rust
use rbs_core::HasCoreState;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, TransactionTrait};
use tokio::sync::{Mutex, MutexGuard};

use std::sync::OnceLock;

use super::model::{Column, Entity};
use super::{Entry, record};
use crate::state::AppState;

/// Les tests partagent l'unique table `audit_log` : ils se relaient plutôt que de se
/// voler leurs lignes.
async fn table_a_soi() -> (MutexGuard<'static, ()>, AppState) {
    static VERROU: OnceLock<Mutex<()>> = OnceLock::new();

    let garde = VERROU.get_or_init(Mutex::default).lock().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    Entity::delete_many()
        .exec(&db)
        .await
        .expect("la table audit_log doit se vider");

    (garde, AppState::new(db, config).expect("état constructible"))
}

/// Le critère qui justifie d'avoir mis le journal en base plutôt que dans un fichier.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_entry_written_in_a_rolled_back_transaction_does_not_exist() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let transaction = db.begin().await.expect("transaction ouvrable");
    let id = record(
        &transaction,
        Entry::new(super::UPDATE, "posts", "annulée").actor("ada"),
    )
    .await
    .expect("l'entrée s'inscrit dans la transaction");
    transaction.rollback().await.expect("transaction annulable");

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible");

    assert!(
        ligne.is_none(),
        "la trace a survécu au rollback du changement qu'elle décrit"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_entry_reads_back_with_every_field_it_was_given() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let changes = serde_json::json!({ "title": { "from": "a", "to": "b" } });
    let id = record(
        db,
        Entry::new(super::UPDATE, "posts", "42")
            .actor("ada")
            .changes(changes.clone()),
    )
    .await
    .expect("l'entrée s'inscrit");

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("la ligne inscrite doit se relire");

    assert_eq!(ligne.actor_id.as_deref(), Some("ada"));
    assert_eq!(ligne.action, "update");
    assert_eq!(ligne.entity, "posts");
    assert_eq!(ligne.entity_id, "42");
    assert_eq!(ligne.changes, changes);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_entry_without_an_actor_stores_null_rather_than_an_empty_string() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let id = record(db, Entry::new(super::DELETE, "posts", "42"))
        .await
        .expect("l'entrée s'inscrit sans acteur");

    let ligne = Entity::find_by_id(id)
        .one(db)
        .await
        .expect("lecture possible")
        .expect("la ligne inscrite doit se relire");

    // La distinction porte : une chaîne vide dirait « un acteur anonyme », `NULL` dit
    // « aucune identité HTTP », ce qui est le cas d'un job ou d'un seed.
    assert!(ligne.actor_id.is_none(), "{:?}", ligne.actor_id);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_entries_of_one_row_read_back_in_order() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    for action in [super::CREATE, super::UPDATE, super::DELETE] {
        record(db, Entry::new(action, "posts", "42"))
            .await
            .expect("l'entrée s'inscrit");
    }
    // Une ligne voisine ne doit pas entrer dans l'histoire de celle-ci.
    record(db, Entry::new(super::CREATE, "posts", "43"))
        .await
        .expect("l'entrée s'inscrit");

    let histoire = Entity::find()
        .filter(Column::Entity.eq("posts"))
        .filter(Column::EntityId.eq("42"))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(db)
        .await
        .expect("lecture possible");

    let actions: Vec<&str> = histoire.iter().map(|ligne| ligne.action.as_str()).collect();
    assert_eq!(actions, ["create", "update", "delete"]);
}
```

Le second tri, sur `Id`, n'est pas décoratif : `created_at` est tronqué à la seconde par MySQL, et trois entrées écrites dans la même seconde n'auraient sinon aucun ordre défini. L'UUIDv7 est monotone, il tranche.

- [ ] **Step 3 : vérifier que le rendu compile — sans Docker**

Un test `#[ignore]` doit quand même compiler. Le prouver **avant** la passe lente, sur un projet jetable :

```bash
cargo run -p rbs-cli --bin rbs -- new /tmp/probe-audit2 --database postgres --yes
cd /tmp/probe-audit2 && cargo run -p rbs-cli --bin rbs -- add audit && cargo check --all-targets
```

Expected: `cargo check` en succès. Le code d'un fragment n'est compilé nulle part ailleurs avant la passe Docker ; sauter cette étape fait découvrir une faute de frappe une heure plus tard.

- [ ] **Step 4 : `cargo fmt --check` sur ce même projet**

Run: `cd /tmp/probe-audit2 && cargo fmt --check`
Expected: aucune sortie.

- [ ] **Step 5 : commit**

Sujet : `test(audit): livre au projet les quatre tests du journal`.

---

### Task 3 : ce que le CLI doit apprendre

**Files:**
- Modify: `crates/rbs-cli/src/cli.rs:56`
- Modify: `crates/rbs-cli/src/lib.rs:451` (le `match` des conseils post-installation)
- Test: `crates/rbs-cli/src/lib.rs` (module `tests`), `crates/rbs-cli/src/new.rs:865` si la liste y est figée

**Interfaces:**
- Consumes: le nom `audit` du fragment (tâche 1).
- Produces: rien que d'autres tâches consomment.

- [ ] **Step 1 : trouver toutes les listes de features figées**

Run: `rg -n 'rate-limit' crates/rbs-cli/src crates/rbs-cli/tests`

Chaque occurrence énumérant les features installables doit gagner `audit`, à sa place alphabétique — `audit` vient **avant** `auth`. Une liste manquée fait échouer un test ailleurs ; c'est voulu.

- [ ] **Step 2 : écrire le test du conseil qui échoue**

Dans le module `tests` du fichier qui porte la fonction de conseil (`crates/rbs-cli/src/lib.rs`), à côté des tests existants sur `jobs` et `storage` :

```rust
#[test]
fn the_audit_fragment_advises_the_migration_and_the_call_site() {
    let conseil = conseil("audit").expect("le fragment pose une table : il doit conseiller");

    assert!(conseil.contains("rbs migrate up"), "{conseil}");
    // Sans ce rappel, le fragment paraît installé et n'enregistre rien : il n'est branché
    // sur aucune route, c'est au service d'appeler.
    assert!(conseil.contains("audit::record"), "{conseil}");
}
```

Remplacer `conseil` par le nom réel de la fonction lue à `lib.rs:451`.

- [ ] **Step 3 : voir le test échouer**

Run: `cargo test -p rbs-cli --lib the_audit_fragment_advises`
Expected: FAIL — `conseil("audit")` rend `None`.

- [ ] **Step 4 : ajouter le bras du `match`**

```rust
// La table n'existe pas encore, et surtout : le fragment n'est branché sur aucune
// route. Installé et jamais appelé, il paraîtrait cassé.
"audit" => Some(
    "rbs migrate up, puis appelez audit::record dans vos services — \
     l'entrée s'écrit dans la transaction du changement",
),
```

- [ ] **Step 5 : mettre à jour l'aide du drapeau**

`cli.rs:56` devient :

```rust
/// Ajoute une feature : audit, auth, ci, cors, docker, jobs, mail, observability, rate-limit, redis, storage.
```

- [ ] **Step 6 : jouer toute la suite unitaire**

Run: `cargo test -p rbs-cli --lib`
Expected: PASS. Un test qui fige le message d'erreur des features installables **doit** échouer ici s'il n'a pas été mis à jour — le corriger, ne pas le contourner.

- [ ] **Step 7 : commit**

Sujet : `feat(audit): inscrit le fragment dans l'aide et les conseils du CLI`.

---

### Task 4 : le test d'intégration, contre une vraie base

**Files:**
- Create: `crates/rbs-cli/tests/integration_audit.rs`

**Interfaces:**
- Consumes: les quatre noms de tests de la tâche 2 ; le fragment de la tâche 1.
- Produces: rien.

- [ ] **Step 1 : lire le modèle**

Lire `crates/rbs-cli/tests/integration_jobs.rs`, en particulier `the_tests_shipped_with_the_fragment_run_against_a_real_database` (lignes 33-63) et les fonctions `cargo_test`, `migrate`, `project_with_jobs`. Reprendre `common::start_postgres`, `common::url_of`, `common::verrou`, `common::cible`.

- [ ] **Step 2 : écrire le test**

```rust
//! Le journal d'un projet réel, joué contre un PostgreSQL en conteneur.
//!
//! Ce qui s'y prouve et que rien d'autre ne prouve : que la trace disparaît avec la
//! transaction annulée qui la motivait. C'est la garantie qui justifie d'avoir mis le
//! journal en base, et elle ne veut rien dire tant qu'aucune vraie transaction n'a été
//! ouverte.

use tempfile::TempDir;

mod common;

/// Les tests que le fragment livre au projet et qui joignent la base.
const TESTS: [&str; 4] = [
    "an_entry_written_in_a_rolled_back_transaction_does_not_exist",
    "an_entry_reads_back_with_every_field_it_was_given",
    "an_entry_without_an_actor_stores_null_rather_than_an_empty_string",
    "the_entries_of_one_row_read_back_in_order",
];

#[test]
#[ignore = "démarre PostgreSQL et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_tests_shipped_with_the_fragment_run_against_a_real_database() {
    let postgres = common::start_postgres();
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_audit(&common::url_of(&postgres), &parent);

    let _cible = common::verrou(&common::cible());

    migrate(&racine);

    let ordinaires = cargo_test(&racine, &[]);
    assert!(
        ordinaires.contains("test result: ok"),
        "`cargo test` du projet a échoué :\n{ordinaires}"
    );

    let sous_conteneur = cargo_test(&racine, &["--", "--ignored"]);

    // `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun** test : sans
    // ces quatre lignes, un fragment qui cesserait de livrer ses tests laisserait
    // celui-ci au vert sans qu'une seule transaction ait été ouverte.
    for test in TESTS {
        assert!(
            sous_conteneur.contains(&format!("test audit::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{sous_conteneur}"
        );
    }
}
```

`project_with_audit`, `migrate` et `cargo_test` se recopient depuis `integration_jobs.rs` en remplaçant `add jobs` par `add audit`. **Recopier plutôt que factoriser** : les binaires de `tests/` ne partagent que `common/`, et y remonter une fonction utilisée deux fois est un changement de portée que cette tâche n'a pas.

- [ ] **Step 3 : lancer le test**

Run: `cargo test -p rbs-cli --test integration_audit -- --ignored --nocapture 2>&1 | tee /tmp/audit-integration.log`
Expected: PASS. Docker doit tourner. Compter plusieurs minutes.

Rediriger la sortie vers un fichier : une suite longue voit ses derniers chiffres rognés dans le terminal.

- [ ] **Step 4 : commit**

Sujet : `test(audit): joue les tests du fragment contre un PostgreSQL en conteneur`.

---

### Task 5 : la documentation, bilingue

**Files:**
- Create: `docs/docs/guides/audit.md`
- Create: `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/audit.md`
- Modify: `docs/docs/cli/add.md` (tableau des fragments, ligne ~48 ; transcript des features installables, ligne ~283 ; liste de l'en-tête, ligne 9)
- Modify: `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/add.md` (les mêmes)

**Interfaces:**
- Consumes: l'API de la tâche 1, les messages de la tâche 3.
- Produces: rien.

- [ ] **Step 1 : lire les deux pages modèles**

`docs/docs/guides/jobs.md` et sa version française. En reprendre le plan et le ton. Vérifier si `docs/docs/guides/_category_.json` ou un `sidebars` doit référencer la nouvelle page.

- [ ] **Step 2 : écrire le guide anglais**

Il doit couvrir, dans cet ordre : à quoi sert le journal et ce qu'il n'est pas (il ne remplace pas `trace.rs`) ; l'appel dans un service, avec la transaction ; l'acteur sous `auth` (`identity.user_id`) et sans ; la forme libre de `changes` ; **ce que le fragment ne fait pas** — il ne câble aucune route du CRUD engendré, c'est au service d'appeler, et c'est un choix, pas un manque.

- [ ] **Step 3 : écrire la version française**

Même contenu, même structure. Orthographe française complète, accents compris.

- [ ] **Step 4 : mettre à jour les tableaux et transcripts de `add.md`**

La ligne du tableau, sur le modèle de celle de `jobs` : les fichiers déposés, la migration, et le conseil post-installation **mot pour mot** celui de la tâche 3. Les listes de features de l'en-tête et du message d'erreur gagnent `audit` en tête d'ordre alphabétique.

- [ ] **Step 5 : jouer les gardes de transcript**

Run: `cargo test -p rbs-cli --test integration_docs -- --ignored --nocapture`
Expected: PASS. Ces tests comparent les blocs de la documentation à la sortie réelle du binaire ; un transcript périmé les fait échouer, et c'est leur raison d'être.

- [ ] **Step 6 : vérifier la parité bilingue**

Run: `node docs/scripts/parite.mjs`
Expected: exit 0.

- [ ] **Step 7 : commit**

Sujet : `docs(audit): documente le journal des écritures dans les deux langues`.

---

### Task 6 : vérification finale du lot

- [ ] **Step 1 : la suite complète**

```bash
cargo test --workspace 2>&1 | tee /tmp/audit-workspace.log | tail -30
```
Expected: 0 échec. Relever le nombre de tests passés — il ira dans la ligne d'`IMPROVE.md`.

- [ ] **Step 2 : la suite lente, Docker**

```bash
cargo test --workspace --no-fail-fast -- --ignored 2>&1 | tee /tmp/audit-docker.log | tail -40
```
Expected: sortie 0. **`--no-fail-fast` n'est pas optionnel** : sans lui la suite s'arrête au premier binaire et masque les échecs suivants.

- [ ] **Step 3 : la non-dérive des exemples**

```bash
cargo test -p rbs-cli --test integration_examples -- --include-ignored 2>&1 | tail -20
```
Expected: PASS, aucune dérive. Aucune template existante n'ayant été touchée, les exemples ne devraient pas bouger — si l'un bouge, c'est un effet de bord à comprendre, pas à normaliser.

- [ ] **Step 4 : lint et format**

```bash
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```
Expected: aucune sortie.

- [ ] **Step 5 : rapport**

Ne rien cocher dans `IMPROVE.md` — c'est la session principale qui le fait, sur les preuves rendues. Rapporter : les commandes lancées, leurs chiffres réels, et **tout défaut trouvé en chemin et corrigé**, en disant ce qu'il était.
