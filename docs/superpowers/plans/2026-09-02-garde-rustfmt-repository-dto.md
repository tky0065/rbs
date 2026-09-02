# Garde `bench::formatted` sur `repository` et `dto` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Poser sur `generate::repository` et `generate::dto` le garde `bench::formatted`
que portent déjà `entity`, `filter`, `seed`, `controller` et `service`, et étendre le point
fixe de `repository.rs.jinja` jusqu'au nom d'entité le plus long du corpus de garde.

**Architecture:** Le garde compare le rendu du gabarit à ce que rustfmt en écrirait, sur un
éventail de noms d'entité. `repository.rs.jinja` reprend les deux macros que
`service.rs.jinja` porte déjà — `entete` pour la règle des cent colonnes, `chaine` pour
celle des soixante de `chain_width` — et les applique aux deux seules constructions du
fichier dont la forme suit le nom de l'entité. `dto.rs.jinja` n'est pas touché : la mesure
le donne déjà point fixe sur toute la plage.

**Tech Stack:** Rust, minijinja (délimiteurs alternatifs `{@ @}` / `{% %}`), rustfmt
édition 2024, `cargo test -p rbs-cli --lib`.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` (frontière noyau / généré) ;
précédent direct : `crates/rbs-cli/templates/feature/service.rs.jinja` et le doc-comment de
`crates/rbs-cli/src/generate/service.rs:180-189`.

## Global Constraints

- Les gabarits vivent dans `crates/rbs-cli/templates/` ; minijinja y emploie `{@ … @}` pour
  l'interpolation et `{% … %}` pour les blocs.
- `Renderer::render` appelle `render_str` : **aucun chargeur de gabarits**, donc aucun
  `{% import %}` entre fichiers. Les macros partagées se recopient, comme le fait déjà
  `service.rs.jinja`.
- Corpus de noms du garde, identique à celui de `controller` et `service` :
  `["tag", "articles", "administrative_documents", "organizational_structures"]`.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- Commits en Conventional Commits, sujet en français à l'impératif, sans majuscule ni point
  final ; aucun identifiant de tâche, aucun renvoi à un fichier de suivi, aucune ligne
  `Co-Authored-By`.
- Branche : `fix/garde-rustfmt-repository-dto`. Jamais `main`.

## Mesure préalable (déjà exécutée, elle fonde le plan)

Balayage de `bench::formatted` contre le rendu, nom d'entité de 2 à 40 caractères.

`repository.rs.jinja` — trois seuils, dans cet ordre :

| Seuil | Construction | Règle rustfmt |
|---|---|---|
| `singular` ≥ 13 | `{singular}.insert(db).await.map_err(conflict_on_duplicate)` et son jumeau `.update(db)` — 65 colonnes, dont 61 de chaîne | `chain_width` = 60 |
| `singular` ≥ 23 | `pub async fn create(db: &DatabaseConnection, {singular}: ActiveModel) -> Result<Model> {` — 101 colonnes | `max_width` = 100 |
| `entity` ≥ 27 | `filter(db, &{entity}Filter::default(), pagination).await` — rustfmt éventaille les arguments de l'appel *et* détache le `.await` | hors périmètre |

`dto.rs.jinja` — **point fixe sur toute la plage 2..40 du nom d'entité.** Aucune de ses
lignes ne grandit assez : la plus longue, `impl From<Model> for {entity}Response {`, vaut
31 + `entity`. Le seul axe qui la fasse bouger est le nom d'un *champ* : à 40 caractères,
`            {champ}: model.{champ},` atteint 101 colonnes. C'est un autre axe que celui de
la tâche, et il reste noté comme constat.

**Borne retenue pour `repository`** : les deux premiers seuils sont corrigés, le troisième
non. Motif : les deux premiers sont des éclatements verticaux que le gabarit sait écrire, et
le corpus de garde les traverse (`organizational_structures` donne `singular` = 24). Le
troisième réimplanterait dans le gabarit un arbitrage d'appel dont la constante — 79
colonnes, ni `max_width` ni `chain_width` — n'est adossée à aucun réglage nommé et qu'une
montée de rustfmt peut déplacer. `format::format_batch` le rattrape à l'écriture.

---

## Structure des fichiers

| Fichier | Responsabilité | Action |
|---|---|---|
| `crates/rbs-cli/src/generate/dto.rs` | rendu des DTO + ses tests | Modifier : ajouter le garde |
| `crates/rbs-cli/src/generate/repository.rs` | rendu du repository + ses tests | Modifier : ajouter le garde |
| `crates/rbs-cli/templates/feature/repository.rs.jinja` | gabarit du repository | Modifier : deux macros, deux points d'application |

---

### Task 1 : le garde de `dto`, sans correction de gabarit

La mesure donne `dto.rs.jinja` point fixe sur toute la plage. Le garde se pose donc seul :
il n'est pas rouge d'abord, il constate — et il rendra rouge toute retouche future du
gabarit qui casserait la propriété.

**Files:**
- Modify: `crates/rbs-cli/src/generate/dto.rs` (ajout dans `mod tests`, avant
  `the_generated_dtos_compile_in_a_fresh_project`)

**Interfaces:**
- Consumes: `bench::formatted(&str) -> String` (déjà importé par
  `use crate::generate::{bench, entity, fields};` en tête du module de tests) ; l'aide
  locale `dto(name: &str, fields: &str) -> String`.
- Produces: rien que d'autres tâches consomment.

- [ ] **Step 1 : écrire le garde**

Insérer dans `mod tests` de `crates/rbs-cli/src/generate/dto.rs`, juste avant
`#[test] #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_generated_dtos_compile_in_a_fresh_project()` :

```rust
    /// Aucune ligne de ce fichier ne suit le nom de l'entité d'assez près pour franchir un
    /// seuil de rustfmt : la plus longue, `impl From<Model> for …Response {`, vaut trente
    /// et un caractères de plus que lui, et le balayage de deux à quarante ne trouve pas
    /// une divergence. Les quatre noms n'y cherchent donc pas un seuil — ils tiennent la
    /// propriété pour la prochaine retouche du gabarit.
    ///
    /// L'axe qui, lui, finit par bouger est le nom d'un champ, pas celui de l'entité : à
    /// quarante caractères, `<champ>: model.<champ>,` atteint 101 colonnes dans
    /// `From<Model>`. `format::format_batch` le rattrape à l'écriture.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        for name in [
            "tag",
            "articles",
            "administrative_documents",
            "organizational_structures",
        ] {
            let rendered = dto(name, "title:string,summary:text:optional,published_at:datetime");

            assert_eq!(
                bench::formatted(&rendered),
                rendered,
                "le rendu de `{name}` diverge de rustfmt"
            );
        }
    }
```

- [ ] **Step 2 : lancer le garde**

```bash
cargo test -p rbs-cli --lib generate::dto::tests::the_render_is_already_what_rustfmt_would_write -- --exact
```

Attendu : `test result: ok. 1 passed`. Le garde constate un point fixe déjà acquis ; s'il
échoue, la mesure préalable était fausse — arrêter et rapporter, ne pas corriger le gabarit
au jugé.

- [ ] **Step 3 : `cargo fmt` sur le test lui-même**

```bash
cargo fmt --all
cargo fmt --all --check
```

Attendu : sortie vide. Si `cargo fmt` a bougé la ligne du `let rendered`, garder ce qu'il
écrit.

- [ ] **Step 4 : commit**

```bash
git add crates/rbs-cli/src/generate/dto.rs
git commit -m "test(generate): compare le rendu des DTO à ce que rustfmt écrirait"
```

---

### Task 2 : le garde de `repository`, vu rouge

Le garde est écrit avant la correction du gabarit, et il doit être **vu rouge** sur
`administrative_documents`. Le diff qu'il affiche est la preuve du seuil.

**Files:**
- Modify: `crates/rbs-cli/src/generate/repository.rs` (ajout dans `mod tests`, avant
  `the_generated_repository_compiles_in_a_fresh_project`)

**Interfaces:**
- Consumes: `bench::formatted` (déjà importé par
  `use crate::generate::{bench, entity, fields, filter};`) ; l'aide locale
  `repository(name: &str, fields: &str) -> String`.
- Produces: le test
  `generate::repository::tests::the_render_is_already_what_rustfmt_would_write`, que la
  tâche 3 doit faire passer au vert.

- [ ] **Step 1 : écrire le garde**

Insérer dans `mod tests` de `crates/rbs-cli/src/generate/repository.rs`, juste avant
`#[test] #[ignore = "compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn the_generated_repository_compiles_in_a_fresh_project()` :

```rust
    /// Deux formes de ce fichier suivent le nom de l'entité, et chacune bascule à sa propre
    /// longueur : les chaînes `…insert(db).await.map_err(…)` aux 60 colonnes de
    /// `chain_width`, dès treize caractères de singulier ; les signatures de `create` et
    /// `update` aux 100 de `max_width`, dès vingt-trois. Un seul nom ne prouverait donc
    /// rien : les quatre balaient la plage où les deux seuils se franchissent.
    ///
    /// Les noms montent jusqu'à `organizational_structures`, dont le singulier fait
    /// vingt-quatre caractères. Au-delà de vingt-six pour l'entité, rustfmt éventaille les
    /// arguments de l'appel `filter(db, &…Filter::default(), pagination).await` que rend
    /// `list` — un arbitrage dont la constante ne porte le nom d'aucun réglage, que le
    /// gabarit ne devine pas et que `format::format_batch` rattrape à l'écriture.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        for name in [
            "tag",
            "articles",
            "administrative_documents",
            "organizational_structures",
        ] {
            let rendered = repository(name, "title:string,email:string:unique");

            assert_eq!(
                bench::formatted(&rendered),
                rendered,
                "le rendu de `{name}` diverge de rustfmt"
            );
        }
    }
```

- [ ] **Step 2 : lancer le garde et le voir rouge**

```bash
cargo test -p rbs-cli --lib generate::repository::tests::the_render_is_already_what_rustfmt_would_write -- --exact
```

Attendu : `FAILED`, avec `le rendu de \`administrative_documents\` diverge de rustfmt` et,
dans le diff de `assert_eq!`, la ligne rendue
`    administrative_document.insert(db).await.map_err(conflict_on_duplicate)` face à
l'éclatement en quatre lignes qu'écrit rustfmt. **Recopier ce diff dans le rapport** : c'est
la preuve du seuil.

- [ ] **Step 3 : ne rien commiter encore**

Le rouge n'est pas un état à figer dans l'historique : la tâche 3 le referme dans le même
commit.

---

### Task 3 : les deux macros de largeur dans `repository.rs.jinja`

**Files:**
- Modify: `crates/rbs-cli/templates/feature/repository.rs.jinja`

**Interfaces:**
- Consumes: le test rouge de la tâche 2.
- Produces: un gabarit point fixe jusqu'à `singular` = 24 / `entity` = 23.

- [ ] **Step 1 : poser les deux macros en tête du gabarit**

Insérer **avant** la première ligne actuelle (`use rbs_core::{Error, Pagination, Result};`)
le bloc suivant. Il est repris de `service.rs.jinja:1-32` — `Renderer::render` appelant
`render_str`, aucun `{% import %}` n'est possible entre deux gabarits.

```jinja
{# Deux formes de ce fichier grandissent avec le nom de l'entité, et rustfmt les replie
    chacune à son propre seuil. La template les écrit donc telles que rustfmt les écrirait,
    plutôt que de laisser la mise en forme rattraper ce qu'elle aurait mal posé.

    `entete` pose une signature : rustfmt la garde sur une ligne tant que celle-ci tient
    dans ses cent colonnes, et l'éclate un paramètre par ligne au-delà. -#}
{% macro entete(nom, parametres, retour) -%}
{% set une_ligne = "pub async fn " ~ nom ~ "(" ~ (parametres | join(", ")) ~ ") -> " ~ retour ~ " {" -%}
{% if une_ligne | length <= 100 -%}
{@ une_ligne @}
{%- else -%}
pub async fn {@ nom @}(
{%- for parametre in parametres %}
    {@ parametre @},
{%- endfor %}
) -> {@ retour @} {
{%- endif %}
{%- endmacro -%}
{# `chaine` pose un receveur suivi de ses maillons. Le seuil n'est plus la largeur de la
    ligne mais `chain_width`, soit soixante colonnes : rustfmt éclate une chaîne bien avant
    qu'elle ne déborde.

    Au-delà de vingt-six caractères d'entité, une troisième forme bouge — l'appel
    `filter(db, &…Filter::default(), pagination).await` de `list`, dont rustfmt éventaille
    les arguments à un seuil qui ne porte le nom d'aucun réglage. La template s'arrête ici :
    `format::format_batch` reprend la main à l'écriture. -#}
{% macro chaine(receveur, maillons, indentation) -%}
{% set une_ligne = receveur ~ (maillons | join("")) -%}
{% if une_ligne | length <= 60 -%}
{@ une_ligne @}
{%- else -%}
{@ receveur @}
{%- for maillon in maillons %}
{@ indentation @}    {@ maillon @}
{%- endfor %}
{%- endif %}
{%- endmacro -%}
```

- [ ] **Step 2 : appliquer les macros à `create` et `update`**

Remplacer dans `crates/rbs-cli/templates/feature/repository.rs.jinja` le bloc :

```jinja
pub async fn create(db: &DatabaseConnection, {@ singular @}: ActiveModel) -> Result<Model> {
    {@ singular @}.insert(db).await.map_err(conflict_on_duplicate)
}

pub async fn update(db: &DatabaseConnection, {@ singular @}: ActiveModel) -> Result<Model> {
    {@ singular @}.update(db).await.map_err(conflict_on_duplicate)
}
```

par :

```jinja
{@ entete("create", ["db: &DatabaseConnection", singular ~ ": ActiveModel"], "Result<Model>") @}
    {@ chaine(singular, [".insert(db)", ".await", ".map_err(conflict_on_duplicate)"], "    ") @}
}

{@ entete("update", ["db: &DatabaseConnection", singular ~ ": ActiveModel"], "Result<Model>") @}
    {@ chaine(singular, [".update(db)", ".await", ".map_err(conflict_on_duplicate)"], "    ") @}
}
```

Les quatre autres signatures — `list`, `filter`, `find`, `delete` — restent écrites en
clair : leur forme ne suit pas le nom de l'entité. `list`, `find` et `delete` n'en portent
pas trace, et celle de `filter` dépasse les cent colonnes quel que soit ce nom : elle est
déjà éclatée en dur et le restera.

- [ ] **Step 3 : lancer le garde et le voir vert**

```bash
cargo test -p rbs-cli --lib generate::repository::tests::the_render_is_already_what_rustfmt_would_write -- --exact
```

Attendu : `test result: ok. 1 passed`.

- [ ] **Step 4 : vérifier que les tests voisins de `repository` tiennent**

Le gabarit vient de bouger : les assertions littérales de `mod tests` — notamment
`the_creation_and_the_update_share_the_same_translation`, qui cherche
`.insert(db).await.map_err(conflict_on_duplicate)`, et
`the_repository_exposes_the_five_crud_operations`, qui cherche `pub async fn create(` —
doivent toujours passer, `article` restant sous les deux seuils.

```bash
cargo test -p rbs-cli --lib generate::repository
```

Attendu : tous verts. Si l'un d'eux casse, le gabarit rend autre chose que ce qu'il rendait
sur un nom court : c'est une régression, pas un test à réécrire.

- [ ] **Step 5 : vérifier que le rendu d'un nom court n'a pas bougé d'un octet**

```bash
cargo test -p rbs-cli --lib
```

Attendu : aucun échec. C'est la suite qui porte les littéraux de rendu.

- [ ] **Step 6 : commit du rouge refermé**

```bash
git add crates/rbs-cli/src/generate/repository.rs crates/rbs-cli/templates/feature/repository.rs.jinja
git commit -m "fix(generate): étend le point fixe du repository aux noms d'entité longs"
```

---

### Task 4 : non-dérive des exemples

Le gabarit a bougé. Les quatre projets d'`examples/` n'emploient que des noms courts —
`article` (7), `post` (4), `upload` (6), `subscriber` (10), tous sous le seuil de treize —
donc le rendu doit être **identique à l'octet**. Le test de non-dérive l'établit ; il n'y a
pas de régénération à faire si et seulement s'il passe.

**Files:**
- Aucun, si le test passe.

**Interfaces:**
- Consumes: le gabarit corrigé par la tâche 3.

- [ ] **Step 1 : lancer le test de non-dérive**

```bash
cargo test -p rbs-cli --test integration_examples
```

Attendu : vert, sans qu'aucun fichier d'`examples/` n'ait été touché. Le test régénère les
quatre projets et les compare octet à octet au versionné.

- [ ] **Step 2 : si et seulement s'il échoue, régénérer par diff**

Ne **jamais** écraser `examples/` : ces projets portent des éditions à la main. Générer dans
un répertoire jetable avec le gabarit d'avant, puis avec celui d'après, differ les deux
générations, et n'appliquer au versionné que ce diff — la marche à suivre est celle
d'`examples/README.md`. Puis relancer le test.

- [ ] **Step 3 : vérification finale du workspace**

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Attendu : trois sorties sans échec ni avertissement. **Ne pas** lancer
`-- --ignored` : la suite Docker est réservée à l'intégration, et un `target/` partagé sous
exécutions concurrentes la rend instable.

- [ ] **Step 4 : commit, s'il y a quelque chose à commiter**

Si le test de non-dérive est passé sans toucher `examples/`, il n'y a rien à commiter : la
tâche est une vérification.

---

## Ce qui reste hors périmètre, et pourquoi

- **Le troisième seuil de `repository`** (`entity` ≥ 27, l'appel `filter(…).await` de
  `list`). Le corriger demanderait au gabarit de rejouer un arbitrage d'appel dont la
  constante n'est adossée à aucun réglage nommé de rustfmt.
- **Le quatrième** (`module` ≥ 37, `let ({module}, total) = tokio::try_join!(…)?;` à 101
  colonnes) : même famille, encore plus loin de la plage utile.
- **L'axe « nom de champ » de `dto`** (≥ 40 caractères, la ligne `{champ}: model.{champ},`
  de `From<Model>`). Autre axe que celui mesuré ici ; à relever comme constat, pas à
  corriger sous couvert de cette tâche.

## Le code que rien ne compile

Le gabarit `repository.rs.jinja` dépose du code Rust qu'aucun test rapide ne compile : seuls
les tests `#[ignore]` sous Docker le font. La correction est **une mise en forme**, à
sémantique constante — les mêmes jetons, répartis autrement sur les lignes. C'est ce qui
rend acceptable de ne pas la compiler ici ; le dire dans le rapport plutôt que de supposer.
