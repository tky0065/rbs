# CRUD fermé par défaut sous `auth` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** quand `auth` figure dans `[package.metadata.rbs].features`, `rbs generate crud` rend neuf routes exigeant un jeton, l'ouverture d'une route redevenant une édition du fichier généré.

**Architecture:** le rendu du contrôleur et des tests bascule sur un booléen `auth` porté par `Feature` et alimenté par les métadonnées du projet ; le garde du fragment `auth` passe d'une égalité à un seuil, sans quoi la garde par défaut `Role::User` refuserait un admin. Aucune ancre, aucune option CLI, aucune modification de `rbs-core`.

**Tech Stack:** Rust · minijinja (délimiteurs alternatifs) · Axum · utoipa · SeaORM · `assert_cmd` + `testcontainers` pour la suite lente.

**Spec:** `docs/superpowers/specs/2026-09-08-crud-securise-par-defaut-design.md`

## Global Constraints

- **Un projet sans `auth` rend exactement ce qu'il rendait**, octet pour octet. `hello-crud`, `file-drop` et `newsletter-queue` doivent repasser `integration_examples` sans qu'une ligne y change.
- **minijinja emploie des délimiteurs alternatifs** : `{@ expression @}` pour une valeur, `{% bloc %}` pour une structure. `{{ }}` n'existe pas dans ces templates.
- **`-%}` mange l'indentation qui suit.** Un blanc perdu n'est vu que par `integration_examples`, qui compare les exemples octet à octet.
- **Ne jamais employer le filtre `default` sur `role`** : `role` vaut `none`, pas `undefined`, et `default` ne le remplacerait pas. L'expression retenue est `{% set role_ecriture = role if role else "User" %}`.
- **Un commentaire explique le *pourquoi*, jamais le *quoi*.** Le code généré ne commente que ses points d'extension.
- **Documentation bilingue** : toute page anglaise modifiée l'est aussi en français, dans le même commit.
- **Commits en Conventional Commits**, sujet en français à l'impératif, sans identifiant de tâche, sans mention d'un plan, d'un backlog ou d'un assistant. Le corps porte le *pourquoi* et un intertitre `Vérifications :` avec les commandes lancées et leur résultat réel.
- **La version en préparation est `1.3.0`** — le workspace la porte (`Cargo.toml:10`), le dernier tag est `v1.2.0`, et `CHANGELOG.md` ouvre déjà une entrée `[1.3.0] — 2026-09-07` non publiée. La fonctionnalité rejoint cette entrée et la note `crates/rbs-cli/notes/1.3.0.md` ; **aucun bump de version n'est à faire**.
- **Sur la suite lente, `--no-fail-fast` est obligatoire** : sans lui l'exécution s'arrête au premier binaire et masque les échecs suivants.
- Branche de travail : `feat/crud-securise-par-defaut`, déjà créée, la spec y est commitée (`deae699`).

---

## Structure des fichiers

| Fichier | Responsabilité après ce plan |
|---|---|
| `crates/rbs-cli/templates/features/auth/guard.rs.jinja` | le trait `RequireRole`, comparant un **seuil** |
| `crates/rbs-cli/templates/features/auth/model.rs.jinja` | l'enum `Role`, ordonné, dont l'ordre de déclaration porte la hiérarchie |
| `crates/rbs-cli/templates/features/auth/tests.rs.jinja` | + un scénario prouvant qu'un admin traverse une garde `User` |
| `crates/rbs-cli/src/generate/feature.rs` | `Feature` porte `auth: bool` et le sérialise |
| `crates/rbs-cli/src/generate/command.rs` | lit `features` et pose `auth` sur la `Feature` |
| `crates/rbs-cli/templates/feature/controller.rs.jinja` | bandeau de tête, garde et OpenAPI sur les neuf handlers |
| `crates/rbs-cli/templates/feature/tests.rs.jinja` | harnais `token()`, en-tête sur les deux constructeurs, test 401 |
| `crates/rbs-cli/src/generate/tests_http.rs` | `creatable` ne dépend plus du rôle ; passe `auth` au contexte |
| `crates/rbs-cli/src/add/mod.rs` | `add auth` nomme les CRUD déjà présents |
| `examples/blog-auth/` | régénéré : contrôleur et tests cessent d'être manuels |
| `crates/rbs-cli/tests/integration_examples.rs` | `the_hand_edits_of_blog_auth_are_in_place` réduit à ce qui reste manuel |
| `docs/`, `CHANGELOG*.md`, `crates/rbs-cli/notes/1.3.0.md` | la politique et les deux points de rupture, en anglais et en français |

L'ordre des tâches est contraignant : la tâche 3 rend un contrôleur qui appelle un garde que la tâche 1 a rendu hiérarchique, et lit un contexte que la tâche 2 a posé.

---

### Tâche 1 : le garde devient un seuil

**Files:**
- Modify: `crates/rbs-cli/templates/features/auth/guard.rs.jinja:24-41`
- Modify: `crates/rbs-cli/templates/features/auth/model.rs.jinja:3-14`
- Modify: `crates/rbs-cli/templates/features/auth/tests.rs.jinja:559-577`
- Test: `crates/rbs-cli/src/add/mod.rs` (module `tests` en fin de fichier)

**Interfaces:**
- Consomme : rien.
- Produit : `RequireRole::require_role(&self, minimum: Role) -> Result<()>` dans le projet généré — un rôle **au moins** égal à `minimum` passe. Les tâches 3 et 6 en dépendent.

- [ ] **Step 1: écrire le test de non-régression sur la template**

Dans `crates/rbs-cli/src/add/mod.rs`, module `tests`, ajouter en tête du module la constante puis le test :

```rust
/// La template du garde, lue telle qu'elle est embarquée.
///
/// Le garde vit dans le projet de l'utilisateur : aucun test Rust de cette crate ne peut
/// l'exécuter. Ce qui se vérifie ici est que la comparaison reste un seuil — une égalité
/// rendue à sa place ferait refuser un admin par la garde que `generate crud` pose par
/// défaut, et seule la suite Docker le dirait.
const GUARD: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/features/auth/guard.rs.jinja"
));

const ROLE_MODEL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/features/auth/model.rs.jinja"
));

#[test]
fn the_role_guard_compares_a_threshold_and_the_enum_is_ordered() {
    assert!(
        GUARD.contains("porte >= minimum"),
        "le garde doit comparer un seuil, non une égalité :\n{GUARD}"
    );
    assert!(
        !GUARD.contains("porte == expected"),
        "l'égalité stricte doit avoir disparu :\n{GUARD}"
    );
    assert!(
        ROLE_MODEL.contains("PartialOrd, Ord"),
        "l'enum Role doit être ordonné pour que le seuil ait un sens :\n{ROLE_MODEL}"
    );
}
```

