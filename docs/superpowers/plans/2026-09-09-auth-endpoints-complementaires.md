# Huit routes de plus au fragment `auth` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Porter le fragment `auth` de cinq à treize routes — changement de mot de passe, réinitialisation, vérification d'adresse, gestion des sessions — sans changer le contrat d'aucune route existante.

**Architecture:** Tout se passe dans `crates/rbs-cli/templates/features/auth/`, un jeu de gabarits minijinja que `rbs add auth` rend dans le projet de l'utilisateur. Une table `one_time_tokens` à usage unique sert à la fois la réinitialisation et la vérification, avec une consommation atomique en un seul `UPDATE`. Le fragment `mail` devient une dépendance dure. `src/auth/` passe d'un fichier par couche à un répertoire par couche, la dépendance `controller → service → repository → model` restant inchangée.

**Tech Stack:** Rust, axum, SeaORM, utoipa, minijinja (délimiteurs alternatifs), `lettre` via le fragment `mail`, `assert_cmd` pour les tests d'intégration du CLI, `testcontainers` + PostgreSQL pour la suite lente.

**Spec:** `docs/superpowers/specs/2026-09-09-auth-endpoints-design.md`

## Global Constraints

Ces règles valent pour **toutes** les tâches. Aucune n'est rappelée dans les tâches.

- **Commits** : Conventional Commits, sujet en français, à l'impératif, sans majuscule initiale ni point final. **Aucun identifiant de tâche**, aucun renvoi à ce plan, à la spec, à `IMPROVE.md`, `TODO.md` ou `ROADMAP.md`. **Jamais de ligne `Co-Authored-By` ni `Claude-Session`.** Le corps porte le *pourquoi* technique, puis un intertitre `Vérifications :` avec les commandes lancées et leur résultat réel.
- **Branche** : `feat/auth-endpoints-complementaires`, déjà créée. Jamais de commit sur `main`.
- **Bloquant en CI, à faire passer avant chaque commit** : `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`.
- **Toolchain** : `rustup update` avant toute passe de vérification qui décide d'un commit final — clippy vert en local ne dit rien de la version que prend `@stable` en CI.
- **Commentaires** : un commentaire explique le *pourquoi*, jamais le *quoi*. Un commentaire qui paraphrase la ligne suivante se supprime. Le code engendré ne commente que ses points d'extension ; pas de bandeau « généré, ne pas modifier ».
- **Taille** : un fichier de feature au-delà de ~200 lignes signale une feature à scinder.
- **Documentation bilingue** : toute page modifiée sous `docs/docs/` l'est aussi sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/`, **dans le même commit**.
- **Délimiteurs minijinja** : les gabarits emploient `{@ variable @}` et `{% bloc %}` — Jinja et `format!` se disputent `{{ }}`. Attention aux blancs : `-%}` mange l'indentation, et un blanc perdu n'est vu que par `integration_examples`.
- **Régénération des exemples** : `examples/blog-auth` se régénère **par diff entre deux générations, jamais par écrasement** — il porte une édition à la main, `src/posts/tests.rs`, que `integration_examples.rs` déclare. La procédure exacte est en Annexe A.
- **Suite lente** : `cargo test -p rbs-cli --no-fail-fast -- --ignored`. `--no-fail-fast` est un drapeau de **cargo** et se place **avant** le `--` : posé après, il part au harnais de test, qui refuse `Unrecognized option: 'no-fail-fast'` et fait échouer la suite entière sans en lancer un seul test. Sans lui, la suite s'arrête au premier binaire et masque les échecs suivants. Rediriger la sortie vers le scratchpad : une sortie longue en arrière-plan est rognée et les chiffres se perdent.
- **Avant la passe lente** : `cargo check` sur `examples/blog-auth` régénéré. Le code des fragments n'est compilé nulle part ailleurs, et une erreur de compilation découverte après vingt minutes de Docker est vingt minutes perdues.
- **Chemins des handlers dans l'ancre `openapi`** : les cinq routes d'origine y sont nommées sans qualification (`crate::auth::controller::login`) et `controller/mod.rs` porte un `pub use session::*;` qui les y amène — le glob est nécessaire parce que `#[utoipa::path]` dépose un type `__path_<handler>` caché qu'un réexport nommé ne suivrait pas, et parce qu'`integration_auth` assère ces chemins tels quels. **Les modules de contrôleur ajoutés par ce plan — `password`, `verification` — sont au contraire nommés qualifiés** (`crate::auth::controller::password::change_password`) et ne reçoivent **aucun** `pub use ...::*;`. Un glob par module verserait treize noms de handlers et autant de types cachés dans un seul espace de noms, où la première collision entre deux modules se résoudrait par un masquage silencieux. Le chemin qualifié dit où le handler vit.
- **Signatures des aides de `tests/mod.rs`**, telles qu'elles existent réellement — les tests des tâches suivantes s'y conforment : `register(api, email) -> (StatusCode, Value)` (le corps est le **second** membre), `authenticate(api, email, mot_de_passe) -> (StatusCode, Value)`, `login(api, email, mot_de_passe) -> Value` (assère 200 et rend la paire), `call(api, requete) -> (StatusCode, Value)`, `registered_user(db) -> repository::Model`, plus `post_json`, `post_json_authenticated`, `without_body` et `fresh_email`.
- **Littéraux de fragments** : toute chaîne déposée par un gabarit est figée des deux côtés de la frontière Docker. `grep -rn "<la chaîne>" crates/rbs-cli/tests/` avant de conclure qu'un changement de texte est sans conséquence.

---

## Structure des fichiers

**Créés dans le fragment** (`crates/rbs-cli/templates/features/auth/`) :

| Fichier | Responsabilité |
|---|---|
| `config.rs.jinja` | `FlowConfig` : `reset_ttl_secs`, `verification_ttl_secs`, `app_url`, lue par `rbs_core::config::section` |
| `repository/mod.rs.jinja` | réexporte `Model`, `ADRESSE_PRISE`, et les trois sous-modules |
| `repository/user.rs.jinja` | `find`, `find_by_email`, `create`, `set_password`, `mark_verified` |
| `repository/refresh_token.rs.jinja` | `create_refresh_token`, `find_refresh_token`, `consume`, `revoke_sessions_of`, `open_sessions_of`, `revoke_session` |
| `repository/one_time_token.rs.jinja` | `issue`, `find`, `consume`, `invalidate_pending`, `purge_expired` |
| `service/mod.rs.jinja` | `issue()`, `profile()`, partagés ; réexporte les trois sous-modules |
| `service/session.rs.jinja` | `register`, `login`, `refresh`, `logout`, `me`, `sessions`, `revoke_session`, `revoke_all_sessions` |
| `service/password.rs.jinja` | `change`, `request_reset`, `reset` |
| `service/verification.rs.jinja` | `request`, `resend`, `verify` |
| `controller/mod.rs.jinja` | réexporte les trois sous-modules |
| `controller/session.rs.jinja` | `register`, `login`, `refresh`, `logout`, `me`, `sessions`, `revoke_session`, `revoke_all_sessions` |
| `controller/password.rs.jinja` | `change_password`, `forgot_password`, `reset_password` |
| `controller/verification.rs.jinja` | `verify_email`, `resend_verification` |
| `tests/mod.rs.jinja` | harnais partagé : `application()`, `connection()`, `call()`, `post_json()`, `fresh_email()`, `register()`, `login()` |
| `tests/session.rs.jinja` | les tests existants du parcours, plus les trois routes de session |
| `tests/password.rs.jinja` | changement, oubli, réinitialisation |
| `tests/verification.rs.jinja` | vérification, renvoi, garde |
| `reinitialisation.html.jinja` | gabarit de courriel, déposé dans `templates/mail/` du projet |
| `verification.html.jinja` | idem |

**Modifiés :** `feature.toml`, `model.rs.jinja`, `dto.rs.jinja`, `migration.rs.jinja`, `guard.rs.jinja`, `mod.rs.jinja` du fragment `auth` ; `service.rs.jinja` du fragment `mail` ; `feature.toml` du fragment `rate-limit` ; `crates/rbs-cli/tests/integration_auth.rs` et `integration_webhooks.rs` ; `examples/blog-auth/**` ; six pages de `docs/`.

**Supprimés :** `repository.rs.jinja`, `service.rs.jinja`, `controller.rs.jinja`, `tests.rs.jinja` du fragment `auth`, remplacés par les répertoires ci-dessus.

---

## Lot 1 — Le socle

### Task 1 : Découper `src/auth/` en un répertoire par couche

Refactorisation pure : à la fin de cette tâche le fragment fait exactement ce qu'il faisait, sur une structure qui porte la suite. Aucune route, aucune fonction, aucun test n'est ajouté ni retiré.

**Files:**
- Create: `crates/rbs-cli/templates/features/auth/repository/{mod,user,refresh_token}.rs.jinja`
- Create: `crates/rbs-cli/templates/features/auth/service/{mod,session}.rs.jinja`
- Create: `crates/rbs-cli/templates/features/auth/controller/{mod,session}.rs.jinja`
- Create: `crates/rbs-cli/templates/features/auth/tests/{mod,session}.rs.jinja`
- Delete: `crates/rbs-cli/templates/features/auth/{repository,service,controller,tests}.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/auth/feature.toml` (section `[[files]]`)
- Modify: `crates/rbs-cli/templates/features/auth/mod.rs.jinja` (rien à changer si `pub mod controller;` reste juste — vérifier)
- Test: `crates/rbs-cli/tests/integration_auth.rs`

**Interfaces:**
- Consumes: rien.
- Produces: les chemins de fichiers ci-dessus, et les réexports que les tâches suivantes utilisent — `repository::{Model, ADRESSE_PRISE, find, find_by_email, create, create_refresh_token, find_refresh_token, consume, revoke_sessions_of}` restent accessibles sous ces noms exacts depuis `super::repository`, et `service::{issue, profile}` depuis `super::service`.

- [ ] **Step 1 : Écrire le test qui exige la nouvelle arborescence**

Ajouter à `crates/rbs-cli/tests/integration_auth.rs` :

```rust
/// La découpe par couche est ce qui rend le fragment lisible à treize routes : chaque
/// couche est un répertoire, et le sens de la dépendance ne change pas.
#[test]
fn each_layer_is_a_directory() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth(&parent);

    for fichier in [
        "src/auth/repository/mod.rs",
        "src/auth/repository/user.rs",
        "src/auth/repository/refresh_token.rs",
        "src/auth/service/mod.rs",
        "src/auth/service/session.rs",
        "src/auth/controller/mod.rs",
        "src/auth/controller/session.rs",
        "src/auth/tests/mod.rs",
        "src/auth/tests/session.rs",
    ] {
        assert!(
            racine.join(fichier).is_file(),
            "{fichier} n'a pas été déposé"
        );
    }

    for ancien in [
        "src/auth/repository.rs",
        "src/auth/service.rs",
        "src/auth/controller.rs",
        "src/auth/tests.rs",
    ] {
        assert!(
            !racine.join(ancien).exists(),
            "{ancien} survit à la découpe"
        );
    }
}
```

- [ ] **Step 2 : Lancer le test pour le voir échouer**

```bash
cargo test -p rbs-cli --test integration_auth -- --exact each_layer_is_a_directory
```

Attendu : FAIL, `src/auth/repository/mod.rs n'a pas été déposé`.

- [ ] **Step 3 : Éclater `repository.rs.jinja`**

`repository/user.rs.jinja` reçoit `find`, `find_by_email`, `ADRESSE_PRISE`, `create`, et `pub use super::super::model::user::Model;`. `repository/refresh_token.rs.jinja` reçoit `create_refresh_token`, `find_refresh_token`, `consume`, `revoke_sessions_of`. Les corps de fonction sont **repris tels quels**, commentaires compris — c'est une découpe, pas une réécriture.

`repository/mod.rs.jinja` :

```rust
//! La couche qui parle à la base, un fichier par table.
//!
//! Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
//! base reste la seule à connaître l'entité.

pub mod refresh_token;
pub mod user;

pub use refresh_token::{consume, create_refresh_token, find_refresh_token, revoke_sessions_of};
pub use user::{ADRESSE_PRISE, Model, create, find, find_by_email};
```

Les réexports existent pour que `service.rs` n'ait pas une ligne à changer : la découpe ne doit pas se voir depuis la couche du dessus.

