# Secrets hors des journaux, `.env` fermé, CI épinglée — plan d'implémentation

> **Pour l'agent :** SOUS-SKILL REQUIS : superpowers:executing-plans. Les étapes se cochent (`- [ ]`).

**But :** que le mot de passe Redis n'atteigne plus le journal, que tout `.env` écrit par
rbs soit en 0600, et que la CI engendrée épingle ses actions par SHA, tenues à jour par
Dependabot.

**Design validé** (le 2026-09-14, par le mainteneur) :
- Redis : `rbs-core` expose le masquage de `db.rs`, `pub fn rbs_core::db::redact_url(url: &str) -> String` —
  additif, dans la 1.5.0 non publiée ; les projets engendrés en 1.5.0 dépendent de
  `rbs-core` 1.5.0. Les fragments `redis` et `rate-limit` l'appellent.
- `.env` : tout `.env` que rbs écrit finit en 0600 sous Unix, **y compris** un `.env`
  existant en 0644 — c'est ce qui protège un projet antérieur au prochain `rbs add auth`.
  Aucune fenêtre où le secret est écrit en 0644 : droits posés sur le descripteur avant
  l'écriture du contenu.
- CI : trois actions épinglées par SHA avec la version en commentaire, et un
  `.github/dependabot.yml` (github-actions, hebdomadaire) déposé par le fragment `ci`.

## Contraintes globales

