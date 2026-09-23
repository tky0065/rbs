# Les références dans les écrans d'administration — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** dans les écrans que `rbs generate crud` émet, une référence se choisit dans un sélecteur à recherche et se lit par un libellé, au lieu d'un UUID ; `auth` expose les comptes par `POST /users/filter`, réservé aux admins.

**Architecture :** `rbs-core` gagne `in` sur `Comparison<T>`, que le `filter.rs` engendré applique — une page résout ses libellés en un appel. Le CLI calcule, au plan, une fiche par référence (méthode de filtre de la cible, colonne libellé devinée ou forcée par `label=`) et la passe à l'écran. Le patron rend un sélecteur (`ChoixReference.vue`) et une résolution (`libelles.ts`), deux fichiers génériques déposés par `frontend-admin`, et par `generate crud` s'ils manquent.

**Tech Stack :** Rust (serde, utoipa, SeaORM, axum), minijinja, Vue 3 + TypeScript, `assert_cmd`, Docusaurus.

**Spec :** `docs/superpowers/specs/2026-09-23-references-dans-les-ecrans-design.md`

## Global Constraints

- Branche `feat/references-dans-les-ecrans` ; ne jamais committer sur `main` ; ne créer aucune autre branche.
- Conventional Commits, sujet en français à l'impératif, sans majuscule ni point final ; corps avec `Vérifications :` et les résultats réels. **Jamais** `Co-Authored-By`, `Claude-Session`, identifiant de tâche, ni mention d'un plan, d'un lot, de `TODO.md`/`ROADMAP.md`.
- Un commentaire dit le *pourquoi*. Les items publics de `rbs-core` portent un `///` d'une à trois lignes.
- Messages du CLI en français uniquement (c'est l'usage de `fields/error.rs` et de `lib.rs`) ; textes des écrans dans la langue du projet (`fr`/`en`), via `dans(lang, fr, en)` côté Rust et `{% if lang == 'en' %}` côté fragment.
- Aucune ancre nouvelle ; aucun fragment nouveau ; aucun workflow CI modifié.
- Le patron d'écran reste la seule template d'écran (ADR-0003) ; l'écran de démonstration doit sortir **octet pour octet identique**.
- Documentation bilingue, fr et en dans le même commit.
- Tests Docker : toujours `--no-fail-fast`, sortie redirigée dans `$SCRATCH` (le scratchpad de session, `export SCRATCH=<scratchpad>`). Un seul job par suite lente (limite de 600 s du shell).
- Régénérer `examples/` **par diff entre deux générations**, jamais par écrasement : les exemples portent des retouches à la main.
- Shell zsh : `command ls`, pas `ls`.
- Un seul push, à la fin, et seulement sur demande du mainteneur.

## Review Focus

1. **Un filtre `in` sur un serveur qui ne le connaît pas** — un opérateur inconnu est ignoré en silence ; la résolution doit alors rendre des libellés *manquants* (identifiant raccourci), jamais un libellé faux. Épinglé en tâche 6 par le test `resoudre_ne_garde_que_les_identifiants_demandes` (le rendu du `libelles.ts` filtre les réponses sur les identifiants demandés).
2. **Un compte `user` qui ouvre un écran portant une référence vers `users`** — 403 sur la résolution et sur la recherche ; la table s'affiche, le sélecteur dit « réservé aux administrateurs ». Épinglé en tâche 6 par le rendu testé du composant (branche `refuse`) et en tâche 7 par `a_user_listing_accounts_gets_403`.
3. **Une référence facultative remise à vide** — le sélecteur doit pouvoir envoyer `null`. Épinglé en tâche 6 par `an_optional_reference_offers_to_clear_it` et, en compilation réelle, par la colonne `auteur:references:users:optional` de la tâche 8.
4. **Une auto-référence** (`parent:references:categories` dans `categories`) — la cible n'est pas encore sur le disque ; la fiche doit se construire depuis les champs de la feature en cours. Épinglé en tâche 5 par `a_self_reference_takes_its_label_from_the_feature_being_generated`.
5. **Un `label=` mal orthographié sur un projet sans frontend** — refusé quand même. Épinglé en tâche 5 par `an_unknown_label_is_refused_even_without_the_admin_shell`.

---

## Carte des fichiers

| Fichier | Rôle | Tâche |
|---|---|---|
| `crates/rbs-core/src/filter/mod.rs`, `…/schema.rs`, `crates/rbs-core/Cargo.toml` | `in` sur `Comparison`, schémas, lint déclassé | 1 |
| `crates/rbs-cli/templates/feature/filter.rs.jinja` | `compare()` applique `in` | 2 |
| `examples/*/src/*/filter.rs` | régénérés | 2 |
| `crates/rbs-cli/src/generate/command.rs`, `crates/rbs-cli/notes/1.10.0.md` | refus d'un noyau antérieur, note | 3 |
| `crates/rbs-cli/src/generate/fields.rs`, `…/fields/error.rs` | grammaire `label=` | 4 |
| `crates/rbs-cli/src/generate/reference.rs` (nouveau), `…/generate/mod.rs`, `…/command.rs`, `crates/rbs-cli/src/lib.rs` | fiche de référence, refus et replis | 5 |
| `crates/rbs-cli/src/ecran.rs`, `…/frontend-admin/client/src/admin/vues/Patron.vue.jinja`, `…/frontend-admin/client/src/admin/references/{ChoixReference.vue,libelles.ts}.jinja` (nouveaux), `…/frontend-admin/feature.toml` | l'écran | 6 |
| `crates/rbs-cli/templates/features/auth/{dto.rs,repository/user.rs,service/account.rs,controller/account.rs,mod.rs,feature.toml,tests/*}.jinja` | la route des comptes | 7 |
| `crates/rbs-cli/tests/integration_frontend.rs` | compilation réelle d'un écran à références | 8 |
| `examples/{blog-auth,event-hub,admin-console,help-desk}/**`, `examples/README*.md`, `crates/rbs-cli/tests/integration_examples.rs` | exemples | 9 |
| `docs/docs/**`, `docs/i18n/fr/**`, `CHANGELOG.md` | documentation, tutoriel | 10 |

---

### Task 1 : `in` sur `Comparison<T>` dans `rbs-core`

**Files :**
- Modify : `crates/rbs-core/src/filter/mod.rs:25-37` (struct), `:107-114` (`ComparisonInput`), `:131-154` (`Deserialize`), module `tests`
- Modify : `crates/rbs-core/src/filter/schema.rs` (macro `comparaison_documentee`, test `the_second_form_names_the_operators`)
- Modify : `crates/rbs-core/Cargo.toml:41` (lints)

**Interfaces :**
- Produces : `Comparison<T>::r#in: Option<Vec<T>>` ; JSON `{ "in": [...] }` accepté sur toute colonne comparable ; propriété `in` dans chaque `*ComparisonOperators`.

- [ ] **Step 1 : tests rouges** — dans `mod.rs`, module `tests`, après `an_object_names_its_operators` :

```rust
    /// `in` vaut sur une colonne comparable comme sur une énumération : c'est ce qui laisse
    /// un écran résoudre les identifiants d'une page entière en un seul appel.
    #[test]
    fn a_comparison_object_names_in() {
        let compare: Comparison<i32> =
            serde_json::from_str(r#"{"in": [1, 2]}"#).expect("objet lisible");

        assert_eq!(compare.r#in, Some(vec![1, 2]));
        assert_eq!(compare.eq, None);
    }

    /// Une liste vide reste une liste : le filtre engendré n'accepte alors aucune ligne,
    /// là où `None` n'en écarterait aucune.
    #[test]
    fn an_empty_in_list_on_a_comparison_stays_an_empty_list() {
        let compare: Comparison<i32> = serde_json::from_str(r#"{"in": []}"#).expect("objet lisible");

        assert_eq!(compare.r#in, Some(Vec::new()));
    }

    #[test]
    fn a_bare_comparison_value_carries_no_in_list() {
        let compare: Comparison<i32> = serde_json::from_str("7").expect("valeur nue lisible");

        assert_eq!(compare.r#in, None);
    }
```

Dans `schema.rs`, test `the_second_form_names_the_operators` : ajouter `"in"` aux cinq listes à six opérateurs (Bool, DateTime, Date, Decimal) et ajouter deux entrées :

```rust
            (
                schema::<UuidComparisonOperators>(),
                vec!["eq", "gt", "gte", "lt", "lte", "in", "is_null"],
            ),
            (
                schema::<IntComparisonOperators>(),
                vec!["eq", "gt", "gte", "lt", "lte", "in", "is_null"],
            ),
```

Run : `cargo test -p rbs-core --all-features filter:: 2>&1 | tail -15`
Expected : FAIL de compilation (`no field r#in on type Comparison<i32>`) — c'est le rouge.

