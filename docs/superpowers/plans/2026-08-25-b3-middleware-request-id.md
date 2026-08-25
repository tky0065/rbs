# Middleware `request_id` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** donner à chaque requête un identifiant de corrélation, repris de l'amont quand il en fournit un, lisible par tout le code de la requête et renvoyé au client.

**Architecture:** le stockage existe déjà (`request_id::current` / `scope`, posé en A4) ; il ne manquait que ce qui l'alimente. Un `axum::middleware::from_fn` suffit : la seule chose à faire est d'envelopper le futur dans `scope`, ce qu'une `tower::Layer` maison exprimerait avec le bruit d'un `Service` en plus. L'en-tête entrant est repris mais borné — cette valeur part dans chaque ligne de log et revient au client, et un `HeaderValue` peut porter des octets arbitraires. Le span `tracing` reste à B4 : l'ouvrir ici donnerait deux spans imbriqués par requête.

**Tech Stack:** `axum` (`middleware::from_fn`, `Next`), `ulid`, `tower` (dev).

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §5.2

## Global Constraints

- `#![warn(missing_docs)]` sur `rbs-core` : tout item public porte un `///` d'une à trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.
- Ne pas ouvrir de span ici — B4 en est responsable.

---

### Task 1 : middleware d'identifiant de requête

**Files:**
- Modify: `crates/rbs-core/src/request_id.rs` (constante, middleware, reprise bornée, + tests)
- Modify: `crates/rbs-core/Cargo.toml` (dépendance `ulid`)

**Interfaces:**
- Consumes: `request_id::scope` (A4).
- Produces: `rbs_core::request_id::X_REQUEST_ID`, `rbs_core::request_id::middleware`.

- [ ] **Step 1 : ajouter la dépendance `ulid`**

`crates/rbs-core/Cargo.toml` : `ulid.workspace = true`.

- [ ] **Step 2 : écrire les tests d'abord**

Dans `request_id.rs`, un routeur monté sur le middleware, interrogé par `oneshot` :

```rust
#[tokio::test]
async fn deux_requetes_recoivent_deux_identifiants_distincts() {
    let premier = appeler(None).await;
    let second = appeler(None).await;

    assert_ne!(premier, second);
    assert_eq!(premier.len(), 26, "un ULID fait 26 caractères : {premier}");
}

#[tokio::test]
async fn un_en_tete_entrant_est_conserve_tel_quel_dans_la_reponse() {
    let vu = appeler(Some("trace-amont-42")).await;

    assert_eq!(vu, "trace-amont-42");
}

#[tokio::test]
async fn un_en_tete_aberrant_est_ignore_au_profit_d_un_ulid_genere() {
    for aberrant in ["x".repeat(129), "avant\nAPRES".to_owned(), String::new()] {
        let vu = appeler(Some(&aberrant)).await;

        assert_ne!(vu, aberrant);
        assert_eq!(vu.len(), 26, "un ULID était attendu : {vu}");
    }
}

#[tokio::test]
async fn le_handler_lit_l_identifiant_de_sa_propre_requete() {
    // Le handler renvoie `current()` dans le corps ; il doit valoir l'en-tête renvoyé.
}
```

- [ ] **Step 3 : lancer les tests, les voir échouer**

Run: `cargo test -p rbs-core request_id`
Expected: échec de compilation, `middleware` inexistant.

- [ ] **Step 4 : implémenter**

- `pub const X_REQUEST_ID: HeaderName` via `HeaderName::from_static`.
- `fn reprendre(headers) -> Option<String>` : l'en-tête n'est repris que s'il est non vide, d'au plus `LONGUEUR_MAX` (128) caractères, et entièrement ASCII imprimable. Commenter le *pourquoi* : log injection et gonflement du journal.
- `pub async fn middleware(request: Request, next: Next) -> Response` : reprend ou génère un `Ulid`, appelle `scope(id.clone(), next.run(request))`, puis insère l'en-tête dans la réponse. Un identifiant repris est déjà validé ASCII imprimable, donc `HeaderValue::from_str` ne peut pas échouer — l'exprimer sans `unwrap` silencieux.

- [ ] **Step 5 : lancer les tests, les voir passer**

Run: `cargo test -p rbs-core request_id`
Expected: 6 passed (2 de A4 + 4 nouveaux).

- [ ] **Step 6 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 7 : commit**

Message : `feat(core): ajoute le middleware d'identifiant de requête`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
