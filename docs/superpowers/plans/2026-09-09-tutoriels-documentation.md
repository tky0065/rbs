# Tutoriels de la documentation — Plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ajouter au site une section « Tutoriels » de neuf pages pas à pas, un module à la fois, sur un cas concret, en anglais et en français.

**Architecture:** Aucun code Rust n'est écrit. Les neuf pages citent le code des quatre projets d'`examples/` par le plugin `remark-code-from-file` (`file=` / `region=`), et leur sortie décisive est gardée par un marqueur `{/* rbs:transcript */}` que `integration_docs` rejoue. Les guides existants ne sont pas modifiés.

**Tech Stack:** Docusaurus 3 (Markdown + i18n `fr`), le plugin maison `docs/plugins/remark-code-from-file.js`, les tests Rust `integration_docs` et `integration_examples`.

**Spec:** `docs/superpowers/specs/2026-09-09-tutoriels-documentation-design.md`

## Global Constraints

Ces règles valent pour **toutes** les tâches. Elles ne sont pas répétées dans chacune.

- **Bilingue dans le même commit.** Toute page anglaise créée ou modifiée a sa jumelle sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/`, dans le **même** commit. `parite.mjs` compare le dernier commit de chaque paire et échoue sinon.
- **Aucun extrait de code écrit à la main.** Tout bloc de code Rust, TOML, YAML ou TypeScript vient d'`examples/` par ` ```rust file=examples/<projet>/<chemin> region=<nom> `. Les blocs ` ```text ` (sorties de commandes) et ` ```bash ` (commandes tapées) sont la seule exception.
- **Le frontmatter est identique dans les deux langues** sauf `title`, qui est traduit. Exemple vérifié : `getting-started.md` porte `title: Getting started` en anglais, `title: Démarrage rapide` en français, et `sidebar_position: 2` des deux côtés.
- **Le français vouvoie.** Les guides écrivent « votre métier », « inscrivez vos jobs ». Ne pas tutoyer.
- **La charpente des titres est identique dans les deux langues.** Même nombre de `##`, dans le même ordre. `parite.mjs` compare la structure, pas le texte.
- **Les liens relatifs pointent la même cible dans les deux langues.** Depuis `docs/docs/tutorials/`, un guide est `../guides/<nom>.md`, une page CLI `../cli/<nom>.md`, le démarrage rapide `../getting-started.md`.
- **Commits : Conventional Commits, en français, à l'impératif, sans majuscule ni point final.** Aucun identifiant de tâche, aucun renvoi à ce plan ou à un fichier de suivi, aucune ligne `Co-Authored-By` ni `Claude-Session`. Le corps porte le pourquoi, puis un intertitre `Vérifications :` avec les commandes lancées et leur résultat réel.
- **Branche :** `docs/tutoriels`, déjà créée. Ne pas travailler sur `main`.

### Le gabarit d'une page

Les neuf pages suivent cette forme, dans cet ordre. `<…>` marque ce que chaque tâche fournit.

````markdown
---
sidebar_position: <N>
title: <titre>
---

# <titre>

<Deux ou trois phrases : le cas concret, et ce qu'on aura à la fin. Nommer la route ou
le comportement obtenu, pas une abstraction.>

:::note

<Admonition de provenance — texte fixe, voir ci-dessous.>

:::

## Il te faut / Ce qu'il vous faut

<Une ligne renvoyant au prologue.>

## 1. <Première étape>

```bash
<la commande>
```

<Le bloc de sortie. Sur l'étape décisive, il est précédé du marqueur de transcript.>

<Une phrase disant ce que cette sortie prouve.>

## 2. <…>          <!-- trois à cinq étapes par page -->

## Vérifier

```bash
curl <…>
```

```json
<la réponse>
```

## Ce qui a été installé

### <rôle du fichier>

```rust file=examples/<projet>/<chemin> region=<nom>
```

<Une ou deux phrases sur ce que ce fichier fait.>

## Pour aller plus loin

- <lien vers le guide du module>
- <lien vers la page CLI de la commande>
- <lien vers le tutoriel suivant>
````

**L'admonition de provenance**, rigoureusement identique sur les neuf pages, seul le nom du
projet variant :

- Anglais : `The code shown here is read from [`examples/<projet>`](https://github.com/tky0065/rbs/tree/main/examples/<projet>), a project CI compiles. In your own project the same files carry the name of your resource.`
- Français : `Le code montré ici est lu dans [`examples/<projet>`](https://github.com/tky0065/rbs/tree/main/examples/<projet>), un projet que la CI compile. Chez vous, les mêmes fichiers portent le nom de votre ressource.`

### La forme d'un marqueur de transcript

Vérifié en session : `rbs add` n'a **pas** besoin d'un PostgreSQL joignable. Aucun marqueur
de cette section ne porte donc `base="oui"`, et tous sont rejoués par le test rapide.

