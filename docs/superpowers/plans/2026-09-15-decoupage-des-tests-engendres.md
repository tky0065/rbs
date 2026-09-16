# Découpage des fichiers de tests engendrés — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal :** aucun fichier de tests rendu chez l'utilisateur — ni `jobs/queue.rs` — ne dépasse
~250 lignes ; les tests se rangent par préoccupation du code qu'ils éprouvent.

**Architecture :** chaque `tests.rs` trop long devient un répertoire `tests/` (`mod.rs` =
harnais partagé + `mod …;`, un fichier par préoccupation commençant par `use super::*;`),
déclaré fichier par fichier dans le `feature.toml` du fragment. `generate crud` rend
plusieurs templates `templates/feature/tests/*.rs.jinja` au lieu d'un. `jobs/queue.rs`
devient `queue/` avec des `pub use` qui gardent les chemins. Aucun corps de test ne change.

**Tech Stack :** Rust 2024, minijinja (délimiteurs alternatifs), Axum, SeaORM, cargo test,
testcontainers (suites Docker `crates/rbs-cli/tests/integration_*.rs`), Docusaurus.

**Spec :** `docs/superpowers/specs/2026-09-15-decoupage-des-tests-engendres-design.md`

## Global Constraints

- Un fichier rendu ≤ ~250 lignes, dans la configuration la plus chargée de ses options.
- **Aucun corps de test ne change** : on déplace, on n'édite que les `use` et les
  visibilités (`pub(super)`) qu'exige le déplacement. Aucun test ne se renomme.
- Hors périmètre : fichiers de 200 à 250 lignes, migration `create_auth_tables`.
- Les exemples se régénèrent **par diff entre deux générations** (commandes exactes :
  `examples/README.md`), jamais par écrasement : leurs régions `// region:` et le test
  403 écrit à la main dans `examples/blog-auth/src/posts/tests.rs` doivent survivre.
  `patch --no-backup-if-mismatch`, sinon un `.orig` fait échouer `integration_examples`.
- minijinja : `-%}` mange l'indentation ; seul `integration_examples` voit un blanc perdu.
- Documentation bilingue : toute page anglaise touchée l'est en français dans le même
  commit (`docs/docs/…` et `docs/i18n/fr/docusaurus-plugin-content-docs/current/…`).
- Commits : Conventional Commits, sujet en français à l'impératif, sans majuscule ni point
  final ; corps = pourquoi + `Vérifications :` avec les commandes et leurs résultats réels.
  Jamais d'identifiant de tâche, de renvoi à un fichier de suivi, ni de ligne
  `Co-Authored-By`/`Claude-Session`.
