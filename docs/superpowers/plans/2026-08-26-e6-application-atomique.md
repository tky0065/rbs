# Application atomique — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:test-driven-development.

**Goal:** Qu'un plan s'applique en entier ou pas du tout, et que `rbs generate` cesse
d'écrire par lui-même pour passer par lui — ce qui lui donne son `--dry-run`.

**Architecture:** Un module `plan::application`. Il écrit les fichiers que le plan a
calculés, en mémorisant ce qu'ils étaient ; sur le premier échec, il défait ce qu'il a
fait. Le contenu d'origine est déjà dans `Fichier::avant` : le rollback n'a rien à relire.

`generate::commande::executer` perd `monter` et `ecrire` : le premier réimplémente le
chaînage d'insertions que `Constructeur::inserer` fait déjà, le second l'écriture que
l'application fait. La commande devient : rendre, planifier, afficher, appliquer.

**Tech Stack:** Rust, `std::fs`, `tempfile`.

**Spec:** `docs/superpowers/specs/2026-08-26-plan-add-design.md` §7.

## Global Constraints

- Branche dédiée `e6-application-atomique`.
- Le `#![allow(dead_code)]` en tête de `plan/mod.rs` doit **sauter** : le module a désormais
  un appelant. S'il reste nécessaire, c'est que la migration est incomplète.
- L'ordre d'écriture est celui de `Plan::fichiers()`, qui est celui où les actions ont été
  planifiées. Un rollback ne peut être correct que s'il suit l'ordre inverse.
- `clippy -D warnings` et `fmt --check` bloquants ; un `///` d'une à trois lignes par item.

## File Structure

- Create: `crates/rbs-cli/src/plan/application.rs`
- Modify: `crates/rbs-cli/src/plan/mod.rs` — `mod application;`, `allow(dead_code)` retiré
- Modify: `crates/rbs-cli/src/generate/commande.rs` — `monter` et `ecrire` supprimés
- Modify: `crates/rbs-cli/src/cli.rs`, `crates/rbs-cli/src/main.rs` — `--dry-run`

---

### Task 1: Appliquer, ou ne rien laisser

**Interfaces:**
- `pub(crate) fn appliquer(plan: &Plan, force: bool) -> Result<Vec<String>, Erreur>` — les
  chemins réellement écrits.
- Un fichier `DejaFait` n'est pas réécrit : l'application ne touche pas à ce qu'elle n'a
  pas à changer.
- Un fichier `Conflit` sans `force` fait **refuser le plan entier avant la première
  écriture**. Le refus nomme les fichiers en cause.
- Les répertoires que l'application a créés sont défaits eux aussi, dans l'ordre inverse
  et seulement s'ils sont restés vides : un rollback qui laisse `src/notes/` derrière lui
  n'a pas restauré grand-chose.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    #[test]
    fn un_plan_sans_conflit_ecrit_tous_ses_fichiers() { }

    #[test]
    fn un_fichier_deja_conforme_n_est_pas_reecrit() {
        // mtime, ou plus simplement : le chemin n'est pas dans les chemins rendus
    }

    #[test]
    fn un_conflit_fait_refuser_le_plan_avant_la_premiere_ecriture() {
        // empreinte du répertoire inchangée, erreur nommant le fichier
    }

    #[test]
    fn un_conflit_force_est_ecrase() { }

    /// Le critère de la tâche.
    #[test]
    fn un_echec_sur_la_quatrieme_action_annule_les_trois_premieres() {
        // Trois fichiers écrivables — deux créations et une modification, pour éprouver
        // les deux formes de restauration — puis un quatrième dont le parent est occupé
        // par un fichier régulier : `create_dir_all` y échoue pour de vrai.
        // Après l'échec : les deux créations ont disparu, la modification a repris son
        // contenu d'origine, et l'empreinte du répertoire égale celle d'avant.
    }

    #[test]
    fn un_repertoire_cree_puis_annule_ne_reste_pas_derriere() { }
```

Le `Plan` se construit ici littéralement (`Plan { racine, actions, fichiers }`) : le module
est un descendant de `plan`, et le rendu comme l'application ne dépendent que des fichiers.

Run: `cargo test -p rbs-cli --bins application::` → Expected: FAIL.

- [ ] **Step 2: Implémenter, puis vérifier**

- [ ] **Step 3: Commit** — `feat(cli): applique un plan en entier, ou pas du tout`

---

### Task 2: `generate` passe par le plan

**Files:**
- Modify: `crates/rbs-cli/src/generate/commande.rs`

`executer` devient : résoudre la racine, valider, vérifier le working tree, rendre les
fichiers, **planifier**, afficher, appliquer. `monter` et `ecrire` disparaissent ;
`metadata::ajouter_feature` cède la place à `PatchToml::InscrireFeature`.

Deux conséquences à vérifier plutôt qu'à supposer :
- `Erreur` perd peut-être des variantes et en gagne une (`Plan`, `Application`). Ne pas
  garder de variante devenue inatteignable.
- Le test `la_feature_est_inscrite_dans_les_metadonnees_du_projet` doit continuer de passer
  sans modification : c'est lui qui prouve que le patch de manifeste survit à la migration.

- [ ] **Step 1: Faire porter la migration par les tests existants**

Les tests de `generate::commande` sont le harnais : ils doivent passer avant et après, à
l'exception de ceux qui vérifient une sortie qui change. Lancer d'abord, noter le compte.

- [ ] **Step 2: Migrer, puis vérifier**

Run: `cargo test -p rbs-cli` → Expected: PASS, même compte ou plus.

- [ ] **Step 3: Retirer le `allow(dead_code)` de `plan/mod.rs`**

Run: `cargo clippy --workspace --all-targets -- -D warnings` → Expected: propre. S'il
reste du code mort, c'est une pièce du plan que la migration n'utilise pas : le dire.

- [ ] **Step 4: Commit** — `refactor(cli): fait passer la génération par le plan`

---

### Task 3: `--dry-run`, et le critère d'E5

**Files:**
- Modify: `crates/rbs-cli/src/cli.rs` — `--dry-run` sur `generate crud` et `generate feature`
- Modify: `crates/rbs-cli/src/main.rs`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    /// Le critère d'E5 : le plan affiché est celui qui sera exécuté.
    #[test]
    fn dry_run_affiche_le_meme_plan_que_l_execution_et_ne_touche_a_rien() {
        // même projet, deux appels : l'un en dry-run, l'autre non.
        // 1. les deux rendus sont égaux caractère pour caractère
        // 2. l'empreinte du répertoire est inchangée après le dry-run
        // 3. après l'exécution réelle, chaque fichier annoncé existe avec le contenu annoncé
    }
```

- [ ] **Step 2: Implémenter, puis vérifier**

- [ ] **Step 3: Bout en bout**

```bash
rbs generate crud articles --fields "titre:string" --dry-run   # -> plan, rien d'écrit
rbs generate crud articles --fields "titre:string"             # -> même plan, puis écriture
```

Consigner les deux sorties réelles.

- [ ] **Step 4: Vérifications finales**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 5: Commit** — `feat(cli): ajoute --dry-run à la génération`

## Après le plan

E6 et E5 deviennent cochables ensemble : E5 tenait à ce que `--dry-run` existe quelque part.
