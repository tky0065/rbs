# Relations entre entités — niveau schéma

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs g crud posts --fields "author:references:users"` engendre la colonne `author_id`, sa clé étrangère, son index, la variante `Relation::Author` et l'`impl Related` — et écrit la variante inverse dans le modèle de la cible.

**Architecture:** Le parseur de `--fields` gagne un huitième type dont le troisième segment est une cible. Sa validation se coupe en deux : `fields::parse` reste pure et syntaxique, `relations::resolve` confronte les cibles à un inventaire des entités lu sur le disque. Le rendu réemploie les templates existantes, la colonne dérivée `author_id` étant exposée aux templates sous le nom `name` — les blocs de colonne ne changent donc pas d'une ligne. Seule l'écriture du côté inverse est nouvelle : elle passe par le mécanisme d'ancres existant, rendu capable de viser un fichier calculé.

**Tech Stack:** Rust 2024, SeaORM (`DeriveRelation`, `Related`, `sea_orm_migration`), minijinja avec délimiteurs alternatifs `{@ @}`, `insta` non utilisé — les assertions sont des `contains` sur le rendu, comme dans tout le lot `generate`.

**Spec:** `docs/superpowers/specs/2026-08-29-relations-design.md`

## Global Constraints

- **Branche `relations-entre-entites`.** Jamais de commit sur `main`.
- **Conventional Commits**, sujet en français à l'impératif, sans majuscule ni point final. Aucun identifiant de tâche, aucun renvoi à ce fichier ni à `TODO.md`, jamais de `Co-Authored-By` ni de mention d'un assistant. Corps portant le *pourquoi*, puis un intertitre `Vérifications :` avec les commandes lancées et leur résultat réel.
- **Identifiants Rust en anglais**, sans exception, selon `docs/superpowers/plans/2026-08-28-glossaire-migration-anglais.md`. Commentaires, doc-comments et messages destinés à l'utilisateur restent **en français** : le glossaire les met hors périmètre.
- **Noms de tests en phrase**, anglais : `a_reference_without_a_target_is_rejected`. Jamais `test_relation_1`.
- **Un commentaire explique le *pourquoi*.** Un commentaire qui paraphrase la ligne suivante se supprime.
- Bloquant avant chaque commit : `cargo fmt --all --check` et `cargo clippy --workspace --all-targets -- -D warnings`.
- Le CLI ne réécrit jamais d'AST : il insère dans des ancres. Ancre absente → il n'écrit rien dans ce fichier et affiche le bloc à coller.
- Séquence imposée à toute commande qui modifie un projet existant : lire → planifier → vérifier → afficher → appliquer.

---

### Task 1: Traduire les identifiants résiduels du parseur de champs

Le glossaire du 2026-08-28 couvre l'interne de `rbs-cli` mais a laissé ces deux fichiers en français. Les tâches suivantes ajoutent six variantes d'erreur : les poser à côté de dix variantes françaises produirait le dépôt bâtard que le glossaire existe pour empêcher.

**Files:**
- Modify: `crates/rbs-cli/src/generate/fields.rs`
- Modify: `crates/rbs-cli/src/generate/fields/error.rs`
- Modify: `crates/rbs-cli/src/generate/command.rs:64`

**Interfaces:**
- Consumes: rien.
- Produces: `Field { name: String, type_: FieldType, unique: bool, optional: bool, index: bool }` ; `FieldsError { errors: Vec<FieldError> }` ; `FieldError { rank: usize, label: String, kind: ErrorKind }` ; `ErrorKind::{InvalidForm, NotSnakeCase{suggestion}, RustKeyword{suggestions}, ReservedName, MigrationNameCollision, DuplicateName{previous_rank}, UnknownType{name}, UnknownModifier{name}, DuplicateModifier{name}, RedundantIndex}` ; `FieldType::NAMES` ; `RUST_KEYWORDS` ; `NAMES_SET_BY_RBS` ; `TABLE_NAME_IN_MIGRATION`.

- [ ] **Step 1: Vérifier que la suite passe avant de toucher quoi que ce soit**

Run: `cargo test -p rbs-cli 2>&1 | tail -20`
Expected: aucun `FAILED`. Noter le nombre de tests passés — il doit être identique à la fin.

- [ ] **Step 2: Renommer les variantes de `ErrorKind` et ses deux champs porteurs**

Dans `crates/rbs-cli/src/generate/fields/error.rs`, appliquer exactement cette correspondance, dans les déclarations comme dans les `match`, les tests compris :

| Français | Anglais |
|---|---|
| `FieldsError.erreurs` | `FieldsError.errors` |
| `FieldError.rang` | `FieldError.rank` |
| `FieldError.libelle` | `FieldError.label` |
| `ErrorKind::FormeInvalide` | `ErrorKind::InvalidForm` |
| `ErrorKind::PasEnSnakeCase` | `ErrorKind::NotSnakeCase` |
| `ErrorKind::MotCleRust` | `ErrorKind::RustKeyword` |
| `ErrorKind::NomReserve` | `ErrorKind::ReservedName` |
| `ErrorKind::NomCollisionMigration` | `ErrorKind::MigrationNameCollision` |
| `ErrorKind::NomEnDouble { rang_precedent }` | `ErrorKind::DuplicateName { previous_rank }` |
| `ErrorKind::TypeInconnu` | `ErrorKind::UnknownType` |
| `ErrorKind::ModificateurInconnu` | `ErrorKind::UnknownModifier` |
| `ErrorKind::ModificateurEnDouble` | `ErrorKind::DuplicateModifier` |
| `ErrorKind::IndexRedondant` | `ErrorKind::RedundantIndex` |
| `fn message(&self, libelle)` | `fn message(&self, label)` |
| `fn index(&self, libelle)` | `fn hint(&self, label)` |
| local `premier` (Display) | `first` |
| local `liste` | `list` |
| local `caracteres` / `caractere` | `characters` / `character` |
| local `rang` (boucles) | `rank` |
| `keyword_suggestions(mot)` | `keyword_suggestions(word)` |
| local `alias` | `alias` (inchangé) |

`fn index` devient `fn hint` : « index » y désignait la ligne d'indication affichée sous l'erreur, homonyme malheureux de l'index de colonne que ce lot va manipuler partout.

**Les chaînes de message ne changent pas.** `"forme attendue : « nom:type[:modificateur…] »"` reste mot pour mot ce qu'il est : c'est du texte destiné à un utilisateur francophone, hors périmètre du glossaire. Les assertions des tests qui les citent ne bougent donc pas non plus.

- [ ] **Step 3: Renommer dans `fields.rs`**

| Français | Anglais |
|---|---|
| `Field.optionnel` | `Field.optional` |
| `FieldType::NOMS` | `FieldType::NAMES` |
| `MOTS_CLES_RUST` | `RUST_KEYWORDS` |
| `NOMS_POSES_PAR_RBS` | `NAMES_SET_BY_RBS` |
| `NOM_DE_LA_TABLE_EN_MIGRATION` | `TABLE_NAME_IN_MIGRATION` |
| local `champ` / `champs` | `field` / `fields` |
| local `erreurs` | `errors` |
| local `rangs_par_nom` | `ranks_by_name` |
| local `rang` / `rang_precedent` | `rank` / `previous_rank` |
| local `portion` | `chunk` |
| local `parties` | `parts` |
| local `type_brut` | `raw_type` |
| local `modificateur` | `modifier` |
| local `drapeau` | `flag` |
| local `inconnu` | `unknown` |
| local `recasse` | `recased` |
| local `mot` | `word` |
| local `caracteres` / `premier` (to_pascal_case) | `characters` / `first` |

Attention à la sérialisation : `state.serialize_field("optional", &self.optionnel)` devient `state.serialize_field("optional", &self.optional)`. **La clé exposée aux templates ne change pas** — elle était déjà `optional`, et les `.jinja` ne bougent donc pas.

- [ ] **Step 4: Répercuter chez l'appelant**

Dans `crates/rbs-cli/src/generate/command.rs:64`, la variante `Champs(fields::FieldsError)` devient `Fields(fields::FieldsError)`. Corriger le site de construction dans le même fichier.

- [ ] **Step 5: Vérifier qu'aucun identifiant français ne subsiste**

Run:
```bash
grep -nE "\b(champ|champs|erreurs|rang|rangs_par_nom|libelle|portion|parties|drapeau|inconnu|optionnel|recasse|modificateur|type_brut|NOMS|MOTS_CLES_RUST|NOMS_POSES_PAR_RBS|NOM_DE_LA_TABLE_EN_MIGRATION|caracteres|premier)\b" \
  crates/rbs-cli/src/generate/fields.rs crates/rbs-cli/src/generate/fields/error.rs
```
Expected: aucune ligne de **code**. Les occurrences restantes acceptables sont uniquement dans des commentaires et des chaînes de message en français — les vérifier une à une.

- [ ] **Step 6: Vérifier que rien n'a changé de comportement**

Run: `cargo test -p rbs-cli 2>&1 | tail -20`
Expected: le même nombre de tests passés qu'au Step 1, aucun `FAILED`. Un renommage qui casse quelque chose le dit ici.

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: silence des deux côtés.

- [ ] **Step 7: Commit**

```bash
git add crates/rbs-cli/src/generate/fields.rs crates/rbs-cli/src/generate/fields/error.rs crates/rbs-cli/src/generate/command.rs
git commit -m "refactor(generate): passe à l'anglais les identifiants restants du parseur de champs"
```

---

### Task 2: Inventorier les entités d'un projet

**Files:**
- Create: `crates/rbs-cli/src/generate/entities.rs`
- Modify: `crates/rbs-cli/src/generate/mod.rs`

**Interfaces:**
- Consumes: rien.
- Produces:
```rust
pub(crate) struct Entity {
    /// Nom de la table, tel que `table_name` le déclare : `users`.
    pub table: String,
    /// Chemin du module portant l'entité : `crate::auth::model::user`.
    pub module_path: String,
    /// Fichier porteur, relatif à la racine du projet : `src/auth/model.rs`.
    pub file: String,
}
pub(crate) fn scan(root: &Path) -> Vec<Entity>;
pub(crate) fn find<'a>(entities: &'a [Entity], table: &str) -> Option<&'a Entity>;
pub(crate) fn tables(entities: &[Entity]) -> Vec<String>;
```

- [ ] **Step 1: Écrire les tests d'inventaire**

Créer `crates/rbs-cli/src/generate/entities.rs` avec **uniquement** ce bloc de tests, plus les signatures des trois fonctions renvoyant `unimplemented!()` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn project(features: &[(&str, &str)]) -> TempDir {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        for (module, source) in features {
            let directory = root.path().join("src").join(module);
            fs::create_dir_all(&directory).expect("le répertoire se crée");
            fs::write(directory.join("model.rs"), source).expect("l'écriture aboutit");
        }
        root
    }

    const PLAIN: &str = r#"
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model { pub id: Uuid }
"#;

    // `auth` déclare deux entités dans des modules imbriqués. La table `users` est la
    // cible la plus probable de toute relation : un scan qui ne lirait que les
    // répertoires la déclarerait introuvable.
    const NESTED: &str = r#"
pub mod user {
    #[sea_orm(table_name = "users")]
    pub struct Model { pub id: Uuid }
}

pub mod refresh_token {
    #[sea_orm(table_name = "refresh_tokens")]
    pub struct Model { pub id: Uuid }
}
"#;

    #[test]
    fn a_flat_feature_yields_one_entity_at_its_module_root() {
        let root = project(&[("posts", PLAIN)]);
        let found = scan(root.path());

        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].table, "posts");
        assert_eq!(found[0].module_path, "crate::posts::model");
        assert_eq!(found[0].file, "src/posts/model.rs");
    }

    #[test]
    fn nested_modules_are_followed_so_auth_tables_are_visible() {
        let root = project(&[("auth", NESTED)]);
        let found = scan(root.path());
        let users = find(&found, "users").expect("la table users doit être trouvée");

        assert_eq!(users.module_path, "crate::auth::model::user");
        assert_eq!(users.file, "src/auth/model.rs");
        assert!(find(&found, "refresh_tokens").is_some(), "{found:?}");
    }

    #[test]
    fn the_tables_are_listed_sorted_for_a_stable_error_message() {
        let root = project(&[("posts", PLAIN), ("auth", NESTED)]);

        assert_eq!(
            tables(&scan(root.path())),
            ["posts", "refresh_tokens", "users"]
        );
    }

    #[test]
    fn a_project_without_a_src_directory_yields_nothing() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");

        assert!(scan(root.path()).is_empty());
    }

    #[test]
    fn a_module_without_a_table_name_is_ignored() {
        let root = project(&[("health", "pub fn ok() {}\n")]);

        assert!(scan(root.path()).is_empty());
    }
}
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent**

Run: `cargo test -p rbs-cli entities 2>&1 | tail -20`
Expected: FAILED, avec `not implemented` sur les cinq tests.

- [ ] **Step 3: Implémenter le scan**

```rust
//! Inventaire des entités SeaORM d'un projet, lu sur le disque.
//!
//! Le scan est textuel, non un parseur Rust : un modèle lourdement réécrit le fera
//! échouer en refusant une cible, jamais en écrivant une relation fausse.

