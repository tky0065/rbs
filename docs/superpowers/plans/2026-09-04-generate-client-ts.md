# `rbs generate client --lang ts` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs generate client --lang ts` écrit dans le projet un client TypeScript typé,
engendré depuis le document OpenAPI que le projet imprime lui-même.

**Architecture:** La template du projet gagne un binaire `openapi` qui imprime
`ApiDoc::openapi()` en JSON. La commande le lance par `cargo run --bin openapi`, lit sa
sortie, la traduit en TypeScript, et écrit le résultat par le rituel plan → afficher →
appliquer de `crates/rbs-cli/src/plan/`. L'invariant du client vit dans une template
minijinja ; Rust ne calcule que les interfaces et les méthodes.

**Tech Stack:** Rust 2024, clap, serde_json (déjà au manifeste de `rbs-cli`), minijinja
à délimiteurs `{@ @}`, utoipa 5 côté projet engendré.

**Spec:** `docs/superpowers/specs/2026-09-04-generate-client-ts-design.md`

## Global Constraints

- **Commits** : Conventional Commits, sujet en français à l'impératif, sans majuscule ni
  point final. **Jamais de `Co-Authored-By`, jamais de `Claude-Session`**, aucune mention
  d'un assistant. Le corps porte le pourquoi et un intertitre `Vérifications :` avec les
  commandes lancées et leur résultat réel.
- **Commentaires** : le *pourquoi*, jamais le *quoi*. Un commentaire qui paraphrase la
  ligne suivante se supprime. `#![warn(missing_docs)]` ne vise que `rbs-core` ; dans
  `rbs-cli`, les items `pub(crate)` portent tout de même un `///` court, c'est la
  convention du module voisin.
- **minijinja** : délimiteurs de variables `{@ @}`, blocs `{% %}`. `-%}` mange
  l'indentation — un blanc perdu n'est vu que par `integration_examples`.
- **Bloquant en CI** : `cargo clippy --workspace --all-targets -- -D warnings` et
  `cargo fmt --all --check`.
- **Ne pas toucher à `IMPROVE.md`.** Le mainteneur coche lui-même.
- **Documentation bilingue** : toute page anglaise modifiée l'est aussi en français, dans
  le même commit.
- Redirections de sortie longue : préfixer tout fichier de scratchpad par `client-ts-`.

---

## Structure des fichiers

**Créés :**

| Fichier | Responsabilité |
|---|---|
| `crates/rbs-cli/templates/project/src/bin/openapi.rs.jinja` | le binaire qui imprime le document |
| `crates/rbs-cli/src/client/mod.rs` | la commande : options, erreurs, plan, lancement de cargo |
| `crates/rbs-cli/src/client/document.rs` | le document OpenAPI lu en modèle serde |
| `crates/rbs-cli/src/client/ts.rs` | schémas et opérations → TypeScript |
| `crates/rbs-cli/templates/client/ts/client.ts.jinja` | l'invariant du client |
| `crates/rbs-cli/tests/integration_client.rs` | la commande, de bout en bout |
| `examples/hello-crud/clients/ts/client.ts` | la source des extraits de la documentation |
| `docs/docs/cli/client.md` + sa traduction | la page de la commande |

**Modifiés :**

| Fichier | Changement |
|---|---|
| `crates/rbs-cli/templates/project/Cargo.toml.jinja` | la section `[[bin]] openapi` |
| `crates/rbs-cli/templates/project/src/health/controller.rs.jinja` | `tag = "health"` |
| `crates/rbs-cli/templates/feature/controller.rs.jinja` | `operation_id` sur cinq handlers |
| `crates/rbs-cli/src/cli.rs` | la sous-commande `Client` |
| `crates/rbs-cli/src/lib.rs` | `mod client;` et l'aiguillage |
| `crates/rbs-cli/src/generate/controller.rs` | tests de rendu de l'`operation_id` |
| `crates/rbs-cli/tests/integration_examples.rs` | le champ d'exclusion et son test |
| les quatre `examples/*` | le binaire `openapi`, l'`operation_id`, le `tag` |
| `examples/README.md` | la commande qui régénère le client |
| `CHANGELOG.md`, `CHANGELOG.fr.md` | section `## [Unreleased]` |

---

### Task 1 : le binaire `openapi` dans la template de projet

**Files:**
- Create: `crates/rbs-cli/templates/project/src/bin/openapi.rs.jinja`
- Modify: `crates/rbs-cli/templates/project/Cargo.toml.jinja:9-24`
- Test: `crates/rbs-cli/src/new.rs` (module `tests` en fin de fichier)

**Interfaces:**
- Consumes: rien.
- Produces: tout projet engendré porte `src/bin/openapi.rs` et une section
  `[[bin]] name = "openapi" path = "src/bin/openapi.rs"`. La tâche 6 en dépend.

- [ ] **Step 1 : écrire le test qui échoue**

Dans le module `tests` de `crates/rbs-cli/src/new.rs`, à côté des tests existants
(chercher `fn the_manifest_declares` ou le test qui lit `manifest.contains("[[bin]]")`,
vers la ligne 663) :

```rust
#[test]
fn the_generated_project_carries_a_binary_that_prints_the_openapi_document() {
    let project = fixtures::Project::new().create();

    let binaire = fs::read_to_string(project.root.join("src/bin/openapi.rs"))
        .expect("le binaire openapi doit être engendré");

    // Le nom de crate, et non `crate::` : un binaire séparé atteint `ApiDoc` par la
    // bibliothèque du projet, dont Cargo a remplacé les tirets par des soulignés.
    assert!(
        binaire.contains("demo_api::openapi::ApiDoc::openapi()"),
        "{binaire}"
    );
    assert!(binaire.contains("to_pretty_json"), "{binaire}");

    let manifeste = fs::read_to_string(project.root.join("Cargo.toml")).expect("manifeste");
    assert!(
        manifeste.contains("name = \"openapi\""),
        "le manifeste doit déclarer le binaire :\n{manifeste}"
    );
    assert!(
        manifeste.contains("path = \"src/bin/openapi.rs\""),
        "{manifeste}"
    );
}
```

Si le nom du constructeur de fixture diffère, reprendre celui qu'emploie le test voisin
dans le même module — ne pas inventer une API de fixture.

- [ ] **Step 2 : lancer le test et vérifier qu'il échoue**

```bash
cargo test -p rbs-cli --lib new::tests::the_generated_project_carries_a_binary_that_prints_the_openapi_document
```

Attendu : FAIL, `le binaire openapi doit être engendré`.

- [ ] **Step 3 : écrire la template du binaire**

`crates/rbs-cli/templates/project/src/bin/openapi.rs.jinja` :

```rust
//! Imprime le document OpenAPI du projet, sans démarrer de serveur.
//!
//! `rbs generate client` lit cette sortie. Elle sert aussi à figer le contrat en CI :
//! `cargo run --bin openapi > openapi.json` puis un `git diff` qui doit rester vide.

use utoipa::OpenApi;

fn main() -> Result<(), serde_json::Error> {
    println!("{}", {@ crate_name @}::openapi::ApiDoc::openapi().to_pretty_json()?);

    Ok(())
}
```

`crate_name` est la variable que `src/main.rs.jinja` emploie déjà : le nom du paquet, les
tirets remplacés par des soulignés.

- [ ] **Step 4 : déclarer le binaire au manifeste**

Dans `crates/rbs-cli/templates/project/Cargo.toml.jinja`, après la section `[[bin]]` de
`seed` (lignes 22-24), ajouter :

```toml

[[bin]]
name = "openapi"
path = "src/bin/openapi.rs"
```

et corriger le commentaire de `default-run`, ligne 9, qui dit « deux binaires » :

```toml
# Le paquet porte trois binaires : sans ce choix, `cargo run` ne saurait plus lequel lancer.
```

- [ ] **Step 5 : lancer les tests**

```bash
cargo test -p rbs-cli --lib new::
```

Attendu : PASS, y compris le test ajouté. Si un test existant comptait les fichiers d'un
projet neuf (chercher `assert_eq!` sur une longueur dans `new.rs`), le mettre à jour et
noter le nouveau nombre dans le message de commit.

- [ ] **Step 6 : prouver que le binaire compile et imprime**

Le rendu de la template n'est pas sa compilation. Sur `examples/hello-crud` :

```bash
cp -R examples/hello-crud /tmp/client-ts-t1 && cd /tmp/client-ts-t1
```

y déposer à la main le `src/bin/openapi.rs` et la section `[[bin]]` que la template
produit (nom de crate `hello_crud`), corriger le chemin `../../crates/rbs-core` en chemin
absolu, puis :

```bash
cargo run --bin openapi | head -20
```

Attendu : du JSON commençant par `{"openapi": "3.1.0"`, exit 0. Reporter la taille
(`| wc -c`) dans le message de commit. Ne pas commiter `/tmp/client-ts-t1`.

- [ ] **Step 7 : fmt, clippy, commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/templates/project crates/rbs-cli/src/new.rs
git commit
```

Message (adapter les chiffres aux résultats réels) :

```
feat(new): imprime le document OpenAPI depuis un binaire du projet

Le contrat était déjà écrit dans le code — un `#[utoipa::path]` par
handler — et ne se lisait qu'en démarrant le serveur. Un troisième
binaire le rend sans base ni port, ce qui le met à portée d'un
générateur de client comme d'un `git diff` de CI.

Vérifications :
`cargo test -p rbs-cli --lib new::` : N passés, 0 échoué.
`cargo run --bin openapi` sur une copie d'examples/hello-crud : exit 0,
N octets de JSON.
```

---

### Task 2 : un `operationId` unique, et le tag du handler de santé

**Files:**
- Modify: `crates/rbs-cli/templates/feature/controller.rs.jinja` (les cinq blocs `#[utoipa::path(`)
- Modify: `crates/rbs-cli/templates/project/src/health/controller.rs.jinja`
- Test: `crates/rbs-cli/src/generate/controller.rs` (module `tests`)