- [ ] **Step 4 : Éclater `service.rs.jinja`**

`service/session.rs.jinja` reçoit `register`, `login`, `refresh`, `logout`, `me` et la constante `HASH_DE_COMPARAISON`. `service/mod.rs.jinja` garde `issue()` et `profile()` — les seules fonctions que les trois futurs sous-modules partagent — et réexporte :

```rust
//! La couche qui porte les règles, un fichier par parcours.
//!
//! `issue()` et `profile()` vivent ici plutôt que dans l'un des parcours : les trois s'en
//! servent, et les descendre dans l'un d'eux ferait dépendre les deux autres de ce
//! voisin-là.

pub mod session;

pub use session::{login, logout, me, refresh, register};

// ... issue() et profile(), repris tels quels
```

`issue` et `profile` passent de `async fn` / `fn` privées à `pub(super)`. `profile` est appelé par `session.rs` : `use super::profile;`.

- [ ] **Step 5 : Éclater `controller.rs.jinja` et `tests.rs.jinja`**

`controller/session.rs.jinja` reçoit les cinq handlers avec leurs `#[utoipa::path]`, inchangés. `controller/mod.rs.jinja` :

```rust
pub mod session;

pub use session::{login, logout, me, refresh, register};
```

Les réexports comptent : l'ancre `openapi` du manifeste nomme `crate::auth::controller::login`, et `integration_auth.rs` l'assère. Rien de ce que le projet montre au monde ne change de chemin.

`tests/mod.rs.jinja` reçoit tout ce qui est partagé — `PASSWORD`, `application()`, `connection()`, `call()`, `without_body()`, `post_json()`, `fresh_email()` et les aides d'inscription/connexion — et déclare `mod session;`. Ces aides passent en `pub(super)` ou `pub(crate)` selon ce que `cargo check` réclame. `tests/session.rs.jinja` reçoit les tests, avec `use super::*;` en tête.

- [ ] **Step 6 : Mettre le manifeste à jour**

Dans `feature.toml`, remplacer les quatre entrées `[[files]]` supprimées par les neuf nouvelles, chacune sur le modèle :

```toml
[[files]]
source      = "repository/mod.rs.jinja"
destination = "src/auth/repository/mod.rs"
```

- [ ] **Step 7 : Régénérer `examples/blog-auth` et compiler**

Suivre l'Annexe A, puis :

```bash
cargo check --manifest-path examples/blog-auth/Cargo.toml
```

Attendu : compile sans erreur ni avertissement.

- [ ] **Step 8 : Lancer la suite rapide du CLI**

```bash
cargo test -p rbs-cli --test integration_auth
cargo test -p rbs-cli --test integration_examples
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Attendu : tout au vert, `each_layer_is_a_directory` compris.

- [ ] **Step 9 : Commit**

```bash
git add -A
git commit -F - <<'EOF'
refactor(auth): fait de chaque couche du fragment un répertoire

À treize routes, `service.rs` dépasserait 350 lignes et `controller.rs` 280 — bien
au-delà du seuil où un fichier de feature cesse de se lire d'un coup. La découpe
prend les devants sans rien changer d'autre : mêmes fonctions, mêmes corps, mêmes
commentaires, et des réexports dans chaque `mod.rs` pour que la couche du dessus
n'ait pas une ligne à changer.

Le sens de la dépendance ne bouge pas : controller → service → repository → model.
C'est le nombre de fichiers par couche qui change, pas leur rôle.

Vérifications :
- cargo test -p rbs-cli --test integration_auth : N passés, 0 échec
- cargo test -p rbs-cli --test integration_examples : N passés, 0 échec
- cargo check sur examples/blog-auth régénéré : succès
- clippy -D warnings et fmt --check : propres
EOF
```

---

### Task 2 : `mail` requis, et les trois clés de configuration

**Files:**
- Modify: `crates/rbs-cli/templates/features/auth/feature.toml`
- Create: `crates/rbs-cli/templates/features/auth/config.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/auth/mod.rs.jinja`
- Modify: `crates/rbs-cli/tests/integration_auth.rs`, `crates/rbs-cli/tests/integration_webhooks.rs`
- Test: `crates/rbs-cli/tests/integration_auth.rs`

**Interfaces:**
- Consumes: la structure de la Task 1.
- Produces: `crate::auth::config::FlowConfig { reset_ttl_secs: u64, verification_ttl_secs: u64, app_url: String }`, et l'accesseur `AppState::flows(&self) -> &FlowConfig`. Les tâches 6 à 10 le lisent depuis le contrôleur, jamais depuis le service — le service reçoit les valeurs qu'il lui faut.

- [ ] **Step 1 : Écrire le test qui exige `mail` et les trois clés**

```rust
/// `auth` envoie deux courriels — réinitialisation et vérification. Sans `mail`, les
/// deux parcours s'arrêteraient à la moitié de ce que le fragment promet.
#[test]
fn adding_auth_installs_mail() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth(&parent);

    assert!(
        racine.join("src/modules/mail/mod.rs").is_file(),
        "le fragment mail n'a pas suivi"
    );

    let defaut = fs::read_to_string(racine.join("config/default.toml"))
        .expect("config/default.toml lisible");

    for cle in ["reset_ttl_secs", "verification_ttl_secs", "app_url"] {
        assert!(
            defaut.contains(cle),
            "config/default.toml ne porte pas `{cle}` :\n{defaut}"
        );
    }
}
```

- [ ] **Step 2 : Lancer le test pour le voir échouer**

```bash
cargo test -p rbs-cli --test integration_auth -- --exact adding_auth_installs_mail
```

Attendu : FAIL, `le fragment mail n'a pas suivi`.

- [ ] **Step 3 : Déclarer la dépendance et les clés**

Dans `feature.toml` :

```toml
# `auth` envoie deux courriels — le lien de réinitialisation et celui de vérification — et
# aucun des deux parcours n'a de sens sans destinataire. La dépendance est dure plutôt
# qu'optionnelle : un point d'extension resté vide rendrait 202 sans que rien ne parte.
requires = ["rate-limit", "mail"]
```

et, dans la section `[[config]]` existante :

```toml
[[config]]
file    = "config/default.toml"
section = "auth"
content = """
access_ttl_secs = 900
refresh_ttl_secs = 2592000

# Une heure : le lien de réinitialisation est urgent, et sa fenêtre est la durée pendant
# laquelle une boîte compromise vaut le compte.
reset_ttl_secs = 3600

# Un jour : la vérification ne l'est pas, et un lien trop court fait revenir l'utilisateur
# sur `resend-verification` plus souvent qu'il n'ouvre son courrier.
verification_ttl_secs = 86400

# L'application du client, et non ce serveur : rbs engendre une API, et le lien du
# courriel mène à l'écran qui postera le jeton.
app_url = "http://localhost:3000"
"""
```

- [ ] **Step 4 : Déposer `config.rs.jinja`**

```rust
use serde::Deserialize;

/// Ce que les parcours de réinitialisation et de vérification lisent dans `[auth]`.
///
/// Ces trois clés vivent ici et non dans `rbs_core::config::AuthConfig`, qui les
/// ignorerait : le noyau porte ce qui ne varie pas d'un projet à l'autre, et l'adresse de
/// votre application n'entre pas dans cette catégorie. C'est donc ce fichier que vous
/// ouvrirez pour changer les durées ou l'URL des liens.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct FlowConfig {
    /// Durée de vie du lien de réinitialisation, en secondes.
    pub reset_ttl_secs: u64,
    /// Durée de vie du lien de vérification, en secondes.
    pub verification_ttl_secs: u64,
    /// Racine des liens envoyés par courriel, sans barre finale.
    pub app_url: String,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            reset_ttl_secs: 3600,
            verification_ttl_secs: 86_400,
            app_url: "http://localhost:3000".to_string(),
        }
    }
}

impl FlowConfig {
    /// Lit la section `[auth]`, dont elle ne retient que ses trois clés.
    pub fn from_config() -> anyhow::Result<Self> {
        Ok(rbs_core::config::section::<Self>("auth")?)
    }

    /// Le lien à mettre dans un courriel, `app_url` et la barre finale réconciliées.
    ///
    /// La barre est retirée plutôt que supposée absente : `http://exemple.test/` dans un
    /// fichier de configuration est aussi naturel que sans, et donnerait sinon un lien à
    /// double barre que certains clients de messagerie coupent.
    pub fn link(&self, path: &str, token: &str) -> String {
        format!("{}/{path}?token={token}", self.app_url.trim_end_matches('/'))
    }
}
```

- [ ] **Step 5 : Câbler l'état**

Dans `feature.toml`, deux ancres de plus :

```toml
[[anchors]]
anchor  = "state_champs"
content = "pub flows: crate::auth::config::FlowConfig,"

[[anchors]]
anchor  = "state_init"
content = "flows: crate::auth::config::FlowConfig::from_config()?,"
```

et dans `mod.rs.jinja`, à côté de l'`impl HasAuth` qui y vit déjà pour la même raison :

```rust
pub mod config;

// L'accesseur vit ici et non dans `state.rs` : il arrive avec la feature, et repart avec
// elle.
impl AppState {
    /// Les réglages des parcours de réinitialisation et de vérification.
    pub fn flows(&self) -> &config::FlowConfig {
        &self.flows
    }
}
```

- [ ] **Step 6 : Rattraper le test de la chaîne transitive**

`crates/rbs-cli/tests/integration_webhooks.rs:85` décrit la chaîne `webhooks → jobs, auth → rate-limit`. Elle passe par `mail`. Lire le test, mettre l'assertion et son commentaire à jour.

- [ ] **Step 7 : Régénérer, compiler, lancer**

```bash
# Annexe A, puis :
cargo check --manifest-path examples/blog-auth/Cargo.toml
cargo test -p rbs-cli --test integration_auth
cargo test -p rbs-cli --test integration_webhooks
cargo test -p rbs-cli --test integration_examples
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```

`blog-auth` gagne ici `lettre`, la section `[mail]`, Mailpit dans `docker-compose.yml`, `templates/mail/bienvenue.html` et `RBS_MAIL__SMTP_PASSWORD` dans `.env.example`. C'est attendu : le vérifier dans le diff plutôt que de le subir.

- [ ] **Step 8 : Commit**

```bash
git add -A
git commit -F - <<'EOF'
feat(auth): fait de mail une dépendance du fragment et ajoute les réglages des liens

Les deux parcours qui arrivent — réinitialisation et vérification — envoient un
courriel, et aucun n'a de sens sans destinataire. La dépendance est dure plutôt
qu'optionnelle : un point d'extension laissé vide rendrait 202 sans que rien ne
parte, ce qui est la pire des deux façons d'échouer.

Les trois clés se lisent dans le projet et non dans le noyau. `AuthConfig` est
`#[non_exhaustive]` et les ignorerait ; `FlowConfig` les prend par
`config::section("auth")`, comme `MailConfig` le fait déjà pour `[mail]`. La durée
d'un lien et l'adresse d'une application sont des réglages de produit, pas de
runtime : ils appartiennent au projet.

