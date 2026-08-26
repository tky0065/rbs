# Squelette de projet généré — conception

Ce document fige le contenu de `templates/project/`, le squelette que `rbs new` déroule.
Il est plus détaillé que les autres notes de conception parce que C3, C7 et tout le lot D
écrivent *dans* ces fichiers : leur forme est une interface, pas un détail
d'implémentation. La conception d'ensemble reste
`docs/superpowers/specs/2026-08-25-rbs-design.md`, dont ce document précise les §3.3,
§4.5 et §5.4.

## 1. Convention de nommage des templates

Toutes les templates portent le suffixe `.jinja`, sans exception. Le chemin de sortie est
le chemin de la template privé de ce suffixe.

Ce n'est pas cosmétique. Un fichier réellement nommé `.gitignore` sous
`templates/project/` serait lu par Git comme un `.gitignore` du dépôt rbs lui-même et
retirerait des fichiers du suivi. Le suffixe supprime cette classe de problème d'un coup :
il empêche aussi Cargo de découvrir un `Cargo.toml` qui n'est pas un manifeste, et
rust-analyzer d'analyser des `.rs` qui ne compilent pas — ils contiennent des `{@ … @}`.

## 2. Arborescence

```
templates/project/
├── Cargo.toml.jinja
├── .env.example.jinja
├── .gitignore.jinja
├── config/
│   ├── default.toml.jinja
│   └── development.toml.jinja
├── src/
│   ├── main.rs.jinja
│   ├── router.rs.jinja
│   ├── state.rs.jinja
│   ├── openapi.rs.jinja
│   └── features/
│       ├── mod.rs.jinja
│       └── health/
│           ├── mod.rs.jinja
│           └── controller.rs.jinja
└── migration/
    ├── Cargo.toml.jinja
    └── src/
        └── lib.rs.jinja
```

## 3. Variables de template

| Variable | Exemple | Rôle |
|---|---|---|
| `nom_projet` | `mon-api` | nom du paquet Cargo et du répertoire, tel que saisi |
| `nom_crate` | `mon_api` | même nom en snake_case, pour les chemins de modules |
| `rbs_core_dep` | `"0.1"` | fragment de dépendance injecté tel quel dans `Cargo.toml` |

`rbs_core_dep` existe parce que `rbs-core` n'est pas encore publié sur crates.io. Écrire
`rbs-core = "0.1"` en dur rendrait tout projet généré incompilable, donc les critères de
C7 et C8 inatteignables. La variable vaut `"0.1"` par défaut et
`{ path = "/chemin/vers/crates/rbs-core" }` quand `rbs new --core-path <dir>` est passé.
Les tests d'intégration et les projets d'`examples/` utilisent le flag ; l'utilisateur
final reçoit la version publiée. Le flag disparaîtra quand `rbs-core` sera sur crates.io,
sans que les templates changent.

## 4. Les quatre ancres

| Ancre | Fichier | Contenu inséré |
|---|---|---|
| `<rbs:features>` | `src/features/mod.rs` | `pub mod users;` |
| `<rbs:routes>` | `src/router.rs` | `.merge(features::users::routes())` |
| `<rbs:openapi>` | `src/openapi.rs` | les `paths(...)` utoipa d'une feature |
| `<rbs:migrations>` | `migration/src/lib.rs` | `Box::new(m20260826_000001_create_users::Migration)` |