**Interfaces:**
- Consumes: rien.
- Produces: le document d'un projet à deux features CRUD porte des `operationId`
  distincts, de la forme `<module>_<action>` : `articles_list`, `articles_create`,
  `articles_find`, `articles_update`, `articles_delete`. La tâche 5 en dépend pour ses
  noms de méthodes.

**Pourquoi :** utoipa prend le nom nu de la fonction handler faute de `operation_id =`.
Deux features CRUD dans un même projet produisent donc deux opérations d'`operationId`
`list`, ce que la spécification OpenAPI interdit. Relevé le 2026-09-04 sur la sortie
réelle de `cargo run --bin openapi` : `"operationId": "list"`.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans le module `tests` de `crates/rbs-cli/src/generate/controller.rs` :

```rust
#[test]
fn each_handler_carries_an_operation_id_prefixed_by_its_module() {
    let rendered = render(&Feature::fresh("articles", Vec::new()));

    for action in ["list", "create", "find", "update", "delete"] {
        assert!(
            rendered.contains(&format!("operation_id = \"articles_{action}\"")),
            "`{action}` doit porter son operation_id :\n{rendered}"
        );
    }
}
```

`render` et `Feature::fresh` sont ceux qu'emploient déjà les tests voisins du module ;
reprendre exactement leur forme d'appel plutôt que celle-ci si elle diffère.

Dans le module `tests` de `crates/rbs-cli/src/new.rs` :

```rust
#[test]
fn the_health_handler_declares_its_own_tag() {
    let project = fixtures::Project::new().create();

    let controleur = fs::read_to_string(project.root.join("src/health/controller.rs"))
        .expect("le contrôleur de santé doit être engendré");

    // Sans `tag =`, utoipa retombe sur le chemin de module et le document porte
    // `crate::health::controller` en guise de section.
    assert!(controleur.contains("tag = \"health\""), "{controleur}");
}
```

- [ ] **Step 2 : lancer les deux tests et vérifier qu'ils échouent**

```bash
cargo test -p rbs-cli --lib generate::controller::tests::each_handler_carries_an_operation_id_prefixed_by_its_module
cargo test -p rbs-cli --lib new::tests::the_health_handler_declares_its_own_tag
```

Attendu : FAIL tous les deux.

- [ ] **Step 3 : poser les `operation_id`**

Dans `crates/rbs-cli/templates/feature/controller.rs.jinja`, sur chacun des cinq blocs
`#[utoipa::path(`, ajouter une ligne juste après le `tag = "{@ module @}",` :

```
    operation_id = "{@ module @}_list",
```

et de même `_create`, `_find`, `_update`, `_delete` sur les quatre autres, dans l'ordre
où les handlers apparaissent : `list`, `create`, `find`, `update`, `delete`.

Attention à l'indentation : quatre espaces, comme les lignes voisines. Ne pas introduire
de `{%- ... -%}` ici — le bloc est du texte constant à ceci près.

- [ ] **Step 4 : poser le tag de santé**

Dans `crates/rbs-cli/templates/project/src/health/controller.rs.jinja`, dans le bloc
`#[utoipa::path(`, ajouter après la ligne `path = "/health",` :

```
    tag = "health",
    operation_id = "health",
```

- [ ] **Step 5 : lancer les tests**

```bash
cargo test -p rbs-cli --lib
```

Attendu : PASS. `integration_examples` échouera à cette étape — c'est normal, la tâche 7
régénère les exemples. Le noter, ne pas le corriger ici.

- [ ] **Step 6 : fmt, clippy, commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/templates crates/rbs-cli/src/generate/controller.rs crates/rbs-cli/src/new.rs
git commit
```

```
fix(openapi): nomme chaque opération, que deux features rendaient homonyme

utoipa prend le nom nu du handler faute d'`operation_id`. Un projet à
deux CRUD produisait donc deux opérations d'identifiant `list`, ce que
la spécification interdit — invisible dans les exemples, qui n'ont
qu'une ressource chacun. Le handler de santé, lui, n'avait pas de tag
et le document le rangeait sous `crate::health::controller`.

Vérifications :
`cargo test -p rbs-cli --lib` : N passés, 0 échoué.
`integration_examples` échoue tant que les exemples n'ont pas suivi.
```

---

### Task 3 : le document OpenAPI lu en modèle serde

**Files:**
- Create: `crates/rbs-cli/src/client/document.rs`
- Create: `crates/rbs-cli/src/client/mod.rs` (réduit, pour cette tâche, à `pub(crate) mod document;` plus son doc-commentaire de module)
- Modify: `crates/rbs-cli/src/lib.rs` (ajouter `mod client;` dans la liste des modules, en ordre alphabétique)

**Interfaces:**
- Consumes: rien.
- Produces:

```rust
pub(crate) struct Document {
    pub paths: BTreeMap<String, PathItem>,
    pub schemas: BTreeMap<String, Schema>,
}
pub(crate) struct PathItem {
    /// Les opérations de ce chemin, méthode HTTP en majuscules.
    pub operations: Vec<(String, Operation)>,
}
pub(crate) struct Operation {
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<Schema>,
    /// Statut → schéma du corps, `None` quand la réponse n'a pas de contenu.
    pub responses: BTreeMap<u16, Option<Schema>>,
    pub secured: bool,
}
pub(crate) struct Parameter {
    pub name: String,
    pub location: Location,   // Path | Query | Autre(String)
    pub description: Option<String>,
    pub required: bool,
    pub schema: Schema,
}
pub(crate) enum Location { Path, Query, Autre(String) }

/// Un schéma, tel que le document l'écrit — aucune résolution, aucun jugement.
pub(crate) enum Schema {
    Ref(String),                       // le nom du composant, `#/components/schemas/` retiré
    Primitive { kind: String, nullable: bool, enumeration: Vec<String> },
    Array { items: Box<Schema>, nullable: bool },
    Object {
        properties: Vec<(String, Schema)>,   // dans l'ordre du document
        required: BTreeSet<String>,
        additional: Option<Box<Schema>>,
        nullable: bool,
        description: Option<String>,
    },
    Union(Vec<Schema>),                // oneOf, anyOf
    Intersection(Vec<Schema>),         // allOf
    Inconnu,
}

