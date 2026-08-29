---
sidebar_position: 8
title: Cache
---

# Cache

`rbs add redis` installe un cache Redis dans un projet existant : trois fichiers sous
`src/cache/`, une section `[cache]` dans la configuration, et un champ sur votre
`AppState`. Aucune route, aucun middleware — la feature est une brique, et ce que vous
mettez en cache ne regarde que vous.

Tous les extraits de cette page viennent de
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop), un
projet engendré par le CLI et compilé en CI. Rien ici n'est écrit à la main pour la
documentation.

## Ce qui est installé

```text
$ rbs add redis
redis : cache Redis : pool paresseux partagé par l'état, valeurs typées par serde

plan pour /private/tmp/rbs-demo/depot

  + src/cache/mod.rs      créé
  + src/cache/config.rs   créé
  + src/cache/tests.rs    créé
  ~ src/main.rs           modifié
  ~ src/state.rs          modifié
  ~ docker-compose.yml    modifié
  ~ Cargo.toml            modifié
  ~ config/default.toml   modifié

  8 fichiers à écrire
✓ redis installée — 3 fichiers

  un Redis doit écouter à l'URL de la section [cache] de config/default.toml
```

L'indice est déjà satisfait : la ligne `docker-compose.yml` ci-dessus, c'est `redis`
inséré dans le compose du projet, sans profil — `docker compose up -d`, tel quel, celui
que lance [`rbs dev`](../cli/dev.md), le monte avec la base. Rien à démarrer à la main.

La feature s'appelle `redis` et le module s'appelle `cache` : le premier nomme la crate
que déclare le manifeste, le second nomme ce que votre code appelle.
[`rbs doctor`](../cli/doctor.md) la rapporte sous `redis`, le nom que porte le manifeste.

## Configuration

Les défauts vivent dans la feature et non dans le noyau — `rbs-core` n'oppose rien à une
section qu'il ne connaît pas : c'est donc dans ce fichier qu'ils se lisent et se changent.

```rust file=examples/file-drop/src/cache/config.rs
```

L'installation ajoute à `config/default.toml` une section `[cache]` portant ces deux mêmes
valeurs, pour que le réglage soit visible là où tous les autres le sont.
`config/{env}.toml` et les variables `RBS_CACHE__*` la surchargent comme n'importe quelle
autre section — voir le [guide de configuration](./configuration.md). Le mot de passe a sa
place dans l'URL : `redis://:secret@hote:6379/0`.

## La construction, et pourquoi le démarrage reste synchrone

```rust file=examples/file-drop/src/cache/mod.rs region=construction
```

Rien ne se connecte ici. `deadpool` bâtit un pool paresseux qui joint le serveur au premier
appel, ce qui permet à `AppState::new` de rester synchrone — un constructeur qui devrait
`await` une connexion rendrait asynchrone chaque champ de l'état, et un projet ne
démarrerait plus parce qu'un cache est éteint.

La contrepartie est qu'une mauvaise URL ne se signale pas au démarrage mais à la première
requête. Pour le savoir avant vos utilisateurs, demandez à
[`rbs doctor`](../cli/doctor.md).

## Lire et écrire

Les valeurs sont typées et non des chaînes : tout ce qui est `Serialize` entre, tout ce qui
est `DeserializeOwned` sort, avec `serde_json` entre les deux.

```rust file=examples/file-drop/src/cache/mod.rs region=lecture
```

Une clé absente ou expirée rend `Ok(None)`, non une erreur. Cette distinction fait toute
l'ergonomie du type : l'état ordinaire d'un cache est de *ne pas avoir* la valeur, et
l'appelant enchaîne sur sa source de vérité sans avoir à filtrer une variante d'erreur. Une
clé qui porte des octets que serde ne sait pas relire, elle, rend bien une erreur.

`set` applique la durée de vie configurée ; `set_ttl` prend la sienne, et une durée nulle
signifie aucune expiration.

## Invalider par préfixe

L'invalidation d'une clé unique existe, mais un projet qui met en cache une liste paginée
ne sait pas combien de pages il a servies. La feature invalide donc tout un préfixe :

```rust file=examples/file-drop/src/cache/mod.rs region=invalidate_prefix
```

`SCAN` plutôt que `KEYS` : le second parcourt tout l'espace de clés en bloquant le serveur,
ce qui passe inaperçu sur un portable et fait une panne en production.

`SCAN MATCH` prend un glob, interprété à l'autre bout, et un métacaractère présent dans
votre préfixe l'élargirait. Les clés que le serveur rend sont donc filtrées à nouveau ici :

```rust file=examples/file-drop/src/cache/mod.rs region=to_delete
```

Une suppression ne se défait pas — c'est tout l'argument de la revérification du préfixe du
côté où il est sûr.

## Ce que l'exemple en fait

`file-drop` met en cache le total de sa table `uploads`, et non la page :

```rust file=examples/file-drop/src/uploads/service.rs region=list
```

Le choix mérite un instant. `COUNT(*)` parcourt toute la table à chaque appel, quand la
page n'en lit que `per_page` lignes — la moitié coûteuse est donc celle qui vaut d'être
mise en cache. Et `Page` est `Serialize` sans être `Deserialize` : elle se rend, elle ne se
relit pas, si bien que mettre la page en cache exigerait de toucher au noyau.

Chaque écriture invalide, et toutes les trois le font :

```rust file=examples/file-drop/src/uploads/service.rs region=create
```

`update` et `delete` portent la même ligne. N'invalider qu'à la création laisserait un
total périmé derrière chaque modification.

## Les tests

Le `src/cache/tests.rs` engendré se sépare en deux, et la séparation est délibérée.

Quatre tests n'ont besoin d'aucun serveur : l'aller-retour typé par `encode`/`decode`, la
clé absente qui se décode en `None`, le filtre de préfixe et l'échappement du glob. Ce sont
les parties qui peuvent être fausses toutes seules, et `cargo test` les lance.

Trois autres sont `#[ignore]` parce qu'ils joignent le Redis que décrit la section
`[cache]` — la ronde complète, l'expiration, et un préfixe portant un métacaractère.
`cargo test -- --ignored` les lance contre le serveur du projet, et `RBS_CACHE__URL` en
surcharge l'adresse. Voir le [guide des tests](./testing.md).

## Ce qu'elle vous laisse

- **quoi mettre en cache** — la feature ne met rien en cache d'elle-même. Le choix de
  l'exemple, un total plutôt qu'une page, est un exemple et non une règle ;
- **le plan de nommage des clés** — `uploads:` est une convention de l'exemple. Les clés
  sont des chaînes plates, et rien n'impose de schéma ;
- **la ruée sur une clé froide** — une clé absente fait reconstruire la valeur par toutes
  les requêtes concurrentes. Ni verrou ni requête unique ici ;
- **le cache en panne** — un Redis qui cesse de répondre transforme les lectures en
  erreurs, non en absences. Si vous préférez dégrader que rompre, cette décision revient à
  votre service, là où se trouve la source de vérité.

Le code est dans votre arborescence, sans bandeau vous disant de ne pas y toucher.
