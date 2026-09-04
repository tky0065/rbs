# `rbs generate crud --soft-delete`

**Tâche 74 d'`IMPROVE.md`.** `templates/feature/repository.rs.jinja:92` appelle
`Entity::delete_by_id(id).exec(db)` : un `DELETE /articles/{id}` reçu par erreur retire la
ligne définitivement, et rien dans le projet engendré ne permet de la retrouver. Le
drapeau engendre la variante logique — la ligne reste, marquée d'une date.

## Ce qui est décidé

**Le `DELETE` devient logique, et rien d'autre ne change.** Pas de route `restore`, pas de
paramètre `?include_deleted`, pas de purge. La promesse tenue est « une suppression
accidentelle n'est plus définitive » ; restaurer une ligne se fait en SQL, le temps qu'un
besoin réel dise quelle forme la route devrait prendre. Une route de restauration pose
d'ailleurs une question que ce drapeau ne sait pas trancher seul : qui a le droit de
restaurer.

Le contrat HTTP est **inchangé**. `DELETE` rend 204, un second `DELETE` rend 404, un `GET`
sur une ligne supprimée rend 404, et la ligne disparaît de `list` comme de `filter`. Vu du
client, `--soft-delete` ne se voit pas : c'est le but.

## La colonne

`deleted_at`, `timestamp with time zone`, **nullable, sans défaut**. Nulle tant que la
ligne vit ; portant l'instant de la suppression ensuite.

Elle est **injectée par le générateur**, pas déclarée dans `--fields`. Deux conséquences :

- `deleted_at` rejoint `id`, `created_at` et `updated_at` parmi les noms que rbs pose
  lui-même — mais **seulement quand `--soft-delete` est passé**. Sans le drapeau, rien
  n'injecte la colonne et `--fields "deleted_at:datetime"` reste une déclaration
  légitime, qu'il serait gratuit de casser.
- Le refus se fait donc dans `command::plan_for`, après `fields::parse`, et non dans
  `NAMES_SET_BY_RBS` (`fields.rs:611`) qui ne connaît pas les options. Patron : la
  validation `validate_role` et son erreur `RoleSansAuth` (`command.rs:145-152, 345`). Le
  message nomme le drapeau et la colonne.

Un index l'accompagne — `idx_<table>_deleted_at` — puisque désormais **toute** lecture la
filtre.

## L'unicité, et le moteur qui ne suit pas

Un champ `unique` sous soft-delete pose un problème que la suppression physique n'avait
pas : la ligne supprimée occupe toujours la valeur. Un utilisateur qui se réinscrit avec
l'adresse qu'il avait avant sa suppression reçoit un 409 que rien ne lui explique.

La réponse est l'**index partiel** : l'unicité ne porte que sur les lignes vivantes.

```sql
CREATE UNIQUE INDEX uq_users_email ON users (email) WHERE deleted_at IS NULL
```

**MySQL ne sait pas le faire, et sea-query ne le protège pas.** Vérifié dans
`sea-query-1.0.2` : `IndexCreateStatement` porte bien un `r#where`
(`src/index/create.rs:218`), mais le builder MySQL l'écrit comme les deux autres
(`src/backend/mysql/index.rs:147`) — le `CREATE UNIQUE INDEX … WHERE …` produit serait une
erreur de syntaxe, et un projet MySQL sous `--soft-delete` ne migrerait pas du tout. Le
dépôt supporte trois moteurs (`crates/rbs-cli/src/database.rs:23`) et un test les couvre
(`each_engine_produces_a_project_whose_tests_pass`).

**La migration engendrée branche donc à l'exécution**, sur `manager.get_database_backend()` :

```rust
// PostgreSQL et SQLite savent restreindre un index à un sous-ensemble de lignes :
// deux lignes portent alors la même valeur si l'une est supprimée. MySQL ne le sait
// pas — l'unicité y reste globale, et une valeur supprimée y reste réservée.
let mut index = Index::create()
    .if_not_exists()
    .unique()
    .name("uq_users_email")
    .table(Users::Table)
    .col(Users::Email)
    .to_owned();

if manager.get_database_backend() != DbBackend::MySql {
    index = index.and_where(Expr::col(Users::DeletedAt).is_null()).to_owned();
}

manager.create_index(index).await?;
```

Une seule migration pour les trois moteurs, et la limite écrite là où on la lit. Le
`.unique_key()` posé sur la `ColumnDef` (`migration.rs.jinja`, boucle des colonnes)
**disparaît** sous `--soft-delete` : le laisser rendrait la contrainte de colonne
inconditionnelle, et l'index partiel n'y changerait rien.

## Le repository, seule couche touchée

Trois points, tous dans `repository.rs.jinja` — le service, le contrôleur, les DTO et les
seeds ne bougent pas. C'est la règle du projet qui le veut : la couche qui parle à la base
est la seule à construire une requête.

| Fonction | Sous `--soft-delete` |
|---|---|
| `filter` (dont `list` dépend) | `filter::apply(Entity::find().filter(Column::DeletedAt.is_null()), filtre)?` |
| `find` | `Entity::find_by_id(id).filter(Column::DeletedAt.is_null()).one(db)` |
| `delete` | un `update_many` posant `deleted_at` |

```rust
pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    // La ligne déjà supprimée n'est pas retouchée : sans cette seconde condition, un
    // second DELETE rendrait 204 là où la suppression physique rendait 404.
    let effet = Entity::update_many()
        .col_expr(Column::DeletedAt, Expr::value(chrono::Utc::now()))
        .filter(Column::Id.eq(id))
        .filter(Column::DeletedAt.is_null())
        .exec(db)
        .await?;

    Ok(effet.rows_affected > 0)
}
```