Vérifications :
- cargo test -p rbs-cli --test integration_auth : N passés, 0 échec
- cargo test -p rbs-cli --test integration_webhooks : N passés, 0 échec
- cargo check sur examples/blog-auth régénéré : succès
- clippy -D warnings et fmt --check : propres
EOF
```

---

### Task 3 : La table `one_time_tokens` et la colonne `email_verified_at`

**Files:**
- Modify: `crates/rbs-cli/templates/features/auth/migration.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/auth/model.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/auth/dto.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/auth/service/mod.rs.jinja` (`profile`)
- Test: `crates/rbs-cli/tests/integration_auth.rs`

**Interfaces:**
- Consumes: la structure de la Task 1.
- Produces: `model::TokenPurpose::{PasswordReset, EmailVerification}` ; `model::one_time_token::{Model, Entity, Column, ActiveModel}` avec les champs `id, user_id, token_hash, purpose, expires_at, consumed_at, created_at, updated_at` ; `model::user::Model.email_verified_at: Option<DateTimeWithTimeZone>` ; `dto::UserResponse.email_verified_at: Option<DateTimeWithTimeZone>`.

- [ ] **Step 1 : Écrire le test**

```rust
/// Une table et une colonne de plus dans la migration qui crée les tables : un projet
/// déjà engendré n'a rien à rattraper, elle n'altère toujours rien.
#[test]
fn the_migration_creates_the_one_time_tokens_table() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth(&parent);

    let migrations: Vec<_> = fs::read_dir(racine.join("migration/src"))
        .expect("la crate migration existe")
        .filter_map(|entree| entree.ok())
        .map(|entree| entree.file_name().to_string_lossy().to_string())
        .filter(|nom| nom.contains("create_auth_tables"))
        .collect();

    let source = fs::read_to_string(
        racine
            .join("migration/src")
            .join(migrations.first().expect("la migration du fragment existe")),
    )
    .expect("migration lisible");

    for attendu in [
        "OneTimeTokens::Table",
        "OneTimeTokens::Purpose",
        "OneTimeTokens::ConsumedAt",
        "Users::EmailVerifiedAt",
        "idx_one_time_tokens_token_hash",
    ] {
        assert!(
            source.contains(attendu),
            "la migration ne porte pas `{attendu}` :\n{source}"
        );
    }
}
```

- [ ] **Step 2 : Lancer le test pour le voir échouer**

```bash
cargo test -p rbs-cli --test integration_auth -- --exact the_migration_creates_the_one_time_tokens_table
```

Attendu : FAIL, `la migration ne porte pas OneTimeTokens::Table`.

- [ ] **Step 3 : Écrire la migration**

Dans `migration.rs.jinja`, ajouter une colonne au `create_table` de `Users`, après `Role` :

```rust
                    // Nulle tant que l'adresse n'est pas prouvée. La date et non un
                    // booléen : savoir *quand* une adresse a été vérifiée est ce qui
                    // permet, un jour, de redemander une preuve aux plus anciennes.
                    .col(
                        ColumnDef::new(Users::EmailVerifiedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
```

puis, après la table `RefreshTokens` et avant son index, une troisième table :

```rust
        manager
            .create_table(
                Table::create()
                    .table(OneTimeTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OneTimeTokens::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(OneTimeTokens::UserId).uuid().not_null())
                    // L'empreinte, jamais le jeton : la même règle que pour les jetons de
                    // rafraîchissement, et pour la même raison.
                    .col(ColumnDef::new(OneTimeTokens::TokenHash).string().not_null())
                    // L'usage fait partie de la recherche : sans lui, un jeton de
                    // vérification vaudrait comme jeton de réinitialisation.
                    .col(ColumnDef::new(OneTimeTokens::Purpose).string().not_null())
                    .col(
                        ColumnDef::new(OneTimeTokens::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    // Nul tant que le jeton vit. La ligne survit à son usage : c'est elle
                    // qui distingue un jeton déjà joué d'un jeton jamais émis.
                    .col(
                        ColumnDef::new(OneTimeTokens::ConsumedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(OneTimeTokens::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(OneTimeTokens::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_one_time_tokens_user_id")
                            .from(OneTimeTokens::Table, OneTimeTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_one_time_tokens_token_hash")
                    .table(OneTimeTokens::Table)
                    .col(OneTimeTokens::TokenHash)
                    .to_owned(),
            )
            .await?;
```

Le `down` supprime `OneTimeTokens` **avant** `Users`, comme `RefreshTokens` : la table qui porte la clé étrangère part la première. Et l'énumération d'identifiants :

```rust
#[derive(DeriveIden)]
enum OneTimeTokens {
    Table,
    Id,
    UserId,
    TokenHash,
    Purpose,
    ExpiresAt,
    ConsumedAt,
    CreatedAt,
    UpdatedAt,
}
```

plus `EmailVerifiedAt` dans `enum Users`.

- [ ] **Step 4 : Écrire le modèle**

Dans `model.rs.jinja`, après `Role` :

```rust
/// Ce à quoi un jeton à usage unique donne droit.
///
/// Stocké en texte comme [`Role`] : un troisième usage s'ajoute ici, sans migration.
/// Aucun ordre n'est porté par cette énumération — les usages ne se contiennent pas, et
/// l'ordre de déclaration n'y a donc pas la portée qu'il a sur les rôles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum TokenPurpose {
    #[sea_orm(string_value = "password_reset")]
    PasswordReset,
    #[sea_orm(string_value = "email_verification")]
    EmailVerification,
}
```

`email_verified_at: Option<DateTimeWithTimeZone>` s'ajoute à `user::Model`, après `role`. Et le module `one_time_token`, calqué sur `refresh_token` — même `ActiveModelBehavior::new` posant `Uuid::now_v7()`, mêmes ancres `<rbs:relations:one_time_tokens>` et `<rbs:related:one_time_tokens>` :

```rust
pub mod one_time_token {
    use sea_orm::ActiveValue::Set;
    use sea_orm::entity::prelude::*;

    use super::TokenPurpose;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "one_time_tokens")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub user_id: Uuid,
        #[sea_orm(indexed)]
        pub token_hash: String,
        pub purpose: TokenPurpose,
        pub expires_at: DateTimeWithTimeZone,
        pub consumed_at: Option<DateTimeWithTimeZone>,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        // <rbs:relations:one_time_tokens>
        // </rbs:relations:one_time_tokens>
    }

    // <rbs:related:one_time_tokens>
    // </rbs:related:one_time_tokens>

    /// L'identifiant est posé ici, et non par un défaut de colonne : `uuidv7()` n'a
    /// d'équivalent à écrire ni en MySQL ni en SQLite.
    impl ActiveModelBehavior for ActiveModel {
        fn new() -> Self {
            Self {
                id: Set(Uuid::now_v7()),
                ..ActiveModelTrait::default()
            }
        }
    }
}
```

`model.rs` approche alors les ~180 lignes : sous le seuil, mais c'est le prochain fichier à surveiller.

- [ ] **Step 5 : Remonter l'état par `UserResponse`**

Dans `dto.rs.jinja` :

```rust
    /// Nul tant que l'adresse n'est pas prouvée.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub email_verified_at: Option<DateTimeWithTimeZone>,
```

et dans `service/mod.rs.jinja`, la fonction `profile` recopie le champ. C'est ce qui fait que `GET /auth/me` porte l'information sans qu'aucune route ne s'ajoute.

- [ ] **Step 6 : Régénérer, compiler, lancer**

```bash
# Annexe A, puis :
cargo check --manifest-path examples/blog-auth/Cargo.toml
cargo test -p rbs-cli --test integration_auth
cargo test -p rbs-cli --test integration_examples
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```

- [ ] **Step 7 : Commit**

```bash
git add -A
git commit -F - <<'EOF'
feat(auth): ajoute la table des jetons à usage unique et l'état de vérification

Une seule table pour la réinitialisation et la vérification, distinguées par une
colonne `purpose` : la mécanique de consommation atomique est la même des deux
côtés, et l'écrire deux fois serait deux fois l'occasion de se tromper.

`consumed_at` et non `revoked_at`, qui nomme la colonne homologue des jetons de
rafraîchissement : ce n'est pas le même verbe. Un jeton de rafraîchissement est
retiré ; celui-ci s'épuise en servant.

La migration crée les tables et n'en altère toujours aucune — un projet déjà
engendré et migré n'a rien à rattraper.

Vérifications :
- cargo test -p rbs-cli --test integration_auth : N passés, 0 échec
- cargo check sur examples/blog-auth régénéré : succès
- clippy -D warnings et fmt --check : propres
EOF
```

---

### Task 4 : Le repository des jetons à usage unique

**Files:**
- Create: `crates/rbs-cli/templates/features/auth/repository/one_time_token.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/auth/repository/mod.rs.jinja`, `feature.toml`
- Create: la section de tests dans `crates/rbs-cli/templates/features/auth/tests/password.rs.jinja` (le fichier naît ici, avec le seul test de concurrence)
- Test: `crates/rbs-cli/tests/integration_auth.rs`, plus la suite lente

**Interfaces:**
- Consumes: `model::{TokenPurpose, one_time_token}` de la Task 3.
- Produces, sous `super::repository::one_time_token` :
  - `async fn issue(db: &DatabaseConnection, user_id: Uuid, purpose: TokenPurpose, fingerprint: String, expires_at: DateTimeWithTimeZone) -> Result<()>`
  - `async fn find(db: &DatabaseConnection, fingerprint: &str, purpose: TokenPurpose) -> Result<Option<Model>>`
  - `async fn consume(db: &DatabaseConnection, id: Uuid) -> Result<bool>`
  - `async fn invalidate_pending(db: &DatabaseConnection, user_id: Uuid, purpose: TokenPurpose) -> Result<u64>`
  - `async fn purge_expired(db: &DatabaseConnection) -> Result<u64>`

- [ ] **Step 1 : Écrire le test de concurrence, dans le projet engendré**

`tests/password.rs.jinja` naît avec ce seul test — celui qui prouve l'invariant qui compte :

```rust
use super::*;

/// Deux consommations simultanées du même jeton : une seule passe.
///
/// C'est la garantie que la condition portée par l'`UPDATE` achète, et qu'une lecture
/// suivie d'une écriture ne donnerait pas — les deux franchiraient la lecture avant que
/// l'une ait écrit, et poseraient chacune leur mot de passe.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_token_is_consumed_once_even_under_concurrency() {
    let db = connection().await;
    let compte = registered_user(&db).await;

    let jeton = rbs_core::token::random();
    crate::auth::repository::one_time_token::issue(
        &db,
        compte.id,
        crate::auth::model::TokenPurpose::PasswordReset,
        rbs_core::token::fingerprint(&jeton),
        (Utc::now() + chrono::Duration::hours(1)).fixed_offset(),
    )
    .await
    .expect("le jeton s'émet");

    let ligne = crate::auth::repository::one_time_token::find(
        &db,
        &rbs_core::token::fingerprint(&jeton),
        crate::auth::model::TokenPurpose::PasswordReset,
    )
    .await
    .expect("la lecture aboutit")
    .expect("le jeton vient d'être émis");

    let (un, deux) = tokio::join!(
        crate::auth::repository::one_time_token::consume(&db, ligne.id),
        crate::auth::repository::one_time_token::consume(&db, ligne.id),
    );

    let passes = [un.expect("pas d'erreur"), deux.expect("pas d'erreur")]
        .into_iter()
        .filter(|passe| *passe)
        .count();

    assert_eq!(passes, 1, "deux consommations concurrentes ont abouti");
}
```

`registered_user(&db)` est une aide à ajouter à `tests/mod.rs.jinja` : elle insère un utilisateur avec une adresse fraîche et rend le `Model`. La déclarer `pub(super)` et l'écrire par `repository::create(db, &fresh_email(), "hash sans valeur").await`.

- [ ] **Step 2 : Écrire le test rapide côté CLI**

```rust
/// Le repository des jetons est déposé, et la purge y est, prête à être branchée.
#[test]
fn the_one_time_token_repository_is_written() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_auth(&parent);

    let source = fs::read_to_string(racine.join("src/auth/repository/one_time_token.rs"))
        .expect("src/auth/repository/one_time_token.rs lisible");

    for attendu in [
        "pub async fn issue",
        "pub async fn find",
        "pub async fn consume",
        "pub async fn invalidate_pending",
        "pub async fn purge_expired",
    ] {
        assert!(source.contains(attendu), "le repository ne porte pas `{attendu}`");
    }
}
```

- [ ] **Step 3 : Lancer les deux pour les voir échouer**

```bash
cargo test -p rbs-cli --test integration_auth -- --exact the_one_time_token_repository_is_written
```

Attendu : FAIL, `src/auth/repository/one_time_token.rs lisible`.

- [ ] **Step 4 : Écrire le repository**

```rust
use rbs_core::Result;
use sea_orm::prelude::{DateTimeWithTimeZone, Expr, Uuid};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use super::super::model::{TokenPurpose, one_time_token};

pub use one_time_token::Model;

/// Ouvre un jeton à usage unique.
///
/// `fingerprint` et non le jeton : une base lue par un tiers ne lui donne aucun lien
/// qu'il puisse jouer.
pub async fn issue(
    db: &DatabaseConnection,
    user_id: Uuid,
    purpose: TokenPurpose,
    fingerprint: String,
    expires_at: DateTimeWithTimeZone,
) -> Result<()> {
    one_time_token::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(fingerprint),
        purpose: Set(purpose),
        expires_at: Set(expires_at),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

/// Retrouve un jeton par son empreinte **et son usage**.
///
/// L'usage fait partie de la recherche : sans lui, un jeton de vérification — plus long à
/// périmer, et envoyé à toute inscription — vaudrait comme jeton de réinitialisation.
pub async fn find(
    db: &DatabaseConnection,
    fingerprint: &str,
    purpose: TokenPurpose,
) -> Result<Option<Model>> {
    Ok(one_time_token::Entity::find()
        .filter(one_time_token::Column::TokenHash.eq(fingerprint))
        .filter(one_time_token::Column::Purpose.eq(purpose))
        .one(db)
        .await?)
}

/// Consomme un jeton, et dit si c'est bien cet appel qui l'a fait.
///
/// La péremption est **dans** la condition de l'`UPDATE` et non dans la lecture qui le
/// précède : deux réinitialisations concurrentes du même jeton franchiraient sinon toutes
/// deux la lecture, et poseraient chacune leur mot de passe — la seconde gagnant sans que
/// la première le sache.
pub async fn consume(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    let touchees = one_time_token::Entity::update_many()
        .col_expr(
            one_time_token::Column::ConsumedAt,
            Expr::current_timestamp().into(),
        )
        .filter(one_time_token::Column::Id.eq(id))
        .filter(one_time_token::Column::ConsumedAt.is_null())
        .filter(Expr::col(one_time_token::Column::ExpiresAt).gt(Expr::current_timestamp()))
        .exec(db)
        .await?;

    Ok(touchees.rows_affected == 1)
}

/// Ferme les jetons encore vivants du même compte et du même usage.
///
/// Appelée avant chaque émission. Sans elle, l'utilisateur qui redemande un lien parce que
/// le premier est parti dans une boîte qu'il ne contrôle plus laisse ce premier lien
/// valide jusqu'à son terme.
pub async fn invalidate_pending(
    db: &DatabaseConnection,
    user_id: Uuid,
    purpose: TokenPurpose,
) -> Result<u64> {
    let touchees = one_time_token::Entity::update_many()
        .col_expr(
            one_time_token::Column::ConsumedAt,
            Expr::current_timestamp().into(),
        )
        .filter(one_time_token::Column::UserId.eq(user_id))
        .filter(one_time_token::Column::Purpose.eq(purpose))
        .filter(one_time_token::Column::ConsumedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected)
}

/// Supprime les jetons périmés, et dit combien.
///
/// La table croît d'une ligne par demande et n'en perd aucune : sans appel périodique,
/// elle est la seule du projet dont la taille suit le trafic anonyme. Le fragment ne
/// branche pas la tâche — `rbs add scheduler` vous donne où la poser, et cette fonction
/// est ce qu'elle appellera. Retirez ce `#[allow]` en la branchant.
#[allow(dead_code)]
pub async fn purge_expired(db: &DatabaseConnection) -> Result<u64> {
    let supprimees = one_time_token::Entity::delete_many()
        .filter(Expr::col(one_time_token::Column::ExpiresAt).lt(Expr::current_timestamp()))
        .exec(db)
        .await?;

    Ok(supprimees.rows_affected)
}
```

Note d'exécution : `col_expr` attend un `SimpleExpr`. Si `Expr::current_timestamp().into()` ne convient pas à la version de SeaORM du projet, reprendre la forme exacte de `repository/refresh_token.rs`, qui compile déjà — c'est le même appel.

- [ ] **Step 5 : Déclarer le module et le fichier**

`repository/mod.rs.jinja` gagne `pub mod one_time_token;`. Ne **pas** réexporter ses fonctions à plat : `consume` et `find` existent déjà sous ce nom pour les jetons de rafraîchissement, et deux `consume` au même niveau seraient une collision autant qu'une confusion. Les appelants écrivent `one_time_token::consume(...)`.

`feature.toml` gagne les deux entrées `[[files]]` : `repository/one_time_token.rs.jinja` et `tests/password.rs.jinja`. `tests/mod.rs.jinja` gagne `mod password;`.

- [ ] **Step 6 : Régénérer, compiler, lancer**

```bash
# Annexe A, puis :
cargo check --manifest-path examples/blog-auth/Cargo.toml
cargo test -p rbs-cli --test integration_auth
cargo test -p rbs-cli --test integration_examples
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```

- [ ] **Step 7 : Lancer la suite lente, une fois pour le lot**

Docker doit tourner.

```bash
mkdir -p "$SCRATCHPAD"
cargo test -p rbs-cli --no-fail-fast -- --ignored > "$SCRATCHPAD/lot1.txt" 2>&1
tail -40 "$SCRATCHPAD/lot1.txt"
```

Attendu : 0 échec. C'est la seule preuve que le fragment fonctionne vraiment.

- [ ] **Step 8 : Commit**

```bash
git add -A
git commit -F - <<'EOF'
feat(auth): ajoute le repository des jetons à usage unique

La consommation porte sa condition dans l'UPDATE — non consommé et non périmé —
plutôt que de suivre une lecture. Deux réinitialisations concurrentes du même jeton
franchiraient sinon toutes deux la lecture avant que l'une ait écrit, et poseraient
chacune leur mot de passe.

`find` prend l'usage autant que l'empreinte : un jeton de vérification, plus long à
périmer et envoyé à chaque inscription, vaudrait sinon comme jeton de
réinitialisation.

`purge_expired` est livrée mais non branchée : la table croît d'une ligne par
demande anonyme, et le fragment ne prend pas sur lui d'exiger un ordonnanceur.

Vérifications :
- cargo test -p rbs-cli --test integration_auth : N passés, 0 échec
- suite lente `-- --ignored --no-fail-fast` : N passés, 0 échec
- clippy -D warnings et fmt --check : propres
EOF
```

---

## Lot 2 — Le mot de passe

### Task 5 : `send_template_detached` sur le `Mailer`

Le `Mailer` sait envoyer un gabarit en attendant (`send_template`) et envoyer un message sans attendre (`send_detached`). Les deux parcours de `auth` ont besoin des deux à la fois.

**Files:**
- Modify: `crates/rbs-cli/templates/features/mail/service.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/mail/tests.rs.jinja`
- Test: `crates/rbs-cli/tests/integration_mail.rs`

**Interfaces:**
- Produces: `Mailer::send_template_detached<S: Serialize>(&self, recipient: &str, subject: &str, template: &str, context: S) -> Result<()>` — rend `Err` si le gabarit est absent ou mal formé, `Ok` dès que l'envoi est lancé.

- [ ] **Step 1 : Écrire le test**

Dans `crates/rbs-cli/tests/integration_mail.rs`, sur le modèle des tests voisins :

```rust
/// Le rendu est fait avant de rendre la main, l'envoi non : un gabarit absent est une
/// erreur que l'appelant voit, une panne de SMTP n'en est pas une.
#[test]
fn the_mailer_can_render_now_and_send_later() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_mail(&parent);

    let source = fs::read_to_string(racine.join("src/modules/mail/service.rs"))
        .expect("src/modules/mail/service.rs lisible");

    assert!(
        source.contains("pub fn send_template_detached"),
        "le Mailer ne sait pas rendre maintenant et envoyer plus tard"
    );
}
```

Reprendre le nom exact de l'aide de fixture de ce fichier — elle ne s'appelle pas forcément `project_with_mail`. Lire les tests voisins avant d'écrire celui-ci.

- [ ] **Step 2 : Lancer le test pour le voir échouer**

```bash
cargo test -p rbs-cli --test integration_mail -- --exact the_mailer_can_render_now_and_send_later
```

Attendu : FAIL.

- [ ] **Step 3 : Écrire la méthode**

Dans `service.rs.jinja`, à côté de `send_detached` :

```rust
    /// Rend `template` maintenant, et lance l'envoi sans l'attendre.
    ///
    /// La dissymétrie est voulue : un gabarit absent ou mal formé est une faute du projet,
    /// que l'appelant doit voir tout de suite ; une panne du serveur SMTP n'en est pas une,
    /// et ne doit pas retenir la réponse HTTP. C'est ce qui permet à `/auth/forgot-password`
    /// de répondre en un temps qui ne dit pas si l'adresse est inscrite.
    pub fn send_template_detached<S: Serialize>(
        &self,
        recipient: &str,
        subject: &str,
        template: &str,
        context: S,
    ) -> Result<()> {
        let body = self.templates.render(template, context)?;

        self.send_detached(self.message(recipient, subject, body)?);

        Ok(())
    }
```

- [ ] **Step 4 : Vérifier**

```bash
cargo test -p rbs-cli --test integration_mail
cargo check --manifest-path examples/newsletter-queue/Cargo.toml
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```

`newsletter-queue` et `file-drop` portent `mail` : les régénérer tous deux selon l'Annexe A, puis `cargo test -p rbs-cli --test integration_examples`.

- [ ] **Step 5 : Commit**

```bash
git add -A
git commit -F - <<'EOF'
feat(mail): rend un gabarit maintenant et lance l'envoi sans l'attendre

Le transport savait envoyer un gabarit en attendant, et un message sans attendre ;
les deux parcours d'authentification qui arrivent ont besoin des deux à la fois.

La dissymétrie est le point : un gabarit absent est une faute du projet, que
l'appelant voit tout de suite ; une panne de SMTP n'en est pas une, et ne doit pas
retenir une réponse dont le temps ne doit rien apprendre à qui la mesure.

Vérifications :
- cargo test -p rbs-cli --test integration_mail : N passés, 0 échec
- cargo test -p rbs-cli --test integration_examples : N passés, 0 échec
- clippy -D warnings et fmt --check : propres
EOF
```

---

### Task 6 : `POST /auth/change-password`

**Files:**
- Create: `crates/rbs-cli/templates/features/auth/service/password.rs.jinja`, `controller/password.rs.jinja`
- Modify: `dto.rs.jinja`, `mod.rs.jinja`, `service/mod.rs.jinja`, `controller/mod.rs.jinja`, `repository/user.rs.jinja`, `tests/password.rs.jinja`, `feature.toml`
- Test: `crates/rbs-cli/tests/integration_auth.rs`, plus la suite lente

**Interfaces:**
- Consumes: `repository::one_time_token` (Task 4), `service::issue` (Task 1), `FlowConfig` (Task 2).
- Produces:
  - `repository::user::set_password(db: &DatabaseConnection, id: Uuid, hash: &str) -> Result<()>`
  - `service::password::change(db: &DatabaseConnection, auth: &AuthConfig, user_id: Uuid, input: ChangePasswordRequest) -> Result<TokenPair>`
  - `dto::ChangePasswordRequest { current_password: String, new_password: String }`
  - `controller::password::change_password`

- [ ] **Step 1 : Écrire les tests dans le projet engendré**

Dans `tests/password.rs.jinja` :

```rust
/// Le parcours nominal : la paire rendue est utilisable, et l'ancienne ne l'est plus.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn changing_the_password_returns_a_usable_pair_and_closes_the_others() {
    let api = application().await;
    let email = fresh_email();

    register(&api, &email).await;
    let premiere = login(&api, &email, PASSWORD).await;

    let (statut, corps) = call(
        &api,
        post_json_authenticated(
            "/auth/change-password",
            premiere["access_token"].as_str().expect("jeton d'accès"),
            json!({
                "current_password": PASSWORD,
                "new_password": "un autre mot de passe assez long",
            }),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::OK, "corps : {corps}");

    // L'ancienne session est fermée : son jeton de rafraîchissement ne tourne plus.
    let (rejet, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": premiere["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(rejet, StatusCode::UNAUTHORIZED);

    // Celle que le changement vient de rendre, elle, tourne.
    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": corps["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::OK);
}

/// Un mot de passe courant faux rend 403 et non 401 : l'appelant est identifié, son
/// Bearer est bon. Un 401 lui dirait que son jeton est mort et déclencherait un
/// rafraîchissement inutile.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_wrong_current_password_is_forbidden_not_unauthorized() {
    let api = application().await;
    let email = fresh_email();

    register(&api, &email).await;
    let paire = login(&api, &email, PASSWORD).await;

    let (statut, _) = call(
        &api,
        post_json_authenticated(
            "/auth/change-password",
            paire["access_token"].as_str().expect("jeton d'accès"),
            json!({
                "current_password": "ce n'est pas le bon mot de passe",
                "new_password": "un autre mot de passe assez long",
            }),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::FORBIDDEN);
}
```

`post_json_authenticated(chemin, jeton, body)` est une aide à ajouter à `tests/mod.rs.jinja` : c'est `post_json` avec un en-tête `authorization: Bearer <jeton>`.

- [ ] **Step 2 : Écrire le test rapide côté CLI**

Étendre le test existant `the_five_auth_paths_are_mounted` — le renommer `the_auth_paths_are_mounted` et lui donner les treize chemins au fur et à mesure des tâches. Y ajouter ici `"/auth/change-password"`.

- [ ] **Step 3 : Lancer pour voir échouer**

```bash
cargo test -p rbs-cli --test integration_auth -- --exact the_auth_paths_are_mounted
```

Attendu : FAIL, `/auth/change-password` absent de `src/auth/mod.rs`.

- [ ] **Step 4 : Écrire le DTO**

Dans `dto.rs.jinja` :

```rust
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct ChangePasswordRequest {
    // La même borne haute que sur `login`, et pour la même raison : sans elle, la route
    // hache en Argon2 tout ce qu'on lui poste.
    #[validate(length(min = 12, max = 128))]
    pub current_password: String,
    #[validate(length(min = 12, max = 128))]
    pub new_password: String,
}
```

- [ ] **Step 5 : Écrire le repository**

Dans `repository/user.rs.jinja` :

```rust
/// Remplace le hash du mot de passe.
///
/// L'`UPDATE` ne touche que cette colonne : charger le modèle pour le réécrire en entier
/// écraserait ce qu'une autre requête a changé entre-temps.
pub async fn set_password(db: &DatabaseConnection, id: Uuid, hash: &str) -> Result<()> {
    Entity::update_many()
        .col_expr(user::Column::PasswordHash, Expr::value(hash))
        .filter(user::Column::Id.eq(id))
        .exec(db)
        .await?;

    Ok(())
}
```

- [ ] **Step 6 : Écrire le service**

`service/password.rs.jinja` :

```rust
use rbs_core::config::AuthConfig;
use rbs_core::{Error, Result, hash};
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::super::dto::{ChangePasswordRequest, TokenPair};
use super::super::repository;
use super::issue;

/// Change le mot de passe d'un compte identifié, et rend une paire neuve.
///
/// Toutes les sessions tombent, celle de l'appelant comprise : la requête porte un jeton
/// d'accès, et rien ne le relie à la ligne qui l'a émis — la session courante ne peut pas
/// être épargnée faute d'être identifiable. Plutôt que de déconnecter quelqu'un qui vient
/// de faire la bonne chose, on révoque puis on réémet.
pub async fn change(
    db: &DatabaseConnection,
    auth: &AuthConfig,
    user_id: Uuid,
    input: ChangePasswordRequest,
) -> Result<TokenPair> {
    let utilisateur = repository::find(db, user_id)
        .await?
        // Un jeton valide dont le compte a disparu ne vaut pas mieux qu'un jeton invalide.
        .ok_or(Error::Unauthorized)?;

    // `Forbidden` et non `Unauthorized` : l'appelant est identifié, son Bearer est bon.
    // Un 401 lui dirait que son jeton est mort, et son client tenterait un
    // rafraîchissement qui ne réglerait rien.
    if !hash::verify_password(&input.current_password, &utilisateur.password_hash)? {
        return Err(Error::Forbidden);
    }

    let nouveau = hash::hash_password(&input.new_password)?;
    repository::user::set_password(db, user_id, &nouveau).await?;

    let fermees = repository::revoke_sessions_of(db, user_id).await?;

    // Ni l'adresse ni les jetons : l'identifiant du compte suffit à retrouver ce qui s'est
    // passé, et le journal ne porte pas ce que la réponse tait.
    tracing::info!(
        user_id = %user_id,
        sessions_revoquees = fermees,
        "mot de passe changé : les sessions du compte sont révoquées"
    );

    issue(db, auth, &utilisateur).await
}
```

- [ ] **Step 7 : Écrire le contrôleur**

`controller/password.rs.jinja` :

```rust
use axum::extract::State;
use rbs_core::{HasAuth, HasCoreState, Identity, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::super::dto::{ChangePasswordRequest, TokenPair};
use super::super::service;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/auth/change-password",
    tag = "auth",
    operation_id = "auth_change_password",
    security(("bearer" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "mot de passe changé, paire neuve", body = TokenPair),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "mot de passe courant refusé", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<ChangePasswordRequest>,
) -> Result<TokenPair> {
    // `sub` porte l'identifiant sous forme de chaîne : le jeton est signé, mais rien ne
    // garantit que celui-ci a été émis par une version du service qui y mettait un UUID.
    let id = Uuid::parse_str(&identite.user_id).map_err(|_| rbs_core::Error::Unauthorized)?;

    service::password::change(state.core().db(), state.auth(), id, input).await
}
```

Attention à l'ordre des extracteurs : `ValidatedJson` consomme le corps et doit rester en dernier.

- [ ] **Step 8 : Monter la route et l'inscrire au document**

`mod.rs.jinja` : `.route("/auth/change-password", post(controller::password::change_password))`. `controller/mod.rs.jinja` : `pub mod password;`. `service/mod.rs.jinja` : `pub mod password;`. `feature.toml`, ancre `openapi` : `crate::auth::controller::password::change_password,`.

- [ ] **Step 9 : Régénérer, compiler, lancer**

```bash
# Annexe A, puis :
cargo check --manifest-path examples/blog-auth/Cargo.toml
cargo test -p rbs-cli --test integration_auth
cargo test -p rbs-cli --test integration_examples
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```

- [ ] **Step 10 : Commit**

```bash
git add -A
git commit -F - <<'EOF'
feat(auth): ajoute le changement de mot de passe

La route rend une paire neuve plutôt qu'un 204. Le changement révoque toutes les
sessions du compte, celle de l'appelant comprise : la requête porte un jeton
d'accès, et rien ne le relie à la ligne de rafraîchissement qui l'a émis — la
session courante ne peut pas être épargnée faute d'être identifiable. La réémettre
évite de déconnecter quelqu'un qui vient de faire la bonne chose.

Un mot de passe courant faux rend 403 et non 401 : l'appelant est identifié, son
Bearer est bon, et un 401 lui ferait tenter un rafraîchissement qui ne réglerait
rien.

Vérifications :
- cargo test -p rbs-cli --test integration_auth : N passés, 0 échec
- cargo check sur examples/blog-auth régénéré : succès
- clippy -D warnings et fmt --check : propres
EOF
```

---

### Task 7 : `POST /auth/forgot-password` et `POST /auth/reset-password`

**Files:**
- Create: `crates/rbs-cli/templates/features/auth/reinitialisation.html.jinja`
- Modify: `service/password.rs.jinja`, `controller/password.rs.jinja`, `dto.rs.jinja`, `mod.rs.jinja`, `tests/password.rs.jinja`, `feature.toml`
- Modify: `crates/rbs-cli/templates/features/rate-limit/feature.toml`
- Modify: `docs/docs/guides/auth.md` et son miroir français
- Test: `crates/rbs-cli/tests/integration_auth.rs`, plus la suite lente

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces:
  - `dto::EmailRequest { email: String }` — servira aussi à `resend-verification`
  - `dto::ResetPasswordRequest { token: String, new_password: String }`
  - `service::password::request_reset(db, ttl_secs: u64, email: &str) -> Result<Option<(repository::Model, String)>>` — `None` quand aucun compte ne porte l'adresse ; le `String` est le jeton **en clair**
  - `service::password::reset(db, input: ResetPasswordRequest) -> Result<()>`

- [ ] **Step 1 : Écrire les tests dans le projet engendré**

```rust
/// Le parcours entier, jeton compris.
///
/// Le test passe par la couche service pour l'émission : la base ne garde que
/// l'empreinte, et aucune lecture ne rendrait le jeton en clair que le courriel porte.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_reset_token_sets_a_new_password_and_closes_every_session() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let paire = login(&api, &email, PASSWORD).await;

    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let (statut, corps) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": jeton, "new_password": "un troisieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT, "corps : {corps}");

    // Le nouveau mot de passe ouvre, l'ancien non.
    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/login",
            json!({ "email": email, "password": "un troisieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::OK);

    let (refuse, _) = call(
        &api,
        post_json("/auth/login", json!({ "email": email, "password": PASSWORD })),
    )
    .await;
    assert_eq!(refuse, StatusCode::UNAUTHORIZED);

    // Et la session ouverte avant la réinitialisation est tombée.
    let (rejet, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": paire["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(rejet, StatusCode::UNAUTHORIZED);
}

/// Le même jeton, deux fois : la seconde est refusée.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_reset_token_serves_once() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let corps = json!({ "token": jeton, "new_password": "un quatrieme mot de passe long" });

    let (premier, _) = call(&api, post_json("/auth/reset-password", corps.clone())).await;
    assert_eq!(premier, StatusCode::NO_CONTENT);

    let (second, _) = call(&api, post_json("/auth/reset-password", corps)).await;
    assert_eq!(second, StatusCode::UNAUTHORIZED);
}

/// Une seconde demande ferme la première : un lien parti dans une boîte qu'on ne
/// contrôle plus cesse de valoir dès qu'on en redemande un.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_new_request_invalidates_the_previous_link() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (_, premier) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");
    let (_, second) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let (refuse, _) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": premier, "new_password": "un cinquieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(refuse, StatusCode::UNAUTHORIZED);

    let (accepte, _) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": second, "new_password": "un cinquieme mot de passe long" }),
        ),
    )
    .await;
    assert_eq!(accepte, StatusCode::NO_CONTENT);
}

/// Une adresse qu'aucun compte ne porte rend 202 et n'écrit rien : distinguer les deux
/// cas ferait de cette route l'oracle d'énumération que le hash témoin de `login` écarte
/// de l'autre côté.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn forgetting_an_unknown_address_is_accepted_and_writes_nothing() {
    let api = application().await;
    let db = connection().await;
    let inconnue = fresh_email();

    let (statut, _) = call(
        &api,
        post_json("/auth/forgot-password", json!({ "email": inconnue })),
    )
    .await;

    assert_eq!(statut, StatusCode::ACCEPTED);

    // Aucun compte ne porte l'adresse, donc aucun jeton n'a pu être émis : l'émission
    // part de `find_by_email`, et rien d'autre n'y mène.
    assert!(
        crate::auth::repository::find_by_email(&db, &inconnue)
            .await
            .expect("la lecture aboutit")
            .is_none(),
        "la demande a créé un compte"
    );
}
```

`one_time_tokens_count_for(&db, user_id)` est l'aide à ajouter à `tests/mod.rs.jinja` — elle compte les jetons **d'un compte**, jamais de la table entière :

```rust
pub(super) async fn one_time_tokens_count_for(db: &DatabaseConnection, user_id: Uuid) -> u64 {
    crate::auth::model::one_time_token::Entity::find()
        .filter(crate::auth::model::one_time_token::Column::UserId.eq(user_id))
        .count(db)
        .await
        .expect("le comptage aboutit")
}
```

Le compte est borné à un utilisateur parce que les tests partagent une base qu'ils ne vident pas et que `cargo test` les exécute en parallèle : un compte global serait faux dès qu'un autre test inscrit quelqu'un pendant la mesure.

- [ ] **Step 2 : Lancer pour voir échouer**

Ces tests ne compilent pas encore. C'est l'échec attendu :

```bash
cargo check --manifest-path examples/blog-auth/Cargo.toml --tests
```

Attendu : FAIL, `service::password::request_reset` inconnue.

- [ ] **Step 3 : Écrire les DTO**

```rust
/// Ce que postent `forgot-password` et `resend-verification`.
///
/// Une seule structure pour les deux : elles prennent la même chose, et deux structures
/// identiques divergeraient un jour sans raison.
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct EmailRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[validate(length(min = 12, max = 128))]
    pub new_password: String,
}
```

- [ ] **Step 4 : Écrire le service**

Dans `service/password.rs.jinja` :

```rust
/// Ouvre un jeton de réinitialisation, et rend le compte avec le jeton **en clair**.
///
/// Le jeton en clair ne se relit nulle part : la base n'en garde que l'empreinte. Le
/// rendre ici est ce qui permet au contrôleur de le mettre dans un courriel — et aux
/// tests du projet de dérouler le parcours entier sans qu'aucun SMTP soit joignable.
///
/// `None` quand aucun compte ne porte l'adresse. C'est l'appelant qui décide d'en tirer
/// une réponse indiscernable, et il le fait.
pub async fn request_reset(
    db: &DatabaseConnection,
    ttl_secs: u64,
    email: &str,
) -> Result<Option<(repository::Model, String)>> {
    let Some(utilisateur) = repository::find_by_email(db, email).await? else {
        return Ok(None);
    };

    repository::one_time_token::invalidate_pending(db, utilisateur.id, TokenPurpose::PasswordReset)
        .await?;

    let jeton = token::random();
    repository::one_time_token::issue(
        db,
        utilisateur.id,
        TokenPurpose::PasswordReset,
        token::fingerprint(&jeton),
        (Utc::now() + Duration::seconds(ttl_secs as i64)).fixed_offset(),
    )
    .await?;

    Ok(Some((utilisateur, jeton)))
}

