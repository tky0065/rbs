# Réparation des ancres sur un manifeste illisible — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** faire procéder `rbs doctor --fix` sur un projet dont le `Cargo.toml` est illisible,
au lieu d'abandonner la commande entière avant tout diagnostic.

**Architecture:** `repair_anchors` (`crates/rbs-cli/src/lib.rs`) désigne la racine par
`metadata::project_root`, qui refuse un manifeste illisible. Le refus est légitime pour les
commandes qui écrivent *dans* le manifeste ; il ne l'est pas ici, où `doctor::anchors::repair`
ne lit aucune donnée du manifeste et n'écrit que dans des fichiers source déjà porteurs
d'ancres. On passe donc à `metadata::racine`, qui rend la racine *et* la faute, et on ne
retient que la racine : la faute reste dite par les contrôles `agents`, `versions` et `base`
du diagnostic qui suit dans la même commande.

**Tech Stack:** Rust, `cargo test -p rbs-cli --lib`.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` (section `rbs doctor`) ; la
doctrine de `project_root` est portée par son propre doc-comment,
`crates/rbs-cli/src/metadata.rs`.

## Global Constraints

- `metadata::project_root` n'est pas assouplie : sa doctrine reste juste pour ses autres
  appelants (`add`, `generate`, `upgrade`, `migrate`, `seed`). C'est `repair_anchors` qui est
  l'exception, et le commentaire doit dire pourquoi.
- Aucune template ni fragment touché : `examples/` ne doit pas dériver.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`
  bloquants.

---

### Task 1 : `--fix` repose une ancre malgré un manifeste cassé

**Files:**
- Modify: `crates/rbs-cli/src/lib.rs` (`repair_anchors`)
- Test: `crates/rbs-cli/src/lib.rs` (`mod tests`, à côté de
  `doctor_fix_refuses_a_dirty_working_tree_unless_forced`)

**Interfaces:**
- Consomme : `metadata::racine(&Path) -> Result<metadata::Racine, metadata::RootError>`, dont
  le champ `manifeste: Result<Manifeste, metadata::Error>`.
- Produit : rien de nouveau — la signature de `repair_anchors` ne bouge pas.

- [x] **Step 1 : écrire le test rouge**

```rust
/// La réparation d'une ancre ne lit rien du manifeste et n'y écrit rien : la refuser
/// parce qu'il est cassé faisait dire à `--fix` le contraire de ce que le diagnostic du
/// même passage sait faire depuis qu'il survit à ce cas.
#[test]
fn doctor_fix_repairs_anchors_despite_a_broken_manifest() {
    let (_parent, root) = projet();
    amputer(&root, "src/router.rs", "<rbs:routes>");
    amputer(&root, "src/router.rs", "</rbs:routes>");
    fs::write(root.join("Cargo.toml"), "[package\nname = \"demo-api\"\n")
        .expect("manifeste cassé");

    let repair = repair_anchors(&root, false, true).expect("la réparation aboutit");

    assert_eq!(repair.reposees, vec!["routes".to_string()]);
}
```

- [x] **Step 2 : le voir échouer**

Run: `cargo test -p rbs-cli --lib -- --exact tests::doctor_fix_repairs_anchors_despite_a_broken_manifest`
Expected: FAIL — `la réparation aboutit: ... n'est pas un TOML valide`.

- [x] **Step 3 : l'implémentation minimale**

Dans `repair_anchors`, remplacer

```rust
let root = metadata::project_root(directory).map_err(doctor::Error::from)?;
```

par la racine seule, la faute du manifeste laissée au diagnostic qui suit :

```rust
// `racine` et non `project_root` : la doctrine du second — ne rien écrire dans un
// projet dont on ne sait pas lire l'état — vise les commandes qui écrivent *dans* le
// manifeste. Reposer une ancre n'en lit aucune donnée et n'écrit que dans des fichiers
// source qui en portent déjà. S'y arrêter faisait dire à `--fix` le contraire du
// diagnostic du même passage, qui sait nommer un manifeste cassé sans s'y arrêter — et
// faisait tomber avec lui le rapport entier. La faute reste dite, par les contrôles
// qui suivent.
let root = metadata::racine(directory)
    .map_err(doctor::Error::from)?
    .root;
```

- [x] **Step 4 : le voir passer, puis toute la suite**

Run: `cargo test -p rbs-cli --lib` puis `cargo test --workspace`
Expected: PASS, aucun test voisin cassé.

- [x] **Step 5 : clippy, fmt, commit**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add crates/rbs-cli/src/lib.rs docs/superpowers/plans/2026-09-02-reparation-des-ancres-sur-manifeste-illisible.md
git commit
```