```
{/* rbs:transcript cmd="rbs add jobs" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git commit -q -m init" dans="demo" */}
```

Trois points que l'implémenteur doit connaître, tous lus dans `crates/rbs-cli/tests/integration_docs.rs` :

- `setup=` découpe sur ` && ` et lance chaque commande ; le `git commit` est obligatoire, `add` refusant un arbre de travail sale et `rbs new` n'ayant pas committé.
- `normalise()` masque des deux côtés les horodatages (`m20260909_090303` → `m<horodatage>`), les versions, les durées et les adresses. Le chemin du tmpdir devient `<tmp>`, et `…/` dans la page devient `<tmp>/` : une page écrit donc `plan pour …/demo`.
- Si un bloc résiste malgré tout, `extrait="oui"` réduit l'oracle aux lignes citées, cherchées dans l'ordre.

## Structure des fichiers

**Créés — 20 :**

| Fichier | Responsabilité |
|---|---|
| `docs/docs/tutorials/_category_.json` | Libellé et position 3 de la catégorie |
| `docs/docs/tutorials/setup.md` | Prologue : le projet que les huit autres reprennent |
| `docs/docs/tutorials/first-resource.md` | `generate crud` |
| `docs/docs/tutorials/auth.md` | `add auth` |
| `docs/docs/tutorials/storage.md` | `add storage` |
| `docs/docs/tutorials/mail.md` | `add mail` |
| `docs/docs/tutorials/cache.md` | `add redis` |
| `docs/docs/tutorials/jobs.md` | `add jobs` |
| `docs/docs/tutorials/observability.md` | `add observability` |
| `docs/docs/tutorials/typescript-client.md` | `generate client --lang ts` |
| `docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/` | Les dix mêmes, traduits |

**Modifiés :**

| Fichier | Modification |
|---|---|
| `docs/i18n/fr/docusaurus-plugin-content-docs/current.json` | Clé `sidebar.docsSidebar.category.Tutorials` |
| `docs/docs/architecture.md` + fr | `sidebar_position` 3 → 4 |
| `docs/docs/cli/_category_.json` + fr | `position` 4 → 5 |
| `docs/docs/guides/_category_.json` + fr | `position` 5 → 6 |
| `docs/docs/compatibility.md` + fr | `sidebar_position` 6 → 7 |
| `docs/docs/intro.md` + fr | Une entrée « Tutorials » en tête de « Where to go next » |
| `examples/newsletter-queue/src/modules/observability/mod.rs` | Un marqueur `// region:` (tâche 8 seulement) |

Les dix-neuf guides ne sont pas touchés.

---

### Task 1: La charpente de la section et le prologue

La catégorie, le décalage des positions, et la page 0. Elles vont ensemble : une catégorie
sans page fait échouer le build, et un décalage de positions à moitié posé donne une
sidebar dans le désordre.

**Files:**
- Create: `docs/docs/tutorials/_category_.json`, `docs/docs/tutorials/setup.md`
- Create: `docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/_category_.json`, `.../tutorials/setup.md`
- Modify: `docs/i18n/fr/docusaurus-plugin-content-docs/current.json`
- Modify: `docs/docs/architecture.md:3`, `docs/docs/compatibility.md:3`, et leurs jumelles fr
- Modify: `docs/docs/cli/_category_.json`, `docs/docs/guides/_category_.json`, et leurs jumelles fr
- Test: `crates/rbs-cli/tests/integration_docs.rs` (non modifié — il découvre les pages seul)

**Interfaces:**
- Consomme : rien.
- Produit : `tutorials/setup.md`, dont les huit tâches suivantes citent le lien `./setup.md` dans leur section « Il te faut », et le projet `demo` créé par `rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo`, dont chaque page suivante reprend le nom dans son `setup=`.

- [ ] **Step 1: Écrire le marqueur et son bloc, et le faire échouer**

Créer `docs/docs/tutorials/setup.md` avec, en section « 1. Créer le projet », un marqueur
dont le bloc de sortie est **délibérément faux** (une ligne modifiée), pour vérifier que
l'oracle mord :

````markdown
{/* rbs:transcript cmd="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" */}
```text
$ rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
✓ demo créé — 99 fichiers
```
````

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: FAIL, « `rbs new …` ne rend plus ce que la page montre », le diff opposant `99 fichiers` à `21 fichiers`.

- [ ] **Step 3: Corriger le bloc avec la sortie réelle**

Lancer la commande pour de bon et coller sa sortie. Elle est, à la date d'écriture :

