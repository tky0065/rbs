---
sidebar_position: 7
title: rbs seed
---

# `rbs seed`

Insère les données de démonstration du projet en lançant son binaire `seed`. Ce que les
seeds contiennent, et comment ils s'écrivent, est le [guide des seeds](../guides/seeds.md) ;
cette page est la commande.

:::note
rbs parle français dans ses écrans d'aide et dans ses sorties. Tous les blocs de terminal
de cette page sont verbatim, capturés en lançant la commande.
:::

## Synopsis

```text
$ rbs seed --help
Insère les données de démonstration du projet

Usage: rbs seed [OPTIONS]

Options:
      --force    Insère même sous RBS_ENV=production
  -h, --help     Print help
  -V, --version  Print version
```

Comme [`rbs migrate`](./migrate.md), la commande enveloppe un binaire du projet plutôt que
de parler elle-même à la base. rbs ne gagne aucun client SQL, et le code qui insère reste
là où vous pouvez le lire et le modifier.

## Lancer les seeds

```text
$ rbs seed
subscribers : inséré
✓ seeds insérés
```

Une ligne par seed, dans l'ordre de l'ancre `<rbs:seeds>`, puis un résumé.

## Rien à insérer

```text
$ rbs seed
✓ aucun seed déclaré — rien à insérer
```

Code de sortie 0, et cargo n'est jamais démarré. Ce n'est pas un échec : un projet qui n'a
encore engendré aucun CRUD n'a rien à peupler, et le dire ne coûte rien. L'absence de
compilation est ce qui rend la réponse instantanée — et, accessoirement, ce qu'un test
constate en mesurant la durée de la commande.

Un projet dont `src/seeds/` manque tout à fait — créé avant que le répertoire existe —
reçoit un message nommant le fichier à créer et le bloc `[[bin]]` à ajouter, plutôt que
l'erreur de manifeste qu'aurait rendue cargo.

## Le refus en production

```text
$ RBS_ENV=production rbs seed
erreur : RBS_ENV=production : les seeds sont des données de démonstration, et rbs refuse de les insérer en production — relancez avec --force si c'est bien ce que vous voulez
```

Code de sortie 1, et le binaire du projet n'est **pas** lancé — cargo ne démarre même pas.

Le garde-fou vit dans la commande et non dans le code engendré, et c'est délibéré. Un seed
est un fichier fait pour être modifié ; un refus posé à l'intérieur est un refus qu'on
supprime par mégarde en réécrivant le code autour. Ici, rien de ce que vous faites à vos
seeds ne peut le retirer.

`--force` est le passage. Il faut le taper, et c'est toute l'idée.

:::warning
`RBS_ENV` est lue dans l'environnement, non dans le projet. Un shell où elle est exportée
depuis une commande précédente refusera, et un shell de production où elle n'a jamais été
posée ne refusera pas. C'est la même variable qui choisit `config/production.toml` — voir
le [guide de la configuration](../guides/configuration.md).
:::

## Échecs

| Situation | Ce qui se passe |
|---|---|
| Hors d'un projet | Refus nommant ce qu'elle a cherché |
| Pas de `src/seeds/` | Message nommant le fichier à créer et son bloc `[[bin]]` |
| Aucun seed déclaré | `✓`, code de sortie 0, cargo non démarré |
| `RBS_ENV=production` sans `--force` | Refus nommant `--force`, binaire non lancé |
| Base injoignable | Le binaire des seeds ping avant le premier insert et le dit |
| Un seed échoue | L'erreur propre au binaire ; les seeds précédents sont déjà insérés |

Cette dernière ligne mérite deux lectures : les seeds ne sont pas enveloppés dans une seule
transaction. Un échec à mi-parcours laisse en place les lignes déjà insérées.
