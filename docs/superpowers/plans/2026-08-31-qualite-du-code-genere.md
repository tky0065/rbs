# Qualité du code généré — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Corriger six défauts du code que `rbs` engendre — bornes de mot de passe, lignes
orphelines des tests CRUD, statuts non couverts, contrat OpenAPI incomplet, tests `auth`
non marqués `#[ignore]`, et le verbe de la mise à jour, qui annonce un remplacement là où
le service fusionne.

**Architecture:** Tout se joue dans `crates/rbs-cli/templates/`, plus le module de rendu
`crates/rbs-cli/src/generate/tests_http.rs` pour les deux nouveaux drapeaux de contexte.
Chaque changement de template est prouvé par un test de rendu dans le module `#[cfg(test)]`
du générateur correspondant, puis répercuté sur les quatre projets d'`examples/` par diff
entre deux générations, le test de non-dérive `integration_examples` faisant l'oracle.

**Tech Stack:** Rust, minijinja (délimiteurs alternatifs `{@ @}` / `{% %}`), utoipa,
validator, SeaORM, assert_cmd + testcontainers.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` (autorité d'architecture) ;
énoncés des tâches dans `IMPROVE.md`, section P2, entrées 34, 45, 46, 47, 48, 49.

## Global Constraints

- Délimiteurs minijinja alternatifs : expressions `{@ … @}`, blocs `{% … %}`.
- Le code généré ne commente que ses points d'extension ; un commentaire dit le *pourquoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`
  sont bloquants ; le code **généré** doit lui aussi compiler sous `-D warnings` (donc :
  aucune aide ni import inutilisé dans un rendu réduit).
- `IMPROVE.md` ne se coche pas et ne se modifie pas.
- La mise à jour générée passe de `PUT` à `PATCH`, sans alias `put` conservé. Les projets
  déjà engendrés ne sont pas touchés : seule la génération future change.
- Documentation bilingue : toute page anglaise modifiée l'est aussi en français, même commit.
- Commits : Conventional Commits, sujet français à l'impératif, corps avec `Vérifications :`,
  sans `Co-Authored-By` ni `Claude-Session`, sans identifiant de tâche.
- Régénération d'`examples/` **par diff entre deux générations**, jamais par écrasement.

---

### Task 1: Bornes de mot de passe sur les DTO d'`auth`

**Files:**
- Modify: `crates/rbs-cli/templates/features/auth/dto.rs.jinja:10-19`
- Modify: `docs/docs/guides/auth.md:210`
- Modify: `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/auth.md:215`
- Test: `crates/rbs-cli/src/add/mod.rs` (module `#[cfg(test)]` du fragment)

**Interfaces:**
- Consumes: rien.
- Produces: le DTO `auth` porte `#[validate(length(min = 12, max = 128))]` sur
  `RegisterRequest.password` **et** `LoginRequest.password`.

- [ ] **Step 1: Repérer où se testent les rendus du fragment `auth`**

Run: `grep -n "dto.rs" crates/rbs-cli/src/add/mod.rs`
S'il n'existe aucun test de rendu du DTO, écrire le test dans le module `#[cfg(test)]` de
`crates/rbs-cli/src/add/mod.rs`, sur le modèle des tests voisins qui lisent une template
du fragment.

- [ ] **Step 2: Écrire le test qui échoue**

```rust
/// Un mot de passe sans borne haute fait hacher en Argon2 un corps de plusieurs
/// mégaoctets : la borne est ce qui sépare une API d'un amplificateur.
#[test]
fn the_two_password_fields_carry_a_lower_and_an_upper_bound() {
    let dto = fragment_template("auth", "dto.rs.jinja");

    assert_eq!(
        dto.matches("#[validate(length(min = 12, max = 128))]").count(),
        2,
        "les deux mots de passe doivent être bornés :\n{dto}"
    );
    assert!(
        !dto.contains("length(min = 8)"),
        "la borne à 8 sans maximum subsiste :\n{dto}"
    );
}
```

- [ ] **Step 3: Lancer le test et le voir échouer**

