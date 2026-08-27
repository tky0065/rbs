# M1 — Fragment `mail` : manifeste, section `[mail]`, transport dans l'état

Premier fragment à déclarer une crate tierce en `default_features = false`, champ mis au
schéma par `K2` précisément pour `lettre` : ses défauts activent `native-tls`, donc OpenSSL
sur les trois plateformes de la CI générée.

1. `templates/features/mail/feature.toml` — quatre fichiers, l'ancre `features`, les deux
   ancres d'état, `lettre 0.11` en `smtp-transport, builder, pool, tokio1-rustls`, la
   section `[mail]` de `config/default.toml`, et `RBS_MAIL__SMTP_PASSWORD` en `[[env]]`.
2. Le module déposé, `src/mail/`, sans `controller` ni `repository` : le fragment ne monte
   aucune route (§2.7), la chaîne s'arrête à `service → config`.
   - `config.rs` : `MailConfig` et `Tls`, chaque défaut porté par `#[serde(default = "…")]`
     comme le veut §2.2 ; `smtp_password` n'a de valeur nulle part sur le disque.
   - `service.rs` : `Mailer`, construit par `depuis_config()` — faillible, synchrone,
     `AsyncSmtpTransport::relay` ne faisant aucune E/S — et `envoyer`.
   - `mod.rs` : la façade du module.
   - `tests.rs` : les tests du projet généré.
3. `FEATURES_CONNUES` (`new.rs:23`) reçoit `"mail"`, `suite()` (`lib.rs:188`) sa ligne de
   suite — un test du dépôt exige que toute feature installable en ait une.
4. **TDD** : `le_mot_de_passe_smtp_...` dans `add/mod.rs`, planifiant un vrai `add mail` et
   lisant le `.env.example` et les `config/*` projetés. Écrit et vu rouge avant le fragment.
5. **Morsure** : `smtp_password` écrit dans la section `[mail]` du manifeste de fragment ;
   le test doit tomber.
6. **Preuve du premier critère** : `rbs new` puis `rbs add mail` dans un répertoire
   temporaire, puis `cargo clippy --workspace --all-targets -- -D warnings` et
   `rustfmt --edition 2024 --check` sur le projet produit, au niveau exigé depuis `I1`.
