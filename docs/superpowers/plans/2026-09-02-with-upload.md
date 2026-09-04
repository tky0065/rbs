# `rbs generate crud --with-upload` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un drapeau `--with-upload` sur `rbs generate crud` qui engendre trois routes de contenu binaire branchées sur le trait `Storage` du fragment `storage` — exactement ce que `examples/file-drop` porte aujourd'hui à la main.

**Architecture:** Le drapeau engendre ce qui existe déjà, écrit à la main, dans un exemple compilé en CI. Le contrôleur passe le stockage sans le toucher, le service dérive la clé de l'`id` et parle au trait, le repository ne bouge pas. Une garde refuse le drapeau sur un projet sans le fragment `storage`, avant toute écriture.

**Tech Stack:** Rust 2024, clap (dérive), minijinja aux délimiteurs `{@ @}`, axum (`Bytes`, `DefaultBodyLimit`), le trait `Storage` du fragment (`templates/features/storage/mod.rs.jinja:35`).

**Spec:** `docs/superpowers/specs/2026-09-02-with-upload-design.md`

**Dépendance :** ce plan consomme la struct `GenerateArgs` posée par la **Task 1 du plan `2026-09-02-soft-delete.md`**. Il s'exécute après elle, et sur les mêmes templates : les deux ne peuvent pas tourner en parallèle.

## Global Constraints

- **Délimiteurs minijinja alternatifs** : `{@ variable @}`, jamais `{{ }}` (`crates/rbs-cli/src/template.rs:19-22`).
- **`UndefinedBehavior::Strict`** (`template.rs:26`) : une variable référencée et absente du contexte fait échouer le rendu. `mod.rs` est rendu par `controller::render_mod`, dont le contexte est **reconstruit** (`controller.rs:28-33`, `context! { with_tests, ..Value::from_serialize(feature) }`) : la clé `with_upload` y arrive par la sérialisation de `Feature`, mais **le vérifier est un point de contrôle de la Task 3**.
- **Les templates écrivent ce que rustfmt écrirait** : 100 colonnes (`max_width`), 98 pour un `use`, 60 pour une chaîne (`chain_width`) et pour les arguments d'un appel (`fn_call_width`).
- **Un commentaire explique le *pourquoi*, jamais le *quoi*.**
- Un fichier de feature au-delà de ~200 lignes signale une feature à scinder — `controller.rs.jinja` est déjà à 200 lignes, et gagne trois handlers : **surveiller, et scinder le fichier engendré si le seuil est franchi de beaucoup.**
- Le code **engendré** doit passer `clippy -D warnings` : un `use` posé sans usage le ferait échouer.
- Bancs `#[ignore]` : Docker requis, **`--no-fail-fast` obligatoire**, noms de feature et de migration uniques (`static CARGO: Mutex<()>`, `bench.rs:80`).
- Rediriger la sortie des suites longues vers le scratchpad.
- CHANGELOG et documentation **par paire** anglais/français dans le même commit ; `docs/scripts/parite.mjs` le contrôle. `examples/README.md` a aussi sa paire `examples/README.fr.md`.
- Conventional Commits, sujet français à l'impératif, sans identifiant de tâche, sans renvoi à un fichier de suivi, **sans `Co-Authored-By`**. Intertitre `Vérifications :`.
- Branche `improve/p3-features-lot-un`.

## File Structure

| Fichier | Responsabilité | Action |
|---|---|---|
| `crates/rbs-cli/src/cli.rs:136-158` | déclaration du drapeau | Modifier |
| `crates/rbs-cli/src/lib.rs` | `GenerateArgs`, le `match`, `Options` | Modifier |
| `crates/rbs-cli/src/generate/command.rs` | `Options`, garde `storage`, report sur `Feature` | Modifier |
| `crates/rbs-cli/src/generate/feature.rs` | le champ et sa clé | Modifier |
| `crates/rbs-cli/src/generate/mount.rs:12, 27-50` | les trois handlers à l'ancre `openapi` | Modifier |
| `crates/rbs-cli/templates/feature/service.rs.jinja` | `content_key`, dépôt, relecture, présence ; `delete` étendu | Modifier |
| `crates/rbs-cli/templates/feature/controller.rs.jinja` | trois handlers annotés | Modifier |
| `crates/rbs-cli/templates/feature/mod.rs.jinja` | trois routes, constante de taille, `DefaultBodyLimit` | Modifier |
| `examples/file-drop/**` | ses trois handlers deviennent engendrés | Modifier |
| `crates/rbs-cli/tests/integration_examples.rs:230-235` | l'entrée `edite_a_la_main` se réduit | Modifier |
| `CHANGELOG` ×2, `examples/README` ×2, guide ×2 | documentation | Modifier |

---

### Task 1: Le drapeau et la garde `storage`

**Files:**
- Modify: `crates/rbs-cli/src/cli.rs:136-158`, `crates/rbs-cli/src/lib.rs`, `crates/rbs-cli/src/generate/command.rs`, `crates/rbs-cli/src/generate/feature.rs`
- Test: `crates/rbs-cli/src/generate/command.rs` (`mod tests`), `crates/rbs-cli/src/generate/feature.rs` (`mod tests`), `crates/rbs-cli/src/cli.rs` (`mod tests`)

**Interfaces:**
- Consomme : `GenerateArgs` (Task 1 du plan soft-delete), `metadata::Metadata.features` (`metadata.rs:98`), le patron `validate_role` / `Error::RoleSansAuth` (`command.rs:145-152, 345`).
- Produit : `Feature.with_upload: bool` posé par `Feature::with_upload()`, `command::Error::UploadSansStorage`, et la clé template **`with_upload`**.