Run: `cargo test -p rbs-cli --lib the_two_password_fields_carry_a_lower_and_an_upper_bound`
Expected: FAIL — le compte vaut 0.

- [ ] **Step 4: Poser les bornes dans la template**

```jinja
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 12, max = 128))]
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    // La borne vaut autant à la connexion : sans elle, `/auth/login` hache en Argon2 tout
    // ce qu'on lui poste, sans qu'aucun compte n'ait à exister.
    #[validate(length(min = 12, max = 128))]
    pub password: String,
}
```

- [ ] **Step 5: Vérifier que les mots de passe des tests générés tiennent dans les bornes**

Run: `grep -n "PASSWORD\|mot de passe" crates/rbs-cli/templates/features/auth/tests.rs.jinja`
Attendu : `const PASSWORD: &str = "un mot de passe assez long";` (26 caractères) et
`"un tout autre mot de passe"` (26 caractères) — les deux entre 12 et 128, rien à changer.
Si un littéral tombe sous 12, l'allonger.

- [ ] **Step 6: Relancer le test**

Run: `cargo test -p rbs-cli --lib the_two_password_fields_carry_a_lower_and_an_upper_bound`
Expected: PASS

- [ ] **Step 7: Aligner la documentation, dans les deux langues**

`docs/docs/guides/auth.md` : remplacer
`- **password policy** — the DTO validates a minimum length, nothing more;`
par
`- **password policy** — the DTO validates a length between 12 and 128 characters, nothing more;`

`docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/auth.md` : remplacer
`- **la politique de mot de passe** — le DTO valide une longueur minimale, rien de plus ;`
par
`- **la politique de mot de passe** — le DTO valide une longueur de 12 à 128 caractères, rien de plus ;`

- [ ] **Step 8: Commit**

```bash
git add crates/rbs-cli/templates/features/auth/dto.rs.jinja \
        crates/rbs-cli/src/add/mod.rs docs/docs/guides/auth.md \
        docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/auth.md
git commit
```

---

### Task 2: Les tests CRUD ne laissent plus de lignes derrière eux

**Files:**
- Modify: `crates/rbs-cli/templates/feature/tests.rs.jinja` (test
  `two_creations_in_a_row_carry_increasing_ids`)
- Test: `crates/rbs-cli/src/generate/tests_http.rs` (module `#[cfg(test)]`)

**Interfaces:**
- Consumes: les aides `request`, `without_body`, `call` du gabarit, déjà présentes sous
  `{% if creatable %}`.
- Produces: aucune nouvelle variable de contexte.

- [ ] **Step 1: Écrire le test de rendu qui échoue**

Dans le module `#[cfg(test)]` de `crates/rbs-cli/src/generate/tests_http.rs` :

```rust
/// Les tests tournent sur la base de développement, sans transaction : ce qu'ils créent,
/// ils le suppriment, faute de quoi la table enfle de deux lignes par `cargo test`.
#[test]
fn the_uuid_scenario_deletes_what_it_created() {
    let rendered = trials("articles", CHAMPS);

    let scenario = rendered
        .split("async fn two_creations_in_a_row_carry_increasing_ids()")
        .nth(1)
        .expect("le scénario doit être rendu");

    assert_eq!(
        scenario.matches(r#"without_body("DELETE""#).count(),
        2,
        "les deux lignes créées doivent être supprimées :\n{scenario}"
    );
}
```

- [ ] **Step 2: Lancer le test et le voir échouer**

Run: `cargo test -p rbs-cli --lib the_uuid_scenario_deletes_what_it_created`
Expected: FAIL — le compte vaut 0.

- [ ] **Step 3: Nettoyer dans la template**

Remplacer la fin du scénario par :

