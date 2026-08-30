---
sidebar_position: 3
title: rbs add
---

# `rbs add`

Installe une feature dans un projet existant. Elle en livre sept : `auth`, `ci`, `jobs`,
`docker`, `mail`, `redis` et `storage`.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

```text
$ rbs add --help
Ajoute une feature : auth, ci, docker, jobs, mail, redis, storage

Usage: rbs add [OPTIONS] <FEATURE>

Arguments:
  <FEATURE>  Feature à installer

Options:
      --force                  Applique les modifications même si le working tree Git est sale
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -y, --yes                    Prend les valeurs par défaut sans rien demander : le CLI reste scriptable
  -h, --help                   Print help
  -V, --version                Print version
```

| Flag | Effet |
|---|---|
| `--force` | Applique même si le working tree Git est sale, et écrase les fichiers signalés en conflit. |
| `--template-dir <CHEMIN>` | Lit les fragments dans un répertoire portant un sous-répertoire par feature, au lieu de ceux embarqués dans le binaire. |
| `-y`, `--yes` | Global, et sans effet ici : `rbs add` ne demande rien. |

## Les sept features

| Feature | Fichiers | Suite |
|---|---|---|
| `docker` | `.dockerignore`, `Dockerfile`, et ses services `api`/`migrate` insérés dans le compose du projet — un `docker-compose.yml` entier s'il n'y en a pas | `docker compose --profile app up --build` |
| `ci` | `.github/workflows/ci.yml` | `git push` |
| `auth` | huit fichiers sous `src/auth/`, une migration, et quatre fichiers du projet modifiés | recopier le secret, puis `rbs migrate up` |
| `jobs` | sept fichiers sous `src/jobs/`, une migration, et une section `[jobs]` de configuration | `rbs migrate up`, puis inscrire vos jobs dans `src/jobs/mod.rs` |
| `redis` | trois fichiers sous `src/cache/`, et un service `redis` inséré dans le compose du projet | le compose le porte déjà — `docker compose up -d` le démarre |
| `mail` | cinq fichiers sous `src/mail/`, un gabarit d'exemple, et un service `mailpit` inséré dans le compose du projet | régler `[mail]` dans `config/default.toml` — un SMTP local par défaut |
| `storage` | quatre fichiers sous `src/storage/` | ignorer `./storage`, ou passer le backend à `s3` |

Les trois dernières sont les briques des guides [cache](../guides/cache.md),
[courriel](../guides/mail.md) et [stockage](../guides/storage.md). Aucune ne monte de
route : elles arrivent sur votre `AppState`, et ce qui les appelle vous revient.

Un projet engendré par `rbs new` porte déjà un `docker-compose.yml` : `docker` écrit
`Dockerfile` et `.dockerignore`, et insère ses deux services — `api`, `migrate` — dans
l'ancre `# <rbs:services>` du compose, sous le profil `app` :

```text
$ rbs add docker
docker : Dockerfile multi-étapes, .dockerignore et services de déploiement

plan pour /private/tmp/rbs-demo/blog

  + Dockerfile           créé
  + .dockerignore        créé
  ~ docker-compose.yml   modifié
  ~ Cargo.toml           modifié
  ~ AGENTS.md            modifié

  5 fichiers à écrire
✓ docker installée — 2 fichiers

  docker compose --profile app up --build
```

`migrate` et `api` portent `profiles: ["app"]` : c'est le profil qui les bâtit et les
démarre. `docker compose up -d` seul — ce que [`rbs dev`](./dev.md) lance — laisse
l'infrastructure tranquille.

Un projet sans compose où insérer — SQLite, ou créé avant rbs 1.1.0 — en reçoit un entier :

```text
$ rbs add docker
docker : Dockerfile multi-étapes, .dockerignore et services de déploiement

plan pour /private/tmp/rbs-demo/depot

  + Dockerfile           créé
  + .dockerignore        créé
  + docker-compose.yml   créé
  ~ Cargo.toml           modifié
  ~ AGENTS.md            modifié

  5 fichiers à écrire
✓ docker installée — 3 fichiers

  docker compose --profile app up --build
```

