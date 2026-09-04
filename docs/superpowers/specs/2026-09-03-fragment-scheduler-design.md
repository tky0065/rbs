# `rbs add scheduler`

**Tâche 77 d'`IMPROVE.md`.** Le fragment `jobs` sait exécuter un travail hors du cycle
d'une requête, réessayer et abandonner — mais **rien ne l'enfile sinon un événement**.
Une purge nocturne, un rapport quotidien, une relance d'abonnement n'ont aucun déclencheur
dans un projet engendré : il faut un `cron` système hors du dépôt, qui appelle une route
qu'on écrit pour lui.

## Ce qui est décidé

**Le scheduler déclenche, il n'exécute pas.** Un tick réserve une échéance due, puis
enfile un job dans la table `jobs` ; le worker fait le reste. Les réessais, le registre,
la journalisation et l'exécution existent déjà et sont éprouvés — les réécrire pour la
seule raison qu'ils seraient déclenchés par une horloge donnerait deux boucles à maintenir
au lieu d'une.

Le fragment déclare donc `requires = ["jobs"]` dans son manifeste. Le mécanisme existe
(`manifest.rs:46`, résolu par `add/mod.rs:396`) et **installe la dépendance manquante**
plutôt que de refuser : `rbs add scheduler` sur un projet nu pose `jobs` puis `scheduler`.

### Un seul réplica gagne une échéance

C'est la seule difficulté réelle du fragment. Trois instances de l'API, c'est trois
tickers ; la purge nocturne ne doit tourner qu'une fois.

Une échéance se réserve par un `UPDATE` conditionnel :

```sql
UPDATE schedules
SET next_run_at = ?, last_run_at = ?, updated_at = ?
WHERE kind = ? AND next_run_at <= ?
```

`rows_affected == 1` désigne le gagnant ; les perdants voient zéro. La condition est
évaluée sous le verrou de ligne que l'`UPDATE` pose lui-même : le second ticker attend le
commit du premier, relit `next_run_at` déjà avancé, et n'affecte rien.

**Ce fragment n'écrit donc aucun SQL par moteur**, contrairement à `queue.rs.jinja` et ses
trois dialectes. Il n'en a pas besoin : la clause vise une clé primaire, il n'y a rien à
« sauter » — pas de `SKIP LOCKED`, pas d'`UPDATE … RETURNING`. Ce qui rendait le dépilage
d'un job difficile, c'est qu'il fallait *élire* une ligne parmi beaucoup ; ici la ligne
est nommée.

### La réservation et l'enfilage partagent une transaction

`begin` → `UPDATE` conditionnel → si la ligne est gagnée, `jobs::enqueue(&txn, …)` →
`commit`. Sans cette transaction, un crash entre les deux perd l'exécution — l'échéance
est avancée, le job n'est jamais né, et personne ne s'en aperçoit avant le lendemain.

`enqueue` prenant un `ConnectionTrait` (`features/jobs/queue.rs.jinja:80-88`), il n'y a
rien à ajouter au fragment `jobs` pour cela.

### Le calendrier est déclaré en code, la base ne porte que l'état

```rust
pub fn schedules() -> Vec<Schedule> {
    vec![Schedule::every::<demo::Log>("0 3 * * *", || demo::Log {
        message: "purge nocturne".into(),
    })]
}
```

Symétrique de `jobs::registry()`, et pour les mêmes raisons. `Schedule::every::<J>` tire le
`kind` de `J::KIND` : **une échéance pointant vers un job non inscrit est impossible à
écrire**. Déclarer le calendrier en base l'aurait rendue possible — un `kind` mal
orthographié y échouerait chaque nuit sans que rien ne le dise — et aurait sorti la
configuration du dépôt, où elle se relit et se révise.

Le prix est qu'un changement de calendrier passe par un déploiement. C'est le prix normal
d'une configuration versionnée.

La fabrique (`|| demo::Log { … }`) construit la charge utile à chaque déclenchement plutôt
qu'une fois pour toutes : une charge qui porte une date la veut à l'instant du tick.

### Cinq champs comme six

La crate `cron` attend six champs, la seconde en tête : `0 0 3 * * *`. Le crontab Unix en a
cinq, et `0 3 * * *` **échoue** — vérifié par exécution contre `cron 0.17.0`.

Le fragment accepte les deux : une expression à cinq champs est préfixée de `0 `, ce que le
crontab Unix veut dire de toute façon. Six champs passent intacts, toute autre longueur est
refusée. Sans cela, chaque utilisateur se brûle une fois sur un piège qui n'est pas le
sien — coller une ligne de son crontab est le premier geste évident.

Les échéances se calculent **en UTC**, ce que la documentation dit : `0 3 * * *` est 3 h
UTC, pas 3 h à Paris.

### Une expression illisible arrête le démarrage

Au boot, `sync` compile chaque expression. Une seule invalide, et le processus s'arrête en
la nommant. L'alternative — retirer l'échéance fautive et continuer — donne un service qui
paraît sain et dont une tâche ne tourne jamais, le mode de panne le plus coûteux à
diagnostiquer. C'est un écart assumé avec `jobs::worker::spawn`, qui, lui, laisse l'API
répondre quand sa configuration est illisible : là-bas le service HTTP est en jeu, ici une
liste statique que le développeur vient d'écrire.

## La table

Migration `create_schedules`, table `schedules` :

