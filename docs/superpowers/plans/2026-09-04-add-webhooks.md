# `rbs add webhooks` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** livrer le fragment `webhooks` — webhooks sortants signés HMAC-SHA256, table
d'abonnements, livraison par la file `jobs` existante.

**Architecture:** un répertoire `crates/rbs-cli/templates/features/webhooks/` déposé sur le
moule de `scheduler`, plus une treizième ancre `// <rbs:jobs>` dans le registre de la file,
sans laquelle aucun fragment ne peut inscrire un job.

**Tech Stack:** Rust 2024, Axum 0.8, SeaORM 2.0, minijinja (délimiteurs `{@ @}`),
`reqwest` 0.13, `hmac` 0.13, `sha2` 0.11.

**Spec:** `docs/superpowers/specs/2026-09-04-add-webhooks-design.md`

## Global Constraints

- **Branche** `feat/add-webhooks`. Ne jamais merger.
- **Commits** : Conventional Commits, sujet français à l'impératif, sans majuscule ni point
  final. **Jamais** de `Co-Authored-By`, de `Claude-Session`, d'identifiant de tâche ni de
  renvoi à un fichier de suivi. Corps portant le *pourquoi*, puis un intertitre
  `Vérifications :` avec les commandes lancées et leur résultat réel.
- **Commentaires** : le *pourquoi*, jamais le *quoi*. Un commentaire qui paraphrase la
  ligne suivante se supprime.
- **Un fichier de feature au-delà de ~200 lignes** signale une feature à scinder.
- **minijinja** utilise `{@ ... @}` et non `{{ }}`. `-%}` mange l'indentation.
- **Documentation bilingue dans le même commit** : `docs/docs/` et
  `docs/i18n/fr/docusaurus-plugin-content-docs/current/`.
- **Ne pas toucher `IMPROVE.md`.** Le mainteneur coche lui-même.
- Bloquants en CI : `cargo clippy --workspace --all-targets -- -D warnings` et
  `cargo fmt --all --check`.
- Sorties longues redirigées vers `/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad/webhooks-*.txt`.

**Noms figés, employés à l'identique par toutes les tâches :**

| Chose | Nom |
|---|---|
| Table | `webhook_subscriptions` |
| Migration | `create_webhook_subscriptions` |
| `KIND` du job | `webhooks::deliver` |
| En-tête de signature | `X-Rbs-Signature: t=<unix>,v1=<hex>` |
| En-têtes annexes | `X-Rbs-Event`, `X-Rbs-Delivery` |
| Routes | `POST`/`GET` `/webhooks/subscriptions`, `DELETE /webhooks/subscriptions/{id}` |
| Section de config | `[webhooks]`, clé `timeout_secs = 10` |
| Ancre nouvelle | `jobs` → `// <rbs:jobs>` dans `src/jobs/mod.rs` |

---

## Structure des fichiers

```
crates/rbs-cli/src/anchors.rs                       + const JOBS, ANCRES: [Anchor; 13]
crates/rbs-cli/templates/features/jobs/mod.rs.jinja + le bloc d'ancre dans registry()

crates/rbs-cli/templates/features/webhooks/
  feature.toml           le manifeste : requires, fichiers, ancres, migration, deps, config
  mod.rs.jinja           modules, routes(), matches(), réexports          (~70 l.)
  config.rs.jinja        section [webhooks]                                (~30 l.)
  model.rs.jinja         entité webhook_subscriptions                      (~45 l.)
  dto.rs.jinja           CreateSubscription, SubscriptionCreated, SubscriptionResponse (~60 l.)
  repository.rs.jinja    seul à parler à SeaORM                            (~70 l.)
  service.rs.jinja       emit(), create(), list(), revoke()                (~90 l.)
  controller.rs.jinja    trois handlers annotés utoipa                     (~90 l.)
  signature.rs.jinja     HMAC-SHA256, forme de l'en-tête                   (~60 l.)
  delivery.rs.jinja      Event, Delivery (le Job), Sender                  (~140 l.)
  migration.rs.jinja     la table                                          (~80 l.)
  tests.rs.jinja         les dix tests livrés                              (~200 l.)

crates/rbs-cli/src/lib.rs        + le conseil de fin d'installation
crates/rbs-cli/src/cli.rs        + l'énumération du `--help`
crates/rbs-cli/src/templates.rs  + `webhooks` dans la liste testée, + test du manifeste
crates/rbs-cli/templates/agents/{en,fr}.md.jinja  + l'énumération
crates/rbs-cli/tests/integration_webhooks.rs      nouveau

docs/docs/guides/webhooks.md          + sa traduction française
docs/docs/{compatibility,getting-started}.md      les comptes d'ancres
docs/docs/cli/{add,new,doctor,generate,completions}.md  les comptes
docs/docs/guides/{jobs,scheduler}.md  le renvoi vers webhooks, l'ancre cassée de scheduler
CHANGELOG.md, CHANGELOG.fr.md         section `## [Unreleased]`
```

**Chemin de vérification rapide** (à rejouer après chaque tâche qui touche une template) —
`cargo check` sur un projet réel prend des secondes, la passe Docker des minutes :

```bash
SCRATCH=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/9832dd0a-6053-4238-9bec-043bc12c253d/scratchpad
cargo build -p rbs-cli --bin rbs 2>&1 | tail -3
rm -rf "$SCRATCH/webhooks-demo" && mkdir -p "$SCRATCH/webhooks-demo"
cd "$SCRATCH/webhooks-demo" && /Users/yacoubakone/dev/rs/.claude/worktrees/agent-a6017bfed407c0cf2/target/debug/rbs \
  new demo-api --database postgres \
  --database-url "postgres://rbs:rbs@localhost:5432/demo" \
  --core-path /Users/yacoubakone/dev/rs/.claude/worktrees/agent-a6017bfed407c0cf2/crates/rbs-core --yes
cd demo-api && /Users/yacoubakone/dev/rs/.claude/worktrees/agent-a6017bfed407c0cf2/target/debug/rbs add webhooks
cargo check --workspace 2>&1 | tail -40
```

---

## Task 1 : la treizième ancre, `// <rbs:jobs>`

**Files:**
- Modify: `crates/rbs-cli/src/anchors.rs` (constante `JOBS`, `ANCRES`, tests)
- Modify: `crates/rbs-cli/templates/features/jobs/mod.rs.jinja` (`registry()`)

**Interfaces:**
- Consumes: rien.
- Produces: `crate::anchors::JOBS`, une `Anchor` de nom `jobs`, fichier
  `src/jobs/mod.rs`, `comment: "//"`, `sorted: false`, `optional: true`,
  `after: ".register::<demo::Log>()"`. `ANCRES` passe à `[Anchor; 13]`.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `crates/rbs-cli/src/anchors.rs`, module `tests`, **remplacer**
`only_the_services_anchor_is_optional` par sa version à deux ancres et ajouter le test de
la nouvelle :

```rust
    /// Une ancre optionnelle est l'exception : les onze autres décrivent un fichier que le
    /// squelette écrit toujours, et leur absence est un défaut. Les deux qui le sont vivent
    /// dans un fichier qu'un fragment dépose — le compose de `docker`, le registre de
    /// `jobs` — et manquent légitimement à qui n'a pas installé ce fragment.
    #[test]
    fn only_the_anchors_of_a_fragment_deposited_file_are_optional() {
        let optionnelles: Vec<&str> = ANCRES
            .iter()
            .filter(|anchor| anchor.optional)
            .map(|anchor| anchor.name.as_ref())
            .collect();

        assert_eq!(optionnelles, ["services", "jobs"]);
    }

    /// Sans elle, un fragment ne peut pas inscrire de job : le worker n'exécute que ce que
    /// `registry()` connaît, et `add` refuse tout nom d'ancre hors du registre.
    // `JOBS` étant un `const`, clippy évalue `.optional` à la compilation et signale
    // l'assertion comme triviale ; elle mord pourtant si quelqu'un change le champ.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn the_jobs_anchor_lives_in_the_queue_registry_and_is_optional() {
        assert_eq!(JOBS.file, "src/jobs/mod.rs");
        assert_eq!(JOBS.opening(), "// <rbs:jobs>");
        assert!(JOBS.optional);
        assert!(ANCRES.contains(&JOBS));
    }
```

