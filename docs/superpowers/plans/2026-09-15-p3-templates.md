# Lot P3 templates engendrés — plan d'implémentation

> **Pour l'agent qui exécute :** suivre ce plan tâche par tâche avec
> `superpowers:executing-plans`, en TDD (`superpowers:test-driven-development`) : voir le
> test échouer avant d'écrire le code. Les étapes se cochent (`- [ ]`) au fil de l'eau.

**But :** assainir le fragment `webhooks`, doter le manifeste et le Dockerfile engendrés
d'une MSRV, d'un profil release et d'un cache de dépendances, et faire lire le stockage
en flux.

**Architecture :** trois tâches séquentielles sur une branche. `webhooks` d'abord (un
exemple), le manifeste ensuite (les cinq exemples), le stockage enfin (un exemple, et la
spec la plus lourde).

**Stack :** templates minijinja (délimiteurs `{@ @}`, `{% %}`), SeaORM 2, axum 0.8,
validator 0.21, aws-sdk-s3, BuildKit.

**Spec :** `docs/superpowers/specs/2026-09-15-storage-en-flux-design.md` (tâche 3) ;
design validé en conversation le 2026-09-15 pour les tâches 1 et 2 ; lignes 64, 63 et 55
d'`IMPROVE.md` (elles portent le `fichier:ligne` du diagnostic).

## Contraintes globales

- **Branche :** `git checkout -b improve/p3-templates improve/p3-lot`. Avant la première
  ligne : `git rev-list --count HEAD..improve/p3-lot` doit rendre `0`, sinon
  `git merge improve/p3-lot` d'abord.
- **Commits :** Conventional Commits, sujet français à l'impératif, sans majuscule ni
  point final. Corps : le *pourquoi*, puis `Vérifications :` avec les commandes lancées
  et leur résultat réel. **Jamais** d'identifiant de tâche (`#55`…), de renvoi à
  `IMPROVE.md`/`TODO.md`/un plan, ni de ligne `Co-Authored-By` ou `Claude-Session`.
- **Ne pas toucher `IMPROVE.md`**, ni `crates/rbs-core`, ni `crates/rbs-cli/src/cli.rs`,
  ni les templates `auth` : un autre agent y travaille en parallèle.
- **Langue engendrée :** tout message d'erreur rendu par le code engendré a ses deux
  versions `{% if lang == "en" %}…{% else %}…{% endif %}`.
- **Blancs minijinja :** `-%}` mange l'indentation de la ligne suivante ; un blanc perdu
  ne se voit que dans `integration_examples`.
- **Documentation bilingue :** toute page `docs/docs/…` modifiée l'est aussi sous
  `docs/i18n/fr/docusaurus-plugin-content-docs/current/…`, dans le même commit.
  `CHANGELOG.md` et `CHANGELOG.fr.md` : section `[1.5.0]`, même nombre d'items.
- **Exemples :** régénérer **par diff entre deux générations** (`examples/README.md`
  donne les commandes exactes), jamais par écrasement : les exemples portent des éditions
  à la main. `patch --no-backup-if-mismatch`, sinon un `.orig` fait échouer
  `integration_examples`. Après toute retouche d'`examples/`, `npm run build` sous
  `docs/` (après `npm ci`, le worktree est neuf) : seul le build voit une région citée
  disparue.
- **Code d'un fragment :** `cargo check --manifest-path examples/<ex>/Cargo.toml
  --all-targets` avant toute passe Docker.
- **Passes lentes (Docker) :** une commande par suite, `--no-fail-fast` **avant** le `--`,
  sortie redirigée vers
  `/private/tmp/claude-501/-Users-yacoubakone-dev-rs/b9dae65b-ac43-40c1-833d-4c73520587e2/scratchpad/lotB-<suite>.log`
  (préfixe `lotB-` obligatoire : un autre agent écrit dans le même répertoire). Le shell
  coupe à 600 s : lancer en arrière-plan.
- **Littéraux :** avant de changer un message ou un code de statut engendré, `grep -rn`
  dans `crates/rbs-cli/tests` — les tests lents figent des chaînes de fragments.
- **Dépendances :** dernière version stable, vérifiée par `cargo add --dry-run <crate>`
  (`cargo info` remonte les pré-publications).
- **Shell zsh :** `ls` est `eza` (`command ls`), guillemets autour des motifs `*`,
  jamais `===` nu. Le binaire se lance par `cargo run -q -p rbs-cli --bin rbs -- …`.
- **Portes finales de chaque tâche :** `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- Hors périmètre rencontré en route → le noter dans le rapport final, ne pas le corriger.

