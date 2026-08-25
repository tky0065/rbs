# Formateur de logs `json` et bascule — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** émettre un objet JSON par ligne en production, et choisir le formateur par `RBS_LOG_FORMAT` sans perdre le filtrage `RUST_LOG`.

**Architecture:** `JsonFormat` est le symétrique de `PrettyFormat` — un `FormatEvent` qui sert aussi de `FormatFields`. La feature `json` de `tracing-subscriber` n'est pas utilisable : elle impose les clés `timestamp` et `fields.message`, là où la spec demande `ts`, `level`, `msg` à plat. La bascule vit dans `logs::init()`, qui lit `RBS_LOG_FORMAT` et pose un `EnvFilter`. Une valeur inconnue fait échouer le démarrage, comme une configuration invalide en A5.

**Tech Stack:** `tracing-subscriber` (features `env-filter`, `json` non utilisée), `serde_json`, `ChronoUtc`.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §5.2

## Global Constraints

- `#![warn(missing_docs)]` sur `rbs-core` : tout item public porte un `///` d'une à trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.
- Dépendances déclarées dans `[workspace.dependencies]`, reprises par `.workspace = true`.
- Une erreur de démarrage ne devient jamais une réponse HTTP : elle a son propre type, hors de `Error` (précédent posé par `ConfigError` en A5).

---

### Task 1 : formateur `json` et bascule

**Files:**
- Create: `crates/rbs-core/src/logs/json.rs` (implémentation + `#[cfg(test)] mod tests`)
- Modify: `crates/rbs-core/src/logs/mod.rs` (`LogFormat`, `LogError`, `init`, + `mod tests`)
- Modify: `crates/rbs-core/Cargo.toml` (feature `env-filter`)

**Interfaces:**
- Consumes: `PrettyFormat::new()` de la tâche A6, qui sert à la fois de `FormatEvent` et de `FormatFields`.
- Produces: `rbs_core::logs::JsonFormat` (mêmes constructeurs implicites : `JsonFormat::new()`), `rbs_core::logs::LogFormat` (`Pretty` par défaut, `Json`, `FromStr<Err = LogError>`), `rbs_core::logs::LogError`, et `rbs_core::logs::init() -> Result<(), LogError>`.

- [ ] **Step 1 : activer `env-filter`**

`crates/rbs-core/Cargo.toml` : `tracing-subscriber = { workspace = true, features = ["fmt", "ansi", "chrono", "env-filter"] }`.

- [ ] **Step 2 : écrire les tests d'abord**

Dans `json.rs`, un `mod tests` réutilisant le montage de `pretty.rs` (`Tampon`, `capture`) et couvrant :

```rust
#[test]
fn chaque_ligne_est_un_json_valide_portant_ts_level_et_msg() {
    let sortie = capture(|| {
        tracing::info!("serveur démarré");
        tracing::warn!(actives = 18, "pool proche de la saturation");
        tracing::error!("requête refusée");
    });

    let lignes: Vec<&str> = sortie.lines().collect();
    assert_eq!(lignes.len(), 3, "trois lignes attendues : {sortie:?}");
    for ligne in lignes {
        let objet: serde_json::Value =
            serde_json::from_str(ligne).unwrap_or_else(|e| panic!("ligne non JSON ({e}) : {ligne}"));
        for cle in ["ts", "level", "msg"] {
            assert!(objet.get(cle).is_some(), "clé {cle} absente : {ligne}");
        }
    }
}

#[test]
fn les_champs_conservent_leur_type_json() {
    let sortie = capture(|| tracing::error!(status = 422, latency_ms = 12.4, actif = true, "refus"));

    let objet: serde_json::Value = serde_json::from_str(sortie.trim()).expect("ligne non JSON");
    assert_eq!(objet["status"], serde_json::json!(422));
    assert_eq!(objet["latency_ms"], serde_json::json!(12.4));
    assert_eq!(objet["actif"], serde_json::json!(true));
    assert_eq!(objet["msg"], serde_json::json!("refus"));
}

#[test]
fn les_champs_d_un_span_parent_remontent_dans_l_objet() {
    let sortie = capture(|| {
        let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
        let _entree = span.enter();
        tracing::error!(status = 422, "requête refusée");
    });

    let objet: serde_json::Value = serde_json::from_str(sortie.trim()).expect("ligne non JSON");
    assert_eq!(objet["request_id"], serde_json::json!("01JQ3F8K2P"));
    assert_eq!(objet["status"], serde_json::json!(422));
}
```

