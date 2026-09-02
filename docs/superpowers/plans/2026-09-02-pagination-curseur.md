# Pagination par curseur dans `rbs-core` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ajouter à `rbs-core` un extracteur `Cursor` et une page `CursorPage<T>` qui paginent sur l'`id` UUIDv7 sans jamais compter les lignes, sans toucher au code que le CLI engendre.

**Architecture:** Deux types dans le module `pagination` existant, frères de `Pagination` et `Page<T>` dont ils reprennent les constantes de bornage et le traitement asymétrique des paramètres illisibles. Purement additif : aucune signature existante ne change, aucune template n'est touchée, aucun projet engendré ne voit de différence.

**Tech Stack:** Rust 2024, axum (`FromRequestParts`), serde, utoipa (`ToSchema`), `sea_orm::prelude::Uuid`, tokio-test.

**Spec:** `docs/superpowers/specs/2026-09-02-pagination-curseur-design.md`

## Global Constraints

- `#![warn(missing_docs)]` est posé sur `rbs-core` : **tout item public porte un `///`** d'une à trois lignes. Un item sans doc fait échouer la compilation en CI (`-D warnings`).
- Un commentaire explique le *pourquoi*, jamais le *quoi*. Un commentaire qui paraphrase la ligne suivante se supprime. Seuls les `///` de `missing_docs` échappent à cette règle.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont bloquants en CI.
- `rbs-core` est en `1.2.0`, **non publiée** (crates.io s'arrête à `1.1.0`). L'ajout est additif : ni montée de version ni note de migration. `cargo semver-checks -p rbs-core --all-features` doit passer.
- Les trois constantes du module sont partagées, jamais redéclarées : `PAGE_PAR_DEFAUT = 1`, `PAR_PAGE_PAR_DEFAUT = 20`, `PAR_PAGE_MAX = 100`.
- Documentation bilingue : toute page modifiée sous `docs/` en anglais l'est aussi en français, **dans le même commit**. `docs/scripts/parite.mjs` le contrôle.
- Le `CHANGELOG.md` et le `CHANGELOG.fr.md` vont par paire, eux aussi dans le même commit.
- Commits en Conventional Commits, sujet français à l'impératif, sans identifiant de tâche, sans renvoi à un fichier de suivi, **sans ligne `Co-Authored-By` ni mention d'un assistant**. Corps portant le *pourquoi* et un intertitre `Vérifications :`.
- Travailler sur la branche `improve/p3-features-lot-un` (déjà créée) ou une branche dédiée ; jamais sur `main`.

## File Structure

| Fichier | Responsabilité | Action |
|---|---|---|
| `crates/rbs-core/Cargo.toml` | active `sea-orm/with-uuid` pour le seul `rbs-core` | Modifier |
| `crates/rbs-core/src/pagination.rs` | `Cursor`, `CursorPage<T>`, `CursorMeta`, leurs tests | Modifier |
| `crates/rbs-core/src/lib.rs:63` | ré-export `pub use pagination::{…}` | Modifier |
| `CHANGELOG.md` / `CHANGELOG.fr.md` | note `[Unreleased] / Added` | Modifier |
| `docs/docs/guides/filtering.md` + paire fr | section sur le curseur | Modifier |

Tout tient dans `pagination.rs`, qui passe d'environ 220 à environ 400 lignes — le module reste une seule responsabilité, la pagination, et le scinder séparerait deux types qui partagent leurs constantes et leur doctrine de bornage.

---

### Task 1: `Cursor`, l'extracteur

**Files:**
- Modify: `crates/rbs-core/Cargo.toml:64` (ligne `sea-orm.workspace = true`)
- Modify: `crates/rbs-core/src/pagination.rs` (après `impl FromRequestParts for Pagination`, ligne 92)
- Test: `crates/rbs-core/src/pagination.rs`, dans le `mod tests` existant (ligne 127)

**Interfaces:**
- Consomme : `PAR_PAGE_PAR_DEFAUT`, `PAR_PAGE_MAX` (`pagination.rs:19-22`), `crate::Error` (`pagination.rs:13`), le helper de test `query(&str) -> (StatusCode, Value)` (`pagination.rs:139`).
- Produit : `pub struct Cursor` avec `pub fn after(&self) -> Option<Uuid>` et `pub fn per_page(&self) -> u64`, plus `impl<S> FromRequestParts<S> for Cursor`. La Task 2 s'en sert.

**Pourquoi `sea-orm/with-uuid` et non la crate `uuid` :** `rbs-core` n'a aujourd'hui aucune dépendance UUID — la tâche 73 avait retiré `with-uuid` du workspace au motif exact que la crate n'en faisait aucun usage. Le curseur en fait un. Réactiver la feature sur le seul `rbs-core` donne le **même type** que celui des entités engendrées (sea-orm ré-exporte `uuid`), là où une dépendance directe ouvrirait un écart de version entre les deux `Uuid` du graphe.

- [ ] **Step 1: Activer la feature sea-orm**

Dans `crates/rbs-core/Cargo.toml`, remplacer la ligne `sea-orm.workspace = true` par :

```toml
# `Cursor` pagine sur l'`id`, un UUIDv7 : le type vient de sea-orm plutôt que d'une
# dépendance directe à `uuid`, pour qu'il soit celui-là même que portent les entités
# engendrées, sans écart de version possible entre les deux.
sea-orm = { workspace = true, features = ["with-uuid"] }
```

- [ ] **Step 2: Écrire les tests qui échouent**

Dans `crates/rbs-core/src/pagination.rs`, à la fin du `mod tests`, ajouter d'abord un second helper à côté de `query` :

```rust
    /// Interroge `/curseur` avec la chaîne de requête donnée et rend `(statut, body JSON)`.
    async fn curseur(query: &str) -> (StatusCode, Value) {
        async fn handler(cursor: Cursor) -> axum::Json<Value> {
            axum::Json(json!({
                "after": cursor.after().map(|id| id.to_string()),
                "per_page": cursor.per_page(),
            }))
        }

        let response = Router::new()
            .route("/curseur", get(handler))
            .oneshot(
                Request::builder()
                    .uri(format!("/curseur?{query}"))
                    .body(Body::empty())
                    .expect("requête valide"),
            )
            .await
            .expect("le router doit répondre");

        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");

        (status, serde_json::from_slice(&bytes).expect("corps JSON"))
    }

    #[tokio::test]
    async fn the_first_page_needs_no_cursor() {
        let (status, body) = curseur("").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["after"], Value::Null, "sans `after`, on part du début");
        assert_eq!(body["per_page"], PAR_PAGE_PAR_DEFAUT);
    }

    #[tokio::test]
    async fn a_readable_cursor_is_carried_through() {
        let id = "01926b3e-0000-7000-8000-000000000000";
        let (status, body) = curseur(&format!("after={id}")).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["after"], id);
    }

    #[tokio::test]
    async fn an_unreadable_cursor_answers_400() {
        let (status, body) = curseur("after=pas-un-uuid").await;

        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "un curseur illisible se signale, il ne s'ignore pas : {body}"
        );
        assert_eq!(body["status"], 400);
    }

    #[tokio::test]
    async fn the_cursor_shares_the_page_size_bounds() {
        let (status, body) = curseur("per_page=5000").await;

        assert_eq!(status, StatusCode::OK, "le plafonnement est muet : {body}");
        assert_eq!(body["per_page"], PAR_PAGE_MAX);

        let (_, body) = curseur("per_page=0").await;
        assert_eq!(body["per_page"], 1, "une page vide n'aurait aucun sens");
    }
```

- [ ] **Step 3: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-core pagination::tests -- --nocapture`
Expected: FAIL à la **compilation**, `cannot find type 'Cursor' in this scope`.

- [ ] **Step 4: Écrire `Cursor`**

Dans `crates/rbs-core/src/pagination.rs`, ajouter après la ligne 92 (fin de l'`impl FromRequestParts for Pagination`) :

```rust
/// Fenêtre de pagination par curseur, déjà bornée.
///
/// Là où [`Pagination`] saute `offset` lignes pour atteindre une page, le curseur reprend
/// la marche à l'`id` où elle s'était arrêtée : le moteur ne parcourt plus les lignes
/// qu'il va jeter, et une insertion survenue entre deux requêtes ne décale plus la
/// fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cursor {
    after: Option<Uuid>,
    per_page: u64,
}

