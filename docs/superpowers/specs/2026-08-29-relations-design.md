# Les relations entre entités

Date : 2026-08-29
Statut : validé, prêt pour le plan d'implémentation
Portée : jalon v1.2. Huit lots, `R0` à `R8`, ordonnés par dépendance réelle.

## 1. Le problème

rbs ne sait pas déclarer qu'une entité en référence une autre. Le constat tient en
quatre points, tous vérifiés dans le dépôt du 2026-08-29 :

- `generate/fields.rs` connaît **sept types** — `string`, `int`, `float`, `bool`,
  `uuid`, `datetime`, `text` — et trois modificateurs, `unique`, `optional`, `index`.
  Aucun ne désigne une autre entité.
- `templates/feature/model.rs.jinja` émet `pub enum Relation {}`, **vide, toujours**.
  Les cinq modèles d'`examples/` le confirment sans exception.
- `templates/feature/migration.rs.jinja` n'émet aucun `.foreign_key(…)`.
- La seule clé étrangère du dépôt est **écrite à la main**, dans le fragment `auth` :
  `refresh_tokens.user_id → users.id`. Le générateur ne sait pas la produire.

Ce n'est pas un refus assumé. `ROADMAP.md` §Hors périmètre exclut nommément GraphQL, la
multi-tenancy, les WebSockets, gRPC, l'administration générée et les paiements. Les
relations n'y figurent pas, et ne figurent dans aucun jalon non plus : c'est un trou.

Un générateur de CRUD qui ne sait pas produire une clé étrangère laisse son utilisateur
écrire à la main la partie exacte que SeaORM rend la plus verbeuse — la variante de
`Relation`, l'`impl Related`, la contrainte de migration et son index — c'est-à-dire
qu'il l'abandonne là où il devait servir.

## 2. Ce que la conception fige

Quatre décisions, prises avant le reste et dont tout découle.

**Une relation se déclare là où elle existe.** `belongs_to` pose une colonne : il vit
donc dans `--fields`, comme un huitième type. `has_many` et le plusieurs-à-plusieurs ne
posent aucune colonne sur la table qui les déclare : ils prennent leurs propres flags.

**Une cible introuvable est un refus, pas un avertissement.** Le CLI inventorie les
entités du projet et refuse avant d'écrire quoi que ce soit, en nommant celles qu'il
connaît. C'est la ligne de `rbs new` devant une URL de base étrangère.

**Rien n'est joint sans qu'on l'ait demandé.** `GET /posts` rend la clé ; l'objet lié
n'arrive que sur `?include=`. Le coût d'une requête ne se paye pas par défaut.

**Le défaut ne détruit pas.** Une clé étrangère est `ON DELETE RESTRICT` sauf mention
contraire. Supprimer une ligne référencée rend `409`, jamais un effacement en chaîne
que personne n'a écrit.

## 3. La grammaire

### 3.1 Le huitième type

```
rbs g crud posts \
  --fields "title:string, body:text, author:references:users" \
  --has-many comments \
  --many-to-many tags
```

Le nom déclaré est celui de la **relation** — `author` — et la colonne en est dérivée,
`author_id`. Cette dérivation est ce qui permet à la variante SeaORM, à la clé étrangère
et au champ du DTO de porter des noms cohérents sans que l'utilisateur les répète.

Le troisième segment est la **cible** : le nom de la table telle qu'elle existe dans le
projet. Les segments suivants sont des modificateurs, mots nus comme aujourd'hui :

| Modificateur | Effet |
|---|---|
| `optional` | colonne `NULL`, `Option<Uuid>` en Rust |
| `cascade` | `ON DELETE CASCADE` |
| `nullify` | `ON DELETE SET NULL` — exige `optional` |
| `unique` | contrainte d'unicité, donc **un-à-un** |

Le un-à-un ne coûte donc aucune grammaire supplémentaire : il est un `belongs_to` dont
la colonne est unique, ce qui est exactement ce qu'il est en base.

**L'index est implicite.** Sans index sur la colonne portante, chaque suppression dans
la table cible parcourt la table portante en entier pour vérifier la contrainte. Le
`unique` le remplace quand il est présent.

### 3.2 Les six refus

Collectés en une passe, comme les fautes de champ le sont déjà — l'utilisateur corrige
sa ligne d'un coup.

| Écrit | Refus |
|---|---|
| `author:references` | cible manquante |
| `author_id:references:users` | la colonne est dérivée ; suggère `author` |
| `author:references:users:nullify` | `SET NULL` sur une colonne `NOT NULL` ; exige `optional` |
| `author:references:users:cascade:nullify` | deux politiques contradictoires |
| `author:references:users:index` | redondant : une clé étrangère est déjà indexée |
| `author:references:unknown` | cible introuvable, message nommant les entités connues |

