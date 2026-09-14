# Lot P3 « templates » — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** fermer quatre défauts du code engendré — `contains` qui lit `%`/`_` comme jokers, balayage O(n) du compteur de rate-limit, absence de compression HTTP, relecture inutile des jobs après `mark_done`/`retry_or_fail`.

**Architecture :** quatre retouches de templates sous `crates/rbs-cli/templates/`, chacune reportée dans `examples/` (test de non-dérive) et prouvée par un test engendré ou par la suite existante, puis passe Docker suite par suite.

**Tech Stack :** minijinja (délimiteurs `{@ @}` / `{% %}`), SeaORM 2.0.3 / sea-query 1.0.2, axum 0.8, tower-http 0.7.

**Spec :** design approuvé en session le 2026-09-14 (tâches 51, 53, 54, 56 d'`IMPROVE.md`, lire leurs lignes).

## Global Constraints

- Branche : `git checkout -b improve/p3-templates improve/p3-lot`. Avant la première ligne : `git rev-list --count HEAD..improve/p3-lot` doit rendre `0`.
- Ne jamais toucher `IMPROVE.md`, `TODO.md`, `CHANGELOG.md`, `CHANGELOG.fr.md` : l'orchestrateur s'en charge.
- Commits : Conventional Commits, sujet français à l'impératif sans majuscule ni point final, **aucun identifiant de tâche** (pas de « 51 », « #53 »), aucune mention de backlog/plan, **aucune ligne `Co-Authored-By` ni `Claude-Session`**, corps = pourquoi technique + intertitre `Vérifications :` avec les commandes réellement lancées et leur résultat.
- Commentaires : le *pourquoi*, jamais le *quoi*. Code engendré : ne commente que ses points d'extension et ses choix non évidents.
- minijinja : strip **à gauche seul** (`{%- if … %}`), jamais `-%}` devant une ligne indentée.
- Après **chaque** template modifiée : `cargo test -p rbs-cli --lib` **entier** (jamais filtré : la garde rustfmt des fragments vit dans `templates::tests`) puis `cargo test -p rbs-cli --test integration_examples -- --include-ignored`. Une dérive est une faute du gabarit, jamais des exemples.
- Fixtures figées : si `crates/rbs-cli/fixtures/` porte un rendu touché, `RBS_FIGE=1 cargo test -p rbs-cli --lib` le repose ; relire le diff.
- Report dans `examples/` : **par diff entre deux générations** (ancien CLI = `improve/p3-lot`, nouveau = ta branche), commandes exactes d'`examples/README.md`, appliquer par `patch --no-backup-if-mismatch` ou `git apply`, jamais par écrasement (régions `// region:` et éditions manuelles). Ignorer `migration/src/m*.rs`, `migration/src/lib.rs`, `Cargo.lock` dans le diff d'horodatage. Ensuite `find examples -name '*.orig' -o -name '*.rej'` doit être vide.
- Le code d'un fragment n'est compilé par aucun test rapide : `cargo check --all-targets` **dans l'exemple porteur** avant toute passe Docker (`hello-crud`, `blog-auth` pour `rate-limit`/`auth`, `newsletter-queue` et `event-hub` pour `jobs`).
- Sorties longues : rediriger vers `/private/tmp/claude-501/-Users-yacoubakone-dev-rs/479da9f6-2d4c-4897-b574-61f82e638062/scratchpad/templates-<nom>.log` (préfixe `templates-` obligatoire : le scratchpad est partagé avec l'agent du lot CLI). Totaliser par `awk '/^test result/ {p+=$4; f+=$6} END {print p" passés, "f" échecs"}'`.
- Passes Docker : **une suite par commande**, `--no-fail-fast` **avant** le `--` (`cargo test -p rbs-cli --test integration_crud --no-fail-fast -- --include-ignored`), `run_in_background`, jamais deux suites nommant `demo-api` en même temps (`grep -l '"demo-api"' crates/rbs-cli/tests/integration_*.rs`). La cible `target/rbs-integration` de ton worktree part froide : la première passe est longue.
- Tests d'un exemple (ignorés, base requise) : `docker run -d --rm -e POSTGRES_USER=rbs -e POSTGRES_PASSWORD=rbs -e POSTGRES_DB=<base du .env> -p <port libre>:5432 postgres:14-alpine`, adapter `RBS_DATABASE__URL` dans le `.env` de l'exemple **sans le committer**, `cargo run --manifest-path ../../Cargo.toml -p rbs-cli --bin rbs -- migrate up`, puis `cargo test -- --include-ignored <nom>`.
- Doc : toute page EN modifiée l'est aussi en FR (`docs/i18n/fr/docusaurus-plugin-content-docs/current/…`) dans le même commit. Après toute retouche d'`examples/` : `cd docs && npm ci` (une fois) puis `npm run build`, lire le vrai code de sortie.

---

### Task 1 : `contains` cherche la valeur à la lettre

**Files :**
- Modify : `crates/rbs-cli/templates/feature/filter.rs.jinja:1-3` (imports) et `:114-123` (`matches`)
- Modify : `crates/rbs-cli/templates/feature/tests.rs.jinja` (nouveau test après `the_filter_narrows_the_list`, `:445`)
- Modify : le calcul du contexte de rendu des tests CRUD (`grep -rn '"filterable"' crates/rbs-cli/src/generate`) pour exposer `contains_field`
- Report : `examples/*/src/*/filter.rs`, `examples/*/src/*/tests.rs` (ou leur emplacement réel)
- Doc : `docs/docs/guides/filtering.md` + jumelle FR, si elles décrivent `contains` (grep `contains`)

**Interfaces :** Produit la variable de template `contains_field` (nom du premier champ dont l'opérateur de filtre est `TextMatch`, hors champ email ou soumis à une validation de format) — absente s'il n'y en a pas, et le test n'est alors pas rendu.

- [ ] **Step 1 : le test engendré, rendu sous `{%- if contains_field %}`**

```rust
/// `%` et `_` sont des jokers de LIKE : lus tels quels, `contains: "%"` rendrait toute la
/// table.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn contains_reads_percent_and_underscore_literally() {
    let api = application().await;
    let collection = "/{@ module @}";
    let marque = Uuid::new_v4().simple().to_string()[..10].to_string();
    let avec = format!("{marque}%_");
    let sans = format!("{marque}xy");

    for valeur in [&avec, &sans] {
        let mut sent = creation();
        sent["{@ contains_field @}"] = json!(valeur);
        let (status, created) = call(&api, request("POST", collection, sent)).await;
        assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    }

    let chemin = format!("{collection}/filter");
    for motif in [format!("{marque}%"), format!("{marque}_")] {
        let critere = json!({ "{@ contains_field @}": { "contains": motif } });
        let (status, page) = call(&api, request("POST", &chemin, critere)).await;
        assert_eq!(status, StatusCode::OK, "filtre refusé : {page}");

        let valeurs: Vec<&str> = page["data"]
            .as_array()
            .expect("la liste rend un tableau")
            .iter()
            .map(|ligne| ligne["{@ contains_field @}"].as_str().expect("texte rendu"))
            .collect();
        assert_eq!(valeurs, vec![avec.as_str()], "« {motif} » doit se lire à la lettre : {page}");
    }
}
```

Adapter au besoin la longueur de `marque` à un `string(N)` court, et vérifier que `creation()` rend des valeurs uniques à chaque appel (champ `unique`).

- [ ] **Step 2 : voir le test échouer** — le reporter dans `examples/hello-crud`, base montée (cf. Global Constraints), `cargo test -- --include-ignored contains_reads_percent` → FAIL : deux valeurs rendues pour `…%`.

- [ ] **Step 3 : le correctif**

Imports de `filter.rs.jinja` : ajouter `use sea_orm::sea_query::LikeExpr;`. Remplacer le commentaire et la ligne `contains` de `matches` :

```rust
    // `contains` rend un LIKE '%…%' dont `%`, `_` et `!` sont échappés : la valeur se
    // cherche à la lettre. `!` plutôt que `\` : sea-query écrit le caractère d'échappement
    // en littéral SQL, et MySQL y lit `\` comme un échappement de chaîne. La casse suit la
    // collation du moteur : PostgreSQL la distingue, MySQL l'ignore par défaut.
    null_condition(colonne, recherche.is_null)
        .add_option(recherche.eq.clone().map(|valeur| colonne.eq(valeur)))
        .add_option(recherche.contains.as_deref().map(|valeur| colonne.like(motif(valeur))))
}

fn motif(valeur: &str) -> LikeExpr {
    let mut echappee = String::with_capacity(valeur.len() + 2);
    for caractere in valeur.chars() {
        if matches!(caractere, '!' | '%' | '_') {
            echappee.push('!');
        }
        echappee.push(caractere);
    }
    LikeExpr::new(format!("%{echappee}%")).escape('!')
}
```

(`grep -rn "struct TextMatch" -A10 crates/rbs-core/src` : si `contains` n'est pas un `Option<String>`, adapter `as_deref`.)

- [ ] **Step 4 : voir le test passer** dans `examples/hello-crud` (même commande) → PASS ; puis gardes rapides (`--lib` entier, `integration_examples -- --include-ignored`) après report dans les cinq exemples.

- [ ] **Step 5 : commit** — `fix(filter): échappe les jokers de LIKE dans contains`

### Task 2 : le compteur mémoire ne rebalaie qu'après avoir doublé

**Files :**
- Modify : `crates/rbs-cli/templates/features/rate-limit/counter.rs.jinja` (branche mémoire, `:66-131`)
- Modify : `crates/rbs-cli/templates/features/rate-limit/tests.rs.jinja` (tests sous la **même** condition que la branche mémoire du compteur — lire la condition en tête de `counter.rs.jinja`)
- Report : `examples/blog-auth/src/modules/rate_limit/`, `examples/event-hub/src/modules/rate_limit/`

**Interfaces :** Produit, dans `counter.rs` (branche mémoire) : `pub(super) const SEUIL_DE_BALAYAGE: usize` et `pub(super) fn prochain_seuil(vivantes: usize) -> usize`.

- [ ] **Step 1 : tests**

```rust
#[test]
fn the_sweep_threshold_doubles_past_the_floor() {
    use super::counter::{SEUIL_DE_BALAYAGE, prochain_seuil};

    assert_eq!(prochain_seuil(0), SEUIL_DE_BALAYAGE);
    assert_eq!(prochain_seuil(SEUIL_DE_BALAYAGE / 2), SEUIL_DE_BALAYAGE);
    assert_eq!(prochain_seuil(SEUIL_DE_BALAYAGE), 2 * SEUIL_DE_BALAYAGE);
    assert_eq!(prochain_seuil(usize::MAX), usize::MAX);
}

#[tokio::test]
async fn live_windows_survive_the_sweep() {
    let counter = super::Counter::new().expect("compteur constructible");
    let fenetre = Duration::from_secs(60);

    for i in 0..super::counter::SEUIL_DE_BALAYAGE {
        counter.hit(&format!("cle-{i}"), fenetre).await.expect("compté");
    }

    // La table est au seuil : ce coup-ci la balaie, et ne doit perdre aucune fenêtre vivante.
    assert_eq!(counter.hit("cle-0", fenetre).await.expect("compté"), 2);
}
```

(Adapter le chemin `super::counter::…` à la façon dont `tests.rs` atteint déjà `Counter`.)

- [ ] **Step 2 : voir échouer** — report dans `examples/blog-auth`, `cargo test --lib rate_limit` → erreur de compilation : `prochain_seuil` introuvable.

- [ ] **Step 3 : le correctif** (branche mémoire)

```rust
/// Taille en deçà de laquelle la table n'est jamais balayée.
pub(super) const SEUIL_DE_BALAYAGE: usize = 10_000;

/// Le compteur à fenêtre fixe du projet, partagé par tous les handlers.
#[derive(Debug, Clone, Default)]
pub struct Counter {
    table: Arc<Mutex<Table>>,
}

#[derive(Debug)]
struct Table {
    fenetres: HashMap<String, Fenetre>,
    seuil: usize,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            fenetres: HashMap::new(),
            seuil: SEUIL_DE_BALAYAGE,
        }
    }
}

/// La taille à laquelle le prochain balayage aura lieu, après un balayage qui a laissé
/// `vivantes` fenêtres.
///
/// Rebalayer au même seuil ferait parcourir toute la table sous le verrou à chaque
/// requête dès que 10 000 clients sont actifs à la fois. En doublant, un balayage n'a
/// lieu qu'après autant d'insertions qu'il a gardé de fenêtres : son coût amorti reste
/// constant, et la table ne dépasse pas le double des clés vivantes.
pub(super) fn prochain_seuil(vivantes: usize) -> usize {
    SEUIL_DE_BALAYAGE.max(vivantes.saturating_mul(2))
}
```

Dans `hit`, remplacer le verrou sur `fenetres` et le bloc de balayage :

```rust
        let mut table = self
            .table
            .lock()
            .unwrap_or_else(|empoisonne| empoisonne.into_inner());
        let Table { fenetres, seuil } = &mut *table;

        if fenetres.len() >= *seuil {
            fenetres.retain(|_, fenetre| fenetre.echeance > maintenant);
            *seuil = prochain_seuil(fenetres.len());
        }
```

Garder le commentaire existant sur le verrou empoisonné et celui sur l'amortissement (à fusionner avec la doc de `prochain_seuil`, sans redite).

- [ ] **Step 4 : voir passer** dans `examples/blog-auth` (`cargo test --lib rate_limit`), puis gardes rapides et report dans `event-hub`.

- [ ] **Step 5 : commit** — `perf(rate-limit): ne rebalaie la table qu'après l'avoir vue doubler`

### Task 3 : compression gzip des réponses

**Files :**
- Modify : `crates/rbs-cli/templates/project/src/router.rs.jinja:7,27-32`
- Modify : `crates/rbs-cli/templates/project/Cargo.toml.jinja:39`
- Modify : `crates/rbs-cli/templates/feature/tests.rs.jinja` (un test)
- Vérifier : `grep -rn 'tower-http\|"timeout"' crates/rbs-cli/templates/features/*/feature.toml crates/rbs-cli/src crates/rbs-cli/tests` — un fragment qui ajoute une feature à `tower-http` (cors…) et les tests qui figent la ligne `features = ["timeout"…]` doivent suivre.
- Report : les cinq exemples (`Cargo.toml`, `src/router.rs`, `Cargo.lock` par `cargo build` dans l'exemple).
- Doc : `grep -rn 'TimeoutLayer\|tower-http\|<rbs:layers>' docs/docs docs/i18n` — une page qui décrit les couches du squelette la mentionne (EN+FR).

- [ ] **Step 1 : test engendré** (dans `tests.rs.jinja`, sans passer par `call`, qui décode du JSON)

```rust
/// Le squelette compresse ce que le client accepte de recevoir compressé.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_list_travels_compressed_when_the_client_accepts_it() {
    let api = application().await;
    let collection = "/{@ module @}";
    let (status, created) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");

    let mut demande = request("GET", collection, Value::Null);
    demande
        .headers_mut()
        .insert("accept-encoding", "gzip".parse().expect("en-tête valide"));
    let reponse = api.clone().oneshot(demande).await.expect("réponse");

    assert_eq!(reponse.status(), StatusCode::OK);
    assert_eq!(
        reponse.headers().get("content-encoding").map(|v| v.as_bytes()),
        Some(&b"gzip"[..])
    );
}
```

(Adapter la construction de la requête GET à ce que `request(...)` accepte réellement ; sous `auth`, la même en-tête d'autorisation que les autres lectures.)

- [ ] **Step 2 : voir échouer** dans `examples/hello-crud` (base montée) → `content-encoding` absent.

- [ ] **Step 3 : le correctif**

`Cargo.toml.jinja` : `tower-http = { version = "0.7", features = ["timeout", "compression-gzip"] }`.

`router.rs.jinja` : `use tower_http::compression::CompressionLayer;` et, dans `<rbs:layers>`, après le `TimeoutLayer` :

```rust
        // Le document OpenAPI dépasse vite la centaine de Ko. Le prédicat par défaut
        // épargne les petits corps, les images et les flux SSE, que la compression
        // ralentirait sans rien gagner.
        .layer(CompressionLayer::new())
```

- [ ] **Step 4 : voir passer** dans `hello-crud` ; gardes rapides ; report dans les cinq exemples ; `cargo check --all-targets` dans chacun.

- [ ] **Step 5 : commit** — `perf(project): compresse les réponses en gzip`

### Task 4 : `mark_done` et `retry_or_fail` n'écrivent que leurs colonnes

Diagnostic corrigé : `ActiveModel::update` n'écrit déjà que les colonnes `Set` (sea-orm `query/update.rs:108-117`) ; ce qu'il coûte, c'est la relecture de la ligne entière, payload compris — `RETURNING` de toutes les colonnes sur PostgreSQL et SQLite, `SELECT` de plus sur MySQL — pour un modèle que personne ne lit. Pas de test rouge possible : le comportement ne change pas, la preuve est la suite jobs existante.

**Files :**
- Modify : `crates/rbs-cli/templates/features/jobs/queue.rs.jinja:296-304` et `:331-354`
- Report : `examples/newsletter-queue/src/modules/jobs/queue.rs`, `examples/event-hub/src/modules/jobs/queue.rs`

- [ ] **Step 1 : le correctif**

```rust
/// Marque un job réussi.
///
/// Un `UPDATE` ciblé plutôt que `ActiveModel::update` : celui-ci relit la ligne entière,
/// payload compris — par `RETURNING` sur PostgreSQL et SQLite, par un `SELECT` de plus sur
/// MySQL — pour rendre un modèle que personne ne lit.
pub async fn mark_done(db: &DatabaseConnection, job: &Model) -> anyhow::Result<()> {
    Entity::update_many()
        .col_expr(Column::Status, Expr::value(Status::Done))
        .col_expr(Column::LastError, Expr::value(Option::<String>::None))
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now().fixed_offset()))
        .filter(Column::Id.eq(job.id))
        .exec(db)
        .await?;

    Ok(())
}
```

Dans `retry_or_fail`, remplacer les six lignes `let mut ligne … ligne.update(db).await?;` par :

```rust
    Entity::update_many()
        .col_expr(Column::Status, Expr::value(status))
        .col_expr(Column::LastError, Expr::value(Some(format!("{error:#}"))))
        .col_expr(
            Column::AvailableAt,
            Expr::value(a_la_seconde((Utc::now() + attente).fixed_offset())),
        )
        .col_expr(Column::UpdatedAt, Expr::value(Utc::now().fixed_offset()))
        .filter(Column::Id.eq(job.id))
        .exec(db)
        .await?;
```

Retirer les imports devenus inutiles (`ActiveModel`, `Set`) seulement si plus rien ne les emploie dans le fichier.

- [ ] **Step 2 : vérifier** — gardes rapides ; report dans les deux exemples ; `cargo clippy --all-targets -- -D warnings` dans `newsletter-queue` et `event-hub`.

- [ ] **Step 3 : commit** — `perf(jobs): inscrit le sort d'un job sans relire sa ligne`

### Task 5 : passe finale

- [ ] `cargo fmt --all --check` et `cargo clippy --workspace --all-targets -- -D warnings` : aucune sortie.
- [ ] `cargo test -p rbs-cli --lib` entier ; `cargo test -p rbs-cli --test integration_examples -- --include-ignored`.
- [ ] `find examples -name '*.orig' -o -name '*.rej'` vide ; `cd docs && npm run build` exit 0.
- [ ] Docker, une suite par job en arrière-plan, sortie redirigée (`templates-<suite>.log`) : `integration_crud` (tests CRUD engendrés : Tasks 1 et 3), `integration_jobs` puis `integration_webhooks` (Task 4), la suite qui exerce `rate-limit` en mémoire (`grep -ln 'rate-limit' crates/rbs-cli/tests/integration_*.rs`, probablement `integration_auth`), `integration_new`. Chaque commande : `cargo test -p rbs-cli --test <suite> --no-fail-fast -- --include-ignored > …/templates-<suite>.log 2>&1`.
- [ ] Rapport final : par tâche, commit, test rouge vu (sortie citée), test vert, et chiffres de chaque suite (passés / échecs / ignorés). Tout échec rencontré est cité tel quel, jamais résumé en « vert ».
