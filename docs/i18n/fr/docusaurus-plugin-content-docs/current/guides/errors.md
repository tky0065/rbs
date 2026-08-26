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

| Variante | Statut | `title` | Corps |
|---|---|---|---|
| `NotFound(&'static str)` | 404 | `Not Found` | `detail` nomme la ressource |
| `BadRequest(String)` | 400 | `Bad Request` | `detail` porte la cause |
| `Validation(ValidationErrors)` | 422 | `Validation failed` | `errors`, champ par champ |
| `Unauthorized` | 401 | `Unauthorized` | — |
| `Forbidden` | 403 | `Forbidden` | — |
| `Conflict(String)` | 409 | `Conflict` | `detail` porte le message |
| `Domain { status, code, message }` | le vôtre | le `code` | `detail` porte le message |
| `Database(DbErr)` | 500 | `Internal Server Error` | une phrase fixe |
| `Internal(anyhow::Error)` | 500 | `Internal Server Error` | une phrase fixe |

Trois d'entre elles s'atteignent sans jamais être nommées : `DbErr`, `anyhow::Error` et
`ValidationErrors` ont chacune une implémentation de `From`, et `?` les convertit au
passage. `Domain` est la porte de sortie — une erreur métier qui choisit son statut et un
code stable — et elle existe pour éviter qu'un projet généré n'empile sa propre hiérarchie
d'erreurs par-dessus celle-ci.

`BadRequest` et `Validation` partagent une frontière qui mérite d'être dite : 400 signifie
*je n'ai pas pu lire ton corps*, 422 signifie *je l'ai lu, il enfreint une règle*.

## Le corps

Les réponses suivent la RFC 9457, avec le type de média `application/problem+json`. Un
échec de validation ressemble à ceci :

```json
{
  "type": "about:blank",
  "title": "Validation failed",
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

`Database` et `Internal` sont les deux variantes qui ne disent rien. Leur source part au
journal en `ERROR` et s'y arrête ; le client obtient « une erreur interne est survenue »
et l'identifiant de requête. C'est délibéré : une chaîne de connexion, un hôte, un secret
manquant sont autant de choses qu'un message d'erreur livre volontiers à qui demande. Deux
tests n'existent que pour échouer le jour où une source fuiterait dans le corps.

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
