# Jobs : bail sur les lignes `running` abandonnées — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un job réservé par un worker qui meurt (crash, `docker stop`, rolling restart) redevient `pending` passé un bail, au lieu de rester `running` pour toujours.

**Architecture:** Aucune colonne nouvelle : `updated_at` est posé par la réservation (`queue.rs.jinja`, `SET … updated_at = $3`). Une fonction `queue::requeue_stale(db, lease)` fait un `UPDATE jobs SET status = 'pending', updated_at = now WHERE status = 'running' AND updated_at < now - lease` par `Entity::update_many().col_expr(…)` (portable sur les trois moteurs, instant lié en paramètre comme tout le fragment — jamais `Expr::current_timestamp()`, qui casse sur SQLite). Le worker l'appelle au démarrage puis **à chaque tour** de sa boucle, avant la réservation. `attempts`, déjà incrémenté à la réservation, n'est pas rendu : un crash dépense sa tentative comme un échec, et `max_attempts` borne toujours les reprises. Le bail vient d'une clé `lease_secs` (défaut 300) de la section `[jobs]`.

**Tech Stack:** minijinja (`{@ @}`), SeaORM `update_many` + `sea_query::Expr`, chrono `TimeDelta`, tests `#[ignore]` joints à la base, régénération de `examples/newsletter-queue` par diff.

