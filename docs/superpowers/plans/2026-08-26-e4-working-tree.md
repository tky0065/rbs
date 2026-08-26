# Vérification du working tree — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development pour chaque tâche. Les étapes se suivent en cochant les `- [ ]`.

**Goal:** Qu'une commande qui modifie un projet refuse de le faire par-dessus des
modifications non commitées, et que `--force` — déclaré depuis D10 mais sans effet — passe
outre. Un `git checkout` doit toujours pouvoir défaire ce que rbs a écrit.

**Architecture:** Un module `git`, qui ne sait qu'une chose : l'état du working tree à une
racine donnée. Il n'interroge pas une bibliothèque — `git status --porcelain` par
`std::process::Command` suffit et parle exactement le langage du développeur. Trois
situations valent « rien à protéger » : hors dépôt Git, `git` introuvable, working tree
propre. Le branchement se fait dans `generate::commande::executer`, où la racine du projet
est déjà résolue, et non dans `main.rs` qui ne la connaît pas.

**Tech Stack:** Rust, `std::process::Command`, `tempfile`.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md`

## Global Constraints

- Branche dédiée `e4-working-tree`, jamais `main`.
- Ne toucher **ni** `TODO.md`, **ni** `crates/rbs-cli/src/plan/`, **ni**
  `crates/rbs-cli/src/ancres.rs`, **ni** `crates/rbs-cli/src/metadata.rs` : les branches E2
  et E3 tournent en parallèle sur ces fichiers.
- Trois décisions arrêtées à la conception, à ne pas rouvrir :
  - Hors dépôt Git, ou `git` introuvable → la commande passe **sans un mot**. Un
    avertissement inutile use la confiance dans les vrais.
  - Seuls les fichiers **suivis** modifiés bloquent. Les fichiers non suivis ne bloquent
    pas : ce sont précisément ceux que le CLI s'apprête à créer.
  - `rbs new` reste hors périmètre : elle crée son propre répertoire.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont
  bloquants.
- Un `///` d'une à trois lignes sur chaque item ; aucun commentaire qui paraphrase la ligne
  suivante.
- Les `-> N passed` des messages de commit se remplacent par le compte réellement affiché.

## File Structure

- Create: `crates/rbs-cli/src/git.rs`
- Modify: `crates/rbs-cli/src/main.rs:50-67` — l'avertissement « `--force` est sans effet »
  disparaît, `force` est transmis
- Modify: `crates/rbs-cli/src/generate/commande.rs` — `Options` gagne `force`, `executer`
  vérifie, `Erreur` gagne une variante

---

### Task 1: Le module `git`

**Files:**
- Create: `crates/rbs-cli/src/git.rs`
- Modify: `crates/rbs-cli/src/main.rs` (déclaration `mod git;`)

**Interfaces:**
- Produces: `pub(crate) fn fichiers_modifies(racine: &Path) -> Vec<String>` — les chemins des
  fichiers suivis modifiés, vide si le working tree est propre, hors dépôt, ou `git` absent.
- Lecture de `git status --porcelain` lancé avec `current_dir(racine)`. Les lignes dont le
  code de statut est `??` sont ignorées. Un code de sortie non nul vaut « rien à protéger ».
- Attention au format : `XY chemin`, et `R  ancien -> nouveau` pour un renommage. Le chemin
  se prend après la troisième colonne ; pour un renommage, garder la destination.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    /// Un dépôt Git jetable, avec un commit initial : `git status` d'un dépôt sans commit
    /// se comporte autrement.
    fn depot() -> TempDir { /* git init -q, config user.*, un fichier, git add, git commit -q */ }

    #[test]
    fn un_working_tree_propre_ne_signale_rien() {
        assert!(fichiers_modifies(depot().path()).is_empty());
    }

    #[test]
    fn un_fichier_suivi_modifie_est_signale() {
        // réécrire le fichier commité -> ["suivi.txt"]
    }

    #[test]
    fn un_fichier_non_suivi_ne_bloque_pas() {
        // créer un fichier jamais ajouté -> vide
    }

    #[test]
    fn un_repertoire_hors_depot_ne_signale_rien() {
        assert!(fichiers_modifies(TempDir::new().unwrap().path()).is_empty());
    }