/// Consomme un jeton et pose le nouveau mot de passe.
///
/// Toutes les sessions tombent : on ne sait pas si le compte avait été pris, et les
/// laisser tourner reviendrait à valider la prise.
pub async fn reset(db: &DatabaseConnection, input: ResetPasswordRequest) -> Result<()> {
    let fingerprint = token::fingerprint(&input.token);

    // Jeton inconnu, périmé ou déjà consommé : la même erreur pour les trois. Les
    // distinguer renseignerait sur l'état des demandes en cours.
    let ligne = repository::one_time_token::find(db, &fingerprint, TokenPurpose::PasswordReset)
        .await?
        .ok_or(Error::Unauthorized)?;

    // Rien ici ne relit `consumed_at` ni `expires_at` : c'est `consume` qui porte les deux
    // conditions, et elle seule peut les porter sans laisser passer deux
    // réinitialisations concurrentes.
    if !repository::one_time_token::consume(db, ligne.id).await? {
        return Err(Error::Unauthorized);
    }

    let nouveau = hash::hash_password(&input.new_password)?;
    repository::user::set_password(db, ligne.user_id, &nouveau).await?;

    let fermees = repository::revoke_sessions_of(db, ligne.user_id).await?;

    tracing::info!(
        user_id = %ligne.user_id,
        sessions_revoquees = fermees,
        "mot de passe réinitialisé : les sessions du compte sont révoquées"
    );

    Ok(())
}
```

- [ ] **Step 5 : Écrire le gabarit du courriel**

`reinitialisation.html.jinja` — attention, ce fichier est rendu **deux fois** : une fois par le CLI en déposant le fragment, une fois par le projet en envoyant le message. Le second rendu emploie `{{ }}` et le premier `{@ @}` : écrire les variables du message en `{{ }}` ne heurte donc rien.

```html
<!doctype html>
<html lang="fr">
  <body>
    <p>Bonjour,</p>
    <p>
      Vous avez demandé à réinitialiser votre mot de passe. Suivez
      <a href="{{ link }}">ce lien</a> pour en choisir un nouveau :
    </p>
    <p><a href="{{ link }}">{{ link }}</a></p>
    <p>
      Le lien vaut {{ heures }} heure(s). Si vous n'êtes pas à l'origine de cette
      demande, il n'y a rien à faire : votre mot de passe actuel reste valable.
    </p>
  </body>