Ajouter aussi, dans le même module de tests, le contrôle que la template porte l'ancre et
son accroche — l'ancre vit dans un fichier de fragment, que le balayage des templates du
squelette ne visite pas :

```rust
    /// L'accroche d'une ancre est vérifiée contre la template qui la porte, et celle-ci
    /// vit sous `features/` : le balayage du squelette ne la rencontre jamais.
    #[test]
    fn the_jobs_anchor_and_its_hook_are_in_the_queue_fragment() {
        let source = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/features/jobs/mod.rs.jinja"),
        )
        .expect("la template du fragment jobs doit se lire");

        assert!(source.contains(&JOBS.opening()), "{source}");
        assert!(source.contains(&JOBS.closing()), "{source}");
        assert_eq!(
            source.matches(JOBS.after).count(),
            1,
            "l'accroche doit paraître une fois exactement"
        );
        assert!(
            source.find(JOBS.after) < source.find(&JOBS.opening()),
            "l'ancre doit suivre son accroche"
        );
    }
```

- [ ] **Step 2 : lancer les tests, vérifier qu'ils échouent**

```bash
cargo test -p rbs-cli --lib anchors:: 2>&1 | tail -20
```
Attendu : erreur de compilation, `cannot find value JOBS in this scope`.

- [ ] **Step 3 : écrire la constante et l'étendre à `ANCRES`**

Dans `crates/rbs-cli/src/anchors.rs`, juste après `HEALTH_PROBES` :

```rust
/// Inscription d'un job au registre que le worker de la file consulte.
///
/// Seule ancre à vivre dans un fichier qu'un fragment dépose plutôt que le squelette, avec
/// celle du compose : `src/jobs/mod.rs` n'existe que sur un projet qui a installé la file.
/// C'est ce qui la rend optionnelle — un projet sans file n'a pas à passer pour incomplet.
///
/// Sans elle, `registry()` ne s'écrit qu'à la main : un fragment ne peut viser qu'une ancre
/// de ce registre, et le worker n'exécute que ce que `registry()` lui a déclaré.
pub(crate) const JOBS: Anchor = Anchor {
    name: Cow::Borrowed("jobs"),
    file: Cow::Borrowed("src/jobs/mod.rs"),
    comment: "//",
    sorted: false,
    optional: true,
    after: ".register::<demo::Log>()",
};
```

Puis `pub(crate) const ANCRES: [Anchor; 13] = [ …, HEALTH_PROBES, JOBS ];`

Dans `crates/rbs-cli/templates/features/jobs/mod.rs.jinja`, remplacer le corps de
`registry()` :

```rust
/// Les jobs de ce projet. Inscrivez les vôtres ici.
pub fn registry() -> Registry {
    Registry::new()
        .register::<demo::Log>()
    // <rbs:jobs>
    // </rbs:jobs>
}
```

- [ ] **Step 4 : lancer les tests, vérifier qu'ils passent**

```bash
cargo test -p rbs-cli --lib 2>&1 | tail -20
cargo clippy -p rbs-cli --all-targets -- -D warnings 2>&1 | tail -5
cargo fmt --all --check
```
Attendu : `test result: ok`, aucun avertissement.

