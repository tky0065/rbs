# I7 — Enregistrement OpenAPI de l'auth

## But

Que le document publié décrive l'authentification : les cinq chemins, le schéma de
sécurité, et la route qui l'exige.

## Ce qui existe déjà

L'ancre `openapi` du manifeste inscrit les cinq handlers depuis I1. Ce lot ajoute le
schéma de sécurité et la preuve que le document servi les porte.

## Où vit le schéma bearer

Dans `ReponsesCommunes`, le modifier du noyau, sous `#[cfg(feature = "auth")]` — la
feature qu'`add auth` active déjà dans le manifeste du projet. Un HTTP bearer JWT ne varie
pas d'un projet à l'autre, et ce modifier enrichit déjà `components` des réponses 401 et
403 : le schéma les rejoint.

L'alternative — une seconde ancre `openapi-modifiers` dans le squelette — rouvrirait le
lot C4 et ferait échouer `add auth` sur tout projet généré avant elle.

## Seul `me` porte le schéma

`refresh` et `logout` s'authentifient par leur corps, pas par un en-tête. Leur apposer
`security` décrirait une exigence que le serveur ne pose pas.

## Écart au TODO

Le critère nomme `/openapi.json` ; le document vit sur `/api-docs/openapi.json` depuis C4,
et c'est l'URL que Swagger UI charge. Le critère porte sur le document, non sur sa route :
le test le lit où il est.

## Hors périmètre, fait quand même

La CI ne compile jamais `rbs-core` avec ses features : `cargo test --workspace` et
`cargo clippy --workspace --all-targets` n'en activent aucune, et tout le lot G — Argon2,
JWT, jetons, `Identity`, `AuthConfig` — n'est vérifié par aucune exécution automatique.
Le schéma ajouté ici tomberait dans le même angle mort. `--all-features` sur les deux
étapes : mesuré propre avant d'être écrit, et 18 tests du noyau passent de non couverts à
couverts.

## Étapes

1. Tests rouges : le document porte les cinq chemins, le schéma bearer est déclaré, `me` le
   porte.
2. `rbs-core` : le schéma dans `ReponsesCommunes`, avec son test unitaire.
3. `controller.rs` : `security` sur `me`.
4. `ci.yml` : `--all-features`.
