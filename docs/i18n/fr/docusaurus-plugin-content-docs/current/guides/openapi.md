---
sidebar_position: 4
title: OpenAPI
---

# OpenAPI

Un projet généré se documente lui-même. `rbs new` écrit un `src/openapi.rs` qui déclare le
document, `rbs generate crud` y inscrit les nouveaux handlers, et deux réglages décident
de ce qui est servi en HTTP.

## Le document

Un `#[derive(OpenApi)]`, une liste d'opérations, un modificateur :

```rust file=examples/hello-crud/src/openapi.rs region=document
```

L'ancre `// <rbs:openapi>` est l'endroit où `rbs generate crud` écrit. Le CLI ne réécrit
jamais d'AST : il insère entre des marqueurs en commentaires. Supprimez l'ancre et le CLI
n'écrit rien — il affiche le bloc à coller plutôt que de deviner où il va.

Ajouter une opération à la main est le même geste : annoter le handler d'un
`#[utoipa::path(...)]`, puis le nommer dans `paths(...)`.

## `CommonResponses`, déclarée une fois

`modifiers(&CommonResponses)` porte à lui seul toute la documentation des erreurs. Il
parcourt le document fini et fait trois choses :

- il enregistre le schéma `ProblemDetails` — le type même qui produit les corps d'erreur à
  l'exécution, de sorte que le schéma et la réponse ne peuvent pas diverger ;
- il ajoute une **422** et une **500** à toute opération qui n'en déclare pas déjà une. Ce
  sont les deux seules réponses que *toute* opération peut produire : le runtime valide
  partout et peut défaillir partout ;
- il enregistre `BadRequest`, `Unauthorized`, `Forbidden`, `NotFound` et `Conflict` sous
  `components/responses`, référençables par nom depuis un handler qui peut réellement les
  rendre.

Une opération qui documente son propre 422 garde le sien. Un handler en sait plus sur son
cas que le runtime, et sa description n'est pas écrasée.

L'alternative serait de répéter les cinq mêmes réponses sur chaque handler du projet, ce
qui est exactement le genre de chose qui cesse d'être vrai à la troisième feature.

## Les deux URL, et les deux interrupteurs

```rust file=examples/hello-crud/src/openapi.rs region=exposition
```

| URL | Ce qui y est servi |
|---|---|
| `/docs` | Swagger UI |
| `/api-docs/openapi.json` | le document lui-même |

`docs.swagger_ui` et `docs.openapi_json` valent `true` par défaut — la documentation doit
exister dès la génération du projet ; la couper est un geste de mise en production, pas
l'état initial. Le [guide de configuration](./configuration.md) dit où les écrire.

Une asymétrie mérite d'être connue, et elle se lit dans le code ci-dessus : Swagger UI
charge le document par HTTP et monte lui-même cette route. Afficher l'interface implique
donc d'exposer le document, et le router une seconde fois ferait paniquer Axum au
démarrage. **Pour n'exposer que le document, coupez `docs.swagger_ui`** — c'est la
combinaison qui sert à générer des clients ou à vérifier un contrat depuis la CI. La
combinaison inverse n'existe pas, et la demander ne change rien.

## Jugez par vous-même

Démarrez le projet et récupérez le document :

```bash
curl -s localhost:8080/api-docs/openapi.json | jq '.components.responses | keys'
```

Ou lisez les tests, qui portent sur un document rendu plutôt que sur le code qui le
construit :

```bash
cargo test -p rbs-core openapi::tests
```