</html>
```

Déclaré dans `feature.toml` :

```toml
[[files]]
source      = "reinitialisation.html.jinja"
destination = "templates/mail/reinitialisation.html"
```

- [ ] **Step 6 : Écrire les contrôleurs**

```rust
// L'envoi part détaché, et le statut est 202 quoi qu'il arrive. Attendre le SMTP
// rendrait par le temps de réponse ce que le code de statut refuse de dire : quelques
// centaines de millisecondes séparent une adresse inscrite d'une adresse inconnue.
#[utoipa::path(
    post,
    path = "/auth/forgot-password",
    tag = "auth",
    operation_id = "auth_forgot_password",
    request_body = EmailRequest,
    responses(
        (status = 202, description = "demande acceptée, que l'adresse soit inscrite ou non"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<EmailRequest>,
) -> Result<StatusCode> {
    let flows = state.flows();

    if let Some((utilisateur, jeton)) =
        service::password::request_reset(state.core().db(), flows.reset_ttl_secs, &input.email)
            .await?
    {
        state.mail().send_template_detached(
            &utilisateur.email,
            "Réinitialisation de votre mot de passe",
            "reinitialisation.html",
            minijinja::context! {
                link => flows.link("reset-password", &jeton),
                heures => flows.reset_ttl_secs / 3600,
            },
        )?;
    }

    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    post,
    path = "/auth/reset-password",
    tag = "auth",
    operation_id = "auth_reset_password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 204, description = "mot de passe réinitialisé, sessions révoquées"),
        (status = 401, description = "jeton inconnu, périmé ou déjà consommé", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn reset_password(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<ResetPasswordRequest>,
) -> Result<StatusCode> {
    service::password::reset(state.core().db(), input).await?;

    Ok(StatusCode::NO_CONTENT)
}
```

`minijinja::context!` demande que `minijinja` soit une dépendance du projet : le fragment `mail` l'apporte déjà. Si le rendu se fait plutôt par `serde_json::json!`, l'employer — `send_template_detached` prend n'importe quel `Serialize`.

- [ ] **Step 7 : Monter, inscrire, limiter**

Les deux routes dans `mod.rs.jinja`, les deux `operation_id` dans l'ancre `openapi`, et dans `crates/rbs-cli/templates/features/rate-limit/feature.toml` :

```toml
routes = [
  { path = "/auth/login", limit = 5, window_secs = 60 },
  # Ces deux routes envoient un courriel à une adresse que l'appelant choisit. Sans borne,
  # elles font du projet un relais de harcèlement — et le coût est payé par le titulaire
  # de l'adresse, qui n'a rien demandé.
  { path = "/auth/forgot-password", limit = 3, window_secs = 3600 },
  { path = "/auth/resend-verification", limit = 3, window_secs = 3600 },
]
```

`/auth/resend-verification` est posée dès maintenant : la route arrive en Task 8, la limite ne coûte rien avant elle, et l'oublier ensuite coûterait cher.

- [ ] **Step 8 : Mettre la documentation à jour**

`docs/docs/guides/auth.md` et `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/auth.md`, **dans le même commit** : une section sur le parcours de réinitialisation, le fait que `mail` arrive avec `auth`, et les trois clés de `[auth]`. Ne recopier aucun extrait à la main — les guides citent les exemples.

- [ ] **Step 9 : Régénérer, compiler, lancer**

```bash
# Annexe A, puis :
cargo check --manifest-path examples/blog-auth/Cargo.toml --tests
cargo test -p rbs-cli --test integration_auth
cargo test -p rbs-cli --test integration_examples
cargo test -p rbs-cli --test integration_docs
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```

- [ ] **Step 10 : Lancer la suite lente**

```bash
cargo test -p rbs-cli --no-fail-fast -- --ignored > "$SCRATCHPAD/lot2.txt" 2>&1
tail -40 "$SCRATCHPAD/lot2.txt"
```

- [ ] **Step 11 : Commit**

```bash
git add -A
git commit -F - <<'EOF'
feat(auth): ajoute l'oubli et la réinitialisation du mot de passe

Les deux routes rendent 202 et 204 sans jamais dire si l'adresse est inscrite :
c'est l'invariant qui gouverne déjà `login`, où le hash témoin fait payer un Argon2
à une adresse inconnue. L'envoi du courriel part détaché pour la même raison —
attendre le SMTP rendrait par le temps de réponse ce que le statut refuse de dire.

`request_reset` rend le jeton en clair à son appelant. La base n'en garde que
l'empreinte, et sans ce retour aucun test du projet ne pourrait dérouler le parcours
entier : le contrôleur le passe au transport, puis le laisse tomber.

Une nouvelle demande ferme la précédente, et le service de limite de débit borne les
deux routes à trois par heure : elles envoient un courriel à une adresse que
l'appelant choisit.

Vérifications :
- cargo test -p rbs-cli --test integration_auth : N passés, 0 échec
- suite lente `-- --ignored --no-fail-fast` : N passés, 0 échec
- clippy -D warnings et fmt --check : propres
EOF
```

---

## Lot 3 — La vérification d'adresse

### Task 8 : `POST /auth/verify-email`, `POST /auth/resend-verification`, et l'envoi à l'inscription

**Files:**
- Create: `service/verification.rs.jinja`, `controller/verification.rs.jinja`, `tests/verification.rs.jinja`, `verification.html.jinja`
- Modify: `dto.rs.jinja`, `mod.rs.jinja`, `service/mod.rs.jinja`, `controller/mod.rs.jinja`, `controller/session.rs.jinja`, `repository/user.rs.jinja`, `feature.toml`
- Modify: `docs/docs/guides/auth.md` et son miroir français
- Test: `crates/rbs-cli/tests/integration_auth.rs`, plus la suite lente

**Interfaces:**
- Produces:
  - `repository::user::mark_verified(db, id: Uuid) -> Result<()>`
  - `service::verification::request(db, ttl_secs: u64, email: &str) -> Result<Option<(repository::Model, String)>>` — **une seule** fonction pour l'inscription et le renvoi : les deux partent d'une adresse et rendent le jeton en clair. En faire deux ferait appeler le repository depuis le contrôleur de `register`, ce que la dépendance des couches interdit.
  - `service::verification::verify(db, token: &str) -> Result<()>`
  - `dto::TokenRequest { token: String }`

- [ ] **Step 1 : Écrire les tests dans le projet engendré**

`tests/verification.rs.jinja` :

```rust
use super::*;

/// L'inscription ouvre un jeton de vérification : c'est ce qui fait que le courriel part
/// sans qu'aucune route ne soit appelée.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn registering_opens_a_verification_token() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;

    let compte = crate::auth::repository::find_by_email(&db, &email)
        .await
        .expect("la lecture aboutit")
        .expect("le compte vient d'être créé");

    assert_eq!(
        one_time_tokens_count_for(&db, compte.id).await,
        1,
        "l'inscription n'a ouvert aucun jeton"
    );
}

/// Le parcours nominal : `email_verified_at` passe de nul à daté, et `me` le montre.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn verifying_marks_the_address_and_shows_on_me() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    // `register` rend `(StatusCode, Value)` : le corps est le second membre.
    let (_, compte) = register(&api, &email).await;
    assert!(
        compte["email_verified_at"].is_null(),
        "une adresse fraîchement inscrite n'est pas vérifiée"
    );

    let (_, jeton) = crate::auth::service::verification::request(&db, 86400, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte vient d'être créé");

    let (statut, _) = call(
        &api,
        post_json("/auth/verify-email", json!({ "token": jeton })),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT);

    let paire = login(&api, &email, PASSWORD).await;
    let (_, profil) = call(
        &api,
        get_authenticated("/auth/me", paire["access_token"].as_str().expect("jeton")),
    )
    .await;

    assert!(
        !profil["email_verified_at"].is_null(),
        "me ne montre pas la vérification : {profil}"
    );
}

/// Un jeton de réinitialisation ne vaut pas comme jeton de vérification.
///
/// C'est ce que l'usage porté par la recherche achète : sans lui, la table unique serait
/// une faille au lieu d'une économie.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn a_reset_token_does_not_verify_an_address() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let (_, jeton) = crate::auth::service::password::request_reset(&db, 3600, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");

    let (statut, _) = call(
        &api,
        post_json("/auth/verify-email", json!({ "token": jeton })),
    )
    .await;

    assert_eq!(statut, StatusCode::UNAUTHORIZED);
}

/// Une adresse inconnue rend 202, comme `forgot-password` et pour la même raison.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn resending_to_an_unknown_address_is_accepted() {
    let api = application().await;

    let (statut, _) = call(
        &api,
        post_json("/auth/resend-verification", json!({ "email": fresh_email() })),
    )
    .await;

    assert_eq!(statut, StatusCode::ACCEPTED);
}
```

`get_authenticated(chemin, jeton)` est une aide à ajouter à `tests/mod.rs.jinja`, sur le modèle de `post_json_authenticated`.

- [ ] **Step 2 : Lancer pour voir échouer**

```bash
cargo check --manifest-path examples/blog-auth/Cargo.toml --tests
```

Attendu : FAIL, `service::verification` inconnu.

- [ ] **Step 3 : Écrire le DTO, le repository, le service**

```rust
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct TokenRequest {
    pub token: String,
}
```

```rust
/// Date la vérification de l'adresse.
///
/// La date et non un booléen : savoir *quand* une adresse a été prouvée est ce qui
/// permet, un jour, d'en redemander la preuve aux plus anciennes.
pub async fn mark_verified(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    Entity::update_many()
        .col_expr(user::Column::EmailVerifiedAt, Expr::current_timestamp().into())
        .filter(user::Column::Id.eq(id))
        .exec(db)
        .await?;

    Ok(())
}
```

`service/verification.rs.jinja` :

```rust
use chrono::{Duration, Utc};
use rbs_core::{Error, Result, token};
use sea_orm::DatabaseConnection;

