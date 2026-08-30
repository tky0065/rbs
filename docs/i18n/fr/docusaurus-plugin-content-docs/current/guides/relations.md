---
sidebar_position: 5.5
title: Relations
---

# Relations

Un générateur qui produit une feature CRUD mais ne sait pas produire de clé étrangère laisse
son utilisateur écrire à la main la partie exacte que SeaORM rend la plus fastidieuse : la
variante `Relation`, l'`impl Related`, la contrainte de la migration, et l'index qu'elle exige.
Le huitième type de [`--fields`](../cli/generate.md#les-huit-types), `references`, referme ce
trou — entièrement depuis la ligne de commande, sans qu'aucune base ne tourne.

```text
rbs g crud posts --fields "title:string, author:references:users"
```

## Une référence est un champ, parce que c'est une colonne

`belongs_to` pose une colonne — `author_id` — elle vit donc dans `--fields` comme n'importe
quel autre champ, pas derrière un drapeau séparé. Le nom déclaré est celui de la relation,
`author` ; la colonne s'en dérive. Cette dérivation est ce qui permet à la variante SeaORM, à
la clé étrangère et à la migration de s'accorder sur un nom sans que la ligne de commande ne
le répète trois fois :

```rust title="src/posts/model.rs"
#[sea_orm(indexed)]
pub author_id: Uuid,
```

```rust title="src/posts/model.rs"
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::auth::model::user::Entity",
        from = "Column::AuthorId",
        to = "crate::auth::model::user::Column::Id",
        on_delete = "Restrict"
    )]
    Author,
    // <rbs:relations:posts>
    // </rbs:relations:posts>
}
```

```rust title="migration/.../m…_create_posts.rs"
.foreign_key(
    ForeignKey::create()
        .name("fk_posts_author_id")
        .from(Posts::Table, Posts::AuthorId)
        .to(Users::Table, Users::Id)
        .on_delete(ForeignKeyAction::Restrict),
)
```

Le troisième segment du champ est la cible : le nom d'une table telle qu'elle existe dans le
projet, pas un type que le CLI inventerait sur place. `rbs generate` inventorie chaque entité
sous `src/` avant de regarder ce qui a été tapé, y compris une entité nichée dans un module
plutôt que logée sous son propre répertoire — dans un projet avec `auth`, `users` vit dans
`src/auth/model.rs`, aux côtés de `refresh_tokens`, ce qui explique précisément pourquoi les
deux extraits ci-dessus nomment `crate::auth::model::user::Entity` pour un champ qui n'a jamais
écrit que `users`. Une cible absente de cet inventaire est refusée, nommément, aux côtés de
celles que le CLI connaît :

```text
$ rbs g crud comments --fields "body:text, author:references:writers" --dry-run
erreur : relation « author » — « writers » est introuvable dans ce projet
        → entités connues : comments, posts, refresh_tokens, users
```

Une cible que l'inventaire connaît, mais dont aucune migration ne crée encore la table, est
refusée sur le même principe : une clé étrangère qui la viserait échouerait dès l'application
des migrations, loin de la commande qui l'a posée :

```text
$ rbs g crud comments --fields "body:text, draft:references:drafts" --dry-run
erreur : relation « draft » — « drafts » n'a pas de migration dans ce projet
        → une clé étrangère la viserait avant qu'aucune migration ne crée sa table : écrivez sa migration avec `rbs migrate new`
```

## Deux formes, et un index qui n'est jamais optionnel

Une référence nue est plusieurs-à-un : n'importe quel nombre de posts peut nommer le même
auteur. `unique` ne coûte aucune grammaire supplémentaire pour la rendre un-à-un — c'est un
`belongs_to` dont la colonne se trouve unique, ce qui est exactement ce qu'est une relation
un-à-un dans une base relationnelle :

```text
owner:references:users:unique:cascade
```

```rust title="src/profiles/model.rs"
#[sea_orm(unique)]
pub owner_id: Uuid,
```

```rust title="migration/.../m…_create_profiles.rs"
.col(
    ColumnDef::new(Profiles::OwnerId)
        .uuid()
        .not_null()
        .unique_key(),
)
.foreign_key(
    ForeignKey::create()
        .name("fk_profiles_owner_id")
        .from(Profiles::Table, Profiles::OwnerId)
        .to(Users::Table, Users::Id)
        .on_delete(ForeignKeyAction::Cascade),
)
```

Toute autre référence reçoit un index qu'elle n'a pas demandé — `idx_posts_author_id`
ci-dessus — parce qu'il n'est pas optionnel : sans lui sur la colonne portante, supprimer une
ligne de `users` ferait parcourir à PostgreSQL chaque ligne de `posts` pour vérifier la
contrainte qu'elle s'apprête à violer. `unique` indexe déjà la colonne du seul fait d'être une
contrainte, ce qui explique que `profiles` ne reçoive aucun `create_index` séparé. Demander
`index` en plus de l'un ou de l'autre est refusé comme redondant — le même refus qu'un simple
`unique:index` reçoit sur une colonne ordinaire, et la raison pour laquelle une référence ne
prend jamais `index` explicitement :

```text
$ rbs g crud comments --fields "body:text, author:references:users:index" --dry-run
erreur : champ 2 « author » — « index » redondant : une clé étrangère est déjà indexée
        → retirez « index »
```

`optional` et les deux politiques de suppression se lisent comme sur n'importe quelle clé
étrangère : `cascade` pour `ON DELETE CASCADE`, ci-dessus, et `nullify` pour `ON DELETE SET
NULL`. `nullify` sur une colonne `NOT NULL` est refusé plutôt que d'exiger silencieusement
`optional` à votre place — la nullabilité d'une colonne n'est pas quelque chose que cette
grammaire déduit d'une politique choisie trois mots plus loin — et demander les deux politiques
à la fois est refusé comme la contradiction que c'est :

```text
$ rbs g crud comments --fields "body:text, author:references:users:nullify" --dry-run
erreur : champ 2 « author » — « nullify » sur une colonne non nullable
        → ajoutez « optional », ou choisissez « cascade »

$ rbs g crud comments --fields "body:text, author:references:users:optional:cascade:nullify" --dry-run
erreur : champ 2 « author » — « cascade » et « nullify » se contredisent
        → gardez l'un des deux
```

Sans mention, une référence est `ON DELETE RESTRICT`, comme `posts.author_id` ci-dessus :
supprimer une ligne référencée échoue plutôt que d'emporter ses dépendants avec elle,
silencieusement, dans un ordre que personne n'a choisi.

## Le côté qu'on n'a pas demandé

Déclarer `author:references:users` sur `posts` implique que `users` a des posts. Plutôt que
d'en faire un second drapeau à tenir synchronisé avec le premier, le CLI écrit lui-même le
`has_many` inverse, dans la même exécution, dans le modèle de la cible :

```rust title="src/auth/model.rs (pub mod user)"
pub enum Relation {
    // <rbs:relations:users>
    #[sea_orm(has_many = "crate::posts::model::Entity")]
    Posts,
    // </rbs:relations:users>
}

// <rbs:related:users>
impl Related<crate::posts::model::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Posts.def()
    }
}
// </rbs:related:users>
```

Deux ancres, pas une : une variante `Relation` vit dans les accolades de l'énumération, un
`impl Related` ne le peut pas. Leur fichier se trouve de la même manière que la cible
elle-même : depuis l'inventaire des entités, pas depuis un répertoire deviné à partir du nom
de la table — ce qui explique pourquoi le cas du module imbriqué compte de nouveau ici. Sur un
projet engendré avant que cette ancre n'existe, le CLI n'écrit rien dans ce fichier plutôt que
de deviner où l'insertion irait ; il achève tout le reste et affiche le bloc à coller à la
main, la même règle que [chaque ancre de ce générateur](../cli/generate.md#les-ancres) suit.

`--has-many` existe pour cette réparation précisément, et pour elle seule : un enfant qui porte
déjà sa clé, dont le parent n'a jamais reçu la variante en retour. Il refuse un parent
introuvable et un enfant qui ne porte pas réellement la clé attendue — ce second contrôle
existe pour que la variante écrite soit une que SeaORM accepterait, plutôt qu'une qui échouerait
à compiler quarante secondes plus tard :

```text
$ rbs g crud users --has-many categories --dry-run
erreur : categories ne porte aucune colonne référençant `users` : ajoutez-la avant de relancer `--has-many categories`
```

## Une cible visée deux fois perd son raccourci

Deux relations nommant la même cible — `author` et `reviewer`, toutes deux
`references:users` — sont une forme réelle, pas une erreur. Mais `impl Related<T>` ne prend que
le type cible pour clé : deux implémentations pour le même couple de types, c'est `rustc` qui
refuse un `impl` de trait dupliqué, pas un arbitrage que le CLI pourrait faire. Aucune des deux
n'est donc écrite, d'aucun côté, et un commentaire explique pourquoi aux deux endroits où le
code manquant aurait dû se trouver :

```rust title="src/reviews/model.rs"
// `users` est visée par 2 relations (`Author`, `Reviewer`) : `Related` serait ambigu, et son modèle ne reçoit donc pas non plus le `has_many` en retour, qui l'exige. Joindre explicitement, par exemple
// `Entity::find().join(JoinType::LeftJoin, Relation::Author.def())`.
```

```rust title="src/auth/model.rs (pub mod user)"
// `reviews` vise cette table par 2 relations (`Author`, `Reviewer`) : pas de `has_many`
// ici, `EntityTrait::has_many` exigeant le `Related` que `reviews` ne peut pas poser
// sans arbitrer entre elles. Joindre explicitement depuis le côté portant.
```

Les colonnes, les clés étrangères et les simples variantes `Relation` restent, elles,
inchangées — seul le raccourci qu'un unique trait ambigu ne peut pas fournir manque.

## Une référence requise laisse son entité sans seed

`rbs generate crud` écrit par défaut un fichier de seed à côté de la feature, une ligne qu'il
peut insérer sans rien demander à la base. Une référence requise casse cela : le seed aurait
besoin d'un vrai `author_id` tiré de `users`, et n'a aucune ligne vers laquelle pointer qu'il
puisse défendre comme correcte. Le silence n'aurait rien expliqué, la commande dit donc
exactement pourquoi elle l'a écarté :

```text
aucun seed pour posts : la référence « author » est requise, et un seed ne peut pas deviner vers quelle ligne pointer
```

Rendre la référence `optional` contourne le problème directement — un `posts` sans seed se sème
proprement, `author_id` absent — et reste le seul remède disponible depuis `--fields` seul :
tout le reste de cette page s'applique à une référence optionnelle exactement comme à une
référence requise.
