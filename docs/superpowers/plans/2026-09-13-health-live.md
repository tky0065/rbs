# `/health/live` et `HEALTHCHECK` — plan d'implémentation

> **Pour les agents :** SOUS-SKILL REQUIS : superpowers:executing-plans. Étapes à cocher (`- [ ]`).

**But :** une route de liveness `GET /health/live` qui rend 200 sans rien interroger, à
côté de `/health` (readiness, inchangée), et un `HEALTHCHECK` du Dockerfile du fragment
`docker` qui la sonde.

**Architecture :** tout est dans les templates du squelette (`templates/project/src/health/`,
`templates/project/src/openapi.rs.jinja`) et du fragment (`templates/features/docker/Dockerfile.jinja`).
`rbs-core` inchangé. Les cinq `examples/` suivent par régénération différentielle.

**Spec :** `docs/superpowers/specs/2026-09-13-lot-features-p2-design.md`, section « 35 ».

## Contraintes globales

- `crate::health::controller::health,` reste **immédiatement** au-dessus de `// <rbs:openapi>`
  (accroche de `anchors::OPENAPI`, `anchors.rs:149`) : `live` s'insère avant elle.
- `operation_id = "health_live"`, `tag = "health"`, `path = "/health/live"`.
- Pas d'alias `/health/ready`. `/health` inchangée.
- `HEALTHCHECK` par bash et `/dev/tcp`, texte exact de la spec, dans l'étage `runtime`.
- Bloquant : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `integration_examples`, `npm run build` sous `docs/`.
- Passes lentes : un binaire par commande, `--no-fail-fast` avant le `--`, sortie vers
  `…/scratchpad/g2-<nom>.txt`.

---

### Tâche 1 : la route dans le squelette

**Fichiers :**
- Modifier : `crates/rbs-cli/templates/project/src/health/controller.rs.jinja`
- Modifier : `crates/rbs-cli/templates/project/src/health/mod.rs.jinja`
- Modifier : `crates/rbs-cli/templates/project/src/openapi.rs.jinja`
- Test : `crates/rbs-cli/src/new.rs` (module `tests`)

- [ ] **Étape 1 : test rouge** — dans `new.rs`, après `the_health_handler_declares_its_own_tag` :

```rust
    /// Une liveness qui sonde la base fait redémarrer l'API en boucle quand c'est la base
    /// qui tombe : `/health/live` ne prend pas l'état, et ne peut donc rien interroger.
    #[test]
    fn the_liveness_route_is_mounted_documented_and_queries_nothing() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        let controleur = read(&project.root.join("src/health/controller.rs"));
        assert!(controleur.contains("pub async fn live() -> StatusCode"), "{controleur}");
        assert!(controleur.contains("path = \"/health/live\""), "{controleur}");
        assert!(controleur.contains("operation_id = \"health_live\""), "{controleur}");

        let routes = read(&project.root.join("src/health/mod.rs"));
        assert!(routes.contains(".route(\"/health/live\", get(controller::live))"), "{routes}");
        assert!(routes.contains(".route(\"/health\", get(controller::health))"), "{routes}");

        // `health` reste la ligne d'accroche de `<rbs:openapi>` : `live` passe avant elle.
        let openapi = read(&project.root.join("src/openapi.rs"));
        assert!(
            openapi.contains(
                "crate::health::controller::live,\n        crate::health::controller::health,\n        // <rbs:openapi>"
            ),
            "{openapi}"
        );
    }
```

- [ ] **Étape 2 :** `cargo test -p rbs-cli --lib new::tests::the_liveness_route` → FAIL (`live` absent).
- [ ] **Étape 3 : implémentation**
  - `controller.rs.jinja` : `use axum::http::StatusCode;` (entre `extract::State` et
    `response::Response`), puis après `health` :

```rust
// Ne prend pas l'état : une sonde de vie qui interrogerait la base ferait redémarrer l'API
// en boucle le jour où c'est la base qui tombe. Cette question-là est celle de `/health`.
#[utoipa::path(
    get,
    path = "/health/live",
    tag = "health",
    operation_id = "health_live",
    responses((status = 200, description = "le processus répond, sans interroger ses dépendances"))
)]
pub async fn live() -> StatusCode {
    StatusCode::OK
}
```

  - `mod.rs.jinja` : `Router::new().route("/health", get(controller::health)).route("/health/live", get(controller::live))`,
    en chaîne sur plusieurs lignes comme rustfmt la rend.
  - `openapi.rs.jinja` : `crate::health::controller::live,` sur la ligne au-dessus de
    `crate::health::controller::health,`.
- [ ] **Étape 4 :** `cargo test -p rbs-cli --lib` **entier** (gardes rustfmt du squelette,
  accroches des ancres) → vert.

### Tâche 2 : `HEALTHCHECK` du fragment `docker`

**Fichiers :** Modifier `crates/rbs-cli/templates/features/docker/Dockerfile.jinja` ;
Test `crates/rbs-cli/src/templates.rs` (à côté de `the_docker_builder_installs_what_the_build_needs`).

- [ ] **Étape 1 : test rouge**

```rust
    /// L'image ne porte ni curl ni wget : la sonde passe par bash et `/dev/tcp`, et vise
    /// `/health/live` — une sonde sur `/health` tuerait le conteneur quand la base tombe.
    #[test]
    fn the_docker_runtime_probes_the_liveness_route() {
        let source = read(&Path::new(RACINE_FEATURES).join("docker/Dockerfile.jinja"));
        let runtime = source
            .split("AS runtime")
            .nth(1)
            .expect("le Dockerfile doit avoir une étape runtime");

        let sonde = runtime
            .lines()
            .skip_while(|ligne| !ligne.starts_with("HEALTHCHECK"))
            .take(2)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(sonde.contains("\"bash\", \"-c\""), "{runtime}");
        assert!(sonde.contains("/dev/tcp/127.0.0.1/8080"), "{sonde}");
        assert!(sonde.contains("GET /health/live HTTP/1.1"), "{sonde}");
    }
```