**Pourquoi la garde vient avant tout écrit :** sans le fragment, `state.storage()` n'existe pas. Le projet engendré ne compilerait plus, et l'utilisateur recevrait une erreur de `rustc` sur du code qu'il n'a pas écrit.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/rbs-cli/src/generate/feature.rs`, `mod tests` :

```rust
    #[test]
    fn an_ordinary_feature_carries_no_upload() {
        let feature = Feature::fresh("articles", Vec::new());
        let rendu = serde_json::to_value(&feature).expect("la feature se sérialise");

        assert_eq!(rendu["with_upload"], false);
    }

    #[test]
    fn uploading_marks_the_feature() {
        let feature = Feature::fresh("articles", Vec::new()).uploading();
        let rendu = serde_json::to_value(&feature).expect("la feature se sérialise");

        assert_eq!(rendu["with_upload"], true);
    }
```

Dans `crates/rbs-cli/src/generate/command.rs`, `mod tests` :

```rust
    #[test]
    fn upload_without_the_storage_feature_names_the_command_that_repairs_it() {
        let message = Error::UploadSansStorage.to_string();

        assert!(
            message.contains("rbs add storage"),
            "un refus qui ne dit pas comment le lever fait chercher : {message}"
        );
        assert!(
            message.contains("storage"),
            "le refus doit nommer la feature attendue : {message}"
        );
    }
```

Dans `crates/rbs-cli/src/cli.rs`, `mod tests` :

```rust
    #[test]
    fn generate_crud_accepts_with_upload() {
        let cli = Cli::try_parse_from(["rbs", "generate", "crud", "articles", "--with-upload"])
            .expect("la ligne doit être acceptée");

        let Commands::Generate {
            command: GenerateCommands::Crud { with_upload, .. },
        } = cli.command
        else {
            panic!("la sous-commande doit être `generate crud`");
        };

        assert!(with_upload);
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli --lib -- upload 2>&1 | tail -20`
Expected: FAIL à la compilation — `no method named 'uploading'`, `no variant named 'UploadSansStorage'`, `has no field named 'with_upload'`.

- [ ] **Step 3: Déclarer le drapeau et le faire descendre**

Dans `crates/rbs-cli/src/cli.rs`, variante `GenerateCommands::Crud`, après `soft_delete` :

```rust
        /// Ajoute trois routes de contenu binaire ; exige la feature storage.
        #[arg(long)]
        with_upload: bool,
```

Dans `crates/rbs-cli/src/lib.rs` : ajouter `with_upload: bool` à `GenerateArgs`, le lire dans le bras `Crud`, poser `false` dans le bras `Feature`, l'ajouter à la destructuration et à la construction d'`Options`.

Dans `crates/rbs-cli/src/generate/command.rs`, à la fin d'`Options` :

```rust
    /// Ajoute au CRUD trois routes de contenu binaire, adossées au fragment `storage`.
    pub with_upload: bool,
```

- [ ] **Step 4: Porter le drapeau sur `Feature`**

Dans `crates/rbs-cli/src/generate/feature.rs`, ajouter le champ :

```rust
    /// Le CRUD porte des routes de contenu binaire.
    pub with_upload: bool,
```

`with_upload: false` dans `Feature::fresh`, puis :

```rust
    /// La même feature, dotée de ses routes de contenu.
    pub(crate) fn uploading(mut self) -> Self {
        self.with_upload = true;
        self
    }
```

Dans l'`impl Serialize`, **passer `serialize_struct("Feature", 11)` à `12`** (la valeur `11` vient du plan soft-delete) et ajouter :

```rust
        state.serialize_field("with_upload", &self.with_upload)?;
```

- [ ] **Step 5: Écrire la garde**

Dans l'`enum Error` de `command.rs`, à côté de `RoleSansAuth` :

```rust
    /// `--with-upload` réclamé sur un projet dépourvu du fragment qui porte le stockage.
    #[error(
        "`--with-upload` exige la feature `storage`, absente de ce projet : lancez \
         `rbs add storage`, puis relancez la génération"
    )]
    UploadSansStorage,
```

Dans `plan_for`, **immédiatement après** le bloc `if let Some(role) = &options.role` (ligne ~217-219) :

```rust
    // Avant tout rendu, comme le garde de rôle : sans le fragment, `state.storage()`
    // n'existe pas et le projet engendré cesserait de compiler — une erreur de rustc sur
    // du code que l'utilisateur n'a pas écrit.
    if options.with_upload && !metadonnees.features.iter().any(|feature| feature == "storage") {
        return Err(Error::UploadSansStorage);
    }
```

Et, là où `Feature` est construite (le bloc que le plan soft-delete a laissé) :

```rust
    let feature = if options.with_upload {
        feature.uploading()
    } else {
        feature
    };
```

- [ ] **Step 6: Lancer les tests, lint, commit**

Run: `cargo test -p rbs-cli --lib && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected: tests verts ; clippy et fmt sans sortie.

```bash
git add crates/rbs-cli/src/cli.rs crates/rbs-cli/src/lib.rs crates/rbs-cli/src/generate/command.rs crates/rbs-cli/src/generate/feature.rs
git commit -F - <<'EOF'
feat(generate): fait descendre un drapeau de dépôt de fichier, et le refuse sans son fragment

Le drapeau se pose sur la feature et devient une clé du contexte des gabarits,
que rien ne lit encore.

Le refus tombe avant tout rendu, comme le garde de rôle et pour la même raison :
sans le fragment de stockage, l'accesseur d'état n'existe pas et le projet
engendré cesserait de compiler. L'utilisateur recevrait une erreur du
compilateur sur du code qu'il n'a pas écrit, là où il reçoit désormais le nom
de la commande qui répare.

Vérifications :
- cargo test -p rbs-cli --lib : 0 échec, dont les quatre tests du drapeau et de la garde
- cargo clippy -p rbs-cli --all-targets -- -D warnings : aucune sortie
- cargo fmt --all --check : aucune sortie
EOF
```

