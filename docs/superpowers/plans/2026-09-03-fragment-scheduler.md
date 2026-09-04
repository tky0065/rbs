# Fragment `scheduler` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs add scheduler` installe un déclencheur calendaire qui réserve une échéance due et enfile un job dans la file existante — une seule fois, quel que soit le nombre de réplicas.

**Architecture:** Un onzième fragment de `crates/rbs-cli/templates/features/`, qui déclare `requires = ["jobs"]` et s'installe donc avec lui. Il dépose sept fichiers sous `src/scheduler/`, une migration, et deux ancres (`features`, `startup`). Le calendrier est déclaré en code et typé par le job qu'il vise ; la table ne porte que l'état et sert de verrou entre réplicas.

**Tech Stack:** Rust, minijinja (**délimiteurs alternatifs** : `{@ … @}` pour les expressions, `{% … %}` pour les blocs), SeaORM, crate `cron` **0.17**, `assert_cmd` + `testcontainers`.

**Spec:** `docs/superpowers/specs/2026-09-03-fragment-scheduler-design.md` — à lire en entier avant la tâche 1.

## Global Constraints

- **Worktree** : tout se fait dans `.claude/worktrees/scheduler`, branche `feature/fragment-scheduler`. Ne jamais commiter sur `main`.
- **Commits** : Conventional Commits, sujet **en français**, à l'impératif, sans majuscule initiale ni point final. **Aucune** ligne `Co-Authored-By`, aucune mention d'un assistant. Corps = le pourquoi technique + un intertitre `Vérifications :` portant les commandes lancées et leur résultat réel.
- **Commentaires** : expliquent le *pourquoi*, jamais le *quoi*.
- **Ne pas modifier le fragment `jobs`.** Il est utilisé par `examples/newsletter-queue`, et le toucher fait dériver les exemples — un coût que ce lot n'a pas à porter. Tout ce dont le scheduler a besoin, `jobs` l'expose déjà : `enqueue` prend un `ConnectionTrait`, `Job::KIND` est public.
- **Version de `cron`** : `0.17`, résolue contre l'index le 2026-09-03. Ne pas écrire `*` ni une version antérieure.
- **Expressions cron en UTC**, et tronquées à la seconde à l'écriture — MySQL rend `timestamp` sans partie fractionnaire et *arrondit*, ce qui placerait une échéance dans son propre futur.
- **Lint bloquant** : `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sans sortie.
- **Documentation bilingue** : toute page anglaise modifiée l'est aussi en français, **dans le même commit**.
- **Interdit** : ajouter une tâche au backlog pour un problème rencontré en chemin. Un défaut trouvé se corrige dans le lot, et se dit dans le rapport.

---

### Task 1 : le manifeste, la table, l'entité

**Files:**
- Create: `crates/rbs-cli/templates/features/scheduler/feature.toml`
- Create: `crates/rbs-cli/templates/features/scheduler/model.rs.jinja`
- Create: `crates/rbs-cli/templates/features/scheduler/migration.rs.jinja`
- Create: `crates/rbs-cli/templates/features/scheduler/config.rs.jinja`
- Test: `crates/rbs-cli/src/templates.rs` (module `tests`)

**Interfaces:**
- Consumes: rien.
- Produces: `model::{Entity, Model, ActiveModel, Column}` sur la table `schedules` (`kind: String` clé primaire, `next_run_at: DateTimeWithTimeZone`, `last_run_at: Option<DateTimeWithTimeZone>`, `created_at`, `updated_at`) ; `Config { poll_interval_secs: u64 }` avec `Config::load()`.

- [ ] **Step 1 : lire le moule**

Lire `templates/features/jobs/feature.toml`, `jobs/model.rs.jinja`, `jobs/migration.rs.jinja`, `jobs/config.rs.jinja`. Le commentaire de `jobs/feature.toml` sur `crate_path` vs `crate_name` s'applique mot pour mot à l'ancre `startup` de ce fragment : le relire.

- [ ] **Step 2 : écrire le test CLI qui échoue**

Dans le module `tests` de `crates/rbs-cli/src/templates.rs` :

```rust
#[test]
fn the_scheduler_fragment_requires_jobs_and_carries_both_anchors() {
    let source = read(&Path::new(RACINE_FEATURES).join("scheduler/feature.toml"));
    let manifest = crate::manifest::read(&source, "scheduler/feature.toml")
        .expect("le manifeste du fragment scheduler doit se lire");

    // Le scheduler déclenche sans exécuter : sans la file, il n'aurait nulle part où
    // enfiler, et l'installation poserait un projet qui ne compile pas.
    assert_eq!(manifest.feature.requires, ["jobs"]);

    let migration = manifest
        .migration
        .as_ref()
        .expect("le fragment pose une table, il doit donc porter une migration");
    assert_eq!(migration.name, "create_schedules");

    let ancres: Vec<&str> = manifest
        .anchors
        .iter()
        .map(|ancre| ancre.anchor.as_str())
        .collect();
    assert_eq!(ancres, ["features", "startup"]);
}
```

Adapter les noms de champs à ce que `crates/rbs-cli/src/manifest.rs` déclare réellement : **le lire avant**, et corriger l'appel plutôt que la structure.

- [ ] **Step 3 : voir le test échouer**

Run: `cargo test -p rbs-cli --lib the_scheduler_fragment`
Expected: FAIL — le fichier n'existe pas.

- [ ] **Step 4 : écrire `feature.toml`**

```toml
[feature]
description = "déclenchement calendaire : une échéance due enfile un job, une seule fois entre réplicas"
requires    = ["jobs"]

