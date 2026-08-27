# I3 — Register et login

## But

Les deux premiers corps de service de la feature auth, et la preuve qu'ils ne renseignent
pas un attaquant : ni par la réponse, ni par les logs, ni par le temps de réponse.

## Ce que le noyau fournit déjà

`hash::hacher` / `hash::verifier` (Argon2id), `jwt::signer` et `Claims`,
`token::aleatoire` / `token::empreinte`, `HasAuth::auth()` pour le secret et les durées de
vie. Rien à ajouter à `rbs-core`. `db::connect` pose `sqlx_logging(false)` : les
paramètres des requêtes ne sont jamais journalisés, ce qui écarte en amont la fuite du
hash par SeaORM.

## Le point sensible

Un email inconnu doit coûter le même temps qu'un mot de passe faux. Sans vérification sur
le chemin « utilisateur absent », la réponse tombe en une fraction de milliseconde contre
une soixantaine pour Argon2, et ce seul écart énumère les comptes inscrits. `login`
vérifie donc contre un hash factice calculé une fois — `LazyLock` — quand l'utilisateur
n'existe pas, puis rend le même `Error::Unauthorized` dans les deux cas.

## Changement de signature

`login(db, entree)` devient `login(db, auth: &AuthConfig, entree)` : l'émission des jetons
demande le secret et les TTL. Le controller passe `state.auth()`.

## Où vivent les preuves

Les quatre tests de comportement vont dans `tests.rs.jinja` — ce que l'utilisateur reçoit
est ce qui prouve la feature — et un test `#[ignore]` d'`integration_auth` les exécute
réellement : PostgreSQL, `rbs new`, `add auth`, `migrate up`, puis `cargo test` dans le
projet généré.

**Écart assumé** : la moitié « logs » du deuxième critère ne tient pas là. `logs::init()`
pose l'abonné global sur stdout sans injection de writer, et le moteur de fragments (lot H,
clos) ne sait pas ajouter la dev-dependency `tracing-subscriber` qu'une capture in-process
exigerait. Cette moitié se prouve côté rbs, sur la sortie réelle du binaire lancé — preuve
plus forte, du reste : elle couvre aussi le middleware `trace` du noyau.

## Étapes

1. Tests rouges dans `tests.rs.jinja` : 201 et profil créé, hash absent de la réponse, 409
   sur email pris, réponses identiques sur mot de passe faux et email inconnu, durées du
   même ordre.
2. Test rouge dans `integration_auth.rs` : les tests du projet généré passent.
3. Test rouge dans `integration_auth.rs` : la sortie du binaire ne porte ni `$argon2` ni le
   mot de passe en clair.
4. `repository.rs` : `find_by_email`, `create`, `create_refresh_token`. `create` traduit la
   violation d'unicité en `Error::Conflict` — sans quoi deux inscriptions simultanées du
   même email rendent une 409 et une 500 selon qui gagne la course.
5. `service.rs` : corps de `register` et `login`. `login` émet la paire et stocke
   l'empreinte du refresh ; la rotation reste à I4.
6. `controller.rs` : `state.auth()` passé à `login`.
7. `mod.rs` : `#![allow(dead_code)]` retiré si la compilation le permet, son commentaire
   corrigé sinon — `refresh`, `logout` et `me` restent des stubs jusqu'à I5.

## Hors périmètre

`refresh`, `logout`, `me`. L'enregistrement OpenAPI (I7). Le parcours de bout en bout (J2).
