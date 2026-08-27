# J2 — parcours d'auth de bout en bout

**But :** prouver, contre un PostgreSQL réel et le binaire réellement lancé, que les huit
étapes du parcours d'authentification s'enchaînent — ce que les 21 tests d'auth du projet
généré, montés en `oneshot` sur le `Router` et isolés les uns des autres, ne prouvent pas.

**Fichier touché :** `crates/rbs-cli/tests/integration_auth.rs` (deux tests `#[ignore]`
ajoutés, un helper HTTP généralisé).

**Spec :** `TODO.md`, tâche J2 — critère unique : *« Le parcours entier joué contre un
PostgreSQL réel : register → login → 401 sans jeton → 403 en `user` → refresh → ancien
refresh 401 → logout → refresh 401. »*

## Contraintes

- `#[ignore]` par défaut, comme `integration_crud` — le critère l'exige.
- `CARGO_TARGET_DIR = common::cible()` sur toute compilation : sans elle, chaque test
  recompile l'arborescence entière.
- PostgreSQL 18 (`uuidv7()` dans les migrations générées).
- Le `.env` du projet reçoit `RBS_AUTH__SECRET` : `add auth` ne l'écrit que dans
  `.env.example`.

## Tâche 1 — généraliser le client HTTP

`poster_json` ne sait que poster, sans en-tête, et rend la réponse brute. Le parcours a
besoin de `GET`, d'un `Authorization`, et du corps séparé du statut.

- [ ] Remplacer par `requete(port, methode, chemin, jeton: Option<&str>, corps: Option<&str>) -> (u16, Value)`
      — statut parsé depuis la ligne de statut, corps depuis ce qui suit `\r\n\r\n`
      (`Value::Null` si vide, ce que rend le 204 du logout).
- [ ] Adapter `le_hash_n_apparait_pas_dans_les_logs_du_serveur`, son unique appelant :
      `reponse.starts_with("HTTP/1.1 201")` devient `statut == 201`.
- [ ] `cargo test -p rbs-cli --test integration_auth` → les 3 tests non ignorés passent,
      et la crate compile (le test ignoré adapté est compilé, pas joué).

## Tâche 2 — le parcours, sur projet généré

- [ ] Écrire `le_parcours_d_auth_se_joue_de_bout_en_bout` : conteneur, projet
      `new` + `add auth`, `migrate up`, `cargo build`, binaire lancé sur un port libre.
- [ ] Les huit appels, un seul compte, dans l'ordre :
      `register` 201 · `login` 200 → paire A · `GET /auth/me` sans jeton 401 ·
      `me` avec l'accès de A 200 · `refresh(A)` 200 → paire B, `me` avec l'accès de B 200 ·
      `refresh(A)` 401 · `logout(B)` 204 · `refresh(B)` 401.
- [ ] Morsure : retirer la garde `revoked_at IS NULL` du fragment → la dernière étape
      doit échouer. Puis restaurer.

L'étape 4 n'est pas dans le critère mais tient la chaîne : sans elle, un `login` qui
émettrait un jeton que la garde refuse passerait toutes les autres étapes.

## Tâche 3 — le 403, sur `examples/blog-auth`

Le fragment `add auth` ne livre aucune route admin dans le binaire ; `blog-auth` en porte
une (`POST /posts`, gardé `require_role(Role::Admin)`).

- [ ] `copier(source, destination)` récursif, sautant `target` et `.git`.
- [ ] Réécrire dans le `Cargo.toml` copié `path = "../../crates/rbs-core"` en chemin
      absolu, séparateurs normalisés en `/` — un `\` de Windows est un échappement dans
      une chaîne TOML basique.
- [ ] Réécrire le `.env` : URL du conteneur, `RBS_AUTH__SECRET`.
- [ ] `une_route_gardee_refuse_un_user_authentifie` : `register` 201 · `login` 200 ·
      `POST /posts` sans jeton 401 · `POST /posts` en `user` 403 · `GET /posts` 200.
- [ ] Morsure : retirer `identite.require_role(Role::Admin)?` de `posts::controller::create`
      → le 403 doit échouer. Puis restaurer.

La dernière ligne discrimine : sans elle, un 403 rendu par une route cassée passerait pour
une garde qui fonctionne.

## Vérification finale

- [ ] `cargo test -p rbs-cli --test integration_auth -- --ignored` → 6 passed.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`.