```jinja
#[tokio::test]
async fn two_creations_in_a_row_carry_increasing_ids() {
    let api = application().await;
    let collection = "/{@ module @}";

    let (status, premier) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {premier}");
    let (status, second) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {second}");

    let lire = |rendered: &Value| {
        Uuid::parse_str(rendered["id"].as_str().expect("identifiant rendu"))
            .expect("identifiant lisible")
    };
    let (premier, second) = (lire(&premier), lire(&second));

    // Les tests jouent sur la base du `.env`, hors transaction : sans ces suppressions la
    // table enfle de deux lignes à chaque exécution.
    for identifiant in [premier, second] {
        let resource = format!("{collection}/{identifiant}");
        let (status, _) = call(&api, without_body("DELETE", &resource)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
    }

    assert_eq!(premier.get_version_num(), 7, "{premier} n'est pas un UUIDv7");
    assert!(second > premier, "{second} ne suit pas {premier}");
}
```

- [ ] **Step 4: Relancer le test de rendu**

Run: `cargo test -p rbs-cli --lib the_uuid_scenario_deletes_what_it_created`
Expected: PASS

- [ ] **Step 5: Ne pas commiter encore**

Le rendu change : `examples/` suivra en Task 5. Enchaîner sur la Task 3.

---

### Task 3: Deux scénarios de plus au gabarit CRUD — 422 et 409

**Files:**
- Modify: `crates/rbs-cli/src/generate/tests_http.rs` (fonction `render`, deux drapeaux)
- Modify: `crates/rbs-cli/templates/feature/tests.rs.jinja`
- Test: `crates/rbs-cli/src/generate/tests_http.rs` (module `#[cfg(test)]`)

**Interfaces:**
- Consumes: `Field::validates_email() -> bool`, `Field::unique: bool`,
  `Field::column_name() -> String` (déjà publics dans `crates/rbs-cli/src/generate/fields.rs`).
- Produces: deux variables de contexte pour la template :
  - `email_field: Option<String>` — nom de colonne du premier champ à contrainte d'e-mail,
    `None` si aucun ou si `creatable` est faux ;
  - `unique_field: bool` — vrai si au moins un champ envoyé porte `unique`.

**Statuts, vérifiés dans le code et non supposés :**
`crates/rbs-core/src/extract.rs` — `ValidatedJson` désérialise puis appelle `validate()` ;
l'échec devient `Error::Validation`. `crates/rbs-core/src/error.rs` — `Error::Validation`
rend `StatusCode::UNPROCESSABLE_ENTITY`, donc **422** (et non 400, réservé au corps
illisible). `crates/rbs-cli/templates/feature/repository.rs.jinja` — `conflict_on_duplicate`
traduit `SqlErr::UniqueConstraintViolation` en `Error::Conflict`, rendu **409**.

- [ ] **Step 1: Écrire les tests de rendu qui échouent**

```rust
/// `ValidatedJson` existe pour ce chemin : un corps lisible mais non conforme rend 422,
/// là où un corps illisible rend 400. Rien ne l'éprouvait.
#[test]
fn an_email_field_earns_a_422_scenario() {
    let rendered = trials("articles", CHAMPS);

    assert!(
        rendered.contains("async fn an_invalid_email_returns_422()"),
        "le scénario 422 est absent :\n{rendered}"
    );
    assert!(
        rendered.contains("StatusCode::UNPROCESSABLE_ENTITY"),
        "le statut attendu doit être 422 :\n{rendered}"
    );
    assert!(
        rendered.contains(r#"body["errors"]["email"]"#),
        "le 422 doit nommer le champ fautif :\n{rendered}"
    );
}

/// Sans champ `unique`, un rejeu ne provoque aucun conflit : le scénario n'aurait rien
/// à prouver et échouerait.
#[test]
fn a_unique_field_earns_a_409_scenario() {
    let rendered = trials("articles", CHAMPS);

    assert!(
        rendered.contains("async fn a_replayed_unique_value_returns_409()"),
        "le scénario 409 est absent :\n{rendered}"
    );
    assert!(
        rendered.contains("StatusCode::CONFLICT"),
        "le statut attendu doit être 409 :\n{rendered}"
    );
}

/// Les deux scénarios sont conditionnés par ce que `--fields` demande.
#[test]
fn a_feature_without_email_or_unique_carries_neither_scenario() {
    let rendered = trials("articles", "title:string,body:text,published:bool");

    assert!(
        !rendered.contains("an_invalid_email_returns_422"),
        "aucun champ ne porte de contrainte d'e-mail :\n{rendered}"
    );
    assert!(
        !rendered.contains("a_replayed_unique_value_returns_409"),
        "aucun champ n'est unique :\n{rendered}"
    );
}

/// Sans création possible, les deux scénarios tombent avec les autres.
#[test]
fn a_required_reference_also_drops_the_422_and_409_scenarios() {
    let rendered = trials("posts", "email:string:unique,author:references:users");

    assert!(!rendered.contains("an_invalid_email_returns_422"), "{rendered}");
    assert!(!rendered.contains("a_replayed_unique_value_returns_409"), "{rendered}");
}
```

