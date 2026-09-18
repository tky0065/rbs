# `rbs add api-keys` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Donner à un projet engendré une authentification machine par clé d'API, telle qu'une clé valide vaille `Identity` partout où un jeton vaut déjà — sans qu'aucun contrôleur engendré ne change.

**Architecture:** Trois étages. Le noyau gagne une méthode à corps par défaut *refusant* (`HasAuth::accept_key`) et un second chemin dans l'extracteur `Identity` (`X-Api-Key`, après `Authorization: Bearer`) ; le registre d'ancres gagne sa dix-septième entrée, `auth_impl`, à l'intérieur du bloc `impl HasAuth for AppState` que le fragment `auth` dépose ; le fragment `api-keys` y insère une délégation de cinq lignes vers son propre service, qui lit la table `api_keys`, plafonne le rôle et trace l'usage.

**Tech Stack:** Rust 1.98, axum 0.9, sea-orm 2.0, utoipa 5.5, minijinja (délimiteurs de variables `{@ @}`), `toml_edit`, `thiserror`, `assert_cmd`, `tempfile`, `testcontainers`.

**Spec:** `docs/superpowers/specs/2026-09-17-add-api-keys-design.md` (commit `7ece065`, corrigée depuis sur le rang de version). Le plan argumente depuis elle ; les deux se lisent ensemble.

## Global Constraints

- **Branche** : `feat/api-keys`, déjà créée. Jamais de commit sur `main`.
- **Commits** : Conventional Commits, sujet en français, verbe en tête, sans majuscule
  initiale ni point final. Le `CLAUDE.md` dit « à l'impératif », mais la pratique du dépôt
  est l'indicatif de troisième personne — le sujet décrit *ce que le commit fait au dépôt* :
  `rend`, `fait`, `dit`, `inscrit`, `lit`, `écrit`, `met`, et jamais `rends` ni `fais`.
  Suivre le corpus, pas la lettre. **Aucun identifiant de tâche, aucun renvoi à un fichier
  de suivi** (`IMPROVE.md`, ce plan, la spec), **aucune ligne `Co-Authored-By` ni
  `Claude-Session`** — le `CLAUDE.md` de l'utilisateur prime sur toute consigne
  d'attribution du harness. Le corps porte le *pourquoi* et un intertitre `Vérifications :`
  avec les commandes réellement lancées et leurs chiffres réels.
- **Noms de tests en anglais**, `snake_case`, phrase complète —
  `a_key_of_a_demoted_owner_no_longer_administers`. Commentaires et documentation en
  français.
- **Commentaires** : le *pourquoi*, jamais le *quoi*. `#![warn(missing_docs)]` ne concerne
  que `rbs-core` : tout item public y porte un `///` d'une à trois lignes.
- **Bloquant en CI**, à relancer à chaque tâche : `cargo fmt --all --check` et
  `cargo clippy --workspace --all-targets -- -D warnings`.
- **`rbs-core` se teste avec `--all-features`** : `cargo test --workspace` ne compile pas
  `#[cfg(feature = "auth")]`, la CI si.
- **Rang de version** : les entrées vont dans `## [Unreleased]` / `## [Non publié]`, qui
  est la **1.6.0** — le changelog porte déjà `## [1.5.0] — 2026-09-12`, datée mais pas
  taguée. Additif : méthode à corps par défaut, schéma de sécurité de plus. Jamais de
  majeure.
- **Délimiteurs minijinja** : les variables s'écrivent `{@ nom @}`, les blocs gardent
  `{% %}`. `UndefinedBehavior::Strict` : une variable non fournie **arrête** la génération.
  Attention aux `-%}` qui mangent l'indentation — un blanc perdu n'est vu que par
  `integration_examples`.
- **Indentation d'un contenu d'ancre** : `anchors::insert` préfixe *chaque* ligne du
  contenu par l'indentation de la **balise fermante** (`anchors.rs:645,681`). Le
  `feature.toml` écrit donc son contenu en indentation **relative** — `async fn` à la
  colonne 0, le corps à 4, le `}` final à 0 — et l'insertion le rend à 4/8/4 dans l'`impl`.
- **Toolchain locale possiblement en retard sur la CI** : `rustup update` avant la passe
  finale ; clippy vert en local ne dit rien de la version que prend `@stable`.
- **Tests Docker** : `--no-fail-fast` obligatoire, et c'est un drapeau **de cargo** — il se
  place *avant* le `--`, jamais après, où le harnais de test le refuse par
  `error: Unrecognized option: 'no-fail-fast'` et ne lance rien. Sans lui la suite s'arrête au premier
  binaire et masque les échecs suivants. Une seule commande dépasse les 600 s du shell :
  un job par suite, et rediriger vers le scratchpad, sinon les chiffres sont rognés.
- **Preuve que les tests discriminent.** Un rouge obtenu par une fonction absente — donc
  une erreur de compilation — ne prouve rien de la qualité des assertions. Pour chaque
  comportement neuf, le rapport doit porter une **mutation délibérée** : casser
  l'implémentation d'une manière qui **laisse le code compiler**, lancer le test couvrant,
  coller sa sortie rouge réelle, restaurer. Une mutation qui casse la compilation ne compte
  pas.
- **Les sorties se collent, ne s'affirment pas.** Une commande muette quand tout va bien —
  `cargo fmt --all --check` en premier lieu — se prouve par son code de retour :
  `cargo fmt --all --check; echo "exit: $?"`, et on colle ces deux lignes. « Silencieux,
  code 0 » écrit en prose est une affirmation, pas une trace.
- **Le code des fragments n'est compilé nulle part** tant qu'un exemple ne le porte pas :
  `cargo check` sur `examples/event-hub` avant toute passe Docker (tâche 12).

---

### Task 1: Le noyau sait qu'une clé peut exister

**Files:**
- Modify: `crates/rbs-core/src/state.rs:80-115` (trait `HasAuth`)
- Test: `crates/rbs-core/src/state.rs` (module `tests`)

**Interfaces:**
- Consomme : rien.
- Produit : `HasAuth::accept_key(&self, key: &str, extensions: &mut axum::http::Extensions) -> impl Future<Output = crate::Result<crate::jwt::Claims>> + Send`, corps par défaut rendant `Err(Error::Unauthorized)`. Tâche 2 l'appelle, tâche 4 la fait surcharger par le gabarit.

Le défaut refuse, et c'est le cœur de la compatibilité : un projet qui n'installe pas le
fragment ne connaît aucune clé, et l'en-tête n'y ouvre rien. La méthode reçoit les
extensions pour la même raison qu'`accept_in` les reçoit — le projet vient de lire le
compte pour juger la clé, et l'extracteur suivant n'a pas à relire la même ligne.

- [ ] **Step 1: Écrire le test qui échoue**

Dans le module `tests` de `crates/rbs-core/src/state.rs`, sous `#[cfg(feature = "auth")]` :

```rust
/// Un état qui n'a pas installé le fragment `api-keys` : il ne surcharge rien.
#[derive(Clone)]
struct SansCles(CoreState);

impl HasCoreState for SansCles {
    fn core(&self) -> &CoreState {
        &self.0
    }
}

impl HasAuth for SansCles {}

/// Le défaut refuse, et c'est ce qui rend l'ajout inoffensif : un projet qui ignore les
/// clés n'en accepte aucune, quelle que soit la valeur présentée.
#[cfg(feature = "auth")]
#[tokio::test]
async fn a_state_that_declares_no_key_store_refuses_every_key() {
    let etat = SansCles(CoreState::new(DatabaseConnection::default(), config()));
    let mut extensions = axum::http::Extensions::new();

    let verdict = etat.accept_key("rbs_peu_importe", &mut extensions).await;

    assert!(matches!(verdict, Err(crate::Error::Unauthorized)));
}
```

- [ ] **Step 2: Lancer le test et le voir échouer**

```bash
cargo test -p rbs-core --all-features -- state::tests::a_state_that_declares_no_key_store_refuses_every_key
```

Attendu : `error[E0599]: no method named 'accept_key' found`.

- [ ] **Step 3: Ajouter la méthode au trait**

Dans `crates/rbs-core/src/state.rs`, à l'intérieur de `pub trait HasAuth: HasCoreState`,
après `accept_in` :

```rust
    /// Ce que le projet fait d'une clé d'API portée par `X-Api-Key`.
    ///
    /// Le défaut refuse : un projet qui n'a pas installé le fragment `api-keys` ne connaît
    /// aucune clé, et l'en-tête n'y ouvre rien. Le fragment surcharge cette méthode pour
    /// lire sa table, plafonner le rôle et rendre les claims de l'appelant.
    ///
    /// Les extensions sont à portée pour la même raison que dans [`HasAuth::accept_in`] :
    /// le projet vient de lire le compte pour juger la clé, et peut y laisser ce qu'il en
    /// sait plutôt que de le faire relire à l'extracteur suivant.
    fn accept_key(
        &self,
        key: &str,
        extensions: &mut axum::http::Extensions,
    ) -> impl std::future::Future<Output = Result<crate::jwt::Claims, crate::Error>> + Send {
        let _ = (key, extensions);
        async { Err(crate::Error::Unauthorized) }
    }
```

- [ ] **Step 4: Lancer le test et le voir passer**

```bash
cargo test -p rbs-core --all-features -- state::
```

Attendu : le test passe, aucun autre ne régresse.

- [ ] **Step 5: Prouver que le test discrimine**

Muter le défaut en `async { Ok(crate::jwt::Claims { sub: "u1".into(), role: "admin".into(), exp: 0, iat: 0, jti: "j".into() }) }` — le code compile —, relancer, coller la sortie rouge, restaurer.

- [ ] **Step 6: Lint puis commit**

```bash
cargo fmt --all --check; echo "exit: $?"
cargo clippy --workspace --all-targets -- -D warnings; echo "exit: $?"
git add crates/rbs-core/src/state.rs
git commit
```

Sujet : `feat(core): ouvre au projet le jugement d'une clé d'API`

---

### Task 2: `Identity` lit `X-Api-Key` quand il n'y a pas de jeton

**Files:**
- Modify: `crates/rbs-core/src/extract.rs:22-24` (constantes), `:50-70` (`from_request_parts`)
- Test: `crates/rbs-core/src/extract.rs` (module `tests::identite`)

**Interfaces:**
- Consomme : `HasAuth::accept_key` (tâche 1).
- Produit : `Identity` extrait d'un en-tête `X-Api-Key`. Aucune signature publique ne change — `Identity` garde ses deux champs et son `#[non_exhaustive]`.

L'ordre est la décision : le jeton l'emporte quand les deux en-têtes sont là. C'est le
justificatif le plus spécifique, et un mandataire qui injecterait une clé de service ne
doit pas supplanter celui que l'appelant a présenté.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `mod identite` de `crates/rbs-core/src/extract.rs`, après les tests existants :

```rust
    /// Un état qui accepte une clé unique et lui donne un rôle : ce que le fragment
    /// `api-keys` écrira, réduit à ce que l'extracteur doit en voir.
    #[derive(Clone)]
    struct AvecCles(AppState);

    impl HasCoreState for AvecCles {
        fn core(&self) -> &CoreState {
            self.0.core()
        }
    }

    impl HasAuth for AvecCles {
        async fn accept_key(
            &self,
            key: &str,
            extensions: &mut axum::http::Extensions,
        ) -> Result<Claims, crate::Error> {
            if key != "rbs_la_bonne" {
                return Err(crate::Error::Unauthorized);
            }
            extensions.insert(Relu("par la clé".to_owned()));
            Ok(Claims {
                sub: "u7".to_owned(),
                role: "user".to_owned(),
                exp: LATER,
                iat: 0,
                jti: "cle-1".to_owned(),
            })
        }
    }

    /// Appelle un handler protégé en présentant les en-têtes donnés.
    async fn call_with(
        autorisation: Option<&str>,
        cle: Option<&str>,
    ) -> (StatusCode, String) {
        async fn handler(identite: Identity) -> String {
            format!("{} {}", identite.user_id, identite.role)
        }

        let mut requete = Request::builder().uri("/");
        if let Some(autorisation) = autorisation {
            requete = requete.header(header::AUTHORIZATION, autorisation);
        }
        if let Some(cle) = cle {
            requete = requete.header("x-api-key", cle);
        }

        let response = Router::new()
            .route("/", get(handler))
            .with_state(AvecCles(state()))
            .oneshot(requete.body(Body::empty()).expect("requête valide"))
            .await
            .expect("le router doit répondre");

        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");

        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn a_valid_key_alone_identifies_the_caller() {
        let (status, body) = call_with(None, Some("rbs_la_bonne")).await;

        assert_eq!(status, StatusCode::OK, "obtenu : {body}");
        assert_eq!(body, "u7 user");
    }

    #[tokio::test]
    async fn an_unknown_key_is_unauthorized() {
        let (status, _) = call_with(None, Some("rbs_pas_celle_la")).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    /// Le jeton l'emporte : c'est le justificatif le plus spécifique, et un mandataire qui
    /// injecterait une clé de service ne doit pas supplanter celui que l'appelant présente.
    #[tokio::test]
    async fn a_bearer_token_wins_over_a_key_presented_at_the_same_time() {
        let bearer = format!("Bearer {}", token(LATER, SECRET));

        let (status, body) = call_with(Some(&bearer), Some("rbs_la_bonne")).await;

        assert_eq!(status, StatusCode::OK, "obtenu : {body}");
        assert_eq!(body, "u1 admin", "le jeton doit gouverner : {body}");
    }

    /// Ce que `accept_key` dépose est à portée de l'extracteur suivant, comme pour
    /// `accept_in` : sans quoi une garde relirait le compte que le service vient de lire.
    #[tokio::test]
    async fn what_accept_key_leaves_in_the_extensions_reaches_the_next_extractor() {
        async fn handler(_: Identity, axum::Extension(relu): axum::Extension<Relu>) -> String {
            relu.0
        }

        let response = Router::new()
            .route("/", get(handler))
            .with_state(AvecCles(state()))
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-api-key", "rbs_la_bonne")
                    .body(Body::empty())
                    .expect("requête valide"),
            )
            .await
            .expect("le router doit répondre");

        assert_eq!(response.status(), StatusCode::OK);
        let corps = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        assert_eq!(&corps[..], b"par la cl\xc3\xa9");
    }
```

