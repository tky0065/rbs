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

`title`, le `detail` fixe des variantes ci-dessus, les messages qu'un fragment écrit dans
une erreur `Domain` ou une règle `validator`, et les descriptions communes d'OpenAPI
partagées par toutes les opérations d'un même statut suivent tous `[server] lang` — `fr`
par défaut, ou `en` :

```toml
[server]
lang = "en"
```

Le régler suffit ; aucun handler, aucun gabarit, aucune annotation OpenAPI ne le lit lui-même.

Ce qui n'en dépend pas :

- le `title` de `Domain` lui-même — le `code` que vous avez choisi, pas un mot que rbs
  aurait décidé à votre place ;
- les codes qu'une règle `validator` inscrit dans `errors` (`email`, `length`, …) — un
  client est censé les comparer, pas les lire ;
- le `message` que vous passez à `BadRequest`, `Conflict`, `Domain` et les autres — vous
  l'avez écrit, sa traduction vous revient.

Ce qui reste en français quoi qu'il arrive : les lignes de journal, les commentaires de
code, et les courriels envoyés par `mail` et `auth` — une réinitialisation de mot de
passe, une vérification d'adresse. Rien de tout cela ne relève de `[server] lang`.

Basculer un projet existant tient en une ligne — `lang = "en"` sous `[server]` — mais ne
change que ce que le runtime rend à partir de là. Un message qu'un fragment a déjà écrit
dans votre code (une erreur `Domain`, un message `validator`) garde la langue dans
laquelle vous l'avez écrit ; seule votre propre relecture le traduit.

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
