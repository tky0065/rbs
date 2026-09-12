# Plan — une expression cron modifiée prend effet au redémarrage

Tâche 16 d'`IMPROVE.md` (Easy). `sync::reconcilier` garde tel quel le `next_run_at` de
tout `kind` déjà connu : passer de `0 3 1 * *` à `*/5 * * * *` laisse le prochain tick au
1er du mois, sans log. Le but : au démarrage, une échéance **à venir** dont l'occurrence
recalculée diffère de celle stockée est déplacée ; une échéance **échue** ne l'est pas.

## Design (validé, sans colonne ni migration)

Pour chaque `(kind, prochaine)`, `prochaine` étant l'occurrence fraîche tronquée à la
seconde et `maintenant = a_la_seconde(Utc::now())` le même instant que celui des
horodatages :

- ligne absente → insertion tolérante au conflit (inchangé) ;
- ligne présente et `next_run_at <= maintenant` → gardée : elle part au prochain tick, et
  la réservation l'avancera selon la nouvelle expression ;
- ligne présente, à venir, `next_run_at == prochaine` → gardée (le « redeploy ») ;
- ligne présente, à venir, différente → `update_many().col_expr(NextRunAt, prochaine)
  .col_expr(UpdatedAt, maintenant)` filtré sur `kind` **et** sur l'ancien `next_run_at`
  (un ticker qui aurait réservé entre la lecture et l'écriture n'est pas écrasé), plus un
  `tracing::info!` portant `kind`, ancienne et nouvelle échéance.

Fichiers touchés : `crates/rbs-cli/templates/features/scheduler/{sync,tests}.rs.jinja`,
`crates/rbs-cli/tests/integration_scheduler.rs`, `docs/docs/guides/scheduler.md` et sa
traduction (la section « Changing an expression » conseille un `DELETE` qui n'a plus lieu
d'être).

## Étapes (TDD)

- [ ] 1. `tests.rs.jinja` : `a_changed_expression_moves_the_next_occurrence` — réconcilie
  avec `0 3 1 * *`, lit `next_run_at`, réconcilie avec `*/5 * * * *`, exige une échéance
  différente et à moins de cinq minutes de `Utc::now()`.
- [ ] 2. `tests.rs.jinja` : `a_due_schedule_is_not_moved_by_a_changed_expression` —
  `echeance_due` (ligne à `now - 1 h`), réconcilie avec `*/5 * * * *`, exige
  `next_run_at` inchangé : c'est le tick, pas la réconciliation, qui la rejoue.
- [ ] 3. `integration_scheduler.rs` : `TESTS_SOUS_CONTENEUR` passe à 9 entrées avec les
  deux noms ; le commentaire « trois des dix… les sept autres » suit (douze, neuf).
- [ ] 4. Rouge : `cargo test -p rbs-cli --test integration_scheduler --no-fail-fast`
  redirigé vers le scratchpad — `the_tests_shipped_with_the_fragment_run_against_a_real_database`
  doit échouer sur le premier nouveau test.
- [ ] 5. `sync.rs.jinja` : la boucle devient `if let Some(connue) = find_by_id…` avec les
  trois cas ci-dessus, et le commentaire « Une échéance déjà connue garde… » est réécrit
  pour dire pourquoi seule une échéance à venir et divergente bouge.
- [ ] 6. Vert : même commande, les deux tests d'intégration passés, les neuf noms exigés.
- [ ] 7. `docs/docs/guides/scheduler.md` + FR : « What a restart does » et « Changing an
  expression » disent la règle nouvelle ; le `DELETE` disparaît.
- [ ] 8. `cargo test -p rbs-cli --lib scheduler`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo fmt --all --check`, `cargo test -p rbs-cli --test
  integration_examples` (aucun exemple ne porte `scheduler` : confirme qu'il reste vert).
