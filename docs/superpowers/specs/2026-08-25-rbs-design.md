# rbs — Rust Backend Starter · Spécification de design

Date : 2026-08-25
Statut : validé, prêt pour le plan d'implémentation

## 1. Objectif

`rbs` fournit aux développeurs backend Rust un socle et un outillage pour démarrer
une API HTTP de production sans réécrire à chaque projet la même plomberie :
gestion d'erreurs, logs, configuration, accès base, documentation OpenAPI.

La stack est fixée : **Rust · Axum · SeaORM · utoipa/Swagger · PostgreSQL**.

Le projet livre deux choses indissociables :

1. un runtime (`rbs-core`) qui porte le boilerplate invisible ;
2. un CLI (`rbs`) qui génère le code que le développeur va lire et modifier.

## 2. Décisions arbitrées

Six décisions structurantes ont été prises en amont de ce document. Elles ne sont
pas rouvertes par l'implémentation.

| # | Décision | Retenu | Écarté |
|---|---|---|---|
| D1 | Modèle de distribution | Hybride : noyau publié + code de feature généré et visible | Template pur (non upgradable) · Framework runtime (trop de magie) |
| D2 | Features optionnelles | Primitives derrière feature flags dans le noyau + glue générée dans le projet | Tout dans le noyau (non personnalisable) · Une crate par feature (6 crates à versionner) |
| D3 | Injection de code | Ancres en commentaires `// <rbs:xxx>` | Auto-découverte par macro · Réécriture AST via `syn` |
| D4 | Périmètre v0.1 | Socle nu, boucle complète, sans feature optionnelle Rust | Socle + auth · CLI complet d'un bloc |
| D5 | Ambition | Open source public, publication crates.io | Outil personnel · Public léger |
| D6 | Documentation | Docusaurus, i18n intégré, FR + EN | mdBook deux livres · Markdown brut |
| D7 | Clés primaires des entités générées | UUIDv7 en colonne `uuid` native, produit par `DEFAULT uuidv7()` | ULID en `char(26)` · UUIDv4 · entier auto-incrémenté |

**Conséquence assumée de D6** : Docusaurus n'exécute pas les extraits de code. Les
exemples de la documentation sont extraits de projets réels du dossier `examples/`,
compilés en CI. Aucun extrait de code n'est écrit à la main dans le Markdown.

**Conséquence assumée de D7** : `uuidv7()` n'est une fonction native de PostgreSQL qu'à partir de la **version 18**. C'est donc la version minimale de tout projet généré par rbs. Elle est inscrite dans le `docker-compose` généré, dans l'image `testcontainers` des tests, et vérifiée par `rbs doctor`.

## 3. Architecture

### 3.1 Structure du dépôt

```
rbs/
├── Cargo.toml                 # workspace
├── crates/
│   ├── rbs-core/              # runtime, publié sur crates.io
│   └── rbs-cli/               # binaire `rbs`, publié sur crates.io
├── templates/
│   ├── project/               # squelette de `rbs new`
│   └── features/              # fragments de `rbs add` / `rbs generate`
├── examples/                  # projets réels, compilés en CI, source des extraits de docs
└── docs/                      # Docusaurus (toolchain Node isolée ici)
```

Pas de crate `rbs-macros` : aucune macro procédurale n'est nécessaire, les ancres
remplacent la magie qu'elle porterait.

### 3.2 Frontière noyau / généré

Règle : *`rbs-core` porte ce qui n'a pas de raison de varier d'un projet à l'autre ;
le CLI génère tout ce que le développeur voudra lire ou modifier.*

| `rbs-core` (invisible, upgradable) | Généré dans le projet (visible, éditable) |
|---|---|
| `Error` / `Result`, conversion HTTP | `main.rs`, `router.rs`, `state.rs` |
| Initialisation `tracing` (pretty / json) | Features (`model`, `dto`, `repository`, `service`, `controller`) |
| Chargement et validation de la config | Migrations |
| Pool SeaORM, health check | `Cargo.toml`, `.env.example` |
| Extracteurs validés, pagination, middlewares | Tests |
| Helpers OpenAPI / utoipa | |

### 3.3 Anatomie d'un projet généré

```
mon-api/
├── Cargo.toml              # rbs-core = "0.1", axum, sea-orm, utoipa, tokio
├── .env.example
├── config/
│   ├── default.toml
│   └── development.toml
├── src/
│   ├── main.rs             # ~25 lignes : config → logs → db → router → serve
│   ├── router.rs           # montage des features · ancre <rbs:routes>
│   ├── state.rs            # AppState du projet
│   └── features/
│       ├── mod.rs          # ancre <rbs:features>
│       └── health/
└── migration/              # crate SeaORM migration · ancre <rbs:migrations>
```

