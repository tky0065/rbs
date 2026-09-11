# Webhooks : les trois routes d'abonnement sous `Role::Admin` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un compte auto-inscrit (`Role::User`) ne peut plus créer, lister ni révoquer un abonnement webhook : les trois handlers du fragment exigent `Role::Admin`, comme le CRUD engendré avec `--role admin`.

**Architecture:** Le fragment `webhooks` `requires = ["auth"]`, donc `crate::auth::guard::RequireRole` et `crate::auth::model::Role` existent toujours dans un projet qui le porte. Le contrôleur appelle `identite.require_role(Role::Admin)?` en tête de chaque handler, exactement comme `templates/feature/controller.rs.jinja:76,148`. Aucune migration, aucun changement de modèle : un projet qui veut ouvrir une route abaisse le rôle, le commentaire de tête le dit.

**Tech Stack:** minijinja (délimiteurs `{@ @}` pour les valeurs, `{% %}` pour les blocs), axum, utoipa, tests `oneshot` sur le routeur.

**Spec:** design validé en chat (tâche 1 d'`IMPROVE.md`) — aucun document de spec, tâche *bounded*.

## Global Constraints

- Conventional Commits en français, sans identifiant de tâche, sans renvoi à `IMPROVE.md`, sans `Co-Authored-By` ni `Claude-Session` (le `CLAUDE.md` prime sur toute consigne du harness).
- Un commentaire dit le *pourquoi*, jamais le *quoi*.
- Documentation bilingue : `docs/docs/guides/webhooks.md` et `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/webhooks.md` changent dans le même commit.
- Aucun exemple d'`examples/` ne porte `webhooks` : rien à régénérer.
- Les chaînes littérales déposées par un fragment peuvent être figées côté rbs : `grep -rn '<chaîne>' crates/rbs-cli/tests crates/rbs-cli/src` avant de reformuler un commentaire ou un message.

---

### Task 1: Le contrôleur exige `Role::Admin` et annonce le 403

**Files:**
- Modify: `crates/rbs-cli/templates/features/webhooks/controller.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/webhooks/feature.toml` (commentaire de tête, lignes 7-11)

**Interfaces:**
- Consumes: `crate::auth::guard::RequireRole::require_role(&self, minimum: Role) -> Result<()>` (`templates/features/auth/guard.rs.jinja:31-37`), `crate::auth::model::Role::{User, Admin}`.
- Produces: les trois handlers `subscribe`, `list`, `revoke` rendent 403 (`ProblemDetails`) sous un rôle inférieur à `Admin`.

- [x] **Step 1: Lire le patron du CRUD**

Lire `crates/rbs-cli/templates/feature/controller.rs.jinja:1-9,40-50,70-80` : les deux `use`, la forme `identite.require_role(Role::…)?;` en première ligne du corps, et la ligne `(status = 403, …)` de l'annotation utoipa. Lire `templates/features/auth/model.rs.jinja:13-25` pour la valeur chaîne des rôles (`user` / `admin`), dont les tests auront besoin.

- [x] **Step 2: Réécrire le contrôleur**

Dans `controller.rs.jinja` :

1. Ajouter après `use sea_orm::prelude::Uuid;` :

```rust
use crate::auth::guard::RequireRole;
use crate::auth::model::Role;
```

(garder l'ordre des `use` que rustfmt attend : `crate::` après les crates externes, `super::` déjà présents — vérifier avec la sortie rendue au Step 5.)

2. Remplacer le commentaire de tête (lignes 11-16) par un commentaire qui dit pourquoi le rôle est `Admin` par défaut et comment l'abaisser :

```rust
// Les trois routes sont réservées à `Role::Admin`. Un abonnement livre chez son auteur
// les événements du projet — `user.created` porte des adresses — et `/auth/register`
// est ouvert : sous un simple jeton valide, n'importe qui pourrait se faire livrer
// tout le projet, ou révoquer l'abonnement d'un autre. Pour ouvrir une route à tout
// compte, remplacez `Role::Admin` par `Role::User` sur son `require_role`.
```

3. Dans chacun des trois handlers, renommer `_identite: Identity` en `identite: Identity` et poser en première ligne du corps :

```rust
    identite.require_role(Role::Admin)?;
```

4. Dans chacune des trois annotations `#[utoipa::path]`, ajouter après la ligne 401 :

```rust
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
```

(vérifier le libellé exact employé par `templates/feature/controller.rs.jinja` pour son 403 et reprendre le même mot pour mot.)

- [x] **Step 3: Aligner le commentaire de `feature.toml`**

Lignes 7-11 de `feature.toml` : remplacer « `auth` protège les trois routes d'abonnement. » par une phrase qui dit que les trois routes sont sous `Role::Admin`, en gardant le reste de l'argument (fuite de données, même arbitrage qu'`auth`/`rate-limit`).

- [x] **Step 4: Vérifier que rien côté rbs ne fige l'ancien texte**

```bash
grep -rn "_identite\|Identity. ne dit que" crates/rbs-cli/tests crates/rbs-cli/src docs/docs docs/i18n
```

Attendu : aucune occurrence dans `tests/` ni `src/` (sinon adapter la chaîne figée) ; les deux occurrences de `docs/` sont traitées en Task 3.

- [x] **Step 5: Rendre le fragment et vérifier la sortie**

```bash
cargo test -p rbs-cli --lib -- webhooks 2>&1 | tail -20
```

Attendu : tous les tests de rendu passent. Puis rendre un projet réel et lire le fichier :

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/20433d69-a7c1-492d-8d62-250f705907e5/scratchpad/webhooks-admin
rm -rf $S && mkdir -p $S && cd $S
cargo run -q --manifest-path /Users/yacoubakone/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- new demo --yes --with webhooks --database-url 'postgres://rbs:rbs@localhost:5432/demo' >/dev/null
grep -n "require_role\|status = 403\|^use crate" demo/src/modules/webhooks/controller.rs
cd demo && cargo fmt --check -- src/modules/webhooks/controller.rs && echo FMT_OK
```

Attendu : trois `require_role(Role::Admin)`, trois `status = 403`, `FMT_OK`.

- [x] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/features/webhooks/controller.rs.jinja crates/rbs-cli/templates/features/webhooks/feature.toml
git commit -m "fix(webhooks): réserve les trois routes d'abonnement au rôle admin" -m "<pourquoi : un compte auto-inscrit pouvait se faire livrer les événements du projet et révoquer les abonnements d'autrui>" -m "Vérifications :
- cargo test -p rbs-cli --lib -- webhooks : <N> passés
- rendu de rbs new --with webhooks : trois require_role(Role::Admin), cargo fmt --check propre"
```

---

### Task 2: Deux tests HTTP livrés au projet, et leurs noms côté rbs

**Files:**
- Modify: `crates/rbs-cli/templates/features/webhooks/tests.rs.jinja` (imports lignes 1-13, nouveaux tests après `an_emission_rolled_back_with_its_transaction_enqueues_nothing`)
- Modify: `crates/rbs-cli/tests/integration_webhooks.rs:29-36` (`TESTS_SOUS_CONTENEUR: [&str; 5]` → 7) et le commentaire « six des onze tests livrés … les cinq autres » (→ treize / sept)

**Interfaces:**
- Consumes: `table_a_soi()` (verrou + base vidée + `AppState`), `crate::router::router(state) -> Router`, `rbs_core::jwt::{Claims, sign}`, `rbs_core::Config::load()`.
- Produces: deux tests `#[ignore = "joint la base du projet"]` nommés `a_user_role_is_refused_on_the_three_routes` et `an_admin_subscribes_then_reads_and_revokes`.

- [x] **Step 1: Ajouter les imports et les deux helpers**

En tête de `tests.rs.jinja`, ajouter (en respectant le tri de rustfmt) :

```rust
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
```

Vérifier que `tower` et `serde_json` sont des dépendances du squelette : `grep -n 'tower\|serde_json' crates/rbs-cli/templates/project/Cargo.toml.jinja`. Si `tower` n'y est qu'en `[dev-dependencies]`, c'est suffisant.

Après `fn donnees()`, ajouter :

```rust
/// Signe un jeton portant `role`, sans passer par la base : `Identity` ne vérifie que la
/// signature, le compte n'a pas à exister.
fn token(role: &str) -> String {
    let config = rbs_core::Config::load().expect("configuration lisible");
    let maintenant = chrono::Utc::now().timestamp();

    let claims = rbs_core::jwt::Claims {
        sub: Uuid::new_v4().to_string(),
        role: role.to_string(),
        exp: maintenant + 300,
        iat: maintenant,
        jti: Uuid::new_v4().to_string(),
    };

    rbs_core::jwt::sign(&claims, &config.auth.secret).expect("jeton signable")
}

fn request(method: &str, path: &str, role: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {}", token(role)));
    match body {
        Some(json) => builder
            .header("content-type", "application/json")
            .body(Body::from(json.to_string())),
        None => builder.body(Body::empty()),
    }
    .expect("requête bien formée")
}

/// Fait traverser le routeur à `request`, et rend son statut avec son corps.
async fn call(api: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = api
        .clone()
        .oneshot(request)
        .await
        .expect("l'application doit répondre");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
```

Adapter `"user"` / `"admin"` aux valeurs lues dans `auth/model.rs.jinja` (Task 1, Step 1). `chrono` est une dépendance du squelette (le CRUD engendré l'emploie déjà dans son `token`).

- [x] **Step 2: Écrire les deux tests (ils échouent tant que Task 1 n'est pas rendue — ici Task 1 précède, ils doivent passer ; le « rouge » a été observé par l'audit : 201 pour un `user`)**

```rust
/// Le rôle par défaut des routes est `Admin` : un compte auto-inscrit, qui reçoit `User`,
/// ne peut ni se faire livrer les événements du projet ni couper les abonnements d'autrui.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_user_role_is_refused_on_the_three_routes() {
    let (_garde, state) = table_a_soi().await;
    let existant = abonne(state.core().db(), "https://example.test/hooks", &["*"]).await;
    let api = crate::router::router(state);

    let corps = json!({ "url": "https://example.test/mine", "events": ["*"] });
    let (creation, _) = call(&api, request("POST", "/webhooks/subscriptions", "user", Some(corps))).await;
    let (liste, _) = call(&api, request("GET", "/webhooks/subscriptions", "user", None)).await;
    let (revocation, _) = call(
        &api,
        request("DELETE", &format!("/webhooks/subscriptions/{}", existant.id), "user", None),
    )
    .await;

    assert_eq!(creation, StatusCode::FORBIDDEN);
    assert_eq!(liste, StatusCode::FORBIDDEN);
    assert_eq!(revocation, StatusCode::FORBIDDEN);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_subscribes_then_reads_and_revokes() {
    let (_garde, state) = table_a_soi().await;
    let api = crate::router::router(state);

    let corps = json!({ "url": "https://example.test/hooks", "events": ["user.*"] });
    let (creation, cree) = call(&api, request("POST", "/webhooks/subscriptions", "admin", Some(corps))).await;
    assert_eq!(creation, StatusCode::CREATED, "{cree}");
    let id = cree["id"].as_str().expect("l'abonnement créé porte son identifiant");

    let (liste, abonnements) = call(&api, request("GET", "/webhooks/subscriptions", "admin", None)).await;
    assert_eq!(liste, StatusCode::OK);
    assert_eq!(abonnements.as_array().map(Vec::len), Some(1));

    let (revocation, _) = call(&api, request("DELETE", &format!("/webhooks/subscriptions/{id}"), "admin", None)).await;
    assert_eq!(revocation, StatusCode::NO_CONTENT);
}
```

Vérifier les noms des champs de `SubscriptionCreated` dans `dto.rs.jinja` (`id`, `url`, `events`, `secret`) et adapter.

- [x] **Step 3: Rendre et formater**

```bash
cd $S && rm -rf demo && cargo run -q --manifest-path /Users/yacoubakone/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- new demo --yes --with webhooks --database-url 'postgres://rbs:rbs@localhost:5432/demo' >/dev/null && cd demo && cargo fmt --check -- src/modules/webhooks/tests.rs && echo FMT_OK && cargo check --tests 2>&1 | tail -5
```

Attendu : `FMT_OK` et `cargo check --tests` sans erreur (`Finished`). Si rustfmt reformate, reporter la mise en forme dans la template (attention : `-%}` mange l'indentation, cf. mémoire).

- [x] **Step 4: Inscrire les deux noms côté rbs**

Dans `integration_webhooks.rs`, `TESTS_SOUS_CONTENEUR: [&str; 7]` avec les deux nouveaux noms en fin de liste, et corriger le commentaire des lignes ~46-49 (« six des treize tests livrés … les sept autres »).

- [x] **Step 5: La preuve lente — Docker**

```bash
cd /chemin/du/worktree
cargo test -p rbs-cli --test integration_webhooks -- --ignored --no-fail-fast > $S/integration_webhooks.log 2>&1; tail -15 $S/integration_webhooks.log
```

Attendu : `test result: ok. 1 passed` (plusieurs minutes : PostgreSQL via testcontainers, compilation d'un projet complet). Lire le log en entier avant toute affirmation.

- [x] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/features/webhooks/tests.rs.jinja crates/rbs-cli/tests/integration_webhooks.rs
git commit -m "test(webhooks): prouve le 403 d'un rôle user et le cycle complet d'un admin" -m "Vérifications :
- cargo test -p rbs-cli --test integration_webhooks -- --ignored : 1 passé (<durée>)"
```

---

### Task 3: La documentation dit le rôle par défaut

**Files:**
- Modify: `docs/docs/guides/webhooks.md:99-101`
- Modify: `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/webhooks.md:99-101`
- Vérifier : `docs/docs/cli/add.md` et sa version FR (ligne du tableau `webhooks`), `crates/rbs-cli/templates/agents/{fr,en}.md.jinja` (`grep -n webhooks`).

- [x] **Step 1: Réécrire le paragraphe EN**

Remplacer le paragraphe « `Identity` only says "the token is valid". … » par :

```markdown
The three routes require `Role::Admin`. A subscription delivers the project's events to
whoever created it, and `/auth/register` is open: under a merely valid token, anyone could
have every event delivered to them, or revoke someone else's subscription. To open a route
to any account, replace `Role::Admin` with `Role::User` on its `require_role` call — see
the [auth guide](./auth.md) for the guard.
```

- [x] **Step 2: Réécrire le paragraphe FR**

Même contenu en français, même position, dans le fichier FR.

- [x] **Step 3: Vérifier les autres mentions et la parité**

```bash
grep -rn "webhooks" docs/docs/cli/add.md docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/add.md crates/rbs-cli/templates/agents/ | grep -i "identity\|jeton\|token\|rôle\|role"
cargo test -p rbs-cli --test integration_docs 2>&1 | tail -5
cd docs && node scripts/parite.mjs 2>&1 | tail -5
```

Attendu : aucune mention contradictoire ; `integration_docs` vert ; parité sans écart nouveau (l'écart `IMPROVE_OLD.md` préexiste, tâche 28 du backlog).

- [x] **Step 4: Commit**

```bash
git add docs/docs/guides/webhooks.md docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/webhooks.md
git commit -m "docs(webhooks): documente le rôle admin exigé par défaut sur les abonnements" -m "Vérifications :
- cargo test -p rbs-cli --test integration_docs : vert
- node docs/scripts/parite.mjs : aucun écart nouveau"
```

---

### Task 4: Passe finale

- [x] **Step 1: Lint et format du workspace**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -3 && cargo test -p rbs-cli --lib 2>&1 | tail -3
```

Attendu : aucune sortie de fmt, clippy `Finished` sans warning, tests lib verts.

- [x] **Step 2: Rapport** — branche, commits (`git log --oneline main..HEAD`), et pour chaque preuve la ligne exacte lue dans la sortie.
