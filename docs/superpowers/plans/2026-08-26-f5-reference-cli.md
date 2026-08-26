# Référence du CLI (FR + EN) — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`
> ou `superpowers:executing-plans`. Les étapes sont en cases à cocher.

**Goal:** Chaque commande, chaque flag, avec un exemple de sortie réelle.

**Architecture:** Une page par commande plutôt qu'une page fleuve : le critère demande
chaque flag *et* une sortie réelle, ce qui donne cinq pages courtes au lieu d'une page
qu'on ne relit jamais. Elles vivent sous `docs/docs/cli/`, avec un `_category_.json` qui
place le groupe en quatrième position du sommaire.

Une sortie « réelle » se capture en lançant la commande. Une sortie plausible écrite de
mémoire est le défaut exact que ce critère cherche à empêcher — c'est aussi ce qui rend
cette page vérifiable, là où le reste de la documentation ne l'est que par le build.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §4 ;
surface réelle : `crates/rbs-cli/src/cli.rs`.

## Global Constraints

- Branche dédiée `f5-reference-cli`, jamais `main`.
- Bilingue dans le même commit, chaque page anglaise a son miroir FR.
- Groupe en `sidebar_position: 4` via `_category_.json`.
- **Interdiction d'écrire dans `examples/hello-crud`** (propriété de F4 et F6). Cette
  tâche ne cite aucun extrait de code source : uniquement des sorties de terminal.
- Toute sortie citée est **exécutée** avant d'être écrite. Aucune exception.
- Conventional Commits, sujet en français, sans identifiant de tâche.

## Surface à couvrir — relevée dans `crates/rbs-cli/src/cli.rs`

Flags globaux, valables sur toutes les commandes :

| Flag | Effet |
|---|---|
| `--template-dir <CHEMIN>` | Répertoire de templates remplaçant celles embarquées |
| `--yes`, `-y` | Prend les valeurs par défaut sans rien demander |

| Commande | Arguments et flags |
|---|---|
| `new <nom>` | `--database-url <URL>`, `--with <FEATURES>` (séparées par virgules), `--core-path <CHEMIN>` |
| `add <feature>` | `--force` — features disponibles : `docker`, `ci` |
| `generate crud <nom>` (alias `g`) | `--fields <CHAMPS>`, `--force`, `--dry-run` |
| `generate feature <nom>` | `--force`, `--dry-run` |
| `migrate up` / `down` / `status` | aucun flag propre |
| `migrate new <nom>` | aucun flag propre |
| `doctor` | aucun flag propre |

**Aucune de ces lignes ne se recopie sans être revérifiée** contre `cli.rs` au moment
d'écrire : ce tableau est un point de départ, pas une source.

## File Structure

- Create: `docs/docs/cli/_category_.json`
- Create: `docs/docs/cli/{new,generate,add,migrate,doctor}.md`
- Create: `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/_category_.json`
- Create: `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/{new,generate,add,migrate,doctor}.md`

---

### Task 1: Capturer les sorties

- [x] **Step 1:** `cargo build -p rbs-cli`, puis `rbs --help` et `rbs <cmd> --help` pour
      les cinq commandes et les sous-commandes de `generate` et `migrate`. Enregistrer
      chaque sortie dans le répertoire de travail temporaire.
      → `cargo build -p rbs-cli` puis les 13 écrans d’aide capturés (`--help`, `--version`, les 5 commandes, `generate crud|feature`, `migrate up|down|status|new`).
- [x] **Step 2:** Confronter les sorties au tableau ci-dessus. Tout écart → le tableau a
      tort, la sortie a raison.
      → Un seul écart : `--with` est déclaré mais 0.1.0 refuse `docker` et `ci` en renvoyant vers `rbs add`. Documenté tel quel.
- [x] **Step 3:** Dans un répertoire vierge, lancer un parcours réel : `new`, `generate
      crud --dry-run` puis sans, `generate feature`, `migrate status`, `migrate new`,
      `add docker`, `doctor`. Capturer chaque sortie, y compris les cas d'échec utiles
      (`doctor` sans base joignable, `generate` sur un working tree sale).
      → Parcours complet dans `/private/tmp/rbs-demo`, PostgreSQL 18 en conteneur : `new`, `generate crud --dry-run` puis réel, `generate feature`, ancre retirée, `migrate status/up/new/down`, `add docker/ci`, `doctor` au vert et en échec, plus les cas hors projet, working tree sale, `--fields` fautif, conflit `--template-dir`.
- [x] **Step 4:** Vérifier que l'alias `g` se comporte comme `generate` et le noter.
      → `rbs g --help` rend l’aide de `generate`, et le test `l_alias_g_parse_comme_generate` de `cli.rs` le confirme. Noté sur la page `generate`.