- [ ] **Step 2: Lancer les tests et les voir échouer**

Run: `cargo test -p rbs-cli --lib generate::tests_http`
Expected: FAIL sur les deux premiers (chaînes absentes) ; les deux derniers passent déjà.

- [ ] **Step 3: Ajouter les deux drapeaux au contexte**

Dans `render`, après le calcul de `sent` :

```rust
        context! {
            module => feature.module(),
            creatable,
            role => feature.role,
            blocking_reference => blocking.map(|field| field.relation_name()),
            fields => fields,
            compared => names(sent, |champ| !timestamp(champ)),
            timestamped => names(sent, timestamp),
            suffix => sent.iter().any(textual),
            // Les deux scénarios n'ont de sens que si `--fields` les rend atteignables :
            // sans contrainte d'e-mail rien ne rend 422, sans colonne unique rien ne
            // rend 409, et le test échouerait faute de refus à observer.
            email_field => sent.iter().find(|champ| champ.validates_email()).map(Field::column_name),
            unique_field => sent.iter().any(|champ| champ.unique),
        },
```

- [ ] **Step 4: Ajouter les deux scénarios à la template**

Dans `crates/rbs-cli/templates/feature/tests.rs.jinja`, à l'intérieur du bloc
`{% if creatable %}`, juste après `two_creations_in_a_row_carry_increasing_ids` :

```jinja
{% if email_field %}
/// Un corps lisible mais non conforme rend 422, et non le 400 du corps illisible.
///
/// C'est le chemin qu'ouvre `ValidatedJson` : il désérialise, puis valide, et le refus
/// nomme le champ fautif.
#[tokio::test]
async fn an_invalid_email_returns_422() {
    let api = application().await;
    let mut sent = creation();
    sent["{@ email_field @}"] = Value::from("pas-une-adresse");

    let (status, body) = call(&api, request("POST", "/{@ module @}", sent)).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["status"], 422, "{body}");
    assert!(
        body["errors"]["{@ email_field @}"].is_array(),
        "le refus doit nommer le champ fautif : {body}"
    );
}
{% endif %}
{%- if unique_field %}
/// Une valeur déjà prise sur une colonne `unique` est une faute du client, pas une panne.
///
/// Sans la traduction que pose le repository, le doublon remonterait en 500.
#[tokio::test]
async fn a_replayed_unique_value_returns_409() {
    let api = application().await;
    let collection = "/{@ module @}";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");

    let (status, body) = call(&api, request("POST", collection, sent)).await;

    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["status"], 409, "{body}");

    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");
    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}
{% endif %}
```

- [ ] **Step 5: Relancer les tests de rendu**

Run: `cargo test -p rbs-cli --lib generate::tests_http`
Expected: PASS — et corriger `the_four_scenarios_are_declared`, dont le compte de
`#[tokio::test]` change : renommer en `the_scenarios_are_declared` et compter les scénarios
attendus pour `CHAMPS` (qui porte `email:string:unique`), soit six.

- [ ] **Step 6: Vérifier l'absence d'aide inutilisée**

Run: `cargo test -p rbs-cli --lib generate::tests_http`
`a_field_less_feature_carries_no_unused_helper` et
`the_reduced_file_imports_only_what_it_uses` doivent rester verts : les deux scénarios
n'introduisent aucun import neuf (`Value` et `StatusCode` sont déjà là).

- [ ] **Step 7: Commit des tâches 2 et 3**

```bash
git add crates/rbs-cli/templates/feature/tests.rs.jinja crates/rbs-cli/src/generate/tests_http.rs
git commit
```

