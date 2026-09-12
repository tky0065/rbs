# Arrêt gracieux des projets engendrés — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un projet engendré qui reçoit Ctrl-C ou SIGTERM cesse d'accepter, laisse finir les requêtes en vol et le job en cours, puis pousse le dernier lot de spans avant de sortir en 0.

**Architecture:** `rbs-core` gagne un module `shutdown` (`Shutdown` = `CancellationToken` + `TaskTracker` de tokio-util, plus `signal()` qui écoute Ctrl-C et SIGTERM), porté par `CoreState` et donc par tout `AppState`. `main.rs.jinja` sert avec `with_graceful_shutdown(shutdown.on_signal())`, attend les tâches de fond au plus `server.shutdown_timeout_secs`, puis appelle `logs::shutdown()`. Le worker de `jobs` et le ticker de `scheduler` se détachent par `shutdown.spawn` et font `tokio::select!` entre leur sommeil et `requested()`. Aucune nouvelle ancre.

**Tech Stack:** tokio 1.53 (`signal`, `sync`, `macros`), tokio-util 0.7 (`rt`), axum 0.8 (`with_graceful_shutdown`), minijinja (`{@ @}`), assert_cmd + testcontainers pour la preuve Docker.

**Spec:** `docs/superpowers/specs/2026-09-12-arret-gracieux-design.md`

## Global Constraints

- `rbs-core` : `#![warn(missing_docs)]`, tout item public porte son `///`. API publique en anglais.
- Un commentaire explique le *pourquoi*, jamais le *quoi* ; le code engendré ne commente que ses points d'extension.
- Commits Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.
- Les templates ne sont compilées par aucun test rapide : régénérer les quatre exemples **par diff entre deux générations** (`examples/README.md`), puis `cargo check` et `cargo clippy --all-targets -- -D warnings` dans chacun avant la passe Docker.
- Passe lente : `cargo test -p rbs-cli --test integration_jobs -- --ignored --no-fail-fast`, sortie dans `$SCRATCHPAD/jobs-<étape>.log`.
- Toute page de `docs/` modifiée en anglais l'est en français dans le même commit.
- `integration_docs -- --include-ignored` (14 passés) et `integration_examples` (19 passés) sont les oracles de la doc et des exemples.

---

### Task 1: `rbs_core::shutdown` — le signal et les tâches à attendre

**Files:**
- Create: `crates/rbs-core/src/shutdown.rs`
- Modify: `crates/rbs-core/Cargo.toml` (tokio `signal`, `sync` ; `tokio-util = { workspace = true, features = ["rt"] }`), `Cargo.toml` racine (`tokio-util = "0.7.x"` — dernière stable, vérifier par `cargo add --dry-run`), `crates/rbs-core/src/lib.rs` (`pub mod shutdown;`)

**Interfaces:**
- Produces: `Shutdown::{new, request, is_requested, requested, spawn, wait, on_signal}`, `shutdown::signal()` — signatures du spec.

- [ ] **Step 1: Écrire les tests rouges** dans `shutdown.rs` (module `tests`) :

```rust
#[tokio::test]
async fn a_task_that_listens_finishes_before_wait_returns() {
    let shutdown = Shutdown::new();
    let fini = Arc::new(AtomicBool::new(false));
    let temoin = Arc::clone(&fini);
    let ecoute = shutdown.clone();
    shutdown.spawn(async move {
        ecoute.requested().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        temoin.store(true, Ordering::SeqCst);
    });

    assert_eq!(shutdown.wait(Duration::from_secs(5)).await, 0);
    assert!(fini.load(Ordering::SeqCst));
}

#[tokio::test]
async fn a_task_that_ignores_the_signal_is_counted_at_the_deadline() {
    let shutdown = Shutdown::new();
    shutdown.spawn(std::future::pending::<()>());
    assert_eq!(shutdown.wait(Duration::from_millis(50)).await, 1);
}

#[tokio::test]
async fn requested_resolves_at_once_after_the_request_and_on_every_clone() {
    let shutdown = Shutdown::new();
    let clone = shutdown.clone();
    assert!(!clone.is_requested());
    shutdown.request();
    shutdown.request();
    assert!(clone.is_requested());
    tokio::time::timeout(Duration::from_millis(50), clone.requested()).await.expect("résolu");
}

#[cfg(unix)]
#[tokio::test]
async fn the_listener_resolves_when_the_process_receives_sigterm() {
    let ecoute = Listeners::new().expect("gestionnaires posés");
    std::process::Command::new("kill").args(["-TERM", &std::process::id().to_string()]).status().expect("kill");
    tokio::time::timeout(Duration::from_secs(5), ecoute.wait()).await.expect("SIGTERM reçu");
}
```