---

### Task 2: Le service — clé dérivée, dépôt, relecture, présence

**Files:**
- Modify: `crates/rbs-cli/templates/feature/service.rs.jinja`
- Test: `crates/rbs-cli/src/generate/service.rs` (`mod tests`)

**Interfaces:**
- Consomme : la clé `with_upload`, le trait `crate::storage::Storage` et `crate::storage::StorageError` du fragment (`templates/features/storage/mod.rs.jinja:17-54`).
- Produit, dans le `service.rs` engendré :
  - `fn content_key(id: Uuid) -> String`
  - `pub async fn put_content(db: &DatabaseConnection, storage: &dyn Storage, id: Uuid, content: Vec<u8>) -> Result<()>`
  - `pub async fn get_content(storage: &dyn Storage, id: Uuid) -> Result<Vec<u8>>`
  - `pub async fn has_content(storage: &dyn Storage, id: Uuid) -> Result<bool>`
  - `pub async fn delete(db, storage: &dyn Storage, id) -> Result<()>` sous le drapeau (un paramètre de plus qu'aujourd'hui)

  La Task 3 appelle ces quatre signatures **telles quelles**.

**Le modèle littéral** est `examples/file-drop/src/uploads/service.rs`, lignes 22-28 (`content_key`) et sa région `contenu` en fin de fichier. Le reprendre, en remplaçant `uploads` par `{@ module @}` et `upload` par `{@ singular @}`.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/rbs-cli/src/generate/service.rs`, `mod tests`, en reprenant le helper `service(name, fields)` du module (ligne 23) :

```rust
    /// Rend le service d'une feature dotée de ses routes de contenu.
    fn service_with_upload(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields).uploading()).expect("le service doit se rendre")
    }

    #[test]
    fn the_key_is_derived_from_the_id() {
        let rendered = service_with_upload("articles", "title:string");

        assert!(
            rendered.contains(r#"format!("articles/{id}")"#),
            "la clé range les objets sous le nom du module, rien d'autre ne les \
             distingue :\n{rendered}"
        );
    }

    #[test]
    fn putting_content_reads_the_row_first() {
        let rendered = service_with_upload("articles", "title:string");
        let put = rendered
            .split("pub async fn put_content")
            .nth(1)
            .expect("put_content doit être rendu");
        let lecture = put.find("repository::find").expect("la ligne doit être lue");
        let depot = put.find("storage").expect("le dépôt doit avoir lieu");

        assert!(
            lecture < depot,
            "sans lecture préalable, le magasin accumulerait des objets qu'aucune \
             ressource ne réclame :\n{put}"
        );
    }

    #[test]
    fn deleting_the_row_removes_its_content() {
        let rendered = service_with_upload("articles", "title:string");
        let delete = rendered
            .split("pub async fn delete")
            .nth(1)
            .expect("delete doit être rendu");

        assert!(
            delete.contains("storage") && delete.contains("content_key(id)"),
            "le contenu part avec la ligne :\n{delete}"
        );
    }

    #[test]
    fn a_missing_object_is_the_only_client_error() {
        let rendered = service_with_upload("articles", "title:string");

        assert!(
            rendered.contains("StorageError::NotFound(_) => Error::NotFound(\"contenu\")"),
            "les autres erreurs du stockage sont des pannes, pas des fautes du \
             client :\n{rendered}"
        );
    }

    #[test]
    fn an_ordinary_service_knows_nothing_of_storage() {
        let rendered = service("articles", "title:string");

        assert!(
            !rendered.contains("storage") && !rendered.contains("content_key"),
            "témoin :\n{rendered}"
        );
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli --lib -- service::tests 2>&1 | tail -20`
Expected: FAIL sur les quatre premiers ; `an_ordinary_service_knows_nothing_of_storage` passe déjà.

- [ ] **Step 3: Étendre la template**

Dans `crates/rbs-cli/templates/feature/service.rs.jinja` :

**(a)** Compléter le bloc `use`, après `use super::repository::{self, ActiveModel};` :

```jinja
{%- if with_upload %}
use crate::storage::{Storage, StorageError};
{%- endif %}
```

**(b)** Ajouter la clé, juste après le bloc `use` :

```jinja
{% if with_upload %}
/// Clé du contenu déposé pour `id`.
///
/// Le stockage est un magasin plat : c'est ce préfixe qui range les objets de cette
/// ressource, et rien d'autre ne les distingue.
fn content_key(id: Uuid) -> String {
    format!("{@ module @}/{id}")
}
{% endif %}
```

**(c)** Remplacer la fonction `delete` en fin de fichier par :

```jinja
{%- if with_upload %}
{@ entete("delete", ["db: &DatabaseConnection", "storage: &dyn Storage", "id: Uuid"], "Result<()>") @}
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("{@ singular @}"));
    }

    // Le contenu part avec la ligne. `delete` est idempotent des deux côtés du trait :
    // une ressource créée sans contenu ne fait donc pas échouer sa suppression.
    storage
        .delete(&content_key(id))
        .await
        .map_err(|error| Error::Internal(anyhow::anyhow!("{error}")))
}