- [ ] **Step 2: lancer le test pour le voir échouer**

Run: `cargo test -p rbs-cli --lib add::tests::the_role_guard_compares_a_threshold_and_the_enum_is_ordered`
Expected: FAIL — « le garde doit comparer un seuil, non une égalité ».

- [ ] **Step 3: ordonner l'enum `Role`**

Dans `templates/features/auth/model.rs.jinja`, remplacer le `derive` et le `///` de l'enum :

```rust
/// Rôle applicatif, stocké en texte.
///
/// Un rôle de plus s'ajoute ici, sans migration : c'est le fichier que vous ouvrirez
/// pour le faire, et la garde `require_role` le suit.
///
/// **L'ordre de déclaration porte la hiérarchie** : `require_role` compare un seuil, si
/// bien qu'une variante insérée entre deux autres déplace le seuil de toutes les gardes
/// du projet. Un rôle plus étendu s'ajoute donc en fin d'énumération.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum Role {
    #[sea_orm(string_value = "user")]
    User,
    #[sea_orm(string_value = "admin")]
    Admin,
}
```

- [ ] **Step 4: faire du garde un seuil**

Dans `templates/features/auth/guard.rs.jinja`, remplacer le trait et son implémentation (le reste du fichier, `use` compris, ne bouge pas) :

```rust
/// Exige un rôle **au moins** égal à celui donné.
///
/// `Identity` vient du noyau, qui ne connaît le rôle qu'en clair : l'enum `Role` vit ici,
/// dans le projet, et c'est ce trait qui les réunit. Un rôle de plus dans `model.rs` est
/// aussitôt utilisable par cette garde.
///
/// L'appel se fait en tête de handler, après l'extraction de `Identity` — laquelle rejette
/// déjà une requête sans jeton. La garde ne répond donc jamais à qui n'est pas identifié.
///
/// ```ignore
/// pub async fn supprimer(identite: Identity, ...) -> Result<StatusCode> {
///     identite.require_role(Role::Admin)?;
///     ...
/// }
/// ```
pub trait RequireRole {
    /// Rend [`Error::Forbidden`] si l'appelant porte un rôle inférieur à `minimum`.
    fn require_role(&self, minimum: Role) -> Result<()>;
}

