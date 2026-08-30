# Commentaire de fin de ligne dans le `.env`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `dotenv::parse` coupe le commentaire qui suit une valeur, pour que `RBS_ENV=production # ne jamais semer` soit lu `production` et que le garde-fou de `rbs seed` se prononce.

**Architecture:** une fonction privée `strip_comment`, intercalée entre le découpage sur `=` et `unquote`, appliquant la règle de `dotenvy` : hors guillemets un `#` n'ouvre un commentaire que s'il est précédé d'un blanc ou ouvre la valeur ; entre guillemets il est littéral et seul ce qui suit le guillemet fermant tombe. `seed.rs` n'est pas touché — son égalité stricte (`:172`) redevient correcte dès que la valeur est propre.

**Tech Stack:** Rust, `std` seul. Aucune dépendance ajoutée.

**Spec:** `IMPROVE.md` tâche 2 (P0, Bug). Design validé en chat le 2026-08-30 ; option retenue : règle dotenvy stricte, contre « tout `#` coupe », pour qu'un mot de passe `s3#cret` survive.

## Global Constraints

- Commits en Conventional Commits, sujet français à l'impératif, **aucun** identifiant de tâche, aucun renvoi à `IMPROVE.md`, aucune ligne `Co-Authored-By` ni mention d'un assistant (`CLAUDE.md`, section Commits).
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont bloquants.
- Un seul fichier de production est touché : `crates/rbs-cli/src/dotenv.rs`. Ne rien changer à `seed.rs`, `doctor/`, ni `plan/text.rs` — ils consomment ce parseur et bénéficient du correctif sans modification.

---

### Task 1: couper le commentaire de fin de ligne

**Files:**
- Modify: `crates/rbs-cli/src/dotenv.rs:41-56` (`parse_line`), ajout d'une fonction privée à côté d'`unquote` (`:58-69`)
- Test: `crates/rbs-cli/src/dotenv.rs`, module `tests` existant (`:79`)

**Interfaces:**
- Produces: `fn strip_comment(value: &str) -> &str`, privée au module. Aucune API publique ne change : `parse`, `read` et `value` gardent leur signature.

- [ ] **Step 1: écrire les tests qui échouent**

Dans le module `tests` de `dotenv.rs`, à la suite des tests existants :

```rust
/// Le critère de la tâche : c'est ce cas qui désarmait le refus de semer en production.
#[test]
fn a_trailing_comment_is_cut_from_the_value() {
    let paires = parse("RBS_ENV=production # ne jamais semer\n");

    assert_eq!(value(&paires, "RBS_ENV"), Some("production"));
}

/// Un `#` collé à la valeur en fait partie : sans cette réserve, tout mot de passe qui
/// en porte un se ferait tronquer en silence.
#[test]
fn a_hash_without_a_leading_blank_belongs_to_the_value() {
    let paires = parse("PASSWORD=s3#cret\n");

    assert_eq!(value(&paires, "PASSWORD"), Some("s3#cret"));
}

#[test]
fn a_quoted_value_keeps_its_hash_and_loses_what_follows() {
    let paires = parse("PASSWORD=\"a # b\" # commentaire\n");

    assert_eq!(value(&paires, "PASSWORD"), Some("a # b"));
}

#[test]
fn a_value_reduced_to_a_comment_is_empty() {
    let paires = parse("PASSWORD=  # à remplir\n");

    assert_eq!(value(&paires, "PASSWORD"), Some(""));
}

/// Un guillemet ouvert et jamais fermé ne fait pas disparaître la valeur.
#[test]
fn an_unclosed_quote_keeps_the_rest_of_the_line() {
    let paires = parse("A=\"non fermé\n");

    assert_eq!(value(&paires, "A"), Some("\"non fermé"));
}
```

- [ ] **Step 2: lancer les tests et les voir échouer**

Run: `cargo test -p rbs-cli dotenv::`
Expected: FAIL sur `a_trailing_comment_is_cut_from_the_value` (`"production # ne jamais semer"` au lieu de `"production"`), `a_quoted_value_keeps_its_hash_and_loses_what_follows` et `a_value_reduced_to_a_comment_is_empty`. Les deux autres passent déjà — c'est voulu : ils gardent le comportement qui doit survivre.

**Lire la sortie et vérifier que l'échec est bien celui-là** avant d'écrire la moindre ligne d'implémentation.

- [ ] **Step 3: écrire l'implémentation**

Dans `crates/rbs-cli/src/dotenv.rs`, `parse_line` (`:55`) devient :

```rust
    Some((
        key.to_string(),
        unquote(strip_comment(value.trim())).to_string(),
    ))
```

Et la fonction, posée entre `parse_line` et `unquote` :

```rust
/// Retire le commentaire qui suit une valeur, selon la règle de `dotenvy`.
///
/// Le `#` ne coupe que précédé d'un blanc, ou en tête de valeur : un mot de passe
/// `s3#cret` se ferait sinon tronquer en silence. Entre guillemets il est littéral, et
/// seul ce qui suit le guillemet fermant tombe — c'est `unquote` qui dénude ensuite.
fn strip_comment(value: &str) -> &str {
    if let Some(quote) = value.chars().next().filter(|c| *c == '"' || *c == '\'') {
        return match value[1..].find(quote) {
            Some(fin) => &value[..fin + 2],
            None => value,
        };
    }

    let mut precedent = None;
    for (index, caractere) in value.char_indices() {
        if caractere == '#' && precedent.is_none_or(char::is_whitespace) {
            return value[..index].trim_end();
        }
        precedent = Some(caractere);
    }

    value
}
```

Note sur les indices : `fin` est un décalage en octets dans `value[1..]`, et un guillemet fait un octet — d'où `fin + 2` pour inclure le guillemet fermant. `char_indices` rend directement des décalages en octets valides.

- [ ] **Step 4: lancer les tests et les voir passer**

Run: `cargo test -p rbs-cli dotenv::`
Expected: PASS, tous les tests du module — dont les préexistants `the_surrounding_quotes_are_stripped` (`A="info,api=debug"`) et `only_the_first_equals_separates_the_key_from_the_value` (une URL avec `?opt=1`), qui prouvent qu'aucune valeur légitime n'a été rognée.

- [ ] **Step 5: vérifier les consommateurs du parseur**

Run: `cargo test -p rbs-cli`
Expected: PASS. `plan::text::add_variable` (`plan/text.rs:51`), `doctor::auth` (`:68`), `doctor::env` et `migrate::project_variables` lisent tous par ce parseur ; aucun ne doit régresser.

- [ ] **Step 6: prouver le garde-fou de bout en bout**

Ajouter au module `tests` de `crates/rbs-cli/src/seed.rs` un test qui pose un `.env` portant `RBS_ENV=production # ne jamais semer` et vérifie que `production(root, |_| None)` rend `true` — reprendre la fixture du module, `fn production` prend déjà l'environnement en paramètre pour être testable.

Run: `cargo test -p rbs-cli seed::`
Expected: PASS.

- [ ] **Step 7: commit**

```bash
git add crates/rbs-cli/src/dotenv.rs crates/rbs-cli/src/seed.rs
git commit -m "fix(dotenv): coupe le commentaire de fin de ligne d'une valeur"
```

---

### Vérification finale (avant de rendre la main)

- [ ] `cargo test --workspace` — lire la sortie, pas la supposer
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo fmt --all --check`