```text
$ rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
✓ demo créé — 21 fichiers

  cd demo
  docker compose up -d   # la base du .env, montée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

- [ ] **Step 4: Relancer le test et vérifier qu'il passe**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: PASS.

- [ ] **Step 5: Écrire le reste du prologue, dans les deux langues**

`title: Setting up` / `title: Préparer le terrain`, `sidebar_position: 1`. Quatre étapes,
suivant le gabarit : `rbs new` (l'étape décisive, marquée), `cd demo && docker compose up -d`,
`rbs migrate up`, `cargo run`. La section « Vérifier » interroge `curl localhost:8080/health`.

La page ne cite **aucun** code — elle ne montre que des commandes — et sa section « Ce qui a
été installé » est donc remplacée par un renvoi vers `../architecture.md`.

Pour l'installation du CLI, renvoyer à `../getting-started.md` plutôt que de la répéter.

- [ ] **Step 6: Créer la catégorie dans les deux langues**

`docs/docs/tutorials/_category_.json` :

```json
{
  "label": "Tutorials",
  "position": 3
}
```

Le fichier français porte le même contenu — Docusaurus lit le libellé traduit dans
`current.json`, où il faut ajouter, à côté des deux clés existantes :

```json
  "sidebar.docsSidebar.category.Tutorials": {
    "message": "Tutoriels",
    "description": "The label for category Tutorials in sidebar docsSidebar"
  }
```

- [ ] **Step 7: Décaler les positions**

`architecture.md` 3 → 4, `cli/_category_.json` 4 → 5, `guides/_category_.json` 5 → 6,
`compatibility.md` 6 → 7. Les quatre, dans les deux langues, soit huit fichiers.

- [ ] **Step 8: Construire le site et vérifier la parité**

Run: `cd docs && npm run clear && npm run build && node scripts/parite.mjs`
Expected: le build passe (aucun lien mort, les deux locales), `parite.mjs` ne signale rien
sur `tutorials/setup.md`.

- [ ] **Step 9: Commit**

```bash
git add docs/docs/tutorials docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials \
  docs/i18n/fr/docusaurus-plugin-content-docs/current.json \
  docs/docs/architecture.md docs/docs/compatibility.md \
  docs/docs/cli/_category_.json docs/docs/guides/_category_.json \
  docs/i18n/fr/docusaurus-plugin-content-docs/current/architecture.md \
  docs/i18n/fr/docusaurus-plugin-content-docs/current/compatibility.md \
  docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/_category_.json \
  docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/_category_.json
git commit -m "docs: ouvre une section tutoriels sur un prologue commun"
```

---

### Task 2: Ma première ressource — `generate crud`

**Files:**
- Create: `docs/docs/tutorials/first-resource.md` et sa jumelle fr
- Test: `cargo test -p rbs-cli --test integration_docs`

**Interfaces:**
- Consomme : `./setup.md` (le projet `demo`).
- Produit : le lien `./first-resource.md`, cité par la tâche 3 et par la section « Pour aller plus loin » du prologue.

- [ ] **Step 1: Écrire le marqueur avec un bloc faux**

`sidebar_position: 2`, `title: Your first resource` / `Votre première ressource`. Le cas :
des articles qu'on crée, liste et modifie. L'étape décisive est la génération :

````markdown
{/* rbs:transcript cmd="rbs generate crud articles --fields title:string,body:text,published:bool" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo" dans="demo" */}
```text
$ rbs generate crud articles --fields title:string,body:text,published:bool
CETTE LIGNE EST FAUSSE
```
````

`generate` ne demande pas d'arbre propre : le `setup=` n'a pas besoin de `git commit`.

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: FAIL sur `first-resource.md`, avec la sortie réelle en regard.

- [ ] **Step 3: Coller la sortie réelle et écrire la page**

Reprendre la sortie que l'échec vient d'imprimer. Les étapes : générer, `rbs migrate up`,
`cargo run`. La section « Vérifier » fait un `POST /articles` au `curl` puis un `GET`.

Les commandes reprennent **exactement** celles qui construisent `hello-crud`
(`examples/README.md`), sans quoi le code cité ne correspondrait pas :
`--fields 'title:string,body:text,published:bool'`.

- [ ] **Step 4: Écrire « Ce qui a été installé »**

Quatre extraits, tous existants — ne créer aucune région :

````markdown
```rust file=examples/hello-crud/src/articles/model.rs region=entite
```
```rust file=examples/hello-crud/src/articles/dto.rs region=entree
```
```rust file=examples/hello-crud/src/articles/controller.rs region=create
```
```rust file=examples/hello-crud/src/articles/repository.rs region=list
```
````

Chacun sous un titre disant le rôle du fichier, et suivi d'une ou deux phrases. C'est ici
qu'on énonce la règle de dépendance — `controller → service → repository → model` — en
renvoyant à `../architecture.md` pour le détail.

- [ ] **Step 5: Traduire la page**

Même charpente de titres, mêmes `file=`/`region=`, mêmes cibles de liens. Seul le texte change.

- [ ] **Step 6: Vérifier**

Run: `cargo test -p rbs-cli --test integration_docs && cd docs && npm run clear && npm run build && node scripts/parite.mjs`
Expected: les trois passent.

- [ ] **Step 7: Commit**

```bash
git add docs/docs/tutorials/first-resource.md docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/first-resource.md
git commit -m "docs: ajoute le tutoriel du premier CRUD"
```

---

### Task 3: Fermer l'API aux inconnus — `add auth`

**Files:**
- Create: `docs/docs/tutorials/auth.md` et sa jumelle fr

**Interfaces:**
- Consomme : `./setup.md`, `./first-resource.md`.
- Produit : le lien `./auth.md`.

- [ ] **Step 1: Écrire le marqueur avec un bloc faux**

`sidebar_position: 3`, `title: Locking the API down` / `Fermer l'API aux inconnus`. Le cas :
tout le monde lit les billets, seul un administrateur en écrit.

