---
sidebar_position: 2
title: Configuration
---

# Configuration

Un projet généré lit ses réglages dans cinq couches fusionnées dans un ordre fixe. Un
port, une taille de pool, une URL de base se changent dans la couche qui correspond à la
durée de vie du changement : un fichier pour le projet, une variable d'environnement pour
la machine.

## Les cinq couches

Chaque couche recouvre la précédente :

1. **Les défauts** portés par `rbs-core`.
2. **`config/default.toml`** — les réglages communs à tous les environnements.
3. **`config/{RBS_ENV}.toml`** — les réglages du profil actif. `RBS_ENV` vaut
   `development` à défaut.
4. **`.env`** — les seules clés préfixées `RBS_`, et pour cette lecture seulement. rbs ne
   les exporte jamais dans l'environnement du processus, pour que la précédence entre les
   deux dernières couches reste explicite.
5. **Les variables d'environnement** préfixées `RBS_`.

Dans un nom de variable, `__` sépare les niveaux : `RBS_DATABASE__URL` alimente
`database.url`, `RBS_SERVER__PORT` alimente `server.port`. Les deux fichiers TOML sont
optionnels — un projet entièrement configuré par l'environnement se charge tout aussi
bien.

Le profil lui-même est résolu en deux temps : les couches indépendantes du profil sont
fusionnées une première fois pour en extraire `env`, qui désigne alors le
`config/{env}.toml` de l'assemblage final. `RBS_ENV` fonctionne donc depuis
l'environnement *comme* depuis `.env`.

Voici `config/default.toml` tel que `rbs new` l'écrit :

```toml file=examples/hello-crud/config/default.toml
```

Et le fichier de profil, qui ne porte que ce qui diffère :

```toml file=examples/hello-crud/config/development.toml
```

`.env` est l'endroit où vit l'URL de la base, à côté des deux variables de log et — sur
un projet dont le compose porte une base — des identifiants que celui-ci interpole :

```bash file=examples/hello-crud/.env.example
```

## Tous les réglages

| Clé | Variable | Défaut |
|---|---|---|
| `env` | `RBS_ENV` | `development` |
| `server.host` | `RBS_SERVER__HOST` | `127.0.0.1` |
| `server.port` | `RBS_SERVER__PORT` | `8080` |
| `database.url` | `RBS_DATABASE__URL` | **aucun — requis** |
| `database.max_connections` | `RBS_DATABASE__MAX_CONNECTIONS` | `10` |
| `database.min_connections` | `RBS_DATABASE__MIN_CONNECTIONS` | `0` |
| `database.connect_timeout_secs` | `RBS_DATABASE__CONNECT_TIMEOUT_SECS` | `5` |
| `database.acquire_timeout_secs` | `RBS_DATABASE__ACQUIRE_TIMEOUT_SECS` | `5` |
| `database.idle_timeout_secs` | `RBS_DATABASE__IDLE_TIMEOUT_SECS` | `600` |
| `database.max_lifetime_secs` | `RBS_DATABASE__MAX_LIFETIME_SECS` | `1800` |
| `docs.swagger_ui` | `RBS_DOCS__SWAGGER_UI` | `true` |
| `docs.openapi_json` | `RBS_DOCS__OPENAPI_JSON` | `true` |

`database.url` est la seule clé sans défaut. Rien de sensé ne peut être deviné à sa
place : son absence arrête le processus au démarrage, avec un message qui nomme le champ.

### Pourquoi `docs.swagger_ui` et `docs.openapi_json` font deux réglages

Les deux besoins ne sont pas symétriques. Couper l'interface en gardant le document est ce
qu'on fait pour générer des clients ou vérifier un contrat depuis la CI ; l'inverse n'a
pas d'usage. Un seul booléen ne saurait pas l'exprimer, il y en a donc deux. Le
[guide OpenAPI](./openapi.md) dit ce que chacun monte réellement.

## Échouer au démarrage n'est pas une réponse HTTP

Le chargement rend `Result<Config, ConfigError>`, et `ConfigError` est un type à part —
délibérément *pas* l'`Error` du runtime. Une erreur de runtime sait devenir une réponse
`application/problem+json` ; une erreur de démarrage n'a aucun client à qui répondre. Elle
remonte à `main`, qui la propage :

```rust file=examples/hello-crud/src/main.rs region=demarrage
```

`Config::load()` lit depuis le répertoire courant : un projet se lance donc depuis sa
propre racine, là où se trouvent `config/` et `.env`.

## Jugez par vous-même

Surchargez un réglage sans toucher à un fichier :

```bash
RBS_SERVER__PORT=9090 cargo run
```

Chacune des règles de précédence ci-dessus porte un test, et le module se lit d'une
traite :

```bash
cargo test -p rbs-core config::tests
```