/// Dépose le contenu de `id`, la ressource devant exister.
{@ entete("put_content", ["db: &DatabaseConnection", "storage: &dyn Storage", "id: Uuid", "content: Vec<u8>"], "Result<()>") @}
    // La ligne est lue avant le dépôt : sans elle, le magasin accumulerait des objets
    // qu'aucune ressource ne réclame.
    repository::find(db, id)
        .await?
        .ok_or(Error::NotFound("{@ singular @}"))?;

    storage
        .put(&content_key(id), content)
        .await
        .map_err(|error| Error::Internal(anyhow::anyhow!("{error}")))
}

/// Un contenu est-il déposé pour `id` ?
///
/// `exists` plutôt qu'un `get` dont on jetterait le corps : la question ne demande pas de
/// transférer l'objet, et les deux backends savent y répondre sans le lire.
pub async fn has_content(storage: &dyn Storage, id: Uuid) -> Result<bool> {
    storage
        .exists(&content_key(id))
        .await
        .map_err(|error| Error::Internal(anyhow::anyhow!("{error}")))
}

/// Rend le contenu déposé pour `id`.
pub async fn get_content(storage: &dyn Storage, id: Uuid) -> Result<Vec<u8>> {
    storage
        .get(&content_key(id))
        .await
        // `NotFound` est le seul cas qui vienne du client : les autres sont des pannes.
        .map_err(|error| match error {
            StorageError::NotFound(_) => Error::NotFound("contenu"),
            autre => Error::Internal(anyhow::anyhow!("{autre}")),
        })
}
{%- else %}
pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    if !repository::delete(db, id).await? {
        return Err(Error::NotFound("{@ singular @}"));
    }

    Ok(())
}
{%- endif %}
```

**Attention aux blancs** : `-%}` mange l'indentation, et un blanc perdu n'est vu que par `integration_examples`. Vérifier le rendu exact à l'étape suivante.

- [ ] **Step 4: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-cli --lib -- service::tests`
Expected: PASS.

- [ ] **Step 5: Vérifier la conformité rustfmt**

Run: `cargo test -p rbs-cli --lib -- service::tests::the_render_is_already_what_rustfmt_would_write --exact`
Expected: PASS, la plage de divergence inchangée. Le service passe de 118 à ~180 lignes engendrées sous le drapeau — sous le seuil de 200 qui signalerait une feature à scinder, mais tout juste.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/feature/service.rs.jinja crates/rbs-cli/src/generate/service.rs
git commit -F - <<'EOF'
feat(generate): engendre le service de dépôt et de relecture d'un contenu

La clé est dérivée de l'identifiant plutôt que rangée en colonne : une colonne
la dupliquerait sans jamais en diverger. Le dépôt lit la ligne d'abord, sans
quoi le magasin accumulerait des objets qu'aucune ressource ne réclame, et la
suppression emporte le contenu.

Des cinq erreurs du trait, une seule vient du client — l'objet absent. Les
autres sont des pannes et remontent en interne : les confondre ferait répondre
404 sur un bucket injoignable.

Le code reprend celui qu'un exemple du dépôt portait à la main, paramétré par
le nom de l'entité.

Vérifications :
- cargo test -p rbs-cli --lib -- service::tests : 0 échec
- le rendu reste conforme à ce que rustfmt écrirait
EOF
```

---

### Task 3: Le contrôleur, les routes et l'inscription à l'OpenAPI

**Files:**
- Modify: `crates/rbs-cli/templates/feature/controller.rs.jinja`
- Modify: `crates/rbs-cli/templates/feature/mod.rs.jinja`
- Modify: `crates/rbs-cli/src/generate/mount.rs:12, 27-50`
- Test: `crates/rbs-cli/src/generate/controller.rs` (`mod tests`), `crates/rbs-cli/src/generate/mount.rs` (`mod tests`)

**Interfaces:**
- Consomme : les quatre signatures de service de la Task 2, la clé `with_upload`.
- Produit : trois handlers, trois routes, et neuf lignes à l'ancre `openapi` au lieu de six.

**Le point qu'aucune compilation ne signale :** `mount.rs:12` porte `const HANDLERS: [&str; 6]`, la liste des handlers inscrits à l'ancre `openapi`. Les trois nouveaux doivent y figurer, faute de quoi les routes existeraient sans apparaître au document — et rien ne le dirait.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/rbs-cli/src/generate/mount.rs`, `mod tests` :

```rust
    #[test]
    fn the_three_content_handlers_reach_the_openapi_anchor() {
        let mounts = pour("articles", anchors::FEATURES, true);
        let openapi = mounts
            .iter()
            .find(|mount| mount.anchor == anchors::OPENAPI)
            .expect("l'ancre openapi doit être visée");

        assert_eq!(
            openapi.lines.len(),
            9,
            "trois handlers de plus ; sans eux les routes existent hors du document, \
             et rien ne le signale : {:?}",
            openapi.lines
        );
        assert!(
            openapi
                .lines
                .iter()
                .any(|line| line.contains("controller::put_content")),
            "{:?}",
            openapi.lines
        );
    }

    #[test]
    fn an_ordinary_feature_mounts_six_handlers() {
        let mounts = pour("articles", anchors::FEATURES, false);
        let openapi = mounts
            .iter()
            .find(|mount| mount.anchor == anchors::OPENAPI)
            .expect("l'ancre openapi doit être visée");

        assert_eq!(openapi.lines.len(), 6, "témoin : {:?}", openapi.lines);
    }
```

Dans `crates/rbs-cli/src/generate/controller.rs`, `mod tests` :