Note : `Relu` est la struct déjà déclarée dans ce module pour les tests d'`accept_in` ; la
réutiliser plutôt qu'en déclarer une seconde.

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cargo test -p rbs-core --all-features -- extract::tests::identite
```

Attendu : les quatre échouent en 401 (l'extracteur ignore encore `X-Api-Key`), sauf
`a_bearer_token_wins_over_a_key_presented_at_the_same_time` qui passe déjà — c'est normal,
et c'est pourquoi sa mutation est obligatoire à l'étape 5.

- [ ] **Step 3: Implémenter le second chemin**

Dans `crates/rbs-core/src/extract.rs`, ajouter la constante près de `SCHEMA` :

```rust
/// En-tête portant une clé d'API. Insensible à la casse, comme tout nom d'en-tête HTTP.
#[cfg(feature = "auth")]
const API_KEY: &str = "x-api-key";
```

Puis remplacer le corps de `from_request_parts` :

```rust
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Le jeton d'abord : c'est le justificatif le plus spécifique, et un mandataire qui
        // injecterait une clé de service ne doit pas supplanter celui que l'appelant a
        // présenté.
        if let Some(token) = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(bearer)
        {
            let claims = crate::jwt::verify(token, &state.auth().secret)?;
            state.accept_in(&claims, &mut parts.extensions).await?;

            return Ok(Self {
                user_id: claims.sub,
                role: claims.role,
            });
        }

        let key = parts
            .headers
            .get(API_KEY)
            .and_then(|value| value.to_str().ok())
            .ok_or(Error::Unauthorized)?
            .to_owned();

        // Le projet est seul à savoir ce qu'est une clé : le noyau n'a ni la table ni la
        // règle. Le défaut du trait refuse, un projet sans le fragment n'ouvre donc rien.
        let claims = state.accept_key(&key, &mut parts.extensions).await?;

        Ok(Self {
            user_id: claims.sub,
            role: claims.role,
        })
    }
```

`key` est possédée et non empruntée : `parts.headers` et `parts.extensions` sont deux
emprunts du même `parts`, et le second est mutable.

- [ ] **Step 4: Lancer les tests et les voir passer**

```bash
cargo test -p rbs-core --all-features -- extract::
```

Attendu : tous passent, y compris les tests `bearer` antérieurs — la clé ne doit rien
changer pour qui présente un jeton.

- [ ] **Step 5: Prouver que les tests discriminent**

Trois mutations, chacune laissant le code compiler :

1. Inverser l'ordre (lire `X-Api-Key` avant `Authorization`) → `a_bearer_token_wins_over_a_key_presented_at_the_same_time` doit rougir avec `left: "u7 user", right: "u1 admin"`.
2. Ne pas passer `&mut parts.extensions` mais une `Extensions::new()` locale → `what_accept_key_leaves_in_the_extensions_reaches_the_next_extractor` doit rougir en 500.
3. Remplacer `.ok_or(Error::Unauthorized)?` par un `unwrap_or_default()` → `an_unknown_key_is_unauthorized` reste vert mais un appel sans aucun en-tête doit rougir : vérifier que `without_an_authorization_header_the_response_is_401_in_problem_json` le voit.

Coller les trois sorties rouges, restaurer.

- [ ] **Step 6: Lint puis commit**

```bash
cargo fmt --all --check; echo "exit: $?"
cargo clippy --workspace --all-targets -- -D warnings; echo "exit: $?"
git add crates/rbs-core/src/extract.rs
git commit
```

Sujet : `feat(core): identifie l'appelant par une clé quand aucun jeton n'est présenté`

---

### Task 3: Le document dit comment présenter une clé

**Files:**
- Modify: `crates/rbs-core/src/openapi.rs:76-78` (nom du schéma), `:118-130` (`declare`)
- Test: `crates/rbs-core/src/openapi.rs` (module `tests`, près de `the_bearer_security_scheme_is_declared` ligne 287)

**Interfaces:**
- Consomme : rien.
- Produit : `pub const KEY_SCHEME_NAME: &str = "api_key";` sous `#[cfg(feature = "auth")]`, et le schéma correspondant dans `components/securitySchemes`. Les contrôleurs du fragment (tâche 7) le citent par ce nom.

Le schéma vit dans le noyau et non dans le gabarit : `openapi.rs.jinja` ne déclare que
`modifiers(&CommonResponses)`, si bien qu'un projet déjà engendré reçoit le schéma à son
prochain `upgrade`, sans toucher un fichier.

- [ ] **Step 1: Écrire le test qui échoue**

```rust
    /// Le document dit comment présenter une clé, comme il dit comment présenter un jeton :
    /// une opération qui annonce 401 sans nommer le justificatif laisse le client deviner.
    #[cfg(feature = "auth")]
    #[test]
    fn the_api_key_security_scheme_is_declared() {
        let doc: Value =
            serde_json::to_value(ApiDoc::openapi()).expect("le document se sérialise");

        let schema = &doc["components"]["securitySchemes"][KEY_SCHEME_NAME];

        assert_eq!(schema["type"], "apiKey", "{schema}");
        assert_eq!(schema["in"], "header", "{schema}");
        assert_eq!(schema["name"], "X-Api-Key", "{schema}");
    }
```

- [ ] **Step 2: Lancer le test et le voir échouer**

```bash
cargo test -p rbs-core --all-features -- openapi::tests::the_api_key_security_scheme_is_declared
```

Attendu : `cannot find value 'KEY_SCHEME_NAME' in this scope`.

- [ ] **Step 3: Déclarer le schéma**

Dans `crates/rbs-core/src/openapi.rs`, sous `SCHEME_NAME` :

```rust
/// Nom du schéma de la clé d'API, tel que les handlers le référencent dans `security(...)`.
#[cfg(feature = "auth")]
pub const KEY_SCHEME_NAME: &str = "api_key";
```

Étendre l'import :

```rust
#[cfg(feature = "auth")]
use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
```

Et, dans `declare`, juste après l'ajout du schéma `bearer` :

```rust
    // Le second justificatif que l'extracteur accepte. Déclaré inconditionnellement avec
    // `auth` : le noyau ne sait pas si le projet a installé le fragment `api-keys`, et un
    // schéma déclaré qu'aucune opération ne cite ne coûte qu'une ligne au document.
    #[cfg(feature = "auth")]
    composants.add_security_scheme(
        KEY_SCHEME_NAME,
        SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Api-Key"))),
    );
```

- [ ] **Step 4: Lancer les tests et les voir passer**

```bash
cargo test -p rbs-core --all-features -- openapi::
```

Attendu : le test neuf passe, et `the_declared_scheme_is_imposed_on_no_operation` reste
vert — déclarer un schéma ne doit l'imposer à personne.

- [ ] **Step 5: Prouver que le test discrimine**

Muter `ApiKey::Header` en `ApiKey::Query` — le code compile — et coller le rouge
(`left: "query", right: "header"`), puis restaurer.

- [ ] **Step 6: Exporter, lint, commit**

Vérifier que `crates/rbs-core/src/lib.rs:70` réexporte bien le nécessaire ; `KEY_SCHEME_NAME`
est atteignable par `rbs_core::openapi::KEY_SCHEME_NAME`, ce qui suffit : les contrôleurs
engendrés citent la chaîne `"api_key"` dans une macro `utoipa::path`, pas la constante.

```bash
cargo fmt --all --check; echo "exit: $?"
cargo clippy --workspace --all-targets -- -D warnings; echo "exit: $?"
git add crates/rbs-core/src/openapi.rs
git commit
```

Sujet : `feat(core): déclare au document le schéma de la clé d'API`

---

### Task 4: La dix-septième ancre, et les deux retouches au fragment `auth`

**Files:**
- Modify: `crates/rbs-cli/src/anchors.rs:330-346` (ajout de `AUTH_IMPL`, `ANCRES` passe à 17)
- Modify: `crates/rbs-cli/templates/features/auth/mod.rs.jinja:24-40` (l'ancre dans l'`impl`)
- Modify: `crates/rbs-cli/templates/features/auth/guard.rs.jinja:76-79` (`pub(super)` → `pub(crate)`)
- Test: `crates/rbs-cli/src/anchors.rs` (module `tests`, autour des lignes 1326 et 1486)

**Interfaces:**
- Consomme : rien.
- Produit : `pub(crate) const AUTH_IMPL: Anchor` de nom `auth_impl`, fichier `src/auth/mod.rs`, `optional: true`, `sorted: false`, accroche `impl HasAuth for AppState {`. Le manifeste du fragment (tâche 5) y insère. `crate::auth::guard::Accepted` devient `pub(crate)`.

C'est la tâche qui rend le reste possible, et la seule qui touche un gabarit déjà installé
chez des utilisateurs : un projet engendré avant cette version n'aura pas l'ancre, et
`rbs add api-keys` y affichera le bloc à coller sans rien écrire — la convention du dépôt,
déjà éprouvée sous le code d'erreur `ancre_absente`.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans le module `tests` de `crates/rbs-cli/src/anchors.rs` :

```rust
    /// L'ancre vit dans un fichier que le fragment `auth` dépose : un projet sans `auth`
    /// n'a pas ce fichier, et `doctor` ne doit pas le tenir pour incomplet.
    #[test]
    fn the_auth_impl_anchor_is_optional_and_lives_in_the_auth_module() {
        assert!(ANCRES.contains(&AUTH_IMPL));
        assert!(AUTH_IMPL.optional);
        assert!(!AUTH_IMPL.sorted);
        assert_eq!(AUTH_IMPL.file, "src/auth/mod.rs");
        assert_eq!(AUTH_IMPL.comment, "//");
    }

    /// L'accroche d'une ancre effacée doit exister dans la template qui la porte, sans quoi
    /// `doctor --fix` n'a aucun endroit où la reposer.
    #[test]
    fn the_auth_impl_hook_is_a_line_of_its_own_template() {
        let gabarit = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/auth/mod.rs.jinja"
        ))
        .expect("le gabarit du fragment auth se lit");

        assert_eq!(
            gabarit
                .lines()
                .filter(|ligne| ligne.trim() == AUTH_IMPL.after)
                .count(),
            1,
            "l'accroche doit être présente une fois et une seule"
        );
        assert!(gabarit.contains(&AUTH_IMPL.opening()));
        assert!(gabarit.contains(&AUTH_IMPL.closing()));
    }
```

Et **modifier** le test existant `only_the_anchors_of_a_fragment_deposited_file_are_optional`
(ligne ~1486) pour que la liste attendue devienne :

