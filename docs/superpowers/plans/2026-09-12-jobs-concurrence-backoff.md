# Worker de jobs : concurrence bornée et délai de reprise doublé — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Le worker exécute jusqu'à `[jobs] concurrency` jobs de front, un job raté attend `retry_delay_secs × 2^(tentative−1)` borné par `retry_max_delay_secs`, et l'arrêt gracieux attend les jobs en cours.

**Architecture:** `run_with` tient un `JoinSet` borné : remplir jusqu'à `concurrency` par des réservations, puis `select!` entre la fin d'un job, le tour suivant (file vide) et l'arrêt ; à l'arrêt, drainer le `JoinSet`. `Registry` passe sous `Arc`, `execute` garde sa signature. `queue::retry_delay(config, attempts)` est une fonction pure que `retry_or_fail` appelle.

**Tech Stack:** tokio `JoinSet` (feature `rt`, déjà là), `tokio::sync::Barrier` (feature `sync`, déjà là), minijinja (`{@ @}`, aucun tag dans ces fichiers).

**Spec:** `docs/superpowers/specs/2026-09-12-jobs-concurrence-backoff-design.md`

## Global Constraints

- Mêmes règles que le plan de l'arrêt gracieux : commentaires *pourquoi*, commits Conventional sans identifiant ni attribution, exemples régénérés par diff, `cargo check` + clippy sur le projet jetable avant Docker, sorties Docker dans `$SCRATCHPAD/jobs-15-*.log`, doc bilingue, changelog en deux langues.
- Les tests livrés se relaient sur la table `jobs` par `verrou_base()` : tout nouveau test qui joint la base passe par `table_a_soi()`.
- `integration_jobs::TESTS` exige les tests livrés par nom : y ajouter le nouveau.

---

### Task 1: Le délai de reprise (`queue.rs.jinja`, `config.rs.jinja`, `feature.toml`)

- [ ] **Step 1: Tests rouges** dans `tests.rs.jinja`, section sans base :

```rust
fn delai(retry_delay_secs: u64, retry_max_delay_secs: u64) -> Config {
    Config { max_attempts: 5, retry_delay_secs, retry_max_delay_secs, poll_interval_secs: 1, lease_secs: 300, concurrency: 1 }
}

#[test]
fn the_retry_delay_doubles_with_each_attempt() {
    let config = delai(30, 3600);
    assert_eq!(queue::retry_delay(&config, 1), Duration::from_secs(30));
    assert_eq!(queue::retry_delay(&config, 2), Duration::from_secs(60));
    assert_eq!(queue::retry_delay(&config, 5), Duration::from_secs(480));
}

#[test]
fn the_retry_delay_is_capped_and_saturates_rather_than_overflowing() {
    let config = delai(30, 3600);
    assert_eq!(queue::retry_delay(&config, 20), Duration::from_secs(3600));
    assert_eq!(queue::retry_delay(&config, 200), Duration::from_secs(3600));
}

#[test]
fn a_zero_retry_delay_stays_zero() {
    assert_eq!(queue::retry_delay(&delai(0, 3600), 7), Duration::ZERO);
}
```

- [ ] **Step 2: Rouge** sur le projet jetable : `cargo test --lib -- modules::jobs::tests` → champs inconnus / `retry_delay` absent.
- [ ] **Step 3: `Config`** : `retry_max_delay_secs: u64` (défaut 3600), `concurrency: usize` (défaut 4) ; `feature.toml` : les deux clés dans `[[config]]` avec leur commentaire ; `config()` de `tests.rs.jinja` mis à jour.
- [ ] **Step 4: `queue::retry_delay`** :