La signature ne change pas — `Result<bool>` — donc `service::delete` et son
`Error::NotFound` restent tels quels.

Le bloc `use` gagne `ColumnTrait` et `QueryFilter`, ainsi que `super::model::Column` et
`sea_orm::prelude::Expr`. Ces imports sont **conditionnés** par le drapeau : les poser
toujours vaudrait un `unused_imports` sur un CRUD ordinaire, et le projet compile ses
exemples sous `clippy -D warnings`.

## Ce que le générateur doit apprendre

L'ordre est celui que la cartographie du pipeline a établi.

1. `cli.rs:136-156` — `#[arg(long)] soft_delete: bool` sur `GenerateCommands::Crud`.
2. `lib.rs:93-106` — **le tuple de destructuration passe de sept à neuf éléments**. Il est
   déjà à la limite du lisible ; on le remplace par une struct nommée dans le même
   mouvement, plutôt que d'ajouter deux positions anonymes à une fonction qui en compte
   déjà sept (`lib.rs:455-463`). C'est du code que ce lot touche, pas un détour.
3. `command.rs:24-39` — `soft_delete: bool` sur `Options` ; refus si un champ se nomme
   `deleted_at` ; report sur `Feature` à l'image de `.guarded(role)`.
4. `feature.rs:16-23` et son `Serialize` manuel `:222-237` — le champ et sa clé, en
   **incrémentant le `serialize_struct("Feature", 10)`**.
5. Templates : `migration.rs.jinja` (colonne, index, index partiel, `DeletedAt` dans
   l'`enum DeriveIden`), `model.rs.jinja` (le champ sur `Model`), `repository.rs.jinja`
   (les trois points ci-dessus).
6. `tests_http.rs:36-56` — contexte reconstruit : il doit recevoir `soft_delete`
   explicitement, les templates tournant en `UndefinedBehavior::Strict`
   (`template.rs:26`). Les contextes de `filter.rs:44` et `seed.rs:30` n'y touchent pas,
   leurs templates n'y faisant aucune référence.

## Tests

**Unitaires**, dans les `mod tests` des générateurs touchés — le motif du dépôt, un helper
qui rend puis des `assert!(rendered.contains(…))` :

| Test | Ce qu'il prouve |
|---|---|
| `the_delete_marks_the_row_instead_of_removing_it` | `repository` : plus de `delete_by_id`, un `update_many` |
| `every_read_hides_the_deleted_rows` | `filter` et `find` portent `DeletedAt.is_null()` |
| `a_second_delete_still_answers_404` | la condition `DeletedAt.is_null()` du `delete` |
| `the_unique_constraint_moves_to_a_partial_index` | plus de `.unique_key()`, un `and_where` branché |
| `mysql_keeps_a_global_uniqueness` | la branche `!= DbBackend::MySql` est présente et commentée |
| `an_ordinary_crud_is_unchanged` | **témoin** : sans le drapeau, le rendu est identique à l'existant |
| `a_field_named_deleted_at_is_refused` | le refus nomme le drapeau |
| `the_reads_import_only_what_they_use` | pas de `ColumnTrait` sans le drapeau |

**Banc `#[ignore]`** (`bench::Project::fresh()` + `compile()`) : un CRUD `--soft-delete`
compile, migre contre PostgreSQL, et ses scénarios HTTP passent — le seul test qui prouve
que la migration est valide.

La couverture des trois moteurs est **égale, et par exécution** : PostgreSQL prouve la
branche partielle contre un conteneur ; SQLite la prouve une seconde fois sans conteneur,
ce qui vaut la peine puisque les deux moteurs n'écrivent pas le même SQL ; et MySQL, dont
le dépôt porte un conteneur (`common::start_mysql`), prouve l'autre branche —
`a_soft_deleting_crud_keeps_a_global_uniqueness_on_mysql` y applique la migration et exige
le refus du rebond, l'unicité y restant globale.

Ce banc a levé au passage une crainte qui pesait sur le drapeau, et qui ne le concernait
pas : le banc des trois moteurs engendre un CRUD sans champ `unique` ni `index`, si bien
qu'`Index::create().if_not_exists()` n'avait jamais rencontré MySQL, qui ne connaît pas
`IF NOT EXISTS` sur un `CREATE INDEX`. Le constructeur d'index MySQL de `sea-query 1.0.2`
ne lit simplement jamais ce drapeau (`src/backend/mysql/index.rs`,
`prepare_index_create_statement`) : le SQL reçu est un `CREATE UNIQUE INDEX` nu, et les
deux index sont créés. Aucun défaut antérieur ne se cachait là.

**Conformité rustfmt** : `bench::longueurs_divergentes` sur le repository et la migration,
les deux templates écrivant elles-mêmes ce que rustfmt écrirait
(`repository.rs.jinja:1-6`).

## Documentation

- `CHANGELOG.md` et `CHANGELOG.fr.md`, `[Unreleased] / Added`, en disant la limite MySQL —
  c'est le genre de détail qu'on ne découvre pas deux fois avec plaisir.
- Le guide du CRUD engendré et sa paire française, règle de parité oblige.
- Aucun exemple d'`examples/` ne passe au drapeau : aucun n'en a besoin, et les y faire
  passer ferait dériver quatre projets pour une démonstration.