impl RequireRole for Identity {
    fn require_role(&self, minimum: Role) -> Result<()> {
        // Un rôle que l'enum ne connaît plus vient d'un jeton signé par une version
        // antérieure du projet : il n'ouvre rien, et ne fait pas tomber le serveur.
        let porte = Role::try_from_value(&self.role).map_err(|_| Error::Forbidden)?;

        // Le seuil, et non l'égalité : un Admin satisfait une exigence User. C'est
        // l'ordre de déclaration de l'enum qui range les variantes.
        if porte >= minimum {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
}
```

Retirer aussi le `#[allow(dead_code)]` et les trois lignes de commentaire qui le précèdent (`guard.rs.jinja:21-24`) : depuis ce plan, tout CRUD généré appelle la garde, et l'exemption vaudrait permission sur du code réellement mort.

- [ ] **Step 5: relancer le test**

Run: `cargo test -p rbs-cli --lib add::tests::the_role_guard_compares_a_threshold_and_the_enum_is_ordered`
Expected: PASS.

- [ ] **Step 6: prouver le seuil dans le projet généré**

Dans `templates/features/auth/tests.rs.jinja`, à côté de `admin_only_route` (ligne 559), ajouter la route et le scénario :

```rust
/// Une route ouverte à tout compte authentifié, montée pour les tests seuls.
///
/// C'est la forme que `rbs generate crud` pose par défaut : le seuil le plus bas, qu'un
/// rôle plus étendu satisfait aussi.
async fn user_or_above_route() -> Router {
    async fn restricted(identite: Identity) -> rbs_core::Result<StatusCode> {
        identite.require_role(Role::User)?;

        Ok(StatusCode::OK)
    }

    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");

    Router::new()
        .route("/ouverte", get(restricted))
        .with_state(AppState::new(db, config).expect("état partagé constructible"))
}

/// Un administrateur traverse une garde posée sur `User`.
///
/// C'est ce que le seuil promet, et ce qu'une égalité stricte refuserait — le cas est
/// exactement celui de toute route générée par `rbs generate crud` sous `auth`.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_satisfies_a_user_requirement() {
    let api = user_or_above_route().await;
    let token = admin_token(&api).await;

    let (status, _) = call(&api, with_token("GET", "/ouverte", &token)).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "un admin doit satisfaire une exigence User"
    );
}
```

Le nom du helper qui ouvre une session d'administrateur se lit dans le fichier, autour de la ligne 597 (« Inscrit un compte, le promeut administrateur, et ouvre une session à ce titre ») : reprendre **son** nom réel plutôt que `admin_token` si celui-ci diffère, et le même `call` que les scénarios voisins.

- [ ] **Step 7: vérifier que le fragment se rend et compile encore**

Run: `cargo test -p rbs-cli --lib && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected: PASS sur les trois. La preuve d'exécution du nouveau scénario viendra de la tâche 8, qui monte un PostgreSQL.

- [ ] **Step 8: commit**

```bash
git add crates/rbs-cli/templates/features/auth crates/rbs-cli/src/add/mod.rs
git commit -m "feat(auth): compare un seuil de rôle plutôt qu'une égalité"
```

Corps du message : dire que la garde par défaut à venir nomme `Role::User` et qu'une égalité y refuserait un administrateur, et que l'ordre de déclaration de l'enum porte désormais une sémantique, documentée dans le fichier que l'on ouvre pour ajouter un rôle.

---

### Tâche 2 : `Feature` sait que le projet porte `auth`

**Files:**
- Modify: `crates/rbs-cli/src/generate/feature.rs:14-58` et `:238-258`
- Modify: `crates/rbs-cli/src/generate/command.rs:281-284`
- Test: `crates/rbs-cli/src/generate/feature.rs` (module `tests`), `crates/rbs-cli/src/generate/command.rs` (module `tests`)

**Interfaces:**
- Consomme : rien de la tâche 1.
- Produit :
  - `Feature { auth: bool, .. }` — champ public au sein de la crate ;
  - `Feature::authenticated(self) -> Self`, qui pose `auth = true` ;
  - la clé `auth` dans le contexte minijinja de **toutes** les templates de feature.

- [ ] **Step 1: écrire le test de sérialisation**

Dans le module `tests` de `crates/rbs-cli/src/generate/feature.rs` :

```rust
#[test]
fn an_authenticated_feature_carries_the_flag_to_the_templates() {
    let fields = fields::parse("title:string").expect("champs valides");
    let feature = Feature::fresh("articles", fields).authenticated();

    let rendered = minijinja::Value::from_serialize(&feature);

    assert_eq!(
        rendered.get_attr("auth").expect("la clé auth doit exister"),
        minijinja::Value::from(true),
        "les templates lisent `auth` pour poser la garde"
    );
}
```

Si le module `tests` de ce fichier n'importe pas encore `fields`, reprendre l'import que les tests voisins utilisent.

- [ ] **Step 2: lancer le test pour le voir échouer**

Run: `cargo test -p rbs-cli --lib generate::feature::tests::an_authenticated_feature_carries_the_flag_to_the_templates`
Expected: FAIL — `no method named authenticated`.

- [ ] **Step 3: poser le champ, le constructeur et la sérialisation**

Dans `feature.rs`, ajouter au `struct Feature`, après `role` :

```rust
    /// Le projet porte le fragment `auth` : les routes générées exigent un jeton.
    ///
    /// Distinct de `role`, qui ne nomme que le rôle exigé des écritures : un projet peut
    /// porter `auth` sans qu'aucun `--role` ait été passé, et c'est le cas courant.
    pub auth: bool,
```

`fresh` l'initialise à `false`, et le builder rejoint ses voisins :

```rust
    /// La même feature, sur un projet portant `auth` : ses routes exigent un jeton.
    pub(crate) fn authenticated(mut self) -> Self {
        self.auth = true;
        self
    }
```

Dans `impl Serialize`, porter le compte de champs de `12` à `13` et ajouter, à côté de `role` :

```rust
        state.serialize_field("auth", &self.auth)?;
```

La tâche 4 ajoutera un second champ, `role_value`, qui portera le compte à `14` : le poser dès maintenant est aussi acceptable, sa raison d'être est décrite au step 3 de cette tâche-là.

- [ ] **Step 4: relancer le test**

Run: `cargo test -p rbs-cli --lib generate::feature::tests::an_authenticated_feature_carries_the_flag_to_the_templates`
Expected: PASS.

- [ ] **Step 5: écrire le test de la commande**

Dans le module `tests` de `crates/rbs-cli/src/generate/command.rs`, à côté de `a_role_guards_the_three_writes_of_the_generated_controller` :

```rust
/// Sans aucun drapeau, la seule présence de `auth` ferme les routes générées.
#[test]
fn a_project_carrying_auth_closes_the_generated_routes_without_any_flag() {
    let (_parent, root) = project_with_auth();

    run(&options(&root, "articles", Some("title:string"), true))
        .expect("la génération doit aboutir");

    let controller = read(&root.join("src/articles/controller.rs"));

    assert_eq!(
        controller.matches("identite: Identity,").count(),
        6,
        "les six routes doivent extraire l'identité :\n{controller}"
    );
    assert_eq!(
        controller.matches("identite.require_role(Role::User)?;").count(),
        6,
        "les six routes doivent porter la garde par défaut :\n{controller}"
    );
}

/// Le même projet sans `auth` : le rendu ne porte rien du garde.
#[test]
fn a_project_without_auth_keeps_its_routes_open() {
    let (_parent, root) = project();

    run(&options(&root, "articles", Some("title:string"), true))
        .expect("la génération doit aboutir");

    let controller = read(&root.join("src/articles/controller.rs"));

    assert!(
        !controller.contains("Identity") && !controller.contains("require_role"),
        "sans `auth`, le contrôleur est inchangé :\n{controller}"
    );
}
```

Le compte de `6` vaut pour un CRUD sans `--with-upload` : `list`, `filter`, `create`, `find`, `update`, `delete`.

- [ ] **Step 6: lancer les deux tests pour les voir échouer**

Run: `cargo test -p rbs-cli --lib generate::command::tests::a_project_carrying_auth`
Expected: FAIL — le contrôleur ne porte encore aucune garde. Le second test passe déjà : c'est le témoin de non-régression, il doit rester vert de bout en bout.

- [ ] **Step 7: alimenter le drapeau depuis les métadonnées**

Dans `command.rs`, `plan_for`, remplacer la construction de la feature (actuellement `let feature = match &options.role { … }`) par :

```rust
    // La présence du fragment suffit : aucun drapeau ne la demande, et c'est le sens du
    // défaut fermé — un projet qui a installé de quoi fermer ne rend pas des routes
    // anonymes au premier `generate crud`.
    let feature = Feature::fresh(&options.name, fields);
    let feature = if metadonnees.features.iter().any(|feature| feature == "auth") {
        feature.authenticated()
    } else {
        feature
    };
    let feature = match &options.role {
        Some(role) => feature.guarded(role),
        None => feature,
    };
```

- [ ] **Step 8: relancer les tests de rendu**

Run: `cargo test -p rbs-cli --lib generate::`
Expected: `a_project_carrying_auth_closes_the_generated_routes_without_any_flag` échoue encore (la template n'est pas écrite — c'est la tâche 3), tous les autres passent. **Ne pas modifier la template ici.**

- [ ] **Step 9: commit**

```bash
git add crates/rbs-cli/src/generate/feature.rs crates/rbs-cli/src/generate/command.rs
git commit -m "feat(generate): porte la présence du fragment auth jusqu'aux templates"
```

Le commit laisse un test rouge, nommé dans son corps comme la tâche suivante le rend vert ; si cela gêne la CI locale, fusionner les tâches 2 et 3 en un seul commit à la fin de la tâche 3.

---

### Tâche 3 : le contrôleur fermé

**Files:**
- Modify: `crates/rbs-cli/templates/feature/controller.rs.jinja` (tout le fichier)
- Test: `crates/rbs-cli/src/generate/controller.rs` (module `tests`)

**Interfaces:**
- Consomme : la clé `auth` du contexte (tâche 2), `require_role` en seuil (tâche 1).
- Produit : un `controller.rs` dont les neuf handlers portent `identite: Identity`, `identite.require_role(Role::X)?`, `security(("bearer" = []))` et les réponses 401 et 403.

- [ ] **Step 1: écrire les tests de rendu**

Dans le module `tests` de `crates/rbs-cli/src/generate/controller.rs`, ajouter le helper puis les tests :

```rust
    fn authenticated(name: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields).authenticated())
            .expect("le controller doit se rendre")
    }

    fn authenticated_and_guarded(name: &str, role: &str) -> String {
        let fields = fields::parse("title:string").expect("champs valides");
        render(&Feature::fresh(name, fields).authenticated().guarded(role))
            .expect("le controller doit se rendre")
    }

    /// Les six routes portent la même garde, lectures comprises.
    #[test]
    fn under_auth_every_route_requires_a_token() {
        let rendered = authenticated("articles");

        assert_eq!(
            rendered.matches("identite: Identity,").count(),
            6,
            "les six routes doivent extraire l'identité :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("identite.require_role(Role::User)?;").count(),
            6,
            "les six routes doivent porter le seuil par défaut :\n{rendered}"
        );
        assert_eq!(
            rendered.matches(r#"security(("bearer" = [])),"#).count(),
            6,
            "les six annotations doivent déclarer le schéma :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("status = 401").count(),
            6,
            "les six annotations doivent documenter le refus sans jeton :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("status = 403").count(),
            6,
            "les six annotations doivent documenter le rôle insuffisant :\n{rendered}"
        );
    }

    /// `--role` ne substitue le nom que sur les écritures.
    #[test]
    fn a_role_raises_the_threshold_of_the_writes_only() {
        let rendered = authenticated_and_guarded("articles", "admin");

        assert_eq!(
            rendered.matches("identite.require_role(Role::Admin)?;").count(),
            3,
            "create, update et delete doivent monter au rôle demandé :\n{rendered}"
        );
        assert_eq!(
            rendered.matches("identite.require_role(Role::User)?;").count(),
            3,
            "list, filter et find restent au seuil par défaut :\n{rendered}"
        );
    }

    /// Le mode d'emploi est écrit une fois, en tête du fichier.
    #[test]
    fn the_way_to_open_a_route_is_documented_once() {
        let rendered = authenticated("articles");

        assert_eq!(
            rendered.matches("retirez le paramètre").count(),
            1,
            "le mode d'emploi ne se répète pas sur chaque handler :\n{rendered}"
        );
        assert!(
            rendered.starts_with("//!"),
            "le bandeau ouvre le fichier :\n{rendered}"
        );
    }

    /// Les trois routes de contenu suivent la même règle.
    #[test]
    fn under_auth_the_content_routes_require_a_token_too() {
        let fields = fields::parse("title:string").expect("champs valides");
        let rendered = render(
            &Feature::fresh("uploads", fields).authenticated().uploading(),
        )
        .expect("le controller doit se rendre");

        assert_eq!(
            rendered.matches("identite.require_role(Role::User)?;").count(),
            9,
            "les neuf routes doivent porter la garde :\n{rendered}"
        );
    }

    /// Témoin : sans `auth`, le rendu ne porte rien du garde.
    #[test]
    fn without_auth_the_controller_carries_nothing_of_the_guard() {
        let rendered = controller("articles");

        assert!(
            !rendered.contains("Identity")
                && !rendered.contains("require_role")
                && !rendered.contains("bearer"),
            "sans `auth`, le rendu est inchangé :\n{rendered}"
        );
    }
```

Les tests existants qui affirment l'ancien régime doivent être **remplacés**, non conservés : `without_a_role_the_controller_carries_nothing_of_the_guard` (`controller.rs:403`), `the_filter_route_stays_open_under_a_role` (`controller.rs:494`) et `without_a_role_the_content_routes_carry_nothing_of_the_guard` (`controller.rs:634`) portent des promesses que la spec renverse — le filtre est désormais gardé, et l'absence de garde tient à `auth`, non à `role`. Les tests qui vérifient les comptes sous `--role` (`controller.rs:358` et `:595`) deviennent les deux tests ci-dessus après lecture ligne à ligne : reprendre leurs assertions utiles plutôt que d'en perdre.

- [ ] **Step 2: lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli --lib generate::controller`
Expected: FAIL sur `under_auth_every_route_requires_a_token` — aucune garde rendue.

- [ ] **Step 3: écrire le bandeau et les variables de rôle**

En tête de `templates/feature/controller.rs.jinja`, **avant** la ligne `{% set dto = … %}` :

```jinja
{% if auth -%}
//! Toutes les routes exigent un jeton : `Identity` rend 401 sans jeton valide, et
//! `require_role` 403 en deçà du rôle nommé. Sur une route à durcir, montez le rôle ;
//! sur une route à ouvrir au public, retirez le paramètre `identite`, l'appel à
//! `require_role`, l'entrée `security` et les réponses 401 et 403 de son annotation.

{% endif -%}
{% set role_lecture = "User" -%}
{% set role_ecriture = role if role else "User" -%}
{% set dto = ["Create" ~ entity, entity ~ "Response", "Update" ~ entity] | sort -%}
```

`role if role else "User"` et non `role | default("User")` : `role` vaut `none`, que `default` ne remplace pas.

- [ ] **Step 4: basculer les conditions de la template**

Dans tout le fichier, remplacer `{%- if role %}` par `{%- if auth %}`, et les six occurrences de `Role::{@ role @}` par `Role::{@ role_ecriture @}` sur les écritures. Puis **ajouter** ce que l'ancien régime ne posait pas :

- l'import : la ligne `use rbs_core::{HasCoreState, Identity, …}` est déjà conditionnée, faire porter la condition par `auth` ;
- `use crate::auth::guard::RequireRole;` et `use crate::auth::model::Role;` de même ;
- sur `list`, `filter`, `find`, `get_content` et `head_content` : le paramètre `identite: Identity,` placé **juste après** `State(state): State<AppState>,` — `Identity` implémente `FromRequestParts` et doit précéder tout extracteur qui consomme le corps —, l'appel `identite.require_role(Role::{@ role_lecture @})?;` en tête de corps suivi d'une ligne vide, `security(("bearer" = [])),` après `operation_id`, et les deux réponses :

```
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json"),
```

Supprimer le commentaire de `filter` qui justifie son absence de garde (`controller.rs.jinja:60-61`, « Le garde de rôle ne s'y applique donc pas, pas plus qu'à `list` ou `find` ») : il devient faux. Le reste du `///` de `filter` — le corps qui porte les conditions — se garde.

Deux gardes de longueur existent dans ce fichier (`signature_find`, les `use` éclatés à 98 colonnes) : la signature de `find` gagne un paramètre sous `auth` et ne tient donc plus jamais sur une ligne dans ce régime. Conditionner la garde plutôt que la supprimer :

```jinja
{%- if auth %}
pub async fn find(
    State(state): State<AppState>,
    identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<{@ entity @}Response>> {
    identite.require_role(Role::{@ role_lecture @})?;

{%- else %}
{%- set signature_find = "pub async fn find(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<" ~ entity ~ "Response>> {" %}
{%- if signature_find | length <= 100 %}
{@ signature_find @}
{%- else %}
pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<{@ entity @}Response>> {
{%- endif %}
{%- endif %}
```

Le `delete` porte la même bascule (`controller.rs.jinja:186-196`) : sa forme courte n'existe plus sous `auth`.

- [ ] **Step 5: relancer les tests de rendu**

Run: `cargo test -p rbs-cli --lib generate::controller`
Expected: PASS sur les cinq tests neufs et sur les témoins sans `auth`.

- [ ] **Step 6: vérifier que le rendu compile réellement**

Le rendu est une chaîne : rien dans les tests ci-dessus ne dit que `rustc` l'accepte. Générer un projet et le compiler :

```bash
cd "$(mktemp -d)" && cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- \
  new probe --yes --core-path ~/dev/rs/crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/probe' --lang fr
cd probe && git add -A && git commit -q -m 'projet neuf'
cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- add auth
cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud posts --fields 'title:string,body:text' --force
cargo clippy --all-targets -- -D warnings && cargo fmt --all --check
```

Expected: aucune erreur, aucun avertissement, aucune divergence de formatage. `cargo fmt --all --check` est la seule chose qui attrape un blanc que `-%}` aurait mangé.

- [ ] **Step 7: commit**

```bash
git add crates/rbs-cli/templates/feature/controller.rs.jinja crates/rbs-cli/src/generate/controller.rs
git commit -m "feat(generate): ferme les routes du CRUD dès que le projet porte auth"
```

---

### Tâche 4 : les tests générés signent leur jeton

**Files:**
- Modify: `crates/rbs-cli/templates/feature/tests.rs.jinja:1-80` et `:383-423`
- Modify: `crates/rbs-cli/src/generate/tests_http.rs:29-40`
- Test: `crates/rbs-cli/src/generate/tests_http.rs` (module `tests`), `crates/rbs-cli/src/generate/command.rs` (module `tests`)

**Interfaces:**
- Consomme : la clé `auth`, le contrôleur fermé.
- Produit : un `tests.rs` qui exerce le cycle complet sous `auth`, et un scénario `an_anonymous_request_returns_401`.

- [ ] **Step 1: écrire les tests de rendu**

Dans le module `tests` de `crates/rbs-cli/src/generate/tests_http.rs` :

```rust
    /// Sous `auth`, le harnais signe son propre jeton et les scénarios qui écrivent
    /// restent rendus : un fichier réduit aux refus perdrait tout ce qu'il éprouvait.
    #[test]
    fn under_auth_the_harness_signs_its_own_token() {
        let fields = fields::parse("title:string").expect("champs valides");
        let rendered = render(&Feature::fresh("articles", fields).authenticated())
            .expect("les tests doivent se rendre");

        assert!(
            rendered.contains("fn token(role: &str) -> String"),
            "le harnais doit savoir signer un jeton :\n{rendered}"
        );
        assert!(
            rendered.contains("rbs_core::jwt::sign"),
            "le jeton se signe par le noyau, sans passer par la base :\n{rendered}"
        );
        assert!(
            rendered.contains("the_full_lifecycle_goes_through_the_api"),
            "le cycle complet doit rester exercé sous auth :\n{rendered}"
        );
        assert!(
            rendered.contains("an_anonymous_request_returns_401"),
            "le refus sans jeton doit être éprouvé :\n{rendered}"
        );
        assert!(
            !rendered.contains("POST /auth/login"),
            "le renoncement doit avoir disparu :\n{rendered}"
        );
    }

    /// Le rôle signé est celui que le contrôleur exige.
    #[test]
    fn the_signed_role_matches_the_one_the_controller_requires() {
        let fields = fields::parse("title:string").expect("champs valides");
        let rendered = render(
            &Feature::fresh("articles", fields).authenticated().guarded("admin"),
        )
        .expect("les tests doivent se rendre");

        assert!(
            rendered.contains(r#"token("admin")"#),
            "les écritures gardées exigent un jeton du rôle demandé :\n{rendered}"
        );
    }
```

Et, dans `command.rs`, **remplacer** `the_generated_tests_stop_writing_once_the_feature_is_guarded` (`command.rs:857` environ) : la promesse s'inverse.

```rust
/// Les tests engendrés exercent le cycle complet malgré la garde : le harnais signe.
#[test]
fn the_generated_tests_keep_writing_under_a_guard() {
    let (_parent, root) = project_with_auth();

    run(&guarded(&root, "articles", "admin")).expect("la génération doit aboutir");

    let tests = read(&root.join("src/articles/tests.rs"));

    assert!(
        tests.contains("the_full_lifecycle_goes_through_the_api"),
        "le cycle complet doit rester exercé :\n{tests}"
    );
    assert!(
        tests.contains("an_anonymous_request_returns_401"),
        "le refus sans jeton doit être éprouvé :\n{tests}"
    );
}
```

- [ ] **Step 2: lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli --lib generate::tests_http`
Expected: FAIL — `fn token` absent.

- [ ] **Step 3: rendre les scénarios de nouveau atteignables**

Dans `tests_http.rs`, `render` : `creatable` ne doit plus dépendre du rôle.

```rust
    let blocking = feature.required_reference();
    // Le rôle n'écarte plus rien : le harnais signe son jeton, et une écriture gardée
    // s'exerce comme les autres. Seule une référence requise reste bloquante — le
    // fichier ne sait pas quelle ligne cible désigner.
    let creatable = blocking.is_none();
```

et le contexte gagne les deux clés :

```rust
            auth => feature.auth,
            // Le rôle que le harnais signe, tel qu'il s'écrit en base.
            signed_role => feature.role_value.clone().unwrap_or_else(|| "user".to_string()),
```

**Le jeton porte le rôle tel qu'il s'écrit en base** — `admin`, `super_admin` — quand `Feature::role` le porte en PascalCase pour la variante Rust. Ne pas reconstruire l'un depuis l'autre : `SuperAdmin.to_lowercase()` rend `superadmin`, que `Role::try_from_value` refuse, et la garde répondrait 403 à un jeton pourtant signé — un échec qui n'apparaîtrait que sous Docker. `guarded` conserve donc les deux formes ; dans `feature.rs` (tâche 2), ajouter le champ à côté de `role` :

```rust
    /// Le même rôle, tel qu'il est saisi et stocké en base : `super_admin`.
    ///
    /// La variante Rust ne suffit pas à le reconstituer — `SuperAdmin` en minuscules
    /// perd le tiret bas —, et c'est cette forme-ci que porte le jeton.
    pub role_value: Option<String>,
```

`fresh` l'initialise à `None`, `guarded` pose `self.role_value = Some(role.to_string())` avant de dériver la variante, et `Serialize` l'écrit sous `role_value` (le compte de champs passe alors à `14`).

Mettre également à jour le `///` de `render` (`tests_http.rs:19-28`), dont le second paragraphe décrit le renoncement supprimé.

- [ ] **Step 4: écrire le harnais dans la template**

Dans `templates/feature/tests.rs.jinja`, supprimer le bloc `{% elif role -%}` du bandeau de tête (lignes 7-10). Ajouter, sous `auth`, l'import et le harnais, après la fonction `call` :

```jinja
{%- if auth %}

/// Signe un jeton portant `role`, sans passer par la base.
///
/// L'extracteur `Identity` ne vérifie que la signature : le compte n'a pas à exister
/// pour qu'elle tienne, et le fichier n'a donc ni compte à créer ni ligne à nettoyer.
fn token(role: &str) -> String {
    let config = rbs_core::Config::load().expect("configuration lisible");
    let maintenant = chrono::Utc::now().timestamp();

    let claims = rbs_core::jwt::Claims {
        sub: Uuid::new_v4().to_string(),
        role: role.to_string(),
        exp: maintenant + 300,
        iat: maintenant,
        jti: Uuid::new_v4().to_string(),
    };

    rbs_core::jwt::sign(&claims, &config.auth.secret).expect("jeton signable")
}
{%- endif %}
```

et faire porter l'en-tête par les deux constructeurs, qui sont les seuls points de passage des requêtes :

```jinja
fn without_body(method: &str, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
{%- if auth %}
        .header("authorization", format!("Bearer {}", token("{@ signed_role @}")))
{%- endif %}
        .body(Body::empty())
        .expect("requête bien formée")
}
```

— et de même dans `request(method, path, body)`, après la ligne `.header("content-type", …)`.

Vérifier que `chrono` est bien une dépendance du projet généré (le squelette la porte pour SeaORM) ; sinon, prendre l'horloge par `std::time::SystemTime` :

```rust
    let maintenant = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("horloge postérieure à 1970")
        .as_secs() as i64;
```

C'est la forme à préférer si le doute subsiste : elle ne dépend d'aucune crate.

- [ ] **Step 5: remplacer le scénario terminal**

En fin de `tests.rs.jinja`, le bloc `{%- if role %} … {%- else %} … {%- endif %}` (lignes 383-423) devient : le test 400 est rendu **dans les deux régimes** — signé sous `auth`, puisque `Identity` s'exécute avant que le corps ne soit lu — et le test 401 s'ajoute sous `auth` :

```jinja
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unreadable_body_returns_400() {
    let api = application().await;
    let truncated = Request::builder()
        .method("POST")
        .uri("/{@ module @}")
        .header("content-type", "application/json")
{%- if auth %}
        .header("authorization", format!("Bearer {}", token("{@ signed_role @}")))
{%- endif %}
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (status, body) = call(&api, truncated).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}
{%- if auth %}

/// Sans jeton, la requête est refusée avant même que le corps ne soit lu.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_anonymous_request_returns_401() {
    let api = application().await;
    let anonyme = Request::builder()
        .method("POST")
        .uri("/{@ module @}")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("requête bien formée");

    let (status, body) = call(&api, anonyme).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401, "{body}");
}
{%- endif %}
```

- [ ] **Step 6: relancer les tests de rendu**

Run: `cargo test -p rbs-cli --lib generate::`
Expected: PASS partout, y compris `a_project_carrying_auth_closes_the_generated_routes_without_any_flag` laissé rouge par la tâche 2.

- [ ] **Step 7: compiler le projet témoin**

Reprendre le projet de la tâche 3, step 6 (ou le régénérer), puis :

```bash
cargo clippy --all-targets -- -D warnings && cargo fmt --all --check
```

Expected: propre. Un `token` non appelé — cas d'une feature à référence requise — ferait échouer `-D warnings` : si le cas se présente, conditionner le harnais à `creatable` en plus de `auth`.

- [ ] **Step 8: commit**

```bash
git add crates/rbs-cli/templates/feature/tests.rs.jinja crates/rbs-cli/src/generate/tests_http.rs crates/rbs-cli/src/generate/command.rs
git commit -m "feat(generate): fait signer aux tests engendrés le jeton qu'exigent leurs routes"
```

---

### Tâche 5 : `add auth` nomme les CRUD déjà ouverts

**Files:**
- Modify: `crates/rbs-cli/src/add/mod.rs` (la fin de `plan_for` ou la structure `Planned`, selon ce que la lecture montre)
- Modify: `crates/rbs-cli/src/lib.rs` (l'affichage de fin de `add`, autour de la ligne 329)
- Test: `crates/rbs-cli/src/add/mod.rs` (module `tests`)

**Interfaces:**
- Consomme : `metadata::Metadata.features`.
- Produit : une ligne de sortie nommant chaque module CRUD présent au moment où `auth` s'installe.

- [ ] **Step 1: lire d'abord**

Lire `crates/rbs-cli/src/add/mod.rs:162` (`plan_for`) et le bras `Commands::Add` de `lib.rs` pour voir **où** la sortie de fin est produite et par quelle structure elle transite. Le message se pose là où les autres remèdes se posent (`Planned::remedy`, `add/mod.rs:138`), et non par un `println!` isolé.

- [ ] **Step 2: écrire le test**

```rust
/// Un CRUD généré avant `auth` reste ouvert : le CLI ne réécrit pas un fichier existant,
/// mais se taire ferait croire l'API fermée.
#[test]
fn installing_auth_names_the_cruds_that_stay_open() {
    let (_parent, root) = Project::new().features(&["health", "posts"]).create();

    let planned = plan_for(&options(&root, "auth")).expect("l'installation doit se planifier");

    let message = planned.remedy().expect("un avertissement doit être rendu");

    assert!(
        message.contains("posts"),
        "le module déjà présent doit être nommé : {message}"
    );
    assert!(
        !message.contains("health"),
        "un fragment n'est pas un CRUD : {message}"
    );
}
```

Adapter `options(&root, "auth")` au constructeur d'options réel de `add` — le module `tests` du fichier en porte un.

- [ ] **Step 3: lancer le test pour le voir échouer**

Run: `cargo test -p rbs-cli --lib add::tests::installing_auth_names_the_cruds_that_stay_open`
Expected: FAIL.

- [ ] **Step 4: implémenter**

La liste des CRUD est celle des `features` du manifeste **privée des fragments connus** — ceux que `templates/features/` porte, énumérés par le catalogue que `add` consulte déjà — et de `health`, que `rbs new` pose. Le message, affiché seulement quand `auth` s'installe et que la liste n'est pas vide :

```rust
format!(
    "les features déjà générées restent publiques : {liste}. Le CLI ne réécrit pas un \
     fichier existant — sur chaque handler à fermer, ajoutez le paramètre \
     `identite: Identity`, l'appel `identite.require_role(Role::User)?`, l'entrée \
     `security((\"bearer\" = []))` et les réponses 401 et 403 de son annotation."
)
```

- [ ] **Step 5: relancer le test**

Run: `cargo test -p rbs-cli --lib add::`
Expected: PASS.

- [ ] **Step 6: vérifier le message tel qu'il s'affiche**

Sur le projet témoin, générer un CRUD **avant** d'installer `auth` et lire la sortie réelle. Attention : `crates/rbs-cli/tests/integration_docs.rs` vérifie les transcriptions du site — un message du CLI qui change peut rendre `docs/` rouge. Lancer `cargo test -p rbs-cli --test integration_docs` avant de conclure.

- [ ] **Step 7: commit**

```bash
git add crates/rbs-cli/src/add crates/rbs-cli/src/lib.rs
git commit -m "feat(add): signale les features restées publiques quand auth arrive après elles"
```

---

### Tâche 6 : `examples/blog-auth` régénéré

**Files:**
- Modify: `examples/blog-auth/src/posts/controller.rs`, `examples/blog-auth/src/posts/tests.rs`, `examples/blog-auth/src/auth/guard.rs`, `examples/blog-auth/src/auth/model.rs`
- Modify: `crates/rbs-cli/tests/integration_examples.rs:598` (`the_hand_edits_of_blog_auth_are_in_place`) et son inventaire de fichiers (`:31`, `:67`)
- Modify: `examples/README.md`, `examples/README.fr.md`

**Interfaces:**
- Consomme : tout ce qui précède.
- Produit : un exemple dont le contrôleur n'est plus édité à la main.

- [ ] **Step 1: régénérer par diff, jamais par écrasement**

Générer `blog-auth` dans un répertoire temporaire avec les commandes exactes d'`examples/README.md:44-52`, puis **comparer** au versionné (`diff -ru`) et reporter les différences dues au générateur. Un écrasement perdrait les éditions manuelles que l'exemple garde encore.

- [ ] **Step 2: reprendre le contrôleur tel qu'il se génère**

`src/posts/controller.rs` devient le rendu brut : les cinq routes portent la garde, `list` et `find` comprises. La commande de génération de l'exemple **ne prend toujours pas `--role`** — c'est le sens de la démonstration —, si bien que les écritures nomment `Role::User` là où elles nommaient `Role::Admin`. Si l'exemple doit rester « seul un admin écrit », c'est `--role admin` qu'il faut ajouter à la commande du README, dans les deux langues, et non éditer le fichier.

**Décision à prendre à cette étape et à écrire dans le commit** : ajouter `--role admin` à la commande de l'exemple, ce qui conserve la promesse de son README (« only an admin can write ») et démontre les deux régimes à la fois.

- [ ] **Step 3: reprendre les tests tels qu'ils se génèrent**

`src/posts/tests.rs` perd son harnais manuel (inscription, promotion en base, connexion) au profit du `token()` généré. Les trois tests que l'exemple ajoutait à la main — 401 sans jeton, 403 pour un `user`, lecture par un anonyme — sont à reconsidérer un par un : le premier est désormais généré, le deuxième garde sa valeur (il éprouve le seuil), le troisième devient faux puisque la lecture est fermée. Ne garder que ce qui prouve encore quelque chose, et le déclarer dans le README.

- [ ] **Step 4: mettre l'inventaire des éditions à jour**

`the_hand_edits_of_blog_auth_are_in_place` (`integration_examples.rs:598`) doit tomber à ce qui reste réellement manuel. Il lit aujourd'hui `src/auth/guard.rs` pour y vérifier l'absence du `#[allow(dead_code)]` : la tâche 1 l'ayant retiré de la template, cette assertion n'atteste plus une édition mais le rendu — la déplacer ou la supprimer.

- [ ] **Step 5: réécrire les deux README**

`examples/README.md:118-119` (« `blog-auth` carries three more … no command wires a guard onto a route you generated ») est la phrase que ce plan rend fausse. La remplacer dans les deux langues par ce qui est vrai après coup : le contrôleur est généré, l'exemple ne garde que les éditions qu'aucune commande ne produit.

- [ ] **Step 6: prouver la non-dérive**

Run: `cargo test -p rbs-cli --test integration_examples -- --ignored --no-fail-fast`
Expected: les quatre exemples se régénèrent à l'identique. **Les trois exemples sans `auth` ne doivent porter aucune modification** — si l'un d'eux bouge, la condition de bascule fuit.

- [ ] **Step 7: commit**

```bash
git add examples crates/rbs-cli/tests/integration_examples.rs
git commit -m "test(examples): régénère blog-auth dont le garde n'est plus posé à la main"
```

---

### Tâche 7 : la documentation, le changelog et la note de version

**Files:**
- Modify: les pages `generate crud` et `add auth` du site, dans `docs/`, en anglais **et** en français
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md` (entrée `[1.3.0]`)
- Modify: `crates/rbs-cli/notes/1.3.0.md`

- [ ] **Step 1: localiser les pages**

```bash
grep -rln "generate crud" docs/ | sort
grep -rln "require_role\|--role" docs/ | sort
```

Traiter chaque page trouvée dans les deux langues. `docs/` porte sa propre toolchain Node : ne rien y lancer d'autre que ce que le dépôt prévoit.

- [ ] **Step 2: écrire la politique**

Ce que la page `generate crud` doit désormais dire : sur un projet portant `auth`, les routes générées exigent un jeton et l'ouverture d'une route est une édition — avec les quatre éléments à retirer. Que `--role` monte le seuil des écritures. Que le seuil est hiérarchique et que l'ordre de déclaration de `Role` le porte.

- [ ] **Step 3: le changelog, dans les deux langues**

Sous `## [1.3.0]`, section `Changed`, une entrée disant le renversement du défaut et, en `Fixed` ou dans la même entrée, le passage du garde au seuil. Le ton du fichier est celui des entrées voisines : longues, factuelles, écrites pour qui installe.

- [ ] **Step 4: la note de version**

`crates/rbs-cli/notes/1.3.0.md` reçoit les deux points que la spec impose :

1. sur un projet portant `auth`, `generate crud` rend désormais des routes fermées ;
2. `require_role` compare un seuil dans les projets neufs et une égalité dans les anciens — avec les quatre lignes à remplacer dans `src/auth/guard.rs` pour migrer.

- [ ] **Step 5: vérifier les transcriptions**

Run: `cargo test -p rbs-cli --test integration_docs`
Expected: PASS — c'est le seul test qui dit qu'une sortie du CLI citée par le site a divergé.

- [ ] **Step 6: commit**

```bash
git add docs CHANGELOG.md CHANGELOG.fr.md crates/rbs-cli/notes/1.3.0.md
git commit -m "docs: décrit la fermeture par défaut des routes générées sous auth"
```

---

### Tâche 8 : la passe lente

**Files:** aucun, sauf correction.

- [ ] **Step 1: la suite rapide**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: PASS. Consigner les comptes réels de tests.

- [ ] **Step 2: `cargo check` sur l'exemple porteur avant la passe lente**

Run: `cargo check --manifest-path examples/blog-auth/Cargo.toml --all-targets`
Expected: PASS. Le code des fragments n'est compilé nulle part ailleurs avant Docker ; cette vérification évite de découvrir une erreur triviale au bout d'une heure.

- [ ] **Step 3: la suite Docker**

Run: `cargo test --workspace -- --ignored --no-fail-fast 2>&1 | tee "$SCRATCHPAD/passe-lente.txt"`

`--no-fail-fast` n'est pas facultatif : sans lui l'exécution s'arrête au premier binaire et masque les échecs suivants. Rediriger vers un fichier : la sortie d'une suite longue est rognée à l'affichage.

Expected: 0 échec. Les tests qui portent cette fonctionnalité et qu'il faut voir passer nommément :
- `integration_auth::a_guarded_route_rejects_an_authenticated_user` ;
- `integration_auth::an_admin_satisfies_a_user_requirement` (tâche 1) ;
- `integration_auth::the_auth_tests_of_the_generated_project_pass` ;
- `integration_crud` dans son ensemble ;
- `integration_examples` et `the_hand_edits_of_blog_auth_are_in_place`.

- [ ] **Step 4: les tests des exemples**

Ils ne tournent dans aucune suite : les prouver exige un PostgreSQL monté à la main et `--include-ignored`. Sur `examples/blog-auth`, migrations appliquées :

```bash
cargo test --manifest-path examples/blog-auth/Cargo.toml -- --include-ignored --no-fail-fast
```

Expected: PASS — c'est la seule preuve que le harnais `token()` fonctionne contre un vrai serveur.

- [ ] **Step 5: consigner**

Reporter les comptes réels dans le corps du dernier commit, sous `Vérifications :`. Un critère non prouvé se déclare `PARTIEL` ou `BLOQUÉ : [raison]` — jamais coché sur la foi d'un fichier écrit.

---

## Auto-revue

**Couverture de la spec :**

| Section de la spec | Tâche |
|---|---|
| Bascule sur la présence de `auth` | 2 |
| Une seule forme pour les neuf handlers | 3 |
| Aucune option CLI nouvelle | — (rien à faire, vérifié par l'absence de modification de `cli.rs`) |
| `--role` substitue le nom sur les écritures | 3 (`a_role_raises_the_threshold_of_the_writes_only`) |
| Garde hiérarchique, ordre de déclaration documenté | 1 |
| Contrôleur rendu, bandeau une fois | 3 |
| Tests générés, harnais qui signe | 4 |
| `auth` installé après coup | 5 |
| Rendu inchangé sans `auth` | 2 et 3 (témoins), 6 (non-dérive des trois exemples) |
| `rbs-core` intouché | — (aucune tâche ne le modifie) |
| Versions et note | 7 |
| Critères d'acceptation | 8 |

**Points laissés à l'exécutant, et pourquoi :** le nom réel du helper d'authentification dans `auth/tests.rs.jinja` (tâche 1, step 6), la structure exacte par laquelle la sortie de `add` transite (tâche 5, step 1), et le sort de `--role admin` sur `blog-auth` (tâche 6, step 2) demandent une lecture du fichier au moment de l'écrire. Chacun est signalé sur place avec ce qu'il faut lire pour trancher.

**Effet de bord à surveiller :** `crates/rbs-cli/src/client/document.rs:190` marque `secured` toute opération portant un `security` non vide, et le client TypeScript généré change donc de rendu pour un projet sous `auth`. Aucune tâche ne le modifie, mais `cargo test -p rbs-cli --test integration_client` doit passer en tâche 8 — si un test y compte les opérations protégées, c'est là qu'il parlera.
