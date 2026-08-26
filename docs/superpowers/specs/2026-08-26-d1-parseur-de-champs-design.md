# Parseur de champs — design

*2026-08-26 · tâche D1 · statut : validé*

## 1. Objet

`rbs generate crud users --fields "name:string,email:string:unique"` décrit un schéma
en une chaîne de caractères. Ce document fige la grammaire de cette chaîne, le modèle
de données qu'elle produit, et les erreurs qu'elle refuse.

La décision est structurante : D2 à D8 consomment tous la sortie de ce parseur. Une
grammaire qui bouge après D2 invalide cinq tâches.

## 2. Grammaire

```
fields     := field ("," field)*
field      := nom ":" type (":" modificateur)*
nom        := [a-z][a-z0-9_]*          sans « _ » final
type       := string | int | float | bool | uuid | datetime | text
modificateur := unique | optional | index
```

Les espaces autour de chaque séparateur sont ignorés : `"name:string, email:string"`
et `"name:string,email:string"` sont équivalents. Une chaîne vide produit une liste
vide, sans erreur — c'est ce que `rbs generate feature` (D10) réutilise pour un
squelette sans champ.

`id`, `created_at` et `updated_at` ne se déclarent jamais : ils sont posés par rbs sur
toute entité (§3.6 de la spec générale).

**Ce qui n'est pas dans la grammaire de la v0.1**, et n'y sera pas ajouté sans un
nouveau design : les relations (`author:ref:users`), les types paramétrés
(`string(255)`, `decimal(10,2)`), les valeurs par défaut. La grammaire est fermée à
sept types et trois modificateurs.

## 3. Emplacement

```
crates/rbs-cli/src/generate/mod.rs      module du lot D
crates/rbs-cli/src/generate/fields.rs   le parseur, son modèle et ses erreurs
```

Rien de tout cela n'entre dans `rbs-core` : la grammaire sert au CLI au moment du
scaffolding et n'a aucune existence dans le runtime d'un projet généré.

Aucune commande n'appelle encore `parse_fields` — le premier appelant est le générateur
d'entité, tâche suivante. Le module porte donc un `#![allow(dead_code)]` que cette
tâche-là supprime. L'alternative, brancher une commande `generate crud` qui se
contenterait d'afficher les champs reconnus, ferait exister une commande qui ne génère
rien : une dette plus coûteuse que l'attribut.

## 4. Modèle de données

```rust
pub(crate) fn parse_fields(input: &str) -> Result<Vec<Field>, FieldsError>

pub(crate) struct Field {
    pub name: String,
    pub ty: FieldType,
    pub unique: bool,
    pub optional: bool,
    pub index: bool,
}

pub(crate) enum FieldType { String, Int, Float, Bool, Uuid, Datetime, Text }
```

Les modificateurs sont trois booléens plutôt qu'un `Vec<Modifier>`. L'ensemble est
fermé, et les consommateurs veulent écrire `field.unique`, pas parcourir un vecteur.
Un modificateur paramétré, s'il arrive un jour, cassera cette forme — c'est assumé :
il cassera aussi la grammaire, donc ce design.

## 5. Projections de types

La connaissance des types vit ici, sur `FieldType`, et nulle part ailleurs. Les
générateurs D2 et D7 consomment des chaînes déjà résolues ; leurs templates ne
contiennent aucun `{% if type == ... %}`. Ajouter un type en v0.2 est une variante
d'énumération et trois bras de `match`, dans un seul fichier.

| Méthode | Porteur | Consommateur | Rôle |
|---|---|---|---|
| `FieldType::rust_type()` | type nu | D2, D3 | type Rust de la colonne |
| `Field::rust_type()` | champ complet | D2, D3 | idem, enveloppé dans `Option<…>` si `optional` |
| `FieldType::migration_method()` | type nu | D7 | méthode du `ColumnDef` SeaORM |
| `FieldType::column_type_attr()` | type nu | D2 | `Option<&str>` — seulement si le type diffère du défaut SeaORM |

Table de correspondance :

| Type | `rust_type()` | `migration_method()` | `column_type_attr()` |
|---|---|---|---|
| `string` | `String` | `string()` | — |
| `text` | `String` | `text()` | `Some("Text")` |
| `int` | `i32` | `integer()` | — |
| `float` | `f64` | `double()` | — |
| `bool` | `bool` | `boolean()` | — |
| `uuid` | `Uuid` | `uuid()` | — |
| `datetime` | `DateTimeWithTimeZone` | `timestamp_with_time_zone()` | — |