---

### Tâche 1 : `webhooks` — sans clone, révocation atomique, motifs validés par le DTO

**Fichiers :**
- Modifier : `crates/rbs-cli/templates/features/webhooks/service.rs.jinja:39` (clone), `:70-79` (boucle des motifs, à retirer)
- Modifier : `crates/rbs-cli/templates/features/webhooks/repository.rs.jinja:59-85` (`revoke`)
- Modifier : `crates/rbs-cli/templates/features/webhooks/dto.rs.jinja:15-16`
- Modifier : `crates/rbs-cli/templates/features/webhooks/tests.rs.jinja`
- Régénérer : `examples/event-hub`
- Modifier : `docs/docs/guides/webhooks.md` + jumelle FR si elle cite le 400 d'un motif vide

- [ ] **Étape 1 : tests qui échouent** dans `tests.rs.jinja`, sur le modèle de
`a_user_role_is_refused_on_the_three_routes` (aides `table_a_soi`, `token`, `request`,
`call`) :
  - `an_empty_pattern_is_refused_by_validation` — `POST /webhooks` (rôle admin) avec
    `{"url": "https://example.test/x", "events": ["user.*", "  "]}` → **422**, et le
    corps nomme le champ `events`. Aujourd'hui : 400.
  - `revoking_twice_keeps_the_first_date` — s'il n'existe pas déjà (`grep -n 'revoke'
    tests.rs.jinja`) : révoquer, relire `revoked_at`, révoquer encore, constater la même
    date.
  Ces tests tournent contre une base : ils vivent dans la passe lente. Les rendre
  compilables tout de suite par `cargo check` sur l'exemple régénéré (étape 4) ; les voir
  échouer puis passer dans `integration_webhooks` (étape 5). Vérifier le nom de la suite :
  `command ls crates/rbs-cli/tests | grep -i webhook`.

- [ ] **Étape 2 : implémenter**
  - `service.rs.jinja:39` : `serde_json::from_value(abonnement.events)?` — déplacer le
    champ ; `abonnement.id` (Copy) reste lisible ensuite.
  - `repository.rs.jinja`, `revoke` : un seul `UPDATE` conditionnel, sur l'idiome
    qu'emploie déjà `auth` (`features/auth/repository/refresh_token.rs.jinja:100-110`,
    le lire et copier sa forme) :

    ```rust
    // Un seul UPDATE conditionnel : deux révocations concurrentes ne peuvent pas écrire
    // chacune sa date, la première gagne et la seconde ne touche aucune ligne.
    Entity::update_many()
        .col_expr(Column::RevokedAt, Expr::value(quand))
        .col_expr(Column::UpdatedAt, Expr::value(quand))
        .filter(Column::Id.eq(id))
        .filter(Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    find(db, id).await?.ok_or(Error::NotFound("abonnement"))   // "subscription" en `en`
    ```
    Garder le doc-commentaire « Révoquer deux fois ne repousse pas la date… ».
  - `dto.rs.jinja` : `#[validate(length(min = 1), custom(function = "motifs_non_vides"))]`
    sur `events`, et la fonction dans le même fichier, message dans les deux langues :

    ```rust
    fn motifs_non_vides(motifs: &[String]) -> Result<(), ValidationError> {
        if motifs.iter().any(|motif| motif.trim().is_empty()) {
            return Err(ValidationError::new("blank").with_message(
                {% if lang == "en" %}"an event pattern cannot be empty"{% else %}"un motif d'événement ne peut pas être vide"{% endif %}.into(),
            ));
        }
        Ok(())
    }
    ```
    Vérifier la signature qu'attend validator 0.21 pour un `Vec<String>` (`&Vec<String>`
    ou `&[String]`) et s'il existe déjà un validateur `custom` dans les templates
    (`grep -rn 'custom(function' crates/rbs-cli/templates`) : suivre sa forme.
  - Retirer la boucle `for motif in &input.events { … }` du service.

