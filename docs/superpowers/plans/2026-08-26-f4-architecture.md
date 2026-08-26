# Architecture (FR + EN) — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`
> ou `superpowers:executing-plans`. Les étapes sont en cases à cocher.

**Goal:** Une page qui explique les trois décisions structurantes de rbs — la frontière
noyau/généré, l'anatomie d'une feature, la règle de dépendance — en les montrant sur du
code réel plutôt qu'en les affirmant.

**Architecture:** Trois sections, dans cet ordre, parce que chacune motive la suivante.

1. **La frontière noyau / généré.** `rbs-core` porte ce qui n'a aucune raison de varier
   d'un projet à l'autre ; le CLI écrit dans le projet tout ce que l'utilisateur voudra
   lire ou modifier. Le test qui tranche : « ce code, un développeur voudra-t-il le
   relire ? » — si oui, il est généré. Lister les 11 modules de `rbs-core` et dire, pour
   chacun, pourquoi il est du côté noyau.
2. **L'anatomie d'une feature.** Les six fichiers — `mod`, `model`, `dto`, `repository`,
   `service`, `controller` — montrés sur `examples/hello-crud/src/articles/`.
3. **La règle de dépendance.** `controller → service → repository → model`, strictement
   unidirectionnelle. Un `service` ne touche jamais `DatabaseConnection`, un `controller`
   ne construit jamais de requête SeaORM. Le montrer par les signatures, pas par un
   paragraphe : le `service` prend un `&DatabaseConnection` en argument sans jamais le
   stocker, le `repository` est le seul à importer SeaORM.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §5 ; `CLAUDE.md` §Architecture.

## Global Constraints

- Branche dédiée `f4-architecture`, jamais `main`.
- Bilingue dans le même commit : `docs/docs/architecture.md` et son miroir FR.
- `sidebar_position: 3`.
- Aucun bloc de code Rust écrit à la main.
- **Propriété exclusive de cette tâche** dans `examples/hello-crud` :
  `src/articles/{mod,model,dto,repository,service,controller}.rs`. Ne toucher à aucun
  autre fichier de l'exemple — `config/`, `src/main.rs`, `src/openapi.rs`, `src/state.rs`,
  `src/router.rs`, `migration/`, `src/articles/tests.rs` appartiennent à F6.
- Les marqueurs de région vivent dans l'exemple, **jamais** dans les templates du CLI :
  un utilisateur de `rbs new` n'a aucune raison de recevoir des commentaires qui servent
  la documentation du projet.
- Conventional Commits, sujet en français, sans identifiant de tâche.

## File Structure

- Create: `docs/docs/architecture.md`
- Create: `docs/i18n/fr/docusaurus-plugin-content-docs/current/architecture.md`
- Modify: `examples/hello-crud/src/articles/*.rs` — ajout de marqueurs `// region:` /
  `// endregion:` uniquement, aucune ligne de code touchée

---

### Task 1: Poser les régions sur l'exemple

- [x] **Step 1:** Lire les six fichiers de `examples/hello-crud/src/articles/` en entier
      avant d'écrire quoi que ce soit. La page décrit ce code-là, pas le code attendu.
- [x] **Step 2:** Poser les marqueurs sur les fragments que la page citera. Au minimum :
      `model` sur la déclaration de l'entité SeaORM, `dto` sur une paire requête/réponse,
      `repository` sur une méthode qui construit une requête SeaORM, `service` sur une
      méthode qui applique une règle métier, `controller` sur un handler. La région
      `create` de `controller.rs` existe déjà — ne pas la redéfinir.
- [ ] **Step 3:** PARTIEL — voir Step 14 : l'échec est préexistant et sans rapport avec
      les marqueurs. `git diff` ne montre que des lignes `// region:` ajoutées.