[[files]]
source      = "mod.rs.jinja"
destination = "src/scheduler/mod.rs"

[[files]]
source      = "config.rs.jinja"
destination = "src/scheduler/config.rs"

[[files]]
source      = "model.rs.jinja"
destination = "src/scheduler/model.rs"

[[files]]
source      = "sync.rs.jinja"
destination = "src/scheduler/sync.rs"

[[files]]
source      = "ticker.rs.jinja"
destination = "src/scheduler/ticker.rs"

[[files]]
source      = "tests.rs.jinja"
destination = "src/scheduler/tests.rs"

[[anchors]]
anchor  = "features"
content = "pub mod scheduler;"

# Comme le worker de la file, le ticker vit dans le processus de l'API : c'est ce qui lui
# donne l'`AppState` du projet. `crate_path` et non `crate_name`, pour la raison
# qu'énonce le manifeste de `jobs`.
[[anchors]]
anchor  = "startup"
content = "{@ crate_path @}::scheduler::spawn(state.clone());"

[migration]
source = "migration.rs.jinja"
name   = "create_schedules"

[[dependencies]]
name    = "cron"
version = "0.17"

# Le ticker dort entre deux tours de boucle.
[cargo.tokio]
features = ["time"]

[[config]]
file    = "config/default.toml"
section = "scheduler"
content = """
# Attente entre deux examens du calendrier. Une échéance à la minute n'a pas besoin d'un
# réveil par seconde ; trente secondes bornent le retard de déclenchement à trente
# secondes.
poll_interval_secs = 30
"""
```

Les fichiers `mod.rs.jinja`, `sync.rs.jinja`, `ticker.rs.jinja` et `tests.rs.jinja` sont déclarés ici et écrits aux tâches 2 à 4. **La tâche 1 doit les créer au moins vides** — un fichier déclaré et absent fait échouer l'installation.

Vérifier la forme exacte de `[[config]]` et de `[[dependencies]]` dans `jobs/feature.toml` et dans `manifest.rs` : reprendre les noms de clés observés.

- [ ] **Step 5 : écrire `migration.rs.jinja`**

Sur le modèle de `jobs/migration.rs.jinja`, table `schedules` :

| Colonne | Déclaration |
|---|---|
| `Kind` | `.text().not_null().primary_key()` |
| `NextRunAt` | `.timestamp_with_time_zone().not_null()` |
| `LastRunAt` | `.timestamp_with_time_zone().null()` |
| `CreatedAt` | `.timestamp_with_time_zone().not_null().default(Expr::current_timestamp())` |
| `UpdatedAt` | `.timestamp_with_time_zone().not_null().default(Expr::current_timestamp())` |

**Aucun index** : toute lecture passe par la clé primaire ou balaie une table qui compte autant de lignes que le projet a d'échéances. Un commentaire le dit, pour qu'on ne « répare » pas cette absence.

Attention MySQL : une colonne `TEXT` ne peut pas être clé primaire sans longueur. Utiliser `.string_len(191)` plutôt que `.text()` si le moteur le refuse — **et le vérifier à la tâche 6, sur les trois moteurs**. En cas de doute à l'écriture, choisir `.string_len(191)` d'emblée : un `KIND` est un identifiant court, et 191 est la limite d'un index `utf8mb4` sous MySQL 5.7.

- [ ] **Step 6 : écrire `model.rs.jinja`**

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "schedules")]
pub struct Model {
    /// Le `KIND` du job déclenché. La clé primaire *est* l'unicité de l'échéance.
    #[sea_orm(primary_key, auto_increment = false)]
    pub kind: String,
    /// L'échéance. La réservation la compare, puis l'avance à l'occurrence suivante.
    pub next_run_at: DateTimeWithTimeZone,
    /// Le dernier déclenchement, ou rien tant qu'il n'y en a pas eu.
    pub last_run_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:schedules>
    // </rbs:relations:schedules>
}

// <rbs:related:schedules>
// </rbs:related:schedules>

impl ActiveModelBehavior for ActiveModel {}
```

Pas de `new()` qui pose un UUID : la clé est le `kind`, fourni par l'appelant. Reprendre la convention de nommage des ancres `relations:`/`related:` observée dans `jobs/model.rs.jinja`.

- [ ] **Step 7 : écrire `config.rs.jinja`**

Sur le modèle exact de `jobs/config.rs.jinja`, avec le seul champ `poll_interval_secs: u64` (défaut 30) et `Config::load()` appelant `rbs_core::config::section("scheduler")`.

- [ ] **Step 8 : voir le test passer**

Run: `cargo test -p rbs-cli --lib the_scheduler_fragment`
Expected: PASS.

- [ ] **Step 9 : commit**

Sujet : `feat(scheduler): pose la table des échéances et son manifeste`.

---

### Task 2 : le calendrier déclaré en code

**Files:**
- Modify: `crates/rbs-cli/templates/features/scheduler/mod.rs.jinja`