```rust
pub(super) fn retry_delay(config: &Config, attempts: i32) -> Duration {
    let doublements = u32::try_from(attempts.saturating_sub(1)).unwrap_or(0);
    let facteur = 2u64.checked_pow(doublements).unwrap_or(u64::MAX);
    let secondes = config.retry_delay_secs.saturating_mul(facteur).min(config.retry_max_delay_secs);
    Duration::from_secs(secondes)
}
```
et `retry_or_fail` : `TimeDelta::from_std(retry_delay(config, job.attempts)).unwrap_or_else(|_| TimeDelta::seconds(0))`.
- [ ] **Step 5: Vert** : `cargo test --lib` sur le projet jetable, clippy, fmt.
- [ ] **Step 6: `doctor/jobs.rs`** (bloc de remède + liste du test), **`add/mod.rs`** (liste des clés) ; `cargo test -p rbs-cli --lib doctor::jobs` et `add::` verts.
- [ ] **Step 7: Commit** `feat(jobs): double le délai de reprise à chaque tentative, sous un plafond`.

### Task 2: La concurrence (`worker.rs.jinja`)

- [ ] **Step 1: Test rouge** dans `tests.rs.jinja` (joint la base) :

```rust
/// Un job qui n'avance que si un second est en cours au même instant.
#[derive(Debug, Serialize, Deserialize)]
struct Meet;

fn rendez_vous() -> &'static Barrier {
    static RENDEZ_VOUS: OnceLock<Barrier> = OnceLock::new();
    RENDEZ_VOUS.get_or_init(|| Barrier::new(2))
}

impl Job for Meet { KIND = "tests::meet"; run: rendez_vous().wait().await; Ok(()) }

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn jobs_run_side_by_side_up_to_the_configured_concurrency() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    let premier = queue::enqueue(db, &Meet).await.expect("le job s'enfile");
    let second = queue::enqueue(db, &Meet).await.expect("le job s'enfile");

    let mut config = config(5);
    config.concurrency = 2;
    let shutdown = state.core().shutdown().clone();
    shutdown.spawn(worker::run_with(state.clone(), registry(), config));

    attendre_le_statut(db, premier, Status::Done, Duration::from_secs(10)).await;
    attendre_le_statut(db, second, Status::Done, Duration::from_secs(10)).await;
    assert_eq!(shutdown.wait(Duration::from_secs(10)).await, 0);
}
```
`registry()` inscrit `Meet` ; l'ajouter à `TESTS` d'`integration_jobs.rs`.
- [ ] **Step 2: Rouge** : le projet jetable contre un PostgreSQL local n'est pas monté ici ; le rouge se constate par la passe Docker sur `worker.rs.jinja` d'avant (`git stash` du seul fichier) — ou, à défaut, par la lecture : la boucle séquentielle bloque le premier `Meet` sur la barrière. Consigner ce qui a été fait.
- [ ] **Step 3: La boucle** selon le spec, `dire_si_panique(fini: Result<(), JoinError>)` qui `error!` sur `Err`.
- [ ] **Step 4:** projet jetable : check, clippy, fmt, `cargo test --lib`.
- [ ] **Step 5: Commit** `feat(jobs): exécute jusqu'à [jobs] concurrency jobs de front`.

### Task 3: Exemple, documentation, changelog, note, Docker

- [ ] **Step 1:** régénérer `newsletter-queue` par diff (`jobs-regen.sh`), reporter `worker.rs`, `queue.rs`, `config.rs`, `tests.rs`, `config/default.toml` ; `cargo check` + clippy dans l'exemple ; `integration_examples` → 19 passés.
- [ ] **Step 2:** `docs/docs/cli/doctor.md` EN/FR (bloc de remède), guide `jobs` EN/FR (config, worker, retries) ; `integration_docs -- --include-ignored` ; `cd docs && npm test && npm run typecheck`.
- [ ] **Step 3:** `CHANGELOG` EN/FR, `notes/1.5.0.md`.
- [ ] **Step 4:** Docker : `jobs-docker.sh 15-vert jobs` (et `examples` par `cargo test -p rbs-cli --test integration_examples`).
- [ ] **Step 5:** fin de passe : fmt, clippy workspace, `cargo test -p rbs-cli --lib`, `cargo test -p rbs-core --all-features`.
- [ ] **Step 6: Commit** `docs(jobs): décrit la concurrence du worker et le délai de reprise doublé`.