- Lire `CLAUDE.md` à la racine : commentaires = le *pourquoi* ; `#![warn(missing_docs)]`
  sur `rbs-core` (un `///` d'une à trois lignes) ; doc modifiée en anglais **et** en
  français dans le même commit ; avant d'ajouter à `rbs-core`, vérifier que ce n'est pas
  du code à engendrer (ici non : le masquage ne varie pas d'un projet à l'autre).
- Commits : Conventional Commits, sujet français à l'impératif sans majuscule ni point
  final ; corps = pourquoi + `Vérifications :` avec commandes et résultats réels. **Aucun**
  identifiant de tâche, aucun renvoi à `IMPROVE.md`, à un plan, un lot ou un backlog,
  **jamais** de `Co-Authored-By`, `Claude-Session` ni mention d'IA.
- Ne pas toucher `IMPROVE.md` ni `TODO.md`.
- Branche : `git checkout -b improve/p3-divers improve/p3-secu` **avant la première
  ligne**, puis `git rev-list --count HEAD..improve/p3-secu` doit rendre 0.
- Exemples (`file-drop` porte `redis`, `event-hub` porte `ci`) : régénération par diff
  entre deux générations (commandes dans `examples/README.md`), jamais par écrasement ;
  `patch --no-backup-if-mismatch` ; oracle `cargo test -p rbs-cli --test integration_examples`.
- Sorties longues : `$SCRATCH/divers-<nom>.log`, jamais un nom partagé avec l'autre agent.
- Tests Docker : `--no-fail-fast`, une suite par commande, en arrière-plan redirigé.

---

### Tâche 1 : `redact_url` dans `rbs-core`, appelée par `redis` et `rate-limit`

**Fichiers :** `crates/rbs-core/src/db.rs:87-138` (et ses tests, qui ont déjà une aide
`mask`) ; `crates/rbs-cli/templates/features/redis/mod.rs.jinja:37-39` ;
`crates/rbs-cli/templates/features/rate-limit/counter.rs.jinja:24-28` (branche
`{% if "redis" in features %}` seule) ; `crates/rbs-cli/src/templates.rs` (test de rendu) ;
page de doc de `rbs-core` si elle liste `db::connect` (`grep -rn 'db::connect' docs/docs docs/i18n`).

- [ ] Tests dans `db.rs` : remplacer l'aide `mask` par `redact_url` et ajouter
  ```rust
  #[test]
  fn a_redis_url_loses_its_password() {
      assert_eq!(redact_url("redis://:s3cret@cache:6379/0"), "redis://:***@cache:6379/0");
      assert_eq!(redact_url("redis://cache:6379"), "redis://cache:6379");
  }
  ```
  Le voir échouer (fonction absente).
- [ ] Implémenter :
  ```rust
  /// Rend `url` avec son mot de passe remplacé par `***`, pour la citer dans un journal.
  ///
  /// Le masquage des erreurs de [`connect`], pour les autres URL à secret d'un projet.
  pub fn redact_url(url: &str) -> String {
      strip(url, password(url))
  }
  ```
- [ ] Test de rendu dans `templates.rs` : `redis/mod.rs.jinja` et `rate-limit/counter.rs.jinja`
  (avec `redis` installé) rendus ne contiennent plus `config.url)` / `cache.url)` dans un
  `format!` et contiennent `rbs_core::db::redact_url(`. Le voir échouer, puis remplacer
  dans les deux templates : `format!("pool Redis inconstructible pour `{}`", rbs_core::db::redact_url(&config.url))`.
- [ ] Commit `fix(redis): masque le mot de passe de l'URL dans l'erreur de construction du pool`.

### Tâche 2 : `.env` en 0600

**Fichiers :** `crates/rbs-cli/src/secret.rs` (module existant, `tire_au_hasard`) ;
`crates/rbs-cli/src/new.rs:440-450` ; `crates/rbs-cli/src/plan/application.rs:90-135`
(écriture **et** restauration du `undo`) ; tout autre écrivain du `.env` :
`grep -rn 'FICHIER_ENV\|"\.env"' crates/rbs-cli/src | grep -v test` (ex. `doctor --fix`).

**Interface produite :** `crate::secret::write(path: &Path, contenu: &[u8]) -> io::Result<()>` —
`fs::write` ordinaire, sauf pour un fichier nommé `.env` : ouvert par
`OpenOptions::new().write(true).create(true).truncate(true)` avec `.mode(0o600)` sous
`cfg(unix)`, puis `file.set_permissions(Permissions::from_mode(0o600))` **avant**
`write_all` (couvre un `.env` préexistant sans fenêtre à 0644). `.env.example` n'est pas
concerné.

- [ ] Tests `#[cfg(unix)]` : dans `new.rs`, un projet créé a `.env` en `0o600` (`metadata().permissions().mode() & 0o777`)
  et `.env.example` inchangé ; dans `plan/application.rs`, un `.env` préexistant en 0644
  réécrit par une application de plan finit en 0600. Les voir échouer.
- [ ] Implémenter `secret::write` et l'appeler aux points d'écriture ci-dessus.
- [ ] Commit `fix(cli): écrit le .env en 0600`.

### Tâche 3 : CI engendrée épinglée par SHA, avec Dependabot

**Fichiers :** `crates/rbs-cli/templates/features/ci/.github/workflows/ci.yml.jinja:55-62` ;
nouveau `crates/rbs-cli/templates/features/ci/.github/dependabot.yml.jinja` + son
`[[files]]` dans `ci/feature.toml` ; test dans `crates/rbs-cli/src/templates.rs` ;
`docs/docs/cli/add.md` et sa version FR (ligne du fragment `ci`) ; `rbs doctor` s'il
contrôle les fichiers du fragment `ci` (`grep -rn 'ci.yml' crates/rbs-cli/src/doctor`).

SHA relevés le 2026-09-14 (`gh api repos/<dépôt>/commits/<ref>`) :
- `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1`
- `dtolnay/rust-toolchain@6bed0761d98439e5a578e2877258200ad565ba87 # stable` — **ajouter**
  `toolchain: stable` sous `with:` : l'action déduit la toolchain du nom de la référence,
  qu'un SHA ne porte pas.
- `Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2`

- [ ] Test dans `templates.rs` : `every_action_of_the_generated_ci_is_pinned_by_a_sha` —
  rendre `ci.yml.jinja` (postgres et mysql), chaque ligne `uses:` porte `@` suivi de 40
  hexadécimaux. Et `dependabot.yml` est déclaré par le fragment. Le voir échouer.
- [ ] Épingler ; commentaire *pourquoi* au-dessus des `steps` (un tag se déplace, un SHA
  non ; Dependabot propose les montées).
- [ ] `dependabot.yml.jinja` :
  ```yaml
  # Les actions de ci.yml sont épinglées par SHA : un tag peut être déplacé sous vos pieds,
  # un SHA non. Dependabot propose chaque montée de version, SHA et commentaire compris.
  version: 2
  updates:
    - package-ecosystem: github-actions
      directory: /
      schedule:
        interval: weekly
      commit-message:
        prefix: ci
      groups:
        actions:
          patterns: ['*']
  ```
- [ ] Doc EN + FR ; commit `fix(ci): épingle par SHA les actions de la CI engendrée`.

### Tâche 4 : exemples, CHANGELOG, vérification

- [ ] Régénérer `examples/file-drop` (redis) et `examples/event-hub` (ci) par diff entre
  générations ; `cargo check --all-targets` dans `file-drop`.
- [ ] `CHANGELOG.md` et `CHANGELOG.fr.md`, section 1.5.0 : *Added* / *Ajouté*
  `rbs_core::db::redact_url` ; *Fixed* / *Corrigé* le mot de passe Redis, le `.env` en
  0600, la CI épinglée + Dependabot. Même nombre d'items dans les deux fichiers.
- [ ] Vérifications (lues, résultats réels dans les messages de commit) :
  `cargo fmt --all --check` ; `cargo clippy --workspace --all-targets -- -D warnings` ;
  `cargo test -p rbs-core` ; `cargo test -p rbs-cli --lib` ;
  `cargo test -p rbs-cli --test integration_examples` ;
  `cargo test -p rbs-cli --test integration_redis --no-fail-fast -- --include-ignored` ;
  `cargo test -p rbs-cli --test integration_new --no-fail-fast -- --include-ignored` ;
  `npm ci && npm run build` sous `docs/`.
- [ ] Rapport final : commits, sorties réelles (chiffres passés/échoués), écarts.
