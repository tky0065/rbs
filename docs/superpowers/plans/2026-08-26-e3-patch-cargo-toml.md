# Patch de `Cargo.toml` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development pour chaque tâche. Les étapes se suivent en cochant les `- [ ]`.

**Goal:** Qu'un plan sache ajouter une dépendance et activer une feature sur une dépendance
existante, sans qu'un commentaire ni un alignement du manifeste ne bouge ailleurs.

**Architecture:** `metadata.rs` porte déjà `inscrire_feature`, la partie pure du patch de
`[package.metadata.rbs]`. Deux fonctions du même moule s'y ajoutent, en `toml_edit` : texte
en entrée, `Option<String>` en sortie, `None` quand il n'y a rien à faire. `PatchToml` gagne
deux variantes et `Constructeur::patcher` les route. Aucune écriture disque nouvelle.

**Tech Stack:** Rust, `toml_edit 0.25`, `thiserror`.

**Spec:** `docs/superpowers/specs/2026-08-26-plan-add-design.md`

## Global Constraints

- Branche dédiée `e3-patch-cargo-toml`, jamais `main`.
- Ne toucher **ni** `TODO.md` **ni** `crates/rbs-cli/src/ancres.rs` **ni**
  `crates/rbs-cli/src/doctor/` : ils appartiennent à la branche E2 qui tourne en parallèle.
- Dans `plan/mod.rs`, ne modifier que `patcher` et, si nécessaire, l'enum `Erreur` — E2
  touche `inserer` dans le même fichier. Ne pas reformater.
- `ajouter_feature` (`metadata.rs:145`) garde son contrat : `generate/commande.rs:124`
  l'appelle. Ne pas la casser en factorisant.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont
  bloquants.
- Un `///` d'une à trois lignes sur chaque item ; aucun commentaire qui paraphrase la ligne
  suivante.
- Les `-> N passed` des messages de commit se remplacent par le compte réellement affiché.

## File Structure

- Modify: `crates/rbs-cli/src/metadata.rs` — deux fonctions pures et leurs variantes d'erreur
- Modify: `crates/rbs-cli/src/plan/action.rs:29-34` — `PatchToml` gagne deux variantes
- Modify: `crates/rbs-cli/src/plan/mod.rs` — `patcher` route les trois variantes

---

### Task 1: Ajouter une dépendance

**Files:**
- Modify: `crates/rbs-cli/src/metadata.rs`

**Interfaces:**
- Produces: `pub fn ajouter_dependance(texte: &str, dep: &Dependance, nom: &str) -> Result<Option<String>, Erreur>`
- Produces: `pub struct Dependance { pub nom: String, pub version: String, pub features: Vec<String> }`
- `Ok(None)` si la dépendance est déjà déclarée **avec au moins ce qui est demandé** —
  décider et documenter : une version différente est un conflit, pas un silence.
- La table `[dependencies]` peut être absente d'un manifeste : la créer.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    const MANIFESTE: &str = r#"[package]
name = "demo"
version = "0.1.0"