**Interfaces:**
- Consumes: `jobs::Job` et `jobs::enqueue` du fragment `jobs`, via `crate::jobs`.
- Produces:
  - `pub struct Schedule { pub kind: &'static str, pub expression: String, … }`
  - `pub fn Schedule::every<J: Job>(expression: impl Into<String>, fabrique: fn() -> J) -> Self`
  - `pub fn Schedule::compiler(&self) -> anyhow::Result<cron::Schedule>`
  - `pub(super) async fn Schedule::enfiler(&self, txn: &DatabaseTransaction) -> anyhow::Result<Uuid>`
  - `pub fn normaliser(expression: &str) -> anyhow::Result<String>`
  - `pub(super) fn a_la_seconde(instant: DateTimeWithTimeZone) -> DateTimeWithTimeZone`
  - `pub fn schedules() -> Vec<Schedule>`
  - `pub fn spawn(state: AppState)`

- [ ] **Step 1 : écrire les tests de la normalisation, qui échouent**

Ces tests-là n'ont besoin d'aucune base : ils vont dans `tests.rs.jinja` **sans** `#[ignore]`.

```rust
use std::str::FromStr;

use super::{Schedule, normaliser};

/// La crate `cron` attend six champs, la seconde en tête ; le crontab Unix en a cinq. Un
/// utilisateur qui colle une ligne de son crontab doit être servi, pas puni.
#[test]
fn a_five_field_expression_means_the_same_as_its_six_field_form() {
    let cinq = normaliser("0 3 * * *").expect("cinq champs sont acceptés");
    let six = normaliser("0 0 3 * * *").expect("six champs sont acceptés");

    assert_eq!(cinq, six);

    let horaire = cron::Schedule::from_str(&cinq).expect("l'expression normalisée compile");
    let base = chrono::DateTime::parse_from_rfc3339("2026-09-03T10:00:00+00:00")
        .expect("instant lisible");

    assert_eq!(
        horaire.after(&base).next().expect("une occurrence suit"),
        chrono::DateTime::parse_from_rfc3339("2026-09-04T03:00:00+00:00")
            .expect("instant lisible")
    );
}

#[test]
fn an_expression_of_any_other_length_is_refused_by_name() {
    let erreur = normaliser("0 3 * *").expect_err("quatre champs ne veulent rien dire");

    // Le message doit porter l'expression : c'est la seule chose qui distingue une
    // échéance fautive des autres au démarrage.
    assert!(erreur.to_string().contains("0 3 * *"), "{erreur}");
}

#[test]
fn an_unparsable_expression_is_refused_even_with_the_right_field_count() {
    normaliser("0 99 * * *").expect_err("99 n'est pas une minute");
}
```

- [ ] **Step 2 : voir les trois tests échouer**

Sur un projet jetable :

```bash
cargo run -p rbs-cli --bin rbs -- new /tmp/probe-sched --database postgres --yes
cd /tmp/probe-sched && cargo run -p rbs-cli --bin rbs -- add scheduler && cargo test scheduler::
```
Expected: FAIL à la compilation — `normaliser` n'existe pas.

Ce projet jetable est **l'outil de travail des tâches 2 à 4** : le code d'un fragment n'est compilé nulle part ailleurs avant la passe Docker. Le régénérer après chaque modification de template (`rbs add` ne réécrit pas un fichier déjà posé : supprimer `src/scheduler/` et relancer, ou recréer le projet).

- [ ] **Step 3 : écrire `mod.rs.jinja`**