- [ ] **Step 2 : implémentation**

`mod.rs`, dans `Comparison<T>` après `lte` :

```rust
    /// Appartenance à l'une des valeurs citées. Une liste vide n'en accepte aucune.
    pub r#in: Option<Vec<T>>,
```

Dans `ComparisonInput<T>` après `lte` : `r#in: Option<Vec<T>>,`. Dans `impl Deserialize for Comparison<T>` : `r#in: None,` dans le bras `Bare`, `r#in: operateurs.r#in,` dans le bras `Operators`.

`schema.rs`, macro `comparaison_documentee`, dans `$operateurs` après `lte` :

```rust
            /// Appartenance à l'une des valeurs citées. Une liste vide n'en accepte aucune.
            pub r#in: Option<Vec<$valeur>>,
```

`Cargo.toml`, dans `[package.metadata.cargo-semver-checks.lints]`, en tête :

```toml
# `Comparison<T>` gagne `in`. Un champ public ajouté rompt la construction par littéral
# sans `..Default::default()` ; le type ne se construit que par sa désérialisation, dans le
# `filter.rs` engendré, et aucun gabarit ni exemple ne l'écrit en littéral. La note 1.10.0
# le dit. À rétablir en `deny` au prochain champ ajouté.
constructible_struct_adds_field = "allow"
```

- [ ] **Step 3 : vert**

Run : `cargo test -p rbs-core --all-features 2>&1 | grep "test result"` puis `cargo clippy -p rbs-core --all-features --all-targets -- -D warnings` et `cargo fmt --all --check`
Expected : tout vert. Vérifier aussi `grep -rn "Comparison {" crates examples | grep -v "pub struct\|impl"` → aucune construction littérale ailleurs.

- [ ] **Step 4 : commit**

```bash
git add crates/rbs-core
git commit -F - <<'EOF'
feat(core): accepte l'opérateur in sur une colonne comparable

Seules les énumérations connaissaient `in`, et un opérateur inconnu est
ignoré en silence : un client qui voulait les lignes de vingt
identifiants recevait la table entière. `Comparison<T>` le lit
désormais, et les schémas OpenAPI le décrivent.

Vérifications :
- tests neufs rouges (champ absent) puis verts
- cargo test -p rbs-core --all-features : <résultat réel>
- cargo clippy -p rbs-core --all-features --all-targets -- -D warnings : <résultat réel>
EOF
```

---

### Task 2 : le `filter.rs` engendré applique `in`

**Files :**
- Modify : `crates/rbs-cli/templates/feature/filter.rs.jinja` (fonction `compare`)
- Modify : `examples/*/src/*/filter.rs` (tous les `filter.rs` engendrés)
- Test : `crates/rbs-cli/src/generate/command.rs` ou le module de tests qui rend `filter.rs` (`grep -rn "fn compare<T" crates/rbs-cli/src`)

**Interfaces :**
- Consumes : `Comparison<T>::r#in` (tâche 1).
- Produces : `POST /<table>/filter { "id": { "in": [...] } }` filtre réellement.

- [ ] **Step 1 : test rouge** — repérer le test qui rend `filter.rs` (`grep -rn "filter.rs" crates/rbs-cli/src/generate/*.rs | grep -i test`) ; y ajouter :

```rust
    /// `in` est lu par le noyau : un `compare()` qui l'ignorerait rendrait la table entière
    /// à qui demande vingt identifiants.
    #[test]
    fn the_generated_compare_applies_in() {
        let rendu = filter_rendu("articles", "title:string,views:int");

        assert!(
            rendu.contains(".add_option(compare.r#in.clone().map(|valeurs| colonne.is_in(valeurs)))"),
            "{rendu}"
        );
    }
```

(`filter_rendu` : le helper de rendu existant dans ce module ; s'il n'y en a pas, rendre via `fichiers(&Feature::fresh(..), true)` et prendre le fichier `src/<nom>/filter.rs`.)

Run : `cargo test -p rbs-cli --lib the_generated_compare_applies_in`
Expected : FAIL.

- [ ] **Step 2 : implémentation** — dans `filter.rs.jinja`, fonction `compare`, après la ligne `lte` :

```
        .add_option(compare.r#in.clone().map(|valeurs| colonne.is_in(valeurs)))
```

Run : le test → PASS ; `cargo test -p rbs-cli --lib generate:: 2>&1 | grep "test result"` → vert (le test de rustfmt sur les rendus y compris).

- [ ] **Step 3 : les exemples suivent** — chaque `filter.rs` d'exemple est engendré sans retouche : y ajouter la même ligne au même endroit.

```bash
for f in $(grep -rl "fn compare<T: Into<Value> + Clone>" examples --include=filter.rs); do
  perl -0pi -e 's/(        \.add_option\(compare\.lte\.clone\(\)\.map\(\|valeur\| colonne\.lte\(valeur\)\)\)\n)/$1        .add_option(compare.r#in.clone().map(|valeurs| colonne.is_in(valeurs)))\n/' "$f"
done
git diff --stat examples
```

Run : `cargo test -p rbs-cli --test integration_examples --no-fail-fast > $SCRATCH/t2.log 2>&1; grep "test result\|FAILED" $SCRATCH/t2.log`
Expected : vert (26/26). Puis, pour un exemple : `cd examples/help-desk && cargo check -q` → vert.

