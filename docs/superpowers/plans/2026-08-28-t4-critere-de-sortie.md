# T4 — Critère de sortie du jalon

**Conception.** Trois vérifications, dont une seule demande du code.

Le troisième critère se mesure et ne s'écrit pas : `git diff --stat` sur trois intervalles
de l'historique. Le second est déjà porté par
`integration_jobs::a_job_enqueued_before_the_process_is_killed_runs_after_the_restart`, qui
tue le processus entre l'enfilage et l'exécution puis le relance — « le projet livré » y
étant le projet engendré, comme au critère de `R4`.

Le premier ne l'est pas. `each_engine_produces_a_project_that_compiles` s'arrête à
`cargo build` pour MySQL et SQLite, ce qui ne demande aucune base ; le critère exige
`cargo test`, qui en demande une par moteur. Le test monte donc d'un cran : une base
démarrée par moteur, `migrate up`, puis `cargo test` du projet engendré.

Les démarreurs de conteneurs vivent aujourd'hui dans `integration_jobs.rs`. Deux fichiers
de tests en ayant besoin, ils remontent dans `common/` — c'est ce que ce module est.

Le test reste `#[ignore]`, comme tout ce qui demande Docker, et rejoint donc le step
`cargo test -- --ignored` de la CI.

## Étapes

1. Remonter `start_postgres`, `start_mysql`, `url_of`, `url_of_mysql` et leurs constantes
   dans `crates/rbs-cli/tests/common/mod.rs`, `integration_jobs` les y prenant désormais.
2. `each_engine_produces_a_project_that_compiles` devient
   `each_engine_produces_a_project_whose_tests_pass` : PostgreSQL et MySQL en conteneur,
   SQLite sur fichier, chacun dans sa cible de compilation, `migrate up` puis `cargo test`.
3. Preuves : le test joué ; la morsure d'une requête d'un moteur donnée à un autre, qui
   doit faire tomber ce moteur seul ; `integration_jobs -- --ignored` pour le second
   critère ; les trois `git diff --stat` pour le troisième.
