# Point fixe rustfmt des gabarits de `rbs generate` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** faire que le rendu brut des gabarits de `rbs generate crud` soit déjà ce que
rustfmt écrirait, pour tout nom d'entité de 1 à 23 caractères, et que le test qui s'en dit
garant le prouve réellement.

**Architecture :** le test `the_render_goes_through_rustfmt_without_a_diff_whatever_the_name_length`
(`crates/rbs-cli/src/generate/command.rs:826`) lit aujourd'hui les fichiers *écrits* par
`run()`, que `format::format_batch` (`command.rs:280`) a déjà reformatés — il compare
rustfmt à lui-même et ne peut pas échouer. On le fait lire `render()`, le rendu brut. Cinq
gabarits divergent alors, et se corrigent par le mécanisme que `controller.rs.jinja:19`
pratique déjà : construire la ligne candidate en `{% set %}`, tester sa longueur contre le
seuil rustfmt applicable, écrire la forme compacte ou la forme éclatée. Enfin, la liste des
longueurs divergentes cesse de vivre dans quatre listes de noms écrites à la main et devient
une mesure, `bench::longueurs_divergentes`, partagée par les neuf gardes.

**Tech Stack :** Rust 2024, minijinja (délimiteurs alternatifs `{@ @}` / `{% %}`), rustfmt
appelé en sous-processus, `cargo test -p rbs-cli --lib`.

**Spec :** `IMPROVE.md` tâches 122, 124, 125, 126, 107, 119, 120 ; design approuvé en session
le 2026-09-02 (voie *bounded*, cinq pièces).

## Global Constraints

- **`format_batch` reformate le rendu à l'écriture** : rendre un gabarit point fixe ne
  change **rien** à la sortie livrée. `examples/` ne doit pas bouger d'un octet, et
  `integration_examples` reste vert sans qu'on y touche. Un diff dans `examples/` est le
  signal qu'on a changé le code produit, pas seulement sa mise en forme — s'arrêter et le
  signaler.
- **Ne pas réimplanter le remplissage glouton de rustfmt.** Le basculement autorisé est
  binaire — une ligne compacte *ou* un élément par ligne — comme `controller.rs.jinja:19`.
  Répartir N éléments sur plusieurs lignes en les remplissant est écarté (tâche 107) : cela
  réimplanterait une règle qu'une montée de rustfmt peut déplacer.
- **Les deux seuils de rustfmt en jeu**, tous deux à leur valeur par défaut :
  `fn_call_width = 60` sur les **arguments** d'un appel (parenthèses exclues), et
  `max_width = 100` sur la **ligne entière** d'une signature.
- **Cible du point fixe : 1 à 23 caractères de singulier.** 23, c'est
  `administrative_document` — le singulier du dernier nom de l'éventail du test — et c'est
  aussi le seuil naturel où `service.rs` et `controller.rs` s'arrêtent. Au-delà, la
  frontière est documentée, pas repoussée.
- **Aucune nouvelle entrée dans `IMPROVE.md`** (consigne du 2026-09-02) : ce qu'on croise
  en chemin se corrige dans ce lot.
- Commentaires : le *pourquoi*, jamais le *quoi* (`CLAUDE.md`). Commits en Conventional
  Commits, sujet français à l'impératif, sans identifiant de tâche ni renvoi à un fichier
  de suivi, **sans ligne `Co-Authored-By` ni `Claude-Session`**.

## Mesures de départ

Relevées le 2026-09-02 sur un balayage de noms d'entité de 1 à 40 caractères, identiques
pour les trois jeux de champs éprouvés (`title:string,views:int`, un jeu à cinq champs, et
la feature sans champ) — **les seuils ne dépendent que de la longueur du nom d'entité** :

| Rendu | Longueurs divergentes | Diagnostic |
|---|---|---|
| `model.rs`, `dto.rs`, `seed.rs` | aucune | point fixe sur [1, 40] |
| `service.rs` | 24 … 40 | seuil 23, connu (tâche 107) |
| `repository.rs` | 27 … 40 | seuil 26, connu (tâche 125) |
| `controller.rs` | **1, 2**, puis 24 … 40 | seuil 23 connu **+ un trou bas non répertorié** |
| `mod.rs` | 10 … 40 | seuil 9, à repousser |
| `filter.rs` | 13 … 40 | seuil 12, à repousser |
| `tests.rs` | **1 … 40** | jamais point fixe |
| migration | **1 … 40** | jamais point fixe |

`tests.rs` et la migration sont précisément les deux seuls rendus sans garde
`the_render_is_already_what_rustfmt_would_write` : le trou du test global recouvrait
exactement le trou des gardes.

---

### Task 1 : `bench::formatted` force `newline_style=Unix`

**Files:**
- Modify: `crates/rbs-cli/src/generate/bench.rs:436-460`

**Interfaces:**
- Consumes: rien.
- Produces: `pub(crate) fn formatted(source: &str) -> String`, signature inchangée.

- [ ] **Step 1 : lire les deux fonctions côte à côte**

`crates/rbs-cli/src/generate/format.rs:40-55` passe `--config newline_style=Unix` et écrit
pourquoi ; `bench.rs:436` ne le passe pas. Sur une machine Windows, le défaut « Auto »
retombe sur le style de la plateforme et les neuf gardes tomberaient en bloc sur une fin de
ligne plutôt que sur une mise en forme.