- [ ] **Step 4 : commit** — `feat(generate): applique l'opérateur in dans le filtre engendré`, corps : pourquoi, et `Vérifications :` (test rouge→vert, `--lib`, `integration_examples`, `cargo check` d'un exemple).

---

### Task 3 : refus d'un noyau antérieur, et note 1.10.0

**Files :**
- Modify : `crates/rbs-cli/src/generate/command.rs` (enum `Error`, `plan_for`, tests)
- Create : `crates/rbs-cli/notes/1.10.0.md`

**Interfaces :**
- Consumes : `crate::upgrade::nombres(&str) -> Option<[u64; 3]>` ; `metadonnees.version` (la version rbs du projet).
- Produces : `Error::NoyauAnterieur { projet: String }`, code `noyau_anterieur`.

- [ ] **Step 1 : tests rouges** — dans les tests de `command.rs` :

```rust
    /// Un `filter.rs` d'aujourd'hui lit `in`, que le noyau n'a qu'à partir de 1.10.0 : le
    /// rendre sur un noyau antérieur laisserait un projet qui ne compile plus.
    #[test]
    fn a_core_older_than_the_in_operator_is_refused_once_the_cli_ships_it() {
        assert!(noyau_anterieur("1.9.0", "1.10.0"));
        assert!(!noyau_anterieur("1.10.0", "1.10.0"));
        assert!(!noyau_anterieur("1.11.2", "1.12.0"));
    }

    /// Tant que le workspace n'est pas monté en 1.10.0, les projets qu'il crée portent
    /// l'ancien numéro : la garde doit dormir, sans quoi aucun exemple ne se régénère.
    #[test]
    fn the_guard_sleeps_while_the_cli_itself_is_older() {
        assert!(!noyau_anterieur("1.9.0", "1.9.0"));
    }

    #[test]
    fn an_unreadable_version_is_not_taken_for_an_older_one() {
        assert!(!noyau_anterieur("inconnue", "1.10.0"));
    }
```

Run : `cargo test -p rbs-cli --lib noyau_anterieur` → FAIL (fonction absente).

- [ ] **Step 2 : implémentation** — dans `command.rs` :

```rust
/// La version de `rbs-core` qui apporte `in` sur les colonnes comparables.
const NOYAU_IN: [u64; 3] = [1, 10, 0];

/// Le projet porte-t-il un noyau antérieur à `in`, alors que ce CLI l'engendre ?
///
/// La seconde condition endort la garde tant que le workspace n'a pas été monté : les
/// projets qu'il crée d'ici là portent encore l'ancien numéro. Une version illisible
/// n'est pas tenue pour antérieure — mieux vaut un projet qui échoue à compiler qu'un
/// refus à tort.
fn noyau_anterieur(projet: &str, cli: &str) -> bool {
    let (Some(projet), Some(cli)) = (crate::upgrade::nombres(projet), crate::upgrade::nombres(cli))
    else {
        return false;
    };

    cli >= NOYAU_IN && projet < NOYAU_IN
}
```

Variante d'`Error` (à côté des autres, message et remède en français) :

```rust
    /// Le noyau du projet précède l'opérateur `in` que le filtre engendré emploie.
    #[error(
        "le projet est en rbs {projet} : le filtre engendré emploie l'opérateur `in`, que \
         rbs-core n'a qu'à partir de 1.10.0"
    )]
    NoyauAnterieur {
        /// La version du projet.
        projet: String,
    },
```

Dans `code()` : `Error::NoyauAnterieur { .. } => "noyau_anterieur",` ; dans `remede()` : `Error::NoyauAnterieur { .. } => Some("lancez `rbs upgrade`, puis relancez la commande".to_string()),` (suivre la forme des bras voisins). Dans `plan_for`, juste après la lecture de `metadonnees` :

```rust
    if noyau_anterieur(&metadonnees.version, env!("CARGO_PKG_VERSION")) {
        return Err(Error::NoyauAnterieur {
            projet: metadonnees.version.clone(),
        });
    }
```

Run : les trois tests → PASS ; `cargo test -p rbs-cli --lib 2>&1 | grep "test result"` → vert (vérifier que `crate::upgrade::nombres` est bien `pub(crate)` : il l'est, `upgrade.rs:269`).

- [ ] **Step 3 : la note** — `crates/rbs-cli/notes/1.10.0.md` :

```markdown
# rbs 1.10.0

## `Comparison<T>` gagne `in`

`rbs-core` accepte `{ "colonne": { "in": [...] } }` sur toute colonne comparable, et plus
seulement sur une énumération. Le `filter.rs` que `rbs generate crud` engendre l'applique.
Un code qui construit un `Comparison` par littéral, sans `..Default::default()`, doit
ajouter `r#in: None` — aucun fichier engendré ne le fait.

Un `filter.rs` engendré avant 1.10.0 ignore `in` en silence : reprendre, dans la fonction
`compare`, la ligne `.add_option(compare.r#in.clone().map(|valeurs| colonne.is_in(valeurs)))`.

## `generate crud` exige un noyau 1.10.0

Le filtre qu'il engendre lit `in`. Sur un projet antérieur, la commande refuse et
renvoie à `rbs upgrade`, qui aligne la version de `rbs-core`.
```

- [ ] **Step 4 : commit** — `feat(generate): refuse d'engendrer sur un noyau antérieur à l'opérateur in`, avec `Vérifications :`.

---

### Task 4 : la grammaire `label=`

**Files :**
- Modify : `crates/rbs-cli/src/generate/fields.rs` (`Reference`, boucle des modificateurs `:656-705`, tests)
- Modify : `crates/rbs-cli/src/generate/fields/error.rs` (`ErrorKind`, message, remède, liste des modificateurs admis)

**Interfaces :**
- Produces : `Reference { target, on_delete, label: Option<String> }` ; `ErrorKind::InvalidLabel` ; `label` refusé hors référence par `ErrorKind::UnknownModifier`.

- [ ] **Step 1 : tests rouges** — dans les tests de `fields.rs` :

```rust
    #[test]
    fn a_reference_takes_a_label() {
        let champs = parse("ticket:references:tickets:cascade:label=sujet").expect("valide");

        assert_eq!(champs[0].reference().expect("référence").label.as_deref(), Some("sujet"));
    }

    #[test]
    fn a_reference_without_label_leaves_it_to_inference() {
        let champs = parse("ticket:references:tickets").expect("valide");

        assert_eq!(champs[0].reference().expect("référence").label, None);
    }

    #[test]
    fn a_label_on_a_scalar_is_an_unknown_modifier() {
        let erreur = parse("titre:string:label=titre").expect_err("refus attendu").to_string();

        assert!(erreur.contains("modificateur inconnu « label=titre »"), "{erreur}");
    }

    #[test]
    fn an_empty_label_is_refused() {
        let erreur = parse("ticket:references:tickets:label=").expect_err("refus attendu").to_string();

        assert!(erreur.contains("« label » attend un nom de colonne"), "{erreur}");
    }

    #[test]
    fn a_repeated_label_is_refused() {
        let erreur = parse("ticket:references:tickets:label=a:label=b")
            .expect_err("refus attendu")
            .to_string();

        assert!(erreur.contains("modificateur « label » en double"), "{erreur}");
    }
```

Run : `cargo test -p rbs-cli --lib generate::fields` → FAIL (compilation : champ `label` absent).

- [ ] **Step 2 : implémentation**

`Reference` gagne :

```rust
    /// Colonne de la cible qui la représente à l'écran, quand `label=` l'a forcée.
    pub label: Option<String>,
```

(Toute construction de `Reference { .. }` dans le crate reçoit `label: None` — `grep -rn "Reference {" crates/rbs-cli/src`.)

Dans la boucle des modificateurs, juste après le bloc `max=` :

```rust
        // `label=<colonne>` ne vaut que sur une référence : sur un scalaire, il tombe dans
        // le bras des modificateurs inconnus, comme `cascade`.
        if is_reference && let Some(colonne) = modifier.strip_prefix("label=") {
            let FieldKind::Reference(reference) = &mut field.kind else {
                unreachable!("is_reference vient d'être établi");
            };
            if reference.label.is_some() {
                return Err(error(
                    name,
                    ErrorKind::DuplicateModifier {
                        name: "label".to_string(),
                    },
                ));
            }
            if colonne.is_empty() {
                return Err(error(name, ErrorKind::InvalidLabel));
            }
            reference.label = Some(colonne.to_string());
            continue;
        }
```

Dans `error.rs` : variante `InvalidLabel,` ; message `Self::InvalidLabel => "« label » attend un nom de colonne de la table visée".to_string(),` ; remède `Self::InvalidLabel => Some(format!("exemple : « {label}:references:tickets:label=sujet »")),` ; la liste des modificateurs admis (deux occurrences, message et test) devient `"unique, optional, index, max=<n> — sur une référence : cascade, nullify, label=<colonne>"`.

Run : tests → PASS ; `cargo test -p rbs-cli --lib 2>&1 | grep "test result"` → vert (le test `an_unknown_modifier_lists_the_allowed_ones` suit la nouvelle liste).

- [ ] **Step 3 : commit** — `feat(generate): accepte label=<colonne> sur une référence`.

---

### Task 5 : la fiche de référence, ses refus et ses replis

**Files :**
- Create : `crates/rbs-cli/src/generate/reference.rs`
- Modify : `crates/rbs-cli/src/generate/mod.rs` (`mod reference;`), `crates/rbs-cli/src/generate/command.rs` (`plan_for`, `Error`, `Planned`), `crates/rbs-cli/src/lib.rs:983` (affichage)

**Interfaces :**
- Consumes : `Field::reference()`, `Reference::label`, `entities::find`, `crate::client::ts::nom_de_methode`.
- Produces :

```rust
pub(crate) struct Fiche {
    /// La colonne de la table engendrée : `ticket_id`.
    pub cle: String,
    /// L'en-tête, humanisé depuis le nom de la relation : `Ticket`.
    pub entete: String,
    /// La méthode du client qui filtre la cible : `ticketsFilter`.
    pub methode: String,
    /// La colonne libellé de la cible : `Some("sujet")`, `None` → identifiant raccourci.
    pub libelle: Option<String>,
}
/// Une issue par champ référence, dans l'ordre des champs.
pub(crate) enum Issue { Selecteur(Fiche), Texte }
/// Ce que le plan annonce d'une référence qui ne reçoit pas tout.
pub(crate) struct Repli { pub relation: String, pub cible: String, pub cause: Cause }
pub(crate) enum Cause { SansRouteDeFiltre, SansColonneTextuelle }
pub(crate) struct LabelInconnu { relation: String, cible: String, colonne: String, connues: Vec<String> }
pub(crate) fn colonnes_textuelles(source: &str, table: &str) -> Vec<String>;
pub(crate) fn a_une_route_de_filtre(root: &Path, table: &str) -> bool;
pub(crate) fn verifier_les_labels(root: &Path, feature: &Feature, entities: &[Entity]) -> Result<(), LabelInconnu>;
pub(crate) fn fiches(root: &Path, feature: &Feature, entities: &[Entity]) -> (Vec<Issue>, Vec<Repli>);
```

- [ ] **Step 1 : tests rouges** — créer `reference.rs` avec son module de tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const MODELE: &str = r#"
#[sea_orm(table_name = "tickets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub sujet: String,
    pub resume: Option<String>,
    pub statut: TicketStatut,
    pub auteur_id: Uuid,
}
"#;

    #[test]
    fn the_textual_columns_are_the_strings_of_the_model_in_order() {
        assert_eq!(colonnes_textuelles(MODELE, "tickets"), ["sujet", "resume"]);
    }

    #[test]
    fn a_model_file_holding_several_tables_is_read_for_the_right_one() {
        let source = format!("{MODELE}\n#[sea_orm(table_name = \"autres\")]\npub struct Model {{\n    pub nom: String,\n}}\n");

        assert_eq!(colonnes_textuelles(&source, "autres"), ["nom"]);
    }

    #[test]
    fn a_filter_route_is_found_by_its_operation_id() {
        let racine = tempfile::TempDir::new().expect("répertoire");
        std::fs::create_dir_all(racine.path().join("src/tickets")).expect("dossier");
        std::fs::write(
            racine.path().join("src/tickets/controller.rs"),
            "    operation_id = \"tickets_filter\",\n",
        )
        .expect("écriture");

        assert!(a_une_route_de_filtre(racine.path(), "tickets"));
        assert!(!a_une_route_de_filtre(racine.path(), "refresh_tokens"));
    }
}
```

et, dans les tests de `command.rs` (projet jetable via les helpers existants `project_with_auth`, `options`, `commit`) :

```rust
    #[test]
    fn an_unknown_label_is_refused_even_without_the_admin_shell() {
        let (_dir, root) = project_with_auth();
        commit(&root);
        run(&options(&root, "tickets", Some("sujet:string"), true)).expect("tickets");
        commit(&root);

        let erreur = run(&options(
            &root,
            "commentaires",
            Some("corps:text,ticket:references:tickets:label=titre"),
            true,
        ))
        .expect_err("refus attendu")
        .to_string();

        assert!(erreur.contains("« titre »"), "{erreur}");
        assert!(erreur.contains("sujet"), "les colonnes connues sont nommées : {erreur}");
    }

    #[test]
    fn a_self_reference_takes_its_label_from_the_feature_being_generated() {
        let feature = Feature::fresh(
            "categories",
            fields::parse("nom:string,parent:references:categories:optional").expect("valide"),
        );
        let racine = tempfile::TempDir::new().expect("répertoire");

        let (issues, _) = super::super::reference::fiches(racine.path(), &feature, &[]);

        let super::super::reference::Issue::Selecteur(fiche) = &issues[0] else {
            panic!("un sélecteur attendu : {issues:?}");
        };
        assert_eq!(fiche.libelle.as_deref(), Some("nom"));
        assert_eq!(fiche.methode, "categoriesFilter");
    }
```

Run : `cargo test -p rbs-cli --lib reference` → FAIL (module absent).

- [ ] **Step 2 : implémentation de `reference.rs`**

```rust
//! Ce qu'un écran engendré sait d'une référence : où chercher ses lignes, et comment les
//! nommer.
//!
//! Séparé de `ecran.rs`, qui ne lit pas le disque : la colonne libellé se relève dans le
//! `model.rs` de la cible, et la route de filtre dans les contrôleurs du projet.

use std::fmt;
use std::fs;
use std::path::Path;

use super::entities::{self, Entity};
use super::feature::Feature;
use super::fields::FieldType;

// (structures `Fiche`, `Issue`, `Repli`, `LabelInconnu` : voir Interfaces ci-dessus,
// chacune `#[derive(Debug, Clone, PartialEq, Eq)]`.)

/// Les colonnes `String` ou `Option<String>` que le `struct Model` de `table` déclare.
///
/// Même lecture textuelle que `alter::colonnes_declarees`, le type en plus : une
/// énumération ou une référence n'y ressemblent pas, et c'est ce qui les écarte.
pub(crate) fn colonnes_textuelles(source: &str, table: &str) -> Vec<String> {
    let attribut = format!("table_name = \"{table}\"");
    let mut colonnes = Vec::new();
    let mut dans_la_structure = false;

    for ligne in source.lines() {
        let ligne = ligne.trim();
        if !dans_la_structure {
            dans_la_structure = ligne.contains(&attribut);
            continue;
        }
        if ligne == "}" {
            break;
        }
        if let Some(reste) = ligne.strip_prefix("pub ")
            && let Some((nom, type_)) = reste.split_once(':')
        {
            let type_ = type_.trim().trim_end_matches(',');
            if type_ == "String" || type_ == "Option<String>" {
                colonnes.push(nom.trim().to_string());
            }
        }
    }

    colonnes
}

/// La cible expose-t-elle `POST /<table>/filter` ? Lu à son `operation_id`, que
/// `generate crud` et le fragment `auth` écrivent tous deux.
pub(crate) fn a_une_route_de_filtre(root: &Path, table: &str) -> bool {
    let cherche = format!("operation_id = \"{table}_filter\"");
    fichiers_rust(&root.join("src"))
        .iter()
        .any(|fichier| fs::read_to_string(fichier).is_ok_and(|contenu| contenu.contains(&cherche)))
}
```

`fichiers_rust` : parcours récursif de `src/` rendant les `*.rs` (écrire une petite fonction `fn fichiers_rust(dossier: &Path) -> Vec<PathBuf>` avec `fs::read_dir`, en ignorant les erreurs de lecture).

Les colonnes textuelles d'une cible : si `cible == feature.module()` (auto-référence), les champs de la feature dont `field.reference().is_none() && field.enum_variants().is_empty() && matches!(field.column_type(), FieldType::String | FieldType::Text)`, par `column_name()` ; sinon `entities::find(entities, cible)` puis `colonnes_textuelles(&fs::read_to_string(root.join(&entity.file)).unwrap_or_default(), cible)`.

`verifier_les_labels` : pour chaque référence portant `label: Some(colonne)`, erreur `LabelInconnu` si `colonne` n'est pas dans ces colonnes textuelles. `Display` :

```rust
impl fmt::Display for LabelInconnu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "relation « {} » — « {} » n'est pas une colonne textuelle de « {} »\n        \
             → colonnes textuelles : {}",
            self.relation,
            self.colonne,
            self.cible,
            if self.connues.is_empty() { "aucune".to_string() } else { self.connues.join(", ") }
        )
    }
}
```

`fiches` : pour chaque champ référence, dans l'ordre des champs —
- route : auto-référence → vraie ; sinon `a_une_route_de_filtre(root, cible)`. Fausse → `Issue::Texte`, et un `Repli { cause: Cause::SansRouteDeFiltre, .. }`.
- libellé : `label` s'il est donné, sinon la première colonne textuelle. Aucune → la fiche reste un `Issue::Selecteur` avec `libelle: None` (le sélecteur montre des identifiants raccourcis), et un `Repli { cause: Cause::SansColonneTextuelle, .. }` l'annonce.
- `methode` : `crate::client::ts::nom_de_methode(&format!("{cible}_filter"))`.
- `entete` : `humanise(field.relation_name())` — rendre `humanise` de `ecran.rs` `pub(crate)`.

- [ ] **Step 3 : le branchement dans `command.rs`**

Après `relations::ensure_migrations_exist(...)` :

```rust
    // Avant le rendu, et que l'écran soit engendré ou non : une colonne mal orthographiée ne
    // doit pas attendre, silencieuse, le jour où le projet recevra le shell.
    reference::verifier_les_labels(&root, &feature_provisoire, &entities).map_err(Error::LabelInconnu)?;
