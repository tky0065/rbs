---
sidebar_position: 3
title: Architecture
---

# Architecture

rbs repose sur trois décisions. Où passe la frontière entre le cadre et votre code ; de
quoi une feature est faite ; dans quel sens ses morceaux ont le droit de pointer. Chacune
motive la suivante, d'où l'ordre de cette page — et chacune se montre sur
[`examples/hello-crud`](https://github.com/tky0065/rbs/tree/main/examples/hello-crud), un
projet que la CI compile, plutôt que sur du code écrit pour l'occasion.

## La frontière noyau / généré

Tout cadre de travail doit dire ce qu'il garde et ce qu'il cède. rbs y répond par un seul
test, appliqué à chaque morceau de code dont il a la charge :

> **Un développeur voudra-t-il le relire ?**

Si non, sa place est dans `rbs-core` — une dépendance de votre `Cargo.toml`, mise à jour
comme n'importe quelle autre. Ouvrir un pool de connexions, lire un numéro de page,
colorer une ligne de log, traduire une erreur en corps RFC 9457 : personne n'ouvre ces
fichiers-là, et personne ne devrait avoir à le faire.

Si oui, l'outil en ligne de commande `rbs` l'écrit dans votre `src/`, et il vous
appartient. La forme d'un `Article`, la règle qui décide quand il est publiable, la route
qui les liste : voilà le code qu'on ouvre le jour où quelque chose change.

Notez ce que ce test n'est pas. Il ne trie pas par généralité : un service CRUD est assez
générique pour vivre dans le noyau derrière un trait, et rbs le génère quand même, parce
qu'un service est le premier endroit où l'on regarde quand une règle métier bouge. C'est
pourquoi aucun fichier généré ne porte de bandeau « généré, ne pas modifier ». Il est fait
pour être modifié, et la frontière existe pour que le modifier suffise.

### Ce que porte le noyau

Onze modules publics, tous du côté « personne ne veut relire ça » :

| Module | Ce qu'il fait | Pourquoi il ne varie jamais |
|---|---|---|
| `config` | Charge et valide la configuration de l'application | Superposer fichiers, environnement et valeurs par défaut est le même problème partout |
| `db` | Ouvre le pool de connexions au démarrage | Une base injoignable doit arrêter le processus, pas surgir au premier appel HTTP |
| `error` | L'erreur du runtime et son alias `Result` | Une seule erreur, une seule traduction HTTP — sa valeur tient à être partagée |
| `extract` | Les extracteurs de requête, dont `ValidatedJson` | Désérialiser puis valider un corps est de la plomberie, identique partout |
| `health` | Le handler de santé | Le noyau porte le contrôle ; le projet généré décide où le monter |
| `logs` | Les formateurs `pretty` et `json` | Un format de log est un style de maison, pas une décision de projet |
| `openapi` | `ProblemDetails` et les réponses d'erreur communes | Déclarées une fois, les réponses d'erreur ne peuvent plus diverger entre opérations |
| `pagination` | L'extracteur `Pagination` et l'enveloppe `Page` | Les bornes sont des constantes du noyau, ce qui garde l'extracteur sans état |
| `request_id` | L'identifiant de corrélation de la requête courante | Lu par les logs et les réponses d'erreur, qui ne l'ont jamais reçu en paramètre |
| `state` | `CoreState` — pool et configuration — et `HasCoreState` | Le projet possède son `AppState` ; le noyau ne possède que ce qu'il doit y atteindre |
| `trace` | Le span par requête et le log de son issue | Toute API HTTP veut le même span, avec les mêmes champs |

La crate réexporte la poignée d'items qu'une feature nomme sans cesse — `Config`, `Error`,
`Result`, `ValidatedJson`, `ProblemDetails`, `Page`, `Pagination`, `CoreState`,
`HasCoreState` — pour que le code généré les importe directement depuis `rbs_core`.

`AppState` est la frontière en miniature. Le noyau porte `CoreState`, le pool et la
configuration ; votre projet déclare son propre `AppState` autour, libre d'y gagner un
client Redis ou un service mail sans demander la permission au cadre. Les handlers du
noyau atteignent le pool par le trait `HasCoreState`, quel que soit l'état qui l'enveloppe.

### Un feature flag rempli, trois encore vides

`rbs-core` déclare quatre features Cargo au-delà des pilotes de base. L'une porte du code,
les trois autres réservent un nom et rien d'autre :

| Flag | État | Ce qu'il porte |
|---|---|---|
| `auth` | **rempli** | Hachage Argon2, JWT, jetons opaques, extracteur d'identité |
| `redis` | vide | Un client Redis partagé par l'état applicatif |
| `mail` | vide | Envoi de courriels et rendu de gabarits |
| `storage` | vide | Stockage de fichiers, local ou compatible S3 |

En activer une des trois vides ne compile rien de plus et n'ajoute aucune dépendance : ce
n'est pas une erreur, c'est simplement sans effet. Les nommer à l'avance n'a rien coûté et
a tranché la question de leur nom ; `auth` est ce à quoi cette réservation servait, et elle
a été remplie sans rien renommer.

Les trois vides sont aussi hors de la promesse de compatibilité : elles ne portent aucune
API publique à geler. Les remplir est un ajout, jamais une rupture.

## L'anatomie d'une feature

Tout ce qui est du côté généré s'organise par feature, jamais par couche : un répertoire
par ressource, six fichiers dedans. `rbs generate crud articles` les écrit tous.

```text
src/articles/
├── mod.rs          déclare les voisins, expose les routes
├── model.rs        l'entité SeaORM — la table, en Rust
├── dto.rs          ce qui traverse la frontière HTTP, à l'aller comme au retour
├── repository.rs   le seul endroit où se construit une requête
├── service.rs      les règles métier
└── controller.rs   le HTTP : extraction, codes de statut, OpenAPI
```

Pas de `src/models/`, pas de `src/services/`. Une feature se lit, se déplace et se
supprime d'un bloc, et un simple listing du répertoire dit ce que l'API fait.

### `mod.rs` — le câblage

Il déclare les cinq autres fichiers et publie les routes de la feature sous forme de
`Router` que le routeur du projet fusionne. Rien d'autre n'y vit.

```rust file=examples/hello-crud/src/articles/mod.rs region=routes
```

### `model.rs` — l'entité

La table en type Rust, et le seul endroit qui la décrit. La clé primaire est un UUID que
l'application engendre, non une séquence que la base distribue.

```rust file=examples/hello-crud/src/articles/model.rs region=entite
```

### `dto.rs` — les types de la frontière

Séparés du modèle à dessein : une colonne n'est pas un champ de votre API. `CreateArticle`
dit ce qu'un client a le droit d'envoyer — `id`, `created_at` et `updated_at` en sont
absents parce qu'ils ne sont pas au client de les fixer —

```rust file=examples/hello-crud/src/articles/dto.rs region=entree
```

— et `ArticleResponse` dit ce qu'il reçoit en retour, avec les annotations `utoipa` qui le
portent dans le document OpenAPI.

```rust file=examples/hello-crud/src/articles/dto.rs region=reponse
```

Le jour où ces deux-là s'écartent de la table, ils s'écartent seuls : le modèle ne les
suit pas.

### `repository.rs` — les requêtes

Le seul fichier qui construise une requête SeaORM. Il prend la connexion en argument et
rend des modèles, jamais des DTO.

```rust file=examples/hello-crud/src/articles/repository.rs region=list
```

### `service.rs` — les règles

Il compose les appels au dépôt et traduit le résultat en DTO. « Introuvable » est un
verdict métier, pas un verdict de base de données : il se décide ici, ce qui laisse le
dépôt libre de rendre une `Option`.

```rust file=examples/hello-crud/src/articles/service.rs region=find
```

### `controller.rs` — la surface HTTP

Extraction, code de statut, annotation OpenAPI, et rien de plus. `ValidatedJson` a déjà
rejeté un corps illisible ou invalide avant que cette fonction ne s'exécute.

```rust file=examples/hello-crud/src/articles/controller.rs region=create
```

## La règle de dépendance

Les six fichiers sont ordonnés, et les flèches pointent toutes dans le même sens :

```text
controller ──> service ──> repository ──> model
     │            │                          ▲
     └────────────┴───────> dto ─────────────┘
```

Rien ne pointe vers la gauche. Un dépôt n'appelle pas un service ; un modèle ne sait rien
du HTTP. C'est ce qui rend chaque fichier lisible isolément : pour comprendre `service.rs`
il faut connaître les signatures du dépôt, ni les extracteurs d'Axum ni le constructeur de
requêtes de SeaORM.

Deux conséquences en découlent, et elles se voient dans les imports plutôt que dans une
règle qu'il faudrait retenir. Voici ce que le dépôt nomme :

```rust file=examples/hello-crud/src/articles/repository.rs region=imports
```

et voici ce que le service nomme :

```rust file=examples/hello-crud/src/articles/service.rs region=imports
```

**Un contrôleur ne construit jamais de requête.** Il a un `State`, donc il le pourrait.
Demandons à la feature quels fichiers connaissent SeaORM :

```bash
grep -l sea_orm examples/hello-crud/src/articles/*.rs
```

```text
examples/hello-crud/src/articles/controller.rs
examples/hello-crud/src/articles/dto.rs
examples/hello-crud/src/articles/filter.rs
examples/hello-crud/src/articles/model.rs
examples/hello-crud/src/articles/repository.rs
examples/hello-crud/src/articles/service.rs
```

Six fichiers sur sept — « seul le dépôt importe SeaORM » serait donc faux, et il vaut la
peine de dire pourquoi. Quatre de ces autres fichiers nomment `sea_orm` pour ses types
scalaires, `Uuid` et `DateTimeWithTimeZone`, qui traversent toutes les couches ; le service
y ajoute `ActiveValue::Set` pour bâtir le modèle actif qu'il transmet. `filter.rs` fait
exception, et délibérément : il nomme les traits de requête, puisqu'il traduit un corps en
conditions. Il appartient à la couche du dépôt, ce que garantit le fait que `repository.rs`
en soit le seul appelant. La sonde plus étroite est la sonde honnête :

```bash
grep -l 'Entity::' examples/hello-crud/src/articles/*.rs
```

```text
examples/hello-crud/src/articles/repository.rs
```

Un seul fichier. `Entity::find` est appelé dans `repository.rs` et nulle part ailleurs —
`filter.rs` reçoit le `Select` déjà ouvert et le rend restreint, sans jamais atteindre
l'entité lui-même. Tout le vocabulaire de requête s'arrête à ces deux fichiers, et les
trois couches au-dessus ne le voient jamais.

**Un service ne détient jamais de connexion.** `DatabaseConnection` figure bien dans ses
imports, et l'extrait `find` ci-dessus dit pourquoi : le service reçoit un
`&DatabaseConnection` et le passe tel quel au dépôt. Il n'en range jamais un dans une
structure, n'appelle jamais une méthode dessus, n'apprend jamais quelle base est derrière.
L'emprunt traverse ; la connaissance, non.

C'est un coût assumé. Faire descendre `db` dans chaque signature s'écrit plus longuement
que garder une poignée, et cela achète un service qu'on lit sans savoir ce qu'est un pool.

### Quand un fichier s'allonge

**Un fichier de feature au-delà de ~200 lignes vous dit que la feature est à scinder.** Le
seuil n'est pas un lint, c'est une habitude de lecture : passé cette longueur, un fichier
cesse de tenir en une seule assise, et la frontière qui garde chaque couche lisible
commence à se payer en défilement.

La feature `articles` de `hello-crud` fait 26, 21, 49, 47, 77 et 101 lignes — un CRUD
complet, avec pagination, validation et document OpenAPI, dont aucun fichier n'atteint la
moitié du seuil. Le jour où l'un des vôtres le franchit, la réponse est rarement une
fonction plus courte. C'est que le répertoire abrite deux ressources sous un seul nom.
