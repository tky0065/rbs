# Lot H — Le moule des fragments : plan d'implémentation

> **Pour un agent exécutant :** SOUS-SKILL REQUIS — `superpowers:test-driven-development`
> pour chaque tâche. Les étapes sont cochables (`- [ ]`).

**But :** apprendre à `rbs add` à installer un fragment qui apporte du code Rust — des
insertions d'ancres, une migration, des patchs de manifeste et de configuration — **sans
que le CLI connaisse aucune feature par son nom**.

**Architecture :** un `feature.toml` par répertoire de `templates/features` déclare ce que
l'installation fait au projet ; `add` en devient l'interprète générique. Les briques
existent déjà et se réutilisent telles quelles — `ancres::inserer`, `plan::Constructeur`,
`plan::PatchToml`, `metadata::ajouter_feature_a_dependance`, `plan::application`. Ce lot
câble, il n'invente pas.

**Pile :** `serde` + `toml` pour le manifeste, `toml_edit` pour les patchs (déjà présents).

**Spec :** `docs/superpowers/specs/2026-08-27-v0.2-auth-design.md` (§2.4, §2.5).
**Backlog :** `TODO.md`, tâches `H1` à `H6`. Les lignes `✓` y sont la liste des tests.

## Contraintes globales

- **Ce lot ne touche que `crates/rbs-cli/`.** Le lot G travaille en parallèle dans
  `crates/rbs-core/`. Ne modifier aucun fichier de `rbs-core`, d'`examples/`, de `docs/`
  — ni **`TODO.md`**, qui est coché par l'orchestrateur.
- **`add.rs` compte 358 lignes**, au-delà du seuil de scission du CLAUDE.md. Ce lot lui
  ajoute de la matière : sortir le manifeste et son interprétation dans leurs propres
  modules, ne pas gonfler `add.rs`.
- Un commentaire dit le *pourquoi*, jamais le *quoi*. `clippy -D warnings` et
  `fmt --check` bloquants.
- Commits Conventional Commits, sujet français à l'impératif. **Aucun identifiant de
  tâche (`H1`…), aucun renvoi à `TODO.md` ou à ce plan, aucune ligne `Co-Authored-By`.**
  Corps : le pourquoi technique, puis un intertitre `Vérifications :` avec les commandes
  lancées et leur sortie réelle.
- Un commit par tâche, au minimum.

## Le format `feature.toml`

La spec §2.4 en donne la forme. Ce plan **comble trois points qu'elle ne montre pas** —
le nom de la migration, la section de configuration et la variable d'environnement — et
fixe le schéma complet :

```toml
[feature]
description = "JWT, Argon2, rôles"       # requis

[[fichiers]]                             # optionnel — absent, tout le répertoire est copié
source = "model.rs.jinja"
cible  = "src/features/auth/model.rs"

[[ancres]]                               # optionnel
ancre   = "features"                     # l'une des cinq de `ancres::ANCRES`
contenu = "mod auth;"

[migration]                              # optionnel
source = "users.rs.jinja"
nom    = "create_users"                  # sert au nom horodaté du fichier

[cargo.rbs-core]                         # optionnel, une table par crate
features = ["auth"]

[[config]]                               # optionnel
fichier = "config/default.toml"
section = "auth"
contenu = """
access_ttl_secs = 900
refresh_ttl_secs = 2592000
"""

[[env]]                                  # optionnel
cle         = "RBS_AUTH__SECRET"
valeur      = "changez-moi"
commentaire = "Secret de signature HS256, au moins 32 octets"
```

**`[[fichiers]]` absent = comportement actuel.** C'est ce qui permet à `H2` de migrer
`docker` et `ci` sans toucher à leurs tests : leur manifeste ne portera qu'un
`[feature] description`.

**Toutes les structs portent `#[serde(deny_unknown_fields)]`** — le `✓` de `H1` l'exige :
un champ inconnu doit être une erreur nommant le champ *et* le fichier, pas un silence.

## Structure de fichiers

