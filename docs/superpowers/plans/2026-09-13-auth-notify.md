# Sortir l'orchestration courriel des contrôleurs `auth` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** aucun contrôleur du fragment `auth` n'enchaîne plus « ouvrir un jeton » puis « envoyer le courriel » : un helper `notify` de la couche service porte le seul envoi, et `register`, `forgot-password` et `resend-verification` n'appellent plus qu'une fonction de service chacun — sans rien changer à ce que voit le client.

**Architecture :** `service/mod.rs` gagne `notify(mail, destinataire, objet, gabarit, contexte)`, qui appelle `send_template_detached` et journalise l'échec avec `user_id`. `verification::send_link` et `password::send_reset_link` enveloppent `request` / `request_reset` (inchangées : les tests du projet s'en servent pour obtenir le jeton en clair) et appellent `notify`. `session::register` reçoit le client mail et `FlowConfig` et appelle `verification::send_link` une fois le compte créé. Les contrôleurs passent `state.mail()` et `state.flows()` comme ils passent déjà `state.core().db()` et `state.auth()`.

**Tech Stack :** gabarits minijinja (délimiteurs `{@ @}` / `{% %}` — aucun dans les fichiers touchés), Rust/axum/SeaORM dans le projet engendré, `lettre` via le `Mailer` du fragment `mail`.

**Spec :** design validé par le mainteneur, transmis avec la tâche — constat du 2026-09-13 : `controller/session.rs.jinja:37-56`, `controller/verification.rs.jinja:30-50`, `controller/password.rs.jinja:56-80`.

## Global Constraints