- [ ] **Step 3 (énoncé d'origine):** `cargo test -p rbs-cli --test integration_examples` → doit rester au
      vert. Le test filtre les lignes `// region:` de sa comparaison ; s'il échoue, c'est
      qu'une ligne de code a bougé, ce que cette tâche interdit.

### Task 2: Écrire la page anglaise

- [x] **Step 4:** Frontmatter (`sidebar_position: 3`, `title: Architecture`), puis la
      section « The core / generated boundary ». Établir le critère de tri avant de lister
      les modules, sinon la liste n'apprend rien.
- [x] **Step 5:** Tableau des 11 modules publics de `rbs-core` — `config`, `db`, `error`,
      `extract`, `health`, `logs`, `openapi`, `pagination`, `request_id`, `state`,
      `trace` — une ligne de justification chacun. Vérifier la liste contre
      `crates/rbs-core/src/lib.rs` au moment d'écrire, pas de mémoire.
- [x] **Step 6:** Mentionner que les quatre feature flags `auth`, `redis`, `mail`,
      `storage` sont **déclarés mais vides** en v0.1 : les activer ne change rien, ils
      réservent seulement leur nom pour la v0.2.
- [x] **Step 7:** Section « Anatomy of a feature » : les six fichiers, chacun introduit
      par ce qu'il fait, puis son extrait `file=`/`region=`.
- [x] **Step 8:** *(nuancé — `grep -l sea_orm` rend cinq fichiers sur six ; la page cite
      le résultat effectif et la sonde étroite `grep -l 'Entity::'`, qui rend
      `repository.rs` seul.)*
- [x] **Step 8 (énoncé d'origine):** Section « The dependency rule » : la chaîne
      `controller → service → repository → model`, et ce qu'elle interdit. Appuyer sur les
      imports réels — `repository.rs` est le seul fichier de la feature à importer
      `sea_orm` ; le vérifier par `grep -l sea_orm examples/hello-crud/src/articles/*.rs`
      et citer le résultat.
- [x] **Step 9:** Clore sur le seuil des ~200 lignes : un fichier de feature au-delà
      signale une feature à scinder.

### Task 3: Écrire la page française

- [x] **Step 10:** Traduire dans
      `docs/i18n/fr/docusaurus-plugin-content-docs/current/architecture.md`. Même
      frontmatter, mêmes directives `file=`/`region=` — les extraits sont partagés, ils
      ne se traduisent pas.
- [x] **Step 11:** Parité : même nombre de titres, même ordre de sections.

### Task 4: Prouver

- [x] **Step 12:** `cd docs && npm run build` → deux `[SUCCESS]`. Une région citée mais
      absente fait sortir le plugin en erreur : ce build prouve que chaque extrait existe.
- [x] **Step 13:** *(le garde-fou ne mord qu'après `npx docusaurus clear` : un build
      incrémental réutilise le MDX en cache et passe sur un extrait périmé.)* Éprouver le garde-fou : renommer temporairement une région citée,
      relancer le build, constater l'échec **et lire le message**, puis restaurer.
      Un contrôle qui ne mord pas n'est pas une preuve.
- [ ] **Step 14:** PARTIEL — `cargo test -p rbs-cli --test integration_examples` échoue,
      mais à l'identique avant et après cette tâche : `+ .env est produit mais absent de
      l'exemple`. Dérive préexistante (`examples/hello-crud/.gitignore` ignore `.env`, que
      `rbs new` produit), sans rapport avec les marqueurs de région. Vérifié par
      `git stash` puis relance sur `ddd846c` : message identique.
- [x] **Step 15:** Commit : `docs: décrit l'architecture du cadre en anglais et en
      français`.

### Task 5: Cocher

- [ ] **Step 16:** NON FAIT — `TODO.md` est hors du périmètre d'écriture de cette tâche.
      La ligne de preuve est remontée à l'appelant, qui coche.
- [ ] **Step 16 (énoncé d'origine):** F4 n'a pas de ligne `✓` explicite dans `TODO.md` : la preuve est le
      build bilingue plus le test de non-dérive. Cocher `- [x] **F4**` avec les deux
      résultats sur une seule ligne.