| Fichier | Responsabilité |
|---|---|
| `crates/rbs-cli/src/manifeste.rs` | **créé** — le schéma `feature.toml`, sa lecture, ses erreurs |
| `crates/rbs-cli/src/add/mod.rs` | `add.rs` **déplacé** — orchestration, inchangée dans son esprit |
| `crates/rbs-cli/src/add/installation.rs` | **créé** — traduit un `Manifeste` en actions de plan |
| `crates/rbs-cli/src/plan/action.rs` | **modifié** — actions de section TOML et de ligne `.env` |
| `crates/rbs-cli/templates/features/docker/feature.toml` | **créé** — manifeste trivial |
| `crates/rbs-cli/templates/features/ci/feature.toml` | **créé** — manifeste trivial |
| `crates/rbs-cli/tests/integration_add.rs` | **modifié** — les 4 tests actuels **intacts**, nouveaux ajoutés |

**Commande de preuve du lot :**
`cargo test -p rbs-cli --bins` et `cargo test -p rbs-cli --test integration_add`.

---

### Tâche H1 — Format `feature.toml` et son parseur

**Fichiers :** créer `crates/rbs-cli/src/manifeste.rs` ; le déclarer dans `lib.rs`.

**Produit** (ce dont H2 à H6 dépendent — noms et types exacts) :

```rust
pub(crate) struct Manifeste {
    pub feature: Description,
    pub fichiers: Vec<FichierDeclare>,      // #[serde(default)]
    pub ancres: Vec<InsertionDeclaree>,     // #[serde(default)]
    pub migration: Option<MigrationDeclaree>,
    pub cargo: BTreeMap<String, PatchCrate>, // #[serde(default)]
    pub config: Vec<SectionDeclaree>,       // #[serde(default)]
    pub env: Vec<VariableDeclaree>,         // #[serde(default)]
}
pub(crate) struct Description   { pub description: String }
pub(crate) struct FichierDeclare { pub source: String, pub cible: String }
pub(crate) struct InsertionDeclaree { pub ancre: String, pub contenu: String }
pub(crate) struct MigrationDeclaree { pub source: String, pub nom: String }
pub(crate) struct PatchCrate    { pub features: Vec<String> }
pub(crate) struct SectionDeclaree { pub fichier: String, pub section: String, pub contenu: String }
pub(crate) struct VariableDeclaree { pub cle: String, pub valeur: String,
                                     pub commentaire: Option<String> }

/// Lit le manifeste d'un fragment. `nom` ne sert qu'aux messages d'erreur.
pub(crate) fn lire(texte: &str, nom: &str) -> Result<Manifeste, Erreur>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    #[error("{fichier} est invalide : {source}")]
    Invalide { fichier: String, source: toml::de::Error },
}
```

`BTreeMap` et non `HashMap` : l'ordre des patchs doit être déterministe, sans quoi
l'affichage du plan varierait d'une exécution à l'autre et les tests deviendraient
instables.

- [ ] **Étape 1 — Écrire les deux tests d'abord**, dans `manifeste.rs` :

```rust
#[test]
fn un_manifeste_valide_se_deserialise() {
    // le manifeste complet du plan ci-dessus, en &str
    // assert sur chaque section : 1 fichier, 1 ancre, la migration, cargo["rbs-core"],
    // 1 section de config, 1 variable d'environnement
}

#[test]
fn un_champ_inconnu_nomme_le_champ_et_le_fichier() {
    let erreur = lire("[feature]\ndescription = \"x\"\ninconnu = 1\n",
                      "features/auth/feature.toml").unwrap_err();
    let message = erreur.to_string();
    assert!(message.contains("inconnu"), "{message}");
    assert!(message.contains("features/auth/feature.toml"), "{message}");
}

#[test]
fn un_manifeste_minimal_ne_declare_que_sa_description() {
    // "[feature]\ndescription = \"docker\"\n" → toutes les listes vides, pas d'erreur.
    // C'est le manifeste que H2 donnera à `docker` et `ci`.
}
```

- [ ] **Étape 2 — Les voir échouer.** `cargo test -p rbs-cli --bins manifeste::`

