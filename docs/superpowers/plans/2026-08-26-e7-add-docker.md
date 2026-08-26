# `rbs add docker` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:test-driven-development.

**Goal:** Qu'un projet généré reçoive son `Dockerfile`, son `docker-compose.yml` et son
`.dockerignore` par une commande, et que `docker compose up` en fasse une API qui répond
sur une base migrée.

**Architecture:** `add` n'invente aucune mécanique. Elle reprend la séquence de
`generate::commande` — racine, garde Git, plan, affichage, application — sur un catalogue
de fichiers rendus depuis `templates/features/<feature>/`. Aucune ancre n'est touchée :
le plan ne porte que des `Creer` et un `PatchToml::InscrireFeature`, dont `action.rs`
annonce déjà qu'il les attend.

Le point neuf est la provenance des templates : `templates.rs` n'embarque que le
squelette. Il embarque désormais aussi `templates/features/`, et `Source` sait s'ouvrir
sur un sous-répertoire de feature.

**Tech Stack:** Rust, `include_dir`, `minijinja`, `toml_edit`, Docker Compose.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §3.1, §4.1, §4.2 · conséquence
assumée de D7 (PostgreSQL 18 minimum, l. 38).

## Global Constraints

- Branche dédiée `e7-add-docker`.
- **PostgreSQL 18** dans le compose : `uuidv7()` n'est natif qu'à partir de là, et D7 en
  dépend. Pas de `postgres:16`, pas de `postgres:latest`.
- `config/default.toml` fixe `host = "127.0.0.1"` : une API conteneurisée avec ce défaut
  est injoignable depuis l'hôte. Le compose passe `RBS_SERVER__HOST=0.0.0.0`, qui l'emporte
  par la couche « variables d'environnement » de `rbs_core::config`. Ne pas toucher au TOML.
- Une template porte le suffixe `.jinja`, une destination ne le porte jamais.
  `templates::destination` est le seul endroit qui connaît cette convention.
- Le code généré ne porte pas de bandeau « généré, ne pas modifier ».
- `clippy -D warnings` et `fmt --check` bloquants ; un `///` d'une à trois lignes par item.

## File Structure

- Create: `templates/features/docker/Dockerfile.jinja`
- Create: `templates/features/docker/docker-compose.yml.jinja`
- Create: `templates/features/docker/.dockerignore.jinja`
- Create: `crates/rbs-cli/src/add/mod.rs` — séquence commune, dispatch, erreurs
- Create: `crates/rbs-cli/src/add/docker.rs` — catalogue et contexte de rendu de `docker`
- Modify: `crates/rbs-cli/src/templates.rs` — `Source::feature`, second `include_dir!`
- Modify: `crates/rbs-cli/src/metadata.rs` — `nom_du_paquet`
- Modify: `crates/rbs-cli/src/main.rs` — `mod add;`, bras `Commands::Add`

---

### Task 1: Les templates d'une feature ont une provenance

**Interfaces:**
- `Source::feature(repertoire: Option<&Path>, feature: &str) -> Result<Source, Inconnue>`
  — `repertoire` est celui de `--template-dir`, auquel le nom de la feature est joint.
- `Source::fichiers()` inchangée : elle rend les mêmes `Fichier { destination, source }`,
  triés, suffixe retiré.
- `Inconnue { feature: String }` — feature dont aucun répertoire n'existe.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    #[test]
    fn la_source_d_une_feature_restitue_ses_fichiers_embarques() {
        // docker -> [".dockerignore", "Dockerfile", "docker-compose.yml"], aucune vide
    }

    #[test]
    fn une_feature_inconnue_est_signalee_par_son_nom() {
        // Source::feature(None, "auth") -> Err, message contenant "auth"
    }

    #[test]
    fn un_repertoire_de_templates_prend_le_pas_pour_une_feature() {
        // --template-dir/docker/Dockerfile.jinja l'emporte sur l'embarqué
    }

    #[test]
    fn chaque_template_de_feature_porte_le_suffixe_jinja() { }

    #[test]
    fn chaque_template_de_feature_se_rend_avec_son_contexte() {
        // nom_projet, nom_crate — les deux seules variables des fragments
    }
