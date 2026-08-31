# rbs

Un cadre de travail Rust pour les API web, bâti sur Axum et SeaORM. Il donne à un projet ce
qui n'a aucune raison de varier d'une API à l'autre — erreurs, logs, configuration, accès à
la base, OpenAPI — et génère le reste dans vos propres sources, où vous pouvez le lire et le
modifier.

*[English version](README.md).*

## Statut

Version 1.1.0. Les six jalons de la feuille de route sont livrés — le socle,
l'authentification, les intégrations, le confort, la stabilité, les agents — et
[`CHANGELOG.fr.md`](CHANGELOG.fr.md) dit ce que chacun apporte.

**rbs suit le versionnage sémantique à partir de la 1.0.** L'API publique de `rbs-core` est
figée : à l'intérieur de la 1.x, rien n'est retiré, renommé ni doté d'un autre sens, et
`cargo-semver-checks` fait échouer la construction plutôt que de laisser passer. Le format
des ancres en commentaires et de `[package.metadata.rbs]` est couvert lui aussi : un projet
engendré par une version du CLI reste lisible par la suivante. Le code engendré dans vos
propres sources ne l'est pas — il vous appartient dès qu'il est écrit, et aucune version de
rbs ne le réécrit. La [page de compatibilité](https://tky0065.github.io/rbs/fr/compatibility)
énonce les cinq périmètres.

## Installation

Rust 1.85 ou plus. Un projet généré tourne sur PostgreSQL 14 ou plus, MySQL 8.0 ou plus, ou
SQLite 3.35 ou plus.

```bash
cargo install rbs-cli
```

Le paquet s'appelle `rbs-cli` ; le binaire installé s'appelle `rbs`.

Cette commande vous donne le binaire, et rien d'autre : le
**[guide de démarrage](https://tky0065.github.io/rbs/fr/getting-started)** porte la base de
données que les commandes attendent, et la sortie de chacune d'elles. Suivez-le plutôt que
l'esquisse ci-dessous, qui laisse les deux de côté.

> L'écosystème Ruby publie un outil sans rapport, lui aussi nommé `rbs`. Si
> `rbs --version` affiche quelque chose comme `rbs 3.10.0`, c'est celui-là qui l'emporte
> sur votre `PATH` — utilisez `rbs-cli`, que la même installation a déposé sous un nom que
> personne d'autre ne revendique.

## Aperçu

Ce que fait rbs, en quatre commandes — d'un répertoire vide à une API CRUD, avec son
entité, sa migration, son document OpenAPI et ses tests d'intégration :

```bash
rbs new blog-api
cd blog-api
rbs generate crud articles --fields 'title:string,body:text,published:bool'
rbs migrate up
```

C'est la silhouette de la chose, pas une transcription à coller : il y faut une base de
données qui répond, et `rbs new` vous posera deux questions que les commandes ci-dessus ne
portent pas — trois si vous omettez aussi le nom. Le
[guide de démarrage](https://tky0065.github.io/rbs/fr/getting-started) en donne la version
exécutable, avec la sortie de chaque commande.

## Ce que rbs apporte

`rbs-core` est le runtime : erreurs typées rendues en documents de problème RFC 9457, un
formateur de logs qui reste lisible en développement et devient du JSON en production, une
configuration validée au démarrage, la pagination, l'état de l'application. Il porte ce qui
n'a aucune raison de varier d'un projet à l'autre.

La commande `rbs` écrit tout le reste dans votre propre projet — model, DTO, repository,
service, controller, migration, tests — en Rust clair, sans macro à déplier.

Ce code généré est fait pour être modifié. Rien n'y porte de bandeau « généré, ne pas
modifier », parce que rien ne vient réécrire vos changements.

Les features dépendent dans un seul sens : `controller → service → repository → model`.
Voici l'intégralité du handler `POST /articles`, tel que `rbs generate crud` l'écrit, lu
depuis
[`examples/hello-crud/src/articles/controller.rs`](examples/hello-crud/src/articles/controller.rs) :

```rust
#[utoipa::path(
    post,
    path = "/articles",
    tag = "articles",
    request_body = CreateArticle,
    responses(
        (status = 201, description = "article créé", body = ArticleResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(entree): ValidatedJson<CreateArticle>,
) -> Result<(StatusCode, Json<ArticleResponse>)> {
    let article = service::create(state.core().db(), entree).await?;

    Ok((StatusCode::CREATED, Json(article)))
}
```

Le controller passe la requête au service et traduit le résultat en code de statut — le
service, lui, ne voit jamais de `DatabaseConnection`.

## Documentation

Le site est à l'adresse **<https://tky0065.github.io/rbs/fr/>** : démarrage, architecture,
référence du CLI, guides. Le binaire porte huit commandes — `new`, `add`, `generate`,
`migrate`, `seed`, `dev`, `doctor`, `upgrade` — et le site documente chacune d'elles.

[`CHANGELOG.fr.md`](CHANGELOG.fr.md) dit ce qu'a apporté chaque version, écrit pour qui
installe rbs. [`ROADMAP.md`](ROADMAP.md) prend l'autre sens : ce que couvrent les jalons, et
ce qui est délibérément laissé de côté.

## Contribuer

Commencez par [`CONTRIBUTING.fr.md`](CONTRIBUTING.fr.md) : ce qu'il faut installer, les
vérifications que lance la CI, et les conventions que le dépôt s'impose. Travailler sur le
code Rust ne demande jamais Node. Le projet suit le
[Contributor Covenant](CODE_OF_CONDUCT.fr.md).

## Licence

Double licence, [MIT](LICENSE-MIT) ou [Apache 2.0](LICENSE-APACHE), au choix.
