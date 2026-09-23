---
sidebar_position: 3.5
title: rbs remove
---

# `rbs remove`

Retire une feature qu'[`rbs add`](./add.md) avait installée : ses fichiers, les lignes
qu'elle avait insérées dans chaque ancre, sa migration et — quand plus aucune autre feature
installée ne les réclame — ses dépendances. La commande défait le manifeste du fragment
section par section, dans l'ordre inverse de celui qui l'avait posé, et suit les deux mêmes
règles qu'`add` : aucun AST n'est jamais réécrit, et rien ne s'écrit tant que le plan
entier n'est pas connu de réussir.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
:::

## Synopsis

{/* rbs:transcript cmd="rbs remove --help" */}
```text
$ rbs remove --help
Retire une feature installée : ses fichiers, ses ancres, sa migration et ses dépendances

Utilisation : rbs remove [OPTIONS] <FEATURE>

Arguments :
  <FEATURE>  Feature à retirer

Options :
      --force                  Retire même si un fichier a été modifié, ou si le working tree Git est sale
      --dry-run                Affiche le plan sans rien écrire
      --json                   Rend le plan, ou l'erreur, en un document JSON sur la sortie standard
      --template-dir <CHEMIN>  Répertoire de templates remplaçant celles embarquées dans le binaire
  -h, --help                   Affiche l'aide
  -V, --version                Affiche la version
```

| Flag | Effet |
|---|---|
| `--force` | Retire même si un fichier a divergé d'un rendu neuf du fragment, ou si le working tree Git est sale. |
| `--dry-run` | Affiche le plan et s'arrête. Rien n'est écrit. |
| `--json` | Rend le plan — ou l'erreur — en un seul document JSON sur la sortie standard, au lieu du texte coloré. [Le guide des agents](../guides/agents.md#lire-un-plan-en-json) porte le document et les codes d'erreur. |
| `--template-dir <CHEMIN>` | Lit le manifeste du fragment dans un répertoire portant un sous-répertoire par feature, au lieu de ceux embarqués dans le binaire — le même répertoire qu'`add` aurait utilisé pour l'installer. |

Sans `--template-dir`, seuls les noms qu'`add` installe sont acceptés par `remove`.
Un CRUD engendré par `rbs generate crud` n'en fait pas partie, même si son nom voisine les
vrais fragments dans `[package.metadata.rbs] features` : `remove` le refuse exactement
comme il refuse un nom qui n'a jamais désigné une feature.

## Retirer une feature

{/* rbs:transcript cmd="rbs remove cors" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add cors && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m cors" dans="demo" */}
```text
$ rbs remove cors
cors : CORS : origines, méthodes et en-têtes autorisés, énumérés par la configuration

plan pour …/demo

  - src/modules/cors/mod.rs      supprimé
  - src/modules/cors/config.rs   supprimé
  - src/modules/cors/tests.rs    supprimé
  ~ src/modules/mod.rs           modifié
  ~ src/router.rs                modifié
  ~ Cargo.toml                   modifié
  ~ config/default.toml          modifié
  ~ AGENTS.md                    modifié

  5 à modifier, 3 à supprimer
✓ cors retirée — 5 modifiés, 3 supprimés
  tower-http appartient au squelette, jamais retirée

  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

Les marqueurs du plan rejoignent les trois qu'[`add`](./add.md#lidempotence) porte déjà :
`-` supprimé, `~` modifié, `·` inchangé, `!` en conflit. Le plan reconstruit exactement
les fichiers qu'`add` avait écrits, ce qui lui permet de distinguer
une ligne que le fragment avait lui-même insérée d'une ligne que le développeur a ajoutée à
côté, et de ne retirer que la première : `src/modules/mod.rs` perd ici son
`pub mod cors;` mais garde tout ce qu'un fragment posé plus tard y a monté.

`tower-http` est nommée plutôt que retirée parce qu'elle appartient au squelette
lui-même — chaque projet en dépend avant même qu'une feature soit installée — l'une des
choses [jamais retirées](#ce-qui-nest-jamais-retiré) plus bas.

## Le schéma garde ses tables

La migration d'un fragment est retrouvée par le suffixe de son nom de fichier, aucun
manifeste ne gardant l'horodatage auquel elle a été créée, et retirée comme n'importe quel
autre fichier :

{/* rbs:transcript cmd="rbs remove jobs" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add jobs && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m jobs" dans="demo" extrait="oui" */}
```text
$ rbs remove jobs
  - migration/src/m20260917_141948_create_jobs.rs   supprimé
  ~ migration/src/lib.rs                            modifié

