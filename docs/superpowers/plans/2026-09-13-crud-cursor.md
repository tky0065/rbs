# `rbs generate crud --cursor` — plan d'implémentation

> **Pour les agents :** sous-skill requis : `superpowers:subagent-driven-development`.
> Les étapes se suivent en cases `- [ ]`.

**But :** sous `--cursor`, `GET /<ressource>` prend `Query<Cursor>` (`after`, `per_page`)
et rend `CursorPage<<Entite>Response>` ; la route de filtre reste en `Page`/`Pagination`.

**Architecture :** un booléen `cursor` de plus sur `Feature` (sérialisé pour les
gabarits), posé par `generate::command` depuis `Options.cursor`. Quatre gabarits de
`templates/feature/` gagnent une branche `{%- if cursor %}` : repository (`list` propre,
`id < after`, `ORDER BY id DESC`, `LIMIT`), service (`CursorPage::new`), contrôleur
(extracteur `Cursor`, annotation OpenAPI), tests engendrés (marche jusqu'à l'extinction
de `next`, sans doublon). Le rendu par défaut ne bouge pas d'un octet.

**Spec :** `docs/superpowers/specs/2026-09-13-lot-features-p2-design.md`, section « 38 ».

## Contraintes globales

- `rbs_core::Cursor` : `after() -> Option<Uuid>` (borne **exclusive** : `Column::Id.lt(after)`),
  `per_page() -> u64` ; `rbs_core::CursorPage::new(data: Vec<T>, cursor: &Cursor, dernier: Option<Uuid>)`
  (`crates/rbs-core/src/pagination.rs:103-238`). Extracteur `Cursor` = `FromRequestParts`.
- Sous `--soft-delete`, la condition `deleted_at IS NULL` demeure sur la liste par curseur.
- La route de filtre reste en `Page`/`Pagination`, tri libre compris.
- Le commentaire engendré « Un seul chemin de lecture » (`repository.rs.jinja:66-67`)
  devient, sous `--cursor`, la raison de l'exception : un curseur sur l'`id` est faux dès
  que le tri porte sur une autre colonne.
- Compatible avec `--role`, `--soft-delete`, `--with-upload` (aucun refus croisé ;
  `--has-many` reste le chemin de réparation, qui ne rend aucun gabarit).
- Les gabarits de `generate` s'écrivent **déjà** comme rustfmt les écrirait (gardes
  `bench::longueurs_divergentes`) ; les blancs sous drapeau se figent par
  `bench::fige` (`RBS_FIGE=1 cargo test` repose une fixture).
- Minijinja : délimiteurs `{@ @}`, convention du strip à gauche seul (`{%- if … %}`).
- Les descriptions OpenAPI des gabarits sont en français seul (aucune bascule `lang`).
- Aucun exemple n'emploie `--cursor` : `integration_examples` doit rester vert **sans
  toucher `examples/`**. S'il bouge, c'est une régression du rendu par défaut.
- `cargo test -p rbs-cli --lib` **entier** (jamais filtré) après toute retouche de gabarit.

---

### Tâche 1 : le drapeau jusqu'au contexte de rendu

**Fichiers :** `crates/rbs-cli/src/cli.rs`, `crates/rbs-cli/src/lib.rs`
(`GenerateArgs`), `crates/rbs-cli/src/generate/command.rs` (`Options`),
`crates/rbs-cli/src/generate/feature.rs`.

**Interfaces produites :** `Options.cursor: bool` ; `Feature.cursor: bool` ;
`Feature::paged_by_cursor(self) -> Self` ; clé `cursor` du contexte de rendu.

- [ ] Tests rouges : `generate_crud_accepts_cursor` (cli.rs, modèle de
  `generate_crud_accepts_soft_delete`) ; dans `feature.rs`, le contexte sérialisé porte
  `cursor: false` par défaut et `true` après `paged_by_cursor()`.