Vérifier de plus que le rendu de la template reste du Rust que rustfmt accepte : le chemin
de vérification rapide en tête de plan, sans `rbs add webhooks` (qui n'existe pas encore) —
`rbs add jobs` suffit, puis `cargo fmt --check` dans le projet engendré.

- [ ] **Step 5 : commiter**

```bash
git add crates/rbs-cli/src/anchors.rs crates/rbs-cli/templates/features/jobs/mod.rs.jinja
git commit
```
Message : `feat(anchors): ouvre le registre de la file aux fragments`. Corps : le worker
n'exécute que ce que `registry()` connaît, et un fragment ne peut écrire que dans une ancre
du registre ; sans celle-ci, aucun fragment ne peut poser un job. Optionnelle comme celle
du compose, son fichier étant déposé par un fragment.

---

## Task 2 : le manifeste, la migration, le modèle et la configuration

**Files:**
- Create: `crates/rbs-cli/templates/features/webhooks/feature.toml`
- Create: `crates/rbs-cli/templates/features/webhooks/migration.rs.jinja`
- Create: `crates/rbs-cli/templates/features/webhooks/model.rs.jinja`
- Create: `crates/rbs-cli/templates/features/webhooks/config.rs.jinja`
- Modify: `crates/rbs-cli/src/templates.rs` (liste énumérée + test du manifeste)

**Interfaces:**
- Consumes: `crate::anchors::JOBS` (Task 1).
- Produces: l'entité `crate::webhooks::model::{Entity, Model, ActiveModel, Column}` sur
  `webhook_subscriptions`, champs `id: Uuid`, `url: String`, `events: Json`,
  `secret: String`, `revoked_at: Option<DateTimeWithTimeZone>`, `created_at`, `updated_at`.
  `crate::webhooks::Config { timeout_secs: u64 }` avec `Config::load()`.

- [ ] **Step 1 : écrire le test qui échoue**

Dans `crates/rbs-cli/src/templates.rs`, module de tests, ajouter `"webhooks"` à la liste
énumérée de `a_feature_without_a_fragment_is_refused_by_name` (ordre alphabétique : après
`"storage"`), et ajouter :

```rust
    /// Le fragment ne fait ni boucle ni horloge : il enfile dans la file et s'inscrit à son
    /// registre. Les deux ancres le disent, et `requires` en fait la condition.
    #[test]
    fn the_webhooks_fragment_requires_the_queue_and_registers_its_delivery_job() {
        let source = read(&Path::new(RACINE_FEATURES).join("webhooks/feature.toml"));
        let manifest = crate::manifest::read(&source, "webhooks/feature.toml")
            .expect("le manifeste du fragment webhooks doit se lire");

        assert!(manifest.feature.requires.contains(&"jobs".to_string()));
        assert!(manifest.feature.requires.contains(&"auth".to_string()));

        let ancres: Vec<&str> = manifest
            .anchors
            .iter()
            .map(|insertion| insertion.anchor.as_str())
            .collect();

        assert!(ancres.contains(&"jobs"), "{ancres:?}");
        assert!(ancres.contains(&"routes"), "{ancres:?}");
        assert!(ancres.contains(&"openapi"), "{ancres:?}");
    }
```

- [ ] **Step 2 : lancer, vérifier l'échec**

```bash
cargo test -p rbs-cli --lib templates:: 2>&1 | tail -20
```
Attendu : panique sur la lecture de `webhooks/feature.toml`, fichier absent.

- [ ] **Step 3 : écrire le manifeste et les trois templates**

`feature.toml` — le manifeste **n'est pas** une template minijinja, sauf le champ `content`
des ancres, qui l'est :

```toml
[feature]
description = "webhooks sortants : abonnements, signature HMAC horodatée, livraison par la file"
# `jobs` porte la livraison-avec-réessais : réservation sans double dépilage, `attempts`,
# `available_at`, `last_error`. Un second mécanisme de réessai n'aurait laissé que deux
# boucles à maintenir.
#
# `auth` protège les trois routes d'abonnement. Une création laissée ouverte permettrait à
# n'importe qui de faire livrer chez lui les événements du projet — `user.created` porte des
# adresses. C'est le même arbitrage qu'`auth`, qui exige `rate-limit` pour la même raison :
# une protection dont l'absence se paie en fuite de données n'est pas facultative.
requires = ["jobs", "auth"]

[[files]]
source      = "mod.rs.jinja"
destination = "src/webhooks/mod.rs"

[[files]]
source      = "config.rs.jinja"
destination = "src/webhooks/config.rs"

[[files]]
source      = "model.rs.jinja"
destination = "src/webhooks/model.rs"

[[files]]
source      = "dto.rs.jinja"
destination = "src/webhooks/dto.rs"

[[files]]
source      = "repository.rs.jinja"
destination = "src/webhooks/repository.rs"

[[files]]
source      = "service.rs.jinja"
destination = "src/webhooks/service.rs"

[[files]]
source      = "controller.rs.jinja"
destination = "src/webhooks/controller.rs"

[[files]]
source      = "signature.rs.jinja"
destination = "src/webhooks/signature.rs"

[[files]]
source      = "delivery.rs.jinja"
destination = "src/webhooks/delivery.rs"

[[files]]
source      = "tests.rs.jinja"
destination = "src/webhooks/tests.rs"

[[anchors]]
anchor  = "features"
content = "pub mod webhooks;"

[[anchors]]
anchor  = "routes"
content = ".merge(crate::webhooks::routes())"

[[anchors]]
anchor  = "openapi"
content = """
crate::webhooks::controller::subscribe,
crate::webhooks::controller::list,
crate::webhooks::controller::revoke,
"""

# Le worker n'exécute que ce que le registre connaît : sans cette ligne, chaque livraison
# partirait en réessai puis en échec sous « aucun job n'est inscrit ».
[[anchors]]
anchor  = "jobs"
content = ".register::<crate::webhooks::delivery::Delivery>()"

# Un client par processus, donc un pool de connexions partagé : le construire à chaque
# livraison rouvrirait une session TLS par tentative.
[[anchors]]
anchor  = "state_champs"
content = "pub webhooks: crate::webhooks::Sender,"

[[anchors]]
anchor  = "state_init"
content = "webhooks: crate::webhooks::Sender::from_config()?,"

[migration]
source = "migration.rs.jinja"
name   = "create_webhook_subscriptions"

# Les défauts de reqwest 0.13 sont rustls : rien à faire de plus pour éviter OpenSSL, à la
# différence de la 0.12 dont le défaut était native-tls.
[[dependencies]]
name    = "reqwest"
version = "0.13"

[[dependencies]]
name    = "hmac"
version = "0.13"

[[dependencies]]
name    = "sha2"
version = "0.11"

[[dependencies]]
name    = "async-trait"
version = "0.1"

# `events` est un tableau JSON, que sea-orm ne sait lire que sous cette feature.
[cargo.sea-orm]
features = ["with-json"]

# `Identity` garde les trois routes et `token::random` tire le secret d'un abonnement : les
# deux vivent derrière cette feature du noyau. `auth` l'active déjà — la redéclarer dit que
# webhooks en dépend pour son propre compte.
[cargo.rbs-core]
features = ["auth"]

# `sync` : les tests livrés se relaient sur la table des abonnements et sur la file, et leur
# verrou traverse un `await`.
[cargo.tokio]
features = ["sync"]

[[config]]
file    = "config/default.toml"
section = "webhooks"
content = """
# Temps laissé au receveur pour répondre. Au-delà, la livraison compte pour un échec et
# repart en réessai : un endpoint lent ne doit pas retenir un worker.
timeout_secs = 10
"""
```

`migration.rs.jinja` — sur le moule de `jobs/migration.rs.jinja` :

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WebhookSubscriptions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WebhookSubscriptions::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WebhookSubscriptions::Url)
                            .string()
                            .not_null(),
                    )
                    // Les motifs écoutés, tableau de chaînes. Le tri se fait en Rust : la
                    // recherche dans un tableau JSON n'a pas de forme commune aux trois
                    // moteurs, et les motifs à préfixe l'auraient de toute façon interdite.
                    .col(
                        ColumnDef::new(WebhookSubscriptions::Events)
                            .json()
                            .not_null(),
                    )
                    // En clair, et il ne peut pas ne pas l'être : la livraison le relit pour
                    // signer, là où un mot de passe n'a jamais à être relu. La table est
                    // donc aussi sensible que les données qu'elle protège.
                    .col(
                        ColumnDef::new(WebhookSubscriptions::Secret)
                            .string()
                            .not_null(),
                    )
                    // Une date plutôt qu'un booléen : elle porte le booléen et le moment.
                    // La ligne survit à sa révocation, et une livraison déjà en file y
                    // trouve encore de quoi savoir qu'elle n'a plus lieu d'être.
                    .col(
                        ColumnDef::new(WebhookSubscriptions::RevokedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(WebhookSubscriptions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(WebhookSubscriptions::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await

        // Aucun index de plus, et ce n'est pas un oubli : l'émission lit tous les
        // abonnements non révoqués pour les trier elle-même, et un index sur une colonne
        // nulle dans presque toutes les lignes n'épargnerait pas ce parcours. La table
        // compte les abonnés du projet, non les événements émis.
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(WebhookSubscriptions::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum WebhookSubscriptions {
    Table,
    Id,
    Url,
    Events,
    Secret,
    RevokedAt,
    CreatedAt,
    UpdatedAt,
}
```

`model.rs.jinja` — sur le moule de `jobs/model.rs.jinja`, avec les deux ancres de relation
qu'y pose la convention :

```rust
use sea_orm::ActiveValue::Set;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "webhook_subscriptions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Où POSTer la livraison.
    pub url: String,
    /// Les motifs écoutés : `*`, `user.*` ou `user.created`.
    pub events: Json,
    /// Le secret de signature, propre à cet abonnement et rendu une seule fois.
    pub secret: String,
    /// La révocation, datée. Nulle tant que l'abonnement sert.
    pub revoked_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // <rbs:relations:webhook_subscriptions>
    // </rbs:relations:webhook_subscriptions>
}

// <rbs:related:webhook_subscriptions>
// </rbs:related:webhook_subscriptions>

/// L'identifiant est posé ici, et non par un défaut de colonne : `uuidv7()` n'a
/// d'équivalent à écrire ni en MySQL ni en SQLite.
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::now_v7()),
            ..ActiveModelTrait::default()
        }
    }
}
```

`config.rs.jinja` — calqué sur `scheduler/config.rs.jinja` :

```rust
use serde::Deserialize;

/// Section `[webhooks]` de la configuration du projet.
///
/// Le défaut est porté ici plutôt que par le noyau : il est lisible et modifiable à
/// l'endroit même où la section est déclarée. `config/{env}.toml` et la variable
/// `RBS_WEBHOOKS__TIMEOUT_SECS` le surchargent comme pour toute autre section.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Temps laissé au receveur pour répondre, en secondes.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Config {
    /// Relit la cascade de configuration pour la seule section `[webhooks]`.
    pub fn load() -> Result<Self, rbs_core::config::ConfigError> {
        rbs_core::config::section("webhooks")
    }
}