pub(crate) fn parse(json: &str) -> Result<Document, Erreur>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    #[error("la sortie du binaire openapi n'est pas du JSON : {0}")]
    Json(#[from] serde_json::Error),
    #[error("la sortie du binaire openapi n'est pas un document OpenAPI : `{champ}` manque")]
    ChampManquant { champ: &'static str },
}
```

L'analyse passe par `serde_json::Value` et non par des `#[derive(Deserialize)]` : le
document 3.1 écrit `"type"` tantôt en chaîne, tantôt en tableau (`["string","null"]`), et
un `enum` serde pour cela coûte plus qu'il ne rapporte. La conversion est explicite,
fonction par fonction.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/rbs-cli/src/client/document.rs`, module `tests` en fin de fichier :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(json: &str) -> Document {
        parse(json).expect("le document doit s'analyser")
    }

    #[test]
    fn a_nullable_string_is_read_as_a_nullable_primitive() {
        let document = parse_ok(
            r#"{"components":{"schemas":{"S":{"type":"object","properties":{
                 "a":{"type":["string","null"]}}}}}}"#,
        );

        let Schema::Object { properties, .. } = &document.schemas["S"] else {
            panic!("S doit être un objet");
        };
        let (_, champ) = &properties[0];
        assert!(
            matches!(champ, Schema::Primitive { kind, nullable: true, .. } if kind == "string"),
            "{champ:?}"
        );
    }

    #[test]
    fn a_reference_keeps_only_the_component_name() {
        let document = parse_ok(
            r#"{"components":{"schemas":{"S":{"type":"object","properties":{
                 "a":{"$ref":"#/components/schemas/Autre"}}}}}}"#,
        );

        let Schema::Object { properties, .. } = &document.schemas["S"] else {
            panic!("S doit être un objet");
        };
        assert_eq!(properties[0].1, Schema::Ref("Autre".to_string()));
    }

    #[test]
    fn the_properties_keep_the_order_of_the_document() {
        let document = parse_ok(
            r#"{"components":{"schemas":{"S":{"type":"object","properties":{
                 "z":{"type":"string"},"a":{"type":"string"}}}}}}"#,
        );

        let Schema::Object { properties, .. } = &document.schemas["S"] else {
            panic!("S doit être un objet");
        };
        let noms: Vec<&str> = properties.iter().map(|(nom, _)| nom.as_str()).collect();
        assert_eq!(noms, ["z", "a"]);
    }

    #[test]
    fn an_operation_carries_its_verb_in_upper_case() {
        let document = parse_ok(
            r#"{"paths":{"/a":{"get":{"operationId":"lire","responses":{}},
                               "post":{"operationId":"ecrire","responses":{}}}}}"#,
        );

        let verbes: Vec<&str> = document.paths["/a"]
            .operations
            .iter()
            .map(|(verbe, _)| verbe.as_str())
            .collect();
        assert_eq!(verbes, ["GET", "POST"]);
    }

    #[test]
    fn a_response_without_content_is_read_as_a_status_without_schema() {
        let document = parse_ok(
            r#"{"paths":{"/a":{"delete":{"operationId":"d","responses":{
                 "204":{"description":"supprimé"}}}}}}"#,
        );

        let (_, operation) = &document.paths["/a"].operations[0];
        assert_eq!(operation.responses[&204], None);
    }

    #[test]
    fn only_an_application_json_body_is_read() {
        let document = parse_ok(
            r#"{"paths":{"/a":{"post":{"operationId":"p","responses":{"200":{
                 "description":"ok","content":{"application/problem+json":{
                   "schema":{"$ref":"#/components/schemas/ProblemDetails"}}}}}}}}}"#,
        );

        let (_, operation) = &document.paths["/a"].operations[0];
        assert_eq!(operation.responses[&200], None);
    }

    #[test]
    fn a_security_requirement_marks_the_operation() {
        let document = parse_ok(
            r#"{"paths":{"/a":{"get":{"operationId":"g","responses":{},
                 "security":[{"bearer":[]}]}}}}"#,
        );

        let (_, operation) = &document.paths["/a"].operations[0];
        assert!(operation.secured);
    }

    #[test]
    fn a_payload_that_is_not_json_is_refused() {
        assert!(matches!(parse("pas du json"), Err(Erreur::Json(_))));
    }

    #[test]
    fn a_json_that_is_not_a_document_is_refused() {
        assert!(matches!(
            parse("[]"),
            Err(Erreur::ChampManquant { champ: "openapi" })
        ));
    }
}
```

Note pour l'implémenteur : `Schema` doit dériver `Debug, Clone, PartialEq` pour que ces
assertions compilent. `Document` : `Debug`.

- [ ] **Step 2 : lancer les tests et vérifier qu'ils échouent**

```bash
cargo test -p rbs-cli --lib client::document
```

Attendu : erreur de compilation, `unresolved module client`.

- [ ] **Step 3 : écrire `document.rs`**

Points d'attention, tous relevés sur la sortie réelle du 2026-09-04 :

- Le `type` d'un schéma est soit une chaîne, soit un tableau qui contient `"null"` :
  extraire `nullable` de la présence de `"null"`, et le type restant de l'autre entrée.
- Un `$ref` vaut `#/components/schemas/<Nom>` : ne garder que `<Nom>`.
- `content` : ne lire que `application/json`. `application/problem+json` décrit le corps
  d'erreur, que le client ne rend jamais — il le jette dans `ApiError`.
- `properties` doit garder l'ordre du document : `serde_json` a la feature
  `preserve_order`ou non selon le workspace. **Vérifier** : si `Map` est une
  `BTreeMap`, l'ordre est alphabétique et le test `the_properties_keep_the_order_of_the_document`
  échouera — dans ce cas, changer ce test en assertion d'ordre alphabétique stable et
  écrire dans le doc-commentaire du champ que l'ordre est celui de `serde_json`, pas celui
  du document. Ne pas ajouter la feature `preserve_order` pour ce seul confort : elle
  change le comportement de tout le CLI.
- Un document sans `paths` ni `components` est licite : rendre des collections vides,
  n'échouer que si `openapi` manque.
- `responses` : les clés sont des chaînes ; ignorer celles qui ne s'analysent pas en `u16`
  (`"default"`) plutôt qu'échouer.

- [ ] **Step 4 : lancer les tests**

```bash
cargo test -p rbs-cli --lib client::document
```

Attendu : 9 passés, 0 échoué.

- [ ] **Step 5 : fmt, clippy, commit**

```bash
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/client crates/rbs-cli/src/lib.rs
git commit
```

```
feat(client): lit le document OpenAPI d'un projet en un modèle propre

L'analyse passe par `serde_json::Value` plutôt que par des dérivations :
OpenAPI 3.1 écrit le type d'un champ nullable en tableau
(`["string","null"]`), et un enum serde pour ce seul cas coûterait plus
qu'une conversion explicite. Seul `application/json` est lu ; le corps
`problem+json` décrit l'erreur, que le client jette au lieu de la rendre.

Vérifications :
`cargo test -p rbs-cli --lib client::document` : 9 passés, 0 échoué.
```

---

### Task 4 : des schémas aux types TypeScript

**Files:**
- Create: `crates/rbs-cli/src/client/ts.rs`
- Modify: `crates/rbs-cli/src/client/mod.rs` (ajouter `pub(crate) mod ts;`)

**Interfaces:**
- Consumes: `client::document::{Document, Schema}` (tâche 3).
- Produces:

```rust
/// Le nom TypeScript d'un composant : `Page_PostResponse` → `PagePostResponse`.
pub(crate) fn identifiant(nom: &str) -> String;

/// L'expression de type TypeScript d'un schéma.
pub(crate) fn type_de(schema: &Schema) -> String;

/// Une interface prête à écrire.
pub(crate) struct Interface {
    pub nom: String,
    pub doc: Option<String>,
    pub corps: String,   // le bloc `{ … }` complet, indenté
}

pub(crate) fn interfaces(document: &Document) -> Result<Vec<Interface>, Erreur>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    #[error("les schémas `{premier}` et `{second}` donnent le même type TypeScript `{rendu}` : renommez l'un des deux")]
    IdentifiantsHomonymes { premier: String, second: String, rendu: String },
    #[error("le schéma `{nom}` référence `{cible}`, que le document ne déclare pas")]
    ReferenceInconnue { nom: String, cible: String },
    // les variantes de la tâche 5 s'y ajouteront
}
```

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/rbs-cli/src/client/ts.rs`, module `tests` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::document;

    fn schemas(json: &str) -> Document {
        document::parse(json).expect("document valide")
    }

    /// Le schéma d'une propriété d'un composant `S`, raccourci de tous les tests de type.
    fn type_du_champ(json_du_champ: &str) -> String {
        let document = schemas(&format!(
            r#"{{"components":{{"schemas":{{"S":{{"type":"object",
               "properties":{{"a":{json_du_champ}}}}}}}}}}}"#
        ));
        let Schema::Object { properties, .. } = &document.schemas["S"] else {
            panic!("S doit être un objet");
        };
        type_de(&properties[0].1)
    }

    #[test]
    fn a_uuid_is_still_a_string() {
        assert_eq!(type_du_champ(r#"{"type":"string","format":"uuid"}"#), "string");
    }

    #[test]
    fn an_integer_is_a_number() {
        assert_eq!(type_du_champ(r#"{"type":"integer","format":"int64"}"#), "number");
    }

    #[test]
    fn a_nullable_string_is_a_union_with_null() {
        assert_eq!(type_du_champ(r#"{"type":["string","null"]}"#), "string | null");
    }

    #[test]
    fn an_array_takes_the_suffix_of_its_items() {
        assert_eq!(
            type_du_champ(r#"{"type":"array","items":{"type":"string"}}"#),
            "string[]"
        );
    }

    #[test]
    fn an_array_of_nullable_items_is_parenthesised() {
        assert_eq!(
            type_du_champ(r#"{"type":"array","items":{"type":["string","null"]}}"#),
            "(string | null)[]"
        );
    }

    #[test]
    fn a_map_becomes_a_record() {
        assert_eq!(
            type_du_champ(
                r#"{"type":"object","additionalProperties":{"type":"array","items":{"type":"string"}}}"#
            ),
            "Record<string, string[]>"
        );
    }

    #[test]
    fn an_object_without_anything_is_a_record_of_unknown() {
        assert_eq!(type_du_champ(r#"{"type":"object"}"#), "Record<string, unknown>");
    }

    #[test]
    fn a_string_enum_becomes_a_union_of_literals() {
        assert_eq!(
            type_du_champ(r#"{"type":"string","enum":["admin","user"]}"#),
            "\"admin\" | \"user\""
        );
    }

    #[test]
    fn an_inline_object_is_rendered_inline() {
        assert_eq!(
            type_du_champ(
                r#"{"type":"object","required":["a"],"properties":{"a":{"type":"string"},"b":{"type":"boolean"}}}"#
            ),
            "{ a: string; b?: boolean }"
        );
    }

    #[test]
    fn a_schema_without_a_type_is_unknown() {
        assert_eq!(type_du_champ("{}"), "unknown");
    }

    #[test]
    fn a_component_name_loses_what_is_not_an_identifier() {
        assert_eq!(identifiant("Page_PostResponse"), "PagePostResponse");
        assert_eq!(identifiant("ProblemDetails"), "ProblemDetails");
    }

    #[test]
    fn two_components_that_render_the_same_identifier_are_refused() {
        let document = schemas(
            r#"{"components":{"schemas":{"Page_A":{"type":"object"},"PageA":{"type":"object"}}}}"#,
        );

        let erreur = interfaces(&document).expect_err("la collision doit être refusée");

        let message = erreur.to_string();
        assert!(message.contains("Page_A"), "{message}");
        assert!(message.contains("PageA"), "{message}");
    }

    #[test]
    fn a_reference_to_an_absent_component_is_refused() {
        let document = schemas(
            r#"{"components":{"schemas":{"S":{"type":"object","properties":{
                 "a":{"$ref":"#/components/schemas/Fantome"}}}}}}"#,
        );

        let erreur = interfaces(&document).expect_err("la référence pendante doit être refusée");

        assert!(erreur.to_string().contains("Fantome"), "{erreur}");
    }

    #[test]
    fn an_interface_renders_its_required_and_optional_properties() {
        let document = schemas(
            r#"{"components":{"schemas":{"CreatePost":{"type":"object",
                 "required":["title"],"properties":{
                   "title":{"type":"string"},"draft":{"type":["boolean","null"]}}}}}}"#,
        );

        let rendues = interfaces(&document).expect("rendu");

        assert_eq!(rendues.len(), 1);
        assert_eq!(rendues[0].nom, "CreatePost");
        assert_eq!(
            rendues[0].corps,
            "{\n  title: string;\n  draft?: boolean | null;\n}"
        );
    }
}
```

Note : `type_du_champ` rend un `String` ; pour `an_inline_object_is_rendered_inline`,
l'objet inline se rend **sur une ligne**, l'objet nommé d'une interface **sur plusieurs**.
C'est la seule différence entre les deux rendus, et elle tient à ce que l'un est une
expression et l'autre une déclaration.

- [ ] **Step 2 : lancer les tests et vérifier qu'ils échouent**

```bash
cargo test -p rbs-cli --lib client::ts
```

Attendu : erreur de compilation.

- [ ] **Step 3 : écrire `ts.rs`**

Règles, dans l'ordre où `type_de` doit les appliquer :

1. `Schema::Ref(nom)` → `identifiant(nom)`.
2. `Schema::Primitive { kind, nullable, enumeration }` : `enumeration` non vide → union de
   littéraux `"a" | "b"` ; sinon `string`, `number` (pour `integer` et `number`),
   `boolean`, `unknown` par défaut. `nullable` ajoute `| null`.
3. `Schema::Array { items, nullable }` → `T[]`, `T` parenthésé s'il contient un espace.
4. `Schema::Object { additional: Some(s), .. }` → `Record<string, T>`.
   `Schema::Object { properties, .. }` non vides → `{ a: A; b?: B }` sur une ligne.
   Objet vide → `Record<string, unknown>`.
5. `Union` → `A | B`, `Intersection` → `A & B`.
6. `Inconnu` → `unknown`.

`identifiant` : couper sur tout ce qui n'est pas `[A-Za-z0-9]`, capitaliser la première
lettre de chaque tronçon, recoller. Un identifiant qui commencerait par un chiffre reçoit
un `_` en tête.

`interfaces` : parcourir `document.schemas`, rendre chacun, et tenir une
`BTreeMap<String, String>` de l'identifiant vers le nom d'origine pour détecter la
collision. La vérification des `$ref` se fait par un parcours récursif de chaque schéma
avant le rendu — un `$ref` pendant doit être une erreur, pas un `unknown` muet.

Le `description` d'un composant devient un doc-commentaire `/** … */` ; les retours à la
ligne y sont préfixés de ` * `.

- [ ] **Step 4 : lancer les tests**

```bash
cargo test -p rbs-cli --lib client::ts
```

Attendu : 14 passés, 0 échoué.

- [ ] **Step 5 : fmt, clippy, commit**

```
feat(client): traduit les schémas OpenAPI en types TypeScript

`unknown` et non `any` sur un schéma que le générateur ne sait pas
décrire : le consommateur doit se prononcer, non passer sans le voir.
Deux composants qui se réduiraient au même identifiant sont une erreur
qui les nomme tous les deux — un renommage silencieux rendrait le client
faux sans rien signaler. Un `$ref` pendant, de même.

Vérifications :
`cargo test -p rbs-cli --lib client::ts` : 14 passés, 0 échoué.
```

---

### Task 5 : des opérations aux méthodes, et la template du client

**Files:**
- Modify: `crates/rbs-cli/src/client/ts.rs`
- Create: `crates/rbs-cli/templates/client/ts/client.ts.jinja`

**Interfaces:**
- Consumes: `identifiant`, `type_de`, `Interface`, `interfaces` (tâche 4) ;
  `Document`, `Operation`, `Parameter`, `Location` (tâche 3).
- Produces:

```rust
/// Une méthode prête à écrire.
pub(crate) struct Methode {
    pub nom: String,
    pub doc: Vec<String>,        // les lignes du doc-commentaire, sans les marqueurs
    pub signature: String,       // `articlesList(query: ArticlesListQuery = {})`
    pub corps: String,           // le `return this.request<…>(…);` complet
}

/// Le client entier, rendu.
pub(crate) fn rendre(document: &Document, projet: &str) -> Result<String, Erreur>;
```

et trois variantes d'erreur de plus :

```rust
    #[error("les opérations `{premiere}` et `{seconde}` donnent la même méthode `{rendu}` : posez un `operation_id` sur l'un des deux handlers")]
    MethodesHomonymes { premiere: String, seconde: String, rendu: String },
    #[error("l'opération `{operation}` n'a pas d'operationId : posez un `operation_id` sur son handler")]
    SansOperationId { operation: String },
    #[error("l'opération `{operation}` déclare un paramètre `{parametre}` en `{emplacement}`, que le générateur ne sait pas poser")]
    ParametreNonSupporte { operation: String, parametre: String, emplacement: String },
```

- [ ] **Step 1 : écrire les tests qui échouent**

À ajouter au module `tests` de `crates/rbs-cli/src/client/ts.rs` :

```rust
    /// Un document minimal portant une seule opération, pour les tests de méthode.
    fn une_operation(chemin: &str, verbe: &str, corps_json: &str) -> Document {
        document::parse(&format!(
            r#"{{"openapi":"3.1.0","paths":{{"{chemin}":{{"{verbe}":{corps_json}}}}}}}"#
        ))
        .expect("document valide")
    }

    #[test]
    fn an_operation_id_becomes_a_camel_case_method() {
        let rendu = rendre(
            &une_operation("/articles", "get", r#"{"operationId":"articles_list","responses":{}}"#),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("articlesList("), "{rendu}");
    }

    #[test]
    fn a_path_parameter_becomes_a_positional_argument() {
        let rendu = rendre(
            &une_operation(
                "/articles/{id}",
                "get",
                r#"{"operationId":"articles_find","parameters":[
                     {"name":"id","in":"path","required":true,"schema":{"type":"string","format":"uuid"}}],
                   "responses":{"200":{"description":"ok","content":{"application/json":{
                     "schema":{"type":"string"}}}}}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("articlesFind(id: string): Promise<string>"), "{rendu}");
        assert!(rendu.contains("${encodeURIComponent(String(id))}"), "{rendu}");
    }

    #[test]
    fn the_query_parameters_are_gathered_in_an_exported_interface() {
        let rendu = rendre(
            &une_operation(
                "/articles",
                "get",
                r#"{"operationId":"articles_list","parameters":[
                     {"name":"page","in":"query","required":false,"schema":{"type":"integer"}},
                     {"name":"per_page","in":"query","required":false,"schema":{"type":"integer"}}],
                   "responses":{}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("export interface ArticlesListQuery {"), "{rendu}");
        assert!(rendu.contains("page?: number;"), "{rendu}");
        assert!(rendu.contains("articlesList(query: ArticlesListQuery = {})"), "{rendu}");
    }

    #[test]
    fn a_required_query_parameter_makes_the_argument_required() {
        let rendu = rendre(
            &une_operation(
                "/recherche",
                "get",
                r#"{"operationId":"recherche","parameters":[
                     {"name":"q","in":"query","required":true,"schema":{"type":"string"}}],
                   "responses":{}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("recherche(query: RechercheQuery)"), "{rendu}");
        assert!(!rendu.contains("RechercheQuery = {}"), "{rendu}");
    }

    #[test]
    fn the_arguments_run_path_then_body_then_query() {
        let rendu = rendre(
            &une_operation(
                "/articles/{id}",
                "patch",
                r#"{"operationId":"articles_update","parameters":[
                     {"name":"id","in":"path","required":true,"schema":{"type":"string"}},
                     {"name":"dry","in":"query","required":false,"schema":{"type":"boolean"}}],
                   "requestBody":{"required":true,"content":{"application/json":{
                     "schema":{"type":"string"}}}},
                   "responses":{}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(
            rendu.contains("articlesUpdate(id: string, body: string, query: ArticlesUpdateQuery = {})"),
            "{rendu}"
        );
    }

    #[test]
    fn a_204_alone_returns_void() {
        let rendu = rendre(
            &une_operation(
                "/articles/{id}",
                "delete",
                r#"{"operationId":"articles_delete","parameters":[
                     {"name":"id","in":"path","required":true,"schema":{"type":"string"}}],
                   "responses":{"204":{"description":"supprimé"},
                                "404":{"description":"absent"}}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("articlesDelete(id: string): Promise<void>"), "{rendu}");
    }

    #[test]
    fn several_successful_responses_are_unioned() {
        let rendu = rendre(
            &une_operation(
                "/a",
                "post",
                r#"{"operationId":"a_create","responses":{
                     "200":{"description":"ok","content":{"application/json":{"schema":{"type":"string"}}}},
                     "202":{"description":"accepté","content":{"application/json":{"schema":{"type":"boolean"}}}}}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("Promise<string | boolean>"), "{rendu}");
    }

    #[test]
    fn a_secured_operation_says_so_in_its_doc_comment() {
        let rendu = rendre(
            &une_operation(
                "/moi",
                "get",
                r#"{"operationId":"moi","security":[{"bearer":[]}],"responses":{}}"#,
            ),
            "demo",
        )
        .expect("rendu");

        assert!(rendu.contains("requiert un jeton"), "{rendu}");
    }

    #[test]
    fn two_operations_of_the_same_name_are_refused() {
        let document = document::parse(
            r#"{"openapi":"3.1.0","paths":{
                 "/a":{"get":{"operationId":"list","responses":{}}},
                 "/b":{"get":{"operationId":"list","responses":{}}}}}"#,
        )
        .expect("document valide");

        let erreur = rendre(&document, "demo").expect_err("la collision doit être refusée");

        let message = erreur.to_string();
        assert!(message.contains("list"), "{message}");
        assert!(message.contains("operation_id"), "{message}");
    }

    #[test]
    fn an_operation_without_an_operation_id_is_refused() {
        let document = une_operation("/a", "get", r#"{"responses":{}}"#);

        let erreur = rendre(&document, "demo").expect_err("l'absence doit être refusée");

        assert!(erreur.to_string().contains("operation_id"), "{erreur}");
    }

    #[test]
    fn a_header_parameter_is_refused_rather_than_ignored() {
        let document = une_operation(
            "/a",
            "get",
            r#"{"operationId":"a","parameters":[
                 {"name":"X-Tenant","in":"header","required":true,"schema":{"type":"string"}}],
               "responses":{}}"#,
        );

        let erreur = rendre(&document, "demo").expect_err("le paramètre doit être refusé");

        let message = erreur.to_string();
        assert!(message.contains("X-Tenant"), "{message}");
        assert!(message.contains("header"), "{message}");
    }

    #[test]
    fn the_rendered_client_carries_the_project_name_and_the_error_class() {
        let rendu = rendre(&une_operation("/a", "get", r#"{"operationId":"a","responses":{}}"#), "demo-api")
            .expect("rendu");

        assert!(rendu.contains("demo-api"), "{rendu}");
        assert!(rendu.contains("export class ApiError extends Error"), "{rendu}");
        assert!(rendu.contains("export class ApiClient"), "{rendu}");
    }

    #[test]
    fn a_problem_details_interface_is_emitted_even_when_the_document_has_none() {
        let rendu = rendre(&une_operation("/a", "get", r#"{"operationId":"a","responses":{}}"#), "demo")
            .expect("rendu");

        assert!(rendu.contains("interface ProblemDetails"), "{rendu}");
    }
```

- [ ] **Step 2 : lancer les tests et vérifier qu'ils échouent**

```bash
cargo test -p rbs-cli --lib client::ts
```

Attendu : erreur de compilation, `rendre` introuvable.

- [ ] **Step 3 : écrire la template**

`crates/rbs-cli/templates/client/ts/client.ts.jinja`. Elle reçoit
`{ projet, interfaces, methodes, problem_details_manquant }`.

```
// Client de l'API {@ projet @}, engendré par `rbs generate client --lang ts`.
//
// Régénérez-le après chaque changement de contrat plutôt que de le retoucher : la
// commande refuse d'écraser un fichier modifié, et `--force` lève ce refus.

{% if problem_details_manquant -%}
/** Corps d'erreur RFC 9457, tel que rbs-core le rend. */
export interface ProblemDetails {
  type: string;
  title: string;
  status: number;
  detail?: string | null;
  errors?: Record<string, string[]> | null;
  request_id?: string | null;
}

{% endif -%}
{% for interface in interfaces -%}
{@ interface.doc @}export interface {@ interface.nom @} {@ interface.corps @}

{% endfor -%}
/** Ce que le client jette sur une réponse hors 2xx. */
export class ApiError extends Error {
  readonly status: number;
  readonly body: unknown;
  readonly problem?: ProblemDetails;

  constructor(status: number, body: unknown) {
    const problem = isProblem(body) ? body : undefined;
    super(problem?.title ?? `HTTP ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.body = body;
    this.problem = problem;
  }
}

