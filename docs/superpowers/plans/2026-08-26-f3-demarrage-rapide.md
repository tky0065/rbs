# Démarrage rapide (FR + EN) — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`
> ou `superpowers:executing-plans`. Les étapes sont en cases à cocher.

**Goal:** Une page qui mène de l'installation à une API CRUD qui répond, suivie à la
lettre, sans qu'aucune étape n'exige de connaissance extérieure au texte.

**Architecture:** La page est linéaire et se lit d'un bout à l'autre : prérequis →
installation → `rbs new` → base de données → `rbs migrate up` → `rbs generate crud` →
premières requêtes. Chaque commande est suivie de sa **sortie réelle**, capturée en la
lançant, jamais reconstituée de mémoire. C'est ce qui distingue une page de démarrage
d'une liste de commandes : le lecteur sait à quoi reconnaître qu'il n'a pas dévié.

Le seul extrait de code Rust cité est la région `create` de
`examples/hello-crud/src/articles/controller.rs`, qui existe déjà. **Cette tâche n'ajoute
aucun marqueur de région** : `examples/hello-crud` appartient à F4 et F6, et deux agents
n'écrivent pas le même fichier.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` — critère de sortie de la v0.1.

## Global Constraints

- Branche dédiée `f3-demarrage-rapide`, jamais `main`.
- Bilingue dans le même commit : `docs/docs/getting-started.md` et
  `docs/i18n/fr/docusaurus-plugin-content-docs/current/getting-started.md`.
- `sidebar_position: 2`.
- Aucun bloc de code Rust écrit à la main : `file=` / `region=` du plugin
  `remark-code-from-file`. Les blocs `bash` de sortie de terminal sont, eux, écrits —
  mais recopiés d'une exécution réelle.
- **Interdiction d'écrire dans `examples/hello-crud`** (propriété de F4 et F6).
- Conventional Commits, sujet en français, sans identifiant de tâche.

## File Structure

