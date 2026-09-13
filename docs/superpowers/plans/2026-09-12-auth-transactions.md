# Auth : les parcours à plusieurs écritures passent en transaction — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `change`, `reset`, `refresh`, `verify` et `revoke_sessions` écrivent tout ou rien : leurs écritures passent par une transaction que les dépôts d'`auth` acceptent au même titre que la connexion.

**Architecture:** Les trois dépôts et les deux aides de `service/mod.rs` prennent `&impl ConnectionTrait` ; les cinq services ouvrent `db.begin()`, écrivent avec la transaction, committent — un `?` avant le commit l'abandonne et SeaORM l'annule. La première instruction de chaque transaction est une écriture (verrou SQLite pris d'emblée) ; lectures et Argon2 restent devant.

**Tech Stack:** SeaORM 2.0 (`ConnectionTrait`, `TransactionTrait`, `DatabaseTransaction`), tests `#[ignore]` joints à la base, régénération de `examples/blog-auth` par diff.

**Spec:** `docs/superpowers/specs/2026-09-12-auth-transactions-design.md`

## Global Constraints

- Fichiers sous `crates/rbs-cli/templates/features/auth/` sauf mention contraire.
- Aucun dépôt n'ouvre de transaction ; seuls les services le font, et une seule par parcours.
- Première instruction de chaque transaction : une écriture. Le hachage Argon2 précède `begin`.
- `examples/blog-auth` se régénère **par diff** entre deux générations (recette : `examples/README.md` § blog-auth), jamais par écrasement ; `cargo test -p rbs-cli --test integration_examples` est l'oracle (19 passés).
- Passe lente : `cargo test -p rbs-cli --test integration_auth -- --ignored --no-fail-fast > $SCRATCHPAD/auth-txn-<étape>.log 2>&1`, puis lire le fichier.
- Commits Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.
- Un commentaire dit le *pourquoi*, jamais le *quoi*.

---

### Task 1 : Les tests rouges

**Files:**
- Modify: `tests/password.rs.jinja` (après `an_expired_reset_token_is_refused_by_consume_and_removed_by_the_purge`)
- Modify: `tests/session.rs.jinja` (après `a_valid_refresh_returns_a_new_pair`)
- Modify: `crates/rbs-cli/tests/integration_auth.rs` (après `the_one_time_token_repository_is_written`)

**Interfaces:**
- Consumes: `registered_user`, `reset_token_expiring_in`, `login_as`, `session_row`, `connection` des tests engendrés ; `project_with_auth` d'`integration_auth`.

- [ ] **Step 1 : le test engendré de `password`**

```rust
/// Ce que `reset` enchaîne — consommer le jeton, poser le mot de passe — se défait d'un
/// bloc quand la transaction qui le porte est abandonnée : les dépôts acceptent la
/// transaction du service, et rien de ce qu'ils y ont écrit ne lui survit. C'est la
/// mécanique sur laquelle `change` et `reset` reposent pour ne pas brûler un jeton sans
/// poser le mot de passe qu'il promettait.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_rolled_back_transaction_leaves_the_token_and_the_password_untouched() {
    let db = connection().await;
    let compte = registered_user(&db).await;
    let jeton = reset_token_expiring_in(&db, compte.id, chrono::Duration::hours(1)).await;

    let transaction = db.begin().await.expect("transaction ouvrable");
    let consomme = crate::auth::repository::one_time_token::consume(&transaction, jeton.id)
        .await
        .expect("pas d'erreur");
    assert!(consomme, "le jeton vient d'être émis");
    crate::auth::repository::user::set_password(&transaction, compte.id, "un hash abandonné")
        .await
        .expect("pas d'erreur");
    transaction.rollback().await.expect("transaction annulable");

    let consomme = crate::auth::repository::one_time_token::consume(&db, jeton.id)
        .await
        .expect("pas d'erreur");
    assert!(consomme, "le jeton a été consommé par une transaction annulée");

    let relu = crate::auth::repository::find(&db, compte.id)
        .await
        .expect("la lecture aboutit")
        .expect("le compte existe");
    assert_eq!(
        relu.password_hash, compte.password_hash,
        "le mot de passe a été posé par une transaction annulée"
    );
}
```

Ajouter `use sea_orm::TransactionTrait;` en tête de `tests/password.rs.jinja` (après `use chrono::Utc;`).

- [ ] **Step 2 : le test engendré de `session`**

```rust
/// Ce que `refresh` enchaîne — tourner la session, en ouvrir une neuve — se défait d'un
/// bloc quand la transaction qui le porte est abandonnée : une erreur entre les deux ne
/// laisse pas au client une session tournée sans paire pour la remplacer.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_rolled_back_rotation_leaves_the_session_open_and_opens_no_other() {
    let api = application().await;
    let db = connection().await;
    let (id, _) = login_as(&api).await;
    let session = session_row(&db, id).await;

    let transaction = db.begin().await.expect("transaction ouvrable");
    let tournee = crate::auth::repository::refresh_token::rotate(&transaction, session.id)
        .await
        .expect("pas d'erreur");
    assert_eq!(tournee, Rotation::Done, "la session vient d'être ouverte");
    crate::auth::repository::create_refresh_token(
        &transaction,
        id,
        rbs_core::token::fingerprint(&rbs_core::token::random()),
        (Utc::now() + chrono::Duration::hours(1)).fixed_offset(),
    )
    .await
    .expect("pas d'erreur");
    transaction.rollback().await.expect("transaction annulable");

    let ouvertes = crate::auth::repository::open_sessions_of(&db, id)
        .await
        .expect("la lecture aboutit");
    assert_eq!(
        ouvertes.iter().map(|ligne| ligne.id).collect::<Vec<_>>(),
        vec![session.id],
        "une transaction annulée a tourné la session ou en a ouvert une autre"
    );

    let tournee = crate::auth::repository::refresh_token::rotate(&db, session.id)
        .await
        .expect("pas d'erreur");
    assert_eq!(tournee, Rotation::Done, "la session a été tournée par une transaction annulée");
}
```

En tête de `tests/session.rs.jinja` : `use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};` et `use crate::auth::repository::refresh_token::Rotation;`.

- [ ] **Step 3 : le test rapide d'`integration_auth`**

```rust
/// Les services qui enchaînent plusieurs écritures les font en une transaction, que les
/// dépôts acceptent au même titre que la connexion.
#[test]
fn the_services_chaining_writes_open_one_transaction() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth(&parent);

    for service in ["password", "session", "verification"] {
        let chemin = format!("src/auth/service/{service}.rs");
        let source = fs::read_to_string(racine.join(&chemin)).expect("service lisible");

        for attendu in ["db.begin().await?", ".commit().await?"] {
            assert!(
                source.contains(attendu),
                "`{chemin}` ne porte pas `{attendu}`"
            );
        }
    }

    for depot in ["user", "one_time_token", "refresh_token"] {
        let chemin = format!("src/auth/repository/{depot}.rs");
        let source = fs::read_to_string(racine.join(&chemin)).expect("dépôt lisible");

        assert!(
            source.contains("db: &impl ConnectionTrait") && !source.contains("&DatabaseConnection"),
            "`{chemin}` prend encore la connexion plutôt qu'un `ConnectionTrait`"
        );
    }
}
```

- [ ] **Step 4 : voir le rouge**

Run : `cargo test -p rbs-cli --test integration_auth the_services_chaining_writes_open_one_transaction`
Expected : FAIL, « `src/auth/service/password.rs` ne porte pas `db.begin().await?` ».

Run, sur un projet jetable engendré dans le scratchpad (`rbs new jetable --yes --core-path <racine>/crates/rbs-core --database-url 'postgres://rbs:rbs@localhost:5432/jetable'`, commit, `rbs add auth`) : `cargo check --all-targets`
Expected : FAIL, `expected `&DatabaseConnection`, found `&DatabaseTransaction`` sur les deux tests.

- [ ] **Step 5 : Commit**

```bash
git add crates/rbs-cli/templates/features/auth/tests/ crates/rbs-cli/tests/integration_auth.rs
git commit -m "test(auth): éprouve qu'une transaction annulée ne laisse rien des dépôts"
```

---

### Task 2 : Les dépôts et les aides du service prennent `&impl ConnectionTrait`

**Files:**
- Modify: `repository/user.rs.jinja`, `repository/one_time_token.rs.jinja`, `repository/refresh_token.rs.jinja` (toutes les signatures)
- Modify: `service/mod.rs.jinja:28-32` (`issue`), `:77` (`close_every_session`)

**Interfaces:**
- Produces: chaque `pub async fn` des trois dépôts en `db: &impl ConnectionTrait` ; `async fn issue(db: &impl ConnectionTrait, auth: &AuthConfig, utilisateur: &Model)` ; `pub(super) async fn close_every_session(db: &impl ConnectionTrait, user_id: Uuid)`.

- [ ] **Step 1 : les imports**

`user.rs.jinja` : `use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};`
`one_time_token.rs.jinja` : idem.
`refresh_token.rs.jinja` : `use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};`
`service/mod.rs.jinja` : `use sea_orm::ConnectionTrait;` à la place de `use sea_orm::DatabaseConnection;`.

- [ ] **Step 2 : les signatures** — `sed -i '' 's/db: &DatabaseConnection/db: \&impl ConnectionTrait/g'` sur les trois dépôts ; puis `rustfmt` à la main sur les signatures qui repassent sous 100 colonnes (`stamp_sessions_revoked`, `open_sessions_of`, `find_refresh_token` tiennent sur une ligne ? vérifier avec `cargo fmt` sur le projet jetable). Dans `service/mod.rs.jinja`, `issue` et `close_every_session`.

- [ ] **Step 3 : commentaire de tête du dépôt** — dans `repository/mod.rs.jinja`, après le paragraphe existant :

```rust
//!
//! Chaque fonction prend un `ConnectionTrait` et non la connexion : une transaction en
//! est un, et c'est le service qui décide si l'écriture qu'il demande vit seule ou dans
//! la suite d'un parcours.
```

- [ ] **Step 4 : vérifier** — projet jetable : `cargo check --all-targets` vert (les deux tests compilent, les services passent encore la connexion). Le test rapide reste rouge sur les services.

- [ ] **Step 5 : Commit**

```bash
git add crates/rbs-cli/templates/features/auth/repository/ crates/rbs-cli/templates/features/auth/service/mod.rs.jinja
git commit -m "refactor(auth): les dépôts acceptent une transaction comme la connexion"
```

---

### Task 3 : Les services ouvrent la transaction

**Files:**
- Modify: `service/password.rs.jinja` (`change`, `reset`)
- Modify: `service/session.rs.jinja` (`refresh`, `revoke_sessions`)
- Modify: `service/verification.rs.jinja` (`verify`)

- [ ] **Step 1 : `password.rs.jinja`** — import `use sea_orm::{DatabaseConnection, TransactionTrait};`. `change`, à partir du hachage :

```rust
    let nouveau = hash::hash_password(&input.new_password)?;

    // Tout ou rien : un mot de passe changé dont les sessions resteraient ouvertes
    // laisserait tourner celles d'un compte que ce changement vient peut-être de
    // reprendre. Le hachage est fait avant : Argon2 coûte des dizaines de millisecondes,
    // et un verrou d'écriture ne se tient pas pendant un calcul.
    let transaction = db.begin().await?;

    repository::user::set_password(&transaction, user_id, &nouveau).await?;

    // Une boîte compromise a pu recevoir … (commentaire existant inchangé)
    repository::one_time_token::invalidate_pending(&transaction, user_id, TokenPurpose::PasswordReset)
        .await?;

    let fermees = close_every_session(&transaction, user_id).await?;

    // Rechargé : `issue` lit l'estampille que la fermeture vient de poser, et n'émettrait
    // sinon qu'un jeton né dans la seconde qu'elle vient de tuer.
    let utilisateur = repository::find(&transaction, user_id)
        .await?
        .ok_or(Error::Unauthorized)?;

    let paire = issue(&transaction, auth, &utilisateur).await?;
    transaction.commit().await?;

    // Ni l'adresse ni les jetons : … (commentaire existant)
    tracing::info!(…);

    Ok(paire)
```

`reset`, à partir du hachage — déplacer `let nouveau = hash::hash_password(...)` **avant** `consume` :

```rust
    let nouveau = hash::hash_password(&input.new_password)?;

    // Tout ou rien : un jeton consommé sans que le mot de passe suive serait brûlé pour
    // rien, et l'utilisateur devrait en redemander un.
    let transaction = db.begin().await?;

    // Rien ici ne relit `consumed_at` ni `expires_at` : … (commentaire existant)
    if !repository::one_time_token::consume(&transaction, ligne.id).await? {
        return Err(Error::Unauthorized);
    }

    repository::user::set_password(&transaction, ligne.user_id, &nouveau).await?;

    // Symétrique à `change` : … (commentaire existant)
    repository::one_time_token::invalidate_pending(&transaction, ligne.user_id, TokenPurpose::PasswordReset)
        .await?;

    let fermees = close_every_session(&transaction, ligne.user_id).await?;
    transaction.commit().await?;
```

- [ ] **Step 2 : `session.rs.jinja`** — import `use sea_orm::{DatabaseConnection, TransactionTrait};`. `refresh` :

```rust
    // Tout ou rien : une session tournée sans paire rendue laisserait le client avec un
    // jeton mort et rien pour le remplacer — et son prochain essai serait un rejeu.
    let transaction = db.begin().await?;

    match repository::refresh_token::rotate(&transaction, session.id).await? {
        Rotation::Done => {}
        Rotation::Replayed => {
            let fermees = close_every_session(&transaction, session.user_id).await?;
            // La révocation est ce qu'un rejeu doit laisser derrière lui : elle se
            // committe avant que l'erreur ne sorte.
            transaction.commit().await?;

            tracing::warn!(…);

            return Err(Error::Unauthorized);
        }
        // … `Closed` inchangé : rien n'a été écrit, la transaction abandonnée s'annule.
    }

    let utilisateur = repository::find(&transaction, session.user_id)
        .await?
        .ok_or(Error::Unauthorized)?;

    let paire = issue(&transaction, auth, &utilisateur).await?;
    transaction.commit().await?;

    Ok(paire)
```

`revoke_sessions` :

```rust
pub async fn revoke_sessions(db: &DatabaseConnection, user_id: Uuid) -> Result<()> {
    // Les deux écritures de `close_every_session` ou aucune : fermer les
    // rafraîchissements en laissant vivre les accès est ce qu'elle existe pour empêcher.
    let transaction = db.begin().await?;
    close_every_session(&transaction, user_id).await?;
    transaction.commit().await?;

    Ok(())
}
```

- [ ] **Step 3 : `verification.rs.jinja`** — import `use sea_orm::{DatabaseConnection, TransactionTrait};`. `verify` :

```rust
    // Tout ou rien : un jeton consommé sans que l'adresse soit datée serait brûlé pour
    // rien, et l'utilisateur devrait en redemander un.
    let transaction = db.begin().await?;

    if !repository::one_time_token::consume(&transaction, ligne.id).await? {
        return Err(Error::Unauthorized);
    }

    repository::user::mark_verified(&transaction, ligne.user_id).await?;
    transaction.commit().await?;

    Ok(())
```

- [ ] **Step 4 : vérifier** — `cargo test -p rbs-cli --test integration_auth the_services_chaining_writes_open_one_transaction` vert ; projet jetable : `cargo check --all-targets` et `cargo clippy --all-targets -- -D warnings` verts, `cargo fmt --check` vert.

- [ ] **Step 5 : Commit**

```bash
git add crates/rbs-cli/templates/features/auth/service/
git commit -m "fix(auth): change, reset, refresh, verify et la révocation écrivent tout ou rien"
```

---

### Task 4 : L'exemple `blog-auth`

**Files:**
- Modify: `examples/blog-auth/src/auth/{repository,service,tests}/*.rs`

- [ ] **Step 1 : deux générations** — `git stash`-free : un worktree sur `main` (`git worktree add $SCRATCHPAD/avant main`), `cargo build -p rbs-cli --bin rbs` des deux côtés, générer `blog-auth` deux fois selon `examples/README.md` (dans `$SCRATCHPAD/gen-avant/` et `$SCRATCHPAD/gen-apres/`).
- [ ] **Step 2 : le diff** — `diff -ruN gen-avant/blog-auth gen-apres/blog-auth -x Cargo.lock -x migration -x .git > auth-txn.patch` ; ne doit toucher que `src/auth/`. Appliquer sur `examples/blog-auth` par `patch -p1 --dry-run` puis `patch -p1`.
- [ ] **Step 3 : vérifier** — `cargo test -p rbs-cli --test integration_examples` (19 passés) ; dans `examples/blog-auth` : `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- [ ] **Step 4 : Commit**

```bash
git add examples/blog-auth
git commit -m "chore(examples): reporte sur blog-auth les transactions du fragment auth"
```

---

### Task 5 : La passe lente

- [ ] **Step 1** : `cargo test -p rbs-cli --test integration_auth -- --ignored --no-fail-fast > $SCRATCHPAD/auth-txn-lent.log 2>&1` ; lire le fichier : tous passés, dont `the_auth_tests_of_the_generated_project_pass` et `..._on_sqlite` ; les deux tests neufs y apparaissent en ` ... ok`.
- [ ] **Step 2** : `cargo test -p rbs-cli --test integration_docs -- --include-ignored` (14 passés).

---

### Task 6 : Journal et clôture

**Files:**
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md` (section `Fixed` de `[1.5.0]`)

- [ ] **Step 1 : l'item anglais**, en fin de `Fixed` :

```markdown
- **`change-password`, `reset-password`, `refresh`, `verify-email` and `DELETE
  /auth/sessions` write everything or nothing.** Each used to chain its writes on
  separate connections: a failure between consuming a reset token and setting the
  password burnt the token for nothing, one between the new password and the revocation
  left the sessions of a possibly compromised account open, and one between rotating a
  refresh token and issuing the new pair left the client with a dead token and no
  replacement — whose next attempt counted as a replay. Every `auth` repository now takes
  `&impl ConnectionTrait`, like `jobs::enqueue`, and the five services open one
  transaction. No migration: a project already carrying `auth` gets the rule by copying
  the fragment's `repository/` and `service/` directories.
```

- [ ] **Step 2 : l'item français**, même position, même sens.
- [ ] **Step 3 : vérifications finales** — `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p rbs-cli --lib`, `cargo test -p rbs-core --all-features`, `cd docs && npm test && npm run typecheck`.
- [ ] **Step 4 : Commit**

```bash
git add CHANGELOG.md CHANGELOG.fr.md
git commit -m "docs(changelog): consigne les transactions des parcours d'auth"
```