```

(`feature_provisoire` : si la `Feature` n'est pas encore construite à cet endroit, placer l'appel juste après la construction de `feature`, avant `render`.) Variante `Error::LabelInconnu(reference::LabelInconnu)` avec `#[error("{0}")]`, code `label_inconnu`.

Là où `ecran` est construit : calculer `let (issues, replis) = reference::fiches(&root, &feature, &entities);` seulement si l'écran est émis, passer `issues` à l'écran (tâche 6), et poser dans `Planned` un champ `replis_de_reference: Vec<String>` — une phrase par repli :
- `SansRouteDeFiltre` vers `users` → `la référence « auteur » reste une saisie d'identifiant : « users » n'expose pas POST /users/filter — voir « Lister les comptes » dans le guide auth`
- `SansRouteDeFiltre` vers une autre cible → `la référence « … » reste une saisie d'identifiant : « {cible} » n'expose pas de route de filtre`
- `SansColonneTextuelle` → `la référence « … » s'affichera par son identifiant raccourci : « {cible} » n'a pas de colonne textuelle — label=<colonne> en choisit une`

Dans `lib.rs`, après le bloc `required_reference` : `for repli in &planned.replis_de_reference { hors_du_document(&format!("\n  {repli}"), json); }`. `plan_repair` et tout constructeur de `Planned` reçoivent `replis_de_reference: Vec::new()`.

Run : `cargo test -p rbs-cli --lib 2>&1 | grep "test result"` → vert, les deux tests neufs de `command.rs` compris.

- [ ] **Step 4 : commit** — `feat(generate): établit la fiche de chaque référence de l'écran`.

---

### Task 6 : l'écran — sélecteur, libellés, fichiers génériques

