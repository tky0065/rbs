# Connexion base — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** ouvrir le pool SeaORM depuis la configuration avec des timeouts explicites, et échouer au démarrage — pas au premier appel HTTP — sur une URL inexploitable.

**Architecture:** un module `db` exposant une seule fonction `connect(&DatabaseConfig)`. Les six réglages du pool deviennent des champs de `DatabaseConfig` pourvus de défauts, donc invisibles pour qui n'y touche pas et surchargeables par `RBS_DATABASE__*`. L'échec de connexion porte son propre type `ConnectError`, hors de `Error` : une panne au boot ne devient jamais une réponse HTTP (précédent posé par `ConfigError` en A5). Le message nomme le champ à corriger et masque le mot de passe, parce que cette erreur part dans les logs de démarrage.

**Tech Stack:** `sea-orm` (features `sqlx-postgres`, `runtime-tokio-rustls`, `macros`), `thiserror`, `tokio` (`rt-multi-thread` pour les tests).

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §3.2, §5.3

## Global Constraints

- `#![warn(missing_docs)]` sur `rbs-core` : tout item public porte un `///` d'une à trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.
- Dépendances déclarées dans `[workspace.dependencies]`, reprises par `.workspace = true`.
- Aucun test ne requiert de base démarrée : une URL syntaxiquement invalide est rejetée avant tout accès réseau.

---

### Task 1 : pool SeaORM configuré

**Files:**
- Create: `crates/rbs-core/src/db.rs` (implémentation + `#[cfg(test)] mod tests`)
- Modify: `crates/rbs-core/src/config.rs` (six champs sur `DatabaseConfig` + leurs défauts)
- Modify: `crates/rbs-core/src/lib.rs` (`pub mod db`)
- Modify: `Cargo.toml`, `crates/rbs-core/Cargo.toml` (features `sea-orm` et `tokio`)

**Interfaces:**
- Consumes: `crate::config::DatabaseConfig` de la tâche A5.
- Produces: `rbs_core::db::connect() -> Result<DatabaseConnection, ConnectError>`, `rbs_core::db::ConnectError`.

- [ ] **Step 1 : activer les features `sea-orm`**

Sans driver ni runtime, `Database::connect` n'existe pas. Workspace : `sea-orm = { version = "2.0.2", features = ["sqlx-postgres", "runtime-tokio-rustls", "macros", "with-chrono", "with-uuid"] }`. `tokio` gagne `rt-multi-thread` en dev-dependency.

- [ ] **Step 2 : écrire les tests d'abord**

Dans `db.rs` :

```rust
#[tokio::test]
async fn une_url_invalide_echoue_avec_un_message_nommant_le_champ() {
    let config = config("pas-une-url");

    let erreur = connect(&config).await.expect_err("URL inexploitable");

    let message = erreur.to_string();
    assert!(message.contains("database.url"), "champ non nommé : {message}");
}

#[tokio::test]
async fn le_mot_de_passe_n_apparait_pas_dans_le_message_d_erreur() {
    let config = config("postgres://user:s3cr3t@/base introuvable");

    let erreur = connect(&config).await.expect_err("URL inexploitable");

    let message = format!("{erreur}{:?}", erreur);
    assert!(!message.contains("s3cr3t"), "mot de passe divulgué : {message}");
}
```

Dans `config.rs`, un test des défauts du pool sous `Jail`.

- [ ] **Step 3 : lancer les tests, les voir échouer**

Run: `cargo test -p rbs-core db`
Expected: échec de compilation, `db::connect` inexistant.

- [ ] **Step 4 : étendre `DatabaseConfig`**

Six champs `max_connections`, `min_connections`, `connect_timeout_secs`, `acquire_timeout_secs`, `idle_timeout_secs`, `max_lifetime_secs`, chacun avec son `Serialized::default` dans `figment()`, aux côtés de `server.host` et `server.port`. `url` reste le seul champ sans défaut.

- [ ] **Step 5 : implémenter `db.rs`**

- `pub enum ConnectError` (thiserror), variante unique portant la `DbErr` source ; le message nomme `database.url` et `RBS_DATABASE__URL`.
- Une fonction de masquage remplace le mot de passe de l'URL avant qu'il n'entre dans un message. `DbErr` recrache l'URL complète et cette erreur part dans les logs — commenter ce pourquoi.
- `connect` construit un `ConnectOptions` depuis la configuration, pose les six réglages et `sqlx_logging(false)` : sea-orm loguerait via `log`, en doublon du middleware de trace de B4.

- [ ] **Step 6 : lancer les tests, les voir passer**

Run: `cargo test -p rbs-core`
Expected: tous verts, dont les deux nouveaux de `db` et celui des défauts.

- [ ] **Step 7 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 8 : commit**

Message : `feat(core): ouvre le pool sea-orm depuis la configuration`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