function isProblem(body: unknown): body is ProblemDetails {
  return (
    typeof body === "object" &&
    body !== null &&
    "title" in body &&
    "status" in body
  );
}

/** En-têtes de chaque requête. Une fonction pour un jeton qui tourne. */
export type Headers =
  | Record<string, string>
  | (() => Record<string, string> | Promise<Record<string, string>>);

export interface ApiClientOptions {
  /** Racine de l'API : `https://api.exemple.fr`, ou `/api` sur le même domaine. */
  baseUrl: string;
  /** En-têtes posés sur chaque requête. C'est ici que va le jeton. */
  headers?: Headers;
  /** `fetch` à employer. `globalThis.fetch` par défaut. */
  fetch?: typeof globalThis.fetch;
}

export class ApiClient {
  private readonly baseUrl: string;
  private readonly headers: Headers;
  private readonly fetchImpl: typeof globalThis.fetch;

  constructor(options: ApiClientOptions) {
    // La barre finale est retirée ici plutôt qu'à chaque appel : les chemins du
    // document commencent tous par une barre.
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.headers = options.headers ?? {};
    this.fetchImpl = options.fetch ?? globalThis.fetch.bind(globalThis);
  }

{% for methode in methodes -%}
{@ methode.doc @}  {@ methode.signature @} {
{@ methode.corps @}
  }

{% endfor -%}
  private async request<T>(
    method: string,
    path: string,
    options: { query?: Record<string, unknown>; body?: unknown } = {},
  ): Promise<T> {
    const search = new URLSearchParams();
    for (const [cle, valeur] of Object.entries(options.query ?? {})) {
      if (valeur !== undefined && valeur !== null) {
        search.set(cle, String(valeur));
      }
    }

    // Concaténation, et non `new URL` : une racine relative est le cas normal d'une
    // application servie depuis son propre domaine, et `new URL("/api")` jette.
    const queryString = search.toString();
    const url = `${this.baseUrl}${path}${queryString ? `?${queryString}` : ""}`;

    const headers: Record<string, string> = { accept: "application/json" };
    Object.assign(
      headers,
      typeof this.headers === "function" ? await this.headers() : this.headers,
    );
    if (options.body !== undefined) {
      headers["content-type"] = "application/json";
    }

    const response = await this.fetchImpl(url, {
      method,
      headers,
      body: options.body === undefined ? undefined : JSON.stringify(options.body),
    });

    const payload = await parse(response);

    if (!response.ok) {
      throw new ApiError(response.status, payload);
    }

    return payload as T;
  }
}

