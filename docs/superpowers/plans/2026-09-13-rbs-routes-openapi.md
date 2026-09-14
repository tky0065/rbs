# `rbs routes` et `rbs openapi export` — plan d'implémentation

> **Pour les agents :** sous-skill requis : `superpowers:executing-plans`.
> Les étapes se suivent en cases `- [ ]`.

**But :** montrer méthode, chemin, `operation_id` et garde de chaque route en une
commande (`rbs routes [--json]`), et sortir le document OpenAPI sans serveur
(`rbs openapi export [--out FICHIER]`).

**Architecture :** l'obtention du document — les deux refus (projet sans bibliothèque,
binaire `openapi` absent) puis `cargo run --quiet --bin openapi` — quitte `client/mod.rs`
pour un module `openapi.rs` que `client`, `routes` et `openapi export` appellent.
`client/document.rs` garde l'analyse, étendue à la `security` globale du document.
`routes.rs` réduit le `Document` en lignes triées et les rend en table ou en JSON.

**Spec :** `docs/superpowers/specs/2026-09-13-lot-features-p2-design.md`, section « 39 ».

## Contraintes globales

- Table `MÉTHODE  CHEMIN  OPERATION_ID  GARDE`, tri par chemin puis méthode (GET, POST,
  PUT, PATCH, DELETE ; les autres verbes après, dans l'ordre de `VERBES`).
- `GARDE` = `bearer` quand l'opération (ou, à défaut de clé propre, le document) déclare
  une `security` non vide ; `public` sinon. Une `security: []` explicite sur l'opération
  la rend publique (sémantique OpenAPI : elle remplace la globale).
- `--json` : tableau d'objets `{ "methode", "chemin", "operation_id", "garde" }`, seul
  document de la sortie standard (la compilation du projet reste sur stderr).
- Messages des refus inchangés mot pour mot (`integration_client` les fige).
- Ne pas déborder sur la tâche 65 : les `#[allow(dead_code)]` de `client/document.rs`
  restent, seul le commentaire que l'extension de `parse` rend faux (« que ce lot ne lit
  pas encore ») change.
- Doc bilingue dans le même commit ; commit sans identifiant de tâche ni attribution.

---

### Tâche 1 : module `openapi` (obtention factorisée)

**Fichiers :** Créer `crates/rbs-cli/src/openapi.rs` ; modifier `crates/rbs-cli/src/client/mod.rs`, `crates/rbs-cli/src/lib.rs` (`mod openapi;`).

- [ ] Déplacer `BINAIRE`, `BIBLIOTHEQUE`, `imprime_le_document` et les variantes
  `SansBibliotheque`, `SansBinaire`, `Cargo`, `BinaireEnEchec` (+ le remède de
  `SansBinaire`) dans `openapi::Obtention` ; `pub(crate) fn imprimer(root: &Path) ->
  Result<String, Obtention>` fait les deux refus dans cet ordre puis lance cargo.
- [ ] `client::Error` : les quatre variantes deviennent `#[error(transparent)]
  Openapi(#[from] crate::openapi::Obtention)` ; `remedy()` délègue.
- [ ] Les deux tests existants de `client` restent verts sans changement d'assertion ;
  deux tests de `openapi` : projet sans `src/lib.rs` → `Obtention::SansBibliotheque`,
  sans binaire → `SansBinaire` et remède contenant `[[bin]]`.

### Tâche 2 : `security` globale dans l'analyse

**Fichiers :** `crates/rbs-cli/src/client/document.rs`.

- [ ] Tests rouges : `a_global_security_marks_the_operations_that_declare_none`,
  `an_explicit_empty_security_makes_an_operation_public_despite_the_global_one`.
- [ ] `parse` lit `value.get("security")` (tableau non vide) et le passe à
  `parse_path_item` → `parse_operation(value, globale: bool)` :
  `secured = match value.get("security").and_then(Value::as_array) { Some(e) => !e.is_empty(), None => globale }`.
  Le commentaire « que ce lot ne lit pas encore » est remplacé par la règle.

### Tâche 3 : `routes.rs` (réduction, tri, rendus)

**Fichiers :** Créer `crates/rbs-cli/src/routes.rs`.

**Interfaces :** `struct Route { methode: String, chemin: String, operation_id: Option<String>, garde: Garde }` (`Serialize`, `garde` en `"bearer"`/`"public"`) ; `fn lister(&Document) -> Vec<Route>` ; `fn table(&[Route]) -> String` ; `fn json(&[Route]) -> String` ; `fn run(directory: &Path, json: bool) -> Result<String, openapi::Error>`.

- [ ] Tests rouges sur un document fixe (`/users` GET+POST sécurisés, `/health` GET
  public, `/users/{id}` DELETE+GET+PATCH+PUT, une opération sans `operationId`) :
  ordre attendu, garde, table exacte (colonnes alignées sur la plus longue valeur,
  deux espaces, `-` pour un `operation_id` absent, aucune espace en fin de ligne),
  JSON réanalysé par `serde_json` avec les quatre clés et `operation_id: null` si absent.
- [ ] Implémenter ; largeur de colonne en `chars().count()` (« MÉTHODE » porte un É).

### Tâche 4 : commandes `routes` et `openapi export`

**Fichiers :** `crates/rbs-cli/src/cli.rs`, `crates/rbs-cli/src/lib.rs`, `crates/rbs-cli/src/openapi.rs`.

- [ ] `openapi::Error` (commandes) : `PasUnProjet`, `Metadata`, `Obtention(#[from])`,
  `Document(#[from] client::document::Erreur)`, `Acces(#[from] errors::Acces)` ;
  `depuis_la_racine!` ; `remedy()` délègue à `Obtention`.
- [ ] `openapi::document(directory) -> Result<(PathBuf, String), Error>` :
  `metadata::project_root` puis `imprimer`. `openapi::exporter(directory, out:
  Option<&Path>) -> Result<Option<String>, Error>` : analyse le texte par
  `document::parse` (refus d'écrire autre chose qu'un document), l'écrit dans `out`
  (relatif au répertoire courant) et rend `None`, ou rend `Some(texte)`.
- [ ] CLI : `Routes { #[arg(long)] json: bool }` ; `Openapi { #[command(subcommand)]
  command: OpenapiCommands }` avec `Export { #[arg(long, value_name = "FICHIER")] out:
  Option<PathBuf> }`. Tests de parsing + `"routes"`, `"openapi"` dans la liste d'aide.
- [ ] `lib.rs` : erreur sur stderr, remède sur stderr quand `--json` (la sortie standard
  ne porte que le document), code 1.
- [ ] Test `assert_cmd` rapide (`crates/rbs-cli/tests/integration_routes.rs`) : binaire
  retiré → `rbs routes` et `rbs openapi export` refusent en nommant `src/bin/openapi.rs`.
- [ ] Test lent `#[ignore]` : projet `routes-api --with auth`, `rbs routes --json` →
  au moins un `bearer`, `GET /health` en `public` ; `rbs openapi export --out doc.json` →
  fichier analysable portant `openapi` ; `rbs routes` (table) contient l'en-tête.

### Tâche 5 : documentation et vérification

- [ ] `docs/docs/cli/routes.md` (`sidebar_position: 2.6`), `docs/docs/cli/openapi.md`
  (`2.7`), jumelles FR ; synopsis = `--help` réel, extrait de table réel.
  `cli/client.md` (EN+FR) : le gel du contrat en CI renvoie à `rbs openapi export`.
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, passe lente `integration_routes` et `integration_client`
  (un binaire par commande), `npm run build`.
- [ ] Commit `feat(cli): ajoute rbs routes et rbs openapi export`.