```rust
    /// Rend le contrôleur d'une feature dotée de ses routes de contenu.
    fn controller_with_upload(name: &str, fields: &str) -> String {
        let fields = fields::parse(fields).expect("les champs du test doivent être valides");
        render(&Feature::fresh(name, fields).uploading()).expect("le contrôleur doit se rendre")
    }

    #[test]
    fn the_three_content_handlers_are_rendered() {
        let rendered = controller_with_upload("articles", "title:string");

        for handler in ["put_content", "get_content", "head_content"] {
            assert!(
                rendered.contains(&format!("pub async fn {handler}")),
                "{handler} manque :\n{rendered}"
            );
        }
        assert!(
            rendered.contains("content: Bytes"),
            "le corps voyage brut : en JSON il passerait en base64, donc deux fois en \
             mémoire :\n{rendered}"
        );
    }

    #[test]
    fn the_content_routes_are_mounted() {
        let fields = fields::parse("title:string").expect("champs");
        let rendered = render_mod(&Feature::fresh("articles", fields).uploading(), false)
            .expect("le mod doit se rendre");

        assert!(
            rendered.contains(r#""/articles/{id}/content""#),
            "les trois routes partagent un chemin :\n{rendered}"
        );
        assert!(
            rendered.contains("put(controller::put_content)")
                && rendered.contains("get(controller::get_content)")
                && rendered.contains("head(controller::head_content)"),
            "{rendered}"
        );
    }

    #[test]
    fn the_upload_route_alone_raises_the_body_limit() {
        let fields = fields::parse("title:string").expect("champs");
        let rendered = render_mod(&Feature::fresh("articles", fields).uploading(), false)
            .expect("le mod doit se rendre");

        assert_eq!(
            rendered.matches("DefaultBodyLimit::max").count(),
            1,
            "posée sur le routeur, la limite relèverait aussi celle des routes JSON, \
             qu'aucun besoin ne justifie :\n{rendered}"
        );
        assert!(rendered.contains("const TAILLE_MAX"), "{rendered}");
    }

    #[test]
    fn an_ordinary_module_mounts_no_content_route() {
        let fields = fields::parse("title:string").expect("champs");
        let rendered =
            render_mod(&Feature::fresh("articles", fields), false).expect("le mod doit se rendre");

        assert!(
            !rendered.contains("content") && !rendered.contains("DefaultBodyLimit"),
            "témoin :\n{rendered}"
        );
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli --lib -- mount::tests controller::tests 2>&1 | tail -20`
Expected: FAIL — `pour` prend deux arguments, les handlers n'existent pas.

- [ ] **Step 3: Rendre `HANDLERS` conditionnel**

Dans `crates/rbs-cli/src/generate/mount.rs`, remplacer la ligne 12 et ajuster `pour` :

```rust
/// Les handlers que le controller généré expose, dans l'ordre où ils y sont écrits.
const HANDLERS: [&str; 6] = ["list", "filter", "create", "find", "update", "delete"];

/// Les trois handlers que `--with-upload` ajoute, dans le même ordre.
///
/// Ils s'inscrivent à l'ancre `openapi` comme les six autres : montées sans y figurer,
/// les routes existeraient hors du document, et aucune compilation ne le dirait.
const HANDLERS_CONTENU: [&str; 3] = ["put_content", "get_content", "head_content"];
```

Dans `pour(module: &str, features: Anchor)`, ajouter un troisième paramètre `with_upload: bool` et remplacer la construction du `Mount` de l'ancre `OPENAPI` par :

```rust
        Mount {
            anchor: anchors::OPENAPI,
            lines: HANDLERS
                .iter()
                .chain(if with_upload {
                    HANDLERS_CONTENU.iter()
                } else {
                    [].iter()
                })
                .map(|handler| format!("crate::{module}::controller::{handler},"))
                .collect(),
        },
```

Si le typage du `chain` résiste, écrire à la place :

```rust
            lines: HANDLERS
                .iter()
                .chain(HANDLERS_CONTENU.iter().take(if with_upload { 3 } else { 0 }))
                .map(|handler| format!("crate::{module}::controller::{handler},"))
                .collect(),
```

Répercuter le nouvel argument sur l'appelant, dans `command.rs` (autour de la ligne 296) : `mount::pour(&module, features, options.with_upload)`.

- [ ] **Step 4: Écrire les trois handlers**

Dans `crates/rbs-cli/templates/feature/controller.rs.jinja`, compléter le bloc `use` :

```jinja
{%- if with_upload %}
use axum::body::Bytes;
use axum::response::IntoResponse;
{%- endif %}
```

et ajouter en fin de fichier, après `delete` :

```jinja
{% if with_upload %}
// Le contenu voyage hors du DTO : un corps binaire n'a pas sa place dans un JSON, et le
// faire passer en base64 obligerait à charger deux fois le fichier en mémoire.
#[utoipa::path(
    put,
    path = "/{@ module @}/{id}/content",
    tag = "{@ module @}",
    params(("id" = Uuid, Path, description = "identifiant de {@ singular @}")),
    request_body(content = String, content_type = "application/octet-stream"),
    responses(
        (status = 204, description = "contenu déposé"),
        (status = 404, description = "{@ singular @} introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn put_content(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    content: Bytes,
) -> Result<StatusCode> {
    service::put_content(
        state.core().db(),
        state.storage().as_ref(),
        id,
        content.to_vec(),
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/{@ module @}/{id}/content",
    tag = "{@ module @}",
    params(("id" = Uuid, Path, description = "identifiant de {@ singular @}")),
    responses(
        (status = 200, description = "contenu du {@ singular @}", content_type = "application/octet-stream"),
        (status = 404, description = "contenu introuvable", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn get_content(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let content = service::get_content(state.storage().as_ref(), id).await?;

    Ok(([("content-type", "application/octet-stream")], content))
}

#[utoipa::path(
    head,
    path = "/{@ module @}/{id}/content",
    tag = "{@ module @}",
    params(("id" = Uuid, Path, description = "identifiant de {@ singular @}")),
    responses(
        (status = 204, description = "un contenu est déposé"),
        (status = 404, description = "aucun contenu", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn head_content(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    if service::has_content(state.storage().as_ref(), id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(rbs_core::Error::NotFound("contenu"))
    }
}
{% endif %}
```