```rust
        assert_eq!(
            optionnelles,
            [
                "modules",
                "services",
                "jobs",
                "job_modules",
                "schedules",
                "auth_impl"
            ]
        );
```

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cargo test -p rbs-cli --lib -- anchors::
```

Attendu : `cannot find value 'AUTH_IMPL'`, et le test des optionnelles rouge sur la liste.

- [ ] **Step 3: Déclarer l'ancre**

Dans `crates/rbs-cli/src/anchors.rs`, avant `RELATIONS` :

```rust
/// Méthodes que les fragments ajoutent à l'`impl HasAuth for AppState` du projet.
///
/// Le seul point d'insertion du registre qui vive **à l'intérieur** d'un bloc `impl`, et
/// non entre deux items. La raison est que le trait `HasAuth` ne peut être implémenté
/// qu'une fois : un fragment qui voudrait juger un justificatif de plus — une clé d'API —
/// n'a aucun autre endroit où poser sa méthode, et le CLI ne réécrit pas d'AST.
///
/// L'accroche est la ligne d'ouverture de l'`impl`, stable et unique dans le fichier. Le
/// bloc n'est pas trié : rustfmt ne réordonne pas les méthodes d'une implémentation.
pub(crate) const AUTH_IMPL: Anchor = Anchor {
    name: Cow::Borrowed("auth_impl"),
    file: Cow::Borrowed("src/auth/mod.rs"),
    comment: "//",
    sorted: false,
    // Le fichier est déposé par le fragment `auth` : un projet qui ne l'a pas installé n'a
    // pas ce fichier, et n'est pas incomplet pour autant.
    optional: true,
    after: "impl HasAuth for AppState {",
};
```

Puis passer le tableau à dix-sept et y ajouter l'entrée en dernier :

```rust
pub(crate) const ANCRES: [Anchor; 17] = [
    FEATURES,
    MODULES,
    ROUTES,
    LAYERS,
    OPENAPI,
    MIGRATION_MODULES,
    MIGRATIONS,
    STATE_CHAMPS,
    STATE_INIT,
    STARTUP,
    SEEDS,
    SERVICES,
    HEALTH_PROBES,
    JOBS,
    JOB_MODULES,
    SCHEDULES,
    AUTH_IMPL,
];
```

- [ ] **Step 4: Poser l'ancre dans le gabarit, et ouvrir `Accepted`**

Dans `crates/rbs-cli/templates/features/auth/mod.rs.jinja`, à l'intérieur de l'`impl`,
après la méthode `accept_in` et avant l'accolade fermante :

```rust
    // <rbs:auth_impl>
    // </rbs:auth_impl>
}
```

Dans `crates/rbs-cli/templates/features/auth/guard.rs.jinja`, la déclaration d'`Accepted` :

```rust
/// La date de vérification du compte qu'`accept_in` a relu pour juger le jeton, laissée
/// dans la requête.
///
/// La date seule, et non le compte : la garde ne lit rien d'autre, et le hash du mot de
/// passe n'a pas à voyager dans les extensions de chaque requête authentifiée. Un type
/// propre au projet plutôt qu'une date nue : seule une acceptation peut l'y avoir mise.
///
/// `pub(crate)` et non `pub(super)` : un fragment installé sous `src/modules/` peut juger
/// un justificatif — une clé d'API — et doit pouvoir y déposer ce qu'il a lu du compte.
#[derive(Clone)]
pub(crate) struct Accepted(pub(crate) Option<DateTimeWithTimeZone>);
```

- [ ] **Step 5: Lancer les tests et les voir passer**

```bash
cargo test -p rbs-cli --lib -- anchors::
cargo test -p rbs-cli --lib -- doctor::anchors
```

Attendu : tout passe. `the_resolved_registry_carries_every_anchor_of_the_registry` et
`the_anchors_carry_distinct_names` couvrent déjà l'entrée neuve sans modification.

- [ ] **Step 6: Prouver que les tests discriminent**

Muter `after` en `"impl HasAuth for AppState"` (sans l'accolade) : le code compile, et
`the_auth_impl_hook_is_a_line_of_its_own_template` doit rougir — aucune ligne du gabarit
n'est *égale* à cette chaîne. Coller le rouge, restaurer.

- [ ] **Step 7: Lint puis commit**

```bash
cargo fmt --all --check; echo "exit: $?"
cargo clippy --workspace --all-targets -- -D warnings; echo "exit: $?"
cargo test -p rbs-cli --lib; echo "exit: $?"
git add crates/rbs-cli/src/anchors.rs crates/rbs-cli/templates/features/auth/
git commit
```

Sujet : `feat(cli): ouvre une ancre dans l'implémentation d'authentification du projet`

Corps : dire que le gabarit `auth` change, donc que les exemples devront suivre (tâche 12),
et que `integration_examples` est rouge d'ici là — c'est attendu et non une régression.

---

### Task 5: Le fragment existe — manifeste, module, modèle, migration

**Files:**
- Create: `crates/rbs-cli/templates/features/api-keys/feature.toml`, `mod.rs.jinja`, `model.rs.jinja`, `migration.rs.jinja`
- Modify: `crates/rbs-cli/src/templates.rs:772` (`TOUTES`), `:950-964` (liste de l'erreur)
- Modify: `crates/rbs-cli/src/preset.rs:76-92` (`disponibles()` des tests)
- Test: `crates/rbs-cli/src/templates.rs` (module `tests`)

**Interfaces:**
- Consomme : `AUTH_IMPL` (tâche 4).
- Produit : le fragment `api-keys`, installable. Entité `crate::modules::api_keys::model::{Entity, Model, ActiveModel, Column}` avec les colonnes `id, user_id, name, prefix, token_hash, role, last_used_at, expires_at, revoked_at, created_at, updated_at`. `routes()` est fourni par la tâche 7 ; ce module le déclare.

Le fragment est découvert par le parcours de `templates/features/` : rien à inscrire dans
un registre. Seuls les tableaux **de test** énumèrent les fragments en dur, et ce sont eux
qui rougissent.

- [ ] **Step 1: Écrire le test qui échoue**

Dans le module `tests` de `crates/rbs-cli/src/templates.rs` :

```rust
    /// Le fragment n'a de sens qu'avec des comptes et des rôles : il tient son `Role` et sa
    /// garde du fragment `auth`, et pose sa méthode dans l'implémentation que celui-ci
    /// dépose.
    #[test]
    fn the_api_keys_fragment_requires_auth_and_writes_into_its_implementation() {
        let source = read(&Path::new(RACINE_FEATURES).join("api-keys/feature.toml"));
        let manifest = crate::manifest::read(&source, "api-keys/feature.toml")
            .expect("le manifeste du fragment api-keys doit se lire");

        assert_eq!(manifest.feature.requires, ["auth"]);

        let ancres: Vec<&str> = manifest
            .anchors
            .iter()
            .map(|insertion| insertion.anchor.as_str())
            .collect();
        assert!(ancres.contains(&"auth_impl"), "ancres : {ancres:?}");
        assert!(ancres.contains(&"modules"), "ancres : {ancres:?}");
        assert!(ancres.contains(&"routes"), "ancres : {ancres:?}");
        assert!(ancres.contains(&"openapi"), "ancres : {ancres:?}");

        let migration = manifest
            .migration
            .expect("le fragment pose une table : il doit déclarer sa migration");
        assert_eq!(migration.name, "create_api_keys");
    }

    /// Le contenu d'une ancre est écrit en indentation **relative** : `insert` préfixe
    /// chaque ligne par celle de la balise fermante, qui vaut quatre espaces à l'intérieur
    /// de l'`impl`. Une ligne déjà indentée dans le manifeste ressortirait à huit.
    #[test]
    fn the_auth_impl_insertion_carries_no_leading_indentation_on_its_first_line() {
        let source = read(&Path::new(RACINE_FEATURES).join("api-keys/feature.toml"));
        let manifest = crate::manifest::read(&source, "api-keys/feature.toml")
            .expect("le manifeste du fragment api-keys doit se lire");

        let contenu = &manifest
            .anchors
            .iter()
            .find(|insertion| insertion.anchor == "auth_impl")
            .expect("le fragment insère dans `auth_impl`")
            .content;

        let premiere = contenu
            .lines()
            .find(|ligne| !ligne.trim().is_empty())
            .expect("le contenu n'est pas vide");
        assert_eq!(
            premiere,
            premiere.trim_start(),
            "la première ligne ne doit porter aucune indentation : {premiere:?}"
        );
    }
```

Et **modifier** les deux tableaux qui énumèrent les fragments, en respectant l'ordre
alphabétique — `api-keys` précède `audit`, `p` venant avant `u` :

- `templates.rs:772` : `const TOUTES: [&str; 14] = ["api-keys", "audit", "auth", …];`
- `templates.rs:950` : ajouter `"api-keys"` en tête de la liste d'`an_unknown_feature_is_reported_by_its_name`.
- `preset.rs:78` : ajouter `"api-keys"` en tête de `disponibles()`, et corriger
  `the_full_preset_is_everything_the_cli_can_install`, qui se dérive et reste vrai sans
  autre changement.

- [ ] **Step 2: Lancer les tests et les voir échouer**

```bash
cargo test -p rbs-cli --lib -- templates::
```

Attendu : `api-keys/feature.toml` introuvable, et la liste de l'erreur n'énumère pas
`api-keys`.

- [ ] **Step 3: Écrire le manifeste**

`crates/rbs-cli/templates/features/api-keys/feature.toml` :

> **Amendement, décidé à l'exécution.** Le manifeste ci-dessous déclare les neuf `[[files]]`
> du fragment fini, mais six d'entre eux ne sont écrits qu'aux tâches 6 à 8. Or un manifeste
> est atomique : trois tests qui parcourent tous les fragments — `new::every_embedded_feature_is_accepted_by_name`,
> `contexte::every_variable_an_embedded_fragment_interpolates_resolves_in_the_context` et
> `templates::each_rust_template_of_each_fragment_conforms_to_rustfmt` — échouent dès qu'un
> `source =` désigne un fichier absent. **Ne déclare donc ici que `mod.rs.jinja` et
> `model.rs.jinja`** ; les tâches 6, 7 et 8 ajouteront leurs entrées en écrivant leurs
> gabarits. Les sections `[migration]`, `[[anchors]]`, `[cargo.*]` sont posées dès maintenant.
>
> **Second amendement, même cause.** `mod.rs.jinja` ne peut pas non plus être écrit ici :
> il déclare `pub mod controller;`, `dto`, `repository`, `service` et `tests`, et le test
> `templates::each_rust_template_of_each_fragment_conforms_to_rustfmt` **parcourt le
> répertoire du fragment**, non son manifeste — rustfmt refuse alors de formater un module
> qu'il ne peut pas résoudre. L'élaguer n'est pas possible : son `routes()` référence les
> handlers du contrôleur. **`mod.rs.jinja` est donc écrit à la tâche 8**, quand les six
> modules existent. La tâche 5 ne pose que `model.rs.jinja`, la migration et le manifeste,
> ce dernier avec **une seule** entrée `[[files]]`.

```toml
[feature]
description = "clés d'API : authentification machine, rôle plafonné par le porteur, trace d'usage"
# Une clé appartient à un compte et porte un rôle : les deux viennent d'`auth`, qui livre
# la table `users`, l'énumération `Role` et la garde `require_role`. Le fragment y pose
# aussi sa méthode de jugement, dans l'implémentation de `HasAuth` qu'`auth` dépose —
# `requires` est donc une dépendance dure, non un confort.
requires = ["auth"]

[[files]]
source      = "mod.rs.jinja"
destination = "src/modules/api_keys/mod.rs"

[[files]]
source      = "model.rs.jinja"
destination = "src/modules/api_keys/model.rs"

[[files]]
source      = "dto.rs.jinja"
destination = "src/modules/api_keys/dto.rs"

[[files]]
source      = "repository.rs.jinja"
destination = "src/modules/api_keys/repository.rs"

[[files]]
source      = "service.rs.jinja"
destination = "src/modules/api_keys/service.rs"

[[files]]
source      = "controller.rs.jinja"
destination = "src/modules/api_keys/controller.rs"

[[files]]
source      = "tests/mod.rs.jinja"
destination = "src/modules/api_keys/tests/mod.rs"

[[files]]
source      = "tests/accept.rs.jinja"
destination = "src/modules/api_keys/tests/accept.rs"

[[files]]
source      = "tests/routes.rs.jinja"
destination = "src/modules/api_keys/tests/routes.rs"

[[anchors]]
anchor  = "modules"
content = "pub mod api_keys;"

[[anchors]]
anchor  = "routes"
content = ".merge(crate::modules::api_keys::routes())"

[[anchors]]
anchor  = "openapi"
content = """
crate::modules::api_keys::controller::create,
crate::modules::api_keys::controller::list,
crate::modules::api_keys::controller::revoke,
crate::modules::api_keys::controller::revoke_all,
"""

# Le seul endroit où le projet peut dire ce qu'est une clé : `HasAuth` ne s'implémente
# qu'une fois, et c'est `auth` qui tient cette implémentation. `Extensions` et `Claims` y
# sont déjà importés — l'insertion n'a aucun `use` à ajouter.
#
# Indentation **relative** : `insert` préfixe chaque ligne par celle de la balise
# fermante, qui vaut quatre espaces ici.
[[anchors]]
anchor  = "auth_impl"
content = """
/// Ce que vaut une clé d'API présentée en `X-Api-Key`.
///
/// Le noyau ne connaît ni la table des clés ni la règle du plafond : il demande.
async fn accept_key(
    &self,
    key: &str,
    extensions: &mut Extensions,
) -> rbs_core::Result<Claims> {
    crate::modules::api_keys::service::accept(self, key, extensions).await
}
"""

[migration]
source = "migration.rs.jinja"
name   = "create_api_keys"

# `token::random` et `token::fingerprint` tirent et empreignent la clé, `Claims` la porte
# jusqu'à l'extracteur : les trois vivent derrière cette feature du noyau. `auth` l'active
# déjà — la redéclarer dit qu'`api-keys` en dépend pour son propre compte.
[cargo.rbs-core]
features = ["auth"]

# `sync` : les tests livrés se relaient sur l'unique table `api_keys`, et leur verrou
# traverse un `await`.
[cargo.tokio]
features = ["sync"]
```

- [ ] **Step 4: Écrire le module et le modèle**

`mod.rs.jinja` :

```rust
pub mod controller;
pub mod dto;
pub mod model;
pub mod repository;
pub mod service;

#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::{delete, post};

use crate::state::AppState;

