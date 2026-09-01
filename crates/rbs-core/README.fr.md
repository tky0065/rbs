# rbs-core

Le runtime partagé des projets engendrés par [rbs](https://github.com/tky0065/rbs), un cadre
de travail Rust pour les API web bâti sur Axum et SeaORM.

*[English version](README.md).*

Cette crate porte ce qui n'a aucune raison de différer d'une API à l'autre. Tout ce qu'un
développeur voudra lire ou modifier est engendré dans ses propres sources par la commande
`rbs`, et non caché ici.

## On ne l'ajoute pas à la main, d'ordinaire

`rbs new` inscrit `rbs-core` dans le manifeste du projet qu'il engendre, avec la feature de
base de données que vous avez choisie. Rien n'interdit un `cargo add rbs-core` dans un projet
sans rapport, mais la crate est conçue comme la moitié runtime d'un projet engendré —
commencez par [`rbs-cli`](https://crates.io/crates/rbs-cli) et le
[guide de démarrage](https://tky0065.github.io/rbs/fr/getting-started).

## Ce qu'elle vous donne

- **Les erreurs.** Un seul type `Error` pour toute la chaîne — repository, service,
  controller — et un alias `Result<T>`. Chaque variante se rend en réponse
  `application/problem+json` conforme à la RFC 9457 ; les échecs de la base et les échecs
  internes gardent leur source pour le log et ne la laissent jamais fuir vers le client.
- **La configuration.** Cinq couches fusionnées dans l'ordre — valeurs par défaut,
  `config/default.toml`, `config/{RBS_ENV}.toml`, `.env`, environnement — et validées au
  démarrage.
- **Les logs.** Deux formateurs, choisis par `RBS_LOG_FORMAT` : l'un lisible en
  développement, l'autre en JSON pour la production.
- **L'état.** `CoreState` tient le pool de connexions et la configuration, et `HasCoreState`
  permet aux handlers du runtime de l'atteindre à travers l'`AppState` dans lequel votre
  projet l'enveloppe.
- **La plomberie des requêtes.** L'extracteur `ValidatedJson`, `Pagination`/`Page`, un
  identifiant de corrélation par requête, une couche de trace HTTP, et une route `/health`.
- **OpenAPI.** `ProblemDetails` et `CommonResponses` déclarent les réponses d'erreur une
  fois, pour tous les chemins utoipa du projet.

## Drapeaux de features

`postgres` (par défaut), `mysql` et `sqlite` choisissent le pilote SeaORM — n'en prenez
qu'un.

`auth` ajoute le hachage de mots de passe Argon2, la signature et la vérification de JWT, les
jetons opaques, et un extracteur `Identity`.

`observability` greffe une couche d'export OTLP sur le souscripteur que pose `logs::init()`,
quand `OTEL_EXPORTER_OTLP_ENDPOINT` nomme un collecteur. Elle vit ici plutôt que dans le code
engendré parce que `set_global_default` s'appelle une seule fois, et que cet appel est la
première ligne d'un `main` engendré.

`redis`, `mail` et `storage` sont déclarées mais **vides** : elles réservent les noms
d'extensions prévues et n'activent rien aujourd'hui.

## Ce qu'elle laisse délibérément de côté

Votre `AppState`, votre routeur, vos entités, vos features. Tout cela est engendré dans votre
projet, en Rust clair, sans macro à déplier, et aucune version de rbs ne le réécrit.

## Stabilité

À l'intérieur de la ligne 1.x, l'API publique de cette crate est figée : rien n'est retiré,
renommé ni doté d'un autre sens, et `cargo-semver-checks` fait échouer la construction
plutôt que de laisser passer. La
[page de compatibilité](https://tky0065.github.io/rbs/fr/compatibility) énonce les
périmètres.

## Documentation

Le site est à l'adresse <https://tky0065.github.io/rbs/fr/> — architecture, guides,
référence du CLI. La version anglaise vit à <https://tky0065.github.io/rbs/>.

Rust 1.94 ou plus.

## Licence

Sous double licence MIT ou Apache-2.0, à votre choix.