````markdown
{/* rbs:transcript cmd="rbs add auth" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git commit -q -m init" dans="demo" */}
```text
$ rbs add auth
CETTE LIGNE EST FAUSSE
```
````

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: FAIL sur `auth.md`.

Si l'échec porte sur le `git commit` plutôt que sur le bloc (« Author identity unknown »),
c'est que l'environnement n'a pas de `user.email` : passer
`git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init` dans le `setup=`.

- [ ] **Step 3: Coller la sortie réelle et écrire la page**

Étapes : `rbs add auth`, `rbs migrate up`, puis la génération protégée —
`rbs generate crud posts --fields 'title:string,body:text,published:bool' --role admin`,
exactement la commande qui construit `blog-auth`.

Le point à expliquer, et c'est le cœur pédagogique de la page : sur un projet portant
`auth`, `generate crud` ferme **toutes** les routes qu'il écrit au seuil le plus bas ;
`--role admin` ne relève que les trois écritures. Sans le drapeau, la lecture serait
protégée elle aussi.

La section « Vérifier » montre les trois réponses qui séparent les régimes : `401` sans
jeton, `403` avec un jeton `user` sur `POST /posts`, `200` avec ce même jeton sur `GET /posts`.

- [ ] **Step 4: Écrire « Ce qui a été installé »**

Trois extraits, tous existants :

````markdown
```rust file=examples/blog-auth/src/posts/controller.rs region=create
```
```rust file=examples/blog-auth/src/auth/guard.rs region=require_role
```
```rust file=examples/blog-auth/src/posts/tests.rs region=refus
```
````

- [ ] **Step 5: Traduire la page**

- [ ] **Step 6: Vérifier**

Run: `cargo test -p rbs-cli --test integration_docs && cd docs && npm run clear && npm run build && node scripts/parite.mjs`
Expected: les trois passent.

- [ ] **Step 7: Commit**

```bash
git add docs/docs/tutorials/auth.md docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/auth.md
git commit -m "docs: ajoute le tutoriel de la protection des routes"
```

---

### Task 4: Recevoir un fichier — `add storage`

**Files:**
- Create: `docs/docs/tutorials/storage.md` et sa jumelle fr

**Interfaces:**
- Consomme : `./setup.md`.
- Produit : le lien `./storage.md`, cité par la tâche 5.

- [ ] **Step 1: Écrire le marqueur avec un bloc faux**

`sidebar_position: 4`, `title: Taking a file` / `Recevoir un fichier`. Le cas : un client
dépose un justificatif ; à la fin, `PUT /uploads/{id}/content` range le fichier et répond `204`.

````markdown
{/* rbs:transcript cmd="rbs add storage" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git commit -q -m init" dans="demo" */}
```text
$ rbs add storage
CETTE LIGNE EST FAUSSE
```
````

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: FAIL sur `storage.md`.

- [ ] **Step 3: Coller la sortie réelle et écrire la page**

Étapes : `rbs add storage`, puis la génération qui monte les routes de contenu —
`rbs generate crud uploads --fields 'title:string,owner_email:string,content_type:string,size:int' --with-upload`,
la commande de `file-drop`.

Deux choses à expliquer : `add storage` ne monte **aucune** route (comme toutes les
briques), c'est `--with-upload` qui écrit les trois gestionnaires `PUT`, `GET` et `HEAD` ;
et `owner_email` gagne sa contrainte de validation dans le DTO du seul fait de son suffixe.

`add mail` et `add redis` ne sont **pas** lancés ici. La page prévient que `file-drop` les
porte, ce qui explique les lignes de son service que cette page ne montre pas.

- [ ] **Step 4: Écrire « Ce qui a été installé »**

Trois extraits, tous existants :