**Note :** le `delete` engendré appelle désormais `service::delete(state.core().db(), state.storage().as_ref(), id)` sous le drapeau — ajuster son corps de la même manière, avec un `{% if with_upload %}`.

- [ ] **Step 5: Monter les routes**

Dans `crates/rbs-cli/templates/feature/mod.rs.jinja`, compléter le bloc `use` et ajouter la constante :

```jinja
{%- if with_upload %}
use axum::extract::DefaultBodyLimit;
use axum::routing::head;

/// Taille maximale d'un contenu déposé. Relevez-la si vos fichiers sont plus gros ; elle
/// ne vaut que pour la route de dépôt, les routes JSON gardant la limite d'axum.
const TAILLE_MAX: usize = 10 * 1024 * 1024;
{%- endif %}
```

et, à la fin de la chaîne de `routes()`, après la route `/{@ module @}/{id}` :

```jinja
{%- if with_upload %}
        .route(
            "/{@ module @}/{id}/content",
            put(controller::put_content)
                .get(controller::get_content)
                .head(controller::head_content)
                .layer(DefaultBodyLimit::max(TAILLE_MAX)),
        )
{%- endif %}
```

Compléter l'import `use axum::routing::{get, post};` en `{get, post, put}` sous le drapeau.

**Point de contrôle `UndefinedBehavior::Strict` :** `mod.rs` passe par `controller::render_mod` (`controller.rs:28-33`), dont le contexte est `context! { with_tests, ..Value::from_serialize(feature) }`. La clé `with_upload` y arrive par la sérialisation de `Feature` — le test `the_content_routes_are_mounted` du Step 1 échouerait au rendu si ce n'était pas le cas.

- [ ] **Step 6: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-cli --lib -- mount::tests controller::tests`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/rbs-cli/templates/feature/controller.rs.jinja crates/rbs-cli/templates/feature/mod.rs.jinja crates/rbs-cli/src/generate/mount.rs crates/rbs-cli/src/generate/controller.rs crates/rbs-cli/src/generate/command.rs
git commit -F - <<'EOF'
feat(generate): monte les trois routes de contenu et les inscrit au document

Le corps voyage en octet-stream et non en JSON : en base64 il passerait deux
fois en mémoire. La borne de taille est posée sur la seule route de dépôt —
sur le routeur, elle relèverait aussi celle des routes JSON, qu'aucun besoin
ne justifie.

Les trois handlers rejoignent la liste que le montage inscrit à l'ancre du
document OpenAPI. C'est le point qu'on oublie : montées sans y figurer, les
routes existeraient hors du document et aucune compilation ne le dirait.

Vérifications :
- cargo test -p rbs-cli --lib -- mount::tests controller::tests : 0 échec
EOF
```

---

### Task 4: Le banc — le code engendré s'accorde vraiment au trait

**Files:**
- Create: un test `#[ignore]` dans `crates/rbs-cli/tests/integration_crud.rs`

**Interfaces:**
- Consomme : les Tasks 1 à 3 ; `bench::Project::fresh()`, `Project::rbs_ok`, `Project::compile()`, `Project::clippy()` (`bench.rs:137, 341, 443, 464`).
- Produit : la seule preuve que les appels engendrés satisfont les cinq méthodes du trait `Storage`. Les tests unitaires comparent des chaînes.

**Nom de feature :** `attachments`, jamais `uploads` — `file-drop` porte déjà ce nom et les projets de banc partagent `target/rbs-integration`.

- [ ] **Step 1: Écrire le test**

```rust
/// Le CRUD à routes de contenu compile contre le trait que le fragment installe.
///
/// Les tests unitaires comparent des chaînes de caractères : seul ce banc dit si les
/// appels engendrés satisfont les cinq méthodes de `Storage`.
#[tokio::test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
async fn an_uploading_crud_compiles_against_the_storage_trait() {
    let projet = bench::Project::fresh();

    projet.rbs_ok(&["add", "storage", "--force"]);
    projet.rbs_ok(&[
        "generate",
        "crud",
        "attachments",
        "--fields",
        "title:string",
        "--with-upload",
        "--force",
    ]);

    projet.compile();
    projet.clippy();
}
```

Adapter les noms de helpers à ce que `bench.rs` expose ; le test voisin de `integration_crud.rs:23` en est le modèle.

- [ ] **Step 2: Écrire le test de la garde**

Dans le même fichier, un test **sans compilation** — donc rapide et non `#[ignore]` s'il tient sans Docker :

