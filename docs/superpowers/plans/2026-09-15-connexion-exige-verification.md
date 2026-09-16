# La connexion exige une adresse vérifiée, par défaut

Design validé en conversation le 2026-09-15 : clé `[auth] login_requires_verification`,
`true` par défaut ; un compte non vérifié reçoit le 401 du mauvais mot de passe, après le
même hachage.

1. Tests rouges (template `auth/tests/` et ses deux rendus) : aides `verified`,
   `signed_up`, `access_token_for` ; une adresse non vérifiée refusée puis ouverte par la
   preuve ; `register` + `login` au même 401 pour une adresse libre et une prise. Les
   tests qui se connectent après inscription passent par `signed_up` ; celui de
   `VerifiedIdentity` signe son jeton.
2. `FlowConfig.login_requires_verification` (défaut `true`), la clé dans `feature.toml` et
   la config des deux exemples ; `service::login` prend `&FlowConfig` ; le contrôleur
   passe `state.flows()` ; un test service à `false`.
3. `integration_auth` : 401 avant, `UPDATE` par psql, 200 après. `integration_lang` lance
   son serveur avec `RBS_AUTH__LOGIN_REQUIRES_VERIFICATION=false`.
4. Doc : guide auth (routes, `VerifiedIdentity`), tutoriel auth (vérifier avant de se
   connecter), anglais et français ; CHANGELOG ; note 1.5.0.
5. Vérifier : tests d'auth de `blog-auth` contre PostgreSQL, `integration_examples`,
   `--lib`, fmt, clippy, doc, `integration_auth` et `integration_lang` sous Docker.