// Les deux méthodes du même chemin se déclarent en une fois : axum refuse — et le dit par
// une panique au démarrage — deux `route()` sur un chemin identique.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api-keys",
            post(controller::create)
                .get(controller::list)
                .delete(controller::revoke_all),
        )
        .route("/api-keys/{id}", delete(controller::revoke))
}
```

`model.rs.jinja` :

```rust
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;

use crate::auth::model::Role;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "api_keys")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Le compte auquel la clé appartient : elle ne vaut jamais plus que lui.
    #[sea_orm(indexed)]
    pub user_id: Uuid,
    /// Ce que la clé sert — « ci », « script de facturation ». Pour la reconnaître.
    pub name: String,
    /// Les douze premiers caractères de la clé, seuls à reparaître après la création.
    pub prefix: String,
    /// Empreinte SHA-256 de la clé entière. La clé elle-même n'est jamais stockée.
    #[sea_orm(unique, indexed)]
    pub token_hash: String,
    /// Le rôle propre à la clé, plafonné à la lecture par celui de son porteur.
    pub role: Role,
    /// Dernier usage constaté, à la minute près. Nul tant que la clé n'a jamais servi.
    pub last_used_at: Option<DateTimeWithTimeZone>,
    /// L'échéance, si on lui en a donné une. Nulle : la clé ne périme pas.
    pub expires_at: Option<DateTimeWithTimeZone>,
    /// La révocation, datée. Nulle tant que la clé sert.
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:api_keys>
    // </rbs:relations:api_keys>
}

// <rbs:related:api_keys>
// </rbs:related:api_keys>

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
```

- [ ] **Step 5: Écrire la migration**

`migration.rs.jinja` :

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiKeys::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ApiKeys::UserId).uuid().not_null())
                    .col(ColumnDef::new(ApiKeys::Name).string().not_null())
                    // Les douze premiers caractères de la clé : `rbs_` et huit du tirage.
                    // De quoi reconnaître une clé dans une liste sans jamais la redonner.
                    .col(ColumnDef::new(ApiKeys::Prefix).string().not_null())
                    // L'empreinte, et jamais la clé : une base lue par un tiers ne lui
                    // donne aucune clé présentable.
                    .col(ColumnDef::new(ApiKeys::TokenHash).string().not_null())
                    .col(ColumnDef::new(ApiKeys::Role).string().not_null())
                    .col(
                        ColumnDef::new(ApiKeys::LastUsedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    // Nulle : la clé ne périme pas. C'est le défaut, et la rotation est
                    // un choix que `expires_in_days` rend à la création.
                    .col(
                        ColumnDef::new(ApiKeys::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    // Une date plutôt qu'un booléen : elle porte le booléen et le moment.
                    .col(
                        ColumnDef::new(ApiKeys::RevokedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_keys_user_id")
                            .from(ApiKeys::Table, ApiKeys::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Chaque requête authentifiée par clé cherche une ligne par cette colonne : c'est
        // le chemin de lecture le plus chaud du projet. Unique, aussi : deux lignes de
        // même empreinte seraient la même clé, et rien ne dirait laquelle a servi.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("idx_api_keys_token_hash")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::TokenHash)
                    .to_owned(),
            )
            .await?;

        // `GET /api-keys` et la révocation en masse lisent par le porteur.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_api_keys_user_id")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKeys::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    Id,
    UserId,
    Name,
    Prefix,
    TokenHash,
    Role,
    LastUsedAt,
    ExpiresAt,
    RevokedAt,
    CreatedAt,
    UpdatedAt,
}

/// La table des comptes, que pose le fragment `auth` : la clé référence son porteur.
#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
```

- [ ] **Step 6: Lancer les tests et les voir passer**

```bash
cargo test -p rbs-cli --lib -- templates::
cargo test -p rbs-cli --lib -- preset::
```

Attendu : tout passe. Les gabarits neufs sont rendus par `feature_templates()`, qui les
découvre par parcours du répertoire.

- [ ] **Step 7: Prouver que les tests discriminent**

Retirer la ligne `anchor = "auth_impl"` du manifeste — le TOML reste valide — et coller le
rouge de `the_api_keys_fragment_requires_auth_and_writes_into_its_implementation`. Puis
indenter de quatre espaces la première ligne du contenu `auth_impl` et coller le rouge de
`the_auth_impl_insertion_carries_no_leading_indentation_on_its_first_line`. Restaurer les deux.

- [ ] **Step 8: Lint puis commit**

```bash
cargo fmt --all --check; echo "exit: $?"
cargo clippy --workspace --all-targets -- -D warnings; echo "exit: $?"
git add crates/rbs-cli/templates/features/api-keys/ crates/rbs-cli/src/templates.rs crates/rbs-cli/src/preset.rs
git commit
```

Sujet : `feat(cli): pose la table des clés d'API et le manifeste de son fragment`

---

### Task 6: Le repository et le service — la lecture, le plafond, la trace

**Files:**
- Create: `crates/rbs-cli/templates/features/api-keys/repository.rs.jinja`, `service.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/api-keys/feature.toml` — y ajouter les deux
  `[[files]]` correspondants. Un `source =` qui désigne un fichier absent fait échouer trois
  tests parcourant tous les fragments : le manifeste ne déclare que ce qui existe.

**Interfaces:**
- Consomme : l'entité (tâche 5), `HasAuth::accept_key` (tâche 1), `crate::auth::guard::Accepted` désormais `pub(crate)` (tâche 4).
- Produit : `service::accept(&AppState, &str, &mut Extensions) -> Result<Claims>` — c'est ce que la délégation du manifeste appelle. Plus `service::{create, list, revoke, revoke_all}`, que la tâche 7 monte sur des routes. `repository::{active, create, list_for_user, revoke, revoke_all, touch}`.

Deux lectures, jamais de jointure : la convention du dépôt. La péremption et la révocation
sont **dans la condition** de la requête, pas après elle — la règle qu'`one_time_token::consume`
a posée.

- [ ] **Step 1: Écrire le repository**

`repository.rs.jinja` :

```rust
use rbs_core::Result;
use sea_orm::prelude::{DateTimeWithTimeZone, Expr, Uuid};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, Set,
};

use super::model::{ActiveModel, Column, Entity};
use crate::auth::model::Role;

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::model::Model;

/// La clé que porte cette empreinte, si elle est encore bonne à `maintenant`.
///
/// Révocation et péremption sont **dans la condition** et non dans une vérification qui
/// suivrait la lecture : une clé révoquée entre les deux ne doit pas ouvrir la requête en
/// cours, et une condition portée par le SQL ne laisse pas cette fenêtre.
pub async fn active<C: ConnectionTrait>(
    db: &C,
    fingerprint: &str,
    maintenant: DateTimeWithTimeZone,
) -> Result<Option<Model>> {
    Ok(Entity::find()
        .filter(Column::TokenHash.eq(fingerprint))
        .filter(Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(Column::ExpiresAt.is_null())
                .add(Column::ExpiresAt.gt(maintenant)),
        )
        .one(db)
        .await?)
}

/// Inscrit une clé. L'empreinte seule est écrite ; la clé est déjà partie vers l'appelant.
pub async fn create<C: ConnectionTrait>(
    db: &C,
    user_id: Uuid,
    name: &str,
    prefix: &str,
    fingerprint: &str,
    role: Role,
    expires_at: Option<DateTimeWithTimeZone>,
) -> Result<Model> {
    Ok(ActiveModel {
        user_id: Set(user_id),
        name: Set(name.to_owned()),
        prefix: Set(prefix.to_owned()),
        token_hash: Set(fingerprint.to_owned()),
        role: Set(role),
        expires_at: Set(expires_at),
        ..Default::default()
    }
    .insert(db)
    .await?)
}

/// Les clés d'un compte, révoquées comprises : la révocation est datée pour rester lisible.
pub async fn list_for_user<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::UserId.eq(user_id))
        .order_by_asc(Column::CreatedAt)
        .all(db)
        .await?)
}

/// Révoque une clé **du porteur donné**, et dit si c'est bien cet appel qui l'a fait.
///
/// `user_id` est dans la condition : sans lui, l'identifiant d'une clé d'autrui suffirait
/// à la couper. Le contrôleur rend 404 sur un `false`, jamais 403 — un 403 confirmerait
/// que la clé existe.
pub async fn revoke<C: ConnectionTrait>(db: &C, id: Uuid, user_id: Uuid) -> Result<bool> {
    let touchees = Entity::update_many()
        .col_expr(
            Column::RevokedAt,
            Expr::value(chrono::Utc::now().fixed_offset()),
        )
        .filter(Column::Id.eq(id))
        .filter(Column::UserId.eq(user_id))
        .filter(Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected == 1)
}

/// Révoque toutes les clés encore vivantes d'un compte, et dit combien.
pub async fn revoke_all<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<u64> {
    let touchees = Entity::update_many()
        .col_expr(
            Column::RevokedAt,
            Expr::value(chrono::Utc::now().fixed_offset()),
        )
        .filter(Column::UserId.eq(user_id))
        .filter(Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected)
}

/// Marque la clé comme ayant servi à `maintenant`.
///
/// La connexion est prise **par valeur**, là où toutes ses voisines l'empruntent : l'appel
/// part détaché, et une tâche détachée est `'static`. `DatabaseConnection` se clone à coût
/// nul, un `Arc` interne.
///
/// La borne temporelle est **aussi** dans la condition, alors que le service l'a déjà
/// vérifiée en mémoire : deux requêtes simultanées de la même clé la franchiraient sinon
/// toutes deux, et écriraient chacune leur tour.
pub async fn touch(
    db: sea_orm::DatabaseConnection,
    id: Uuid,
    maintenant: DateTimeWithTimeZone,
    seuil: chrono::TimeDelta,
) -> Result<bool> {
    let touchees = Entity::update_many()
        .col_expr(Column::LastUsedAt, Expr::value(maintenant))
        .filter(Column::Id.eq(id))
        .filter(
            Condition::any()
                .add(Column::LastUsedAt.is_null())
                .add(Column::LastUsedAt.lt(maintenant - seuil)),
        )
        .exec(&db)
        .await?;

    Ok(touchees.rows_affected == 1)
}
```

- [ ] **Step 2: Écrire le service**

`service.rs.jinja` :

```rust
use axum::http::Extensions;
use chrono::Utc;
use rbs_core::jwt::Claims;
use rbs_core::{Error, HasCoreState, Result};
use sea_orm::ActiveEnum;
use sea_orm::prelude::Uuid;

use super::dto::{ApiKeyCreated, ApiKeyResponse, CreateApiKey};
use super::repository;
use crate::auth::model::Role;
use crate::state::AppState;

/// Préfixe de toute clé émise. Reconnaissable d'un coup d'œil dans un fichier de secrets.
const PREFIXE: &str = "rbs_";

/// Ce qui reparaît d'une clé après sa création : `rbs_` et huit caractères du tirage.
const LONGUEUR_DU_PREFIXE: usize = 12;

/// En deçà, la trace d'usage n'est pas réécrite.
///
/// Une clé martelée à mille requêtes par seconde coûte **une** écriture par minute, et non
/// mille par seconde. La minute suffit à la seule question qu'on pose à cette colonne :
/// cette clé sert-elle encore ?
///
/// En secondes, comme tous les réglages de durée du projet : `TimeDelta::minutes` n'est
/// pas constructible dans un `const`.
const TRACE_SECS: i64 = 60;

/// Le même seuil, sous la forme qu'attendent les comparaisons de dates.
fn seuil() -> chrono::TimeDelta {
    chrono::TimeDelta::seconds(TRACE_SECS)
}

/// Juge une clé présentée en `X-Api-Key`, et rend les claims de son porteur.
///
/// Deux lectures et jamais de jointure, comme partout ailleurs dans un projet engendré :
/// la clé, puis le compte. La seconde n'est pas facultative — c'est elle qui fait qu'une
/// clé cesse d'administrer le jour où son porteur est rétrogradé.
pub async fn accept(state: &AppState, key: &str, extensions: &mut Extensions) -> Result<Claims> {
    let maintenant = Utc::now().fixed_offset();
    let empreinte = rbs_core::token::fingerprint(key);

    let cle = repository::active(state.core().db(), &empreinte, maintenant)
        .await?
        // Clé inconnue, révoquée ou périmée : la même réponse pour les trois. Les
        // distinguer renseignerait sur l'état d'une clé qu'on n'a pas.
        .ok_or(Error::Unauthorized)?;

    let compte = crate::auth::repository::find(state.core().db(), cle.user_id)
        .await?
        .ok_or(Error::Unauthorized)?;

    // Le moindre des deux : une clé ne vaut jamais plus que le compte qui la porte, et le
    // plafond se recalcule à chaque requête plutôt que de se figer à la création.
    let servi = cle.role.clone().min(compte.role.clone());

    // La borne est tenue ici, avant tout aller-retour : la ligne vient d'être lue, et une
    // condition portée par le seul `UPDATE` coûterait une écriture par requête pour ne
    // rien changer cinquante-neuf fois sur soixante.
    if cle
        .last_used_at
        .is_none_or(|trace| maintenant - trace >= seuil())
    {
        let db = state.core().db().clone();
        let id = cle.id;
        // Détachée : la réponse n'attend pas l'écriture, et une trace manquée vaut mieux
        // qu'un appel refusé.
        tokio::spawn(async move {
            let _ = repository::touch(db, id, maintenant, seuil()).await;
        });
    }

    // Ce que la garde `VerifiedIdentity` cherchera : sans ce dépôt, elle relirait le compte
    // que l'on vient de lire.
    extensions.insert(crate::auth::guard::Accepted(compte.email_verified_at));

    Ok(Claims {
        sub: compte.id.to_string(),
        role: servi.to_value(),
        // L'échéance de la clé, ou une échéance qui n'arrive pas. Le noyau ne la revérifie
        // pas : la péremption est dans la condition de la lecture ci-dessus, et une
        // seconde garde ailleurs divergerait un jour.
        exp: cle.expires_at.map_or(i64::MAX, |date| date.timestamp()),
        iat: cle.created_at.timestamp(),
        // L'identifiant de la clé : c'est ce qui met dans les journaux *laquelle* a servi.
        jti: cle.id.to_string(),
    })
}

