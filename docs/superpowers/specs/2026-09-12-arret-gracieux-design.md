# Arrêt gracieux des projets engendrés

Date : 2026-09-12
Portée : `crates/rbs-core/src/{shutdown,state,config,lib}.rs`, `crates/rbs-core/Cargo.toml`,
`crates/rbs-cli/templates/project/src/main.rs.jinja`, `templates/project/config/default.toml.jinja`,
`templates/features/jobs/{worker,tests}.rs.jinja`, `templates/features/scheduler/{mod,ticker,tests}.rs.jinja`,
le message d'`add observability` (`crates/rbs-cli/src/lib.rs`), les quatre projets d'`examples/`,
guides `jobs`, `scheduler`, `observability`, `configuration` et tutoriel `observability` en deux
langues, `examples/README{,.fr}.md`, `CHANGELOG` en deux langues, `crates/rbs-cli/notes/1.5.0.md`.
Hors portée : un second signal qui forcerait la sortie pendant le drainage ; l'arrêt du
listener `/metrics` d'`observability` (il n'a aucun état en vol) ; `stop_grace_period` dans le
compose du fragment `docker` ; un binaire worker séparé.

## Le problème

`main.rs.jinja` sert le routeur par un `axum::serve(...).await?` nu. Aucun projet engendré
n'écoute Ctrl-C ni SIGTERM, et aucun n'appelle `rbs_core::logs::shutdown()` — seul
`examples/newsletter-queue` le fait, à la main. Conséquences, vérifiées dans le code du 12
septembre :

- `docker stop`, `kubectl delete pod`, `systemctl stop` envoient SIGTERM : sans gestionnaire,
  le processus meurt sur-le-champ. Les requêtes en vol sont coupées, connexion fermée sans
  réponse.
- Le dernier lot de spans OTLP est perdu : `logs::shutdown()` existe (`rbs-core/src/logs/mod.rs:140`)
  et personne ne l'appelle.
- Le worker de `jobs` (`worker.rs.jinja`) et le ticker de `scheduler` (`ticker.rs.jinja`)
  tournent dans le processus de l'API, détachés par l'ancre `startup`. Un job en cours
  d'exécution est tué avec le processus, sa ligne reste `running`, et le bail de
  `lease_secs` (300 s par défaut) est le seul à la rendre à la file — cinq minutes de
  retard sur chaque déploiement, pour un job qui aurait fini en une seconde.

## Options pesées

### A. Une quinzième ancre, `// <rbs:shutdown>`

Celle que suggère le backlog : `main.rs.jinja` gagnerait, après `axum::serve`, une ancre où
`jobs` et `scheduler` inséreraient chacun leur instruction d'arrêt. Le mécanisme des ancres
optionnelles existe (`anchors.rs`, `plan/mod.rs`).

Coût : le compte « quatorze ancres » vit dans `CLAUDE.md`, `docs/docs/compatibility.md`,
`cli/add.md`, `cli/generate.md`, `cli/new.md` et leurs versions françaises, et dans `rbs
doctor` ; une ancre de plus exige une note d'`upgrade` et un `doctor --fix` qui sache la
reposer. Surtout, ce qu'elle porterait — un `.await` sur une poignée de tâche — suppose que
chaque fragment ait rangé sa poignée quelque part d'accessible depuis `main`, donc une
seconde ancre ou une variable engendrée par fragment. La solution grossit à chaque brique
de fond, et chaque brique doit écrire le même `select!`.

### B. Un signal d'arrêt porté par `rbs-core`, exposé par `CoreState` — retenue

L'écoute d'un signal POSIX, un drapeau « arrêt demandé » et l'attente d'un ensemble de
tâches n'ont aucune raison de varier d'un projet à l'autre : c'est exactement le critère du
CLAUDE.md pour entrer dans le noyau. `rbs-core` est en 1.5.0 non publiée, l'ajout est
additif (`CoreState` est `#[non_exhaustive]` à champs privés, un champ de plus ne casse
aucun projet).

Le contenu des ancres `startup` ne change pas : `spawn(state.clone())` et
`spawn(state.clone()).await?` restent tels quels, si bien qu'un `rbs add jobs` sur un projet
engendré avant cette version compile toujours — son worker s'arrête simplement avec le
processus, comme aujourd'hui, tant que le projet n'a pas repris la fin de `main.rs`
(documentée dans la note 1.5.0).

### C. Le même signal, engendré dans le projet (`src/shutdown.rs`)