```rust
pub mod config;
pub mod model;
pub mod sync;
pub mod ticker;

#[cfg(test)]
mod tests;

use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use chrono::Timelike;
use sea_orm::DatabaseTransaction;
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};

pub use config::Config;

use crate::jobs::{self, Job};
use crate::state::AppState;

/// L'enfilage d'une échéance, son type de job oublié.
///
/// La transaction est empruntée : c'est elle qui rend la réservation de l'échéance et la
/// naissance du job indissociables.
type Enfilage = Arc<
    dyn for<'a> Fn(
            &'a DatabaseTransaction,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<Uuid>> + Send + 'a>>
        + Send
        + Sync,
>;

/// Une échéance : un job, et quand le déclencher.
pub struct Schedule {
    /// Le `KIND` du job visé, d'où la ligne de la table tire sa clé.
    pub kind: &'static str,
    /// L'expression, telle qu'elle a été écrite — normalisée à la compilation.
    pub expression: String,
    enfiler: Enfilage,
}

impl Schedule {
    /// Déclare qu'un job doit être enfilé aux occurrences de `expression`.
    ///
    /// Le `kind` vient de `J::KIND` et non d'une chaîne : une échéance qui viserait un job
    /// non inscrit au registre est ainsi inécrivable.
    ///
    /// La fabrique est un pointeur de fonction et non une fermeture : elle est rejouée à
    /// chaque déclenchement — une charge utile qui porte une date la veut à l'instant du
    /// tick — et ne peut capturer aucun état qui aurait vieilli entre-temps.
    pub fn every<J: Job>(expression: impl Into<String>, fabrique: fn() -> J) -> Self {
        Self {
            kind: J::KIND,
            expression: expression.into(),
            enfiler: Arc::new(move |transaction| {
                Box::pin(async move { jobs::enqueue(transaction, &fabrique()).await })
            }),
        }
    }

    /// Compile l'expression, en acceptant les deux formes.
    pub fn compiler(&self) -> anyhow::Result<cron::Schedule> {
        let normalisee = normaliser(&self.expression)?;

        cron::Schedule::from_str(&normalisee)
            .map_err(|source| anyhow::anyhow!("`{}` : {source}", self.expression))
    }

    /// Enfile le job de cette échéance dans la transaction qui vient de la réserver.
    pub(super) async fn enfiler(&self, transaction: &DatabaseTransaction) -> anyhow::Result<Uuid> {
        (self.enfiler)(transaction).await
    }
}

/// Ramène une expression à la forme que `cron` attend : six champs, la seconde en tête.
///
/// Le crontab Unix en a cinq et ne dit rien des secondes ; les cinq champs valent donc
/// « à la seconde zéro ». Sans cette normalisation, coller une ligne de son crontab —
/// le premier geste évident — arrête le démarrage.
pub fn normaliser(expression: &str) -> anyhow::Result<String> {
    let champs = expression.split_whitespace().count();

    let normalisee = match champs {
        5 => format!("0 {expression}"),
        6 => expression.to_string(),
        autre => anyhow::bail!(
            "`{expression}` porte {autre} champ(s) : une expression cron en compte cinq \
             (minute heure jour mois jour-de-semaine) ou six, la seconde en tête"
        ),
    };

    // Compilée ici et jetée : le but est de refuser tout de suite une expression que
    // `Schedule::compiler` refuserait plus tard, au démarrage.
    cron::Schedule::from_str(&normalisee)
        .map_err(|source| anyhow::anyhow!("`{expression}` : {source}"))?;

    Ok(normalisee)
}

/// Tronque un instant à la seconde, tel qu'il sera stocké.
///
/// MySQL rend `timestamp` sans précision fractionnaire et **arrondit** ce qu'on y écrit :
/// une échéance à `…34,6 s` y devient `…35 s`, soit une échéance que sa propre date place
/// dans le futur et qu'aucun tick ne verra.
///
/// Elle vit ici et non dans `ticker` parce que la réconciliation l'utilise aussi : les
/// deux écrivent dans la même colonne, et une troncature d'un seul côté rendrait
/// inégales deux dates qui doivent se comparer.
pub(super) fn a_la_seconde(instant: DateTimeWithTimeZone) -> DateTimeWithTimeZone {
    instant.with_nanosecond(0).unwrap_or(instant)
}

/// Les échéances de ce projet. Déclarez les vôtres ici.
///
/// Les expressions sont évaluées **en UTC** : `0 3 * * *` est 3 h UTC.
pub fn schedules() -> Vec<Schedule> {
    vec![Schedule::every::<crate::jobs::demo::Log>("0 3 * * *", || {
        crate::jobs::demo::Log {
            message: "échéance quotidienne".to_string(),
        }
    })]
}

/// Détache le ticker dans le runtime, et rend la main aussitôt.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        if let Err(error) = ticker::run(state).await {
            tracing::error!(%error, "le ticker du calendrier ne démarre pas");
        }
    });
}
```

**Vérifier `jobs::demo::Log`** : le champ s'appelle `message` et est une `String` (`templates/features/jobs/demo.rs.jinja`). Si le fragment `jobs` a changé, adapter — ne pas inventer.

- [ ] **Step 4 : voir les trois tests passer**

Run: `cd /tmp/probe-sched && cargo test scheduler::tests`
Expected: 3 passed.

- [ ] **Step 5 : commit**

Sujet : `feat(scheduler): déclare le calendrier en code, typé par le job visé`.

---

### Task 3 : la réconciliation au démarrage

**Files:**
- Modify: `crates/rbs-cli/templates/features/scheduler/sync.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/scheduler/tests.rs.jinja`

**Interfaces:**
- Consumes: `Schedule`, `schedules()`, `model::*` des tâches 1 et 2.
- Produces: `pub async fn sync::reconcilier(db: &DatabaseConnection, schedules: &[Schedule]) -> anyhow::Result<()>`.

- [ ] **Step 1 : écrire les tests qui échouent**

Ils joignent la base : `#[ignore = "joint la base du projet"]`. Reprendre la fonction `table_a_soi()` de `jobs/tests.rs.jinja`, adaptée à la table `schedules`.

```rust
/// Un calendrier de test, indépendant de celui que le projet déclare.
fn calendrier(expression: &'static str) -> Vec<Schedule> {
    vec![Schedule::every::<crate::jobs::demo::Log>(expression, || {
        crate::jobs::demo::Log {
            message: "test".to_string(),
        }
    })]
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_newly_declared_schedule_is_inserted_with_its_next_occurrence() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");

    let ligne = Entity::find_by_id("log")
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance déclarée doit avoir été insérée");

    assert!(ligne.next_run_at > chrono::Utc::now());
    assert!(ligne.last_run_at.is_none(), "rien n'a encore été déclenché");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_schedule_removed_from_the_code_is_removed_from_the_table() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");
    // Sans cette suppression, une échéance retirée du code resterait due pour toujours,
    // que plus personne ne réserverait ni ne ferait avancer.
    sync::reconcilier(db, &[])
        .await
        .expect("la réconciliation aboutit");

    assert!(
        Entity::find_by_id("log")
            .one(db)
            .await
            .expect("lecture possible")
            .is_none(),
        "l'échéance retirée du code survit dans la table"
    );
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_redeploy_does_not_move_the_next_occurrence_of_a_known_schedule() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");
    let avant = Entity::find_by_id("log")
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance existe")
        .next_run_at;

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");
    let apres = Entity::find_by_id("log")
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance existe")
        .next_run_at;

    // C'est ce qui rend un déploiement invisible pour le calendrier : un redémarrage ne
    // rejoue pas une échéance passée et ne repousse pas une échéance imminente.
    assert_eq!(avant, apres);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unparsable_expression_stops_the_reconciliation() {
    let (_garde, state) = table_a_soi().await;

    let erreur = sync::reconcilier(state.core().db(), &calendrier("0 99 * * *"))
        .await
        .expect_err("une expression illisible doit arrêter le démarrage");

    assert!(erreur.to_string().contains("0 99 * * *"), "{erreur}");
}
```

