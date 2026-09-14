---
sidebar_position: 6
title: Ne pas recalculer deux fois
---

# Ne pas recalculer deux fois

C'est le sixième des neuf tutoriels. Il reprend `demo` juste après [Recevoir un
fichier](./storage.md) — en cours d'exécution, avec la ressource `uploads` de cette page.
Le cas : un `COUNT(*)` lu mille fois par minute, que trois écritures rendent périmé dès
qu'il change.

## Ce qu'il vous faut

Rien de plus qu'à [Recevoir un fichier](./storage.md) : le même `demo` en cours
d'exécution, avec `uploads` monté.

## 1. Installer la fonctionnalité

```bash
rbs add redis
```

{/* rbs:transcript cmd="rbs add redis" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add redis
redis : cache Redis : pool paresseux partagé par l'état, valeurs typées par serde

plan pour …/demo

  + src/modules/cache/mod.rs      créé
  + src/modules/cache/config.rs   créé
  + src/modules/cache/tests.rs    créé
  + src/modules/mod.rs            créé
  ~ src/lib.rs                    modifié
  ~ src/state.rs                  modifié
  ~ src/health/controller.rs      modifié
  ~ docker-compose.yml            modifié
  ~ Cargo.toml                    modifié
  ~ config/default.toml           modifié
  ~ AGENTS.md                     modifié

  4 à créer, 7 à modifier
✓ redis installée — 4 créés, 7 modifiés

  le compose du projet porte déjà un service redis — docker compose up -d le démarre ; sans compose, faites écouter un Redis à l'URL de [cache] de config/default.toml
```

Le nom mérite d'être signalé, c'est un piège : la commande s'appelle `redis`, et tout ce
qu'elle écrit se nomme `cache` — `src/modules/cache/`, `mod cache;`, `state.cache()`.
Seuls la dépendance Cargo et cette commande gardent le nom `redis` ; votre propre code ne
l'écrit jamais. Comme `storage` et `mail` avant elle, la brique ne monte aucune route : ce
que vous obtenez, c'est un `Cache` sur `AppState`, et décider quoi y ranger vous revient
entièrement. `add storage` et `add mail` ne sont pas relancées ici ;
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop) porte
les deux à côté de `redis`, câblées dans le service unique que cette page lit plus bas.

## Vérifier

`generate crud` n'a aucune raison de savoir qu'une brique de cache existe, donc rien ne
câble `Cache` dans le service d'`uploads` à votre place — les appels ci-dessous sont du
code de `file-drop`, lu plus bas sur cette page. Ce que `cargo test` peut vérifier sans
Redis en vie, c'est la part sans réseau : une invalidation par préfixe retire-t-elle
exactement les clés sous ce préfixe, et rien qui commence seulement de la même façon ?

```bash
cargo test modules::cache::
```

```text
running 7 tests
test modules::cache::tests::a_missing_key_returns_none_and_not_an_error ... ok
test modules::cache::tests::a_prefix_carrying_a_glob_metacharacter_is_escaped ... ok
test modules::cache::tests::invalidate_prefix_only_removes_the_keys_of_the_targeted_prefix ... ok
test modules::cache::tests::a_cached_value_reads_back_deserialized ... ok
test modules::cache::tests::a_prefix_with_a_metacharacter_only_removes_what_it_designates ... ignored, joint le Redis de la section [cache]
test modules::cache::tests::a_value_with_a_one_second_ttl_is_gone_after_the_wait ... ignored, joint le Redis de la section [cache]
test modules::cache::tests::the_full_run_plays_against_a_server ... ignored, joint le Redis de la section [cache]

test result: ok. 4 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

La preuve de la seule propriété qui ne demande aucun serveur :
`invalidate_prefix_only_removes_the_keys_of_the_targeted_prefix` donne quatre clés à la
fonction — deux sous le préfixe, deux qui ne font que lui ressembler — et vérifie que
seules les deux premières sont retenues pour la suppression. Les trois tests ignorés sont
ceux qui parlent réellement à Redis ; `docker compose up -d` démarre l'instance du
projet, et c'est `cargo test -- --ignored` qui les atteint.

## Ce qui a été installé

Trois fichiers, lus depuis
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop) — la
seule commande `add redis` ci-dessus, lancée sur un projet compilé en CI.

:::note
`file-drop` porte les trois briques v0.3 à la fois — `storage`, `mail` et `redis` — sur
un même projet, si bien que son `src/uploads/service.rs` dépose aussi du contenu et
envoie des mails, ce que cette page n'a installé ni l'un ni l'autre. L'extrait `list`
ci-dessous est la part de ce fichier que le cache seul explique.
:::

### Lire et écrire

```rust file=examples/file-drop/src/modules/cache/mod.rs region=lecture
```

Une clé absente ou expirée répond `Ok(None)`, pas une erreur — l'état ordinaire d'un
cache est de *ne pas avoir* la valeur, et l'appelant retombe sur sa source de vérité sans
inspecter un genre d'erreur.

### Invalider par préfixe

```rust file=examples/file-drop/src/modules/cache/mod.rs region=invalidate_prefix
```

`SCAN` plutôt que `KEYS`, de sorte que le serveur ne soit jamais bloqué à parcourir tout
son espace de clés pour répondre à une seule invalidation.

### Ce que l'exemple met en cache

`file-drop` met en cache le **total**, pas la page :

```rust file=examples/file-drop/src/uploads/service.rs region=list
```

Ce choix est le cœur de cette page. `COUNT(*)` parcourt toute la table à chaque appel,
quand la page elle-même ne lit que `per_page` lignes — c'est la moitié coûteuse qui
mérite d'être mise en cache. Et `Page` est `Serialize` mais pas `Deserialize` : elle se
rend, mais la relire du cache demanderait de la rendre désérialisable dans `rbs-core`, un
coût que le noyau ne porte pas pour un choix qu'un seul projet a fait. Chaque écriture
défait le compte qu'elle invalide — `create`, `update` et `delete` appellent tous
`invalidate_prefix`, si bien qu'un total périmé ne survit jamais à l'écriture qui l'a
rendu périmé.

## Pour aller plus loin

- [Cache](../guides/cache.md) couvre la construction, pourquoi elle reste synchrone, et
  ce que la feature vous laisse faire — la protection contre l'emballement en fait
  partie.
- [`rbs add`](../cli/add.md) couvre les onze autres features que `demo` pourrait encore
  installer, `storage` et `redis` désormais toutes deux sur lui.
- [Tests](../guides/testing.md) est le harnais contre lequel se scinde le `cache/tests.rs`
  engendré — quatre tests sans serveur, trois qui en demandent un.
- [Sortir le travail long de la requête](./jobs.md) est le tutoriel suivant : une
  campagne de 5 000 lettres, enfilées sans faire attendre l'appelant qu'une seule parte.
