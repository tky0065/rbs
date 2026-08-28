# R4 — `integration_jobs` sous conteneur : la survie au redémarrage

**Conception.** Deux tests `#[ignore]`, sur le modèle d'`integration_storage`, chacun avec
son PostgreSQL en conteneur.

Le premier lance `cargo test --workspace` puis `-- --ignored` sur le projet engendré et
vérifie nommément que les tests du fragment ont bien tourné : `cargo test -- --ignored`
sort en 0 même quand il ne filtre aucun test.

Le second prouve la survie, et le prouve littéralement : la file est garnie, le binaire du
projet est lancé puis **tué** avant son premier tour de boucle — le job est encore
`pending`, ce qui est asserté —, puis relancé ; le job passe alors à `done`. Une file en
mémoire échouerait à la première étape.

## Étapes

1. `crates/rbs-cli/tests/integration_jobs.rs`, conteneur `postgres:18`.
2. Enfilage et relevé du statut par `psql` dans le conteneur : le test du dépôt n'a ni
   sea-orm ni client SQL, et ne doit pas en gagner un.
3. Intervalle de scrutation piloté par `RBS_JOBS__POLL_INTERVAL_SECS` : long pour la phase
   « tué avant exécution », court pour la phase « relancé ».
4. Morsure : faire porter la file par une `Mutex<Vec<…>>` en mémoire — la survie tombe.
