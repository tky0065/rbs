---
sidebar_position: 9
title: Appeler l'API en TypeScript
---

# Appeler l'API en TypeScript

C'est le neuvième et dernier des neuf tutoriels. Il reprend `demo` juste après [Votre
première ressource](./first-resource.md) — en cours d'exécution, avec le CRUD `articles`
de cette page et sa migration appliquée — et le lit dans un client typé plutôt que dans
une réponse de serveur typée à la main côté front. Le cas : un navigateur ou un script
Node qui appelle `articles`, `PATCH` compris, sans qu'une seule interface soit écrite
deux fois.

## Ce qu'il vous faut

Rien de plus qu'à [Votre première ressource](./first-resource.md) : le même `demo`, avec
le CRUD `articles` engendré sur cette page. Aucun serveur n'a besoin de tourner pour
celle-ci — `generate client` compile le projet plutôt que de l'appeler.

## 1. Engendrer le client

```bash
rbs generate client --lang ts
```

{/* rbs:transcript cmd="rbs generate client --lang ts" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && rbs generate crud articles --fields title:string,body:text,published:bool" dans="demo" */}
```text
$ rbs generate client --lang ts
plan pour …/demo

  + clients/ts/client.ts   créé

  1 fichier à écrire
✓ client engendré — clients/ts/client.ts porte 7 opérations
```

Rien ici ne lit `src/articles/` directement, et rien ne devine la forme d'une route
depuis son gestionnaire : la commande lance `src/bin/openapi.rs` — le troisième binaire
que `rbs new` a écrit aux côtés de `demo` lui-même et du lanceur de graines — et lit ce
que `ApiDoc::openapi()` imprime sur sa sortie standard. C'est ce qui permet à la commande
de fonctionner sans aucun serveur à l'écoute ni aucune base joignable : le document est
un artefact de build, pas une réponse réseau. Sept opérations, c'est les cinq routes
d'`articles`, `POST /articles/filter`, et `GET /health` — chaque gestionnaire qui porte
un `operationId`, ce que chacun de ceux que `rbs generate crud` écrit fait par défaut.

Le point qui porte le reste de cette page : le client est lu depuis un document que le
projet expose déjà, si bien qu'il suit le serveur plutôt qu'une seconde copie de ses
types, entretenue à la main. Changez un champ par un nouveau `rbs generate crud`, ou
retouchez à la main la signature d'un gestionnaire, et c'est le tout prochain `rbs
generate client --lang ts` qui rattrape l'écart — pas une erreur d'exécution qu'un
appelant signale des jours plus tard. Régénérer après chaque changement de contrat est
la boucle pour laquelle cette commande est faite, pas une étape ajoutée par-dessus.

## Vérifier

Le fichier qui vient d'être écrit est sa propre preuve, et le relire n'exige rien de
plus que `rbs` lui-même : relancer exactement la même commande.

```bash
rbs generate client --lang ts
```

{/* rbs:transcript cmd="rbs generate client --lang ts" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && rbs generate crud articles --fields title:string,body:text,published:bool && rbs generate client --lang ts" dans="demo" */}
```text
$ rbs generate client --lang ts
plan pour …/demo

  · clients/ts/client.ts   inchangé

  1 inchangé
✓ client engendré — clients/ts/client.ts porte 7 opérations
```

`inchangé` est une preuve par idempotence : lire le même document OpenAPI une seconde
fois produit le même fichier, octet pour octet, si bien qu'il ne restait rien à écrire
au second passage. Le compte d'opérations réimprimé est le même sept — pas recalculé
depuis le fichier sur disque, mais relu à neuf depuis `ApiDoc::openapi()` à chaque fois,
ce qui rend cette commande sûre à relancer après chaque `rbs generate crud`, et non une
seule fois pour toutes.

## Ce qui a été installé

Deux extraits, lus depuis
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud) —
la même commande montrée plus haut, lancée sur le CRUD exact que [Votre première
ressource](./first-resource.md) a engendré.

### Le client

`ApiClient` est une classe configurable plutôt qu'un jeu de fonctions libres : l'URL de
base et les en-têtes se posent une fois, à la construction, plutôt que d'être passés à
chaque appel.

```typescript file=examples/hello-crud/clients/ts/client.ts region=classe
```

### Les opérations

Une méthode par opération, nommée d'après son `operationId` en camelCase.
`articlesFilter` poste plutôt qu'il ne lit, parce que les conditions qu'il porte
n'entreraient pas dans une URL ; `health` rend `Promise<void>`, puisque `GET /health`
répond sans corps à analyser.

```typescript file=examples/hello-crud/clients/ts/client.ts region=methodes
```

## Pour aller plus loin

- [`rbs generate client`](../cli/client.md) couvre les options que cette page n'a pas
  employées — `--out`, `--force`, `--dry-run` — et ce que fait la commande quand elle
  trouve un client que vous avez depuis retouché à la main.
- [OpenAPI](../guides/openapi.md) couvre le document depuis lequel le client de cette
  page a été lu, et ce qui fait qu'un gestionnaire porte un `operationId` en premier
  lieu.
- [`rbs generate`](../cli/generate.md) couvre la grammaire complète de `--fields`, pour
  une ressource plus riche que celle que cette série a bâtie une commande à la fois.
