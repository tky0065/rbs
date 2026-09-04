---
sidebar_position: 5
title: rbs doctor
---

# `rbs doctor`

Diagnostique un projet généré par six contrôles : les ancres,
[`AGENTS.md`](../guides/agents.md), les relations déjà écrites dans ses modèles, le
`.env`, les versions et la base. Chacun est indépendant et rend son verdict sans
interrompre les autres — un diagnostic qui s'arrête au premier problème oblige à le
relancer autant de fois qu'il y a de problèmes.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

```text
$ rbs doctor --help
Diagnostique le projet : ancres, .env, base joignable, versions

Usage: rbs doctor [OPTIONS]

Options:
      --json     Rend le rapport en JSON sur la sortie standard, pour un script ou une CI
      --fix      Repose les ancres absentes avant de diagnostiquer
      --force    Repose les ancres même si le working tree Git est sale
  -h, --help     Print help
  -V, --version  Print version
```

Trois flags, et trois seulement. `--json` rend le rapport en document ; `--fix` repose les
ancres absentes avant de diagnostiquer, et `--force` le laisse écrire sur un working tree
sale. `--force` ne lève que cette garde-là, et est donc refusé seul : rien d'autre n'écrit
dans `doctor`, si bien qu'isolé il serait pris puis ignoré. `--template-dir` et `--yes` ne sont pas acceptés ici : chacun est déclaré sur les
commandes qui le lisent, si bien qu'en passer un est une erreur de clap plutôt qu'un flag
pris puis ignoré.

## Les six contrôles

| Contrôle | Ce qu'il regarde |
|---|---|
| `ancres` | Les onze ancres Rust en commentaire : `// <rbs:features>` dans `src/lib.rs` — ou dans `src/main.rs`, sur un projet engendré avant que cette bibliothèque existe — `// <rbs:routes>` et `// <rbs:layers>` dans `src/router.rs`, `// <rbs:openapi>` dans `src/openapi.rs`, `// <rbs:migration_modules>` et `// <rbs:migrations>` dans `migration/src/lib.rs`, `// <rbs:state_champs>` et `// <rbs:state_init>` dans `src/state.rs`, `// <rbs:startup>` dans `src/main.rs`, `// <rbs:seeds>` dans `src/seeds/main.rs`, `// <rbs:health_probes>` dans `src/health/controller.rs` — plus l'ancre YAML `# <rbs:services>` dans `docker-compose.yml`, douzième et optionnelle : un projet sans compose n'en a aucune à porter. |
| `agents` | [`AGENTS.md`](../guides/agents.md) : présent, ses deux zones présentes, la version du guide accordée à celle du CLI, l'inventaire accordé au projet, chaque feature déclarée adossée à un répertoire — et, en simple avertissement, un répertoire de `src/` que rien ne déclare. Couvert à part plus bas. |
| `relations` | Les deux ancres qu'un modèle réclame pour recevoir une relation — `// <rbs:relations:table>` et `// <rbs:related:table>`, une paire par entité. Hors du registre des ancres ci-dessus, puisque le fichier qui les porte dépend des features du projet. Il ne rougit que pour un modèle qui porte déjà un `belongs_to` ou un `has_many` sans l'une de ses deux ancres — un état vraisemblablement issu d'une retouche à la main, puisque [`rbs generate`](./generate.md) n'en laisse jamais derrière lui. |
| `.env` | Toute variable déclarée par `.env.example` est renseignée dans `.env`. `.env.example` sert de référence parce qu'il est versionné et généré avec le squelette — une liste tenue dans le CLI aurait fait deux vérités à synchroniser. |
| `versions` | Le rbs inscrit dans `[package.metadata.rbs]`, la dépendance `rbs-core`, et le CLI qui diagnostique. |
| `base` | Le pilote compilé au manifeste face au schéma de l'URL, puis une connexion TCP en moins de trois secondes, puis la version du serveur — demandée au binaire de la crate `migration`, rbs n'embarquant aucun client SQL. Chaque moteur a son plancher, et chaque plancher sa raison : PostgreSQL 14, le plus ancien encore maintenu ; MySQL 8.0, pour `FOR UPDATE SKIP LOCKED` ; SQLite 3.35, pour `UPDATE … RETURNING`. |

