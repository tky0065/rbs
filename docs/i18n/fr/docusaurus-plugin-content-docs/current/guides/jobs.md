---
sidebar_position: 11
title: Jobs
---

# Jobs en arrière-plan

`rbs add jobs` installe une file de travaux dans un projet existant : sept fichiers sous
`src/jobs/`, une migration pour la table `jobs`, et un worker démarré avec le serveur.
Comme les autres briques, elle ne monte aucune route — le moment où un travail sort du
cycle de la requête est une décision que seul votre métier peut prendre.

Tous les extraits de cette page viennent de
[`examples/newsletter-queue`](https://github.com/tky0065/rbs/tree/main/examples/newsletter-queue),
un projet engendré par le CLI et compilé en CI. Rien ici n'est écrit à la main pour la
documentation.

## Ce qui est installé

```text
$ rbs add jobs
jobs : jobs en arrière-plan : une table, un enfilage transactionnel, un worker qui réessaie

plan pour /private/tmp/rbs-demo/demo

  + src/jobs/mod.rs                                 créé
  + src/jobs/config.rs                              créé
  + src/jobs/model.rs                               créé
  + src/jobs/queue.rs                               créé
  + src/jobs/worker.rs                              créé
  + src/jobs/demo.rs                                créé
  + src/jobs/tests.rs                               créé
  + migration/src/m20260830_111505_create_jobs.rs   créé
  ~ migration/src/lib.rs                            modifié
  ~ src/lib.rs                                      modifié
  ~ src/main.rs                                     modifié
  ~ Cargo.toml                                      modifié
  ~ config/default.toml                             modifié
  ~ AGENTS.md                                       modifié

  14 fichiers à écrire
✓ jobs installée — 8 fichiers

  rbs migrate up, puis inscrivez vos jobs dans src/jobs/mod.rs
```

La migration vient avec, et [`rbs migrate up`](../cli/migrate.md) est donc la commande
suivante : tant que la table `jobs` n'existe pas, le worker démarre et ne trouve rien à
lire.

## Une table, et non Redis

La file est une ligne de votre base, et c'est là toute la conception.

Un job poussé dans Redis vit hors de la transaction qui l'a créé. Annulez cette
transaction — une validation qui échoue deux lignes plus loin, une contrainte violée au
commit — et la ligne sur laquelle votre job allait agir n'a jamais existé, quand le job est
déjà parti. L'inverse est pire : le commit réussit, la poussée dans Redis échoue, et le
travail est perdu en silence.

Enfilez dans la même transaction et ni l'un ni l'autre n'arrive. Le job existe si et
seulement si le travail qui l'a motivé existe.

Le prix est honnête : le débit est borné par votre base, et une file qui demanderait la
diffusion d'un courtier n'est pas ce qu'est cette feature. L'échange est délibéré, et
l'exemple `newsletter-queue` est bâti autour de lui.

## Configuration

```rust file=examples/newsletter-queue/src/jobs/config.rs
```

Trois réglages, chacun avec un défaut écrit là où la section est déclarée plutôt que dans
le noyau, de sorte que vous les lisiez et les changiez au même endroit :

```toml
[jobs]
max_attempts = 5
retry_delay_secs = 30
poll_interval_secs = 1
```

`config/{env}.toml` et les variables `RBS_JOBS__*` les surchargent comme celles de toute
autre section — voir le [guide de la configuration](./configuration.md).

## Écrire un job

Un job est un type sérialisable qui implémente un trait :

```rust file=examples/newsletter-queue/src/jobs/mod.rs region=trait
```

`KIND` est écrit dans la ligne et sert de clé au registre. Le renommer sans migration
laisse en file des jobs que plus rien ne sait exécuter — cette constante appartient à vos
données, non à votre seul code.

Voici celui de l'exemple :

```rust file=examples/newsletter-queue/src/jobs/newsletter.rs region=job
```

Deux décisions bonnes à reprendre. Le payload porte un identifiant plutôt qu'une adresse :
entre l'enfilage et l'exécution, un abonné a pu corriger la sienne, et c'est celle de
l'envoi qui compte. Et l'envoi est *attendu* — une erreur rendue par `run` vaut réessai, ce
qui est toute la raison d'en avoir fait un job.

## L'inscrire

```rust file=examples/newsletter-queue/src/jobs/mod.rs region=registry
```

Un `kind` absent du registre est traité comme un échec du job et non du worker : la ligne
part en réessai puis en échec définitif, et la file continue d'avancer. Le registre est le
seul endroit où l'oubli se voie, et il ne se voit qu'à l'exécution — raison pour laquelle
le `demo::Log` livré est fait pour être remplacé plutôt que laissé à côté des vôtres.

## Enfiler

```rust file=examples/newsletter-queue/src/jobs/queue.rs region=enqueue
```

`db` est un `ConnectionTrait` et non une connexion, et c'est tout le propos de cette
feature : une transaction en est un. Passez-lui celle que votre métier tient déjà.

`enqueue_at` prend une date à la place, pour un travail qui ne doit pas devenir exécutable
sur-le-champ.

## Ce que l'exemple en fait

`newsletter-queue` diffuse une lettre à chaque abonné confirmé, et les enfile toutes dans
la transaction qui les a lus :

```rust file=examples/newsletter-queue/src/subscribers/service.rs region=broadcast
```

Remplacez `&transaction` par `db` sur cet appel et la suite de tests vous le dit : les
lettres survivent au rollback qui les a annulées. C'est la panne qu'une file en base
existe pour empêcher, et l'exemple l'affirme par un test plutôt que par une phrase.

La route répond `202` et non `200` — les lettres sont enfilées, non envoyées :

```rust file=examples/newsletter-queue/src/subscribers/controller.rs region=broadcast
```

## Le worker

```rust file=examples/newsletter-queue/src/jobs/worker.rs
```

Il est détaché par `main.rs` à côté du serveur et rend la main aussitôt. Trois défaillances
sont traitées là où elles surviennent, et chaque réponse est délibérée :

- **la configuration est illisible** — le worker se retire en le disant, plutôt que
  d'emporter le serveur avec lui. L'API répond encore, et la file se remplit sans se
  vider ;
- **la base est momentanément injoignable** — le worker dort et retente au tour suivant,
  plutôt que de rendre la main pour de bon ;
- **le sort du job ne peut pas être inscrit** — la ligne reste en `running` et n'est plus
  dépilée. Le dire est tout ce que le worker peut faire ; la base ne répond pas.

:::note
Il y a un worker par processus, et il scrute. Plusieurs processus peuvent en faire tourner
un chacun : le dépilage réserve une ligne et incrémente son compteur en une seule requête —
avec `FOR UPDATE SKIP LOCKED` sur PostgreSQL et MySQL 8, une transaction immédiate sur
SQLite — de sorte que deux workers n'obtiennent jamais la même ligne, quel que soit leur
entrelacement.
:::

## Réessai et échec définitif

Un job qui rend une erreur est réessayé après `retry_delay_secs`, jusqu'à `max_attempts`
fois, puis marqué `failed` avec sa dernière erreur conservée dans la ligne. Plus rien ne le
réessaie ensuite, et rien ne vous en avertit non plus — `status = 'failed'` dans la table
`jobs` est là qu'ils vivent, et les surveiller vous appartient.

Le compteur est incrémenté à la réservation, non à l'échec. Un worker tué en cours de job a
donc déjà dépensé la tentative : le job n'est pas condamné à être réessayé sans fin par un
processus qui meurt dessus à chaque fois.

## Tests

Le `src/jobs/tests.rs` livré tourne contre une vraie base, comme tout test qui en touche
une — voir le [guide des tests](./testing.md). Quatre d'entre eux sont ceux à garder quand
vous modifiez le fragment :

- un job enfilé dans une transaction **annulée** n'existe pas après coup ;
- un job enfilé dans une transaction committée est visible du worker ;
- deux workers concurrents ne réservent jamais le même job — 200 jobs, 8 workers ;
- un job qui échoue est réessayé puis marqué en échec après la dernière tentative.

Le premier est celui qui justifie la conception. S'il ne passe pas, la file aurait aussi
bien pu être en mémoire.

## Ce qu'elle vous laisse

- **le moment d'enfiler** — ni route, ni hook, ni événement. L'exemple enfile à la
  diffusion parce que c'est ce dont son métier parle. Un job qu'il faut enfiler à l'horloge
  plutôt que sur un événement, c'est ce à quoi sert
  [`rbs add scheduler`](./scheduler.md) : il déclenche, et cette file exécute toujours ;
- **la surveillance des jobs en échec** — les lignes sont là, et rien ne les regarde ;
- **le nettoyage des lignes terminées** — rien ne purge la table ;
- **priorités et jobs uniques** — ni l'une ni les autres ne sont ici. `enqueue_at` est la
  seule primitive de planification que porte ce fragment ; les expressions cron vivent dans
  le [scheduler](./scheduler.md), qui enfile dans cette table même ;
- **un second processus pour le worker** — le worker tourne dans le processus de l'API.
  Les séparer demande un second binaire, qu'il vous revient d'ajouter.

Le code est dans votre arborescence, sans bandeau vous disant de ne pas y toucher.