---

### Task 4: Le contrat OpenAPI déclare le 400 de la pagination

**Files:**
- Modify: `crates/rbs-cli/templates/feature/controller.rs.jinja` (annotation de `list`)
- Test: `crates/rbs-cli/src/generate/controller.rs` (module `#[cfg(test)]`)

**Interfaces:**
- Consumes: `ProblemDetails`, déjà importé par la template.
- Produces: rien.

**Constat à re-vérifier avant d'écrire :** `create` et `update` déclarent **déjà** leur 409
(`controller.rs.jinja`, blocs `responses` de `create` et `update`). Seul le 400 de `list`
manque. `Pagination::from_request_parts` (`crates/rbs-core/src/pagination.rs`) rend bien
`Error::BadRequest` sur `per_page=abc`. Ne rien « réparer » qui le soit déjà.

- [ ] **Step 1: Écrire le test de rendu qui échoue**

```rust
/// `per_page=abc` rend 400 : un document qui ne l'annonce pas fait débugger au client une
/// pagination qui « ne marche pas ».
#[test]
fn the_list_declares_the_400_of_the_pagination() {
    let rendered = render(&feature("articles", "title:string")).expect("controller rendu");

    let liste = rendered
        .split("pub async fn list(")
        .next()
        .expect("l'annotation précède le handler");

    assert!(
        liste.contains(r#"(status = 400, description = "pagination illisible""#),
        "le 400 de la pagination n'est pas déclaré :\n{liste}"
    );
}
```

Adapter le nom de l'aide (`feature(...)`) à celle qu'emploie déjà le module de tests de
`crates/rbs-cli/src/generate/controller.rs`.

- [ ] **Step 2: Lancer le test et le voir échouer**

Run: `cargo test -p rbs-cli --lib the_list_declares_the_400_of_the_pagination`
Expected: FAIL

- [ ] **Step 3: Compléter l'annotation de `list`**

```jinja
    responses(
        (status = 200, description = "page de {@ module @}", body = Page<{@ entity @}Response>),
        (status = 400, description = "pagination illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
```

Même forme que les autres statuts du fichier : `body = ProblemDetails` et
`content_type = "application/problem+json"`. Les entrées de `components/responses` que pose
`CommonResponses` ne sont adressables que par nom depuis le document, sans type Rust
implémentant `ToResponse` — les référencer depuis `#[utoipa::path]` n'est pas possible ici.

- [ ] **Step 4: Relancer le test**

Run: `cargo test -p rbs-cli --lib the_list_declares_the_400_of_the_pagination`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-cli/templates/feature/controller.rs.jinja crates/rbs-cli/src/generate/controller.rs
git commit
```

---

### Task 5: Les tests d'`auth` suivent la convention `#[ignore]`

**Files:**
- Modify: `crates/rbs-cli/templates/features/auth/tests.rs.jinja`
- Modify: `docs/docs/guides/auth.md:200-202`
- Modify: `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/auth.md:206`
- Test: `crates/rbs-cli/src/add/mod.rs` (module `#[cfg(test)]`)

**Interfaces:**
- Consumes: rien.
- Produces: rien.

**Constat :** les 21 tests du fichier passent tous par `application()`, `connection()` ou
`admin_only_route()`, qui ouvrent une connexion à la base du `.env`. Aucun n'est purement
unitaire — tous prennent donc le marqueur. Forme reprise de `features/jobs/tests.rs.jinja` :
`#[ignore = "joint la base du projet"]`, précédé d'un commentaire d'introduction unique.

- [ ] **Step 1: Écrire le test de rendu qui échoue**

```rust
/// `rbs new --with auth && cargo test` doit passer sans PostgreSQL, comme avec `jobs`.
#[test]
fn every_auth_test_joining_the_database_is_ignored() {
    let tests = fragment_template("auth", "tests.rs.jinja");

    assert_eq!(
        tests.matches("#[tokio::test]").count(),
        tests.matches(r#"#[ignore = "joint la base du projet"]"#).count(),
        "chaque test joint la base et doit porter le marqueur :\n{tests}"
    );
}
```

