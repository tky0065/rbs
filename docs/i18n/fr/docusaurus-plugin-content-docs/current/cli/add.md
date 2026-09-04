---
sidebar_position: 3
title: rbs add
---

# `rbs add`

Installe une feature dans un projet existant. Elle en livre onze : `audit`, `auth`,
`ci`, `cors`, `docker`, `jobs`, `mail`, `observability`, `rate-limit`, `redis` et
`storage`.
Installe une feature dans un projet existant. Elle en livre onze : `auth`, `ci`, `cors`,
`docker`, `jobs`, `mail`, `observability`, `rate-limit`, `redis`, `scheduler` et
`storage`.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

```text
$ rbs add --help
Ajoute une feature : audit, auth, ci, cors, docker, jobs, mail, observability, rate-limit, redis, scheduler, storage

Usage: rbs add [OPTIONS] <FEATURE>

Arguments:
  <FEATURE>  Feature à installer

Options:
      --force                  Applique les modifications même si le working tree Git est sale
      --dry-run                Affiche le plan sans rien écrire
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -h, --help                   Print help
  -V, --version                Print version
```

| Flag | Effet |
|---|---|
| `--force` | Applique même si le working tree Git est sale, et écrase les fichiers signalés en conflit. |
| `--dry-run` | Affiche le plan et s'arrête. Rien n'est écrit. |
| `--template-dir <CHEMIN>` | Lit les fragments dans un répertoire portant un sous-répertoire par feature, au lieu de ceux embarqués dans le binaire. |

## Les douze features

| Feature | Fichiers | Suite |
|---|---|---|
| `docker` | `.dockerignore`, `Dockerfile`, et ses services `api`/`migrate` insérés dans le compose du projet — un `docker-compose.yml` entier s'il n'y en a pas | `docker compose --profile app up --build` |
| `ci` | `.github/workflows/ci.yml` | `git push` |
| `auth` | huit fichiers sous `src/auth/`, une migration, quatre fichiers du projet modifiés — et `rate-limit`, qu'elle exige | `rbs migrate up` |
| `jobs` | sept fichiers sous `src/jobs/`, une migration, et une section `[jobs]` de configuration | `rbs migrate up`, puis inscrire vos jobs dans `src/jobs/mod.rs` |
| `scheduler` | six fichiers sous `src/scheduler/`, une migration, une section `[scheduler]`, un ticker dans `// <rbs:startup>` — et `jobs`, qu'elle exige | `rbs migrate up`, puis déclarer vos échéances dans `src/scheduler/mod.rs` |
| `redis` | trois fichiers sous `src/cache/`, et un service `redis` inséré dans le compose du projet | le compose le porte déjà — `docker compose up -d` le démarre |
| `mail` | cinq fichiers sous `src/mail/`, un gabarit d'exemple, et un service `mailpit` inséré dans le compose du projet | régler `[mail]` dans `config/default.toml` — un SMTP local par défaut |
| `storage` | quatre fichiers sous `src/storage/` | ignorer `./storage`, ou passer le backend à `s3` |
| `cors` | trois fichiers sous `src/cors/`, une section `[cors]` de configuration, et une couche dans `// <rbs:layers>` | énumérer vos origines dans `[cors]` — vide, donc rien d'origine croisée ne passe |
| `rate-limit` | quatre fichiers sous `src/rate_limit/`, une section `[rate_limit]`, un champ sur `AppState`, et une couche dans `// <rbs:layers>` | derrière un reverse proxy, régler `rate_limit.trust_forwarded_for` |
| `observability` | quatre fichiers sous `src/observability/`, une section `[observability]`, une couche dans `// <rbs:layers>`, et un second listener dans `// <rbs:startup>` | nommer un collecteur dans `OTEL_EXPORTER_OTLP_ENDPOINT` — sans lui rien n'est exporté |
| `audit` | quatre fichiers sous `src/audit/`, et une migration | `rbs migrate up`, puis appeler `audit::record` dans vos services — l'entrée s'écrit dans la transaction du changement |