### 3.4 Anatomie d'une feature

```
features/users/
├── mod.rs          pub fn routes() -> Router<AppState>
├── model.rs        entité SeaORM
├── dto.rs          CreateUser / UpdateUser / UserResponse (+ validator, + ToSchema)
├── repository.rs   accès données pur — ne connaît que model.rs
├── service.rs      logique métier — ne connaît que repository.rs et dto.rs
└── controller.rs   handlers Axum + annotations utoipa — ne connaît que service.rs
```

**Règle de dépendance stricte et unidirectionnelle** :
`controller → service → repository → model`.

Chaque couche ne voit que la suivante. Un `service` n'accède jamais directement à
`DatabaseConnection` ; un `controller` ne construit jamais de requête SeaORM. Cette
règle rend chaque fichier lisible isolément et testable sans démarrer le serveur.

### 3.5 Flux de scaffolding : CLI d'abord

`rbs generate crud users --fields "name:string,email:string:unique"` génère **à la
fois** l'entité SeaORM et la migration correspondante, à partir de la description
des champs.

C'est l'inverse du flux `sea-orm-cli generate entity`, qui lit une base existante.
Le flux « CLI d'abord » évite d'exiger une base démarrée pour scaffolder. Le cas
« base legacy existante » n'est pas couvert en v0.1 ; il sera traité ultérieurement
par un passe-plat vers `sea-orm-cli`.

### 3.6 Clés primaires

Toute entité générée porte une clé primaire `id` de type `Uuid`, en colonne PostgreSQL
`uuid` native. La valeur est un **UUIDv7** (RFC 9562) : ses 48 bits de tête encodent
l'horodatage milliseconde, ce qui rend les clés croissantes dans le temps et garde les
insertions groupées en fin d'index, là où un UUIDv4 aléatoire fragmente le B-tree.

La valeur est produite par la base, la migration posant `DEFAULT uuidv7()` sur la
colonne. Une insertion faite hors de l'application — un script, un import, `psql` —
obtient donc un identifiant valide sans dupliquer la logique de génération.

`id` n'est jamais déclaré dans `--fields` : il est implicite, comme `created_at` et
`updated_at`.

**Version minimale : PostgreSQL 18.** `uuidv7()` n'y est native qu'à partir de cette
version ; `gen_random_uuid()`, disponible depuis la 13, produit un v4. Le choix est
assumé plutôt que contourné par une fonction PL/pgSQL maison : le SQL d'un projet
généré reste lisible et ne porte pas de compatibilité que rbs devrait maintenir.

## 4. Le CLI `rbs`

### 4.1 Surface de commandes

```
rbs new <nom>              création interactive ou --with <features> --yes
rbs add <feature>          v0.1 : docker, ci — v0.2+ : auth, redis, mail, storage
rbs generate crud <nom>    alias `rbs g crud` · --fields "name:string,email:string:unique"
rbs generate feature <nom> squelette vide, 6 fichiers, zéro champ
rbs migrate up|down|status|new <nom>
rbs doctor                 diagnostic : ancres, .env, base joignable, versions
```

Toute question interactive possède son équivalent en flag. `--yes` prend les défauts
sans rien demander : le CLI reste scriptable et testable en CI.

### 4.2 Détection du contexte projet

L'état du projet est stocké dans son `Cargo.toml`, sans fichier supplémentaire :

```toml
[package.metadata.rbs]
version  = "0.1.0"          # version de rbs qui a généré le projet
features = ["health"]       # mis à jour par chaque `rbs add`
```

Un seul emplacement, versionné par Git. C'est aussi ce qui rendra un futur
`rbs upgrade` possible sans mécanisme supplémentaire.

### 4.3 Moteur de templates

`minijinja`, templates embarquées dans le binaire via `include_dir` — l'installation
est autonome, sans fichier externe. Un flag `--template-dir` permet de surcharger les
templates, pour les tests et pour le développement de rbs lui-même.

**Contrainte connue** : Jinja utilise `{{ }}` et les `format!("{{}}")` de Rust aussi.
Des délimiteurs alternatifs sont configurés dès la première template, plutôt que
d'échapper au cas par cas.

### 4.4 Mécanisme d'écriture

Toute commande qui modifie un projet existant suit cette séquence, sans exception :

```
1. LIRE       état du projet + metadata.rbs
2. PLANIFIER  construire le plan complet en mémoire (créations, insertions, patchs toml)
3. VÉRIFIER   ancres présentes ? feature déjà installée ? working tree Git propre ?
4. AFFICHER   le plan complet à l'utilisateur
5. APPLIQUER  écriture atomique, avec restauration si une étape échoue
```

Quatre garde-fous en découlent :

