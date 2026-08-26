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
crates/rbs-cli/src/generate/mod.rs            module de la commande `rbs generate`
crates/rbs-cli/src/generate/champs.rs         modèle des champs et analyse
crates/rbs-cli/src/generate/champs/erreur.rs  erreurs, messages et suggestions
```

Rien de tout cela n'entre dans `rbs-core` : la grammaire sert au CLI au moment du
scaffolding et n'a aucune existence dans le runtime d'un projet généré.

Le nommage suit celui du CLI, qui est en français — `rbs-cli` est un binaire, non une
API publique. Seul `rbs-core`, publié sur crates.io et consommé par du code généré,
est en anglais.

Aucune commande n'appelle encore `analyser` — le premier appelant est le générateur
d'entité, tâche suivante. Le module porte donc un `#![allow(dead_code)]` que cette
tâche-là supprime. L'alternative, brancher une commande `generate crud` qui se
contenterait d'afficher les champs reconnus, ferait exister une commande qui ne génère
rien : une dette plus coûteuse que l'attribut.

## 4. Modèle de données

```rust
pub(crate) fn analyser(entree: &str) -> Result<Vec<Champ>, ErreurChamps>

pub(crate) struct Champ {
    pub nom: String,
    pub type_: TypeChamp,
    pub unique: bool,
    pub optionnel: bool,
    pub index: bool,
}

pub(crate) enum TypeChamp { String, Int, Float, Bool, Uuid, Datetime, Text }
```

Les variantes de `TypeChamp` gardent les mots de la grammaire, qui sont invariants.

Les modificateurs sont trois booléens plutôt qu'un `Vec<Modificateur>`. L'ensemble est
fermé, et les consommateurs veulent écrire `champ.unique`, pas parcourir un vecteur.
Un modificateur paramétré, s'il arrive un jour, cassera cette forme — c'est assumé :
il cassera aussi la grammaire, donc ce design.

## 5. Projections de types

La connaissance des types vit ici, sur `TypeChamp`, et nulle part ailleurs. Les
générateurs de l'entité et de la migration consomment des chaînes déjà résolues ; leurs
templates ne contiennent aucun test sur le type. Ajouter un type en v0.2 est une
variante d'énumération et trois bras de `match`, dans un seul fichier.

| Méthode | Porteur | Rôle |
|---|---|---|
| `TypeChamp::nom()` | type nu | le mot de la grammaire, `"uuid"` |
| `TypeChamp::type_rust()` | type nu | type Rust de la colonne |
| `Champ::type_rust()` | champ complet | idem, enveloppé dans `Option<…>` si `optionnel` |
| `TypeChamp::methode_migration()` | type nu | méthode du `ColumnDef` SeaORM |
| `TypeChamp::attribut_column_type()` | type nu | `Option<&str>` — seulement si le type diffère du défaut SeaORM |

Table de correspondance :

| Type | `type_rust()` | `methode_migration()` | `attribut_column_type()` |
|---|---|---|---|
| `string` | `String` | `string()` | — |
| `text` | `String` | `text()` | `Some("Text")` |
| `int` | `i32` | `integer()` | — |
| `float` | `f64` | `double()` | — |
| `bool` | `bool` | `boolean()` | — |
| `uuid` | `Uuid` | `uuid()` | — |
| `datetime` | `DateTimeWithTimeZone` | `timestamp_with_time_zone()` | — |

`Champ` implémente `serde::Serialize` à la main : la forme sérialisée expose ses cinq
champs **et** les trois projections, si bien qu'une template écrit `{@ champ.type_rust @}`
comme elle écrit `{@ champ.nom @}`. Une méthode Rust n'est pas visible depuis minijinja ;
sans cette sérialisation, chaque générateur devrait reconstruire une structure de vue.
Les clés sérialisées sont `nom`, `type`, `unique`, `optionnel`, `index`, `type_rust`,
`methode_migration` et `attribut_column_type` — `type` plutôt que `type_`, le
soulignement n'existant que pour contourner le mot-clé Rust.

Il n'y a pas de projection vers du SQL : les migrations générées sont écrites avec le
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
pub(crate) struct ErreurChamps { erreurs: Vec<ErreurChamp> }

pub(crate) struct ErreurChamp {
    rang: usize,        // rang du champ dans la chaîne, à partir de 1
    libelle: String,    // le nom du champ, ou la portion brute si le nom est illisible
    nature: NatureErreur,
}

pub(crate) enum NatureErreur {
    FormeInvalide,
    PasEnSnakeCase { suggestion: String },
    MotCleRust { suggestions: Vec<String> },
    NomReserve,
    TypeInconnu { nom: String },
    ModificateurInconnu { nom: String },
    ModificateurEnDouble { nom: String },
    IndexRedondant,
}
```

Le `Display` de `ErreurChamps` rend une ligne de diagnostic par erreur, suivie d'une
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
- un cas par type, vérifiant `type_`, `type_rust()`, `methode_migration()` et
  `attribut_column_type()`
- un cas par modificateur, vérifiant le booléen correspondant
- `optional` enveloppe le type Rust dans `Option<…>`
- ordre des modificateurs libre : `email:string:unique:optional` ≡ `email:string:optional:unique`
- espaces tolérés autour de `,` et `:`
- chaîne vide → `Ok(vec![])`
- plusieurs champs conservent leur ordre de déclaration
- la forme sérialisée d'un `Champ` porte bien `type_rust`, `methode_migration` et
  `attribut_column_type` en plus des cinq champs

**Erreurs**
- une assertion sur le message rendu pour chacune des huit variantes de `NatureErreur`
- une chaîne portant trois fautes distinctes remonte trois erreurs, dans l'ordre des champs
- un champ portant deux fautes ne remonte que la première

## 9. Ce que ce design n'ouvre pas

Le parseur ne lit aucun fichier, ne touche pas au disque, n'a pas besoin d'un projet
rbs autour de lui. C'est une fonction pure sur une chaîne — ce qui rend ses tests
instantanés et permet à D2 à D8 de s'appuyer dessus sans montage.
