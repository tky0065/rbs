# Lot features P2 : `rbs test`, santé, jobs, doctor, curseur, routes, JSON, binaires, CLAUDE.md

**Date** : 2026-09-13 · **Backlog** : `IMPROVE.md`, tâches 34 à 42 (P2, section « Features »)
· **Designs validés** le 2026-09-13.

Les chemins `templates/…` s'entendent sous `crates/rbs-cli/templates/`, les autres sous
`crates/rbs-cli/src/` ou tels quels depuis la racine.

## Organisation

| Groupe | Branche | Tâches, dans l'ordre | Pourquoi ensemble |
|---|---|---|---|
| G1 | `improve/p2-doctor-jobs` | 37 → 36 | partagent le validateur cron `src/cron.rs` |
| G2 | `improve/p2-sante-claude` | 35 → 42 | régénèrent tous deux les cinq `examples/` |
| G3 | `improve/p2-cli` | 34 → 39 → 38 → 40 | passent tous par `cli.rs` et `lib.rs` |
| — | `improve/p2-features` | 41 | aucun code Rust : faite par l'orchestrateur |

Chaque groupe part de `improve/p2-features` (qui porte cette spec), jamais de `main`.
Conflits attendus à la fusion : `cli.rs`/`lib.rs` (G1 ajoute `generate job`, G3 ses
commandes), `examples/` (G1 régénère event-hub et newsletter-queue, G2 les cinq). Les
exemples se résolvent par régénération, `integration_examples` en oracle. Après fusion,
`generate job` reçoit le `--json` de la tâche 40.

---

## 37 — Contrôles `doctor` pour les fragments qui n'en ont aucun (Easy, G1)

**Problème.** `doctor/mod.rs:303` : `FEATURE_CHECKS: [(&str, Controle); 7]` couvre auth ×2,
redis, mail, storage, jobs, observability. Rien pour cors, rate-limit, scheduler, webhooks,
audit, docker, ci.

**Décision.** Sept contrôles, un par fragment, sur le modèle de `doctor/mail.rs` (un
module par contrôle, `TITRE`, `check`, `check_with` éprouvable) :

| Fragment | Contrôle | Verdict |
|---|---|---|
| `cors` | section `[cors]` de `config/default.toml` (via `section_check`) ; `origins = []` | section absente ✗ ; origines vides ⚠ (`Check::warned`), c'est le défaut sûr : le signaler sans rendre le projet malade |
| `rate-limit` | section `[rate_limit]` | absente ✗ |
| `scheduler` | chaque expression littérale de `Schedule::every::<…>("…"` dans `src/modules/scheduler/mod.rs`, validée par `crate::cron` | illisible ✗, en nommant l'expression et l'erreur du crate ; fichier absent ✗ |
| `webhooks` | la ligne `registre = registre.register::<crate::modules::webhooks::delivery::Delivery>();` dans `src/modules/jobs/mod.rs` | absente ✗, remède = la ligne à coller dans `<rbs:jobs>` (sans elle, chaque livraison part en échec : ancre `jobs` de `webhooks/feature.toml`) |
| `audit` | la migration `create_audit_log` inscrite dans `migration/src/lib.rs` | absente ✗ |
| `docker` | `config/production.toml` présent (le service `api` du compose pose `RBS_ENV: production`) | absent ✗ |
| `ci` | `.github/workflows/ci.yml` présent | absent ✗ |

