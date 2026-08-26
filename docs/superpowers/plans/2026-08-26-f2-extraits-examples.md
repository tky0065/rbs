# Extraits de code depuis `examples/` — plan d'implémentation

**Goal:** Que les blocs de code de la documentation soient lus dans un projet réel qui
compile, et qu'aucun ne soit écrit à la main dans le Markdown.

**Architecture:** La spec pose la contrainte comme conséquence assumée de D6 — Docusaurus
n'exécute pas les extraits, et cette compensation est le prix du choix. `examples/` est
donc versionné : au moment où le site se construit, aucune commande cargo ne tourne, les
sources doivent être présentes.

Versionner ouvre le défaut symétrique. Le jour où une template du CLI change, l'exemple
commité ne bouge pas et les extraits se mettent à mentir, sans que rien ne le signale.
D'où le test de non-dérive : ce n'est pas un test de confort, c'est ce qui rend le
versionnement acceptable.

Les projets d'`examples/` déclarent leur propre `[workspace]` — Cargo interdit
l'imbrication, ils ne peuvent donc pas être membres du workspace racine, qui les exclut
explicitement. Ils dépendent de `rbs-core` par `--core-path`, comme la spec du squelette
l'a prévu pour les tests d'intégration.

Les marqueurs de région vivent dans l'exemple et jamais dans les templates du CLI : un
utilisateur de `rbs new` n'a aucune raison de recevoir des commentaires qui servent la
documentation du projet. Le test de non-dérive compare donc en les ignorant.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §D6 et sa conséquence assumée,
`docs/superpowers/specs/2026-08-26-squelette-projet-design.md` §3.

## Global Constraints

- Branche dédiée `f2-extraits-examples`.
- Aucun bloc de code Rust écrit à la main dans le Markdown.
- Le plugin échoue **dur** : fichier ou région absents font tomber le build du site, ils
  ne dégradent pas l'extrait en silence.
- `ci.yml` reste sans Node — la garantie de CONTRIBUTING se vérifie en comptant les
  occurrences, et compiler du Rust ne la met pas en cause.
- Un seul exemple pour la v0.1.

## File Structure

- Create: `examples/hello-crud/` (généré), `docs/plugins/remark-code-from-file.js`,
  `crates/rbs-cli/tests/examples_non_derive.rs`
- Modify: `Cargo.toml` (exclude), `docs/docusaurus.config.ts`, `.github/workflows/ci.yml`,
  `docs/docs/intro.md` et sa version française (un extrait réel, pour prouver la chaîne)

---

### Task 1: Le plugin d'extraction

- [ ] **Step 1:** Écrire les cas d'échec d'abord — fichier absent, région absente, région
      non refermée — et les voir échouer.
- [ ] **Step 2:** Écrire le plugin : `file=` seul rend le fichier entier, `region=` en
      découpe une part entre `// region: <nom>` et `// endregion: <nom>`.
- [ ] **Step 3:** Brancher le plugin dans `docusaurus.config.ts`.

### Task 2: L'exemple

- [ ] **Step 4:** Générer `examples/hello-crud` par `rbs new --core-path` puis
      `rbs generate crud`, avec des champs qui donnent à voir plusieurs types.
- [ ] **Step 5:** `cargo check --workspace` dans l'exemple — lire la sortie réelle.
- [ ] **Step 6:** Poser les marqueurs de région sur les fragments que la documentation
      citera.
- [ ] **Step 7:** `exclude = ["examples"]` au workspace racine, et vérifier que
      `cargo test --workspace` du dépôt reste intact.

### Task 3: La non-dérive

- [ ] **Step 8:** Écrire le test qui régénère dans un répertoire temporaire et compare en
      ignorant les marqueurs ; le voir échouer avant de le voir passer.
- [ ] **Step 9:** Vérifier qu'il mord : modifier l'exemple, constater le rouge.

### Task 4: La CI et la preuve

- [ ] **Step 10:** Étape `cargo check` sur `examples/hello-crud` dans `ci.yml` ; vérifier
      que le compte de Node y reste à 0.
- [ ] **Step 11:** Citer un extrait réel depuis `intro.md` FR et EN, et construire le site.
- [ ] **Step 12:** **Prouver que le garde-fou mord** : casser l'exemple, lancer les
      commandes exactes de la CI, constater le rouge, réparer, constater le vert. Idem
      pour une région supprimée contre le build du site.
- [ ] **Step 13:** Cocher F2 dans `TODO.md`, avec sa preuve sur une ligne, en disant ce
      qui est prouvé localement et ce qui ne l'est pas.
- [ ] **Step 14: Commit** — `docs: tire les extraits de la documentation d'un projet compilé`
