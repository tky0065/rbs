---
sidebar_position: 7
title: Sortir le travail long de la requête
---

# Sortir le travail long de la requête

C'est le septième des neuf tutoriels. Il reprend `demo` ; cette page n'a besoin de rien
au-delà de [Préparer le terrain](./setup.md). Le cas : une campagne de 5 000 lettres,
enfilées sans faire attendre l'appelant qu'une seule parte.

## Ce qu'il vous faut

Rien de plus qu'à [Préparer le terrain](./setup.md) : le même `demo`, sa base
joignable. Les vérifications de cette page passent par `cargo test`, pas par `curl` —
rien ici n'exige que le serveur tourne.

## 1. Installer la fonctionnalité

```bash
rbs add jobs
```

{/* rbs:transcript cmd="rbs add jobs" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add jobs
jobs : jobs en arrière-plan : une table, un enfilage transactionnel, un worker qui réessaie

plan pour …/demo

  + src/modules/jobs/mod.rs                         créé
  + src/modules/jobs/config.rs                      créé
  + src/modules/jobs/model.rs                       créé
  + src/modules/jobs/queue.rs                       créé
  + src/modules/jobs/worker.rs                      créé
  + src/modules/jobs/demo.rs                        créé
  + src/modules/jobs/tests/mod.rs                   créé
  + src/modules/jobs/tests/lease.rs                 créé
  + src/modules/jobs/tests/reservation.rs           créé
  + src/modules/jobs/tests/retry.rs                 créé
  + src/modules/jobs/tests/worker.rs                créé
  + migration/src/m20260909_090303_create_jobs.rs   créé
  ~ migration/src/lib.rs                            modifié
  + src/modules/mod.rs                              créé
  ~ src/lib.rs                                      modifié
  ~ src/main.rs                                     modifié
  ~ Cargo.toml                                      modifié
  ~ config/default.toml                             modifié
  ~ AGENTS.md                                       modifié

  13 à créer, 6 à modifier
✓ jobs installée — 13 créés, 6 modifiés

  rbs migrate up, puis `rbs generate job <nom>` pour écrire un job
```

Comme chaque brique qu'installe `rbs add`, `jobs` ne monte aucune route. Ce qu'elle
câble d'elle-même, c'est le worker : `src/main.rs` gagne une ligne `// <rbs:startup>`
qui le lance avant que le serveur ne démarre, si bien qu'il scrute déjà quand `cargo
run` répond — contre une table qui n'existe pas encore, ce qui explique justement qu'il
ne trouve rien à faire jusqu'à l'étape suivante. Enfiler quoi que ce soit reste à vous
d'écrire, sur le modèle du job `Log` de démonstration que l'installation a déjà inscrit
dans `registry()`.

Le compromis dont parle cette page se lit le mieux contre `send_detached` de [Envoyer un
courriel](./mail.md) : cet appel lance une tâche et rend la main, et un SMTP indisponible
la minute que dure l'essai perd le courriel purement et simplement — une ligne de
journal, rien de plus. Un job paie pour la garantie inverse, et le prix est réel : le
débit est borné par la base qui fait la réservation, pas par le nombre de tâches qu'un
processus peut lancer.

## 2. Appliquer la migration

```bash
git add -A && git commit -q -m "jobs installée"
rbs migrate up
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.51s
     Running `target/debug/migration up`
✓ migrations appliquées
```

La preuve que la table `jobs` existe désormais, et avec elle quelque chose que le worker
déjà lancé peut scruter : la migration qu'`add jobs` a écrite un instant plus tôt est
appliquée, et un job enfilé à partir de maintenant est une ligne, pas quelque chose tenu
seulement dans la mémoire du processus qui l'a enfilé.

## Vérifier

Le point sur lequel toute cette page repose ne quitte jamais la base, si bien qu'aucune
route montée ne le montrerait. `add jobs` l'a écrit sous forme de test, `#[ignore]` parce
qu'il exige la table qui vient d'être migrée :

```bash
cargo test modules::jobs::tests:: -- --ignored
```

```text
running 4 tests
test modules::jobs::tests::reservation::a_job_enqueued_in_a_rolled_back_transaction_does_not_exist ... ok
test modules::jobs::tests::reservation::a_job_enqueued_in_a_committed_transaction_is_visible_to_the_worker ... ok
test modules::jobs::tests::lease::a_failing_job_is_retried_then_marked_failed_after_the_last_attempt ... ok
test modules::jobs::tests::reservation::two_concurrent_workers_never_reserve_the_same_job ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.43s
```

La preuve du point tout entier : `a_job_enqueued_in_a_rolled_back_transaction_does_not_exist`
enfile dans une transaction, l'annule, et vérifie que la ligne a disparu — la file est
une table, si bien qu'`INSERT` et le rollback s'y appliquent exactement comme à toute
autre ligne que cette transaction a touchée. Son voisin lance le même appel avec
`commit` à la place, et la ligne suivante le réserve aussitôt. Déplacez la file vers
Redis, ou tout magasin hors de la base, et les deux tests seraient à réécrire : un
rollback y laisse le job debout, en désaccord avec la ligne qui l'a motivé.

## Ce qui a été installé

Trois extraits, lus depuis
[`examples/newsletter-queue`](https://github.com/tky0065/rbs/tree/main/examples/newsletter-queue) —
un projet engendré de la même façon et compilé en CI.

:::note
`newsletter-queue` porte une ressource `subscribers` avec une action `POST
/subscribers/broadcast` qui enfile une lettre par abonné confirmé. Cette action est du
code métier, pas quelque chose que `rbs generate crud` produit — `add jobs` n'installe
que la file, le trait, et le worker ci-dessous. Les trois extraits qui suivent montrent
ce à quoi ressemble le câblage d'une vraie campagne dans cette file.
:::

### Le job

`SendNewsletter` porte l'identifiant de l'abonné, et non son adresse : entre l'enfilage
et l'exécution, il a pu la corriger, et c'est celle de l'envoi qui compte.

```rust file=examples/newsletter-queue/src/modules/jobs/newsletter.rs region=job
```

### Le registre

Là où le job `Log` de démonstration ci-dessus est inscrit par l'installation elle-même,
`SendNewsletter` est inscrit à la main — la seule ligne que `registry()` demande une
fois qu'un job vous appartient.

```rust file=examples/newsletter-queue/src/modules/jobs/mod.rs region=registry
```

### L'enfilage

`broadcast` lit les abonnés confirmés et enfile un job par abonné, le tout dans une
seule transaction.

```rust file=examples/newsletter-queue/src/subscribers/service.rs region=broadcast
```

Le `&transaction` de cet appel est le point entier de la page : passez `db` à la place,
et la campagne devient deux choses capables de se contredire — des abonnés lus à un
instant commité, et des jobs qu'une panne entre la lecture et le commit pourrait encore
perdre ou dupliquer, en bloc.

## Pour aller plus loin

- [Jobs](../guides/jobs.md) couvre en entier le scrutage et le réessai du worker, et
  comment programmer un job pour plus tard avec `enqueue_at`.
- [`rbs add`](../cli/add.md) couvre les douze autres features que `demo` pourrait
  encore installer, `jobs` désormais dessus.
- [Tests](../guides/testing.md) est le harnais contre lequel `jobs/tests/` engendré
  tourne, et ce que `-- --ignored` atteint qu'un simple `cargo test` n'atteint pas.
- [Voir ce que fait l'API](./observability.md) est le tutoriel suivant : une route est
  devenue lente, et `/metrics` dit enfin depuis quand.
