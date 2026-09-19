---
sidebar_position: 11.5
title: Scheduler
---

# Le déclenchement calendaire

`rbs add scheduler` donne un calendrier à un projet : neuf fichiers sous `src/modules/scheduler/`,
une migration pour la table `schedules`, et un ticker démarré avec le serveur. C'est la
réponse à la dernière ligne du [guide des jobs](./jobs.md) — une file sait exécuter un
travail et le réessayer, mais rien ne l'enfile sinon un événement que vous écrivez.

**Le scheduler déclenche, il n'exécute pas.** Un tick réserve une échéance due et enfile un
job dans la table `jobs` ; le worker fait le reste. Les réessais, le registre, la
journalisation et l'exécution existent déjà et sont éprouvés — les réécrire pour la seule
raison qu'une horloge les aurait lancés donnerait deux boucles à maintenir au lieu d'une.

C'est pourquoi le fragment exige `jobs`, et c'est le seul du
[tableau d'`rbs add`](../cli/add.md#les-quatorze-features) à entraîner une autre feature avec
lui, en dehors d'`auth`. Sur un projet nu, `rbs add scheduler` pose `jobs` d'abord et
`scheduler` ensuite, dans un même plan :

{/* rbs:transcript cmd="rbs add scheduler" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add scheduler
scheduler : déclenchement calendaire : une échéance due enfile un job, une seule fois entre réplicas
scheduler exige jobs : posée avec elle

plan pour …/demo

  + src/modules/jobs/mod.rs                              créé
  + src/modules/jobs/config.rs                           créé
  + src/modules/jobs/model.rs                            créé
  + src/modules/jobs/queue/mod.rs                        créé
  + src/modules/jobs/queue/reserve.rs                    créé
  + src/modules/jobs/queue/outcome.rs                    créé
  + src/modules/jobs/worker.rs                           créé
  + src/modules/jobs/demo.rs                             créé
  + src/modules/jobs/tests/mod.rs                        créé
  + src/modules/jobs/tests/lease.rs                      créé
  + src/modules/jobs/tests/reservation.rs                créé
  + src/modules/jobs/tests/retry.rs                      créé
  + src/modules/jobs/tests/worker.rs                     créé
  + migration/src/m20260913_132217_create_jobs.rs        créé
  ~ migration/src/lib.rs                                 modifié
  + src/modules/mod.rs                                   créé
  ~ src/lib.rs                                           modifié
  ~ src/main.rs                                          modifié
  ~ Cargo.toml                                           modifié
  ~ config/default.toml                                  modifié
  + src/modules/scheduler/mod.rs                         créé
  + src/modules/scheduler/config.rs                      créé
  + src/modules/scheduler/model.rs                       créé
  + src/modules/scheduler/sync.rs                        créé
  + src/modules/scheduler/ticker.rs                      créé
  + src/modules/scheduler/tests/mod.rs                   créé
  + src/modules/scheduler/tests/expression.rs            créé
  + src/modules/scheduler/tests/sync.rs                  créé
  + src/modules/scheduler/tests/ticker.rs                créé
  + migration/src/m20260913_132217_create_schedules.rs   créé
  ~ AGENTS.md                                            modifié

  25 à créer, 6 à modifier
✓ scheduler installée — 25 créés, 6 modifiés

  rbs migrate up, puis `rbs generate job <nom> --every "<cron>"` pour déclarer une échéance dans src/modules/scheduler/mod.rs — les expressions sont évaluées en UTC
```

Deux migrations viennent avec lui : [`rbs migrate up`](../cli/migrate.md) est donc la
commande suivante, car tant que les deux tables n'existent pas, ni le ticker ni le worker
n'ont rien à lire.

## Déclarer une échéance

Le calendrier est déclaré en code, dans `src/modules/scheduler/mod.rs`, et la base n'en porte que
l'état. `schedules()` est au ticker ce que `registry()` est au worker — l'unique liste que
vous éditez. L'exemple garde l'échéance de démonstration telle que le fragment la pose :

```rust file=examples/event-hub/src/modules/scheduler/mod.rs region=schedules
```

`Schedule::every::<J>` prend le job en paramètre de type et tire le `kind` de `J::KIND`,
au lieu d'une chaîne que vous retaperiez. **Une échéance qui viserait un job absent du
registre est donc inécrivable** : le compilateur la refuse, là où un calendrier rangé en
base aurait accepté un `kind` mal orthographié et aurait échoué chaque nuit sans rien dire.

Le second argument est une fabrique, non une valeur. Elle est rejouée à chaque
déclenchement : une charge utile qui porte une date reçoit celle du tick, et non celle du
déploiement.

Changer le calendrier passe par un déploiement. C'est le prix ordinaire d'une configuration
versionnée, et c'est ce qui rend la liste relisible dans un diff.

[`rbs generate job <nom> --every "<cron>"`](../cli/generate.md#rbs-generate-job) est la commande qui écrit
une entrée comme celle ci-dessus : le fichier du job sous `src/modules/jobs/`, son
inscription au worker, et — à cause de `--every` — la ligne `calendrier.push(…)` entre les
balises `// <rbs:schedules>`, les trois dans un seul plan. `--every` refuse une expression
que `Schedule::compiler` refuserait au démarrage, et refuse une seconde échéance pour un
job déjà planifié sous une autre expression — la ligne se clé par `kind`, et la commande ne
choisit jamais l'une des deux à votre place.

### Un calendrier antérieur à l'ancre

Un projet engendré avant 1.5.0 déclare encore `schedules()` comme un littéral `vec![]`,
sans balises `// <rbs:schedules>` à l'intérieur :

```rust
pub fn schedules() -> Vec<Schedule> {
    vec![Schedule::every::<crate::modules::jobs::demo::Log>(
        "0 3 * * *",
        || crate::modules::jobs::demo::Log {
            message: "échéance quotidienne".to_string(),
        },
    )]
}
```

`rbs doctor --fix` ne peut pas reposer l'ancre sur un tel projet : une ancre se repose sous
la ligne qu'elle déclare comme accroche, et celle de cette fonction —
`let mut calendrier = Vec::new();` — n'y existe pas encore. `rbs doctor` signale l'ancre
absente comme toujours, mais le remède ici est une retouche à la main plutôt qu'une
relance. Une ancre posée à l'intérieur d'une expression ne survit pas à rustfmt dès qu'un
second élément s'y ajoute — c'est ce qui a fait quitter à [`jobs::registry`](./jobs.md#linscrire)
sa chaîne d'appels `.register()` pour `// <rbs:jobs>`. Réécrivez donc la fonction en
instructions, et posez vous-même les balises :

```rust
#[allow(clippy::vec_init_then_push)]
pub fn schedules() -> Vec<Schedule> {
    let mut calendrier = Vec::new();
    // <rbs:schedules>
    // </rbs:schedules>
    calendrier.push(Schedule::every::<crate::modules::jobs::demo::Log>(
        "0 3 * * *",
        || crate::modules::jobs::demo::Log {
            message: "échéance quotidienne".to_string(),
        },
    ));
    calendrier
}
```

Une fois l'ancre en place, `rbs generate job --every` y écrit comme sur n'importe quel
autre projet. `#[allow(clippy::vec_init_then_push)]` n'est pas décoratif : clippy préfère
un littéral `vec![]`, précisément la seule forme qu'une ancre ne peut pas porter — un
second `.push()` qu'un fragment ou un job engendré ajoute plus tard passerait sans
problème, mais le commentaire de la balise posé entre deux éléments d'un littéral est
défiguré par le premier `cargo fmt` qui s'ensuit.

## Cinq champs ou six

La crate `cron` attend six champs, la seconde en tête : `0 0 3 * * *`. Le crontab Unix en a
cinq, et `0 3 * * *` est ce que tout le monde a dans les doigts. Le fragment accepte les
deux — cinq champs sont préfixés de `0 `, ce que la ligne de crontab veut dire de toute
façon, et six passent intacts. Toute autre longueur est refusée en nommant l'expression :

```rust file=examples/event-hub/src/modules/scheduler/mod.rs region=normaliser
```

## Tout est en UTC

Les occurrences se calculent en UTC. `0 3 * * *` est 3 h UTC — ni 3 h à Paris, ni 3 h à
l'heure locale de la machine. Pour un lecteur français, cela fait 5 h en été et 4 h en
hiver ; si une tâche doit se déclencher à une heure locale fixe de part et d'autre d'un
changement d'heure, aucune expression cron ne le fera, et c'est au job lui-même de décider
s'il est dû.

Une expression illisible arrête le démarrage. Chaque expression est compilée avant la
première écriture, et une seule invalide interrompt le processus en la nommant. Retirer
l'échéance fautive et continuer donnerait un service qui paraît sain et dont une tâche ne
tourne jamais — le mode de panne le plus coûteux à diagnostiquer. C'est un écart assumé
avec le worker, qui, lui, laisse l'API répondre quand sa propre configuration est illisible :
là-bas un service HTTP est en jeu, ici une liste statique que le développeur vient
d'écrire.

## Un seul réplica gagne une échéance due

Trois instances de l'API, ce sont trois tickers, et la purge nocturne ne doit tourner
qu'une fois. C'est toute la raison d'être de la table `schedules` : elle est l'état partagé
par lequel les réplicas s'arbitrent.

Une échéance se réserve par un `UPDATE` conditionnel — `WHERE kind = ? AND next_run_at <= ?` :

```rust file=examples/event-hub/src/modules/scheduler/ticker.rs region=reserve
```

`rows_affected == 1` désigne le gagnant ; les perdants voient zéro. La condition est
évaluée sous le verrou de ligne que l'`UPDATE` pose lui-même : le second ticker attend le
commit du premier, relit un `next_run_at` déjà avancé, et n'affecte rien. Pas de
`SKIP LOCKED`, pas de SQL par dialecte — contrairement au dépilage d'un job, il n'y a ici
aucune ligne à *élire*, elle est nommée par sa clé primaire.

**La réservation et l'enfilage partagent une transaction.** `begin`, `UPDATE` conditionnel,
puis — si la ligne est gagnée — `enqueue`, puis `commit`. Sans cette transaction, un arrêt
entre les deux avancerait l'échéance sans jamais créer le job, et personne ne s'en
apercevrait avant l'occurrence suivante.

La table est petite par construction : une ligne par échéance déclarée, clé sur le `kind`
du job.

| Colonne | Type | Note |
|---|---|---|
| `kind` | `varchar(191)`, clé primaire | Le `KIND` du job déclenché. La clé primaire *est* l'unicité |
| `next_run_at` | `timestamptz` | L'échéance. La réservation la compare, puis l'avance |
| `last_run_at` | `timestamptz`, nullable | Le dernier déclenchement, ou rien tant qu'il n'y en a pas eu |
| `created_at` | `timestamptz` | |
| `updated_at` | `timestamptz` | |

## Ce que fait un redémarrage

Au démarrage, le ticker réconcilie la table avec `schedules()` :

- un `kind` déclaré dans le code et absent de la table est **inséré**, son `next_run_at`
  posé à la prochaine occurrence de son expression ;
- un `kind` présent dans la table et **plus déclaré** est supprimé — sans quoi une échéance
  retirée du code resterait due pour toujours, réservée par personne ;
- un `kind` déjà connu dont le `next_run_at` est **échu** est laissé tel quel : le
  prochain tick le réserve, puis l'avance selon l'expression que le code porte désormais.
  Un redémarrage ne rejoue jamais une occurrence passée, et n'en perd jamais une non plus ;
- un `kind` déjà connu dont le `next_run_at` est **encore à venir** est comparé à la
  prochaine occurrence de son expression, calculée à l'instant. Égale — le redéploiement
  ordinaire — rien ne bouge ; différente, la ligne est déplacée sur la nouvelle occurrence
  et un log `info` nomme le `kind`, l'ancienne échéance et la nouvelle.

## Changer une expression

La dernière règle est ce qui fait qu'un changement de `schedules()` prend effet **au
démarrage suivant**. Avant elle, la table gardait l'ancienne date jusqu'au déclenchement de
l'ancienne expression : passer de `0 3 1 * *` à `*/5 * * * *` laissait le prochain tick au
premier du mois, sans un log.

Le seul cas que le démarrage laisse intact est une échéance déjà due quand il survient —
un `0 3 * * *` redémarré à 3 h 10 part au premier tick, comme il l'aurait fait sans le
changement, et ne suit la nouvelle expression qu'ensuite. La déplacer effacerait en
silence une occurrence que le processus avait déjà gagnée.

## La configuration

```toml file=examples/event-hub/config/default.toml region=scheduler
```

Un seul réglage : le temps que le ticker dort entre deux examens du calendrier. Une
échéance à la minute n'a pas besoin d'un réveil par seconde, et trente secondes bornent à
trente secondes le retard d'un déclenchement. `config/{env}.toml` et
`RBS_SCHEDULER__POLL_INTERVAL_SECS` le surchargent comme toute autre section — voir le
[guide de la configuration](./configuration.md).

Le sommeil est interruptible : sur Ctrl-C ou SIGTERM, le ticker finit le tour qu'il a
commencé — chaque échéance est une transaction courte —, n'en commence aucun autre et rend
la main, si bien que `main` ne l'attend pas trente secondes avant de sortir.

## Les tests

Le `src/modules/scheduler/tests/` engendré tourne contre une vraie base, comme tout test qui
en touche une — voir le [guide des tests](./testing.md). Cinq d'entre eux sont ceux qu'il
vaut la peine de garder si vous éditez le fragment :

- une expression à cinq champs et sa forme à six donnent la même prochaine occurrence, et
  toute autre longueur est refusée ;
- la réconciliation insère un `kind` nouveau, supprime un `kind` retiré, laisse intact le
  `next_run_at` d'un `kind` connu dont l'expression n'a pas changé, le déplace quand elle
  a changé, et laisse une échéance échue où elle est ;
- une échéance due est réservée — `next_run_at` avance, `last_run_at` est posé, et un job
  du bon `kind` est apparu dans la file ;
- **deux réservations concurrentes de la même échéance : une seule gagne**, et la file ne
  porte qu'un job ;
- une échéance non due est laissée tranquille.

Le quatrième est celui qui justifie la table. Sans lui, la garantie n'est qu'une intention.

## Ce qu'il vous laisse

- **ce que fait une échéance** — elle enfile un job, et le job est le vôtre. Le
  `demo::Log` engendré est là pour être remplacé ;
- **les heures locales** — les expressions sont en UTC, et rien ne les convertit ;
- **le rattrapage après une interruption** — une échéance manquée pendant l'arrêt du
  processus se déclenche une fois au tick suivant, et non une fois par occurrence manquée ;
- **la surveillance du calendrier** — les lignes sont là, `last_run_at` dit quand chacune
  s'est déclenchée pour la dernière fois, et rien ne les regarde.