- [ ] **Step 2 : ajouter les deux arguments**

Dans `crates/rbs-cli/src/generate/bench.rs`, remplacer :

```rust
    let mut rustfmt = std::process::Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout", "--quiet"])
```

par :

```rust
    let mut rustfmt = std::process::Command::new("rustfmt")
        .args([
            "--edition",
            "2024",
            "--emit",
            "stdout",
            "--quiet",
            "--config",
            "newline_style=Unix",
        ])
```

- [ ] **Step 3 : compléter le doc-comment**

Ajouter au doc-comment existant de `bench::formatted`, après le paragraphe actuel :

```rust
/// `newline_style` est forcé pour la même raison qu'en `format::formatted` : son défaut,
/// « Auto », retombe sur le style de la plateforme, et les gardes compareraient un rendu
/// LF à une sortie CRLF.
```

- [ ] **Step 4 : les gardes existants restent verts**

Run : `cargo test -p rbs-cli --lib the_render_is_already_what_rustfmt_would_write`
Expected : PASS, 4 tests (`dto`, `service`, `repository`, `filter`) — le comportement ne
change pas sur une plateforme LF, la correction porte sur les autres.

- [ ] **Step 5 : commit**

```bash
git add crates/rbs-cli/src/generate/bench.rs
git commit -m "fix(generate): force le style de fin de ligne du banc rustfmt"
```

---

### Task 2 : le mesureur de seuil, à un seul endroit

**Files:**
- Modify: `crates/rbs-cli/src/generate/bench.rs` (ajout en fin de fichier, à côté de `formatted`)

**Interfaces:**
- Consumes: `bench::formatted` (Task 1).
- Produces:
  `pub(crate) fn longueurs_divergentes(rendu: impl Fn(&str) -> String) -> Vec<usize>` —
  balaie des noms de 1 à 40 caractères, rend les longueurs dont le rendu diverge de rustfmt.
  Les tâches 3 à 8 s'en servent comme assertion unique de chaque garde.

- [ ] **Step 1 : écrire le test du mesureur lui-même**

Dans le `mod tests` de `crates/rbs-cli/src/generate/bench.rs` (le créer s'il n'existe pas,
avec `use super::*;`) :

```rust
    /// Le mesureur doit distinguer un rendu point fixe d'un rendu qui ne l'est jamais,
    /// sans quoi les gardes qui s'appuient dessus ne prouveraient rien.
    #[test]
    fn the_measurer_tells_a_fixed_point_from_a_diverging_render() {
        let point_fixe = longueurs_divergentes(|name| format!("pub struct {name};\n"));
        assert!(
            point_fixe.is_empty(),
            "une déclaration courte est point fixe à toute longueur : {point_fixe:?}"
        );

        let jamais = longueurs_divergentes(|name| format!("pub struct {name} ;\n"));
        assert_eq!(
            jamais,
            (1..=40).collect::<Vec<usize>>(),
            "l'espace avant le point-virgule diverge à toute longueur"
        );
    }
```

- [ ] **Step 2 : lancer le test, le voir échouer**

Run : `cargo test -p rbs-cli --lib the_measurer_tells_a_fixed_point`
Expected : FAIL à la compilation — `cannot find function `longueurs_divergentes``.

- [ ] **Step 3 : écrire le mesureur**

Dans `crates/rbs-cli/src/generate/bench.rs`, à côté de `formatted` :

```rust
/// Les longueurs de nom pour lesquelles `rendu` n'est pas déjà ce que rustfmt écrirait.
///
/// Une liste de quatre noms écrite à la main ne dit pas où le gabarit bascule : elle
/// échoue sans nommer le seuil, et ment en silence le jour où une montée de rustfmt le
/// déplace. Un balayage rend la frontière elle-même, et un `assert_eq!` sur l'intervalle
/// affiche l'ancien seuil et le nouveau.
///
/// Le nom passé à `rendu` est fait d'un `a` répété terminé par un `e` : il ne se pluralise
/// pas, donc sa longueur est bien celle du singulier que portent les gabarits.
pub(crate) fn longueurs_divergentes(rendu: impl Fn(&str) -> String) -> Vec<usize> {
    (1..=40usize)
        .filter(|taille| {
            let name = "a".repeat(taille - 1) + "e";
            let rendered = rendu(&name);
            formatted(&rendered) != rendered
        })
        .collect()
}
```

- [ ] **Step 4 : lancer le test, le voir passer**