/// Ce que le client a écrit dans la chaîne de requête, avant bornage.
#[derive(Debug, Deserialize)]
struct ParamsCurseur {
    after: Option<Uuid>,
    per_page: Option<u64>,
}

impl Cursor {
    /// Construit une fenêtre en ramenant `per_page` dans ses bornes.
    pub fn new(after: Option<Uuid>, per_page: u64) -> Self {
        Self {
            after,
            per_page: per_page.clamp(1, PAR_PAGE_MAX),
        }
    }

    /// Identifiant après lequel reprendre, `None` pour la première page.
    ///
    /// La borne est **exclusive** : le repository écrit `Column::Id.lt(after)`, sans quoi
    /// chaque page réafficherait la dernière ligne de la précédente.
    pub fn after(&self) -> Option<Uuid> {
        self.after
    }

    /// Nombre d'éléments par page.
    pub fn per_page(&self) -> u64 {
        self.per_page
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new(None, PAR_PAGE_PAR_DEFAUT)
    }
}

impl<S> FromRequestParts<S> for Cursor
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Même asymétrie que `Pagination`, et pour la même raison : une taille de page
        // hors bornes est ramenée en silence, un curseur illisible est signalé. Repartir
        // du début sur un `after` cassé ferait boucler un client sur la première page
        // sans que rien ne le lui dise.
        let Query(parametres) = Query::<ParamsCurseur>::from_request_parts(parts, state)
            .await
            .map_err(|rejet| Error::BadRequest(rejet.body_text()))?;

        Ok(Self::new(
            parametres.after,
            parametres.per_page.unwrap_or(PAR_PAGE_PAR_DEFAUT),
        ))
    }
}
```

Et compléter le bloc `use` en tête de fichier (`pagination.rs:8-13`) :

```rust
use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use sea_orm::prelude::Uuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::Error;
```

- [ ] **Step 5: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-core pagination::tests`
Expected: PASS, 9 tests (les 5 existants plus les 4 nouveaux).