- [ ] **Step 2 : voir les tests échouer**

Run: `cd /tmp/probe-sched && cargo test scheduler:: -- --ignored`
Expected: FAIL à la compilation — `sync::reconcilier` n'existe pas.

- [ ] **Step 3 : écrire `sync.rs.jinja`**

```rust
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::Schedule;
use super::model::{ActiveModel, Column, Entity};

/// Aligne la table sur le calendrier déclaré en code.
///
/// Toute expression est compilée **avant** le premier écrit : une seule illisible arrête
/// le démarrage plutôt que de laisser un service qui paraît sain et dont une tâche ne
/// tournera jamais — le mode de panne le plus coûteux à diagnostiquer.
pub async fn reconcilier(
    db: &DatabaseConnection,
    schedules: &[Schedule],
) -> anyhow::Result<()> {
    let mut prochaines = Vec::with_capacity(schedules.len());
    for schedule in schedules {
        let horaire = schedule.compiler()?;
        let prochaine = horaire
            .after(&Utc::now().fixed_offset())
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!("`{}` n'a plus aucune occurrence à venir", schedule.expression)
            })?;

        prochaines.push((schedule.kind, super::a_la_seconde(prochaine)));
    }

    let declares: Vec<&str> = prochaines.iter().map(|(kind, _)| *kind).collect();

    // Une échéance retirée du code resterait sinon due pour toujours, sans que rien ne la
    // réserve ni ne la fasse avancer.
    let mut suppression = Entity::delete_many();
    if !declares.is_empty() {
        suppression = suppression.filter(Column::Kind.is_not_in(declares.iter().copied()));
    }
    suppression.exec(db).await?;

    for (kind, prochaine) in prochaines {
        // Une échéance déjà connue garde son `next_run_at` : un redémarrage ne rejoue pas
        // une occurrence passée et ne repousse pas une occurrence imminente.
        if Entity::find_by_id(kind).one(db).await?.is_some() {
            continue;
        }

        ActiveModel {
            kind: Set(kind.to_string()),
            next_run_at: Set(prochaine),
            last_run_at: Set(None),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}
```

`created_at` et `updated_at` viennent des défauts de la table. Si SeaORM refuse `..Default::default()` sur un `ActiveModel` dont la clé n'est pas auto-incrémentée, poser explicitement `created_at` et `updated_at` avec `Utc::now().fixed_offset()` tronqué — et le dire en commentaire.

- [ ] **Step 4 : voir les tests passer**

Il faut une base. Deux voies : monter un PostgreSQL 14+ à la main et pointer `.env` du projet jetable dessus, puis `rbs migrate up` ; ou attendre la tâche 6. **Préférer la première** : découvrir à la tâche 6 que la réconciliation ne marche pas coûte une passe Docker entière.

Run: `cd /tmp/probe-sched && cargo test scheduler:: -- --ignored`
Expected: 4 passed (les trois de la tâche 2 restent hors `--ignored`).

- [ ] **Step 5 : commit**

Sujet : `feat(scheduler): réconcilie la table avec le calendrier au démarrage`.

---

### Task 4 : le tick, la réservation, l'enfilage

**Files:**
- Modify: `crates/rbs-cli/templates/features/scheduler/ticker.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/scheduler/tests.rs.jinja`

**Interfaces:**
- Consumes: `Schedule`, `schedules()`, `sync::reconcilier`, `model::*`.
- Produces:
  - `pub async fn run(state: AppState) -> anyhow::Result<()>`
  - `pub(super) async fn tick(db: &DatabaseConnection, schedules: &[Schedule]) -> anyhow::Result<usize>` — rend le nombre d'échéances déclenchées

- [ ] **Step 1 : écrire les tests qui échouent**