✓ jobs retirée — 6 modifiés, 14 supprimés

  la migration est retirée du projet, mais le schéma garde ses tables : `rbs migrate down` devait passer avant

  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

`remove` n'ouvre jamais de connexion : elle édite les fichiers du projet, rien dans sa
base. Défaire les tables qu'une migration a créées est le travail de `rbs migrate down`, et
il doit s'exécuter **avant** le retrait — une fois le fichier de migration disparu,
`sea-orm-cli` n'a plus rien à lire pour écrire le `DOWN`. Un schéma laissé ainsi reste
invisible à [`rbs doctor`](./doctor.md) : aucun de ses contrôles n'interroge jamais la
base pour savoir quelles migrations y ont réellement été appliquées — `base` se borne à
vérifier que le pilote compilé dans le projet correspond au schéma de l'URL, qu'une
connexion répond dans les trois secondes, puis la version du serveur.

Une deuxième migration portant le même suffixe — un renommage, une copie faite à la
main — est refusée plutôt que devinée : `remove` ne tranchera pas entre deux candidates à
la place du développeur.

## Des fragments qui l'exigent encore

{/* rbs:transcript cmd="rbs remove mail" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add auth && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m auth && rbs add webhooks && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m webhooks" dans="demo" */}
```text
$ rbs remove mail
erreur : `mail` est encore exigée par auth, webhooks : retirez-les d'abord, dans l'ordre de votre choix
```

`webhooks` n'exige pas `mail` elle-même — elle exige `auth`, qui exige `mail`. Les
dépendants sont nommés par fermeture transitive, jusqu'au point fixe et non au premier
saut : ne nommer que `auth` ici aurait fait retirer cette dernière au développeur, pour
qu'il se heurte aussitôt à un second refus identique contre `webhooks`, que cette commande
savait déjà venir. Rien n'est écrit tant que l'un des deux dépendants reste installé ;
retirez-les d'abord, dans l'ordre qui vous convient.

## Ce qui n'est jamais retiré

Cinq choses qu'un retrait laisse intactes, toutes nommées dans le rapport plutôt que
traitées :

{/* rbs:transcript cmd="rbs remove docker" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add docker && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m docker" dans="demo" */}
```text
$ rbs remove docker
docker : Dockerfile multi-étapes, .dockerignore et services de déploiement

plan pour …/demo

  - Dockerfile           supprimé
  - .dockerignore        supprimé
  ~ docker-compose.yml   modifié
  ~ Makefile             modifié
  ~ Cargo.toml           modifié
  ~ AGENTS.md            modifié

  4 à modifier, 2 à supprimer
✓ docker retirée — 4 modifiés, 2 supprimés
  docker-compose.yml n'est pas retiré : posé seulement s'il manquait, le retrait ne peut pas savoir si ce fragment en est l'auteur
  config/production.toml n'est pas retiré : posé seulement s'il manquait, le retrait ne peut pas savoir si ce fragment en est l'auteur
  POSTGRES_USER n'est pas retirée de .env, à faire à la main si elle ne sert plus
  POSTGRES_PASSWORD n'est pas retirée de .env, à faire à la main si elle ne sert plus
  POSTGRES_DB n'est pas retirée de .env, à faire à la main si elle ne sert plus

  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

- **Les fichiers posés seulement s'ils manquaient.** Le manifeste de `docker` marque ainsi
  `docker-compose.yml` et `config/production.toml` : `add` ne les écrit que sur un projet
  qui n'en avait aucun, si bien qu'un projet qui en portait déjà un le garde tel qu'il l'a
  trouvé. Un fragment posé de cette façon ne revendique jamais la paternité du fichier, et
  le retrait ne peut pas savoir si cette installation l'a écrit ou seulement trouvé —
  `mail` et `redis` insèrent tous deux leur propre service dans ce même compose, et le
  supprimer ici emporterait leur travail aussi.
- **Les variables d'environnement.** `.env` est gitignoré — la seule écriture d'un retrait
  qu'aucun `git checkout` ne défera jamais — la décision de retirer une ligne y appartient
  donc au développeur, jamais prise à sa place. `.env.example`, versionné, n'est pas touché
  non plus : il continue de documenter la variable pour qui lira le projet ensuite.
- **Les dépendances qu'un autre fragment installé déclare encore.** Chaque autre
  manifeste installé est lu avant de planifier le moindre retrait, si bien qu'une crate
  réclamée par deux fragments survit tant que l'un des deux reste.
- **Une feature d'une dépendance qu'un autre fragment installé réclame encore**,
  distincte de la crate elle-même : `auth`, `scheduler`, `rate-limit` et `redis` activent
  tous la feature `time` de tokio, si bien que retirer l'un d'eux pendant qu'un autre reste
  installé la laisse en place.
- **Les dépendances que le squelette lui-même déclare**, `tower-http` ci-dessus en étant
  une : `rbs new` les pose dans `Cargo.toml` avant même qu'aucune feature n'existe, et
  aucun retrait ne les réclame.

## Un nom inconnu

{/* rbs:transcript cmd="rbs remove graphql" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs remove graphql
erreur : `graphql` n'est pas un fragment : api-keys, audit, auth, ci, cors, docker, frontend, frontend-admin, jobs, mail, observability, rate-limit, redis, scheduler, storage, webhooks
```