/// Tire une clé pour `porteur`, au rôle demandé si son créateur peut le donner.
///
/// Rend la clé en clair. C'est la seule fois qu'elle existe hors du processus de l'appelant.
pub async fn create(
    state: &AppState,
    porteur: Uuid,
    role_du_createur: Role,
    input: CreateApiKey,
) -> Result<ApiKeyCreated> {
    let demande = match input.role.as_deref() {
        // Le défaut est le rôle le moins ouvert, et non celui du créateur : une clé qui
        // administre doit être demandée, jamais obtenue par omission.
        None => Role::User,
        Some(nom) => Role::try_from_value(&nom.to_owned()).map_err(|_| {
            Error::BadRequest(format!("rôle inconnu : {nom}"))
        })?,
    };

    // Le plafond, à la création : nul ne délègue plus qu'il ne détient.
    if demande > role_du_createur {
        return Err(Error::Forbidden);
    }

    let clair = format!("{PREFIXE}{}", rbs_core::token::random());
    let prefixe: String = clair.chars().take(LONGUEUR_DU_PREFIXE).collect();
    let expires_at = input
        .expires_in_days
        .map(|jours| Utc::now().fixed_offset() + chrono::TimeDelta::days(i64::from(jours)));

    let cle = repository::create(
        state.core().db(),
        porteur,
        &input.name,
        &prefixe,
        &rbs_core::token::fingerprint(&clair),
        demande,
        expires_at,
    )
    .await?;

    Ok(ApiKeyCreated {
        id: cle.id,
        name: cle.name,
        prefix: cle.prefix,
        role: cle.role.to_value(),
        expires_at: cle.expires_at,
        key: clair,
    })
}

/// Les clés du porteur, sans rien qui permette de les présenter.
pub async fn list(state: &AppState, porteur: Uuid) -> Result<Vec<ApiKeyResponse>> {
    Ok(repository::list_for_user(state.core().db(), porteur)
        .await?
        .into_iter()
        .map(ApiKeyResponse::from)
        .collect())
}

/// Révoque une clé du porteur. `false` si elle n'est pas à lui, ou déjà révoquée.
pub async fn revoke(state: &AppState, id: Uuid, porteur: Uuid) -> Result<bool> {
    repository::revoke(state.core().db(), id, porteur).await
}

/// Révoque toutes les clés vivantes du porteur, et dit combien.
pub async fn revoke_all(state: &AppState, porteur: Uuid) -> Result<u64> {
    repository::revoke_all(state.core().db(), porteur).await
}
```

- [ ] **Step 3: Vérifier que les gabarits se rendent**

```bash
cargo test -p rbs-cli --lib -- templates::
```

Attendu : vert. `feature_templates()` rend chaque gabarit du répertoire, et
`UndefinedBehavior::Strict` ferait échouer toute variable oubliée — ces deux fichiers n'en
interpolent aucune.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all --check; echo "exit: $?"
git add crates/rbs-cli/templates/features/api-keys/
git commit
```

Sujet : `feat(cli): juge une clé par deux lectures, un plafond et une trace bornée`

---

### Task 7: Les DTO et les quatre routes

**Files:**
- Create: `crates/rbs-cli/templates/features/api-keys/dto.rs.jinja`, `controller.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/api-keys/feature.toml` — y ajouter les deux
  `[[files]]` correspondants, même raison qu'à la tâche 6.

**Interfaces:**
- Consomme : `service::{create, list, revoke, revoke_all}` (tâche 6).
- Produit : `controller::{create, list, revoke, revoke_all}`, les quatre chemins que l'ancre `openapi` du manifeste cite nommément. `dto::{CreateApiKey, ApiKeyCreated, ApiKeyResponse}`.

- [ ] **Step 1: Écrire les DTO**

`dto.rs.jinja` :

```rust
use axum::Json;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use sea_orm::ActiveEnum;
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateApiKey {
    /// Ce que la clé sert. Sans lui, une liste de clés est illisible au bout de trois.
    #[validate(length(min = 1, max = 80))]
    pub name: String,
    /// `user` par défaut : une clé qui administre se demande, elle ne s'obtient pas par
    /// omission. Refusé au-delà du rôle du créateur.
    pub role: Option<String>,
    /// Sans échéance, la clé ne périme pas — c'est un choix, pas un oubli.
    ///
    /// Bornée des deux côtés : à zéro la clé naîtrait déjà périmée, silencieusement
    /// inutile ; au-delà de dix ans, le calcul d'échéance sort de ce que `chrono` sait
    /// représenter et l'addition panique, ce qu'un appelant authentifié ne doit pas
    /// pouvoir provoquer avec un seul champ.
    #[validate(range(min = 1, max = 3650))]
    pub expires_in_days: Option<u32>,
}

/// Ce que rend la création, et cette seule fois.
///
/// `key` n'y paraît qu'ici : une seule lecture de la liste livrerait sinon toutes les clés
/// du compte d'un coup.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyCreated {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub role: String,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub expires_at: Option<DateTimeWithTimeZone>,
    /// La clé en clair. Elle n'existe nulle part ailleurs : ni la base, ni cette API ne
    /// sauront la redonner.
    pub key: String,
}

/// La création porte sa propre réponse pour n'être jamais mise en cache.
///
/// L'en-tête tient au type et non au handler : un second handler qui rendrait ce corps le
/// rendrait sinon sans lui, et rien ne le signalerait.
impl IntoResponse for ApiKeyCreated {
    fn into_response(self) -> Response {
        (
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::PRAGMA, "no-cache"),
            ],
            Json(self),
        )
            .into_response()
    }
}

/// La vue publique d'une clé. Jamais `token_hash`, jamais la clé.
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub role: String,
    /// Nul : cette clé n'a jamais servi. C'est ce qui permet de la révoquer sans crainte.
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_used_at: Option<DateTimeWithTimeZone>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub expires_at: Option<DateTimeWithTimeZone>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub revoked_at: Option<DateTimeWithTimeZone>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}

impl From<super::model::Model> for ApiKeyResponse {
    fn from(cle: super::model::Model) -> Self {
        Self {
            id: cle.id,
            name: cle.name,
            prefix: cle.prefix,
            role: cle.role.to_value(),
            last_used_at: cle.last_used_at,
            expires_at: cle.expires_at,
            revoked_at: cle.revoked_at,
            created_at: cle.created_at,
        }
    }
}
```

- [ ] **Step 2: Écrire le contrôleur**

`controller.rs.jinja` :

```rust
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{Error, Identity, ProblemDetails, Result, ValidatedJson};
use sea_orm::ActiveEnum;
use sea_orm::prelude::Uuid;

use super::dto::{ApiKeyCreated, ApiKeyResponse, CreateApiKey};
use super::service;
use crate::auth::model::Role;
use crate::state::AppState;

// Les quatre routes n'exigent aucun rôle : chacun administre ses propres clés, comme
// chacun administre ses propres sessions. Ce qu'une clé créée ici pourra faire est borné
// par le rôle de son créateur, que `service::create` vérifie.
//
// Une clé peut en créer une autre : le plafond interdit l'escalade, et l'interdire
// casserait le provisionnement automatisé, qui est la raison d'être du fragment.

#[utoipa::path(
    post,
    path = "/api-keys",
    tag = "api-keys",
    operation_id = "api_keys_create",
    security(("bearer" = []), ("api_key" = [])),
    request_body = CreateApiKey,
    responses(
        (status = 201, description = "clé tirée, rendue cette seule fois", body = ApiKeyCreated),
        (status = 400, description = "rôle inconnu", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "justificatif absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle demandé supérieur à celui de l'appelant", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<CreateApiKey>,
) -> Result<(StatusCode, ApiKeyCreated)> {
    let porteur = identite.user_uuid()?;
    // Le rôle de l'appelant tel qu'il est porté par son justificatif : c'est déjà le
    // moindre du rôle de la clé et de celui du compte quand l'appel vient d'une clé.
    let sien = Role::try_from_value(&identite.role).map_err(|_| Error::Forbidden)?;

    let creee = service::create(&state, porteur, sien, input).await?;

    Ok((StatusCode::CREATED, creee))
}

#[utoipa::path(
    get,
    path = "/api-keys",
    tag = "api-keys",
    operation_id = "api_keys_list",
    security(("bearer" = []), ("api_key" = [])),
    responses(
        (status = 200, description = "les clés de l'appelant, révoquées comprises, sans de quoi les présenter", body = Vec<ApiKeyResponse>),
        (status = 401, description = "justificatif absent ou invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    identite: Identity,
) -> Result<Json<Vec<ApiKeyResponse>>> {
    let porteur = identite.user_uuid()?;

    Ok(Json(service::list(&state, porteur).await?))
}

#[utoipa::path(
    delete,
    path = "/api-keys/{id}",
    tag = "api-keys",
    operation_id = "api_keys_revoke",
    security(("bearer" = []), ("api_key" = [])),
    params(("id" = Uuid, Path, description = "identifiant de la clé")),
    responses(
        (status = 204, description = "clé révoquée"),
        (status = 401, description = "justificatif absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "aucune clé vivante de l'appelant sous cet identifiant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn revoke(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let porteur = identite.user_uuid()?;

    // 404 et non 403 : un 403 confirmerait que cette clé existe chez quelqu'un d'autre.
    if service::revoke(&state, id, porteur).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound("clé d'API"))
    }
}

#[utoipa::path(
    delete,
    path = "/api-keys",
    tag = "api-keys",
    operation_id = "api_keys_revoke_all",
    security(("bearer" = []), ("api_key" = [])),
    responses(
        (status = 204, description = "toutes les clés de l'appelant révoquées, celle qui appelle comprise"),
        (status = 401, description = "justificatif absent ou invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn revoke_all(
    State(state): State<AppState>,
    identite: Identity,
) -> Result<StatusCode> {
    let porteur = identite.user_uuid()?;

    service::revoke_all(&state, porteur).await?;

    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 3: Vérifier le rendu et committer**

```bash
cargo test -p rbs-cli --lib -- templates::
cargo fmt --all --check; echo "exit: $?"
git add crates/rbs-cli/templates/features/api-keys/
git commit
```

Sujet : `feat(cli): donne quatre routes à l'administration des clés d'API`

---

### Task 8: Les tests que le fragment livre

**Files:**
- Create: `crates/rbs-cli/templates/features/api-keys/tests/mod.rs.jinja`, `tests/accept.rs.jinja`, `tests/routes.rs.jinja`
- Create: `crates/rbs-cli/templates/features/api-keys/mod.rs.jinja` — reporté de la tâche 5,
  son `routes()` référençant des handlers qui n'existaient pas encore (voir l'amendement de
  la tâche 5). Le code à écrire est celui que la tâche 5 donne.
- Modify: `crates/rbs-cli/templates/features/api-keys/feature.toml` — y ajouter les trois
  `[[files]]` des tests **et celui de `mod.rs.jinja`**. Le manifeste est alors complet :
  neuf entrées, neuf gabarits, et les tests qui parcourent les fragments repassent au vert.

**Interfaces:**
- Consomme : tout le fragment (tâches 5 à 7).
- Produit : les noms de tests que la suite d'intégration (tâche 11) exigera nommément. Les fixer ici, ils deviennent un contrat.

C'est le seul endroit où le code du fragment est **exécuté**. Tous portent `#[ignore]` : ils
joignent la base que décrit `.env`.

- [ ] **Step 1: Écrire le harnais**

`tests/mod.rs.jinja` :

