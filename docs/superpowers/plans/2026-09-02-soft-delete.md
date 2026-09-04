# `rbs generate crud --soft-delete` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un drapeau `--soft-delete` sur `rbs generate crud` qui rend le `DELETE` logique — la ligne reste, marquée d'une date — sans que le contrat HTTP du projet engendré change d'un octet.

**Architecture:** Le drapeau descend jusqu'à `Feature`, que les templates lisent. Trois templates changent : la migration (colonne, index, unicité déplacée dans un index partiel branché sur le moteur), le modèle (le champ), et le repository — seule couche autorisée à construire une requête, donc seule à filtrer les lignes supprimées et à réécrire le `delete`.

**Tech Stack:** Rust 2024, clap (dérive), minijinja aux délimiteurs `{@ @}`, SeaORM 2.0.2 / sea-query 1.0.2, `assert_cmd` + `testcontainers` pour les bancs.

**Spec:** `docs/superpowers/specs/2026-09-02-soft-delete-design.md`

## Global Constraints

- **Délimiteurs minijinja alternatifs** : les expressions s'écrivent `{@ variable @}`, jamais `{{ }}` (`crates/rbs-cli/src/template.rs:19-22`). Les blocs `{% %}` et commentaires `{# #}` gardent la syntaxe Jinja.
- **`UndefinedBehavior::Strict`** (`template.rs:26`) : une variable référencée par une template et absente du contexte **fait échouer le rendu**. Trois générateurs reconstruisent leur contexte au lieu de sérialiser `Feature` — `filter.rs:44`, `tests_http.rs:36`, `seed.rs:30` — et ne recevront `soft_delete` que si leur template le référence. Aucune ne le référencera dans ce plan.
- **Les templates écrivent ce que rustfmt écrirait**, elles ne le laissent pas rattraper : seuils de 100 colonnes (`max_width`), 98 pour un `use`, 60 pour une chaîne (`chain_width`) et pour les arguments d'un appel (`fn_call_width`). Les macros `entete` et `chaine` de `repository.rs.jinja:7-36` sont là pour ça.
- **Un commentaire explique le *pourquoi*, jamais le *quoi*.** Le code engendré ne commente que ses points d'extension et ses pièges ; pas de bandeau « généré, ne pas modifier ».
- Un fichier de feature au-delà de ~200 lignes signale une feature à scinder.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont bloquants en CI. Le code **engendré** doit lui aussi passer `clippy -D warnings` : un `use` inutile sous un mode le ferait échouer.
- Les bancs `#[ignore]` compilent un projet Axum + SeaORM complet et exigent Docker. **`--no-fail-fast` est obligatoire** sur la suite lente : sans lui elle s'arrête au premier binaire et masque les échecs suivants.
- `static CARGO: Mutex<()>` (`bench.rs:80`) sérialise les invocations cargo, les projets partageant `target/rbs-integration` : **deux bancs ne doivent jamais poser une feature ni une migration de même nom.**
- Rediriger la sortie des suites longues vers le scratchpad, sinon les chiffres sont rognés.
- CHANGELOG et documentation vont **par paire** anglais/français, dans le même commit. `docs/scripts/parite.mjs` le contrôle.
- Commits en Conventional Commits, sujet français à l'impératif, sans identifiant de tâche, sans renvoi à un fichier de suivi, **sans `Co-Authored-By` ni mention d'un assistant**. Corps avec un intertitre `Vérifications :`.
- Branche `improve/p3-features-lot-un`, jamais `main`.

## File Structure

| Fichier | Responsabilité | Action |
|---|---|---|
| `crates/rbs-cli/src/lib.rs:93-115, 455-478` | descente des drapeaux : le tuple positionnel devient une struct | Modifier |
| `crates/rbs-cli/src/cli.rs:136-158` | déclaration du drapeau | Modifier |
| `crates/rbs-cli/src/generate/command.rs:24-39, 204-245` | `Options`, refus de `deleted_at`, report sur `Feature` | Modifier |
| `crates/rbs-cli/src/generate/feature.rs:16-23, 222-237` | le champ et sa clé de sérialisation | Modifier |
| `crates/rbs-cli/templates/feature/migration.rs.jinja` | colonne, index, index partiel branché | Modifier |
| `crates/rbs-cli/templates/feature/model.rs.jinja` | le champ sur `Model` | Modifier |
| `crates/rbs-cli/templates/feature/repository.rs.jinja` | lectures filtrées, `delete` réécrit | Modifier |
| `crates/rbs-cli/templates/feature/tests.rs.jinja` | assertion du second `DELETE` (inconditionnelle) | Modifier |
| `CHANGELOG.md` / `.fr.md`, guide + paire fr | notes et documentation | Modifier |

---

### Task 1: Remplacer le tuple positionnel par une struct

