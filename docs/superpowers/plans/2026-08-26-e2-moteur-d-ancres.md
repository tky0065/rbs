# Moteur d'ancres — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development pour chaque tâche. Les étapes se suivent en cochant les `- [ ]`.

**Goal:** Qu'une insertion refusée dise la vérité et donne le remède : le fichier absent se
distingue de l'ancre disparue, et l'échec affiche le bloc à recoller comme `doctor` le fait.

**Architecture:** Rien de neuf. `ancres::inserer` fait déjà lecture, insertion avant la
balise fermante et idempotence ; `plan::Constructeur::inserer` l'appelle. Le travail est de
corriger une variante d'erreur héritée trop large et de remonter dans `crate::ancres` le
bloc à recoller, aujourd'hui privé dans `doctor/ancres.rs`.

**Tech Stack:** Rust, `thiserror`, `tempfile` (déjà en dev-dependency).

**Spec:** `docs/superpowers/specs/2026-08-26-plan-add-design.md`, section « Deux points
relevés à la revue de ce module ».

## Global Constraints

- Branche dédiée `e2-moteur-ancres`, jamais `main`.
- Ne toucher **ni** `TODO.md` **ni** `crates/rbs-cli/src/metadata.rs` : `TODO.md` est coché
  par l'orchestrateur après intégration, et `metadata.rs` appartient à la branche E3 qui
  tourne en parallèle. Un conflit y serait gratuit.
- `plan/mod.rs` est également touché par E3, mais dans `patcher` : ne pas reformater le
  fichier, ne modifier que `inserer` et l'enum `Erreur`.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont
  bloquants.
- Un `///` d'une à trois lignes sur chaque item ; aucun commentaire qui paraphrase la ligne
  suivante.
- Les `-> N passed` des messages de commit se remplacent par le compte réellement affiché.

## File Structure

- Modify: `crates/rbs-cli/src/ancres.rs` — `bloc()` y monte, `pub(crate)`
- Modify: `crates/rbs-cli/src/doctor/ancres.rs` — devient appelant, perd son `bloc()` privé
- Modify: `crates/rbs-cli/src/plan/mod.rs` — variante `FichierAbsent`, `inserer` la produit

---

### Task 1: Le bloc à recoller appartient aux ancres

`doctor/ancres.rs:52` sait fabriquer le bloc à coller pour une ancre disparue. L'échec de
planification doit dire la même chose ; deux copies divergeraient.

**Files:**
- Modify: `crates/rbs-cli/src/ancres.rs`
- Modify: `crates/rbs-cli/src/doctor/ancres.rs:34,51-54`

**Interfaces:**
- Produces: `pub(crate) fn bloc(ancre: &Ancre) -> String` dans `crate::ancres` — ou, mieux,
  une méthode `Ancre::bloc(&self) -> String`, cohérente avec `ouverture()` et `fermeture()`
  qui sont déjà des méthodes. Choisir la méthode.

- [ ] **Step 1: Écrire le test qui échoue**

Dans le module `tests` de `crates/rbs-cli/src/ancres.rs` :

```rust
    #[test]
    fn le_bloc_a_recoller_porte_les_deux_balises_de_l_ancre() {
        assert_eq!(ROUTES.bloc(), "// <rbs:routes>\n// </rbs:routes>");
    }
```

Run: `cargo test -p rbs-cli --bins ancres::` → Expected: FAIL (méthode inexistante).

- [ ] **Step 2: Implémenter**

Ajouter `Ancre::bloc`, puis remplacer l'appel de `doctor/ancres.rs:34` par `a.bloc()` et
supprimer la fonction privée `bloc`.

Run: `cargo test -p rbs-cli --bins` → Expected: PASS, `doctor` compris.

- [ ] **Step 3: Commit**

```bash
git add crates/rbs-cli/src/ancres.rs crates/rbs-cli/src/doctor/ancres.rs
git commit -m "refactor(cli): rattache le bloc à recoller à l'ancre qu'il répare

Le diagnostic n'est plus seul à devoir dire comment recoller une ancre : la
planification bute sur la même absence et doit donner le même remède. Une
seconde copie du format aurait divergé de la première.

Vérifications :
- cargo test -p rbs-cli --bins -> <compte réel> passed"
```

---

### Task 2: Un fichier absent n'est pas une ancre absente

`Constructeur::inserer` sur `migration/src/lib.rs` inexistant rend « ancre `// <rbs:migrations>`
introuvable dans migration/src/lib.rs ». Le lecteur cherche une balise dans un fichier qui
n'est pas là. C'est le défaut déjà corrigé entre `ManifesteAbsent` et `Metadonnees`.

**Files:**
- Modify: `crates/rbs-cli/src/plan/mod.rs:64-98` (enum `Erreur`) et `inserer`
- Test: `crates/rbs-cli/src/plan/mod.rs`, module `tests`

