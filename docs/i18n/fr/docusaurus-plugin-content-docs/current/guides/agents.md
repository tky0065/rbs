---
sidebar_position: 13
title: AGENTS.md
---

# AGENTS.md

Un agent lâché dans un projet rbs n'a aucun moyen de savoir que rbs existe. Il voit des
fichiers Rust, il écrit des fichiers Rust : il recrée à la main les six fichiers d'une
feature, oublie la migration, ignore les ancres, et casse la dépendance unidirectionnelle
sur laquelle repose l'architecture. Le CLI est juste là, et rien ne dit à l'agent de s'en
servir.

`rbs new` répond à cela en écrivant `AGENTS.md` à la racine du projet — le mode d'emploi
de rbs, écrit pour un agent plutôt que pour un humain. `AGENTS.md` est un format neutre,
déjà lu tel quel par Claude Code, Codex, Cursor et Copilot ; rbs n'engendre aucun fichier
propre à un outil en particulier.

## Les deux zones que rbs possède

Seules deux parties d'`AGENTS.md` appartiennent à rbs, chacune délimitée par un
commentaire HTML :

```text
# <projet> — mode d'emploi pour agents

<!-- rbs:guide 1.1.0 -->
… le mode d'emploi …
<!-- /rbs:guide -->

<!-- rbs:inventory -->
… l'état du projet …
<!-- /rbs:inventory -->

## Notes du projet
```

`rbs:guide` est le mode d'emploi proprement dit — la règle du CLI d'abord, le tableau des
commandes, des recettes, l'architecture imposée, la liste des ancres, ce que rbs ne
couvre pas, et les commandes à lancer avant de conclure. Son marqueur d'ouverture porte la
version du CLI qui l'a écrit, et c'est ce numéro que [`rbs upgrade`](../cli/upgrade.md)
compare et réécrit.

`rbs:inventory` est l'état du projet lui-même, recalculé en entier à chaque écriture : la
version de rbs et le moteur de base, les fragments installés, les entités engendrées, et
les ancres que le projet porte réellement. Il reste court et factuel exprès, pour qu'un
agent n'ait pas à explorer l'arborescence pour savoir ce qu'elle contient déjà.

**Tout ce qui vit hors de ces deux zones vous appartient, et rbs ne le réécrit jamais** —
ni le titre, ni la section `## Notes du projet` que `rbs new` laisse vide, ni un titre que
vous ajoutez de votre côté. C'est la même règle que pour le code que rbs engendre : ce
fichier est fait pour être modifié, et les marqueurs sont la seule promesse que rbs fait
sur ce qu'il touchera.

Les zones d'un vrai projet, engendré en français, se lisent ainsi :

```text
# blog — mode d'emploi pour agents

<!-- rbs:guide 1.1.0 -->
## Le CLI d'abord
## Les commandes
## Recettes
## Architecture imposée
## Les ancres
## Ce que rbs ne couvre pas
## Vérifier avant de conclure
<!-- /rbs:guide -->

<!-- rbs:inventory -->
- rbs 1.1.0 · base postgres
- Fragments installés : aucun
- Entités engendrées : aucune
- Ancres du projet : features (src/lib.rs), routes (src/router.rs), openapi (src/openapi.rs), migration_modules (migration/src/lib.rs), migrations (migration/src/lib.rs), state_champs (src/state.rs), state_init (src/state.rs), startup (src/main.rs), seeds (src/seeds/main.rs), services (docker-compose.yml)
<!-- /rbs:inventory -->

## Notes du projet
```

## Qui écrit quoi

| Commande | Effet sur `AGENTS.md` |
|---|---|
| `rbs new` | Écrit le fichier entier : guide, inventaire, titre et une section de notes vide. |
| `rbs add <feature>` | Régénère la zone d'inventaire. |
| `rbs generate crud\|feature` | Régénère la zone d'inventaire. |
| `rbs upgrade` | Régénère le guide et l'inventaire ; recrée le fichier s'il a disparu. |
| `rbs doctor` | Ne change rien — il ne fait que constater. |
| `rbs migrate`, `rbs seed`, `rbs dev` | Aucun effet. |

`upgrade` est la seule commande qui a mandat de remettre le projet en accord avec le CLI,
et c'est pourquoi elle est aussi la seule à recréer un fichier supprimé. `add` et
`generate` ne régénèrent que l'inventaire : elles connaissent la feature ou l'entité
qu'elles viennent d'installer, pas si le CLI lui-même a changé de version — cette
comparaison n'appartient qu'à `upgrade`.