| Colonne | Type | Note |
|---|---|---|
| `kind` | `text` PK | Le `K::KIND` du job déclenché. La clé primaire *est* l'unicité |
| `next_run_at` | `timestamptz` | L'échéance. La réservation la compare et l'avance |
| `last_run_at` | `timestamptz` null | Le dernier déclenchement, ou rien tant qu'il n'y en a pas |
| `created_at` | `timestamptz` | |
| `updated_at` | `timestamptz` | |

Aucun index de plus : toute lecture passe par la clé primaire ou balaie une table qui
compte autant de lignes que le projet a d'échéances déclarées.

L'instant écrit est **tronqué à la seconde**, comme `queue.rs.jinja:a_la_seconde` :
MySQL rend `timestamp` sans partie fractionnaire et *arrondit* ce qu'on y écrit, ce qui
placerait une échéance à `…34,6 s` dans son propre futur.

## La réconciliation au démarrage

`sync` aligne la table sur `schedules()` :

- un `kind` déclaré et absent de la table est inséré, `next_run_at` à la prochaine
  occurrence de son expression ;
- un `kind` présent dans la table et **plus déclaré** est supprimé — sans quoi une
  échéance retirée du code resterait due pour toujours, jamais réservée par personne ;
- un `kind` déjà présent garde son `next_run_at`. Le redémarrage ne rejoue pas une
  échéance passée et ne repousse pas une échéance imminente ; c'est ce qui rend un déploiement
  invisible pour le calendrier.

Le troisième point vaut d'être dit à l'envers : **changer l'expression d'une échéance
existante ne prend effet qu'à son prochain déclenchement.** La table garde l'ancienne
échéance jusque-là. La documentation le dit, et la manœuvre pour forcer — supprimer la
ligne — tient en une phrase.

## Les fichiers

```
templates/features/scheduler/
  feature.toml         requires = ["jobs"], dépendance `cron`, ancres `features` et `startup`, section [scheduler]
  mod.rs.jinja         `Schedule`, `Schedule::every`, `schedules()` — la liste à éditer
  config.rs.jinja      section `[scheduler]` : `poll_interval_secs`
  model.rs.jinja       l'entité `schedules`
  sync.rs.jinja        la réconciliation au démarrage
  ticker.rs.jinja      la boucle, la réservation, l'enfilage
  migration.rs.jinja   la table
  tests.rs.jinja       les tests livrés avec le fragment
```

L'ancre `startup` porte `{@ crate_path @}::scheduler::spawn(state.clone());`, comme celle
de `jobs` — `crate_path` et non `crate_name`, pour la raison qu'énonce
`features/jobs/feature.toml`.

`[scheduler] poll_interval_secs = 30` : une échéance à la minute n'a pas besoin d'un
réveil par seconde, et trente secondes bornent le retard de déclenchement à trente
secondes. Le champ est là pour être changé sans rouvrir le fragment.

## Ce que le CLI doit apprendre

- `cli.rs:56` : `scheduler` rejoint la liste des features de l'aide du drapeau.
- `lib.rs:451` : le conseil post-installation — la migration, puis la liste à éditer dans
  `src/scheduler/mod.rs`.
- `docs/docs/cli/add.md:283` fige le message d'erreur énumérant les features
  installables ; il change avec son transcript.

La dépendance `cron` est déclarée par `[[dependencies]]` du manifeste, comme
`async-trait` l'est pour `jobs`. Version **0.17**, résolue à l'instant contre l'index ;
elle tire `chrono`, que le projet engendré porte déjà.

## Tests

**Du fragment, dans le projet engendré** (`tests.rs.jinja`) :

1. Une expression à cinq champs et son équivalent à six donnent la même prochaine
   échéance ; une expression à quatre champs est refusée.
2. `sync` insère un `kind` nouveau, supprime un `kind` retiré, et **laisse intact le
   `next_run_at` d'un `kind` déjà connu**.
3. Une échéance due est réservée : `next_run_at` avance à l'occurrence suivante,
   `last_run_at` est posé, et **un job du bon `kind` est apparu dans la file**.
4. **Deux réservations concurrentes de la même échéance : une seule gagne**, et la file ne
   porte qu'un job. C'est la garantie qui justifie la table ; sans ce test elle n'est
   qu'une intention.
5. Une échéance non due n'est pas réservée.

**Du CLI** (sans Docker) : le manifeste déclare `requires = ["jobs"]`, la migration et les
deux ancres ; le rendu des templates est un point fixe de `rustfmt`.

**D'intégration** (sous Docker) :

6. `rbs add scheduler` sur un projet nu installe **`jobs` puis `scheduler`**, et le projet
   compile et passe `clippy -D warnings`.
7. Les deux migrations s'appliquent et la suite du projet engendré passe.

## Documentation

- `docs/docs/guides/scheduler.md` et sa version française : déclarer une échéance, les
  deux formes d'expression, l'UTC, le comportement au redémarrage et au changement
  d'expression, et le fait qu'un seul réplica déclenche.
- La ligne du tableau de `docs/docs/cli/add.md` et sa version française, en disant que le
  fragment entraîne `jobs`.
- `docs/docs/guides/jobs.md` gagne le renvoi vers le scheduler à l'endroit où elle dit
  qu'un job s'enfile sur événement.
- Les deux transcripts qui énumèrent les features installables.