- [ ] **Étape 3 — Implémenter.** `#[serde(deny_unknown_fields)]` sur **chaque** struct.
      Vérifier que `toml` est déjà une dépendance de `rbs-cli` (`grep toml
      crates/rbs-cli/Cargo.toml`) ; `toml_edit` seul ne suffit pas à désérialiser.

- [ ] **Étape 4 — Les voir passer.** `cargo test -p rbs-cli --bins manifeste::` → 3 passed.

- [ ] **Étape 5 — Éprouver.** Retirer `deny_unknown_fields` → le deuxième test doit
      **échouer**. Remettre.

- [ ] **Étape 6 — fmt, clippy, commit.**
      `git commit -m "feat(cli): déclare ce qu'un fragment de feature installe"`

---

### Tâche H2 — `add` interprète le manifeste ; `docker` et `ci` migrés

**Le `✓` de cette tâche est une contrainte de non-régression : les tests actuels d'`add`
passent SANS ÊTRE MODIFIÉS.** Si un test doit changer, le comportement a changé, et c'est
un échec de la tâche — pas une occasion de retoucher le test.

**Fichiers :** `add.rs` → `add/mod.rs` ; créer `add/installation.rs` ; créer les deux
`feature.toml` triviaux.

- [ ] **Étape 1 — Lire l'existant avant d'écrire.** `crates/rbs-cli/src/add.rs` en
      entier, `templates.rs` (`Source::feature`, `fichiers()`), et
      `crates/rbs-cli/tests/integration_add.rs` (les 4 tests à ne pas casser).

- [ ] **Étape 2 — Le piège de cette tâche.** `Source::fichiers()` liste **tout** le
      répertoire du fragment : `feature.toml` s'y trouverait, et serait copié dans le
      projet de l'utilisateur. Les tests actuels comptent les fichiers écrits — ils
      tomberaient. **`feature.toml` doit être exclu de la copie**, dans `templates.rs`.
      Écrire ce test d'abord :

```rust
#[test]
fn le_manifeste_du_fragment_n_est_pas_copie_dans_le_projet() {
    // Source::feature(None, "docker").fichiers() ne contient aucune destination
    // nommée "feature.toml"
}
```

- [ ] **Étape 3 — Le voir échouer**, puis exclure `feature.toml`, puis le voir passer.

- [ ] **Étape 4 — Écrire les manifestes triviaux.**
      `templates/features/docker/feature.toml` et `templates/features/ci/feature.toml`,
      chacun réduit à `[feature]` + `description`.

- [ ] **Étape 5 — Brancher la lecture.** `planifier()` lit le `feature.toml` du fragment
      et le passe à `installation::actions(&manifeste, ...)`. Un fragment **sans**
      manifeste doit rester une erreur claire, pas un panic. Avec un manifeste trivial,
      le plan produit doit être **exactement** celui d'aujourd'hui.

- [ ] **Étape 6 — La preuve du `✓`.** Lancer, et lire :

```bash
git diff --stat -- crates/rbs-cli/tests/integration_add.rs   # attendu : aucune ligne
cargo test -p rbs-cli --test integration_add                 # attendu : 4 passed
cargo test -p rbs-cli --bins                                 # tout vert
```

      **Le premier `git diff` fait partie de la preuve** : il établit que les tests n'ont
      pas été retouchés. Le consigner.

- [ ] **Étape 7 — fmt, clippy, commit.**
      `git commit -m "refactor(cli): fait de add l'interprète du manifeste des fragments"`

---

### Tâche H3 — Insertions dans les ancres déclarées

**Fichiers :** `add/installation.rs`.

