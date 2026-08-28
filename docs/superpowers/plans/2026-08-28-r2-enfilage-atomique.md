# R2 — Enfilage typé et atomicité avec le métier

**Conception.** `enqueue` prend `&impl ConnectionTrait` et non `&DatabaseConnection`.
C'est la seule ligne qui compte : `DatabaseTransaction` implémente `ConnectionTrait`, donc
un appelant qui tient une transaction l'y passe et le job vit ou meurt avec elle. Rien
d'autre — ni verrou, ni compensation — n'est nécessaire, et c'est là toute la justification
d'avoir choisi la table contre Redis.

Le typage vient d'un trait `Job` portant `const KIND` et `run`, et bornant `Serialize` :
`enqueue(&txn, &SendWelcome { .. })` sérialise le payload une fois, sans que l'appelant
nomme la file ni la colonne.

## Étapes

1. Deux tests `#[ignore]` livrés au projet, dans `src/jobs/tests.rs` :
   - un job enfilé dans une transaction **annulée** n'existe pas après le rollback ;
   - un job enfilé dans une transaction committée est visible du dépilage.
2. Les voir échouer avant que `enqueue` n'existe.
3. `enqueue<C: ConnectionTrait, J: Job>` dans `src/jobs/queue.rs`.
4. Morsure : remplacer `&txn` par `&db` dans le test de rollback — le test doit tomber.