- Create: `docs/docs/getting-started.md`
- Create: `docs/i18n/fr/docusaurus-plugin-content-docs/current/getting-started.md`
- Ne pas modifier : `docs/docs/intro.md` (mise à jour du sommaire réservée à l'intégration)

---

### Task 1: Exécuter le parcours avant de l'écrire

- [x] **Step 1:** Créer un répertoire de travail vierge hors du dépôt
      (`mktemp -d`) et s'y placer.
- [x] **Step 2:** Lancer `cargo run -p rbs-cli -- new demo --database-url
      postgres://rbs:rbs@localhost:5432/demo --core-path <racine>/crates/rbs-core --yes`
      depuis le dépôt, en visant le répertoire temporaire. Copier la sortie réelle.
- [x] **Step 3:** Démarrer un PostgreSQL (`docker run --rm -e POSTGRES_USER=rbs
      -e POSTGRES_PASSWORD=rbs -e POSTGRES_DB=demo -p 5432:5432 -d postgres:18`) et
      relever la commande exacte qui a marché.
- [x] **Step 4:** Lancer `rbs migrate up`, puis `rbs generate crud articles --fields
      "title:string,body:text,published:bool"`, puis `rbs migrate up` de nouveau.
      Copier chaque sortie.
- [x] **Step 5:** `cargo run` dans le projet, puis `curl` sur `/health`, `POST /articles`,
      `GET /articles`. Copier les réponses réelles.
- [x] **Step 6:** Consigner par écrit, dans le brouillon, **toute** étape qui a demandé une
      connaissance absente du texte : chacune est une ligne à ajouter à la page.

### Task 2: Écrire la page anglaise

- [x] **Step 7:** Rédiger `docs/docs/getting-started.md` : frontmatter
      (`sidebar_position: 2`, `title: Getting started`), prérequis (Rust stable,
      PostgreSQL, `cargo install --git` avec la réserve que le dépôt n'est pas encore
      public), puis le parcours de la Task 1 dans l'ordre.
- [x] **Step 8:** Coller les sorties réelles sous chaque commande, dans des blocs sans
      langage ou en `text`, jamais en `bash` (ce sont des sorties, pas des commandes).
- [x] **Step 9:** Citer la région existante :
      ```` ```rust file=examples/hello-crud/src/articles/controller.rs region=create ```` —
      vérifier que la région `create` existe toujours avant de l'écrire.
- [x] **Step 10:** Terminer par un renvoi vers `/architecture` et `/cli/generate`, en
      liens relatifs Docusaurus.

### Task 3: Écrire la page française

- [x] **Step 11:** Traduire dans
      `docs/i18n/fr/docusaurus-plugin-content-docs/current/getting-started.md`. Même
      frontmatter, `title: Démarrage rapide`. Les sorties de terminal restent identiques :
      elles ne se traduisent pas.
- [x] **Step 12:** Vérifier la parité :
      `grep -c '^#' ` sur les deux fichiers doit donner le même compte.

### Task 4: Prouver

- [x] **Step 13:** `cd docs && npm run build` → lire la sortie, exiger deux `[SUCCESS]`.
      `onBrokenLinks: 'throw'` fait tomber le build sur un lien mort, et le plugin
      d'extraits sur une région disparue : ce build couvre les deux.
- [x] **Step 14:** Rejouer la page **à la lettre** dans un second répertoire vierge, en
      ne lisant que le texte écrit. Toute hésitation → retour à la Step 7.
- [x] **Step 15:** Commit : `docs: ajoute le guide de démarrage rapide en anglais et en
      français`, corps portant le détail des commandes vérifiées.

### Task 5: Cocher, ou annoter

- [x] **Step 16:** Le critère est « suivi à la lettre sur une machine vierge, sans
      intervention extérieure ». **Attention :** compiler un projet neuf tire
      `utoipa-swagger-ui`, qui télécharge son archive depuis `github.com`. Si l'hôte ne
      résout pas `github.com` (cas connu, cf. E8 et F2), la Step 14 s'arrête au `cargo
      run`. Ce n'est **pas** un critère rempli.
- [x] **Step 17:** Parcours complet → cocher `- [x] **F3**` avec la preuve sur une ligne.
      Parcours interrompu → laisser `- [ ]` et annoter `PARTIEL 2026-08-26 : …` en nommant
      l'étape atteinte et le mur rencontré. Ne substituer aucun critère approchant.

---

## Preuves

- **Step 1** répertoire `scratchpad/replay` vierge, hors dépôt.
- **Step 2** `rbs new demo --yes …` — sortie recopiée ; sans `--yes` la commande refuse hors terminal, ce refus est documenté dans la page.
- **Step 3** `docker run --rm -d --name rbs-demo -e POSTGRES_USER=rbs -e POSTGRES_PASSWORD=rbs -e POSTGRES_DB=demo -p 5432:5432 postgres:18` → PostgreSQL 18.6.
- **Step 4** les trois commandes lancées, sorties recopiées telles quelles.
- **Step 5** `cargo run` compile, sert sur 127.0.0.1:8080 ; `/health` 200, `POST /articles` 201, `GET /articles` 200 — réponses recopiées.
- **Step 6** trois manques relevés et ajoutés à la page : `--yes` obligatoire hors terminal, `--core-path` obligatoire tant que `rbs-core` n'est pas publié, collision de nom avec le `rbs` de Ruby sur le `PATH`.
- **Step 7** `docs/docs/getting-started.md` écrit.
- **Step 8** toutes les sorties en blocs `text`, aucune en `bash`.
- **Step 9** `grep -n 'region: create' examples/hello-crud/src/articles/controller.rs` → lignes 28 et 47.
- **Step 10** `/architecture` et `/cli/generate` n'existent pas encore et feraient tomber `onBrokenLinks: 'throw'` ; renvois vers `./guides/logs.md` et la feuille de route.
- **Step 11** miroir français écrit, sorties de terminal inchangées.
- **Step 12** `grep -c '^#'` → 13 des deux côtés.
- **Step 13** `npm run build` → deux `[SUCCESS]` (en, fr).
- **Step 14** parcours rejoué intégralement dans `scratchpad/replay`, base neuve, jusqu'aux trois `curl`.
- **Step 15** commit sur `f3-demarrage-rapide`.
- **Step 16** le réseau était rétabli : `cargo run` a compilé et démarré, la Step 14 n'a pas été interrompue.
- **Step 17** ligne de preuve remise à l'intégration ; `TODO.md` n'est pas touché par cette branche.
