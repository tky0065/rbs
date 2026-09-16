# Types `date`, `decimal` et `enum(…)` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal :** `--fields` accepte `date`, `enum(a,b,c)` et `decimal`, rendus jusqu'au bout —
modèle, migration, DTO, OpenAPI, filtre, seed, tests engendrés, client TypeScript.

**Architecture :** trois livraisons successives, chacune verte et committée : `date`, puis
`enum`, puis `decimal`. Les correspondances de types restent centralisées dans
`FieldType` (`crates/rbs-cli/src/generate/fields.rs`) ; le noyau ne reçoit que des schémas
de documentation et l'opérateur `OneOf`, jamais de traduction en requête.

**Tech Stack :** Rust 2024, minijinja (délimiteurs alternatifs `{@ @}`), SeaORM 2.0,
sea-query, utoipa 5, chrono, rust_decimal, testcontainers.

**Spec :** `docs/superpowers/specs/2026-09-15-types-date-decimal-enum-design.md`

## Global Constraints

- Un type nouveau se déclare dans `FieldType` et dans **toutes** ses méthodes (`NAMES`,
  `parse`, `name`, `rust_type`, `migration_method`, `column_type_attribute`) ; les
  consommateurs à mettre à jour sont `generate/filter.rs`, `generate/seed.rs`,
  `generate/tests_http.rs` et `generate/fields/error.rs`. Un `match` non exhaustif ne
  compile pas : c'est le garde-fou, ne l'affaiblissez pas par un `_ =>`.
- Aucun exemple d'`examples/` ne porte ces types : `integration_examples` doit rester à
  22/0 sans qu'un fichier d'exemple change. Si une fixture de
  `crates/rbs-cli/fixtures/` change, c'est un signal : dites pourquoi dans le rapport.
- Le noyau (`crates/rbs-core`) ne gagne que des schémas de documentation et l'opérateur
  `OneOf` ; `#![warn(missing_docs)]` y impose un `///` d'une à trois lignes sur chaque item
  public. Ajouts rétrocompatibles, dans la 1.5.0 non publiée.
- Documentation bilingue : toute page anglaise touchée l'est en français dans le même
  commit (`docs/docs/…` et `docs/i18n/fr/docusaurus-plugin-content-docs/current/…`).
- Commits : Conventional Commits, sujet en français à l'impératif, sans majuscule ni point
  final ; corps = pourquoi + `Vérifications :` avec les commandes et leurs résultats réels.
  Jamais d'identifiant de tâche, de renvoi à un fichier de suivi, ni de ligne
  `Co-Authored-By`/`Claude-Session`.
- Commandes longues au premier plan, délai 600000 ms, sortie redirigée vers le scratchpad
  `/private/tmp/claude-501/-Users-yacoubakone-dev-rs/0b6e0e7f-013a-42af-a9d8-9bc0f246cf81/scratchpad`,
  puis `grep` ; jamais d'arrière-plan, jamais de tour terminé en attente d'un processus.
- Bloquants : `cargo fmt --all --check` et
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Connu et hors sujet : `aws-smithy-types` 1.7.0 casse la compilation de tout projet
  engendré portant `storage` ; les tests Docker qui en dépendent échouent pour cette seule
  raison.

## Repères

- Grammaire et types : `src/generate/fields.rs` (`FieldType` l.10-100, `parse` l.340,
  `parse_field` l.397, `impl Serialize for Field` l.304), fautes dans
  `src/generate/fields/error.rs`.
- Rendu : `templates/feature/model.rs.jinja` (modèle), `migration.rs.jinja` (colonnes,
  `field.migration_method` et les trois formes selon la longueur), `dto.rs.jinja`,
  `filter.rs.jinja`, `tests/*.rs.jinja` ; générateurs correspondants sous
  `src/generate/`.
- Filtre : `src/generate/filter.rs` (`schema`, `textual`, `scalar_type`), noyau
  `crates/rbs-core/src/filter/schema.rs` (macro `comparaison_documentee!`) et
  `filter/mod.rs` (`Comparison`, `TextMatch`).
- Doc : table des types `docs/docs/cli/generate.md:214-221`, aide de `--fields` (texte
  clap, `src/cli.rs`), opérateurs `docs/docs/guides/filtering.md:41`.
- Dépendances d'un projet : `plan::PatchToml::AjouterDependance` et
  `AjouterFeatureADependance` (précédent : `src/add/installation.rs:171-183`) ;
  `generate` construit déjà son `plan::Builder` (`src/generate/command.rs:438-457`).

---

### Task 1 : `date`

**Files :**
- Modify : `src/generate/fields.rs` (variante `Date`, `NAMES`, `parse`, `name`,
  `rust_type` → `Date`, `migration_method` → `date()`), `src/generate/filter.rs`
  (`schema` → `DateComparisonSchema`, `scalar_type` → `Date`),
  `src/generate/seed.rs`, `src/generate/tests_http.rs` (valeurs de création et de
  modification)
