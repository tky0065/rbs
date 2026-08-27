# J4 — `doctor` diagnostique l'auth

**But :** un utilisateur bloqué par l'auth lance `rbs doctor` et apprend ce qui le bloque,
au lieu de lire un échec au démarrage du binaire.

**Fichiers :** `crates/rbs-cli/src/doctor/auth.rs` (créé), `doctor/mod.rs` (le contrôle
inscrit dans `executer`).

**Spec :** `TODO.md`, tâche J4 — trois critères, plus un quatrième constat arbitré en
conception (secret resté à la valeur d'exemple).

## Contraintes

- Le contrôle n'entre dans le rapport que si `auth` figure dans
  `[package.metadata.rbs].features` : un projet sans auth n'a pas à lire une ligne
  à son sujet.
- **Un** `Controle` qui agrège ses défauts, comme `env::controler` agrège ses variables
  manquantes.
- L'environnement l'emporte sur le `.env`, comme dans `migrate::variables_du_projet`.
- Le seuil de 32 octets duplique `SECRET_MINIMUM` de `rbs-core` : les deux crates sont
  indépendantes par construction, la duplication est assumée et commentée.

## Tâche 1 — le module et ses quatre constats

- [ ] Écrire les tests d'abord, un par constat, sur un projet créé par `new::creer`
      comme le fait `env.rs` : secret absent → `Etat::Echec` et le détail nomme
      `RBS_AUTH__SECRET` ; secret de 31 octets → `Echec` ; `[auth]` retirée de
      `config/default.toml` → `Echec` ; `.env` portant la valeur de `.env.example` →
      `Echec`.
- [ ] Les voir échouer (le module n'existe pas).
- [ ] `controler(racine)` déléguant à `controler_avec(racine, env)` où
      `env: impl Fn(&str) -> Option<String>` — la closure rend l'environnement testable
      sans toucher aux variables du processus.
- [ ] La section se lit par `toml_edit`, déjà en dépendance : `doc.get("auth").is_some()`
      ne confond pas une section avec un `[auth]` en commentaire.
- [ ] Le constat « valeur d'exemple » compare `.env` à `.env.example` — aucune chaîne en
      dur, donc rien à resynchroniser si `add auth` reformule sa ligne.

## Tâche 2 — inscrire le contrôle

- [ ] `executer` lit `metadata::lire(&racine.join("Cargo.toml"))` et n'ajoute
      `auth::controler` que si la feature y est.
- [ ] Test : un projet sans `auth` n'a aucun contrôle de titre `auth` ; un projet avec
      `auth` en a un.

## Vérification

- [ ] `cargo test -p rbs-cli doctor` → tous verts, et une morsure par critère.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`.
