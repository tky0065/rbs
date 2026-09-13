# Montée des dépendances en retard — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs-core` passe à argon2 0.6 et jsonwebtoken 11 sans qu'un projet en production perde ses mots de passe ni ses sessions ; les fragments `redis` et `storage` engendrent les dernières mineures de `redis` et `aws-sdk-s3`.

**Architecture:** Deux fixtures figées sous l'ancienne version — un hash PHC d'argon2 0.5.3 et un JWT HS256 de jsonwebtoken 10.3.0 — deviennent des tests unitaires de `hash.rs` et `jwt.rs`, verts avant la montée et gardiens après. La montée touche `[workspace.dependencies]` et `hash.rs` (API de `password-hash` 0.6). Les deux `feature.toml` changent d'une ligne chacun, et `examples/file-drop` suit par diff entre deux générations.

**Tech Stack:** argon2 0.6.0 / password-hash 0.6.1 / phc 0.6.1, jsonwebtoken 11.0.0 (`rust_crypto`), redis 1.7.0, aws-sdk-s3 1.146.1, `cargo audit`, `cargo semver-checks`.

**Spec:** consigne du mainteneur du 2026-09-13 (montée des dépendances en retard) ; pas de document de conception séparé, les décisions sont reprises ci-dessous.

## Global Constraints

- Toujours la dernière version stable, résolue par `cargo add --dry-run <crate>` dans une crate jetable — relevé du 2026-09-13 : `argon2` 0.6.0, `jsonwebtoken` 11.0.0, `redis` 1.7.0, `aws-sdk-s3` 1.146.1, `deadpool-redis` 0.23.1, `async-trait` 0.1.92, `thiserror` 2.0.20, `serde_json` 1.0.151, `rsa` 0.9.10.
- Aucun type d'`argon2` ni de `jsonwebtoken` dans l'API publique de `rbs-core` : les signatures `pub` de `hash.rs` et `jwt.rs` ne changent pas. `cargo semver-checks` (CI : `package: rbs-core`, `feature-group: all-features`) doit rester vert.
- Pas de `cargo update` global : le `Cargo.lock` racine et ceux d'`examples/` ne bougent que de ce que la montée impose.
- Version du workspace inchangée (1.5.0). `CHANGELOG*`, `IMPROVE.md`, `TODO.md`, `ROADMAP.md`, `templates/agents/*` intouchés.
- L'entrée `RUSTSEC-2026-0235` de `.cargo/audit.toml` n'est pas touchée.
- Commits Conventional, sujet en français à l'impératif ; corps : le pourquoi puis `Vérifications :` ; aucun identifiant de tâche, aucune ligne d'attribution.

## Constats préalables (relevés avant le plan)