- [ ] **Step 6: Ré-exporter et vérifier le lint**

Dans `crates/rbs-core/src/lib.rs:63`, remplacer :

```rust
pub use pagination::{Page, Pagination};
```

par :

```rust
pub use pagination::{Cursor, Page, Pagination};
```

Run: `cargo clippy -p rbs-core --all-targets -- -D warnings && cargo fmt --all --check`
Expected: aucune sortie.

- [ ] **Step 7: Commit**

```bash
git add crates/rbs-core/Cargo.toml crates/rbs-core/src/pagination.rs crates/rbs-core/src/lib.rs
git commit -F - <<'EOF'
feat(core): ajoute un extracteur de pagination par curseur

`Pagination` fait sauter au moteur les lignes qu'il va jeter, et une insertion
survenue entre deux requêtes décale la fenêtre : la page 2 réaffiche une ligne
déjà vue, ou en saute une. `Cursor` reprend la marche à l'`id` où elle s'était
arrêtée, ce que l'UUIDv7 rend possible sans tri supplémentaire.

Le type de l'identifiant vient de `sea-orm/with-uuid` et non d'une dépendance
directe à `uuid` : c'est ainsi le même `Uuid` que portent les entités
engendrées, sans écart de version possible dans le graphe. La feature avait été
retirée du workspace faute d'usage ; elle en a un désormais, sur cette seule
crate.

Vérifications :
- cargo test -p rbs-core pagination::tests : 9 passés, 0 échec
- cargo clippy -p rbs-core --all-targets -- -D warnings : aucune sortie
- cargo fmt --all --check : aucune sortie
EOF
```

---

### Task 2: `CursorPage<T>`, la page rendue

