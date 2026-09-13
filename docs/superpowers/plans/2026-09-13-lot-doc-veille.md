# Lot documentation et veille — plan d'implémentation

> **Pour l'exécutant :** exécuté par l'orchestrateur lui-même (`superpowers:executing-plans`),
> en parallèle des trois agents du lot. Cases `- [ ]` cochées au fil de l'eau.

**Objectif :** corriger la documentation livrée ou vitrine qui ment (AGENTS.md, README,
CLAUDE.md, ROADMAP, page `completions`) en gardant par test ce qui peut l'être, et poser la
veille de dépendances qui manque (npm, Dependabot, cargo-deny).

**Architecture :** aucune ligne de runtime. Les gardes vont dans `integration_docs.rs`
(passe rapide) ; les gabarits AGENTS se prouvent par `cargo test -p rbs-cli --lib agents::`
puis par `integration_examples` après régénération des `examples/*/AGENTS.md`.

**Spec :** design validé en chat le 2026-09-13 (tâches bornées 26, 27, 28, 29, 30, 33 d'`IMPROVE.md`).

## Contraintes globales

- Documentation bilingue : page EN et FR dans le même commit.
- Commits Conventional, français, impératif, sans identifiant de tâche, sans `Co-Authored-By`
  ni `Claude-Session`.
- Ne pas toucher aux fichiers des agents parallèles : `crates/rbs-core/**`, gabarits
  `features/auth/**`, `project/config/**`, `feature.toml` de `redis`/`storage`, `Cargo.toml`
  racine, `Cargo.lock`, `.cargo/audit.toml`.

---

### Tâche 26 : AGENTS.md livré

**Fichiers :** `crates/rbs-cli/templates/agents/{fr,en}.md.jinja`, `examples/*/AGENTS.md`,
éventuellement un test de `crates/rbs-cli/src/agents.rs`.

- [x] Test rouge dans `agents.rs` : le guide rendu (fr, en) contient `sept fichiers`/`seven files`
  pour `generate feature`, `huit fichiers`/`eight files` pour `crud`, ne contient pas
  `generate feature webhooks`, contient `Identity` et `require_role`.
- [x] Corriger les deux gabarits : ligne `crud` → huit fichiers (vérifié : mod, model, dto,
  filter, repository, service, controller, tests + seed + migration) ; ligne `feature` →
  sept ; recette → `rbs generate feature reports` ; architecture « ajoute à ces sept
  fichiers un `tests.rs` » ; « La quatrième ligne est celle qui compte » ; nouveau
  paragraphe inconditionnel « Sur un projet qui porte `auth` » : `Identity` (ou
  `require_role`) en argument du handler, `security(("bearer" = []))` dans
  `#[utoipa::path]`, 401 et 403 dans `responses`. Inconditionnel parce que `add` ne
  réécrit pas le guide : un paragraphe conditionnel manquerait au projet qui ajoute
  `auth` après coup.
- [x] `cargo test -p rbs-cli --lib agents::` vert.
- [x] Régénérer les quatre `examples/*/AGENTS.md` (zone guide seule) ;
  `cargo test -p rbs-cli --test integration_examples` vert.
- [x] Après fusion de la langue des réponses : ligne `rbs new` → `--lang fr|en` fixe la
  langue du projet (ce fichier et les réponses HTTP) ; régénérer à nouveau.
- [x] Commit `docs(agents): …`.

### Tâche 27 : version des README gardée

**Fichiers :** `README.md:12`, `README.fr.md:12`, `crates/rbs-cli/tests/integration_docs.rs`.

- [x] Test rouge dans `integration_docs.rs` (passe rapide) :
  `the_readmes_announce_the_version_of_the_workspace` — pour `README.md` et
  `README.fr.md` (racine = `CARGO_MANIFEST_DIR/../..`), une ligne commence par
  `format!("Version {}.", env!("CARGO_PKG_VERSION"))` (rbs-cli hérite de la version du
  workspace).
- [x] `cargo test -p rbs-cli --test integration_docs the_readmes` → FAIL (1.2.0).
- [x] Passer les deux README à `Version 1.5.0.` → PASS.
- [x] Commit `docs(readme): …`.

### Tâche 30 : page `completions` gardée

**Fichiers :** `docs/docs/cli/completions.md:84-96`, FR `…/current/cli/completions.md:88-100`.

- [x] Remplacer les deux blocs par des blocs marqués
  `{/* rbs:transcript cmd="rbs completions bash" extrait="oui" */}` (invite `$ rbs completions bash`,
  les deux lignes `rbs__subcmd__add)` / `opts=…`) et
  `{/* rbs:transcript cmd="rbs completions zsh" extrait="oui" */}` (la ligne `':feature …'`).
- [x] « The eleven names » / « Les onze noms » → « These names » / « Ces noms ».
- [x] `cargo test -p rbs-cli --test integration_docs` vert ; un nom retiré à la main du
  bloc bash doit le rendre rouge (vérifier puis remettre).
- [x] `cd docs && npm run build` vert. Commit `docs(completions): …`.

### Tâche 29 : ROADMAP à jour

**Fichiers :** `ROADMAP.md` (jalons et tableau d'état).

- [x] Une section `### v1.2`, `### v1.3`, `### v1.4`, `### v1.5` sous `## Jalons`, un
  paragraphe chacune, tiré du CHANGELOG (1.2 : client TS, webhooks/scheduler/audit, curseur,
  soft delete… à relire dans `CHANGELOG.md` ; 1.3 : routes fermées, `require_role`
  à seuil, `src/modules/` ; 1.4 : treize routes `auth`, vérification d'adresse,
  `one_time_tokens` ; 1.5 : arrêt gracieux, concurrence des jobs, SSRF des webhooks,
  correctifs de sécurité `auth`).
- [x] Quatre lignes au tableau, dates des tags (`git log -1 --format=%ci v1.2.0`…), 1.5
  « prête, non publiée ».
- [x] `grep -cE 'webhook|scheduler|audit|client' ROADMAP.md` > 0. Commit `docs(roadmap): …`.

### Tâche 28 : CLAUDE.md et parite.mjs

**Fichiers :** `CLAUDE.md:7-11,70`, `docs/scripts/parite.mjs:36`.

- [x] `RACINE_MONOLINGUE` += `'IMPROVE_OLD.md'` ; `node docs/scripts/parite.mjs` → 0 écart.
- [x] `CLAUDE.md` : « État du dépôt » réécrit (six jalons v0.1 → v1.1, puis 1.2 → 1.4
  publiées et 1.5.0 prête) ; `completions` dans la liste des commandes de l'arborescence.
- [x] Après la tâche 31 : « quatre projets d'exemple » → cinq (`CLAUDE.md:9,62`).
- [x] Commit `docs: …`.

### Tâche 33 : veille des dépendances

**Fichiers :** `docs/package.json`, `docs/package-lock.json`, `docs/audit-ci.jsonc`,
`.github/workflows/docs.yml`, `.github/dependabot.yml`, `deny.toml`, `.github/workflows/ci.yml`.

- [x] `cd docs && npm audit fix` (sans `--force`) ; `overrides` `serialize-javascript`
  `^7.0.5` si `npm run build` reste vert, sinon exception écrite.
- [x] `docs/audit-ci.jsonc` : `high: true`, `skip-dev: true`, `allowlist` des GHSA sans
  correctif (image-size `GHSA-w3rx-r6r6-pgpr`, `GHSA-5p2g-fcmc-qvqq`), chaque entrée
  commentée avec ce qui la lèvera ; `npx --yes audit-ci@7.1.0 --config audit-ci.jsonc` vert
  en local, rouge si on retire une entrée.
- [x] Étape `npm audit` dans `docs.yml` après `npm ci`.
- [x] `.github/dependabot.yml` : `cargo` (`/`), `npm` (`/docs`), `github-actions` (`/`),
  hebdomadaire, groupés ; commentaire : `examples/` et les `feature.toml` restent hors
  de sa portée.
- [x] `deny.toml` : `[licenses]` = liste réelle du graphe (`cargo deny list`), `[sources]`
  crates.io seul, `[bans] multiple-versions = "warn"` ; en tête, les advisories restent à
  `cargo audit`. `cargo deny --all-features check licenses sources bans` vert.
- [x] Job `deny` dans `ci.yml`, sur le modèle du job `audit`.
- [x] Commits `ci(docs): …`, `ci: …`.

### Vérifications finales du lot doc

- [x] `cargo test -p rbs-cli --lib`, `--test integration_docs`, `--test integration_examples`,
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cd docs && npm run typecheck && npm test && npm run build`, `node docs/scripts/parite.mjs`.