- [ ] **Étape 3 : littéraux** — `grep -rn "motif d'événement\|event pattern cannot"
crates/rbs-cli/tests docs/docs docs/i18n` ; mettre à jour ce qui attendait un 400.

- [ ] **Étape 4 : régénérer event-hub, compiler**

```bash
cargo check --manifest-path examples/event-hub/Cargo.toml --all-targets
cargo clippy --manifest-path examples/event-hub/Cargo.toml --all-targets -- -D warnings
cargo test -p rbs-cli --test integration_examples
```

- [ ] **Étape 5 : passe lente webhooks** (en arrière-plan, sortie `lotB-webhooks.log`),
puis `grep -E '^test result|FAILED|panicked'`.

- [ ] **Étape 6 : CHANGELOG** (`Changed` : un motif vide rend 422 comme une URL
invalide ; `Fixed` : la révocation concurrente), portes finales, **commit** :
`fix(webhooks): révoque en un UPDATE conditionnel et valide les motifs dans le DTO`.

---

### Tâche 2 : MSRV, profil release et cache BuildKit dans le projet engendré

**Fichiers :**
- Modifier : `crates/rbs-cli/templates/project/Cargo.toml.jinja` (après `edition`, et en fin de fichier)
- Modifier : `crates/rbs-cli/templates/features/docker/Dockerfile.jinja:1,11-14,23-24`
- Modifier : les contextes de rendu qui portent `rbs_version` (`grep -rn 'rbs_version =>'
  crates/rbs-cli/src` : `new.rs:401`, et celui d'`add` qui rend le Dockerfile —
  `grep -rn 'project_name =>' crates/rbs-cli/src`), plus les contextes de test
  (`templates.rs:340`, `metadata.rs:760`)
- Régénérer : les cinq `examples/*` (manifeste) et `examples/event-hub/Dockerfile`
- Modifier : pages de doc qui citent `rust:1-slim-trixie` ou le Dockerfile (`grep -rn
  'slim-trixie\|cargo build --release' docs/docs docs/i18n --include='*.md'`)

**Interface produite :** variable de rendu `rust_version`, valeur
`env!("CARGO_PKG_RUST_VERSION")` du CLI — `1.94` aujourd'hui, la MSRV du workspace et
donc de `rbs-core`, dont tout projet engendré dépend.

- [ ] **Étape 1 : test qui échoue** — dans les tests de rendu du squelette (trouver ceux
qui lisent `Cargo.toml` rendu : `grep -rn 'edition = \\"2024\\"\|default-run'
crates/rbs-cli/src crates/rbs-cli/tests`), asserter :
  - `rust-version = "1.94"` (lu depuis `env!("CARGO_PKG_RUST_VERSION")` dans le test, pas
    en dur) ;
  - la table `[profile.release]` avec `lto = "thin"`, `codegen-units = 1`, `strip = true` ;
  - un test de rendu du Dockerfile : `FROM rust:1.94-slim-trixie`, `--mount=type=cache`,
    ligne `# syntax=docker/dockerfile:1` en tête.

```bash
cargo test -p rbs-cli --lib   # attendu : les nouveaux tests échouent
```

- [ ] **Étape 2 : manifeste**

```toml
edition = "2024"
rust-version = "{@ rust_version @}"
```
et en fin de fichier :

```toml
# `thin` garde l'essentiel du gain de `lto = true` pour une fraction du temps d'édition de
# liens ; une seule unité de code laisse l'optimiseur voir tout le paquet. Le binaire part
# sans symboles : l'image n'a pas de débogueur pour s'en servir.
[profile.release]
lto = "thin"
codegen-units = 1
strip = true
```

- [ ] **Étape 3 : Dockerfile**

```dockerfile
# syntax=docker/dockerfile:1
FROM rust:{@ rust_version @}-slim-trixie AS builder
…
WORKDIR /build
COPY . .
# Les montages de cache gardent le registre et `target/` d'un build à l'autre : un commit
# ne recompile plus sea-orm ni axum. Ils n'entrent pas dans la couche, d'où la copie des
# deux binaires hors de `target/` dans la même instruction.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release --workspace --bins \
 && mkdir -p /build/out \
 && cp target/release/{@ project_name @} target/release/migration /build/out/
…
COPY --from=builder /build/out/{@ project_name @} /usr/local/bin/
COPY --from=builder /build/out/migration /usr/local/bin/
```
Garder tels quels les commentaires existants (curl, `--bins`, templates de mail,
HEALTHCHECK). Vérifier que l'image épinglée existe :
`docker manifest inspect rust:1.94-slim-trixie > /dev/null && echo ok`.

