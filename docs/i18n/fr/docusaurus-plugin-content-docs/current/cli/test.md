---
sidebar_position: 6.5
title: rbs test
---

# `rbs test`

Lance les tests du projet comme sa CI les lance : les services dont il a besoin, les
migrations en attente, puis `cargo test` sur tout le workspace, tests de base compris. Une
commande au lieu de trois, et le code de sortie est celui de `cargo test`.

:::note
rbs parle français dans ses écrans d'aide et dans ses sorties. Tous les blocs de terminal
de cette page sont verbatim, capturés en lançant la commande.
:::

## Synopsis

{/* rbs:transcript cmd="rbs test --help" */}
```text
$ rbs test --help
Lance les tests du projet : services, migrations, puis cargo test sur tout le workspace

Utilisation : rbs test [OPTIONS] [FILTRE] [-- <ARGS>...]

Arguments :
  [FILTRE]   Ne lance que les tests dont le chemin contient ce motif
  [ARGS]...  Arguments du harnais de test, passés après `--` (ex. --nocapture)

Options :
      --no-compose  Ne remonte pas les services du compose : ils tournent déjà, ou ailleurs
      --no-migrate  N'applique pas les migrations en attente
  -h, --help        Affiche l'aide
  -V, --version     Affiche la version
```

| Argument ou option | Effet |
|---|---|
| `FILTRE` | Passé à `cargo test` comme filtre : seuls les tests dont le chemin le contient tournent — `rbs test articles` lance les tests de la feature `articles`. |
| `--no-compose` | Saute `docker compose up -d`, comme pour [`rbs dev`](./dev.md#synopsis) : les services tournent déjà, ou ailleurs. L'attente de la base reste. |
| `--no-migrate` | Saute `rbs migrate up` : les tests tournent sur le schéma tel qu'il est. |
| `-- ARGS` | Tout ce qui suit `--` va au harnais de test, après `--include-ignored` : `rbs test -- --nocapture`, `rbs test articles -- --test-threads=1`. |

## Le plan

Comme [`rbs dev`](./dev.md), `rbs test` montre ce qu'elle va faire avant de le faire. Les
trois premières étapes sont celles de `rbs dev` — le même code les planifie — et le
serveur y est remplacé par les tests :

{/* rbs:transcript cmd="rbs test --no-compose articles -- --nocapture" setup="rbs new demo --yes --lang fr --database-url postgres://rbs:secret@127.0.0.1:1/demo" dans="demo" extrait="oui" */}
```text
  base        127.0.0.1:1
  migrations  rbs migrate up
  tests       cargo test --workspace --no-fail-fast articles -- --include-ignored --nocapture
```

1. **`docker compose up -d`**, si le projet porte un `docker-compose.yml` et que
   `--no-compose` est absent ;
2. **l'attente de la base**, sautée pour SQLite. Les deux mêmes patiences que
   `rbs dev` : 30 secondes après avoir remonté le compose, 3 sinon ;
3. **[`rbs migrate up`](./migrate.md)**, sauf sous `--no-migrate` — les tests engendrés
   supposent le schéma en place ;
4. **`cargo test --workspace --no-fail-fast [FILTRE] -- --include-ignored [ARGS]`**, avec les
   variables du `.env` du projet, exactement comme [`rbs migrate`](./migrate.md) lance son
   propre binaire.

La dernière ligne est la commande que lance le workflow installé par
[`rbs add ci`](./add.md), drapeau pour drapeau. Chacun y gagne sa place :

- `--workspace` couvre la crate `migration` autant que l'application ;
- `--no-fail-fast` continue après le premier binaire de test rouge, pour qu'une seule
  passe montre tous les échecs et non le premier ;
- `--include-ignored` lance les tests qui joignent la base. Ils sont marqués
  `#[ignore = "joint la base du projet"]` pour qu'un `cargo test` nu reste rapide là où
  rien ne tourne ; ici la base vient d'être montée et migrée, ils ont tout lieu de
  tourner. Voir le [guide des tests](../guides/testing.md).

## Code de sortie

Le code de sortie est celui qu'a rendu `cargo test`, inchangé — 101 quand un test échoue
ou que le projet ne compile pas.
Une CI qui enchaîne `rbs test` distingue donc un test rouge d'une commande qui n'a pas pu
démarrer, dont le code est 1, 2 ou 3 selon ce qui l'a arrêtée — voir les [codes de
sortie](./doctor.md#codes-de-sortie). Un test rouge se termine sur :

{/* rbs:libre raison="exige un test rouge, donc une base démarrée et la compilation entière du projet" */}
```text
erreur : `cargo test` a échoué (code 101)
```

## Échecs

Tout ce qui précède les tests échoue comme [`rbs dev`](./dev.md#échecs), avec les mêmes
messages :

{/* rbs:transcript cmd="rbs test --no-compose" setup="rbs new demo --yes --lang fr --database-url postgres://rbs:secret@127.0.0.1:1/demo" dans="demo" extrait="oui" */}
```text
en attente de la base (127.0.0.1:1) ...
erreur : rien ne répond sur 127.0.0.1:1 : la base du projet n'est pas démarrée

démarrez-la — `docker compose up -d` à la racine du projet — ou corrigez RBS_DATABASE__URL dans le .env du projet
```

| Situation | Ce qui se passe |
|---|---|
| Pas de `.env`, ou pas d'URL de base dedans | Refus nommant le fichier et la variable, code 1 |
| Rien n'écoute | Le message ci-dessus, après la patience applicable, code 3 |
| Une migration échoue | L'erreur propre au binaire de migration, et aucun test ne tourne, code 1 |
| Un test échoue | Le rapport de `cargo test`, puis la ligne ci-dessus, et son code |
| Hors d'un projet | Refus nommant ce qu'elle a cherché, code 2 |
