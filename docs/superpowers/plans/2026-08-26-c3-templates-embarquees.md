# Templates embarquées — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** que le binaire `rbs` porte l'arborescence de `templates/project/` en lui, et sache lui substituer un répertoire du disque quand `--template-dir` est passé.

**Architecture:** un énuméré `Source` à deux variantes — `Embarquees`, adossée à un `Dir` `include_dir!`, et `Repertoire(PathBuf)` — et une seule méthode qui rend un `Vec<Fichier>`, chaque `Fichier` portant sa destination et la source non rendue. Les deux variantes passent par le même point de sortie, donc le développement de rbs et l'installation d'un utilisateur exercent le même code.

Le retrait du suffixe `.jinja` vit là, et nulle part ailleurs. C'est le seul endroit du CLI qui parcourt les templates ; l'implémenter chez l'appelant reviendrait à le réimplémenter dans `rbs new`, puis dans `rbs add`, puis dans `rbs generate`. La convention est posée au §1 du design du squelette et elle n'est pas cosmétique : un fichier réellement nommé `.gitignore` sous `templates/project/` retirerait des fichiers du suivi du dépôt rbs lui-même.

La liste est triée par destination. `include_dir` et `fs::read_dir` ne rendent pas leurs entrées dans le même ordre, et `read_dir` n'en garantit aucun : sans tri, un test comparant les deux sources serait vert sur une machine et rouge sur une autre.

Les erreurs sont des `io::Error` dont le message nomme le chemin en cause. Un `--template-dir` mal saisi est l'erreur la plus probable de ce flag, et « No such file or directory » sans chemin ne la corrige pas.

**Tech Stack:** include_dir 0.7, tempfile (tests).

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §4.3, `docs/superpowers/specs/2026-08-26-squelette-projet-design.md` §1

## Global Constraints

- Aucune commande du CLI n'est implémentée : `rbs new` est C7, et le critère « génère un projet » ne peut pas être atteint avant elle. Ce que la tâche prouve, c'est que les templates sont dans le binaire et lisibles sans le dépôt.
- Les templates de `templates/project/` ne sont pas touchées, ni `cli.rs` où `--template-dir` est déjà déclaré.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` restent propres.

---

### Task 1 : la source de templates

**Files:**
- Modify: `crates/rbs-cli/Cargo.toml` (`include_dir`, `tempfile` en dev)
- Modify: `crates/rbs-cli/src/templates.rs` (`Source`, `Fichier`, tests existants regroupés)
- Modify: `crates/rbs-cli/src/main.rs` (le module n'est plus `#[cfg(test)]`)

- [ ] **Step 1 : écrire les tests d'abord**

Quatre tests : la source embarquée restitue les 14 fichiers du squelette avec leurs chemins de sortie ; aucune destination ne porte le suffixe `.jinja` ; un `--template-dir` pointant sur un répertoire temporaire prend le pas sur l'embarqué ; un `--template-dir` inexistant échoue avec un message nommant le chemin.

Run: `cargo test -p rbs-cli`
Expected: échec de compilation — `Source` et `Fichier` n'existent pas encore.

- [ ] **Step 2 : embarquer l'arborescence**

`include_dir!("$CARGO_MANIFEST_DIR/../../templates/project")` dans un `static`, et un constructeur prenant l'`Option<&Path>` de `--template-dir`.

- [ ] **Step 3 : le parcours**

Une descente récursive par variante, un point de sortie commun qui retire le suffixe `.jinja` et trie par destination.

- [ ] **Step 4 : vérifier l'ensemble**

Run: `cargo test -p rbs-cli`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 5 : commit**

Message : `feat(cli): embarque les templates du squelette dans le binaire`, corps portant le *pourquoi* du point de sortie unique et un intertitre `Vérifications :`.
