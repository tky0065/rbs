---
sidebar_position: 3
title: Erreurs
---

# Erreurs

Chaque couche d'une feature — repository, service, controller — rend
`rbs_core::Result<T>`, c'est-à-dire `Result<T, rbs_core::Error>`. Vous choisissez une
variante, vous la rendez avec `?`, et la réponse est écrite pour vous : le bon statut, un
corps `application/problem+json`, et l'identifiant de la requête qui nomme la ligne
laissée dans le journal.

## Les variantes et leur statut

| Variante | Statut | `title` en | `title` fr | Corps |
|---|---|---|---|---|
| `NotFound(&'static str)` | 404 | `Not Found` | `Introuvable` | `detail` nomme la ressource |
| `BadRequest(String)` | 400 | `Bad Request` | `Requête invalide` | `detail` porte la cause |
| `Validation(ValidationErrors)` | 422 | `Validation failed` | `Validation échouée` | `errors`, champ par champ |
| `Unauthorized` | 401 | `Unauthorized` | `Authentification requise` | — |
| `Forbidden` | 403 | `Forbidden` | `Accès interdit` | — |
| `Conflict(String)` | 409 | `Conflict` | `Conflit` | `detail` porte le message |
| `Domain { status, code, message }` | le vôtre | le `code` | le `code` | `detail` porte le message |
| `Database(DbErr)` | 500 | `Internal Server Error` | `Erreur interne` | une phrase fixe |
| `Internal(anyhow::Error)` | 500 | `Internal Server Error` | `Erreur interne` | une phrase fixe |