````markdown
```rust file=examples/file-drop/src/modules/storage/mod.rs region=trait
```
```rust file=examples/file-drop/src/uploads/controller.rs region=put_content
```
```rust file=examples/file-drop/src/uploads/service.rs region=contenu
```
````

- [ ] **Step 5: Traduire la page**

- [ ] **Step 6: Vérifier**

Run: `cargo test -p rbs-cli --test integration_docs && cd docs && npm run clear && npm run build && node scripts/parite.mjs`
Expected: les trois passent.

- [ ] **Step 7: Commit**

```bash
git add docs/docs/tutorials/storage.md docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/storage.md
git commit -m "docs: ajoute le tutoriel du dépôt de fichiers"
```

---

### Task 5: Envoyer un mail — `add mail`

**Files:**
- Create: `docs/docs/tutorials/mail.md` et sa jumelle fr

**Interfaces:**
- Consomme : `./storage.md` (l'accusé porte sur le dépôt de la page précédente).
- Produit : le lien `./mail.md`.

- [ ] **Step 1: Écrire le marqueur avec un bloc faux**

`sidebar_position: 5`, `title: Sending mail` / `Envoyer un mail`. Le cas : l'accusé de
réception du dépôt.

````markdown
{/* rbs:transcript cmd="rbs add mail" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git commit -q -m init" dans="demo" */}
```text
$ rbs add mail
CETTE LIGNE EST FAUSSE
```
````

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: FAIL sur `mail.md`.

- [ ] **Step 3: Coller la sortie réelle et écrire la page**

Le point pédagogique : `send_detached` n'attend pas l'envoi, et c'est un choix qui se paie —
une erreur n'y laisse qu'une ligne de log. La page dit quand c'est ce qu'on veut (un accusé
de réception) et quand ça ne l'est pas (renvoyer vers `./jobs.md`, où l'attente permet le
réessai).

Signaler le mécanisme `[[env]]` : le fragment dépose dans `.env` un secret tiré au hasard,
et la configuration SMTP est à renseigner.

- [ ] **Step 4: Écrire « Ce qui a été installé »**

Trois extraits, tous existants :

````markdown
```rust file=examples/file-drop/src/modules/mail/service.rs region=send_template
```
```rust file=examples/file-drop/src/modules/mail/service.rs region=send_detached
```
```rust file=examples/file-drop/src/uploads/service.rs region=notify
```
````

- [ ] **Step 5: Traduire la page**

- [ ] **Step 6: Vérifier**

Run: `cargo test -p rbs-cli --test integration_docs && cd docs && npm run clear && npm run build && node scripts/parite.mjs`
Expected: les trois passent.

- [ ] **Step 7: Commit**

```bash
git add docs/docs/tutorials/mail.md docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/mail.md
git commit -m "docs: ajoute le tutoriel de l'envoi de mail"
```

---

### Task 6: Ne pas recalculer deux fois — `add redis`

**Files:**
- Create: `docs/docs/tutorials/cache.md` et sa jumelle fr

**Interfaces:**
- Consomme : `./storage.md` (la liste mise en cache est celle des dépôts).
- Produit : le lien `./cache.md`.

- [ ] **Step 1: Écrire le marqueur avec un bloc faux**

`sidebar_position: 6`, `title: Not computing twice` / `Ne pas recalculer deux fois`. Le cas :
un `COUNT(*)` lu mille fois par minute, que trois écritures invalident.

````markdown
{/* rbs:transcript cmd="rbs add redis" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git commit -q -m init" dans="demo" */}
```text
$ rbs add redis
CETTE LIGNE EST FAUSSE
```
````

Le nom de la commande est `redis`, le module qu'elle installe s'appelle `cache` : la page
doit le dire, c'est un piège documenté dans `examples/README.md` (« `rbs add redis` writes
`mod cache;`, not `mod redis;` »).

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: FAIL sur `cache.md`.

- [ ] **Step 3: Coller la sortie réelle et écrire la page**

Le point pédagogique, tiré de `file-drop` : c'est le **total** qui est mis en cache, pas la
page de résultats — `Page` n'est que `Serialize`, et la relire du cache demanderait de la
rendre désérialisable dans le noyau. Un débutant apprend ici que ce qu'on met en cache se
choisit, et pourquoi.

L'invalidation vient avec : les trois écritures appellent `invalidate_prefix`.

- [ ] **Step 4: Écrire « Ce qui a été installé »**

Trois extraits, tous existants :

````markdown
```rust file=examples/file-drop/src/modules/cache/mod.rs region=lecture
```
```rust file=examples/file-drop/src/modules/cache/mod.rs region=invalidate_prefix
```
```rust file=examples/file-drop/src/uploads/service.rs region=list
```
````

- [ ] **Step 5: Traduire la page**

- [ ] **Step 6: Vérifier**