```

Run: `cargo test -p rbs-cli --bins templates::` → Expected: FAIL.

- [ ] **Step 2: Écrire les trois templates de `docker`**

`Dockerfile` — deux étapes, `rust:1-slim` puis `debian:trixie-slim`. `sea-orm` tire
`runtime-tokio-rustls` : aucune dépendance à OpenSSL, donc rien à installer côté builder.
L'étape finale reçoit `ca-certificates`, un utilisateur non privilégié, `config/`, et les
**deux** binaires du workspace dans `/usr/local/bin` — le service one-shot a besoin de
`migration`. `CMD ["{@ nom_projet @}"]`, sans `ENTRYPOINT`, pour qu'un `command:` du
compose puisse lui substituer `migration up`.

`docker-compose.yml` — `db` (`postgres:18-alpine`, `pg_isready` en healthcheck, volume
nommé), `migrate` (même image, `command: ["migration", "up"]`, `restart: "no"`), `api`
(`RBS_SERVER__HOST=0.0.0.0`, port 8080 publié). Le chaînage est
`db: service_healthy` → `migrate: service_completed_successfully` → `api`.

`.dockerignore` — `target`, `.git`, `.github`, `.env` : le contexte de build ne porte ni
la cible de compilation, ni les secrets, dont les valeurs viennent du compose.

- [ ] **Step 3: Implémenter `Source::feature`, puis vérifier**

Run: `cargo test -p rbs-cli --bins templates::` → Expected: PASS.

- [ ] **Step 4: Commit** — `feat(cli): embarque les fragments de feature`

---

### Task 2: `rbs add docker` planifie et applique

**Files:**
- Create: `crates/rbs-cli/src/add/mod.rs`, `crates/rbs-cli/src/add/docker.rs`
- Modify: `crates/rbs-cli/src/metadata.rs`, `crates/rbs-cli/src/main.rs`

**Interfaces:**
- `pub(crate) fn planifier(options: &Options) -> Result<Planifiee, Erreur>` — `Options
  { feature, repertoire, force, template_dir }`, `Planifiee { plan, fichiers }`.
- `pub fn nom_du_paquet(cargo_toml: &Path) -> Result<String, Erreur>` dans `metadata` —
  `package.name`, dont les templates tirent le nom du binaire et celui de la base.
- Le contexte de rendu est `{ nom_projet, nom_crate }`, `nom_crate` étant `nom_projet`
  tirets remplacés par des soulignés, comme `new::nom_crate`.
- L'application passe par `plan::application::appliquer`, sans redite.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
    #[test]
    fn le_plan_de_docker_cree_ses_trois_fichiers_et_inscrit_la_feature() {
        // 4 fichiers touchés : les trois du catalogue + Cargo.toml
        // features = ["health", "docker"] dans le manifeste projeté
    }

    #[test]
    fn planifier_ne_modifie_pas_le_repertoire_du_projet() {
        // empreinte du répertoire identique avant et après
    }

    #[test]
    fn relancer_sur_un_projet_deja_dockerise_donne_un_plan_sans_effet() {
        // appliquer, replanifier : tous les fichiers en Statut::DejaFait
    }

    #[test]
    fn hors_d_un_projet_rbs_la_commande_refuse() { }

    #[test]
    fn un_working_tree_sale_refuse_sans_force_et_passe_avec() { }

    #[test]
    fn une_feature_inconnue_est_refusee_en_nommant_celles_qui_existent() {
        // `rbs add auth` -> message citant docker et ci
    }

    #[test]
    fn le_compose_projete_vise_postgres_18_et_ouvre_l_hote() {
        // le contenu projeté contient "postgres:18" et "RBS_SERVER__HOST"
        // -- le critère de D7 ne doit pas pouvoir régresser en silence
    }
```

Run: `cargo test -p rbs-cli --bins add::` → Expected: FAIL.

- [ ] **Step 2: Implémenter, puis vérifier**

Le bras `Commands::Add` de `main.rs` reprend celui de `generate` : planifier, afficher le
plan, appliquer, rendre compte. Le `nommer` de `main.rs` perd sa branche `Add`.

Run: `cargo test -p rbs-cli` → Expected: PASS.

- [ ] **Step 3: Commit** — `feat(cli): ajoute la commande add et la feature docker`

---

### Task 3: Le critère — `docker compose up`

- [ ] **Step 1: Générer un projet, l'équiper, le démarrer**

```bash
cd "$SCRATCH" && rbs new demo-docker --database-url postgres://postgres:postgres@localhost:5432/demo_docker --yes
cd demo-docker && git add -A && git commit -m "init"
rbs add docker
docker compose up -d --build
curl -fsS localhost:8080/health
docker compose down -v
```

Consigner la sortie réelle de `curl` et l'état des trois services. Un `/health` qui répond
prouve les deux moitiés du critère : l'API écoute, et sa base est joignable — le service
`migrate` a nécessairement abouti pour que `api` démarre.

- [ ] **Step 2: Vérifications finales**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 3: Cocher E7 dans `TODO.md`, avec sa preuve sur une ligne**

- [ ] **Step 4: Commit** — `docs: acte la commande add docker`

## Après le plan

E8 hérite du socle : `Source::feature` et `add::planifier` ne changent pas, `ci.rs`
n'apporte qu'un catalogue d'un fichier. E9 éprouvera les quatre scénarios sur les deux.