Run : `cargo test -p rbs-cli --lib the_measurer_tells_a_fixed_point`
Expected : PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/rbs-cli/src/generate/bench.rs
git commit -m "test(generate): mesure la plage où un gabarit est point fixe de rustfmt"
```

---

### Task 3 : `tests.rs.jinja` — l'`assert_eq!` à trois arguments

**Files:**
- Modify: `crates/rbs-cli/templates/feature/tests.rs.jinja:237`
- Modify: `crates/rbs-cli/src/generate/tests_http.rs` (ajout du garde manquant)

**Interfaces:**
- Consumes: `bench::longueurs_divergentes` (Task 2), `tests_http::render(&Feature)`.
- Produces: le rendu de `tests.rs` point fixe à toute longueur.

- [ ] **Step 1 : écrire le garde manquant, qui échoue**

Dans le `mod tests` de `crates/rbs-cli/src/generate/tests_http.rs`, sur le modèle exact de
celui de `dto.rs:239` (mêmes `use`, même helper de rendu local) :

```rust
    /// Ce fichier n'a aucune ligne qui suive le nom de l'entité d'assez près pour franchir
    /// un seuil : sa seule bascule tenait à un `assert_eq!` à trois arguments, dont les
    /// arguments dépassent les soixante colonnes de `fn_call_width` quel que soit le nom.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(|name| {
            let fields = fields::parse("title:string,views:int").expect("champs valides");
            tests_http::render(&Feature::fresh(name, fields)).expect("rendu")
        });

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu des tests HTTP diverge de rustfmt à ces longueurs de nom"
        );
    }
```

Adapter les chemins d'appel aux `use` déjà présents dans le fichier (`super::*` y donne
`render`; `crate::generate::bench` et `crate::generate::fields` sont importés comme dans
`dto.rs`).

- [ ] **Step 2 : lancer le garde, le voir échouer**

Run : `cargo test -p rbs-cli --lib tests_http::tests::the_render_is_already_what_rustfmt_would_write`
Expected : FAIL — `divergentes` vaut `[1, 2, …, 40]`, le rendu diverge à toute longueur.

- [ ] **Step 3 : éclater l'`assert_eq!` dans le gabarit**

Dans `crates/rbs-cli/templates/feature/tests.rs.jinja`, remplacer la ligne 237 :

```
    assert_eq!(premier.get_version_num(), 7, "{premier} n'est pas un UUIDv7");
```

par exactement ce que rustfmt en fait — les arguments de la macro valent plus de soixante
colonnes, donc un par ligne, sans virgule finale :

```
    assert_eq!(
        premier.get_version_num(),
        7,
        "{premier} n'est pas un UUIDv7"
    );
```

- [ ] **Step 4 : lancer le garde, le voir passer**

Run : `cargo test -p rbs-cli --lib tests_http::tests::the_render_is_already_what_rustfmt_would_write`
Expected : PASS.

Si d'autres longueurs restent listées, ce sont d'autres lignes du même gabarit : les lire
avec `cargo test -p rbs-cli --lib tests_http -- --nocapture` et appliquer le même traitement,
sans jamais deviner la forme — c'est celle que rustfmt rend qu'on recopie.

- [ ] **Step 5 : la suite du module reste verte**

Run : `cargo test -p rbs-cli --lib tests_http`
Expected : PASS, tous les tests du module.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/templates/feature/tests.rs.jinja crates/rbs-cli/src/generate/tests_http.rs
git commit -m "fix(generate): écrit les tests engendrés dans la forme de rustfmt"
```

---

### Task 4 : `migration.rs.jinja` — les colonnes compactes quand rustfmt les compacte

**Files:**
- Modify: `crates/rbs-cli/templates/feature/migration.rs.jinja:13-22`
- Modify: `crates/rbs-cli/src/generate/migration.rs` (ajout du garde manquant)

**Interfaces:**
- Consumes: `bench::longueurs_divergentes` (Task 2), `migration::render(&Feature, &str)`.
- Produces: le rendu de la migration point fixe sur [1, 23].

**Ce qui diverge :** le gabarit écrit la colonne `Id` **toujours** éclatée, quand rustfmt la
compacte tant que les arguments de `.col(…)` tiennent en soixante colonnes ; et il écrit les
colonnes de champs **toujours** compactes, quand rustfmt les éclate au-delà. `CreatedAt` et
`UpdatedAt` ne sont pas concernées : leurs arguments valent 99 caractères plus l'iden, donc
elles restent éclatées à toute longueur, et le gabarit les écrit déjà ainsi.

- [ ] **Step 1 : écrire le garde manquant, qui échoue**

Dans le `mod tests` de `crates/rbs-cli/src/generate/migration.rs`, sur le modèle de celui de
`dto.rs:239` :

```rust
    /// Deux régimes se croisent dans ce fichier, tous deux régis par les soixante colonnes
    /// de `fn_call_width` : la colonne `Id`, que rustfmt compacte tant que ses arguments
    /// tiennent, et les colonnes de champs, qu'il éclate dès qu'elles débordent. Le
    /// gabarit écrivait chacune dans un seul de ces deux régimes.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(|name| {
            let fields = fields::parse("title:string,views:int").expect("champs valides");
            render(&Feature::fresh(name, fields), "m20260101_000000").expect("rendu")
        });

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu de la migration diverge de rustfmt à ces longueurs de nom"
        );
    }
```