Run: `cargo test -p rbs-cli --test integration_docs && cd docs && npm run clear && npm run build && node scripts/parite.mjs`
Expected: les trois passent.

- [ ] **Step 7: Commit**

```bash
git add docs/docs/tutorials/cache.md docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/cache.md
git commit -m "docs: ajoute le tutoriel de la mise en cache"
```

---

### Task 7: Sortir le travail long de la requête — `add jobs`

**Files:**
- Create: `docs/docs/tutorials/jobs.md` et sa jumelle fr

**Interfaces:**
- Consomme : `./setup.md`, et renvoie à `./mail.md` pour le contraste avec `send_detached`.
- Produit : le lien `./jobs.md`.

- [ ] **Step 1: Écrire le marqueur avec un bloc faux**

`sidebar_position: 7`, `title: Moving long work out of the request` / `Sortir le travail
long de la requête`. Le cas : 5 000 lettres enfilées sans faire attendre l'appelant.

````markdown
{/* rbs:transcript cmd="rbs add jobs" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git commit -q -m init" dans="demo" */}
```text
$ rbs add jobs
CETTE LIGNE EST FAUSSE
```
````

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: FAIL sur `jobs.md`.

- [ ] **Step 3: Coller la sortie réelle et écrire la page**

La sortie réelle, obtenue en session et donnée ici pour que l'implémenteur reconnaisse la
forme attendue (l'horodatage de la migration et le chemin varient, et sont masqués par
`normalise()`) :

```text
$ rbs add jobs
jobs : jobs en arrière-plan : une table, un enfilage transactionnel, un worker qui réessaie

plan pour …/demo

  + src/modules/jobs/mod.rs                         créé
  + src/modules/jobs/config.rs                      créé
  + src/modules/jobs/model.rs                       créé
  + src/modules/jobs/queue.rs                       créé
  + src/modules/jobs/worker.rs                      créé
  + src/modules/jobs/demo.rs                        créé
  + src/modules/jobs/tests.rs                       créé
  + migration/src/m20260909_090303_create_jobs.rs   créé
  ~ migration/src/lib.rs                            modifié
  + src/modules/mod.rs                              créé
  ~ src/lib.rs                                      modifié
  ~ src/main.rs                                     modifié
  ~ Cargo.toml                                      modifié
  ~ config/default.toml                             modifié
  ~ AGENTS.md                                       modifié

  15 fichiers à écrire
✓ jobs installée — 8 fichiers

  rbs migrate up, puis inscrivez vos jobs dans src/modules/jobs/mod.rs
```

Les étapes suivantes : `rbs migrate up` (sans la table `jobs`, le worker démarre et ne lit
rien), l'inscription du job dans `registry()`, et l'enfilage depuis un service.

Le point pédagogique, qui est le cœur du module : la file est une **ligne en base**, donc
l'enfilage vit dans la transaction qui l'a motivé. Un travail poussé dans Redis survit au
rollback qui l'annule ; ici, le job existe si et seulement si le travail qui l'a motivé
existe. Dire aussi le prix — le débit est borné par la base.

- [ ] **Step 4: Écrire « Ce qui a été installé »**

Trois extraits, tous existants :

````markdown
```rust file=examples/newsletter-queue/src/modules/jobs/newsletter.rs region=job
```
```rust file=examples/newsletter-queue/src/modules/jobs/mod.rs region=registry
```
```rust file=examples/newsletter-queue/src/subscribers/service.rs region=broadcast
```
````

Sur le dernier, souligner le `&transaction` : sur `db`, les lettres survivraient au
rollback qui les annule.

- [ ] **Step 5: Traduire la page**

- [ ] **Step 6: Vérifier**

Run: `cargo test -p rbs-cli --test integration_docs && cd docs && npm run clear && npm run build && node scripts/parite.mjs`
Expected: les trois passent.

- [ ] **Step 7: Commit**

```bash
git add docs/docs/tutorials/jobs.md docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/jobs.md
git commit -m "docs: ajoute le tutoriel de la file de travaux"
```

---

### Task 8: Voir ce que fait l'API — `add observability`

La seule tâche qui touche `examples/`, et donc la seule qui doit rejouer le test de
non-dérive.

**Files:**
- Create: `docs/docs/tutorials/observability.md` et sa jumelle fr
- Modify: `examples/newsletter-queue/src/modules/observability/mod.rs` (un marqueur `// region:`)

**Interfaces:**
- Consomme : `./setup.md`.
- Produit : le lien `./observability.md`.

- [ ] **Step 1: Vérifier ce que le module expose et poser la région**

Run: `grep -n "region:" examples/newsletter-queue/src/modules/observability/*.rs`
Expected: seule `tests.rs` porte une région (`cardinalite`).

