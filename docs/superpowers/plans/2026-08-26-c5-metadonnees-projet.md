# Métadonnées projet — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** lire et mettre à jour `[package.metadata.rbs]`, l'unique état d'un projet rbs, avant qu'une commande n'ait besoin de s'en servir.

**Architecture:** un module `metadata` de deux fonctions, `lire` et `ajouter_feature`, et rien d'autre. C'est la première des cinq étapes de §4.4 (`LIRE`) et la dernière (`APPLIQUER`) réduites à ce que le manifeste porte ; le plan, l'affichage et la restauration appartiennent aux commandes, qui n'existent pas encore.

L'édition passe par `toml_edit` sur un `DocumentMut`, jamais par une sérialisation complète : un `Cargo.toml` est un fichier que le développeur relit et commente, et `rbs add` s'y ajoute au milieu de ses propres dépendances. Sérialiser reviendrait à lui reformater son manifeste à chaque commande.

`ajouter_feature` sort sans toucher au fichier quand la feature est déjà là. L'idempotence de §4.4 est ici et pas ailleurs : elle porte sur `metadata.rbs`, pas sur la présence des fichiers d'une feature, et une commande qui l'appellerait deux fois ne doit rien produire la seconde fois — pas même une réécriture à contenu identique, qui salirait le working tree.

L'absence de la section est l'erreur « ce répertoire n'est pas un projet rbs ». Elle nomme le manifeste : l'utilisateur qui la déclenche s'est presque toujours trompé de répertoire, et c'est le chemin qui le lui dit.

**Ce que la tâche ne peut pas prouver :** que les métadonnées d'un projet *généré* se relisent. `rbs new` n'existe pas avant C7. Le plus proche est de rendre `templates/project/Cargo.toml.jinja` avec le `Renderer` et d'écrire le résultat dans un répertoire temporaire — même contenu, même chemin de code de lecture, seul le déroulement complet du squelette manque.

**Tech Stack:** toml_edit 0.25, thiserror 2, tempfile en dev.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §4.2, §4.4

## Global Constraints

- Aucune commande du CLI n'est implémentée, `rbs new` pas même partiellement.
- Aucun fichier de `crates/rbs-core/` ni de `templates/project/` n'est modifié.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` restent propres.

---

### Task 1 : les tests

**Files:**
- Create: `crates/rbs-cli/src/metadata.rs`
- Modify: `crates/rbs-cli/Cargo.toml` (`toml_edit`, `thiserror`, `tempfile` en dev)
- Modify: `crates/rbs-cli/src/main.rs` (déclaration du module)

- [ ] **Step 1 : écrire les tests d'abord**

Quatre tests, contre le seul module qu'ils décrivent : les métadonnées d'un manifeste obtenu en rendant `templates/project/Cargo.toml.jinja` puis en l'écrivant dans un répertoire temporaire donnent la version `0.1.0` et la feature `health` ; deux appels de `ajouter_feature` avec la même feature ne laissent qu'une entrée ; un manifeste sans `[package.metadata.rbs]` échoue avec un message portant le chemin du fichier ; l'écriture laisse intacts les commentaires et l'ordre des autres sections.

Run: `cargo test -p rbs-cli`
Expected: échec — le module `metadata` n'existe pas.

---

### Task 2 : le module

**Files:**
- Modify: `crates/rbs-cli/src/metadata.rs`

- [ ] **Step 1 : le type et les erreurs**

`Metadonnees { version, features }`, et une énumération d'erreurs `thiserror` distinguant l'accès au fichier, le TOML invalide, la section absente et un champ absent ou mal typé. Chaque variante porte le chemin du manifeste.

- [ ] **Step 2 : `lire`**

Parsing en `DocumentMut`, descente `package.metadata.rbs`, puis les deux champs. Un `features` contenant autre chose que des chaînes est une erreur de champ, pas une entrée silencieusement ignorée.

- [ ] **Step 3 : `ajouter_feature`**

Même descente en mutable, sortie immédiate si la feature est déjà présente, sinon un `push` dans le tableau et une réécriture du document rendu.

- [ ] **Step 4 : prouver le critère**

Run: `cargo test -p rbs-cli`
Expected: tout vert, dont les quatre tests de métadonnées.

---

### Task 3 : vérification et commit

- [ ] **Step 1 : vérifier l'ensemble**

Run: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`
Expected: tout vert, aucun warning.

- [ ] **Step 2 : commit**

Message : `feat(cli): lit et met à jour les métadonnées rbs d'un projet`, corps portant le *pourquoi* de `toml_edit` et de l'idempotence, puis un intertitre `Vérifications :`.
