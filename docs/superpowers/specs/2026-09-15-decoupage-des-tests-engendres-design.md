# Découpage des fichiers de tests engendrés

Date : 2026-09-15. Design validé en conversation.

## Problème

Le `CLAUDE.md` fixe à ~200 lignes la taille d'un fichier de feature. Les fichiers que
l'utilisateur reçoit la dépassent de loin, surtout les tests : mesurés dans `examples/`,
`auth/tests/session.rs` fait 1313 lignes, `webhooks/tests.rs` 837, `jobs/tests.rs` 591,
`auth/tests/password.rs` 550, `scheduler/tests.rs` 429, `storage/tests.rs` 424,
`auth/tests/verification.rs` 315, et le `tests.rs` de `generate crud` de 357 à 547 selon
les options. Côté code, `jobs/queue.rs` fait 365 lignes.

## Règle

Tout fichier de tests rendu au-delà de ~250 lignes est scindé **par préoccupation du code
qu'il éprouve** — une route, une couche, un mécanisme. Chaque fichier obtenu tient sous
~250 lignes rendues, dans la configuration la plus chargée de ses options. Hors périmètre :
les fichiers de 200 à 250 lignes, et la migration `create_auth_tables` (262 lignes), dont
le découpage changerait l'historique du schéma.

## 1. Tests des fragments

Chaque fichier trop long devient un répertoire `tests/` :

- `tests/mod.rs` porte le harnais partagé (états, aides, constantes) et les déclarations
  `mod <préoccupation>;` ;
- chaque fichier de préoccupation commence par `use super::*;`, comme `auth/tests/` le fait
  déjà ;
- chaque fichier est déclaré dans le `feature.toml` du fragment — le test
  `every_file_the_fragment_ships_is_declared_in_its_manifest` y veille ;
- le `mod tests;` du parent ne change pas : Rust résout `tests/mod.rs`.

Chemins publics conservés : `crate::modules::jobs::tests::verrou_base()`, qu'appellent les
tests de `scheduler` et de `webhooks`, reste dans `jobs/tests/mod.rs`.

Découpage visé (les frontières exactes se fixent à la lecture, sous la règle) :

| Fichier | Devient |
|---|---|
| `auth/tests/session.rs` | `registration`, `login`, `refresh`, `sessions`, `roles`, `openapi` |
| `auth/tests/password.rs` | `tokens`, `change`, `reset` |
| `auth/tests/verification.rs` | `verification`, `guard` |
| `webhooks/tests.rs` | `tests/` : `signature`, `emission`, `routes`, `target`, `blocked` |
| `jobs/tests.rs` | `tests/` : `retry`, `queue`, `lease`, `worker` |
| `scheduler/tests.rs` | `tests/` : `expression`, `sync`, `ticker` |
| `storage/tests.rs` | `tests/` : `files`, `s3` |

`auth/tests/mod.rs` (273 lignes) est déjà un harnais : il reste tel quel s'il ne reçoit
rien, et se scinde sinon.

## 2. Tests du CRUD engendré

`rbs generate crud` écrit `src/<nom>/tests/` au lieu de `src/<nom>/tests.rs` :

- `mod.rs` : harnais (`application`, `call`, `token`, `bearer`, `request`,
  `without_body`, `compare`, `filled`, `unique_number`, `creation`, `modification`) ;
- `lifecycle.rs` : cycle complet, compression, identifiants croissants, parcours au curseur ;
- `errors.rs` : 404, 400 d'un corps illisible, 422 d'une adresse, 400 d'un tri inconnu,
  409 d'une valeur unique rejouée ;
- `filter.rs` sous un champ filtrable ; `access.rs` sous `auth` (les 401 anonymes) ;
  `content.rs` sous `--with-upload`.

Un fichier sans contenu pour les options données n'est ni rendu ni déclaré. Un template
par fichier sous `templates/feature/tests/` ; `tests_http.rs` rend la liste.

## 3. `jobs/queue.rs`

`queue.rs` devient `queue/` : `mod.rs` (`enqueue`, `enqueue_at`, `a_la_seconde`),
`reserve.rs` (les requêtes de réservation par moteur et `reserver_prochain_job`),
`outcome.rs` (`Reprise`, `requeue_stale`, `mark_done`, `retry_delay`, `retry_or_fail`).
Des `pub use` gardent les chemins `queue::…` qu'emploient le worker, les tests et
`jobs::enqueue`.

## 4. Projets existants

Leurs fichiers ne bougent pas : `add` ne repasse pas, et rien ne casse.

`rbs generate crud X --force` sur un CRUD qui porte encore `src/X/tests.rs` : le plan
signale un conflit avant toute écriture — le fichier nommé, la raison (les tests vivent
désormais dans `tests/`), le remède (le supprimer puis relancer). Le CLI ne supprime
jamais un fichier lui-même. La note 1.5.0 le dit.

## 5. Exemples et documentation

- Les cinq exemples sont régénérés par diff entre deux générations, pour garder leurs
  régions posées à la main ; `integration_examples` est l'oracle.
- Les citations `file=… region=…` suivent le fichier où leur région atterrit :
  `guides/errors.md`, `guides/testing.md`, `guides/auth.md`, `tutorials/auth.md`.
- Les transcriptions qui listent `src/articles/tests.rs` (`getting-started.md`,
  `cli/generate.md`, anglais et français) listent le répertoire ; `integration_docs` les
  garde.
- Le `CHANGELOG` (anglais et français) et la note 1.5.0 décrivent la disposition.

## 6. Vérification

Aucun test ne change de corps : un déplacement se prouve par le même nombre de tests passés
avant et après, suite par suite.

- Tests hors base et `--include-ignored` contre PostgreSQL sur les exemples concernés,
  avant et après, comptes comparés.
- `integration_examples`, `integration_docs`, `cargo test -p rbs-cli --lib`, fmt, clippy.
- Les suites Docker des fragments touchés (`auth`, `webhooks`, `jobs`, `scheduler`,
  `storage`) et celles du CRUD, une par commande.
- `npm run build` sous `docs/` et `parite.mjs`.