- Suites Docker : une par commande, `--no-fail-fast -- --include-ignored`, sortie
  redirigée vers le scratchpad (`/private/tmp/claude-501/-Users-yacoubakone-dev-rs/0b6e0e7f-013a-42af-a9d8-9bc0f246cf81/scratchpad`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` et
  `cargo fmt --all --check` sont bloquants ; sur un exemple : `cargo fmt --check` et
  `cargo clippy --all-targets -- -D warnings` depuis son répertoire.

## Outils de vérification communs

**Inventaire des tests d'un exemple** — la preuve qu'un déplacement n'a rien perdu. Le
segment qui suit `tests::` est retiré, pour que `auth::tests::session::x` et
`auth::tests::login::x` se comparent :

```bash
inventaire() { # $1 = répertoire d'exemple, $2 = fichier de sortie
  (cd "$1" && cargo test --lib -- --list --include-ignored 2>/dev/null) \
    | sed -n 's/: test$//p' \
    | sed -E 's/::tests::[a-z_]+::/::tests::/' \
    | sort > "$2"
}
```

Avant la tâche : `inventaire examples/<ex> $S/<ex>-avant.txt` ; après :
`inventaire examples/<ex> $S/<ex>-apres.txt` ; `diff` des deux → **vide**, et
`wc -l` identique.

**Tailles** — aucune sortie attendue hors exceptions du périmètre :

```bash
find examples/*/src -name '*.rs' -not -path '*/target/*' \
  \( -path '*/tests/*' -o -name tests.rs -o -path '*/jobs/queue*' \) \
  -exec wc -l {} + | awk '$1>250 && $2!="total"'
```

**Transcriptions de la doc** : `cargo test -p rbs-cli --test integration_docs` (les
transcriptions marquées rejouent `rbs add`/`rbs generate` et comparent la liste des
fichiers et le bilan « N créés » à la page).

---

### Task 1 : `storage` — `tests.rs` → `tests/`

**Files :**
- Delete : `crates/rbs-cli/templates/features/storage/tests.rs.jinja`
- Create : `crates/rbs-cli/templates/features/storage/tests/mod.rs.jinja` (imports, `root`,
  `read`, `round`, `files_under`, `mod files; mod s3;`), `tests/files.rs.jinja` (tests du
  backend fichiers : `the_file_backend_puts_gets_attests_then_deletes` →
  `the_probe_reports_a_root_that_vanished`), `tests/s3.rs.jinja` (`s3_config` et les
  quatre tests S3)
- Modify : `crates/rbs-cli/templates/features/storage/feature.toml` (une entrée
  `source`/`destination` par fichier, même forme que les entrées existantes, destination
  `src/modules/storage/tests/<f>.rs`)
- Modify : `examples/file-drop/src/modules/storage/tests.rs` → `tests/{mod,files,s3}.rs`
- Modify : transcriptions `docs/docs/guides/storage.md:29`, `docs/docs/tutorials/storage.md:34`
  et leurs jumelles françaises (ligne du fichier + compte du bilan)

**Interfaces :** Produces le patron repris par les tâches 2 à 6 : `mod.rs` porte les aides
en `pub(super)` si un sous-module les appelle, et `mod <f>;` ; chaque sous-module ouvre
par `use super::*;` puis ses propres `use`.

- [ ] **Step 1 :** `inventaire examples/file-drop $S/file-drop-avant.txt`.
- [ ] **Step 2 :** créer les trois templates en déplaçant le code tel quel ; supprimer
  `tests.rs.jinja` ; déclarer les trois fichiers dans `feature.toml`.
- [ ] **Step 3 :** reproduire le même découpage dans `examples/file-drop` (même contenu que
  les templates — ce fragment n'a pas de balise Jinja qui diffère à ce rendu ; le vérifier
  par `diff` de chaque paire).
- [ ] **Step 4 :** `cargo test -p rbs-cli --test integration_examples` → 22/0 ;
  `cargo test -p rbs-cli --lib` → 0 échec (dont la déclaration des fichiers du fragment).
- [ ] **Step 5 :** inventaire après, `diff` vide ; tailles sans sortie ; fmt et clippy sur
  `examples/file-drop`.
- [ ] **Step 6 :** transcriptions à jour ; `cargo test -p rbs-cli --test integration_docs`
  → 0 échec ; `npm run build` sous `docs/` → exit 0.
- [ ] **Step 7 :** `cargo test -p rbs-cli --test integration_storage --no-fail-fast -- --include-ignored` → 0 échec.
- [ ] **Step 8 :** commit `refactor(storage): range les tests du fragment par backend`.

### Task 2 : `scheduler` — `tests.rs` → `tests/`

**Files :**
- Delete : `crates/rbs-cli/templates/features/scheduler/tests.rs.jinja`
- Create : `tests/mod.rs.jinja` (imports, `Scheduled`, `jobs_declenches`, `table_a_soi`,
  `calendrier`, `echeance_due`, `mod expression; mod sync; mod ticker;`),
  `tests/expression.rs.jinja` (les trois tests de `normaliser`), `tests/sync.rs.jinja`
  (`a_newly_declared_schedule_is_inserted_with_its_next_occurrence` →
  `an_unparsable_expression_stops_the_reconciliation`), `tests/ticker.rs.jinja`
  (`a_due_schedule_is_not_moved_by_a_changed_expression` →
  `the_ticker_stops_when_shutdown_is_requested`)
- Modify : `crates/rbs-cli/templates/features/scheduler/feature.toml`
- Modify : `crates/rbs-cli/src/cron.rs:181` — `fragment("tests.rs.jinja")` lit désormais
  `fragment("tests/expression.rs.jinja")` : c'est là que vivent les appels
  `normaliser("…")` que le test compare au CLI. Vérifier que `fragment` accepte un
  sous-chemin ; sinon l'adapter.
- Modify : `examples/event-hub/src/modules/scheduler/tests.rs` → `tests/…`
- Modify : transcription `docs/docs/guides/scheduler.md:50` et sa jumelle française

- [ ] **Step 1 :** inventaire `examples/event-hub` avant.
- [ ] **Step 2 :** découper les templates, déclarer dans `feature.toml`, repointer
  `cron.rs`.
- [ ] **Step 3 :** `cargo test -p rbs-cli --lib cron::` → le test
  `the_expressions_the_fragment_tests_judge_are_judged_alike` passe et juge au moins trois
  expressions (lire son compteur `jugees` : il ne doit pas tomber à zéro).
- [ ] **Step 4 :** même découpage dans `examples/event-hub` ; `integration_examples` 22/0.
- [ ] **Step 5 :** inventaire après, `diff` vide ; tailles ; fmt/clippy `event-hub`.
- [ ] **Step 6 :** transcriptions, `integration_docs`, `npm run build`.
- [ ] **Step 7 :** `integration_scheduler` sous Docker → 0 échec.
- [ ] **Step 8 :** commit `refactor(scheduler): range les tests du fragment par mécanisme`.

### Task 3 : `jobs` — `tests.rs` → `tests/`

**Files :**
- Delete : `crates/rbs-cli/templates/features/jobs/tests.rs.jinja`
- Create : `tests/mod.rs.jinja` (imports, `Succeeds`, `AlwaysFails`, `Slow`, `Meet`,
  `rendez_vous`, `registry`, `detached_state`, `verrou_base` **en `pub(crate)`**,
  `table_a_soi`, `config`, `delai`, `reserved_since`, `attendre_le_statut`,
  `mod retry; mod queue; mod lease; mod worker;`), `tests/retry.rs.jinja` (les trois
  tests de délai et les deux tests de statut/payload, plus
  `an_unregistered_kind_is_reported_rather_than_silently_dropped`), `tests/queue.rs.jinja`
  (`a_job_enqueued_in_a_rolled_back_transaction_does_not_exist` →
  `two_concurrent_workers_never_reserve_the_same_job`), `tests/lease.rs.jinja`
  (`a_failing_job_is_retried_then_marked_failed_after_the_last_attempt` →
  `a_job_abandoned_on_its_last_attempt_is_failed_rather_than_requeued`),
  `tests/worker.rs.jinja` (les deux tests du worker)
- Modify : `crates/rbs-cli/templates/features/jobs/feature.toml`
- Modify : `examples/newsletter-queue` et `examples/event-hub`,
  `src/modules/jobs/tests.rs` → `tests/…`
- Modify : transcriptions `guides/jobs.md:32`, `guides/scheduler.md:37`,
  `tutorials/jobs.md:36` et jumelles

**Interfaces :** Consumes/Produces `crate::modules::jobs::tests::verrou_base()` — appelé par
les tests de `scheduler` et de `webhooks` ; le chemin ne change pas (`pub(crate) mod tests;`
dans `jobs/mod.rs`, `pub(crate) fn verrou_base` dans `tests/mod.rs`).

- [ ] **Step 1 :** inventaires avant (`newsletter-queue`, `event-hub`).
- [ ] **Step 2 :** découper, déclarer ; nommer le sous-module `queue` ne masque pas
  `super::queue` du fragment : dans `tests/queue.rs`, `use super::*;` importe le module
  `queue` du harnais — vérifier la résolution par la compilation, renommer le fichier en
  `reservation.rs` si elle échoue.
- [ ] **Step 3 :** même découpage dans les deux exemples ; `integration_examples` 22/0.
- [ ] **Step 4 :** inventaires après, `diff` vides ; tailles ; fmt/clippy des deux exemples.
- [ ] **Step 5 :** transcriptions, `integration_docs`, `npm run build`.
- [ ] **Step 6 :** `integration_jobs` puis `integration_scheduler` sous Docker → 0 échec.
- [ ] **Step 7 :** commit `refactor(jobs): range les tests du fragment par étape de la file`.

### Task 4 : `jobs/queue.rs` → `queue/`

**Files :**
- Delete : `crates/rbs-cli/templates/features/jobs/queue.rs.jinja`
- Create : `queue/mod.rs.jinja` (`enqueue`, `enqueue_at`, `a_la_seconde`, `mod reserve;
  mod outcome;`, `pub use reserve::reserver_prochain_job;`,
  `pub use outcome::{Reprise, mark_done, requeue_stale, retry_or_fail};`,
  `pub(super) use outcome::retry_delay;`), `queue/reserve.rs.jinja` (`COLONNES`, les
  quatre requêtes `RESERVATION_*`/`ELECTION_MYSQL`, `reserver_prochain_job`,
  `reserver_en_un_coup`, `reserver_en_deux_temps`), `queue/outcome.rs.jinja` (`Reprise`,
  `ABANDON`, `requeue_stale`, `mark_done`, `retry_delay`, `retry_or_fail`)
- Modify : `crates/rbs-cli/templates/features/jobs/feature.toml` (trois destinations
  `src/modules/jobs/queue/<f>.rs`)
- Modify : les deux exemples, `src/modules/jobs/queue.rs` → `queue/…`
- Vérifier : `grep -rn 'queue\.rs' crates/rbs-cli/src crates/rbs-cli/tests docs/docs docs/i18n examples/README.md`
  — chaque occurrence suit (transcriptions `+ src/modules/jobs/queue.rs créé`, citations
  `file=… region=…`)

**Interfaces :** chemins conservés : `jobs::enqueue`, `jobs::enqueue_at` (réexport de
`jobs/mod.rs`), `queue::reserver_prochain_job`, `queue::requeue_stale`,
`queue::mark_done`, `queue::retry_or_fail`, `queue::retry_delay` (vu des tests de jobs).
Un `generate job` ou un fragment qui écrit `jobs::queue::…` doit compiler inchangé.

- [ ] **Step 1 :** inventaires avant des deux exemples.
- [ ] **Step 2 :** découper, déclarer ; aucun appelant ne change de ligne.
- [ ] **Step 3 :** exemples ; `integration_examples` 22/0 ; tailles.
- [ ] **Step 4 :** `grep` ci-dessus traité ; `integration_docs` ; `npm run build`.
- [ ] **Step 5 :** `integration_jobs`, `integration_scheduler`, `integration_webhooks` →
  0 échec ; inventaires après, `diff` vides.
- [ ] **Step 6 :** commit `refactor(jobs): scinde la file en dépôt, réservation et issue`.

### Task 5 : `webhooks` — `tests.rs` → `tests/`

**Files :**
- Delete : `crates/rbs-cli/templates/features/webhooks/tests.rs.jinja`
- Create : `tests/mod.rs.jinja` (imports communs, `table_a_soi`, `table_a_soi_stricte`,
  `abonne`, `abonne_revoque`, `inserer`, `donnees`, `token`, `request`, `call`,
  `livraisons`, `receveur_local`, `mod …;`), `tests/signature.rs.jinja` (`VECTEUR` et les
  six tests de signature et de motifs), `tests/emission.rs.jinja` (les cinq tests
  `emitting_…` → `an_emission_rolled_back_…`), `tests/routes.rs.jinja` (les quatre tests de
  routes `a_user_role_…` → `revoking_twice_keeps_the_first_date`), `tests/target.rs.jinja`
  (la section « Cibles » sans base : `private_loopback_…` →
  `a_connection_failure_is_not_a_refusal`, avec `client_filtre`),
  `tests/blocked.rs.jinja` (`an_admin_subscribing_a_private_url_gets_400` →
  `in_development_a_local_receiver_is_reached`)
- Modify : `crates/rbs-cli/templates/features/webhooks/feature.toml`
- Modify : `examples/event-hub/src/modules/webhooks/tests.rs` → `tests/…`
- Modify : transcription `guides/webhooks.md:48` et jumelle

- [ ] **Step 1 :** inventaire avant `event-hub`.
- [ ] **Step 2 :** découper, déclarer ; l'intertitre `// ── Cibles ──` disparaît avec la
  section qu'il séparait.
- [ ] **Step 3 :** exemple ; `integration_examples` ; tailles ; fmt/clippy.
- [ ] **Step 4 :** transcriptions, `integration_docs`, `npm run build`.
- [ ] **Step 5 :** `integration_webhooks` → 0 échec ; inventaire après, `diff` vide.
- [ ] **Step 6 :** commit `refactor(webhooks): range les tests du fragment par préoccupation`.

### Task 6 : `auth` — `session`, `password`, `verification`

**Files :**
- Delete : `crates/rbs-cli/templates/features/auth/tests/session.rs.jinja`,
  `tests/password.rs.jinja`
- Create sous `crates/rbs-cli/templates/features/auth/tests/` :
  - `registration.rs.jinja` : `registration_returns_202_without_a_body` →
    `an_address_taken_in_another_case_is_the_same_account` (inscription, casse, temps)
    sauf les tests de connexion ;
  - `login.rs.jinja` : `me_without_a_token_returns_401`,
    `me_with_an_unreadable_token_returns_401`,
    `an_unverified_address_logs_in_only_after_verification`,
    `register_then_login_answers_the_same_for_a_free_and_a_taken_address`,
    `without_the_rule_an_unverified_address_logs_in`,
    `login_ignores_the_case_of_the_address`,
    `a_wrong_password_and_an_unknown_email_return_the_same_401`,
    `an_unknown_email_costs_the_same_time_as_a_wrong_password`,
    `me_returns_the_callers_profile` ;
  - `refresh.rs.jinja` : `a_valid_refresh_returns_a_new_pair` →
    `the_other_sessions_of_the_same_account_stay_valid` (rotation, rejeu, expiration,
    logout) — s'il dépasse ~250 lignes, les tests de logout passent dans `logout.rs.jinja` ;
  - `sessions.rs.jinja` : les quatre tests de liste et de révocation nommée/totale ;
  - `roles.rs.jinja` : `admin_only_route`, `user_or_above_route`, `with_token`,
    `login_as_admin` et les cinq tests de rôle ;
  - `openapi.rs.jinja` : `openapi_document` et les trois tests OpenAPI ;
  - `tokens.rs.jinja` : les quatre tests du dépôt de jetons à usage unique
    (`a_token_is_consumed_once_even_under_concurrency` →
    `a_rolled_back_transaction_leaves_the_token_and_the_password_untouched`, avec
    `reset_token_expiring_in`) ;
  - `change.rs.jinja` : les trois tests de `change-password` ;
  - `reset.rs.jinja` : `a_reset_token_sets_a_new_password_and_closes_every_session` →
    `forgetting_a_registered_address_is_accepted_and_opens_a_token` ;
  - `guard.rs.jinja` : `the_verified_guard_opens_only_after_verification`,
    `identity_leaves_the_account_it_read_for_the_verified_guard` (retirés de
    `verification.rs.jinja`).
- Modify : `tests/mod.rs.jinja` — reçoit les aides partagées par plusieurs fichiers
  (`login_as`, `refresh_for`, `refresh`, `session_row`, `logout`, `access_for` selon leurs
  appelants) et les `mod …;`. S'il dépasse ~250 lignes, les constructeurs de requêtes
  (`call`, `without_body`, `post_json`, `post_json_authenticated`, `get_authenticated`,
  `delete_authenticated`) passent dans `tests/http.rs.jinja`, que `mod.rs` importe par
  `mod http; use http::*;` (les sous-modules les reçoivent par leur `use super::*;`).
- Modify : `crates/rbs-cli/templates/features/auth/feature.toml`
- Modify : `examples/blog-auth` et `examples/event-hub`, `src/auth/tests/…` ; la région
  `jeton_admin` de `blog-auth` suit `login_as_admin` dans `roles.rs`
- Modify : `docs/docs/guides/auth.md:452` (`file=…/tests/session.rs region=jeton_admin` →
  `…/tests/roles.rs`) ; transcriptions `cli/add.md:267-270`, `guides/auth.md:46-49`,
  `tutorials/auth.md:73-76` et jumelles
- Modify : `crates/rbs-cli/tests/integration_auth.rs:244-257` (liste des fichiers posés)

- [ ] **Step 1 :** inventaires avant (`blog-auth`, `event-hub`).
- [ ] **Step 2 :** découper les templates, déclarer ; aides partagées dans `mod.rs`.
- [ ] **Step 3 :** exemples ; `integration_examples` ; tailles (y compris `mod.rs`).
- [ ] **Step 4 :** tests d'auth de `blog-auth` contre PostgreSQL :
  `RBS_DATABASE__URL=postgres://rbs:rbs@localhost:55476/blog_auth RBS_AUTH__SECRET=$(openssl rand -hex 32) cargo test --lib auth:: -- --include-ignored`
  depuis `examples/blog-auth` → 65 passés, 0 échoué (le conteneur `rbs-pg-76` tourne ;
  sinon voir la mémoire des services de test).
- [ ] **Step 5 :** doc (citation, transcriptions), `integration_docs`, `npm run build`,
  `node docs/scripts/parite.mjs` → 0 écart.
- [ ] **Step 6 :** `integration_auth` → 16/0 ; inventaires après, `diff` vides.
- [ ] **Step 7 :** commit `refactor(auth): range les tests du fragment par route`.

### Task 7 : CRUD engendré — `tests.rs` → `tests/`

**Files :**
- Delete : `crates/rbs-cli/templates/feature/tests.rs.jinja`
- Create sous `crates/rbs-cli/templates/feature/tests/` :
  - `mod.rs.jinja` : les `use`, `application`, `call`, le bloc `auth` (`JETON`, `token`,
    `bearer`), `request`, `without_body`, `compare`, `filled`, `unique_number`,
    `creation`, `modification` (lignes 1-213 de l'ancien template, conditions comprises),
    puis les `mod …;` — chacun sous la même condition que le fichier qu'il déclare ;
  - `lifecycle.rs.jinja` : `the_full_lifecycle_goes_through_the_api`,
    `the_list_travels_compressed_when_the_client_accepts_it`,
    `two_creations_in_a_row_carry_increasing_ids`, et sous `cursor`
    `the_cursor_walks_every_page_without_duplicates` (rendu si `creatable`) ;
  - `errors.rs.jinja` : `an_invalid_email_returns_422` (sous `email_field`),
    `an_unknown_sort_column_returns_400`, `a_replayed_unique_value_returns_409` (sous
    `unique_field`), `an_unknown_id_returns_404`, `an_unreadable_body_returns_400` —
    toujours rendu ;
  - `filter.rs.jinja` : `the_filter_narrows_the_list`,
    `contains_reads_percent_and_underscore_literally` — rendu sous `filterable` ;
  - `access.rs.jinja` : `an_anonymous_request_returns_401`,
    `an_anonymous_read_returns_401` — rendu sous `auth` ;
  - `content.rs.jinja` : `binary`, `call_raw` et les scénarios de contenu — rendu sous
    `with_upload`.
  Relire l'ancien template ligne à ligne pour affecter chaque scénario et chaque garde
  Jinja ; un scénario qu'aucune condition ne rend plus est une faute.
- Modify : `crates/rbs-cli/src/generate/tests_http.rs` — `render(feature)` rend
  `Result<Vec<(&'static str, String)>, minijinja::Error>` : `("tests/mod.rs", …)` puis
  chaque fichier dont la condition tient ; le contexte minijinja actuel est partagé par
  tous les rendus et reçoit les booléens de présence (`with_lifecycle`, `with_filter`,
  `with_access`, `with_content`) que `mod.rs.jinja` lit pour ses `mod …;`. Les tests
  unitaires du module (`rendered.contains(…)`, fixtures figées
  `the_guarded_trials_render_the_frozen_fixture`,
  `the_cursor_trials_render_the_frozen_fixture`, rustfmt) portent sur la concaténation
  des fichiers rendus, ou sur le fichier visé ; les fixtures figées se scindent comme les
  templates.
- Modify : `crates/rbs-cli/src/generate/command.rs:636-638` — un `files.push` par fichier
  rendu ; `command.rs:1110` (liste attendue) nomme `tests/mod.rs` et `tests/errors.rs`.
- Modify : `crates/rbs-cli/tests/integration_examples.rs:78` (`edite_a_la_main`) et
  `:714-721` — le fichier où atterrit le test 403 écrit à la main de
  `examples/blog-auth/src/posts/` (dans `access.rs`) ; `integration_crud.rs:652`
  (commentaire).
- Modify : les cinq CRUD d'exemple (`hello-crud/src/articles`, `blog-auth/src/posts`,
  `file-drop/src/uploads`, `newsletter-queue/src/subscribers`, `event-hub/src/orders`),
  régénérés par diff ; les régions (`harnais`, `cycle_de_vie`, `erreur_404`,
  `corps_illisible`, `jeton`, `refus`) suivent leur code.
- Modify : citations `guides/errors.md:153,158`, `guides/testing.md:17,30`,
  `guides/auth.md:462,470`, `tutorials/auth.md:375` (chemins `…/tests/<f>.rs`) ;
  transcriptions `getting-started.md:242`, `cli/generate.md:352,380`,
  `tutorials/first-resource.md:38`, `tutorials/storage.md:76`, `tutorials/auth.md:130`
  et toutes leurs jumelles françaises ; toute page qui décrit « `tests.rs` » du CRUD
  (`grep -rn 'tests\.rs' docs/docs docs/i18n`).

- [ ] **Step 1 :** inventaires avant des cinq exemples.
- [ ] **Step 2 :** écrire d'abord le test unitaire rouge dans `tests_http.rs` :
  `a_complete_crud_renders_its_tests_as_a_directory` — pour une feature avec champ
  filtrable, `auth` et `--with-upload`, `render` rend exactement
  `["tests/mod.rs", "tests/lifecycle.rs", "tests/errors.rs", "tests/filter.rs",
  "tests/access.rs", "tests/content.rs"]`, et sans ces options
  `["tests/mod.rs", "tests/lifecycle.rs", "tests/errors.rs"]` ;
  `cargo test -p rbs-cli --lib generate::tests_http` → échec de compilation (type rendu).
- [ ] **Step 3 :** templates, `render`, `command.rs` ; le test passe ; toute la suite
  `--lib` → 0 échec, les tests rustfmt et fixtures compris.
- [ ] **Step 4 :** régénérer les cinq exemples par diff ; `integration_examples` 22/0 ;
  tailles ; fmt/clippy des cinq.
- [ ] **Step 5 :** doc (citations, transcriptions, descriptions), `integration_docs`,
  `npm run build`, `parite.mjs`.
- [ ] **Step 6 :** `integration_crud`, `integration_cursor`, `integration_storage`,
  `integration_auth` sous Docker, un par commande → 0 échec ; inventaires après, `diff`
  vides.
- [ ] **Step 7 :** commit `refactor(generate): range les tests du CRUD engendré par préoccupation`.

### Task 8 : notes de version et passe finale

**Files :**
- Modify : `CHANGELOG.md` et `CHANGELOG.fr.md`, section 1.5.0 « Changed »/« Modifié » :
  les tests que posent `add auth`, `add jobs`, `add scheduler`, `add storage`,
  `add webhooks` et `generate crud` vivent dans des répertoires `tests/`, un fichier par
  préoccupation ; `jobs/queue.rs` devient `jobs/queue/` ; un projet déjà engendré garde
  ses fichiers.
- Modify : `crates/rbs-cli/notes/1.5.0.md` — une section courte : rien à faire pour un
  projet existant ; un CRUD engendré désormais porte `tests/`.

- [ ] **Step 1 :** écrire les trois entrées.
- [ ] **Step 2 :** passe finale : fmt, clippy workspace, `cargo test --workspace`,
  `cargo test -p rbs-core --all-features`, `--lib`, `integration_examples`,
  `integration_docs`, `npm run build`, parité, puis chaque suite Docker touchée une par
  commande.
- [ ] **Step 3 :** commit `docs: décrit la disposition des tests engendrés`.