Dans `mod.rs`, un `mod tests` couvrant la bascule :

```rust
#[test]
fn le_format_se_lit_depuis_son_nom() {
    assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
    assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
    assert_eq!(" JSON ".parse::<LogFormat>().unwrap(), LogFormat::Json);
    assert_eq!(LogFormat::default(), LogFormat::Pretty);
}

#[test]
fn un_format_inconnu_est_refuse_en_nommant_la_variable_et_les_valeurs_admises() {
    let erreur = "texte".parse::<LogFormat>().unwrap_err().to_string();

    assert!(erreur.contains("RBS_LOG_FORMAT"), "variable non nommée : {erreur}");
    assert!(erreur.contains("texte"), "valeur fautive absente : {erreur}");
    assert!(erreur.contains("pretty") && erreur.contains("json"), "valeurs admises absentes : {erreur}");
}
```

- [ ] **Step 3 : lancer les tests, les voir échouer**

Run: `cargo test -p rbs-core logs`
Expected: échec de compilation, `JsonFormat` et `LogFormat` inexistants.

- [ ] **Step 4 : implémenter `JsonFormat`**

Points structurants de `json.rs` :

- `pub struct JsonFormat { horodatage: ChronoUtc }`, `ChronoUtc::new("%Y-%m-%dT%H:%M:%S%.3fZ".to_owned())`, plus `new()` et `Default`.
- Un visiteur `ChampsJson { message: String, champs: Map<String, Value> }` implémentant `record_bool`, `record_i64`, `record_u64`, `record_f64`, `record_str` et `record_debug` : les types JSON sont préservés, `message` est routé à part.
- `format_event` construit une `Map` — `ts`, `level`, `target`, `msg`, puis les champs de l'événement — et y fusionne ceux des spans parents avant d'écrire l'objet suivi d'un saut de ligne.
- `impl FormatFields for JsonFormat` sérialise les champs d'un span en objet JSON complet ; `format_event` le relit avec `serde_json::from_str` pour le fusionner. Le registry ne conserve les champs d'un span que sous forme de texte déjà formaté : c'est le seul point d'accès sans écrire une `Layer` maison. Commenter ce pourquoi.
- L'ordre des clés est celui de `serde_json::Map` (alphabétique) : déterministe, et sans conséquence pour un agrégateur de logs.

- [ ] **Step 5 : implémenter la bascule dans `mod.rs`**

- `pub enum LogError` (thiserror) : `FormatInconnu(String)` — message nommant `RBS_LOG_FORMAT`, la valeur fautive et les deux valeurs admises — et `DejaInitialise(#[from] tracing::subscriber::SetGlobalDefaultError)`. Documenter qu'elle est distincte d'`Error` pour la même raison que `ConfigError`.
- `pub enum LogFormat { #[default] Pretty, Json }` avec `FromStr` insensible à la casse et tolérant aux espaces.
- `pub fn init() -> Result<(), LogError>` : lit `RBS_LOG_FORMAT` (absente → défaut, valeur non-unicode → `FormatInconnu` via `to_string_lossy`), construit `EnvFilter::try_from_default_env()` avec repli sur `info`, puis pose le subscriber correspondant par `tracing::subscriber::set_global_default`.

- [ ] **Step 6 : lancer les tests, les voir passer**

Run: `cargo test -p rbs-core logs`
Expected: 10 passed (5 de A6 + 5 nouveaux).

- [ ] **Step 7 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

Run: `RBS_LOG_FORMAT=json cargo run -p rbs-core --example logs_pretty` après avoir fait passer l'exemple par `logs::init()`
Expected: cinq objets JSON, un par ligne.

- [ ] **Step 8 : commit**

Message : `feat(core): ajoute le format de logs json et la bascule par variable d'environnement`, corps portant le *pourquoi* et un intertitre `Vérifications :`.
