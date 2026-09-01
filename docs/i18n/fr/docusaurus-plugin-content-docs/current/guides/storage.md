---
sidebar_position: 10
title: Stockage
---

# Stockage

`rbs add storage` installe un stockage d'objets dans un projet existant : quatre fichiers
sous `src/storage/`, et un `Arc<dyn Storage>` sur votre `AppState`. Deux backends
l'accompagnent — le système de fichiers local et S3 — et tout l'intérêt de la feature est
que votre code ne puisse pas dire auquel des deux il parle.

Tous les extraits de cette page viennent de
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop), un
projet engendré par le CLI et compilé en CI. Rien ici n'est écrit à la main pour la
documentation.

## Ce qui est installé

```text
$ rbs add storage
storage : stockage d'objets : un trait à cinq méthodes, deux backends — fichiers et S3

plan pour /private/tmp/rbs-demo/depot

  + src/storage/mod.rs         créé
  + src/storage/files.rs       créé
  + src/storage/s3.rs          créé
  + src/storage/tests.rs       créé
  ~ src/lib.rs                 modifié
  ~ src/state.rs               modifié
  ~ src/health/controller.rs   modifié
  ~ Cargo.toml                 modifié
  ~ config/default.toml        modifié
  ~ .env.example               modifié
  ~ AGENTS.md                  modifié

  11 fichiers à écrire
✓ storage installée — 4 fichiers

  les objets vont sous ./storage : ajoutez-le à .gitignore, ou passez storage.backend à "s3" et recopiez les RBS_STORAGE__* de .env.example
```

L'étape suivante n'est pas décorative : avec le backend `fs` par défaut, les objets déposés
atterrissent dans `./storage`, au cœur de votre arborescence, et `git status` les montrera.

## Cinq méthodes

```rust file=examples/file-drop/src/storage/mod.rs region=trait
```

Voilà tout le contrat. Délibérément absents : le listage, la copie, les URL signées, les
métadonnées, le flux. Quatre méthodes, c'est ce que deux backends peuvent honorer à
l'identique, et l'abstraction ne vaut ni plus ni moins que cette identité. La cinquième ne
transporte rien et ne répond qu'à `GET /health` : c'est elle qui empêche la route de dire
`ok` sur un stockage que votre projet ne joint plus.

Les échecs tiennent en une énumération, ce qui permet à l'appelant de distinguer une erreur
du client d'une panne :

```rust file=examples/file-drop/src/storage/mod.rs region=erreurs
```

`NotFound` est la seule variante qui vienne de l'appelant. `delete` ne la lève pas — une clé
absente n'y est pas une erreur, car le `DeleteObject` de S3 réussit sur une clé qu'il ne
trouve pas, et deux backends qui divergeraient là-dessus ne seraient pas substituables.

## Choisir un backend

```rust file=examples/file-drop/src/storage/mod.rs region=build
```

Le message d'erreur énonce les valeurs admises plutôt que de s'en remettre à une
énumération serde, et le backend inconnu est refusé plutôt que replié silencieusement sur
`fs` — un projet qui croit écrire dans S3 et écrit sur un disque local est un projet qui
perd ses données au déploiement suivant.

`AppState` porte un `Arc<dyn Storage>` et non un paramètre générique. Le rendre générique
aurait propagé un paramètre de type dans la signature de chaque handler du projet.

Le backend local est assez court pour être lu en entier :

```rust file=examples/file-drop/src/storage/files.rs
```

Le backend S3 est `src/storage/s3.rs`. Sa seule décision qui mérite d'être connue : les
identifiants viennent de la configuration, non de la chaîne de fournisseurs par défaut du
SDK. Cette chaîne est asynchrone et interroge le service de métadonnées de l'instance, ce
qu'un `AppState::new` synchrone ne peut ni lancer ni attendre — `aws-config` n'est donc pas
une dépendance ici, et `Credentials`, `Region` et `BehaviorVersion` sont pris dans
`aws_sdk_s3::config`. `force_path_style` met le bucket dans le chemin plutôt que dans le
sous-domaine, ce qu'attend MinIO.

## Les clés, et celle qui cherche à s'échapper

Un nom d'objet vient souvent d'un utilisateur : il n'est donc pas remis tel quel au système
de fichiers.

```rust file=examples/file-drop/src/storage/mod.rs region=normalize
```