- [ ] **Étape 4 : passer `rust_version` aux contextes** — même valeur partout, une seule
source : une constante `RUST_VERSION: &str = env!("CARGO_PKG_RUST_VERSION")` là où vit
déjà la version (`new.rs` ou `templates.rs`), réutilisée par `add`.

```bash
cargo test -p rbs-cli --lib   # attendu : verts
```

- [ ] **Étape 5 : régénérer les cinq exemples, compiler, construire l'image**

```bash
cargo test -p rbs-cli --test integration_examples
for ex in blog-auth event-hub file-drop hello-crud newsletter-queue; do
  cargo check --manifest-path "examples/$ex/Cargo.toml" --all-targets || echo "ÉCHEC $ex"
done
cd examples/event-hub && time docker build -t rbs-event-hub-test . && \
  touch src/main.rs && time docker build -t rbs-event-hub-test .   # le second doit réutiliser les dépendances
docker run --rm rbs-event-hub-test ls -la /usr/local/bin/
```
Consigner les deux durées : elles sont la preuve du cache.

- [ ] **Étape 6 : doc et CHANGELOG** (`Changed`), `npm run build` sous `docs/`, portes
finales, **commit** : `feat(templates): pose rust-version, un profil release et un cache
BuildKit dans le projet engendré`. Le corps dit pourquoi `thin` et non `fat`, et pourquoi
pas de `rust-toolchain.toml` (il imposerait un téléchargement de toolchain à chaque
utilisateur ; `rust-version` suffit à signaler une toolchain trop vieille).

---

### Tâche 3 : lire les objets en flux, déposer des `Bytes`

Lire la spec avant de commencer : `docs/superpowers/specs/2026-09-15-storage-en-flux-design.md`.

**Fichiers :**
- Modifier : `crates/rbs-cli/templates/features/storage/mod.rs.jinja` (trait, `Object`, `ContentStream`)
- Modifier : `crates/rbs-cli/templates/features/storage/files.rs.jinja:78-98`
- Modifier : `crates/rbs-cli/templates/features/storage/s3.rs.jinja:50-81`
- Modifier : `crates/rbs-cli/templates/features/storage/tests.rs.jinja` (ronde, test du flux, test hors trait)
- Modifier : `crates/rbs-cli/templates/features/storage/feature.toml` (`bytes`, `futures-util`, `tokio-util` + `io`)
- Modifier : `crates/rbs-cli/templates/feature/service.rs.jinja:176-215` (`put_content`, `get_content`)
- Modifier : `crates/rbs-cli/templates/feature/controller.rs.jinja:320-370`
- Modifier : `crates/rbs-cli/templates/feature/tests.rs.jinja:718-760` (`content-length`)
- Régénérer : `examples/file-drop`
- Modifier : `docs/docs/guides/storage.md` (+ FR) s'il cite les signatures ;
  `crates/rbs-cli/notes/1.5.0.md` ; les deux CHANGELOG

**Interfaces produites (code engendré) :**

```rust
pub type ContentStream = futures_util::stream::BoxStream<'static, std::io::Result<bytes::Bytes>>;
pub struct Object { pub length: Option<u64>, pub body: ContentStream }
async fn put(&self, key: &str, content: bytes::Bytes) -> Result<(), StorageError>;
async fn get(&self, key: &str) -> Result<Object, StorageError>;
// service CRUD
pub async fn put_content(db, storage: &dyn Storage, id: Uuid, content: bytes::Bytes) -> Result<()>;
pub async fn get_content(storage: &dyn Storage, id: Uuid) -> Result<storage::Object>;
```

- [ ] **Étape 1 : inventaire des appelants** — `grep -rn '\.put(\|\.get(\|Vec<u8>'
crates/rbs-cli/templates/features/storage crates/rbs-cli/templates/feature
examples/file-drop/src` : tout appelant du trait (y compris le code écrit à la main de
`file-drop`) est dans la liste des fichiers à modifier.