fn default_timeout() -> u64 {
    10
}
```

- [ ] **Step 4 : lancer les tests, vérifier qu'ils passent**

```bash
cargo test -p rbs-cli --lib 2>&1 | tail -10
```
Attendu : `test result: ok`. Le fragment n'est pas encore installable — les sept autres
templates manquent — c'est la tâche suivante.

- [ ] **Step 5 : commiter**

```bash
git add crates/rbs-cli/templates/features/webhooks crates/rbs-cli/src/templates.rs
git commit
```
Message : `feat(webhooks): pose la table des abonnements et son manifeste`.

---

## Task 3 : la signature

**Files:**
- Create: `crates/rbs-cli/templates/features/webhooks/signature.rs.jinja`

**Interfaces:**
- Consumes: rien.
- Produces:
  - `pub fn sign(secret: &str, timestamp: i64, body: &[u8]) -> String` — le `v1` seul, en
    hexadécimal minuscule ;
  - `pub fn header(secret: &str, timestamp: i64, body: &[u8]) -> String` — la valeur
    complète de l'en-tête, `t=<timestamp>,v1=<hex>` ;
  - `pub const HEADER: &str = "x-rbs-signature";`
  - `pub const HEADER_EVENT: &str = "x-rbs-event";`
  - `pub const HEADER_DELIVERY: &str = "x-rbs-delivery";`

Le corps signé est `"<timestamp>.<corps>"`. Les tests de cette fonction vivent dans
`tests.rs.jinja` (Task 7) : ce fichier-ci n'en porte aucun, comme les autres templates du
projet, dont les tests sont regroupés.

- [ ] **Step 1 : écrire la template**

```rust
// `KeyInit` porte `new_from_slice` depuis hmac 0.13 : la 0.12 le réexportait par `Mac`, et
// l'omettre ne se voit qu'à la compilation, sur une erreur qui ne nomme pas la version.
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// L'en-tête qui porte la signature.
pub const HEADER: &str = "x-rbs-signature";

/// L'en-tête qui nomme l'événement, lisible sans ouvrir le corps.
pub const HEADER_EVENT: &str = "x-rbs-event";

/// L'en-tête qui identifie l'événement, stable d'un réessai à l'autre.
///
/// C'est par lui que le receveur déduplique : la file peut livrer deux fois — une réponse
/// perdue après traitement — et sans cet identifiant, rien ne le lui dirait.
pub const HEADER_DELIVERY: &str = "x-rbs-delivery";

/// Signe un corps daté, et rend le condensat en hexadécimal minuscule.
///
/// **L'horodatage entre dans la signature**, et c'est ce qui ferme le rejeu : un tiers qui
/// capte une livraison ne peut pas la resservir plus tard sous une date fraîche sans
/// invalider le condensat.
pub fn sign(secret: &str, timestamp: i64, body: &[u8]) -> String {
    // `new_from_slice` n'échoue que sur une longueur de clé impossible, et HMAC en accepte
    // toutes : le `expect` est ici la façon de dire qu'il n'y a pas de cas d'échec.
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepte une clé de n'importe quelle longueur");

    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);

    mac.finalize()
        .into_bytes()
        .iter()
        .map(|octet| format!("{octet:02x}"))
        .collect()
}

/// La valeur de l'en-tête `X-Rbs-Signature`, prête à être posée.
///
/// `v1=` nomme le schéma plutôt que de laisser le condensat nu : le jour où un second
/// arrive, les deux cohabitent dans le même en-tête et un receveur à jour choisit.
pub fn header(secret: &str, timestamp: i64, body: &[u8]) -> String {
    format!("t={timestamp},v1={}", sign(secret, timestamp, body))
}
```

- [ ] **Step 2 : déclarer le fichier au manifeste**

Il l'est déjà (Task 2). Vérifier :

```bash
grep -c "signature.rs.jinja" crates/rbs-cli/templates/features/webhooks/feature.toml
```
Attendu : `1`.

- [ ] **Step 3 : commiter**

```bash
git add crates/rbs-cli/templates/features/webhooks/signature.rs.jinja
git commit
```
Message : `feat(webhooks): signe les livraisons en HMAC-SHA256 horodaté`.

---

## Task 4 : le modèle de données du projet — dto, repository, service

**Files:**
- Create: `crates/rbs-cli/templates/features/webhooks/dto.rs.jinja`
- Create: `crates/rbs-cli/templates/features/webhooks/repository.rs.jinja`
- Create: `crates/rbs-cli/templates/features/webhooks/service.rs.jinja`

**Interfaces:**
- Consumes: `super::model` (Task 2), `super::delivery::{Delivery, Event}` (Task 5),
  `super::matches` (Task 6).
- Produces:
  - `dto::CreateSubscription { url: String, events: Vec<String> }` — `Deserialize`,
    `ToSchema`, `Validate` ;
  - `dto::SubscriptionCreated { id, url, events, secret, created_at }` — `Serialize`,
    `ToSchema` ;
  - `dto::SubscriptionResponse { id, url, events, revoked_at, created_at }` ;
  - `repository::{actifs, find, create, revoke}` ;
  - `service::{emit, subscribe, list, revoke}`, dont
    `pub async fn emit<C, T>(db: &C, event: &str, data: &T) -> anyhow::Result<usize>`
    avec `C: ConnectionTrait`, `T: Serialize`.

- [ ] **Step 1 : écrire `dto.rs.jinja`**

```rust
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateSubscription {
    /// L'URL du receveur. `https` en production : la signature authentifie l'émetteur,
    /// elle ne chiffre pas ce qui est livré.
    #[validate(url)]
    pub url: String,
    /// Les motifs écoutés. `*` pour tout, `user.*` pour une famille, `user.created` pour un
    /// événement précis.
    #[validate(length(min = 1))]
    pub events: Vec<String>,
}

/// Ce que rend la création, et elle seule.
///
/// `secret` n'y paraît qu'ici : une seule lecture de la liste livrerait sinon les secrets de
/// tous les abonnés d'un coup.
#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionCreated {
    pub id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub secret: String,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubscriptionResponse {
    pub id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub revoked_at: Option<DateTimeWithTimeZone>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeWithTimeZone,
}
```

- [ ] **Step 2 : écrire `repository.rs.jinja`**

Seule couche qui construit une requête SeaORM. Générique sur `ConnectionTrait` là où
l'appelant peut passer une transaction — c'est ce qui rend l'émission transactionnelle.

```rust
use rbs_core::{Error, Result};
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use super::model::{ActiveModel, Column, Entity};

// Le service passe par cette porte plutôt que par `model.rs` : la couche qui parle à la
// base reste la seule à connaître l'entité.
pub use super::model::Model;

/// Les abonnements que rien n'a révoqués.
///
/// `ConnectionTrait` et non une connexion : l'émission se fait dans la transaction du
/// métier, et la liste doit être lue avec la même vue qu'elle.
pub async fn actifs<C: ConnectionTrait>(db: &C) -> Result<Vec<Model>> {
    Ok(Entity::find()
        .filter(Column::RevokedAt.is_null())
        .order_by_asc(Column::CreatedAt)
        .all(db)
        .await?)
}

pub async fn find<C: ConnectionTrait>(db: &C, id: Uuid) -> Result<Option<Model>> {
    Ok(Entity::find_by_id(id).one(db).await?)
}

/// Inscrit un abonnement, les horodatages venant des défauts de la table.
pub async fn create<C: ConnectionTrait>(
    db: &C,
    url: &str,
    events: &[String],
    secret: &str,
) -> Result<Model> {
    let nouveau = ActiveModel {
        url: Set(url.to_owned()),
        events: Set(serde_json::json!(events)),
        secret: Set(secret.to_owned()),
        revoked_at: Set(None),
        ..Default::default()
    };

    Ok(nouveau.insert(db).await?)
}

/// Date la révocation d'un abonnement, ou dit qu'il n'existe pas.
///
/// Révoquer deux fois ne repousse pas la date : la première révocation est celle qui compte,
/// et un client qui rejoue son appel ne doit pas réécrire l'histoire.
pub async fn revoke<C: ConnectionTrait>(
    db: &C,
    id: Uuid,
    quand: DateTimeWithTimeZone,
) -> Result<Model> {
    let abonnement = find(db, id)
        .await?
        .ok_or(Error::NotFound("abonnement"))?;

    if abonnement.revoked_at.is_some() {
        return Ok(abonnement);
    }

    let mut ligne: ActiveModel = abonnement.into();
    ligne.revoked_at = Set(Some(quand));
    ligne.updated_at = Set(quand);

    Ok(ligne.update(db).await?)
}
```

- [ ] **Step 3 : écrire `service.rs.jinja`**

```rust
use chrono::Utc;
use rbs_core::{Error, Result};
use sea_orm::prelude::Uuid;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde::Serialize;