use std::fs;
use std::path::Path;

/// Une entité trouvée dans le projet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entity {
    /// Nom de la table, tel que `table_name` le déclare : `users`.
    pub table: String,
    /// Chemin du module portant l'entité : `crate::auth::model::user`.
    pub module_path: String,
    /// Fichier porteur, relatif à la racine du projet : `src/auth/model.rs`.
    pub file: String,
}

/// Parcourt `src/*/model.rs` et relève toute entité déclarée.
pub(crate) fn scan(root: &Path) -> Vec<Entity> {
    let mut found = Vec::new();

    let Ok(entries) = fs::read_dir(root.join("src")) else {
        return found;
    };

    let mut modules: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    // L'ordre de `read_dir` dépend du système de fichiers : sans tri, le message
    // nommant les entités connues changerait d'une machine à l'autre.
    modules.sort();

    for module in modules {
        let file = format!("src/{module}/model.rs");
        let Ok(source) = fs::read_to_string(root.join(&file)) else {
            continue;
        };
        collect(&source, &format!("crate::{module}::model"), &file, &mut found);
    }

    found
}

/// Relève les entités d'un seul fichier, en suivant ses modules imbriqués.
///
/// Le suivi est indispensable : la table `users` d'un projet authentifié vit sous
/// `src/auth/model.rs`, dans `pub mod user`, et non dans un `src/users/`.
fn collect(source: &str, module_path: &str, file: &str, found: &mut Vec<Entity>) {
    let mut current = module_path.to_string();
    let mut depth: usize = 0;

    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("pub mod ") {
            if let Some(name) = rest.split(['{', ';', ' ']).next().filter(|n| !n.is_empty()) {
                current = format!("{module_path}::{name}");
                depth = 1;
                continue;
            }
        }

        if let Some(table) = table_name(trimmed) {
            found.push(Entity {
                table,
                module_path: current.clone(),
                file: file.to_string(),
            });
            continue;
        }

        // Un module imbriqué se referme sur une accolade en début de ligne : la suite
        // du fichier appartient de nouveau au module racine.
        if depth == 1 && trimmed == "}" {
            current = module_path.to_string();
            depth = 0;
        }
    }
}

/// Extrait `users` de `#[sea_orm(table_name = "users")]`.
fn table_name(line: &str) -> Option<String> {
    let rest = line.split_once("table_name")?.1;
    let rest = rest.split_once('"')?.1;
    let (name, _) = rest.split_once('"')?;

    (!name.is_empty()).then(|| name.to_string())
}

/// Retrouve l'entité portant cette table.
pub(crate) fn find<'a>(entities: &'a [Entity], table: &str) -> Option<&'a Entity> {
    entities.iter().find(|entity| entity.table == table)
}