- Modify : `crates/rbs-core/src/filter/schema.rs` (`DateSchema` sur le modèle de
  `DateTimeSchema`, puis `comparaison_documentee!(DateComparisonSchema, …)`),
  `crates/rbs-core/src/filter/mod.rs` (réexport)
- Modify : `docs/docs/cli/generate.md` (table des types), `docs/docs/guides/filtering.md`
  (table des opérateurs), `src/cli.rs` (aide de `--fields`), et les jumelles françaises

**Interfaces :** Produces `FieldType::Date`, `rbs_core::DateComparisonSchema` — la tâche 3
suit le même chemin pour `Decimal`.

- [ ] **Step 1 :** test rouge dans `fields.rs` : `FieldType::parse("date")` rend
  `Some(FieldType::Date)`, et un champ `due:date` se rend en `Date` dans le modèle et en
  `.date()` dans la migration (tests de `entity.rs` et `migration.rs` sur le modèle des
  tests `datetime` existants). `cargo test -p rbs-cli --lib` → échec attendu.
- [ ] **Step 2 :** ajouter la variante et ses correspondances ; compiler jusqu'à ce que
  tous les `match` exhaustifs soient traités.
- [ ] **Step 3 :** noyau : `DateSchema` (format `date`) et `DateComparisonSchema`, avec
  leurs `///` ; test du schéma OpenAPI sur le modèle de celui de `DateTimeSchema`
  (`schema.rs`, module `tests`). `cargo test -p rbs-core --all-features` → 0 échec.