### Task 2: Les cinq pages anglaises

- [x] **Step 5:** `_category_.json` : `{"label": "CLI reference", "position": 4}`.
      → `docs/docs/cli/_category_.json`.
- [x] **Step 6:** `new.md` — synopsis, chaque flag avec son effet, la sortie réelle de
      `rbs new`, ce que la commande écrit, et l'inscription dans
      `[package.metadata.rbs]` qui porte l'idempotence.
      → `docs/docs/cli/new.md`.
- [x] **Step 7:** `generate.md` — `crud` et `feature`, la grammaire de `--fields` telle
      que `crates/rbs-cli/src/generate/champs.rs` l'implémente (relire le module, ne pas
      la deviner), et le couple `--dry-run` / exécution réelle montrant le **même** plan.
      → `docs/docs/cli/generate.md` — grammaire relue dans `crates/rbs-cli/src/generate/champs.rs` : sept types, trois modificateurs, quatre familles de noms refusées, collecte des fautes en une passe.
- [x] **Step 8:** `add.md` — `docker` et `ci`, la mécanique d'ancres
      (`// <rbs:features>`, `<rbs:routes>`, `<rbs:openapi>`, `<rbs:migrations>`), ce qui
      se passe quand une ancre manque — le CLI n'écrit rien et affiche le bloc à coller —
      et `--force` face à un working tree sale.
      → `docs/docs/cli/add.md`.
- [x] **Step 9:** `migrate.md` — `up`, `down`, `status`, `new`, avec la sortie réelle de
      `status` sur un projet qui a des migrations en attente et sur un projet à jour.
      → `docs/docs/cli/migrate.md` — `status` en attente, à jour, et mixte.
- [x] **Step 10:** `doctor.md` — les quatre contrôles (ancres, `.env`, base joignable,
      versions), avec une sortie au vert et une sortie en échec.
      → `docs/docs/cli/doctor.md` — sortie au vert, sortie à trois échecs, et la variante « base joignable, version illisible ».

### Task 3: Les cinq pages françaises

- [x] **Step 11:** Miroir complet sous `i18n/fr/…/cli/`, `_category_.json` avec
      `"label": "Référence du CLI"`, même `position`.
      → Cinq pages + `_category_.json` sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/`. **Le label FR ne prend pas effet** : Docusaurus traduit les libellés de catégorie par `i18n/fr/docusaurus-plugin-content-docs/current.json`, clé `sidebar.docsSidebar.category.CLI reference`, fichier hors du périmètre de cette tâche.
- [x] **Step 12:** Les sorties de terminal ne se traduisent pas — le CLI parle français,
      elles sont déjà les mêmes des deux côtés. Seul le texte d'accompagnement change.
      → Blocs de terminal identiques des deux côtés, seul le texte d’accompagnement traduit.
- [x] **Step 13:** Parité : même nombre de fichiers, même nombre de titres par fichier.
      → 6 fichiers de chaque côté ; titres par fichier : new 9/9, generate 13/13, add 9/9, migrate 9/9, doctor 7/7.

### Task 4: Prouver

- [x] **Step 14:** `cd docs && npm run build` → deux `[SUCCESS]`.
      → `cd docs && npm run build` → `[SUCCESS] Generated static files in "build".` et `[SUCCESS] Generated static files in "build/fr".`
- [x] **Step 15:** Contrôle de couverture : extraire les noms de flags de `cli.rs`
      (`grep -o '\-\-[a-z-]*' crates/rbs-cli/src/cli.rs | sort -u`) et vérifier que
      chacun apparaît dans `docs/docs/cli/`. Consigner le compte des deux côtés.
      → Le grep du plan rend **0 flag** : `cli.rs` déclare ses flags par `#[arg(long)]`, sans littéral `--`. Relevé sur la surface rendue par clap (les 13 écrans d’aide) : 10 flags — `--core-path`, `--database-url`, `--dry-run`, `--fields`, `--force`, `--help`, `--template-dir`, `--version`, `--with`, `--yes`. 10 documentés dans `docs/docs/cli/`, 10 côté FR, aucun manquant.
- [x] **Step 16:** Commit : `docs: publie la référence du CLI en anglais et en français`.

### Task 5: Cocher

- [x] **Step 17:** Le critère est « chaque commande, chaque flag, avec un exemple de
      sortie réelle ». Les trois volets se prouvent : le compte de flags de la Step 15, le
      build de la Step 14, et le fait que chaque sortie citée vient de la Task 1. Cocher
      `- [x] **F5**` avec ces preuves condensées sur une ligne. Un flag non couvert →
      `- [ ]` + annotation `PARTIEL` le nommant.
      → `TODO.md` n’est pas modifié par cette tâche.