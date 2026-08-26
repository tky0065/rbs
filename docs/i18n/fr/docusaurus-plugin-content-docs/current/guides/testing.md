---
sidebar_position: 6
title: Tests
---

# Tests

`rbs generate crud` écrit un fichier de tests à côté de la feature qu'il génère, et le
monte dans le `mod.rs` de celle-ci. Les tests qu'il contient passent par HTTP, contre une
vraie base. C'est un banc de départ, pas une suite : ils prouvent le câblage et vous
laissent les règles.

## Le banc

L'application est montée dans le processus. Aucun socket n'est ouvert, aucune tâche de
serveur n'est lancée — le routeur est construit exactement comme `main` le construit, et
les requêtes lui sont remises directement :

```rust file=examples/hello-crud/src/articles/tests.rs region=harnais
```

La configuration est chargée comme le binaire la charge, ce qui veut dire que les tests
parlent à la base nommée dans votre `.env`. **Ils supposent les migrations déjà
appliquées.** Ils ne tournent ni contre un mock ni contre un substitut en mémoire : un
repository qui compile contre SeaORM mais écrit du SQL cassé est précisément l'échec qu'un
mock masquerait.

## Ce que le CLI génère

Un test parcourt le cycle de vie complet de la ressource — création, relecture, liste,
mise à jour, suppression, puis relecture pour confirmer qu'elle a disparu :

```rust file=examples/hello-crud/src/articles/tests.rs region=cycle_de_vie
```

Deux autres éprouvent les chemins d'erreur que le runtime traite tout seul : un
identifiant inconnu rend 404, un corps illisible rend 400. Les deux figurent dans le
[guide des erreurs](./errors.md).

Les valeurs textuelles portent un suffixe tiré au sort. Sans lui, un champ `unique` ferait
échouer la seconde exécution de la suite sur la ligne laissée par la première.

## Ce qu'il vous laisse

Tout ce qui est propre à votre domaine, c'est-à-dire tout ce qui compte :

- les règles métier — ce qui rend une valeur acceptable au-delà de son type ;
- les autorisations — qui peut lire, qui peut écrire ;
- vos propres cas limites — concurrence, bornes de pagination, états dont une ressource ne
  peut pas sortir.

Le fichier généré est du Rust ordinaire dans votre arbre de sources. Complétez-le,
scindez-le, supprimez-en ce qui cesse de servir. Rien ne le marque comme généré, parce que
rien ne doit vous empêcher de le modifier.

## Les lancer

Depuis la racine du projet, avec une base joignable :

```bash
rbs migrate up
cargo test
```

La première commande n'est pas facultative : `application()` échoue avec un message qui le
dit si le schéma n'est pas là.

## Comment rbs se teste lui-même

Les tests d'intégration du cadre ne supposent rien de démarré. Ils lancent un conteneur
PostgreSQL 18 avec `testcontainers`, génèrent un projet dans un répertoire temporaire,
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

PostgreSQL 18 n'y est pas négociable, pour la raison donnée dans le
[guide des migrations](./migrations.md) : les clés primaires générées ont `uuidv7()` pour
défaut, native seulement à partir de cette version.
