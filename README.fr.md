# rbs

Un cadre de travail Rust pour les API web, bâti sur Axum et SeaORM. Il donne à un projet ce
qui n'a aucune raison de varier d'une API à l'autre — erreurs, logs, configuration, accès à
la base, OpenAPI — et génère le reste dans vos propres sources, où vous pouvez le lire et le
modifier.

*[English version](README.md).*

## Statut

La version 0.1 est en construction.

**Aucune promesse semver n'est faite avant la 1.0.** L'API publique de `rbs-core` peut
changer entre deux versions mineures, sans cycle de dépréciation et sans chemin de
migration. Épinglez une version exacte, lisez le journal des commits avant de monter de
version, et attendez-vous à corriger du code. Figer cette API, c'est ce que veut dire 1.0
ici ; jusque-là, rien n'est figé.

rbs n'est pas non plus publié sur crates.io — cela viendra également avec la 1.0.

## Installation

Rust 1.85 ou plus. Les projets générés visent PostgreSQL 18 ou plus.

```bash
cargo install --git https://github.com/tky0065/rbs rbs-cli
```

Le paquet s'appelle `rbs-cli` ; le binaire installé s'appelle `rbs`.

Cette commande vous donne le binaire, et rien d'autre. Tant que `rbs-core` n'est pas sur
crates.io, un projet généré a besoin d'une copie locale contre laquelle compiler — ce qui
suppose de cloner le dépôt plutôt que d'installer depuis lui. Le
**[guide de démarrage](https://tky0065.github.io/rbs/fr/getting-started)** porte cette
séquence, ainsi que la base de données que les commandes attendent ; suivez-le plutôt que
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

C'est la silhouette de la chose, pas une transcription à coller : la 0.1 réclame deux
arguments de plus que ceux montrés ici, et une base de données qui répond. Le
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
référence du CLI, guides. Le binaire porte cinq commandes — `new`, `add`, `generate`,
`migrate`, `doctor` — et le site documente chacune d'elles.

[`ROADMAP.md`](ROADMAP.md) énumère ce que couvre la 0.1, ce qu'ajoutent les jalons suivants,
et ce qui est délibérément laissé de côté.

## Contribuer

Commencez par [`CONTRIBUTING.fr.md`](CONTRIBUTING.fr.md) : ce qu'il faut installer, les
vérifications que lance la CI, et les conventions que le dépôt s'impose. Travailler sur le
code Rust ne demande jamais Node. Le projet suit le
[Contributor Covenant](CODE_OF_CONDUCT.md).

## Licence

Double licence, [MIT](LICENSE-MIT) ou [Apache 2.0](LICENSE-APACHE), au choix.