**Files :**
- Modify : `crates/rbs-cli/src/ecran.rs`
- Modify : `crates/rbs-cli/templates/features/frontend-admin/client/src/admin/vues/Patron.vue.jinja`
- Create : `crates/rbs-cli/templates/features/frontend-admin/client/src/admin/references/ChoixReference.vue.jinja`, `…/references/libelles.ts.jinja`
- Modify : `crates/rbs-cli/templates/features/frontend-admin/feature.toml` (deux `[[files]]`)
- Modify : `crates/rbs-cli/src/generate/command.rs` (dépôt des deux fichiers s'ils manquent)

**Interfaces :**
- Consumes : `reference::Issue`, `reference::Fiche` (tâche 5).
- Produces : `Ecran::avec_references(self, issues: &[Issue]) -> Self` ; `Ecran.references: Vec<ReferenceEcran { cle, methode, libelle: Option<String>, optionnel }>` ; rendu `'reference'` ; composant `'reference'` ; `ecran::GENERIQUES: [(&str, &str); 2]` (destination, template).

- [ ] **Step 1 : tests rouges** — dans les tests d'`ecran.rs` :

```rust
    fn avec_ticket() -> Ecran {
        let feature = Feature::fresh(
            "commentaires",
            fields::parse("corps:text,ticket:references:tickets,auteur:references:users:optional")
                .expect("valide"),
        );
        let issues = vec![
            Issue::Selecteur(Fiche {
                cle: "ticket_id".into(),
                entete: "Ticket".into(),
                methode: "ticketsFilter".into(),
                libelle: Some("sujet".into()),
            }),
            Issue::Selecteur(Fiche {
                cle: "auteur_id".into(),
                entete: "Auteur".into(),
                methode: "usersFilter".into(),
                libelle: Some("email".into()),
            }),
        ];
        Ecran::pour(&feature, Lang::Fr).avec_references(&issues)
    }

    #[test]
    fn a_reference_column_is_read_by_its_label_and_is_not_sortable() {
        let ecran = avec_ticket();
        let colonne = ecran.colonnes.iter().find(|c| c.cle == "ticket_id").expect("colonne");

        assert_eq!(colonne.rendu, "reference");
        assert_eq!(colonne.libelle, "Ticket");
        assert!(!colonne.triable);
    }

    #[test]
    fn a_reference_field_is_chosen_not_typed() {
        let rendu = rendu(&avec_ticket());

        assert!(rendu.contains("import ChoixReference from '@/admin/references/ChoixReference.vue'"), "{rendu}");
        assert!(rendu.contains(":chercher=\"REFERENCES.ticket_id.chercher\""), "{rendu}");
        assert!(rendu.contains("api.ticketsFilter("), "{rendu}");
        assert!(rendu.contains("sujet: motif === '' ? undefined : { contains: motif }"), "{rendu}");
        assert!(rendu.contains("id: { in: ids }"), "{rendu}");
    }

    #[test]
    fn an_optional_reference_offers_to_clear_it() {
        let rendu = rendu(&avec_ticket());

        let auteur = rendu
            .split("id=\"champ-auteur_id\"")
            .nth(1)
            .and_then(|suite| suite.split("/>").next())
            .expect("le sélecteur de l'auteur est rendu");
        let ticket = rendu
            .split("id=\"champ-ticket_id\"")
            .nth(1)
            .and_then(|suite| suite.split("/>").next())
            .expect("le sélecteur du ticket est rendu");

        assert!(auteur.contains("optionnel"), "l'auteur est facultatif : {auteur}");
        assert!(!ticket.contains("optionnel"), "le ticket est requis : {ticket}");
    }

    #[test]
    fn a_reference_left_as_text_keeps_the_text_input() {
        let feature = Feature::fresh("sessions", fields::parse("jeton:references:refresh_tokens").expect("valide"));
        let issues = vec![Issue::Texte];
        let rendu = rendu(&Ecran::pour(&feature, Lang::Fr).avec_references(&issues));

        assert!(!rendu.contains("ChoixReference"), "{rendu}");
        assert!(rendu.contains("id=\"champ-jeton_id\""), "{rendu}");
    }

    #[test]
    fn the_demonstration_screen_does_not_move_by_a_byte() {
        assert_eq!(
            rendu(&Ecran::demonstration(Lang::Fr)),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/admin-console/frontend/src/admin/vues/Demonstration.vue"))
        );
    }
```

(Si `examples/admin-console/.../Demonstration.vue` porte des marqueurs de région, comparer après `normalize` comme le fait `integration_examples`, ou comparer au rendu de `main` capturé avant la tâche : `git show HEAD:crates/...` n'aide pas ; le plus simple est de figer le rendu actuel dans `$SCRATCH/demonstration.vue` **avant** toute modification du patron, et de comparer au fichier par `include_str!` d'un chemin de test — noter le choix au journal.)

Run : `cargo test -p rbs-cli --lib ecran::` → FAIL (compilation : `avec_references` absent).

- [ ] **Step 2 : `ecran.rs`**

```rust
/// Ce que l'écran sait d'une colonne référence : comment chercher ses lignes et les nommer.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReferenceEcran {
    /// La colonne : `ticket_id`.
    pub cle: String,
    /// La méthode du client qui filtre la cible : `ticketsFilter`.
    pub methode: String,
    /// La colonne libellé de la cible, absente quand l'identifiant raccourci en tient lieu.
    pub libelle: Option<String>,
    /// La référence peut rester vide.
    pub optionnel: bool,
}
```

`Ecran` gagne `pub references: Vec<ReferenceEcran>` (vide dans `demonstration` et `pour`). Méthode :

```rust
    /// Branche les références que le plan a pu résoudre ; celles qui restent en texte
    /// gardent le champ de saisie et la colonne brute.
    pub(crate) fn avec_references(mut self, issues: &[Issue]) -> Self {
        for issue in issues {
            let Issue::Selecteur(fiche) = issue else { continue };
            let optionnel = self
                .champs
                .iter()
                .find(|champ| champ.cle == fiche.cle)
                .is_some_and(|champ| champ.optionnel);

            for colonne in self.colonnes.iter_mut().filter(|c| c.cle == fiche.cle) {
                colonne.rendu = "reference".to_string();
                colonne.libelle = fiche.entete.clone();
                colonne.triable = false;
            }
            for propriete in self.proprietes.iter_mut().filter(|p| p.cle == fiche.cle) {
                propriete.rendu = "reference".to_string();
                propriete.libelle = fiche.entete.clone();
            }
            for champ in self.champs.iter_mut().filter(|c| c.cle == fiche.cle) {
                champ.composant = "reference".to_string();
                champ.libelle = fiche.entete.clone();
            }
            self.references.push(ReferenceEcran {
                cle: fiche.cle.clone(),
                methode: fiche.methode.clone(),
                libelle: fiche.libelle.clone(),
                optionnel,
            });
        }
        self.composants = composants(&self.champs, self.filtrable);
        self
    }
```

`Champ.controle` reste `texte` : `corps()` et `saisie()` n'ont rien à changer. `composants()` ne retient pas `reference` (la template l'importe elle-même sous `{% if ecran.references %}`), mais un champ `reference` compte pour `label` — déjà vrai (`!champs.is_empty()`). Le champ `input` n'est plus exigé par un champ `reference` : la condition `champ.composant == "input"` l'exclut déjà.

- [ ] **Step 3 : les deux fichiers génériques**

`libelles.ts.jinja` :

```ts
/** Une ligne référencée, telle qu'un écran la montre : son identifiant et son libellé. */
export interface Entree {
  cle: string
  libelle: string
}

/** L'identifiant raccourci, quand aucun libellé ne le remplace. */
export function courte(cle: string): string {
  return cle.length > 8 ? `${cle.slice(0, 8)}…` : cle
}

/**
 * Les libellés des identifiants d'une page, en un appel.
 *
 * Ne garde que les identifiants demandés : une source qui ignorerait l'opérateur `in` rend
 * des lignes quelconques, et un libellé attribué à la mauvaise ligne serait pire qu'un
 * identifiant. Une panne rend une table vide — l'écran montre alors les identifiants.
 */
export async function resoudre(
  lire: (ids: string[]) => Promise<Entree[]>,
  ids: readonly (string | null)[],
): Promise<Map<string, string>> {
  const demandes = [...new Set(ids.filter((id): id is string => typeof id === 'string'))]
  const libelles = new Map<string, string>()

  if (demandes.length === 0) {
    return libelles
  }

  try {
    const voulus = new Set(demandes)
    for (const entree of await lire(demandes)) {
      if (voulus.has(entree.cle)) {
        libelles.set(entree.cle, entree.libelle)
      }
    }
  } catch {
    // Un refus (403 pour un compte qui n'est pas admin) ou une panne : la table reste
    // lisible par ses identifiants, et aucune alerte ne couvre l'écran.
  }

  return libelles
}

/** La panne est-elle un refus de droits ? */
export function refusee(cause: unknown): boolean {
  return typeof cause === 'object' && cause !== null && 'status' in cause && cause.status === 403
}
```

`ChoixReference.vue.jinja` :