Vérifié avant même que le manifeste du projet ne soit lu : un nom qui n'a jamais été un
fragment ne devient pas « déjà absent » pour avoir été tapé sur un projet qui ne l'a jamais
installé — il reste refusé.

## Idempotence

Retirer ce qui n'est pas installé n'est pas un échec — la même règle
qu'[`add`](./add.md#lidempotence) suit, en miroir :

{/* rbs:transcript cmd="rbs remove docker" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs remove docker
✓ docker n'est pas installée — rien à faire
```

`[package.metadata.rbs] features` est ce que la commande lit, exactement comme `add` : un
nom absent de cette liste n'a rien de planifié contre lui, quels que soient les fichiers
qui se trouvent sur le disque.

## Un working tree sale

`remove` édite `Cargo.toml`, donc — comme `add` — elle refuse de s'exécuter sur des
modifications non commitées :

{/* rbs:transcript cmd="rbs remove cors" setup="rbs new demo --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add cors && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m cors && rbs add ci" dans="demo" */}
```text
$ rbs remove cors
erreur : le working tree n'est pas propre : AGENTS.md, Cargo.toml — commitez, ou relancez avec --force
```

Les fichiers non suivis ne comptent pas : rien ici ne s'apprête à en créer un. `--force`
s'exécute quand même.

## Conflits

Un fichier que le fragment retirerait mais dont le contenu ne correspond plus à ce qu'en
rendrait aujourd'hui le fragment n'est ni supprimé ni laissé silencieusement en place. Le
plan le marque `!`, et la commande s'arrête, exactement comme le conflit d'`add` pour un
fichier qu'elle écraserait :

{/* rbs:libre raison="exige de modifier à la main src/modules/cors/config.rs, ce que le rejeu ne sait pas faire sans shell" */}
```text
$ rbs remove cors
cors : CORS : origines, méthodes et en-têtes autorisés, énumérés par la configuration

plan pour …/demo

  - src/modules/cors/mod.rs      supprimé
  ! src/modules/cors/config.rs   conflit — relancer avec --force
  - src/modules/cors/tests.rs    supprimé
  ~ src/modules/mod.rs           modifié
  ~ src/router.rs                modifié
  ~ Cargo.toml                   modifié
  ~ config/default.toml          modifié
  ~ AGENTS.md                    modifié

  5 à modifier, 2 à supprimer, 1 en conflit
erreur : src/modules/cors/config.rs — relancer avec --force pour les écraser
```

C'est ce qui protège un fichier retouché à la main de disparaître sans trace : `remove`
recalcule ce qu'`add` aurait écrit aujourd'hui et le compare à ce qui est réellement sur
le disque, octet pour octet. Tout ce qui diverge — une ligne ajoutée, une valeur
changée — marque ce seul fichier `!`, mais retient le plan tout entier : sans `--force`,
rien n'est écrit du tout — ni `config.rs`, ni les deux suppressions, ni les cinq
modifications autour. `--force` écrit le plan tout entier quand même, `config.rs`
compris, le même plan affiché d'abord :