- Aucun changement de comportement visible du client : `register` rend toujours 201 même si le courriel échoue, `forgot-password` et `resend-verification` toujours 202 ; mêmes objets (`"Confirmez votre adresse"`, `"Réinitialisation de votre mot de passe"`), mêmes gabarits (`verification.html`, `reinitialisation.html`), mêmes liens (`flows.link("verify-email", …)`, `flows.link("reset-password", …)`, `heures => flows.reset_ttl_secs / 3600`).
- **Un seul** commentaire explique pourquoi l'échec d'envoi ne se propage pas (fusion des trois : contrat 201/202, oracle d'énumération), dans `notify`.
- Aucune chaîne visible du client modifiée (une autre branche traduit `ADRESSE_PRISE`, `NotFound("session")` et les descriptions OpenAPI) : ne toucher ni la ligne `Error::Conflict(ADRESSE_PRISE…)`, ni la ligne `use super::super::repository::{self, ADRESSE_PRISE, …}`, ni les attributs `#[utoipa::path]`.
- Hors périmètre, à noter seulement : déplacer les écritures en base dans la tâche détachée, `email_verified_at`, la purge des jetons.
- `examples/blog-auth/src/auth/controller/session.rs` sous 200 lignes ; aucun fichier de feature touché au-delà de ~200 lignes.
- Ne pas toucher `CHANGELOG*.md`, `IMPROVE.md`, `TODO.md`, `ROADMAP.md`, `crates/rbs-cli/templates/agents/*`.
- Commits : Conventional Commits, sujet français à l'impératif, corps = pourquoi + `Vérifications :` ; jamais de `Co-Authored-By`, `Claude-Session`, identifiant de tâche ni renvoi à un fichier de suivi.
- Documentation bilingue : toute page EN modifiée l'est en FR dans le même commit.

---

### Task 1 : l'envoi descend dans la couche service

**Files :**
- Modify : `crates/rbs-cli/templates/features/auth/service/mod.rs.jinja` (en-tête, imports, `notify`, `session_view`)
- Modify : `crates/rbs-cli/templates/features/auth/service/verification.rs.jinja` (imports, doc de `request`, `send_link`)
- Modify : `crates/rbs-cli/templates/features/auth/service/password.rs.jinja` (imports, doc de `request_reset`, `send_reset_link`)
- Modify : `crates/rbs-cli/templates/features/auth/service/session.rs.jinja` (imports, signature de `register`, `session_view` retiré)
- Modify : `crates/rbs-cli/templates/features/auth/controller/{session,verification,password}.rs.jinja` (corps de `register`, `resend_verification`, `forgot_password`)
- Test : `crates/rbs-cli/src/templates.rs` (module `tests`, après `no_conflict_of_the_auth_fragment_echoes_the_address_it_refuses`)
- Test : `crates/rbs-cli/templates/features/auth/tests/{verification,password}.rs.jinja` (deux tests de caractérisation, exécutés seulement par la passe Docker)

**Interfaces :**
- Produces (dans le projet engendré, module `crate::auth::service`) :
  - `pub(super) fn notify(mail: &Mailer, destinataire: &Model, objet: &str, gabarit: &str, contexte: impl Serialize)`
  - `pub async fn verification::send_link(db: &DatabaseConnection, mail: &Mailer, flows: &FlowConfig, email: &str) -> Result<()>`
  - `pub async fn password::send_reset_link(db: &DatabaseConnection, mail: &Mailer, flows: &FlowConfig, email: &str) -> Result<()>`
  - `pub async fn register(db: &DatabaseConnection, mail: &Mailer, flows: &FlowConfig, input: RegisterRequest) -> Result<UserResponse>` (réexporté par `service`)
  - `fn session_view(session: repository::refresh_token::Model) -> SessionResponse` déplacée de `service/session.rs` vers `service/mod.rs`
  - où `Mailer` = `crate::modules::mail::Mailer`, `FlowConfig` = `super::super::config::FlowConfig` (depuis `service/*`), `Model` = `super::repository::Model`.

`session_view` descend dans `mod.rs` à côté de `profile()`, l'autre passage du modèle vers la réponse — sa doc le cite déjà (« Même règle que `profile()` ») — pour que `service/session.rs`, à 198 lignes, reste sous 200 après avoir reçu les paramètres de `register`. On ne déplace pas `register` : son corps porte `ADRESSE_PRISE`, que l'autre branche touche.

- [ ] **Step 1 : écrire le test de structure qui échoue**

Dans `crates/rbs-cli/src/templates.rs`, module `tests`, juste après la fonction `no_conflict_of_the_auth_fragment_echoes_the_address_it_refuses` :

```rust
    /// Le courriel du fragment `auth` part de la couche service, et d'un seul endroit.
    ///
    /// Un contrôleur qui ouvre le jeton puis envoie le courriel enchaîne deux services, ce
    /// que la dépendance des couches réserve au service — et chaque copie de l'envoi
    /// portait sa propre raison de ne pas propager l'échec.
    #[test]
    fn the_auth_mail_leaves_from_the_service_layer_only() {
        let racine = Path::new(RACINE_FEATURES).join("auth");

        for controleur in ["session", "password", "verification"] {
            let source = read(&racine.join(format!("controller/{controleur}.rs.jinja")));

            for orchestration in ["send_template_detached", "::request(", "request_reset("] {
                assert!(
                    !source.contains(orchestration),
                    "controller/{controleur}.rs appelle `{orchestration}` :\n{source}"
                );
            }
        }

        let envois: usize = ["mod", "session", "password", "verification"]
            .into_iter()
            .map(|parcours| {
                read(&racine.join(format!("service/{parcours}.rs.jinja")))
                    .matches("send_template_detached(")
                    .count()
            })
            .sum();
        assert_eq!(envois, 1, "l'envoi doit passer par le seul `notify`");

        let commun = read(&racine.join("service/mod.rs.jinja"));
        assert!(
            commun.contains("pub(super) fn notify("),
            "service/mod.rs ne porte pas `notify` :\n{commun}"
        );
    }
```

- [ ] **Step 2 : le lancer, le voir échouer**

Run : `cargo test -p rbs-cli --lib the_auth_mail_leaves_from_the_service_layer_only`
Expected : FAIL — `controller/session.rs appelle \`send_template_detached\``.

- [ ] **Step 3 : `service/mod.rs.jinja`**

En-tête du module (remplace les lignes 3-5) :

```rust
//! `issue()`, `profile()` et `notify()` vivent ici plutôt que dans l'un des parcours :
//! plusieurs s'en servent, et les descendre dans l'un d'eux ferait dépendre les autres de
//! ce voisin-là. `session_view()` rejoint `profile()`, l'autre passage du modèle vers la
//! réponse.
```

Imports : ajouter `use serde::Serialize;` après `use sea_orm::prelude::Uuid;`, remplacer `use super::dto::{TokenPair, UserResponse};` par `use super::dto::{SessionResponse, TokenPair, UserResponse};`, et ajouter `use crate::modules::mail::Mailer;` après `use super::repository::{self, Model};`.

En fin de fichier, après `profile()` :

```rust

/// La vue publique d'une session.
///
/// Même règle que `profile()` pour `UserResponse` : `SessionResponse` ne porte pas
/// `token_hash`, et cette fonction est le seul passage du modèle vers la réponse.
fn session_view(session: repository::refresh_token::Model) -> SessionResponse {
    SessionResponse {
        id: session.id,
        created_at: session.created_at,
        expires_at: session.expires_at,
    }
}

/// Envoie un courriel à un compte, sans que son échec atteigne la réponse.
pub(super) fn notify(
    mail: &Mailer,
    destinataire: &Model,
    objet: &str,
    gabarit: &str,
    contexte: impl Serialize,
) {
    // `send_template_detached` rend le gabarit sur-le-champ et peut donc échouer —
    // gabarit absent, mal formé, ou adresse que `lettre` refuse d'analyser. L'échec reste
    // au journal, pour deux raisons. Ce qui précède l'envoi est écrit et ne se défait
    // plus : le propager rendrait un 500 après coup, là où `register` promet « toujours
    // 201 ». Et on n'arrive ici que si l'adresse porte un compte : un 500 ferait de cette
    // branche la seule à se distinguer, et dirait à qui essaie plusieurs adresses
    // lesquelles sont inscrites — ce que le 202 de `forgot-password` et de
    // `resend-verification` existe pour taire. Ces deux routes sont aussi le rattrapage
    // d'un courriel qui ne part pas.
    if let Err(error) = mail.send_template_detached(&destinataire.email, objet, gabarit, contexte)
    {
        tracing::error!(
            user_id = %destinataire.id,
            gabarit,
            %error,
            "envoi du courriel échoué"
        );
    }
}
```

(La mise en forme exacte de l'`if let` est celle que rend `cargo fmt` sur l'exemple, Task 2 : la reporter à l'identique dans le gabarit.)

- [ ] **Step 4 : `service/verification.rs.jinja`**

Imports (remplacent le second groupe) :

```rust
use super::super::config::FlowConfig;
use super::super::model::TokenPurpose;
use super::super::repository;
use super::{normalise, notify};
use crate::modules::mail::Mailer;
```

Doc de `request` (remplace les lignes 9-14) :

```rust
/// Ouvre un jeton de vérification, et rend le compte avec le jeton **en clair**.
///
/// Le jeton sort en clair pour `send_link`, qui le met dans un courriel, et pour les tests
/// du projet, qui déroulent le parcours sans qu'aucun SMTP soit joignable.
```

Après `request`, avant `verify` :

```rust
/// Envoie un lien de vérification neuf, si un compte porte l'adresse.
///
/// Une seule fonction pour l'inscription et pour le renvoi : les deux partent d'une
/// adresse. Une adresse inconnue ne reçoit rien, et l'appelant n'en sait rien.
pub async fn send_link(
    db: &DatabaseConnection,
    mail: &Mailer,
    flows: &FlowConfig,
    email: &str,
) -> Result<()> {
    if let Some((utilisateur, jeton)) = request(db, flows.verification_ttl_secs, email).await? {
        notify(
            mail,
            &utilisateur,
            "Confirmez votre adresse",
            "verification.html",
            minijinja::context! { link => flows.link("verify-email", &jeton) },
        );
    }

    Ok(())
}
```

- [ ] **Step 5 : `service/password.rs.jinja`**

Imports : ajouter `use super::super::config::FlowConfig;` en tête du second groupe, remplacer `use super::{close_every_session, issue, normalise};` par `use super::{close_every_session, issue, normalise, notify};`, ajouter `use crate::modules::mail::Mailer;` en fin de groupe.

Doc de `request_reset`, ligne 83 : « Le rendre ici est ce qui permet au contrôleur de le mettre dans un courriel » → « Le rendre ici est ce qui permet à `send_reset_link` de le mettre dans un courriel » (reste du paragraphe inchangé, rewrap à 90 colonnes).

Après `request_reset`, avant `reset` :

```rust
/// Envoie un lien de réinitialisation neuf, si un compte porte l'adresse.
pub async fn send_reset_link(
    db: &DatabaseConnection,
    mail: &Mailer,
    flows: &FlowConfig,
    email: &str,
) -> Result<()> {
    if let Some((utilisateur, jeton)) = request_reset(db, flows.reset_ttl_secs, email).await? {
        notify(
            mail,
            &utilisateur,
            "Réinitialisation de votre mot de passe",
            "reinitialisation.html",
            minijinja::context! {
                link => flows.link("reset-password", &jeton),
                heures => flows.reset_ttl_secs / 3600,
            },
        );
    }

    Ok(())
}
```

- [ ] **Step 6 : `service/session.rs.jinja`**

Imports : ajouter `use super::super::config::FlowConfig;` avant `use super::super::dto::{`, remplacer `use super::{close_every_session, issue, normalise, profile};` par `use super::{close_every_session, issue, normalise, profile, session_view};`, ajouter `use crate::modules::mail::Mailer;` après. La ligne `use super::super::repository::{self, ADRESSE_PRISE, refresh_token::Rotation};` ne bouge pas.

`register` :

```rust
pub async fn register(
    db: &DatabaseConnection,
    mail: &Mailer,
    flows: &FlowConfig,
    input: RegisterRequest,
) -> Result<UserResponse> {
    let email = normalise(&input.email);

    if repository::find_by_email(db, &email).await?.is_some() {
        return Err(Error::Conflict(ADRESSE_PRISE.to_owned()));
    }

    let hash = hash::hash_password(&input.password)?;
    let cree = repository::create(db, &email, &hash).await?;
    super::verification::send_link(db, mail, flows, &cree.email).await?;

    Ok(profile(cree))
}
```

Supprimer `session_view` et sa doc (lignes 166-176) ainsi que la ligne vide qui suit.

- [ ] **Step 7 : les trois contrôleurs**

`controller/session.rs.jinja`, corps de `register` :

```rust
) -> Result<(StatusCode, Json<UserResponse>)> {
    let cree = service::register(state.core().db(), state.mail(), state.flows(), input).await?;

    Ok((StatusCode::CREATED, Json(cree)))
}
```

`controller/verification.rs.jinja`, corps de `resend_verification` :

```rust
) -> Result<StatusCode> {
    service::verification::send_link(state.core().db(), state.mail(), state.flows(), &input.email)
        .await?;

    Ok(StatusCode::ACCEPTED)
}
```

`controller/password.rs.jinja`, corps de `forgot_password` :

```rust
) -> Result<StatusCode> {
    service::password::send_reset_link(
        state.core().db(),
        state.mail(),
        state.flows(),
        &input.email,
    )
    .await?;

    Ok(StatusCode::ACCEPTED)
}
```

Les commentaires au-dessus des `#[utoipa::path]` de `forgot_password` et `resend_verification` (statut 202, envoi détaché, temps de réponse) restent : ils décrivent la route. La mise en forme exacte est celle de `cargo fmt` sur l'exemple (Task 2).

- [ ] **Step 8 : deux tests de caractérisation dans le fragment**

Aucun test HTTP ne passe aujourd'hui par la branche qui envoie de `forgot-password` ni de `resend-verification` (seules les adresses inconnues sont testées). Ils tournent dans la passe Docker (`integration_auth`), pas avant.

`tests/verification.rs.jinja`, après `resending_to_an_unknown_address_is_accepted` :

```rust

/// Une adresse inscrite rend le même 202, et le renvoi émet un jeton neuf.
///
/// C'est la branche qui envoie le courriel : un échec d'envoi y rendrait un 500 que
/// l'adresse inconnue ne rend jamais, et l'écart dirait lesquelles sont inscrites.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn resending_to_a_registered_address_is_accepted() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;

    let (statut, _) = call(
        &api,
        post_json("/auth/resend-verification", json!({ "email": email })),
    )
    .await;

    assert_eq!(statut, StatusCode::ACCEPTED);

    let compte = crate::auth::repository::find_by_email(&db, &email)
        .await
        .expect("la lecture aboutit")
        .expect("le compte vient d'être créé");

    // Deux lignes : celle de l'inscription, que le renvoi a close, et la neuve.
    assert_eq!(
        one_time_tokens_count_for(&db, compte.id).await,
        2,
        "le renvoi n'a ouvert aucun jeton"
    );
}
```

`tests/password.rs.jinja`, après `forgetting_an_unknown_address_is_accepted_and_writes_nothing` :

```rust

/// Une adresse inscrite rend le même 202 qu'une adresse inconnue, et un jeton est émis.
///
/// C'est la branche qui envoie le courriel : un échec d'envoi y rendrait un 500 que
/// l'adresse inconnue ne rend jamais, et l'écart dirait lesquelles sont inscrites.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn forgetting_a_registered_address_is_accepted_and_opens_a_token() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();

    register(&api, &email).await;
    let compte = crate::auth::repository::find_by_email(&db, &email)
        .await
        .expect("la lecture aboutit")
        .expect("le compte vient d'être créé");
    let avant = one_time_tokens_count_for(&db, compte.id).await;

    let (statut, _) = call(
        &api,
        post_json("/auth/forgot-password", json!({ "email": email })),
    )
    .await;

    assert_eq!(statut, StatusCode::ACCEPTED);
    assert_eq!(
        one_time_tokens_count_for(&db, compte.id).await,
        avant + 1,
        "la demande n'a ouvert aucun jeton de réinitialisation"
    );
}
```

(`invalidate_pending` pose `consumed_at` sans supprimer : d'où 2 lignes après le renvoi. Le limiteur de débit n'impute rien sans `ConnectInfo`, que le routeur des tests ne pose pas : pas de 429.)

- [ ] **Step 9 : tests rapides**

Run : `cargo test -p rbs-cli --lib` → tous verts, dont `the_auth_mail_leaves_from_the_service_layer_only`, `every_auth_test_joining_the_database_is_ignored`, `no_conflict_of_the_auth_fragment_echoes_the_address_it_refuses`.
Run : `grep -rn 'send_template_detached\|request_reset(\|::request(' crates/rbs-cli/tests/` → aucun littéral figé sur l'ancien code.

La compilation du code engendré n'est prouvée qu'en Task 2 (`cargo check` sur l'exemple) : ne pas committer avant, les deux tâches se committent séparément mais Task 1 n'est validée qu'avec la preuve de Task 2.

- [ ] **Step 10 : commit**

```bash
git add crates/rbs-cli/src/templates.rs crates/rbs-cli/templates/features/auth
git commit -m "refactor(auth): envoie les courriels depuis la couche service" -m "<pourquoi + Vérifications :>"
```

### Task 2 : reporter le changement sur `examples/blog-auth`

**Files :**
- Modify : `examples/blog-auth/src/auth/{controller,service,tests}/…` — par diff entre deux générations, jamais par écrasement (régions `// region:` à préserver).

- [ ] **Step 1 : génération « avant »** (CLI construit sur la base, avant Task 1) sous `target/regen/avant/`, commandes de `examples/README.md` section `blog-auth`, binaire copié dans le scratchpad (`rbs-avant`).
- [ ] **Step 2 : génération « après »** avec le CLI reconstruit après Task 1, sous `target/regen/apres/`, mêmes commandes.
- [ ] **Step 3 : diff** — `diff -ruN -x .git -x target -x migration -x .env avant/blog-auth apres/blog-auth` : attendu, seulement `src/auth/controller/*`, `src/auth/service/*`, `src/auth/tests/{password,verification}.rs`. Les migrations (horodatage) et `.env` (secret tiré) diffèrent par construction et sont exclus.
- [ ] **Step 4 : appliquer** le diff dans `examples/blog-auth` (`patch -p2`), vérifier que les marqueurs `// region:` survivent ; reporter à la main tout hunk rejeté.
- [ ] **Step 5 : compiler l'exemple** — dans `examples/blog-auth` : `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`. Si `cargo fmt` reformate, reporter la forme exacte dans les gabarits (Task 1) et régénérer.
- [ ] **Step 6 : oracle** — `cargo test -p rbs-cli --test integration_examples` vert.
- [ ] **Step 7 : `wc -l`** des trois contrôleurs et des quatre services de `examples/blog-auth/src/auth` : `controller/session.rs` < 200, aucun fichier au-delà de ~200.
- [ ] **Step 8 : commit** `chore(examples): …`.

### Task 3 : documentation (EN + FR, même commit)

**Files :**
- Modify : `docs/docs/guides/auth.md`, `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/auth.md`
- Modify : `examples/blog-auth/src/auth/service/mod.rs` (région `notify`), `examples/blog-auth/src/auth/service/verification.rs` (région `send_link`), `examples/blog-auth/src/auth/controller/session.rs` (région `register` retirée si plus citée)

- [ ] **Step 1 :** le paragraphe qui suit la région `forgot_password` dit « a missing one fails the request » — faux aujourd'hui déjà, le contrôleur avale l'erreur. Le récrire : le contrôleur passe l'adresse à `service::password::send_reset_link`, l'envoi passe par `notify`, seul point d'envoi de la feature ; `send_template_detached` rend le gabarit tout de suite et détache la livraison ; un rendu qui échoue est journalisé avec l'identifiant du compte et n'atteint jamais la réponse. Citer ```` ```rust file=examples/blog-auth/src/auth/service/mod.rs region=notify ````.
- [ ] **Step 2 :** le paragraphe « Registering and resending share one service function… » cite la région `register` du contrôleur, qui ne montre plus le partage. Le récrire autour de `verification::send_link` et citer ```` ```rust file=examples/blog-auth/src/auth/service/verification.rs region=send_link ````. Retirer les marqueurs `register` du contrôleur s'ils ne sont plus cités (`grep -rn 'region=register' docs/`).
- [ ] **Step 3 :** poser les marqueurs `// region: notify` / `// endregion: notify` et `// region: send_link` / `// endregion: send_link` dans l'exemple (doc comprise).
- [ ] **Step 4 :** `cd docs && npm ci && npm test && npm run build` ; `cargo test -p rbs-cli --test integration_docs` ; `cargo test -p rbs-cli --test integration_examples`.
- [ ] **Step 5 : commit** `docs(auth): …`.

### Task 4 : vérification finale

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test -p rbs-cli --lib`
- [ ] `cargo test -p rbs-cli --test integration_examples`
- [ ] Seule, en arrière-plan, sortie vers le scratchpad : `cargo test -p rbs-cli --test integration_auth --no-fail-fast -- --include-ignored` (~500 s) — les deux tests de caractérisation doivent y apparaître et passer.
- [ ] `wc -l` final des contrôleurs et services de `examples/blog-auth/src/auth`.