- [ ] CLI : `/// Pagine GET /<ressource> par curseur ; la route de filtre garde ses pages.`
  `#[arg(long)] cursor: bool` après `with_upload`. `lib.rs` : champ `cursor` de
  `GenerateArgs` (faux pour `feature`), passé à `Options`. `command.rs` : `if
  options.cursor { feature.paged_by_cursor() }` à la suite de `uploading()` ; tout
  `Options { … }` littéral des tests reçoit `cursor: false`.
- [ ] `cargo test -p rbs-cli --lib` vert ; commit
  `feat(generate): accepte --cursor sur generate crud`.

### Tâche 2 : repository et service

**Fichiers :** `crates/rbs-cli/templates/feature/repository.rs.jinja`,
`crates/rbs-cli/templates/feature/service.rs.jinja`, tests de
`crates/rbs-cli/src/generate/repository.rs` et `crates/rbs-cli/src/generate/service.rs`,
`crates/rbs-cli/src/generate/bench.rs` (une `Feature` témoin `cursor`), fixtures
`crates/rbs-cli/fixtures/cursor/{repository,service}.rs`.

- [ ] Tests rouges (rendu de `articles`, `title:string`, `paged_by_cursor()`) :
  - repository : `pub async fn list(db: &DatabaseConnection, cursor: &Cursor) -> Result<Vec<Model>> {`,
    `Column::Id.lt(after)`, `.order_by_desc(Column::Id)`, `.limit(cursor.per_page())` ;
    `filter` garde `pagination: &Pagination` ; sous `soft_deleting()` en plus, le corps de
    `list` porte `Column::DeletedAt.is_null()` ; le commentaire « Un seul chemin de
    lecture » est absent et remplacé par la raison de l'exception ;
  - service : `repository::list(db, cursor)`, `CursorPage::new(`, `.last().map(`, et
    `filter` rend toujours `Page<ArticleResponse>` ;
  - rustfmt : `bench::longueurs_divergentes` sur le rendu `cursor` de chaque fichier rend
    le même ensemble que le rendu par défaut (mesurer celui-ci d'abord ; toute divergence
    propre à la branche se corrige dans le gabarit, par une forme conditionnée à la
    longueur comme les macros `entete`/`chaine`) ;
  - fixtures figées `fixtures/cursor/repository.rs` (cursor + soft_delete) et
    `fixtures/cursor/service.rs` (cursor), posées par `RBS_FIGE=1`, relues à l'œil.
- [ ] Implémenter. Repository sous `cursor` :

```rust
pub async fn list(db: &DatabaseConnection, cursor: &Cursor) -> Result<Vec<Model>> {
    // Deux chemins de lecture, et c'est voulu : un curseur sur l'`id` n'a de sens que
    // trié sur l'`id`, quand le filtre accepte tout tri. La route de filtre garde donc
    // sa pagination par page.
    let mut requete = Entity::find();
    if let Some(after) = cursor.after() {
        requete = requete.filter(Column::Id.lt(after));
    }

    Ok(requete
        .order_by_desc(Column::Id)
        .limit(cursor.per_page())
        .all(db)
        .await?)
}
```

  (`Entity::find().filter(Column::DeletedAt.is_null())` sous `soft_delete`) ; imports
  `rbs_core::{Cursor, Error, Pagination, Result}`, `ColumnTrait`, `QueryFilter`,
  `QueryOrder`, `super::model::{Column, Entity}` dans la forme que rustfmt écrit.
  Service sous `cursor` : `list(db, cursor: &Cursor) -> Result<CursorPage<…Response>>`,
  `let dernier = {@ module @}.last().map(|ligne| ligne.id);` puis
  `CursorPage::new(<chaîne into_iter/map/collect>, cursor, dernier)`.
- [ ] `cargo test -p rbs-cli --lib` entier, `integration_examples` (rapide :
  `cargo test -p rbs-cli --test integration_examples -- --include-ignored`), clippy, fmt ;
  commit `feat(generate): pagine la liste par curseur dans le repository et le service`.

### Tâche 3 : contrôleur et tests engendrés

