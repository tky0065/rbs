---
sidebar_position: 2
title: rbs generate
---

# `rbs generate`

Ajoute une feature à un projet existant : les six fichiers de la feature, plus — pour
`crud` — un fichier de tests, une entité SeaORM et sa migration, écrits depuis `--fields`
sans qu'aucune base ne tourne. C'est l'inverse de `sea-orm-cli generate entity`, qui exige
un schéma préalable.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

{/* rbs:transcript cmd="rbs generate --help" */}
```text
$ rbs generate --help
Génère une feature dans un projet existant

Utilisation : rbs generate <COMMANDE>

Commandes :
  crud     Génère une feature CRUD complète, entité et migration comprises
  feature  Génère une feature vide : six fichiers, aucun champ
  client   Engendre un client typé depuis le document OpenAPI du projet
  job      Génère un job de la file, et son échéance sous --every ; exige la feature jobs
  help     Affiche cette aide, ou celle des commandes données

Options :
  -h, --help     Affiche l'aide
  -V, --version  Affiche la version
```

`g` est un alias de `generate` : `rbs g crud users` et `rbs generate crud users` s'analysent
en la même chose.

`rbs generate` n'accepte ni `--template-dir` ni `--yes` : il ne pose aucune question, et
ses templates sont compilées dans le binaire plutôt que lues depuis un répertoire. En passer
un est une erreur de clap plutôt qu'un flag pris puis ignoré.

## `rbs generate crud`

{/* rbs:transcript cmd="rbs generate crud --help" */}
```text
$ rbs generate crud --help
Génère une feature CRUD complète, entité et migration comprises

Utilisation : rbs generate crud [OPTIONS] <NAME>

Arguments :
  <NAME>  Nom de la feature, au pluriel

Options :
      --fields <CHAMPS>    Champs de l'entité, ex. "name:string,email:string:unique"
      --singular <NOM>     Forme singulière du nom, quand l'heuristique se trompe (ex. news)
      --force              Écrit même si le working tree Git est sale
      --dry-run            Affiche le plan sans rien écrire
      --json               Rend le plan, ou l'erreur, en un document JSON sur la sortie standard
      --has-many <ENTITE>  Entité enfant dont ce modèle doit porter la variante inverse, répétable
      --role <ROLE>        Relève à ce rôle le seuil des écritures ; exige la feature auth
      --soft-delete        Rend le DELETE logique : la ligne reste, marquée d'une date de suppression
      --with-upload        Ajoute trois routes de contenu binaire ; exige la feature storage
      --cursor             Pagine GET /<ressource> par curseur ; la route de filtre garde ses pages
  -h, --help               Affiche l'aide
  -V, --version            Affiche la version
```

