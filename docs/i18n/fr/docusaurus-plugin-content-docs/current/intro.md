---
sidebar_position: 1
slug: /
title: Introduction
---

# rbs

rbs est un cadre de travail pour API web en Rust, bâti sur Axum et SeaORM. Il fournit à un
projet ce qui n'a aucune raison de varier d'une API à l'autre — gestion d'erreurs, logs,
configuration, accès à la base, documentation OpenAPI — et génère le reste dans vos
propres sources, là où vous pourrez le lire et le modifier.

Cette frontière est toute la conception. `rbs-core` porte le runtime. L'outil en ligne de
commande `rbs` écrit les features dans votre projet : modèle, DTO, dépôt, service,
contrôleur. Rien n'y porte la mention « ne pas modifier » : ce code est fait pour être
modifié.

## État

La version 0.1 est en construction. **Aucune promesse semver avant la 1.0** : l'API
publique de `rbs-core` peut changer d'une version mineure à l'autre.

## À quoi ressemble le code généré

Voici le handler `POST /articles`, tel que `rbs generate crud` l'écrit. Il est lu dans
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud), un
projet que la CI compile : aucun bloc de code de cette documentation n'est écrit à la main.

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

Le contrôleur ne fait rien de plus : il passe la requête au service et traduit le résultat
en code de statut. Le service, lui, ne voit jamais de `DatabaseConnection`.

## Par où continuer

Cette documentation s'écrit en même temps que le code. Les pages ci-dessous se remplissent
à mesure que le jalon 0.1 se referme :

- **Démarrage rapide** — de l'installation à une API CRUD qui répond.
- **Architecture** — la frontière noyau/généré, l'anatomie d'une feature, la règle de
  dépendance.
- **Référence du CLI** — chaque commande, chaque option, avec une sortie réelle.
- **Guides** — configuration, logs, erreurs, OpenAPI, migrations, tests.

La [feuille de route](https://github.com/tky0065/rbs/blob/main/ROADMAP.md) liste ce qui
entre dans le périmètre de la 0.1 et ce qui en est délibérément exclu.
