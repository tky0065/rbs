# O3 — `doctor` diagnostique les trois features

**But :** un utilisateur bloqué par `cache`, `mail` ou `storage` lance `rbs doctor` et
apprend ce qui le bloque, au lieu de lire un échec au démarrage du binaire. Leçon de `J4`,
étendue aux trois features du jalon v0.3.

**Fichiers :** `crates/rbs-cli/src/doctor/redis.rs`, `mail.rs`, `storage.rs` (créés),
`doctor/mod.rs` (les trois contrôles inscrits dans `run`).

**Spec :** `TODO.md`, tâche O3 — trois critères, plus deux constats arbitrés en
conception (mot de passe SMTP vide alors que `smtp_user` est renseigné ; clés S3 restées
à leur valeur d'exemple).

## Contraintes

- Un contrôle n'entre au rapport que si sa feature figure dans
  `[package.metadata.rbs].features`. Les noms déclarés sont `redis`, `mail`, `storage` —
  et non `cache`, que le TODO emploie pour désigner la même feature : son module et sa
  section s'appellent `cache`, son fragment et sa déclaration s'appellent `redis`. Le
  titre du contrôle est **`redis`**, comme `auth` porte le nom de sa feature déclarée.
- **Un** `Check` par feature, qui agrège ses défauts, comme `auth::check` et `env::check`.
- L'environnement l'emporte sur le `.env`, comme dans `auth::check_with` : la closure
  `env: impl Fn(&str) -> Option<String>` garde l'environnement testable sans toucher aux
  variables du processus.
- Les sections se lisent par `toml_edit` sur `config/default.toml` seul. La cascade
  `config/<env>.toml` reste hors de portée du CLI, qui ne sait pas quel `RBS_ENV`
  l'utilisateur emploiera — limite à commenter dans chaque module.
- `env::check` signale déjà toute variable de `.env.example` absente du `.env`.
  Le contrôle `mail` en redit une (le critère l'exige, et son message dit *pourquoi* la
  variable compte), mais porte surtout le défaut qu'`env` ne peut pas voir.

## Tâche 1 — `redis`, la section `[cache]`

- [ ] Tests d'abord, sur un projet créé par `new::create` comme le fait `auth.rs` :
      `[cache]` retirée de `config/default.toml` → `State::Echec`, le détail nomme
      `[cache]` ; `[cache]` mise en commentaire → `Echec` (une section commentée n'est pas
      une section) ; section en place → `State::Bon`.
- [ ] Les voir échouer (le module n'existe pas).
- [ ] `redis.rs` : `check(root)` lit `config/default.toml` par `toml_edit` et vérifie
      `document.get("cache").is_some()`. Remède : le bloc `[cache]` à coller, tiré du
      `[[config]]` de `templates/features/redis/feature.toml`.

## Tâche 2 — `mail`, le mot de passe SMTP

- [ ] Tests d'abord : `RBS_MAIL__SMTP_PASSWORD` absente du `.env` et de l'environnement →
      `Echec`, le détail nomme la variable ; `smtp_user` renseigné dans `[mail]` et mot de
      passe vide → `Echec` ; `smtp_user` vide et mot de passe vide → `Bon` (Mailpit local,
      ce que le fragment déclare lui-même) ; section `[mail]` absente → `Echec` ; mot de
      passe venu de l'environnement seul → `Bon`.
- [ ] Les voir échouer.
- [ ] `mail.rs` : `check` délègue à `check_with(root, env)`. Le mot de passe se lit
      `env(CLE).or_else(|| dotenv::value(&fichier, CLE))`. `smtp_user` se lit dans
      `[mail]` par `toml_edit`. Les défauts s'agrègent, les remèdes aussi.

## Tâche 3 — `storage`, le bucket du backend S3

- [ ] Tests d'abord : `backend = "s3"` sans `bucket` en config ni en environnement →
      `Echec`, le détail nomme `bucket` ; `backend = "s3"` avec `RBS_STORAGE__BUCKET`
      renseignée → `Bon` (le bucket a deux sources) ; `backend = "fs"` sans bucket →
      `Bon` (le bucket n'a pas de sens hors S3) ; `backend = "s3"` avec les clés restées
      à `changez-moi` → `Echec` ; section `[storage]` absente → `Echec`.
- [ ] Les voir échouer.
- [ ] `storage.rs` : même forme que `mail.rs`. Le backend et le bucket se lisent dans
      `[storage]`, le bucket se replie sur `RBS_STORAGE__BUCKET` puis sur le `.env`. Les
      valeurs d'exemple se comparent à `.env.example` et non à une chaîne écrite ici,
      comme `auth::check_with` compare le secret.

## Tâche 4 — inscrire les trois contrôles

- [ ] `run` remplace ses `if installed_feature` par une boucle sur un tableau
      `[("auth", auth::check), ("redis", redis::check), ("mail", mail::check),
      ("storage", storage::check)]` : la règle est la même pour les quatre, et l'écrire
      une fois vaut mieux que quatre fois.
- [ ] Tests : un projet sans les trois features n'a aucune de leurs lignes ; un projet
      qui les déclare toutes reçoit les trois contrôles, dans cet ordre.

## Preuves à produire

- `cargo test -p rbs-cli doctor::` — les tests neufs et les 46 de `J4` inchangés.
- Les trois critères relus sur la **sortie réelle** du binaire dans un projet doté par
  `rbs add redis`, `rbs add mail` et `rbs add storage`, comme `J4` l'a fait.
- Une morsure par constat : chacune ne doit rendre rouge que son propre test.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`.
