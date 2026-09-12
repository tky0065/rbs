# Auth : le jeton d'accès meurt avec les sessions — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Après un reset, un change-password ou `DELETE /auth/sessions`, tout jeton d'accès émis avant est refusé ; un jeton dont le rôle ne correspond plus à la ligne du compte l'est aussi.

**Architecture:** `rbs-core::HasAuth` gagne une méthode fournie `accept(&claims)` (défaut : accepte), qu'`Identity` appelle après `jwt::verify`. Le fragment `auth` la surcharge dans `impl HasAuth for AppState` : lecture de `users`, refus si le compte manque, si `iat <= users.sessions_revoked_at`, ou si le rôle diffère. `issue()` émet à `max(maintenant, sessions_revoked_at + 1 s)` pour que la paire rendue par `change-password` vive. Les tests qui forgeaient des jetons pour des comptes inexistants créent désormais un compte.

**Tech Stack:** Rust 2024 (`impl Future + Send` en méthode fournie de trait), axum `FromRequestParts`, SeaORM, sea-orm-migration, chrono, minijinja (`{@ @}` dans `feature/tests.rs.jinja`, qui porte des tags `{%- if auth %}`).

**Spec:** `docs/superpowers/specs/2026-09-12-lot-secu-p2-design.md`, section 3.

## Global Constraints

- Ce plan suit `2026-09-12-auth-rotation-vs-fermeture.md` sur la même branche : `refresh_tokens.replaced_at`, `rotate`, `close` existent déjà.
- `rbs-core` change : version `1.5.0` dans `Cargo.toml` racine (`[workspace.package] version`), et l'ajout est **additif** (méthode fournie) — aucun `impl HasAuth for AppState {}` existant ne cesse de compiler.
- Toute date écrite ou comparée vient de Rust, liée en paramètre.
- `examples/blog-auth` régénéré par diff ; `integration_examples` est l'oracle. `examples/blog-auth/src/posts/tests.rs` change (le `token()` forgé) : il vient de `templates/feature/tests.rs.jinja`.
- Passe lente : `cargo test -p rbs-cli --test integration_auth --no-fail-fast -- --ignored > $SCRATCHPAD/auth-lent-11.log 2>&1` — elle joue aussi `the_tests_of_a_crud_generated_under_auth_pass`, qui prouve le nouveau `token()`.
- Commits Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.

---

### Task 1: Le crochet dans `rbs-core`

**Files:**
- Modify: `crates/rbs-core/src/state.rs:72-78` (`HasAuth::accept`)
- Modify: `crates/rbs-core/src/extract.rs:50` (`Identity` appelle `accept`)
- Modify: `Cargo.toml` racine (`version = "1.5.0"`), `Cargo.lock` suit
- Test: `crates/rbs-core/src/extract.rs` (module `tests`)

**Interfaces:**
- Produces: `fn accept(&self, claims: &crate::jwt::Claims) -> impl Future<Output = Result<(), Error>> + Send` sur `HasAuth`, corps par défaut `async { Ok(()) }`.

