# S3 — `reserver_prochain_job` à trois branches

**Conception.** `R3` avait isolé le dépilage dans une seule fonction pour que ce lot n'ait
qu'un corps à écrire, et `the_dequeue_appears_in_a_single_place_of_the_jobs_fragment`
l'épingle. Les trois branches vivent donc **dans** cette fonction, triées à l'exécution
sur `db.get_database_backend()` : le fragment livre les trois, le projet n'en emprunte
qu'une. Un branchement de gabarit rendrait ce test faux et obligerait `rbs add jobs` à
connaître le moteur.

PostgreSQL ne bouge pas. SQLite prend la même requête moins `FOR UPDATE SKIP LOCKED`,
en `?` : un `UPDATE` isolé y est sa propre transaction immédiate, SQLite ne laissant
écrire qu'un processus à la fois, et le `busy_timeout` de sqlx fait attendre celui qui est
bloqué plutôt que de le faire échouer.

MySQL 8 est le cas dur, et pas pour la raison attendue : il connaît `SKIP LOCKED`, mais
l'erreur 1093 lui interdit `UPDATE jobs … WHERE id = (SELECT … FROM jobs …)`, et il n'a
pas d'`UPDATE … RETURNING`. La requête unique y est donc impossible. Il faut une
transaction — `SELECT … FOR UPDATE SKIP LOCKED LIMIT 1`, l'`UPDATE`, la relecture, le
commit. C'est le verrou posé par le `SELECT` et tenu jusqu'au commit qui interdit à deux
workers la même ligne, exactement ce que `SKIP LOCKED` fait ailleurs.

**Le test de concurrence de `R3` vit dans le projet engendré** : le porter sur trois bases,
c'est engendrer trois projets. Chacun compile dans sa cible propre, comme `S1` l'a établi —
les features `sea-orm` diffèrent et une cible commune ferait tout recompiler à chaque
bascule. MySQL tourne en CI comme les autres `#[ignore]`, sur arbitrage : le job Linux joue
déjà `cargo test -- --ignored`, et un test qui ne tourne jamais est un test qui pourrit.

## Étapes

1. `queue.rs.jinja` : `reserver_prochain_job` à trois branches sur
   `db.get_database_backend()`, les trois requêtes en constantes voisines.
2. Test unitaire sur le gabarit rendu : les trois moteurs y paraissent, et
   `SKIP LOCKED` deux fois — PostgreSQL et MySQL, jamais SQLite.
3. `integration_jobs` : `the_tests_shipped_with_the_fragment_run_against_a_real_database`
   passe en boucle sur les trois moteurs, cible par moteur, chacun exigeant
   `two_concurrent_workers_never_reserve_the_same_job ... ok`.
4. Morsures : `SKIP LOCKED` retiré → PostgreSQL **et** MySQL tombent ; l'`UPDATE` de MySQL
   sorti de sa transaction → MySQL tombe seul.
5. Preuves : les trois moteurs verts ; les deux morsures rouges là où elles doivent l'être.