**Consomme :** `manifeste::InsertionDeclaree` (H1), `ancres::{ANCRES, inserer, Absente}`,
`plan::Constructeur::inserer` (existants — les lire avant d'écrire).

Le nom d'ancre du manifeste (`"features"`) se résout contre `ancres::ANCRES` par son
champ `nom`. Un nom inconnu est une erreur du manifeste, pas un silence.

- [ ] **Étape 1 — Écrire les deux tests d'abord.** Le second est le plus important : il
      décrit le comportement que le CLAUDE.md impose et qu'aucun raccourci ne doit
      contourner — **ancre absente → rien n'est écrit, le bloc à coller est affiché,
      sortie en erreur.**

```rust
#[test] fn le_contenu_declare_est_insere_dans_chacune_des_quatre_ancres() {
    // un manifeste déclarant features, routes, openapi, migrations
    // → le fichier de chaque ancre porte la ligne, juste avant la balise fermante
}
#[test] fn une_ancre_absente_n_ecrit_rien_et_affiche_le_bloc() {
    // retirer l'ancre `routes` de src/router.rs du projet de test
    // → sortie en erreur, message portant le bloc à coller,
    //   ET aucun fichier du projet modifié (comparer une empreinte du répertoire
    //   avant/après, comme le fait déjà le test de rollback de E6)
}
```

- [ ] **Étape 2 — Les voir échouer**, implémenter, les voir passer.

- [ ] **Étape 3 — Éprouver.** Faire ignorer l'ancre absente → le second test doit
      **échouer** en trouvant le projet modifié. Remettre.

- [ ] **Étape 4 — fmt, clippy, commit.**
      `git commit -m "feat(cli): insère dans les ancres qu'un fragment déclare"`

---

### Tâche H4 — Migration horodatée déposée par un fragment

**Fichiers :** `add/installation.rs`.

**Consomme :** `manifeste::MigrationDeclaree` (H1) et la génération de migration
existante — **la lire avant d'écrire** : `crates/rbs-cli/src/generate/migration.rs` et
`crates/rbs-cli/src/migrate/nouvelle.rs` portent déjà le format horodaté. Ne pas en
réinventer un second : deux formats d'horodatage dans le même projet, ce sont deux
ordres de migration possibles.

- [ ] **Étape 1 — Écrire les deux tests d'abord :**

```rust
#[test] fn la_migration_du_fragment_est_deposee_au_format_horodate() {
    // migration/src/mYYYYMMDD_HHMMSS_create_users.rs — vérifier le motif, pas la valeur
}
#[test] fn l_ancre_migrations_est_completee_par_l_appel_correspondant() {
    // migration/src/lib.rs porte le `mod` ET l'entrée du Migrator
    // (deux ancres distinctes : MIGRATION_MODULES et MIGRATIONS)
}
```

- [ ] **Étape 2 — Les voir échouer**, implémenter, les voir passer.
      `cargo test -p rbs-cli --bins installation::`

- [ ] **Étape 3 — fmt, clippy, commit.**
      `git commit -m "feat(cli): dépose la migration horodatée d'un fragment"`

---

### Tâche H5 — Patchs de `Cargo.toml`, `config/default.toml` et `.env.example`

**Fichiers :** `add/installation.rs`, `crates/rbs-cli/src/plan/action.rs`.

**Consomme :** `plan::PatchToml::AjouterFeatureADependance` et
`metadata::ajouter_feature_a_dependance` — **ils existent déjà et n'ont aucun appelant**.
Le doc-comment de `action.rs` le dit : « Le premier à s'en servir sera `add auth`, en
v0.2. » C'est cette tâche. Ne pas écrire un second chemin de patch à côté.

Restent à créer : une action de **section TOML** (`config/default.toml` n'est pas un
manifeste Cargo) et une action de **ligne de fichier texte** (`.env.example`). Les deux
doivent être idempotentes : une section déjà présente ne se réécrit pas.

- [ ] **Étape 1 — Écrire les trois tests d'abord :**

```rust
#[test] fn rbs_core_gagne_la_feature_sans_que_le_reste_soit_reformate() {
    // patcher un Cargo.toml réaliste, puis comparer ligne à ligne :
    // seule la ligne de rbs-core diffère, toutes les autres sont identiques
}
#[test] fn les_commentaires_du_developpeur_survivent_au_patch() {
    // un commentaire en fin de ligne ET un commentaire de bloc, préservés
}
#[test] fn la_section_de_configuration_et_la_variable_d_environnement_sont_ajoutees() {
    // config/default.toml gagne [auth] avec ses deux clés
    // .env.example gagne RBS_AUTH__SECRET=... précédé de son commentaire
}
```

Le premier test est celui qui a un vrai risque de faux positif : `toml_edit` préserve la
mise en forme, mais une implémentation qui re-sérialiserait le document passerait un
`assert` trop lâche. **Comparer ligne à ligne**, pas seulement vérifier que la feature
est là.

- [ ] **Étape 2 — Les voir échouer**, implémenter, les voir passer.

- [ ] **Étape 3 — Éprouver.** Remplacer le patch `toml_edit` par une re-sérialisation
      complète → le test des commentaires doit **échouer**. Remettre.

- [ ] **Étape 4 — fmt, clippy, commit.**
      `git commit -m "feat(cli): patche le manifeste, la configuration et l'exemple d'environnement"`

---

### Tâche H6 — Idempotence et tout-ou-rien sur un fragment à code Rust

**Fichiers :** `crates/rbs-cli/tests/integration_add.rs` (ajouts uniquement).

**Consomme :** `metadata` (`[package.metadata.rbs]`) et `plan::application` — dont `E6` a
prouvé le rollback. Cette tâche l'**éprouve sur un fragment à code Rust**, elle ne le
réécrit pas.

**La vérification porte sur `[package.metadata.rbs]`, pas sur la présence des fichiers.**
Un développeur qui supprime un fichier installé ne doit pas voir la feature se
réinstaller à moitié.

- [ ] **Étape 1 — Un fragment de test.** Ce lot n'a pas de fragment à code Rust : `auth`
      est le lot I. En fabriquer un dans un `--template-dir` temporaire, portant un
      manifeste qui exerce les six sections. C'est aussi le premier bout-à-bout du moule.

- [ ] **Étape 2 — Écrire les deux tests d'abord :**

```rust
#[test] fn deux_installations_successives_n_ecrivent_rien_la_seconde() {
    // empreinte du répertoire après la 1re == empreinte après la 2de
}
#[test] fn un_echec_a_mi_parcours_restaure_les_fichiers_deja_ecrits() {
    // injecter l'échec sur une action médiane (motif du test de E6)
    // → empreinte du répertoire identique à l'origine
}
```

- [ ] **Étape 3 — Les voir échouer**, implémenter, les voir passer.
      `cargo test -p rbs-cli --test integration_add`

- [ ] **Étape 4 — Éprouver.** Retirer la restauration → le second test doit **échouer**.
      Remettre. Faire porter l'idempotence sur la présence des fichiers plutôt que sur
      les métadonnées, supprimer un fichier, relancer → le premier test doit **échouer**.

- [ ] **Étape 5 — fmt, clippy, commit.**
      `git commit -m "test(cli): éprouve l'idempotence et le tout-ou-rien d'un fragment à code"`

---

## Vérification finale du lot

```bash
git diff --stat main -- crates/rbs-cli/tests/integration_add.rs   # les 4 tests d'origine intacts
cargo test -p rbs-cli                                             # bins + tests d'intégration
cargo clippy -p rbs-cli --all-targets -- -D warnings
cargo fmt --all --check
wc -l crates/rbs-cli/src/add/*.rs crates/rbs-cli/src/manifeste.rs # aucun fichier démesuré
```

Puis un bout-à-bout réel, qui est la seule preuve que le moule fonctionne hors des tests :
`rbs new` dans un répertoire temporaire, `rbs add docker`, `rbs add ci` — comportement
identique à celui d'avant le lot.

## À rapporter

Pour **chaque** tâche `H1`…`H6`, et séparément : la commande exacte, son résultat réel,
et le résultat de l'étape « éprouver » (quelle mutation, quel test est tombé). Pour `H2`,
joindre la sortie de `git diff --stat` sur `integration_add.rs` — c'est **le** `✓` de la
tâche. Signaler toute tâche dont un `✓` n'a **pas** pu être prouvé : elle restera `- [ ]`
avec une annotation `PARTIEL`, ce qui est un résultat acceptable ; un `✓` déclaré sans
preuve ne l'est pas.
