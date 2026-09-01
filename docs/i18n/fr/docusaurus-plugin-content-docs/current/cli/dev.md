---
sidebar_position: 6
title: rbs dev
---

# `rbs dev`

Démarre le projet en une commande : les services dont il a besoin, les migrations en
attente, puis le serveur, relancé à chaque changement. C'est la commande qu'on laisse
tourner.

:::note
rbs parle français dans ses écrans d'aide et dans ses sorties. Tous les blocs de terminal
de cette page sont verbatim, capturés en lançant la commande.
:::

## Synopsis

```text
$ rbs dev --help
Démarre le projet : services, migrations, serveur relancé à chaque changement

Usage: rbs dev

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Aucun drapeau propre. Ce qu'elle fait dépend entièrement de ce que le projet déclare.

## Le plan

`rbs dev` montre ce qu'elle va faire avant de le faire, comme toute commande qui touche à
un projet existant :

```text
  base        127.0.0.1:1
  migrations  rbs migrate up
  serveur     cargo run, relancé à chaque changement
```

Quatre étapes au plus, dans cet ordre :

1. **`docker compose up -d`**, si et seulement si le projet porte un `docker-compose.yml`
   — écrit par [`rbs new`](./new.md) pour la plupart des projets, que `docker` soit
   installée ou non. Un projet sans l'un ni l'autre ne fait chercher aucun compose ;
2. **l'attente de la base**, jusqu'à ce qu'elle accepte une connexion. Sautée pour SQLite,
   qui n'a pas de serveur à attendre — son URL n'a ni hôte ni port ;
3. **[`rbs migrate up`](./migrate.md)**, pour qu'un changement de schéma récupéré d'un
   collègue s'applique sans seconde commande ;
4. **le serveur**, `cargo run`, relancé à chaque changement sous `src/`.

Un projet avec un compose — le cas par défaut, pour la plupart — montre l'étape en plus,
en tête :

```text
  compose     docker-compose.yml
  base        localhost:15432
  migrations  rbs migrate up
  serveur     cargo run, relancé à chaque changement
```

`docker compose up -d` est appelée sans `--profile` — la même commande que nomme l'indice
de [`rbs new`](./new.md). Cela remonte `db` et ce qu'y ont ajouté [`redis`](../guides/cache.md)
ou [`mail`](../guides/mail.md), tous hors de tout profil, mais jamais `api` ni `migrate` :
[`rbs add docker`](./add.md) a placé les deux sous le profil `app`, précisément pour que
`rbs dev` — qui fait tourner le serveur lui-même, depuis les sources, à chaque
sauvegarde — n'ait jamais d'image à construire. `docker compose --profile app up --build`
est l'autre chemin, celui qui fait tourner le projet comme son propre conteneur au lieu
d'un `cargo run`.

## Deux patiences, et non une

La base reçoit 30 secondes quand `rbs dev` vient de remonter le compose, et 3 quand elle
était censée déjà tourner.

L'asymétrie est le propos. Un conteneur qui vient de démarrer met légitimement des dizaines
de secondes à accepter des connexions. Une base qui devait tourner et ne tourne pas ne
montera jamais d'elle-même — et trente secondes de silence pour apprendre qu'on a oublié de
démarrer PostgreSQL sont trente secondes perdues.

```text
erreur : rien ne répond sur 127.0.0.1:1 : la base du projet n'est pas démarrée

démarrez-la — `docker compose up -d` à la racine du projet — ou corrigez RBS_DATABASE__URL dans le .env du projet
```

Le message nomme l'hôte et le port essayés, et les deux issues. Ce n'est pas une trace de
panique : une base non démarrée est un mardi ordinaire, non un bug de rbs.

## Le watch

Un changement sous `src/` relance le serveur. Un changement sous `target/` non — et
celui-là n'est pas un filtre sur les événements mais un refus de descendre dans le
répertoire. Un build script qui écrit dans `target/debug/build/…/out/` pendant que le
serveur redémarre est exactement la boucle que cela évite.

Le point dur n'est ni le debounce ni le filtrage. C'est la coupure du serveur : un
`cargo run` tué sans son enfant laisse le port occupé, et le geste diffère sur Linux, macOS
et Windows. L'enfant est démarré dans son propre groupe de processus et c'est le groupe
entier qui est signalé, ce qui rend le port libre au redémarrage suivant — affirmé par un
test qui tourne sur les trois plateformes de la CI.

## Échecs

| Situation | Ce qui se passe |
|---|---|
| Pas de `.env`, ou pas d'URL de base dedans | Refus nommant le fichier et la variable |
| URL sans hôte | Refus nommant l'URL — une URL que rbs ne peut pas composer est une URL à corriger |
| Rien n'écoute | Le message ci-dessus, après la patience applicable |
| Une migration échoue | L'erreur propre au binaire de migration, et `rbs dev` s'arrête là |
| Hors d'un projet | Refus nommant ce qu'elle a cherché, comme toute autre commande |

Le serveur lui-même n'est pas supervisé : une fois `cargo run` lancé, sa sortie est la
vôtre, et `Ctrl-C` arrête l'ensemble.