**Spec:** design validé en chat (tâche 3 d'`IMPROVE.md`) — tâche *bounded*, sans document de spec.

## Global Constraints

- Conventional Commits en français, sans identifiant de tâche, sans renvoi à `IMPROVE.md`, sans `Co-Authored-By` ni `Claude-Session`.
- Un commentaire dit le *pourquoi*, jamais le *quoi*.
- Documentation bilingue : `docs/docs/guides/jobs.md`, `docs/docs/cli/doctor.md` et leurs jumeaux sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/` changent dans le même commit.
- `examples/newsletter-queue` porte `jobs` : `integration_examples.rs` compare la régénération octet à octet. Régénérer **par diff** (mémoire : jamais par écrasement — l'exemple porte des éditions manuelles listées dans `examples/README.md:115-124`).
- Le code des fragments n'est compilé par aucun test rapide : `cargo check --tests` dans `examples/newsletter-queue` avant la passe lente.
- Les clés `max_attempts`, `retry_delay_secs`, `poll_interval_secs` sont figées en quatre endroits côté rbs : `src/doctor/jobs.rs:21,46,97`, `src/add/mod.rs:1182`, plus `docs/docs/cli/doctor.md:133` (EN et FR). Chacun reçoit `lease_secs`.

---

### Task 1: `lease_secs` dans la configuration du fragment, et partout où rbs la fige

**Files:**
- Modify: `crates/rbs-cli/templates/features/jobs/config.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/jobs/feature.toml` (bloc `[[config]]`, fin de fichier)
- Modify: `crates/rbs-cli/src/doctor/jobs.rs:21,46,97`
- Modify: `crates/rbs-cli/src/add/mod.rs:1182`
- Modify: `docs/docs/cli/doctor.md:128-134` et `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/doctor.md` (même bloc)
- Modify: `docs/docs/guides/jobs.md:73-81` et FR (`## Configuration`, ligne ~69-86)

**Interfaces:**
- Produces: `Config { max_attempts: i32, retry_delay_secs: u64, poll_interval_secs: u64, lease_secs: u64 }` ; `default_lease() -> u64 = 300`.

- [ ] **Step 1: Tests rouges côté rbs**

Dans `src/doctor/jobs.rs:97` et `src/add/mod.rs:1182`, ajouter `"lease_secs"` aux tableaux de clés. Lancer :

```bash
cargo test -p rbs-cli --lib -- jobs 2>&1 | grep -E 'FAILED|failed|passed' | head
```

Attendu : au moins `a_missing_section_names_every_key` (ou le nom réel) en échec sur `lease_secs`.

- [ ] **Step 2: Le champ et son défaut**

`config.rs.jinja`, après `poll_interval_secs` :

```rust
    /// Durée au-delà de laquelle une réservation sans suite est tenue pour abandonnée,
    /// en secondes. À régler au-dessus du plus long job : un job encore en cours au-delà
    /// du bail est rendu à la file, et rejoué.
    #[serde(default = "default_lease")]
    pub lease_secs: u64,
```

et en bas :

```rust
fn default_lease() -> u64 {
    300
}
```

Mettre à jour le commentaire de doc de `Config` si sa phrase « Les défauts sont portés ici » n'a pas à changer — elle reste vraie, ne pas y toucher.

- [ ] **Step 3: La clé dans `default.toml` et dans les remèdes**

`feature.toml`, bloc `[[config]]` — remplacer le contenu par :

```toml
content = """
# Tentatives d'un job avant qu'il soit tenu pour définitivement en échec, et attente
# avant qu'une tentative ratée redevienne exécutable. Ces deux valeurs se règlent à
# l'usage : elles sont ici pour ne pas avoir à rouvrir le fragment.
max_attempts = 5
retry_delay_secs = 30
poll_interval_secs = 1
# Passé ce délai, un job réservé par un worker qui n'en a jamais inscrit le sort — tué en
# plein travail — redevient dépilable. À tenir au-dessus de la durée du plus long job.
lease_secs = 300
"""
```

`src/doctor/jobs.rs` : les trois chaînes `"max_attempts = 5\nretry_delay_secs = 30\npoll_interval_secs = 1"` reçoivent `\nlease_secs = 300` (ligne 21 : le remède ; ligne 46 : la fixture ; ligne 97 : déjà faite au Step 1).

- [ ] **Step 4: La doc qui transcrit ces clés**

`docs/docs/cli/doctor.md:133` : ajouter `      lease_secs = 300` sous `poll_interval_secs = 1` (même indentation), idem FR. `docs/docs/guides/jobs.md:73-81` : « Three settings » → « Four settings », bloc TOML avec `lease_secs = 300`, et une phrase : « `lease_secs` is the delay after which a reservation nobody reported on — the worker died mid-job — is given back to the queue; keep it above your longest job. » Idem FR (« Trois réglages » → « Quatre »).

- [ ] **Step 5: Vert côté rbs**

```bash
cargo test -p rbs-cli --lib -- jobs 2>&1 | tail -3
cargo test -p rbs-cli --test integration_docs 2>&1 | tail -3
```

Attendu : tout passe, `integration_docs` compris (le bloc de `doctor.md` est un transcript gardé : s'il échoue, lire le diff qu'il rend et aligner).

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/features/jobs/config.rs.jinja crates/rbs-cli/templates/features/jobs/feature.toml crates/rbs-cli/src/doctor/jobs.rs crates/rbs-cli/src/add/mod.rs docs/docs/cli/doctor.md docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/doctor.md docs/docs/guides/jobs.md docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/jobs.md
git commit -m "feat(jobs): expose le bail d'une réservation par lease_secs" -m "Vérifications :
- cargo test -p rbs-cli --lib -- jobs : <N> passés
- cargo test -p rbs-cli --test integration_docs : vert"
```

(L'exemple `newsletter-queue` est mis à jour en Task 3, avec le reste.)

---

### Task 2: `queue::requeue_stale` et son appel par le worker

**Files:**
- Modify: `crates/rbs-cli/templates/features/jobs/queue.rs.jinja` (imports lignes 1-10 ; nouvelle fonction après `reserver_en_deux_temps`)
- Modify: `crates/rbs-cli/templates/features/jobs/worker.rs.jinja:22-46`
- Modify: `crates/rbs-cli/templates/features/jobs/tests.rs.jinja` (`config()` ligne ~123 ; deux tests après `a_failing_job_is_retried_then_marked_failed_after_the_last_attempt`)
- Modify: `crates/rbs-cli/tests/integration_jobs.rs:24-29` (`TESTS: [&str; 4]` → 6)

**Interfaces:**
- Consumes: `Entity::update_many()`, `Column::{Status, UpdatedAt}` (dérivés par `DeriveEntityModel` dans `model.rs.jinja`), `Status::as_str()`.
- Produces: `pub async fn requeue_stale(db: &DatabaseConnection, lease: Duration) -> anyhow::Result<u64>` (nombre de lignes rendues).

- [ ] **Step 1: Écrire les deux tests livrés (rouges : `requeue_stale` n'existe pas)**

Dans `tests.rs.jinja`, `config()` reçoit `lease_secs: 300,` (sinon le fichier ne compile plus). Ajouter `use std::time::Duration;` et `use sea_orm::sea_query::Expr;` / `use sea_orm::{ColumnTrait, QueryFilter}` selon besoin, et `super::model::Column`. Puis :

```rust
/// Vieillit la réservation de `id` : ce que ferait le temps, sans attendre le bail.
async fn reserved_since(db: &DatabaseConnection, id: Uuid, age: Duration) {
    let alors = (chrono::Utc::now() - chrono::TimeDelta::from_std(age).expect("durée finie")).fixed_offset();
    Entity::update_many()
        .col_expr(Column::UpdatedAt, Expr::value(alors))
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .expect("la réservation se vieillit");
}

/// Un worker tué entre la réservation et l'inscription du sort laissait sa ligne en
/// `running` pour toujours : rien ne la dépilait plus.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_job_left_running_past_the_lease_returns_to_the_queue() {
    const BAIL: Duration = Duration::from_secs(300);

    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let id = queue::enqueue(db, &Succeeds { marque: "abandonné".into() })
        .await
        .expect("le job s'enfile");
    let reserve = queue::reserver_prochain_job(db)
        .await
        .expect("dépilage possible")
        .expect("le job est dépilable");
    assert_eq!(reserve.status, Status::Running);
    reserved_since(db, id, BAIL + Duration::from_secs(60)).await;

    let rendus = queue::requeue_stale(db, BAIL).await.expect("reprise possible");
    assert_eq!(rendus, 1);

    let ligne = Entity::find_by_id(id).one(db).await.expect("lecture possible").expect("la ligne survit");
    assert_eq!(ligne.status, Status::Pending);
    // La tentative abandonnée reste dépensée : `max_attempts` borne aussi les crashs.
    assert_eq!(ligne.attempts, 1);

    let repris = queue::reserver_prochain_job(db)
        .await
        .expect("dépilage possible")
        .expect("le job rendu est de nouveau dépilable");
    assert_eq!(repris.id, id);
    assert_eq!(repris.attempts, 2);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_job_running_within_the_lease_is_left_alone() {
    const BAIL: Duration = Duration::from_secs(300);

    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let id = queue::enqueue(db, &Succeeds { marque: "en cours".into() })
        .await
        .expect("le job s'enfile");
    queue::reserver_prochain_job(db)
        .await
        .expect("dépilage possible")
        .expect("le job est dépilable");

    let rendus = queue::requeue_stale(db, BAIL).await.expect("reprise possible");
    assert_eq!(rendus, 0);

    let ligne = Entity::find_by_id(id).one(db).await.expect("lecture possible").expect("la ligne survit");
    assert_eq!(ligne.status, Status::Running);
}
```

- [ ] **Step 2: La fonction dans `queue.rs.jinja`**

Imports : ajouter `std::time::Duration`, `sea_orm::sea_query::Expr`, `sea_orm::{ColumnTrait, QueryFilter}` (fusionner dans le `use sea_orm::{…}` existant) et `Column` dans `use super::model::{…}`. Après `reserver_en_deux_temps` :

```rust
/// Rend à la file les jobs qu'un worker a réservés sans jamais en inscrire le sort.
///
/// Un processus tué entre la réservation et `mark_done` laisse sa ligne en `running`, et
/// rien d'autre ne la dépilerait plus : chaque redémarrage perdrait en silence le job en
/// cours. Passé `lease`, la réservation est tenue pour abandonnée. Un job encore en cours
/// au-delà du bail serait donc rejoué — c'est pourquoi `lease_secs` se règle au-dessus
/// du plus long job.
///
/// `attempts` n'est pas rendu : incrémenté à la réservation, il compte le crash comme
/// une tentative, et `max_attempts` borne les reprises d'un job qui tue son worker.
///
/// La borne est liée en paramètre, comme dans les trois requêtes de réservation : un
/// `now()` du moteur comparerait l'horloge du serveur à un `updated_at` posé par
/// l'application — et, sur SQLite, du texte à du texte d'un autre format.
pub async fn requeue_stale(db: &DatabaseConnection, lease: Duration) -> anyhow::Result<u64> {
    let maintenant = Utc::now().fixed_offset();
    let limite = maintenant - TimeDelta::from_std(lease)?;

    let resultat = Entity::update_many()
        .col_expr(Column::Status, Expr::value(Status::Pending.as_str()))
        .col_expr(Column::UpdatedAt, Expr::value(maintenant))
        .filter(Column::Status.eq(Status::Running.as_str()))
        .filter(Column::UpdatedAt.lt(limite))
        .exec(db)
        .await?;

    Ok(resultat.rows_affected)
}
```

Si `Column::Status.eq(&str)` ne compile pas (colonne `ActiveEnum`), employer `Status::Running` directement (`DeriveActiveEnum` implémente `Into<Value>`) et `Expr::value(Status::Pending)` ; s'inspirer d'`auth/repository/user.rs.jinja:51-63`, qui fait un `update_many().col_expr` sur ce même patron.

- [ ] **Step 3: L'appel dans le worker**

`worker.rs.jinja`, dans `run` :

```rust
    let bail = Duration::from_secs(config.lease_secs);

    tracing::info!(
        poll_interval_secs = config.poll_interval_secs,
        lease_secs = config.lease_secs,
        "worker prêt"
    );

    loop {
        rendre_les_abandonnes(state.core().db(), bail).await;

        match queue::reserver_prochain_job(state.core().db()).await {
```

et, après `run` :

```rust
/// Rend à la file ce qu'un worker mort a laissé en `running` — au démarrage, puis à
/// chaque tour : un `UPDATE` indexé qui ne touche rien le plus souvent, contre une
/// livraison perdue sans bruit à chaque redémarrage.
///
/// Un échec ne retire pas le worker : la base injoignable sera dite par le dépilage qui
/// suit, et la reprise retentera au tour d'après.
async fn rendre_les_abandonnes(db: &sea_orm::DatabaseConnection, bail: Duration) {
    match queue::requeue_stale(db, bail).await {
        Ok(0) => {}
        Ok(rendus) => tracing::warn!(rendus, "jobs abandonnés rendus à la file"),
        Err(error) => tracing::error!(%error, "reprise des jobs abandonnés impossible"),
    }
}
```

(Importer `DatabaseConnection` proprement plutôt que le chemin complet si le fichier a déjà un `use sea_orm`.)

- [ ] **Step 4: Inscrire les deux noms côté rbs**

`integration_jobs.rs` : `TESTS: [&str; 6]` avec `a_job_left_running_past_the_lease_returns_to_the_queue` et `a_job_running_within_the_lease_is_left_alone`. Vérifier que le nombre « quatre » n'est pas écrit en toutes lettres ailleurs dans ce fichier ni dans `docs/docs/guides/jobs.md:181-190` (« Four of them are the ones worth keeping » : cette phrase parle des quatre tests qui comptent, elle reste vraie — ne pas la toucher).

- [ ] **Step 5: Rendre, formater, compiler à froid**

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/20433d69-a7c1-492d-8d62-250f705907e5/scratchpad/jobs-bail
rm -rf $S && mkdir -p $S && cd $S
cargo run -q --manifest-path /Users/yacoubakone/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- new demo --yes --with jobs --database-url 'postgres://rbs:rbs@localhost:5432/demo' >/dev/null
cd demo && cargo fmt --check -- src/modules/jobs/*.rs && echo FMT_OK && cargo check --tests 2>&1 | tail -5
```

Attendu : `FMT_OK`, `Finished`. Si rustfmt reformate, reporter dans la template (attention aux `-%}` qui mangent l'indentation).

- [ ] **Step 6: La preuve lente — Docker**

```bash
cd /chemin/du/worktree
cargo test -p rbs-cli --test integration_jobs -- --ignored --no-fail-fast > $S/integration_jobs.log 2>&1; tail -15 $S/integration_jobs.log
```

Attendu : `test result: ok.` avec tous les tests d'`integration_jobs` passés (plusieurs minutes). Lire le log entier, y chercher `a_job_left_running_past_the_lease_returns_to_the_queue ... ok`.

- [ ] **Step 7: Commit**

```bash
git add crates/rbs-cli/templates/features/jobs/queue.rs.jinja crates/rbs-cli/templates/features/jobs/worker.rs.jinja crates/rbs-cli/templates/features/jobs/tests.rs.jinja crates/rbs-cli/tests/integration_jobs.rs
git commit -m "fix(jobs): rend à la file les jobs qu'un worker mort a laissés en running" -m "<pourquoi, en trois lignes>" -m "Vérifications :
- cargo check --tests sur un projet rendu avec jobs : Finished
- cargo test -p rbs-cli --test integration_jobs -- --ignored : <N> passés (<durée>)"
```

---

### Task 3: `examples/newsletter-queue` suit, et le guide dit le bail

**Files:**
- Modify: `examples/newsletter-queue/config/default.toml`, `src/modules/jobs/{config,queue,worker,tests}.rs`
- Modify: `docs/docs/guides/jobs.md:170-178` (`## Retries and definitive failure`) et FR `:170-180` (`## Réessai et échec définitif`)

- [ ] **Step 1: Régénérer par diff**

Lire `examples/README.md:93-124`. Régénérer `newsletter-queue` dans le scratchpad avec les commandes exactes du README (Cargo `--manifest-path` pointant sur le worktree), puis :

```bash
diff -ru /Users/yacoubakone/dev/rs/examples/newsletter-queue $S/newsletter-queue -x .git -x target > $S/newsletter.diff; grep '^diff' $S/newsletter.diff
```

Attendu : seuls `config/default.toml` et `src/modules/jobs/{config,queue,worker,tests}.rs` diffèrent (plus les éditions manuelles connues : `rbs-core` en `path = "../../crates/rbs-core"`, marqueurs `region`). N'appliquer que les hunks des cinq fichiers du fragment, en préservant les éditions manuelles s'ils en portent (`grep -n 'region' examples/newsletter-queue/src/modules/jobs/*.rs examples/newsletter-queue/config/default.toml`).

- [ ] **Step 2: L'oracle**

```bash
cargo test -p rbs-cli --test integration_examples 2>&1 | tail -5
(cd examples/newsletter-queue && cargo check --tests 2>&1 | tail -3)
```

Attendu : `integration_examples` vert (octet à octet), `cargo check` `Finished`.

- [ ] **Step 3: Le guide**

`docs/docs/guides/jobs.md`, après le paragraphe « The counter is incremented at reservation… » :

```markdown
A worker that dies between reserving a job and reporting on it leaves the row `running`.
Past `lease_secs`, the next worker tour gives it back to the queue: `pending` again, its
attempt spent. A job that legitimately runs longer than the lease is replayed — set the
lease above your longest job.
```

Idem FR, après « Le compteur est incrémenté à la réservation… ». Vérifier que l'extrait `file=examples/newsletter-queue/src/modules/jobs/config.rs` du guide (ligne ~71) n'a pas de `region=` qui exclurait le champ ; si un `region` borne l'extrait, l'étendre.

- [ ] **Step 4: Vérifier**

```bash
cargo test -p rbs-cli --test integration_docs 2>&1 | tail -3
cd docs && node scripts/parite.mjs 2>&1 | tail -5
```

Attendu : vert ; parité sans écart nouveau (`IMPROVE_OLD.md` préexiste).

- [ ] **Step 5: Commit**

```bash
git add examples/newsletter-queue docs/docs/guides/jobs.md docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/jobs.md
git commit -m "docs(jobs): documente le bail et régénère l'exemple newsletter-queue" -m "Vérifications :
- cargo test -p rbs-cli --test integration_examples : vert
- cargo check --tests dans examples/newsletter-queue : Finished
- cargo test -p rbs-cli --test integration_docs : vert
- node docs/scripts/parite.mjs : aucun écart nouveau"
```

---

### Task 4: Passe finale

- [ ] **Step 1:**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3 && cargo test -p rbs-cli --lib 2>&1 | tail -3
grep -rn 'poll_interval_secs' docs crates examples --include='*.md' --include='*.rs' --include='*.toml' --include='*.jinja' | grep -v lease_secs | grep -v 'scheduler.md'
```

Attendu : fmt muet, clippy sans warning, tests verts ; la seconde commande ne liste que des lignes où `lease_secs` figure à côté (bloc TOML) ou des mentions isolées légitimes — la lire et juger.

- [ ] **Step 2: Rapport** — branche, `git log --oneline main..HEAD`, chaque preuve avec la ligne exacte lue.
