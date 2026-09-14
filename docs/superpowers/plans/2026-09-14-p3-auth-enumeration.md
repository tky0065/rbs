# Parcours auth sans oracle — plan d'implémentation

> **Pour l'agent :** SOUS-SKILL REQUIS : superpowers:executing-plans. Les étapes se cochent (`- [ ]`).

**But :** que `register`, `forgot-password` et `resend-verification` ne disent plus, ni par
le statut ni par le temps, si une adresse porte un compte ; que les jetons ne voyagent plus
en query string ; qu'un compte vérifié ne se revérifie pas ; que `one_time_tokens` se purge.

**Architecture :** tout vit dans le fragment `crates/rbs-cli/templates/features/auth/`
(chemins ci-dessous relatifs à ce répertoire sauf mention). La requête HTTP ne fait plus que
`normalise` + `SELECT` (+ Argon2 et l'`INSERT` pour `register`) ; tout le reste — purge,
invalidation, émission du jeton, rendu du courriel — part dans un `tokio::spawn` dont les
erreurs vont au journal.

**Design validé** (le 2026-09-14, par le mainteneur) :
- `register` rend **202 sans corps** dans tous les cas. Adresse neuve : `INSERT` dans la
  requête (un client doit pouvoir se connecter aussitôt), puis émission du lien de
  vérification détachée. Adresse prise (ou violation d'unicité en course) : courriel
  « tentative d'inscription » au titulaire, détaché. Argon2 est calculé dans les deux
  branches. `ADRESSE_PRISE` disparaît.
- `forgot-password` / `resend-verification` : `SELECT` seul dans la requête, le reste détaché.
- Liens : `…/reset-password#token=…` (fragment : jamais envoyé au serveur, absent des
  journaux d'accès et de `Referer`).
- `resend-verification` sur un compte déjà vérifié : 202, aucun jeton. `mark_verified` ne
  réécrit pas une date existante (`WHERE email_verified_at IS NULL`).
- Purge : `purge_expired` est appelée à chaque émission (dans la tâche détachée), et un
  index `idx_one_time_tokens_expires_at` s'ajoute à la migration. Pas de `Schedule`.

## Contraintes globales

- Lire `CLAUDE.md` à la racine avant tout : commentaires = le *pourquoi* seulement ; code
  engendré ne commente que ses points d'extension ; doc modifiée en anglais **et** en
  français dans le même commit.
- Commits : Conventional Commits, sujet français à l'impératif sans majuscule ni point
  final ; corps = pourquoi technique + intertitre `Vérifications :` avec les commandes et
  leur résultat réel. **Aucun** identifiant de tâche, aucun renvoi à `IMPROVE.md`, à un plan,
  un lot ou un backlog, **jamais** de `Co-Authored-By`, `Claude-Session` ni mention d'IA.
- Ne pas toucher `IMPROVE.md` ni `TODO.md` : l'orchestrateur coche.
- Branche : `git checkout -b improve/p3-auth improve/p3-secu` **avant la première ligne**,
  puis `git rev-list --count HEAD..improve/p3-secu` doit rendre 0.
- `examples/blog-auth` et `examples/event-hub` portent `auth` : ils se régénèrent par
  **diff entre deux générations** (ancien CLI / nouveau CLI, commandes exactes dans
  `examples/README.md`), jamais par écrasement — ils portent des éditions manuelles et des
  marqueurs `// region:` cités par la doc. `patch --no-backup-if-mismatch` (un `.orig`
  laissé casse `integration_examples`). L'oracle est
  `cargo test -p rbs-cli --test integration_examples`.
- Sorties longues : rediriger vers `$SCRATCH/auth-<nom>.log` (`SCRATCH` = le scratchpad
  donné dans la consigne), jamais un nom partagé avec l'autre agent.
- Tests Docker : `--no-fail-fast`, une suite par commande (le shell coupe à 600 s ;
  `integration_auth` en prend ~500), en arrière-plan redirigé.
- Le runtime de `#[tokio::test]` est mono-fil : une tâche détachée n'avance que lorsque le
  test cède la main (`.await`). Attendre par `tokio::time::sleep`, jamais `std::thread::sleep`.

---

### Tâche 1 : liens en fragment (`#token=`)

**Fichiers :** `config.rs.jinja:36-46` ; `tests/mod.rs.jinja` (test unitaire) ;
`docs/docs/tutorials/auth.md:307` et
`docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/auth.md:314` ;
`docs/docs/guides/auth.md` + sa version FR (dire au front de lire `location.hash`).

- [ ] Test unitaire (non `#[ignore]`) dans `tests/mod.rs.jinja` :
  ```rust
  /// Le jeton part dans le fragment : un navigateur ne l'envoie jamais au serveur, donc ni
  /// journal d'accès ni en-tête `Referer` ne le portent.
  #[test]
  fn a_link_carries_its_token_in_the_fragment() {
      let flows = crate::auth::config::FlowConfig {
          app_url: "https://exemple.test/".to_owned(),
          ..Default::default()
      };
      assert_eq!(
          flows.link("reset-password", "abc"),
          "https://exemple.test/reset-password#token=abc"
      );
  }
  ```
  (adapter le chemin de `FlowConfig` à celui que `mod.rs.jinja` expose).
- [ ] `link` : `format!("{}/{path}#token={token}", self.app_url.trim_end_matches('/'))`, et
  ajouter au `///` la raison du fragment. Ajouter `pub fn page(&self, path: &str) -> String`
  (même réconciliation de barre, sans jeton) dont `link` se sert — la tâche 4 l'emploie.
- [ ] Doc EN + FR mise à jour (le tutoriel montre `…/reset-password#token=…`).
- [ ] `cargo test -p rbs-cli --lib` vert ; commit `fix(auth): passe les jetons des liens dans le fragment`.

### Tâche 2 : un compte vérifié ne se revérifie pas

**Fichiers :** `service/verification.rs.jinja:15-42` ; `repository/user.rs.jinja:70-82` ;
`tests/verification.rs.jinja`.

- [ ] Deux tests `#[ignore = "joint la base du projet"]` dans `tests/verification.rs.jinja` :
  - `a_verified_address_is_not_sent_a_new_token` : inscrire, vérifier (par
    `service::verification::request` + `verify`), noter le nombre de jetons du compte
    (`one_time_tokens_count_for`), appeler `request(&db, 86400, &email)` → `None`, et le
    compte n'a pas un jeton de plus.
  - `verifying_again_keeps_the_first_date` : vérifier, lire `email_verified_at`, émettre un
    second jeton directement par `repository::one_time_token::issue` (le service le
    refuserait désormais), dormir 1,1 s (`tokio::time::sleep`), `verify` → la date n'a pas
    changé.
- [ ] `request()` : après le `find_by_email`, `if utilisateur.email_verified_at.is_some() { return Ok(None); }`
  avec le *pourquoi* (la date sert à redemander une preuve aux plus anciennes adresses ;
  la rajeunir la fausse).
- [ ] `mark_verified` : `.filter(user::Column::EmailVerifiedAt.is_null())` ; ajuster le `///`.
- [ ] Commit `fix(auth): ne revérifie pas une adresse déjà vérifiée`.

### Tâche 3 : émissions détachées, purge à l'émission

**Fichiers :** `service/password.rs.jinja:82-136` ; `service/verification.rs.jinja` ;
`repository/one_time_token.rs.jinja:101-115` ; `migration.rs.jinja:170-195` ;
`feature.toml` (feature tokio `time`) ; `tests/mod.rs.jinja` ; `tests/password.rs.jinja:470-545` ;
`tests/verification.rs.jinja:90-150` ; `tests/session.rs.jinja:~900-930` ;
`crates/rbs-cli/templates/features/mail/service.rs.jinja:98-103` (le `///` y cite
`forgot-password` : le garder vrai).

**Interfaces produites :**
- `service::password::open_reset(db: &impl ConnectionTrait, ttl_secs: u64, utilisateur: &repository::Model) -> Result<String>` : purge + invalidation + émission, rend le jeton en clair.
- `service::verification::open(db: &impl ConnectionTrait, ttl_secs: u64, utilisateur: &repository::Model) -> Result<String>` : idem pour la vérification.
- `request_reset` et `request` gardent leur signature (les tests les appellent) : `find_by_email` puis `open_*`.
- `service::verification::send_link_detached(db: &DatabaseConnection, mail: &Mailer, flows: &FlowConfig, utilisateur: repository::Model)` : clone `db`/`mail`/`flows` et lance `open` + `notify` dans un `tokio::spawn` ; erreurs → `tracing::error!(user_id = %…, %error, "…")`. La tâche 4 l'appelle.

- [ ] `feature.toml` : `[cargo.tokio] features = ["time"]`, avec le *pourquoi* (les tests
  attendent une tâche détachée) — imiter `scheduler/feature.toml`.
- [ ] Aide de test dans `tests/mod.rs.jinja` :
  ```rust
  /// Attend qu'une condition devienne vraie, cinq secondes au plus.
  ///
  /// Les émissions de jetons partent en tâche détachée : la réponse précède l'écriture.
  async fn eventually<F, Fut>(mut condition: F) -> bool
  where
      F: FnMut() -> Fut,
      Fut: std::future::Future<Output = bool>,
  {
      for _ in 0..50 {
          if condition().await {
              return true;
          }
          tokio::time::sleep(std::time::Duration::from_millis(100)).await;
      }
      false
  }
  ```
- [ ] Test de purge dans `tests/password.rs.jinja` :
  `an_emission_purges_the_expired_tokens_of_every_account` — compte A avec un jeton échu
  (aide existante `:56`), compte B : `request_reset(&db, 3600, &b)` → le jeton échu de A
  n'existe plus. Le voir échouer.
- [ ] Adapter `forgetting_a_registered_address_is_accepted_and_opens_a_token` (et son
  pendant `resend-verification`) : l'assertion de comptage passe par `eventually`.
- [ ] Implémenter `open_reset` / `open` (appellent `repository::one_time_token::purge_expired(db)`
  en premier), `send_reset_link` et `send_link` réduits à `find_by_email` + spawn. Retirer
  le `#[allow(dead_code)]` et réécrire le `///` de `purge_expired` (elle a un appelant).
  `resend-verification` : le test « déjà vérifié » de la tâche 2 se fait avant le spawn,
  sur la ligne lue.
- [ ] Migration : index `idx_one_time_tokens_expires_at` sur `OneTimeTokens::ExpiresAt`,
  même forme que `idx_one_time_tokens_token_hash`, avec son *pourquoi* (chaque émission
  purge par cette colonne).
- [ ] Commit `fix(auth): détache les émissions de jetons et purge les jetons échus`.

### Tâche 4 : `register` en 202 indiscernable

**Fichiers :** `service/session.rs.jinja:1-45` ; `controller/session.rs.jinja:12-32` ;
`repository/user.rs.jinja:25-54` ; `repository/mod.rs.jinja:17` ; nouveau
`inscription.html.jinja` + son `[[files]]` dans `feature.toml` (destination
`templates/mail/inscription.html`) ; `tests/mod.rs.jinja` (aide `register`) ;
`tests/session.rs.jinja:49-160` ; `crates/rbs-cli/src/templates.rs:973-982`
(`MESSAGES_FRANCAIS` perd `ADRESSE_PRISE`) ; `crates/rbs-cli/tests/integration_lang.rs:140-165`
et les usages de `/auth/register` dans `integration_auth.rs` (`:208,708,764,915`) ;
`grep -rn '/auth/register\|auth::repository::create\|StatusCode::CREATED' crates/rbs-cli/templates/feature`
pour le `tests.rs` engendré des CRUD sous auth.

- [ ] `repository::create` rend `Result<Option<Model>>` : `None` sur
  `UniqueConstraintViolation`. Supprimer `ADRESSE_PRISE` (les deux variantes de langue) et
  son export ; mettre à jour tous les appelants de `create`.
- [ ] `service::register(...) -> Result<()>` : Argon2 **avant** le `find_by_email` (dans
  les deux branches — commenter pourquoi) ; neuve → `create` → `Some(cree)` →
  `verification::send_link_detached(db, mail, flows, cree)` ; prise, ou `None` en course →
  relire le compte et `warn_taken` : un `tokio::spawn` qui rend `inscription.html` avec
  `forgot_url => flows.page("forgot-password")` via `notify`.
- [ ] Gabarit `inscription.html.jinja` : même facture que `verification.html.jinja` (HTML
  `lang="fr"`, `{{ }}` minijinja) — « quelqu'un a tenté de créer un compte avec cette
  adresse ; si c'était vous, connectez-vous ou réinitialisez votre mot de passe ; sinon il
  n'y a rien à faire ».
- [ ] Contrôleur : `Result<StatusCode>` → `StatusCode::ACCEPTED`, sans corps ; `utoipa`
  : `202 "inscription reçue, que l'adresse soit neuve ou non"`, `422` ; retirer 201/409.
- [ ] Aide de test `register(api, email)` : après le 202, attendre par `eventually` que le
  compte existe et porte au moins un jeton de vérification — sans quoi la tâche détachée
  de `register` peut invalider le jeton qu'un test s'ouvre juste après. Une inscription en
  double trouve ce jeton aussitôt.
- [ ] Tests de `tests/session.rs.jinja` réécrits :
  `registration_returns_202_without_a_body` (statut, corps `Null`, compte en base) ;
  `a_taken_address_returns_the_same_202_and_creates_nothing` (deux 202, corps égaux, une
  seule ligne `users`) ; `a_taken_address_keeps_its_password` (deuxième inscription avec
  un autre mot de passe : l'ancien connecte, le nouveau non) ;
  `registration_lowercases_the_address` et `an_address_taken_in_another_case_…` vérifiés en
  base. `the_hash_does_not_appear_in_the_response` se fond dans le premier.
- [ ] `integration_lang.rs` : la seconde inscription rend 202 ; l'échantillon `Conflict`
  reste celui du CRUD, plus bas dans le même test. `integration_auth.rs` : 201 → 202.
- [ ] `grep -rn 'twenty-one\|vingt et un' docs crates` : `auth/feature.toml` passe à 22
  destinations, `docs/docs/cli/add.md:48` et sa version FR suivent.
- [ ] Commit `fix(auth): répond 202 à toute inscription et prévient le titulaire d'une adresse prise`.

### Tâche 5 : exemples, doc, notes, vérification

- [ ] Régénérer `examples/blog-auth` et `examples/event-hub` par diff entre générations.
  `cargo check --all-targets` dans chacun avant la passe Docker.
- [ ] Doc EN + FR : `guides/auth.md` (tableau `register` l.71 EN / l.72 FR, paragraphes
  sur le 202 et le `.await` du handler, l.219-236 / 228-283), `guides/errors.md:105` (plus
  de 409 d'inscription), `guides/testing.md` si concerné, `tutorials/auth.md` (sortie de
  l'inscription). `grep -rn '409\|201' docs/docs docs/i18n | grep -i regist`.
- [ ] `CHANGELOG.md` et `CHANGELOG.fr.md`, section 1.5.0 : sous *Changed* / *Modifié* le
  202 d'inscription (projets neufs ; un projet existant garde son code), les émissions
  détachées, les liens en fragment ; sous *Fixed* / *Corrigé* la date de vérification et
  la purge. Même nombre d'items dans les deux fichiers.
- [ ] `crates/rbs-cli/notes/1.5.0.md` : une section courte — index à créer à la main sur
  un projet déjà migré (`CREATE INDEX idx_one_time_tokens_expires_at ON one_time_tokens (expires_at);`),
  et les fichiers à reprendre du fragment pour avoir le nouveau comportement.
- [ ] Vérifications (chacune lue, résultat réel dans le message de commit) :
  `cargo fmt --all --check` ; `cargo clippy --workspace --all-targets -- -D warnings` ;
  `cargo test -p rbs-cli --lib` ; `cargo test -p rbs-cli --test integration_examples` ;
  `cargo test -p rbs-cli --test integration_auth --no-fail-fast -- --include-ignored` ;
  puis `--test integration_lang` (même forme, **après** auth : même nom de projet) ;
  `npm ci && npm run build` sous `docs/`.
- [ ] Commit `docs(auth): …` / `test(examples): …` selon ce qui reste.
- [ ] Rapport final : commits, sorties réelles des passes (chiffres passés/échoués), écarts.