{/* rbs:libre raison="exige la même modification à la main de src/modules/cors/config.rs" */}
```text
$ rbs remove cors --force
cors : CORS : origines, méthodes et en-têtes autorisés, énumérés par la configuration

plan pour …/demo

  - src/modules/cors/mod.rs      supprimé
  ! src/modules/cors/config.rs   conflit — relancer avec --force
  - src/modules/cors/tests.rs    supprimé
  ~ src/modules/mod.rs           modifié
  ~ src/router.rs                modifié
  ~ Cargo.toml                   modifié
  ~ config/default.toml          modifié
  ~ AGENTS.md                    modifié

  5 à modifier, 2 à supprimer, 1 en conflit
✓ cors retirée — 5 modifiés, 3 supprimés
  tower-http appartient au squelette, jamais retirée

  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

Les deux blocs ci-dessus sont capturés pour de vrai, sur un projet où `config.rs` a été
retouché à la main après l'installation de `cors` — ce qui est commun avec le conflit
propre à `add`, c'est l'absence de garde, pas le scénario dont il vient. Aucun ne porte de
marqueur `{/* rbs:transcript */}` : ces deux blocs viennent d'un scénario que la capture
automatique ne sait pas rejouer — une ligne ajoutée à un fichier que Git suit déjà.

Un conflit ne dit pas toujours qu'un fichier a été modifié. `remove` rend le fragment tel
qu'il se rendrait *aujourd'hui*, et certains fragments ne se rendent pas de la même façon
sur tous les projets : le compteur de `rate-limit` s'écrit contre Redis quand le projet
porte `redis`, en mémoire sinon. Poser `rate-limit`, puis `redis` — rien n'oblige les deux
à arriver ensemble, `rate-limit` n'exigeant aucune autre feature — fait diverger les
fichiers de `rate-limit` d'un rendu neuf, et le retrait les marque `!` sans que personne y
ait touché. `--force` est là aussi la réponse.

## Le compilateur est l'oracle

`remove` ne cherche jamais dans le code du projet une référence à la feature qu'elle vient
de retirer — un appel à `webhooks::emit`, un `mail::Service` encore porté par `AppState`,
un `use` laissé pendant. Elle ne connaît que ce que le fragment lui-même déclarait : ses
fichiers, ses lignes d'ancre, sa migration, ses dépendances. Tout ce que le développeur a
écrit *contre* la feature lui est invisible, et c'est exactement à cela que sert
`cargo build` — la ligne que tout retrait réussi affiche :

{/* rbs:transcript cmd="rbs remove cors" setup="rbs new demo --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add cors && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m cors" dans="demo" extrait="oui" */}
```text
  lancez `cargo build` : le compilateur nomme ce qui référençait encore la feature
```

Le compilateur nomme chaque référence restante, une erreur à la fois, avec exactitude —
`remove` ne porte aucune analyse statique qui n'en ferait qu'une partie.

## Lire un plan en JSON

`remove` fait partie des commandes que [`--json` lit](../guides/agents.md#lire-un-plan-en-json)
en un seul document plutôt qu'en texte coloré. `fichiers` porte un troisième compteur à
côté des deux d'`add` — `supprimes`, pour ce qu'un retrait fait le plus souvent :

{/* rbs:transcript cmd="rbs remove cors --dry-run --json" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add cors && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m cors" dans="demo" extrait="oui" */}
```text
$ rbs remove cors --dry-run --json
  "fichiers": {
    "crees": 0,
    "modifies": 5,
    "supprimes": 3
  }
```

Ce qu'un humain aurait encore besoin de voir — l'avertissement de migration, ce qui a été
laissé en place — part sur la sortie d'erreur, exactement comme un avertissement qu'`add`
imprime : le document JSON de la sortie standard ne porte que le plan.

## Échecs

Hors d'un projet :

{/* rbs:transcript cmd="rbs remove cors" */}
```text
$ rbs remove cors
erreur : aucun projet rbs ici : `rbs remove` s'exécute dans un projet créé par `rbs new`
```

Statut de sortie 2 : c'est l'appel qui a besoin d'être corrigé — voir
[les codes de sortie](./doctor.md#codes-de-sortie).

Un projet qui vient de subir un retrait se diagnostique sain :
[`rbs doctor`](./doctor.md) lit le même manifeste que cette commande écrit, et l'inventaire
d'`AGENTS.md` est rafraîchi dans le même plan — un retrait ne laisse jamais les deux se
contredire sur ce que le projet porte encore.