**Files:**
- Modify: `crates/rbs-core/src/pagination.rs` (après l'`impl<T> Page<T>`, ligne ~124)
- Modify: `crates/rbs-core/src/lib.rs:63`
- Test: `crates/rbs-core/src/pagination.rs`, `mod tests`

**Interfaces:**
- Consomme : `Cursor` de la Task 1, notamment `per_page()`.
- Produit : `pub struct CursorPage<T>` et `pub fn new(data: Vec<T>, cursor: &Cursor, dernier: Option<Uuid>) -> Self`. Rien d'ultérieur n'en dépend dans ce plan.

**La règle que `new` porte, et qui justifie son existence :** `next` vaut `None` dès que la page rendue est **plus courte** que `per_page` — c'est la fin de la marche, lisible sans compter la table. Écrite dans chaque repository, cette règle serait fausse une fois sur deux.

Le troisième paramètre est l'`id` du dernier élément, que `CursorPage` ne peut pas déduire : `T` est un DTO quelconque, dont la crate ignore s'il porte un `id`.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans le `mod tests` de `crates/rbs-core/src/pagination.rs` :

```rust
    /// Un identifiant lisible, dont seule l'unicité compte ici.
    fn identifiant(n: u8) -> Uuid {
        Uuid::from_bytes([n; 16])
    }

    #[test]
    fn a_full_page_names_its_successor() {
        let cursor = Cursor::new(None, 3);
        let dernier = identifiant(3);

        let page = CursorPage::new(vec!["a", "b", "c"], &cursor, Some(dernier));
        let rendu = serde_json::to_value(&page).expect("la page se sérialise");

        assert_eq!(rendu["meta"]["next"], dernier.to_string());
        assert_eq!(rendu["meta"]["per_page"], 3);
        assert_eq!(rendu["data"], json!(["a", "b", "c"]));
    }

    #[test]
    fn a_short_page_ends_the_walk() {
        let cursor = Cursor::new(None, 3);

        let page = CursorPage::new(vec!["a", "b"], &cursor, Some(identifiant(2)));
        let rendu = serde_json::to_value(&page).expect("la page se sérialise");

        assert_eq!(
            rendu["meta"]["next"],
            Value::Null,
            "une page plus courte que demandée est la dernière : {rendu}"
        );
    }

    #[test]
    fn an_empty_page_ends_the_walk() {
        let cursor = Cursor::new(Some(identifiant(9)), 3);

        let page = CursorPage::<&str>::new(Vec::new(), &cursor, None);
        let rendu = serde_json::to_value(&page).expect("la page se sérialise");

        assert_eq!(rendu["meta"]["next"], Value::Null);
        assert_eq!(rendu["data"], json!([]));
    }

    #[test]
    fn the_cursor_page_never_counts_the_rows() {
        let cursor = Cursor::new(None, 2);
        let page = CursorPage::new(vec!["a", "b"], &cursor, Some(identifiant(2)));
        let rendu = serde_json::to_value(&page).expect("la page se sérialise");

        let meta = rendu["meta"].as_object().expect("meta est un objet");
        assert!(
            !meta.contains_key("total") && !meta.contains_key("total_pages"),
            "le curseur existe pour ne pas payer le COUNT(*) : {rendu}"
        );
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-core pagination::tests`
Expected: FAIL à la compilation, `cannot find type 'CursorPage' in this scope`.

- [ ] **Step 3: Écrire `CursorPage`**

Après l'`impl<T> Page<T>` de `crates/rbs-core/src/pagination.rs` :

```rust
/// Une page rendue par curseur, et de quoi demander la suivante.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[non_exhaustive]
pub struct CursorPage<T> {
    data: Vec<T>,
    meta: CursorMeta,
}

/// Description d'une page rendue par curseur.
///
/// Ni `total` ni `total_pages` : le `COUNT(*)` qu'ils exigeraient est précisément ce que
/// le curseur existe pour ne pas payer.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
struct CursorMeta {
    per_page: u64,
    next: Option<Uuid>,
}

impl<T> CursorPage<T> {
    /// Enveloppe `data`, `dernier` étant l'`id` du dernier élément rendu.
    ///
    /// `next` s'éteint dès que la page est plus courte que demandée : c'est la fin de la
    /// marche, et elle se lit sans compter la table. Le dernier `id` est passé plutôt que
    /// déduit — `T` est un DTO quelconque, dont cette crate ignore s'il porte un `id`.
    pub fn new(data: Vec<T>, cursor: &Cursor, dernier: Option<Uuid>) -> Self {
        let complete = data.len() as u64 == cursor.per_page();

        Self {
            meta: CursorMeta {
                per_page: cursor.per_page(),
                next: dernier.filter(|_| complete),
            },
            data,
        }
    }
}
```

- [ ] **Step 4: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-core pagination::tests`
Expected: PASS, 13 tests.

- [ ] **Step 5: Ré-exporter, lint et contrôle semver**

Dans `crates/rbs-core/src/lib.rs:63` :

```rust
pub use pagination::{Cursor, CursorPage, Page, Pagination};
```

Run:
```bash
cargo test -p rbs-core
cargo clippy -p rbs-core --all-targets -- -D warnings
cargo fmt --all --check
cargo semver-checks -p rbs-core --all-features
```
Expected: tests verts ; clippy et fmt sans sortie ; semver-checks sans rupture signalée — l'ajout est purement additif.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-core/src/pagination.rs crates/rbs-core/src/lib.rs
git commit -F - <<'EOF'
feat(core): rend une page de curseur qui ne compte jamais ses lignes

`CursorPage` porte `per_page` et le curseur suivant, et rien d'autre : `total`
et `total_pages` relanceraient le balayage complet que la pagination par
curseur vient d'éviter.

`next` s'éteint quand la page rendue est plus courte que celle demandée. La
règle vit dans le constructeur plutôt que dans chaque repository appelant, où
elle serait réécrite avec une chance sur deux d'être fausse. Le dernier
identifiant est passé et non déduit : le type paginé est un DTO quelconque,
dont le noyau ignore s'il porte un `id`.

Vérifications :
- cargo test -p rbs-core : 13 tests de pagination, 0 échec
- cargo semver-checks -p rbs-core --all-features : aucune rupture, ajout additif
- cargo clippy -p rbs-core --all-targets -- -D warnings : aucune sortie
- cargo fmt --all --check : aucune sortie
EOF
```

---

### Task 3: Documentation bilingue et CHANGELOG

**Files:**
- Modify: `CHANGELOG.md` (section `[Unreleased] / Added`)
- Modify: `CHANGELOG.fr.md` (même section)
- Modify: `docs/docs/guides/filtering.md` (après la section « Pagination stays in the query string », ligne 53)
- Modify: `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/filtering.md` (section correspondante)

**Interfaces:**
- Consomme : `Cursor` et `CursorPage<T>` des Tasks 1 et 2.
- Produit : rien de logiciel.

**Ce qui rend cette tâche non facultative :** la documentation du projet ne cite aucune ligne écrite à la main, et `docs/scripts/parite.mjs` échoue si une page anglaise change sans sa paire française. Une doc absente n'est pas un oubli rattrapable plus tard — c'est un contrôle rouge.

- [ ] **Step 1: Lire la section à prolonger**

Run: `sed -n '48,62p' docs/docs/guides/filtering.md`
Objectif : reprendre le ton et le niveau de titre exacts. La section existante s'intitule `## Pagination stays in the query string`.

- [ ] **Step 2: Écrire la section anglaise**

Ajouter après la section « Pagination stays in the query string » de `docs/docs/guides/filtering.md` :

```markdown
## Cursor pagination, for lists that outgrow an offset

`Pagination` asks the engine to walk past the rows it is about to discard, and an insert
between two requests shifts the window — page 2 repeats a row page 1 already showed. Past
a few thousand rows, `Cursor` replaces it:

```
GET /articles?after=0199e0b1-9c4a-7c3e-9d21-6f2a1b0c4d5e&per_page=50
```

`after` is the `id` of the last row you were served, and it is exclusive. Leave it out for
the first page. A malformed `after` answers 400; a `per_page` beyond 100 is quietly capped,
exactly as it is for `Pagination`.

The response drops the counts:

```json
{
  "data": [ … ],
  "meta": { "per_page": 50, "next": "0199e0b1-…" }
}
```

`next` is null once the walk is over. There is no `total`: the `COUNT(*)` it needs is the
cost the cursor exists to avoid.

The generated CRUD keeps `Pagination` — switching it would drop `total` from every
response your clients already read. `Cursor` is there for the routes you write yourself:

```rust
let mut query = Entity::find().order_by_desc(Column::Id);
if let Some(after) = cursor.after() {
    query = query.filter(Column::Id.lt(after));
}
let rows = query.limit(cursor.per_page()).all(db).await?;
let dernier = rows.last().map(|row| row.id);

Ok(Json(CursorPage::new(
    rows.into_iter().map(Into::into).collect(),
    &cursor,
    dernier,
)))
```

The cursor only walks `id` descending — the order `list` already applies, and the one a
UUIDv7 makes total. It does not follow a `sort` you chose: on a column where two rows share
a value, the boundary would be ambiguous and the next page would skip or repeat rows.
```

- [ ] **Step 3: Écrire la section française**

Ajouter la section correspondante dans `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/filtering.md`, au même endroit relatif et au même niveau de titre (`##`), avec le même bloc de code Rust et les mêmes exemples JSON. Titre : `## Pagination par curseur, pour les listes qui débordent un offset`.

- [ ] **Step 4: Écrire les deux notes de CHANGELOG**

Dans `CHANGELOG.md`, sous `## [Unreleased]` → `### Added`, en tête de liste :

```markdown
- `rbs_core::Cursor` and `CursorPage<T>` paginate on the `id` instead of an offset, for
  lists where `OFFSET n` makes the engine walk the rows it is about to discard. `after` is
  exclusive and the response carries no `total` — the `COUNT(*)` it would need is the cost
  the cursor avoids. The generated CRUD is unchanged and keeps `Pagination`: switching it
  would drop `total` from every response already being served.
```

Dans `CHANGELOG.fr.md`, la note française correspondante, au même endroit.

- [ ] **Step 5: Vérifier la parité**

Run: `cd docs && node scripts/parite.mjs`
Expected: exit 0, aucune paire signalée.

Si le script signale un écart de niveau de titre ou de langue de bloc, le corriger avant de continuer — c'est exactement ce qu'il est là pour attraper.

- [ ] **Step 6: Commit**

```bash
git add CHANGELOG.md CHANGELOG.fr.md docs/docs/guides/filtering.md docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/filtering.md
git commit -F - <<'EOF'
docs(pagination): documente le curseur et ce qu'il refuse de faire

La page dit les deux choses qu'un lecteur doit savoir avant de basculer : que
la réponse perd `total`, et que le curseur ne suit que l'`id` décroissant. Sur
un tri choisi, deux lignes de même valeur rendraient la frontière ambiguë et la
page suivante sauterait des lignes — le taire produirait des pages fausses que
rien ne signale.

Le guide donne le repository appelant en entier : le noyau offre le type, il
n'engendre pas son usage.

Vérifications :
- node docs/scripts/parite.mjs : exit 0, aucune paire signalée
EOF
```

---

## Self-Review

**Couverture de la spec** — chaque section a sa tâche :

| Section de la spec | Tâche |
|---|---|
| `Cursor`, l'extracteur, les quatre cas du tableau | Task 1, Steps 2 et 4 |
| Curseur en clair, non encodé | Task 1 (le type est `Uuid`, aucun encodage) |
| `CursorPage<T>`, absence de `total` | Task 2, test `the_cursor_page_never_counts_the_rows` |
| `next` nul en fin de marche | Task 2, tests `a_short_page_ends_the_walk` et `an_empty_page_ends_the_walk` |
| Ce que ça ne fait pas — pas de curseur sur `Sort` | Task 3, Step 2, dernier paragraphe |
| Le repository appelant, `lt` et non `lte` | Task 1 Step 4 (doc de `after()`) et Task 3 Step 2 |
| Les cinq tests nommés | Tasks 1 et 2 — noms repris tels quels, plus trois de renfort |
| Version, semver-checks | Task 2, Step 5 |
| CHANGELOG ×2, guide ×2 | Task 3 |

**Écart assumé avec la spec :** la spec annonce cinq tests, le plan en écrit huit. Les trois de plus (`a_readable_cursor_is_carried_through`, `an_empty_page_ends_the_walk`, `the_cursor_page_never_counts_the_rows`) couvrent le cas passant et l'absence de `total`, que les cinq laissaient hors garde.

**Point non prévu par la spec, tranché ici :** l'activation de `sea-orm/with-uuid` sur `rbs-core` (Task 1, Step 1). La spec supposait `Uuid` disponible ; il ne l'est pas.

**Cohérence des types :** `Cursor::after() -> Option<Uuid>` en Task 1, consommé en Task 2 par `CursorPage::new(data, &cursor, dernier: Option<Uuid>)` et en Task 3 par le bloc du guide. `per_page() -> u64` partout. Aucune divergence de nom.