- [ ] **Step 2: Lancer le test et le voir échouer**

Run: `cargo test -p rbs-cli --lib every_auth_test_joining_the_database_is_ignored`
Expected: FAIL — 21 contre 0.

- [ ] **Step 3: Poser le commentaire d'introduction**

Après la constante `PASSWORD` et avant `async fn application()` :

```jinja
// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.
```

- [ ] **Step 4: Poser le marqueur sur chacun des 21 tests**

Run: `perl -0pi -e 's/#\[tokio::test\]\n/#[tokio::test]\n#[ignore = "joint la base du projet"]\n/g' crates/rbs-cli/templates/features/auth/tests.rs.jinja`
Puis relire le fichier : le marqueur suit `#[tokio::test]`, comme dans `jobs`.

- [ ] **Step 5: Relancer le test**

Run: `cargo test -p rbs-cli --lib every_auth_test_joining_the_database_is_ignored`
Expected: PASS

- [ ] **Step 6: Aligner la documentation, dans les deux langues**

`docs/docs/guides/auth.md` — la phrase « Those go through HTTP against a real database,
like every test rbs generates » doit dire que ces tests sont `#[ignore]` et se lancent par
`cargo test -- --ignored`. Même retouche dans la version française, ligne 206.

- [ ] **Step 7: Commit**

```bash
git add crates/rbs-cli/templates/features/auth/tests.rs.jinja crates/rbs-cli/src/add/mod.rs \
        docs/docs/guides/auth.md \
        docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/auth.md
git commit
```

---

### Task 6: La mise à jour générée devient un `PATCH`

**Files:**
- Modify: `crates/rbs-cli/templates/feature/mod.rs.jinja:21`
- Modify: `crates/rbs-cli/templates/feature/controller.rs.jinja:87-90`
- Modify: `crates/rbs-cli/templates/feature/service.rs.jinja:51-53` (commentaire seul)
- Modify: `crates/rbs-cli/templates/feature/tests.rs.jinja:179`
- Modify: `crates/rbs-cli/src/generate/controller.rs:98,305,374`
- Modify: `crates/rbs-cli/src/generate/repository.rs:173` (commentaire)
- Modify: `crates/rbs-cli/src/generate/tests_http.rs:191`
- Modify: `docs/docs/getting-started.md:354`
- Modify: `docs/i18n/fr/docusaurus-plugin-content-docs/current/getting-started.md:361`

**Interfaces:**
- Consumes: rien.
- Produces: le routeur généré monte `.patch(controller::update)` ; l'annotation utoipa
  déclare `patch` ; les tests générés envoient `PATCH`.

**Ce qui ne change pas :** `service::update` garde sa fusion, ligne pour ligne. Le
`PUT /uploads/{id}/content` de `file-drop` est une édition à la main sur une route de
contenu binaire, qui *remplace* réellement — il reste un `PUT`.

- [ ] **Step 1: Écrire les tests de rendu qui échouent**

Dans `crates/rbs-cli/src/generate/controller.rs`, remplacer la ligne de
`the_five_verbs_and_their_paths_are_documented` :

```rust
            "    patch,\n    path = \"/blog_posts/{id}\",",
```

et, dans `the_module_mounts_the_five_routes` :

```rust
                && rendered.contains(".patch(controller::update)")
```

Dans `crates/rbs-cli/src/generate/tests_http.rs`,
`the_lifecycle_exercises_the_five_routes_and_their_statuses` :

```rust
            r#"request("PATCH", &resource, sent.clone())"#,
```

Ajouter, dans `crates/rbs-cli/src/generate/controller.rs` :

```rust
/// Le service fusionne : un champ absent du corps garde sa valeur. `PUT` promettrait un
/// remplacement que ce code ne fait pas, et `PATCH` dit exactement ce qu'il fait.
#[test]
fn the_update_is_a_patch_and_no_put_survives() {
    let rendered = controller("articles");
    let module = module("articles");

    assert!(!rendered.contains("    put,"), "{rendered}");
    assert!(!module.contains(".put("), "aucun alias `put` ne survit :\n{module}");
}
```

