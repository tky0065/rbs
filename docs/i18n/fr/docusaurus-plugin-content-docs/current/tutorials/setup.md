---
sidebar_position: 1
title: Préparer le terrain
---

# Préparer le terrain

C'est le premier des neuf tutoriels, et celui que chaque page suivante suppose déjà
lu : il mène d'un répertoire vide à un projet nommé `demo`, en cours d'exécution, avec
un contrôle de santé qui répond sur `localhost:8080`. Chaque tutoriel qui suit — CRUD,
authentification, stockage, mail, cache, tâches de fond, observabilité, client
TypeScript généré — reprend le même `demo` exactement à ce point : le projet que vous
construisez ici est celui que vous garderez jusqu'à la fin de la série.

Si le CLI n'est pas encore installé, [Démarrage rapide](../getting-started.md) couvre
`cargo install rbs-cli` et ce que doit répondre `rbs --version`, avec la note sur le
`rbs` de Ruby qui partage le même nom. Cette page suppose cette étape faite.

## Ce qu'il vous faut

Les mêmes qu'au démarrage rapide : **Rust stable**, édition 2024, et **Docker avec
Compose** — `rbs new` écrit le `docker-compose.yml` que cette page démarre quelques
étapes plus bas. **PostgreSQL 14 ou plus récent**, quelle que soit la façon de
l'exécuter, et **curl** pour la dernière section.

## 1. Créer le projet

```bash
rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
```

{/* rbs:transcript cmd="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" */}
```text
$ rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
✓ demo créé — 23 fichiers

  cd demo
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

Cette ligne est un succès, pas une erreur — [Démarrage rapide](../getting-started.md)
explique pourquoi le CLI répond en français quelle que soit votre locale. Vingt-trois
fichiers, une seule commande : la preuve que le projet existe avec une fonctionnalité
`health` déjà câblée, et rien d'autre, puisque `--yes` a pris tous les défauts.
[Architecture](../architecture.md) détaille le rôle de chacun de ces fichiers ; cette
série ne répétera pas cette carte.

## 2. Démarrer la base

```bash
cd demo
docker compose up -d --wait
```

{/* rbs:libre raison="sortie de docker compose, qui démarre un conteneur : hors de portée du rejeu" */}
```text
 Container demo-db-1  Started
 Container demo-db-1  Waiting
 Container demo-db-1  Healthy
```

`Healthy` est le contrôle de santé propre à Compose qui passe, celui que `--wait`
attend : la preuve que le conteneur PostgreSQL accepte déjà des connexions, pas
seulement qu'il a démarré. C'est ce qui rend la commande suivante sûre à lancer
immédiatement.

## 3. Appliquer les migrations

```bash
rbs migrate up
```

```text
✓ migrations appliquées
```

Rien n'a encore changé de forme — la commande n'a créé que la table que SeaORM utilise
pour suivre les migrations appliquées. Cette ligne est la preuve que l'URL dans `.env`
est la bonne, avant que quoi que ce soit d'autre dans le projet n'en dépende.

## 4. Lancer le serveur

```bash
cargo run
```

{/* rbs:libre raison="journal d'un serveur qui tourne : le rejeu ne lance aucun serveur, et l'heure change à chaque démarrage" */}
```text
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

Un démarrage propre ici prouve que tout l'arbre d'Axum, SeaORM et utoipa compile
ensemble — si quoi que ce soit y était mal câblé, cette ligne ne se serait jamais
affichée. C'est aussi pour cela que c'est la commande la plus lente de cette page. Une
fois cette ligne affichée, `demo` écoute, et le terminal qui l'a lancée est désormais
celui du serveur — laissez-le tourner.

## Vérifier

Depuis un second terminal :

```bash
curl -i http://127.0.0.1:8080/health
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
HTTP/1.1 200 OK
content-type: application/json

{"status":"ok","checks":{"database":"ok"}}
```

Un `200` avec `"database":"ok"` prouve deux choses à la fois : le serveur répond, et il
joint le conteneur PostgreSQL démarré à l'étape 2. Chaque tutoriel de la série reprend
exactement à cet état.

## Pour aller plus loin

- [Démarrage rapide](../getting-started.md) poursuit les mêmes commandes plus loin, en
  générant une fonctionnalité CRUD et en lisant son document OpenAPI.
- [Architecture](../architecture.md) place chaque fichier que `rbs new` vient d'écrire
  dans la couche à laquelle il appartient.
- [`rbs new`](../cli/new.md) couvre les options que cette page n'a pas utilisées — une
  base de données existante, ou un projet sans serveur à démarrer.
- [`rbs migrate`](../cli/migrate.md) couvre `down` et `status`, les deux commandes que
  cette page n'a pas encore eu besoin d'employer.
- [Votre première ressource](./first-resource.md) est le tutoriel suivant :
  `rbs generate crud` transforme une déclaration `--fields` en entité, sa migration, et
  chaque couche entre les deux.