**Files:**
- Modify: `crates/rbs-cli/src/lib.rs:93-115` (le `match command` et l'appel), `:455-478` (la fonction `generate`)
- Test: `crates/rbs-cli/src/lib.rs` — aucun test nouveau ; les tests existants sont le filet.

**Interfaces:**
- Consomme : `generate::command::Options` (`command.rs:24-39`), déjà une struct.
- Produit : `struct GenerateArgs { name, fields, complete, force, dry_run, has_many, role }` privée à `lib.rs`, et `fn generate(args: GenerateArgs) -> Result<(), generate::command::Error>`. Les Tasks 2 et le plan `--with-upload` y ajoutent leurs champs.

**Pourquoi cette tâche vient en premier, et pourquoi elle est dans le périmètre :** `lib.rs:93` destructure un tuple à **sept** éléments, aussitôt repassé à une fonction à **sept** paramètres positionnels de même type (`bool`, `bool`, `bool` se suivent). `--soft-delete` puis `--with-upload` le porteraient à neuf, et une inversion entre deux `bool` voisins ne serait rattrapée par aucun compilateur. C'est du code que ce lot traverse de toute façon.

**Aucun changement de comportement.** Si un test existant change de résultat, la refonte est fausse.

- [ ] **Step 1: Établir le vert de départ**

Run: `cargo test -p rbs-cli --lib 2>&1 | tail -5`
Expected: PASS. **Noter le nombre exact de tests** — il doit être identique à la fin de la tâche.

- [ ] **Step 2: Écrire la struct et la fonction**

Dans `crates/rbs-cli/src/lib.rs`, juste avant `fn generate` (ligne 455), ajouter :

```rust
/// Ce que la ligne de commande dit d'une génération.
///
/// Une struct et non des paramètres positionnels : les drapeaux sont pour moitié des
/// `bool` voisins, et une inversion entre deux d'entre eux ne se verrait qu'à l'exécution.
struct GenerateArgs {
    name: String,
    fields: Option<String>,
    complete: bool,
    force: bool,
    dry_run: bool,
    has_many: Vec<String>,
    role: Option<String>,
}
```

Remplacer la signature de `generate` (`lib.rs:455-463`) par :

```rust
fn generate(args: GenerateArgs) -> Result<(), generate::command::Error> {
    let GenerateArgs {
        name,
        fields,
        complete,
        force,
        dry_run,
        has_many,
        role,
    } = args;

    let feature = name.clone();
```

Le corps de la fonction reste **inchangé** à partir de la ligne `// \`--has-many\` répare une feature déjà là`.

- [ ] **Step 3: Réécrire le site d'appel**

Dans `crates/rbs-cli/src/lib.rs`, remplacer le bloc `lib.rs:93-115` par :

```rust
        Commands::Generate { command } => {
            let args = match command {
                GenerateCommands::Crud {
                    name,
                    fields,
                    force,
                    dry_run,
                    has_many,
                    role,
                } => GenerateArgs {
                    name,
                    fields,
                    complete: true,
                    force,
                    dry_run,
                    has_many,
                    role,
                },
                GenerateCommands::Feature {
                    name,
                    force,
                    dry_run,
                } => GenerateArgs {
                    name,
                    fields: None,
                    complete: false,
                    force,
                    dry_run,
                    has_many: Vec::new(),
                    role: None,
                },
            };

            if let Err(error) = generate(args) {
                ui::error(&error.to_string());
                if let Some(remedy) = error.remedy() {
                    ui::info(&format!("\n{remedy}"));
                }
                std::process::exit(1);
            }
        }
```

- [ ] **Step 4: Vérifier que rien n'a bougé**

Run: `cargo test -p rbs-cli --lib 2>&1 | tail -5 && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected: **le même nombre de tests qu'au Step 1**, 0 échec ; clippy et fmt sans sortie.

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-cli/src/lib.rs
git commit -F - <<'EOF'
refactor(cli): nomme les arguments de la génération au lieu de les compter

Le tuple destructuré et la fonction qu'il alimentait portaient sept positions,
dont trois booléens qui se suivent : une inversion entre deux d'entre eux ne se
serait vue qu'à l'exécution, sur un projet engendré de travers. Les drapeaux à
venir en auraient ajouté deux de plus, du même type.

Aucun changement de comportement : le nombre de tests et leur résultat sont
identiques des deux côtés du commit.

Vérifications :
- cargo test -p rbs-cli --lib : même compte qu'avant la refonte, 0 échec
- cargo clippy -p rbs-cli --all-targets -- -D warnings : aucune sortie
- cargo fmt --all --check : aucune sortie
EOF
```

---

### Task 2: Le drapeau, sa descente et le refus de `deleted_at`

**Files:**
- Modify: `crates/rbs-cli/src/cli.rs:136-158`
- Modify: `crates/rbs-cli/src/lib.rs` (`GenerateArgs`, le `match`, la construction d'`Options`)
- Modify: `crates/rbs-cli/src/generate/command.rs:24-39` (`Options`), `:130-160` (une variante d'`Error`), `:204-245` (`plan_for`)
- Modify: `crates/rbs-cli/src/generate/feature.rs:16-23, 26-36, 222-237`
- Test: `crates/rbs-cli/src/generate/feature.rs` (`mod tests`), `crates/rbs-cli/src/cli.rs` (`mod tests`)

**Interfaces:**
- Consomme : `GenerateArgs` de la Task 1.
- Produit :
  - `Feature.soft_delete: bool`, posé par `Feature::soft_deleting()` (consommateur : Tasks 3 à 5, via la clé `soft_delete` du contexte) ;
  - `command::Error::SoftDeleteColonneReservee { colonne: String }` ;
  - la clé template **`soft_delete`** (booléen), disponible dans tout contexte issu de `Feature`.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/rbs-cli/src/generate/feature.rs`, `mod tests` :

```rust
    #[test]
    fn an_ordinary_feature_does_not_delete_softly() {
        let feature = Feature::fresh("articles", Vec::new());
        let rendu = serde_json::to_value(&feature).expect("la feature se sérialise");

        assert_eq!(
            rendu["soft_delete"], false,
            "sans le drapeau, la clé existe et vaut faux : {rendu}"
        );
    }

    #[test]
    fn soft_deleting_marks_the_feature() {
        let feature = Feature::fresh("articles", Vec::new()).soft_deleting();
        let rendu = serde_json::to_value(&feature).expect("la feature se sérialise");

        assert_eq!(rendu["soft_delete"], true);
    }
```

Dans `crates/rbs-cli/src/cli.rs`, `mod tests`, à côté des tests de drapeaux existants :

```rust
    #[test]
    fn generate_crud_accepts_soft_delete() {
        let cli = Cli::try_parse_from(["rbs", "generate", "crud", "articles", "--soft-delete"])
            .expect("la ligne doit être acceptée");

        let Commands::Generate {
            command: GenerateCommands::Crud { soft_delete, .. },
        } = cli.command
        else {
            panic!("la sous-commande doit être `generate crud`");
        };

        assert!(soft_delete);
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli --lib soft_delet 2>&1 | tail -20`
Expected: FAIL à la compilation — `no method named 'soft_deleting'` et `struct 'Crud' has no field named 'soft_delete'`.

- [ ] **Step 3: Déclarer le drapeau**

Dans `crates/rbs-cli/src/cli.rs`, dans la variante `GenerateCommands::Crud`, après le champ `role` (ligne 155-156) :

```rust
        /// Rend le DELETE logique : la ligne reste, marquée d'une date de suppression.
        #[arg(long)]
        soft_delete: bool,
```

- [ ] **Step 4: Le faire descendre**

Dans `crates/rbs-cli/src/lib.rs` : ajouter `soft_delete: bool,` à `GenerateArgs`, le lire dans le bras `Crud` du `match`, poser `soft_delete: false` dans le bras `Feature`, l'ajouter à la destructuration de `generate`, et le passer à `Options`.

Dans `crates/rbs-cli/src/generate/command.rs`, à la fin de `Options` (après `role`, ligne 38) :

```rust
    /// Rend le `DELETE` logique : la ligne reste, sa colonne `deleted_at` datée.
    pub soft_delete: bool,
```

- [ ] **Step 5: Porter le drapeau sur `Feature`**

Dans `crates/rbs-cli/src/generate/feature.rs`, ajouter à la struct (après `role`, ligne 22) :

```rust
    /// Le `DELETE` marque la ligne au lieu de la retirer.
    pub soft_delete: bool,
```

Poser `soft_delete: false` dans `Feature::fresh` (ligne 26-32), puis ajouter après `guarded` :

```rust
    /// La même feature, dont le `DELETE` marque la ligne au lieu de la retirer.
    pub(crate) fn soft_deleting(mut self) -> Self {
        self.soft_delete = true;
        self
    }
```

Dans l'`impl Serialize` (`feature.rs:222-237`), **passer `serialize_struct("Feature", 10)` à `11`** et ajouter, après la ligne de `role` :

```rust
        state.serialize_field("soft_delete", &self.soft_delete)?;
```

Dans `command.rs:237-240`, remplacer la construction de `feature` par :

```rust
    let feature = match &options.role {
        Some(role) => Feature::fresh(&options.name, fields).guarded(role),
        None => Feature::fresh(&options.name, fields),
    };
    let feature = if options.soft_delete {
        feature.soft_deleting()
    } else {
        feature
    };
```

- [ ] **Step 6: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-cli --lib soft_delet`
Expected: PASS, 3 tests.

- [ ] **Step 7: Écrire le test du refus, et le voir échouer**

Dans `crates/rbs-cli/src/generate/command.rs`, `mod tests` :

```rust
    #[test]
    fn a_field_named_deleted_at_is_refused_under_soft_delete() {
        let message = Error::SoftDeleteColonneReservee {
            colonne: "deleted_at".to_owned(),
        }
        .to_string();

        assert!(
            message.contains("--soft-delete") && message.contains("deleted_at"),
            "le refus doit nommer le drapeau et la colonne : {message}"
        );
    }
```

Run: `cargo test -p rbs-cli --lib a_field_named_deleted_at 2>&1 | tail -10`
Expected: FAIL, `no variant named 'SoftDeleteColonneReservee'`.

- [ ] **Step 8: Écrire le refus**

Dans l'`enum Error` de `crates/rbs-cli/src/generate/command.rs`, à côté de `RoleSansAuth` :

```rust
    /// `--soft-delete` sur une entité qui déclare déjà la colonne que le drapeau injecte.
    #[error(
        "`--soft-delete` pose lui-même la colonne `{colonne}` : retirez-la de `--fields`, \
         ou renoncez au drapeau"
    )]
    SoftDeleteColonneReservee {
        /// Nom de la colonne en conflit, tel qu'il a été déclaré.
        colonne: String,
    },
```

Dans `plan_for`, **après** `fields::parse` (ligne 228-229) et avant `entities::scan` :

```rust
    // La colonne est injectée par le drapeau, non déclarée. Hors du drapeau elle reste un
    // nom libre : la réserver dans `NAMES_SET_BY_RBS` casserait un `--fields` légitime
    // sur tous les CRUD qui ne suppriment pas logiquement.
    if options.soft_delete {
        if let Some(champ) = fields.iter().find(|champ| champ.name == "deleted_at") {
            return Err(Error::SoftDeleteColonneReservee {
                colonne: champ.name.clone(),
            });
        }
    }
```

- [ ] **Step 9: Lancer les tests, lint, commit**

Run: `cargo test -p rbs-cli --lib && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected: tests verts ; clippy et fmt sans sortie.

```bash
git add crates/rbs-cli/src/cli.rs crates/rbs-cli/src/lib.rs crates/rbs-cli/src/generate/command.rs crates/rbs-cli/src/generate/feature.rs
git commit -F - <<'EOF'
feat(generate): fait descendre un drapeau de suppression logique jusqu'aux gabarits

Le drapeau se lit sur la ligne de commande, se pose sur la feature et devient
une clé du contexte des gabarits. Rien ne le lit encore : les gabarits suivent.

La colonne `deleted_at` est refusée dans `--fields` sous le drapeau, et
seulement sous lui. L'inscrire parmi les noms que rbs pose lui-même casserait
un `--fields` légitime sur tout CRUD qui ne supprime pas logiquement, alors
qu'aucune colonne n'y est injectée.

Vérifications :
- cargo test -p rbs-cli --lib : 0 échec, dont les trois tests du drapeau
- cargo clippy -p rbs-cli --all-targets -- -D warnings : aucune sortie
- cargo fmt --all --check : aucune sortie
EOF
```

---

### Task 3: La colonne dans le modèle et la migration

**Files:**
- Modify: `crates/rbs-cli/templates/feature/model.rs.jinja`
- Modify: `crates/rbs-cli/templates/feature/migration.rs.jinja`
- Test: `crates/rbs-cli/src/generate/entity.rs` (`mod tests`), `crates/rbs-cli/src/generate/migration.rs` (`mod tests`)

**Interfaces:**
- Consomme : la clé template `soft_delete` (Task 2).
- Produit : la colonne `deleted_at` sur `Model` et dans la migration, la variante `DeletedAt` de l'`enum DeriveIden`, l'index `idx_<table>_deleted_at`. La Task 5 s'appuie sur `Column::DeletedAt`, que la variante `DeletedAt` du modèle rend disponible.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/rbs-cli/src/generate/entity.rs`, `mod tests` — reprendre le helper local du module (celui qui parse les champs et appelle `render`) et ajouter :

```rust
    #[test]
    fn soft_delete_adds_a_nullable_deletion_date() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"))
            .soft_deleting();
        let rendered = render(&feature).expect("le modèle doit se rendre");

        assert!(
            rendered.contains("pub deleted_at: Option<DateTimeWithTimeZone>,"),
            "la colonne est nullable : une ligne vivante n'a pas de date :\n{rendered}"
        );
    }

    #[test]
    fn an_ordinary_model_carries_no_deletion_date() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"));
        let rendered = render(&feature).expect("le modèle doit se rendre");

        assert!(
            !rendered.contains("deleted_at"),
            "sans le drapeau, rien n'est injecté :\n{rendered}"
        );
    }
```

Dans `crates/rbs-cli/src/generate/migration.rs`, `mod tests` — reprendre le helper local du module :

```rust
    #[test]
    fn soft_delete_creates_the_column_and_its_index() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"))
            .soft_deleting();
        let rendered = render(&feature, "20260902_000000")
            .expect("la migration doit se rendre")
            .content;

        assert!(
            rendered.contains("ColumnDef::new(Articles::DeletedAt)"),
            "la colonne manque :\n{rendered}"
        );
        assert!(
            rendered.contains("idx_articles_deleted_at"),
            "toute lecture filtre sur cette colonne, elle doit être indexée :\n{rendered}"
        );
        assert!(
            rendered.contains("    DeletedAt,"),
            "l'enum DeriveIden doit porter la variante :\n{rendered}"
        );
    }

    #[test]
    fn the_unique_constraint_moves_to_a_partial_index() {
        let feature = Feature::fresh(
            "articles",
            fields::parse("title:string:unique").expect("champs"),
        )
        .soft_deleting();
        let rendered = render(&feature, "20260902_000000")
            .expect("la migration doit se rendre")
            .content;

        assert!(
            !rendered.contains(".unique_key()"),
            "la contrainte de colonne serait inconditionnelle, l'index partiel n'y \
             changerait rien :\n{rendered}"
        );
        assert!(
            rendered.contains("uq_articles_title") && rendered.contains("Articles::DeletedAt)"),
            "l'unicité doit passer par un index restreint aux lignes vivantes :\n{rendered}"
        );
    }

    #[test]
    fn mysql_keeps_a_global_uniqueness() {
        let feature = Feature::fresh(
            "articles",
            fields::parse("title:string:unique").expect("champs"),
        )
        .soft_deleting();
        let rendered = render(&feature, "20260902_000000")
            .expect("la migration doit se rendre")
            .content;

        assert!(
            rendered.contains("sea_orm::DbBackend::MySql"),
            "MySQL n'a pas d'index partiel : la migration doit brancher, faute de quoi \
             elle ne s'y applique pas du tout :\n{rendered}"
        );
    }

    #[test]
    fn an_ordinary_migration_keeps_its_unique_key() {
        let feature = Feature::fresh(
            "articles",
            fields::parse("title:string:unique").expect("champs"),
        );
        let rendered = render(&feature, "20260902_000000")
            .expect("la migration doit se rendre")
            .content;

        assert!(rendered.contains(".unique_key()"), "témoin :\n{rendered}");
        assert!(!rendered.contains("deleted_at"), "témoin :\n{rendered}");
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli --lib -- entity::tests migration::tests 2>&1 | tail -20`
Expected: FAIL sur les quatre nouveaux tests, `an_ordinary_*` passant déjà.

- [ ] **Step 3: Ajouter la colonne au modèle**

Dans `crates/rbs-cli/templates/feature/model.rs.jinja`, dans la struct `Model`, après la ligne `pub updated_at: DateTimeWithTimeZone,` :

```jinja
{%- if soft_delete %}
    // Nulle tant que la ligne vit. Le repository la remplit au lieu de retirer la ligne,
    // et écarte de toute lecture celles qui la portent.
    pub deleted_at: Option<DateTimeWithTimeZone>,
{%- endif %}
```

- [ ] **Step 4: Ajouter la colonne et les index à la migration**

Dans `crates/rbs-cli/templates/feature/migration.rs.jinja` :

**(a)** Neutraliser la contrainte de colonne sous le drapeau. Remplacer la ligne
`{%- set unicite = ".unique_key()" if field.unique else "" %}` par :

```jinja
{#- Sous suppression logique, l'unicité quitte la colonne pour un index restreint aux
    lignes vivantes : laissée ici, la contrainte serait inconditionnelle et l'index
    partiel n'y changerait rien. #}
{%- set unicite = ".unique_key()" if field.unique and not soft_delete else "" %}
```

et, dans la forme longue de la même boucle, remplacer
`{@ "\n                            .unique_key()" if field.unique else "" @}` par
`{@ "\n                            .unique_key()" if field.unique and not soft_delete else "" @}`.

**(b)** Ajouter la colonne, après le bloc `UpdatedAt` et avant `.to_owned(),` :

```jinja
{%- if soft_delete %}
                    .col(
                        ColumnDef::new({@ iden @}::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
{%- endif %}
```

**(c)** Après la boucle `{% for field in fields if field.index %}` et avant `Ok(())`, ajouter :

```jinja
{% if soft_delete %}
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_{@ table @}_deleted_at")
                    .table({@ iden @}::Table)
                    .col({@ iden @}::DeletedAt)
                    .to_owned(),
            )
            .await?;
{% for field in fields if field.unique %}
        // PostgreSQL et SQLite savent restreindre un index à un sous-ensemble de lignes :
        // deux lignes portent alors la même valeur si l'une est supprimée. MySQL ne le
        // sait pas — l'unicité y reste globale, et une valeur supprimée y reste réservée.
        let mut uq_{@ table @}_{@ field.name @} = Index::create()
            .if_not_exists()
            .unique()
            .name("uq_{@ table @}_{@ field.name @}")
            .table({@ iden @}::Table)
            .col({@ iden @}::{@ field.pascal_name @})
            .to_owned();

        if !matches!(
            manager.get_database_backend(),
            sea_orm::DbBackend::MySql
        ) {
            uq_{@ table @}_{@ field.name @} = uq_{@ table @}_{@ field.name @}
                .and_where(Expr::col({@ iden @}::DeletedAt).is_null())
                .to_owned();
        }

        manager.create_index(uq_{@ table @}_{@ field.name @}).await?;
{% endfor %}
{% endif %}
```

**(d)** Dans l'`enum {@ iden @}` en fin de fichier, après `UpdatedAt,` :

```jinja
{%- if soft_delete %}
    DeletedAt,
{%- endif %}
```

**Note sur les imports du code engendré :** `sea_orm_migration::prelude::*` met `Index`, `Expr`, `ConditionalStatement` (qui porte `and_where`) et `sea_orm` à portée — vérifié dans `sea-orm-migration-2.0.2/src/prelude.rs`. `DbBackend` n'y est pas ré-exporté nu, d'où le chemin qualifié `sea_orm::DbBackend::MySql`. `matches!` évite de dépendre d'un `PartialEq` sur l'énumération.

- [ ] **Step 5: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-cli --lib -- entity::tests migration::tests`
Expected: PASS.

- [ ] **Step 6: Vérifier la conformité rustfmt du rendu**

Run: `cargo test -p rbs-cli --lib -- migration::tests::the_render_is_already_what_rustfmt_would_write --exact` (ou le nom exact du test de divergence du module).
Expected: PASS. Si la plage de divergence a bougé, **ajuster la template**, pas le test — c'est la template qui doit écrire ce que rustfmt écrirait. Un blanc perdu par un `-%}` n'est vu que par `integration_examples` : le rattraper ici.

- [ ] **Step 7: Commit**

```bash
git add crates/rbs-cli/templates/feature/model.rs.jinja crates/rbs-cli/templates/feature/migration.rs.jinja crates/rbs-cli/src/generate/entity.rs crates/rbs-cli/src/generate/migration.rs
git commit -F - <<'EOF'
feat(generate): pose la colonne de suppression logique et déplace l'unicité

La colonne est nullable et sans défaut : une ligne vivante n'a pas de date. Un
index l'accompagne, toute lecture la filtrant désormais.

L'unicité quitte la contrainte de colonne pour un index restreint aux lignes
vivantes, sans quoi une adresse supprimée resterait réservée et une
réinscription recevrait un 409 que rien n'explique. MySQL n'a pas d'index
partiel et sea-query ne l'en protège pas — son constructeur écrit le WHERE
comme les autres, ce qui y produirait une erreur de syntaxe. La migration
branche donc à l'exécution, et le commentaire engendré dit ce que MySQL y perd.

Vérifications :
- cargo test -p rbs-cli --lib -- entity::tests migration::tests : 0 échec
- le rendu reste conforme à ce que rustfmt écrirait
EOF
```

---

### Task 4: Le repository, lectures filtrées et `delete` réécrit

**Files:**
- Modify: `crates/rbs-cli/templates/feature/repository.rs.jinja`
- Test: `crates/rbs-cli/src/generate/repository.rs` (`mod tests`)

**Interfaces:**
- Consomme : la clé `soft_delete`, la colonne `DeletedAt` de la Task 3.
- Produit : un `repository.rs` engendré dont `list`, `filter` et `find` écartent les lignes supprimées et dont `delete` les marque. La signature `delete(db, id) -> Result<bool>` **ne change pas** — `service.rs.jinja` et son `Error::NotFound` restent intacts.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/rbs-cli/src/generate/repository.rs`, `mod tests` :

```rust
    #[test]
    fn the_delete_marks_the_row_instead_of_removing_it() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"))
            .soft_deleting();
        let rendered = render(&feature).expect("le repository doit se rendre");

        assert!(
            !rendered.contains("delete_by_id"),
            "la ligne ne doit plus partir :\n{rendered}"
        );
        assert!(
            rendered.contains("Entity::update_many()") && rendered.contains("Column::DeletedAt"),
            "la suppression doit dater la colonne :\n{rendered}"
        );
    }

    #[test]
    fn a_second_delete_still_answers_404() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"))
            .soft_deleting();
        let rendered = render(&feature).expect("le repository doit se rendre");

        assert_eq!(
            rendered.matches("Column::DeletedAt.is_null()").count(),
            3,
            "les deux lectures et le delete portent la condition ; sans elle sur le \
             delete, une seconde suppression rendrait 204 :\n{rendered}"
        );
    }

    #[test]
    fn every_read_hides_the_deleted_rows() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"))
            .soft_deleting();
        let rendered = render(&feature).expect("le repository doit se rendre");

        assert!(
            rendered.contains("Entity::find().filter(Column::DeletedAt.is_null())"),
            "`filter`, dont `list` dépend, doit écarter les lignes supprimées :\n{rendered}"
        );
        assert!(
            rendered.contains("Entity::find_by_id(id)") && rendered.contains("QueryFilter"),
            "`find` doit les écarter aussi :\n{rendered}"
        );
    }

    #[test]
    fn an_ordinary_repository_imports_only_what_it_uses() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"));
        let rendered = render(&feature).expect("le repository doit se rendre");

        assert!(rendered.contains("delete_by_id"), "témoin :\n{rendered}");
        assert!(
            !rendered.contains("QueryFilter") && !rendered.contains("ColumnTrait"),
            "un import inutilisé ferait échouer clippy sur le projet engendré :\n{rendered}"
        );
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli --lib -- repository::tests 2>&1 | tail -20`
Expected: FAIL sur les trois premiers, `an_ordinary_repository_imports_only_what_it_uses` passant déjà.

- [ ] **Step 3: Modifier la template**

Dans `crates/rbs-cli/templates/feature/repository.rs.jinja` :

**(a)** Le bloc `use` (lignes 37-46) devient :

```jinja
use rbs_core::{Error, Pagination, Result};
use sea_orm::error::SqlErr;
use sea_orm::prelude::Uuid;
{%- if soft_delete %}
use sea_orm::prelude::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QuerySelect,
};
{%- else %}
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, QuerySelect,
};
{%- endif %}

use super::filter::{self, {@ entity @}Filter};
{%- if soft_delete %}
use super::model::{Column, Entity};
{%- else %}
use super::model::Entity;
{%- endif %}
```

**(b)** Dans `filter`, remplacer `let requete = filter::apply(Entity::find(), filtre)?;` par :

```jinja
{%- if soft_delete %}
    // Le filtre s'applique à ce qui reste : une ligne supprimée n'est plus une ligne que
    // l'API connaisse, et la faire réapparaître par un filtre serait une fuite.
    let requete = filter::apply(Entity::find().filter(Column::DeletedAt.is_null()), filtre)?;
{%- else %}
    let requete = filter::apply(Entity::find(), filtre)?;
{%- endif %}
```

**(c)** Remplacer `find` par :

```jinja
{%- if soft_delete %}
pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id)
        .filter(Column::DeletedAt.is_null())
        .one(db)
        .await?)
}
{%- else %}
pub async fn find(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id).one(db).await?)
}
{%- endif %}
```

**(d)** Remplacer `delete` par :

```jinja
{%- if soft_delete %}
pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    // La ligne déjà supprimée n'est pas retouchée : sans cette seconde condition, un
    // second DELETE rendrait 204 là où il doit rendre 404.
    let effet = Entity::update_many()
        .col_expr(Column::DeletedAt, Expr::value(chrono::Utc::now()))
        .filter(Column::Id.eq(id))
        .filter(Column::DeletedAt.is_null())
        .exec(db)
        .await?;

    Ok(effet.rows_affected > 0)
}
{%- else %}
pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    let effet = Entity::delete_by_id(id).exec(db).await?;

    Ok(effet.rows_affected > 0)
}
{%- endif %}
```

- [ ] **Step 4: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-cli --lib -- repository::tests`
Expected: PASS.

- [ ] **Step 5: Vérifier la conformité rustfmt**

Run: `cargo test -p rbs-cli --lib -- repository::tests::the_guarded_render_is_already_what_rustfmt_would_write --exact` (ou le nom exact du test de divergence du module).
Expected: PASS. Un dépassement de 100 colonnes sur la ligne de `filter::apply` sous soft-delete est le risque principal : si le test le signale, éclater l'appel dans la template.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/feature/repository.rs.jinja crates/rbs-cli/src/generate/repository.rs
git commit -F - <<'EOF'
feat(generate): rend la suppression logique dans la seule couche qui parle à la base

Les deux lectures écartent les lignes datées, et le delete les date au lieu de
les retirer. La condition posée sur le delete lui-même n'est pas redondante :
sans elle, une seconde suppression toucherait la ligne déjà supprimée et
rendrait 204 là où la suppression physique rendait 404.

La signature du delete ne change pas, le service et son erreur d'absence non
plus. Les imports sont conditionnés : posés toujours, ils feraient échouer
clippy sur tout projet engendré sans le drapeau.

Vérifications :
- cargo test -p rbs-cli --lib -- repository::tests : 0 échec
- le rendu reste conforme à ce que rustfmt écrirait
EOF
```

---

### Task 5: Le second DELETE, prouvé par HTTP

**Files:**
- Modify: `crates/rbs-cli/templates/feature/tests.rs.jinja:203-208`
- Test: `crates/rbs-cli/src/generate/tests_http.rs` (`mod tests`)

**Interfaces:**
- Consomme : rien de nouveau.
- Produit : une assertion supplémentaire dans les scénarios HTTP engendrés, **inconditionnelle**.

**Pourquoi inconditionnelle, contre ce que la spec annonçait :** la spec prévoyait de passer `soft_delete` au contexte reconstruit de `tests_http.rs:36-56`. C'est inutile — « un second DELETE répond 404 » est vrai des deux côtés du drapeau (en suppression physique, `rows_affected` vaut 0). L'assertion renforce donc la suite existante au lieu de se dédoubler, et un contexte de moins est à tenir en cohérence.

- [ ] **Step 1: Écrire le test qui échoue**

Dans `crates/rbs-cli/src/generate/tests_http.rs`, `mod tests` :

```rust
    #[test]
    fn a_second_delete_is_exercised() {
        let feature = Feature::fresh("articles", fields::parse("title:string").expect("champs"));
        let rendered = render(&feature).expect("les tests doivent se rendre");

        assert_eq!(
            rendered.matches(r#"without_body("DELETE", &resource)"#).count(),
            5,
            "chaque scénario de suppression doit rejouer le DELETE : c'est la seule \
             assertion qui distingue une suppression logique bien gardée d'une qui \
             rendrait 204 deux fois :\n{rendered}"
        );
    }
```

Le compte attendu est **5** : quatre `DELETE` existants (`tests.rs.jinja:203, 236, 315, 363`) plus celui ajouté au scénario principal.

- [ ] **Step 2: Lancer le test pour le voir échouer**

Run: `cargo test -p rbs-cli --lib -- tests_http::tests::a_second_delete_is_exercised --exact 2>&1 | tail -10`
Expected: FAIL, `left: 4, right: 5`.

- [ ] **Step 3: Ajouter l'assertion à la template**

Dans `crates/rbs-cli/templates/feature/tests.rs.jinja`, remplacer les lignes 203-207 par :

```jinja
    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");

    let (status, _) = call(&api, without_body("GET", &resource)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "elle répond encore");

    // Une seconde suppression ne trouve plus rien à supprimer. L'assertion vaut des deux
    // côtés de `--soft-delete` : c'est elle qui attrape une suppression logique dont la
    // condition de garde manquerait, et qui rendrait alors 204 indéfiniment.
    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "elle se supprime deux fois");
```

- [ ] **Step 4: Lancer le test pour le voir passer**

Run: `cargo test -p rbs-cli --lib -- tests_http::tests`
Expected: PASS.

- [ ] **Step 5: Régénérer les quatre exemples**

La template des tests a changé : les quatre projets d'`examples/` portent chacun ce fichier, et `integration_examples` compare octet à octet.

Run: `cat examples/README.md | grep -n "generate crud" -B 6 | head -40`
Puis rejouer, **par diff entre deux générations et jamais par écrasement**, la commande de régénération de chaque exemple, en reportant le seul écart attendu — les quatre lignes ajoutées au scénario de suppression.

Run: `cargo test -p rbs-cli --test integration_examples -- --include-ignored 2>&1 | tail -20`
Expected: 17 passés, aucune dérive.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/templates/feature/tests.rs.jinja crates/rbs-cli/src/generate/tests_http.rs examples/
git commit -F - <<'EOF'
test(generate): rejoue la suppression dans les scénarios engendrés

Une seconde suppression doit répondre 404. L'assertion vaut des deux côtés du
drapeau de suppression logique — en suppression physique la ligne n'existe plus
— et c'est la seule qui attrape une suppression logique dont la condition de
garde manquerait : elle rendrait alors 204 indéfiniment, sans que rien d'autre
ne le signale.

Les quatre exemples suivent, la template étant celle qu'ils portent tous.

Vérifications :
- cargo test -p rbs-cli --lib -- tests_http::tests : 0 échec
- cargo test -p rbs-cli --test integration_examples -- --include-ignored : 17 passés, aucune dérive
EOF
```

---

### Task 6: Le banc — la migration s'applique vraiment

**Files:**
- Create: un test `#[ignore]` dans `crates/rbs-cli/tests/integration_crud.rs`
- Test: le même fichier

**Interfaces:**
- Consomme : tout ce qui précède ; `bench::Project::fresh()`, `bench::TestDatabase::start()`, `Project::migrate(url)`, `Project::compile()`, `Project::test_of(...)` (`crates/rbs-cli/src/generate/bench.rs:36, 137, 291, 374, 464`).
- Produit : la seule preuve que la migration engendrée est du SQL valide. Les tests unitaires ne lisent qu'une chaîne de caractères.

**Nom de feature et de migration :** utiliser `soft_articles`, jamais `articles` — `static CARGO: Mutex<()>` sérialise les invocations mais les projets partagent `target/rbs-integration`, et deux bancs de même nom de migration se marcheraient dessus.

- [ ] **Step 1: Écrire le test**

Dans `crates/rbs-cli/tests/integration_crud.rs`, sur le modèle de `a_generated_crud_migrates_and_passes_its_tests_against_postgresql` (ligne 23) :

```rust
/// La migration d'une suppression logique s'applique, et son unicité ne porte que sur les
/// lignes vivantes.
///
/// Les tests unitaires lisent une chaîne de caractères : seul ce banc dit si PostgreSQL
/// accepte l'index partiel que la template écrit.
#[tokio::test]
#[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
async fn a_soft_deleting_crud_migrates_and_hides_its_deleted_rows() {
    let base = bench::TestDatabase::start().await;
    let projet = bench::Project::fresh_on(&base.url());

    projet.rbs_ok(&[
        "generate",
        "crud",
        "soft_articles",
        "--fields",
        "title:string:unique",
        "--soft-delete",
        "--force",
    ]);

    projet.migrate(&base.url());
    projet.compile();
    projet.test_of("soft_articles");
}
```

Adapter les noms exacts des helpers à ce que `bench.rs` expose ; le test voisin de la ligne 23 en est le modèle littéral.

- [ ] **Step 2: Lancer le banc**

Run: `cargo test -p rbs-cli --test integration_crud -- --ignored --no-fail-fast a_soft_deleting_crud > /private/tmp/claude-501/-Users-yacoubakone-dev-rs/47075687-159f-44e4-8a83-b1b3a396f17f/scratchpad/banc-soft-delete.txt 2>&1; tail -30 /private/tmp/claude-501/-Users-yacoubakone-dev-rs/47075687-159f-44e4-8a83-b1b3a396f17f/scratchpad/banc-soft-delete.txt`
Expected: PASS. Docker doit tourner.

**Si la migration échoue**, lire le SQL réellement émis avant de toucher à la template : c'est là que se verra un `and_where` mal placé ou un `Expr` absent du prélude.

- [ ] **Step 3: Ajouter le banc SQLite**

Le même scénario sur SQLite, qui écrit un SQL différent de PostgreSQL pour le même index partiel, et ne demande aucun conteneur. Modèle : le test `each_engine_produces_a_project_whose_tests_pass`.

Run: `cargo test -p rbs-cli --test integration_crud -- --ignored --no-fail-fast soft 2>&1 | tail -20`
Expected: les deux bancs verts.

- [ ] **Step 4: Commit**

```bash
git add crates/rbs-cli/tests/integration_crud.rs
git commit -F - <<'EOF'
test(generate): éprouve la migration de suppression logique contre deux moteurs

Les tests unitaires lisent une chaîne de caractères ; seul un banc dit si le
moteur accepte l'index partiel que la template écrit. PostgreSQL et SQLite
l'écrivent différemment, d'où les deux.

La couverture reste inégale et il faut le dire : MySQL n'a pas de conteneur
dans ce dépôt, et sa branche n'est prouvée que par le rendu — le test lit que
le branchement est écrit, non que MySQL l'accepte.

Vérifications :
- cargo test -p rbs-cli --test integration_crud -- --ignored --no-fail-fast soft : 2 passés
EOF
```

---

### Task 7: CHANGELOG et documentation bilingues

**Files:**
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md`
- Modify: le guide du CRUD engendré sous `docs/docs/` et sa paire sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/`

**Interfaces:** rien de logiciel.

- [ ] **Step 1: Localiser le guide à prolonger**

Run: `ls docs/docs/guides/ && grep -rln "generate crud" docs/docs/ | head`
Retenir la page qui documente le CRUD engendré ; sa paire française porte le même chemin relatif sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/`.

- [ ] **Step 2: Écrire les deux notes de CHANGELOG**

Dans `CHANGELOG.md`, sous `## [Unreleased]` → `### Added` :

```markdown
- `rbs generate crud --soft-delete` makes `DELETE` logical: the row stays, its `deleted_at`
  column dated, and every read hides it. The HTTP contract is unchanged — 204 on delete,
  404 on a second one, 404 on reading a deleted row — so no client notices. A `unique`
  field moves its constraint to an index restricted to live rows, which is what lets
  someone re-register with an address they had before. **MySQL has no partial index**: the
  generated migration branches at run time and keeps a global uniqueness there, so on MySQL
  a deleted value stays reserved.
```

Dans `CHANGELOG.fr.md`, la note française correspondante, au même endroit.

- [ ] **Step 3: Écrire les deux sections de guide**

Une section dans chaque langue couvrant : ce que le drapeau change (rien, vu du client), la colonne, l'index partiel et sa limite MySQL, et ce que le drapeau **ne fait pas** — ni route de restauration, ni `?include_deleted`, ni purge. Restaurer se fait en SQL.

- [ ] **Step 4: Vérifier la parité**

Run: `cd docs && node scripts/parite.mjs`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add CHANGELOG.md CHANGELOG.fr.md docs/
git commit -F - <<'EOF'
docs(soft-delete): documente le drapeau et ce qu'il ne fait pas

Deux choses qu'un lecteur doit trouver écrites : que le contrat HTTP ne change
pas, et que MySQL n'a pas d'index partiel — une valeur supprimée y reste
réservée. La seconde est le genre de détail qu'on n'aime pas découvrir deux
fois.

La section dit aussi ce qui n'existe pas : ni restauration, ni corbeille, ni
purge. Restaurer se fait en SQL tant qu'aucun besoin réel n'a dit quelle forme
la route devrait prendre.

Vérifications :
- node docs/scripts/parite.mjs : exit 0
EOF
```

---

### Task 8: Vérification de bout en bout

**Files:** aucun. Une tâche de preuve.

- [ ] **Step 1: La suite rapide**

Run: `cargo test --workspace 2>&1 | tail -20`
Expected: 0 échec. Noter le compte.

- [ ] **Step 2: La suite lente**

Run: `cargo test --workspace -- --ignored --no-fail-fast > /private/tmp/claude-501/-Users-yacoubakone-dev-rs/47075687-159f-44e4-8a83-b1b3a396f17f/scratchpad/suite-lente-soft-delete.txt 2>&1; echo "code $?"; grep -E "^test result|FAILED" /private/tmp/claude-501/-Users-yacoubakone-dev-rs/47075687-159f-44e4-8a83-b1b3a396f17f/scratchpad/suite-lente-soft-delete.txt`
Expected: code 0, aucun `FAILED`. Docker requis.

- [ ] **Step 3: Non-dérive des exemples**

Run: `cargo test -p rbs-cli --test integration_examples -- --include-ignored 2>&1 | tail -10`
Expected: 17 passés, aucune dérive.

- [ ] **Step 4: Lint et format**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: aucune sortie.

- [ ] **Step 5: Cocher la tâche**

Dans `IMPROVE.md`, cocher la ligne 74 : `- [x]` et, en fin de ligne, ` — Fait le 2026-09-02 : ` suivi de ce qui a été fait, de l'écart MySQL, et des **chiffres réels** relevés aux steps 1 à 4. Une case ne se coche que sur une preuve exécutée.

---

## Self-Review

**Couverture de la spec :**

| Section de la spec | Tâche |
|---|---|
| Contrat HTTP inchangé | Task 5 (assertion du second DELETE), Task 6 (banc) |
| Colonne `deleted_at` nullable sans défaut | Task 3, Steps 3-4 |
| Refus de `deleted_at` sous le seul drapeau | Task 2, Steps 7-8 |
| Index `idx_<table>_deleted_at` | Task 3, Step 4(c) |
| Index partiel, branche MySQL | Task 3, Step 4(c) et test `mysql_keeps_a_global_uniqueness` |
| `.unique_key()` retiré sous le drapeau | Task 3, Step 4(a) et test `the_unique_constraint_moves_to_a_partial_index` |
| Les trois points du repository | Task 4 |
| Imports conditionnés | Task 4, test `an_ordinary_repository_imports_only_what_it_uses` |
| Le tuple devient une struct | Task 1 |
| `serialize_struct` incrémenté | Task 2, Step 5 |
| Les huit tests unitaires nommés | Tasks 2 à 4 — noms repris tels quels |
| Bancs PostgreSQL et SQLite, aveu MySQL | Task 6 |
| CHANGELOG ×2, guide ×2 | Task 7 |

**Deux écarts assumés avec la spec, tous deux vers moins de code :**

1. **`tests_http.rs` ne reçoit pas `soft_delete`** (Task 5). L'assertion utile vaut des deux côtés du drapeau : la conditionner la dédoublerait pour rien, et un contexte reconstruit de moins reste à tenir en cohérence.
2. **Le test `every_read_hides_the_deleted_rows` compte trois occurrences** de `is_null()`, là où la spec en décrivait deux lectures : le `delete` en porte une troisième, et c'est elle qui tient le 404 du second appel.

**Cohérence des types :** `Feature::soft_deleting()` (Task 2) → clé `soft_delete` (Task 2) → lue par les trois templates (Tasks 3-4). `Error::SoftDeleteColonneReservee { colonne }` défini et testé en Task 2, nulle part ailleurs. `delete(db, id) -> Result<bool>` inchangée, donc `service.rs.jinja` n'apparaît dans aucune tâche — c'est voulu, et le banc de la Task 6 le vérifie en compilant.