```

Run: `cargo test -p rbs-cli --bins git::` → Expected: FAIL.

- [ ] **Step 2: Implémenter, puis vérifier**

Run: `cargo test -p rbs-cli --bins git::` → Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/rbs-cli/src/git.rs crates/rbs-cli/src/main.rs
git commit -m "feat(cli): lit l'état du working tree du projet

Une commande qui écrit dans un projet doit pouvoir être défaite par un
`git checkout`. Cela suppose de savoir ce qui, avant elle, n'était pas
commité. `git status --porcelain` parle le langage du développeur ; une
bibliothèque Git serait une dépendance pour une seule question.

Les fichiers non suivis ne comptent pas : ce sont ceux que le CLI s'apprête
à créer.

Vérifications :
- cargo test -p rbs-cli --bins git:: -> <compte réel> passed"
```

---

### Task 2: `rbs generate` refuse un projet sale, `--force` passe outre

**Files:**
- Modify: `crates/rbs-cli/src/generate/commande.rs` (`Options`, `Erreur`, `executer:96-130`)
- Modify: `crates/rbs-cli/src/main.rs:50-67`

**Interfaces:**
- `Options` gagne `pub force: bool`.
- `Erreur` gagne :
  ```rust
  /// Le projet porte des modifications non commitées, qu'une génération rendrait
  /// indiscernables des siennes.
  #[error("le working tree n'est pas propre : {fichiers} — commitez, ou relancez avec --force")]
  WorkingTreeSale { fichiers: String },
  ```
  Nommer les fichiers, pas seulement leur nombre : c'est ce que le développeur doit aller
  voir. Au-delà de cinq, tronquer avec « … et N autres ».
- Dans `executer`, la vérification se place **après** la résolution de `racine` et **avant**
  `nom::valider` : rien ne doit avoir été calculé ni écrit quand elle refuse.
- `main.rs` perd son bloc `if force { ui::warn(...) }` et transmet `force` dans `Options`.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans le module `tests` de `generate/commande.rs`, avec les helpers de projet déjà présents :

```rust
    #[test]
    fn un_projet_sale_refuse_la_generation_et_n_ecrit_rien() {
        // projet généré, git init + commit, puis modification d'un fichier suivi
        // -> Err(Erreur::WorkingTreeSale), et src/<module> n'existe pas
    }

    #[test]
    fn un_projet_sale_accepte_la_generation_avec_force() {
        // même projet, force: true -> Ok, la feature est là
    }

    #[test]
    fn un_projet_hors_depot_git_genere_sans_force() {
        // c'est le cas de tous les tests existants : ils ne doivent pas régresser
    }
```

Run: `cargo test -p rbs-cli --bins` → Expected: FAIL sur les deux premiers.

- [ ] **Step 2: Implémenter, puis vérifier**

Run: `cargo test -p rbs-cli --bins` → Expected: PASS, tous les tests de `generate` compris.

- [ ] **Step 3: Vérifier de bout en bout**

Sur un projet réel, dans un répertoire temporaire :

```bash
cargo run -p rbs-cli -- new demo-api --database-url postgres://rbs:rbs@localhost:5432/demo --yes
# git init, add, commit, puis modifier src/main.rs
cargo run -p rbs-cli -- generate feature notes      # -> refus, code de sortie 1
cargo run -p rbs-cli -- generate feature notes --force   # -> génère
```

Consigner les deux sorties réelles : c'est la preuve du critère `✓`.

- [ ] **Step 4: Vérifications finales**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-cli/src/generate/commande.rs crates/rbs-cli/src/main.rs
git commit -m "feat(cli): refuse de générer par-dessus un working tree sale

`--force` était déclaré depuis la commande generate mais n'avait aucun effet,
et le CLI le disait à chaque appel. Il en a un désormais : sans lui, une
génération lancée sur des modifications non commitées mêlerait ses fichiers
aux leurs, et le `git checkout` de repli n'aurait plus de cible nette.

L'erreur nomme les fichiers en cause plutôt que leur nombre : c'est là que le
développeur doit aller.

Vérifications :
- cargo test --workspace -> <compte réel> passed
- bout en bout : projet sale -> refus code 1 ; avec --force -> feature générée"
```

## Après le plan

Rapporter à l'orchestrateur la commande de preuve et son résultat réel pour le critère `✓`
(dépôt sale → refus ; avec `--force` → exécution), y compris la sortie du bout en bout.
Ne pas cocher `TODO.md`.