Une ancre disparue ne casse rien tant qu'aucune génération n'a lieu : c'est précisément
pourquoi `doctor` la cherche avant que [`rbs generate`](./generate.md) ne bute dessus.

Le pilote passe avant la connexion, et c'est délibéré. Un serveur qui répond ne prouve rien
quand le pilote compilé dans votre binaire ne sait pas parler son protocole, et sonder le
port d'abord ferait payer trois secondes à un diagnostic qui tient dans deux lectures de
fichier :

```text
  ✗ base        le manifeste compile `sqlx-postgres` et RBS_DATABASE__URL est une URL `mysql://`
      alignez les deux : la feature `sqlx-mysql` de sea-orm au manifeste, ou une URL `postgres://` dans le .env
```

C'est la contradiction que [`rbs new`](./new.md) refuse d'emblée, rencontrée ici après coup
— sur un projet dont le `.env` a été édité plus tard.

## Les deux avertissements

Tout autre verdict ci-dessus est un succès ou un échec. `agents` peut aussi avertir, à une
seule condition : un répertoire de `src/` qu'aucun fragment installé et qu'aucune feature
déclarée dans `[package.metadata.rbs]` n'explique — du code que personne n'a engendré.

```text
  ! agents      écrit hors du CLI : webhooks
      légitime si rbs ne couvre pas ce code ; sinon, rbs generate le reprend
```

Il reste un avertissement plutôt qu'un échec, et c'est voulu. Écrire à la main ce que rbs
n'engendre pas — un endpoint qui n'est pas un CRUD, un client HTTP externe, une règle
métier — est légitime et prévu ; c'est précisément ce qu'[`AGENTS.md`](../guides/agents.md)
dit à un agent de faire quand il rencontre du code que rbs n'a pas vocation à engendrer.
Faire échouer la commande là-dessus rendrait `rbs doctor` rouge sur un projet parfaitement
sain dès que quelqu'un ajoute ce genre de code, ce qui rendrait le contrôle inutilisable en
CI. Un avertissement ne change ni le code de sortie ni le verdict d'ensemble : un projet
qui ne porte qu'un avertissement continue de sortir en 0 et d'être rapporté comme sain —
seul un échec véritable change cela.

Le second appartient à `gardes`, et n'existe que sur un projet portant
[`auth`](../guides/auth.md) : une feature dont `create`, `update` ou `delete` n'appelle
aucune `require_role`.

```text
  ! gardes      écritures anonymes : articles, comments
      réservez-les à un rôle : `rbs generate crud <nom> --fields … --role admin` pose le garde à la génération, et `identite.require_role(Role::Admin)?` le pose à la main — voir le guide de l'authentification
```

Le même raisonnement, deux fois. Une API qui écrit sans demander qui appelle est un choix
légitime — un catalogue public, un service derrière une passerelle qui authentifie déjà —
et le constat ne peut donc pas être un échec. Et la garde se reconnaît à ce seul appel : un
projet qui protège ses écritures autrement est nommé ici aussi.

## Les features installées

Chaque feature qui porte de la configuration ajoute une ligne à elle, et cette ligne
n'existe que sur un projet qui a déclaré la feature. `auth` en ajoute deux — son secret, et
le contrôle `gardes` ci-dessus. `jobs` est celle que ce jalon a ajoutée :

```text
  ✗ jobs        config/default.toml ne porte pas de section `[jobs]`
      ajoutez à config/default.toml :
      [jobs]
      max_attempts = 5
      retry_delay_secs = 30
      poll_interval_secs = 1
