---
sidebar_position: 12
title: Seeds
---

# Seeds

Tout projet créé par `rbs new` porte `src/seeds/`, un second binaire dont l'unique travail
est d'insérer des données de démonstration. [`rbs seed`](../cli/seed.md) le lance.

Tous les extraits de cette page viennent de
[`examples/newsletter-queue`](https://github.com/tky0065/rbs/tree/main/examples/newsletter-queue),
un projet engendré par le CLI et compilé en CI. Rien ici n'est écrit à la main pour la
documentation.

## Du Rust typé, non du SQL

Un seed passe par l'entité que votre migration a produite :

```rust file=examples/newsletter-queue/src/seeds/subscribers.rs region=seed
```

Un champ renommé casse ici à la compilation. Un fichier `.sql` d'`INSERT` casserait en
silence à l'exécution, le jour où quelqu'un le lance — soit le jour où vous voulez le moins
l'apprendre.

`id`, `created_at` et `updated_at` tiennent leur valeur des défauts de colonne, et c'est
donc `..Default::default()` que vous écrivez au lieu de les choisir. Cette seule graphie
importe au-delà du confort : c'est l'unique point d'écriture par entité, et c'est là que
l'identifiant v7 est posé.

Notez l'attribut `#[path]`. Le binaire des seeds est une racine de crate distincte de celle
de l'application : il rejoint l'entité par son chemin plutôt que par une arborescence de
modules qu'il ne partage pas.

## L'ancre

```rust file=examples/newsletter-queue/src/seeds/main.rs region=ancre
```

`rbs generate crud` écrit le fichier du seed et ajoute son nom ici, dans l'ordre de
génération — qui est aussi celui des migrations, et le seul qui reste juste le jour où un
seed dépendra d'un autre.

Un `mod` non inline ne s'écrit pas dans un bloc : la déclaration des modules et leur
enchaînement se font donc d'un seul geste, à hauteur d'item :

```rust file=examples/newsletter-queue/src/seeds/main.rs region=macro
```

Ce macro est la raison pour laquelle il n'y a ici qu'un seul point d'insertion au lieu de
deux. `migration/`, qui n'a pas d'équivalent, demande deux ancres pour le même travail.

Rien n'interdit d'ajouter un seed à la main : créez le fichier, ajoutez son nom entre les
marqueurs.

## Les lancer

```text
$ rbs seed
subscribers : inséré
✓ seeds insérés
```

Une ligne par seed, dans l'ordre de l'ancre, et un résumé. Sur un projet où aucun seed n'a
encore été déclaré :

```text
$ rbs seed
✓ aucun seed déclaré — rien à insérer
```

Code de sortie 0, et cargo n'est jamais démarré. Un projet qui n'a rien à insérer n'a pas
échoué.

:::warning
`rbs seed` refuse de tourner sous `RBS_ENV=production` :

```text
$ RBS_ENV=production rbs seed
erreur : RBS_ENV=production : les seeds sont des données de démonstration, et rbs refuse de les insérer en production — relancez avec --force si c'est bien ce que vous voulez
```

Le refus vit dans la commande, non dans le code engendré. Un seed est fait pour être
modifié, et un garde-fou qu'on peut retirer par mégarde en réécrivant le fichier autour
n'est pas un garde-fou. `--force` est le passage, et il faut le taper.
:::

## Où ils tournent

Le binaire des seeds lit `RBS_DATABASE__URL` et se connecte seul — il ne passe pas par
`AppState` et ne démarre aucun serveur. Il ping la base avant le premier seed, de sorte
qu'une base injoignable se dise d'emblée plutôt qu'au milieu d'un insert.

Cela veut dire aussi que les seeds ne voient que vos migrations : ni middleware, ni
validation, ni couche de service. Ce qu'ils insèrent est ce que la base acceptera.

## Ce qu'ils vous laissent

- **l'idempotence** — lancer `rbs seed` deux fois insère deux fois, et une contrainte
  d'unicité vous le dira la seconde. Faites vérifier le seed d'abord s'il doit être
  rejouable ;
- **l'ordre entre entités** — celui de l'ancre est l'ordre de génération ; un seed qui
  dépend des lignes d'un autre doit venir après lui, et déplacer la ligne est la façon de
  le dire ;
- **le volume** — ce sont des lignes de démonstration, insérées une à une. Un jeu de
  100 000 lignes demande un autre outil ;
- **les données de production** — c'est à quoi servent les migrations, et le refus
  ci-dessus est là pour tenir les deux séparés.

Le code est dans votre arborescence, sans bandeau vous disant de ne pas y toucher.