Chaque ancre est une paire `// <rbs:nom>` / `// </rbs:nom>`, et l'insertion se fait juste
avant la balise fermante (§4.5 de la conception d'ensemble).

`src/openapi.rs` n'apparaît pas dans l'anatomie de §3.3, mais §5.4 exige une ancre
`<rbs:openapi>` : il lui faut un fichier. L'isoler évite d'alourdir `router.rs`, qui est
le fichier que le développeur rouvre le plus souvent.

## 5. Contenu des fichiers structurants

### `src/main.rs`

Le critère de la tâche porte sur ce fichier : il doit tenir en une vingtaine de lignes
lisibles sans documentation.

```rust
mod features;
mod openapi;
mod router;
mod state;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rbs_core::logs::init()?;

    let config = rbs_core::Config::load()?;
    let adresse = format!("{}:{}", config.server.host, config.server.port);
    let db = rbs_core::db::connect(&config.database).await?;

    let app = router::router(state::AppState::new(db, config));

    let listener = tokio::net::TcpListener::bind(&adresse)
        .await
        .with_context(|| format!("impossible d'écouter sur {adresse}"))?;

    tracing::info!(%adresse, "démarrage");
    axum::serve(listener, app).await?;

    Ok(())
}
```

L'ordre des cinq étapes — logs, configuration, base, routeur, écoute — est la seule chose
que ce fichier a à dire. `logs::init()` vient en premier pour que l'échec du chargement de
configuration soit lui-même journalisé.

### `src/state.rs`

```rust
use rbs_core::{Config, CoreState, HasCoreState};
use sea_orm::DatabaseConnection;

/// État partagé par tous les handlers du projet.
#[derive(Debug, Clone)]
pub struct AppState {
    core: CoreState,
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config) -> Self {
        Self {
            core: CoreState::new(db, config),
        }
    }
}

impl HasCoreState for AppState {
    fn core(&self) -> &CoreState {
        &self.core
    }
}
```

Le projet possède son propre `AppState` dès la génération, même s'il n'ajoute encore aucun
champ : y greffer un client Redis ou un compteur ne demandera pas de restructurer
`main.rs` ni les handlers.

### `src/router.rs`

```rust
use axum::Router;
use axum::middleware::from_fn;

use crate::features;
use crate::openapi;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(features::health::routes())
        // <rbs:routes>
        // </rbs:routes>
        .merge(openapi::routes())
        .layer(from_fn(rbs_core::trace::middleware))
        .layer(from_fn(rbs_core::request_id::middleware))
        .with_state(state)
}
```

L'ordre des deux `layer` n'est pas indifférent : `request_id` est ajouté en dernier, donc
s'exécute le plus à l'extérieur, et le span ouvert par `trace` porte déjà l'identifiant.

### `src/features/mod.rs`

```rust
pub mod health;
// <rbs:features>
// </rbs:features>
```

### `src/features/health/`

Feature vitrine mince : deux fichiers. `rbs-core` fournit déjà le handler et sa
vérification de base ; la dupliquer serait du code que le noyau devrait maintenir deux
fois. Ce que le projet gagne, c'est un exemple de la forme d'une feature — un `mod.rs`
exposant `routes()`, un `controller.rs` portant les annotations utoipa — sous les yeux du
développeur dès la génération, et éditable.

`mod.rs` :

```rust
pub mod controller;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(controller::sante))
}
```

**`controller` est un module public, et l'ancre `<rbs:openapi>` reçoit le chemin complet
du handler** — `crate::features::health::controller::sante`, jamais un ré-export. C'est
une contrainte d'utoipa, pas un choix de style : `#[utoipa::path]` génère à côté du
handler une struct `__path_sante`, et `paths(...)` la résout dans le module du dernier
segment du chemin donné. Un `pub use controller::sante;` laisserait cette struct
inaccessible et le projet généré ne compilerait pas :

```
error[E0433]: cannot find `__path_sante` in `health`
note: struct `crate::features::health::controller::__path_sante` exists but is inaccessible
```

L'autre correctif possible, `pub use controller::{__path_sante, sante};`, exposerait un
identifiant magique dans un fichier que l'utilisateur est censé lire et modifier. Le
chemin complet ne coûte rien et n'apprend rien de faux à qui lit le code.

**Le lot D suit la même convention** : chaque feature générée expose `pub mod controller;`
et l'insertion dans `<rbs:openapi>` prend la forme
`crate::features::<nom>::controller::<handler>`.

`controller.rs` :

```rust
use axum::extract::State;
use axum::response::Response;

use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "l'application et ses dépendances répondent"))
)]
pub async fn sante(state: State<AppState>) -> Response {
    rbs_core::health::handler(state).await
}
```

La route est montée par le projet, pas par le noyau : c'est ce qui la rend éditable. Seule
la logique — le `ping` de la base et le corps de la réponse — reste dans `rbs-core`, qui
n'a aucune raison de varier d'un projet à l'autre. Monter directement
`rbs_core::health::routes()` aurait privé le développeur du point d'entrée qu'il modifiera
le premier.

### `src/openapi.rs`

```rust
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    modifiers(&rbs_core::ReponsesCommunes),
    paths(
        crate::features::health::sante,
        // <rbs:openapi>
        // </rbs:openapi>
    )
)]
pub struct ApiDoc;
```

`routes()` monte Swagger UI sur `/docs` et le document sur `/api-docs/openapi.json`, et
consulte la configuration avant chacun des deux :

```rust
pub fn routes(config: &rbs_core::Config) -> Router<AppState> {
    let mut router = Router::new();

    if config.docs.openapi_json {
        router = router.route("/api-docs/openapi.json", get(document));
    }
    if config.docs.swagger_ui {
        router = router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()));
    }

    router
}
```

`config.docs` n'existe pas encore dans `rbs_core::Config` : il est ajouté par la tâche
`B10`, avec les deux champs `swagger_ui` et `openapi_json`, tous deux à `true` par défaut.
Sans elle, §5.4 — « les deux sont désactivables par configuration en production » — resterait
lettre morte.

### `Cargo.toml`

```toml
[workspace]
members = ["migration"]
resolver = "3"

[package]
name = "{@ nom_projet @}"
version = "0.1.0"
edition = "2024"

[package.metadata.rbs]
version = "0.1.0"
features = ["health"]

[dependencies]
rbs-core = {@ rbs_core_dep @}
…
```

Les versions des dépendances sont figées en dur, alignées sur celles du workspace rbs :
axum 0.8, sea-orm 2.0, utoipa 5.5, utoipa-swagger-ui 9.0, tokio 1.53, serde 1.0,
validator 0.21, anyhow 1.0, tracing 0.1.

La section `[package.metadata.rbs]` est présente dès la génération, avec `health` comme
seule feature installée. C5 en écrira la lecture et la mise à jour ; C4 se contente de la
poser.

### `migration/`

Crate SeaORM standard, membre du workspace du projet. `src/lib.rs` porte l'ancre
`<rbs:migrations>` dans le `Vec` retourné par `MigratorTrait::migrations`. Aucune
migration initiale n'est générée : le vecteur est vide, et `rbs generate crud` le
remplira.

## 6. Périmètre

C4 écrit des fichiers de templates, rien d'autre. Le rendu est C2, l'embarquement dans le
binaire est C3, la commande `rbs new` est C7.

**Ce que la tâche peut prouver** : un test lisant `templates/project/` et vérifiant que
chaque ancre attendue est présente, dans le bon fichier, correctement refermée, et que la
convention de suffixe est respectée.

**Ce qu'elle ne peut pas prouver** : que le projet généré compile et démarre. Aucun rendu
n'est possible avant C7. Le critère de lecture de `main.rs` est validé par le propriétaire
du projet, celui de la compilation attend C7 et C8.