Rejetée : ce fichier serait identique dans tous les projets, personne n'aurait à le lire
ni à le modifier, et chaque correctif du noyau devrait être recopié à la main. C'est le
cas d'école de ce que la frontière noyau / engendré range dans le noyau.

## Le module `rbs_core::shutdown`

```rust
/// Le signal d'arrêt du processus, et les tâches de fond qu'il doit attendre.
#[derive(Debug, Clone, Default)]
pub struct Shutdown { token: CancellationToken, tasks: TaskTracker }

impl Shutdown {
    pub fn new() -> Self;
    /// Demande l'arrêt. Idempotent.
    pub fn request(&self);
    pub fn is_requested(&self) -> bool;
    /// Se résout à la demande d'arrêt, tout de suite si elle est déjà faite.
    /// Fait pour `tokio::select!` : l'abandonner n'a aucun effet.
    pub fn requested(&self) -> impl Future<Output = ()> + Send + 'static;
    /// Détache une tâche de fond que `wait` attendra.
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>;
    /// Demande l'arrêt si ce n'est pas fait, puis attend les tâches détachées par
    /// `spawn`, au plus `timeout`. Rend le nombre de tâches encore en cours à
    /// l'échéance — zéro quand l'arrêt est propre — et l'inscrit au journal.
    pub async fn wait(&self, timeout: Duration) -> usize;
    /// Se résout au premier Ctrl-C ou SIGTERM, et demande l'arrêt.
    /// Fait pour `axum::serve(...).with_graceful_shutdown(...)`.
    pub fn on_signal(&self) -> impl Future<Output = ()> + Send + 'static;
}

/// Se résout au premier Ctrl-C ou, hors Windows, SIGTERM — celui que `docker stop`,
/// systemd et Kubernetes envoient.
pub async fn signal();
```

`CancellationToken` et `TaskTracker` viennent de `tokio-util` (feature `rt`), déjà dans
l'arbre de tout projet engendré par `h2` → `hyper` → `axum` : la dépendance ne coûte
aucune compilation. Les réécrire à la main (`watch` + compteur + `Notify` + garde de
`Drop`) serait soixante lignes pour retrouver, en moins sûr, ce que l'équipe de tokio
maintient.

`tokio` gagne les features `signal` et `sync` dans `rbs-core`. `signal()` n'installe les
gestionnaires qu'au premier `poll` ; sur une plateforme non Unix, seul Ctrl-C est écouté.

`CoreState` gagne un champ `shutdown: Shutdown`, créé par `CoreState::new`, et un accesseur
`shutdown(&self) -> &Shutdown`. Les clones de `CoreState` partagent le même signal (le
token et le tracker sont des `Arc` internes) : l'`AppState` que reçoivent le routeur, le
worker et le ticker est le même signal que celui que `main` déclenche.

## `server.shutdown_timeout_secs`

`ServerConfig` gagne `shutdown_timeout_secs: u64`, défaut figment `30`, inscrit dans
`config/default.toml.jinja` sous `[server]` comme `timeout_secs`. C'est la borne de
`Shutdown::wait` : au-delà, `main` sort sans attendre les tâches de fond restantes, qui
meurent avec le processus — et le bail de `lease_secs` reprend alors son rôle.

Pourquoi pas `server.timeout_secs` : il borne une *requête* ; un job a le droit de durer plus
longtemps (`lease_secs` est à 300 s). Trente secondes : c'est la grâce que Kubernetes
accorde par défaut avant SIGKILL, et un défaut au-delà attendrait un signal qui ne viendra
jamais. Docker n'en accorde que dix — un projet dont les jobs dépassent dix secondes règle
`stop_grace_period` dans son compose, ce qui est hors de cette tâche.

## La fin de `main.rs.jinja`

```rust
    let state = state::AppState::new(db, config)?;

    // <rbs:startup>
    // </rbs:startup>

    let shutdown = state.core().shutdown().clone();
    let app = router::router(state);
    …
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(shutdown.on_signal())
        .await?;

    shutdown.wait(arret).await;
    rbs_core::logs::shutdown();

    Ok(())
```

`arret` est lu de `config.server.shutdown_timeout_secs` avant que `AppState::new` ne
consomme `config`, comme `adresse`. Le clone du signal est pris avant `router::router(state)`
pour la même raison. `main.rs` gagne `use rbs_core::HasCoreState;`.