```vue
<script setup lang="ts">
import { ref, watch } from 'vue'

import { Input } from '@/components/ui/input'

import { courte, refusee, type Entree } from './libelles'

const props = defineProps<{
  id: string
  modelValue: string
  libelle: string | undefined
  chercher: (motif: string) => Promise<Entree[]>
  optionnel?: boolean
}>()

const emit = defineEmits<{ 'update:modelValue': [valeur: string] }>()

const TEXTES = {
{%- if lang == 'en' %}
  recherche: 'Search…',
  aucun: 'None',
  rien: 'No match',
  reserve: 'Reserved to administrators',
  indisponible: 'Search unavailable',
{%- else %}
  recherche: 'Rechercher…',
  aucun: 'Aucun',
  rien: 'Aucune correspondance',
  reserve: 'Réservé aux administrateurs',
  indisponible: 'Recherche indisponible',
{%- endif %}
} as const

/** Ce que le champ montre au repos : le libellé de la valeur, sinon son identifiant. */
function repos(): string {
  if (props.modelValue === '') {
    return ''
  }
  return props.libelle ?? courte(props.modelValue)
}

const saisie = ref(repos())
const choisi = ref(saisie.value)
const ouvert = ref(false)
const resultats = ref<Entree[]>([])
const actif = ref(-1)
const faute = ref<string | null>(null)
let minuterie: ReturnType<typeof setTimeout> | undefined

// Le formulaire se rouvre sur une autre ligne, ou le libellé arrive après lui : le repos suit.
watch(
  () => [props.modelValue, props.libelle],
  () => {
    choisi.value = repos()
    if (!ouvert.value) {
      saisie.value = choisi.value
    }
  },
)

async function chercher(motif: string): Promise<void> {
  try {
    resultats.value = await props.chercher(motif)
    faute.value = null
  } catch (cause) {
    resultats.value = []
    faute.value = refusee(cause) ? TEXTES.reserve : TEXTES.indisponible
  }
  actif.value = resultats.value.length > 0 ? 0 : -1
}

function ouvrir(): void {
  ouvert.value = true
  void chercher('')
}

function taper(valeur: string | number): void {
  saisie.value = String(valeur)
  ouvert.value = true
  clearTimeout(minuterie)
  minuterie = setTimeout(() => void chercher(saisie.value.trim()), 250)
}

function choisir(entree: Entree | null): void {
  emit('update:modelValue', entree?.cle ?? '')
  choisi.value = entree?.libelle ?? ''
  saisie.value = choisi.value
  ouvert.value = false
}

function fermer(): void {
  ouvert.value = false
  saisie.value = choisi.value
}

function clavier(evenement: KeyboardEvent): void {
  if (evenement.key === 'ArrowDown') {
    evenement.preventDefault()
    actif.value = Math.min(actif.value + 1, resultats.value.length - 1)
  } else if (evenement.key === 'ArrowUp') {
    evenement.preventDefault()
    actif.value = Math.max(actif.value - 1, 0)
  } else if (evenement.key === 'Enter' && ouvert.value) {
    evenement.preventDefault()
    const entree = resultats.value[actif.value]
    if (entree !== undefined) {
      choisir(entree)
    }
  } else if (evenement.key === 'Escape') {
    fermer()
  }
}
</script>

<!--
  Un champ qui cherche, et une liste sous lui. `mousedown.prevent` et non `click` : le clic
  arriverait après la perte du focus, qui a déjà refermé la liste.
-->
<template>
  <div class="relative">
    <Input
      :id="id"
      role="combobox"
      autocomplete="off"
      :aria-expanded="ouvert"
      :aria-controls="`${id}-liste`"
      :placeholder="TEXTES.recherche"
      :model-value="saisie"
      @update:model-value="taper"
      @focus="ouvrir"
      @blur="fermer"
      @keydown="clavier"
    />
    <ul
      v-if="ouvert"
      :id="`${id}-liste`"
      role="listbox"
      class="absolute z-50 mt-1 max-h-64 w-full overflow-auto border border-border bg-background shadow-md"
    >
      <li
        v-if="optionnel"
        role="option"
        :aria-selected="false"
        class="cursor-pointer px-3 py-2 text-muted-foreground hover:bg-muted"
        @mousedown.prevent="choisir(null)"
      >
        {{ TEXTES.aucun }}
      </li>
      <li
        v-for="(entree, rang) in resultats"
        :key="entree.cle"
        role="option"
        :aria-selected="rang === actif"
        class="cursor-pointer px-3 py-2 hover:bg-muted"
        :class="{ 'bg-muted': rang === actif }"
        @mousedown.prevent="choisir(entree)"
      >
        {{ entree.libelle }}
      </li>
      <li v-if="faute" class="px-3 py-2 text-destructive">{{ faute }}</li>
      <li v-else-if="resultats.length === 0" class="px-3 py-2 text-muted-foreground">
        {{ TEXTES.rien }}
      </li>
    </ul>
  </div>
</template>
```

`feature.toml` de `frontend-admin`, à côté des autres `[[files]]` :

```toml
# Le sélecteur et la résolution des références : un seul exemplaire pour tout le projet,
# que chaque écran engendré importe. `generate crud` les dépose aussi s'ils manquent.
[[files]]
source      = "client/src/admin/references/ChoixReference.vue.jinja"
destination = "frontend/src/admin/references/ChoixReference.vue"

[[files]]
source      = "client/src/admin/references/libelles.ts.jinja"
destination = "frontend/src/admin/references/libelles.ts"
```

Dans `ecran.rs` :

```rust
/// Les deux fichiers génériques des références, là où le fragment les dépose.
///
/// `generate crud` les rend s'ils manquent — projet équipé du shell avant qu'ils
/// n'existent — et ne les écrase jamais : ils appartiennent au projet une fois posés.
pub(crate) const GENERIQUES: [(&str, &str); 2] = [
    (
        "frontend/src/admin/references/ChoixReference.vue",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/features/frontend-admin/client/src/admin/references/ChoixReference.vue.jinja")),
    ),
    (
        "frontend/src/admin/references/libelles.ts",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/features/frontend-admin/client/src/admin/references/libelles.ts.jinja")),
    ),
];
```

Dans `command.rs`, quand l'écran porte au moins une référence : pour chaque `(destination, template)` de `GENERIQUES` dont `root.join(destination)` n'existe pas, rendre `template` avec `context! { lang => Lang::of_project(&root).name() }` et `builder.create(destination, &rendu)?`. Test dans `command.rs` : `a_crud_with_a_reference_lays_the_generic_files_when_they_are_missing` (projet portant `frontend-admin` : les supprimer, lancer un crud à référence, les trouver au plan en `créé`) et `…_leaves_them_alone_when_present` (le plan ne les nomme pas).

- [ ] **Step 4 : le patron** — sous `{% if ecran.references %}` seulement, pour que la démonstration ne bouge pas :

1. Imports, après ceux de `@/api` :

```
{% if ecran.references %}import ChoixReference from '@/admin/references/ChoixReference.vue'
import { courte, resoudre, type Entree } from '@/admin/references/libelles'
{% endif %}
```

2. `type Rendu = 'texte' | 'date' | 'instant'{% if ecran.references %} | 'reference'{% endif %}`

3. Après `function raison(...)` de la branche `{%- if ecran.api %}` :

```
{%- if ecran.references %}

/**
 * Les tables que cet écran référence : comment en chercher les lignes par leur libellé, et
 * comment en relire une page par identifiants.
 */
const REFERENCES = {
{%- for reference in ecran.references %}
  {@ reference.cle @}: {
    chercher: async (motif: string): Promise<Entree[]> =>
      (
        await api.{@ reference.methode @}(
          {% if reference.libelle %}{ {@ reference.libelle @}: motif === '' ? undefined : { contains: motif } }{% else %}{}{% endif %},
          { page: 1, per_page: 20 },
        )
      ).data.map((ligne) => ({ cle: ligne.id, libelle: {% if reference.libelle %}ligne.{@ reference.libelle @} ?? courte(ligne.id){% else %}courte(ligne.id){% endif %} })),
    lire: async (ids: string[]): Promise<Entree[]> =>
      (
        await api.{@ reference.methode @}({ id: { in: ids } }, { page: 1, per_page: ids.length })
      ).data.map((ligne) => ({ cle: ligne.id, libelle: {% if reference.libelle %}ligne.{@ reference.libelle @} ?? courte(ligne.id){% else %}courte(ligne.id){% endif %} })),
  },
{%- endfor %}
} as const

/** Les libellés résolus, par colonne puis par identifiant. */
const libelles = ref<Record<string, Map<string, string>>>({})

/** Relit les libellés des lignes montrées, une requête par colonne référence. */
async function nommer(montrees: readonly Ligne[]): Promise<void> {
  for (const [cle, reference] of Object.entries(REFERENCES)) {
    const ids = montrees.map((ligne) => ligne[cle as keyof Ligne] as string | null)
    libelles.value = { ...libelles.value, [cle]: await resoudre(reference.lire, ids) }
  }
}

/** Ce que l'opérateur lit d'une cellule : le libellé d'une référence, sinon la valeur. */
function lisible(cle: keyof Ligne, valeur: string | number | boolean | null, rendu: Rendu): string {
  if (rendu === 'reference' && typeof valeur === 'string') {
    return libelles.value[cle]?.get(valeur) ?? courte(valeur)
  }
  return afficher(valeur, rendu)
}
{%- endif %}
```