Un compose réécrit à la main qui a perdu son ancre `# <rbs:services>` n'est pas touché :
la commande n'écrit rien et affiche le bloc à recoller :

```text
$ rbs add docker
erreur : ancre # <rbs:services> introuvable dans docker-compose.yml

dans docker-compose.yml :
# <rbs:services>
# </rbs:services>
```

```text
$ rbs add ci
ci : workflow GitHub Actions : fmt, clippy et tests sur PostgreSQL

plan pour /private/tmp/rbs-demo/blog

  + .github/workflows/ci.yml   créé
  ~ Cargo.toml                 modifié
  ~ AGENTS.md                  modifié

  3 fichiers à écrire
✓ ci installée — 1 fichier

  git push : le workflow s'exécute à la prochaine poussée
```

```text
$ rbs add auth
auth : authentification JWT : Argon2, jetons d'accès et de rafraîchissement, rôles

plan pour /private/tmp/rbs-demo/blog

  + src/auth/mod.rs                                        créé
  + src/auth/model.rs                                      créé
  + src/auth/dto.rs                                        créé
  + src/auth/repository.rs                                 créé
  + src/auth/service.rs                                    créé
  + src/auth/controller.rs                                 créé
  + src/auth/guard.rs                                      créé
  + src/auth/tests.rs                                      créé
  + migration/src/m20260830_110856_create_auth_tables.rs   créé
  ~ migration/src/lib.rs                                   modifié
  ~ src/lib.rs                                             modifié
  ~ src/router.rs                                          modifié
  ~ src/openapi.rs                                         modifié
  ~ Cargo.toml                                             modifié
  ~ config/default.toml                                    modifié
  ~ .env.example                                           modifié
  ~ AGENTS.md                                              modifié

  17 fichiers à écrire
✓ auth installée — 9 fichiers

  recopiez RBS_AUTH__SECRET de .env.example vers votre .env, puis rbs migrate up
```