# les dépendances du projet
[dependencies]
axum = "0.9"       # le serveur
tokio = { version = "1", features = ["macros"] }
"#;

    #[test]
    fn une_dependance_absente_s_ajoute_sans_deplacer_le_reste() {
        let rendu = ajouter_dependance(
            MANIFESTE,
            &Dependance { nom: "redis".into(), version: "0.32".into(), features: vec![] },
            "Cargo.toml",
        )
        .expect("le manifeste est valide")
        .expect("la dépendance est absente");

        assert!(rendu.contains(r#"redis = "0.32""#));
        assert!(rendu.contains("# les dépendances du projet"));
        assert!(rendu.contains(r#"axum = "0.9"       # le serveur"#));
    }

    #[test]
    fn une_dependance_deja_declaree_ne_produit_aucun_texte() { /* -> None */ }

    #[test]
    fn une_dependance_avec_features_se_declare_en_table_inline() {
        // redis = { version = "0.32", features = ["tokio-comp"] }
    }

    #[test]
    fn un_manifeste_sans_table_dependencies_en_recoit_une() { /* … */ }
```

Run: `cargo test -p rbs-cli --bins metadata::` → Expected: FAIL.

- [ ] **Step 2: Implémenter, puis vérifier**

Run: `cargo test -p rbs-cli --bins metadata::` → Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/rbs-cli/src/metadata.rs
git commit -m "feat(cli): ajoute une dépendance au manifeste sans le reformater

Une feature installée dans un projet existant apporte ses dépendances. Le
manifeste appartient au développeur : ses commentaires et ses alignements
traversent l'ajout intacts, faute de quoi le diff de `rbs add` serait illisible.

Vérifications :
- cargo test -p rbs-cli --bins metadata:: -> <compte réel> passed"
```

---

### Task 2: Activer une feature sur une dépendance existante

Cas réel : `sea-orm` est là, `rbs add auth` veut sa feature `with-uuid`. Une dépendance
déclarée en chaîne (`axum = "0.9"`) doit devenir une table inline sans perdre son commentaire
de fin de ligne.

**Files:**
- Modify: `crates/rbs-cli/src/metadata.rs`

**Interfaces:**
- Produces: `pub fn ajouter_feature_a_dependance(texte: &str, dep: &str, feature: &str, nom: &str) -> Result<Option<String>, Erreur>`
- `Ok(None)` si la feature y est déjà. Dépendance absente → `Err`, variante nommée : c'est
  une erreur de programmation de l'appelant, pas une situation à contourner.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    #[test]
    fn une_feature_s_ajoute_a_une_dependance_deja_en_table_inline() {
        // tokio: ["macros"] -> ["macros", "rt-multi-thread"]
    }

    #[test]
    fn une_dependance_en_chaine_devient_une_table_inline_en_gardant_son_commentaire() {
        // axum = "0.9"  # le serveur
        // -> axum = { version = "0.9", features = ["macros"] }  # le serveur
    }

    #[test]
    fn une_feature_deja_active_ne_produit_aucun_texte() { /* -> None */ }

    #[test]
    fn une_dependance_absente_est_refusee() { /* -> Err */ }
```

Run: `cargo test -p rbs-cli --bins metadata::` → Expected: FAIL.

- [ ] **Step 2: Implémenter, puis vérifier**

Run: `cargo test -p rbs-cli --bins metadata::` → Expected: PASS.

- [ ] **Step 3: Commit** — sujet : `feat(cli): active une feature sur une dépendance déjà déclarée`

---

### Task 3: Le critère `✓` — préservation à l'octet près

`✓ Test : commentaires et formatage du fichier préservés à l'octet près hors zone modifiée.`

C'est une obligation distincte : un test qui compare **tout le fichier** moins la zone
touchée, et non trois `contains` bien choisis.

**Files:**
- Test: `crates/rbs-cli/src/metadata.rs`, module `tests`

- [ ] **Step 1: Écrire le test**

Manifeste témoin riche : commentaires de tête, ligne vide, commentaires de fin de ligne,
alignements irréguliers, table `[dev-dependencies]`, section finale
`[package.metadata.rbs]`. Après chacun des trois patchs (dépendance, feature, feature rbs),
comparer ligne à ligne l'original et le rendu, et affirmer que **seules** les lignes
attendues diffèrent :

```rust
    fn lignes_modifiees(avant: &str, apres: &str) -> Vec<String> { /* diff naïf, suffit ici */ }

    #[test]
    fn un_patch_ne_modifie_que_sa_propre_ligne() {
        // assert_eq!(lignes_modifiees(TEMOIN, &rendu), ["…"]);
    }
```

Run: `cargo test -p rbs-cli --bins metadata::` → Expected: PASS, ou FAIL révélant un vrai
défaut de préservation à corriger.

- [ ] **Step 2: Commit** — sujet : `test(cli): prouve qu'un patch de manifeste ne déborde pas de sa ligne`

---

### Task 4: Le plan sait commander les trois patchs

**Files:**
- Modify: `crates/rbs-cli/src/plan/action.rs:29-34`
- Modify: `crates/rbs-cli/src/plan/mod.rs` (`patcher`, ~ligne 178)

**Interfaces:**
- `PatchToml::AjouterDependance(Dependance)` et `PatchToml::AjouterFeatureADependance { dependance: String, feature: String }`
- `patcher` remplace son `let PatchToml::InscrireFeature(feature) = &patch;` par un `match`
  sur les trois variantes. Le calcul du statut, la projection et l'`Erreur::ManifesteAbsent`
  restent inchangés : ils ne dépendent pas de la variante.

- [ ] **Step 1: Écrire les tests qui échouent**

Dans le module `tests` de `plan/mod.rs`, sur le modèle des tests de `patcher` existants :
un plan qui ajoute une dépendance a le statut `AFaire` et un `apres` qui la contient ; le
même plan rejoué sur le résultat a le statut `DejaFait`.

Run: `cargo test -p rbs-cli --bins plan::` → Expected: FAIL.

- [ ] **Step 2: Implémenter, puis vérifier**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 3: Commit** — sujet : `feat(cli): étend le plan aux deux patchs de dépendance`

## Après le plan

Rapporter à l'orchestrateur la commande de preuve et son compte réel pour le critère `✓`
(préservation à l'octet près). Ne pas cocher `TODO.md`.