Le cinquième réemploie `IndexRedundant`, qui existe déjà pour `unique` + `index`.

### 3.3 Le côté inverse n'est pas un flag

Déclarer `author:references:users` sur `posts` **implique** que `users` a des `posts`.
Le CLI écrit donc la variante inverse dans le modèle de la cible dans la foulée, sans
qu'on la demande. Une relation déclarée deux fois, une fois par bout, serait une
occasion de les faire diverger.

`--has-many` subsiste pour le seul cas qu'il sert réellement : une entité enfant qui
existe **déjà** avec sa clé, dont le parent n'a jamais reçu sa variante — un projet
engendré avant ce jalon, ou un modèle écrit à la main. C'est un chemin de réparation.

Il se valide comme le reste : le scan doit trouver dans l'entité nommée une colonne qui
référence la nôtre. `--has-many comments` sur un `comments` sans `post_id` est refusé en
le disant — sans quoi le CLI écrirait une variante que SeaORM rejetterait quarante
secondes plus tard.

## 4. L'inventaire des entités

Le CLI ne sait rien des entités d'un projet : `[package.metadata.rbs]` retient la
version, les features et le moteur, et rien ne parcourt `src/`. Le refus du §2 en
dépend, et il lui faut deux faits par entité : la **table**, pour la contrainte, et le
**chemin de module**, pour écrire `crate::…::Entity`.

Nouveau module `generate/entities.rs`. Il parcourt `<root>/src/*/model.rs` et retient,
pour chaque `#[sea_orm(table_name = "…")]` rencontré, la table, le chemin de module et
le fichier porteur — ce dernier parce que c'est là que s'écrira la variante inverse.

**Les modules imbriqués sont suivis, et ce n'est pas un cas d'école.** Dans un projet
avec `auth`, la table `users` n'est pas dans `src/users/model.rs` : elle est dans
`src/auth/model.rs`, sous `pub mod user { … }`, aux côtés de `refresh_token`. Or `users`
est la cible la plus probable de toutes. Un scan qui ne lirait que les répertoires la
déclarerait introuvable.

L'entité en cours de génération rejoint l'inventaire **avant** validation :
`parent:references:posts` sur `posts` — un arbre — doit passer.

Le scan est textuel, non un parseur Rust. Un modèle lourdement réécrit le fera échouer
en **refusant**, jamais en écrivant faux. C'est le sens du choix : la source de vérité
est le code qui existe, non un inventaire tenu dans le manifeste qui serait aveugle à
tout ce qui précède ce jalon — et qu'il aurait fallu amorcer par ce même scan.

## 5. Le niveau schéma

### 5.1 Le modèle et ses deux ancres

```rust
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "crate::auth::model::user::Entity",
              from = "Column::AuthorId", to = "crate::auth::model::user::Column::Id",
              on_delete = "Restrict")]
    Author,
    // <rbs:relations>
    // </rbs:relations>
}

impl Related<crate::auth::model::user::Entity> for Entity {
    fn to() -> RelationDef { Relation::Author.def() }
}
// <rbs:related>
// </rbs:related>
```

Deux ancres et non une : les variantes vivent dans les accolades de l'énumération, les
`impl Related` ne le peuvent pas. `doctor` les ajoute à son contrôle, qui en surveille
huit aujourd'hui.

Écrire dans un fichier existant impose l'idempotence qu'exige le §4.4 de la spec
d'origine : une variante déjà présente sous ce nom n'est pas écrite une seconde fois, et
si elle est présente en désignant une **autre** cible, la commande refuse plutôt que de
laisser deux relations homonymes dans une même énumération.

Sur un projet **antérieur à ce jalon**, les ancres sont absentes du modèle cible. Le
CLI applique la règle du dépôt sans la contourner : il n'écrit rien dans ce fichier,
achève tout le reste, et affiche le bloc à coller. `rbs upgrade` ne posera pas l'ancre —
il n'écrit que dans `Cargo.toml`, délibérément.

### 5.2 La migration

La colonne, la contrainte nommée `fk_posts_author_id`, son index. La traduction de
l'erreur de contrainte en `409` se fait dans le service **généré** : `Error::Conflict`
existe déjà dans `rbs-core`, dont l'API est figée depuis la v1.0 et n'a pas à bouger.

## 6. Le niveau HTTP

### 6.1 Lecture : le chargement par lots

`?include=` accepte plusieurs relations. La page est lue comme aujourd'hui, puis **une
requête par relation demandée** ramène les cibles par leurs identifiants, et le
recollement se fait en Rust.

