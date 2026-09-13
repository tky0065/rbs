# `--with-upload` livre les tests de ses trois routes de contenu

Date : 2026-09-12
Portée : `crates/rbs-cli/templates/feature/tests.rs.jinja`,
`crates/rbs-cli/src/generate/tests_http.rs`, `crates/rbs-cli/tests/integration_crud.rs`,
`crates/rbs-cli/tests/integration_auth.rs`, `examples/file-drop/src/uploads/tests.rs`,
guide `storage` et page `rbs generate` en deux langues, `CHANGELOG` en deux langues.
Hors portée : la scission de `tests.rs.jinja` (tâche 68 du backlog, qui la décidera pour
tout le fichier), un test du contenu après `DELETE` sous `--soft-delete` (le comportement
« ligne estampillée, contenu gardé » est documenté, pas encore éprouvé), un test S3 des
routes de contenu (la ronde du fragment `storage` couvre déjà la substituabilité).

## Le problème

`rbs generate crud <nom> --with-upload` écrit `put_content`, `get_content` et
`head_content`, les monte sur `/<nom>/{id}/content` derrière `TAILLE_MAX`, et n'écrit
**aucun** test pour eux : `grep -c with_upload templates/feature/tests.rs.jinja` rend 0.
Seul `integration_crud::the_deposited_content_round_trips_through_the_running_server`
les exerce, côté rbs, par des requêtes HTTP écrites à la main. L'utilisateur qui modifie
son contrôleur et lance `cargo test -- --include-ignored` ne verra jamais une régression
sur PUT/GET/HEAD, sur le 404 d'une ligne sans contenu, ni sur le 401 sous `auth`.

## Ce qui est engendré

Un bloc `{% if with_upload %}` dans `tests.rs.jinja`, en fin de fichier, avec deux aides
et trois à quatre scénarios selon les drapeaux :

- `binary(method, path, body: Vec<u8>) -> Request<Body>` : la requête au corps
  `application/octet-stream`, avec le `bearer()` sous `auth` — le pendant de `request`,
  qui n'envoie que du JSON.
- `call_raw(api, request) -> (StatusCode, Option<String>, Vec<u8>)` : statut,
  `Content-Type` et corps **tel quel**. `call` lit le corps comme du JSON ; le contenu
  déposé est binaire, et c'est l'octet rendu qui se compare.

Scénarios, tous `#[ignore = "joint la base du projet"]` comme leurs voisins :

1. `the_content_round_trips_through_put_get_and_head` — *sous `creatable`* : POST une
   ligne ; HEAD et GET `/content` rendent 404 (aucun contenu encore) ; PUT d'un corps
   binaire (octets non UTF-8 compris) rend 204 ; GET rend 200, `application/octet-stream`
   et le corps à l'identique ; HEAD rend 204 ; un second PUT remplace, et GET rend le
   nouveau corps ; DELETE la ligne.
2. `an_unknown_id_has_no_content` — toujours : PUT, GET et HEAD sur un UUID jamais créé
   rendent 404 chacun. Le PUT surtout : c'est lui qui, sans la lecture préalable de la
   ligne, déposerait un objet qu'aucune ressource ne réclame.
3. `a_content_beyond_the_limit_returns_413` — toujours : un corps de `TAILLE_MAX + 1`
   octets est refusé en 413. Le test lit `super::TAILLE_MAX`, la constante que `mod.rs`
   engendre : relever la borne garde le test juste. Le corps est bâti en mémoire et joué
   par `oneshot`, sans réseau.
4. `an_anonymous_content_request_returns_401` — *sous `auth`* : PUT puis GET sans jeton
   rendent 401, avant toute lecture de ligne.

Sous une référence requise (`creatable` faux), le cycle (1) tombe comme les autres
scénarios qui créent ; 2, 3 et 4 restent, aucun ne créant de ligne.

`tests_http::render` passe `with_upload => feature.with_upload` au contexte — le
rendu ne le recevait pas.

### Rejeté

- **Un fichier `tests_content.rs.jinja` à part.** Le fichier fait 508 lignes et la tâche
  68 le note ; mais `application()`, `call`, `token()` et le `OnceLock` du jeton sont
  privés au module `tests`, et un second module devrait ou les dupliquer, ou les exposer
  en `pub(super)` — et le jeton doit rester unique par binaire de test. La scission
  concerne tout le fichier, pas ce bloc : elle se décide en une fois, à la tâche 68.
- **Tester le contenu après `DELETE` sous `--soft-delete`.** Ce serait un cinquième
  scénario sous un troisième drapeau, pour un comportement que le guide documente déjà ;
  hors périmètre de ce bug, qui est l'absence de tout test.
- **Comparer le `Content-Type` rendu à celui envoyé.** Le trait `Storage` ne garde que
  des octets et `get_content` rend toujours `application/octet-stream` : c'est cette
  constante que le test attend, et non un écho de l'en-tête reçu.

## Ce qui l'exige côté rbs

- `integration_crud::the_deposited_content_round_trips_through_the_running_server`
  (SQLite, sans conteneur) lance en plus `cargo test --workspace -- --include-ignored` dans
  le projet engendré et **exige nommément** `attachments::tests::<scénarios 1, 2, 3> ... ok`
  dans sa sortie, comme le banc PostgreSQL le fait pour le filtre.
- `integration_auth::the_tests_of_a_crud_generated_under_auth_pass` reçoit `add storage`
  avant `generate crud articles --role admin --with-upload`, et exige
  `articles::tests::an_anonymous_content_request_returns_401 ... ok` et le cycle de
  contenu en plus des trois scénarios qu'il nomme déjà. C'est le seul banc où la branche
  `auth` du bloc compile et joue ; sans lui, elle n'aurait aucun oracle.
- Tests unitaires de rendu dans `tests_http.rs` : le bloc apparaît avec `uploading()`,
  n'apparaît pas sans ; le refus anonyme du contenu n'apparaît que sous `auth` ; le rendu
  `uploading().authenticated()` est déjà ce que rustfmt écrirait (`bench::formatted`) —
  aucun exemple ne rend cette combinaison.
- `examples/file-drop/src/uploads/tests.rs` n'est pas édité à la main : il se remplace
  par le rendu neuf, et `integration_examples` le compare.

## Documentation

- Guide `storage`, section des routes engendrées (en, fr) : une phrase sur les tests
  livrés avec le drapeau, et ce qu'ils attestent.
- Page `rbs generate`, ligne `--with-upload` du tableau (en, fr) : les tests font partie
  de ce que le drapeau écrit.
- `CHANGELOG` 1.5.0, *Fixed* / *Corrigé*, un item par langue.
