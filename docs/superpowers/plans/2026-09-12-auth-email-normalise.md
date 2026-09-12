# Auth : l'adresse est normalisée avant d'atteindre la base — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `Victime@ex.fr`, ` victime@ex.fr ` et `victime@ex.fr` désignent le même compte à l'inscription, à la connexion, à l'oubli de mot de passe et au renvoi de vérification.

**Architecture:** Une fonction `normalise` dans `service/mod.rs.jinja` (`trim` puis minuscules), appliquée à l'entrée des quatre parcours qui reçoivent une adresse. Le DTO ne change pas.

**Tech Stack:** Rust, tests `#[ignore]` joints à la base, régénération de `examples/blog-auth` par diff.

**Spec:** `docs/superpowers/specs/2026-09-12-lot-secu-p2-design.md`, section 4.

## Global Constraints

- Ce plan suit `2026-09-12-auth-jeton-acces-revocable.md` sur la même branche.
- Fichiers sous `crates/rbs-cli/templates/features/auth/`.
- `examples/blog-auth` régénéré par diff ; `integration_examples` est l'oracle.
- Passe lente : `cargo test -p rbs-cli --test integration_auth --no-fail-fast -- --ignored > $SCRATCHPAD/auth-lent-12.log 2>&1`.
- Commits Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.

---

### Task 1: La normalisation et ses tests

**Files:**
- Modify: `service/mod.rs.jinja` (`normalise`)
- Modify: `service/session.rs.jinja:25-41` (`register`, `login`), `service/password.rs.jinja:68-73` (`request_reset`), `service/verification.rs.jinja` (`request`, lire ses premières lignes pour trouver l'appel à `find_by_email`)
- Test: `tests/session.rs.jinja`

- [x] **Step 1: Tests rouges** — avec `normalise` rendue identité sur le projet jetable : `login_ignores_the_case_of_the_address` et `an_address_taken_in_another_case_is_a_conflict` FAILED (`401/200`, `201/409`). Le test des blancs par la route rend 422 (`#[validate(email)]` refuse les blancs) : il ne fait varier que la casse (`registration_lowercases_the_address`), et un test unitaire non `#[ignore]`, `an_address_is_trimmed_and_lowercased_before_the_table`, prouve le `trim`, après `an_email_already_taken_returns_409_without_repeating_it` :

```rust
/// La casse et les blancs ne font pas deux comptes : sans cela, l'attaquant inscrit
/// `Victime@ex.fr`, la victime clique le lien de vérification qu'elle reçoit, et le
/// compte de l'attaquant porte son adresse, vérifiée.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn registration_lowercases_and_trims_the_address() {
    let api = application().await;
    let base = fresh_email();
    let bruitee = format!("  {}  ", base.to_uppercase());

    let (status, profile) = register(&api, &bruitee).await;

    assert_eq!(status, StatusCode::CREATED, "{profile}");
    assert_eq!(profile["email"], base);
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn login_ignores_the_case_of_the_address() {
    let api = application().await;
    let email = fresh_email();
    register(&api, &email).await;

    let (status, paire) = authenticate(&api, &email.to_uppercase(), PASSWORD).await;

    assert_eq!(status, StatusCode::OK, "{paire}");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_address_taken_in_another_case_is_a_conflict() {
    let api = application().await;
    let email = fresh_email();
    register(&api, &email).await;

    let (status, body) = register(&api, &email.to_uppercase()).await;

    assert_eq!(status, StatusCode::CONFLICT, "{body}");
}
```

`fresh_email()` rend `<uuid>@exemple.test`, en minuscules : `to_uppercase()` en fait bien une autre chaîne.

- [x] **Step 2: `normalise`** dans `service/mod.rs.jinja` (`use super::normalise;` se place après `super::super::…`, rustfmt l'exige) :

```rust
/// L'adresse telle que la base la voit.
///
/// Minuscules et sans blancs : la partie locale est théoriquement sensible à la casse
/// (RFC 5321 §2.4), aucun fournisseur ne l'honore, et c'est l'attaquant qui en
/// profiterait — deux comptes pour une boîte, dont un vérifié par l'autre.
pub(super) fn normalise(email: &str) -> String {
    email.trim().to_lowercase()
}
```

Appliquer :

- `register` : `let email = normalise(&input.email);` puis `find_by_email(db, &email)` et `create(db, &email, &hash)`.
- `login` : `let email = normalise(&input.email);` puis `find_by_email(db, &email)`.
- `password::request_reset(db, ttl_secs, email)` : `let email = normalise(email);` puis `find_by_email(db, &email)`.
- `verification::request` : idem à l'entrée.

Les quatre imports : `use super::normalise;` (`session.rs`, `password.rs`, `verification.rs` sont des sous-modules de `service`).

- [x] **Step 3: Vérifier** — `cargo check` vert ; `cargo test --lib -- --include-ignored auth::tests` → 52 passed; 0 failed — `cargo check` sur projet jetable, puis `cargo test --lib -- --ignored auth::tests` avec base : les trois tests neufs passent, `an_email_already_taken_returns_409_without_repeating_it` et les tests de temps constant aussi.

- [x] **Step 4: Régénérer `examples/blog-auth` par diff** — patch des cinq templates (sans tag), miroir identique ; clippy de l'exemple vert ; `integration_examples` → 19 passed, `cargo clippy --all-targets -- -D warnings` dans l'exemple, `cargo test -p rbs-cli --test integration_examples`.

- [x] **Step 5: Passe lente** — `auth-lent-12.log` : 8 passed; 0 failed (206 s), dont le banc SQLite ; `fmt --check` vert ; clippy workspace 0 ; `rbs-cli --lib` 1151 passed `integration_auth` ; `cargo fmt --all --check` ; `cargo clippy --workspace --all-targets -- -D warnings` ; `cargo test -p rbs-cli --lib`.

- [x] **Step 6: Commit** — 9019622

```bash
git add crates/rbs-cli/templates/features/auth/ examples/blog-auth/
git commit -m "fix(auth): normalise l'adresse avant l'inscription, la connexion et les deux demandes anonymes"
```

---

### Task 2: Notes de version et guide

**Files:**
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md`
- Modify: `docs/docs/guides/auth.md` (+ FR) : une phrase dans la section des routes `register`/`login`, ou là où `#[validate(email)]` est cité (`grep -n 'validate(email)\|lowercase' docs/docs/guides/auth.md`)

- [x] **Step 1: CHANGELOG**, `### Fixed` de 1.5.0 (EN + FR ; les trois ALTER/UPDATE regroupés sous « Projets déjà générés », comme le spec le demande) :

```markdown
- **Email addresses are trimmed and lowercased** before `register`, `login`,
  `forgot-password` and `resend-verification`. Two registrations differing only by case
  used to make two accounts — and the verification link of one landed in the other's
  mailbox. **Projects generated before 1.5.0:** `UPDATE users SET email = lower(trim(email));`
  — the unique key refuses it if two accounts differ only by case, which is the case to
  settle by hand — then copy `normalise` and its four call sites from the fragment.
```

- [x] **Step 2: Guide** — EN + FR, paragraphe avant « the same 401 » — une phrase : « Addresses are trimmed and lowercased before they reach the table; `Alice@Example.test` and `alice@example.test` are one account. »

- [x] **Step 3: `cargo test -p rbs-cli --test integration_docs`** → 13 passed; 0 failed; 1 ignored (223 s). Commit :

```bash
git commit -am "docs(auth): dit que l'adresse est normalisée avant la table"
```