```rust
pub async fn load_authors(db: &DatabaseConnection, posts: &[Model])
    -> Result<HashMap<Uuid, user::Model>>
```

Le nom est dérivé de la relation, non de la cible : `author` donne `load_author`,
`tags` donne `load_tags`. Deux relations vers la même table — `author` et `reviewer`
vers `users` — gardent ainsi deux fonctions distinctes.

C'est la décision structurante de cette section. `find_also_related` ne porte qu'**une**
relation par requête : générer les combinaisons — `list_with_author`,
`list_with_tags`, `list_with_author_and_tags`… — explose en 2ⁿ avec le nombre de
relations. Le chargement par lots donne **une fonction par relation**, sans N+1, et
traite le un-à-plusieurs, le plusieurs-à-un et le plusieurs-à-plusieurs de la même
manière — là où la jointure ne sait pas faire le dernier.

### 6.2 La projection, et ce qu'elle n'expose pas

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct PostResponse {
    pub id: Uuid,
    pub title: String,
    pub author_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<PostAuthor>,
    // …
}

/// Projection de l'entité liée. Elle vit ici, et se taille à la main.
#[derive(Debug, Serialize, ToSchema)]
pub struct PostAuthor { pub id: Uuid, pub email: String }
```

La projection est bâtie depuis les champs que le scan a lus sur le modèle cible,
**moins une liste d'exclusion** : tout nom contenant `password`, `hash`, `token` ou
`secret` est écarté. Sans elle, `GET /posts?include=author` sur un projet avec `auth`
exposerait `password_hash` au premier essai.

C'est une heuristique, et elle est traitée comme telle : elle est doublée par deux
garde-fous — `include` n'est jamais implicite, et la struct est posée dans le fichier de
l'utilisateur, à trois lignes de son regard. La documentation le dit explicitement
plutôt que de laisser croire à une garantie.

Le champ suit la cardinalité : `Option<PostAuthor>` pour un `belongs_to`, dont l'objet
lié est unique, et `Option<Vec<…>>` pour un `has_many` ou un plusieurs-à-plusieurs. Le
`Option` externe reste dans les trois cas : il distingue « non demandé », donc absent de
la réponse, de « demandé et vide », qui rend `[]`.

`?include=unknown` → `422`, nommant les relations connues.

### 6.3 Écriture

`author_id: Uuid` entre dans `CreatePost` et `UpdatePost` comme n'importe quelle
colonne. Une valeur qui ne référence rien fait échouer la contrainte, que le service
traduit en `409` avec le nom de la relation en cause — la même traduction qui sert au
`DELETE` restreint du parent.

## 7. Le plusieurs-à-plusieurs

`--many-to-many tags` sur `posts` engendre trois choses.

**Une migration de jonction**, séparée de celle de `posts` — elle doit pouvoir être
créée plus tard, quand `tags` existe. Table `posts_tags`, colonnes `post_id` et
`tag_id`, clé primaire composite, les deux clés étrangères en `ON DELETE CASCADE`. La
cascade est ici le bon défaut et ne contredit pas le §2 : supprimer un post efface ses
**liens**, jamais ses tags.

**Une entité de jonction**, `src/posts/tags_link.rs`, sans quoi SeaORM ne sait pas
traverser :

```rust
impl Related<tag::Entity> for Entity {
    fn to()  -> RelationDef { tags_link::Relation::Tag.def() }
    fn via() -> Option<RelationDef> { Some(tags_link::Relation::Post.def().rev()) }
}
```

Le côté inverse suit la règle du §3.3 : l'`impl Related<post::Entity>` est écrit dans
l'ancre `<rbs:related>` de `tags`, ce qui rend la traversée possible dans les deux sens
sans qu'on la déclare deux fois.

La feature passe de six à sept fichiers. La convention du dépôt tient : le septième
n'est pas de la logique de feature qui aurait dû être scindée, c'est une **entité
distincte**, et aussi courte qu'un modèle puisse l'être.

**Trois routes**, idempotentes :

```
GET    /posts/{id}/tags            liste
PUT    /posts/{id}/tags/{tag_id}   attache → 204, même deux fois
DELETE /posts/{id}/tags/{tag_id}   détache → 204, même si le lien n'existait pas
```

Un `id` ou un `tag_id` inconnu rend `404`. Le corps est vide des deux côtés : il n'y a
rien à représenter qu'un lien.

## 8. Langue des identifiants

Tout identifiant introduit par ce travail est en anglais, comme l'impose
`docs/superpowers/plans/2026-08-28-glossaire-migration-anglais.md`, dont le périmètre
couvre l'interne de `rbs-cli` : `RelationKind::{BelongsTo, HasMany, ManyToMany}`,
`OnDelete::{Restrict, Cascade, SetNull}`, `entities::scan`, `load_authors`, `attach`,
`detach`, `PostAuthor`. Commentaires et messages destinés à l'utilisateur restent
français, le glossaire les mettant hors périmètre.

La migration du 2026-08-28 a laissé des restes dans les deux fichiers que ce travail
étend : dix variantes de `ErrorKind` (`FormeInvalide`, `NomEnDouble`, `TypeInconnu`,
`ModificateurInconnu`, `IndexRedondant`…), le champ `Field.optionnel`, trois constantes
et une centaine de variables locales. Ajouter six refus à côté d'elles produirait le
dépôt bâtard que le glossaire existe pour empêcher. D'où le lot `R0`, en tête.

## 9. Les lots

| | Lot | Preuve exigée |
|---|---|---|
| R0 | Identifiants résiduels de `generate/fields.rs` et `fields/error.rs` traduits selon le glossaire | `cargo test -p rbs-cli` inchangé ; aucun identifiant français ne subsiste dans les deux fichiers |
| R1 | Type `references`, les six refus, `entities::scan` | tests unitaires sur chaque refus ; le scan retrouve `users` dans `src/auth/model.rs`, module imbriqué |
| R2 | Niveau schéma : modèle, migration, deux ancres, `doctor` | projet engendré qui compile ; `migrate up` pose la contrainte ; ancre retirée → rien n'est écrit et le bloc s'affiche |
| R3 | Côté inverse automatique, flag `--has-many` de réparation | la variante paraît dans le modèle cible ; sur un projet sans ancre, rien n'est écrit dans ce fichier |
| R4 | Lecture `?include=` : chargement par lots, projection, exclusion | `?include=author` imbrique ; `?include=unknown` → 422 ; **`password_hash` absent de la réponse** |
| R5 | Écriture du un-à-plusieurs, traduction en 409 | `author_id` inconnu → 409 ; `DELETE` du parent référencé → 409 |
| R6 | Plusieurs-à-plusieurs : migration, entité, trois routes | attacher deux fois → 204 deux fois ; détacher un lien absent → 204 |
| R7 | `examples/blog-auth` étendu, compilé en CI | `cargo build` de l'exemple en CI |
| R8 | Documentation bilingue et `CHANGELOG` | `npm run parite` → 0 écart ; blocs de terminal rejoués |

La preuve qui compte reste celle du dépôt : le test `assert_cmd` qui engendre un projet,
**le compile** et le fait tourner contre un PostgreSQL de `testcontainers`. Il gagne un
scénario portant les trois formes de relation d'un bout à l'autre.

## 10. L'exemple et la documentation

**L'exemple.** `examples/blog-auth` est étendu plutôt qu'un nouveau créé : c'est le seul
dont le domaine soit réellement relationnel, il est déjà compilé en CI, et surtout il
porte le cas difficile — `posts.author → users` doit se résoudre vers
`crate::auth::model::user::Entity`, dans un module imbriqué. Il reçoit les trois formes :
la clé vers `users`, un `comments` en un-à-plusieurs, un `tags` en
plusieurs-à-plusieurs.

Une dépendance à lever avant ce lot : `rbs-core 1.1.0` n'est pas publiée sur crates.io,
et un projet engendré ne compile aujourd'hui qu'avec `--core-path`.

**La documentation**, anglais et français dans le même commit :

- la page de `rbs generate crud` — le huitième type, les deux flags, les six refus ;
- une page de concept **Relations** — les trois formes, `?include=`, la liste
  d'exclusion et pourquoi elle n'est qu'une heuristique, les routes d'attachement ;
- la page d'architecture — la règle de dépendance unidirectionnelle face à une relation
  croisée ;
- la page de compatibilité et le `CHANGELOG`.

Deux contraintes du site s'appliquent : les blocs de terminal sont **capturés**, jamais
écrits à la main, et l'instrument de parité ne voit ni les tableaux ni les dernières
lignes — les tableaux de refus devront être relus à l'œil, des deux côtés.

## 11. Hors périmètre

- **Les relations polymorphes.** Une colonne qui référence l'une ou l'autre table selon
  un discriminant n'a pas de clé étrangère, donc pas d'intégrité en base. C'est un
  patron d'ORM dynamique, non de SeaORM.
- **La suppression en cascade applicative.** La cascade se déclare en base, où elle est
  atomique, jamais dans le service.
- **Le tri et le filtre sur une relation** — `?sort=author.name`. C'est une grammaire de
  requête, un sujet à part entière.
- **La lecture d'une base existante** pour en déduire les relations. Le §3.5 de la spec
  d'origine renvoie déjà ce cas à un passe-plat vers `sea-orm-cli`.