```rust
/// `--with-upload` sur un projet sans le fragment refuse en nommant la commande qui répare.
#[tokio::test]
#[ignore = "engendre un projet complet"]
async fn uploading_without_the_storage_feature_is_refused() {
    let projet = bench::Project::fresh();

    let sortie = projet.rbs(&[
        "generate",
        "crud",
        "attachments",
        "--fields",
        "title:string",
        "--with-upload",
        "--force",
    ]);

    assert!(!sortie.status.success(), "le refus doit être un échec");
    let texte = String::from_utf8_lossy(&sortie.stderr);
    assert!(
        texte.contains("rbs add storage"),
        "le refus doit nommer la commande qui répare : {texte}"
    );
    assert!(
        !projet.path().join("src/attachments").exists(),
        "le refus tombe avant tout écrit"
    );
}
```

- [ ] **Step 3: Lancer les bancs**

Run: `cargo test -p rbs-cli --test integration_crud -- --ignored --no-fail-fast upload > /private/tmp/claude-501/-Users-yacoubakone-dev-rs/47075687-159f-44e4-8a83-b1b3a396f17f/scratchpad/banc-upload.txt 2>&1; echo "code $?"; tail -30 /private/tmp/claude-501/-Users-yacoubakone-dev-rs/47075687-159f-44e4-8a83-b1b3a396f17f/scratchpad/banc-upload.txt`
Expected: code 0, deux bancs verts.

- [ ] **Step 4: Commit**

```bash
git add crates/rbs-cli/tests/integration_crud.rs
git commit -F - <<'EOF'
test(generate): éprouve le CRUD à routes de contenu contre le trait du fragment

Les tests unitaires comparent des chaînes ; seul un banc dit si les appels
engendrés satisfont les cinq méthodes du trait de stockage, et si le tout passe
clippy sans avertissement.

Le second banc prouve que le refus tombe avant tout écrit : le répertoire de la
feature n'existe pas après un refus.

Vérifications :
- cargo test -p rbs-cli --test integration_crud -- --ignored --no-fail-fast upload : 2 passés
EOF
```

---

### Task 5: `file-drop` cesse d'être écrit à la main

**Files:**
- Modify: `examples/file-drop/**` (régénéré)
- Modify: `crates/rbs-cli/tests/integration_examples.rs:57-81` (la commande) et `:230-235` (`edite_a_la_main`)
- Modify: `examples/README.md`, `examples/README.fr.md`

**Interfaces:** aucune.

**Pourquoi c'est le meilleur test du lot, et gratuit :** `file-drop` porte les trois handlers à la main, suivis par des marqueurs `region:` et exclus de la comparaison octet à octet. Les faire engendrer transforme `integration_examples` en preuve que l'engendré vaut l'écrit à la main — sur du code qu'un humain a jugé bon.

**Attention :** régénérer **par diff entre deux générations, jamais par écrasement**. Le test de non-dérive est l'oracle.

- [ ] **Step 1: Ajouter le drapeau à la commande de référence**

Dans `crates/rbs-cli/tests/integration_examples.rs`, entrée `file-drop` (lignes 57-81), ajouter `--with-upload` aux arguments de `generate crud`.

- [ ] **Step 2: Voir la dérive**

Run: `cargo test -p rbs-cli --test integration_examples -- --include-ignored file_drop 2>&1 | tail -40`
Expected: **FAIL**, avec le diff exact entre ce que le CLI produit désormais et ce que le dépôt versionne. C'est ce diff qui pilote l'étape suivante.

- [ ] **Step 3: Reporter le diff dans l'exemple**

Appliquer à `examples/file-drop/` les écarts que le test signale, et **seulement** eux. Attendus : les trois handlers passent de la forme écrite à la main à la forme engendrée, les marqueurs `region: put_content` / `region: head_content` / `region: contenu` disparaissent avec le code qu'ils délimitaient, `mod.rs` gagne sa route et sa constante.

Si un écart n'était pas attendu, **c'est une template à corriger, pas l'exemple** : le `service.rs` de `file-drop` orchestre aussi le cache et le mail, que la template n'engendre pas. Ces parties-là restent des éditions manuelles.

- [ ] **Step 4: Réduire `edite_a_la_main`**

Dans `integration_examples.rs:230-235`, retirer de l'entrée `file-drop` les fichiers qui ne portent plus d'édition manuelle. Les tests dédiés (`the_hand_edits_of_file_drop_are_in_place`) doivent suivre.

- [ ] **Step 5: Vérifier la non-dérive**

Run: `cargo test -p rbs-cli --test integration_examples -- --include-ignored 2>&1 | tail -10`
Expected: 17 passés, aucune dérive.

- [ ] **Step 6: Mettre à jour les deux README**

Dans `examples/README.md` et `examples/README.fr.md` : la commande de régénération de `file-drop` gagne `--with-upload` ; le paragraphe décrivant ses trois handlers écrits à la main (`README.md:148`) dit désormais qu'ils sont engendrés ; la liste de ses éditions manuelles se réduit d'autant.

- [ ] **Step 7: Commit**

```bash
git add examples/ crates/rbs-cli/tests/integration_examples.rs
git commit -F - <<'EOF'
refactor(examples): fait engendrer les routes de contenu que file-drop écrivait à la main

Les trois handlers étaient écrits à la main, suivis par des marqueurs de région
et exclus de la comparaison octet à octet. Engendrés, ils entrent dans le champ
du contrôle de non-dérive : celui-ci prouve désormais que la template vaut le
code qu'un humain avait jugé bon, sur le projet même qui en avait établi la
forme.

Le reste des éditions manuelles demeure — le service y orchestre aussi le cache
et le courriel, qu'aucune template n'engendre.

Vérifications :
- cargo test -p rbs-cli --test integration_examples -- --include-ignored : 17 passés, aucune dérive
EOF
```

---

### Task 6: Documentation et vérification de bout en bout