## Choisir la langue

`rbs new --lang fr|en` choisit la langue dans laquelle le mode d'emploi est écrit. Sans le
flag, rbs se rabat sur l'environnement : `LC_ALL` d'abord, puis `LANG` — une valeur qui
commence par `fr` donne le français, toute autre valeur non vide donne l'anglais, et
l'absence de valeur donne le français, la langue du dépôt rbs lui-même.

Le choix s'inscrit dans le manifeste plutôt que d'être redéduit à chaque commande :

```toml
[package.metadata.rbs]
lang = "en"
```

Sans cette clé, [`rbs add`](../cli/add.md) et [`rbs upgrade`](../cli/upgrade.md) devraient
deviner la langue du projet depuis l'environnement de celui qui les lance — réécrivant un
guide anglais en français le jour où quelqu'un de l'équipe lance la commande depuis une
locale française. La lire depuis le manifeste fait au contraire que le fichier reste dans
la langue où le projet a été créé, indépendamment de qui le touche ensuite.

## Ce que vérifie `rbs doctor`

[`rbs doctor`](../cli/doctor.md) lance un contrôle `agents` parmi les autres :

| Ce qu'il constate | Verdict |
|---|---|
| `AGENTS.md` absent | échec — `rbs upgrade` le recrée |
| La zone `rbs:guide` ou `rbs:inventory` manque | échec — le bloc à coller s'affiche |
| Le guide est d'une version différente de celle du CLI | échec — `rbs upgrade` réécrit le guide |
| L'inventaire rendu diffère de celui du disque | échec — `rbs upgrade` le recalcule |
| Une feature déclarée dans le manifeste sans son `src/<nom>/` | échec — `rbs add <nom>`, ou retirez la ligne du manifeste |
| Un répertoire de `src/` qu'aucun fragment ni aucune feature déclarée n'explique | **avertissement** |

Cette dernière ligne rend vérifiable la règle du CLI d'abord : un répertoire que rien dans
le manifeste n'explique est du code que personne n'a engendré. Il reste un avertissement
plutôt qu'un échec, et c'est voulu — écrire à la main ce que rbs ne couvre pas est
légitime et prévu, c'est même tout l'objet de la section « ce que rbs ne couvre pas » du
guide. En faire un échec rendrait `rbs doctor` rouge sur un projet parfaitement sain dès
qu'on ajoute à la main un gestionnaire de webhook ou un client HTTP externe — exactement
le genre de code que cet outil n'a pas vocation à engendrer.

Un avertissement ne change ni le code de sortie ni le verdict final : un projet qui ne
porte qu'un avertissement continue de sortir en 0 et d'être rapporté comme sain dans
l'ensemble — seul un échec véritable change cela.

```text
$ rbs doctor
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
     Running `target/debug/migration version`
  ✓ ancres      les 10 points d'insertion sont en place
  ! agents      écrit hors du CLI : webhooks
      légitime si rbs ne couvre pas ce code ; sinon, rbs generate le reprend
  ✓ relations   les modèles portent leurs ancres de relation
  ✓ .env        les 4 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core pris d'un chemin local alignés sur le CLI 1.1.0
  ✓ base        postgres 18.6 répond sur localhost:55502
✓ le projet est sain
```

## Quand une zone a disparu

Supprimer un marqueur est traité comme la suppression d'une ancre de code : la commande
qui aurait dû y écrire n'écrit rien, et affiche à la place le bloc exact à coller.

```text
$ rbs add redis
[…]
attention : AGENTS.md ne porte pas la zone `rbs:inventory` — collez ce bloc pour la rétablir :

<!-- rbs:inventory -->
<!-- /rbs:inventory -->
✓ redis installée — 3 fichiers
```

Le reste de la commande va tout de même à son terme — une zone manquante dans un fichier
de documentation n'est jamais une raison de refuser d'installer une feature. Collez le
bloc, et la prochaine commande qui touche `AGENTS.md` le remplira à nouveau.

Supprimer le fichier entier va plus loin encore : `rbs add` et `rbs generate` aboutissent
sans même le mentionner. La seule commande qui le repose est
[`rbs upgrade`](../cli/upgrade.md), puisque remettre le projet en accord avec le CLI
courant est précisément son rôle :

```text
$ rbs upgrade
rbs 1.1.0 → 1.1.0

plan pour /private/tmp/rbs-demo/blog2

  · Cargo.toml   inchangé
  + AGENTS.md    créé

  1 fichier à écrire, 1 inchangé
✓ manifeste aligné sur rbs 1.1.0
```