**Fichiers :** `crates/rbs-cli/templates/feature/controller.rs.jinja`,
`crates/rbs-cli/templates/feature/tests.rs.jinja`, tests de
`crates/rbs-cli/src/generate/controller.rs` et `crates/rbs-cli/src/generate/tests_http.rs`,
fixtures `crates/rbs-cli/fixtures/cursor/{controller,tests}.rs`.

- [ ] Tests rouges :
  - contrôleur : `list` porte `cursor: Cursor`, `-> Result<Json<CursorPage<ArticleResponse>>>`,
    `body = CursorPage<ArticleResponse>`, `("after" = Option<Uuid>, Query, …)`, plus de
    paramètre `page` sur `list` ; `filter` garde `pagination: Pagination` et
    `body = Page<ArticleResponse>` ; sous `authenticated()` le garde et `security` restent ;
  - tests engendrés : `the_cursor_walks_every_page_without_duplicates` présent sous
    `cursor` et absent par défaut ; le cycle de vie n'assert plus `meta.total` sous
    `cursor` (il vérifie `meta.per_page == 50`) ; sans `creatable`, la marche n'est pas
    engendrée ;
  - rustfmt : balayages sur les rendus `cursor` (et `cursor` + `authenticated()`) ;
  - fixtures figées `fixtures/cursor/controller.rs` (cursor + auth) et
    `fixtures/cursor/tests.rs` (cursor).
- [ ] Implémenter. Annotation de `list` sous `cursor` :

```text
    params(
        ("after" = Option<Uuid>, Query, description = "identifiant après lequel reprendre ; absent, la première page"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    responses(
        (status = 200, description = "page de {@ module @}", body = CursorPage<{@ entity @}Response>),
        (status = 400, description = "curseur ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json"){% if auth %}, …401, 403…{% endif %}
    )
```

  Corps : `Ok(Json(service::list(state.core().db(), &cursor).await?))`.
  Test engendré de la marche (sous `creatable` et `cursor`) : trois créations, pages de
  deux (`?per_page=2`, puis `&after=<next>`), boucle bornée à 10 000 pages avec
  constat que la marche s'est éteinte (`next` absent) plutôt qu'arrêtée par la borne ;
  aucun doublon (`HashSet`), les trois créations vues, la marche décroissante ; les trois
  lignes supprimées à la fin.
- [ ] `cargo test -p rbs-cli --lib` entier, `integration_examples`, clippy, fmt ; commit
  `feat(generate): rend la liste par curseur dans le contrôleur et ses tests`.

### Tâche 4 : preuve lente

**Fichiers :** Créer `crates/rbs-cli/tests/integration_cursor.rs`.

- [ ] Test `#[ignore]` : `common::start_postgres()`, `rbs new cursor-api --database-url
  <url> --core-path <noyau> --yes --with auth` (garde et `security` exercés), compose
  retiré, `generate crud articles --fields title:string --cursor --soft-delete`, verrou sur
  `common::cible()`, `rbs migrate up`, `cargo test --no-fail-fast -- --include-ignored`
  (CARGO_TARGET_DIR) → vert et `articles::tests::the_cursor_walks_every_page_without_duplicates`
  dans la sortie ; puis `rbs generate client --lang ts --force` → succès, le client
  porte une interface `CursorPage…` et `articlesList(`.
- [ ] Lancé seul en arrière-plan :
  `cargo test -p rbs-cli --test integration_cursor --no-fail-fast -- --include-ignored > …/g3-cursor.txt 2>&1`.
- [ ] Commit `test(generate): éprouve --cursor sur un projet compilé et son client`.

### Tâche 5 : documentation

- [ ] `docs/docs/cli/generate.md` (table des drapeaux, EN+FR), `docs/docs/guides/filtering.md`
  (section pagination par curseur, EN+FR : pourquoi la route de filtre reste en pages).
- [ ] `cd docs && npm run build` → 0 ; commit `docs(generate): documente --cursor`.