`Field` implémente `serde::Serialize` à la main : la forme sérialisée expose ses cinq
champs **et** les trois projections, si bien qu'une template écrit `{{ f.rust_type }}`
comme elle écrit `{{ f.name }}`. Une méthode Rust n'est pas visible depuis minijinja ;
sans cette sérialisation, chaque générateur devrait reconstruire une structure de vue.

Il n'y a pas de `sql_type()` : les migrations générées par D7 sont écrites avec le
constructeur SeaORM, pas en SQL brut. Une projection sans appelant est une projection
à ne pas écrire.

## 6. Validation

Chaque champ est validé dans cet ordre, et **s'arrête à sa première erreur**. Le
parseur poursuit néanmoins avec les champs suivants : l'utilisateur voit toutes ses
fautes en une exécution, pas une par tentative.

1. **Forme** — la partie contient un nom et un type séparés par `:`.
2. **Nom syntaxique** — `^[a-z][a-z0-9_]*$`, sans `_` final.
3. **Nom ∉ mots-clés Rust** — mots-clés stricts et réservés des éditions 2015 à 2021,
   en liste littérale. Un champ nommé `type` produirait une entité que rustc refuse.
4. **Nom ∉ noms imposés** — `id`, `created_at`, `updated_at`. La migration porterait
   deux fois la même colonne.
5. **Type connu** — parmi les sept.
6. **Modificateurs** — connus, sans doublon, et `unique` avec `index` refusé : un
   index unique *est* un index, la migration en poserait deux sur une seule colonne.

Ce qui passe sans commentaire, faute de canal d'avertissement à construire pour un
gain incertain : `text:unique` (l'index B-tree de PostgreSQL plafonne vers 2704 octets,
mais c'est un choix légitime sur des textes courts) et `bool:index` (inutile plus que
faux).

## 7. Erreurs

```rust
pub(crate) struct FieldsError { errors: Vec<FieldError> }

pub(crate) struct FieldError {
    index: usize,        // rang du champ dans la chaîne, à partir de 1
    raw: String,         // la portion telle que l'utilisateur l'a écrite
    kind: FieldErrorKind,
}

pub(crate) enum FieldErrorKind {
    MalformedSpec,
    NotSnakeCase { suggestion: String },
    RustKeyword { suggestion: String },
    ReservedName,
    UnknownType { name: String },
    UnknownModifier { name: String },
    DuplicateModifier { name: String },
    RedundantIndex,
}
```

Le `Display` de `FieldsError` rend une ligne de diagnostic par erreur, suivie d'une
ligne `→` de suggestion :

```
erreur : champ 1 « Title » — le nom doit être en snake_case
        → essayez « title »
erreur : champ 2 « type » — « type » est un mot-clé Rust
        → essayez « kind » ou « type_ »
```

Les suggestions sont mécaniques : conversion en snake_case pour un nom mal cassé
(`firstName` → `first_name`) ; pour un mot-clé, une table de quatre alias usuels
(`type`→`kind`, `ref`→`reference`, `match`→`matching`, `move`→`movement`) doublée
d'un repli par suffixe `_` ; pour un type ou un modificateur inconnu, l'énumération
des valeurs admises.

## 8. Tests

Le critère de la tâche — « chaque type et modificateur, plus les messages d'erreur de
syntaxe » — se décompose ainsi. Ces tests sont écrits avant le code.

**Analyse nominale**
- un cas par type, vérifiant `ty`, `rust_type()`, `migration_method()` et `column_type_attr()`
- un cas par modificateur, vérifiant le booléen correspondant
- `optional` enveloppe le type Rust dans `Option<…>`
- ordre des modificateurs libre : `email:string:unique:optional` ≡ `email:string:optional:unique`
- espaces tolérés autour de `,` et `:`
- chaîne vide → `Ok(vec![])`
- plusieurs champs conservent leur ordre de déclaration
- la forme sérialisée d'un `Field` porte bien `rust_type`, `migration_method` et
  `column_type_attr` en plus des cinq champs

**Erreurs**
- une assertion sur le message rendu pour chacune des huit variantes de `FieldErrorKind`
- une chaîne portant trois fautes distinctes remonte trois erreurs, dans l'ordre des champs
- un champ portant deux fautes ne remonte que la première

## 9. Ce que ce design n'ouvre pas

Le parseur ne lit aucun fichier, ne touche pas au disque, n'a pas besoin d'un projet
rbs autour de lui. C'est une fonction pure sur une chaîne — ce qui rend ses tests
instantanés et permet à D2 à D8 de s'appuyer dessus sans montage.