`cors`, `rate-limit` et `observability` sont les trois qui empilent un middleware au lieu
de monter une route : leur couche va dans `// <rbs:layers>`, à l'intérieur de `trace` et
de `request_id` — voir [Les ancres](#les-ancres).

Les trois qui les précèdent sont les briques des guides [cache](../guides/cache.md),
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
  · .env.example         inchangé
  · .env                 inchangé
  ~ Cargo.toml           modifié
  ~ AGENTS.md            modifié

  5 fichiers à écrire, 2 inchangés
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
  ~ .env.example         modifié
  ~ .env                 modifié
  ~ Cargo.toml           modifié
  ~ AGENTS.md            modifié

  7 fichiers à écrire
✓ docker installée — 3 fichiers

  docker compose --profile app up --build
```

Le compose est versionné : il nomme les identifiants de la base au lieu de les porter —
`POSTGRES_USER`, `POSTGRES_PASSWORD` et `POSTGRES_DB`, ou leurs équivalents MySQL, que
Compose interpole depuis `.env` au démarrage du service. Un projet créé avant rbs 1.1.0
n'en déclare aucun : `add docker` les écrit donc dans son `.env`, avec les valeurs de
l'URL de base du projet, et donne à `.env.example` les mêmes clés sur les valeurs de
démonstration du moteur. Une clé déjà déclarée est laissée telle quelle — d'où les deux
lignes `inchangé` du premier plan ci-dessus, sur un projet que `rbs new` a déjà équipé.

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
$ rbs add cors
cors : CORS : origines, méthodes et en-têtes autorisés, énumérés par la configuration

plan pour /private/tmp/rbs-demo/blog

  + src/cors/mod.rs       créé
  + src/cors/config.rs    créé
  + src/cors/tests.rs     créé
  ~ src/lib.rs            modifié
  ~ src/router.rs         modifié
  ~ Cargo.toml            modifié
  ~ config/default.toml   modifié
  ~ AGENTS.md             modifié

  8 fichiers à écrire
✓ cors installée — 3 fichiers

  énumérez vos origines dans [cors] de config/default.toml — la liste est vide, donc aucune requête d'origine croisée ne passe
```

La section `[cors]` qu'elle écrit commence par `origins = []` : rien d'origine croisée
n'est autorisé tant que le projet n'a pas nommé ses clients. Y inscrire `"*"` ouvre l'API
à toutes les origines, et est refusé avec `credentials = true` — un navigateur ignore les
identifiants envoyés à une origine joker, et croire le contraire est le vrai danger. Une
section illisible rend une couche qui n'autorise rien, et le journal dit pourquoi.

```text
$ rbs add rate-limit
rate-limit : limite de débit : un compteur par adresse cliente, plus strict sur les routes qui coûtent cher

plan pour /private/tmp/rbs-demo/depot

  + src/rate_limit/mod.rs       créé
  + src/rate_limit/config.rs    créé
  + src/rate_limit/counter.rs   créé
  + src/rate_limit/tests.rs     créé
  ~ src/lib.rs                  modifié
  ~ src/state.rs                modifié
  ~ src/router.rs               modifié
  ~ Cargo.toml                  modifié
  ~ config/default.toml         modifié
  ~ AGENTS.md                   modifié

  10 fichiers à écrire
✓ rate-limit installée — 4 fichiers

  derrière un reverse proxy, passez rate_limit.trust_forwarded_for à true — sinon tous les clients partagent l'adresse du proxy
```

Une fenêtre fixe par adresse cliente : `limit` requêtes par `window_secs`, puis un 429
`application/problem+json` portant un `Retry-After`. Les entrées `[[rate_limit.routes]]`
tiennent des limites plus serrées sur un préfixe de chemin, la première qui correspond
l'emportant.

`src/rate_limit/counter.rs` s'écrit de deux façons, décidées à la pose du fragment. Avec
`redis` installé, le compteur vit sur le serveur du cache — deux instances derrière un
répartiteur doivent compter ensemble. Sans lui, le compteur vit dans le processus, et la
limite effective est multipliée par le nombre d'instances. `rbs add redis` avant `rbs add
rate-limit` est ce qui choisit la première.

Une requête dont l'adresse cliente est inconnue n'est pas comptée : un compteur unique
pour tout le monde ferait payer à chacun ce qu'un seul consomme. `axum::serve` fournit
cette adresse ; derrière un reverse proxy c'est celle du proxy, ce à quoi sert
`rate_limit.trust_forwarded_for = true` — à ne jamais lever sur une API exposée en direct,
où n'importe quel client pourrait alors se choisir une identité par requête.

```text
$ rbs add auth
auth : authentification JWT : Argon2, jetons d'accès et de rafraîchissement, rôles
auth exige rate-limit : posée avec elle

plan pour /private/tmp/rbs-demo/blog

  + src/auth/mod.rs                                        créé
  + src/auth/model.rs                                      créé
  + src/auth/dto.rs                                        créé
  + src/auth/repository.rs                                 créé
  + src/auth/service.rs                                    créé
  + src/auth/controller.rs                                 créé
  + src/auth/guard.rs                                      créé
  + src/auth/tests.rs                                      créé
  + migration/src/m20260831_095328_create_auth_tables.rs   créé
  ~ migration/src/lib.rs                                   modifié
  ~ src/lib.rs                                             modifié
  ~ src/router.rs                                          modifié
  ~ src/openapi.rs                                         modifié
  ~ Cargo.toml                                             modifié
  ~ config/default.toml                                    modifié
  ~ .env.example                                           modifié
  ~ .env                                                   modifié
  + src/rate_limit/mod.rs                                  créé
  + src/rate_limit/config.rs                               créé
  + src/rate_limit/counter.rs                              créé
  + src/rate_limit/tests.rs                                créé
  ~ src/state.rs                                           modifié
  ~ AGENTS.md                                              modifié

  23 fichiers à écrire
✓ auth installée — 13 fichiers

  rbs migrate up
```

`auth` est l'un des deux fragments qui en exigent un autre —
[`scheduler`](../guides/scheduler.md) est l'autre, et il entraîne `jobs`, dont il déclenche
la file. `POST /auth/login` hache un Argon2 même pour un email inconnu — c'est ce qui empêche d'énumérer les comptes — et chaque requête
anonyme y coûte donc 19 Mio. Sans limite de débit, la protection contre l'énumération est
un déni de service offert au premier venu : le fragment arrive donc avec `rate-limit`, le
plan le nomme avant que rien ne s'écrive, et la section `[rate_limit]` qu'il pose tient
`/auth/login` à cinq requêtes par minute contre 120 en global.

Les deux features s'inscrivent dans `[package.metadata.rbs]` : `rbs add rate-limit` après
coup est sans effet. L'avoir installée avant ne l'est pas davantage — le plan dresse alors
`auth` seule.

`auth` est la seule feature qui touche à votre `.env` : le secret de signature y est tiré
et écrit à l'installation, pendant que `.env.example`, versionné, garde un placeholder.
Il n'y a rien à recopier, et la migration est la seule étape qui reste. Le
[guide de l'authentification](../guides/auth.md) prend la suite.

Dans chaque plan, la ligne `Cargo.toml` est l'endroit où l'installation s'inscrit :

```text
[package.metadata.rbs]
version = "1.0.0"
features = ["health", "docker", "ci", "auth", "rate-limit"]
database = "postgres"
```

Tout autre nom est refusé avec la liste de ce qui est installable :

```text
$ rbs add graphql
erreur : `graphql` n'est pas une feature installable : audit, auth, ci, cors, docker, jobs, mail, observability, rate-limit, redis, scheduler, storage
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
generate`](./generate.md#les-ancres) qui insère dans les onze ancres en commentaires Rust
du projet — `// <rbs:features>` (dans `src/lib.rs`, ou dans `src/main.rs` sur un projet
sans bibliothèque — voir [plus bas](./generate.md#les-ancres)), `// <rbs:routes>`,
`// <rbs:layers>`, `// <rbs:openapi>`, `// <rbs:migration_modules>`, `// <rbs:migrations>`,
`// <rbs:state_champs>`, `// <rbs:state_init>`, `// <rbs:startup>` et `// <rbs:seeds>`.

`// <rbs:layers>` est l'endroit où un fragment empile un middleware, et elle ne
s'interchange pas avec `// <rbs:routes>` qui la précède de quelques lignes : un `.layer()`
enveloppe ceux qui le précèdent, donc une couche insérée là s'exécute *après* `request_id`
et `trace`, jamais avant. C'est la seule position qui lui donne l'identifiant de la requête
et qui fait entrer ses propres réponses — un 429, un préflight refusé — dans le journal
comme n'importe quelle autre. `cors`, `rate-limit` et `observability` s'en servent
toutes les trois.

`docker` est le seul fragment que `rbs add` installe à faire lui-même exception : ses
services `api` et `migrate` vont dans `# <rbs:services>`, l'ancre YAML que porte un
compose — voir [plus haut](#les-douze-features). La règle est la même partout : aucun AST
n'est jamais réécrit, et une ancre absente fait que la commande n'écrit rien et affiche le
bloc à recoller. [`rbs doctor`](./doctor.md) les contrôle toutes les douze — onze sur un
projet sans compose pour en porter une onzième.

Un projet engendré avant l'existence de `// <rbs:layers>` ne la porte pas, et `rbs upgrade`
ne l'ajoute pas : cette commande aligne le manifeste et les zones de l'`AGENTS.md`, et ne
touche à aucun fichier source qui appartient au développeur. `rbs doctor` signale l'ancre
absente avec le bloc à coller, et `rbs add cors` refuse de la même façon — rien d'écrit,
le bloc affiché.

## Les échecs

Hors d'un projet :

```text
$ rbs add docker
erreur : aucun projet rbs ici : `rbs add` s'exécute dans un projet créé par `rbs new`
```

Chacun de ces cas sort en code 1.