- [ ] **Étape 2 :** `cargo test -p rbs-cli --lib templates::tests::the_docker_runtime_probes` → FAIL.
- [ ] **Étape 3 :** entre `EXPOSE 8080` et le commentaire du `CMD`, le bloc de la spec,
  précédé d'un commentaire : pourquoi bash (aucun client HTTP dans l'image), pourquoi
  `/health/live` et non `/health`, le port qui suit `EXPOSE`, dans le Dockerfile pour valoir
  sous un `docker run` nu.
- [ ] **Étape 4 :** `cargo test -p rbs-cli --lib` entier → vert.

### Tâche 3 : test lent — la liveness survit à la base

**Fichier :** `crates/rbs-cli/tests/integration_new.rs`.

- [ ] **Étape 1 :** test `#[ignore]` `the_liveness_route_outlives_the_database` : un
  PostgreSQL (`common::start_postgres`), `rbs new sonde-vie --database-url <url> --core-path <noyau> --yes`,
  `rbs migrate up`, `cargo build` (cible `common::cible()`, sous `common::verrou`), le
  binaire lancé sur un port libre (`RBS_SERVER__PORT`) ; assertions :
  `GET /health/live` → 200, `GET /health` → 200, `GET /api-docs/openapi.json` porte
  `"/health/live"` et `"/health"` ; puis `postgres.stop()` ; `GET /health/live` → 200,
  `GET /health` → 503. Aides locales : `Serveur` (garde `Drop`), `attendre_ecoute`,
  `requete` (HTTP/1.1 écrite à la main, comme `integration_lang`).
- [ ] **Étape 2 :** la preuve rouge du comportement est celle de la tâche 1 (route absente) ;
  ce test prouve le comportement réel. Lancer en arrière-plan :
  `cargo test -p rbs-cli --test integration_new --no-fail-fast -- --ignored the_liveness_route_outlives_the_database > …/g2-live.txt 2>&1` → `1 passed`.

### Tâche 4 : exemples

- [ ] **Étape 1 : test rouge** — ajouter `"healthLive("` aux listes de méthodes de
  `the_typescript_client_of_hello_crud_is_in_place` (`integration_examples.rs`) et de
  `the_command_writes_a_client_that_carries_one_method_per_operation` (`integration_client.rs`) ;
  `cargo test -p rbs-cli --test integration_examples` → FAIL (client et cinq dérives).
- [ ] **Étape 2 :** générer les cinq exemples avec `g2-rbs-avant` (binaire d'avant) et avec
  le CLI neuf (`scratchpad/g2-gen-avant`, `g2-gen-apres`), selon `examples/README.md` ;
  `diff -ru` hors `.git`, `.env`, `Cargo.lock`, `migration/` ; appliquer par `patch` piloté
  depuis Python (`input=`) ; reprendre à la main `newsletter-queue/src/openapi.rs`
  (édité à la main). Relire l'emplacement de chaque hunk.
- [ ] **Étape 3 :** `rbs generate client --lang ts --force` dans `examples/hello-crud`.
- [ ] **Étape 4 :** `cargo test -p rbs-cli --test integration_examples -- --include-ignored` → vert ;
  `cargo check --all-targets` dans `examples/event-hub`.

### Tâche 5 : preuve Docker réelle

- [ ] Projet jetable `rbs new sonde-docker --with docker --database-url postgres://rbs:rbs@db:5432/sonde --yes`
  dans le scratchpad, noyau recopié dans le contexte (`vendor/rbs-core`, dépendance par
  chemin — `rbs-core` 1.5.0 n'est pas publié) ; `docker build -t g2-sonde-docker .` ;
  réseau `g2-sonde`, un PostgreSQL nommé `db`, puis l'API (`RBS_ENV=production`,
  `RBS_SERVER__HOST=0.0.0.0`, `RBS_DATABASE__URL`) ; attendre ; `docker inspect --format
  '{{.State.Health.Status}}'` → `healthy`. Arrêter `db` : le conteneur reste `healthy`,
  `/health` rend 503. Nettoyer conteneurs, réseau, image.

### Tâche 6 : documentation (EN + FR, même commit)

- `getting-started.md` : `/health` = readiness, `/health/live` = liveness ; `paths` porte
  aussi `/health/live`.
- `cli/new.md` : la phrase d'intro nomme les deux routes.
- `cli/add.md`, section `docker` : le `HEALTHCHECK`, sa cible, et Kubernetes qui l'ignore
  (ses sondes se configurent dans le manifeste : `livenessProbe` → `/health/live`,
  `readinessProbe` → `/health`).
- `tutorials/typescript-client.md` : 7 → 8 opérations (transcriptions et prose), `healthLive`.
- `tutorials/first-resource.md` : les six opérations s'ajoutent à `/health` et `/health/live`.
- [ ] `cd docs && npm run clear && npm run build` → code 0.

### Tâche 7 : vérification et commit

- [ ] `cargo fmt --all --check` ; `cargo clippy --workspace --all-targets -- -D warnings` ;
  `cargo test --workspace` ; `integration_docs` (rapide) ; `integration_client` (lent) ;
  `integration_new` (lent). Lire chaque `test result`.
- [ ] Commit `feat(templates): …`, corps : pourquoi + `Vérifications :` avec sorties réelles.