use super::delivery::{Delivery, Event};
use super::dto::{CreateSubscription, SubscriptionCreated, SubscriptionResponse};
use super::{matches, repository};
use crate::jobs;

/// Émet un événement : un job de livraison par abonné qui l'écoute.
///
/// `db` est un `ConnectionTrait` et non une connexion, et c'est délibéré : passez-lui la
/// transaction du métier, et les livraisons naissent si et seulement si elle est committée.
/// Un `user.created` livré alors que l'inscription a été annulée serait un mensonge que
/// rien ne rattrape.
///
/// Rend le nombre de livraisons enfilées. Aucun abonné concerné n'est pas une erreur : un
/// projet sans webhook configuré émet dans le vide, ce qui est le cas nominal.
pub async fn emit<C, T>(db: &C, event: &str, data: &T) -> anyhow::Result<usize>
where
    C: ConnectionTrait,
    T: Serialize + ?Sized,
{
    // Une seule enveloppe pour tous les abonnés : son `id` est ce que le receveur voit, et
    // deux abonnés du même événement doivent en lire le même.
    let enveloppe = Event {
        id: Uuid::now_v7(),
        event: event.to_string(),
        created_at: Utc::now().fixed_offset(),
        data: serde_json::to_value(data)?,
    };

    let mut enfilees = 0;

    for abonnement in repository::actifs(db).await? {
        let motifs: Vec<String> = serde_json::from_value(abonnement.events.clone())?;

        if !matches(&motifs, event) {
            continue;
        }

        jobs::enqueue(
            db,
            &Delivery {
                subscription: abonnement.id,
                event: enveloppe.clone(),
            },
        )
        .await?;
        enfilees += 1;
    }

    Ok(enfilees)
}

/// Inscrit un abonnement et tire son secret.
///
/// Le secret est propre à l'abonnement, et non au projet : un secret commun donnerait à
/// chaque abonné de quoi contrefaire les événements livrés à tous les autres.
pub async fn subscribe(
    db: &DatabaseConnection,
    input: CreateSubscription,
) -> Result<SubscriptionCreated> {
    for motif in &input.events {
        if motif.trim().is_empty() {
            return Err(Error::BadRequest(
                "un motif d'événement ne peut pas être vide".to_string(),
            ));
        }
    }

    let secret = rbs_core::token::random();
    let inscrit = repository::create(db, &input.url, &input.events, &secret).await?;

    Ok(SubscriptionCreated {
        id: inscrit.id,
        url: inscrit.url,
        events: input.events,
        // Rendu cette seule fois : la liste ne le porte pas, et la base est le seul autre
        // endroit où il vit.
        secret,
        created_at: inscrit.created_at,
    })
}

pub async fn list(db: &DatabaseConnection) -> Result<Vec<SubscriptionResponse>> {
    repository::actifs(db)
        .await?
        .into_iter()
        .map(reponse)
        .collect()
}

pub async fn revoke(db: &DatabaseConnection, id: Uuid) -> Result<()> {
    repository::revoke(db, id, Utc::now().fixed_offset()).await?;

    Ok(())
}

/// Le rendu d'un abonnement, secret exclu.
fn reponse(abonnement: repository::Model) -> Result<SubscriptionResponse> {
    Ok(SubscriptionResponse {
        id: abonnement.id,
        url: abonnement.url,
        events: serde_json::from_value(abonnement.events)
            .map_err(|source| Error::Internal(source.into()))?,
        revoked_at: abonnement.revoked_at,
        created_at: abonnement.created_at,
    })
}
```

- [ ] **Step 4 : vérifier le rendu des trois templates**

Elles ne compilent pas encore seules — `delivery` et `matches` arrivent aux tâches 5 et 6.
Se contenter du rendu minijinja :

```bash
cargo test -p rbs-cli --lib templates:: 2>&1 | tail -10
```
Attendu : `test result: ok` — `each_template_renders_with_the_context_of_a_creation` couvre
le squelette ; pour les fragments, la preuve vient du `cargo check` de la Task 6.

- [ ] **Step 5 : commiter**

```bash
git add crates/rbs-cli/templates/features/webhooks
git commit
```
Message : `feat(webhooks): enfile une livraison par abonné dans la transaction du métier`.

---

## Task 5 : la livraison — l'enveloppe, le job, le client

**Files:**
- Create: `crates/rbs-cli/templates/features/webhooks/delivery.rs.jinja`

**Interfaces:**
- Consumes: `super::{repository, signature, Config}`, `crate::jobs::Job`,
  `crate::state::AppState`.
- Produces:
  - `pub struct Event { id: Uuid, event: String, created_at: DateTimeWithTimeZone, data: serde_json::Value }`,
    `Clone + Serialize + Deserialize` ;
  - `pub struct Delivery { subscription: Uuid, event: Event }`, `Serialize + Deserialize`,
    `impl Job` avec `const KIND: &str = "webhooks::deliver"` ;
  - `pub struct Sender(reqwest::Client)`, `Clone`, avec
    `pub fn from_config() -> anyhow::Result<Self>` et
    `pub(super) async fn post(&self, url, headers…) -> anyhow::Result<()>`.

- [ ] **Step 1 : écrire la template**

```rust
use std::time::Duration;

use rbs_core::HasCoreState;
use sea_orm::prelude::{DateTimeWithTimeZone, Uuid};
use serde::{Deserialize, Serialize};

use super::{Config, repository, signature};
use crate::jobs::Job;
use crate::state::AppState;

/// Ce que le receveur lit.
///
/// `id` est tiré à l'émission et voyage dans la charge utile du job : il est donc le même à
/// chaque réessai, ce qui est toute sa raison d'être — c'est la clé de déduplication du
/// receveur.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event: String,
    pub created_at: DateTimeWithTimeZone,
    pub data: serde_json::Value,
}

/// Une livraison à un abonné, telle qu'elle attend dans la file.
///
/// L'abonnement est désigné par son identifiant et non recopié : l'URL et le secret sont
/// relus au dépilage, si bien qu'un secret tourné s'applique aux livraisons déjà en file et
/// qu'une révocation les arrête.
#[derive(Debug, Serialize, Deserialize)]
pub struct Delivery {
    pub subscription: Uuid,
    pub event: Event,
}

#[async_trait::async_trait]
impl Job for Delivery {
    const KIND: &'static str = "webhooks::deliver";

    async fn run(&self, state: &AppState) -> anyhow::Result<()> {
        let db = state.core().db();

        let Some(abonnement) = repository::find(db, self.subscription).await? else {
            // L'abonnement a disparu de la table : il n'y a rien à livrer et rien à
            // réessayer. Un `Err` ferait cinq tentatives sur une ligne qui n'existe plus.
            tracing::info!(
                subscription = %self.subscription,
                event = %self.event.event,
                "livraison sans abonnement : abandonnée"
            );
            return Ok(());
        };

        // Relue au dépilage et non à l'émission : un abonnement révoqué entre les deux ne
        // reçoit rien, et le job se termine en succès — il n'y a rien à réessayer.
        if abonnement.revoked_at.is_some() {
            tracing::info!(
                subscription = %abonnement.id,
                event = %self.event.event,
                "abonnement révoqué : livraison abandonnée"
            );
            return Ok(());
        }

        // Sérialisé une fois, signé et envoyé : ces octets-là et pas d'autres. Sérialiser
        // deux fois exposerait à ce que l'ordre des clés change entre la signature et
        // l'envoi, et le receveur rejetterait une signature pourtant honnête.
        let corps = serde_json::to_vec(&self.event)?;
        let horodatage = chrono::Utc::now().timestamp();

        state
            .webhooks()
            .post(
                &abonnement.url,
                &signature::header(&abonnement.secret, horodatage, &corps),
                &self.event.event,
                self.event.id,
                corps,
            )
            .await
    }
}