async function parse(response: Response): Promise<unknown> {
  if (response.status === 204) {
    return undefined;
  }

  const type = response.headers.get("content-type") ?? "";
  if (type.includes("json")) {
    // Un corps annoncé JSON mais vide ne doit pas masquer le statut réel.
    const texte = await response.text();
    return texte.length === 0 ? undefined : JSON.parse(texte);
  }

  const texte = await response.text();
  return texte.length === 0 ? undefined : texte;
}
```

Attention : cette template contient `${…}` (littéraux de gabarit TypeScript), qui ne sont
pas des délimiteurs minijinja, et `{@ @}` là où une valeur est injectée. Ne pas y écrire
de `{@` littéral.

- [ ] **Step 4 : écrire le rendu Rust**

Dans `ts.rs` :

- `nom_de_methode(operation_id)` : `articles_list` → `articlesList`. Couper sur `_`, `-`
  et l'espace, capitaliser les tronçons suivants, garder le premier tel quel en minuscule
  initiale.
- `rendre` collecte d'abord toutes les opérations (chemin, verbe, opération), en ordre
  déterministe (`document.paths` est une `BTreeMap`, `operations` un `Vec` dans l'ordre du
  document), puis :
  1. refuse une opération sans `operationId` ;
  2. refuse un paramètre dont `location` est `Autre` ;
  3. refuse deux noms de méthode identiques ;
  4. construit l'interface de query, si l'opération a des paramètres de query, et l'ajoute
     aux interfaces — donc **après** `interfaces(document)`, et dans le même contrôle de
     collision ;
  5. rend chaque méthode.
- Le chemin de la requête est un littéral de gabarit TypeScript : `/articles/{id}` devient
  `` `/articles/${encodeURIComponent(String(id))}` ``. Sans paramètre, une chaîne simple
  entre guillemets doubles : `"/articles"`.
- Le corps de la méthode :

```
    return this.request<PostResponse>("PATCH", `/posts/${encodeURIComponent(String(id))}`, {
      body,
      query,
    });
```

  Les clés `body` et `query` ne figurent que si l'opération en a. Sans ni l'un ni l'autre,
  le troisième argument est omis.
- `problem_details_manquant` vaut `!document.schemas.contains_key("ProblemDetails")`.
- Le doc-commentaire d'une méthode réunit, dans l'ordre : le `summary`, la `description`,
  la ligne `<VERBE> <chemin>`, et « requiert un jeton » si `secured`. Il est rendu en
  `/** … */` indenté de deux espaces, et vaut la chaîne vide s'il n'y a rien à dire.

- [ ] **Step 5 : lancer les tests**

```bash
cargo test -p rbs-cli --lib client::
```

Attendu : 27 passés, 0 échoué (14 de la tâche 4, 13 de celle-ci).

- [ ] **Step 6 : prouver que le TypeScript engendré est valide**

Rien, dans `cargo test`, ne dit qu'un fichier `.ts` compile. Écrire dans le scratchpad un
petit test manuel :

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad
```

y déposer le rendu d'un document réaliste (le JSON relevé sur `examples/blog-auth` fait
l'affaire ; le regénérer si besoin) puis :

```bash
cd "$S/client-ts-tsc" && npx -y typescript@5 tsc --strict --noEmit --target es2022 --lib es2022,dom client.ts
```

Attendu : exit 0, aucune sortie. Reporter le résultat dans le message de commit. Ce n'est
pas une vérification de CI — CI n'a pas de toolchain Node hors de `docs/` — mais c'est la
seule qui prouve le point.

- [ ] **Step 7 : fmt, clippy, commit**

```
feat(client): engendre la classe ApiClient depuis les opérations du document

Une méthode par opération, les paramètres de chemin en positionnels, le
corps ensuite, la query réunie en une interface exportée que l'appelant
puisse nommer. `headers` accepte une fonction : le fragment `auth` livre
un `refresh`, donc un jeton qui change en cours de session, et un Record
figé obligerait à reconstruire le client à chaque rotation.

Un paramètre d'en-tête ou de cookie est refusé plutôt qu'ignoré — omis
en silence, il produirait un appel qui échoue à l'exécution.

Vérifications :
`cargo test -p rbs-cli --lib client::` : 27 passés, 0 échoué.
`npx tsc --strict --noEmit` sur le client rendu d'un document à deux
features : exit 0.
```

---

### Task 6 : la commande

**Files:**
- Modify: `crates/rbs-cli/src/client/mod.rs`
- Modify: `crates/rbs-cli/src/cli.rs:134` (enum `GenerateCommands`)
- Modify: `crates/rbs-cli/src/lib.rs:90-113` (l'aiguillage de `Commands::Generate`)

**Interfaces:**
- Consumes: `client::ts::rendre` (tâche 5), `client::document::parse` (tâche 3),
  `plan::Builder`, `plan::application::apply`, `metadata::cible`, `git::garde`.
- Produces: `rbs generate client --lang ts [--out DIR] [--force] [--dry-run]`.

```rust
/// Le langage du client demandé.
///
/// Sans rapport avec `lang::Lang`, qui est la langue du guide `AGENTS.md` : ici c'est le
/// langage de programmation de la sortie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Lang { Ts }

pub(crate) struct Options {
    pub lang: Lang,
    pub out: Option<PathBuf>,
    pub directory: PathBuf,
    pub force: bool,
}

pub(crate) struct Planned {
    pub plan: plan::Plan,
    pub fichier: String,
    pub operations: usize,
}

pub(crate) fn plan_for(options: &Options) -> Result<Planned, Error>;
```

- [ ] **Step 1 : déclarer la sous-commande**

Dans `crates/rbs-cli/src/cli.rs`, enum `GenerateCommands`, après `Feature` :

```rust
    /// Engendre un client typé depuis le document OpenAPI du projet.
    Client {
        /// Langage du client.
        #[arg(long, value_name = "LANGAGE")]
        lang: crate::client::Lang,

        /// Répertoire de sortie, relatif à la racine du projet.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,

        /// Écrit même si le working tree Git est sale, ou si le client a été retouché.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,
    },
```

`--lang` est requis : pas de `Option<…>`, pas de `default_value`. Le jour où un second
langage s'ajoute, aucune invocation existante ne change de sens.

Le test `the_clap_declaration_is_consistent` du même fichier couvre déjà la cohérence de
la déclaration ; ajouter à côté :

```rust
    #[test]
    fn the_client_subcommand_requires_its_language() {
        assert!(Cli::try_parse_from(["rbs", "generate", "client"]).is_err());
        assert!(Cli::try_parse_from(["rbs", "generate", "client", "--lang", "go"]).is_err());

        let commande = Cli::try_parse_from(["rbs", "generate", "client", "--lang", "ts"])
            .expect("commande valide");
        let Commands::Generate {
            command: GenerateCommands::Client { lang, out, .. },
        } = commande.command
        else {
            panic!("la sous-commande doit être `client`");
        };
        assert_eq!(lang, crate::client::Lang::Ts);
        assert_eq!(out, None);
    }
```

- [ ] **Step 2 : lancer le test et vérifier qu'il échoue**

```bash
cargo test -p rbs-cli --lib cli::tests::the_client_subcommand_requires_its_language
```

Attendu : erreur de compilation.

- [ ] **Step 3 : écrire les tests de la commande**

Dans `crates/rbs-cli/src/client/mod.rs`, module `tests`. Ces tests n'ont pas besoin de
compiler quoi que ce soit : ils prouvent les refus, qui arrivent tous avant cargo.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn a_project_without_a_library_is_refused_by_naming_it() {
        let project = fixtures::Project::new().create();
        std::fs::remove_file(project.root.join("src/lib.rs")).expect("lib supprimable");

        let erreur = plan_for(&Options {
            lang: Lang::Ts,
            out: None,
            directory: project.root.clone(),
            force: true,
        })
        .expect_err("le projet sans bibliothèque doit être refusé");

        let message = erreur.to_string();
        assert!(message.contains("src/lib.rs"), "{message}");
    }

    #[test]
    fn a_project_without_the_openapi_binary_is_refused_with_the_block_to_paste() {
        let project = fixtures::Project::new().create();
        std::fs::remove_file(project.root.join("src/bin/openapi.rs")).expect("binaire supprimable");

        let erreur = plan_for(&Options {
            lang: Lang::Ts,
            out: None,
            directory: project.root.clone(),
            force: true,
        })
        .expect_err("le projet sans binaire doit être refusé");

        let remede = erreur.remedy().expect("le refus doit porter un remède");
        assert!(remede.contains("[[bin]]"), "{remede}");
        assert!(remede.contains("src/bin/openapi.rs"), "{remede}");
        assert!(remede.contains("ApiDoc::openapi()"), "{remede}");
    }

    #[test]
    fn the_default_output_is_the_typescript_directory_of_clients() {
        assert_eq!(sortie(None, Lang::Ts), PathBuf::from("clients/ts/client.ts"));
    }

    #[test]
    fn an_explicit_output_replaces_the_directory_but_not_the_file_name() {
        assert_eq!(
            sortie(Some(&PathBuf::from("web/src/api")), Lang::Ts),
            PathBuf::from("web/src/api/client.ts")
        );
    }
}
```

`sortie(out: Option<&Path>, lang: Lang) -> PathBuf` est la fonction pure qui décide du
chemin ; l'extraire pour qu'elle se teste sans projet.

- [ ] **Step 4 : lancer les tests et vérifier qu'ils échouent**

```bash
cargo test -p rbs-cli --lib client::tests
```

Attendu : erreur de compilation.

- [ ] **Step 5 : écrire `mod.rs`**

Séquence de `plan_for`, dans l'ordre où les échecs restent inoffensifs :

1. `metadata::cible::<Error>(&options.directory)` — donne `root` et les métadonnées ;
2. `git::garde(&root)` sauf `--force` ;
3. `root.join("src/lib.rs").exists()` — sinon `Error::SansBibliotheque` ;
4. `root.join("src/bin/openapi.rs").exists()` — sinon `Error::SansBinaire` ;
5. `cargo run --quiet --bin openapi`, `current_dir(&root)`, stdout capturé, **stderr
   hérité** pour que la compilation se voie ;
6. `document::parse` sur la sortie ;
7. `ts::rendre(&document, &nom_du_projet)` ;
8. un `plan::Builder`, un seul `create(chemin, &rendu)`.

Erreurs, chacune nommant son fichier relativement à la racine :

```rust
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("aucun projet rbs ici : `rbs generate client` s'exécute dans un projet créé par `rbs new`")]
    PasUnProjet,

    #[error(
        "ce projet n'a pas de src/lib.rs : `ApiDoc` vit dans le binaire principal, où un \
         binaire séparé ne peut pas l'atteindre — `rbs generate client` demande un projet \
         créé par `rbs new` depuis rbs 1.0"
    )]
    SansBibliotheque,

    #[error("src/bin/openapi.rs est absent : rbs ne peut pas lire le document du projet")]
    SansBinaire,

    #[error("cargo n'a pas pu être lancé : {0}")]
    Cargo(#[source] std::io::Error),

    #[error("`cargo run --bin openapi` a échoué (code {code}) : le projet ne compile pas")]
    BinaireEnEchec { code: i32 },

    #[error("{0}")]
    Document(#[from] document::Erreur),

    #[error("{0}")]
    Rendu(#[from] ts::Erreur),

    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),

    #[error(transparent)]
    WorkingTreeSale(#[from] crate::errors::WorkingTreeSale),

    #[error("{0}")]
    Plan(#[from] plan::Error),

    #[error("{0}")]
    Application(#[from] plan::application::Error),

    #[error("{0}")]
    Metadata(#[from] crate::metadata::Error),
}

crate::errors::depuis_la_racine!(Error);
```

`remedy()` sur `SansBinaire` rend le fichier complet et la section `[[bin]]`, sur le modèle
de `seed.rs:93-108` — reprendre sa forme de message. Le nom de crate se lit par
`metadonnees.package_name(&root.join("Cargo.toml"))`, tirets remplacés par des soulignés,
comme `generate/command.rs:253-261` le fait déjà.

- [ ] **Step 6 : aiguiller depuis `lib.rs`**

Dans `crates/rbs-cli/src/lib.rs`, ajouter `mod client;` en ordre alphabétique, et dans le
`match` de `Commands::Generate` traiter `GenerateCommands::Client` **avant** le tuple
existant : les deux autres variantes partagent une signature que `Client` ne partage pas.

```rust
        Commands::Generate { command } => {
            if let GenerateCommands::Client {
                lang,
                out,
                force,
                dry_run,
            } = command
            {
                if let Err(error) = generate_client(lang, out, force, dry_run) {
                    ui::error(&error.to_string());
                    if let Some(remedy) = error.remedy() {
                        ui::info(&format!("\n{remedy}"));
                    }
                    std::process::exit(1);
                }

                return;
            }

            // … le tuple existant, `Client` désormais inatteignable
        }
```

Attention : le `match` existant sur `command` devra recevoir un bras
`GenerateCommands::Client { .. } => unreachable!(…)`, ou être restructuré. Préférer la
restructuration à un `unreachable!` — un `match` complet est plus sûr qu'un commentaire.

`generate_client` suit `generate` de la même façon : plan affiché, `appliquer`, puis

```rust
    ui::success(&format!(
        "client TypeScript engendré — {} pour {}",
        planned.fichier,
        ui::operations(planned.operations)
    ));
```

Si `ui::operations` n'existe pas, écrire la ligne sans elle plutôt que d'ajouter un
helper : `ui::files` est là pour les fichiers, pas pour les opérations.

- [ ] **Step 7 : lancer les tests**

```bash
cargo test -p rbs-cli --lib
```

Attendu : PASS hors `integration_examples`, qui reste rouge jusqu'à la tâche 7.

- [ ] **Step 8 : prouver la commande à la main**

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad
cargo run -p rbs-cli --bin rbs -- new "$S/client-ts-demo" --yes \
  --core-path "$PWD/crates/rbs-core" \
  --database-url 'postgres://rbs:rbs@localhost:5432/demo' --lang fr
cd "$S/client-ts-demo"
cargo run --manifest-path <racine>/Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud articles --fields 'title:string,body:text,published:bool' --force
cargo run --manifest-path <racine>/Cargo.toml -p rbs-cli --bin rbs -- \
  generate client --lang ts --force
sed -n '1,60p' clients/ts/client.ts
```

Attendu : le plan affiché, `+ clients/ts/client.ts créé`, puis un fichier qui porte
`articlesList`, `articlesCreate`, `articlesFind`, `articlesUpdate`, `articlesDelete` et
`health`. Relancer la commande : `= clients/ts/client.ts déjà fait`.

- [ ] **Step 9 : fmt, clippy, commit**

```
feat(cli): engendre un client typé par `rbs generate client --lang ts`

La commande lance `cargo run --bin openapi` dans le projet, lit le
document sur la sortie standard et écrit le client par le rituel du
dépôt — plan affiché, puis appliqué. Le fichier étant projeté comme une
création, la régénération est idempotente et un client retouché part en
conflit plutôt que d'être écrasé.

Un projet sans `src/lib.rs` est refusé en le nommant : `ApiDoc` y vit
dans le binaire principal, hors de portée d'un binaire séparé. Un projet
sans `src/bin/openapi.rs` reçoit le fichier et le bloc `[[bin]]` à
coller, comme `rbs seed` le fait déjà pour le sien.

Vérifications :
`cargo test -p rbs-cli --lib` : N passés, 0 échoué.
`rbs new` + `generate crud articles` + `generate client --lang ts` sur un
projet neuf : client écrit, relance idempotente.
```

---

### Task 7 : les exemples remis à niveau

**Files:**
- Modify: les quatre `examples/*` (fichiers énumérés ci-dessous)
- Create: `examples/hello-crud/clients/ts/client.ts`
- Modify: `crates/rbs-cli/tests/integration_examples.rs`
- Modify: `examples/README.md`

**Interfaces:**
- Consumes: le CLI des tâches 1, 2 et 6.
- Produces: `cargo test -p rbs-cli --test integration_examples` repasse au vert.

**Ce qui bouge dans chaque exemple :** `src/bin/openapi.rs` (nouveau), la section
`[[bin]]` et le commentaire de `default-run` dans `Cargo.toml`, `tag`/`operation_id` dans
`src/health/controller.rs`, `operation_id` dans `src/<ressource>/controller.rs`.

**Régénérer par diff, jamais par écrasement.** `examples/newsletter-queue` et
`examples/file-drop` portent des éditions à la main qu'un `mv` perdrait, et
`newsletter-queue/src/subscribers/controller.rs` porte un handler `broadcast` écrit à la
main, qui devra recevoir son `operation_id = "subscribers_broadcast"` à la main lui aussi.

- [ ] **Step 1 : engendrer les quatre projets à côté, sans toucher aux exemples**

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad/client-ts-regen
mkdir -p "$S"
```

Rejouer, dans `$S`, les commandes exactes qu'`examples/README.md` donne pour chacun des
quatre projets — en remplaçant `mv <projet> examples/<projet>` par un `mv` vers `$S`.

- [ ] **Step 2 : reporter les écarts un par un**

```bash
for p in hello-crud blog-auth file-drop newsletter-queue; do
  diff -ru "examples/$p" "$S/$p" > "$S/../client-ts-diff-$p.txt"
done
wc -l "$S"/../client-ts-diff-*.txt
```

Puis reporter à la main, dans `examples/`, **uniquement** ce que les templates ont changé.
Tout écart qui n'est pas l'un des quatre changements attendus est un signal : le lire
avant de le reporter.

- [ ] **Step 3 : l'`operation_id` du handler écrit à la main**

Dans `examples/newsletter-queue/src/subscribers/controller.rs`, bloc `#[utoipa::path(` de
`broadcast`, après `tag = "subscribers",` :

```
    operation_id = "subscribers_broadcast",
```

- [ ] **Step 4 : le test de non-dérive rapide**

```bash
cargo test -p rbs-cli --test integration_examples 2>&1 | tail -20
```

Attendu : 4 passés, 0 échoué. Sinon, lire l'écart que le test imprime : c'est lui
l'oracle, pas le diff de l'étape 2.

- [ ] **Step 5 : sortir le futur client de la comparaison rapide, et le couvrir à part**

Dans `crates/rbs-cli/tests/integration_examples.rs`, ajouter un champ à `Exemple` :

```rust
    /// Ce qu'engendre une commande que `assert_no_drift` ne rejoue pas.
    ///
    /// `rbs generate client` compile le projet pour lire son document OpenAPI : le
    /// rejouer ici ajouterait une compilation complète à une suite qui n'en contient
    /// aucune. `the_typescript_client_of_hello_crud_is_what_the_cli_produces_today` le
    /// rejoue vraiment, et répond de cette exclusion.
    engendre_a_part: &'static [&'static str],
```

le renseigner à `&[]` partout sauf sur `hello-crud`, où il vaut
`&["clients/ts/client.ts"]`, et l'ajouter au filtre de `normalize_fingerprint` à côté
d'`edite_a_la_main` :

```rust
        .filter(|(chemin, _)| {
            !example
                .engendre_a_part
                .iter()
                .any(|engendre| chemin.as_path() == Path::new(engendre))
        })
```

Puis le test qui en répond, dans le même fichier. Il réutilise `generate`, qui engendre
déjà un `hello-crud` frais avec `common::noyau()` en `--core-path` : le projet temporaire
pointe donc le noyau du dépôt, et rien n'est à réécrire.

```rust
/// Le client versionné est-il encore ce que `rbs generate client` produit ?
///
/// `#[ignore]` : il compile le projet de bout en bout pour lire son document OpenAPI,
/// ce qu'aucun autre test de ce fichier ne fait.
#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn the_typescript_client_of_hello_crud_is_what_the_cli_produces_today() {
    let parent = tempfile::TempDir::new().expect("répertoire temporaire créable");
    let frais = generate(parent.path(), example("hello-crud"));

    assert_cmd::Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&frais)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    let obtenu = std::fs::read_to_string(frais.join("clients/ts/client.ts")).expect("client rendu");
    let attendu =
        std::fs::read_to_string(common::depot().join("examples/hello-crud/clients/ts/client.ts"))
            .expect("le client versionné doit exister");

    // L'exemple porte les marqueurs que la documentation cite ; le rendu frais, non.
    assert_eq!(normalize(&attendu), normalize(&obtenu), "{REGENERER}");
}
```

`normalize` est déjà dans le fichier et filtre les marqueurs de région.

- [ ] **Step 6 : engendrer le client de `hello-crud` et le verser dans l'exemple**

Lancer le test qu'on vient d'écrire : il échouera en montrant que le fichier versionné
n'existe pas. Pour l'obtenir sans travail en double, engendrer le projet à la main :

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad
RACINE=$PWD
cargo run -p rbs-cli --bin rbs -- new "$S/client-ts-hc/hello-crud" --yes \
  --core-path "$RACINE/crates/rbs-core" \
  --database-url 'postgres://rbs:rbs@localhost:5432/hello_crud' --lang fr
cd "$S/client-ts-hc/hello-crud"
cargo run --manifest-path "$RACINE/Cargo.toml" -p rbs-cli --bin rbs -- \
  generate crud articles --fields 'title:string,body:text,published:bool' --force
cargo run --manifest-path "$RACINE/Cargo.toml" -p rbs-cli --bin rbs -- \
  generate client --lang ts --force
```

Copier `clients/ts/client.ts` dans `examples/hello-crud/clients/ts/client.ts`. Vérifier
qu'il ne porte **aucun chemin absolu** :

```bash
grep -n "$S\|/Users/" examples/hello-crud/clients/ts/client.ts; echo "exit=$?"
```

Attendu : exit 1, aucune ligne.

Y poser ensuite les marqueurs de région que la documentation citera — même convention que
les autres exemples, `// region: nom` / `// endregion: nom`. Trois régions : `options`
(l'interface `ApiClientOptions`), `erreur` (la classe `ApiError`), `articles` (les cinq
méthodes de la ressource).

- [ ] **Step 6 bis : vérifier que ce client compile vraiment**

```bash
mkdir -p "$S/client-ts-tsc" && cp examples/hello-crud/clients/ts/client.ts "$S/client-ts-tsc/"
cd "$S/client-ts-tsc" && npx -y typescript@5 tsc --strict --noEmit \
  --target es2022 --lib es2022,dom client.ts
```

Attendu : exit 0, aucune sortie. Vérification locale — CI n'a pas de toolchain Node hors
de `docs/`. Si `npx` échoue faute de réseau, le dire, et ne pas la présenter comme faite.

- [ ] **Step 7 : lancer les deux suites**

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad
cargo test -p rbs-cli --test integration_examples > "$S/client-ts-examples.txt" 2>&1
cargo test -p rbs-cli --test integration_examples -- --ignored --no-fail-fast \
  > "$S/client-ts-examples-ignored.txt" 2>&1
tail -5 "$S/client-ts-examples.txt" "$S/client-ts-examples-ignored.txt"
```

Attendu : 4 passés puis 1 passé, 0 échoué de part et d'autre.

- [ ] **Step 8 : documenter la régénération**

Dans `examples/README.md`, section `hello-crud`, ajouter après les deux commandes
existantes :

```bash
cd examples/hello-crud && cargo run --manifest-path ../../Cargo.toml -p rbs-cli --bin rbs -- \
  generate client --lang ts --force
```

avec une ligne de prose disant que le client est le seul fichier d'`examples/` qu'une
commande engendre sans que le test de non-dérive rapide le rejoue, et pourquoi.

- [ ] **Step 9 : commit**

```
build(examples): remet les quatre exemples au niveau des templates

Le binaire `openapi` et les `operation_id` changent ce que le CLI
produit : les exemples versionnés en sont la seule preuve, et un exemple
périmé fait mentir une documentation qui n'écrit aucune ligne à la main.
`hello-crud` gagne en prime le client TypeScript, d'où le site lira ses
extraits.

Le handler `broadcast` de newsletter-queue est écrit à la main : son
`operation_id` l'est aussi.

Vérifications :
`cargo test -p rbs-cli --test integration_examples` : 4 passés, 0 échoué.
Le même avec `--ignored --no-fail-fast` : 1 passé, 0 échoué.
```

---

### Task 8 : la commande, de bout en bout

**Files:**
- Create: `crates/rbs-cli/tests/integration_client.rs`

**Interfaces:**
- Consumes: le CLI complet.
- Produces: rien que d'autres tâches lisent.

Ces tests compilent un projet engendré : ils portent tous `#[ignore]`, comme les autres
tests lents de `crates/rbs-cli/tests/`. Ils n'ont pas besoin de Docker — aucune base n'est
touchée.

- [ ] **Step 1 : écrire les tests**

```rust
//! `rbs generate client` sur un vrai projet, compilé.
//!
//! Le rendu se prouve dans `src/client/ts.rs`, sans rien compiler. Ce fichier prouve ce
//! que seul un projet réel prouve : que le binaire `openapi` compile, que sa sortie
//! s'analyse, et que le fichier écrit est bien celui que le plan annonçait.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

const CLIENT: &str = "clients/ts/client.ts";

/// Le binaire livré, lancé dans `racine`.
fn rbs(racine: &Path) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(racine);
    commande
}

/// Un projet neuf portant une feature CRUD, prêt à recevoir son client.
///
/// Le `TempDir` est rendu avec la racine : le lâcher effacerait le projet sous le test.
fn projet_avec_crud() -> (TempDir, PathBuf) {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());

    rbs(&racine)
        .args([
            "generate",
            "crud",
            "articles",
            "--fields",
            "title:string,body:text,published:bool",
            "--force",
        ])
        .assert()
        .success();

    (parent, racine)
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn the_command_writes_a_client_that_carries_one_method_per_operation() {
    let (_parent, racine) = projet_avec_crud();

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    let client = fs::read_to_string(racine.join(CLIENT)).expect("le client doit être écrit");

    for methode in [
        "articlesList",
        "articlesCreate",
        "articlesFind",
        "articlesUpdate",
        "articlesDelete",
        "health",
    ] {
        assert!(client.contains(methode), "`{methode}` manque :\n{client}");
    }
    assert!(client.contains("export interface ArticleResponse"), "{client}");
    assert!(client.contains("export class ApiClient"), "{client}");
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn running_the_command_twice_writes_nothing_the_second_time() {
    let (_parent, racine) = projet_avec_crud();

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    let avant = common::empreinte(&racine);

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    // `target/` est hors de l'empreinte : la seconde compilation ne s'y voit pas.
    common::assert_intact(&avant, &racine, "une seconde génération du client");
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn a_hand_edited_client_goes_into_conflict_until_force() {
    let (_parent, racine) = projet_avec_crud();

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    fs::write(racine.join(CLIENT), "// retouché à la main\n").expect("client réécrit");
    common::commiter(&racine, "client retouché");

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts"])
        .assert()
        .failure();

    assert_eq!(
        fs::read_to_string(racine.join(CLIENT)).expect("client"),
        "// retouché à la main\n",
        "le conflit ne doit rien écraser"
    );

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .success();

    assert!(
        fs::read_to_string(racine.join(CLIENT))
            .expect("client")
            .contains("export class ApiClient"),
        "`--force` doit reprendre la main"
    );
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn dry_run_writes_nothing() {
    let (_parent, racine) = projet_avec_crud();
    let avant = common::empreinte(&racine);

    rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force", "--dry-run"])
        .assert()
        .success();

    common::assert_intact(&avant, &racine, "une génération en --dry-run");
    assert!(
        !racine.join(CLIENT).exists(),
        "`--dry-run` ne doit pas créer le client"
    );
}

#[test]
#[ignore = "compile le projet engendré pour lire son document OpenAPI"]
fn an_explicit_out_directory_is_honoured() {
    let (_parent, racine) = projet_avec_crud();

    rbs(&racine)
        .args([
            "generate", "client", "--lang", "ts", "--out", "web/api", "--force",
        ])
        .assert()
        .success();

    assert!(racine.join("web/api/client.ts").exists());
    assert!(
        !racine.join(CLIENT).exists(),
        "`--out` doit déplacer la sortie, non la dupliquer"
    );
}

/// Pas d'`#[ignore]` : le refus arrive avant que cargo ne soit lancé, et c'est le point.
#[test]
fn a_project_without_the_openapi_binary_is_refused_before_cargo_runs() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());

    fs::remove_file(racine.join("src/bin/openapi.rs")).expect("binaire supprimable");

    let sortie = rbs(&racine)
        .args(["generate", "client", "--lang", "ts", "--force"])
        .assert()
        .failure()
        .get_output()
        .clone();

    let rendu = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    assert!(rendu.contains("src/bin/openapi.rs"), "{rendu}");
    assert!(rendu.contains("[[bin]]"), "{rendu}");
    assert!(
        !racine.join(CLIENT).exists(),
        "un refus ne doit rien écrire"
    );
}
```

Si `common::projet` ne rend pas le chemin attendu ou si `assert_intact` a une autre
signature, reprendre celle du fichier `crates/rbs-cli/tests/common/mod.rs` plutôt que
celle écrite ici.

- [ ] **Step 2 : lancer la suite lente**

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad
cargo test -p rbs-cli --test integration_client -- --ignored --no-fail-fast \
  > "$S/client-ts-integration.txt" 2>&1
tail -20 "$S/client-ts-integration.txt"
```

Attendu : 5 passés, 0 échoué. `--no-fail-fast` est obligatoire.

- [ ] **Step 3 : commit**

```
test(client): prouve la commande sur un projet réellement compilé

Le rendu se prouve sans compiler ; que le binaire `openapi` compile,
que sa sortie s'analyse et que le fichier écrit soit celui du plan ne se
prouve que sur un vrai projet. Le refus sur un projet sans binaire reste
hors du lot lent : il arrive avant cargo, et c'est le point.

Vérifications :
`cargo test -p rbs-cli --test integration_client -- --ignored --no-fail-fast` :
5 passés, 0 échoué.
`cargo test -p rbs-cli --test integration_client` : 1 passé, 5 filtrés.
```

---

### Task 9 : la documentation, bilingue

**Files:**
- Create: `docs/docs/cli/client.md`
- Create: `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/client.md`
- Modify: `docs/docs/guides/openapi.md` et sa traduction
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md`

**Interfaces:**
- Consumes: `examples/hello-crud/clients/ts/client.ts` et ses régions (tâche 7).
- Produces: rien.

- [ ] **Step 1 : écrire la page anglaise**

`docs/docs/cli/client.md`, `sidebar_position` à la suite des autres pages du répertoire
(lire `_category_.json` et les `sidebar_position` existants avant de choisir).

Elle doit couvrir, dans cet ordre : ce que la commande fait, l'invocation, d'où vient le
document (le binaire `openapi`, et le fait qu'aucun serveur ne tourne), les trois drapeaux,
la forme du client, l'authentification, les erreurs, la régénération et le conflit, les
deux refus (sans bibliothèque, sans binaire).

Les extraits de code TypeScript viennent d'`examples/`, jamais d'une ligne écrite à la
main :

````
```ts file=examples/hello-crud/clients/ts/client.ts region=options
```
````

Vérifier la syntaxe exacte de l'inclusion sur une page existante — `docs/docs/guides/openapi.md`
emploie `” ```rust file=… region=… ”`. Un exemple d'usage écrit à la main est acceptable
là où aucun fichier d'`examples/` ne le porte : le dire, ou l'éviter.

- [ ] **Step 2 : traduire**

`docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/client.md`, même structure, mêmes
inclusions, même `sidebar_position`.

- [ ] **Step 3 : renvoyer depuis le guide OpenAPI**

Dans `docs/docs/guides/openapi.md`, le paragraphe qui dit que la combinaison « document
seul » est « the one that generates clients or checks a contract from CI » doit désormais
renvoyer à la page de la commande, et dire que rbs engendre le client lui-même sans passer
par HTTP. Même retouche dans la version française.

- [ ] **Step 4 : le changelog, dans les deux langues**

Sous `## [Unreleased]` / `### Added` de `CHANGELOG.md` et `CHANGELOG.fr.md`. Y dire aussi,
sous `### Fixed`, les `operationId` homonymes et le tag du handler de santé — c'est un
changement observable du document que sert un projet déjà déployé.

- [ ] **Step 5 : vérifier la parité et la construction du site**

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad
cd docs && node scripts/parite.mjs > "$S/client-ts-parite.txt" 2>&1; tail -20 "$S/client-ts-parite.txt"
```

puis, si les dépendances du site sont installées :

```bash
cd docs && npm run build > "$S/client-ts-docs-build.txt" 2>&1; tail -20 "$S/client-ts-docs-build.txt"
```

Attendu : parité sans écart, build sans erreur. Le build échoue si une inclusion
`file=…region=…` ne trouve pas sa région — c'est la vérification qui compte. Si les
dépendances ne sont pas installées et que l'installation échoue, **le dire** plutôt que de
conclure.

- [ ] **Step 6 : commit**

```
docs(client): documente `rbs generate client --lang ts`

Les extraits viennent du client versionné d'examples/hello-crud, comme
partout ailleurs sur le site. Le guide OpenAPI renvoyait à un client
qu'il fallait engendrer soi-même depuis le document servi ; rbs le fait.

Vérifications :
`node docs/scripts/parite.mjs` : aucun écart.
`npm run build` dans docs/ : sans erreur.
```

---

## Vérification finale

Avant toute affirmation de succès, appliquer `superpowers:verification-before-completion`
et lancer, en redirigeant vers le scratchpad :

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad
cargo fmt --all --check                              > "$S/client-ts-fmt.txt" 2>&1
cargo clippy --workspace --all-targets -- -D warnings > "$S/client-ts-clippy.txt" 2>&1
cargo test --workspace                               > "$S/client-ts-tests.txt" 2>&1
cargo test --workspace -- --ignored --no-fail-fast    > "$S/client-ts-tests-ignored.txt" 2>&1
grep -E "^(test result|error)" "$S"/client-ts-*.txt
```

Reporter les chiffres exacts — passés, échoués, filtrés — et non « tout est vert ». La
suite `--ignored` exige Docker : si Docker n'est pas là, le dire, et ne pas présenter la
suite rapide comme si elle prouvait la lente.