| Flag | Effet |
|---|---|
| `--fields <CHAMPS>` | Les colonnes de l'entité, dans la grammaire décrite plus bas. Omis, la feature est générée sans colonne propre. |
| `--singular <NOM>` | La forme singulière du nom, quand l'heuristique intégrée se trompe. Elle nomme l'entité, les DTO et les variables locales — `CreateNewsItem` et `let news_item` pour `rbs generate crud news --singular news_item` — tandis que le module, la table et les routes gardent le pluriel. L'heuristique laisse déjà `news`, `series` et `species` intacts ; pour tout autre pluriel invariable ou irrégulier, ce flag est le remède. Doit être en snake_case, et n'être ni un mot-clé Rust ni un module du squelette, vérifié avant toute écriture. |
| `--force` | Écrit même si le working tree Git est sale, et écrase les fichiers signalés en conflit. |
| `--dry-run` | Affiche le plan et s'arrête. Rien n'est écrit. |
| `--json` | Rend le plan — ou l'erreur — en un seul document JSON sur la sortie standard, à la place du texte coloré : chaque action avec son effet, le contenu complet des fichiers créés, et `applique` pour dire si quelque chose a été écrit. Indépendant de `--dry-run`, et accepté aussi par `generate feature`. [Le guide des agents](../guides/agents.md#lire-un-plan-en-json) donne le document et les codes d'erreur. |
| `--has-many <ENTITE>` | Répare le côté lointain d'une relation : écrit dans le modèle d'une feature déjà générée la variante `has_many` qui vise l'enfant nommé, et rien d'autre. Répétable. [Le guide des relations](../guides/relations.md) dit quand c'est nécessaire. |
| `--role <ROLE>` | Relève le seuil des écritures — `create`, `update`, `delete`, et le `PUT` de la route de contenu quand `--with-upload` l'accompagne — à ce rôle plutôt qu'au `Role::User` par défaut. Il n'ouvre ni ne ferme rien : sur un projet portant `auth`, *toutes* les routes engendrées prennent déjà une `Identity` et appellent `require_role`, et les lectures (`list`, `find`, `filter`, et les `GET` et `HEAD` de la route de contenu) gardent simplement le seuil par défaut. Exige la feature [`auth`](../guides/auth.md), et un rôle que son enum `Role` déclare — les deux sont vérifiés avant toute écriture. [Le guide de l'authentification](../guides/auth.md#fermées-par-défaut-à-la-génération) dit ce qu'il faut retirer pour rouvrir une route. |
| `--soft-delete` | Rend `DELETE` logique plutôt que de retirer la ligne. Le contrat HTTP ne change pas, et la contrainte d'un champ `unique` se restreint aux lignes vivantes — sur MySQL elle reste globale, si bien qu'une valeur supprimée y reste réservée. [Le guide des migrations](../guides/migrations.md#suppression-logique) a le reste. |
| `--with-upload` | Monte trois routes sur `/<ressource>/{id}/content` — `PUT`, `GET`, `HEAD` — contre le trait du fragment `storage`. Exige la feature [`storage`](../guides/storage.md), et le fragment sous `src/modules/storage/`, là où `rbs add` le pose depuis la 1.3.0 — les deux sont vérifiés avant toute écriture, et un projet qui porte encore `src/storage/` est refusé tant que le répertoire n'est pas déplacé et ses `use` corrigés. Avec `--role`, le `PUT` rejoint les écritures dont le drapeau relève le seuil ; avec `--soft-delete`, le contenu survit à la ligne que le `DELETE` se contente d'estampiller. Il écrit aussi leurs tests dans `tests/content.rs` — le cycle, les 404, le 413, et le 401 sous `auth`. [Le guide du stockage](../guides/storage.md#les-routes-de-contenu-engendrées) a les deux. |
| `--cursor` | Pagine `GET /<ressource>` par curseur plutôt que par numéro de page : la route prend `after` et `per_page`, et rend `data` avec `meta.next` — l'`id` à passer comme `after` suivant, `null` une fois la marche terminée — et aucun `total`. `POST /<ressource>/filter` garde ses pages, quel que soit son tri : un curseur sur l'`id` est faux dès que l'ordre suit une autre colonne. Se combine avec `--role`, avec `--soft-delete` — les lignes supprimées restent hors de la marche — et avec `--with-upload`. Pour une entité que ses tests peuvent créer — sans référence requise —, les tests engendrés parcourent chaque page jusqu'à l'extinction de `next`, et vérifient qu'aucune ligne ne revient deux fois. [Le guide du filtrage](../guides/filtering.md#pagination-par-curseur-pour-les-listes-qui-débordent-un-offset) a le reste. |

## `rbs generate feature`

{/* rbs:transcript cmd="rbs generate feature --help" */}
```text
$ rbs generate feature --help
Génère une feature vide : six fichiers, aucun champ

Utilisation : rbs generate feature [OPTIONS] <NAME>

Arguments :
  <NAME>  Nom de la feature

Options :
      --singular <NOM>  Forme singulière du nom, quand l'heuristique se trompe (ex. news)
      --force           Écrit même si le working tree Git est sale
      --dry-run         Affiche le plan sans rien écrire
      --json            Rend le plan, ou l'erreur, en un document JSON sur la sortie standard
  -h, --help            Affiche l'aide
  -V, --version         Affiche la version
```

Les mêmes flags moins `--fields`, `--has-many` et `--role` : une feature vide n'a pas de
colonne, donc ni entité digne de ce nom, ni migration, ni relation à réparer ; et elle ne
porte aucun handler qu'une garde protégerait. `--singular` reste : le squelette nomme
toujours son service et ses DTO d'après le singulier.

## `rbs generate job`

{/* rbs:transcript cmd="rbs generate job --help" */}
```text
$ rbs generate job --help
Génère un job de la file, et son échéance sous --every ; exige la feature jobs

Utilisation : rbs generate job [OPTIONS] <NAME>

Arguments :
  <NAME>  Nom du job, en snake_case : celui de son module et de son KIND

Options :
      --every <CRON>  Expression cron de l'échéance, à cinq ou six champs, évaluée en UTC ; exige la feature scheduler
      --force         Écrit même si le working tree Git est sale
      --dry-run       Affiche le plan sans rien écrire
      --json          Rend le plan, ou l'erreur, en un document JSON sur la sortie standard
  -h, --help          Affiche l'aide
  -V, --version       Affiche la version
```

Ni `--fields`, ni entité : un job n'est pas une feature CRUD, et le manifeste ne garde
jamais trace de son nom. `<NAME>` est à la fois le module sous `src/modules/jobs/` et le
`KIND` auquel le registre le retrouve, ce qui en fait un identifiant Rust valide — refusé
sinon, tout comme un mot-clé Rust ou un nom qui entre en collision avec l'un des six
fichiers que le fragment `jobs` porte déjà (`config`, `demo`, `model`, `queue`, `worker`,
`tests`), ou avec `jobs` lui-même — `pub mod jobs;` dans `src/modules/jobs/mod.rs`
nommerait le module comme son propre dossier, ce que `clippy::module_inception` refuse. Le
nom d'une crate que ce fichier ou la template du job emploie l'est aussi — `std`, `core`,
`alloc`, `serde`, `serde_json`, `anyhow`, `async_trait`, `tracing` : déclaré là, le module
masquerait la crate à chaque chemin `serde::…` du fichier.

La commande exige la feature `jobs`, et `--every` exige en plus `scheduler` — chaque refus
nomme la commande qui installe ce qui manque. Un projet qui a reçu l'une ou l'autre avant
1.3.0 la porte encore sous `src/jobs/` ou `src/scheduler/`, que `rbs upgrade` ne déplace
pas : la commande le refuse aussi, en nommant le déplacement à faire à la main. Sur un
projet qui porte `jobs` :

{/* rbs:transcript cmd="rbs generate job purge_sessions --dry-run" setup="rbs new demo --yes --with jobs --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs generate job purge_sessions --dry-run
plan pour …/demo

  + src/modules/jobs/purge_sessions.rs   créé
  ~ src/modules/jobs/mod.rs              modifié

  1 à créer, 1 à modifier

  rien n'a été écrit (--dry-run)
```

Deux ancres ici, toutes deux déposées par le fragment `jobs` et non par le squelette :
`// <rbs:job_modules>` déclare le module, `// <rbs:jobs>` l'inscrit au worker. Sur un
projet qui porte aussi `scheduler`, `--every` ajoute un troisième fichier et une troisième
ancre — `// <rbs:schedules>` pousse l'échéance du job dans le calendrier :

```text
$ rbs generate job purge_sessions --every "0 3 * * *" --dry-run
plan pour /private/tmp/rbs-demo/demo

  + src/modules/jobs/purge_sessions.rs   créé
  ~ src/modules/jobs/mod.rs              modifié
  ~ src/modules/scheduler/mod.rs         modifié

  1 à créer, 2 à modifier

  rien n'a été écrit (--dry-run)
```

Relancer l'une ou l'autre commande ne change rien : un fichier de job déjà présent n'est
jamais réécrit, `--force` compris, et le plan le signale inchangé.

Sur un projet engendré avant 1.5.0, `src/modules/jobs/mod.rs` n'a pas
`// <rbs:job_modules>`. Le fichier du job s'écrit quand même, mais pas sa déclaration — ni
l'inscription et l'échéance, qui nomment le module et empêcheraient le projet de compiler
sans elle : le plan affiche les trois blocs à la place. `rbs doctor --fix` repose cette
ancre, après quoi relancer la commande écrit le reste. Un calendrier encore écrit en
`vec![]` n'a pas de ligne où accrocher `// <rbs:schedules>` ; le plan le dit plutôt que de
promettre `--fix`, et le
[guide scheduler](../guides/scheduler.md#un-calendrier-antérieur-à-lancre) montre la
réécriture.

## La grammaire de `--fields`

Un champ par virgule ; à l'intérieur d'un champ, les deux-points séparent un nom, un type,
et autant de modificateurs qu'on veut :

```text
nom:type[:modificateur…][,nom:type[:modificateur…]…]
```

Les espaces autour de chaque séparateur sont ignorés : `" titre : string , email : string :
unique "` et `"titre:string,email:string:unique"` décrivent les deux mêmes champs. Un
`--fields` vide ne déclare aucun champ. Les champs gardent leur ordre de déclaration dans
l'entité comme dans la migration.

### Les huit types

Il n'y en a pas un neuvième, ni de type `email` : un format de chaîne n'est pas un type de
colonne.

| Type | Rust | Migration |
|---|---|---|
| `string` | `String` | `string()` |
| `text` | `String` | `text()` |
| `int` | `i32` | `integer()` |
| `float` | `f64` | `double()` |
| `bool` | `bool` | `boolean()` |
| `uuid` | `Uuid` | `uuid()` |
| `datetime` | `DateTimeWithTimeZone` | `timestamp_with_time_zone()` |

`string` et `text` partagent leur type Rust : `text` est donc le seul à porter en plus un
type de colonne explicite sur l'entité, sans quoi SeaORM déduirait un `varchar`.

Le huitième, `references`, n'est pas un scalaire du tout : il pointe la colonne vers une
autre entité plutôt que de lui donner un type propre.

```text
author:references:users
```

Le nom déclaré est celui de la *relation*, `author` ; la colonne s'en dérive, `author_id` —
ce qui permet à la variante SeaORM, à la clé étrangère et au champ du DTO de s'accorder sur
un nom sans que personne ne le répète. Le troisième segment est la table cible, telle qu'elle
existe dans le projet ; une table que le CLI ne trouve pas est refusée, nommément, aux côtés
de celles qu'il connaît. Ce qu'une référence écrit des deux côtés de la relation, ses deux
modificateurs propres et la forme de ses refus relèvent de
[Relations](../guides/relations.md), pas de cette page.

### Les six modificateurs

| Modificateur | Effet |
|---|---|
| `unique` | Contrainte d'unicité sur la colonne — sur une référence, c'est ce qui rend la relation un-à-un. |
| `optional` | La colonne devient nullable et le type Rust devient `Option<T>`. |
| `index` | Index simple sur la colonne. |
| `max=<n>` | Réservé aux champs textuels. Borne de longueur dans les DTO générés, en surcharge du défaut. |
| `cascade` | Réservé aux références. `ON DELETE CASCADE`. |
| `nullify` | Réservé aux références. `ON DELETE SET NULL` — exige `optional`. |

Leur ordre est libre et chacun ne peut apparaître qu'une fois. `unique` et `index` ensemble
sont refusés comme redondants : une contrainte d'unicité pose déjà un index — et `index` seul
sur une référence l'est tout autant, sa clé étrangère étant indexée sans qu'on le demande.
`cascade` et `nullify` se contredisent et sont refusés ensemble ; le reste de la grammaire
d'une référence, et pourquoi son index n'est jamais optionnel, vit dans
[Relations](../guides/relations.md).

Ni `unique` ni `index` ne s'applique à un champ `text` : MySQL refuse un index sur une
colonne `TEXT` sans longueur de préfixe (erreur 1170). Le refus vaut pour tous les moteurs,
PostgreSQL compris — une migration engendrée est faite pour tourner partout, et une règle
est une règle. Une colonne de texte qu'on indexe est un `string`, c'est-à-dire un
`varchar(255)`.

### Ce qu'un nom peut être

Un nom de champ est en `snake_case` : il commence par une minuscule ASCII et ne porte que
des minuscules, des chiffres et des soulignés, sans souligné final. Quatre familles de noms
sont refusées d'emblée, chacune produisant sinon un projet qui ne compile pas ou un schéma
faux :

- les 51 mots-clés stricts et réservés de Rust, des éditions 2015 à 2024 — rustc l'aurait
  dit quarante secondes plus tard ;
- `id`, `created_at` et `updated_at`, que rbs pose sur toute entité ;
- `table`, qui entre en collision avec la variante `Table` que `DeriveIden` réserve au nom
  de la table dans la migration ;
- un nom déjà déclaré plus tôt dans le même `--fields`.

Un champ nommé `email`, ou finissant par `_email`, et typé `string` ou `text` reçoit une
contrainte d'email dans les DTO générés. Elle se déduit du nom, seule information dont on
dispose.

### La borne de longueur

Un champ `string` est borné à 255 caractères dans les DTO générés, sans qu'on le demande :
`#[validate(length(max = 255))]`. Rien d'autre ne le borne — `ColumnDef::string()` rend un
`varchar` sans longueur sur PostgreSQL — si bien que sans cette ligne chaque route publique
d'un projet engendré accepte une chaîne de longueur arbitraire. La valeur est celle du
`varchar(255)` traditionnel, assez large pour un nom, un titre ou une adresse.

`text` est le type qu'on choisit *pour* dépasser cette borne : il n'en reçoit donc aucune
par défaut. `max=<n>` pose une borne sur l'un comme sur l'autre, ou élargit et resserre
celle du défaut :

```bash
rbs generate crud articles --fields "titre:string:max=200,resume:text:max=5000"
```

`max=` borne une longueur de texte et est refusé sur tout autre type ; `<n>` est un entier
strictement positif. Une contrainte écrite à la main dans le DTO engendré survit, comme
toute autre retouche : ce code est fait pour être modifié.

### Les erreurs

Toutes les fautes de la ligne sont collectées en une passe : la ligne se corrige d'un coup
plutôt qu'une faute par exécution. Un champ qui en porte deux ne remonte que la première.

```text
$ rbs generate crud tags --fields "Title:string,type:text,prix:decimal,slug:string:unique:index,email:string,email:int" --dry-run
erreur : champ 1 « Title » — le nom doit être en snake_case : minuscules ASCII, chiffres et souligné
        → essayez « title »
erreur : champ 2 « type » — « type » est un mot-clé Rust
        → essayez « kind » ou « type_ »
erreur : champ 3 « prix » — type inconnu « decimal »
        → string, int, float, bool, uuid, datetime, text, references:<table>
erreur : champ 4 « slug » — « index » redondant : « unique » pose déjà un index
        → retirez « index »
erreur : champ 6 « email » — « email » est déjà déclaré au champ 5
        → un nom de champ ne peut apparaître qu'une fois
```

Noter le rang du doublon : le champ 6 est signalé contre le champ 5, et le champ 5 lui-même
est accepté.

```text
$ rbs generate crud tags --fields "id:string,table:string,bio:text:optional:optional" --dry-run
erreur : champ 1 « id » — « id » ne se déclare pas
        → id, created_at et updated_at sont posés sur toute entité
erreur : champ 2 « table » — « table » entrerait en collision avec l'identifiant de la table dans la migration
        → essayez « table_ »
erreur : champ 3 « bio » — modificateur « optional » en double
```

Un champ sans type — ou un séparateur en trop, comme une virgule finale ou `email:string:` —
est une faute de forme, et non un type inconnu :

```text
$ rbs generate crud tags --fields "titre" --dry-run
erreur : champ 1 « titre » — forme attendue : « nom:type[:modificateur…] »
        → exemple : « email:string:unique »
```

## Le plan

Chaque exécution affiche son plan avant d'écrire quoi que ce soit : ce que la commande
s'apprête à faire ne doit pas se découvrir après coup. `--dry-run` s'arrête là.

```text
$ rbs generate crud articles --fields "title:string,body:text,slug:string:unique,published:bool,views:int:optional" --dry-run
plan pour /private/tmp/rbs-demo/blog

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/filter.rs                              créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests/mod.rs                           créé
  + src/articles/tests/lifecycle.rs                     créé
  + src/articles/tests/errors.rs                        créé
  + src/articles/tests/filter.rs                        créé
  + src/seeds/articles.rs                               créé
  + migration/src/m20260830_110925_create_articles.rs   créé
  ~ src/lib.rs                                          modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ src/seeds/main.rs                                   modifié
  ~ Cargo.toml                                          modifié
  ~ AGENTS.md                                           modifié

  13 à créer, 7 à modifier

  rien n'a été écrit (--dry-run)
```

La même commande sans `--dry-run` affiche le même plan, puis l'applique :

```text
$ rbs generate crud articles --fields "title:string,body:text,slug:string:unique,published:bool,views:int:optional"
plan pour /private/tmp/rbs-demo/blog

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/filter.rs                              créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests/mod.rs                           créé
  + src/articles/tests/lifecycle.rs                     créé
  + src/articles/tests/errors.rs                        créé
  + src/articles/tests/filter.rs                        créé
  + src/seeds/articles.rs                               créé
  + migration/src/m20260830_110925_create_articles.rs   créé
  ~ src/lib.rs                                          modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ src/seeds/main.rs                                   modifié
  ~ Cargo.toml                                          modifié
  ~ AGENTS.md                                           modifié

  13 à créer, 7 à modifier
✓ articles générée — 13 créés, 7 modifiés

  la migration m20260830_110925_create_articles reste à appliquer avant de lancer le projet
```

Treize fichiers créés, sept modifiés par leurs ancres. La feature est ensuite inscrite dans le
manifeste, ce qui rend la commande idempotente :

```text
[package.metadata.rbs]
version = "1.2.0"
features = ["health", "articles"]
database = "postgres"
```

Les marques du plan se lisent : `+` créé, `~` modifié, `·` inchangé, `!` en conflit.

`rbs generate feature` écrit six fichiers et aucune migration :

```text
$ rbs generate feature comments --force
plan pour /private/tmp/rbs-demo/blog

  + src/comments/mod.rs          créé
  + src/comments/model.rs        créé
  + src/comments/dto.rs          créé
  + src/comments/repository.rs   créé
  + src/comments/service.rs      créé
  + src/comments/controller.rs   créé
  ~ src/lib.rs                   modifié
  ~ src/router.rs                modifié
  ~ src/openapi.rs               modifié
  ~ Cargo.toml                   modifié
  ~ AGENTS.md                    modifié

  6 à créer, 5 à modifier
✓ comments générée — 6 créés, 5 modifiés
```

## Un working tree sale

Les fichiers générés sont neufs, mais les insertions modifient des fichiers que vous avez
déjà. `rbs generate` refuse donc de passer sur des changements non commités — y compris
sous `--dry-run`, le contrôle ayant lieu pendant la planification :

```text
$ rbs generate feature comments
erreur : le working tree n'est pas propre : Cargo.toml, src/lib.rs, src/openapi.rs, src/router.rs — commitez, ou relancez avec --force
```

Les fichiers non suivis ne comptent pas : ce sont précisément ceux que la commande
s'apprête à créer. Au-delà de cinq noms, la liste est abrégée. `--force` passe outre, ce que
le message suggère et ce que l'exécution ci-dessus a utilisé.

## Les ancres

`rbs generate` ne réécrit jamais d'AST. Il insère entre des marqueurs en commentaires que le
squelette porte. `rbs generate crud` et `rbs generate feature` en emploient six sur seize —
les deux de `src/state.rs`, `// <rbs:layers>` et `// <rbs:startup>` appartiennent aux
fragments qu'installe [`rbs add`](./add.md) :

| Ancre | Fichier |
|---|---|
| `// <rbs:features>` | `src/lib.rs` |
| `// <rbs:routes>` | `src/router.rs` |
| `// <rbs:openapi>` | `src/openapi.rs` |
| `// <rbs:migration_modules>` | `migration/src/lib.rs` |
| `// <rbs:migrations>` | `migration/src/lib.rs` |
| `// <rbs:seeds>` | `src/seeds/main.rs` |

`rbs generate job` en emploie trois, sans en partager aucune avec les deux commandes
ci-dessus ni avec le squelette : chacune vit dans un fichier qu'un fragment dépose, et
`// <rbs:jobs>` reçoit aussi la livraison du fragment `webhooks` :

| Ancre | Fichier |
|---|---|
| `// <rbs:job_modules>` | `src/modules/jobs/mod.rs`, déposée par `jobs` |
| `// <rbs:jobs>` | `src/modules/jobs/mod.rs`, déposée par `jobs` |
| `// <rbs:schedules>` | `src/modules/scheduler/mod.rs`, déposée par `scheduler`, sous `--every` |

`src/lib.rs` est la bibliothèque que porte tout projet engendré : `src/main.rs` et
`src/seeds/main.rs` sont deux racines de crate distinctes, et la bibliothèque est ce qui
permet aux deux d'atteindre les modules d'une feature — modèles compris, maintenant qu'une
relation peut en nommer un depuis un autre. Un projet engendré avant que cette bibliothèque
existe n'en a pas, et sur lui `// <rbs:features>` reste où elle a toujours vécu, dans
`src/main.rs` — `rbs generate` et `rbs doctor` résolvent l'ancre vers le fichier
réellement présent, si bien qu'un projet plus ancien continue de fonctionner sans y
toucher.

Retirez-en une et la commande n'écrit rien du tout — pas même les fichiers de la feature —
et affiche le bloc à recoller :

```text
$ rbs generate feature notes --force
erreur : ancre // <rbs:routes> introuvable dans src/router.rs

dans src/router.rs :
// <rbs:routes>
// </rbs:routes>
```

[`rbs doctor`](./doctor.md) contrôle les seize ancres — onze sur un projet qui ne
porte ni compose, ni file, ni fragment déplacé sous `src/modules/`, les cinq
optionnelles — si bien qu'une ancre disparue se trouve avant qu'une génération ne bute
dessus.

## Les échecs

Une feature déjà là est refusée plutôt que fusionnée :

```text
$ rbs generate crud articles --fields "title:string"
erreur : src/articles existe déjà : la feature `articles` est déjà là
```

Hors d'un projet :

```text
$ rbs generate crud users --dry-run
erreur : aucun projet rbs ici : `rbs generate` s'exécute dans un projet créé par `rbs new`
```

Les deux sortent en code 2 : c'est l'appel qu'il faut corriger — un autre nom, ou le bon
répertoire. Voir les [codes de sortie](./doctor.md#codes-de-sortie).