use super::super::model::TokenPurpose;
use super::super::repository;

/// Ouvre un jeton de vérification, et rend le compte avec le jeton **en clair**.
///
/// Une seule fonction pour l'inscription et pour le renvoi : les deux partent d'une
/// adresse et rendent la même chose. En écrire une seconde qui prendrait le modèle ferait
/// appeler le repository depuis le contrôleur de `register`, et la dépendance des couches
/// ne le permet pas.
pub async fn request(
    db: &DatabaseConnection,
    ttl_secs: u64,
    email: &str,
) -> Result<Option<(repository::Model, String)>> {
    let Some(utilisateur) = repository::find_by_email(db, email).await? else {
        return Ok(None);
    };

    repository::one_time_token::invalidate_pending(
        db,
        utilisateur.id,
        TokenPurpose::EmailVerification,
    )
    .await?;

    let jeton = token::random();
    repository::one_time_token::issue(
        db,
        utilisateur.id,
        TokenPurpose::EmailVerification,
        token::fingerprint(&jeton),
        (Utc::now() + Duration::seconds(ttl_secs as i64)).fixed_offset(),
    )
    .await?;

    Ok(Some((utilisateur, jeton)))
}

/// Consomme un jeton et date la vérification.
///
/// Jeton inconnu, périmé, déjà consommé, ou émis pour un autre usage : la même erreur
/// pour les quatre. Les distinguer renseignerait sur l'état des demandes en cours.
pub async fn verify(db: &DatabaseConnection, token_clair: &str) -> Result<()> {
    let fingerprint = token::fingerprint(token_clair);

    let ligne =
        repository::one_time_token::find(db, &fingerprint, TokenPurpose::EmailVerification)
            .await?
            .ok_or(Error::Unauthorized)?;

    if !repository::one_time_token::consume(db, ligne.id).await? {
        return Err(Error::Unauthorized);
    }

    repository::user::mark_verified(db, ligne.user_id).await
}
```

- [ ] **Step 4 : Faire envoyer `register`**

Dans `controller/session.rs.jinja`, après l'inscription réussie :

```rust
pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<RegisterRequest>,
) -> Result<(StatusCode, Json<UserResponse>)> {
    let cree = service::register(state.core().db(), input).await?;
    let flows = state.flows();

    // Le compte est ouvert avant l'envoi, et l'envoi ne peut plus le défaire : une panne
    // de SMTP à cet instant laisserait sinon une inscription à moitié faite, sans compte
    // et sans message. `resend-verification` est le rattrapage, et il est à la portée de
    // l'utilisateur.
    if let Some((utilisateur, jeton)) = service::verification::request(
        state.core().db(),
        flows.verification_ttl_secs,
        &cree.email,
    )
    .await?
    {
        state.mail().send_template_detached(
            &utilisateur.email,
            "Confirmez votre adresse",
            "verification.html",
            minijinja::context! { link => flows.link("verify-email", &jeton) },
        )?;
    }

    Ok((StatusCode::CREATED, Json(cree)))
}
```

Le contrôleur repasse par l'adresse plutôt que par un modèle : `service::register` rend une `UserResponse`, et lui faire rendre le modèle ferait sortir le hash du mot de passe de la couche qui le garde. Une lecture de plus contre un champ qui n'a aucun chemin vers le client — le compte est bon.

- [ ] **Step 5 : Contrôleurs, gabarit, montage, OpenAPI, limite**

`verify-email` sur le modèle de `reset-password` (204, 401), `resend-verification` sur celui de `forgot-password` (202 toujours, envoi détaché). `verification.html.jinja` sur le modèle de `reinitialisation.html.jinja`. Les deux routes montées, les deux `operation_id` dans l'ancre `openapi`. La limite de débit de `/auth/resend-verification` est déjà posée en Task 7.

- [ ] **Step 6 : Documentation bilingue**

Les deux `guides/auth.md`, dans le même commit : le parcours de vérification, le fait que `login` ne le réclame pas, et le renvoi vers la garde qui arrive en Task 9.

- [ ] **Step 7 : Régénérer, compiler, lancer, commiter**

Même séquence qu'en Task 7, suite lente comprise.

```bash
git add -A
git commit -F - <<'EOF'
feat(auth): ajoute la vérification de l'adresse et son renvoi

