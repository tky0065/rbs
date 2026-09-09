---
sidebar_position: 2
title: Votre première ressource
---

# Votre première ressource

C'est le deuxième des neuf tutoriels, et il reprend `demo` exactement là où
[Préparer le terrain](./setup.md) l'a laissé : en cours d'exécution, sans rien de monté
hormis un contrôle de santé. Celui-ci ajoute la forme dont la plupart des API sont
faites — une table que vous créez, listez et modifiez — avec une seule commande qui
transforme une déclaration `--fields` en entité, sa migration, et chaque couche entre
les deux : `rbs generate crud`. Le cas : les articles d'un blog, un titre, un corps, et
si la pièce est publiée.

## Ce qu'il vous faut

Rien de plus qu'à [Préparer le terrain](./setup.md) : le même `demo`, toujours en cours
d'exécution avec sa base, et `curl` de nouveau pour la dernière section.

## 1. Générer la fonctionnalité

```bash
rbs generate crud articles --fields "title:string,body:text,published:bool"
```

{/* rbs:transcript cmd="rbs generate crud articles --fields title:string,body:text,published:bool" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" dans="demo" */}
```text
$ rbs generate crud articles --fields title:string,body:text,published:bool
plan pour …/demo

  + src/articles/mod.rs                                 créé
  + src/articles/model.rs                               créé
  + src/articles/dto.rs                                 créé
  + src/articles/filter.rs                              créé
  + src/articles/repository.rs                          créé
  + src/articles/service.rs                             créé
  + src/articles/controller.rs                          créé
  + src/articles/tests.rs                               créé
  + src/seeds/articles.rs                               créé
  + migration/src/m20260909_092448_create_articles.rs   créé
  ~ src/lib.rs                                          modifié
  ~ src/router.rs                                       modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                                modifié
  ~ src/seeds/main.rs                                   modifié
  ~ Cargo.toml                                          modifié
  ~ AGENTS.md                                           modifié

  17 fichiers à écrire
✓ articles générée — 10 fichiers

  la migration m20260909_092448_create_articles reste à appliquer avant de lancer le projet
```

Dix fichiers écrits sans aucune base de données lancée : la preuve que `--fields` seul a
suffi à décider la forme de l'entité et sa migration à la fois, le schéma déclaré une
seule fois plutôt que relu depuis un serveur. Les lignes `~` ne sont pas des
réécritures — ce sont des insertions à des ancres en commentaire déjà présentes dans des
fichiers qui vous appartiennent, `// <rbs:features>` dans `src/lib.rs` parmi elles.
Supprimez l'une de ces ancres, et le `generate` suivant affiche le bloc qu'il aurait
inséré au lieu de toucher au fichier.

## 2. Appliquer la migration

```bash
rbs migrate up
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.74s
     Running `target/debug/migration up`
✓ migrations appliquées
```

La crate `migration` recompile parce qu'elle vient de gagner un fichier, et une fois
terminé, la table `articles` existe — la preuve que la migration que `generate` vient
d'écrire n'est pas qu'un fichier sur le disque, mais un changement que la base a
désormais appliqué.

## 3. Lancer le serveur

Le serveur de [Préparer le terrain](./setup.md) fait encore tourner l'ancien binaire.
Arrêtez-le et relancez-le :

```bash
cargo run
```

```text
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

Un démarrage propre ici prouve que `src/articles/` compile dans le routeur : `demo`
écoute désormais avec six nouvelles opérations ajoutées à `/health` — `GET` et `POST` sur
`/articles`, `POST` sur `/articles/filter`, et `GET`, `PATCH` et `DELETE` sur
`/articles/{id}` — si le module avait échoué à compiler, cette ligne ne se serait jamais
affichée.

## Vérifier

Depuis le second terminal, resté ouvert depuis Préparer le terrain :

```bash
curl -i -X POST http://127.0.0.1:8080/articles \
  -H 'Content-Type: application/json' \
  -d '{"title":"Premier article","body":"Bonjour","published":true}'
