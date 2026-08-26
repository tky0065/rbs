# Guides transverses (FR + EN) — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`
> ou `superpowers:executing-plans`. Les étapes sont en cases à cocher.

**Goal:** Six guides — configuration, logs, erreurs, OpenAPI, migrations, tests — qui
répondent chacun à une question qu'un utilisateur se pose une fois le projet généré.

**Architecture:** Un guide répond à « comment fais-je X ? », là où la page d'architecture
répond à « pourquoi est-ce fait ainsi ? ». La distinction décide du contenu : un guide part
d'un besoin, montre le code qui y répond, et s'arrête. Il ne récapitule pas la conception.

`logs.md` existe déjà et sert de patron : un problème posé, un extrait tiré d'un fichier
compilé, une commande à lancer pour juger soi-même. Les cinq autres s'y alignent, y
compris pour la position dans le groupe.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §3 ; surface réelle :
`crates/rbs-core/src/{config,error,openapi,pagination,extract}.rs`.

## Global Constraints

- Branche dédiée `f6-guides-transverses`, jamais `main`.
- Bilingue dans le même commit, chaque guide a son miroir FR.
- Groupe en `sidebar_position: 5` via `_category_.json` — `logs.md` garde
  `sidebar_position: 1` à l'intérieur du groupe.
- Aucun bloc de code Rust ou TOML écrit à la main.
- **Propriété exclusive de cette tâche** dans `examples/hello-crud` : `config/*.toml`,
  `src/main.rs`, `src/openapi.rs`, `src/state.rs`, `src/router.rs`, `migration/**`,
  `src/articles/tests.rs`. **Ne pas toucher** à `src/articles/{mod,model,dto,repository,
  service,controller}.rs` — ils appartiennent à F4.
- Les marqueurs de région vivent dans l'exemple, jamais dans les templates du CLI.
- Ne pas modifier `guides/logs.md` au-delà de ce qu'exige la cohérence du groupe.
- Conventional Commits, sujet en français, sans identifiant de tâche.

## File Structure

- Create: `docs/docs/guides/_category_.json`
- Create: `docs/docs/guides/{configuration,errors,openapi,migrations,testing}.md`
- Create: miroirs FR sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/`
- Modify: `examples/hello-crud/` — marqueurs de région uniquement, sur les fichiers
  listés ci-dessus

---

### Task 1: Poser les régions

- [ ] **Step 1:** Lire `config/default.toml`, `config/development.toml`, `src/main.rs`,
      `src/openapi.rs`, `src/state.rs`, `src/router.rs`, `migration/src/lib.rs`, la
      migration `create_articles` et `src/articles/tests.rs` de l'exemple, en entier.
- [ ] **Step 2:** Poser les marqueurs sur les fragments que les guides citeront, un par
      guide au minimum. Les fichiers TOML acceptent `# region:` — vérifier la syntaxe
      reconnue par `docs/plugins/remark-code-from-file.js` avant d'écrire, et s'y tenir.
- [ ] **Step 3:** `cargo test -p rbs-cli --test integration_examples` → au vert.

### Task 2: `configuration.md`

- [ ] **Step 4:** Relire `crates/rbs-core/src/config.rs`. Décrire la superposition réelle :
      `config/default.toml`, puis `config/{RBS_ENV}.toml`, puis `.env`, puis
      l'environnement — préfixe `RBS_`, profil par défaut quand `RBS_ENV` est absent.
      Vérifier l'ordre dans le code, ne pas le déduire.
- [ ] **Step 5:** Tableau des champs : `server.host`, `server.port`, `database.url` (seul
      champ sans défaut — son absence fait échouer le démarrage), les six réglages de
      pool, et `docs.swagger_ui` / `docs.openapi_json`. Expliquer pourquoi ces deux-là
      sont séparés : couper l'interface en gardant le document sert à générer des clients.
- [ ] **Step 6:** Dire que `ConfigError` est distincte d'`Error` — une erreur de démarrage
      ne devient jamais une réponse HTTP — et citer l'extrait de `config/default.toml`.

### Task 3: `errors.md`

- [ ] **Step 7:** Relire `crates/rbs-core/src/error.rs`. Tableau des variantes réelles —
      `NotFound`, `BadRequest`, `Validation`, `Unauthorized`, `Forbidden`, `Conflict`,
      `Database`, `Internal` — avec le statut HTTP de chacune.
- [ ] **Step 8:** Montrer la forme `ProblemDetails` de la réponse, et le fait que
      `Validation` porte le détail par champ là où `Internal` ne dit rien de plus que
      « une erreur interne est survenue » : ne pas fuir l'interne au client est délibéré.
- [ ] **Step 9:** Montrer comment une erreur remonte jusqu'à la réponse HTTP. **Ne citer
      aucune région de `src/articles/{model,dto,repository,service,controller}.rs`** :
      ces fichiers sont écrits en parallèle par une autre branche, et une région qui n'y
      existe pas encore ferait tomber le build. Citer `src/main.rs` ou `src/router.rs`,
      qui appartiennent à cette tâche, ou décrire la variante sans extrait.

### Task 4: `openapi.md`

- [ ] **Step 10:** Décrire `src/openapi.rs` du projet généré et l'ancre
      `// <rbs:openapi>` où `generate crud` inscrit les nouveaux schémas.
- [ ] **Step 11:** Expliquer `ReponsesCommunes` : les réponses d'erreur sont déclarées
      une fois dans `rbs-core` au lieu d'être répétées sur chaque handler.
- [ ] **Step 12:** Donner les deux URL — `/docs` pour Swagger UI, `/api-docs/openapi.json`
      pour le document — et les deux réglages qui les coupent indépendamment.

### Task 5: `migrations.md`

- [ ] **Step 13:** Poser le point de bascule : `rbs generate crud` produit l'entité **et**
      sa migration depuis `--fields`, sans base démarrée — l'inverse de
      `sea-orm-cli generate entity`, qui lit une base existante.
- [ ] **Step 14:** `migrate up` / `down` / `status` / `new`, l'ancre
      `// <rbs:migrations>` dans `migration/src/lib.rs`, et la migration réelle de
      l'exemple en extrait.

### Task 6: `testing.md`

- [ ] **Step 15:** Citer `src/articles/tests.rs` de l'exemple, expliquer ce que le CLI
      génère comme banc d'essai et ce qu'il laisse à écrire.
- [ ] **Step 16:** Décrire le recours à `testcontainers` pour un vrai PostgreSQL, et dire
      franchement que ces tests sont lents et exigent Docker.

### Task 7: Les six miroirs français

- [ ] **Step 17:** Traduire les cinq nouveaux guides sous `i18n/fr/…/guides/`, plus
      `_category_.json` avec `"label": "Guides"`. `logs.md` FR existe déjà.
- [ ] **Step 18:** Parité : `ls` des deux répertoires identique, même nombre de titres
      par fichier.

### Task 8: Prouver

- [ ] **Step 19:** `cd docs && npm run build` → deux `[SUCCESS]`.
- [ ] **Step 20:** Éprouver le garde-fou : renommer une région citée, constater l'échec du
      build **et lire le message**, restaurer.
- [ ] **Step 21:** `cargo test -p rbs-cli --test integration_examples` → au vert.
- [ ] **Step 22:** Commit : `docs: publie les guides transverses en anglais et en
      français`.

### Task 9: Cocher

- [ ] **Step 23:** Le critère énumère six sujets. Six guides livrés et prouvés → cocher
      `- [x] **F6**`. Un sujet manquant ou non prouvé → `- [ ]` + annotation `PARTIEL` le
      nommant.