Ouvrir `examples/newsletter-queue/src/modules/observability/mod.rs`. La fonction
`pub async fn serve` y commence ligne 37 ; encadrer la portion qui va du `let listener` au
`);` fermant le `Router` — soit les lignes 49 à 59 dans l'état actuel du fichier —, en
plaçant les marqueurs seuls sur leur ligne, à l'indentation du code encadré :

```rust
    // region: exposition
    let listener = tokio::net::TcpListener::bind(&adresse)
        .await
        .with_context(|| format!("impossible d'écouter les métriques sur {adresse}"))?;

    let app = Router::new().route(
        "/metrics",
        get(move || {
            let handle = handle.clone();
            async move { handle.render() }
        }),
    );
    // endregion: exposition
```

C'est le fragment qui dit l'essentiel de la page : un listener à soi, une seule route. Le
plugin désindente d'après la ligne la moins indentée, donc les quatre espaces du corps de
fonction ne se retrouvent pas dans le rendu.

Ne rien modifier d'autre dans le fichier. Le nom `exposition` est déjà employé dans
`hello-crud/src/openapi.rs` pour le même rôle ; le réemployer garde la doc cohérente.

- [ ] **Step 2: Vérifier que la région n'a pas fait dériver l'exemple**

Run: `cargo test -p rbs-cli --test integration_examples`
Expected: PASS. `the_region_markers_are_ignored` garantit que les lignes de marqueur sont
écartées de la comparaison octet à octet ; si le test échoue, c'est que le marqueur n'est
pas seul sur sa ligne ou n'a pas la forme `// region: <nom>`.

- [ ] **Step 3: Écrire le marqueur de transcript avec un bloc faux**

`sidebar_position: 8`, `title: Seeing what the API does` / `Voir ce que fait l'API`. Le cas :
pourquoi cette route est lente.

````markdown
{/* rbs:transcript cmd="rbs add observability" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git commit -q -m init" dans="demo" */}
```text
$ rbs add observability
CETTE LIGNE EST FAUSSE
```
````

- [ ] **Step 4: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: FAIL sur `observability.md`.

- [ ] **Step 5: Coller la sortie réelle et écrire la page**

Le point pédagogique : le listener des métriques est **séparé** de celui de l'application,
et c'est délibéré — on n'expose pas `/metrics` sur le port public. La page montre le port
dans la configuration, puis un `curl` sur `/metrics`.

Le second point, celui qui évite l'erreur classique du débutant : la **cardinalité**. Une
étiquette portant un identifiant fait exploser le nombre de séries. C'est ce que garde
`tests.rs region=cardinalite`.

- [ ] **Step 6: Écrire « Ce qui a été installé »**

Trois extraits, dont un créé au step 1 :

````markdown
```toml file=examples/newsletter-queue/config/default.toml region=metriques
```
```rust file=examples/newsletter-queue/src/modules/observability/mod.rs region=exposition
```
```rust file=examples/newsletter-queue/src/modules/observability/tests.rs region=cardinalite
```
````

- [ ] **Step 7: Traduire la page**

- [ ] **Step 8: Vérifier**

Run: `cargo test -p rbs-cli --test integration_docs --test integration_examples && cd docs && npm run clear && npm run build && node scripts/parite.mjs`
Expected: les quatre passent. Le `build` échouerait si la région `exposition` n'était pas
résolue par le plugin — c'est lui qui prouve que le marqueur du step 1 est bien placé.

- [ ] **Step 9: Commit**

```bash
git add docs/docs/tutorials/observability.md \
  docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/observability.md \
  examples/newsletter-queue/src/modules/observability/mod.rs
git commit -m "docs: ajoute le tutoriel des métriques"
```

---

### Task 9: Appeler l'API en TypeScript — `generate client`

**Files:**
- Create: `docs/docs/tutorials/typescript-client.md` et sa jumelle fr

**Interfaces:**
- Consomme : `./first-resource.md` (le client engendré est celui du CRUD `articles`).
- Produit : le lien `./typescript-client.md`, dernier de la section.

- [ ] **Step 1: Écrire le marqueur avec un bloc faux**

`sidebar_position: 9`, `title: Calling the API from TypeScript` / `Appeler l'API en
TypeScript`. Le cas : un front qui consomme le CRUD sans réécrire les types à la main.

````markdown
{/* rbs:transcript cmd="rbs generate client --lang ts" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && rbs generate crud articles --fields title:string,body:text,published:bool" dans="demo" */}
```text
$ rbs generate client --lang ts
CETTE LIGNE EST FAUSSE
```
````

Le `setup=` génère le CRUD d'abord : sans ressource, le client engendré n'aurait aucune
méthode.

- [ ] **Step 2: Lancer le test et vérifier qu'il échoue**

Run: `cargo test -p rbs-cli --test integration_docs -- --exact the_marked_transcripts_still_render_what_the_docs_show`
Expected: FAIL sur `typescript-client.md`.

