# Ancre optionnelle sur fichier absent : sauter l'insertion, l'annoncer — plan d'implémentation

**Goal :** `rbs add mail`, `redis`, et par transitivité `auth` et `webhooks`, aboutissent sur un projet sans `docker-compose.yml` (SQLite, base distante, projet d'avant la 1.1.0) au lieu d'échouer sur « docker-compose.yml est introuvable ». Le service que le compose aurait reçu est annoncé, bloc YAML à l'appui, y compris en `--dry-run`.

**Architecture :** `anchors::SERVICES` est déjà `optional: true`, mais `plan::Builder::insert` l'ignore. Le plan reçoit une liste d'insertions *sautées* (`Sautee { anchor, lines }`), remplie par `insert` quand l'ancre est optionnelle et que `states(&path).courant` est `None` — un fichier projeté plus tôt dans le même plan (le fragment `docker` écrit le compose puis y insère) compte donc comme présent. `render::plan` affiche ces insertions après le pied du tableau ; `rbs new --with` n'affichant pas de plan, `InstalledFeature` remonte les mêmes lignes. Ancre obligatoire absente → toujours `FichierAbsent` ; fichier présent sans ancre → toujours `Error::Anchor`.

**Spec :** design validé (tâche 4 d'`IMPROVE.md`), tâche bornée, sans document de spec.

## Étapes (TDD)

- [x] 1. `plan/mod.rs` : test rouge (E0599 sur `sautees`, puis les trois cas) — `SERVICES` sur projet vide planifie sans erreur, une insertion sautée nommant l'ancre et portant les lignes, aucun fichier ni action ; `ROUTES` absent reste `FichierAbsent` (test existant) ; `SERVICES` présent sans ancre reste `Anchor` ; compose projeté par `create` puis `SERVICES` inséré → pas de saut.
- [x] 2. (`cargo test -p rbs-cli --lib plan::` : 62 passed) `plan/mod.rs` : `Sautee`, champ `sautees` sur `Builder` et `Plan`, accesseur, branche dans `insert`. Vert.
- [x] 3. (2 failed puis `plan::` 64 passed) `plan/render.rs` : test rouge — le rendu d'un plan portant une insertion sautée nomme le fichier, l'ancre et le bloc, après le pied ; un plan sans fichier mais avec un saut ne dit pas seulement « rien à faire ». Puis `render::sautees` + appel dans `plan`. Vert.
- [x] 4. (1 passed, vert d'emblée) `add/mod.rs` : test rouge — `plan_for(mail)` sur `project_on(Database::Sqlite)` (sans compose) planifie, le rendu nomme `docker-compose.yml` et `mailpit`, le plan ne crée pas de compose. Vert sans changement (preuve que 2 suffit).
- [x] 5. (`cargo test -p rbs-cli --lib` : 1144 passed, 16 ignored) `new.rs` / `lib.rs` : `InstalledFeature.sautees`, affiché sous la ligne `+ feature` de `rbs new`.
- [x] 6. (`--test integration_add adding_auth_to_a_sqlite` : 1 passed en 2,6 s ; exécution manuelle du binaire sur un projet SQLite : code 0, message présent, aucun compose) `tests/integration_add.rs` (sans Docker, non `#[ignore]`) : `rbs new --database sqlite` puis `rbs add auth --dry-run` (message, aucun compose écrit) puis `rbs add auth` (succès, message, aucun compose).
- [x] 7. (fmt OK ; clippy Finished sans avertissement ; lib 1144 passed/16 ignored ; integration_add 14 passed ; integration_docs 13 passed/1 ignored ; integration_examples 19 passed) Vérifications : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p rbs-cli --lib`, `cargo test -p rbs-cli --test integration_add`, `cargo test -p rbs-cli --test integration_docs`, `cargo test -p rbs-cli --test integration_examples`.
- [ ] 8. Commit `fix(add): …`, corps avec le pourquoi et `Vérifications :`.