```rust
/// Pose une échéance déjà due, en court-circuitant la réconciliation.
async fn echeance_due(db: &DatabaseConnection) {
    use sea_orm::ActiveValue::Set;

    ActiveModel {
        kind: Set("log".to_string()),
        next_run_at: Set((chrono::Utc::now() - chrono::TimeDelta::hours(1)).fixed_offset()),
        last_run_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("l'échéance due s'insère");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_due_schedule_enqueues_its_job_and_moves_on() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    echeance_due(db).await;

    let declenchees = ticker::tick(db, &calendrier("0 3 * * *"))
        .await
        .expect("le tick aboutit");
    assert_eq!(declenchees, 1);

    let ligne = Entity::find_by_id("log")
        .one(db)
        .await
        .expect("lecture possible")
        .expect("l'échéance existe");
    assert!(ligne.next_run_at > chrono::Utc::now(), "l'échéance n'a pas avancé");
    assert!(ligne.last_run_at.is_some(), "le déclenchement n'est pas daté");

    // C'est le job qui compte : une échéance qui avance sans rien enfiler serait une
    // horloge qui tourne à vide.
    let jobs = crate::jobs::model::Entity::find()
        .all(db)
        .await
        .expect("lecture possible");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].kind, "log");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_schedule_that_is_not_due_is_left_alone() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    sync::reconcilier(db, &calendrier("0 3 * * *"))
        .await
        .expect("la réconciliation aboutit");

    let declenchees = ticker::tick(db, &calendrier("0 3 * * *"))
        .await
        .expect("le tick aboutit");

    assert_eq!(declenchees, 0);
    assert!(
        crate::jobs::model::Entity::find()
            .all(db)
            .await
            .expect("lecture possible")
            .is_empty(),
        "un job est né d'une échéance qui n'était pas due"
    );
}

/// La garantie qui justifie d'avoir mis le calendrier en base plutôt que dans le
/// processus : trois réplicas, une seule purge nocturne.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "joint la base du projet"]
async fn concurrent_tickers_trigger_a_due_schedule_exactly_once() {
    const TICKERS: usize = 8;

    let (_garde, state) = table_a_soi().await;
    echeance_due(state.core().db()).await;

    let mut taches = Vec::new();
    for _ in 0..TICKERS {
        let db = state.core().db().clone();
        taches.push(tokio::spawn(async move {
            ticker::tick(&db, &calendrier("0 3 * * *"))
                .await
                .expect("le tick aboutit")
        }));
    }

    let mut declenchees = 0;
    for tache in taches {
        declenchees += tache.await.expect("le ticker ne panique pas");
    }

    assert_eq!(declenchees, 1, "{declenchees} réplicas ont cru gagner l'échéance");

    let jobs = crate::jobs::model::Entity::find()
        .all(state.core().db())
        .await
        .expect("lecture possible");
    assert_eq!(jobs.len(), 1, "l'échéance a enfilé {} job(s)", jobs.len());
}
```

- [ ] **Step 2 : voir les tests échouer**

Run: `cd /tmp/probe-sched && cargo test scheduler:: -- --ignored`
Expected: FAIL à la compilation — `ticker::tick` n'existe pas.

- [ ] **Step 3 : écrire `ticker.rs.jinja`**

```rust
use std::time::Duration;

use chrono::Utc;
use rbs_core::HasCoreState;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait, Unchanged,
};

use super::model::{ActiveModel, Column, Entity};
use super::{Config, Schedule, a_la_seconde, schedules, sync};
use crate::state::AppState;

/// Examine le calendrier jusqu'à l'arrêt du processus.
pub async fn run(state: AppState) -> anyhow::Result<()> {
    let config = Config::load()?;
    let schedules = schedules();
    let db = state.core().db();

    sync::reconcilier(db, &schedules).await?;

    let attente = Duration::from_secs(config.poll_interval_secs);
    tracing::info!(
        echeances = schedules.len(),
        poll_interval_secs = config.poll_interval_secs,
        "calendrier prêt"
    );

    loop {
        // Une base momentanément injoignable ne condamne pas le calendrier : le ticker
        // retente au tour suivant plutôt que de rendre la main pour de bon.
        if let Err(error) = tick(db, &schedules).await {
            tracing::error!(%error, "examen du calendrier impossible");
        }

        tokio::time::sleep(attente).await;
    }
}

/// Réserve puis déclenche toutes les échéances dues, et rend leur nombre.
pub(super) async fn tick(
    db: &DatabaseConnection,
    schedules: &[Schedule],
) -> anyhow::Result<usize> {
    let mut declenchees = 0;

    for schedule in schedules {
        if reserver_et_enfiler(db, schedule).await? {
            declenchees += 1;
        }
    }

    Ok(declenchees)
}

/// Tente de gagner une échéance, et enfile son job si elle est gagnée.
///
/// La réservation et l'enfilage partagent une transaction : sans elle, un arrêt entre les
/// deux avancerait l'échéance sans créer le job, et personne ne s'en apercevrait avant
/// l'occurrence suivante.
async fn reserver_et_enfiler(
    db: &DatabaseConnection,
    schedule: &Schedule,
) -> anyhow::Result<bool> {
    let horaire = schedule.compiler()?;
    let maintenant = a_la_seconde(Utc::now().fixed_offset());

    let prochaine = horaire
        .after(&maintenant)
        .next()
        .ok_or_else(|| anyhow::anyhow!("`{}` n'a plus aucune occurrence", schedule.expression))?;

    let transaction = db.begin().await?;

    // La condition porte sur `next_run_at` et non sur une lecture préalable : c'est ce
    // qui rend la réservation atomique. Le verrou de ligne que l'`UPDATE` pose fait
    // attendre le réplica suivant, qui relit une échéance déjà avancée et n'affecte rien.
    let reserve = Entity::update_many()
        .col_expr(Column::NextRunAt, a_la_seconde(prochaine).into())
        .col_expr(Column::LastRunAt, maintenant.into())
        .col_expr(Column::UpdatedAt, maintenant.into())
        .filter(Column::Kind.eq(schedule.kind))
        .filter(Column::NextRunAt.lte(maintenant))
        .exec(&transaction)
        .await?;

    if reserve.rows_affected == 0 {
        transaction.rollback().await?;
        return Ok(false);
    }

    schedule.enfiler(&transaction).await?;
    transaction.commit().await?;

    tracing::debug!(kind = schedule.kind, "échéance déclenchée");

    Ok(true)
}

```

`a_la_seconde` vient de `mod.rs` (tâche 2) : ne pas la redéfinir ici.