/// Les tables connues, triées : c'est ce que le refus d'une cible inconnue énumère.
pub(crate) fn tables(entities: &[Entity]) -> Vec<String> {
    let mut names: Vec<String> = entities.iter().map(|e| e.table.clone()).collect();
    names.sort();
    names.dedup();

    names
}
```

Déclarer le module dans `crates/rbs-cli/src/generate/mod.rs`, en gardant l'ordre alphabétique des `mod` existants :

```rust
pub(crate) mod entities;
```

- [ ] **Step 4: Lancer les tests, vérifier qu'ils passent**

Run: `cargo test -p rbs-cli entities 2>&1 | tail -20`
Expected: `5 passed`.

- [ ] **Step 5: Éprouver le scan sur un vrai projet du dépôt**

Run:
```bash
cargo test -p rbs-cli entities -- --nocapture 2>&1 | tail -5
ls examples/blog-auth/src/*/model.rs
```
Expected: `examples/blog-auth/src/auth/model.rs` et `examples/blog-auth/src/posts/model.rs` existent — ce sont les deux formes que les tests reproduisent.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/generate/entities.rs crates/rbs-cli/src/generate/mod.rs
git commit -m "feat(generate): inventorie les entités d'un projet, modules imbriqués compris"
```

---

### Task 3: Le type `references` dans la grammaire

`fields::parse` reste **pure et syntaxique** : elle ne connaît pas le projet, et ne peut donc pas juger qu'une cible existe. Elle porte les cinq refus qui se voient sur la seule chaîne ; le sixième est la tâche suivante.

**Files:**
- Modify: `crates/rbs-cli/src/generate/fields.rs`
- Modify: `crates/rbs-cli/src/generate/fields/error.rs`

**Interfaces:**
- Consumes: `Field`, `ErrorKind` de la Task 1.
- Produces:
```rust
pub(crate) enum OnDelete { Restrict, Cascade, SetNull }
pub(crate) struct Reference { pub target: String, pub on_delete: OnDelete }
pub(crate) enum FieldKind { Scalar(FieldType), Reference(Reference) }
// Field devient : { name, kind: FieldKind, unique, optional, index }
impl Field {
    pub(crate) fn column_name(&self) -> String;   // `author` -> `author_id`
    pub(crate) fn relation_name(&self) -> &str;   // `author`
    pub(crate) fn reference(&self) -> Option<&Reference>;
}
// ErrorKind gagne : MissingTarget, DerivedColumnName { suggestion: String },
//                   NullifyWithoutOptional, ConflictingOnDelete, RedundantIndexOnReference
```

- [ ] **Step 1: Écrire les tests de la grammaire**

Ajouter au `mod tests` de `crates/rbs-cli/src/generate/fields.rs` :

```rust
    fn only_error(input: &str) -> ErrorKind {
        let mut error = parse(input).expect_err("la chaîne doit être refusée");
        assert_eq!(error.errors.len(), 1, "{error:?}");
        error.errors.remove(0).kind
    }

    #[test]
    fn a_reference_derives_its_column_and_defaults_to_restrict() {
        let fields = parse("author:references:users").expect("la chaîne doit être acceptée");

        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].relation_name(), "author");
        assert_eq!(fields[0].column_name(), "author_id");
        assert_eq!(fields[0].rust_type(), "Uuid");
        let reference = fields[0].reference().expect("le champ porte une référence");
        assert_eq!(reference.target, "users");
        assert_eq!(reference.on_delete, OnDelete::Restrict);
    }

    // Sans index, chaque suppression dans la table cible parcourt la table portante en
    // entier pour vérifier la contrainte.
    #[test]
    fn a_reference_is_indexed_without_having_asked() {
        let fields = parse("author:references:users").expect("la chaîne doit être acceptée");

        assert!(fields[0].index, "{:?}", fields[0]);
    }

    #[test]
    fn a_unique_reference_is_a_one_to_one_and_drops_the_plain_index() {
        let fields = parse("profile:references:profiles:unique").expect("acceptée");

        assert!(fields[0].unique);
        assert!(!fields[0].index, "unique pose déjà un index : {:?}", fields[0]);
    }

    #[test]
    fn an_optional_reference_is_nullable() {
        let fields = parse("author:references:users:optional").expect("acceptée");

        assert!(fields[0].optional);
        assert_eq!(fields[0].rust_type(), "Option<Uuid>");
    }

    #[test]
    fn cascade_and_nullify_pick_the_on_delete_policy() {
        let cascade = parse("author:references:users:cascade").expect("acceptée");
        assert_eq!(cascade[0].reference().unwrap().on_delete, OnDelete::Cascade);

        let nullify = parse("author:references:users:optional:nullify").expect("acceptée");
        assert_eq!(nullify[0].reference().unwrap().on_delete, OnDelete::SetNull);
    }

    #[test]
    fn a_reference_without_a_target_is_rejected() {
        assert_eq!(only_error("author:references"), ErrorKind::MissingTarget);
    }

    #[test]
    fn a_name_ending_in_id_is_rejected_because_the_column_is_derived() {
        assert_eq!(
            only_error("author_id:references:users"),
            ErrorKind::DerivedColumnName {
                suggestion: "author".to_string()
            }
        );
    }

    // `SET NULL` sur une colonne `NOT NULL` échoue à l'exécution, pas à la migration :
    // le refus doit tomber ici.
    #[test]
    fn nullify_without_optional_is_rejected() {
        assert_eq!(
            only_error("author:references:users:nullify"),
            ErrorKind::NullifyWithoutOptional
        );
    }

    #[test]
    fn cascade_and_nullify_together_are_rejected() {
        assert_eq!(
            only_error("author:references:users:optional:cascade:nullify"),
            ErrorKind::ConflictingOnDelete
        );
    }

    #[test]
    fn an_explicit_index_on_a_reference_is_rejected_as_redundant() {
        assert_eq!(
            only_error("author:references:users:index"),
            ErrorKind::RedundantIndexOnReference
        );
    }

    #[test]
    fn cascade_and_nullify_are_refused_on_a_scalar_field() {
        assert_eq!(
            only_error("title:string:cascade"),
            ErrorKind::UnknownModifier {
                name: "cascade".to_string()
            }
        );
    }

    #[test]
    fn two_references_to_the_same_table_keep_their_own_names() {
        let fields = parse("author:references:users,reviewer:references:users").expect("acceptée");

        assert_eq!(fields[0].column_name(), "author_id");
        assert_eq!(fields[1].column_name(), "reviewer_id");
    }
```

Ajouter au `mod tests` de `crates/rbs-cli/src/generate/fields/error.rs` :

```rust
    #[test]
    fn a_missing_target_shows_the_expected_form() {
        let text = rendered(ErrorKind::MissingTarget, "author");
        assert!(text.contains("« references » attend une entité cible"), "{text}");
        assert!(text.contains("→ exemple : « author:references:users »"), "{text}");
    }

    #[test]
    fn a_derived_column_name_suggests_the_bare_form() {
        let text = rendered(
            ErrorKind::DerivedColumnName {
                suggestion: "author".to_string(),
            },
            "author_id",
        );
        assert!(text.contains("la colonne « author_id » est dérivée"), "{text}");
        assert!(text.contains("→ essayez « author »"), "{text}");
    }

    #[test]
    fn nullify_without_optional_explains_the_contradiction() {
        let text = rendered(ErrorKind::NullifyWithoutOptional, "author");
        assert!(text.contains("« nullify » sur une colonne non nullable"), "{text}");
        assert!(text.contains("→ ajoutez « optional »"), "{text}");
    }

    #[test]
    fn two_on_delete_policies_are_named_together() {
        let text = rendered(ErrorKind::ConflictingOnDelete, "author");
        assert!(text.contains("« cascade » et « nullify » se contredisent"), "{text}");
    }

    #[test]
    fn a_redundant_index_on_a_reference_explains_why() {
        let text = rendered(ErrorKind::RedundantIndexOnReference, "author");
        assert!(text.contains("une clé étrangère est déjà indexée"), "{text}");
        assert!(text.contains("→ retirez « index »"), "{text}");
    }
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent**

Run: `cargo test -p rbs-cli fields 2>&1 | tail -30`
Expected: échec de compilation — `OnDelete` introuvable, `ErrorKind::MissingTarget` introuvable, `relation_name` introuvable.

- [ ] **Step 3: Introduire `FieldKind` et déplacer le type scalaire dedans**

Dans `crates/rbs-cli/src/generate/fields.rs` :

```rust
/// Ce que la base fait des lignes portantes quand la ligne cible disparaît.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OnDelete {
    Restrict,
    Cascade,
    SetNull,
}

impl OnDelete {
    /// Nom de la variante `ForeignKeyAction` de sea-orm-migration.
    pub(crate) fn action(self) -> &'static str {
        match self {
            Self::Restrict => "Restrict",
            Self::Cascade => "Cascade",
            Self::SetNull => "SetNull",
        }
    }
}

/// Une référence vers une autre entité, telle que `--fields` la déclare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reference {
    /// Nom de la table visée, tel qu'il a été écrit : `users`.
    pub target: String,
    pub on_delete: OnDelete,
}

/// Un champ décrit soit une colonne scalaire, soit une référence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FieldKind {
    Scalar(FieldType),
    Reference(Reference),
}
```

`Field` devient :

```rust
pub(crate) struct Field {
    /// Le nom déclaré : `title` pour un scalaire, `author` pour une référence — dont la
    /// colonne, elle, est dérivée.
    pub name: String,
    pub kind: FieldKind,
    pub unique: bool,
    pub optional: bool,
    pub index: bool,
}

impl Field {
    /// Nom de la colonne : le nom déclaré, suffixé de `_id` pour une référence.
    pub(crate) fn column_name(&self) -> String {
        match self.kind {
            FieldKind::Reference(_) => format!("{}_id", self.name),
            FieldKind::Scalar(_) => self.name.clone(),
        }
    }

    /// Nom de la relation : le nom déclaré, tel quel.
    pub(crate) fn relation_name(&self) -> &str {
        &self.name
    }

    pub(crate) fn reference(&self) -> Option<&Reference> {
        match &self.kind {
            FieldKind::Reference(reference) => Some(reference),
            FieldKind::Scalar(_) => None,
        }
    }

    pub(crate) fn rust_type(&self) -> String {
        let bare = match &self.kind {
            FieldKind::Scalar(type_) => type_.rust_type(),
            FieldKind::Reference(_) => "Uuid",
        };

        if self.optional {
            format!("Option<{bare}>")
        } else {
            bare.to_string()
        }
    }
}
```

Adapter `validates_email` — une référence n'est jamais un email :

```rust
    pub(crate) fn validates_email(&self) -> bool {
        let FieldKind::Scalar(type_) = &self.kind else {
            return false;
        };
        let textual = matches!(type_, FieldType::String | FieldType::Text);

        textual && (self.name == "email" || self.name.ends_with("_email"))
    }
```

**La sérialisation est le point délicat.** `name` doit exposer la **colonne**, pour que les blocs de colonne des templates ne changent pas d'une ligne :

```rust
impl Serialize for Field {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Field", 13)?;
        // `name` porte la colonne, non le nom déclaré : les templates de colonne
        // — modèle, migration, DTO — n'ont ainsi rien à savoir des relations.
        state.serialize_field("name", &self.column_name())?;
        state.serialize_field("pascal_name", &to_pascal_case(&self.column_name()))?;
        state.serialize_field("type", self.type_name())?;
        state.serialize_field("unique", &self.unique)?;
        state.serialize_field("optional", &self.optional)?;
        state.serialize_field("index", &self.index)?;
        state.serialize_field("rust_type", &self.rust_type())?;
        state.serialize_field("bare_rust_type", &self.bare_rust_type())?;
        state.serialize_field("migration_method", self.migration_method())?;
        state.serialize_field("column_type_attribute", &self.column_type_attribute())?;
        state.serialize_field("valide_email", &self.validates_email())?;
        state.serialize_field("relation", &self.relation_view())?;
        state.end()
    }
}
```

où `relation_view` rend `None` pour un scalaire et, pour une référence, une structure sérialisable que la Task 5 consommera :

```rust
/// Ce qu'une template lit d'une référence, une fois la cible retrouvée dans le projet.
///
/// Elle est posée par `relations::resolve` et non calculée à la sérialisation : elle
/// dépend d'un inventaire du projet, que `Field` ne connaît pas.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RelationView {
    /// Nom de la relation : `author`.
    pub name: String,
    /// Nom de la variante `Relation` : `Author`.
    pub variant: String,
    /// Table visée : `users`.
    pub target: String,
    /// Chemin de l'entité visée : `crate::auth::model::user::Entity`.
    pub entity_path: String,
    /// Chemin de sa colonne d'identifiant : `crate::auth::model::user::Column::Id`.
    pub target_column_path: String,
    /// Identifiant `DeriveIden` de la table visée dans la migration : `Users`.
    pub target_iden: String,
    /// Variante `ForeignKeyAction` : `Restrict`.
    pub on_delete: String,
}
```

`Field` porte la vue et l'expose :

```rust
pub(crate) struct Field {
    // … name, kind, unique, optional, index
    /// Posée par `relations::resolve`, absente jusque-là et pour tout scalaire.
    pub relation: Option<RelationView>,
}

impl Field {
    pub(crate) fn relation(&self) -> Option<&RelationView> {
        self.relation.as_ref()
    }

    pub(crate) fn set_relation(&mut self, view: RelationView) {
        self.relation = Some(view);
    }
}
```

La sérialisation expose `state.serialize_field("relation", &self.relation)?`. Les tests de cette tâche ne lisent que `reference()`, la vue n'étant posée qu'à la Task 4.

Les méthodes `type_name`, `bare_rust_type`, `migration_method` et `column_type_attribute` déléguent au `FieldType` pour un scalaire et rendent respectivement `"references"`, `"Uuid"`, `"uuid()"` et `None` pour une référence.

- [ ] **Step 4: Analyser le type et ses modificateurs**

Dans `parse_field`, après la lecture du nom et avant la boucle des modificateurs :

```rust
    let kind = if raw_type == "references" {
        if name.ends_with("_id") {
            return Err(error(
                name,
                ErrorKind::DerivedColumnName {
                    suggestion: name.trim_end_matches("_id").to_string(),
                },
            ));
        }

        let Some(target) = parts.next().filter(|value| !value.is_empty()) else {
            return Err(error(name, ErrorKind::MissingTarget));
        };

        FieldKind::Reference(Reference {
            target: target.to_string(),
            on_delete: OnDelete::Restrict,
        })
    } else {
        let Some(type_) = FieldType::parse(raw_type) else {
            return Err(error(
                name,
                ErrorKind::UnknownType {
                    name: raw_type.to_string(),
                },
            ));
        };
        FieldKind::Scalar(type_)
    };
```

`parts` étant l'itérateur des segments, la cible est consommée avant la boucle des modificateurs : celle-ci ne change pas de forme. `cascade` et `nullify` n'y sont admis que sur une référence — sur un scalaire ils tombent dans le bras `unknown`, ce qui rend le message « modificateur inconnu » déjà écrit.

L'index implicite et les trois refus croisés se posent après la boucle :

```rust
    if let FieldKind::Reference(reference) = &mut field.kind {
        if cascade && nullify {
            return Err(error(name, ErrorKind::ConflictingOnDelete));
        }
        if nullify && !field.optional {
            return Err(error(name, ErrorKind::NullifyWithoutOptional));
        }
        if field.index {
            return Err(error(name, ErrorKind::RedundantIndexOnReference));
        }

        reference.on_delete = match (cascade, nullify) {
            (true, _) => OnDelete::Cascade,
            (_, true) => OnDelete::SetNull,
            _ => OnDelete::Restrict,
        };
        // L'index n'est pas demandé : il est la condition pour que la vérification de
        // la contrainte ne parcoure pas la table entière.
        field.index = !field.unique;
    } else if field.unique && field.index {
        return Err(error(name, ErrorKind::RedundantIndex));
    }
```

- [ ] **Step 5: Écrire les cinq messages d'erreur**

Dans `fields/error.rs`, ajouter aux deux `match` :

```rust
            Self::MissingTarget => "« references » attend une entité cible".to_string(),
            Self::DerivedColumnName { .. } => {
                format!("la colonne « {label} » est dérivée du nom de la relation")
            }
            Self::NullifyWithoutOptional => {
                "« nullify » sur une colonne non nullable".to_string()
            }
            Self::ConflictingOnDelete => {
                "« cascade » et « nullify » se contredisent".to_string()
            }
            Self::RedundantIndexOnReference => {
                "« index » redondant : une clé étrangère est déjà indexée".to_string()
            }
```

et pour `hint` :

```rust
            Self::MissingTarget => Some("exemple : « author:references:users »".to_string()),
            Self::DerivedColumnName { suggestion } => Some(format!("essayez « {suggestion} »")),
            Self::NullifyWithoutOptional => {
                Some("ajoutez « optional », ou choisissez « cascade »".to_string())
            }
            Self::ConflictingOnDelete => Some("gardez l'un des deux".to_string()),
            Self::RedundantIndexOnReference => Some("retirez « index »".to_string()),
```

Étendre aussi la ligne d'indication de `UnknownModifier`, qui énumère les modificateurs admis : `"unique, optional, index — sur une référence : cascade, nullify"`. Corriger l'assertion du test `an_unknown_modifier_lists_the_three_allowed_ones` en conséquence, et le renommer `an_unknown_modifier_lists_the_allowed_ones`.

- [ ] **Step 6: Lancer les tests, vérifier qu'ils passent**

Run: `cargo test -p rbs-cli fields 2>&1 | tail -20`
Expected: tous passent. Les tests existants du parseur passent sans avoir été touchés, hormis celui de l'indication des modificateurs.

Run: `cargo test -p rbs-cli 2>&1 | tail -20`
Expected: `entity`, `migration`, `dto`, `seed` passent toujours — c'est la preuve que la sérialisation de `name` sur la colonne a bien tenu les templates à l'écart.

- [ ] **Step 7: Commit**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/generate/fields.rs crates/rbs-cli/src/generate/fields/error.rs
git commit -m "feat(generate): ajoute le type references à la grammaire des champs"
```

---

### Task 4: Résoudre les cibles contre l'inventaire

**Files:**
- Create: `crates/rbs-cli/src/generate/relations.rs`
- Modify: `crates/rbs-cli/src/generate/mod.rs`

**Interfaces:**
- Consumes: `entities::{Entity, scan, find, tables}` (Task 2) ; `Field`, `FieldKind`, `RelationView` (Task 3) ; `feature::to_singular`, `fields::to_pascal_case`.
- Produces:
```rust
#[derive(Debug, thiserror::Error)]
pub(crate) struct UnknownTarget { pub relation: String, pub target: String, pub known: Vec<String> }
pub(crate) fn resolve(
    fields: &mut [Field],
    entities: &[Entity],
    generated_table: &str,
) -> Result<(), Vec<UnknownTarget>>;
```

- [ ] **Step 1: Écrire les tests de résolution**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::entities::Entity;
    use crate::generate::fields;

    fn inventory() -> Vec<Entity> {
        vec![
            Entity {
                table: "users".to_string(),
                module_path: "crate::auth::model::user".to_string(),
                file: "src/auth/model.rs".to_string(),
            },
            Entity {
                table: "tags".to_string(),
                module_path: "crate::tags::model".to_string(),
                file: "src/tags/model.rs".to_string(),
            },
        ]
    }

    fn resolved(input: &str, generated: &str) -> Vec<fields::Field> {
        let mut parsed = fields::parse(input).expect("la chaîne doit être acceptée");
        resolve(&mut parsed, &inventory(), generated).expect("les cibles doivent se résoudre");
        parsed
    }

    #[test]
    fn a_target_in_a_nested_module_resolves_to_its_full_path() {
        let fields = resolved("author:references:users", "posts");
        let view = fields[0].relation().expect("la vue de relation est posée");

        assert_eq!(view.entity_path, "crate::auth::model::user::Entity");
        assert_eq!(view.target_column_path, "crate::auth::model::user::Column::Id");
        assert_eq!(view.variant, "Author");
        assert_eq!(view.target_iden, "Users");
        assert_eq!(view.on_delete, "Restrict");
    }

    // Un arbre : l'entité en cours de génération n'est pas encore sur le disque, et
    // doit pourtant être une cible valable.
    #[test]
    fn the_entity_being_generated_is_a_valid_target() {
        let fields = resolved("parent:references:posts:optional", "posts");
        let view = fields[0].relation().expect("la vue de relation est posée");

        assert_eq!(view.entity_path, "Entity");
        assert_eq!(view.target_column_path, "Column::Id");
        assert_eq!(view.target_iden, "Posts");
    }

    #[test]
    fn an_unknown_target_is_rejected_and_names_the_known_tables() {
        let mut parsed = fields::parse("author:references:writers").expect("acceptée");
        let errors = resolve(&mut parsed, &inventory(), "posts")
            .expect_err("une cible inconnue doit être refusée");

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].target, "writers");
        assert_eq!(errors[0].relation, "author");
        assert_eq!(errors[0].known, ["posts", "tags", "users"]);
    }

    #[test]
    fn every_unknown_target_is_collected_in_one_pass() {
        let mut parsed = fields::parse("a:references:x,b:references:y").expect("acceptée");
        let errors = resolve(&mut parsed, &inventory(), "posts").expect_err("refusée");

        assert_eq!(errors.len(), 2, "{errors:?}");
    }

    #[test]
    fn the_message_names_the_relation_the_target_and_the_known_tables() {
        let error = UnknownTarget {
            relation: "author".to_string(),
            target: "writers".to_string(),
            known: vec!["posts".to_string(), "users".to_string()],
        };
        let text = error.to_string();

        assert!(text.contains("« writers » est introuvable"), "{text}");
        assert!(text.contains("author"), "{text}");
        assert!(text.contains("posts, users"), "{text}");
    }

    #[test]
    fn a_scalar_field_is_left_untouched() {
        let fields = resolved("title:string", "posts");

        assert!(fields[0].relation().is_none());
    }
}
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent**

Run: `cargo test -p rbs-cli relations 2>&1 | tail -20`
Expected: échec de compilation, `resolve` et `UnknownTarget` introuvables.

- [ ] **Step 3: Implémenter la résolution**

```rust
//! Confrontation des cibles déclarées dans `--fields` à ce que le projet contient.
//!
//! Séparé du parseur, qui reste pur : une chaîne s'analyse sans projet, une cible ne se
//! juge que contre un inventaire.

use std::fmt;

use super::entities::{self, Entity};
use super::feature::to_singular;
use super::fields::{Field, RelationView, to_pascal_case};

/// Une cible qu'aucune entité du projet ne porte.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UnknownTarget {
    /// Nom de la relation fautive : `author`.
    pub relation: String,
    /// Cible écrite : `writers`.
    pub target: String,
    /// Tables connues, triées.
    pub known: Vec<String>,
}

impl fmt::Display for UnknownTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "erreur : relation « {} » — « {} » est introuvable dans ce projet\n        \
             → entités connues : {}",
            self.relation,
            self.target,
            self.known.join(", ")
        )
    }
}

impl std::error::Error for UnknownTarget {}

/// Résout chaque référence contre l'inventaire, et pose sa vue pour les templates.
///
/// `generated_table` rejoint les cibles admises : elle n'est pas encore sur le disque,
/// et une entité qui se référence elle-même — un arbre — est légitime.
pub(crate) fn resolve(
    fields: &mut [Field],
    entities: &[Entity],
    generated_table: &str,
) -> Result<(), Vec<UnknownTarget>> {
    let mut known = entities::tables(entities);
    if !known.iter().any(|table| table == generated_table) {
        known.push(generated_table.to_string());
        known.sort();
    }

    let mut errors = Vec::new();

    for field in fields.iter_mut() {
        let Some(reference) = field.reference().cloned() else {
            continue;
        };

        let entity_path = if reference.target == generated_table {
            // L'entité se référence elle-même : son module n'existe pas encore, et
            // `Entity` la désigne depuis son propre fichier.
            "Entity".to_string()
        } else {
            match entities::find(entities, &reference.target) {
                Some(entity) => format!("{}::Entity", entity.module_path),
                None => {
                    errors.push(UnknownTarget {
                        relation: field.relation_name().to_string(),
                        target: reference.target.clone(),
                        known: known.clone(),
                    });
                    continue;
                }
            }
        };

        // `Entity` désigne l'entité locale : sa colonne est `Column::Id`, sans chemin.
        let target_column_path = if entity_path == "Entity" {
            "Column::Id".to_string()
        } else {
            entity_path.replace("::Entity", "::Column::Id")
        };

        field.set_relation(RelationView {
            name: field.relation_name().to_string(),
            variant: to_pascal_case(&to_singular(field.relation_name())),
            target: reference.target.clone(),
            entity_path,
            target_column_path,
            target_iden: to_pascal_case(&reference.target),
            on_delete: reference.on_delete.action().to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

Le champ `relation` et ses deux accès sont déjà en place depuis la Task 3 : cette tâche ne fait que les remplir.

Déclarer `pub(crate) mod relations;` dans `generate/mod.rs`.

- [ ] **Step 4: Lancer les tests, vérifier qu'ils passent**

Run: `cargo test -p rbs-cli relations 2>&1 | tail -20`
Expected: `6 passed`.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/generate/relations.rs crates/rbs-cli/src/generate/fields.rs crates/rbs-cli/src/generate/mod.rs
git commit -m "feat(generate): refuse une cible de relation absente du projet"
```

---

### Task 5: Le modèle porte ses relations et ses deux ancres

**Files:**
- Modify: `crates/rbs-cli/templates/feature/model.rs.jinja`
- Modify: `crates/rbs-cli/src/generate/entity.rs` (tests)

**Interfaces:**
- Consumes: `field.relation.{variant, entity_path, on_delete, name}` de la Task 4 ; `field.pascal_name` qui porte déjà `AuthorId`.
- Produces: un `src/<feature>/model.rs` portant `// <rbs:relations>` et `// <rbs:related>`, consommées par la Task 8.

- [ ] **Step 1: Écrire les tests de rendu**

Dans `crates/rbs-cli/src/generate/entity.rs`, remplacer le helper `entity` par une variante qui résout les relations, et ajouter les tests :

```rust
    fn entity_with(name: &str, fields: &str, entities: &[crate::generate::entities::Entity]) -> String {
        let mut parsed = fields::parse(fields).expect("les champs du test doivent être valides");
        crate::generate::relations::resolve(&mut parsed, entities, name)
            .expect("les cibles du test doivent se résoudre");
        render(&Feature::fresh(name, parsed)).expect("l'entité doit se rendre")
    }

    fn users_entity() -> Vec<crate::generate::entities::Entity> {
        vec![crate::generate::entities::Entity {
            table: "users".to_string(),
            module_path: "crate::auth::model::user".to_string(),
            file: "src/auth/model.rs".to_string(),
        }]
    }

    #[test]
    fn a_reference_becomes_a_uuid_column_named_after_the_relation() {
        let rendered = entity_with("posts", "author:references:users", &users_entity());

        assert!(rendered.contains("pub author_id: Uuid,"), "{rendered}");
    }

    #[test]
    fn a_reference_declares_its_variant_and_its_on_delete() {
        let rendered = entity_with("posts", "author:references:users", &users_entity());

        assert!(
            rendered.contains(r#"belongs_to = "crate::auth::model::user::Entity""#),
            "{rendered}"
        );
        assert!(rendered.contains(r#"from = "Column::AuthorId""#), "{rendered}");
        assert!(
            rendered.contains(r#"to = "crate::auth::model::user::Column::Id""#),
            "{rendered}"
        );
        assert!(rendered.contains(r#"on_delete = "Restrict""#), "{rendered}");
        assert!(rendered.contains("    Author,"), "{rendered}");
    }

    #[test]
    fn a_reference_implements_related_towards_its_target() {
        let rendered = entity_with("posts", "author:references:users", &users_entity());

        assert!(
            rendered.contains("impl Related<crate::auth::model::user::Entity> for Entity {"),
            "{rendered}"
        );
        assert!(
            rendered.contains("fn to() -> RelationDef {\n        Relation::Author.def()"),
            "{rendered}"
        );
    }

    #[test]
    fn a_cascade_reference_carries_its_action() {
        let rendered = entity_with("posts", "author:references:users:cascade", &users_entity());

        assert!(rendered.contains(r#"on_delete = "Cascade""#), "{rendered}");
    }

    // Les variantes vivent dans les accolades de l'énumération, les `impl Related` ne le
    // peuvent pas : il faut donc deux ancres, et non une.
    #[test]
    fn the_model_carries_both_anchors_even_without_a_relation() {
        let rendered = entity("posts", "title:string");

        assert!(rendered.contains("    // <rbs:relations>\n    // </rbs:relations>"), "{rendered}");
        assert!(rendered.contains("// <rbs:related>\n// </rbs:related>"), "{rendered}");
    }

    #[test]
    fn a_self_reference_points_at_the_local_entity() {
        let rendered = entity_with("posts", "parent:references:posts:optional", &[]);

        assert!(rendered.contains(r#"belongs_to = "Entity""#), "{rendered}");
        assert!(rendered.contains("pub parent_id: Option<Uuid>,"), "{rendered}");
    }
```

Corriger le test existant `a_field_less_feature_renders_a_complete_entity`, qui asserte `pub enum Relation {}` : l'énumération porte désormais ses ancres. Il devient :

```rust
        assert!(rendered.contains("pub enum Relation {"), "{rendered}");
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent**

Run: `cargo test -p rbs-cli entity 2>&1 | tail -30`
Expected: FAILED sur les six nouveaux tests, `belongs_to` et les ancres absents du rendu.

- [ ] **Step 3: Étendre la template**

Dans `crates/rbs-cli/templates/feature/model.rs.jinja`, remplacer la ligne `pub enum Relation {}` par :

```jinja
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
{%- for field in fields if field.relation %}
    #[sea_orm(
        belongs_to = "{@ field.relation.entity_path @}",
        from = "Column::{@ field.pascal_name @}",
        to = "{@ field.relation.target_column_path @}",
        on_delete = "{@ field.relation.on_delete @}"
    )]
    {@ field.relation.variant @},
{%- endfor %}
    // <rbs:relations>
    // </rbs:relations>
}
{% for field in fields if field.relation %}
impl Related<{@ field.relation.entity_path @}> for Entity {
    fn to() -> RelationDef {
        Relation::{@ field.relation.variant @}.def()
    }
}
{% endfor %}
// <rbs:related>
// </rbs:related>
```

Aucun calcul dans la template : `target_column_path` est posé par `relations::resolve` (Task 4), la logique du chemin appartenant à Rust.

- [ ] **Step 4: Lancer les tests, vérifier qu'ils passent**

Run: `cargo test -p rbs-cli entity 2>&1 | tail -20`
Expected: tous passent.

- [ ] **Step 5: Relire le rendu à l'œil**

Run: `cargo test -p rbs-cli entity::tests::preview -- --ignored --nocapture`
Expected: le modèle s'affiche. Vérifier de visu qu'il n'y a **pas** de ligne vide surnuméraire entre l'énumération et le premier `impl Related`, et que le fichier se termine par une seule fin de ligne — le test `the_render_ends_with_a_single_newline` le vérifie déjà, mais l'ancre `<rbs:related>` est désormais la dernière chose du fichier.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/templates/feature/model.rs.jinja crates/rbs-cli/src/generate/entity.rs crates/rbs-cli/src/generate/relations.rs
git commit -m "feat(generate): rend les relations SeaORM dans le modèle d'une feature"
```

---

### Task 6: La migration pose la clé étrangère

**Files:**
- Modify: `crates/rbs-cli/templates/feature/migration.rs.jinja`
- Modify: `crates/rbs-cli/src/generate/migration.rs` (tests)

**Interfaces:**
- Consumes: `field.relation.{target_iden, on_delete}`, `field.pascal_name`, `table`, `iden`.
- Produces: une migration qui crée la contrainte et son index.

- [ ] **Step 1: Écrire les tests de rendu**

Dans `crates/rbs-cli/src/generate/migration.rs`, ajouter les deux helpers puis les tests :

```rust
    fn users_entity() -> Vec<crate::generate::entities::Entity> {
        vec![crate::generate::entities::Entity {
            table: "users".to_string(),
            module_path: "crate::auth::model::user".to_string(),
            file: "src/auth/model.rs".to_string(),
        }]
    }

    fn migration_with(
        name: &str,
        fields: &str,
        entities: &[crate::generate::entities::Entity],
    ) -> Migration {
        let mut parsed = fields::parse(fields).expect("les champs du test doivent être valides");
        crate::generate::relations::resolve(&mut parsed, entities, name)
            .expect("les cibles du test doivent se résoudre");
        render(&Feature::fresh(name, parsed), HORODATAGE).expect("la migration doit se rendre")
    }
```


```rust
    #[test]
    fn a_reference_creates_its_foreign_key_named_after_table_and_column() {
        let rendered = migration_with("posts", "author:references:users", &users_entity()).content;

        assert!(rendered.contains(r#".name("fk_posts_author_id")"#), "{rendered}");
        assert!(
            rendered.contains(".from(Posts::Table, Posts::AuthorId)"),
            "{rendered}"
        );
        assert!(rendered.contains(".to(Users::Table, Users::Id)"), "{rendered}");
        assert!(
            rendered.contains(".on_delete(ForeignKeyAction::Restrict)"),
            "{rendered}"
        );
    }

    #[test]
    fn the_referencing_column_is_a_not_null_uuid() {
        let rendered = migration_with("posts", "author:references:users", &users_entity()).content;

        assert!(
            rendered.contains("ColumnDef::new(Posts::AuthorId).uuid().not_null()"),
            "{rendered}"
        );
    }

    #[test]
    fn an_optional_reference_is_nullable_and_can_be_set_null() {
        let rendered = migration_with(
            "posts",
            "author:references:users:optional:nullify",
            &users_entity(),
        )
        .content;

        assert!(
            rendered.contains("ColumnDef::new(Posts::AuthorId).uuid().null()"),
            "{rendered}"
        );
        assert!(
            rendered.contains(".on_delete(ForeignKeyAction::SetNull)"),
            "{rendered}"
        );
    }

    // Sans index, la vérification de la contrainte au `DELETE` de la cible parcourt la
    // table portante en entier.
    #[test]
    fn a_reference_gets_its_index() {
        let rendered = migration_with("posts", "author:references:users", &users_entity()).content;

        assert!(rendered.contains(r#".name("idx_posts_author_id")"#), "{rendered}");
    }

    #[test]
    fn the_target_iden_is_declared_once_even_for_two_relations_to_the_same_table() {
        let rendered = migration_with(
            "posts",
            "author:references:users,reviewer:references:users",
            &users_entity(),
        )
        .content;

        assert_eq!(
            rendered.matches("enum Users {").count(),
            1,
            "l'identifiant de la table cible est déclaré deux fois :\n{rendered}"
        );
        assert!(rendered.contains("enum Users {\n    Table,\n    Id,\n}"), "{rendered}");
    }

    #[test]
    fn a_self_reference_does_not_redeclare_its_own_iden() {
        let rendered =
            migration_with("posts", "parent:references:posts:optional", &[]).content;

        assert_eq!(rendered.matches("enum Posts {").count(), 1, "{rendered}");
        assert!(rendered.contains(".to(Posts::Table, Posts::Id)"), "{rendered}");
    }
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent**

Run: `cargo test -p rbs-cli migration 2>&1 | tail -30`
Expected: FAILED, `.foreign_key` absent du rendu.

- [ ] **Step 3: Exposer les identifiants cibles à la template**

Une migration doit déclarer un `DeriveIden` par table qu'elle nomme, **une seule fois** même si deux relations visent la même, et jamais pour sa propre table qu'elle déclare déjà. Ce calcul ne tient pas dans une template : le faire en Rust, dans `Feature`.

Ajouter à `crates/rbs-cli/src/generate/feature.rs` :

```rust
    /// Identifiants `DeriveIden` des tables visées par les relations, dédupliqués et
    /// privés de la sienne propre : une migration ne déclare pas deux fois le même enum.
    pub(crate) fn target_idens(&self) -> Vec<String> {
        let own = self.iden();
        let mut idens: Vec<String> = self
            .fields
            .iter()
            .filter_map(|field| field.relation())
            .map(|relation| relation.target_iden.clone())
            .filter(|iden| *iden != own)
            .collect();
        idens.sort();
        idens.dedup();

        idens
    }
```

et à sa sérialisation :

```rust
        state.serialize_field("target_idens", &self.target_idens())?;
```

en portant le compte de `serialize_struct("Feature", 6)` à `7`.

Ajouter le test dans le même fichier :

```rust
    #[test]
    fn the_target_idens_are_deduplicated_and_exclude_the_own_table() {
        let inventory = [crate::generate::entities::Entity {
            table: "users".to_string(),
            module_path: "crate::auth::model::user".to_string(),
            file: "src/auth/model.rs".to_string(),
        }];
        let mut fields =
            crate::generate::fields::parse("a:references:users,b:references:users,c:references:posts")
                .expect("la chaîne doit être acceptée");
        crate::generate::relations::resolve(&mut fields, &inventory, "posts")
            .expect("les cibles doivent se résoudre");
        let feature = Feature::fresh("posts", fields);

        // `Users` une seule fois pour deux relations, et `Posts` jamais : la migration
        // déclare déjà l'identifiant de sa propre table.
        assert_eq!(feature.target_idens(), ["Users"]);
    }
```

- [ ] **Step 4: Étendre la template de migration**

Dans `crates/rbs-cli/templates/feature/migration.rs.jinja`, après la boucle des colonnes et avant `CreatedAt`, insérer les contraintes :

```jinja
{%- for field in fields if field.relation %}
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_{@ table @}_{@ field.name @}")
                            .from({@ iden @}::Table, {@ iden @}::{@ field.pascal_name @})
                            .to({@ field.relation.target_iden @}::Table, {@ field.relation.target_iden @}::Id)
                            .on_delete(ForeignKeyAction::{@ field.relation.on_delete @}),
                    )
{%- endfor %}
```

`field.name` porte déjà `author_id` : le nom de la contrainte est donc `fk_posts_author_id` sans calcul supplémentaire. La boucle d'index existante couvre déjà les références, `field.index` étant vrai pour elles.

En fin de fichier, après l'énumération de la table, déclarer les identifiants cibles :

```jinja
{% for target in target_idens %}
#[derive(DeriveIden)]
enum {@ target @} {
    Table,
    Id,
}
{% endfor %}
```

- [ ] **Step 5: Lancer les tests, vérifier qu'ils passent**

Run: `cargo test -p rbs-cli migration 2>&1 | tail -20`
Expected: tous passent.

- [ ] **Step 6: Vérifier que la migration compile réellement**

`crates/rbs-cli/src/generate/migration.rs` ne porte aucun test de compilation : le seul qui compile une feature entière est `entity.rs:164`, `the_generated_entity_compiles_in_a_fresh_project`. Une migration avec clé étrangère ne se prouve pas là — elle a besoin que la table cible existe.

Ajouter dans `migration.rs`, sur la forme de celui d'`entity.rs` :

```rust
    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
    fn the_generated_migration_compiles_with_its_foreign_key() {
        let project = bench::Project::fresh();

        // La cible d'abord : une migration ne compile pas contre une table absente de
        // la crate `migration`.
        let users = migration("users", "email:string:unique");
        project.write_migration(&users.module, &users.content);

        let posts = migration_with("posts", "title:string,author:references:users", &users_entity());
        project.write_migration(&posts.module, &posts.content);

        project.compile();
    }
```

Lire `crates/rbs-cli/src/generate/bench.rs` d'abord : si `Project` n'expose pas `write_migration`, l'ajouter sur le modèle de `write_feature`, en écrivant dans `migration/src/` et en inscrivant le module dans `migration/src/lib.rs`.

Run: `cargo test -p rbs-cli migration -- --ignored --nocapture 2>&1 | tail -30`
Expected: le test aboutit. Plusieurs minutes.

- [ ] **Step 7: Commit**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/templates/feature/migration.rs.jinja crates/rbs-cli/src/generate/migration.rs crates/rbs-cli/src/generate/feature.rs
git commit -m "feat(generate): pose la clé étrangère et son index dans la migration"
```

---

### Task 6b: Un seed n'invente pas de clé étrangère

Découvert à l'exécution, absent de la spec comme du plan. `generate/seed.rs:85` projette tout `Uuid` sur `Uuid::from_u128(rang)` : une feature portant `author:references:users` engendre donc `author_id: Set(Uuid::from_u128(1))`, un identifiant qui ne référence rien. Inoffensif tant qu'aucune contrainte n'existe — mais la Task 6 vient de la poser, et `cargo run --bin seeds` échoue désormais sur une violation de clé étrangère.

La règle retenue : une référence **optionnelle** se sème à `None`, valide puisque la colonne accepte le nul ; une référence **requise** rend l'entité non semable, et aucun fichier de seed n'est engendré pour elle. Semer une relation demanderait de connaître une ligne cible existante, ce qu'un seed indépendant ne peut pas savoir — et un seed engendré qui échoue à chaque lancement est un livrable cassé.

**Files:**
- Modify: `crates/rbs-cli/src/generate/seed.rs`
- Modify: `crates/rbs-cli/src/generate/command.rs`

**Interfaces:**
- Consumes: `Field::reference()`, `Field.optional` (Task 3).
- Produces: `pub(crate) fn is_seedable(feature: &Feature) -> bool` dans `seed.rs`.

- [ ] **Step 1: Écrire les tests**

Dans le `mod tests` de `crates/rbs-cli/src/generate/seed.rs` :

```rust
    #[test]
    fn an_optional_reference_is_seeded_as_none() {
        let rendered = seed("posts", "title:string,author:references:users:optional");

        assert!(rendered.contains("author_id: Set(None),"), "{rendered}");
        assert!(
            !rendered.contains("Uuid::from_u128"),
            "un identifiant inventé pointerait vers une ligne inexistante :\n{rendered}"
        );
    }

    // Semer une référence requise demanderait de connaître une ligne cible existante,
    // qu'un seed indépendant ne peut pas savoir. Mieux vaut ne rien engendrer que
    // d'engendrer ce qui échouera à chaque lancement.
    #[test]
    fn a_required_reference_makes_the_entity_unseedable() {
        let with = Feature::fresh(
            "posts",
            fields::parse("title:string,author:references:users").expect("acceptée"),
        );
        let without = Feature::fresh(
            "posts",
            fields::parse("title:string").expect("acceptée"),
        );

        assert!(!is_seedable(&with));
        assert!(is_seedable(&without));
    }

    #[test]
    fn an_optional_reference_leaves_the_entity_seedable() {
        let feature = Feature::fresh(
            "posts",
            fields::parse("author:references:users:optional").expect("acceptée"),
        );

        assert!(is_seedable(&feature));
    }
```

Le helper `seed` du module de tests résout les relations comme celui de la Task 5 :

```rust
    fn seed(name: &str, fields: &str) -> String {
        let entities = [crate::generate::entities::Entity {
            table: "users".to_string(),
            module_path: "crate::auth::model::user".to_string(),
            file: "src/auth/model.rs".to_string(),
        }];
        let mut parsed = fields::parse(fields).expect("les champs du test doivent être valides");
        crate::generate::relations::resolve(&mut parsed, &entities, name)
            .expect("les cibles du test doivent se résoudre");
        render(&Feature::fresh(name, parsed)).expect("le seed doit se rendre")
    }
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent**

Run: `cargo test -p rbs-cli --lib seed 2>&1 | tail -20`
Expected: FAILED — `is_seedable` introuvable, et la référence optionnelle rendue en `Uuid::from_u128`.

- [ ] **Step 3: Implémenter**

Dans `seed.rs`, la valeur d'exemple d'une référence optionnelle est `None`, et la fonction qui décide de l'engendrement :

```rust
/// Une entité portant une référence **requise** ne se sème pas.
///
/// Le seed devrait connaître une ligne cible existante pour poser une valeur qui passe
/// la contrainte, ce qu'un fichier indépendant ne peut pas savoir. Ne rien engendrer vaut
/// mieux qu'engendrer un fichier qui échoue à chaque lancement.
pub(crate) fn is_seedable(feature: &Feature) -> bool {
    !feature
        .fields
        .iter()
        .any(|field| field.reference().is_some() && !field.optional)
}
```

Dans la fonction qui calcule la valeur d'exemple, traiter la référence optionnelle avant le passage par `column_type()` : elle rend `"None".to_string()`.

- [ ] **Step 4: Lancer les tests, vérifier qu'ils passent**

Run: `cargo test -p rbs-cli --lib seed 2>&1 | tail -20`
Expected: tous passent.

- [ ] **Step 5: Brancher la décision dans la génération**

Dans `crates/rbs-cli/src/generate/command.rs`, le seed et son montage ne sont produits que si `seed::is_seedable(&feature)`. Lire d'abord comment le seed est actuellement conditionné — il l'est déjà par `complete`, et la nouvelle condition s'ajoute à celle-là.

Quand le seed est écarté, la commande le dit dans sa sortie, sur le modèle des autres lignes d'information qu'elle affiche :

```
aucun seed pour posts : la référence « author » est requise, et un seed ne peut pas
deviner vers quelle ligne pointer
```

Ajouter le test d'intégration qui le prouve, dans `crates/rbs-cli/tests/integration_relations.rs` si la Task 8 l'a déjà créé, sinon dans `integration_generate.rs` : générer une feature à référence requise, vérifier que `src/<feature>/seed.rs` **n'existe pas**, que la sortie nomme la relation en cause, et que le projet compile malgré tout.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test -p rbs-cli
git add crates/rbs-cli/src/generate/seed.rs crates/rbs-cli/src/generate/command.rs crates/rbs-cli/tests/
git commit -m "fix(generate): n'engendre plus de seed inventant une clé étrangère"
```

---

### Task 7: Une ancre peut viser un fichier calculé

L'ancre du côté inverse vit dans `src/<cible>/model.rs`, chemin connu seulement à l'exécution. Or `plan/mod.rs:180` fait `let path = anchor.file`, et `Anchor.file` est un `&'static str`.

**Files:**
- Modify: `crates/rbs-cli/src/anchors.rs`
- Modify: `crates/rbs-cli/src/plan/mod.rs:180`
- Modify: `crates/rbs-cli/src/doctor/anchors.rs`

**Interfaces:**
- Produces: `Anchor { name, file: Cow<'static, str>, comment, optional }` ; `Anchor::in_file(&self, path: &str) -> Anchor` ; les deux constantes `RELATIONS` et `RELATED`.

- [ ] **Step 1: Écrire les tests**

Dans `crates/rbs-cli/src/anchors.rs` :

```rust
    #[test]
    fn an_anchor_can_be_rebound_to_a_computed_file() {
        let anchor = RELATIONS.in_file("src/posts/model.rs");

        assert_eq!(anchor.file, "src/posts/model.rs");
        assert_eq!(anchor.name, RELATIONS.name);
        assert_eq!(anchor.opening(), "// <rbs:relations>");
    }

    // Les deux ancres du modèle ne rejoignent pas le registre statique : leur fichier
    // dépend des features du projet, que `doctor` énumère autrement.
    #[test]
    fn the_model_anchors_are_absent_from_the_static_registry() {
        for anchor in ANCRES {
            assert_ne!(anchor.name, "relations", "{:?}", anchor);
            assert_ne!(anchor.name, "related", "{:?}", anchor);
        }
    }
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent**

Run: `cargo test -p rbs-cli anchors 2>&1 | tail -20`
Expected: échec de compilation, `in_file` et `RELATIONS` introuvables.

- [ ] **Step 3: Rendre `file` dynamique**

```rust
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Anchor {
    pub name: &'static str,
    /// Chemin du fichier porteur, relatif à la racine du projet.
    ///
    /// Emprunté pour les ancres du registre, dont le fichier est fixe ; possédé pour
    /// celles du modèle d'une feature, dont il dépend du nom de cette feature.
    pub file: Cow<'static, str>,
    pub comment: &'static str,
    pub optional: bool,
}

impl Anchor {
    /// La même ancre, dans un autre fichier.
    pub(crate) fn in_file(&self, path: &str) -> Anchor {
        Anchor {
            file: Cow::Owned(path.to_string()),
            ..self.clone()
        }
    }
}

/// Variantes de l'énumération `Relation` d'un modèle de feature.
///
/// Hors du registre statique : son fichier dépend de la feature visée.
pub(crate) const RELATIONS: Anchor = Anchor {
    name: "relations",
    file: Cow::Borrowed("src/{feature}/model.rs"),
    comment: "//",
    optional: false,
};

/// Implémentations de `Related` d'un modèle de feature.
pub(crate) const RELATED: Anchor = Anchor {
    name: "related",
    file: Cow::Borrowed("src/{feature}/model.rs"),
    comment: "//",
    optional: false,
};
```

Les dix constantes existantes prennent `file: Cow::Borrowed("src/main.rs")` et ainsi de suite.

- [ ] **Step 4: Réparer les emprunts que la perte de `Copy` révèle**

`Anchor` perd `Copy` : `Cow` ne l'est pas.

Run: `cargo build -p rbs-cli 2>&1 | grep -E "^error" | head -20`
Expected: des erreurs `E0507: cannot move out of ... which is behind a shared reference` et `use of moved value`, dans `plan/mod.rs`, `doctor/anchors.rs` et `generate/mount.rs`.

Chacune se répare mécaniquement, en ajoutant `.clone()` ou en passant par référence. `plan/mod.rs:180` devient :

```rust
        let path = anchor.file.to_string();
```

Ne changer aucune logique : ces erreurs sont un effet de bord du type, pas un signe de conception.

- [ ] **Step 5: Lancer toute la suite**

Run: `cargo test -p rbs-cli 2>&1 | tail -20`
Expected: aucun `FAILED`. Le nombre de tests a augmenté des deux nouveaux.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
git add crates/rbs-cli/src/anchors.rs crates/rbs-cli/src/plan/mod.rs crates/rbs-cli/src/doctor/anchors.rs crates/rbs-cli/src/generate/mount.rs
git commit -m "refactor(anchors): permet à une ancre de viser un fichier calculé"
```

---

### Task 8: Écrire le côté inverse dans le modèle de la cible

**Files:**
- Modify: `crates/rbs-cli/src/generate/relations.rs`
- Modify: `crates/rbs-cli/src/generate/command.rs`
- Modify: `crates/rbs-cli/src/cli.rs:98-112`

**Interfaces:**
- Consumes: `anchors::{RELATIONS, RELATED}` (Task 7) ; `entities::Entity` ; `RelationView`.
- Produces:
```rust
/// Ce que le côté inverse ajoute au modèle d'une entité cible.
pub(crate) struct Inverse {
    /// Fichier du modèle cible, relatif à la racine : `src/auth/model.rs`.
    pub file: String,
    /// Lignes de la variante, à insérer dans `<rbs:relations>`.
    pub variant: Vec<String>,
    /// Lignes de l'`impl Related`, à insérer dans `<rbs:related>`.
    pub related: Vec<String>,
}
pub(crate) fn inverses(
    fields: &[Field],
    feature: &Feature,
    entities: &[Entity],
) -> Vec<Inverse>;
```

- [ ] **Step 1: Écrire les tests**

```rust
    #[test]
    fn a_reference_produces_the_has_many_side_in_the_target_model() {
        let mut fields = fields::parse("author:references:users").expect("acceptée");
        resolve(&mut fields, &inventory(), "posts").expect("cibles résolues");
        let feature = Feature::fresh("posts", fields.clone());
        let produced = inverses(&fields, &feature, &inventory());

        assert_eq!(produced.len(), 1);
        assert_eq!(produced[0].file, "src/auth/model.rs");
        assert!(
            produced[0].variant.join("\n").contains(
                r#"#[sea_orm(has_many = "crate::posts::model::Entity")]"#
            ),
            "{:?}",
            produced[0].variant
        );
        assert!(produced[0].variant.join("\n").contains("Posts,"), "{:?}", produced[0].variant);
        assert!(
            produced[0]
                .related
                .join("\n")
                .contains("impl Related<crate::posts::model::Entity> for Entity {"),
            "{:?}",
            produced[0].related
        );
    }

    // Une auto-référence a déjà ses deux côtés dans le même fichier : l'inverse y serait
    // une seconde variante homonyme.
    #[test]
    fn a_self_reference_produces_no_inverse() {
        let mut fields = fields::parse("parent:references:posts:optional").expect("acceptée");
        resolve(&mut fields, &[], "posts").expect("cibles résolues");
        let feature = Feature::fresh("posts", fields.clone());

        assert!(inverses(&fields, &feature, &[]).is_empty());
    }

    #[test]
    fn two_references_to_the_same_target_produce_two_inverses_in_one_file() {
        let mut fields =
            fields::parse("author:references:users,reviewer:references:users").expect("acceptée");
        resolve(&mut fields, &inventory(), "posts").expect("cibles résolues");
        let feature = Feature::fresh("posts", fields.clone());
        let produced = inverses(&fields, &feature, &inventory());

        assert_eq!(produced.len(), 2, "{produced:?}");
        assert!(produced.iter().all(|i| i.file == "src/auth/model.rs"), "{produced:?}");
    }
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent**

Run: `cargo test -p rbs-cli relations 2>&1 | tail -20`
Expected: FAILED, `inverses` introuvable.

- [ ] **Step 3: Implémenter le calcul de l'inverse**

```rust
/// Ce que le côté inverse ajoute au modèle d'une entité cible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Inverse {
    /// Fichier du modèle cible, relatif à la racine : `src/auth/model.rs`.
    pub file: String,
    /// Lignes de la variante, à insérer dans `<rbs:relations>`.
    pub variant: Vec<String>,
    /// Lignes de l'`impl Related`, à insérer dans `<rbs:related>`.
    pub related: Vec<String>,
}

/// Calcule, pour chaque référence, ce qu'il faut écrire dans le modèle de sa cible.
///
/// Déclarer `author:references:users` sur `posts` implique que `users` a des `posts` :
/// la relation n'est écrite qu'une fois, et son inverse en découle. Une auto-référence
/// est exclue — ses deux côtés vivent déjà dans le même fichier.
pub(crate) fn inverses(fields: &[Field], feature: &Feature, entities: &[Entity]) -> Vec<Inverse> {
    let own_entity = format!("crate::{}::model::Entity", feature.module());
    let variant = to_pascal_case(feature.module());

    fields
        .iter()
        .filter_map(|field| {
            let view = field.relation()?;
            if view.target == feature.module() {
                return None;
            }
            let target = entities::find(entities, &view.target)?;

            Some(Inverse {
                file: target.file.clone(),
                variant: vec![
                    format!(r#"    #[sea_orm(has_many = "{own_entity}")]"#),
                    format!("    {variant},"),
                ],
                related: vec![
                    format!("impl Related<{own_entity}> for Entity {{"),
                    format!("    fn to() -> RelationDef {{ Relation::{variant}.def() }}"),
                    "}".to_string(),
                ],
            })
        })
        .collect()
}
```

- [ ] **Step 4: Lancer les tests, vérifier qu'ils passent**

Run: `cargo test -p rbs-cli relations 2>&1 | tail -20`
Expected: tous passent.

- [ ] **Step 5: Brancher l'inverse dans le plan de génération**

Le lot a déjà son mécanisme : `mount::Mount { anchor, lines }`, dont `command.rs:169` empile les listes avant de les donner au builder. L'inverse s'y range plutôt que d'appeler le builder directement.

Ajouter à `crates/rbs-cli/src/generate/mount.rs` :

```rust
/// Ce que le côté inverse d'une relation ajoute au modèle de sa cible.
///
/// Deux ancres et non une : la variante vit dans les accolades de l'énumération, l'`impl
/// Related` ne le peut pas.
pub(crate) fn for_inverse(inverse: &relations::Inverse) -> Vec<Mount> {
    vec![
        Mount {
            anchor: anchors::RELATIONS.in_file(&inverse.file),
            lines: inverse.variant.clone(),
        },
        Mount {
            anchor: anchors::RELATED.in_file(&inverse.file),
            lines: inverse.related.clone(),
        },
    ]
}
```

et dans `command.rs`, à la suite immédiate de la ligne 169 :

```rust
    for inverse in relations::inverses(&fields, &feature, &entities) {
        montages.extend(mount::for_inverse(&inverse));
    }
```

Les montages étant ensuite donnés au builder par la boucle existante — qui appelle `plan::Builder::insert(anchor, &lines)` —, l'ancre absente y produit déjà l'erreur `plan::Error::Anchor`, dont `Error::remedy` tire le bloc à coller. Rien à écrire de ce côté.

L'inventaire se lit une fois, au début de la planification, et sert aux deux usages :

```rust
    let entities = entities::scan(&root);
    relations::resolve(&mut fields, &entities, &options.name).map_err(Error::Targets)?;
```

Ajouter la variante d'erreur, qui rend les refus de cible tous ensemble :

```rust
    /// Une ou plusieurs cibles de relation sont introuvables dans le projet.
    #[error("{}", .0.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    Targets(Vec<relations::UnknownTarget>),
```

- [ ] **Step 6: Ajouter le flag `--has-many` de réparation**

Dans `crates/rbs-cli/src/cli.rs`, sur la variante `Crud` :

```rust
        /// Entité enfant dont ce modèle doit porter la variante inverse, répétable.
        #[arg(long = "has-many", value_name = "ENTITE")]
        has_many: Vec<String>,
```

Le flag n'écrit que le côté inverse dans le modèle **de la feature en cours**, pour une entité enfant qui existe déjà avec sa clé. Il se valide : le scan doit trouver dans le modèle de l'enfant une colonne référençant notre table. Ajouter dans `relations.rs` :

```rust
/// Vérifie que l'entité nommée porte bien une colonne référençant `table`.
///
/// Sans cette vérification, `--has-many` écrirait une variante que SeaORM rejetterait
/// quarante secondes plus tard, à la compilation.
pub(crate) fn child_references(child: &Entity, table: &str, root: &Path) -> bool {
    let Ok(source) = fs::read_to_string(root.join(&child.file)) else {
        return false;
    };
    let expected = format!(r#"belongs_to = "crate::{table}::model::Entity""#);

    source.contains(&expected)
}
```

avec son test :

```rust
    #[test]
    fn a_child_without_a_key_towards_us_is_not_a_valid_has_many() {
        let root = TempDir::new().expect("le répertoire se crée");
        fs::create_dir_all(root.path().join("src/comments")).expect("le répertoire se crée");
        fs::write(root.path().join("src/comments/model.rs"), "pub struct Model {}\n")
            .expect("l'écriture aboutit");
        let child = Entity {
            table: "comments".to_string(),
            module_path: "crate::comments::model".to_string(),
            file: "src/comments/model.rs".to_string(),
        };

        assert!(!child_references(&child, "posts", root.path()));
    }
```

- [ ] **Step 7: Vérifier l'idempotence et l'ancre absente**

Deux comportements que le §4.4 impose. Créer `crates/rbs-cli/tests/integration_relations.rs` :

```rust
//! Ce que l'écriture du côté inverse fait, et ne fait pas, au modèle de la cible.
//!
//! Le test vit ici et non dans le module du générateur : `CARGO_BIN_EXE_rbs`, dont
//! `assert_cmd` a besoin, n'est défini que pour les tests d'intégration.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Lance `rbs` dans `racine` et rend sa sortie, sans exiger qu'elle aboutisse.
fn rbs(racine: &Path, arguments: &[&str]) -> std::process::Output {
    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(racine)
        .args(arguments)
        .output()
        .expect("le binaire doit être lançable")
}

/// Un projet portant déjà `users`, cible de toutes les relations de ce fichier.
fn project_with_users(parent: &Path) -> std::path::PathBuf {
    let racine = common::projet(parent);
    let output = rbs(&racine, &["g", "crud", "users", "--fields", "email:string:unique"]);
    assert!(
        output.status.success(),
        "users doit se générer :
{}",
        String::from_utf8_lossy(&output.stderr)
    );

    racine
}

#[test]
fn a_reference_writes_the_inverse_into_the_target_model() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_users(parent.path());

    let output = rbs(
        &racine,
        &["g", "crud", "posts", "--fields", "title:string,author:references:users"],
    );
    assert!(
        output.status.success(),
        "posts doit se générer :
{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cible = fs::read_to_string(racine.join("src/users/model.rs")).expect("le modèle se lit");
    assert!(
        cible.contains(r#"has_many = "crate::posts::model::Entity""#),
        "la variante inverse est absente :
{cible}"
    );
    assert_eq!(
        cible.matches("    Posts,").count(),
        1,
        "la variante inverse est écrite plus d'une fois :
{cible}"
    );

    let porteur = fs::read_to_string(racine.join("src/posts/model.rs")).expect("le modèle se lit");
    assert!(porteur.contains("    Author,"), "{porteur}");
}

/// Le §4.4 impose l'idempotence : une seconde génération identique n'écrit pas une
/// seconde variante homonyme dans la cible.
#[test]
fn generating_the_same_relation_twice_leaves_a_single_inverse() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_users(parent.path());
    let arguments = ["g", "crud", "posts", "--fields", "title:string,author:references:users"];

    rbs(&racine, &arguments);
    let apres_la_premiere =
        fs::read_to_string(racine.join("src/users/model.rs")).expect("le modèle se lit");

    let seconde = rbs(&racine, &arguments);
    assert!(
        !seconde.status.success(),
        "la seconde génération doit échouer : la feature est déjà là"
    );
    assert_eq!(
        fs::read_to_string(racine.join("src/users/model.rs")).expect("le modèle se lit"),
        apres_la_premiere,
        "la seconde génération a retouché le modèle de la cible"
    );
}

/// Ancre absente : le CLI n'écrit rien dans ce fichier et affiche le bloc à coller.
#[test]
fn a_missing_anchor_in_the_target_writes_nothing_and_shows_the_block() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_users(parent.path());
    common::commiter(&racine, "projet et users");

    let modele = racine.join("src/users/model.rs");
    let source = fs::read_to_string(&modele).expect("le modèle se lit");
    fs::write(&modele, source.replace("    // <rbs:relations>\n", "")).expect("l'écriture aboutit");

    let avant = common::empreinte(&racine);
    let output = rbs(
        &racine,
        &["g", "crud", "posts", "--fields", "title:string,author:references:users", "--force"],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "la génération devait refuser :\n{stderr}");
    assert!(
        stderr.contains("<rbs:relations>") && stderr.contains("src/users/model.rs"),
        "le bloc à coller et son fichier doivent être affichés :\n{stderr}"
    );
    common::assert_intact(&avant, &racine, "une ancre absente laisse le projet intact");
}
```

Le dernier test éprouve la règle entière : rien n'est écrit **nulle part**, `src/posts/` compris. C'est la lecture stricte du §4.4 — le plan se calcule en entier avant que le premier fichier soit écrit, et une ancre disparue le fait échouer à la planification.

- [ ] **Step 8: Éprouver de bout en bout, à la main**

Run:
```bash
cd "$(mktemp -d)" \
  && cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli -- new demo --yes --core-path ~/dev/rs/crates/rbs-core \
  && cd demo \
  && cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli -- g crud users --fields "email:string:unique" \
  && cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli -- g crud posts --fields "title:string,author:references:users" \
  && cargo build
```
Expected: `cargo build` du projet engendré aboutit. Vérifier ensuite à l'œil que `src/users/model.rs` porte `has_many = "crate::posts::model::Entity"` et `src/posts/model.rs` la variante `Author`.

Vérifier le refus :
```bash
cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli -- g crud comments --fields "author:references:writers"
```
Expected: refus nommant `posts, users` et **aucun** répertoire `src/comments/` créé.

- [ ] **Step 9: Commit**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test -p rbs-cli
git add -A
git commit -m "feat(generate): écrit le côté inverse d'une relation dans le modèle de la cible"
```

---

### Task 9: `doctor` surveille les deux ancres du modèle

La spec §5.1 les met sous surveillance. Elles ne peuvent pas rejoindre le registre statique `ANCRES` : leur fichier dépend des features du projet. `doctor` les vérifie donc en énumérant les entités, avec le scan de la Task 2 — comme il traite déjà `redis`, installé en `src/cache/` sous un nom qui n'est pas le sien.

**Files:**
- Create: `crates/rbs-cli/src/doctor/relations.rs`
- Modify: `crates/rbs-cli/src/doctor/mod.rs`

**Interfaces:**
- Consumes: `entities::scan` (Task 2) ; `anchors::{RELATIONS, RELATED}` (Task 7) ; le type `Check` et sa forme, à lire dans `crates/rbs-cli/src/doctor/anchors.rs`.
- Produces: `pub(crate) fn check(root: &Path) -> Check`.

`Check` porte `{ title: &'static str, state: State, detail: String, remedy: Option<String> }`, et se construit par `Check::ok(title, detail)` ou `Check::failed(title, detail, remedy)`.

- [ ] **Step 1: Écrire les tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const MODEL: &str = r#"
#[sea_orm(table_name = "posts")]
pub struct Model { pub id: Uuid }

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations>
    // </rbs:relations>
}
// <rbs:related>
// </rbs:related>
"#;

    fn project(source: &str) -> TempDir {
        let root = TempDir::new().expect("le répertoire se crée");
        let directory = root.path().join("src/posts");
        fs::create_dir_all(&directory).expect("le répertoire se crée");
        fs::write(directory.join("model.rs"), source).expect("l'écriture aboutit");
        root
    }

    #[test]
    fn a_model_carrying_both_anchors_passes() {
        assert_eq!(check(project(MODEL).path()).state, State::Bon);
    }

    #[test]
    fn a_model_missing_one_anchor_fails_by_naming_its_file() {
        let amputé = MODEL.replace("    // </rbs:relations>\n", "");
        let contrôle = check(project(&amputé).path());

        assert_eq!(contrôle.state, State::Echec);
        assert!(contrôle.detail.contains("src/posts/model.rs"), "{contrôle:?}");
        assert!(contrôle.detail.contains("relations"), "{contrôle:?}");
        assert!(
            contrôle.remedy.expect("un remède").contains("<rbs:relations>"),
            "le bloc à coller doit être donné"
        );
    }

    // Un projet engendré avant ce jalon n'a aucune des deux ancres : le contrôle doit
    // le dire une fois par fichier, non deux fois par fichier.
    #[test]
    fn a_model_missing_both_anchors_is_reported_once() {
        let sans = MODEL
            .replace("    // <rbs:relations>\n", "")
            .replace("    // </rbs:relations>\n", "")
            .replace("// <rbs:related>\n", "")
            .replace("// </rbs:related>\n", "");
        let contrôle = check(project(&sans).path());

        assert_eq!(
            contrôle.detail.matches("src/posts/model.rs").count(),
            1,
            "{contrôle:?}"
        );
    }

    #[test]
    fn a_project_without_any_entity_has_nothing_to_report() {
        let root = TempDir::new().expect("le répertoire se crée");

        assert_eq!(check(root.path()).state, State::Bon);
    }
}
```

- [ ] **Step 2: Lancer les tests, vérifier qu'ils échouent**

Run: `cargo test -p rbs-cli doctor::relations 2>&1 | tail -20`
Expected: échec de compilation, `check` introuvable.

- [ ] **Step 3: Implémenter le contrôle**

```rust
//! Les deux ancres qu'un modèle de feature doit porter pour recevoir une relation.
//!
//! Hors du registre statique des ancres : leur fichier dépend des features du projet,
//! qui ne se connaissent qu'en le parcourant.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::anchors::{RELATED, RELATIONS};
use crate::generate::entities;

/// Vérifie que chaque modèle du projet porte ses deux ancres de relation.
pub(crate) fn check(root: &Path) -> Check {
    // Un même fichier peut porter plusieurs entités — `auth` en porte deux : le
    // dédoublonner évite de nommer deux fois le même modèle incomplet.
    let files: BTreeSet<String> = entities::scan(root)
        .into_iter()
        .map(|entity| entity.file)
        .collect();

    let incomplets: Vec<String> = files
        .into_iter()
        .filter(|file| {
            let Ok(source) = fs::read_to_string(root.join(file)) else {
                return false;
            };

            [&RELATIONS, &RELATED].iter().any(|anchor| {
                !source.contains(&anchor.opening()) || !source.contains(&anchor.closing())
            })
        })
        .collect();

    if incomplets.is_empty() {
        return Check::ok(TITRE, "les modèles portent leurs ancres de relation");
    }

    let detail = incomplets
        .iter()
        .map(|file| format!("relations manquent dans {file}"))
        .collect::<Vec<_>>()
        .join(", ");

    let remedy = incomplets
        .iter()
        .map(|file| {
            format!(
                "dans {file} :\n{}\n\n{}",
                RELATIONS.block(),
                RELATED.block()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    Check::failed(TITRE, detail, remedy)
}

const TITRE: &str = "relations";
```

- [ ] **Step 4: Brancher le contrôle**

Dans `crates/rbs-cli/src/doctor/mod.rs`, déclarer `mod relations;` et ajouter son appel à la liste des contrôles, **après** celui des ancres statiques : les deux se lisent ensemble dans le rapport.

- [ ] **Step 5: Vérifier sur un vrai projet**

Run:
```bash
cd "$(mktemp -d)" \
  && cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli -- new demo --yes --core-path ~/dev/rs/crates/rbs-core \
  && cd demo \
  && cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli -- g crud posts --fields "title:string" \
  && cargo run --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli -- doctor
```
Expected: le contrôle des ancres de relation paraît au vert.

Puis retirer `// <rbs:related>` de `src/posts/model.rs` et relancer `doctor` : le contrôle passe au rouge, nomme `src/posts/model.rs` et affiche le bloc à coller.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test -p rbs-cli
git add crates/rbs-cli/src/doctor/relations.rs crates/rbs-cli/src/doctor/mod.rs
git commit -m "feat(doctor): surveille les deux ancres de relation des modèles"
```

---

## Ce que ce plan ne fait pas

Les lots `R4` à `R8` de la spec — lecture `?include=`, écriture et traduction en 409, plusieurs-à-plusieurs, exemple compilé en CI, documentation bilingue — font l'objet de deux plans séparés, écrits à l'issue de celui-ci. Un plan écrit avant d'avoir rencontré le code qu'il planifie planifie une supposition.

À l'issue de ce plan, un projet engendré porte ses clés étrangères, ses variantes SeaORM des deux côtés, ses contraintes et ses index. Aucune route ne les expose encore : `GET /posts` rend `author_id`, et rien de plus.
