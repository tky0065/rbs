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

Version 0.4.0. Les quatre jalons de la feuille de route sont livrés — le socle,
l'authentification, les intégrations, le confort. **rbs suit le versionnage sémantique à
partir de la 1.0** : la [page de compatibilité](./compatibility.md) dit ce que la promesse
couvre, et ce qu'elle laisse délibérément dehors.

## À quoi ressemble le code généré

Voici le handler `POST /articles`, tel que `rbs generate crud` l'écrit. Il est lu dans
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud), un
projet que la CI compile : aucun bloc de code de cette documentation n'est écrit à la main.

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

Le contrôleur ne fait rien de plus : il passe la requête au service et traduit le résultat
en code de statut. Le service reçoit la `DatabaseConnection` et la transmet sans jamais
l'interroger : des six fichiers d'une feature, `repository.rs` est le seul à nommer une
`Entity`.

## Par où continuer

- **[Démarrage rapide](./getting-started.md)** — de l'installation à une API CRUD qui
  répond.
- **[Architecture](./architecture.md)** — la frontière noyau/généré, l'anatomie d'une
  feature, la règle de dépendance.
- **[Référence du CLI](./cli/new.md)** — chaque commande, chaque option, avec une sortie
  réelle.
- **Guides** — [configuration](./guides/configuration.md), [logs](./guides/logs.md),
  [erreurs](./guides/errors.md), [OpenAPI](./guides/openapi.md),
  [migrations](./guides/migrations.md), [tests](./guides/testing.md).

La [feuille de route](https://github.com/tky0065/rbs/blob/main/ROADMAP.md) liste ce qui
entre dans le périmètre et ce qui en est délibérément exclu.
