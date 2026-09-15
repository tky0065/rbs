# Types `date`, `decimal` et `enum(…)` dans `--fields`

Date : 2026-09-15. Design validé en conversation.

## Problème

La grammaire `--fields` connaît sept types scalaires (`generate/fields.rs:12-23`) : pas de
date sans heure, pas de décimal exact, pas d'énumération. `status:enum(draft,published)` est
pourtant le champ le plus fréquent après une chaîne, et un prix en `float` perd des centimes.

## Livraison

Trois livraisons successives, chacune verte et committée : `date`, puis `enum`, puis
`decimal`. Chacune porte sa grammaire, son rendu, ses filtres, ses tests et sa doc.

## 1. `date`

- Grammaire : `due:date`.
- Modèle : `Date` de `sea_orm::prelude` (`chrono::NaiveDate` ; `with-chrono` est déjà
  activé dans le manifeste engendré). Migration : `.date()`.
- JSON : `"2026-09-15"`. OpenAPI : `string`, format `date`.
- Filtre : `Comparison<Date>` ; schéma de documentation `DateComparisonSchema` ajouté à
  `rbs-core` par la macro `comparaison_documentee!` de `filter/schema.rs`.
- Seed et tests engendrés : une date fixe, une autre à la modification ; comparée à la
  lettre (pas d'écart de format entre moteurs pour un `DATE`).

## 2. `enum(a,b,c)`

- Grammaire : `status:enum(draft,published)`. L'analyseur ne coupe plus `--fields` sur une
  virgule placée entre parenthèses. Les valeurs sont en snake_case, distinctes, au moins
  une ; chaque faute a son `ErrorKind` et son message, collectés comme les autres.
- Modèle : une énumération `DeriveActiveEnum` dans le `model.rs` de la feature, nommée du
  nom du champ en PascalCase (`Status`), variantes en PascalCase des valeurs, chaque
  `string_value` égale à la valeur écrite ; `rs_type = "String"`,
  `db_type = "String(StringLen::N(n))"` où `n` est la longueur de la plus longue valeur.
  Elle dérive ce qu'il faut pour les DTO (`Serialize`, `Deserialize`, `ToSchema`) avec la
  valeur écrite comme représentation JSON.
- Un nom de champ dont la forme PascalCase heurte un type que le `model.rs` engendré
  déclare déjà (`Model`, `ActiveModel`, `Entity`, `Column`, `PrimaryKey`, `Relation`) est
  refusé à l'analyse.
- Migration : colonne chaîne de longueur `n` et `CHECK (<colonne> IN (…))` — tenu par
  PostgreSQL, MySQL 8.0.16+ et SQLite ; une colonne optionnelle reste nullable (un `NULL`
  passe le `CHECK`).
- Filtre : nouvel opérateur `OneOf<T>` dans `rbs-core::filter` — `eq`, `in`, `is_null`,
  lu d'une valeur nue (qui vaut `eq`) ou d'un objet, comme `Comparison` — documenté par
  `OneOfSchema`. Le `filter.rs` engendré le traduit en `=`, `IN` et `IS [NOT] NULL`.
- Seed et tests engendrés : la première valeur à la création, la deuxième (ou la première
  s'il n'y en a qu'une) à la modification.
- Client TypeScript : l'énumération de chaînes de l'OpenAPI devient déjà une union de
  littéraux (`client/ts.rs`) ; rien à changer, un test le fixe pour ce cas.

## 3. `decimal`

- Grammaire : `price:decimal`. Pas de `decimal(p,s)` : une précision fixe suffit tant que
  personne n'en demande une autre.
- Modèle : `rust_decimal::Decimal`. Migration : `DECIMAL(19,4)` — MySQL tronquerait sinon
  à `DECIMAL(10,0)`.
- JSON : chaîne (`"12.5000"`), pour qu'un client JavaScript ne perde rien. OpenAPI :
  `string`, format `decimal`.
- Dépendances : sur un champ `decimal`, `generate` ajoute au manifeste du projet
  `rust_decimal` et la feature `with-rust_decimal` de `sea-orm` (et ce qu'exige la
  représentation JSON et OpenAPI), par les actions de plan existantes
  (`AjouterDependance`, `AjouterFeatureADependance`) — jamais par réécriture.
- SQLite : refus avant toute écriture. sqlx-sqlite ne lie pas `rust_decimal` — sa
  documentation l'écarte volontairement, `NUMERIC` n'y gardant que 15 chiffres
  significatifs — et sea-query ne lie un `Decimal` que pour PostgreSQL et MySQL. Le
  message nomme le champ, dit pourquoi, et propose `float` ou un entier en centimes.
- Filtre : `Comparison<Decimal>` ; schéma `DecimalComparisonSchema`, documenté comme une
  chaîne, sans dépendance de `rbs-core` à `rust_decimal`.
- Seed et tests engendrés : une valeur décimale exacte, une autre à la modification.

## 4. Noyau

`rbs-core` gagne `DateComparisonSchema`, `DecimalComparisonSchema`, `OneOf<T>` et
`OneOfSchema` — ajouts rétrocompatibles, dans la 1.5.0 non publiée. Rien d'autre : la
traduction en requête reste dans le `filter.rs` engendré.

## 5. Documentation

La table des types de `docs/docs/cli/generate.md`, le guide des filtres
(`guides/filtering.md`, opérateurs par type), le CHANGELOG et la note 1.5.0 ; anglais et
français dans le même commit.

## 6. Vérification

- Tests unitaires : analyseur (formes valides, chaque faute nommée), rendu du modèle, de
  la migration, des DTO, du filtre, du seed et des tests engendrés, pour chaque type.
- `rbs-core` : lecture de `OneOf` (valeur nue, objet, `in` vide), schémas OpenAPI.
- `integration_crud` : un CRUD portant `date`, `enum(…)` et `decimal`, compilé et testé
  contre PostgreSQL et MySQL ; le même sans `decimal` sous SQLite ; le refus de `decimal`
  sous SQLite éprouvé.
- `integration_examples`, `integration_docs`, `--lib`, fmt, clippy, `npm run build`,
  parité.