- [ ] **Step 2: Rouge** : `cargo test -p rbs-core shutdown::` → erreur de compilation (module absent).
- [ ] **Step 3: Implémenter** `shutdown.rs` : `Shutdown { token: CancellationToken, tasks: TaskTracker }` ; `request` = `cancel()` + `close()` ; `requested` = `token.clone().cancelled_owned()` ; `spawn` = `tasks.spawn` ; `wait` = `request()` puis `timeout(timeout, tasks.wait())`, rend `tasks.len()`, `warn!` si > 0 ; `on_signal` = `async move { signal().await; tracing::info!("arrêt demandé"); token.cancel(); tasks.close(); }` ; `signal()` = `Listeners::new()?.wait().await` (une erreur de pose → `error!` puis `pending()`), `Listeners` privé : `ctrl_c` + `#[cfg(unix)] SignalKind::terminate()`.
- [ ] **Step 4: Vert** : `cargo test -p rbs-core shutdown::` — 4 passés.
- [ ] **Step 5: `CoreState`** : champ `shutdown: Shutdown`, accesseur `shutdown()`, test `the_shutdown_signal_is_shared_by_the_clones_of_the_state`.
- [ ] **Step 6: `ServerConfig.shutdown_timeout_secs`** + défaut figment 30 + les trois littéraux de tests (`state.rs`, `extract.rs`, `health.rs`) ; test dans `config::tests` que le défaut vaut 30.
- [ ] **Step 7:** `cargo test -p rbs-core --all-features`, `cargo clippy -p rbs-core --all-targets --all-features -- -D warnings`, `cargo doc -p rbs-core --no-deps` (missing_docs).
- [ ] **Step 8: Commit** `feat(core): porte le signal d'arrêt et les tâches de fond à attendre`.

### Task 2: `main.rs.jinja`, `config/default.toml.jinja`, worker et ticker

**Files:**
- Modify: `crates/rbs-cli/templates/project/src/main.rs.jinja`, `templates/project/config/default.toml.jinja`, `templates/features/jobs/{worker,tests}.rs.jinja`, `templates/features/scheduler/{mod,ticker,tests}.rs.jinja`, `crates/rbs-cli/tests/integration_jobs.rs` (`TESTS`), `crates/rbs-cli/tests/integration_scheduler.rs` (`TESTS_SOUS_CONTENEUR`)

- [ ] **Step 1: `main.rs.jinja`** — fin de `main` selon le spec ; `let arret = Duration::from_secs(config.server.shutdown_timeout_secs);` à côté d'`adresse` ; `use rbs_core::HasCoreState;`.
- [ ] **Step 2: `default.toml.jinja`** — `shutdown_timeout_secs = 30` sous `[server]`.
- [ ] **Step 3: worker** — `spawn` par `shutdown.spawn` ; `run` → `run_with(state, registry, config)` ; boucle :

```rust
    loop {
        if shutdown.is_requested() { break; }
        reprendre_les_abandonnes(state.core().db(), &config).await;
        match queue::reserver_prochain_job(state.core().db()).await {
            Ok(Some(job)) => execute(&state, &registry, &config, job).await,
            Ok(None) => pause(&shutdown, attente).await,
            Err(error) => { tracing::error!(%error, "dépilage impossible"); pause(&shutdown, attente).await; }
        }
    }
    tracing::info!("worker arrêté");
```

avec `async fn pause(shutdown: &Shutdown, duree: Duration) { tokio::select! { _ = tokio::time::sleep(duree) => {}, _ = shutdown.requested() => {} } }`.