`auth` est la seule feature dont l'étape suivante n'est pas facultative : le fragment
n'écrit `RBS_AUTH__SECRET` que dans `.env.example`, et un projet dont le `.env` ne la
porte pas refuse de démarrer. Le [guide de l'authentification](../guides/auth.md) prend
la suite.

Dans chaque plan, la ligne `Cargo.toml` est l'endroit où l'installation s'inscrit :

```text
[package.metadata.rbs]
version = "1.0.0"
features = ["health", "docker", "ci", "auth"]
database = "postgres"
```

Tout autre nom est refusé avec la liste de ce qui est installable :

```text
$ rbs add graphql
erreur : `graphql` n'est pas une feature installable : auth, ci, docker, jobs, mail, redis, storage
```

## L'idempotence

Installer ce qui est déjà installé n'est pas un échec. Ce que la commande lit, c'est le
manifeste : une feature inscrite dans `[package.metadata.rbs]` court-circuite avant même
qu'un plan soit dressé.

```text
$ rbs add docker
✓ docker est déjà installée — rien à faire
```

L'idempotence tient à ces métadonnées, non à la présence des fichiers. Retirez la feature
du manifeste et les fichiers sont toujours là — le plan les signale inchangés, et n'écrit
que la ligne de manifeste qui manquait :

```text
$ rbs add docker
docker : Dockerfile multi-étapes, .dockerignore et services de déploiement

plan pour /private/tmp/rbs-demo/blog

  · Dockerfile           inchangé
  · .dockerignore        inchangé
  · docker-compose.yml   inchangé
  ~ Cargo.toml           modifié
  ~ AGENTS.md            modifié

  2 fichiers à écrire, 3 inchangés
✓ docker installée — 2 fichiers

  docker compose --profile app up --build
```

Les marques du plan se lisent : `+` créé, `~` modifié, `·` inchangé, `!` en conflit.

## Un working tree sale

`rbs add` modifie `Cargo.toml` : il refuse donc de passer sur des changements non commités.

```text
$ rbs add ci
erreur : le working tree n'est pas propre : Cargo.toml — commitez, ou relancez avec --force
```

Les fichiers non suivis ne comptent pas : ce sont précisément ceux que la commande
s'apprête à créer. Au-delà de cinq noms, la liste est abrégée. `--force` passe outre, ce que
le message suggère.

## Les conflits

Un fichier qui existe avec un contenu que le fragment ne retrouve pas n'est ni fusionné ni
écrasé en silence. Le plan le marque `!`, et la commande s'arrête :

```text
$ rbs add docker --template-dir /private/tmp/rbs-demo/mes-features
docker : Dockerfile minimal, pour l'exemple

plan pour /private/tmp/rbs-demo/blog

  ! Dockerfile   conflit — relancer avec --force
  ~ Cargo.toml   modifié
  ~ AGENTS.md    modifié

  2 fichiers à écrire, 1 en conflit
erreur : Dockerfile — relancer avec --force pour les écraser
```

`Cargo.toml` porte `~`, pas `·` : le manifeste n'inscrit pas encore `docker`, donc y
écrire la ligne de la feature est un vrai changement — le plan se calcule avant que quoi
que ce soit n'échoue, et c'est le conflit qui empêche de l'appliquer. `--force` écrase,
après avoir montré le même plan :

```text
$ rbs add docker --template-dir /private/tmp/rbs-demo/mes-features --force
docker : Dockerfile minimal, pour l'exemple

plan pour /private/tmp/rbs-demo/blog

  ! Dockerfile   conflit — relancer avec --force
  ~ Cargo.toml   modifié
  ~ AGENTS.md    modifié

  2 fichiers à écrire, 1 en conflit
✓ docker installée — 1 fichier

  docker compose --profile app up --build

$ cat Dockerfile
FROM scratch
```

Si l'application du plan échoue à mi-chemin, ce qui a déjà été écrit est défait : une
installation partielle ne laisse pas une feature à moitié posée.

## Des templates prises du disque

`--template-dir` attend un répertoire portant un sous-répertoire par feature — `docker/`,
`ci/`, ou ce que vous y ajoutez — chacun avec ses templates `.jinja`, le suffixe retiré en
sortie. Il remplace le catalogue embarqué au lieu de s'y ajouter : un répertoire qui ne
porte pas la feature demandée est donc un répertoire où aucune feature n'existe.

```text
$ rbs add docker --template-dir /nexistepas
erreur : `docker` n'est pas une feature installable : aucune n'est disponible
```

C'est aussi pourquoi un catalogue vide est refusé ici plutôt qu'au rendu : il produirait
sinon un plan vide, donc une commande qui réussit sans rien faire.

## Les ancres

`rbs add` écrit surtout des fichiers entiers et modifie le manifeste ; c'est [`rbs
generate`](./generate.md#les-ancres) qui insère dans les neuf ancres en commentaires Rust
du projet — `// <rbs:features>` (dans `src/lib.rs`, ou dans `src/main.rs` sur un projet
sans bibliothèque — voir [plus bas](./generate.md#les-ancres)), `// <rbs:routes>`,
`// <rbs:openapi>`, `// <rbs:migration_modules>`, `// <rbs:migrations>`,
`// <rbs:state_champs>`, `// <rbs:state_init>`, `// <rbs:startup>` et `// <rbs:seeds>`.

`docker` est le seul fragment que `rbs add` installe à faire lui-même exception : ses
services `api` et `migrate` vont dans `# <rbs:services>`, l'ancre YAML que porte un
compose — voir [plus haut](#les-sept-features). La règle est la même partout : aucun AST
n'est jamais réécrit, et une ancre absente fait que la commande n'écrit rien et affiche le
bloc à recoller. [`rbs doctor`](./doctor.md) les contrôle toutes les dix — neuf sur un
projet sans compose pour en porter une dixième.

## Les échecs

Hors d'un projet :

```text
$ rbs add docker
erreur : aucun projet rbs ici : `rbs add` s'exécute dans un projet créé par `rbs new`
```

Chacun de ces cas sort en code 1.