4. Dans `charger()`, après `total.value = reponse.total` : `{% if ecran.references %}    void nommer(reponse.lignes)\n{% endif %}` ; dans `detailler()`, après `detaillee.value = await lire(cle)` : `{% if ecran.references %}      void nommer([detaillee.value])\n{% endif %}`.

5. Cellules de table et détail : sous `{% if ecran.references %}`, `{{ lisible(colonne.cle, ligne[colonne.cle], colonne.rendu) }}` avec `:title="colonne.rendu === 'reference' ? String(ligne[colonne.cle] ?? '') : undefined"` sur la `TableCell` ; idem `lisible(propriete.cle, detaillee[propriete.cle], propriete.rendu)` dans le `<dd>`. Hors références, le rendu actuel, inchangé.

6. Formulaire, nouveau bras avant `{%- else %}` :

```
{%- elif champ.composant == 'reference' %}
            <ChoixReference
              id="champ-{@ champ.cle @}"
              v-model="valeurs.{@ champ.cle @}"
              :libelle="libelles.{@ champ.cle @}?.get(valeurs.{@ champ.cle @})"
              :chercher="REFERENCES.{@ champ.cle @}.chercher"
{%- if champ.optionnel %}
              optionnel
{%- endif %}
            />
```

`afficher` garde sa signature ; sous références, `rendu` est toujours lu (le paramètre `_rendu` du cas sans horodatage doit rester `rendu` si `ecran.references` — adapter la condition : `{% if horodate or ecran.references %}rendu{% else %}_rendu{% endif %}`, et le test `noUnusedParameters` de la tâche 8 le prouve).

Run : `cargo test -p rbs-cli --lib ecran:: 2>&1 | grep "test result"` → vert, démonstration identique comprise ; `cargo test -p rbs-cli --lib 2>&1 | grep "test result"` → vert.

- [ ] **Step 5 : commit** — `feat(frontend-admin): choisit et nomme les références dans les écrans engendrés`.

---

### Task 7 : `POST /users/filter` dans le fragment `auth`

**Files :**
- Modify : `crates/rbs-cli/templates/features/auth/dto.rs.jinja` (`UserFilter`, `UserSummary`)
- Modify : `…/auth/repository/user.rs.jinja` (`filter`), `…/auth/service/account.rs.jinja` (`filter_users`), `…/auth/controller/account.rs.jinja` (handler), `…/auth/mod.rs.jinja` (route), `…/auth/feature.toml` (ancre `openapi`, fichier de test)
- Create : `…/auth/tests/users.rs.jinja` ; Modify : `…/auth/tests/mod.rs.jinja` (`mod users;`), `…/auth/tests/roles.rs.jinja` (`login_as_admin` en `pub(super)`), `…/auth/tests/openapi.rs.jinja` (seize opérations)

**Interfaces :**
- Produces : `POST /users/filter`, `operation_id = "users_filter"`, tag `users`, corps `UserFilter { id: Option<Comparison<Uuid>>, email: Option<TextMatch>, sort: Option<Sort> }`, réponse `Page<UserSummary { id: Uuid, email: String }>`, seuil `Role::Admin`. Le client TS nommera la méthode `usersFilter`.

- [ ] **Step 1 : tests rouges (dans le fragment)** — `tests/users.rs.jinja` :

```rust
use super::*;

use super::roles::login_as_admin;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]`.

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn listing_accounts_without_a_token_returns_401() {
    let api = application().await;

    let (status, body) = call(&api, post_json("/users/filter", json!({}))).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
}