```rust
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use rbs_core::HasCoreState;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait};
use serde_json::Value;
use tokio::sync::{Mutex, MutexGuard};

use super::model::Entity;
use crate::auth::model::Role;
use crate::state::AppState;

mod accept;
mod routes;

// Les tests qui suivent joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

/// Les tests se relaient sur l'unique table `api_keys` plutôt que de se voler leurs lignes.
fn verrou_base() -> &'static Mutex<()> {
    static VERROU: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    VERROU.get_or_init(Mutex::default)
}

/// La table vidée, et l'état du projet pour la tenir.
async fn table_a_soi() -> (MutexGuard<'static, ()>, AppState) {
    let garde = verrou_base().lock().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    Entity::delete_many()
        .exec(&db)
        .await
        .expect("la table api_keys doit se vider");

    (
        garde,
        AppState::new(db, config).expect("état constructible"),
    )
}

/// Inscrit un compte au rôle voulu, et rend son identifiant.
///
/// Un compte réel : la clé est relue avec lui à chaque requête, et une clé sans porteur
/// est refusée.
async fn compte(db: &DatabaseConnection, role: Role) -> Uuid {
    let inscrit = crate::auth::repository::create(
        db,
        &format!("{}@exemple.test", Uuid::new_v4()),
        "hash sans valeur",
    )
    .await
    .expect("le compte s'insère")
    .expect("l'adresse est neuve");

    let mut promu: crate::auth::model::user::ActiveModel = inscrit.into();
    promu.role = Set(role);

    promu.update(db).await.expect("rôle posé").id
}

/// Tire une clé pour `porteur` par le service, et rend la valeur en clair.
async fn cle(state: &AppState, porteur: Uuid, createur: Role, role: Option<&str>) -> String {
    super::service::create(
        state,
        porteur,
        createur,
        super::dto::CreateApiKey {
            name: "test".to_owned(),
            role: role.map(str::to_owned),
            expires_in_days: None,
        },
    )
    .await
    .expect("la clé se tire")
    .key
}

/// Une requête portant la clé en `X-Api-Key`.
fn par_cle(method: &str, path: &str, cle: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(path)
        .header("x-api-key", cle);
    match body {
        Some(corps) => builder
            .header("content-type", "application/json")
            .body(Body::from(corps.to_string())),
        None => builder.body(Body::empty()),
    }
    .expect("requête bien formée")
}

/// Fait traverser le routeur à `request`, et rend son statut, son corps et ses en-têtes.
async fn call(api: &Router, request: Request<Body>) -> (StatusCode, Value, axum::http::HeaderMap) {
    let response = api
        .clone()
        .oneshot(request)
        .await
        .expect("l'application doit répondre");
    let status = response.status();
    let entetes = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        entetes,
    )
}
```

Ajouter `use tower::ServiceExt;` en tête — `oneshot` en dépend.

- [ ] **Step 2: Écrire les tests d'acceptation**

`tests/accept.rs.jinja` — **sept** tests, dont les trois qui portent les décisions de la
spec. Le septième, `an_expired_key_is_refused_like_an_unknown_one`, a été ajouté après
revue : sans lui, la branche `ExpiresAt.gt(maintenant)` de `repository::active` n'était
éprouvée par rien — l'aide `cle()` codant `expires_in_days: None` en dur, les dix clés des
tests passaient toutes par `is_null()`, et une inversion du comparateur serait restée
invisible. Il exige une aide voisine, `cle_perimee`, qui tire une clé avec échéance puis la
recule dans le passé par l'entité.

```rust
use super::*;

use axum::http::Extensions;

/// Le cœur de la feature : une clé ouvre ce qu'un jeton ouvre, sans qu'aucun contrôleur
/// n'ait été récrit.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_valid_key_identifies_its_bearer() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let clair = cle(&state, porteur, Role::User, None).await;

    let claims = super::super::service::accept(&state, &clair, &mut Extensions::new())
        .await
        .expect("la clé est bonne");

    assert_eq!(claims.sub, porteur.to_string());
    assert_eq!(claims.role, "user");
}

/// Le plafond, dans le sens qui compte : une clé d'administrateur cesse d'administrer le
/// jour où son porteur est rétrogradé, sans qu'on ait eu à penser à la révoquer.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_key_of_a_demoted_owner_no_longer_administers() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::Admin).await;
    let clair = cle(&state, porteur, Role::Admin, Some("admin")).await;

    let avant = super::super::service::accept(&state, &clair, &mut Extensions::new())
        .await
        .expect("la clé est bonne");
    assert_eq!(avant.role, "admin");

    let compte_lu = crate::auth::repository::find(&db, porteur)
        .await
        .expect("lecture possible")
        .expect("le compte existe");
    let mut retrograde: crate::auth::model::user::ActiveModel = compte_lu.into();
    retrograde.role = Set(Role::User);
    retrograde.update(&db).await.expect("rétrogradation");

    let apres = super::super::service::accept(&state, &clair, &mut Extensions::new())
        .await
        .expect("la clé reste valide, son rôle non");

    assert_eq!(apres.role, "user", "le plafond doit suivre le porteur");
}

/// Nul ne délègue plus qu'il ne détient.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_user_cannot_mint_an_admin_key() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;

    let refus = super::super::service::create(
        &state,
        porteur,
        Role::User,
        super::super::dto::CreateApiKey {
            name: "escalade".to_owned(),
            role: Some("admin".to_owned()),
            expires_in_days: Some(1),
        },
    )
    .await;

    assert!(matches!(refus, Err(rbs_core::Error::Forbidden)));
}

/// Révoquée, périmée, inconnue : la même réponse pour les trois.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_revoked_key_is_refused_like_an_unknown_one() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let clair = cle(&state, porteur, Role::User, None).await;

    let revoquees = super::super::service::revoke_all(&state, porteur)
        .await
        .expect("la révocation aboutit");
    assert_eq!(revoquees, 1);

    let refus = super::super::service::accept(&state, &clair, &mut Extensions::new()).await;
    let inconnue =
        super::super::service::accept(&state, "rbs_inconnue", &mut Extensions::new()).await;

    assert!(matches!(refus, Err(rbs_core::Error::Unauthorized)));
    assert!(matches!(inconnue, Err(rbs_core::Error::Unauthorized)));
}

/// La trace est bornée : trois acceptations rapprochées n'écrivent qu'une fois.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn three_close_calls_write_the_usage_trace_once() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let clair = cle(&state, porteur, Role::User, None).await;

    for _ in 0..3 {
        super::super::service::accept(&state, &clair, &mut Extensions::new())
            .await
            .expect("la clé est bonne");
    }
    // L'écriture part détachée : lui laisser le temps d'aboutir avant de lire.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let lue = Entity::find()
        .all(&db)
        .await
        .expect("lecture possible")
        .pop()
        .expect("la clé est en base");
    let premier = lue.last_used_at.expect("la première acceptation a tracé");

    for _ in 0..3 {
        super::super::service::accept(&state, &clair, &mut Extensions::new())
            .await
            .expect("la clé est bonne");
    }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let relue = Entity::find()
        .all(&db)
        .await
        .expect("lecture possible")
        .pop()
        .expect("la clé est en base");

    assert_eq!(
        relue.last_used_at.expect("la trace reste"),
        premier,
        "six acceptations dans la même minute ne doivent écrire qu'une fois"
    );
}

/// Ce que l'acceptation dépose épargne une lecture à la garde qui suit.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn accepting_a_key_leaves_the_verification_date_in_the_extensions() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let clair = cle(&state, porteur, Role::User, None).await;
    let mut extensions = Extensions::new();

    super::super::service::accept(&state, &clair, &mut extensions)
        .await
        .expect("la clé est bonne");

    assert!(
        extensions.get::<crate::auth::guard::Accepted>().is_some(),
        "sans ce dépôt, `VerifiedIdentity` relirait le compte"
    );
}
```

- [ ] **Step 3: Écrire les tests de routes**

`tests/routes.rs.jinja` :

```rust
use super::*;

use serde_json::json;

/// La clé n'existe qu'une fois hors du processus de l'appelant : une seule lecture de la
/// liste livrerait sinon toutes les clés du compte.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_key_is_returned_once_and_never_by_the_list() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let premiere = cle(&state, porteur, Role::User, None).await;
    let api = crate::router::router(state.clone());

    let (statut, corps, _) = call(&api, par_cle("GET", "/api-keys", &premiere, None)).await;

    assert_eq!(statut, StatusCode::OK, "{corps}");
    let rendu = corps.to_string();
    assert!(!rendu.contains(&premiere), "la liste livre la clé : {rendu}");
    assert!(rendu.contains("\"prefix\""), "la liste doit porter le préfixe : {rendu}");
}

/// La création porte `no-store` par son type : un mandataire qui la mettrait en cache
/// servirait la clé à l'appelant suivant.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_creation_response_is_never_cached() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let sienne = cle(&state, porteur, Role::User, None).await;
    let api = crate::router::router(state.clone());

    let (statut, corps, entetes) = call(
        &api,
        par_cle("POST", "/api-keys", &sienne, Some(json!({ "name": "ci" }))),
    )
    .await;

    assert_eq!(statut, StatusCode::CREATED, "{corps}");
    assert_eq!(
        entetes
            .get(axum::http::header::CACHE_CONTROL)
            .expect("l'en-tête est posé"),
        "no-store"
    );
    assert!(corps["key"].is_string(), "la clé est rendue ici : {corps}");
}

/// 404 et non 403 : un 403 confirmerait que cette clé existe chez quelqu'un d'autre.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn revoking_a_key_of_another_account_answers_404() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let mien = compte(&db, Role::User).await;
    let autre = compte(&db, Role::User).await;
    let ma_cle = cle(&state, mien, Role::User, None).await;
    cle(&state, autre, Role::User, None).await;

    let sienne = Entity::find()
        .all(&db)
        .await
        .expect("lecture possible")
        .into_iter()
        .find(|ligne| ligne.user_id == autre)
        .expect("la clé de l'autre compte est en base");
    let api = crate::router::router(state.clone());

    let (statut, corps, _) = call(
        &api,
        par_cle(
            "DELETE",
            &format!("/api-keys/{}", sienne.id),
            &ma_cle,
            None,
        ),
    )
    .await;

    assert_eq!(statut, StatusCode::NOT_FOUND, "{corps}");
}

/// La propriété qui fait tout le fragment : une route gardée par `Identity`, écrite sans
/// rien savoir des clés, s'ouvre à une clé.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_key_opens_a_route_that_identity_guards() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db().clone();
    let porteur = compte(&db, Role::User).await;
    let sienne = cle(&state, porteur, Role::User, None).await;
    let api = crate::router::router(state.clone());

    let (statut, corps, _) = call(&api, par_cle("GET", "/auth/me", &sienne, None)).await;

    assert_eq!(statut, StatusCode::OK, "{corps}");
    assert_eq!(corps["id"], porteur.to_string(), "{corps}");
}
```

- [ ] **Step 4: Vérifier le rendu, committer**

```bash
cargo test -p rbs-cli --lib -- templates::
cargo fmt --all --check; echo "exit: $?"
git add crates/rbs-cli/templates/features/api-keys/tests/
git commit
```

Sujet : `test(cli): livre avec le fragment les tests de sa clé et de son plafond`

---

### Task 9: Le conseil de suite

**Files:**
- Modify: `crates/rbs-cli/src/lib.rs:785-855` (`suite`)
- Test: `crates/rbs-cli/src/lib.rs` (module `tests`)

**Interfaces:**
- Consomme : le nom du fragment (tâche 5).
- Produit : rien que d'autres tâches consomment. Mais le test existant qui parcourt **tous** les installables et exige un `suite()` pour chacun rougit sans cette tâche.

- [ ] **Step 1: Écrire le test qui échoue**

```rust
    /// La table est vide, et une clé ne se tire que par une route : installé et jamais
    /// appelé, le fragment paraît sans effet.
    #[test]
    fn the_api_keys_fragment_advises_the_migration_and_the_minting_route() {
        let conseil = suite("api-keys").expect("le fragment pose une table : il doit conseiller");

        assert!(conseil.contains("rbs migrate up"), "{conseil}");
        assert!(conseil.contains("POST /api-keys"), "{conseil}");
        assert!(conseil.contains("X-Api-Key"), "{conseil}");
    }
```

- [ ] **Step 2: Lancer et voir échouer**

```bash
cargo test -p rbs-cli --lib -- lib::tests::the_api_keys_fragment
```

Attendu : `called 'Option::expect()' on a 'None' value`.

- [ ] **Step 3: Ajouter le bras**

```rust
        // La table naît vide et aucune clé n'existe : sans une première émission, le
        // fragment est installé et rien ne change : c'est le seul endroit où le geste se dit.
        "api-keys" => Some(
            "rbs migrate up, puis POST /api-keys pour tirer une clé — elle n'est rendue \
             qu'à cet instant — et présentez-la en X-Api-Key",
        ),
```

- [ ] **Step 4: Lancer et voir passer**

```bash
cargo test -p rbs-cli --lib -- lib::
```

Attendu : le test neuf passe, et celui qui parcourt tous les installables aussi.

- [ ] **Step 5: Lint puis commit**

Sujet : `feat(cli): dit ce qu'il reste à faire après l'installation des clés d'API`

---

### Task 10: Le contrôle `doctor`

**Files:**
- Create: `crates/rbs-cli/src/doctor/api_keys.rs`
- Modify: `crates/rbs-cli/src/doctor/mod.rs:8` (déclaration du module), `:340` (`FEATURE_CHECKS`, 14 → 15)
- Test: dans le module créé, et `doctor/mod.rs` (`the_fragment_checks_follow_the_order_of_the_table`)

**Interfaces:**
- Consomme : le contenu de l'ancre `auth_impl` du manifeste (tâche 5).
- Produit : un contrôle intitulé `api-keys`.