**Files:**
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md`, le guide du stockage et sa paire française

- [ ] **Step 1: Écrire les deux notes de CHANGELOG**

Dans `CHANGELOG.md`, `[Unreleased] / Added` :

```markdown
- `rbs generate crud --with-upload` mounts three content routes on the generated
  resource — `PUT`, `GET` and `HEAD` on `/<resource>/{id}/content` — backed by the
  `storage` fragment's trait. The body travels as `application/octet-stream`, not JSON:
  base64 would hold the file in memory twice. The storage key is derived from the `id`,
  so no column carries it. Without the `storage` feature the flag is refused before
  anything is written, naming `rbs add storage`. A body limit applies to the upload route
  alone, as a constant you can raise.
```

Dans `CHANGELOG.fr.md`, la note correspondante.

- [ ] **Step 2: Écrire les deux sections de guide**

Dans le guide du stockage et sa paire française : ce que le drapeau monte, la forme du corps et pourquoi, la clé dérivée, la borne de taille et où la changer, la garde. Dire aussi ce qu'il ne fait pas — un fichier par ligne, pas de table de pièces jointes, aucun filtrage de type MIME.

- [ ] **Step 3: Parité**

Run: `cd docs && node scripts/parite.mjs`
Expected: exit 0. La paire `examples/README.md` / `examples/README.fr.md` fait partie des jeux contrôlés depuis la tâche 72.

- [ ] **Step 4: Suite complète**

Run:
```bash
cargo test --workspace 2>&1 | tail -20
cargo test --workspace -- --ignored --no-fail-fast > /private/tmp/claude-501/-Users-yacoubakone-dev-rs/47075687-159f-44e4-8a83-b1b3a396f17f/scratchpad/suite-lente-upload.txt 2>&1; echo "code $?"; grep -E "^test result|FAILED" /private/tmp/claude-501/-Users-yacoubakone-dev-rs/47075687-159f-44e4-8a83-b1b3a396f17f/scratchpad/suite-lente-upload.txt
cargo test -p rbs-cli --test integration_examples -- --include-ignored 2>&1 | tail -10
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```
Expected: 0 échec partout, code 0 sur la suite lente, 17 passés sans dérive, clippy et fmt sans sortie.

- [ ] **Step 5: Commit et cochage**

```bash
git add CHANGELOG.md CHANGELOG.fr.md docs/
git commit -F - <<'EOF'
docs(upload): documente les routes de contenu et ce qu'elles ne couvrent pas

La page dit la forme du corps et pourquoi elle n'est pas du JSON, où se change
la borne de taille, et ce que le drapeau ne fait pas : un fichier par ligne,
aucune table de pièces jointes, aucun filtrage de type. Le code engendré est
fait pour être modifié, et une liste blanche devinée serait fausse pour tout le
monde.

Vérifications :
- node docs/scripts/parite.mjs : exit 0
EOF
```

Puis, dans `IMPROVE.md`, cocher la ligne 78 avec ` — Fait le 2026-09-02 : ` et les **chiffres réels** du Step 4.

---

## Self-Review

**Couverture de la spec :**

| Section de la spec | Tâche |
|---|---|
| Corps brut, pas de multipart | Task 3, test `the_three_content_handlers_are_rendered` |
| Aucune colonne injectée | aucune tâche n'en ajoute — c'est la décision |
| La garde `storage` | Task 1, Step 5 ; Task 4, Step 2 (avant tout écrit) |
| Les trois routes et leur tableau de réponses | Task 3, Step 4 |
| `DefaultBodyLimit` sur la seule route de dépôt | Task 3, test `the_upload_route_alone_raises_the_body_limit` |
| Aucun filtrage MIME | aucune tâche n'en ajoute ; dit dans la doc, Task 6 |
| Clé dérivée, non stockée | Task 2, test `the_key_is_derived_from_the_id` |
| Ce que chaque couche reçoit | Tasks 2 et 3 ; repository, dto, filter, migration, model intouchés |
| Traduction des erreurs du trait | Task 2, test `a_missing_object_is_the_only_client_error` |
| `mount.rs` HANDLERS | Task 3, Steps 1 et 3 |
| Les huit tests unitaires nommés | Tasks 1 à 3 |
| Banc de compilation | Task 4 |
| `file-drop` régénéré, `edite_a_la_main` réduit | Task 5 |
| CHANGELOG ×2, README ×2, guide ×2 | Tasks 5 et 6 |

**Un écart avec la spec, découvert en écrivant :** la spec listait `tests_http.rs:36-56` parmi les contextes à qui passer `with_upload`. Aucune tâche ne le fait — les scénarios HTTP engendrés ne couvrent pas l'aller-retour de contenu, qui exigerait un backend de stockage monté dans le harnais de test du projet engendré. **C'est un trou assumé** : la compilation et clippy sont prouvés par le banc de la Task 4, l'exécution des routes ne l'est par rien. À dire dans le cochage d'`IMPROVE.md`, en `PARTIEL` sur ce point plutôt qu'en silence.

**Cohérence des types :** les quatre signatures de service (Task 2) sont appelées littéralement par les handlers de la Task 3 — `service::put_content(db, storage, id, content)`, `service::get_content(storage, id)`, `service::has_content(storage, id)`, `service::delete(db, storage, id)`. `Feature::uploading()` → clé `with_upload` → lue par `service.rs.jinja`, `controller.rs.jinja`, `mod.rs.jinja`. `mount::pour(module, features, with_upload)` : le troisième paramètre est ajouté en Task 3 et répercuté sur son unique appelant dans `command.rs`.
