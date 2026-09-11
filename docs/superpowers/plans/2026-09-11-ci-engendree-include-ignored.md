# CI engendrée : exécuter les tests qui joignent la base — plan d'implémentation

**Goal:** La CI qu'installe `rbs add ci` monte une base et migre, puis lance
`cargo test --workspace` : tout test d'un CRUD ou d'un fragment étant `#[ignore = "joint la
base du projet"]`, elle est verte à vide. Le workflow passe à
`cargo test --workspace --no-fail-fast -- --include-ignored`, et le guide des tests cesse de
prescrire un `cargo test` nu.

**Spec:** design validé en chat (tâche 8 d'`IMPROVE.md`), tâche *bounded*.

## Étapes (TDD)

- [x] 1. Test rouge dans `crates/rbs-cli/src/add/mod.rs` (tests du fragment `ci`) : le
  workflow rendu contient `-- --include-ignored` et `--no-fail-fast`. Vu échouer :
  `cargo test -p rbs-cli --lib the_workflow_runs` → 1 failed, avant la template.
- [x] 2. `templates/features/ci/.github/workflows/ci.yml.jinja` : nouvelle commande, commentaire
  de *pourquoi* sur `--include-ignored` ; celui de l'étape `migrations` reste exact. Test vert.
- [x] 3. `docs/docs/guides/testing.md` « Running them » et son miroir FR : `cargo test --
  --include-ignored`, avec la raison. `docs/docs/cli/add.md` ne transcrit pas la ligne
  (grep vide), les templates `agents/{fr,en}.md.jinja` ne contredisent pas la CI : inchangés.
- [x] 4. `grep -rl include-ignored examples/` et `grep -l '"ci"' examples/*/Cargo.toml` vides :
  aucun exemple à régénérer.
- [x] 5. Vérifications : fmt, clippy, `cargo test -p rbs-cli --lib`, `integration_examples`,
  `integration_docs`, `npm run typecheck` et `npm test` sous `docs/` (commandes de
  `.github/workflows/docs.yml`).
- [x] 6. Commit `fix(ci): …` sur la branche du worktree.
