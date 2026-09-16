---
sidebar_position: 6
title: Tests
---

# Tests

`rbs generate crud` écrit un répertoire `tests/` à côté de la feature qu'il génère — un
harnais dans `tests/mod.rs`, puis un fichier par préoccupation — et le monte dans le
`mod.rs` de celle-ci. Les tests qu'il contient passent par HTTP, contre une vraie base.
C'est un banc de départ, pas une suite : ils prouvent le câblage et vous laissent les
règles.

## Le banc

L'application est montée dans le processus. Aucun socket n'est ouvert, aucune tâche de
serveur n'est lancée — le routeur est construit exactement comme `main` le construit, et
les requêtes lui sont remises directement :

```rust file=examples/hello-crud/src/articles/tests/mod.rs region=harnais
```

La configuration est chargée comme le binaire la charge, ce qui veut dire que les tests
parlent à la base nommée dans votre `.env`. **Ils supposent les migrations déjà
appliquées.** Ils ne tournent ni contre un mock ni contre un substitut en mémoire : un
repository qui compile contre SeaORM mais écrit du SQL cassé est précisément l'échec qu'un
mock masquerait.

## Ce que le CLI génère

Un test parcourt le cycle de vie complet de la ressource — création, relecture, liste,
mise à jour, suppression, puis relecture pour confirmer qu'elle a disparu :

```rust file=examples/hello-crud/src/articles/tests/lifecycle.rs region=cycle_de_vie
```

D'autres éprouvent les chemins d'erreur que le runtime traite tout seul. Deux sont
toujours écrits : un identifiant inconnu rend 404, un corps illisible rend 400. Deux de
plus dépendent de ce que vous avez demandé à `--fields`, faute de quoi rien ne les
atteindrait : un champ portant la contrainte d'e-mail vaut un corps lisible mais non
conforme, qui rend 422 ; une colonne `unique` vaut un rejeu de valeur, qui rend 409.
Chacun de ces quatre statuts est décrit dans le [guide des erreurs](./errors.md).

Les valeurs textuelles portent un suffixe tiré au sort, et chaque test supprime les lignes
qu'il a créées. Sans l'un ou l'autre, un champ `unique` ferait échouer la seconde exécution
de la suite sur ce que la première a laissé — les tests partagent la base que nomme votre
`.env`, et aucune transaction ne les annule.

## Ce qu'il vous laisse

Tout ce qui est propre à votre domaine, c'est-à-dire tout ce qui compte :

- les règles métier — ce qui rend une valeur acceptable au-delà de son type ;
- les autorisations — qui peut lire, qui peut écrire ;
- vos propres cas limites — concurrence, bornes de pagination, états dont une ressource ne
  peut pas sortir.

Les fichiers générés sont du Rust ordinaire dans votre arbre de sources. Complétez-les,
scindez-les encore, supprimez-en ce qui cesse de servir. Rien ne les marque comme générés,
parce que rien ne doit vous empêcher de les modifier.

## Les lancer

Depuis la racine du projet :

```bash
rbs test
```

[`rbs test`](../cli/test.md) remonte le compose si le projet en porte un, attend la base,
applique les migrations en attente, puis lance
`cargo test --workspace --no-fail-fast -- --include-ignored` — la commande que lance le
workflow qu'installe `rbs add ci`, après avoir fait les trois mêmes choses. Un filtre
restreint la passe, et ce qui suit `--` va au harnais de test :

```bash
rbs test articles -- --nocapture
```

Son code de sortie est celui de `cargo test` : un script distingue un test rouge ou un projet qui ne
compile pas (101) d'une base qui n'a jamais répondu (3).

À la main, la même chose tient en deux commandes, avec une base joignable :

```bash
rbs migrate up
cargo test --workspace --no-fail-fast -- --include-ignored
```

La première commande n'est pas facultative : `application()` échoue avec un message qui le
dit si le schéma n'est pas là.

Le `--include-ignored` ne l'est pas davantage. Tout test qui joint la base est marqué
`#[ignore = "joint la base du projet"]`, pour qu'un `cargo test` nu reste rapide sur un
poste où rien ne tourne — et il n'y lance alors rien qui compte.

## Comment rbs se teste lui-même

Les tests d'intégration du cadre ne supposent rien de démarré. Ils lancent un conteneur
PostgreSQL avec `testcontainers`, génèrent un projet dans un répertoire temporaire,
appliquent ses migrations et exécutent ses tests — le binaire `rbs` étant invoqué
exactement comme vous l'invoqueriez.

**Ces tests sont lents et exigent Docker.** Démarrer une base et compiler un projet
Axum + SeaORM complet prend plusieurs minutes : ils sont marqués `#[ignore]` et restent
hors d'un `cargo test` ordinaire.

```bash
cargo test -p rbs-cli --test integration_crud -- --ignored
```

Lent qu'il soit, c'est le seul test qui prouve que rbs fonctionne réellement. Tout le
reste vérifie une chaîne de caractères.

### Quelle version de PostgreSQL le harnais démarre

Deux versions comptent, et la CI joue la suite sur les deux.

**La 18 est ce qui est livré.** C'est ce que le `docker-compose.yml` engendré épingle, donc
ce qu'un projet rencontre réellement. C'est le défaut ici : un harnais qui démarre autre
chose que ce qui est livré ne prouve rien de ce qui est livré.

**La 14 est le plancher.** C'est ce que `rbs doctor` fait respecter — la plus ancienne
version encore corrigée côté sécurité — et un plancher que rien n'exerce est une promesse
que personne ne tient. Pour la raison donnée dans le
[guide des migrations](./migrations.md), les clés primaires engendrées sont posées par le
modèle et non par un défaut de colonne : rien de ce qu'exécute un projet engendré ne réclame
le `uuidv7()` arrivé avec PostgreSQL 18. Cette affirmation est désormais éprouvée plutôt
qu'avancée.

`RBS_TEST_PG` choisit la version, la 18 s'appliquant en son absence :

```bash
RBS_TEST_PG=14 cargo test -p rbs-cli --no-fail-fast -- --ignored
```

La variable est lue au démarrage du conteneur et non à la compilation : les deux branches
de la matrice partagent une seule construction et ne diffèrent que par ce que Docker
télécharge. Tous les démarreurs du dépôt — les trois des tests d'intégration, celui du banc
des générateurs — résolvent leur image par la même fonction, si bien qu'aucune version ne
peut être épinglée dans le dos de la matrice.