```

```text
HTTP/1.1 201 Created
content-type: application/json
x-request-id: 01M22QWBW5CFQRAPDH41QB2D52
content-length: 191
date: Wed, 09 Sep 2026 09:27:14 GMT

{"id":"01a0857e-2f85-7d03-a129-6defbf74c73c","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-09-09T09:27:14.569998Z","updated_at":"2026-09-09T09:27:14.569998Z"}
```

`id`, `created_at` et `updated_at` ne sont pas dans le corps de la requête — la preuve
que la ligne a été construite et enregistrée par le serveur, et non renvoyée telle
qu'envoyée.

```bash
curl http://127.0.0.1:8080/articles
```

```text
{"data":[{"id":"01a0857e-2f85-7d03-a129-6defbf74c73c","title":"Premier article","body":"Bonjour","published":true,"created_at":"2026-09-09T09:27:14.569998Z","updated_at":"2026-09-09T09:27:14.569998Z"}],"meta":{"page":1,"per_page":20,"total":1,"total_pages":1}}
```

Le même `id` revient sous `data`, avec `meta` décrivant la page sur laquelle il se
trouve — la preuve que l'écriture de la requête précédente et cette lecture s'accordent
sur la même ligne, à travers le même serveur en cours d'exécution.

## Ce qui a été installé

Quatre des sept fichiers que `generate crud` a écrits dans `src/articles/` — tout ce
répertoire à l'exception de `mod.rs` —, chacun lu depuis
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud) —
la même fonctionnalité, engendrée par la même commande.

### L'entité

`Model` est l'entité SeaORM que `--fields` a produite : une structure, une forme de
ligne, `published` mappé directement sur une colonne `bool`.

```rust file=examples/hello-crud/src/articles/model.rs region=entite
```

### L'entrée

`CreateArticle` est ce que `POST /articles` désérialise. `title` porte une contrainte
`max = 255` qu'aucune syntaxe `--fields` n'a énoncée — le défaut propre au générateur
pour un `string` nu.

```rust file=examples/hello-crud/src/articles/dto.rs region=entree
```

### Le gestionnaire

`create` est tout le contrôleur de cette route : il remet l'entrée validée au service et
transforme le résultat en code de statut. Il n'ouvre jamais lui-même de
`DatabaseConnection`.

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

### La requête

`list` et `filter` forment le repository, la seule couche autorisée à construire une
requête SeaORM. `list` est `filter` sans aucune condition — le même chemin que le `GET`
ci-dessus a emprunté.

```rust file=examples/hello-crud/src/articles/repository.rs region=list
```

Une seule direction de dépendance traverse ces quatre fichiers : contrôleur → service →
repository → modèle, chaque couche ne voyant que la suivante.
[Architecture](../architecture.md) place chaque couche que ces sept fichiers occupent,
y compris les trois que cette page n'a jamais ouverts — `service.rs`, `filter.rs` et
`tests.rs`.

## Pour aller plus loin

- [`rbs generate`](../cli/generate.md) couvre les options que cette page n'a pas
  utilisées — `--has-many`, `--soft-delete`, et `--role` pour une écriture que seuls
  certains appelants peuvent faire.
- [Filtrage](../guides/filtering.md) est ce que `filter.rs` monte : une route `POST
  /articles/filter` que cette page n'a jamais appelée.
- [Tests](../guides/testing.md) lit le fichier `tests.rs` que la même commande a écrit,
  et le harnais contre lequel il tourne.
- [Architecture](../architecture.md) place chaque fichier que `rbs generate crud` vient
  d'écrire dans la couche à laquelle il appartient.
- [Fermer l'API aux inconnus](./auth.md) est le tutoriel suivant : fermer `demo` à qui
  il ne connaît pas, avec une écriture que seul un administrateur peut faire sur une
  seconde ressource.