- [ ] **Step 2: Lancer les tests et les voir échouer**

Run: `cargo test -p rbs-cli --lib generate::controller generate::tests_http`
Expected: FAIL — les annotations disent encore `put`.

- [ ] **Step 3: Passer le routeur en `patch`**

`crates/rbs-cli/templates/feature/mod.rs.jinja` :

```jinja
use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{@ module @}", get(controller::list).post(controller::create))
        .route(
            "/{@ module @}/{id}",
            get(controller::find)
                .patch(controller::update)
                .delete(controller::delete),
        )
}
```

- [ ] **Step 4: Passer l'annotation en `patch` et retirer le commentaire devenu paraphrase**

`crates/rbs-cli/templates/feature/controller.rs.jinja` : supprimer les deux lignes

```
// Un champ absent du corps garde sa valeur : la mise à jour est une fusion, non un
// remplacement, malgré le verbe `PUT` qu'attend un client de CRUD.
```

et remplacer `    put,` par `    patch,`.

- [ ] **Step 5: Réécrire le commentaire de `service.rs.jinja`**

Ce que `PATCH` dit déjà — « un champ absent garde sa valeur » — sort. Ce qu'il ne dit pas
reste : le `null` explicite n'est pas distingué de l'absence.

```jinja
    // `Option` ne distingue pas un champ absent d'un `null` explicite : cette route ne
    // peut donc pas remettre un champ optionnel à NULL. Ajoutez-y le cas si votre API en
    // a besoin.
```

- [ ] **Step 6: Passer le verbe des tests générés**

`crates/rbs-cli/templates/feature/tests.rs.jinja` :

```jinja
    let mise_a_jour = request("PATCH", &resource, sent.clone());
```

- [ ] **Step 7: Aligner les deux commentaires du générateur**

`crates/rbs-cli/src/generate/repository.rs:173` : « `PUT` rendrait 500 » devient
« `PATCH` rendrait 500 ».
`crates/rbs-cli/src/generate/controller.rs`, constante `VERIFICATION` :
`assert!(unit.put.is_some(), "PUT unitaire absent");` devient
`assert!(unit.patch.is_some(), "PATCH unitaire absent");`.

- [ ] **Step 8: Relancer les tests de rendu**

Run: `cargo test -p rbs-cli --lib generate::`
Expected: PASS

- [ ] **Step 9: Aligner la documentation, dans les deux langues**

`docs/docs/getting-started.md:354` : « The three remaining routes — `GET`, `PUT` and
`DELETE` on `/articles/{id}` » devient `GET`, `PATCH` et `DELETE`.
`docs/i18n/fr/…/getting-started.md:361` : même correction.

Vérifier par `grep -rniE '\bput\b' docs/docs docs/i18n` qu'aucune autre page n'annonce un
`PUT` sur une route CRUD générée. Les plans de `docs/superpowers/plans/` sont un journal
clos et ne se réécrivent pas.

- [ ] **Step 10: Commit**

```bash
git add crates/rbs-cli/templates/feature crates/rbs-cli/src/generate \
        docs/docs/getting-started.md \
        docs/i18n/fr/docusaurus-plugin-content-docs/current/getting-started.md
git commit
```

---

### Task 7: Répercuter les templates sur `examples/`

**Files:**
- Modify: `examples/hello-crud/src/articles/tests.rs`
- Modify: `examples/blog-auth/src/auth/{dto.rs,tests.rs}`
- Modify: `examples/file-drop/src/uploads/tests.rs`
- Modify: `examples/newsletter-queue/src/subscribers/tests.rs`
- Modify: les quatre `src/*/controller.rs` de CRUD (annotation de `list`)
- Test: `crates/rbs-cli/tests/integration_examples.rs` (aucun changement attendu)

**Interfaces:**
- Consumes: les templates des tâches 1 à 5.
- Produces: quatre exemples de nouveau identiques à ce que le CLI produit.

**Ce que chaque exemple gagne, selon ses `--fields` :**

