# Affichage du plan — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:test-driven-development.

**Goal:** Montrer, avant toute écriture, ce que la commande fera au projet — un fichier par
ligne, la racine dite une seule fois.

**Architecture:** Un module `plan::rendu`, du même moule que `migrate::rendu` et
`doctor::rendu` : il reçoit une valeur déjà calculée et n'en fait qu'une chaîne. Il lit
`Plan::fichiers()` et non `Plan::actions()` — le TODO demande « fichier par fichier », et
c'est ce qui résout le corollaire relevé à la revue d'E1 : deux insertions de la même ligne
sont deux actions `AFaire`, mais un seul fichier, dont le statut agrégé dit la vérité.

**Tech Stack:** Rust, `console` via `crate::ui`.

**Spec:** `docs/superpowers/specs/2026-08-26-plan-add-design.md` §7.

## Global Constraints

- Branche dédiée `e5-affichage-du-plan`.
- **`--dry-run` n'est pas déclaré ici.** Un flag sans effet est le défaut que le lot E vient
  de corriger sur `--force` : il sera déclaré par E6, qui migre `generate` vers le plan et
  lui donne enfin quelque chose à ne pas faire. E5 reste donc `PARTIEL` jusque-là.
- Le format est validé : puce, chemin aligné, libellé. **Jamais la couleur seule** — la
  sortie doit rester lisible dans un `less`, un fichier de log ou une CI.
- `clippy -D warnings` et `fmt --check` bloquants ; un `///` d'une à trois lignes par item.

## File Structure

- Create: `crates/rbs-cli/src/plan/rendu.rs`
- Modify: `crates/rbs-cli/src/plan/mod.rs` — `mod rendu;` et la ré-exportation

## Le format retenu

```
plan pour /Users/moi/demo-api

  + Dockerfile             créé
  + docker-compose.yml     créé
  ~ Cargo.toml             modifié
  · src/router.rs          inchangé
  ! src/main.rs            conflit — relancer avec --force

  3 fichiers à écrire, 1 inchangé
```

La puce et le libellé se déduisent du couple (`Fichier::avant`, `Fichier::statut`) :

| avant | statut | puce | libellé |
|---|---|---|---|
| `None` | `AFaire` | `+` | `créé` |
| `Some` | `AFaire` | `~` | `modifié` |
| — | `DejaFait` | `·` | `inchangé` |
| — | `Conflit` | `!` | `conflit — relancer avec --force` |

---

### Task 1: Le rendu d'un plan

**Interfaces:**
- Produces: `pub(crate) fn plan(plan: &Plan) -> String`
- Un plan sans fichier rend l'en-tête et `rien à faire`, pas une liste vide.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    #[test]
    fn l_en_tete_porte_la_racine_du_projet_une_seule_fois() { }

    #[test]
    fn un_fichier_absent_est_annonce_cree_et_un_fichier_present_modifie() { }

    #[test]
    fn un_fichier_deja_conforme_est_annonce_inchange() { }

    #[test]
    fn un_conflit_porte_son_remede_sur_sa_ligne() { }

    #[test]
    fn les_libelles_sont_alignes_sur_le_plus_long_chemin() {
        // colonne du libellé identique d'une ligne à l'autre, comptée en caractères :
        // `find` rend des octets et les puces n'en occupent pas le même nombre.
    }

    #[test]
    fn le_pied_compte_les_fichiers_a_ecrire_et_les_inchanges() {
        // accords : « 1 fichier à écrire », « 4 fichiers à écrire, 1 inchangé »
    }

    #[test]
    fn deux_insertions_de_la_meme_ligne_ne_font_qu_une_ligne_de_plan() {
        // le corollaire relevé à la revue d'E1 : deux actions, un fichier, une ligne
    }

    #[test]
    fn un_plan_vide_ne_ment_pas() { }

    #[test]
    fn chaque_etat_se_distingue_sans_la_couleur() {
        // puce et libellé suffisent : la sortie sans TTY ne porte aucun code ANSI
    }
```

Run: `cargo test -p rbs-cli --bins rendu::` → Expected: FAIL.

- [ ] **Step 2: Implémenter, puis vérifier**

Run: `cargo test -p rbs-cli --bins` → Expected: PASS.

- [ ] **Step 3: Vérifications finales**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 4: Commit**

Sujet : `feat(cli): rend le plan lisible, un fichier par ligne`

## Après le plan

Le critère `✓` de la tâche (« `--dry-run` ne modifie rien et affiche le même plan que
l'exécution réelle ») n'est pas prouvable ici : rien n'applique encore un plan. E5 reste
`- [ ]` avec une annotation `PARTIEL`, levée par E6.
