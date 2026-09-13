# Tests engendrés des routes de contenu — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs generate crud --with-upload` livre dans `tests.rs` les scénarios de ses trois routes de contenu, et deux bancs Docker de rbs les exigent nommément.

**Architecture:** Un bloc `{% if with_upload %}` en fin de `tests.rs.jinja`, deux aides (`binary`, `call_raw`) et quatre scénarios sous les mêmes drapeaux que leurs voisins (`creatable`, `auth`). `tests_http::render` passe `with_upload` au contexte. Le banc SQLite d'`integration_crud` et le banc auth d'`integration_auth` jouent les tests du projet engendré et exigent leurs noms.

**Tech Stack:** minijinja (délimiteurs `{@ @}`), `tower::ServiceExt::oneshot`, `assert_cmd`, rustfmt via `bench::formatted`.

**Spec:** `docs/superpowers/specs/2026-09-12-with-upload-tests-engendres-design.md`

## Global Constraints

- `examples/file-drop/src/uploads/tests.rs` n'est pas édité à la main : il se remplace par le rendu neuf, `integration_examples` (19 passés) le compare.
- rustfmt par défaut dans le projet engendré : `max_width` 100, `fn_call_width` 60 — un appel dont les arguments dépassent soixante colonnes s'écrit déjà éclaté.
- Un commentaire dit le *pourquoi* ; le code engendré ne commente que ses points d'extension. Documentation bilingue dans le même commit.
- Commit Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.

---

### Task 1 : le bloc engendré et ses tests de rendu

**Files:**
- Modify: `crates/rbs-cli/templates/feature/tests.rs.jinja` (fin de fichier)
- Modify: `crates/rbs-cli/src/generate/tests_http.rs:29-63` (contexte) et `mod tests`

**Interfaces:**
- Produces: tests engendrés `the_content_round_trips_through_put_get_and_head`, `an_unknown_id_has_no_content`, `a_content_beyond_the_limit_returns_413`, `an_anonymous_content_request_returns_401`.

- [ ] **Step 1 : tests de rendu rouges** dans `tests_http.rs`

```rust
    /// Sous `--with-upload`, les trois routes de contenu ont leurs scénarios.
    #[test]
    fn with_upload_the_content_routes_earn_their_scenarios() {
        let rendered = render(&bench::uploads()).expect("les tests doivent se rendre");

        for scenario in [
            "async fn the_content_round_trips_through_put_get_and_head()",
            "async fn an_unknown_id_has_no_content()",
            "async fn a_content_beyond_the_limit_returns_413()",
        ] {
            assert!(rendered.contains(scenario), "« {scenario} » manque :\n{rendered}");
        }
        assert!(
            rendered.contains("vec![b'x'; super::TAILLE_MAX + 1]"),
            "la borne éprouvée doit être celle que `mod.rs` engendre :\n{rendered}"
        );
        assert!(
            !rendered.contains("an_anonymous_content_request_returns_401"),
            "sans `auth`, aucun refus anonyme :\n{rendered}"
        );
    }

    /// Témoin : sans le drapeau, rien des routes de contenu.
    #[test]
    fn without_upload_no_content_scenario_is_rendered() {
        let rendered = trials("articles", CHAMPS);

        assert!(
            !rendered.contains("/content") && !rendered.contains("fn call_raw("),
            "sans `--with-upload`, le rendu ne porte rien des routes de contenu :\n{rendered}"
        );
    }

    /// Sous `auth`, les routes de contenu sont fermées comme les autres, et le dépôt
    /// porte le jeton.
    #[test]
    fn under_auth_the_content_routes_refuse_an_anonymous_request() {
        let rendered =
            render(&bench::uploads().authenticated()).expect("les tests doivent se rendre");

        assert!(
            rendered.contains("async fn an_anonymous_content_request_returns_401()"),
            "le refus anonyme du contenu doit être éprouvé :\n{rendered}"
        );
        assert!(
            rendered.contains(
                ".header(\"content-type\", \"application/octet-stream\")\n        .header(\"authorization\", bearer())"
            ),
            "le dépôt doit porter le jeton :\n{rendered}"
        );
    }

    /// Aucun exemple ne rend cette combinaison : rustfmt est son seul oracle de forme.
    #[test]
    fn the_content_scenarios_under_auth_are_already_what_rustfmt_would_write() {
        let rendered =
            render(&bench::uploads().authenticated()).expect("les tests doivent se rendre");

        assert_eq!(bench::formatted(&rendered), rendered);
    }

    /// Une référence requise écarte les scénarios qui créent, le reste du bloc demeure.
    #[test]
    fn a_required_reference_keeps_the_content_scenarios_that_create_nothing() {
        let fields = fields::parse("title:string,author:references:users").expect("champs valides");
        let rendered =
            render(&Feature::fresh("posts", fields).uploading()).expect("les tests doivent se rendre");

        assert!(rendered.contains("async fn an_unknown_id_has_no_content()"));
        assert!(!rendered.contains("async fn the_content_round_trips_through_put_get_and_head()"));
        assert!(!rendered.contains("async fn a_content_beyond_the_limit_returns_413()"));
    }
```