- [ ] **Step 4: test livré** `the_worker_finishes_its_job_and_stops_when_shutdown_is_requested` (`#[ignore]`, joint la base) : job `Slow` (dort 1 s), `state.core().shutdown().spawn(worker::run_with(state.clone(), registry(), config(5)))`, attend `running` (≤ 5 s, pas de 100 ms), `wait(10 s) == 0`, ligne `done`, `attempts == 1`. L'ajouter à `TESTS` d'`integration_jobs.rs`.
- [ ] **Step 5: ticker** — `scheduler::spawn` par `shutdown.spawn` ; `run` : même `select!` ; « calendrier arrêté ». Test `the_ticker_stops_when_shutdown_is_requested` (`spawn(state.clone()).await` puis `wait(5 s) == 0`), ajouté à `TESTS_SOUS_CONTENEUR`.
- [ ] **Step 6: Projet jetable** : `rbs new` + `add jobs` + `add scheduler` dans le scratchpad, `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --lib` (tests ordinaires) — avant Docker.
- [ ] **Step 7: Commit** `feat(templates): arrête le serveur, le worker et le ticker sur Ctrl-C ou SIGTERM`.

### Task 3: La preuve Docker

**Files:**
- Modify: `crates/rbs-cli/tests/integration_jobs.rs`

- [ ] **Step 1:** `Serveur::terminer(self) -> (bool, String)` : `kill -TERM pid`, `try_wait` toutes les 100 ms pendant 30 s, sinon `kill` ; rend (sorti en 0, journal).
- [ ] **Step 2:** `fn slow_down_the_demo_job(racine)` réécrit `src/modules/jobs/demo.rs` : `tokio::time::sleep(Duration::from_secs(3)).await` avant le log.
- [ ] **Step 3:** test `a_sigterm_lets_the_job_in_progress_finish_before_the_process_exits` : projet, `slow_down`, migrate, compile, `Serveur::lancer(.., 1)`, `enqueue`, `wait_for_status("running", 15 s)`, `terminer`, assert sorti en 0, `status == "done"`, `attempts == "1"`.
- [ ] **Step 4:** Rouge d'abord : jouer le test avec le `main.rs.jinja` d'avant (`git stash` de la template) — le processus meurt et la ligne reste `running`. Puis vert : `cargo test -p rbs-cli --test integration_jobs -- --ignored --no-fail-fast > $SCRATCHPAD/jobs-14-docker.log`.
- [ ] **Step 5: Commit** `test(jobs): prouve qu'un SIGTERM laisse finir le job en cours`.

### Task 4: Exemples, message d'`add observability`, documentation, changelog, note

- [ ] **Step 1:** Message `"observability"` de `crates/rbs-cli/src/lib.rs` : `"les métriques sont sur http://localhost:9090/metrics ; pour les traces, nommez un collecteur dans OTEL_EXPORTER_OTLP_ENDPOINT"` ; transcriptions EN/FR du tutoriel.
- [ ] **Step 2:** Régénérer les quatre exemples par diff (deux générations, `diff -ru`), appliquer aux `main.rs` et `config/default.toml` ; `newsletter-queue/src/main.rs` garde `// region: arret` autour de `shutdown.wait(arret).await;` … `rbs_core::logs::shutdown();`. Retirer `"src/main.rs"` d'`edite_a_la_main` et du test des éditions manuelles ; `cargo test -p rbs-cli --test integration_examples` → 19 passés ; `cargo check` + clippy dans les quatre.
- [ ] **Step 3:** Guides EN/FR : `observability` (section « Vider le dernier lot »), `jobs` (worker), `scheduler` (ticker), `configuration` (clé) ; `examples/README{,.fr}.md` ; `integration_docs -- --include-ignored` ; `cd docs && npm test && npm run typecheck`.
- [ ] **Step 4:** `CHANGELOG.md` / `CHANGELOG.fr.md` sous 1.5.0 « Added » ; `crates/rbs-cli/notes/1.5.0.md` section « L'arrêt gracieux ».
- [ ] **Step 5:** Fin de passe : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p rbs-cli --lib`, `cargo test -p rbs-core --all-features`.
- [ ] **Step 6: Commit** `docs: décrit l'arrêt gracieux et retire l'appel manuel de logs::shutdown()`.