```

Une feature déclarée dans `[package.metadata.rbs]` dont la section a disparu de la
configuration est un projet qui compile et échoue au démarrage — ce que `doctor` sait dire
à froid, avant que vous ne le lanciez. Une section mise en commentaire ne compte pas pour
une section.

[`observability`](../guides/observability.md) lit une valeur plutôt que de se contenter de
chercher sa section : le port sur lequel son second listener se pose ne peut pas être celui
que l'API écoute.

```text
  ✗ observability `observability.metrics_port` et `server.port` valent tous deux 8080
      donnez aux métriques un port à elles dans config/default.toml :
      [observability]
      metrics_port = 9090
```

L'endpoint OTLP, lui, n'est pas contrôlé. Son absence est un mode de fonctionnement
légitime — un poste de développement n'a pas de collecteur — et non une faute.

## Un rapport machine-lisible

`--json` écrit les mêmes constats en un seul document sur la sortie standard — rien d'autre
n'y va, ni couleur ni glyphe — de sorte qu'une étape de CI peut nommer le contrôle qui a
échoué au lieu de chercher une croix. Le code de sortie garde le sens qu'il avait déjà : 0
quand le projet est sain, 1 quand un contrôle a échoué.

```text
$ rbs doctor --json
{
  "sain": false,
  "checks": [
    {
      "name": "ancres",
      "status": "ok",
      "detail": "les 13 points d'insertion sont en place"
    },
    {
      "name": "base",
      "status": "erreur",
      "detail": "rien ne répond sur 127.0.0.1:5499",
      "remede": "lancez `docker compose up -d` à la racine du projet, ou corrigez l'URL du .env"
    }
  ]
}
```

`status` vaut `ok`, `avertissement` ou `erreur` — les trois états que le rendu texte dessine
`✓`, `!` et `✗`. `remede` n'est présent que sur les contrôles qui en portent un. `sain` est
faux dès qu'un contrôle a échoué, soit la condition même du code de sortie 1.

```bash
rbs doctor --json | jq -r '.checks[] | select(.status != "ok") | "\(.name) : \(.detail)"'
```

## Pourquoi cela prend parfois une minute

Le contrôle `base` lance le binaire de migration du projet, ce qui suppose que cargo
bâtisse d'abord la crate `migration` — une minute ou plus sur un répertoire de compilation
froid. `doctor` annonce cette ligne avant de bloquer plutôt qu'après, pour qu'une attente
muette ne passe jamais pour un blocage :

```text
  ✓ .env        les 7 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core alignés sur le CLI 1.1.0
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
   Compiling sea-orm v2.0.2
   Compiling migration v0.1.0 (/tmp/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 31.57s
  ✓ base        postgres 18.6 répond sur 127.0.0.1:5432
✓ le projet est sain
```

L'annonce est une ligne du seul rendu texte ; `--json` ne la porte jamais.

## Un projet sain

{/* rbs:transcript cmd="rbs doctor" setup="rbs new demo --yes --with jobs --database-url postgres://rbs:secret@localhost:55501/demo" dans="demo" base="oui" extrait="oui" */}
```text
$ rbs doctor
  ✓ ancres      les 13 points d'insertion sont en place
  ✓ agents      guide et inventaire à jour
  ✓ relations   les modèles portent leurs ancres de relation
  ✓ .env        les 7 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core pris d'un chemin local alignés sur le CLI 1.1.0
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
     Running `target/debug/migration version`
  ✓ base        postgres 18.6 répond sur localhost:55501
  ✓ jobs        la configuration de la file est en place
✓ le projet est sain
```

Code de sortie 0.

## Un projet à problèmes

Ci-dessous, le même projet privé de `// <rbs:openapi>` dans `src/openapi.rs`, de
`RBS_LOG_FORMAT` dans `.env`, et avec PostgreSQL arrêté :

```text
$ rbs doctor
  ✗ ancres      openapi manque dans src/openapi.rs
      dans src/openapi.rs :
      // <rbs:openapi>
      // </rbs:openapi>
  ✓ agents      guide et inventaire à jour
  ✓ relations   les modèles portent leurs ancres de relation
  ✗ .env        RBS_LOG_FORMAT absente du .env
      ajoutez au .env :
      RBS_LOG_FORMAT=pretty
  ✓ versions    projet et rbs-core pris d'un chemin local alignés sur le CLI 1.1.0
  ✗ base        rien ne répond sur localhost:55501
      lancez `docker compose up -d` à la racine du projet, ou corrigez l'URL du .env
  ✓ jobs        la configuration de la file est en place
attention : le projet demande votre attention
```

Trois échecs, quatre contrôles encore au vert, et chaque ligne en défaut porte le geste qui
la corrige : le bloc d'ancre à recoller, la ligne de `.env` à ajouter, le serveur à
démarrer.

Code de sortie 1. Un diagnostic qui trouve quelque chose n'est pas un échec de la commande,
mais un script doit pouvoir le distinguer d'un projet sain : le code diffère.

## Reposer les ancres

Une ancre, ce sont deux lignes de commentaire, et rien ne dit où elles vivaient une fois
qu'elles ont disparu. `--fix` les repose : chaque ancre déclare la ligne sous laquelle elle
se tient — `.merge(docs)` pour `// <rbs:layers>`, `core: CoreState::new(db, config),` pour
`// <rbs:state_init>` — et le bloc revient sous cette ligne, à la colonne qui était la
sienne.

La réparation passe avant le diagnostic, pour que le contrôle `ancres` du même rapport
compte ce qui vient d'être reposé plutôt que d'annoncer rouge un projet que la commande
vient de remettre d'aplomb.

Ci-dessous, un projet dont `// <rbs:openapi>` et `// <rbs:state_init>` ont été supprimées :

```text
$ rbs doctor --fix --force
plan pour /private/tmp/rbs-demo/demo

  ~ src/openapi.rs   modifié
  ~ src/state.rs     modifié

  2 fichiers à écrire

✓ 2 ancres reposées : openapi, state_init

  ✓ ancres      les 11 points d'insertion sont en place
  ✓ agents      guide et inventaire à jour
  ✓ relations   les modèles portent leurs ancres de relation
  ✓ .env        les 4 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core 1.1.0 alignés sur le CLI 1.1.0
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running `target/debug/migration version`
  ✓ base        sqlite 3.51 répond sur demo.db
✓ le projet est sain
```

Code de sortie 0. Le plan est affiché avant qu'un octet ne soit écrit, comme pour toute
commande qui touche un projet existant, et l'écriture passe par le même journal : si l'un
des deux fichiers échouait, l'autre serait remis dans l'état où il était.

L'exactitude de la pose se lit directement dans Git, sur un projet dont les ancres ont été
supprimées après le commit :

```text
$ git diff --stat
```

Rien. Les deux blocs sont revenus à l'octet où le squelette les avait posés.

## Un working tree sale

```text
$ rbs doctor --fix
erreur : le working tree n'est pas propre : src/openapi.rs, src/state.rs — commitez, ou relancez avec --force
```

Code de sortie 1. C'est la garde d'[`rbs add`](./add.md), d'[`rbs
generate`](./generate.md) et d'[`rbs upgrade`](./upgrade.md) : ce que la réparation écrit
doit rester discernable de votre propre travail au prochain `git diff`. Commitez, ou passez
`--force`.

La garde vient après le plan et non avant : un projet qui n'a aucune ancre à reposer n'a
rien à protéger, et `rbs doctor --fix` doit pouvoir y répondre depuis un working tree plein
de travail en cours.

```text
$ rbs doctor --fix
✓ aucune ancre à reposer
```

## Quand elle s'abstient

Une ligne d'accroche que le fichier ne porte pas — ou qu'il porte deux fois — ne dit plus
où va le bloc. `--fix` laisse alors l'ancre où elle n'est pas, la nomme, et le contrôle qui
suit affiche le bloc à coller, exactement comme avant :

```text
$ rbs doctor --fix --force
plan pour /private/tmp/rbs-demo/demo

  ~ src/seeds/main.rs   modifié

  1 fichier à écrire

✓ 1 ancre reposée : seeds
attention : layers n'a pas été reposée — la ligne d'accroche `.merge(docs)` est introuvable dans src/router.rs

  ✗ ancres      layers manque dans src/router.rs
      dans src/router.rs :
      // <rbs:layers>
      // </rbs:layers>
```

Code de sortie 1, un contrôle ayant échoué. L'autre ancre a bien été reposée : une
abstention vaut pour une ancre, non pour toute l'exécution.

S'abstenir est le but, non un défaut. `// <rbs:layers>` se tient *à l'intérieur* de `trace`
et de `request_id` — un `.layer()` enveloppe ce qui le précède — si bien qu'une couche
ajoutée à cette ancre voit l'identifiant de la requête, et que ses propres réponses
courtes, un 429 ou un préflight refusé, restent dans la trace. Reposez la même ancre deux
lignes plus bas et plus rien de tout cela ne tient, sans que rien ne le dise avant la
lecture d'un journal. Une ancre reposée au mauvais endroit coûte plus cher qu'une ancre
laissée absente.

Il en va de même d'une ancre dont les deux balises n'ont pas disparu ensemble : celle qui
reste ne dit pas où était l'autre — entre les deux, il y avait tout ce que l'ancre portait.

Sous `--json`, la réparation a son propre objet, pour qu'un script n'ait pas à déduire d'un
verdict devenu vert que quelque chose a été écrit :

```text
$ rbs doctor --fix --force --json
{
  "sain": false,
  "reparation": {
    "reposees": [],
    "laissees": [
      {
        "ancre": "layers",
        "raison": "la ligne d'accroche `.merge(docs)` est introuvable dans src/router.rs"
      }
    ]
  },
  "checks": [
    {
      "name": "ancres",
      "status": "erreur",
      "detail": "layers manque dans src/router.rs",
      "remede": "dans src/router.rs :\n// <rbs:layers>\n// </rbs:layers>"
    }
  ]
}
```

`reparation` est absent sans `--fix`, et le plan n'est jamais affiché sous `--json` : la
sortie standard porte le document, et rien d'autre.

## Joignable mais illisible

Les deux moitiés du contrôle `base` échouent séparément. Ici l'hôte répond sur le port, mais
la version n'a pas pu être lue, la crate `migration` n'ayant pas abouti — le remède nomme la
commande à lancer à la main :

```text
$ rbs doctor
  ✓ ancres      les 13 points d'insertion sont en place
  ✓ agents      guide et inventaire à jour
  ✓ relations   les modèles portent leurs ancres de relation
  ✓ .env        les 7 variables de .env.example sont renseignées
  ✓ versions    projet et rbs-core pris d'un chemin local alignés sur le CLI 1.1.0
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
   Compiling migration v0.1.0 (/private/tmp/rbs-demo/demo/migration)
error[E0425]: cannot find value `url_de_la_base` in this scope
  --> migration/src/main.rs:16:13
   |
16 |     let _ = url_de_la_base;
   |             ^^^^^^^^^^^^^^ not found in this scope

For more information about this error, try `rustc --explain E0425`.
error: could not compile `migration` (bin "migration") due to 1 previous error
  ✗ base        localhost:55501 répond, mais sa version reste inconnue : la crate migration a échoué (code 101)
      vérifiez que `cargo run -p migration -- version` aboutit
  ✓ jobs        la configuration de la file est en place
attention : le projet demande votre attention
```

## Hors d'un projet

```text
$ rbs doctor
erreur : cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici
```

Code de sortie 1.