- [ ] **Step 2 : voir le rouge** — `cargo test -p rbs-cli --lib generate::tests_http` : les quatre premiers échouent (le rendu ne porte pas `with_upload`).
- [ ] **Step 3 : contexte** — dans `render`, après `auth => feature.auth,` : `with_upload => feature.with_upload,`.
- [ ] **Step 4 : le bloc**, ajouté à la toute fin de `tests.rs.jinja`, après le `{%- endif %}` du bloc `auth` :

```jinja
{%- if with_upload %}

/// La requête d'un dépôt : un corps binaire, `application/octet-stream`.
fn binary(method: &str, path: &str, body: Vec<u8>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/octet-stream")
{%- if auth %}
        .header("authorization", bearer())
{%- endif %}
        .body(Body::from(body))
        .expect("requête bien formée")
}

/// Fait traverser le routeur à `request`, et rend son statut, son `Content-Type` et son
/// corps tel quel.
///
/// `call` lit le corps comme du JSON : le contenu déposé est binaire, et c'est l'octet
/// rendu qui se compare.
async fn call_raw(api: &Router, request: Request<Body>) -> (StatusCode, String, Vec<u8>) {
    let response = api
        .clone()
        .oneshot(request)
        .await
        .expect("l'application doit répondre");
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    (status, content_type, bytes.to_vec())
}
{%- if creatable %}

/// L'octet déposé par `PUT` est celui que `GET` rend, et `HEAD` reflète la présence d'un
/// contenu — avant le dépôt, après, et après son remplacement.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_content_round_trips_through_put_get_and_head() {
    let api = application().await;
    let collection = "/{@ module @}";

    let (status, created) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");
    let content = format!("{resource}/content");

    let (status, _, _) = call_raw(&api, without_body("HEAD", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "aucun contenu n'est encore déposé");
    let (status, _, _) = call_raw(&api, without_body("GET", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "aucun contenu n'est encore déposé");

    // Des octets qui ne forment pas de l'UTF-8 : un corps lu comme du texte les perdrait.
    let deposited = b"contenu binaire \xff\xfe\x00".to_vec();
    let (status, _, _) = call_raw(&api, binary("PUT", &content, deposited.clone())).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "dépôt refusé");

    let (status, content_type, read) = call_raw(&api, without_body("GET", &content)).await;
    assert_eq!(status, StatusCode::OK, "le contenu déposé doit se relire");
    assert_eq!(content_type, "application/octet-stream");
    assert_eq!(read, deposited, "l'octet rendu diffère de l'octet déposé");

    let (status, _, _) = call_raw(&api, without_body("HEAD", &content)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "HEAD doit refléter le dépôt");

    let replaced = b"second contenu".to_vec();
    let (status, _, _) = call_raw(&api, binary("PUT", &content, replaced.clone())).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "remplacement refusé");
    let (status, _, read) = call_raw(&api, without_body("GET", &content)).await;
    assert_eq!(status, StatusCode::OK, "le contenu remplacé doit se relire");
    assert_eq!(read, replaced, "le second dépôt doit remplacer le premier");

    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}

/// Un octet de trop franchit `TAILLE_MAX`, et le dépôt est refusé.
///
/// La borne est celle que `mod.rs` engendre : la relever garde ce test juste.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_content_beyond_the_limit_returns_413() {
    let api = application().await;
    let collection = "/{@ module @}";

    let (status, created) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");
    let content = format!("{resource}/content");

    let trop_grand = vec![b'x'; super::TAILLE_MAX + 1];
    let (status, _, _) = call_raw(&api, binary("PUT", &content, trop_grand)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "un octet de trop doit être refusé");

    let (status, _, _) = call_raw(&api, without_body("HEAD", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "le refus ne doit rien avoir déposé");

    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}
{%- endif %}

/// Un identifiant jamais créé n'a pas de contenu, et n'en reçoit pas non plus.
///
/// Le `PUT` surtout : sans la lecture préalable de la ligne, il déposerait un objet
/// qu'aucune ressource ne réclame.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_id_has_no_content() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let content = format!("/{@ module @}/{inconnu}/content");

    let (status, _, _) = call_raw(&api, binary("PUT", &content, b"orphelin".to_vec())).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "PUT sur un identifiant inconnu");

    let (status, body) = call(&api, without_body("GET", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "GET sur un identifiant inconnu");
    assert_eq!(body["status"], 404, "{body}");

    let (status, _, _) = call_raw(&api, without_body("HEAD", &content)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "HEAD sur un identifiant inconnu");
}
{%- if auth %}

/// Les routes de contenu sont fermées comme les autres : sans jeton, 401 avant toute
/// lecture de ligne — l'identifiant inconnu rendrait 404 si la ligne était lue d'abord.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_anonymous_content_request_returns_401() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let content = format!("/{@ module @}/{inconnu}/content");

    let depot = Request::builder()
        .method("PUT")
        .uri(content.as_str())
        .header("content-type", "application/octet-stream")
        .body(Body::from("anonyme"))
        .expect("requête bien formée");
    let (status, body) = call(&api, depot).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401, "{body}");

    let lecture = Request::builder()
        .method("GET")
        .uri(content.as_str())
        .body(Body::empty())
        .expect("requête bien formée");
    let (status, body) = call(&api, lecture).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["status"], 401, "{body}");
}
{%- endif %}
{%- endif %}
```