Écart assumé avec la ligne du backlog : `ci` ne lit pas `config/production.toml`
(`grep -rn production templates/features/ci` ne rend qu'un commentaire), le contrôle vise
donc le fichier que le fragment pose.

**`src/cron.rs`** (nouveau, partagé avec la tâche 36) : `pub(crate) fn valider(expression)
-> Result<String, Erreur>` rendant la forme normalisée. Même normalisation que
la fonction `normaliser` de `templates/features/scheduler/mod.rs.jinja` (5 champs → `0 ` en tête, 6 tels quels,
autre nombre refusé avec le même message), puis `cron::Schedule::from_str`. Nouvelle
dépendance de `rbs-cli` : `cron = "0.17"`, **la version que le fragment déclare**
(`scheduler/feature.toml:49-51`) — même crate, même verdict que le démarrage. Un test
croise les deux : les expressions que les tests du fragment tiennent pour valides ou
invalides le sont aussi pour `crate::cron`.

**Preuves.** Tests unitaires de chaque contrôle (sain, défaut, fichier absent) ;
`cargo test -p rbs-cli doctor::` ; `doctor --json` sur un projet jetable portant les sept
fragments (sortie réelle citée dans le commit). Doc : `docs/docs/cli/doctor.md` et sa
traduction, liste des contrôles.

## 36 — `rbs generate job <nom> [--every "<cron>"]` (Medium, G1)

**Problème.** `cli.rs:138-206` ne connaît que `crud`, `feature`, `client`. Un job s'écrit
en copiant `demo.rs`, et `jobs/mod.rs.jinja:1-5` déclare ses `pub mod` en dur.

**Commande.**

```text
rbs generate job <nom> [--every "<cron>"] [--dry-run] [--force]
```

- `<nom>` en snake_case, validé comme les autres noms (`generate/name.rs`) ; refusé s'il
  heurte un module existant de `src/modules/jobs/` (`config`, `demo`, `model`, `queue`,
  `worker`, `tests`) ou un mot-clé Rust.
- Exige la feature `jobs` installée ; `--every` exige `scheduler`. Sinon refus, remède
  `rbs add jobs` / `rbs add scheduler`.
- `--every` : l'expression passe par `crate::cron::valider` **avant tout plan** — une
  expression illisible est refusée par le CLI plutôt que d'arrêter le démarrage.

**Ce qui est écrit** (séquence lire → planifier → vérifier → afficher → appliquer, comme
`generate feature`) :

1. `src/modules/jobs/<nom>.rs` : `#[derive(Debug, Serialize, Deserialize)] pub struct
   <Nom> {}` et `impl Job for <Nom>` avec `const KIND: &'static str = "<nom>";` et un
   `run` qui journalise (`tracing::info!`) et rend `Ok(())`. Le fichier compile et passe
   clippy tel quel ; son seul commentaire désigne le point d'extension (`run`).
2. `pub mod <nom>;` dans une nouvelle ancre **`<rbs:job_modules>`** de
   `src/modules/jobs/mod.rs`.
3. `registre = registre.register::<<nom>::<Nom>>();` dans `<rbs:jobs>` (existante).
4. Avec `--every` : une instruction qui pousse `Schedule::every::<crate::modules::jobs::<nom>::<Nom>>("<expr>", || …)`
   dans une nouvelle ancre **`<rbs:schedules>`** de `src/modules/scheduler/mod.rs`.

**Ancres** (14 → 16, toutes deux `optional: true`, sur le modèle de `JOBS`,
`anchors.rs:272-283`) :

- `job_modules` dans `templates/features/jobs/mod.rs.jinja`, sous les `pub mod` existants.
- `schedules` dans `templates/features/scheduler/mod.rs.jinja` : `schedules()` est
  **réécrite en instructions** (`let mut calendrier = …; calendrier.push(…); … calendrier`),
  pour la raison que `registry()` documente déjà — une ancre au milieu d'un `vec![]` ne
  survit pas à rustfmt dès qu'un second élément s'y ajoute.
- Chacune déclare une ligne d'accroche **unique** du fichier, vérifiée contre la template
  par le test existant des accroches, pour que `doctor --fix` sache la reposer.
- Un projet antérieur sans ces ancres : insertion sautée, bloc à coller affiché (mécanisme
  `Sautee` existant) ; `doctor` les réclame, `doctor --fix` les repose.

**Idempotence.** Fichier du job déjà présent → le plan le dit, rien n'est réécrit. Un job
n'est pas une feature : rien dans `[package.metadata.rbs].features`.

**À mettre à jour.** `ANCRES` (`anchors.rs:315`), la table des ancres du `CLAUDE.md` racine
(« quatorze » → « seize »), la page des ancres de la doc (EN+FR), `docs/docs/cli/generate.md`
(EN+FR), les guides jobs et scheduler (EN+FR), les exemples `event-hub` et
`newsletter-queue` (templates `jobs` et `scheduler` modifiées), la liste des ancres
d'`AGENTS.md` si elle les énumère.

**Preuves.** Tests unitaires : validation du nom, refus sans `jobs`/`scheduler`, refus
d'une expression illisible, plan attendu, ancre absente → bloc sauté. Test d'intégration
lent : un projet `--with jobs,scheduler`, `generate job purge --every "0 4 * * *"`, le
projet compile et ses tests passent (`--include-ignored`). `integration_examples` vert.

## 35 — `/health/live` (Easy, G2)

**Problème.** `templates/project/src/health/controller.rs.jinja:7-22` : une seule route,
qui sonde la base. Une liveness qui dépend de la base fait redémarrer l'API en boucle
quand c'est la base qui tombe.

**Décision.**

- `controller::live()` : `GET /health/live`, 200 sans rien interroger, `operation_id =
  "health_live"`, documenté `#[utoipa::path]`. Route ajoutée dans
  `templates/project/src/health/mod.rs.jinja`.
- OpenAPI : `crate::health::controller::live,` inséré **avant** `crate::health::controller::health,`
  dans `templates/project/src/openapi.rs.jinja` — cette dernière ligne est l'accroche de
  `<rbs:openapi>` (`anchors.rs:149`) et doit rester immédiatement au-dessus de l'ancre.
- `/health` inchangée : c'est la readiness. Pas d'alias `/health/ready`.
- `HEALTHCHECK` dans `templates/features/docker/Dockerfile.jinja` (étage `runtime`), par
  bash et `/dev/tcp` — l'image `debian:trixie-slim` n'a ni curl ni wget, bash y est :

  ```dockerfile
  HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=3 \
    CMD ["bash", "-c", "exec 3<>/dev/tcp/127.0.0.1/8080 && printf 'GET /health/live HTTP/1.1\\r\\nHost: localhost\\r\\nConnection: close\\r\\n\\r\\n' >&3 && head -n1 <&3 | grep -q ' 200 '"]
  ```

  Commentaire au-dessus : pourquoi bash (pas de client HTTP dans l'image), pourquoi `/live`
  et non `/health`. Dans le Dockerfile plutôt que le compose : il vaut aussi pour un
  `docker run`. Kubernetes l'ignore et configure ses sondes lui-même — la doc le dit.
- `rbs-core` inchangé.

**Preuves.** Test (engendré ou d'intégration) que `/health/live` rend 200 ; le document
OpenAPI engendré porte les deux chemins ; **preuve Docker réelle** : `docker build` d'un
projet engendré `--with docker`, `docker run`, `docker inspect --format '{{.State.Health.Status}}'`
→ `healthy` (sortie citée). Doc EN+FR : pages qui décrivent `/health` (déploiement,
`cli/new.md`, getting-started selon ce qu'elles disent). Cinq exemples régénérés.

## 42 — `CLAUDE.md` engendré (Easy, G2)

**Problème.** `docs/docs/guides/agents.md:15` affirme que Claude Code lit `AGENTS.md` de
lui-même ; il ne le fait que par l'import `@AGENTS.md` d'un `CLAUDE.md`, qu'aucun
gabarit ne pose.

**Décision.**

- `rbs new` écrit `CLAUDE.md` à la racine, une ligne : `@AGENTS.md`.
- `rbs upgrade` le crée **s'il est absent**, et ne réécrit jamais un `CLAUDE.md` existant
  (il appartient à l'utilisateur) — le parc déjà engendré est couvert.
- `guides/agents.md` (EN+FR) : la phrase « rbs n'engendre aucun fichier propre à un outil »
  devient fausse ; dire pourquoi ce fichier d'une ligne existe. `cli/new.md` et
  `cli/upgrade.md` (EN+FR) le listent.
- Les cinq exemples reçoivent leur `CLAUDE.md`.

**Preuves.** Test de `new` (le fichier existe, contenu exact) ; test d'`upgrade` (créé si
absent, intact si présent) ; `integration_examples` vert.

## 34 — `rbs test [FILTRE] [-- <args libtest>]` (Medium, G3)

**Problème.** `docs/docs/guides/testing.md:62-64` prescrit trois commandes à la main ;
`dev/mod.rs:152-176` sait déjà monter le compose, attendre la base et migrer.

**Décision.**

```text
rbs test [FILTRE] [-- <arguments de libtest>]
```

- Étapes : celles de `dev::plan` **sans** `Step::Server` (compose si `docker-compose.yml`,
  attente de la base si le moteur a un serveur, `migrate up`), puis
  `cargo test --workspace --no-fail-fast [FILTRE] -- --include-ignored [args…]` — la
  commande même de la CI engendrée (`templates/features/ci/.github/workflows/ci.yml.jinja:80`).
- Le plan est affiché comme celui de `dev`. Le code de sortie de `cargo test` est propagé
  tel quel : une CI distingue ainsi un test rouge.
- Factoriser plutôt que dupliquer : `dev::plan` produit les étapes communes, chaque
  commande ajoute la sienne.

**Preuves.** Tests unitaires (étapes sans serveur, SQLite sans attente, arguments de cargo
construits avec et sans filtre ni `--`) ; un test lent qui lance `rbs test` sur un projet
engendré et constate le code 0, puis un code non nul après un test rendu rouge. Doc :
`docs/docs/cli/test.md` (EN+FR, et la barre latérale), `guides/testing.md` réécrit autour
de `rbs test` en gardant l'équivalent manuel (EN+FR).

## 39 — `rbs routes [--json]` et `rbs openapi export [--out FICHIER]` (Easy, G3)

**Problème.** `templates/project/src/bin/openapi.rs.jinja` rend le document sans serveur,
`client/document.rs` le parse ; aucune commande ne montre les routes.

**Décision.**

- Obtention du document factorisée hors de `client` : `client/mod.rs:211`
  (`imprime_le_document`) et les deux refus qui le précèdent (projet sans bibliothèque,
  binaire `openapi` absent, avec leur remède) passent dans un module commun que `client`,
  `routes` et `openapi export` appellent.
- `rbs routes` : tableau `MÉTHODE  CHEMIN  OPERATION_ID  GARDE`, trié par chemin puis
  méthode (GET, POST, PUT, PATCH, DELETE) ; `GARDE` = `bearer` quand l'opération (ou le
  document) déclare une `security` non vide, `public` sinon. `--json` : tableau d'objets
  `{ "methode", "chemin", "operation_id", "garde" }`, seul document de la sortie standard.
- `rbs openapi export` : le document sur la sortie standard ; `--out FICHIER` l'écrit dans
  le fichier.
- Retirer au passage ce que la factorisation rend faux dans `client/document.rs` si
  l'extension de `parse` y touche — sans déborder sur la tâche 65 (P3).

**Preuves.** Tests unitaires du rendu (table et JSON) sur un document fixe ; test lent :
`rbs routes --json` sur un projet engendré avec `auth` rend au moins une route `bearer` et
`/health` en `public`. Doc : `docs/docs/cli/routes.md` et `cli/openapi.md` (EN+FR).

## 38 — `rbs generate crud --cursor` (Medium, G3)

**Problème.** `rbs-core/src/pagination.rs` livre `Cursor`/`CursorPage` depuis 1.2.0 ;
aucun gabarit ne les rend.

**Décision.**

- `--cursor` : `GET /<ressource>` prend `Query<Cursor>` (`after`, `per_page`, bornés par
  le noyau) et rend `CursorPage<<Entite>Response>`. Repository : `id < after`,
  `ORDER BY id DESC`, `LIMIT per_page` ; sous `--soft-delete`, la condition
  `deleted_at IS NULL` demeure. `CursorPage::new(data, &cursor, dernier_id)`.
- La **route de filtre reste en `Page`/`Pagination`**, tri libre compris : un curseur sur
  l'`id` est faux dès que le tri porte sur une autre colonne. Le commentaire engendré
  « un seul chemin de lecture » (`repository.rs.jinja:66-67`) dit pourquoi cette entité
  fait exception.
- OpenAPI : `CursorPage<…Response>` enregistré ; le client TypeScript (`generate client`)
  le rend sans intervention.
- Tests engendrés : parcours de pages jusqu'à l'extinction de `next`, absence de doublon
  entre deux pages.
- Compatible avec `--role`, `--soft-delete`, `--with-upload`, `--has-many`.

**Preuves.** Tests unitaires de rendu ; test lent : projet avec une entité `--cursor`,
compilé, tests engendrés passés (`--include-ignored`), puis `generate client` sur ce
projet. Doc : `docs/docs/cli/generate.md` et le guide de pagination/filtrage (EN+FR).

## 40 — `--json` sur les plans et les erreurs (Medium, G3)

**Problème.** `doctor --json` existe (`doctor/json.rs`) ; les plans d'`add`, `generate`,
`upgrade` ne sortent qu'en texte coloré, un agent parse de l'ANSI.

**Décision.**

- `--json` sur `add`, `generate crud|feature|client`, `upgrade` (et `generate job` après
  fusion de G1). Sous `--json`, la sortie standard ne porte **qu'un** document JSON ;
  tout message humain va sur stderr ou se tait.
- Document (clés en français, comme `doctor --json`) :

  ```json
  {
    "commande": "add",
    "racine": "/chemin/du/projet",
    "applique": false,
    "actions": [
      { "chemin": "src/modules/cors/mod.rs", "statut": "a_faire",
        "effet": { "type": "creer", "contenu": "…" } },
      { "chemin": "src/router.rs", "statut": "a_faire",
        "effet": { "type": "inserer", "ancre": "layers", "lignes": ["…"] } }
    ],
    "sautees": [ { "fichier": "…", "ancre": "…", "bloc": "…" } ],
    "fichiers": { "crees": 3, "modifies": 6 }
  }
  ```

  `applique` = `false` sous `--dry-run`, `true` après application. **Contenu complet** des
  fichiers créés. Chaque variante d'`Effect` (`plan/action.rs:20-80`) a sa forme,
  documentée.
- Vue dédiée `plan/json.rs` sur le modèle de `doctor/json.rs` : aucun `Serialize` sur les
  types internes du plan.
- Erreurs sous `--json` : `{ "erreur": { "code": "…", "message": "…", "remede": "…" | null,
  "bloc": "…" | null } }` sur la sortie standard, code de sortie inchangé. Codes stables
  en snake_case (`pas_un_projet`, `arbre_sale`, `ancre_absente`, …), table dans la doc.
- Le rendu humain ne change pas (la tâche 61, P3, s'en occupe).

**Preuves.** Tests unitaires de sérialisation de chaque variante ; tests `assert_cmd`
sans Docker : `rbs new` puis `rbs add cors --dry-run --json` parsé par `serde_json`, rien
d'autre sur stdout ; une erreur d'ancre absente rendue en JSON. Doc : `cli/add.md`,
`cli/generate.md`, `cli/upgrade.md`, `guides/agents.md` (EN+FR), exemple tiré d'une
exécution réelle.

## 41 — Binaires GitHub Releases et `cargo-binstall` (Medium, orchestrateur)

**Décision.**

- `.github/workflows/release.yml` : après `publier`, un job `github-release`
  (`needs: publier`, `permissions: contents: write`) avec
  `taiki-e/create-gh-release-action@v1` (notes tirées de la section de `CHANGELOG.md`),
  puis un job `binaires` en matrice avec `taiki-e/upload-rust-binary-action@v1`, bin
  `rbs`, archives `rbs-<cible>.tar.gz` (`.zip` sous Windows), somme sha256.
- Cibles : `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.
- `[package.metadata.binstall]` dans `crates/rbs-cli/Cargo.toml`, `pkg-url` alignée sur
  ces noms d'archive.
- `README.md`/`README.fr.md` et le guide d'installation (EN+FR) : `cargo binstall rbs-cli`
  et le téléchargement direct.

**Preuves possibles avant un tag** : YAML analysé, `cargo metadata` lit la section
binstall, `cargo package` du CLI inchangé. Le critère « binaires publiés et installables
par binstall » **reste `PARTIEL`** jusqu'au prochain tag poussé.

---

## Règles communes aux trois groupes

- Commits : Conventional Commits en français, sans identifiant de tâche, sans mention du
  backlog ni d'un outil, **sans ligne `Co-Authored-By` ni `Claude-Session`** ; corps avec
  le *pourquoi* et un intertitre `Vérifications :` portant les commandes et leurs sorties
  réelles.
- Documentation bilingue dans le même commit que la page anglaise.
- Bloquant : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`. Toute template touchée : `integration_examples` ; tout exemple
  touché : `npm run build` sous `docs/`. Passes lentes : un binaire de test par commande,
  `--no-fail-fast`, sortie redirigée vers un fichier préfixé du groupe.
- `IMPROVE.md` n'est touché que par l'orchestrateur.