- [ ] **Étape 2 : tests qui échouent** (`storage/tests.rs.jinja`)
  - Une aide de lecture, et la ronde qui s'en sert :

    ```rust
    /// Recolle un objet lu, et rend la taille annoncée avec lui.
    async fn read(storage: &dyn Storage, key: &str) -> Result<(Option<u64>, Vec<u8>), StorageError> {
        let object = storage.get(key).await?;
        let morceaux: Vec<bytes::Bytes> = object.body.try_collect().await.expect("le flux doit se lire");
        Ok((object.length, morceaux.concat()))
    }
    ```
    `round` dépose `bytes::Bytes::from_static(b"%PDF-1.7")`, asserte
    `read(...) == (Some(8), b"%PDF-1.7".to_vec())`, et `NotFound` après suppression.
  - `a_large_object_reads_back_in_several_chunks` — backend fichiers, objet de 1 Mio :
    `storage.get(key).await?.body` rend **plus d'un** morceau, et leur concaténation
    égale le dépôt. C'est la preuve que l'objet n'est plus chargé d'un bloc.
  - Le test hors trait S3 (`an_object_put_by_the_trait_reads_back_through_the_s3_client`)
    dépose un `Bytes`.
  - `feature/tests.rs.jinja`, `the_content_round_trips_through_put_get_and_head` : le GET
    porte `content-length` égal à la taille déposée.

```bash
cargo test -p rbs-cli --lib   # tests de rendu éventuels
```
Les tests du fragment ne compilent que dans un projet engendré : les voir échouer à
l'étape 4 (`cargo check` de `file-drop` régénéré avant l'implémentation, ou
`cargo test` sur le projet sans Docker pour le backend fichiers).

- [ ] **Étape 3 : implémenter** selon la spec — `mod.rs` (type, struct avec un
`///` par item, `Debug` manuel sur `Object` si le trait l'exige), `files.rs`
(`File::open` → `NotFound` ou `unavailable` ; `metadata().len()` ;
`ReaderStream::new(file).boxed()`), `s3.rs` (`ByteStream::from(content)` en dépôt ;
`content_length()` converti en `u64`, `ReaderStream::new(object.body.into_async_read())`
en lecture — `ContentStream` et non `ByteStream` pour ne pas masquer
`aws_sdk_s3::primitives::ByteStream`), `feature.toml` (trois dépendances, dernière
version stable), service et contrôleur CRUD :

```rust
// controller, get_content
let object = service::get_content(state.storage().as_ref(), id).await?;
let mut response = Body::from_stream(object.body).into_response();
let headers = response.headers_mut();
headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
if let Some(length) = object.length {
    headers.insert(CONTENT_LENGTH, HeaderValue::from(length));
}
Ok(response)
```
`put_content` passe `content` sans `to_vec()`. Le commentaire existant sur le base64
(`controller.rs:300-301`) reste vrai.

- [ ] **Étape 4 : régénérer file-drop, compiler, tests sans Docker**

```bash
cargo check --manifest-path examples/file-drop/Cargo.toml --all-targets
cargo clippy --manifest-path examples/file-drop/Cargo.toml --all-targets -- -D warnings
cargo test -p rbs-cli --test integration_examples
```

- [ ] **Étape 5 : passes lentes** (arrière-plan, une par commande) :
`integration_storage` (MinIO : `lotB-storage.log`) puis `integration_crud`
(`lotB-crud.log`) ; `grep -E '^test result|FAILED|panicked'` sur chacune.

- [ ] **Étape 6 : doc, note, CHANGELOG** — `guides/storage.md` EN/FR ; `notes/1.5.0.md` :
un projet existant garde son trait, et voici comment adopter le flux (les deux
signatures, `get_content` du contrôleur) ; CHANGELOG `Changed` (les deux langues).
`npm run build` sous `docs/`. Portes finales. **Commit** : `perf(storage): lit les objets
en flux et dépose des Bytes sans copie`.

---

## Rapport final attendu

Pour chaque tâche : commit (hash), commandes de preuve et leur résultat **copié** (dont
les deux durées de `docker build`), écarts au plan et leur raison, hors-périmètre
rencontré. Dire explicitement ce qui n'a pas pu être prouvé.