/// Le client HTTP des livraisons, partagé par le processus.
///
/// Un `reqwest::Client` porte son pool de connexions : le construire à chaque livraison
/// rouvrirait une session TLS par tentative. Il vit donc dans l'`AppState`, où le job le
/// retrouve.
#[derive(Clone, Debug)]
pub struct Sender {
    client: reqwest::Client,
}

impl Sender {
    /// Construit le client d'après la section `[webhooks]`.
    ///
    /// L'échec remonte au démarrage plutôt qu'à la première livraison : un délai
    /// d'expiration illisible est une faute de configuration, et la découvrir six heures
    /// plus tard dans un journal de worker ne sert personne.
    pub fn from_config() -> anyhow::Result<Self> {
        let config = Config::load()?;

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(config.timeout_secs))
                .build()?,
        })
    }

    /// POSTe un corps signé, et fait d'un refus une erreur — donc un réessai de la file.
    ///
    /// Toute réponse hors 2xx vaut échec, 4xx comprises : un receveur qui répond 400 à une
    /// livraison bien formée est en panne, et le distinguer d'un 503 demanderait de deviner
    /// laquelle des deux parties a tort.
    pub(super) async fn post(
        &self,
        url: &str,
        signature: &str,
        event: &str,
        delivery: Uuid,
        body: Vec<u8>,
    ) -> anyhow::Result<()> {
        let reponse = self
            .client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(signature::HEADER, signature)
            .header(signature::HEADER_EVENT, event)
            .header(signature::HEADER_DELIVERY, delivery.to_string())
            .body(body)
            .send()
            .await?;

        let statut = reponse.status();

        if statut.is_success() {
            tracing::debug!(%url, %event, %delivery, status = statut.as_u16(), "livraison acceptée");
            return Ok(());
        }

        anyhow::bail!("{url} a répondu {statut}");
    }
}
```

- [ ] **Step 2 : commiter**

```bash
git add crates/rbs-cli/templates/features/webhooks/delivery.rs.jinja
git commit
```
Message : `feat(webhooks): livre par la file, en relisant l'abonnement au dépilage`.

---

## Task 6 : `mod.rs`, le contrôleur, et la première compilation réelle

**Files:**
- Create: `crates/rbs-cli/templates/features/webhooks/mod.rs.jinja`
- Create: `crates/rbs-cli/templates/features/webhooks/controller.rs.jinja`

**Interfaces:**
- Produces:
  - `pub fn matches(patterns: &[String], event: &str) -> bool` ;
  - `pub fn routes() -> Router<AppState>` ;
  - `pub use delivery::Sender;`, `pub use service::emit;`, `pub use config::Config;` ;
  - `impl AppState { pub fn webhooks(&self) -> &Sender }` ;
  - `controller::{subscribe, list, revoke}`.

- [ ] **Step 1 : écrire `mod.rs.jinja`**

```rust
pub mod config;
pub mod controller;
pub mod delivery;
pub mod dto;
pub mod model;
pub mod repository;
pub mod service;
pub mod signature;

#[cfg(test)]
mod tests;

use axum::Router;
use axum::routing::{delete, get, post};

pub use config::Config;
pub use delivery::Sender;
// Réexportée pour que le projet écrive `webhooks::emit(&transaction, "user.created", &dto)`
// : tant qu'aucun handler ne le fait, le compilateur la tient pour inutile.
#[allow(unused_imports)]
pub use service::emit;

use crate::state::AppState;

/// L'accès au client de livraison depuis l'état partagé.
///
/// L'accesseur arrive avec la feature et repart avec elle : `state.rs` n'a pas à connaître
/// un type qu'un `rbs add` a déposé.
impl AppState {
    pub fn webhooks(&self) -> &Sender {
        &self.webhooks
    }
}

// Les deux méthodes du même chemin se déclarent en une fois : axum refuse — et le dit par
// une panique au démarrage — deux `route()` sur un chemin identique.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/webhooks/subscriptions",
            post(controller::subscribe).get(controller::list),
        )
        .route("/webhooks/subscriptions/{id}", delete(controller::revoke))
}

/// Dit si l'un des motifs d'un abonnement vaut pour `event`.
///
/// Trois formes, et pas une de plus : `*` pour tout, `user.*` pour une famille,
/// `user.created` pour lui-même.
///
/// Le tri se fait ici et non en SQL, et c'est délibéré : la recherche dans un tableau JSON
/// n'a pas de forme commune à PostgreSQL, MySQL et SQLite, et les motifs à préfixe
/// l'auraient de toute façon interdite. Le prix est une lecture de la table des abonnements
/// par événement émis — sans conséquence tant que les abonnés se comptent en centaines.
pub fn matches(patterns: &[String], event: &str) -> bool {
    patterns.iter().any(|motif| {
        if motif == "*" {
            return true;
        }

        match motif.strip_suffix('*') {
            Some(prefixe) => event.starts_with(prefixe),
            None => motif == event,
        }
    })
}
```

- [ ] **Step 2 : écrire `controller.rs.jinja`**

```rust
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rbs_core::{HasCoreState, Identity, ProblemDetails, Result, ValidatedJson};
use sea_orm::prelude::Uuid;

use super::dto::{CreateSubscription, SubscriptionCreated, SubscriptionResponse};
use super::service;
use crate::state::AppState;

// Les trois routes exigent un appelant authentifié. Une création laissée ouverte
// permettrait à n'importe qui de faire livrer chez lui les événements du projet : ce n'est
// pas une faiblesse théorique, `user.created` porte des adresses.
//
// `Identity` ne dit que « le jeton est valide ». Un projet qui réserve l'administration des
// abonnements à un rôle remplace l'extracteur par sa propre garde — `src/auth/guard.rs`.

#[utoipa::path(
    post,
    path = "/webhooks/subscriptions",
    tag = "webhooks",
    security(("bearer" = [])),
    request_body = CreateSubscription,
    responses(
        (status = 201, description = "abonnement inscrit, secret rendu cette seule fois", body = SubscriptionCreated),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 422, description = "entrée invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn subscribe(
    State(state): State<AppState>,
    _identite: Identity,
    ValidatedJson(input): ValidatedJson<CreateSubscription>,
) -> Result<(StatusCode, Json<SubscriptionCreated>)> {
    let inscrit = service::subscribe(state.core().db(), input).await?;

    Ok((StatusCode::CREATED, Json(inscrit)))
}

#[utoipa::path(
    get,
    path = "/webhooks/subscriptions",
    tag = "webhooks",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "les abonnements actifs, sans leurs secrets", body = Vec<SubscriptionResponse>),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    _identite: Identity,
) -> Result<Json<Vec<SubscriptionResponse>>> {
    Ok(Json(service::list(state.core().db()).await?))
}

#[utoipa::path(
    delete,
    path = "/webhooks/subscriptions/{id}",
    tag = "webhooks",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "identifiant de l'abonnement")),
    responses(
        (status = 204, description = "abonnement révoqué"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 404, description = "abonnement inconnu", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn revoke(
    State(state): State<AppState>,
    _identite: Identity,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    service::revoke(state.core().db(), id).await?;

    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 3 : compiler pour de vrai**

C'est la première fois que le fragment est compilable. Lancer le chemin de vérification
rapide en tête de plan, avec `rbs add webhooks`, et rediriger :

```bash
… | tee "$SCRATCH/webhooks-check.txt" | tail -40
```
Attendu : `cargo check --workspace` du projet engendré aboutit. Corriger jusque-là — toute
erreur relevée ici appartient à cette tâche, y compris dans les templates des tâches 2 à 5.

Vérifier aussi que le projet reste formaté et que l'ancre du registre a bien reçu sa ligne :

```bash
cd "$SCRATCH/webhooks-demo/demo-api" && cargo fmt --all --check && grep -A2 "<rbs:jobs>" src/jobs/mod.rs
```
Attendu : aucune sortie de `fmt`, et `.register::<crate::webhooks::delivery::Delivery>()`
entre les deux balises.

- [ ] **Step 4 : commiter**

```bash
git add crates/rbs-cli/templates/features/webhooks
git commit
```
Message : `feat(webhooks): monte les trois routes d'abonnement derrière l'authentification`.

---

## Task 7 : les tests livrés au projet

**Files:**
- Create: `crates/rbs-cli/templates/features/webhooks/tests.rs.jinja`

**Interfaces:**
- Consumes: tout le fragment. Le verrou de base est celui de `jobs` —
  `crate::jobs::tests::verrou_base()` — pour la raison qu'énonce `scheduler/tests.rs.jinja` :
  deux verrous indépendants laisseraient chaque suite effacer ce que l'autre vient d'écrire.
- Produces: les onze noms de tests que la Task 8 exigera nommément.

**Les six sans base :**

| Nom | Ce qu'il prouve |
|---|---|
| `the_signature_matches_an_independently_computed_vector` | le vecteur ci-dessous |
| `the_signature_changes_with_the_timestamp` | l'horodatage entre dans le condensat, donc le rejeu est fermé |
| `the_signature_header_carries_the_timestamp_and_the_v1_digest` | la forme `t=…,v1=…`, hexadécimal minuscule, 64 caractères |
| `an_exact_pattern_matches_only_its_own_event` | |
| `a_prefix_pattern_matches_every_event_of_its_family` | `user.*` prend `user.created`, pas `order.created` |
| `the_star_pattern_matches_every_event` | |

Le vecteur est calculé hors de Rust, et c'est ce qui lui donne sa valeur : un test qui
compare `sign` à `sign` ne prouve que la stabilité, jamais la justesse. Un receveur écrit
en Python ou en Go doit retrouver ce condensat-là.

```
$ printf '1757000000.{}' | openssl dgst -sha256 -hmac "secret" -hex
SHA2-256(stdin)= a5dba1becd39e5b001811ec483ca620bdfc84f2167017c06f8a0fc0ca2953b75
```

donc `sign("secret", 1_757_000_000, b"{}")` vaut
`"a5dba1becd39e5b001811ec483ca620bdfc84f2167017c06f8a0fc0ca2953b75"`.

**Les cinq qui joignent la base**, tous `#[ignore = "joint la base du projet"]` :