| Exemple | `--fields` | 422 | 409 |
|---|---|---|---|
| `hello-crud` (`articles`) | `title:string,body:text,published:bool` | non | non |
| `blog-auth` (`posts`) | idem | non | non |
| `file-drop` (`uploads`) | `title,owner_email,content_type,size` | oui (`owner_email`) | non |
| `newsletter-queue` (`subscribers`) | `email:string:unique,name,confirmed` | oui | oui |

`examples/blog-auth/src/posts/{tests.rs,controller.rs}` et `src/auth/guard.rs` sont
**exclus** de la comparaison octet à octet (`integration_examples.rs`, champ
`edite_a_la_main`) : ce sont des fichiers retouchés à la main. Ils ne se régénèrent pas —
mais CI les compile, et `src/posts/tests.rs` envoie un `PUT` qui rendrait 405 une fois la
route passée en `PATCH`. Les deux fichiers de `posts` se corrigent donc **à la main** :
- `src/posts/controller.rs` : `    put,` → `    patch,`, et la suppression des deux lignes
  de commentaire sur la fusion ;
- `src/posts/tests.rs` : `requete("PUT", &ressource, …)` → `requete("PATCH", …)`.

`examples/file-drop/src/uploads/{controller.rs,mod.rs}` portent **deux** routes : la mise à
jour CRUD, qui devient `PATCH`, et `put_content` sur `/uploads/{id}/content`, édition à la
main qui remplace réellement le contenu et reste un `PUT`. Ne pas confondre les deux.

- [ ] **Step 1: Régénérer les quatre projets dans un répertoire de travail**

Suivre, commande pour commande, `examples/README.md` — mais **dans un répertoire
temporaire**, jamais par-dessus `examples/`. Écrire les quatre projets sous
`/private/tmp/claude-501/.../scratchpad/regen/`.

- [ ] **Step 2: Diffusion par diff, fichier par fichier**

Pour chaque projet, `diff -ru examples/<projet> <scratchpad>/regen/<projet>` et ne
reporter que les hunks qui viennent des templates modifiées. Les différences attendues
sont : `.env` (secret tiré au hasard), horodatage de migration, chemin de `rbs-core`,
marqueurs `// region:`, et les fichiers de la colonne `edite_a_la_main`. Toute autre
différence est le changement à reporter.

- [ ] **Step 3: Vérifier la non-dérive**

Run: `cargo test -p rbs-cli --test integration_examples`
Expected: PASS — c'est l'oracle.

- [ ] **Step 4: Vérifier que les exemples compilent encore**

Run: `for p in hello-crud blog-auth file-drop newsletter-queue; do cargo clippy --manifest-path examples/$p/Cargo.toml --all-targets -- -D warnings; done`
Expected: aucun avertissement.

- [ ] **Step 5: Commit**

```bash
git add examples/
git commit
```

---

### Task 8: Vérification finale

- [ ] **Step 1: `cargo fmt`**

Run: `cargo fmt --all --check`
Expected: silence.

- [ ] **Step 2: `cargo clippy`**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: aucune erreur.

- [ ] **Step 3: Suite rapide**

Run: `cargo test --workspace`
Expected: verte.

- [ ] **Step 4: Suite Docker, sans abréviation**

Run: `cargo test --workspace --no-fail-fast -- --ignored`
`--no-fail-fast` obligatoire : sans lui la suite s'arrête au premier binaire et masque les
échecs des suivants. Plusieurs minutes attendues. Lire la sortie réelle avant toute
affirmation de succès.

- [ ] **Step 5: Prouver la tâche 48 de bout en bout**

Docker **arrêté**, générer un projet avec `auth` et lancer son `cargo test` :

```bash
cd <scratchpad> && cargo run --manifest-path <worktree>/Cargo.toml -p rbs-cli --bin rbs -- \
  new preuve-auth --yes --core-path <worktree>/crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/preuve_auth' --lang fr
cd preuve-auth && git add -A && git commit -q -m 'projet neuf'
cargo run --manifest-path <worktree>/Cargo.toml -p rbs-cli --bin rbs -- add auth --yes
cargo test
```

Expected: compilation réussie, tests passés, tous les tests d'`auth` en `ignored`.