La clé est parcourue composant par composant et refusée dès qu'un `..` remonte au-dessus de
la racine. Une recherche de sous-chaîne sur `..` serait à la fois trop stricte et trop
laxiste : elle refuserait l'inoffensif `sous/../recu.txt`, et laisserait passer `a/../../b`
sur certaines entrées. Ce qui est refusé, c'est l'évasion, non la présence d'un `..`.

`normalize` sert **les deux** backends — une clé S3 n'est pas plus sûre qu'un chemin — ce
qui est la deuxième condition de leur substituabilité, après un `delete` idempotent.

## Configuration

`backend` vaut `fs` ou `s3` ; `root` est le répertoire du premier, `./storage` par défaut.
Le reste ne concerne que `s3` : `bucket`, `region`, `endpoint` pour toute API compatible S3
autre qu'AWS, `force_path_style`, et les deux identifiants.

Ces identifiants ont leur place dans l'environnement, non dans le `config/default.toml`
versionné : `rbs add storage` écrit `RBS_STORAGE__BUCKET`, `RBS_STORAGE__ACCESS_KEY_ID` et
`RBS_STORAGE__SECRET_ACCESS_KEY` dans `.env.example`. [`rbs doctor`](../cli/doctor.md)
contrôle le couple qui met réellement un déploiement en défaut — un `backend = "s3"` dont
aucun bucket n'est nommé nulle part — et ne dit rien de tout cela tant que le backend est
`fs`, qui n'en a besoin d'aucun.

## Ce que l'exemple en fait

`file-drop` donne à sa ressource `uploads` un point d'entrée de contenu. Le service tient la
table et le magasin au pas :

```rust file=examples/file-drop/src/uploads/service.rs region=contenu
```

Deux choix y sont visibles. La ligne est lue **avant** le dépôt, pour que le magasin
n'accumule pas des objets qu'aucune ressource ne réclame. Et c'est `exists` qui répond à la
requête HEAD, plutôt qu'un `get` dont on jetterait le corps — la question n'exige pas de
transférer l'objet, et les deux backends savent y répondre sans le lire.

La suppression va dans l'autre sens : le contenu part avec la ligne, et `delete` étant
idempotent des deux côtés, une ressource créée sans contenu ne fait pas échouer sa
suppression.

Le contenu voyage hors du DTO :

```rust file=examples/file-drop/src/uploads/controller.rs region=put_content
```

Un corps binaire n'a pas sa place dans un document JSON, et le base64 obligerait à charger
le fichier deux fois en mémoire. Les trois verbes sont montés ensemble :

```rust file=examples/file-drop/src/uploads/mod.rs region=route_contenu
```

## Les tests

Le `src/storage/tests.rs` engendré est bâti autour d'une seule fonction, `round`, qui
éprouve ce que le trait promet — déposer, lire, attester, supprimer — contre un
`&dyn Storage` plutôt que contre un type concret.

C'est la conception même du fichier. `cargo test` joue la ronde contre le backend fichiers,
avec un test de traversée qui éprouve quatre clés fuyantes et assertent à la fois la
variante `RejectedKey` *et* l'absence de fichiers témoins hors de la racine ; il bâtit aussi
un client S3 sans toucher au réseau, et vérifie qu'un backend inconnu est refusé en le
nommant.

Deux tests `#[ignore]` joignent le service de la section `[storage]` — MinIO en
développement. Le premier rejoue **la même** `round`, appelée sans une ligne de différence :
une suite réécrite pour S3 prouverait que S3 marche, jamais que le trait abstrait. Le
second relit un objet par son propre client `aws_sdk_s3` plutôt que par `S3Storage`, dont le
champ est privé, afin que ce qui est réellement arrivé dans le bucket soit observé et non
supposé. Voir le [guide des tests](./testing.md).

## Ce qu'elle vous laisse

- **lister les objets** — absent du trait. Un magasin qu'il faut parcourir réclame de toute
  façon un index dans votre base, ce qu'est la table de l'exemple ;
- **les URL signées** — servir un objet privé veut dire le relayer par votre handler, comme
  le fait l'exemple. Les liens pré-signés sont une fonction de S3 que le trait n'expose
  pas ;
- **les limites de taille** — rien ne borne un corps. C'est à `DefaultBodyLimit` d'Axum que
  cela revient ;
- **les types de contenu** — le magasin ne tient que des octets. L'exemple garde
  `content_type` dans sa table parce que le magasin ne s'en souviendra pas ;
- **le cycle de vie, le versionnage, le chiffrement au repos** — se règlent sur le bucket,
  non ici.

Le code est dans votre arborescence, sans bandeau vous disant de ne pas y toucher.