| Nom | Ce qu'il prouve |
|---|---|
| `emitting_an_event_enqueues_one_delivery_per_listening_subscription` | |
| `a_revoked_subscription_is_not_delivered_to` | |
| `a_subscription_that_does_not_listen_receives_nothing` | |
| `a_delivery_whose_subscription_was_revoked_succeeds_without_a_request` | pas d'HTTP dans les tests |
| `an_emission_rolled_back_with_its_transaction_enqueues_nothing` | l'argument d'`emit(&transaction, …)` |

- [ ] **Step 1 : écrire la template**

Sur le moule de `scheduler/tests.rs.jinja`. Points imposés :

- un helper `async fn table_a_soi() -> (MutexGuard<'static, ()>, AppState)` qui prend
  `crate::jobs::tests::verrou_base()`, vide `webhook_subscriptions` sans filtre et la file
  **seulement de ses lignes** : `Column::Kind.eq(Delivery::KIND)` ;
- un helper `async fn abonne(db, url, events: &[&str]) -> Model` qui insère un abonnement
  actif, et `async fn abonne_revoque(...)` qui pose `revoked_at` ;
- `livraisons(db)` compte les lignes de la file dont `kind == Delivery::KIND` ;
- le test transactionnel ouvre `db.begin()`, appelle `emit(&transaction, …)`, puis
  `transaction.rollback()`, et vérifie que la file est vide ;
- le test de la livraison révoquée construit un `Delivery` à la main et appelle
  `job.run(&state)`, dont il attend `Ok(())` — l'abonnement étant révoqué, aucune requête
  HTTP n'est émise et le test n'a besoin d'aucun serveur.

Un commentaire de tête doit dire, comme `scheduler/tests.rs.jinja`, pourquoi les tests avec
base sont `#[ignore]` et quel verrou ils partagent.

- [ ] **Step 2 : lancer les tests du projet engendré**

Régénérer le projet de démonstration (chemin de vérification rapide), puis :

```bash
cd "$SCRATCH/webhooks-demo/demo-api" && cargo test --workspace 2>&1 | tee "$SCRATCH/webhooks-projet.txt" | tail -25
```
Attendu : les cinq tests sans base passent ; les cinq autres sont `ignored`. Les cinq avec
base demandent un PostgreSQL migré — c'est `integration_webhooks` qui les joue (Task 8).

- [ ] **Step 3 : commiter**

```bash
git add crates/rbs-cli/templates/features/webhooks/tests.rs.jinja
git commit
```
Message : `test(webhooks): livre au projet la preuve de sa signature et de son émission`.

---

## Task 8 : le test d'intégration

**Files:**
- Create: `crates/rbs-cli/tests/integration_webhooks.rs`

**Interfaces:**
- Consumes: `mod common` (`start_postgres`, `url_of`, `cible`, `verrou`, `commiter`,
  `noyau`, `depot`).
- Produces: rien.

- [ ] **Step 1 : écrire le fichier**

Copie fidèle de la structure d'`integration_scheduler.rs`, avec :

- `TESTS_ORDINAIRES: [&str; 6]` et `TESTS_SOUS_CONTENEUR: [&str; 5]`, les onze noms de la
  Task 7, chacun exigé sous la forme
  `"test webhooks::tests::{test} ... ok"` ;
- `the_tests_shipped_with_the_fragment_run_against_a_real_database` : `rbs new`,
  `rbs add webhooks`, `rbs migrate up`, puis les deux flux `cargo test` et
  `cargo test -- --ignored` ;
- dans `project_with_webhooks_on`, l'assertion que les fragments requis ont été entraînés :
  `src/jobs/mod.rs`, `src/auth/mod.rs` **et** `src/rate-limit`… — la feature s'appelle
  `rate-limit` mais son module est `src/rate_limit/mod.rs` : vérifier le nom réel sur le
  projet engendré avant d'écrire l'assertion ;
- l'assertion que l'ancre du registre a reçu la livraison :

```rust
    let registre = fs::read_to_string(racine.join("src/jobs/mod.rs")).expect("registre lisible");
    assert!(
        registre.contains(".register::<crate::webhooks::delivery::Delivery>()"),
        "le job de livraison n'est pas inscrit au registre :\n{registre}"
    );
```

- `every_file_the_fragment_ships_is_declared_in_its_manifest`, copié tel quel avec
  `webhooks` à la place de `scheduler` — il ne demande ni Docker ni compilation.

**Pas de test sur les trois moteurs** : un commentaire de tête doit le dire et pourquoi —
ce que webhooks ajoute au schéma est une colonne JSON et une date nullable, or
`integration_jobs::the_dequeue_never_hands_the_same_job_twice_on_the_three_engines` prouve
déjà la colonne JSON de la file sur les trois.

- [ ] **Step 2 : lancer le test rapide**

```bash
cargo test -p rbs-cli --test integration_webhooks 2>&1 | tail -10
```
Attendu : `every_file_the_fragment_ships_is_declared_in_its_manifest` passe, l'autre est
`ignored`.

- [ ] **Step 3 : lancer la passe lente**

Docker doit tourner. `--no-fail-fast` est **obligatoire** : sans lui la suite s'arrête au
premier binaire et masque les échecs suivants.

