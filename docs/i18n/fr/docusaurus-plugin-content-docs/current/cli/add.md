---
sidebar_position: 3
title: rbs add
---

# `rbs add`

Installe une feature dans un projet existant. La 0.1.0 en livre deux : `docker` et `ci`.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

```text
$ rbs add --help
Ajoute une feature à un projet existant : docker, ci

Usage: rbs add [OPTIONS] <FEATURE>

Arguments:
  <FEATURE>  Feature à installer

Options:
      --force                  Applique les modifications même si le working tree Git est sale
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

| Flag | Effet |
|---|---|
| `--force` | Applique même si le working tree Git est sale, et écrase les fichiers signalés en conflit. |
| `--template-dir <CHEMIN>` | Lit les fragments dans un répertoire portant un sous-répertoire par feature, au lieu de ceux embarqués dans le binaire. |
| `-y`, `--yes` | Global, et sans effet ici : `rbs add` ne demande rien. |

## Les deux features

| Feature | Fichiers | Suite |
|---|---|---|
| `docker` | `.dockerignore`, `Dockerfile`, `docker-compose.yml` | `docker compose up --build` |
| `ci` | `.github/workflows/ci.yml` | `git push` |

```text
$ rbs add docker
plan pour /private/tmp/rbs-demo/blog

  + .dockerignore        créé
  + Dockerfile           créé
  + docker-compose.yml   créé
  ~ Cargo.toml           modifié

  4 fichiers à écrire
✓ docker installée — 3 fichiers

  docker compose up --build
```

```text
$ rbs add ci
plan pour /private/tmp/rbs-demo/blog

  + .github/workflows/ci.yml   créé
  ~ Cargo.toml                 modifié

  2 fichiers à écrire
✓ ci installée — 1 fichier

  git push : le workflow s'exécute à la prochaine poussée
```

La ligne `Cargo.toml` est le quatrième fichier du plan, respectivement le deuxième : le
manifeste est l'endroit où l'installation s'inscrit.

```text
[package.metadata.rbs]
version = "0.1.0"
features = ["health", "articles", "comments", "docker", "ci"]
```

Tout autre nom est refusé avec la liste de ce qui est installable :

```text
$ rbs add graphql
erreur : `graphql` n'est pas une feature installable : ci, docker
```

## L'idempotence

Relancée, la commande annonce chaque fichier inchangé au lieu de le réécrire. Elle réussit
tout de même — installer ce qui est déjà installé n'est pas un échec — et dit que rien n'a
été touché :

```text
$ rbs add docker
plan pour /private/tmp/rbs-demo/blog

  · .dockerignore        inchangé
  · Dockerfile           inchangé
  · docker-compose.yml   inchangé
  · Cargo.toml           inchangé

  4 inchangés
✓ docker installée — 3 fichiers

  docker compose up --build
```

Les marques du plan se lisent : `+` créé, `~` modifié, `·` inchangé, `!` en conflit.

## Un working tree sale

`rbs add` modifie `Cargo.toml` : il refuse donc de passer sur des changements non commités.

```text
$ rbs add ci
erreur : le working tree n'est pas propre : Cargo.toml — commitez, ou relancez avec --force
```

Les fichiers non suivis ne comptent pas : ce sont précisément ceux que la commande
s'apprête à créer. Au-delà de cinq noms, la liste est abrégée. `--force` passe outre, ce que
le message suggère.

## Les conflits

Un fichier qui existe avec un contenu que le fragment ne retrouve pas n'est ni fusionné ni
écrasé en silence. Le plan le marque `!`, et la commande s'arrête :

```text
$ rbs add docker --template-dir /private/tmp/rbs-demo/mes-features
plan pour /private/tmp/rbs-demo/blog

  ! Dockerfile   conflit — relancer avec --force
  · Cargo.toml   inchangé

  1 inchangé, 1 en conflit
erreur : Dockerfile — relancer avec --force pour les écraser
```

`--force` écrase, après avoir montré le même plan :

```text
$ rbs add docker --template-dir /private/tmp/rbs-demo/mes-features --force
plan pour /private/tmp/rbs-demo/blog

  ! Dockerfile   conflit — relancer avec --force
  · Cargo.toml   inchangé

  1 inchangé, 1 en conflit
✓ docker installée — 1 fichier

  docker compose up --build

$ cat Dockerfile
FROM scratch
```

Si l'application du plan échoue à mi-chemin, ce qui a déjà été écrit est défait : une
installation partielle ne laisse pas une feature à moitié posée.

## Des templates prises du disque

`--template-dir` attend un répertoire portant un sous-répertoire par feature — `docker/`,
`ci/`, ou ce que vous y ajoutez — chacun avec ses templates `.jinja`, le suffixe retiré en
sortie. Il remplace le catalogue embarqué au lieu de s'y ajouter : un répertoire qui ne
porte pas la feature demandée est donc un répertoire où aucune feature n'existe.

```text
$ rbs add docker --template-dir /nexistepas
erreur : `docker` n'est pas une feature installable : aucune n'est disponible
```

C'est aussi pourquoi un catalogue vide est refusé ici plutôt qu'au rendu : il produirait
sinon un plan vide, donc une commande qui réussit sans rien faire.

## Les ancres

`rbs add` écrit des fichiers entiers et modifie le manifeste ; c'est
[`rbs generate`](./generate.md#les-ancres) qui insère dans les cinq ancres en commentaires
du projet — `// <rbs:features>`, `// <rbs:routes>`, `// <rbs:openapi>`,
`// <rbs:migration_modules>` et `// <rbs:migrations>`. La règle est la même pour les deux
commandes : aucun AST n'est jamais réécrit, et une ancre absente fait que la commande
n'écrit rien et affiche le bloc à recoller. [`rbs doctor`](./doctor.md) les contrôle toutes
les cinq.

## Les échecs

Hors d'un projet :

```text
$ rbs add docker
erreur : aucun projet rbs ici : `rbs add` s'exécute dans un projet créé par `rbs new`
```

Chacun de ces cas sort en code 1.