- [ ] **Step 5 : voir le vert** — `cargo test -p rbs-cli --lib generate::tests_http` ; si `the_content_scenarios_under_auth_are_already_what_rustfmt_would_write` échoue, éclater dans le gabarit les lignes que rustfmt éclate (`fn_call_width` 60).
- [ ] **Step 6 : `file-drop`** — projet temporaire : `rbs new tmp --yes --core-path <crates/rbs-core> --database-url postgres://rbs:rbs@localhost:5432/file_drop --lang fr`, `rbs add storage`, `rbs generate crud uploads --fields 'title:string,owner_email:string,content_type:string,size:int' --with-upload`, copier `src/uploads/tests.rs` sur `examples/file-drop/src/uploads/tests.rs`. Puis `cd examples/file-drop && cargo test --no-run && cargo clippy --all-targets -- -D warnings && cargo fmt --check`, et `cargo test -p rbs-cli --test integration_examples` (19).

### Task 2 : les deux bancs Docker exigent les scénarios

**Files:**
- Modify: `crates/rbs-cli/tests/integration_crud.rs` (`the_deposited_content_round_trips_through_the_running_server`, avant `Serveur::lancer`)
- Modify: `crates/rbs-cli/tests/integration_auth.rs` (`the_tests_of_a_crud_generated_under_auth_pass`)

- [ ] **Step 1 : `integration_crud`** — après `cargo build`, avant le serveur :

```rust
    // Les scénarios de contenu vivent dans le projet, et s'exigent nommément : un gabarit
    // qui cesserait de les livrer laisserait ce banc au vert, `cargo test` sortant en 0
    // sur une suite amputée. Avant le serveur : les deux partagent le fichier SQLite.
    let sortie = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", &cible)
        .args(["test", "--workspace", "--", "--include-ignored"])
        .output()
        .expect("cargo doit être lançable");
    let joues = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );
    assert!(
        sortie.status.success(),
        "la suite du projet engendré échoue :\n{joues}"
    );
    for scenario in [
        "attachments::tests::the_content_round_trips_through_put_get_and_head ... ok",
        "attachments::tests::an_unknown_id_has_no_content ... ok",
        "attachments::tests::a_content_beyond_the_limit_returns_413 ... ok",
    ] {
        assert!(
            joues.contains(scenario),
            "`{scenario}` n'a pas été joué :\n{joues}"
        );
    }
```

- [ ] **Step 2 : `integration_auth`** — après `common::commiter(&racine, "auth installée")` : `rbs add storage` dans `racine`, `common::commiter(&racine, "storage installée")`, `"--with-upload"` ajouté aux arguments de `generate`, et deux noms de plus dans la liste : `articles::tests::an_anonymous_content_request_returns_401 ... ok` et `articles::tests::the_content_round_trips_through_put_get_and_head ... ok`. Mettre à jour le commentaire de doc du test.
- [ ] **Step 3 : Docker** — `cargo test --no-fail-fast -p rbs-cli --test integration_crud -- --ignored the_deposited_content_round_trips_through_the_running_server > …/scratchpad/storage-t20-crud.log 2>&1` puis `cargo test --no-fail-fast -p rbs-cli --test integration_auth -- --ignored the_tests_of_a_crud_generated_under_auth_pass > …/scratchpad/storage-t20-auth.log 2>&1` ; lire les deux fichiers.

### Task 3 : documentation, changelog, finitions

**Files:**
- Modify: `docs/docs/guides/storage.md` (section « Generated content routes ») et sa version FR (« Les routes de contenu engendrées »)
- Modify: `docs/docs/cli/generate.md:78` et sa version FR `:80` (ligne `--with-upload`)
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md` (`## [1.5.0]`, *Fixed* / *Corrigé*)

- [ ] **Step 1 : guide** — après le tableau des trois routes, un paragraphe : le drapeau écrit aussi leurs tests dans `tests.rs` (cycle PUT → GET → HEAD avec remplacement, 404 d'un identifiant inconnu sur les trois verbes, 413 au-delà de `TAILLE_MAX`, 401 sans jeton sous `auth`), tous `#[ignore]` comme les autres.
- [ ] **Step 2 : page `generate`** — dans la cellule `--with-upload`, une phrase : « It also writes their tests into `tests.rs` — the round trip, the 404s, the 413, and the 401 under `auth`. » / « Il écrit aussi leurs tests dans `tests.rs` — le cycle, les 404, le 413, et le 401 sous `auth`. »
- [ ] **Step 3 : changelog** — un item par langue.
- [ ] **Step 4 : vérifier** — `cargo test -p rbs-cli --test integration_docs -- --include-ignored` (14), `cd docs && npm test` (16) et `npm run typecheck` ; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p rbs-cli --lib`.
- [ ] **Step 5 : commit** `feat(generate): livre avec --with-upload les tests de ses routes de contenu`.