L'inscription ouvre le jeton et lance l'envoi ; le compte est créé avant, et l'envoi
ne peut plus le défaire — une panne de SMTP laisserait sinon une inscription à
moitié faite, sans compte et sans message. `resend-verification` est le rattrapage,
et il est à la portée de l'utilisateur.

`login` ne change pas : un compte non vérifié se connecte. La décision appartient au
projet, à qui la garde livrée à côté donne de quoi la prendre.

L'usage est porté par la recherche du jeton, et un jeton de réinitialisation ne vaut
donc pas comme jeton de vérification — c'est ce qui fait d'une table unique une
économie plutôt qu'une faille.

Vérifications :
- cargo test -p rbs-cli --test integration_auth : N passés, 0 échec
- suite lente `-- --ignored --no-fail-fast` : N passés, 0 échec
- clippy -D warnings et fmt --check : propres
EOF
```

---

### Task 9 : La garde `VerifiedIdentity`

**Files:**
- Modify: `crates/rbs-cli/templates/features/auth/guard.rs.jinja`, `tests/verification.rs.jinja`
- Test: `crates/rbs-cli/tests/integration_auth.rs`, plus la suite lente

**Interfaces:**
- Produces: `guard::VerifiedIdentity`, un extracteur axum qui déréférence vers `Identity`.

- [ ] **Step 1 : Écrire le test**

Dans `tests/verification.rs.jinja`. Le fichier de tests existant monte déjà une route jetable pour éprouver `RequireRole` : reprendre exactement sa forme, y compris la façon dont il passe l'état au `Router`.

```rust
/// La garde rejette avant la vérification et laisse passer après.
///
/// La route est montée ici et nulle part ailleurs : le fragment livre la garde sans
/// l'imposer, et c'est au projet de décider où elle s'applique.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn the_verified_guard_opens_only_after_verification() {
    async fn protegee(_verifiee: crate::auth::guard::VerifiedIdentity) -> StatusCode {
        StatusCode::OK
    }

    let db = connection().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let state = AppState::new(db.clone(), config).expect("état partagé constructible");

    let api = Router::new()
        .route("/protegee", get(protegee))
        .with_state(state.clone());
    let publique = application().await;

    let email = fresh_email();
    register(&publique, &email).await;
    let paire = login(&publique, &email, PASSWORD).await;
    let jeton_acces = paire["access_token"].as_str().expect("jeton d'accès").to_owned();

    let (avant, _) = call(&api, get_authenticated("/protegee", &jeton_acces)).await;
    assert_eq!(avant, StatusCode::FORBIDDEN, "une adresse non vérifiée passe la garde");

    let (_, jeton) = crate::auth::service::verification::request(&db, 86400, &email)
        .await
        .expect("la demande aboutit")
        .expect("le compte existe");
    call(
        &publique,
        post_json("/auth/verify-email", json!({ "token": jeton })),
    )
    .await;

    let (apres, _) = call(&api, get_authenticated("/protegee", &jeton_acces)).await;
    assert_eq!(apres, StatusCode::OK, "une adresse vérifiée est rejetée");
}
```

Le même jeton d'accès sert avant et après : c'est ce qui prouve que l'état est relu en base et non porté par le jeton.

- [ ] **Step 2 : Lancer pour voir échouer**

```bash
cargo check --manifest-path examples/blog-auth/Cargo.toml --tests
```

- [ ] **Step 3 : Écrire la garde**

```rust
/// Une identité dont l'adresse est prouvée.
///
/// À poser sur les routes que vous jugez sensibles : `login` ne réclame pas la
/// vérification, et c'est ici que votre projet décide où elle devient obligatoire.
///
/// L'état est relu en base et non lu dans le jeton : le jeton d'accès porte `sub` et
/// `role`, et y mettre la vérification la figerait pour sa durée — une adresse tout juste
/// vérifiée resterait non vérifiée un quart d'heure.
///
/// ```ignore
/// pub async fn publier(identite: VerifiedIdentity, ...) -> Result<StatusCode> { ... }
/// ```
// Aucune route du fragment ne la porte, et un binaire n'exporte rien qui la tiendrait en
// vie : sans cette ligne, un projet portant `auth` ne compilerait pas sous
// `clippy -D warnings`. Elle se retire dès la première route qui l'emploie.
#[allow(dead_code)]
pub struct VerifiedIdentity(pub Identity);
```

et l'extraction :

```rust
impl<S> FromRequestParts<S> for VerifiedIdentity
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        // `Identity` rejette déjà une requête sans jeton : la garde ne répond donc jamais
        // à qui n'est pas identifié.
        let identite = Identity::from_request_parts(parts, state).await?;

        let id = Uuid::parse_str(&identite.user_id).map_err(|_| Error::Unauthorized)?;
        let etat = AppState::from_ref(state);

        // Un jeton valide dont le compte a disparu ne vaut pas mieux qu'un jeton
        // invalide : `Forbidden` laisserait entendre que le compte existe.
        let utilisateur = crate::auth::repository::find(etat.core().db(), id)
            .await?
            .ok_or(Error::Unauthorized)?;

        if utilisateur.email_verified_at.is_none() {
            return Err(Error::Forbidden);
        }

        Ok(Self(identite))
    }
}
```

Note d'exécution : la borne exacte de `Identity::from_request_parts` et la forme de `FromRequestParts` — avec ou sans `#[async_trait]` — se lisent dans `crates/rbs-core/src/`, où `Identity` est implémenté. Reprendre la forme qui y est employée plutôt que celle-ci si elles divergent.

Cet extracteur relit la base à chaque requête protégée. C'est le prix d'un état qui n'est pas figé pour la durée du jeton, et il est explicite dans la documentation de la garde.

- [ ] **Step 4 : Vérifier et commiter**

Même séquence. Message :

```
feat(auth): ajoute la garde qui exige une adresse vérifiée

`login` laisse passer un compte non vérifié : imposer la vérification à la connexion
ferait cesser l'inscription d'ouvrir une session, et demanderait à tout projet
engendré un SMTP qui marche dès le premier login. La décision appartient au projet,
et cette garde est ce avec quoi il la prend, route par route.

L'état est relu en base plutôt que porté par le jeton : l'y mettre le figerait pour
la durée du jeton, et une adresse tout juste vérifiée resterait non vérifiée un
quart d'heure.
```

---

## Lot 4 — Les sessions

### Task 10 : `GET /auth/sessions`, `DELETE /auth/sessions/{id}`, `DELETE /auth/sessions`

**Files:**
- Modify: `repository/refresh_token.rs.jinja`, `service/session.rs.jinja`, `controller/session.rs.jinja`, `dto.rs.jinja`, `mod.rs.jinja`, `tests/session.rs.jinja`, `feature.toml`
- Modify: `docs/docs/guides/auth.md` et son miroir français
- Test: `crates/rbs-cli/tests/integration_auth.rs`, plus la suite lente

**Interfaces:**
- Produces:
  - `repository::refresh_token::open_sessions_of(db, user_id: Uuid) -> Result<Vec<Model>>`
  - `repository::refresh_token::revoke_session(db, id: Uuid, user_id: Uuid) -> Result<bool>`
  - `dto::SessionResponse { id: Uuid, created_at, expires_at }`

- [ ] **Step 1 : Écrire les tests**

Dans `tests/session.rs.jinja` :