L'ordre est celui des dépendances : `on_signal` demande l'arrêt à l'instant du signal, donc
le worker et le ticker cessent de réserver pendant qu'axum draine les connexions (bornées
par `timeout_secs`) ; `wait` attend ensuite ce qu'ils ont encore en main ; `logs::shutdown()`
vient en dernier, pour que les spans des derniers jobs partent avec le dernier lot.

## Le worker et le ticker

`jobs::worker::spawn` détache par `state.core().shutdown().spawn(...)` et non `tokio::spawn`.
La boucle de `run` fait `tokio::select!` entre le sommeil de `poll_interval_secs` et
`requested()`, et vérifie `is_requested()` en tête de tour : un job réservé est toujours
exécuté jusqu'au bout et son sort inscrit — c'est la garantie que le bail ne rattrape
plus rien en temps normal. `run` se scinde : `run(state)` lit la configuration et le
registre, puis délègue à `run_with(state, registry, config)`, visible du module pour que
les tests livrés jouent la boucle sur un registre à eux. Le worker dit « worker arrêté »
en sortant.

`scheduler::spawn` détache de même, et `ticker::run` fait le même `select!` : un tour
commencé finit (chaque échéance est une transaction courte), aucun nouveau tour ne
commence. `tokio::select!` demande la feature `macros`, que tout projet engendré porte
déjà.

## Ce que l'écosystème du dépôt doit suivre

- **`examples/`** : les quatre `main.rs` et les quatre `config/default.toml` changent.
  `newsletter-queue/src/main.rs` cesse d'être édité à la main : le squelette porte
  désormais l'appel, et l'exemple ne garde que ses marqueurs `// region: arret` autour des
  lignes engendrées (le test de non-dérive ignore les marqueurs). `src/main.rs` sort de
  `edite_a_la_main` et du test `the_hand_edits_of_newsletter_queue_are_in_place` ;
  `examples/README{,.fr}.md` perdent son paragraphe.
- **Message d'`add observability`** : il demandait d'appeler `logs::shutdown()` soi-même ;
  l'instruction est fausse sur un projet 1.5.0. Le message ne parle plus que du collecteur ;
  les deux transcriptions du tutoriel `observability` suivent (`integration_docs`).
- **Guides** : `observability` (« Vider le dernier lot » décrit désormais ce que le squelette
  fait), `jobs` (le worker finit son job et s'arrête), `scheduler` (le ticker s'arrête),
  `configuration` (la clé), en deux langues.
- **`notes/1.5.0.md`** : un projet engendré avant 1.5.0 reprend la fin de son `main.rs` et,
  s'il porte `jobs` ou `scheduler`, les boucles du worker et du ticker ; `rbs upgrade` ne
  réécrit ni l'un ni les autres.
- **`CHANGELOG`** : une entrée « Added » en deux langues.

## Tests

Dans `rbs-core` (`shutdown::tests`, sans base) :

- une tâche qui écoute `requested()` rend la main, et `wait` rend `0` ;
- une tâche qui ignore le signal est comptée à l'échéance (`wait` rend `1`) ;
- `requested()` se résout tout de suite après `request()`, qui est idempotent ;
- un clone du signal voit la demande faite sur l'autre ;
- (Unix) le futur de `signal()` se résout quand le processus reçoit SIGTERM, envoyé par
  `kill -TERM` sur son propre pid une fois les gestionnaires posés.

Dans le projet engendré (`jobs/tests.rs.jinja`, joint la base, exigé nommément par
`integration_jobs`) : `the_worker_finishes_its_job_and_stops_when_shutdown_is_requested` —
un job d'une seconde est enfilé, la boucle détachée par le signal de l'état, l'arrêt
demandé pendant que la ligne est `running`, et `wait` rend `0` avec la ligne `done`.
Dans `scheduler/tests.rs.jinja` : `the_ticker_stops_when_shutdown_is_requested`.

Dans `integration_jobs.rs` (Docker) :
`a_sigterm_lets_the_job_in_progress_finish_before_the_process_exits` — le binaire est lancé,
son job d'exemple réécrit pour dormir trois secondes, un job enfilé par `psql`, SIGTERM
envoyé quand la ligne est `running` ; le processus doit sortir en 0 dans les trente
secondes, la ligne être `done` avec `attempts = 1`.

Ce que ces tests ne prouvent pas : le drainage d'une requête HTTP lente. Aucune route
engendrée n'est lente, et en écrire une pour le test serait tester axum. Le drainage repose
sur `with_graceful_shutdown`, dont c'est la promesse documentée.