```bash
cargo test -p rbs-cli --test integration_webhooks --no-fail-fast -- --ignored \
  2>&1 | tee "$SCRATCH/webhooks-integration.txt" | tail -40
```
Attendu : `test result: ok. 2 passed`.

- [ ] **Step 4 : commiter**

```bash
git add crates/rbs-cli/tests/integration_webhooks.rs
git commit
```
Message : `test(webhooks): joue le fragment contre une base réelle`.

---

## Task 9 : le câblage du CLI

**Files:**
- Modify: `crates/rbs-cli/src/lib.rs` (fonction `suite`, et son test)
- Modify: `crates/rbs-cli/src/cli.rs:56`
- Modify: `crates/rbs-cli/templates/agents/en.md.jinja:22`
- Modify: `crates/rbs-cli/templates/agents/fr.md.jinja:22`

- [ ] **Step 1 : écrire le test qui échoue**

Dans `crates/rbs-cli/src/lib.rs`, module de tests, à côté de
`the_scheduler_fragment_advises_the_migration_and_the_declaration_site` :

```rust
    /// Trois tables à créer — le fragment entraîne `jobs` et `auth` — et une émission qui
    /// n'existe que si le code du projet l'appelle : installé et non appelé, le fragment ne
    /// livre rien.
    #[test]
    fn the_webhooks_fragment_advises_the_migration_and_the_emission_site() {
        let conseil = suite("webhooks").expect("le fragment pose une table : il doit conseiller");

        assert!(conseil.contains("rbs migrate up"), "{conseil}");
        assert!(conseil.contains("webhooks::emit"), "{conseil}");
    }
```

- [ ] **Step 2 : lancer, vérifier l'échec**

```bash
cargo test -p rbs-cli --lib the_webhooks_fragment_advises 2>&1 | tail -10
```
Attendu : `panicked at ... le fragment pose une table`.

- [ ] **Step 3 : écrire le conseil et les énumérations**

Dans `suite`, après la branche `"scheduler"` :

```rust
        // Trois tables — le fragment entraîne `jobs` et `auth` — et une émission qui n'a
        // lieu que si le code du projet l'appelle : installé et jamais appelé, le fragment
        // n'a aucun effet observable, et rien ne le dirait.
        "webhooks" => Some(
            "rbs migrate up, puis appelez webhooks::emit(&transaction, \"user.created\", &dto) \
             là où votre code écrit — les abonnements se créent par POST /webhooks/subscriptions",
        ),
```

Dans `cli.rs:56` et les deux templates `agents`, insérer `webhooks` en fin de liste
alphabétique (après `storage`).

- [ ] **Step 4 : lancer les tests**

```bash
cargo test -p rbs-cli --lib 2>&1 | tail -10
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5
cargo fmt --all --check
```
Attendu : `ok`, aucun avertissement.

- [ ] **Step 5 : commiter**

```bash
git add crates/rbs-cli/src/lib.rs crates/rbs-cli/src/cli.rs crates/rbs-cli/templates/agents
git commit
```
Message : `feat(webhooks): énumère le fragment et conseille son point d'appel`.

---

## Task 10 : la documentation, en deux langues

**Files:**
- Create: `docs/docs/guides/webhooks.md`
- Create: `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/webhooks.md`
- Modify (EN + FR, chaque paire dans le même commit) :
  `docs/docs/cli/add.md` · `new.md` · `doctor.md` · `generate.md` · `completions.md` ·
  `docs/docs/compatibility.md` · `getting-started.md` ·
  `docs/docs/guides/jobs.md` · `guides/scheduler.md`
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md`

- [ ] **Step 1 : relever tous les comptes à corriger**

```bash
grep -rn "douze\|twelve\|onze\|eleven\|treize\|thirteen" docs/docs docs/i18n | grep -v node_modules \
  | tee "$SCRATCH/webhooks-comptes.txt" | wc -l
```
Chaque ligne est à décider une par une : un compte de **features** passe de douze à treize,
un compte d'**ancres** passe de douze à treize aussi, et le sous-compte « onze en Rust »
devient « douze en Rust ». Le compte de `doctor` sur un projet sans compose reste inférieur
d'une unité au total.

Corriger au passage le renvoi cassé de `docs/docs/guides/scheduler.md`, qui pointe
`#the-eleven-features` quand le titre d'`add.md` dit « The twelve features » — l'ancre
devient `#the-thirteen-features` des deux côtés, et son pendant français
`#les-treize-features`.

- [ ] **Step 2 : écrire le guide**

`docs/docs/guides/webhooks.md`, `sidebar_position: 11.6` (juste après `scheduler`, à 11.5),
`title: Webhooks`. Sur le moule du guide `scheduler` : ce qu'installe `rbs add webhooks`,
la sortie réelle du plan (la copier depuis une exécution, pas l'inventer), la table, les
trois routes avec un exemple `curl`, la fonction `emit`, la forme de la signature **avec un
exemple de vérification côté receveur**, et un paragraphe franc sur ce qui n'est pas là :
pas de webhooks entrants, pas de rotation de secret, pas de désactivation automatique d'un
endpoint mort. Puis la traduction française, page pour page.

Le guide doit dire les deux choses qu'un lecteur ne devinera pas :

1. `rbs add webhooks` pose quatre fragments sur un projet nu, et pourquoi ;
2. le secret est stocké en clair et ne peut pas ne pas l'être.

- [ ] **Step 3 : ajouter la ligne au tableau des features**

Dans `docs/docs/cli/add.md` et son pendant français, ajouter `webhooks` au tableau des
features avec sa description et ses fichiers, sur le format des douze autres.

Ajouter dans `docs/docs/guides/jobs.md` (et sa traduction) un paragraphe court : le registre
porte désormais une ancre, et c'est par elle qu'un fragment y inscrit un job — `webhooks`
est le premier à s'en servir.

- [ ] **Step 4 : les deux CHANGELOG**

Sous `## [Unreleased]`, sur le ton et le niveau de détail des entrées `audit` et
`scheduler` déjà présentes.

- [ ] **Step 5 : vérifier la parité et la construction**

```bash
node docs/scripts/parite.mjs 2>&1 | tail -20
cd docs && npm run build 2>&1 | tee "$SCRATCH/webhooks-docs.txt" | tail -20
```
Attendu : parité sans écart, construction sans lien mort. (Si la chaîne Node n'est pas
installée, le dire franchement plutôt que d'affirmer le contraire.)

- [ ] **Step 6 : commiter**

```bash
git add docs CHANGELOG.md CHANGELOG.fr.md
git commit
```
Message : `docs(webhooks): documente les sortants signés et le compte des ancres`.

---

## Task 11 : la vérification finale

- [ ] **Step 1 : la suite rapide**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5
cargo test --workspace 2>&1 | tee "$SCRATCH/webhooks-tests.txt" | grep "test result:"
```
Relever les chiffres exacts, passés et échoués, pour chaque binaire.

- [ ] **Step 2 : la passe lente, ciblée**

```bash
cargo test -p rbs-cli --no-fail-fast --test integration_webhooks --test integration_jobs \
  --test integration_add --test integration_doctor --test integration_examples \
  -- --ignored 2>&1 | tee "$SCRATCH/webhooks-lents.txt" | grep -E "test result:|FAILED"
```
`integration_examples` compare les quatre projets d'`examples/` octet à octet : la treizième
ancre change `src/jobs/mod.rs`, et l'exemple `newsletter-queue` porte `jobs`. **Il faut donc
régénérer cet exemple** — par diff entre deux générations, jamais par écrasement :
`examples/README.md` donne la commande exacte. C'est une correction du lot, non une nouvelle
tâche.

- [ ] **Step 3 : appliquer le skill `superpowers:verification-before-completion`**

Ne rien affirmer qui n'ait été lu dans une sortie de commande. Tout critère non prouvé est
dit non prouvé, avec sa raison.