- argon2 0.6 supprime la feature `std` (#768) ; ses features par défaut `alloc`, `getrandom`, `password-hash` fournissent `PasswordHasher::hash_password(&self, &[u8])`, qui tire lui-même un sel de 16 octets par `getrandom` — `SaltString` et `rand_core::OsRng` disparaissent. `Error::Password` devient `Error::PasswordInvalid`. `PasswordHash` est `phc::PasswordHash`, réexporté par `argon2`, avec `new(&str)` et `Display`.
- Paramètres par défaut identiques entre 0.5.3 et 0.6.0 : `m=19456, t=2, p=1`, sortie de 32 octets. Les nouveaux hashes gardent la forme `$argon2id$v=19$m=19456,t=2,p=1$…`.
- jsonwebtoken 11 : `decode` prend `impl AsRef<[u8]>` (un `&str` convient), `ErrorKind::ExpiredSignature` et `InvalidSignature` existent toujours ; les ruptures (`Algorithm` non exhaustif, `Header.extras`, clés) ne touchent aucun appel de `jwt.rs`. La feature `rust_crypto` tire toujours `rsa` 0.9.10, dernière version parue.
- Fixtures produites par une crate jetable dépendant de `rbs-core` (feature `auth`) avec le `Cargo.lock` du dépôt — `cargo tree` : argon2 v0.5.3, jsonwebtoken v10.3.0.

---

### Task 1 : fixtures de compatibilité, vertes sur les versions actuelles

**Files:**
- Modify: `crates/rbs-core/src/hash.rs` (`mod tests`)
- Modify: `crates/rbs-core/src/jwt.rs` (`mod tests`)

**Interfaces:**
- Consumes: `hash::verify_password(&str, &str) -> crate::Result<bool>`, `jwt::verify(&str, &str) -> Result<Claims, JwtError>`.
- Produces: tests `hash::tests::a_hash_produced_by_argon2_0_5_still_verifies` et `jwt::tests::a_token_signed_by_jsonwebtoken_10_is_still_accepted`, que la Task 2 doit garder verts.

- [ ] **Step 1 : ajouter le test du hash** en fin de `mod tests` de `hash.rs`

```rust
    /// Produit par `hash_password` sous argon2 0.5.3. Un hash stocké survit à toutes les
    /// montées de la crate : s'il cessait d'être vérifiable, chaque compte existant
    /// resterait à la porte.
    const HASH_ARGON2_0_5: &str = "$argon2id$v=19$m=19456,t=2,p=1$25RrSvatxiDwqIo1EoA4tg$rgrgr+jlp/HLwVgJZcn/wVlPr4Vl3d05n9sPcxXZCIg";

    #[test]
    fn a_hash_produced_by_argon2_0_5_still_verifies() {
        assert!(
            verify_password("mot de passe d'avant la montée", HASH_ARGON2_0_5)
                .expect("vérification")
        );
        assert!(!verify_password("un autre mot de passe", HASH_ARGON2_0_5).expect("vérification"));
    }
```

- [ ] **Step 2 : ajouter le test du jeton** en fin de `mod tests` de `jwt.rs`

```rust
    /// Signé par `sign` sous jsonwebtoken 10.3.0, avec `SECRET`. Un jeton émis avant une
    /// montée de la crate doit rester accepté jusqu'à son expiration : sinon chaque
    /// redéploiement déconnecterait tous les utilisateurs.
    const TOKEN_JSONWEBTOKEN_10: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1MSIsInJvbGUiOiJ1c2VyIiwiZXhwIjo0MTAyNDQ0ODAwLCJpYXQiOjE3NTc3MjE2MDAsImp0aSI6ImpldG9uLWQtYXZhbnQtbGEtbW9udGVlIn0.1cV7nL6cDoBmccJzloNGeTmnPiyCIwOwy2uSHteEQ8Q";

    #[test]
    fn a_token_signed_by_jsonwebtoken_10_is_still_accepted() {
        let expected = Claims {
            sub: "u1".to_owned(),
            role: "user".to_owned(),
            exp: LATER,
            iat: 1_757_721_600,
            jti: "jeton-d-avant-la-montee".to_owned(),
        };

        assert_eq!(verify(TOKEN_JSONWEBTOKEN_10, SECRET).expect("vérification"), expected);
    }
```

- [ ] **Step 3 : les deux tests passent sur l'ancienne version** (ce sont des gardes de non-régression : vertes avant la montée, par construction)

Run: `cargo test -p rbs-core --all-features -- hash::tests jwt::tests`
Expected: PASS, dont les deux nouveaux ; `cargo tree -p rbs-core --all-features -i argon2 --depth 0` rend `argon2 v0.5.3`.

- [ ] **Step 4 : `cargo fmt --all --check`** puis commit

```bash
git add crates/rbs-core/src/hash.rs crates/rbs-core/src/jwt.rs
git commit   # test(core): fige un hash argon2 0.5 et un jeton jsonwebtoken 10 comme fixtures
```

### Task 2 : argon2 0.6 et jsonwebtoken 11

**Files:**
- Modify: `Cargo.toml:18` et `Cargo.toml:32`
- Modify: `crates/rbs-core/src/hash.rs:8-45`
- Modify: `crates/rbs-core/src/jwt.rs` seulement si la compilation l'exige
- Modify: `Cargo.lock` (par la seule résolution de cargo)

**Interfaces:**
- Consumes: les deux tests de fixtures de la Task 1.
- Produces: signatures `pub` inchangées de `hash` et `jwt`.

- [ ] **Step 1 : manifeste du workspace**

```toml
# Les features par défaut — `alloc`, `getrandom`, `password-hash` — sont celles dont `hash`
# a besoin : `getrandom` tire le sel. La 0.6 a supprimé `std`.
argon2 = "0.6.0"
```

```toml
jsonwebtoken = { version = "11.0.0", default-features = false, features = ["rust_crypto"] }
```

- [ ] **Step 2 : constater l'échec de compilation**

Run: `cargo check -p rbs-core --all-features`
Expected: FAIL — `SaltString`, `rand_core::OsRng` introuvables, `Error::Password` inconnu.

- [ ] **Step 3 : adapter `hash.rs`**

```rust
use argon2::password_hash::Error as HashError;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

use crate::Error;

pub fn hash_password(password: &str) -> crate::Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|error| Error::Internal(anyhow::anyhow!("hachage Argon2 : {error}")))
}

pub fn verify_password(password: &str, hash: &str) -> crate::Result<bool> {
    let expected = PasswordHash::new(hash)
        .map_err(|error| Error::Internal(anyhow::anyhow!("hash PHC illisible : {error}")))?;

    match Argon2::default().verify_password(password.as_bytes(), &expected) {
        Ok(()) => Ok(true),
        // Le seul cas où l'échec vient du client : il ne doit pas devenir un 500.
        Err(HashError::PasswordInvalid) => Ok(false),
        Err(error) => Err(Error::Internal(anyhow::anyhow!(
            "vérification Argon2 : {error}"
        ))),
    }
}
```

Les `///` et le `//!` restent : « un sel tiré du générateur du système à chaque appel » est toujours vrai, par `getrandom`.

- [ ] **Step 4 : `jwt.rs`** — compiler tel quel ; ne rien toucher si `cargo check` passe.

- [ ] **Step 5 : tests et graphe**

Run: `cargo test -p rbs-core --all-features`
Expected: PASS, 146 tests (144 + 2 fixtures).
Run: `cargo tree -p rbs-core --all-features -i argon2 --depth 0` → `argon2 v0.6.0` ; idem `jsonwebtoken v11.0.0`.
Run: `git diff --stat Cargo.lock` : seules les entrées tirées par argon2/jsonwebtoken bougent.

- [ ] **Step 6 : `cargo fmt --all --check`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`, `cargo semver-checks -p rbs-core --all-features`**, puis commit

```bash
git add Cargo.toml Cargo.lock crates/rbs-core/src/hash.rs crates/rbs-core/src/jwt.rs
git commit   # build(deps): monte argon2 en 0.6 et jsonwebtoken en 11
```

### Task 3 : exception d'audit `rsa`

**Files:**
- Aucun, sauf si `rsa` disparaît de l'arbre.

- [ ] **Step 1** : `cargo tree -i rsa --workspace --all-features`. Constat préalable : jsonwebtoken 11 / `rust_crypto` tire toujours `rsa` 0.9.10, dernière version parue → `RUSTSEC-2023-0071` reste, et son commentaire (« tirée par `jsonwebtoken` ») reste exact. Le fichier n'est pas modifié.
- [ ] **Step 2** : `cargo audit --deny warnings` vert, résultat consigné dans le rapport.

### Task 4 : mineures des fragments `redis` et `storage`, et `file-drop`

**Files:**
- Modify: `crates/rbs-cli/templates/features/redis/feature.toml:36` → `version  = "1.7"`
- Modify: `crates/rbs-cli/templates/features/storage/feature.toml:53` → `version = "1.146"`
- Modify: `examples/file-drop/Cargo.toml` (le diff entre deux générations, rien d'autre), `examples/file-drop/Cargo.lock` (la résolution qu'impose la montée)
- Modify: `examples/*/Cargo.lock` des trois autres exemples : seulement ce qu'impose la montée d'argon2/jsonwebtoken dans `rbs-core` (dépendance par chemin)

Les autres dépendances des deux fragments sont déjà à la dernière mineure : `deadpool-redis` « 0.23 » (0.23.1), `serde_json` « 1.0 », `async-trait` « 0.1 », `thiserror` « 2.0 ». `aws-config` n'est pas une dépendance du fragment `storage`, délibérément (`docs/docs/guides/storage.md:102`).

- [ ] **Step 1 : génération « avant »** — dans le scratchpad, CLI d'avant la modification, les commandes de `examples/README.md` § `file-drop` (projet `file-drop-avant`).
- [ ] **Step 2 : modifier les deux `feature.toml`.**
- [ ] **Step 3 : génération « après »** — mêmes commandes, projet `file-drop-apres`.
- [ ] **Step 4 : `diff -r` des deux générations** (hors `.git`) : attendu, deux lignes de `Cargo.toml`. Reporter ce diff sur `examples/file-drop/Cargo.toml` — jamais d'écrasement.
- [ ] **Step 5 : oracle** — `cargo test -p rbs-cli --test integration_examples` : PASS.
- [ ] **Step 6 : compilation réelle** — dans `examples/file-drop` : `cargo check --all-targets` puis `cargo clippy --all-targets -- -D warnings` ; `cargo tree -i redis --depth 0` → 1.7.0, `cargo tree -i aws-sdk-s3 --depth 0` → 1.146.1. Dans les trois autres exemples : `cargo clippy --all-targets -- -D warnings`. `git diff --stat examples/` : seuls les deux `Cargo.toml`/`Cargo.lock` attendus.
- [ ] **Step 7 : pins résiduels** — `grep -rn '1\.6"\|1\.144' crates docs examples` : rien hors des plans et specs historiques de `docs/superpowers/` (qui consignent la décision de leur jour et ne se réécrivent pas).
- [ ] **Step 8 : commits**

```bash
git add crates/rbs-cli/templates/features/redis/feature.toml crates/rbs-cli/templates/features/storage/feature.toml examples/file-drop
git commit   # build(deps): monte redis en 1.7 et aws-sdk-s3 en 1.146 dans les fragments
git add examples/*/Cargo.lock
git commit   # chore(examples): résout argon2 0.6 et jsonwebtoken 11 dans les verrous des exemples
```

### Task 5 : vérification finale

- [ ] `cargo fmt --all --check` ; `cargo clippy --workspace --all-features --all-targets -- -D warnings` ; `cargo test --workspace --all-features --no-fail-fast` (hors Docker).
- [ ] `cargo semver-checks -p rbs-core --all-features` ; `cargo audit --deny warnings`.
- [ ] Suites Docker, une commande par suite, en arrière-plan, sortie dans le scratchpad, `--no-fail-fast` avant le `--` :
  - `cargo test -p rbs-cli --test integration_auth --no-fail-fast -- --ignored` (hash + JWT dans un projet engendré) ;
  - `cargo test -p rbs-cli --test integration_redis --no-fail-fast -- --ignored` ;
  - `cargo test -p rbs-cli --test integration_storage --no-fail-fast -- --ignored` ;
  - `cargo test -p rbs-cli --test integration_crud --no-fail-fast -- --ignored` (deux tests compilent `storage`) ;
  - `integration_examples` n'a ni `#[ignore]` ni Docker : il est déjà joué par la passe rapide ci-dessus, et c'est l'oracle de la Task 4.