```rust
/// La liste ne montre que les sessions du seul appelant, et jamais l'empreinte.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn the_session_list_shows_only_the_callers_own() {
    let api = application().await;

    let mien = fresh_email();
    register(&api, &mien).await;
    let premiere = login(&api, &mien, PASSWORD).await;
    login(&api, &mien, PASSWORD).await;

    // Un second compte, dont les sessions ne doivent pas apparaître.
    let autre = fresh_email();
    register(&api, &autre).await;
    login(&api, &autre, PASSWORD).await;

    let (statut, corps) = call(
        &api,
        get_authenticated(
            "/auth/sessions",
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::OK);
    let sessions = corps.as_array().expect("une liste");
    assert_eq!(sessions.len(), 2, "sessions rendues : {corps}");
    assert!(
        sessions.iter().all(|session| session.get("token_hash").is_none()),
        "la liste porte l'empreinte d'un jeton : {corps}"
    );
}

/// La session d'autrui ne se révoque pas, et rend 404 plutôt que 403 : un identifiant qui
/// n'est pas le vôtre ne désigne, de votre côté, aucune session.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn revoking_someone_elses_session_is_not_found() {
    let api = application().await;

    let victime = fresh_email();
    register(&api, &victime).await;
    let sienne = login(&api, &victime, PASSWORD).await;

    let attaquant = fresh_email();
    register(&api, &attaquant).await;
    let paire = login(&api, &attaquant, PASSWORD).await;

    let (_, liste) = call(
        &api,
        get_authenticated(
            "/auth/sessions",
            sienne["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    let cible = liste[0]["id"].as_str().expect("un identifiant de session");

    let (statut, _) = call(
        &api,
        delete_authenticated(
            &format!("/auth/sessions/{cible}"),
            paire["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NOT_FOUND);

    // Et la session visée tourne toujours.
    let (encore, _) = call(
        &api,
        post_json(
            "/auth/refresh",
            json!({ "refresh_token": sienne["refresh_token"] }),
        ),
    )
    .await;
    assert_eq!(encore, StatusCode::OK);
}

/// La révocation globale ferme tout, y compris la session qui l'a demandée.
#[tokio::test]
#[ignore = "joint la base décrite par .env"]
async fn revoking_every_session_closes_them_all() {
    let api = application().await;
    let email = fresh_email();

    register(&api, &email).await;
    let premiere = login(&api, &email, PASSWORD).await;
    let seconde = login(&api, &email, PASSWORD).await;

    let (statut, _) = call(
        &api,
        delete_authenticated(
            "/auth/sessions",
            premiere["access_token"].as_str().expect("jeton d'accès"),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT);

    for paire in [&premiere, &seconde] {
        let (rejet, _) = call(
            &api,
            post_json(
                "/auth/refresh",
                json!({ "refresh_token": paire["refresh_token"] }),
            ),
        )
        .await;
        assert_eq!(rejet, StatusCode::UNAUTHORIZED);
    }
}
```

`delete_authenticated(chemin, jeton)` est une aide à ajouter à `tests/mod.rs.jinja`, sur le modèle de `get_authenticated`.

- [ ] **Step 2 : Lancer pour voir échouer**

```bash
cargo check --manifest-path examples/blog-auth/Cargo.toml --tests
```

- [ ] **Step 3 : Écrire le repository**

```rust
/// Les sessions encore ouvertes d'un compte, la plus récente d'abord.
///
/// Ni révoquées ni périmées : ce que la liste montre est ce qu'une révocation fermerait.
pub async fn open_sessions_of(db: &DatabaseConnection, user_id: Uuid) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(refresh_token::Column::UserId.eq(user_id))
        .filter(refresh_token::Column::RevokedAt.is_null())
        .filter(Expr::col(refresh_token::Column::ExpiresAt).gt(Expr::current_timestamp()))
        .order_by_desc(refresh_token::Column::CreatedAt)
        .all(db)
        .await?)
}

/// Ferme une session nommée, et dit si elle appartenait bien au compte.
///
/// Le propriétaire est dans la condition de l'`UPDATE` et non dans une lecture qui le
/// précède : comparer après avoir lu laisserait la révocation de la session d'autrui à
/// portée d'une course.
pub async fn revoke_session(db: &DatabaseConnection, id: Uuid, user_id: Uuid) -> Result<bool> {
    let touchees = Entity::update_many()
        .col_expr(refresh_token::Column::RevokedAt, Expr::current_timestamp().into())
        .filter(refresh_token::Column::Id.eq(id))
        .filter(refresh_token::Column::UserId.eq(user_id))
        .filter(refresh_token::Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected == 1)
}
```

- [ ] **Step 4 : DTO, service, contrôleurs**

```rust
/// La vue publique d'une session.
///
/// Jamais `token_hash` : la vue d'une session n'a aucune raison de porter de quoi la
/// présenter.
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionResponse {
    pub id: Uuid,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
    #[schema(value_type = String, format = DateTime)]
    pub expires_at: DateTimeWithTimeZone,
}
```

Les trois handlers prennent `Identity`, analysent `sub` en `Uuid` comme `me` le fait, et délèguent. `revoke_session` rend `Error::NotFound("session")` quand le repository rend `false`.

- [ ] **Step 5 : Monter, inscrire, documenter, vérifier, commiter**

Trois routes dans `mod.rs.jinja`, trois `operation_id` dans l'ancre `openapi`, les deux `guides/auth.md`. Puis la séquence complète, suite lente comprise.

```
feat(auth): ajoute la liste et la révocation des sessions

Le titulaire voit ce qui est ouvert en son nom et peut le fermer, session par
session ou d'un coup. La vue ne porte jamais l'empreinte du jeton : elle n'a aucune
raison de contenir de quoi présenter la session qu'elle décrit.

La révocation nommée porte le propriétaire dans la condition de l'UPDATE. Lire la
ligne puis comparer laisserait la révocation de la session d'autrui à portée d'une
course.
```

---

### Task 11 : La passe finale

**Files:**
- Modify: `docs/docs/cli/add.md`, `docs/docs/tutorials/auth.md` et leurs deux miroirs français
- Modify: `examples/blog-auth/**` (régénération finale, par diff)
- Test: la suite entière

- [ ] **Step 1 : Mettre à jour la toolchain**

```bash
rustup update
```

Clippy vert en local ne dit rien de la version que prend `@stable` en CI.

- [ ] **Step 2 : Régénérer les trois exemples porteurs**

`blog-auth` (auth), `file-drop` et `newsletter-queue` (mail). Annexe A pour chacun.

- [ ] **Step 3 : Documenter dans `cli/add.md` et `tutorials/auth.md`**

`docs/docs/cli/add.md` et son miroir français : `rbs add auth` installe désormais `rate-limit` **et** `mail`, et dépose treize routes. Vérifier chaque transcription de sortie du CLI que la page cite — `integration_docs` les garde, et aucun des guides ne l'est.

`docs/docs/tutorials/auth.md` et son miroir : le tutoriel déroule le parcours d'authentification pas à pas et s'arrête aujourd'hui à `me`. Y ajouter les trois parcours neufs — changement, réinitialisation, vérification — et signaler que Mailpit, qui arrive désormais avec `auth`, montre sur `http://localhost:8025` les courriels que le projet croit envoyer. C'est ce qui rend le tutoriel exécutable sans compte SMTP.

- [ ] **Step 4 : La passe complète**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace > "$SCRATCHPAD/final-rapide.txt" 2>&1
cargo test -p rbs-cli --no-fail-fast -- --ignored > "$SCRATCHPAD/final-lent.txt" 2>&1
tail -40 "$SCRATCHPAD/final-rapide.txt" "$SCRATCHPAD/final-lent.txt"
```

Attendu : 0 échec partout. Consigner les chiffres réels dans le message de commit.

- [ ] **Step 5 : Vérifier qu'aucune route n'a été oubliée**

```bash
grep -c 'route("/auth' examples/blog-auth/src/auth/mod.rs
```

Attendu : `13`.

- [ ] **Step 6 : Commit**

```bash
git add -A
git commit -F - <<'EOF'
docs: met la page d'add et le tutoriel au niveau des treize routes

La page d'`add` annonçait cinq routes et une seule dépendance : les deux chiffres
ont changé, et la transcription qu'elle cite est gardée par un test — la laisser
périmer aurait fait mentir la documentation autant que rougir la CI.

Le tutoriel s'arrêtait à `me`. Il déroule maintenant les trois parcours neufs, et
dit où les lire : Mailpit arrive avec `auth` et montre sur 8025 les courriels que le
projet croit envoyer, ce qui rend le tutoriel exécutable sans compte SMTP.

Vérifications :
- cargo test --workspace : N passés, 0 échec
- cargo test -p rbs-cli --no-fail-fast -- --ignored : N passés, 0 échec
- clippy -D warnings et fmt --check : propres
EOF
```

---

## Annexe A — Régénérer `examples/blog-auth` sans perdre les éditions

L'exemple porte une édition à la main, `src/posts/tests.rs`, que `integration_examples.rs` déclare. **Ne jamais écraser le répertoire.**

```bash
# Un seul bloc, à copier tel quel. RS est la racine du dépôt ; TMP le répertoire jetable.
# Le `git commit` ci-dessous DOIT s'exécuter dans TMP : lancé à la racine du dépôt, il
# commite le travail en cours sous le message « projet neuf ». C'est arrivé.
RS=/Users/yacoubakone/dev/rs
TMP=$(mktemp -d)

(
  cd "$TMP" || exit 1
  cargo run --manifest-path "$RS/Cargo.toml" -p rbs-cli --bin rbs -- \
    new blog-auth --yes \
    --core-path "$RS/crates/rbs-core" \
    --database-url 'postgres://rbs:rbs@localhost:5432/blog_auth' \
    --lang fr

  cd "$TMP/blog-auth" || exit 1
  # Le garde-fou : si on n'est pas dans TMP, on ne commite rien.
  case "$PWD" in "$TMP"/*) ;; *) echo "REFUS : $PWD n'est pas sous $TMP" >&2; exit 1;; esac
  # L'identité passe par `-c`, jamais par `git config --local` : posée en configuration,
  # elle atterrit sur le dépôt rbs si le `cd` n'a pas pris, et tous les commits suivants
  # de la branche portent son auteur. Trois l'ont porté.
  git add -A
  git -c user.name='rbs' -c user.email='rbs@exemple.test' commit -q -m 'projet neuf'

  cargo run --manifest-path "$RS/Cargo.toml" -p rbs-cli --bin rbs -- add auth
  cargo run --manifest-path "$RS/Cargo.toml" -p rbs-cli --bin rbs -- \
    generate crud posts --fields 'title:string,body:text,published:bool' --role admin --force
)

# La comparaison, depuis la racine du dépôt.
diff -ru "$RS/examples/blog-auth" "$TMP/blog-auth"
```

Reporter les différences à la main sur `examples/blog-auth`, en laissant `src/posts/tests.rs` tel qu'il est. Les sous-shell et le garde-fou `case` ne sont pas décoratifs : un `cd` qui ne prend pas effet fait commiter le travail en cours du dépôt sous le message « projet neuf ». `--core-path` pointe vers la crate locale : sans lui l'exemple dépendrait du `rbs-core` publié, et ne verrait aucun changement du dépôt.

L'oracle est `cargo test -p rbs-cli --test integration_examples`, qui compare octet à octet. Tant qu'il est rouge, le report n'est pas fini. `file-drop` et `newsletter-queue` se régénèrent par les commandes que leur donne `examples/README.md`, même méthode.

## Annexe B — Où les tests vivent, et lesquels prouvent quoi

| Suite | Commande | Ce qu'elle prouve | Docker |
|---|---|---|---|
| Rendu du fragment | `cargo test -p rbs-cli --test integration_auth` | ce que `add auth` dépose, ancres comprises | non |
| Non-dérive des exemples | `cargo test -p rbs-cli --test integration_examples` | que les exemples versionnés valent ce que les templates rendent | non |
| Transcriptions de la doc | `cargo test -p rbs-cli --test integration_docs` | que les sorties de CLI citées par `docs/` sont exactes | non |
| Parcours réels | `cargo test -p rbs-cli --no-fail-fast -- --ignored` | que le projet engendré compile **et** répond contre PostgreSQL | oui |
| Tests du fragment | `src/auth/tests/*.rs` du projet engendré | les invariants du module | oui |

Les tests déposés dans `examples/` ne tournent jamais dans la suite du dépôt : les prouver exige un PostgreSQL monté à la main et `--include-ignored`.