Si `Unchanged` n'est pas utilisé, retirer l'import — `clippy -D warnings` le refuserait. Si `col_expr` n'accepte pas un `DateTimeWithTimeZone` via `.into()`, passer par `sea_orm::sea_query::Expr::value(...)` ; **vérifier contre la version de SeaORM du workspace**, ne pas deviner.

- [ ] **Step 4 : voir les tests passer**

Run: `cd /tmp/probe-sched && cargo test scheduler:: -- --ignored`
Expected: 7 passed sous `--ignored`, et les 3 tests de la tâche 2 toujours verts sous `cargo test` ordinaire.

Si `concurrent_tickers_trigger_a_due_schedule_exactly_once` échoue avec un nombre supérieur à 1, **ne pas ajouter de verrou applicatif** : la cause est dans la clause `WHERE` ou dans la transaction, et c'est là qu'elle se corrige.

- [ ] **Step 5 : prouver que la garantie est bien testée**

Amputer temporairement le filtre `Column::NextRunAt.lte(maintenant)`, relancer le test de concurrence, **le voir échouer**, puis restaurer. Une garantie dont le test reste vert quand on retire ce qui la porte ne prouve rien.

Consigner le résultat des deux passes dans le message de commit.

- [ ] **Step 6 : commit**

Sujet : `feat(scheduler): réserve une échéance due et enfile son job d'un même geste`.

---

### Task 5 : ce que le CLI doit apprendre

**Files:**
- Modify: `crates/rbs-cli/src/cli.rs:56`
- Modify: `crates/rbs-cli/src/lib.rs:451`
- Test: `crates/rbs-cli/src/lib.rs` (module `tests`)

- [ ] **Step 1 : trouver toutes les listes de features figées**

Run: `rg -n 'rate-limit' crates/rbs-cli/src crates/rbs-cli/tests`

Chaque liste énumérant les features installables gagne `scheduler`, à sa place alphabétique — entre `redis` et `storage`.

- [ ] **Step 2 : écrire le test du conseil qui échoue**

```rust
#[test]
fn the_scheduler_fragment_advises_the_migration_and_the_declaration_site() {
    let conseil = conseil("scheduler").expect("le fragment pose une table : il doit conseiller");

    assert!(conseil.contains("rbs migrate up"), "{conseil}");
    // La liste livrée ne contient qu'une échéance d'exemple : sans ce rappel, le
    // fragment paraît installé et ne déclenche rien de ce que le projet attend.
    assert!(conseil.contains("src/scheduler/mod.rs"), "{conseil}");
}
```

Remplacer `conseil` par le nom réel de la fonction lue à `lib.rs:451`.

- [ ] **Step 3 : voir le test échouer**

Run: `cargo test -p rbs-cli --lib the_scheduler_fragment_advises`
Expected: FAIL.

- [ ] **Step 4 : ajouter le bras du `match`**

```rust
// Deux tables à créer — le fragment entraîne `jobs` — et une liste d'échéances qui ne
// contient qu'un exemple : installé et non édité, le calendrier ne déclenche rien
// d'utile.
"scheduler" => Some(
    "rbs migrate up, puis déclarez vos échéances dans src/scheduler/mod.rs — \
     les expressions sont évaluées en UTC",
),
```

- [ ] **Step 5 : mettre à jour l'aide du drapeau**

```rust
/// Ajoute une feature : auth, ci, cors, docker, jobs, mail, observability, rate-limit, redis, scheduler, storage.
```

- [ ] **Step 6 : jouer la suite unitaire**

Run: `cargo test -p rbs-cli --lib`
Expected: PASS. Un test figeant le message d'erreur des features installables doit échouer s'il n'a pas suivi — le corriger.

- [ ] **Step 7 : commit**

Sujet : `feat(scheduler): inscrit le fragment dans l'aide et les conseils du CLI`.

---

### Task 6 : le test d'intégration, sur les trois moteurs

**Files:**
- Create: `crates/rbs-cli/tests/integration_scheduler.rs`

- [ ] **Step 1 : lire le modèle**

`crates/rbs-cli/tests/integration_jobs.rs` en entier — les deux tests, dont celui qui boucle sur les trois moteurs (`the_dequeue_never_hands_the_same_job_twice_on_the_three_engines`, lignes 69-130) et sa gestion d'une cible de compilation par moteur.

- [ ] **Step 2 : écrire le test**

Deux tests, sur le modèle exact d'`integration_jobs.rs` :

1. `the_tests_shipped_with_the_fragment_run_against_a_real_database` — PostgreSQL, exige **nommément** les dix tests livrés, dans le flux où chacun sort. Trois d'entre eux n'ont pas besoin de base et sortent sous `cargo test` ordinaire ; les sept autres sous `-- --ignored`. Les confondre ferait passer le test sans qu'aucun des deux groupes soit vraiment exigé.

```rust
/// Ce que le fragment livre et que `cargo test` joue sans base.
const TESTS_ORDINAIRES: [&str; 3] = [
    "a_five_field_expression_means_the_same_as_its_six_field_form",
    "an_expression_of_any_other_length_is_refused_by_name",
    "an_unparsable_expression_is_refused_even_with_the_right_field_count",
];

/// Ce qu'il livre et qui joint la base.
const TESTS_SOUS_CONTENEUR: [&str; 7] = [
    "a_newly_declared_schedule_is_inserted_with_its_next_occurrence",
    "a_schedule_removed_from_the_code_is_removed_from_the_table",
    "a_redeploy_does_not_move_the_next_occurrence_of_a_known_schedule",
    "an_unparsable_expression_stops_the_reconciliation",
    "a_due_schedule_enqueues_its_job_and_moves_on",
    "a_schedule_that_is_not_due_is_left_alone",
    "concurrent_tickers_trigger_a_due_schedule_exactly_once",
];
```