/// Lister les adresses est une donnée personnelle : un compte ordinaire ne les voit pas.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_user_listing_accounts_gets_403() {
    let api = application().await;
    let (_, paire) = login_as(&api).await;

    let (status, body) = call(
        &api,
        post_json_authenticated("/users/filter", &access_for(&paire), json!({})),
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

/// Un écran résout les auteurs d'une page en un appel : `in` sur l'identifiant, et rien
/// d'autre que l'identifiant et l'adresse en retour.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_resolves_accounts_by_their_ids_and_reads_only_id_and_email() {
    let api = application().await;
    let db = connection().await;
    let admin = login_as_admin(&api, &db).await;
    let premier = registered_user(&db).await;
    let second = registered_user(&db).await;

    let (status, body) = call(
        &api,
        post_json_authenticated(
            "/users/filter",
            &access_for(&admin),
            json!({ "id": { "in": [premier.id, second.id] } }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let lignes = body["data"].as_array().expect("une page");
    assert_eq!(lignes.len(), 2, "{body}");
    for ligne in lignes {
        let cles: Vec<&String> = ligne.as_object().expect("un objet").keys().collect();
        assert_eq!(cles, ["email", "id"], "{body}");
    }
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_admin_searches_accounts_by_email() {
    let api = application().await;
    let db = connection().await;
    let admin = login_as_admin(&api, &db).await;
    let cible = registered_user(&db).await;
    let motif = &cible.email[..12];

    let (status, body) = call(
        &api,
        post_json_authenticated(
            "/users/filter",
            &access_for(&admin),
            json!({ "email": { "contains": motif } }),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body["data"].as_array().expect("une page").iter().any(|l| l["id"] == cible.id.to_string()),
        "{body}"
    );
}
```

(Vérifier en lisant `tests/mod.rs.jinja` que `login_as`, `access_for`, `registered_user`, `connection` y sont visibles depuis un sous-module — ils sont définis au niveau du module `tests` ; passer `login_as_admin` de `roles.rs.jinja` en `pub(super)`.) `tests/openapi.rs.jinja` : ajouter `"/users/filter"` à la liste et renommer le test `the_openapi_document_carries_the_sixteen_auth_operations`. `feature.toml` : un `[[files]]` pour `tests/users.rs.jinja` → `src/auth/tests/users.rs`.

- [ ] **Step 2 : le code** — `dto.rs.jinja` :

```rust
/// Ce que `POST /users/filter` rend d'un compte : de quoi le choisir et le nommer, rien de
/// plus — ni le rôle ni les dates ne servent à une liste de sélection.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserSummary {
    pub id: Uuid,
    pub email: String,
}

/// Les conditions de `POST /users/filter`, écrites comme celles d'un filtre engendré.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UserFilter {
    #[schema(value_type = Option<rbs_core::UuidComparisonSchema>)]
    pub id: Option<Comparison<Uuid>>,
    #[schema(value_type = Option<rbs_core::TextMatchSchema>)]
    pub email: Option<TextMatch>,
    /// Colonnes de tri, préfixées de `-` pour l'ordre décroissant : `email`, `created_at`.
    #[schema(value_type = Option<Vec<String>>)]
    pub sort: Option<Sort>,
}
```

(imports : `rbs_core::{Comparison, Sort, TextMatch}`, `serde::Deserialize` si absent.)

`repository/user.rs.jinja` — `filter` qui applique `id` (`eq`, `in`, `is_null` via le même `compare`), `email` (`eq`, `contains` via le même `motif` échappé), le tri sur `email`/`created_at`/`id` (refus `Error::BadRequest` sinon, message comme `column_of` du filtre engendré), l'ordre par défaut `id` décroissant, la page et le total par `tokio::try_join!` — **recopier** les fonctions `compare`, `matches`, `motif`, `null_condition` du `filter.rs.jinja` engendré en les limitant aux deux colonnes, et les commenter d'une ligne : *mêmes conditions que le filtre engendré, pour qu'un écran interroge les comptes comme une table*. Signature :

```rust
pub async fn filter(
    db: &impl ConnectionTrait,
    filtre: &UserFilter,
    pagination: &Pagination,
) -> Result<(Vec<Model>, u64)>
```

`service/account.rs.jinja` :

```rust
/// Une page de comptes, réduits à ce qu'une liste de sélection montre.
pub async fn filter_users(
    db: &DatabaseConnection,
    filtre: &UserFilter,
    pagination: &Pagination,
) -> Result<Page<UserSummary>> {
    let (comptes, total) = repository::filter(db, filtre, pagination).await?;

    Ok(Page::new(
        comptes
            .into_iter()
            .map(|compte| UserSummary { id: compte.id, email: compte.email })
            .collect(),
        pagination,
        total,
    ))
}
```

`controller/account.rs.jinja` :

```rust
// Réservé aux administrateurs : la liste des adresses est une donnée personnelle, et
// l'espace d'administration est ouvert à toute session.
#[utoipa::path(
    post,
    path = "/users/filter",
    tag = "users",
    operation_id = "users_filter",
    security(("bearer" = [])),
    params(
        ("page" = Option<u64>, Query, description = "numéro de page, à partir de 1"),
        ("per_page" = Option<u64>, Query, description = "éléments par page, 100 au plus")
    ),
    request_body = UserFilter,
    responses(
        (status = 200, description = "page de comptes", body = Page<UserSummary>),
        (status = 400, description = "filtre, tri ou pagination illisible", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 401, description = "jeton absent ou invalide", body = ProblemDetails, content_type = "application/problem+json"),
        (status = 403, description = "rôle insuffisant", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn filter_users(
    State(state): State<AppState>,
    identite: Identity,
    pagination: Pagination,
    Json(filtre): Json<UserFilter>,
) -> Result<Json<Page<UserSummary>>> {
    identite.require_role(Role::Admin)?;

    Ok(Json(service::filter_users(state.core().db(), &filtre, &pagination).await?))
}
```

`mod.rs.jinja`, dans `routes()` : `.route("/users/filter", post(controller::account::filter_users))`. `feature.toml`, ancre `openapi` : ajouter `crate::auth::controller::account::filter_users,`.

- [ ] **Step 3 : prouver dans un projet engendré** — les tests du fragment ne tournent que dans un projet. Le banc Docker d'`auth` :

Run : `cargo test -p rbs-cli --test integration_auth -- --include-ignored --no-fail-fast > $SCRATCH/t7.log 2>&1; grep "test result\|FAILED\|users" $SCRATCH/t7.log`
Expected : vert, les quatre tests `users` exécutés dans le projet engendré (le banc lance les tests du projet ; si ce n'est pas le cas, lancer à la main : `rbs new` jetable avec `--with auth`, `rbs migrate up` sur un PostgreSQL 16, `cargo test -- --include-ignored auth::tests::users`). Rouge d'abord : lancer une fois ces tests **avant** l'étape 2 (route absente → 404/405) et consigner l'échec.

Puis `cargo test -p rbs-cli --lib 2>&1 | grep "test result"` (rendu des templates, formatage rustfmt) → vert.

- [ ] **Step 4 : commit** — `feat(auth): liste les comptes aux administrateurs par POST /users/filter`.

---

### Task 8 : un écran à références passe le compilateur TypeScript

**Files :**
- Modify : `crates/rbs-cli/tests/integration_frontend.rs` (`the_admin_shell_generates_its_client_typechecks_and_builds`)

- [ ] **Step 1** — après la table `bordereaux`, dans le même bloc sous verrou :

```rust
        // Une table à références : vers une table engendrée, forcée par `label=`, et vers
        // les comptes, facultative. C'est le seul endroit où le sélecteur, la résolution
        // et `usersFilter` passent par `vue-tsc`.
        rbs(&projet)
            .env("CARGO_TARGET_DIR", common::cible())
            .args([
                "generate",
                "crud",
                "annotations",
                "--fields",
                "texte:string,bordereau:references:bordereaux:label=titre,auteur:references:users:optional",
            ])
            .assert()
            .success();
```

et, après l'installation npm, une assertion que `frontend/src/admin/vues/Annotations.vue` contient `ChoixReference` et `usersFilter`.

Run : `cargo test -p rbs-cli --test integration_frontend the_admin_shell -- --include-ignored > $SCRATCH/t8.log 2>&1; grep "test result\|FAILED\|error TS" $SCRATCH/t8.log`
Expected : vert (typecheck et build). Un `error TS` se corrige dans le patron ou les fichiers génériques, puis la tâche 6 est relancée.

- [ ] **Step 2 : commit** — `test(frontend): fait compiler un écran à références par vue-tsc`.

---

### Task 9 : les exemples

**Files :** `examples/{blog-auth,event-hub,admin-console,help-desk}/**`, `examples/README.md`, `examples/README.fr.md`, `crates/rbs-cli/tests/integration_examples.rs` (entrée `help-desk` : champs `label=`)

- [ ] **Step 1 : lire le constat** — `cargo test -p rbs-cli --test integration_examples --no-fail-fast > $SCRATCH/t9.log 2>&1` ; lister les exemples en dérive.

- [ ] **Step 2 : régénérer par diff** — pour chaque exemple en dérive : engendrer deux fois dans `$SCRATCH` (le CLI d'avant cette branche, `git stash`/`git worktree` sur `main`, et celui de la branche) avec les commandes d'`examples/README.md`, calculer `diff -ruN avant apres > $SCRATCH/<ex>.patch`, puis `patch -p1 --no-backup-if-mismatch -d examples/<ex> < $SCRATCH/<ex>.patch`. Jamais d'écrasement : les retouches à la main survivent.

- [ ] **Step 3 : `help-desk`** — la commande des commentaires devient `corps:text,ticket:references:tickets:cascade:label=sujet,auteur:references:users` dans `integration_examples.rs` et dans les deux README ; régénérer ; réappliquer les retouches Vue de `Tickets.vue`/`Commentaires.vue` (retrait du champ `auteur` du formulaire — désormais un `ChoixReference` — de `Formulaire`, `VIERGE`, `corps()`, `saisie()`) ; `the_hand_edits_of_help_desk_are_in_place` : remplacer la garde `champ-auteur_id` par l'absence de `REFERENCES.auteur_id.chercher` dans le formulaire des deux écrans. Régénérer le client : `rbs generate client --lang ts --out frontend/src/api --force`.

- [ ] **Step 4 : vérifier** — `integration_examples` vert ; `cargo clippy --workspace --all-targets -- -D warnings` dans chaque exemple touché ; `npm install && npm run typecheck && npm run build` dans `admin-console/frontend` et `help-desk/frontend`, puis suppression de `node_modules`, `dist`, `package-lock.json` ; tests de `help-desk` contre PostgreSQL 16 (`--include-ignored`, Mailpit monté).

- [ ] **Step 5 : commit** — `docs(examples): régénère les exemples avec le sélecteur de références`.

---

### Task 10 : documentation, tutoriel, changelog

**Files :** `docs/docs/cli/generate.md`, `docs/docs/guides/{relations,frontend,auth,filtering}.md`, `docs/docs/tutorials/help-desk.md` et leurs jumeaux sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/`, `CHANGELOG.md`

- [ ] **Step 1** — `cli/generate.md` : `label=<colonne>` dans le tableau des modificateurs (« Reference only. The target column that names a row on the admin screen ; defaults to the first textual column ») et dans la grammaire. `guides/relations.md` : section « On the admin screen » — sélecteur, libellé deviné, `label=`, les deux replis, avec l'erreur de `label=` inconnu en `rbs:transcript` (setup : projet avec `frontend-admin`, crud `tickets`). `guides/filtering.md` : `in` sur toute colonne comparable, exemple `{ "id": { "in": [...] } }`. `guides/frontend.md` : le sélecteur, la résolution, le compte non admin. `guides/auth.md` : la route, ses droits, et la section « Listing accounts » / « Lister les comptes » (code à reprendre pour un projet antérieur). Chaque page en fr dans le même commit.

- [ ] **Step 2 : tutoriel** — `help-desk.md` fr et en : la commande des commentaires porte `label=sujet`, avec une phrase qui dit qu'il est redondant ici et pourquoi il est écrit ; la « limite assumée » de l'étape 5 est remplacée par le sélecteur (extrait `ts file=examples/help-desk/frontend/src/admin/vues/Commentaires.vue region=references` — poser la région dans l'exemple) ; « Verify » cite l'email dans la colonne auteur et le sélecteur de ticket ; transcriptions recollées depuis les sorties réelles.

- [ ] **Step 3 : `CHANGELOG.md`**, sous `[Unreleased]` : *Added* — sélecteur et libellé des références ; `label=` ; `POST /users/filter` ; `in` sur `Comparison`. *Changed* — `generate crud` refuse un noyau antérieur à 1.10.0.

- [ ] **Step 4 : vérifier** — `cargo test -p rbs-cli --test integration_docs --no-fail-fast` (en arrière-plan, sortie dans `$SCRATCH`), `cd docs && npm run build && npm run parite`.

- [ ] **Step 5 : commit** — `docs: décrit le sélecteur de références, label= et la liste des comptes`.

---

### Task 11 : passe finale

- [ ] `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` ; `cargo test -p rbs-core --all-features`.
- [ ] `cargo test -p rbs-cli --test integration_examples` ; `integration_docs` déjà vert en tâche 10.
- [ ] Parcours navigateur de `help-desk` : serveur lancé, `/admin` servi, écran de connexion sans erreur console ; le parcours connecté (sélecteur de ticket, email dans la colonne auteur) est demandé au mainteneur.
- [ ] `git status --short` vide ; aucun `node_modules`, `dist`, `package-lock.json` dans `examples/`.