- **Idempotence.** `rbs add auth` deux fois de suite ne produit rien la seconde fois.
  La vérification porte sur `metadata.rbs`, pas sur la présence des fichiers.
- **Ancre manquante = rien n'est écrit.** Le CLI affiche le bloc à coller et sort en
  erreur. Il ne devine jamais où insérer.
- **Working tree sale = avertissement**, contournable par `--force`. `rbs add` modifie
  des fichiers déjà édités par le développeur : `git checkout` doit toujours pouvoir
  annuler l'opération. C'est le filet de sécurité, plus fiable qu'un système de backup
  maison.
- **Tout ou rien.** Si l'écriture du quatrième fichier échoue, les trois premiers sont
  restaurés.

`Cargo.toml` est édité avec `toml_edit`, et non par appel à `cargo add` : cela préserve
le formatage et les commentaires du développeur, et permet d'ajouter proprement une
feature à une dépendance déjà présente.

### 4.5 Format des ancres

```rust
// src/features/mod.rs
pub mod health;
// <rbs:features>
pub mod users;
// </rbs:features>

// src/router.rs
// <rbs:routes>
.merge(features::users::routes())
// </rbs:routes>
```

L'insertion se fait juste avant la balise fermante. Le contenu existant à l'intérieur
de l'ancre n'est jamais réordonné ni reformaté.

### 4.6 Ergonomie des erreurs

Chaque cas d'échec identifié indique l'action corrective :

```
✗ Ancre <rbs:routes> introuvable dans src/router.rs

  Le fichier a probablement été restructuré. Ajoute ces deux lignes
  dans ta fonction router(), puis relance `rbs add auth` :

      // <rbs:routes>
      // </rbs:routes>
```

## 5. Le transverse

### 5.1 Erreurs

Un type unique dans `rbs-core`, construit avec `thiserror`, implémentant `IntoResponse` :

```rust
pub enum Error {
    NotFound(&'static str),
    Validation(ValidationErrors),
    Unauthorized,
    Forbidden,
    Conflict(String),
    Domain { status: StatusCode, code: &'static str, message: String },
    Database(#[from] DbErr),
    Internal(#[from] anyhow::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
```

La variante `Domain` permet à un projet d'exprimer ses erreurs métier sans forker
`rbs-core` ni empiler une hiérarchie d'erreurs supplémentaire. Toute la chaîne d'une
feature retourne `rbs::Result<T>`.

La réponse HTTP suit **RFC 9457** (`application/problem+json`).

**Règle non négociable** : `Database` et `Internal` ne fuient jamais vers le client.
Elles produisent un 500 générique portant le `request_id` ; la cause réelle est
journalisée sous ce même identifiant.

```json
{ "type": "about:blank", "title": "Validation failed", "status": 422,
  "errors": { "email": ["format invalide"] },
  "request_id": "01JQ3F8K2P" }
```

### 5.2 Logs

`tracing` + `tracing-subscriber`, deux formats commutés par `RBS_LOG_FORMAT` :

```
  pretty (dev)                          json (prod)
  14:32:07  INFO  request                {"ts":"...","level":"INFO",
    → POST /api/users  201  12.4ms        "msg":"request","method":"POST",
    request_id=01JQ3F8K2P                 "path":"/api/users","status":201,
                                          "latency_ms":12.4,"request_id":"01J…"}
```