Chaque nom est cherché sous la forme `test scheduler::tests::<nom> ... ok`, dans le flux correspondant — c'est la garde qu'`integration_jobs.rs` documente : `cargo test -- --ignored` sort en 0 même quand il ne filtre aucun test.

2. `a_due_schedule_is_triggered_once_on_the_three_engines` — boucle PostgreSQL / MySQL / SQLite, une cible de compilation par moteur, exigeant nommément :

```rust
"test scheduler::tests::concurrent_tickers_trigger_a_due_schedule_exactly_once ... ok"
```

C'est ce test qui prouve le choix de la clé primaire `TEXT` sous MySQL et l'atomicité de l'`UPDATE` conditionnel sur les trois moteurs — les deux points que la tâche 1 laissait ouverts.

**Vérifier au passage** que `rbs add scheduler` sur un projet nu installe `jobs` **puis** `scheduler`, et que les deux migrations s'appliquent :

```rust
assert!(racine.join("src/jobs/mod.rs").exists(), "le fragment requis n'a pas été entraîné");
assert!(racine.join("src/scheduler/mod.rs").exists());
```

- [ ] **Step 3 : lancer**

```bash
cargo test -p rbs-cli --test integration_scheduler -- --ignored --nocapture 2>&1 | tee /tmp/scheduler-integration.log
```
Expected: PASS. Docker requis, PostgreSQL et MySQL démarrés. Compter dix minutes ou plus.

Rediriger vers un fichier : les chiffres d'une suite longue sont rognés dans le terminal.

- [ ] **Step 4 : commit**

Sujet : `test(scheduler): joue le déclenchement unique sur les trois moteurs`.

---

### Task 7 : la documentation, bilingue

**Files:**
- Create: `docs/docs/guides/scheduler.md`
- Create: `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/scheduler.md`
- Modify: `docs/docs/guides/jobs.md` et sa version française (le renvoi)
- Modify: `docs/docs/cli/add.md` et sa version française (tableau, en-tête, transcript)

- [ ] **Step 1 : écrire le guide anglais**

Couvrir, dans cet ordre : déclarer une échéance et le fait que le `kind` vient du job ; **les deux formes d'expression**, cinq champs comme six, avec l'exemple `0 3 * * *` ; l'**UTC** ; qu'un seul réplica déclenche, et pourquoi la table existe ; le comportement au redémarrage — une échéance connue garde sa date ; et que **changer l'expression d'une échéance existante ne prend effet qu'à son prochain déclenchement**, avec la manœuvre pour forcer (supprimer la ligne).

- [ ] **Step 2 : écrire la version française**

Même contenu, même structure, orthographe française complète.

- [ ] **Step 3 : le renvoi depuis le guide des jobs**

À l'endroit où `docs/docs/guides/jobs.md` dit qu'un job s'enfile sur événement, ajouter une phrase renvoyant au scheduler. Idem en français.

- [ ] **Step 4 : tableau et transcripts de `add.md`**

La ligne du tableau doit dire que le fragment **entraîne `jobs`** — c'est la seule ligne du tableau dans ce cas, et un utilisateur qui ne s'y attend pas verrait apparaître `src/jobs/`. Le conseil post-installation, mot pour mot celui de la tâche 5. Les listes de features gagnent `scheduler`.

- [ ] **Step 5 : gardes de transcript et parité**

```bash
cargo test -p rbs-cli --test integration_docs -- --ignored --nocapture
node docs/scripts/parite.mjs
```
Expected: PASS, puis exit 0. Le script de parité ne voit ni les tableaux ni certains cas : une ligne de tableau ajoutée d'un seul côté passerait son contrôle. La vérifier à l'œil.

- [ ] **Step 6 : commit**

Sujet : `docs(scheduler): documente le déclenchement calendaire dans les deux langues`.

---

### Task 8 : vérification finale du lot

- [ ] **Step 1 : la suite complète**

```bash
cargo test --workspace 2>&1 | tee /tmp/scheduler-workspace.log | tail -30
```
Expected: 0 échec. Relever le nombre de tests passés.

- [ ] **Step 2 : la suite lente, Docker**

```bash
cargo test --workspace --no-fail-fast -- --ignored 2>&1 | tee /tmp/scheduler-docker.log | tail -40
```
Expected: sortie 0. **`--no-fail-fast` n'est pas optionnel** : sans lui la suite s'arrête au premier binaire et masque les échecs suivants.

- [ ] **Step 3 : la non-dérive des exemples**

```bash
cargo test -p rbs-cli --test integration_examples -- --include-ignored 2>&1 | tail -20
```
Expected: PASS, aucune dérive. Le fragment `jobs` n'ayant pas été touché, `examples/newsletter-queue` ne doit pas bouger. S'il bouge, c'est qu'une contrainte du plan a été enfreinte : comprendre, ne pas régénérer.

- [ ] **Step 4 : lint et format**

```bash
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```
Expected: aucune sortie.

- [ ] **Step 5 : rapport**

Ne rien cocher dans `IMPROVE.md`. Rapporter les commandes, leurs chiffres réels, et **tout défaut trouvé en chemin et corrigé**.