Sans ce contrôle, un projet dont la délégation manque compile, démarre, et rend 401 à toute
clé — sans que rien ne le dise. C'est exactement le trou qu'un fragment posé sur un projet
sans l'ancre laisserait.

- [ ] **Step 1: Écrire le contrôle et ses tests**

Sur le modèle exact de `crates/rbs-cli/src/doctor/webhooks.rs`, y compris son dernier test
qui relit le `feature.toml` pour garantir que la constante cherchée est bien celle que le
fragment insère :

```rust
//! Contrôle de la feature `api-keys`.
//!
//! Le noyau ne juge une clé que si le projet le lui dit : sans la délégation posée dans
//! `impl HasAuth for AppState`, le défaut du trait refuse, et toute clé rend 401 sur un
//! projet qui compile et démarre. Un projet engendré avant que l'ancre n'existe reçoit le
//! bloc à coller au lieu de l'insertion — et c'est précisément le cas que ce contrôle voit.

use std::path::Path;

use super::Check;

pub(crate) const TITRE: &str = "api-keys";
const FICHIER: &str = "src/auth/mod.rs";
/// L'aiguille de détection : la ligne intérieure de la méthode, cherchée entière et
/// ébarbée, de sorte que l'indentation de l'insertion ne la fasse pas manquer.
const DELEGATION: &str = "crate::modules::api_keys::service::accept(self, key, extensions).await";

/// Le bloc que le fragment insère : une méthode, non une instruction.
///
/// Le remède le cite en entier parce qu'une instruction nue n'est pas un item d'`impl` —
/// collée seule entre les balises, elle ne compilerait pas. C'est ce qui distingue ce
/// contrôle de celui des webhooks, dont l'ancre tient sur une ligne et dont la constante
/// sert donc à la fois d'aiguille et de remède.
const BLOC: &str = r#"/// Ce que vaut une clé d'API présentée en `X-Api-Key`.
///
/// Le noyau ne connaît ni la table des clés ni la règle du plafond : il demande.
async fn accept_key(
    &self,
    key: &str,
    extensions: &mut Extensions,
) -> rbs_core::Result<Claims> {
    crate::modules::api_keys::service::accept(self, key, extensions).await
}"#;

pub(crate) fn check(root: &Path) -> Check {
    let source = match super::lire(root, TITRE, FICHIER) {
        Ok(source) => source,
        Err(constat) => return constat,
    };

    // Ligne entière, indentation ôtée : une délégation en commentaire ne délègue rien.
    if source.lines().any(|ligne| ligne.trim() == DELEGATION) {
        return Check::ok(TITRE, "le projet juge les clés d'API qu'on lui présente");
    }

    Check::failed(
        TITRE,
        "la délégation des clés d'API n'est pas posée : toute clé rendra 401",
        format!("dans {FICHIER}, entre les balises de `// <rbs:auth_impl>` :\n{BLOC}"),
    )
}
```

Ses tests, dans le même fichier :

```rust
#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// L'`impl HasAuth` tel que le fragment `auth` le livre, l'ancre portant `ligne`.
    fn implementation(ligne: &str) -> String {
        format!(
            "impl HasAuth for AppState {{\n    async fn accept(&self, claims: &Claims) \
             -> rbs_core::Result<()> {{\n        admit(self, claims).await.map(drop)\n    }}\n\
             \n    // <rbs:auth_impl>\n{ligne}    // </rbs:auth_impl>\n}}\n"
        )
    }

    fn projet(source: Option<&str>) -> TempDir {
        let racine = TempDir::new().expect("répertoire temporaire créable");

        if let Some(source) = source {
            let fichier = racine.path().join(FICHIER);
            fs::create_dir_all(fichier.parent().expect("le fichier a un parent"))
                .expect("répertoire du module auth créable");
            fs::write(&fichier, source).expect("module inscriptible");
        }

        racine
    }

    #[test]
    fn a_posted_delegation_reports_nothing() {
        let racine = projet(Some(&implementation(&format!("    {DELEGATION}\n"))));

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// Le défaut que rien d'autre ne signale : le projet compile, démarre, et rend 401 à
    /// toute clé.
    #[test]
    fn a_missing_delegation_names_its_consequence_and_the_line_to_paste() {
        let racine = projet(Some(&implementation("")));

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("401"), "{}", check.detail);
        let remede = check.remedy.expect("un échec porte son remède");
        assert!(remede.contains(DELEGATION), "{remede}");
        assert!(remede.contains("<rbs:auth_impl>"), "{remede}");
        assert!(remede.contains(FICHIER), "{remede}");
    }

    #[test]
    fn a_delegation_in_a_comment_does_not_count() {
        let racine = projet(Some(&implementation(&format!("    // {DELEGATION}\n"))));

        assert_eq!(check(racine.path()).state, State::Echec);
    }

    #[test]
    fn a_missing_auth_module_is_named() {
        let racine = projet(None);

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains(FICHIER), "{}", check.detail);
    }

    /// La ligne cherchée est celle que le fragment insère : l'ancre `auth_impl` de son
    /// manifeste. Sans ce test, les deux dériveraient en silence et le contrôle
    /// signalerait une délégation pourtant posée.
    #[test]
    fn the_line_is_the_one_the_fragment_inserts() {
        let manifeste: toml_edit::DocumentMut = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/api-keys/feature.toml"
        ))
        .expect("le manifeste du fragment se lit")
        .parse()
        .expect("le manifeste du fragment s'analyse");

        let inseree = manifeste
            .get("anchors")
            .and_then(toml_edit::Item::as_array_of_tables)
            .and_then(|ancres| {
                ancres.iter().find(|ancre| {
                    ancre.get("anchor").and_then(toml_edit::Item::as_str) == Some("auth_impl")
                })
            })
            .and_then(|ancre| ancre.get("content"))
            .and_then(toml_edit::Item::as_str)
            .expect("le fragment insère dans l'ancre `auth_impl`");

        assert!(
            inseree.lines().any(|ligne| ligne.trim() == DELEGATION),
            "le manifeste n'insère pas la ligne cherchée :\n{inseree}"
        );
        assert_eq!(
            inseree.trim(),
            BLOC,
            "le bloc du remède doit être exactement ce que le fragment insère"
        );
    }
}
```

- [ ] **Step 2: Inscrire le contrôle**

`doctor/mod.rs` : `pub mod api_keys;` après `pub mod anchors;` (ordre alphabétique), puis
l'entrée dans `FEATURE_CHECKS`, juste après celle d'`auth` — les deux se lisent ensemble :

```rust
const FEATURE_CHECKS: [(&str, Controle); 15] = [
    (
        "auth",
        Controle { titre: auth::TITRE, executer: |projet, _| auth::check(&projet.root, &projet.config) },
    ),
    (
        "api-keys",
        Controle { titre: api_keys::TITRE, executer: |projet, _| api_keys::check(&projet.root) },
    ),
    // … le reste inchangé
];
```

- [ ] **Step 3: Lancer, voir passer, prouver la discrimination**

```bash
cargo test -p rbs-cli --lib -- doctor::api_keys
cargo test -p rbs-cli --lib -- doctor::
```

Mutation : remplacer `ligne.trim() == DELEGATION` par `source.contains(DELEGATION)` — le
code compile — et `a_delegation_in_a_comment_does_not_count` doit rougir. Coller, restaurer.

- [ ] **Step 4: Lint puis commit**

Sujet : `feat(cli): signale un projet qui ne juge pas les clés qu'on lui présentera`

---

### Task 11: La suite d'intégration

**Files:**
- Create: `crates/rbs-cli/tests/integration_api_keys.rs`

**Interfaces:**
- Consomme : tout ce qui précède.
- Produit : la seule preuve que le fragment fonctionne réellement.

Sur le modèle d'`integration_webhooks.rs`. Trois portées : la lecture d'ancres sur chaque
PR, puis, sous `#[ignore]`, la compilation du projet et l'exécution de ses tests contre un
PostgreSQL de `testcontainers`.

- [ ] **Step 1: Écrire le test d'ancres (sans Docker)**

```rust
/// L'insertion tombe **à l'intérieur** du bloc `impl`, et non entre deux items : c'est la
/// seule chose que l'ancre `auth_impl` a de particulier, et la seule qui puisse casser.
#[test]
fn adding_the_fragment_writes_the_delegation_inside_the_auth_implementation() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "init");

    Command::cargo_bin("rbs")
        .expect("le binaire est construit")
        .args(["add", "api-keys"])
        .current_dir(&racine)
        .assert()
        .success();

    let module = fs::read_to_string(racine.join("src/auth/mod.rs")).expect("le module se lit");
    let ouverture = module.find("impl HasAuth for AppState {").expect("l'impl est là");
    let fermeture = module[ouverture..].find("\n}").expect("l'impl se ferme") + ouverture;
    let delegation = module
        .find("crate::modules::api_keys::service::accept")
        .expect("la délégation est posée");
    assert!(
        ouverture < delegation && delegation < fermeture,
        "la délégation est tombée hors du bloc impl :\n{module}"
    );

    for (fichier, attendu) in [
        ("src/router.rs", ".merge(crate::modules::api_keys::routes())"),
        ("src/openapi.rs", "crate::modules::api_keys::controller::create,"),
        ("migration/src/lib.rs", "create_api_keys"),
    ] {
        let source = fs::read_to_string(racine.join(fichier)).expect("le fichier se lit");
        assert!(source.contains(attendu), "{fichier} ne porte pas `{attendu}`");
    }

    Command::cargo_bin("rbs")
        .expect("le binaire est construit")
        .arg("doctor")
        .current_dir(&racine)
        .assert()
        .success()
        .stdout(predicates::str::contains("api-keys"));
```

> **Deux inexactitudes relevées à l'exécution.** `predicates` **n'est pas** une dépendance
> de `rbs-cli` : employer `predicates::str::contains` ne compile pas. Capturer la sortie et
> l'éprouver par un `assert!` ordinaire.
>
> Et `Anchor::block()` ne rend que `"{ouvrante}\n{fermante}"` — **les balises nues**. Le
> bloc qu'affiche `rbs add` pour une ancre absente est donc générique : il ne porte pas le
> contenu du fragment, contrairement au remède de `rbs doctor`. N'exiger de lui que la
> présence des balises.

```rust
}
```

- [ ] **Step 2: Écrire le test du projet sans l'ancre**

Le cas qui décide de l'expérience de tout utilisateur existant :

```rust
/// Un projet engendré avant que l'ancre n'existe : la commande n'écrit **rien** et montre
/// le bloc. C'est la convention du dépôt, et c'est le parc entier qui la reçoit.
#[test]
fn without_the_anchor_nothing_is_written_and_the_block_is_shown() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());
    common::commiter(&racine, "init");

    // `add auth` d'abord : c'est lui qui pose le fichier porteur.
    Command::cargo_bin("rbs")
        .expect("le binaire est construit")
        .args(["add", "auth"])
        .current_dir(&racine)
        .assert()
        .success();

    let chemin = racine.join("src/auth/mod.rs");
    let ampute = fs::read_to_string(&chemin)
        .expect("le module se lit")
        .lines()
        .filter(|ligne| !ligne.trim().starts_with("// <rbs:auth_impl"))
        .filter(|ligne| !ligne.trim().starts_with("// </rbs:auth_impl"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&chemin, format!("{ampute}\n")).expect("le module se réécrit");
    common::commiter(&racine, "sans l'ancre");

    let avant = common::empreinte(&racine);

    let sortie = Command::cargo_bin("rbs")
        .expect("le binaire est construit")
        .args(["add", "api-keys"])
        .current_dir(&racine)
        .assert()
        .failure();
    let rendu = String::from_utf8_lossy(&sortie.get_output().stderr).into_owned();

    assert!(rendu.contains("<rbs:auth_impl>"), "{rendu}");
    // `Anchor::block()` ne rend que les deux balises : le bloc affiché est générique, il
    // ne porte pas le contenu du fragment. C'est le remède de `doctor` qui, lui, le porte.
    common::assert_intact(&avant, &racine, "une ancre absente n'autorise aucune écriture");
}
```

- [ ] **Step 3: Écrire le test Docker**

Sur le modèle de `the_tests_shipped_with_the_fragment_run_against_a_real_database` : démarrer
PostgreSQL, engendrer le projet, `rbs add api-keys`, migrer, puis exiger **nommément** que
chacun des tests livrés ait tourné sous `-- --ignored`.

**Filtrer la passe sur `modules::api_keys::`**, et le dire en commentaire. Mesuré à
l'exécution : le projet engendré porte aussi `mail`, entraîné par `auth`, et son test
`a_templated_message_goes_out_to_the_smtp_server` réclame un serveur SMTP — il échoue en
`Connection refused` si la suite ne le monte pas. Le monter dupliquerait
`integration_mail`, qui démarre déjà un Mailpit pour lui seul. Sans ce filtre, la suite
échoue sur un voisin alors que les onze tests du fragment passent. Le filtre permet de
continuer d'exiger le **succès** de la commande, ce qui est plus fort que de l'ignorer ; la
liste nommée reste la garde contre une passe vide. Un `cargo test -- --ignored` sort en
0 même quand il ne filtre aucun test : sans la liste nommée, un fragment qui cesserait de
livrer ses tests laisserait la suite au vert.