Reprendre la signature exacte de `migration::render` telle qu'elle est appelée ailleurs dans
le fichier (`command.rs:524` l'appelle avec `&migration::current_timestamp()`) ; un
horodatage fixe suffit ici et rend le test déterministe.

- [ ] **Step 2 : lancer le garde, le voir échouer**

Run : `cargo test -p rbs-cli --lib migration::tests::the_render_is_already_what_rustfmt_would_write`
Expected : FAIL — `divergentes` vaut `[1, 2, …, 40]`.

- [ ] **Step 3 : conditionner la colonne `Id`**

Dans `crates/rbs-cli/templates/feature/migration.rs.jinja`, remplacer :

```
                    .col(
                        ColumnDef::new({@ iden @}::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
```

par :

```
{#- Les arguments d'un appel basculent aux soixante colonnes de `fn_call_width` : au-delà,
    rustfmt pose un élément par ligne. Le gabarit écrit les deux régimes plutôt que d'en
    laisser un à `format_batch`, qui ne rattrape la mise en forme qu'à l'écriture. -#}
{% set col_id = "ColumnDef::new(" ~ iden ~ "::Id).uuid().not_null().primary_key()" -%}
{% if col_id | length <= 60 %}
                    .col({@ col_id @})
{%- else %}
                    .col(
                        ColumnDef::new({@ iden @}::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
{%- endif %}
```

- [ ] **Step 4 : conditionner les colonnes de champs**

Remplacer la boucle :

```
{%- for field in fields %}
                    .col(ColumnDef::new({@ iden @}::{@ field.pascal_name @}).{@ field.migration_method @}.{@ "null()" if field.optional else "not_null()" @}{@ ".unique_key()" if field.unique else "" @})
{%- endfor %}
```

par :

```
{%- for field in fields %}
{% set col = "ColumnDef::new(" ~ iden ~ "::" ~ field.pascal_name ~ ")." ~ field.migration_method ~ "." ~ ("null()" if field.optional else "not_null()") ~ (".unique_key()" if field.unique else "") -%}
{% if col | length <= 60 %}
                    .col({@ col @})
{%- else %}
                    .col(
                        ColumnDef::new({@ iden @}::{@ field.pascal_name @})
                            .{@ field.migration_method @}
                            .{@ "null()" if field.optional else "not_null()" @}{@ "
                            .unique_key()" if field.unique else "" @},
                    )
{%- endif %}
{%- endfor %}
```

- [ ] **Step 5 : lancer le garde et lire ce qui reste**

Run : `cargo test -p rbs-cli --lib migration::tests::the_render_is_already_what_rustfmt_would_write -- --nocapture`
Expected : PASS. Si des longueurs subsistent, la forme éclatée écrite ci-dessus ne
correspond pas à celle de rustfmt : rendre la migration pour l'une de ces longueurs, la
passer à `bench::formatted`, et recopier sa sortie dans le gabarit — jamais l'inverse.

- [ ] **Step 6 : la suite du module reste verte**

Run : `cargo test -p rbs-cli --lib migration`
Expected : PASS. Le module porte des tests sur le SQL produit (`fields.rs:475` pose les
index) : aucun ne doit bouger, la correction ne touche que la mise en forme.

- [ ] **Step 7 : commit**

```bash
git add crates/rbs-cli/templates/feature/migration.rs.jinja crates/rbs-cli/src/generate/migration.rs
git commit -m "fix(generate): écrit les colonnes de la migration dans la forme de rustfmt"
```

---

### Task 5 : `mod.rs.jinja` — la route de collection

**Files:**
- Modify: `crates/rbs-cli/templates/feature/mod.rs.jinja:18`
- Modify: `crates/rbs-cli/src/generate/controller.rs` (le garde de `render_mod`)

**Interfaces:**
- Consumes: `bench::longueurs_divergentes` (Task 2), `controller::render_mod(&Feature, bool)`.
- Produces: le rendu de `mod.rs` point fixe sur [1, 40].

**Le calcul, vérifié :** les arguments de `.route(…)` valent
`"/` + module + `", get(controller::list).post(controller::create)`, soit la longueur du
module plus 50. Ils franchissent les soixante colonnes de `fn_call_width` à partir d'un
module de 11 caractères — ce qui recoupe exactement la mesure, qui voit `mod.rs` diverger dès
un nom d'entité de 10.

- [ ] **Step 1 : écrire le garde, qui échoue**

Dans le `mod tests` de `crates/rbs-cli/src/generate/controller.rs`, à côté du garde existant :

```rust
    /// La route de collection est le seul appel de ce fichier dont les arguments suivent le
    /// nom du module : ils franchissent les soixante colonnes de `fn_call_width` à onze
    /// caractères de module.
    #[test]
    fn the_module_render_is_already_what_rustfmt_would_write() {
        let divergentes = bench::longueurs_divergentes(|name| {
            let fields = fields::parse("title:string,views:int").expect("champs valides");
            render_mod(&Feature::fresh(name, fields), true).expect("rendu")
        });

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu de `mod.rs` diverge de rustfmt à ces longueurs de nom"
        );
    }
```

- [ ] **Step 2 : lancer le garde, le voir échouer**

Run : `cargo test -p rbs-cli --lib the_module_render_is_already_what_rustfmt_would_write`
Expected : FAIL — `divergentes` vaut `[10, 11, …, 40]`.

- [ ] **Step 3 : conditionner la route dans le gabarit**

Dans `crates/rbs-cli/templates/feature/mod.rs.jinja`, remplacer :

```
        .route("/{@ module @}", get(controller::list).post(controller::create))
```

par :

```
{#- Les soixante colonnes de `fn_call_width` : au-delà, rustfmt pose un argument par ligne
    et ajoute la virgule finale. -#}
{% set route_collection = "\"/" ~ module ~ "\", get(controller::list).post(controller::create)" -%}
{% if route_collection | length <= 60 %}
        .route({@ route_collection @})
{%- else %}
        .route(
            "/{@ module @}",
            get(controller::list).post(controller::create),
        )
{%- endif %}
```

Attention à ne pas déplacer le commentaire qui suit immédiatement cette ligne (« Avant
`/{module}/{id}`, sans quoi `filter` serait lu comme un identifiant ») : il documente
l'ordre des routes et doit rester entre la route de collection et celle de filtrage.

- [ ] **Step 4 : lancer le garde, le voir passer**

Run : `cargo test -p rbs-cli --lib the_module_render_is_already_what_rustfmt_would_write`
Expected : PASS.

- [ ] **Step 5 : les tests de routage restent verts**

Run : `cargo test -p rbs-cli --lib controller`
Expected : PASS — en particulier les tests qui assertent la présence des routes par
`contains`, que la forme éclatée casserait s'ils cherchaient la ligne entière.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/templates/feature/mod.rs.jinja crates/rbs-cli/src/generate/controller.rs
git commit -m "fix(generate): écrit la route de collection dans la forme de rustfmt"
```

---

### Task 6 : `filter.rs.jinja` — la signature d'`apply`

**Files:**
- Modify: `crates/rbs-cli/templates/feature/filter.rs.jinja` (la ligne `pub(super) fn apply(`)
- Modify: `crates/rbs-cli/src/generate/filter.rs:302` (le garde existant)

**Interfaces:**
- Consumes: `bench::longueurs_divergentes` (Task 2), `filter::render(&Feature)`.
- Produces: le rendu de `filter.rs` point fixe sur [1, 40].

**Le calcul, vérifié :** `pub(super) fn apply(select: Select<Entity>, filtre: &{Entity}Filter) -> Result<Select<Entity>> {`
vaut 88 caractères plus le nom de l'entité. C'est une signature, donc régie par les cent
colonnes de `max_width` : elle déborde à partir d'une entité de 13 caractères — ce que la
mesure confirme.

- [ ] **Step 1 : remplacer la liste de noms du garde par la mesure**

Dans `crates/rbs-cli/src/generate/filter.rs`, remplacer le corps du garde
`the_render_is_already_what_rustfmt_would_write` (ligne 302) par :

```rust
        let divergentes = bench::longueurs_divergentes(|name| {
            let fields = fields::parse("title:string,views:int").expect("champs valides");
            render(&Feature::fresh(name, fields)).expect("rendu")
        });

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu du filtre diverge de rustfmt à ces longueurs de nom"
        );
```

et son doc-comment par :

```rust
    /// La signature d'`apply` est la seule ligne de ce fichier qui suive le nom de
    /// l'entité : elle vaut 88 caractères de plus que lui, et franchit donc les cent
    /// colonnes de `max_width` à treize.
```

- [ ] **Step 2 : lancer le garde, le voir échouer**

Run : `cargo test -p rbs-cli --lib filter::tests::the_render_is_already_what_rustfmt_would_write`
Expected : FAIL — `divergentes` vaut `[13, 14, …, 40]`. Le garde passait auparavant : sa
liste de quatre noms ne franchissait pas ce seuil, alors même que `articles` en est proche.

- [ ] **Step 3 : conditionner la signature dans le gabarit**

Dans `crates/rbs-cli/templates/feature/filter.rs.jinja`, remplacer :

```
pub(super) fn apply(select: Select<Entity>, filtre: &{@ entity @}Filter) -> Result<Select<Entity>> {
```

par :

```
{#- Les cent colonnes de `max_width` : au-delà, rustfmt pose un paramètre par ligne. -#}
{% set signature = "pub(super) fn apply(select: Select<Entity>, filtre: &" ~ entity ~ "Filter) -> Result<Select<Entity>> {" -%}
{% if signature | length <= 100 %}
{@ signature @}
{%- else %}
pub(super) fn apply(
    select: Select<Entity>,
    filtre: &{@ entity @}Filter,
) -> Result<Select<Entity>> {
{%- endif %}
```

- [ ] **Step 4 : lancer le garde, le voir passer**

Run : `cargo test -p rbs-cli --lib filter::tests::the_render_is_already_what_rustfmt_would_write`
Expected : PASS.

- [ ] **Step 5 : la suite du module reste verte**

Run : `cargo test -p rbs-cli --lib filter`
Expected : PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/templates/feature/filter.rs.jinja crates/rbs-cli/src/generate/filter.rs
git commit -m "fix(generate): écrit la signature du filtre dans la forme de rustfmt"
```

---

### Task 7 : `controller.rs.jinja` — le trou des noms très courts

**Files:**
- Modify: `crates/rbs-cli/templates/feature/controller.rs.jinja` (le handler `find`, et
  tout autre handler que la mesure désignera)
- Modify: `crates/rbs-cli/src/generate/controller.rs` (le garde existant, ligne 324)

**Interfaces:**
- Consumes: `bench::longueurs_divergentes` (Task 2), `controller::render(&Feature)`.
- Produces: le rendu de `controller.rs` point fixe sur [1, 23], divergent au-delà.

**Ce qui diverge, et pourquoi ce n'est pas au backlog :** le garde actuel balaie quatre noms
à partir de `tag`, et ne descend jamais en dessous de trois caractères. Or
`pub async fn find(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<{Entity}Response>> {`
vaut 98 caractères plus le nom de l'entité : à une ou deux lettres, la ligne tient sous les
cent colonnes de `max_width` et rustfmt la **compacte**, quand le gabarit l'écrit éclatée.
Une entité de deux lettres est un cas réel (`os`, `tv`). Ce trou n'est pas répertorié dans
`IMPROVE.md` ; il se corrige ici plutôt que d'ouvrir une entrée.

- [ ] **Step 1 : remplacer la liste de noms du garde par la mesure**

Dans `crates/rbs-cli/src/generate/controller.rs:324`, remplacer le corps du garde par :

```rust
        let divergentes = bench::longueurs_divergentes(|name| {
            let fields = fields::parse("title:string,views:int").expect("champs valides");
            render(&Feature::fresh(name, fields)).expect("rendu")
        });

        assert_eq!(
            divergentes,
            (24..=40).collect::<Vec<usize>>(),
            "la plage où le contrôleur diverge de rustfmt a bougé"
        );
```

Le doc-comment devient :

```rust
    /// Deux formes de ce fichier suivent le nom de l'entité et bornent le point fixe des
    /// deux côtés : la signature de `find`, que rustfmt compacte tant qu'elle tient sous
    /// les cent colonnes de `max_width` — soit jusqu'à deux caractères d'entité — et
    /// l'import des DTO, dont la ligne intérieure déborde à vingt-quatre.
    ///
    /// Au-delà de vingt-trois, rustfmt répartit les trois DTO par remplissage glouton, un
    /// régime que le gabarit ne sait pas écrire et qu'on ne réimplante pas : une montée de
    /// rustfmt le déplacerait. `format::format_batch` le rattrape à l'écriture, donc rien
    /// de mal formé n'atteint l'utilisateur. C'est cette frontière que l'intervalle
    /// ci-dessus fixe : elle est mesurée, et non commentée.
```

- [ ] **Step 2 : lancer le garde, le voir échouer**

Run : `cargo test -p rbs-cli --lib controller::tests::the_render_is_already_what_rustfmt_would_write`
Expected : FAIL — `divergentes` vaut `[1, 2, 24, …, 40]`, l'assertion attend `[24, …, 40]`.

- [ ] **Step 3 : conditionner la signature de `find`**

Dans `crates/rbs-cli/templates/feature/controller.rs.jinja`, remplacer la signature éclatée
de `find` :

```
pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<{@ entity @}Response>> {
```

par :

```
{#- Les cent colonnes de `max_width` : en deçà, rustfmt ramène la signature sur une ligne,
    ce qui n'arrive qu'aux entités d'une ou deux lettres. -#}
{% set signature_find = "pub async fn find(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<" ~ entity ~ "Response>> {" -%}
{% if signature_find | length <= 100 %}
{@ signature_find @}
{%- else %}
pub async fn find(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<{@ entity @}Response>> {
{%- endif %}
```

Attention : ce handler porte un `{%- if role %}` pour le garde de rôle. Si la variante
`role` change la signature, elle a son propre seuil — la mesure de l'étape 4 le dira, et le
même traitement s'y applique.

- [ ] **Step 4 : lancer le garde, lire ce qui reste**

Run : `cargo test -p rbs-cli --lib controller::tests::the_render_is_already_what_rustfmt_would_write`
Expected : PASS avec `divergentes == [24, …, 40]`. Si `1` ou `2` subsistent, un autre
handler porte la même bascule : lire la divergence et lui appliquer le même `{% set %}`.

- [ ] **Step 5 : le garde du contrôleur gardé reste vert**

Run : `cargo test -p rbs-cli --lib controller`
Expected : PASS — y compris
`the_dto_import_splits_itself_once_a_single_line_would_overflow` (`controller.rs:335`), qui
éprouve l'autre bascule du même fichier et ne doit pas bouger.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/templates/feature/controller.rs.jinja crates/rbs-cli/src/generate/controller.rs
git commit -m "fix(generate): compacte la signature de find sur une entité très courte"
```

---

### Task 8 : les gardes restants passent à la mesure

**Files:**
- Modify: `crates/rbs-cli/src/generate/dto.rs:239`
- Modify: `crates/rbs-cli/src/generate/service.rs:179`
- Modify: `crates/rbs-cli/src/generate/repository.rs:243`
- Modify: `crates/rbs-cli/src/generate/entity.rs:419`
- Modify: `crates/rbs-cli/src/generate/seed.rs:423`

**Interfaces:**
- Consumes: `bench::longueurs_divergentes` (Task 2).
- Produces: cinq gardes dont l'assertion est un intervalle mesuré. Aucune API nouvelle.

Ces cinq rendus n'ont **rien à corriger** : la mesure confirme `model.rs`, `dto.rs` et
`seed.rs` point fixe sur toute la plage, `service.rs` jusqu'à 23 et `repository.rs` jusqu'à
26. Ce que la tâche change, c'est que le seuil cesse de vivre dans une prose que rien ne
vérifie (tâches 107, 119, 120, 125) et devienne le chiffre qu'un test affiche quand il bouge.

La tâche 120 — `let ({module}, total) = repository::filter(db, filtre, pagination).await?;`
qui déborde à trente caractères de module — vit dans `service.rs`, donc **au-delà** du seuil
de 23 que ce garde fixe déjà. L'intervalle `(24..=40)` la couvre : elle est close par la
mesure, sans retouche de gabarit, et le doc-comment de l'étape 3 doit la nommer comme la
seconde ligne qui déborde après l'import des DTO.

- [ ] **Step 1 : convertir les trois gardes sans seuil**

Dans `dto.rs`, `entity.rs` et `seed.rs`, remplacer la boucle sur quatre noms par :

```rust
        let divergentes = bench::longueurs_divergentes(|name| {
            let fields = fields::parse("title:string,summary:text:optional,published_at:datetime")
                .expect("champs valides");
            render(&Feature::fresh(name, fields)).expect("rendu")
        });

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "ce rendu diverge de rustfmt à ces longueurs de nom"
        );
```

en adaptant l'appel de rendu à celui déjà employé par le garde du fichier (`seed::render`
prend un `Option<&str>` de nom de crate en second argument).

- [ ] **Step 2 : convertir les deux gardes à seuil**

Dans `service.rs`, l'assertion devient `(24..=40).collect::<Vec<usize>>()` ; dans
`repository.rs`, `(27..=40).collect::<Vec<usize>>()`. Le message d'échec nomme le
déplacement : `"la plage où le service diverge de rustfmt a bougé"`.

- [ ] **Step 3 : réécrire les deux doc-comments qui portaient le seuil en prose**

Le doc-comment de `service.rs:168-177` explique le seuil de 23 en quatre lignes de prose, et
celui de `repository.rs` fait de même pour 26. Les remplacer par la raison, l'intervalle
étant désormais dans le code :

```rust
    /// L'import des DTO borne le point fixe : sa ligne intérieure franchit les
    /// quatre-vingt-dix-huit colonnes d'un `use` à vingt-quatre caractères d'entité, et
    /// rustfmt passe alors au remplissage glouton — un régime qu'on ne réimplante pas, et
    /// que `format::format_batch` rattrape à l'écriture. L'intervalle asserté ci-dessous
    /// est cette frontière ; s'il bouge, le test affiche le nouveau.
```

et, pour `repository.rs` :

```rust
    /// L'appel que rend `list` borne le point fixe : `filter(db, &{Entity}Filter::default(),
    /// pagination).await` franchit les soixante colonnes de `fn_call_width` à vingt-sept
    /// caractères d'entité, et rustfmt éventaille alors ses arguments. Même frontière que
    /// pour le service, à un seuil différent — c'est pourquoi elle est mesurée et non
    /// commentée.
```

- [ ] **Step 4 : les cinq gardes passent**

Run : `cargo test -p rbs-cli --lib the_render_is_already_what_rustfmt_would_write`
Expected : PASS, neuf tests — les sept d'origine plus `migration` et `tests_http`.

- [ ] **Step 5 : éprouver l'axe du nom de champ**

La tâche 126 relève que le point fixe des DTO n'est mesuré que sur l'axe du nom d'entité.
Ajouter dans `dto.rs`, à côté du garde :

```rust
    /// L'axe qui finit par bouger n'est pas le nom de l'entité mais celui d'un champ : dans
    /// `impl From<Model>`, `{champ}: model.{champ},` croît deux fois plus vite que lui.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write_whatever_the_field_length() {
        let divergentes = bench::longueurs_divergentes(|champ| {
            let fields = fields::parse(&format!("{champ}:string")).expect("champs valides");
            render(&Feature::fresh("article", fields)).expect("rendu")
        });

        assert_eq!(
            divergentes,
            Vec::<usize>::new(),
            "le rendu du DTO diverge de rustfmt à ces longueurs de champ"
        );
    }
```

Run : `cargo test -p rbs-cli --lib the_render_is_already_what_rustfmt_would_write_whatever_the_field_length`

Si des longueurs apparaissent, corriger le gabarit par le même mécanisme ; si la divergence
tient à une ligne que rustfmt ne peut pas raccourcir (un champ de struct ne s'éclate pas),
asserter l'intervalle mesuré et l'expliquer dans le doc-comment plutôt que de forcer le
gabarit.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/src/generate/
git commit -m "test(generate): mesure les seuils de mise en forme au lieu de les commenter"
```

---

### Task 9 : le test global lit enfin le rendu brut

**Files:**
- Modify: `crates/rbs-cli/src/generate/command.rs:812-880`

**Interfaces:**
- Consumes: tous les gabarits corrigés (Tasks 3 à 7), `render` (`command.rs:482`).
- Produces: rien — c'est la tâche terminale.

- [ ] **Step 1 : faire lire `render()` au test**

Dans `crates/rbs-cli/src/generate/command.rs`, remplacer le corps du test
`the_render_goes_through_rustfmt_without_a_diff_whatever_the_name_length` : au lieu
d'appeler `run()` puis de relire les fichiers écrits, appeler directement `render`, qui rend
les fichiers **avant** `format::format_batch` :

```rust
        for (name, fields, complete) in cas {
            let fields = fields::parse(fields.unwrap_or_default()).expect("champs valides");
            let feature = Feature::fresh(name, fields);
            let seedable = seed::is_seedable(&feature);

            let (files, _migration) = render(&feature, *complete, seedable, Some("demo"))
                .expect("la génération doit aboutir");

            assert!(!files.is_empty(), "{name} n'a rien rendu");

            for (path, rendu) in &files {
                let formatted = bench::formatted(rendu);

                assert!(
                    formatted == *rendu,
                    "un `cargo fmt` chez l'utilisateur reformaterait {name}/{path}, \
                     qu'il n'a pas touché :\n{}",
                    divergence(rendu, &formatted)
                );
            }
        }
```

Le tuple `cas` reste celui d'aujourd'hui. `run()`, le `project()` temporaire et la relecture
disparaissent du test : rien n'a plus besoin d'être écrit sur le disque, ce qui le rend
aussi nettement plus rapide.

- [ ] **Step 2 : renommer le test et réécrire son doc-comment**

Le nom promettait un garant qu'il n'était pas. Le nouveau :

```rust
    /// La longueur d'un nom de feature est un continuum, et rustfmt bascule à des seuils
    /// que le gabarit doit connaître : `fn_call_width` à soixante colonnes sur les
    /// arguments d'un appel, `max_width` à cent sur une signature. Une forme écrite en dur
    /// n'est juste que pour les noms qui la font tomber du bon côté.
    ///
    /// Ce test lit le rendu **avant** `format::format_batch` : c'est la seule position d'où
    /// il puisse échouer. Lu après, il comparerait la sortie de rustfmt à elle-même.
    ///
    /// L'éventail s'arrête à `administrative_documents` — vingt-trois caractères de
    /// singulier — parce que c'est là que s'arrête le point fixe du contrôleur et du
    /// service. Au-delà, `format_batch` reprend la main, et ce sont les gardes de chaque
    /// module qui fixent la frontière, mesurée par `bench::longueurs_divergentes`.
    #[test]
    fn the_rendered_templates_are_already_what_rustfmt_would_write() {
```

- [ ] **Step 3 : lancer le test**

Run : `cargo test -p rbs-cli --lib the_rendered_templates_are_already_what_rustfmt_would_write`
Expected : PASS. Une divergence ici signale un gabarit que les tâches 3 à 7 ont manqué :
la lire, et la corriger dans le gabarit fautif plutôt que dans le test.

- [ ] **Step 4 : le module entier reste vert**

Run : `cargo test -p rbs-cli --lib generate::command`
Expected : PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/rbs-cli/src/generate/command.rs
git commit -m "test(generate): compare le rendu des gabarits avant leur mise en forme"
```

---

### Task 10 : vérification globale et fermeture

**Files:**
- Modify: `IMPROVE.md` (lignes 172, 191, 192, 202, 204, 205, 206)

- [ ] **Step 1 : la suite complète**

Run : `cargo test --workspace`
Expected : PASS, zéro échec. Relever le compte réellement affiché et le comparer au relevé
d'avant le lot (`787 + 77` au dernier scan) : il doit avoir monté des seuls tests ajoutés
ici. Ne pas prédire ce compte — le lire.

- [ ] **Step 2 : lint et mise en forme**

Run : `cargo clippy --workspace --all-targets -- -D warnings`
Run : `cargo fmt --all --check`
Expected : aucune sortie, exit 0 pour les deux.

- [ ] **Step 3 : la sortie livrée n'a pas bougé**

Run : `cargo test -p rbs-cli --test integration_examples -- --ignored --no-fail-fast`
Expected : PASS **sans avoir touché à `examples/`**. C'est la vérification qui compte :
`format_batch` reformatait déjà le rendu à l'écriture, donc rendre les gabarits point fixe
ne doit rien changer aux fichiers produits. Un diff ici veut dire qu'un gabarit a changé de
contenu et non de mise en forme — s'arrêter, et le rapporter.

Ce test exige Docker (`testcontainers` lance un PostgreSQL) ; sans lui, dire lequel n'a pas
pu être lancé plutôt que de le passer sous silence.

- [ ] **Step 4 : cocher dans `IMPROVE.md`**

Cocher les sept lignes du lot — 107, 119, 120, 122, 124, 125, 126 — chacune avec
` — Fait le 2026-09-02 : ` suivi de la preuve réellement exécutée (le compte de tests, la
plage assertée). Une ligne dont un critère n'est pas prouvé reste `- [ ]` avec une
annotation `PARTIEL` ou `BLOQUÉ`.

Mentionner sur la ligne 107 que le régime glouton reste non écrit **par décision**, et que ce
qui a changé est que la frontière est désormais mesurée.

Ne créer **aucune entrée nouvelle** : le trou des noms d'entité très courts, découvert en
Task 7, est corrigé dans ce lot et mentionné sur la ligne 122.

- [ ] **Step 5 : commit**

```bash
git add IMPROVE.md
git commit -m "docs(improve): coche les sept tâches de mise en forme des gabarits"
```

- [ ] **Step 6 : finir la branche**

Invoquer `superpowers:finishing-a-development-branch`, qui décide de l'intégration.
