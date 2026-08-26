# Tests du mécanisme `add` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:test-driven-development.

**Goal:** Que les quatre garanties du mécanisme — idempotence, ancre manquante, dépôt
sale, rollback — soient éprouvées par la commande telle que l'utilisateur la lance, et
qu'elles tournent sur chaque PR.

**Architecture:** E2 à E6 ont prouvé chaque garantie au niveau de son module, sur des
répertoires temporaires construits à la main. E9 les rejoue une fois de plus, par le
binaire, sur un projet issu de `rbs new`. Ce n'est pas une redite : un test unitaire
prouve que le moteur sait faire, un test d'intégration prouve que la commande le fait.

**Décision de conception :** `add` n'écrit dans aucune ancre. `planifier()` ne fait que
créer des fichiers et patcher `Cargo.toml` ; ni `docker` ni `ci` n'apportent de code Rust,
ce que le TODO assume délibérément. Le scénario « ancre manquante » est donc éprouvé sur
`rbs generate crud`, qui écrit dans les cinq ancres et emploie le **même**
`plan::Constructeur::inserer`. Substitution validée par le user le 2026-08-26 ; elle sera
écrite dans la preuve du TODO plutôt que tue.

**Tech Stack:** Rust, `assert_cmd`, `tempfile`.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §4.4 (séquence et idempotence),
§5.5 niveau 3 (CLI en CI), §7 (ancre supprimée).

## Global Constraints

- Branche dédiée `e9-tests-add`.
- **Aucun `#[ignore]`.** Ces quatre tests ne compilent pas le projet généré et ne
  demandent pas Docker : ils doivent tourner dans `cargo test --workspace`, faute de quoi
  le critère « couverts en CI » n'est pas rempli.
- Le fichier vit dans `tests/` et non dans `src/` : `CARGO_BIN_EXE_rbs` n'est défini que
  pour les tests d'intégration.
- Le rollback se provoque sans injection : un **répertoire** posé à l'emplacement d'un
  fichier que le plan doit écrire fait échouer l'écriture sur toute plateforme.
  `Source::fichiers()` trie par destination — `Dockerfile` précède `docker-compose.yml`,
  le piège va donc sur le second pour qu'au moins une écriture ait eu lieu avant l'échec.

## File Structure

- Create: `crates/rbs-cli/tests/integration_add.rs`

---

### Task 1: Les quatre scénarios

**Interfaces:** aucune signature nouvelle. Le test consomme le binaire, pas la crate.

- [ ] **Step 1: Écrire les quatre tests qui échouent**

```rust
    #[test]
    fn installer_deux_fois_la_meme_feature_ne_produit_rien_la_seconde() {
        // `add docker`, puis `add docker` : code 0, empreinte du projet identique
    }

    #[test]
    fn une_ancre_supprimee_refuse_l_ecriture_et_affiche_le_bloc_a_coller() {
        // `<rbs:routes>` retirée de router.rs → `g crud` refuse, code ≠ 0,
        // le bloc est affiché, `src/notes` n'existe pas
    }

    #[test]
    fn un_working_tree_sale_refuse_sans_force_et_passe_avec() {
        // fichier suivi modifié → refus code 1, rien d'écrit ; --force → Dockerfile écrit
    }

    #[test]
    fn un_echec_en_cours_d_application_restaure_les_fichiers_deja_ecrits() {
        // docker-compose.yml en répertoire → échec ; Dockerfile absent,
        // Cargo.toml sans metadata.rbs docker
    }
```

Run: `cargo test -p rbs-cli --test integration_add` → Expected: FAIL (fichier absent, puis
rouge réel sur chaque assertion).

- [ ] **Step 2: Faire passer**

Aucun code de production attendu — les garanties existent depuis E2–E6. Tout rouge qui
subsiste est un défaut réel du mécanisme, à corriger ici et non à contourner par une
assertion plus molle.

Run: `cargo test -p rbs-cli --test integration_add` → Expected: 4 passed.

- [ ] **Step 3: Prouver que les tests mordent**

Chaque test est mis au rouge par une mutation du code de production, la sortie consignée,
la mutation retirée. Un test d'intégration qui ne mord pas coûte du temps de CI sans rien
garantir.

- [ ] **Step 4: Vérifications finales**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 5: Cocher E9 dans `TODO.md`, avec sa preuve sur une ligne**

- [ ] **Step 6: Commit** — `test(cli): éprouve les quatre garanties du mécanisme add`

## Après le plan

Le lot E est clos, aux deux `PARTIEL` près (A6 attend F1, E8 attend un runner).