- [ ] **Step 1: Test rouge dans `extract.rs`** (module `tests`, derrière `#[cfg(feature = "auth")]` comme les tests `Identity` voisins — lire d'abord ceux-ci, lignes ~200-300, pour reprendre leur `state()` et leur signature de jeton) :

```rust
    /// Un état qui refuse tout jeton, quelle que soit sa signature.
    #[cfg(feature = "auth")]
    #[derive(Clone)]
    struct Refusant(Arc<CoreState>);

    #[cfg(feature = "auth")]
    impl HasCoreState for Refusant {
        fn core(&self) -> &CoreState {
            &self.0
        }
    }

    #[cfg(feature = "auth")]
    impl HasAuth for Refusant {
        fn accept(&self, _: &crate::jwt::Claims) -> impl Future<Output = Result<(), Error>> + Send {
            async { Err(Error::Unauthorized) }
        }
    }

    #[cfg(feature = "auth")]
    #[tokio::test]
    async fn a_state_that_refuses_a_claim_turns_a_signed_token_into_401() {
        async fn handler(identite: Identity) -> String {
            identite.user_id
        }

        let etat = Refusant(Arc::new(state_with_auth()));
        let jeton = signed_token(&etat.0); // reprendre l'aide existante du module

        let response = Router::new()
            .route("/", get(handler))
            .with_state(etat)
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(header::AUTHORIZATION, format!("Bearer {jeton}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
```

Les noms `state_with_auth` et `signed_token` désignent ce que le module de tests d'`extract.rs` fournit déjà pour ses tests `Identity` (vers la ligne 237, `env: "development"`) ; reprendre les noms réels.

- [ ] **Step 2: `HasAuth::accept`**

```rust
#[cfg(feature = "auth")]
pub trait HasAuth: HasCoreState {
    /// Configuration d'authentification portée par cet état.
    fn auth(&self) -> &crate::config::AuthConfig {
        &self.core().config().auth
    }

    /// Dernier mot du projet sur un jeton dont la signature est bonne.
    ///
    /// Le noyau ne connaît ni la table des comptes ni ce qu'une révocation y écrit : il
    /// vérifie la signature, puis demande. Le défaut accepte tout, et c'est ce qu'un
    /// projet sans révocation obtient sans rien écrire.
    fn accept(
        &self,
        claims: &crate::jwt::Claims,
    ) -> impl std::future::Future<Output = Result<(), crate::Error>> + Send {
        let _ = claims;
        async { Ok(()) }
    }
}
```

`extract.rs:50` :

```rust
        let claims = crate::jwt::verify(token, &state.auth().secret)?;
        state.accept(&claims).await?;
```

- [ ] **Step 3: `cargo test -p rbs-core --all-features`** → vert, dont le test neuf ; `cargo clippy -p rbs-core --all-targets --all-features -- -D warnings` → 0 (si `async_fn_in_trait` ou `manual_async_fn` se plaint, garder la forme `impl Future + Send`, qui est celle que le lint recommande pour un trait public).

- [ ] **Step 4: Version** — `Cargo.toml` racine : `version = "1.5.0"`. `cargo build --workspace` pour rafraîchir `Cargo.lock`. Vérifier `grep -rn '1\.4\.0' README.md README.fr.md docs/docs --include=*.md | head` : les mentions masquées par `integration_docs` ne se touchent pas ; celles qui affirment « Version 1.4.0 » se mettent à jour (tâche 27 du backlog dit que README affiche 1.2.0 : ne pas la corriger ici, la noter).

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-core Cargo.toml Cargo.lock
git commit -m "feat(core): laisse le projet refuser un jeton signé par HasAuth::accept"
```

---

### Task 2: Schéma, dépôt et estampille

**Files:**
- Modify: `migration.rs.jinja` (colonne `Users::SessionsRevokedAt` après `EmailVerifiedAt`)
- Modify: `model.rs.jinja` (champ `sessions_revoked_at` sur `user::Model`)
- Modify: `repository/user.rs.jinja` (`stamp_sessions_revoked`)
- Modify: `service/mod.rs.jinja` (`close_every_session`, `issue` avec `iat` borné)
- Modify: `service/session.rs.jinja`, `service/password.rs.jinja` (les quatre appels)

**Interfaces:**
- Produces: `repository::user::stamp_sessions_revoked(db, id) -> Result<DateTimeWithTimeZone>`, `service::close_every_session(db, user_id) -> Result<u64>`.

- [ ] **Step 1: Migration et modèle**

```rust
                    // Estampillée à chaque fermeture de toutes les sessions : un jeton
                    // d'accès émis avant, signature bonne ou pas, ne vaut plus rien.
                    .col(
                        ColumnDef::new(Users::SessionsRevokedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
```

`SessionsRevokedAt,` dans l'enum `Users` ; `pub sessions_revoked_at: Option<DateTimeWithTimeZone>,` dans `user::Model` après `email_verified_at`.

- [ ] **Step 2: Dépôt**

```rust
/// Date la fermeture de toutes les sessions, et rend l'instant écrit.
///
/// Rendu plutôt que relu : `issue` en a besoin dans la même seconde, et une relecture
/// ne dirait pas mieux que ce que cet appel vient de lier.
pub async fn stamp_sessions_revoked(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<DateTimeWithTimeZone> {
    let maintenant = Utc::now().fixed_offset();

    Entity::update_many()
        .col_expr(user::Column::SessionsRevokedAt, Expr::value(maintenant))
        .filter(user::Column::Id.eq(id))
        .exec(db)
        .await?;

    Ok(maintenant)
}
```

(`use chrono::Utc;` et `DateTimeWithTimeZone` dans les imports.)

- [ ] **Step 3: Service — un seul chemin pour fermer tout**

Dans `service/mod.rs.jinja` :

```rust
/// Ferme toutes les sessions d'un compte, jetons d'accès compris.
///
/// Deux écritures, une fonction : les quatre chemins qui ferment un compte — rejeu,
/// changement, réinitialisation, `DELETE /auth/sessions` — passent ici, et aucun ne peut
/// fermer les rafraîchissements en laissant vivre les accès.
pub(super) async fn close_every_session(db: &DatabaseConnection, user_id: Uuid) -> Result<u64> {
    let fermees = repository::revoke_sessions_of(db, user_id).await?;
    repository::user::stamp_sessions_revoked(db, user_id).await?;

    Ok(fermees)
}
```

Remplacer les quatre `repository::revoke_sessions_of(db, …)` par `close_every_session(db, …)` : `session.rs.jinja` (branche `Replayed` et `revoke_sessions`), `password.rs.jinja` (`change`, `reset`).

Dans `change` (`password.rs.jinja`), recharger le compte après la fermeture, avant `issue` :

```rust
    let fermees = close_every_session(db, user_id).await?;
    // Rechargé : `issue` lit l'estampille que la fermeture vient de poser.
    let utilisateur = repository::find(db, user_id)
        .await?
        .ok_or(Error::Unauthorized)?;
```

- [ ] **Step 4: `issue` borne `iat`**

```rust
    // Jamais dans la seconde d'une révocation : `iat` n'a pas mieux que la seconde, et
    // un jeton émis dans celle-là serait indiscernable de ceux qu'elle a tués — la paire
    // que `change-password` rend naîtrait morte.
    let plancher = utilisateur
        .sessions_revoked_at
        .map_or(i64::MIN, |estampille| estampille.timestamp() + 1);
    let iat = Utc::now().timestamp().max(plancher);

    let claims = Claims {
        sub: utilisateur.id.to_string(),
        role: utilisateur.role.clone().to_value(),
        exp: iat + auth.access_ttl_secs as i64,
        iat,
        jti: token::random(),
    };
```

`maintenant` reste utilisé pour l'échéance du rafraîchissement.

- [ ] **Step 5: Commit** (avec la tâche 3 si l'on veut un arbre compilable : le fragment ne compile qu'à la tâche 3 de toute façon, `cargo check` sur projet jetable).

---

### Task 3: La règle dans le projet

**Files:**
- Modify: `mod.rs.jinja:22` (`impl HasAuth for AppState`)
- Test: `tests/session.rs.jinja`, `tests/password.rs.jinja`

- [ ] **Step 1: Tests rouges**

Dans `tests/password.rs.jinja`, après `a_reset_token_sets_a_new_password_and_closes_every_session` :

```rust
/// Le jeton d'accès de l'attaquant tombe avec les sessions : « toutes révoquées » ne
/// voulait rien dire tant qu'un Bearer émis avant restait bon un quart d'heure.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_access_token_issued_before_a_reset_is_refused() {
    let api = application().await;
    let db = connection().await;
    let email = fresh_email();
    register(&api, &email).await;
    let avant = login(&api, &email, PASSWORD).await;
    let acces = avant["access_token"].as_str().expect("jeton d'accès");

    let (ok, _) = call(&api, get_authenticated("/auth/me", acces)).await;
    assert_eq!(ok, StatusCode::OK);

    // Le jeton de réinitialisation est tiré par le service, comme les tests voisins.
    let (compte, jeton) = crate::auth::service::password::request_reset(&db, 600, &email)
        .await
        .expect("demande possible")
        .expect("le compte existe");
    let (statut, corps) = call(
        &api,
        post_json(
            "/auth/reset-password",
            json!({ "token": jeton, "new_password": "un autre mot de passe assez long" }),
        ),
    )
    .await;
    assert_eq!(statut, StatusCode::NO_CONTENT, "{corps} ({})", compte.id);

    let (refus, corps) = call(&api, get_authenticated("/auth/me", acces)).await;
    assert_eq!(refus, StatusCode::UNAUTHORIZED, "{corps}");
}
```

Dans `tests/session.rs.jinja`, après `an_admin_satisfies_a_user_requirement` :

```rust
/// Un rôle retiré en base cesse d'ouvrir la route dans la minute, pas au bout du jeton :
/// le rôle du Bearer est comparé à celui de la ligne à chaque requête.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_demoted_admin_is_refused_with_its_old_token() {
    let api = admin_only_route().await;
    let db = connection().await;
    let paire = login_as_admin(&api, &db).await;
    let acces = access_for(&paire);

    let (ok, _) = call(&api, with_token("GET", "/reserve", &acces)).await;
    assert_eq!(ok, StatusCode::OK);

    let sub = Uuid::parse_str(
        rbs_core::jwt::verify(&acces, &rbs_core::Config::load().expect("config").auth.secret)
            .expect("jeton lisible")
            .sub
            .as_str(),
    )
    .expect("sub lisible");
    let compte = crate::auth::model::user::Entity::find_by_id(sub)
        .one(&db)
        .await
        .expect("table interrogeable")
        .expect("compte présent");
    let mut retrograde: crate::auth::model::user::ActiveModel = compte.into();
    retrograde.role = Set(Role::User);
    retrograde.update(&db).await.expect("compte rétrogradé");

    let (refus, body) = call(&api, with_token("GET", "/reserve", &acces)).await;
    assert_eq!(refus, StatusCode::UNAUTHORIZED, "{body}");
}
```

Le test existant `changing_the_password_returns_a_usable_pair_and_closes_the_others` rejoue déjà la paire rendue sur `/auth/refresh` ; ajouter un appel `GET /auth/me` avec `corps["access_token"]` attendu 200 — c'est lui qui prouve le plancher d'`issue`.

- [ ] **Step 2: L'implémentation dans `mod.rs.jinja`**

```rust
impl HasAuth for AppState {
    /// Ce que la signature ne dit pas : le compte existe-t-il encore, a-t-il fermé ses
    /// sessions depuis l'émission, porte-t-il toujours ce rôle.
    ///
    /// Une lecture de `users` par requête authentifiée — le prix d'un jeton d'accès qui
    /// meurt avec les sessions au lieu de survivre `access_ttl_secs`. Fermer une seule
    /// session nommée ne passe pas ici : rien ne relie un jeton d'accès à sa ligne.
    fn accept(
        &self,
        claims: &rbs_core::jwt::Claims,
    ) -> impl std::future::Future<Output = rbs_core::Result<()>> + Send {
        let id = Uuid::parse_str(&claims.sub);
        let iat = claims.iat;
        let role = claims.role.clone();

        async move {
            let id = id.map_err(|_| Error::Unauthorized)?;
            let compte = repository::find(self.core().db(), id)
                .await?
                .ok_or(Error::Unauthorized)?;

            // La seconde de la révocation comprise : `iat` n'a pas mieux, et `issue`
            // n'émet jamais dedans.
            if compte
                .sessions_revoked_at
                .is_some_and(|estampille| iat <= estampille.timestamp())
            {
                return Err(Error::Unauthorized);
            }

            // Le client rafraîchit, et repart avec le rôle courant.
            if compte.role.to_value() != role {
                return Err(Error::Unauthorized);
            }

            Ok(())
        }
    }
}
```

Imports à ajouter dans `mod.rs.jinja` : `rbs_core::{Error, HasCoreState}`, `sea_orm::ActiveEnum`, `sea_orm::prelude::Uuid`. Si la capture de `self` dans `async move` pose un problème de durée de vie, cloner le `DatabaseConnection` (c'est un `Arc` interne) avant le bloc : `let db = self.core().db().clone();`.

- [ ] **Step 3: `cargo check` sur projet jetable** puis `cargo test --lib -- --ignored auth::tests` avec base : les deux tests neufs passent, tous les anciens aussi.

---

### Task 4: Les tests qui forgeaient un compte inexistant

**Files:**
- Modify: `crates/rbs-cli/templates/feature/tests.rs.jinja:55-75` (`token`)
- Modify: `crates/rbs-cli/templates/features/webhooks/tests.rs.jinja:167-182` (`token`, `request`)
- Modify: `crates/rbs-cli/tests/integration_auth.rs:551-560` (le commentaire qui décrit `token()`)

**Interfaces:**
- Produces: dans les deux fichiers de tests, `async fn token(db: &DatabaseConnection, role: &str) -> String` qui inscrit un compte au rôle voulu et signe pour lui.

- [ ] **Step 1: `feature/tests.rs.jinja`** — remplacer `token` :

```rust
/// Inscrit un compte au rôle voulu et signe un jeton pour lui.
///
/// Un compte réel, et non un `sub` tiré au hasard : `Identity` relit la ligne du compte
/// à chaque requête — rôle et révocations — et un jeton sans compte est refusé.
async fn token(db: &DatabaseConnection, role: &str) -> String {
    use sea_orm::{ActiveModelTrait, ActiveValue::Set};

    let config = rbs_core::Config::load().expect("configuration lisible");
    let compte = crate::auth::repository::create(
        db,
        &format!("{}@exemple.test", Uuid::new_v4()),
        "hash sans valeur",
    )
    .await
    .expect("le compte s'insère");
    let mut promu: crate::auth::model::user::ActiveModel = compte.into();
    promu.role = Set(sea_orm::ActiveEnum::try_from_value(&role.to_string()).expect("rôle connu"));
    let compte = promu.update(db).await.expect("rôle posé");

    let maintenant = chrono::Utc::now().timestamp();
    let claims = rbs_core::jwt::Claims {
        sub: compte.id.to_string(),
        role: role.to_string(),
        exp: maintenant + 300,
        iat: maintenant,
        jti: Uuid::new_v4().to_string(),
    };

    rbs_core::jwt::sign(&claims, &config.auth.secret).expect("jeton signable")
}
```

Lire ensuite chaque appel de `token(` dans le fichier (`grep -n 'token(' feature/tests.rs.jinja`) : ils deviennent `token(&db, "admin").await`, `db` étant la connexion que les tests obtiennent déjà (vérifier le nom de l'aide qui l'ouvre, sinon en ajouter une sur le modèle de `auth/tests/mod.rs.jinja::connection`). Le type de `Role::try_from_value` : `sea_orm::ActiveEnum::try_from_value(&role.to_owned())` rend `Result<Role, DbErr>` ; annoter `let role_enum: crate::auth::model::Role = …`.

- [ ] **Step 2: `webhooks/tests.rs.jinja`** — même remplacement de `token`, et `request(method, path, role, body)` devient `async fn request(db: &DatabaseConnection, …)` qui appelle `token(db, role).await` ; adapter ses deux appelants (`a_user_role_is_refused_on_the_three_routes`, `an_admin_subscribes_then_reads_and_revokes`), qui disposent de `state.core().db()`.

- [ ] **Step 3: `integration_auth.rs`** — le commentaire de `the_tests_of_a_crud_generated_under_auth_pass` reste vrai (trois contrats du noyau) ; ajouter une phrase : « et, depuis que `Identity` relit le compte, le dépôt `auth` du projet ». Vérifier que le test passe toujours (passe lente).

- [ ] **Step 4: Régénérer `examples/blog-auth` par diff** (`src/auth/*`, `migration/src/*`, `src/posts/tests.rs`), `cargo clippy --all-targets -- -D warnings` dans l'exemple, `cargo test -p rbs-cli --test integration_examples`.

- [ ] **Step 5: Passe lente** `integration_auth` (voir contraintes) + `integration_webhooks` (le `token()` des webhooks a changé) : tout vert. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p rbs-cli --lib`.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates crates/rbs-cli/tests examples/blog-auth
git commit -m "fix(auth): refuse un jeton d'accès émis avant la révocation des sessions ou sous un rôle retiré"
```

---

### Task 5: Documentation et notes de version

**Files:**
- Modify: `docs/docs/guides/auth.md:124-126` (+ FR)
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md`

- [ ] **Step 1: Guide** — remplacer « It carries the account id and its role, it is not stored anywhere, and it is verified by signature alone — which is what makes it cheap. It is short-lived because it cannot be revoked. » par :

```markdown
The **access token** is a signed JWT (HS256). It carries the account id and its role and
is not stored anywhere. Its signature is verified first; then the account row is read —
one query per authenticated request — and the token is refused if the account is gone,
if every session was closed after it was issued (`users.sessions_revoked_at`), or if the
role it carries is no longer the account's. Closing a *single* session does not reach
its access token: nothing links the two, and the token lives until `exp`. That is why
it stays short-lived.
```

FR équivalent. Dans le paragraphe « The token cycle », ajouter une phrase sur `sessions_revoked_at` là où sont décrits reset et change.

- [ ] **Step 2: CHANGELOG**, `### Fixed` de 1.5.0 :

```markdown
- **An access token no longer survives the revocation of its sessions.** `rbs-core`'s
  `HasAuth` gains a provided `accept(&claims)` method that `Identity` calls after the
  signature check; the `auth` fragment implements it by reading the account: gone,
  sessions closed after `iat` (`users.sessions_revoked_at`, stamped by reset, change and
  `DELETE /auth/sessions`), or role changed → 401. Tokens are issued past the revocation
  second so the pair returned by `change-password` works at once. Generated tests that
  signed a token for a random `sub` now create an account. **Projects generated before
  1.5.0:** `ALTER TABLE users ADD COLUMN sessions_revoked_at timestamptz NULL;`, then
  copy `impl HasAuth for AppState` from the fragment; without it nothing changes.
```

- [ ] **Step 3: `cargo test -p rbs-cli --test integration_docs`** → vert. Commit :

```bash
git commit -am "docs(auth): dit ce que la lecture du compte ajoute à la signature du jeton"
```