`title` est la seule colonne ci-dessus qui varie avec
[`[server] lang`](#la-langue-du-corps) : celle de `Domain` non — c'est toujours le `code`
que vous avez choisi, quelle que soit la langue du reste du corps.

Trois d'entre elles s'atteignent sans jamais être nommées : `DbErr`, `anyhow::Error` et
`ValidationErrors` ont chacune une implémentation de `From`, et `?` les convertit au
passage. `Domain` est la porte de sortie — une erreur métier qui choisit son statut et un
code stable — et elle existe pour éviter qu'un projet généré n'empile sa propre hiérarchie
d'erreurs par-dessus celle-ci.

`BadRequest` et `Validation` partagent une frontière qui mérite d'être dite : 400 signifie
*je n'ai pas pu lire ton corps*, 422 signifie *je l'ai lu, il enfreint une règle*.

## Le corps

Les réponses suivent la RFC 9457, avec le type de média `application/problem+json`. Un
échec de validation ressemble à ceci, sur un projet avec `[server] lang = "fr"` (le
défaut) :

```json
{
  "type": "about:blank",
  "title": "Validation échouée",
  "status": 422,
  "errors": {
    "email": ["adresse électronique invalide"]
  },
  "request_id": "01JQ3F8K2P"
}
```

Les champs absents ne sont pas sérialisés : `detail`, `errors` et `request_id`
disparaissent quand il n'y a rien à y mettre. `request_id` est rempli par le middleware
que monte le routeur généré, celui-là même qui estampille les lignes de journal — un
client qui vous donne un identifiant vous donne la ligne exacte à regarder.

`title` suit `[server] lang` ci-dessus ; le message de champ sous `errors` non — c'est la
chaîne que porte la règle `validator` de ce champ du DTO, et cette chaîne n'est pas à rbs
de la traduire (la distinction est détaillée [plus bas](#la-langue-du-corps)).

`Database` et `Internal` sont les deux variantes qui ne disent rien. Leur source part au
journal en `ERROR` et s'y arrête ; le client obtient une phrase fixe — « une erreur
interne est survenue » en français, `"an internal error occurred"` en anglais, voir
[plus bas](#la-langue-du-corps) — et l'identifiant de requête. C'est délibéré : une chaîne
de connexion, un hôte, un secret manquant sont autant de choses qu'un message d'erreur
livre volontiers à qui demande. Deux tests n'existent que pour échouer le jour où une
source fuiterait dans le corps.

## La langue du corps

Deux réglages nommés `lang`, à deux moments différents, décident deux choses distinctes.

À *l'exécution*, `[server] lang` — `fr` par défaut, ou `en` ; `RBS_SERVER__LANG` le
surcharge — décide ce que `rbs-core` lui-même écrit dans la réponse : le `title` de tout
corps `problem+json`, le `detail` fixe du 404 (`{ressource} not found` /
`{ressource} introuvable`) et du 500, et les descriptions communes du document OpenAPI —
les six réponses nommées sous `components/responses` et les 422/500 ajoutées à chaque
opération. C'est `Error::parts` dans `crates/rbs-core/src/error.rs`,
`crates/rbs-core/src/openapi.rs`, et la résolution dans `crates/rbs-core/src/lang.rs` :

```toml
[server]
lang = "en"
```

À *la génération*, la langue du projet — choisie une fois par `rbs new --lang` et inscrite
comme `lang` sous `[package.metadata.rbs]` dans `Cargo.toml` — décide la langue dans
laquelle `rbs add` et `rbs generate crud` écrivent les messages qu'ils remettent au
client. Ceux-ci deviennent de simples littéraux de chaîne dans votre code engendré, et
`[server] lang` ne les touche plus ensuite : le contexte `lang` lu dans
`crates/rbs-cli/src/add/mod.rs` et l'appel `.speaking(metadonnees.lang)` dans
`crates/rbs-cli/src/generate/command.rs` choisissent l'un des deux littéraux au rendu,
une fois pour toutes.

Ces messages engendrés, fichier par fichier :

| Fichier | Message |
|---|---|
| `src/auth/repository/user.rs` | `ADRESSE_PRISE`, le 409 d'une inscription en double |
| `src/modules/rate_limit/mod.rs` | le message du 429 |
| `src/modules/webhooks/service.rs` | le 400 d'un motif d'événement vide |
| `src/modules/webhooks/target.rs` | les trois refus d'URL, 400 |
| `src/modules/webhooks/repository.rs` | `NotFound("subscription")` / `"abonnement"` |
| `repository.rs` d'un CRUD engendré | le 409 d'une valeur unique en double |
| `filter.rs` d'un CRUD engendré | le 400 d'une colonne de tri inconnue |
| `controller.rs` et `service.rs` d'un CRUD engendré, sous `--with-upload` | `NotFound("content")` / `"contenu"` |

Ce qui ne suit ni l'un ni l'autre réglage :

- le `title` de `Domain` lui-même — le `code` que vous avez choisi, pas un mot que rbs
  aurait décidé à votre place ;
- les codes qu'une règle `validator` inscrit dans `errors` (`email`, `length`, …) — aucun
  DTO engendré ne leur pose de `message`, ils restent les codes bruts, faits pour être
  comparés plutôt que lus ;
- le `message` que vous passez vous-même à `BadRequest`, `Conflict`, `Domain` et les
  autres.

Ce qui reste en français quoi qu'il arrive : les lignes de journal — sauf les refus d'URL
des webhooks, dont le texte est aussi celui que journalise une livraison bloquée
(`crates/rbs-cli/templates/features/webhooks/delivery.rs.jinja`), et qui suit donc la
langue de génération sur cette seule ligne. Restent aussi hors de portée : les
commentaires de code ; les courriels de `mail` et `auth` (objets et gabarits HTML sous
`templates/mail/`) ; les descriptions et résumés par opération écrits dans les handlers
engendrés (`#[utoipa::path(… description = …)]` et les commentaires de documentation) ; et
les descriptions de schéma que portent les types de `rbs-core` eux-mêmes (les champs de
`ProblemDetails`, `Page`, `CursorPage`, les schémas de filtre).

Basculer un projet existant vers l'anglais tient en deux modifications, toutes deux à la
main : `lang = "en"` sous `[server]` dans `config/default.toml` (exécution) et
`lang = "en"` sous `[package.metadata.rbs]` dans `Cargo.toml` (pour qu'un futur
`add`/`generate` écrive en anglais) — puis traduire à la main les messages déjà engendrés,
listés ci-dessus. `rbs upgrade` n'en réécrit aucun.

Une divergence à surveiller sur un projet plus ancien : un projet engendré avant 1.5.0
sans `--lang` inscrivait `[package.metadata.rbs] lang` d'après la locale — `en` pour
toute locale non française, y compris le `C.UTF-8` des runners CI et des images Docker
(voir `from_locale` dans `crates/rbs-cli/src/lang.rs`) — alors que son exécution, sans
`[server] lang`, parle français. Depuis 1.5.0, `add` et `generate` écrivent leurs messages
dans la langue de la métadonnée : aligner les deux clés, dans un sens ou dans l'autre,
avant de générer dans un tel projet.

## Comment une erreur devient une réponse

Rien n'est câblé à la main. `Error` implémente le `IntoResponse` d'Axum : un handler qui
rend `rbs_core::Result<T>` est déjà un handler Axum valide. Les deux middlewares qui
complètent le tableau sont montés une fois, sur le routeur :

```rust file=examples/hello-crud/src/router.rs region=montage
```

Certaines erreurs n'atteignent jamais votre code. `ValidatedJson<T>` désérialise *puis*
valide, et traduit les deux échecs dans la bonne variante avant que votre controller ne
s'exécute — un corps mal formé en `BadRequest`, un corps qui enfreint une règle
`validator` en `Validation`. `Pagination` fait de même avec un `?page=` ou un `?per_page=`
illisible. Une taille de page hors bornes, en revanche, est ramenée en silence : une borne
n'est pas une faute qu'il faille signaler au client, alors que `per_page=abc` en est une.

Les tests générés éprouvent les deux côtés de cette frontière. Un identifiant inconnu :

```rust file=examples/hello-crud/src/articles/tests.rs region=erreur_404
```

Et un corps que rien ne permet de lire :

```rust file=examples/hello-crud/src/articles/tests.rs region=corps_illisible
```

## Jugez par vous-même

Chaque variante porte un test sur son statut, sur son corps, et — pour les deux internes —
sur ce que son corps ne doit *pas* contenir :

```bash
cargo test -p rbs-core error::tests
```