**Interfaces:**
- Produces: `Erreur::FichierAbsent { chemin: String }`, `#[error("{chemin} est introuvable")]`,
  documentée comme distincte d'`Ancre` : le fichier manque, pas seulement la balise.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    #[test]
    fn inserer_dans_un_fichier_absent_nomme_le_fichier_et_non_l_ancre() {
        let (_temp, racine) = projet_vide();
        let mut plan = Constructeur::nouveau(racine);

        let erreur = plan
            .inserer(crate::ancres::ROUTES, &[".merge(users::routes())".to_string()])
            .expect_err("le fichier n'existe pas");

        assert!(matches!(erreur, Erreur::FichierAbsent { .. }), "{erreur:?}");
        assert!(erreur.to_string().contains("src/router.rs"));
        assert!(!erreur.to_string().contains("<rbs:routes>"));
    }

    #[test]
    fn inserer_dans_un_fichier_present_mais_sans_ancre_reste_une_erreur_d_ancre() {
        // fichier écrit sans ses balises -> Erreur::Ancre, et le message cite `<rbs:routes>`
    }
```

Reprendre les helpers du module `tests` existant (`projet`, `Constructeur::nouveau`) plutôt
que d'en écrire de nouveaux ; en ajouter un pour le projet sans le fichier visé.

Run: `cargo test -p rbs-cli --bins plan::` → Expected: FAIL sur le premier, PASS sur le second.

- [ ] **Step 2: Implémenter**

Dans `inserer`, distinguer avant d'appeler `ancres::inserer` : `etats.courant` vaut `None`
quand le fichier n'existe pas → `Erreur::FichierAbsent`. Vérifier que `etats()` ne confond
pas « absent » et « illisible » : une `io::Error` autre que `NotFound` doit rester `Acces`.

Run: `cargo test -p rbs-cli --bins plan::` → Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/rbs-cli/src/plan/mod.rs
git commit -m "fix(cli): distingue le fichier absent de l'ancre disparue

Planifier une insertion dans un fichier qui n'existe pas se plaignait d'une
balise introuvable, en nommant un fichier que le projet n'a pas. Le lecteur
cherchait une ancre là où c'est le fichier entier qui manque.

Vérifications :
- cargo test -p rbs-cli --bins plan:: -> <compte réel> passed"
```

---

### Task 3: Les deux critères de la tâche, prouvés au niveau du plan

`✓ Test : ancre absente → aucune écriture, code de sortie non nul, bloc affiché.`
`✓ Test : insertion déjà présente → aucune modification.`

Le second est déjà vrai dans `ancres::inserer` mais n'est prouvé que sur des chaînes. Les
deux critères parlent du comportement observable : rien sur le disque, un bloc à l'écran.

**Files:**
- Test: `crates/rbs-cli/src/plan/mod.rs`, module `tests`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    #[test]
    fn une_ancre_absente_laisse_le_projet_intact_et_donne_le_bloc_a_recoller() {
        // projet dont src/router.rs a perdu ses deux balises
        // empreinte du répertoire avant / après (reprendre l'helper de
        // `planifier_ne_modifie_pas_le_repertoire_du_projet`)
        // -> Err(Erreur::Ancre(absente)), empreinte inchangée,
        //    et `absente.ancre.bloc()` porte les deux balises à recoller
    }

    #[test]
    fn inserer_deux_fois_la_meme_ligne_ne_change_rien_la_seconde_fois() {
        // deux plans successifs : le second a le statut DejaFait et un `apres` égal à
        // l'`avant`. Statut::DejaFait est le nom du « aucune modification » du critère.
    }
```

Le « code de sortie non nul » n'appartient pas à cette tâche : aucune commande n'appelle
encore le plan. `rbs generate` prouve déjà ce point (D12) ; le noter dans le rapport final
plutôt que d'inventer un appelant.

- [ ] **Step 2: Faire passer**

Aucune implémentation attendue si Task 2 est faite — si un test échoue, c'est un vrai
défaut du moteur, à corriger dans `ancres.rs` ou `plan/mod.rs`.

Run: `cargo test -p rbs-cli --bins` puis `cargo test --workspace`
Expected: PASS partout.

- [ ] **Step 3: Vérifications finales**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 4: Commit**

```bash
git add crates/rbs-cli/src/plan/mod.rs
git commit -m "test(cli): prouve l'idempotence et l'innocuité du moteur d'ancres

Les deux garanties n'étaient éprouvées que sur des chaînes. Elles portent en
réalité sur le disque : une ancre disparue ne doit rien écrire du tout, et une
ligne déjà montée ne doit pas être réécrite.

Vérifications :
- cargo test --workspace -> <compte réel> passed
- cargo clippy --workspace --all-targets -- -D warnings -> propre"
```

## Après le plan

Rapporter à l'orchestrateur : la commande de preuve et son compte réel pour chacun des deux
critères `✓`, et le fait que le « code de sortie non nul » reste porté par `rbs generate`.
Ne pas cocher `TODO.md`.