- [ ] **Step 3: Coller la sortie réelle et écrire la page**

Le point pédagogique : le client est lu de l'OpenAPI que le projet expose déjà, donc il
suit le serveur sans qu'on écrive un type deux fois. Le regénérer après chaque
`generate crud` fait partie de la boucle.

- [ ] **Step 4: Écrire « Ce qui a été installé »**

Deux extraits, tous existants :

````markdown
```typescript file=examples/hello-crud/clients/ts/client.ts region=classe
```
```typescript file=examples/hello-crud/clients/ts/client.ts region=methodes
```
````

Vérifier la langue du bloc : `parite.mjs` compare la langue et la méta des blocs entre FR
et EN, et le plugin lit `file=` quelle que soit la langue déclarée. Employer la même des
deux côtés.

- [ ] **Step 5: Traduire la page**

- [ ] **Step 6: Vérifier**

Run: `cargo test -p rbs-cli --test integration_docs && cd docs && npm run clear && npm run build && node scripts/parite.mjs`
Expected: les trois passent.

- [ ] **Step 7: Commit**

```bash
git add docs/docs/tutorials/typescript-client.md docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/typescript-client.md
git commit -m "docs: ajoute le tutoriel du client typescript"
```

---

### Task 10: Les raccords et la passe de vérification complète

**Files:**
- Modify: `docs/docs/intro.md` et sa jumelle fr
- Modify: les neuf pages de `tutorials/`, section « Pour aller plus loin », dans les deux langues

**Interfaces:**
- Consomme : les neuf pages des tâches 1 à 9.
- Produit : rien — c'est la tâche terminale.

- [ ] **Step 1: Inscrire la section dans l'introduction**

Dans `docs/docs/intro.md`, section « Where to go next », insérer une entrée **juste après**
celle de « Getting started » — un lecteur installe avant de suivre un tutoriel :

```markdown
- **[Tutorials](./tutorials/setup.md)** — nine step-by-step pages, one module at a time,
  each on a concrete case.
```

Et sa traduction dans la jumelle française.

- [ ] **Step 2: Chaîner les pages**

Chaque page de `tutorials/` termine sa section « Pour aller plus loin » par un lien vers la
suivante, et la dernière (`typescript-client.md`) renvoie aux guides plutôt qu'à une page
inexistante. Vérifier les neuf, dans les deux langues.

- [ ] **Step 3: Faire échouer le build sur un lien mort, pour prouver que le garde-fou mord**

Introduire volontairement une faute dans un lien (`./setup-typo.md`) et lancer le build.

Run: `cd docs && npm run build`
Expected: FAIL, `onBrokenLinks: 'throw'` nommant le lien. Rétablir ensuite.

- [ ] **Step 4: La passe complète**

Les cinq commandes de la spec, dans l'ordre, chacune jusqu'au bout :

```bash
cd docs && npm run clear && npm run build && node scripts/parite.mjs && cd ..
cargo test -p rbs-cli --test integration_docs
cargo test -p rbs-cli --test integration_docs -- --ignored
cargo test -p rbs-cli --test integration_examples
```

Expected: toutes passent. La troisième exige Docker ; elle ne couvre aucun transcrit neuf
(aucun ne porte `base="oui"`) mais prouve que les anciens n'ont pas été cassés par le
décalage des positions.

- [ ] **Step 5: Compter les transcrits pour vérifier qu'aucun n'a été perdu**

Run: `grep -ro "rbs:transcript" docs/docs docs/i18n --include="*.md" | wc -l`
Expected: `38` — les 20 d'avant (10 par langue) plus les 9 pages × 2 langues.

- [ ] **Step 6: Commit**

```bash
git add docs/docs/intro.md docs/i18n/fr/docusaurus-plugin-content-docs/current/intro.md docs/docs/tutorials docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials
git commit -m "docs: raccorde les tutoriels à l'introduction"
```

---

## Ce que ce plan ne fait pas

Rappelé ici pour qu'aucune tâche ne dérive vers ces terrains, tous écartés par la spec :

- **Pas de tutoriel pour `scheduler`, `webhooks`, `audit`, `cors`, `rate-limit`, `ci`, `docker`** — aucun exemple ne porte leur code, et la documentation n'écrit aucun extrait à la main.
- **Aucun guide n'est modifié.** Si un tutoriel semble avoir besoin d'un paragraphe qui vit dans un guide, il y renvoie.
- **Aucun cinquième projet dans `examples/`.**
- **Les `curl` ne sont pas gardés** par un transcrit : le harnais lance une commande dans un répertoire temporaire, il ne démarre pas de serveur.
- **Aucune ligne de Rust dans `crates/`.** Le seul code touché est le marqueur de la tâche 8.
