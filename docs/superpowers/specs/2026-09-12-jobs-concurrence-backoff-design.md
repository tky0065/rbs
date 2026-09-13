# Le worker de jobs : plusieurs jobs de front, et un délai de reprise qui double

Date : 2026-09-12
Portée : `crates/rbs-cli/templates/features/jobs/{config,worker,queue,tests}.rs.jinja` et
`feature.toml`, `crates/rbs-cli/src/doctor/jobs.rs`, le test de `rbs add jobs` dans
`crates/rbs-cli/src/add/mod.rs`, `crates/rbs-cli/tests/integration_jobs.rs`,
`examples/newsletter-queue/`, guide `jobs` et page `doctor` en deux langues, `CHANGELOG` en deux
langues, `crates/rbs-cli/notes/1.5.0.md`.
Hors portée : des priorités, des files nommées, un worker par `kind`, une gigue sur le délai,
un binaire worker séparé.

## Le problème

`worker.rs.jinja` boucle `réserver → execute(...).await` : un job à la fois par processus,
strictement. `config.rs.jinja` n'expose aucune concurrence, et `queue::retry_or_fail` repose
un job raté à `retry_delay_secs` constants. Une livraison webhook attend jusqu'à dix
secondes (`webhooks/config.rs.jinja`) : un `emit` vers cent abonnés dont un tiers ne
répond pas bloque la file plusieurs minutes, mails et échéances du calendrier derrière —
et le receveur en panne est réessayé toutes les trente secondes, cinq fois, sans jamais
lui laisser plus de répit.

S'ajoute un défaut que l'arrêt gracieux vient de rendre visible : un job qui panique
emporte la tâche du worker, détachée sans surveillance. La file cesse de se vider, l'API
répond, et rien ne le dit.

## Ce qui change

### `[jobs] concurrency`

`Config` gagne `concurrency: usize`, défaut **4**, porté par `#[serde(default)]` comme les
autres clés et inscrit dans le bloc `[[config]]` du manifeste. Quatre : le pool de la base
compte dix connexions par défaut (`database.max_connections`), chaque job en cours peut en
tenir une, et la réservation une de plus ; en laisser plus de la moitié à l'API est le
partage qui ne surprend personne. Un projet qui ne fait que livrer des webhooks montera à
seize ; un projet dont les jobs saturent une ressource externe descendra à un, et retrouve
alors le comportement d'aujourd'hui. Zéro vaut un : un worker qui ne réserve rien n'est
pas une configuration, c'est une panne muette.

`rbs doctor` propose la clé dans le bloc qu'il colle quand la section manque, et le test de
`rbs add jobs` l'exige dans `config/default.toml`.

### La boucle

```rust
let registry = Arc::new(registry);
let mut en_cours = JoinSet::new();

while !shutdown.is_requested() {
    reprendre_les_abandonnes(db, &config).await;

    // Remplir jusqu'à `concurrency`, tant que la file donne.
    let mut vide = false;
    while en_cours.len() < config.concurrency && !shutdown.is_requested() {
        match queue::reserver_prochain_job(db).await {
            Ok(Some(job)) => { en_cours.spawn(/* execute(&state, &registry, &config, job) */); }
            Ok(None) => { vide = true; break; }
            Err(error) => { tracing::error!(%error, "dépilage impossible"); vide = true; break; }
        }
    }

    tokio::select! {
        Some(fini) = en_cours.join_next(), if !en_cours.is_empty() => dire_si_panique(fini),
        _ = tokio::time::sleep(attente), if vide => {}
        _ = shutdown.requested() => {}
    }
}

// Ce qui est en main finit : l'arrêt attend, il n'interrompt pas.
while let Some(fini) = en_cours.join_next().await { dire_si_panique(fini); }
```

Un `JoinSet` et non un `Semaphore` : le worker doit *attendre* ses jobs à l'arrêt, et le
`JoinSet` est ce qui les tient. `execute` garde sa signature — les tests livrés jouent le
réessai tour par tour avec elle — et s'exécute dans une tâche à elle ; `Registry` passe
sous `Arc` parce que ses fermetures ne sont pas clonables.