- [ ] **Step 4 :** filtre, seed, tests engendrés ; `cargo test -p rbs-cli --lib` → 0 échec.
- [ ] **Step 5 :** doc (table des types, opérateurs, aide clap) en anglais et en français ;
  `npm run build` sous `docs/` → exit 0 ; `node docs/scripts/parite.mjs` → 0 écart ;
  `cargo test -p rbs-cli --test integration_docs --no-fail-fast -- --include-ignored` si
  une transcription marquée change (elle change si l'aide de `--fields` est transcrite).
- [ ] **Step 6 :** `cargo test -p rbs-cli --test integration_examples` → 22/0 ; fmt,
  clippy ; commit `feat(generate): accepte le type date dans --fields`.

### Task 2 : la grammaire `enum(a,b,c)`

**Files :**
- Modify : `src/generate/fields.rs` — découpage de `--fields` qui ignore une virgule entre
  parenthèses (`parse` l.352), `FieldType::Enum(Vec<String>)` ou `FieldKind::Enum` selon ce
  que le code rend le plus simple (le type porte ses valeurs), `parse_field` qui lit
  `enum(a,b,c)`, `Serialize for Field` qui expose `enum_variants` (valeurs) et
  `enum_type` (nom PascalCase du champ)
- Modify : `src/generate/fields/error.rs` — fautes nouvelles, chacune avec son message :
  parenthèse non fermée, liste vide, valeur non snake_case, valeurs en double, nom de champ
  dont la forme PascalCase heurte un type du modèle engendré (`Model`, `ActiveModel`,
  `Entity`, `Column`, `PrimaryKey`, `Relation`)

**Interfaces :** Produces le champ sérialisé `enum_variants: ["draft","published"]` et
`enum_type: "Status"`, que la tâche 3 rend.

- [ ] **Step 1 :** tests rouges dans `fields.rs` : `parse("status:enum(draft,published)")`
  rend un champ portant ses deux valeurs ; `parse("a:string,status:enum(x,y),b:int")` rend
  trois champs ; chaque faute nouvelle rend son `ErrorKind` et son message. Lancer
  `cargo test -p rbs-cli --lib fields` → échecs attendus.
- [ ] **Step 2 :** implémenter le découpage et l'analyse ; les tests passent.
- [ ] **Step 3 :** `cargo test -p rbs-cli --lib` → 0 échec (aucun autre consommateur ne
  doit régresser ; un `match` non traité se voit à la compilation).
- [ ] **Step 4 :** fmt, clippy ; commit `feat(generate): lit un type enum et ses valeurs
  dans --fields`.

### Task 3 : le rendu d'`enum`

**Files :**
- Modify : `templates/feature/model.rs.jinja` (déclaration `DeriveActiveEnum` par champ
  `enum`, avant la struct `Model` ; `rs_type = "String"`,
  `db_type = "String(StringLen::N(n))"`, un `string_value` par variante, dérives
  `Serialize`/`Deserialize`/`ToSchema`), `migration.rs.jinja` (colonne chaîne de longueur
  `n` et `.check(Expr::col(…).is_in([…]))`), `src/generate/entity.rs`,
  `src/generate/migration.rs`
- Modify : `src/generate/filter.rs` (schéma `OneOfSchema`, opérateur `OneOf<…>`),
  `templates/feature/filter.rs.jinja` (fonction `one_of` : `eq`, `in`, `is_null`)
- Modify : `crates/rbs-core/src/filter/mod.rs` (`OneOf<T>` avec sa lecture d'une valeur nue
  ou d'un objet, sur le modèle de `Comparison`), `filter/schema.rs` (`OneOfSchema`)
- Modify : `src/generate/seed.rs`, `src/generate/tests_http.rs` (première valeur à la
  création, deuxième à la modification), `src/client/ts.rs` (test seulement : une
  énumération de chaînes rend déjà une union de littéraux)

- [ ] **Step 1 :** tests rouges : rendu du modèle (l'énumération et son attribut), de la
  migration (longueur et `CHECK`), du DTO, du filtre, du seed et des tests engendrés pour
  `status:enum(draft,published)` ; lecture de `OneOf` dans le noyau (valeur nue, objet,
  `in` vide, `is_null`).
- [ ] **Step 2 :** noyau d'abord (`OneOf`, `OneOfSchema`, réexports, `///`), puis les
  générateurs et les templates ; `cargo test -p rbs-core --all-features` et
  `cargo test -p rbs-cli --lib` → 0 échec.
- [ ] **Step 3 :** `cargo test -p rbs-cli --test integration_examples` → 22/0 ; fmt,
  clippy.
- [ ] **Step 4 :** doc : la table des types gagne `enum(a,b,c)`, la table des opérateurs
  gagne `in` ; anglais et français ; `npm run build`, `parite.mjs`.
- [ ] **Step 5 :** commit `feat(generate): engendre une énumération SeaORM et sa contrainte`.

### Task 4 : `decimal`

**Files :**
- Modify : `src/generate/fields.rs` (variante `Decimal`, `rust_type` → `Decimal`,
  `migration_method` → `decimal_len(19, 4)`), `src/generate/filter.rs`,
  `src/generate/seed.rs`, `src/generate/tests_http.rs`
- Modify : `src/generate/command.rs` — refus avant toute écriture quand le projet est sous
  SQLite et qu'un champ est `decimal` (`Error` nouvelle, code d'erreur, message nommant le
  champ, la raison et les deux replis `float` ou entier en centimes) ; ajout de la
  dépendance `rust_decimal` et de la feature `with-rust_decimal` de `sea-orm` par
  `plan::PatchToml::AjouterDependance` / `AjouterFeatureADependance` quand un champ est
  `decimal`
- Modify : `crates/rbs-core/src/filter/schema.rs` (`DecimalComparisonSchema`, documenté
  comme une chaîne — le noyau ne dépend pas de `rust_decimal`), `filter/mod.rs`
- Modify : `templates/project/Cargo.toml.jinja` si la feature `with-rust_decimal` doit
  aussi exister dans un projet neuf — sinon ne rien y toucher et laisser le patch faire

- [ ] **Step 1 :** tests rouges : `parse("price:decimal")`, rendu du modèle, de la
  migration (`decimal_len(19, 4)`), du filtre, du seed et des tests ; refus sous SQLite
  (test de `command.rs` sur le modèle de `UploadSansStorage`) ; patch du manifeste (test du
  plan rendu : la dépendance et la feature apparaissent).
- [ ] **Step 2 :** implémenter ; `cargo test -p rbs-cli --lib` et
  `cargo test -p rbs-core --all-features` → 0 échec.
- [ ] **Step 3 :** `integration_examples` 22/0 ; fmt, clippy.
- [ ] **Step 4 :** doc : table des types, opérateurs, aide clap, et la phrase qui dit
  pourquoi SQLite refuse ; anglais et français ; `npm run build`, `parite.mjs`.
- [ ] **Step 5 :** commit `feat(generate): accepte le type decimal hors SQLite`.

### Task 5 : preuve sur les trois moteurs, notes de version, passe finale

**Files :**
- Modify : `crates/rbs-cli/tests/integration_crud.rs` — un banc qui engendre un CRUD
  portant `due:date`, `status:enum(draft,published)` et `price:decimal`, le migre et lance
  ses tests contre PostgreSQL puis MySQL ; le même sans `decimal` sous SQLite ; un banc
  court qui éprouve le refus de `decimal` sous SQLite (sans Docker si possible)
- Modify : `CHANGELOG.md`, `CHANGELOG.fr.md` (section 1.5.0, « Added »/« Ajouté »),
  `crates/rbs-cli/notes/1.5.0.md` — les trois types, et la demi-phrase laissée en suspens
  par la revue précédente : dans la section « La connexion exige une adresse vérifiée », le
  rattrapage de `mark_verified` dans `reset` vaut « ou avec `db` si `reset` n'a pas de
  transaction »

- [ ] **Step 1 :** écrire les bancs ; les lancer un par un, au premier plan, sortie dans le
  scratchpad : PostgreSQL, MySQL, SQLite.
- [ ] **Step 2 :** CHANGELOG et note.
- [ ] **Step 3 :** passe finale : fmt, clippy, `cargo test --workspace`,
  `cargo test -p rbs-core --all-features`, `--lib`, `integration_examples`,
  `integration_docs`, `npm run build`, `parite.mjs`, puis les suites Docker touchées une
  par commande.
- [ ] **Step 4 :** commit `test(generate): éprouve les trois types nouveaux sur les trois
  moteurs` puis `docs: décrit les types date, enum et decimal`.