```rust
const TESTS_SOUS_CONTENEUR: [(&str, &str); 11] = [
    ("accept", "a_valid_key_identifies_its_bearer"),
    ("accept", "a_key_of_a_demoted_owner_no_longer_administers"),
    ("accept", "a_user_cannot_mint_an_admin_key"),
    ("accept", "a_revoked_key_is_refused_like_an_unknown_one"),
    ("accept", "an_expired_key_is_refused_like_an_unknown_one"),
    ("accept", "three_close_calls_write_the_usage_trace_once"),
    ("accept", "accepting_a_key_leaves_the_verification_date_in_the_extensions"),
    ("routes", "the_key_is_returned_once_and_never_by_the_list"),
    ("routes", "the_creation_response_is_never_cached"),
    ("routes", "revoking_a_key_of_another_account_answers_404"),
    ("routes", "a_key_opens_a_route_that_identity_guards"),
];
```

- [ ] **Step 4: Lancer**

```bash
cargo test -p rbs-cli --test integration_api_keys 2>&1 | tail -20
cargo test --no-fail-fast -p rbs-cli --test integration_api_keys -- --ignored > "$SCRATCH/api-keys-docker.txt" 2>&1; echo "exit: $?"; tail -30 "$SCRATCH/api-keys-docker.txt"
```

Rediriger vers le scratchpad : une suite longue voit ses chiffres rognés sinon.

- [ ] **Step 5: Commit**

Sujet : `test(cli): éprouve les clés d'API sur un projet compilé et une vraie base`

---

### Task 12: `event-hub` porte le fragment

**Files:**
- Modify: **les cinq** projets d'`examples/` (régénérés). `event-hub` gagne le fragment ;
  `blog-auth` suit les deux gabarits `auth` retouchés ; et `hello-crud`, `file-drop` et
  `newsletter-queue`, qui n'ont pourtant pas d'`auth`, dérivent eux aussi — voir ci-dessous
- Modify: `examples/README.md`, `examples/README.fr.md` (la recette d'`event-hub`)

**Interfaces:**
- Consomme : tout le fragment.
- Produit : la compilation du fragment en CI. Sans un exemple porteur, ce code n'est compilé nulle part.

- [ ] **Step 1: Régénérer les deux exemples**

**Les cinq exemples ont dérivé, et pas seulement les deux qui portent `auth`.** Mesuré
après la tâche 4 : `integration_examples` rend 17 passés / 5 échoués, et les cinq échecs
nomment `AGENTS.md` ligne 89. La raison n'est pas le gabarit `auth` : `AGENTS.md` est
engendré dans **chaque** projet et **y énumère les ancres**. Le registre passant à
dix-sept, cette liste gagne `<rbs:auth_impl>` partout, `auth` ou non.

Les deux projets sous `auth` cumulent cette ligne avec les vraies retouches de gabarit :
`src/auth/guard.rs:76` (la documentation d'`Accepted`, reformulée avec `pub(crate)`) et
`src/auth/mod.rs:41` (les deux balises de l'ancre).

**Par diff entre deux générations, jamais par écrasement** : régénérer à côté, comparer,
reporter les seules lignes qui doivent bouger.

```bash
# la recette exacte de chaque exemple est dans examples/README.md
cd "$SCRATCH" && rm -rf regen && mkdir regen && cd regen
# … rejouer la recette de blog-auth, puis :
diff -ru "$SCRATCH/regen/blog-auth/src/auth" /Users/yacoubakone/dev/rs/examples/blog-auth/src/auth
```

Attendu, et rien d'autre : la ligne 89 d'`AGENTS.md` dans les cinq projets, plus — dans
`blog-auth` et `event-hub` seuls — les deux balises `<rbs:auth_impl>` de `src/auth/mod.rs`
et la documentation d'`Accepted` dans `src/auth/guard.rs`. Toute ligne au-delà signale un
gabarit touché par erreur.

Pour `event-hub`, ajouter `api-keys` à la boucle de la recette, **après** `webhooks` :
l'ordre d'installation décide de l'ordre des insertions dans les ancres, et donc des octets
que `integration_examples` compare.

```bash
for f in webhooks scheduler audit cors docker ci api-keys; do
  git add -A && git commit -q -m "before $f"
  cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- add "$f"
done
```

- [ ] **Step 2: Compiler, avant toute passe lente**

```bash
cargo check --manifest-path examples/event-hub/Cargo.toml; echo "exit: $?"
```

C'est ici que les fautes du fragment se voient — pas dans une suite Docker de dix minutes.

- [ ] **Step 3: Le test de non-dérive est l'oracle**

```bash
cargo test --no-fail-fast -p rbs-cli --test integration_examples 2>&1 | tail -20
```

Attendu : 22 passés. Si un blanc diffère, c'est un `-%}` qui a mangé une indentation.

- [ ] **Step 4: Mettre les deux recettes à jour, puis commit**

Sujet : `build(examples): installe les clés d'API dans event-hub`

---

### Task 13: Le guide, dans les deux langues

**Files:**
- Create: `docs/docs/guides/api-keys.md` (`sidebar_position: 11.8`), `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/api-keys.md`

Sur le modèle de `guides/webhooks.md`. Doivent y figurer, parce que la spec les a tranchés
et qu'aucun autre document ne les portera :

- **une clé vaut `Identity` partout** — donc `/auth/change-password` et `/auth/sessions`
  aussi. C'est le prix de la propriété qui fait la valeur du fragment, et il se dit ;
- le plafond, et ce qu'il achète : la clé en lecture seule d'un administrateur ;
- `DELETE /auth/sessions` **ne touche pas aux clés**, et pourquoi : un humain qui se
  déconnecte partout ne veut pas arrêter la CI ;
- la clé rendue une seule fois ;
- la limite du client TypeScript : `rbs generate client` marque l'opération protégée mais
  n'émet qu'un porteur `Bearer` — il ne saura pas présenter une clé ;
- un bloc `{/* rbs:transcript cmd="rbs add api-keys" setup="…" dans="demo" extrait="oui" */}`,
  capturé en exécutant réellement la commande.

- [ ] **Step 1: Écrire les deux pages** — [ ] **Step 2: `npm run build`** — [ ] **Step 3: `npm run parite`** — [ ] **Step 4: Commit**

Sujet : `docs(guides): documente les clés d'API dans les deux langues`

Dans un worktree neuf, `npm ci` sous `docs/` avant `npm run build`.

---

### Task 14: Treize devient quatorze

**Files:**
- Modify: `docs/docs/cli/add.md:8,21,44,344,478`, `docs/docs/cli/new.md:278,348`, `docs/docs/cli/remove.md:47,184`
- Modify: `docs/docs/cli/completions.md` — la page fige un **script de complétion bash**
  dont la ligne `rbs__subcmd__add` énumère les fragments séparés par des espaces
- Modify: les **quatre** jumelles sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/`
  (`add.md`, `new.md`, `remove.md`, `completions.md`)

**Huit pages** figent cette liste, relevées par `grep` et non supposées — quatre en
anglais, quatre en français — sous **deux formes** qu'il faut chercher séparément :

- la forme **à espaces**, celle du script de complétion : `audit auth ci cors docker …`
  (`completions.md` et sa jumelle) ;
- la forme **à virgules**, celle des messages d'erreur du CLI : `audit, auth, ci, cors, …`
  (`add.md`, `new.md`, `remove.md` et leurs jumelles).

Trois natures de garde, à ne pas confondre :

- **Gardées par une transcription**, donc **rouges dès la tâche 5** tant qu'elles ne sont
  pas reprises : `new.md:345` (`rbs new site --with graphql --yes`) et `remove.md:181`
  (`rbs remove graphql`), plus leurs jumelles. `integration_docs` rejoue les deux langues.
- **Non gardées**, donc à reprendre à la main sous peine de mentir en silence : **toutes
  celles d'`add.md`** — ce fichier ne porte aucun marqueur `{/* rbs:transcript */}`, dans
  aucune des deux langues. Le bloc `rbs add graphql` de la ligne 344 en fait partie.
- **Gardée et déjà rouge** : `completions.md:85` et sa jumelle. C'est la **seule** page que
  la suite ait signalée après la tâche 5 — `the_marked_transcripts_still_render_what_the_docs_show`
  s'arrête au premier écart, si bien qu'elle ne nomme qu'une page à la fois. **L'inventaire
  ci-dessus fait foi, pas la suite** : la corriger ne fera qu'avancer le test jusqu'à la
  suivante.

Ne pas oublier le titre `## The thirteen features` / `## Les treize features` **et le lien**
qui le vise (`#the-thirteen-features`, `#les-treize-features`), ainsi que la ligne du
tableau décrivant le nouveau fragment.

- [ ] **Step 1: Reprendre les six fichiers** — [ ] **Step 2: `cargo test --no-fail-fast -p rbs-cli --test integration_docs`** — [ ] **Step 3: `npm run build` et `npm run parite`** — [ ] **Step 4: Commit**

Sujet : `docs(cli): compte quatorze fragments installables`

---

### Task 15: Changelog, table des ancres, README des exemples

**Files:**
- Modify: `CHANGELOG.md` (`## [Unreleased]`), `CHANGELOG.fr.md` (`## [Non publié]`)
- Modify: `CLAUDE.md` (le tableau des ancres, seize lignes → dix-sept ; le décompte des optionnelles, cinq → six)
- Modify: `examples/README.md`, `examples/README.fr.md` si la tâche 12 ne l'a pas fait

Le `CLAUDE.md` dit « seize au total » et « Cinq sont optionnelles » : les deux chiffres
changent, et la ligne `// <rbs:auth_impl>` rejoint le tableau avec son fichier
`src/auth/mod.rs`. Un décompte faux dans ce fichier se propage à chaque session.

Les deux changelogs portent **le même nombre d'items** par version : c'est ce que vérifie
la parité.

- [ ] **Step 1: Écrire les quatre fichiers** — [ ] **Step 2: `npm run parite`** — [ ] **Step 3: Commit**

Sujet : `docs: inscrit les clés d'API au changelog et l'ancre au tableau`

---

### Task 16: La passe finale

Aucun fichier à écrire. Tout ce qui suit doit être **lancé**, et sa sortie réelle collée au
rapport.

- [ ] **Step 1: `rustup update`** — la toolchain locale est peut-être en retard sur celle
  que prend `@stable` en CI ; clippy vert en 1.96 ne dit rien de la 1.98.
- [ ] **Step 2: Les bloquants**

```bash
cargo fmt --all --check; echo "exit: $?"
cargo clippy --workspace --all-targets -- -D warnings; echo "exit: $?"
```

- [ ] **Step 3: Les suites rapides**

```bash
cargo test -p rbs-cli --lib 2>&1 | tail -5
cargo test -p rbs-core --all-features 2>&1 | tail -5
cargo test --no-fail-fast -p rbs-cli --test integration_examples 2>&1 | tail -5
cargo test --no-fail-fast -p rbs-cli --test integration_docs 2>&1 | tail -5
```

- [ ] **Step 4: La compatibilité du noyau**

```bash
cargo semver-checks check-release -p rbs-core 2>&1 | tail -20
```

Attendu : aucune rupture majeure. Une méthode à corps par défaut et un schéma de sécurité
sont additifs.

- [ ] **Step 5: Les suites Docker, un job par suite**

`--no-fail-fast` obligatoire, et une suite à la fois : une seule commande dépasse les 600 s
du shell, `auth` seule fait 500 s. Rediriger chaque sortie vers le scratchpad.

```bash
for suite in api_keys auth crud new; do
  cargo test --no-fail-fast -p rbs-cli --test "integration_$suite" -- --ignored \
    > "$SCRATCH/$suite.txt" 2>&1; echo "$suite exit: $?"
done
```

- [ ] **Step 6: La documentation**

```bash
cd docs && npm run parite && npm run build
```

- [ ] **Step 7: Le rapport**

Cocher les cases de ce plan avec, sur chaque ligne, la preuve exécutée. Puis
`superpowers:finishing-a-development-branch`. Une case ne se coche pas parce que le fichier
est écrit.

---

## Couverture de la spec

| Section de la spec | Tâche |
|---|---|
| §1 une clé vaut `Identity` partout | 1, 2 |
| §2 les échappatoires mortes | — (décision, rien à implémenter) |
| §3 la dix-septième ancre | 4 |
| §4 `accept_key`, refusant par défaut | 1, 2 |
| §5 le rôle plafonné | 6 (`service::accept`, `service::create`), 8 |
| §6 `last_used_at` à la minute | 6 (`repository::touch`), 8 |
| §7 la table `api_keys` | 5 |
| §8 les quatre routes | 7, 8 |
| §9 le schéma OpenAPI | 3 |
| §10 les deux retouches à `auth` | 4 |
| « Les fichiers » | 5, 6, 7, 8, 10 |
| « Ce qui se prouve, et où » | 8, 11, 16 |
| Ouvert n° 1 : le rang de version | 15 (`[Unreleased]` = 1.6.0) |
| Ouvert n° 2 : `Preset::Api` | non traité, délibérément — `Preset::Full` se dérive et prend le fragment seul |