Le `select!` n'a que trois cas et chacun a sa garde : la file pleine attend la fin d'un job ;
la file vide attend le tour suivant ou la fin d'un job ; l'arrêt sort dans tous les cas.
La réservation ne se fait jamais avec l'ensemble plein — c'est ce qui borne la
concurrence, et c'est pourquoi la borne est lue à l'entrée de la boucle de remplissage,
pas au `spawn`.

Une tâche qui panique est un `JoinError` : le worker le dit (`error!`, avec l'identifiant du
job s'il est connu) et continue. La ligne reste `running` jusqu'au bail, qui la reprend —
c'est le cas pour lequel le bail existe. Avant, la panique tuait le worker.

### Le délai de reprise

`queue.rs` gagne une fonction pure :

```rust
/// `retry_delay_secs × 2^(attempts − 1)`, borné par `retry_max_delay_secs`.
pub(super) fn retry_delay(config: &Config, attempts: i32) -> Duration
```

`attempts` est celui de la ligne, déjà incrémenté à la réservation : la première tentative
ratée attend `retry_delay_secs`, la deuxième le double, et ainsi de suite. La
multiplication sature (`checked_pow` puis `saturating_mul`) avant d'être bornée : un
`max_attempts` de soixante ne fait pas déborder un `u64`. `retry_or_fail` l'appelle à la
place du délai constant.

`Config` gagne `retry_max_delay_secs: u64`, défaut **3600**. Aux défauts (`30`, cinq
tentatives) le plafond n'est jamais atteint — 30, 60, 120, 240 secondes ; il compte pour
qui monte `max_attempts` à vingt pour réessayer un receveur pendant une journée, et sans
lui ce réglage attendrait bientôt des semaines. Une heure est le plafond que les
services de webhooks pratiquent.

`retry_delay_secs = 0` reste zéro à toute tentative : les tests livrés qui rejouent le
réessai sans attendre ne changent pas.

## Ce que l'écosystème du dépôt doit suivre

- `tests.rs.jinja` : le littéral `Config { … }` de `config()` gagne les deux champs ; trois
  tests unitaires du délai (sans base) ; un test de concurrence (joint la base), exigé
  nommément par `integration_jobs`.
- `doctor/jobs.rs` : le bloc de remède et la liste de clés du test ; `add/mod.rs` : la liste
  de clés exigées ; `docs/docs/cli/doctor.md` EN/FR : la transcription du remède.
- `examples/newsletter-queue` : `worker.rs`, `queue.rs`, `config.rs`, `tests.rs` et
  `config/default.toml` suivent la génération.
- Guide `jobs` EN/FR : le bloc de configuration (six clés), la section « The worker » (la
  note « un worker par processus » dit désormais « jusqu'à `concurrency` jobs de front »,
  et le sort d'un job qui panique), la section « Retries » (le délai double).
- `CHANGELOG` EN/FR sous 1.5.0 ; `notes/1.5.0.md` : les deux clés ont un défaut, un projet
  déjà engendré n'a rien à écrire, et reprend `worker.rs` et `queue.rs` du fragment s'il
  veut la concurrence.

## Tests

Sans base, dans `tests.rs.jinja` :

- `the_retry_delay_doubles_with_each_attempt` : `(30 s, 1) → 30`, `(30 s, 2) → 60`,
  `(30 s, 5) → 480` ;
- `the_retry_delay_is_capped` : `(30 s, 20)` avec plafond 3600 → 3600 ;
- `the_retry_delay_saturates_rather_than_overflowing` : `(30 s, 200)` → le plafond, sans
  panique ;
- `a_zero_retry_delay_stays_zero` : `(0 s, 7) → 0`.

Avec la base : `jobs_run_side_by_side_up_to_the_configured_concurrency` — un job `Meet`
attend une `Barrier` de deux, deux `Meet` sont enfilés, la boucle tourne avec
`concurrency = 2` ; les deux passent `done` en moins de dix secondes. Un worker
strictement séquentiel bloque le premier sur la barrière et le test échoue sur le
délai : c'est le rouge. Puis `shutdown.wait(10 s) == 0`.

Docker : `integration_jobs` au complet (les tests livrés sont exigés par nom, dont le
nouveau), et `integration_examples` pour la non-dérive de `newsletter-queue`.

Ce que ces tests ne prouvent pas : un gain de débit chiffré. Le critère est structurel —
deux jobs qui doivent se rencontrer se rencontrent — et non une mesure de temps, que la
machine de CI rendrait instable.