Un middleware ouvre un span par requête et y attache le `request_id` (ULID, généré ou
repris de l'en-tête `x-request-id` entrant). Tout log émis pendant la requête le porte
automatiquement ; il est renvoyé au client dans `x-request-id`. `RUST_LOG` reste
respecté pour le filtrage fin.

Le formateur `pretty` par défaut de `tracing-subscriber` étant verbeux et pauvre en
couleur, `rbs-core` implémente son propre `FormatEvent`. C'est ce qui sépare des logs
qu'on lit de logs qu'on subit.

### 5.3 Configuration

`figment` fusionne, dans cet ordre :

```
valeurs par défaut → config/default.toml → config/{RBS_ENV}.toml → .env → variables d'environnement
```

Le résultat est désérialisé dans une struct `Config` typée et **validée au démarrage**.
Une variable manquante ou mal formée fait échouer le boot avec un message nommant le
champ fautif — jamais un `unwrap()` au premier appel HTTP six heures plus tard.

### 5.4 OpenAPI

`utoipa` + `utoipa-swagger-ui`. Les DTO dérivent `ToSchema`, les handlers portent
`#[utoipa::path(...)]`. **Ces annotations sont écrites par le CLI**, ce qui garantit
que la documentation existe dès la génération.

Une ancre `<rbs:openapi>` enregistre les paths de chaque feature. Swagger UI est monté
sur `/docs`, le document JSON sur `/api-docs/openapi.json`. Les deux sont désactivables
par configuration en production.

### 5.5 Tests

Trois niveaux :

1. **`rbs-core`** — tests unitaires, dont les conversions erreur → réponse HTTP.
2. **Projet généré** — `rbs generate crud` génère un fichier de tests d'intégration HTTP
   couvrant le CRUD complet contre l'application montée en mémoire. Un starter qui
   génère du code sans tests enseigne à ne pas en écrire.
3. **CLI en CI** — `assert_cmd` + `tempfile` : `rbs new`, puis `rbs g crud`, puis
   `rbs add`, sur un projet temporaire, suivis de `cargo build` et de l'exécution des
   tests du projet généré. C'est le seul test qui prouve que rbs fonctionne. Il est
   lent ; il tourne sur chaque PR.

Base de données de test : `testcontainers` avec PostgreSQL, plutôt qu'une base locale
à configurer qui divergerait d'une machine à l'autre.

### 5.6 Conventions de code

Exigences vérifiables, non déclaratives :

- **Un commentaire explique le *pourquoi*, jamais le *quoi*.** Un commentaire qui
  paraphrase la ligne suivante est un défaut. `// incrémente le compteur` au-dessus de
  `count += 1` est supprimé.
- **`#![warn(missing_docs)]` sur `rbs-core`** — les items publics portent un `///` d'une
  à trois lignes. C'est la surface publique ; elle est exemptée de la règle précédente.
- **Le code généré ne commente que ses points d'extension.** Pas de bandeau « généré par
  rbs, ne pas modifier » : ce code est fait pour être modifié.
- **`cargo clippy -- -D warnings` et `cargo fmt --check`** bloquants en CI, sur rbs comme
  sur le projet généré par le test d'intégration.
- **Signal de taille** : un fichier de feature dépassant ~200 lignes indique une feature
  à scinder. Indicatif, non bloquant.

## 6. Périmètre

### 6.1 v0.1

Inclus : `rbs new`, `rbs generate crud`, `rbs generate feature`, `rbs add docker|ci`,
`rbs migrate`, `rbs doctor`, PostgreSQL, erreurs, logs, configuration, OpenAPI,
documentation FR/EN, dépôt public, CI.

**Critère de sortie** : un tiers clone, installe, génère une API CRUD qui tourne, sans
poser de question. Tant qu'une explication de vive voix est nécessaire pour démarrer,
la v0.1 n'est pas terminée.

### 6.2 Hors périmètre

GraphQL, multi-tenancy, WebSockets, gRPC, interface d'administration générée, gestion
des paiements. Non pas « plus tard » : hors sujet. Un starter qui tente de tout couvrir
ne couvre rien proprement.

### 6.3 Licence

`MIT OR Apache-2.0`, double licence, conformément à la convention de l'écosystème Rust.
Le choix vaut pour les deux crates publiées et pour le code généré par le CLI : un projet
généré appartient à son auteur, sans obligation de licence héritée.

## 7. Risques identifiés

| Risque | Impact | Traitement |
|---|---|---|
| Conflit de délimiteurs minijinja / `format!` Rust | Templates cassées silencieusement | Délimiteurs alternatifs configurés dès la première template · test de rendu sur une template contenant `format!` |
| Ancre supprimée par le développeur | `rbs add` inopérant | Échec explicite avec bloc à coller · `rbs doctor` détecte les ancres manquantes |
| Divergence FR/EN de la documentation | Une langue devient obsolète | Revue de parité à chaque jalon · EN et FR modifiés dans le même commit |
| Test d'intégration CLI lent en CI | Contournement de la CI | Assumé : il tourne sur chaque PR, c'est le seul test qui prouve le produit |
| API de `rbs-core` figée trop tôt | Rupture pour les utilisateurs précoces | Pas de promesse de semver avant la v1.0, annoncé dans le README |
| Toolchain Node dans un dépôt Rust | Friction pour les contributeurs | `docs/` isolé, CI séparée, contribution au code possible sans Node |

## 8. Dépendances retenues

**`rbs-core`** : axum, tokio, sea-orm, tower, tower-http, tracing, tracing-subscriber,
serde, thiserror, anyhow, figment, validator, utoipa, utoipa-swagger-ui, ulid.

**`rbs-cli`** : clap (derive), minijinja, include_dir, toml_edit, inquire, console,
anyhow.

La vérification du working tree se fait par un appel à `git status --porcelain` en
sous-processus, et non via `git2` : une dépendance à libgit2 est disproportionnée pour
un unique appel, et `git` est nécessairement présent chez un utilisateur de rbs.

**Projet généré** : `uuid` (features `v7`, `serde`) et `sea-orm` avec `with-uuid`, pour la clé primaire décrite en §3.6.

**Tests** : assert_cmd, tempfile, testcontainers.
