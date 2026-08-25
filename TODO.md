# TODO — rbs

Tâches actionnables. **Seul le jalon en cours est détaillé** ; les suivants figurent en
grosses mailles et seront détaillés à leur ouverture, avec ce que le jalon précédent
aura appris.

Design de référence : [`docs/superpowers/specs/2026-08-25-rbs-design.md`](docs/superpowers/specs/2026-08-25-rbs-design.md)
Vision et jalons : [`ROADMAP.md`](ROADMAP.md)

Chaque tâche porte son critère de validation (`✓`). Une case ne se coche jamais sur une
impression.

---

## 🚧 v0.1 — Socle

Ordre imposé par les dépendances réelles : `A → B → C → D → E`, `F` démarre en parallèle
dès que `C` est terminé.

### Lot A — Fondations

- [x] **A1** · Workspace Cargo — vérifié 2026-08-25 · `cargo metadata --no-deps` → membres `rbs-core`, `rbs-cli`, aucun paquet racine · `cargo build --workspace` → Finished
      Conversion du paquet `rs` existant à la racine en workspace.
      Deux crates `crates/rbs-core` et `crates/rbs-cli`, dépendances partagées dans
      `[workspace.dependencies]`, édition 2024.
      ✓ `cargo build --workspace` passe sur un workspace vide de logique.
      ✓ Plus aucun `src/` ni `[package]` à la racine du dépôt.

- [x] **A2** · CI minimale — vérifié 2026-08-25 · PR #1 portant un warning clippy → check
      `fmt · clippy · test` en échec sur `cargo clippy` (code 101), `mergeStateStatus:
      BLOCKED`, `gh pr merge` refusé (« base branch policy prohibits the merge »)
      GitHub Actions : `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
      Linux uniquement à ce stade.
      ✓ Un PR avec un warning clippy est bloqué.

- [x] **A3** · Type `Error` et alias `Result` — vérifié 2026-08-25 · `cargo test -p rbs-core error` → 5 passed (les 3 conversions `From` + `Domain` + l'alias)
      Variantes `NotFound`, `Validation`, `Unauthorized`, `Forbidden`, `Conflict`,
      `Domain`, `Database`, `Internal`. Construit avec `thiserror`.
      ✓ Tests unitaires sur chaque conversion `From`.

- [x] **A4** · `IntoResponse` conforme RFC 9457 — vérifié 2026-08-25 · `cargo test -p rbs-core` → 16 passed, dont `validation_repond_422_avec_le_detail_des_champs` et `database_repond_500_generique_sans_le_message_de_la_source` lancés seuls → 1 passed chacun
      Réponse `application/problem+json` incluant le `request_id`.
      ✓ Test : `Validation` → 422 avec le détail des champs.
      ✓ Test : `Database` → 500 générique, **sans** le message de la source.

- [ ] **A5** · Chargement de configuration
      `figment` fusionnant défauts → `config/default.toml` → `config/{RBS_ENV}.toml` →
      `.env` → variables d'environnement, désérialisé dans une struct typée.
      ✓ Test : variable requise manquante → échec au boot, message nommant le champ.
      ✓ Test : une variable d'environnement écrase la valeur du fichier TOML.

- [ ] **A6** · Formateur de logs `pretty`
      `FormatEvent` maison : horodatage court, niveau coloré, cible, champs alignés.
      Le formateur par défaut de `tracing-subscriber` est trop verbeux.
      ✓ Inspection visuelle sur les cinq niveaux + capture dans les docs.
      ✓ Test : couleurs absentes quand la sortie n'est pas un TTY.

- [ ] **A7** · Formateur de logs `json` et bascule
      `RBS_LOG_FORMAT=pretty|json`, `RUST_LOG` respecté pour le filtrage.
      ✓ Test : chaque ligne de sortie est un JSON valide contenant `ts`, `level`, `msg`.

### Lot B — Noyau HTTP

- [ ] **B1** · Connexion base
      Initialisation du pool SeaORM depuis la configuration, avec timeouts explicites.
      ✓ Test : URL invalide → erreur au boot, message actionnable.

- [ ] **B2** · `AppState`
      Structure partagée portant le pool et la configuration, clonable à coût nul.
      ✓ Un handler d'exemple extrait `State<AppState>` et compile.

- [ ] **B3** · Middleware `request_id`
      ULID généré, ou repris de l'en-tête `x-request-id` entrant. Injecté dans le span
      `tracing`, renvoyé dans la réponse.
      ✓ Test : deux requêtes → deux identifiants distincts.
      ✓ Test : en-tête entrant fourni → conservé tel quel dans la réponse.

- [ ] **B4** · Middleware de trace
      Un span par requête : méthode, chemin, statut, latence. Le `request_id` est porté
      par tous les logs émis pendant la requête.
      ✓ Test : un log émis dans un handler contient le `request_id` de sa requête.

- [ ] **B5** · Extracteur JSON validé
      Wrapper autour de `Json` appliquant `validator`, produisant `Error::Validation`.
      ✓ Test : corps invalide → 422 avec le détail par champ.
      ✓ Test : JSON malformé → 400, pas 500.

- [ ] **B6** · Pagination
      Paramètres de requête `page` / `per_page` avec bornes, et enveloppe de réponse
      paginée.
      ✓ Test : `per_page` au-delà du maximum → plafonné, pas d'erreur.

- [ ] **B7** · Helpers OpenAPI
      Réponses d'erreur communes déclarées une fois, réutilisables par les features.
      ✓ Le document généré décrit 422 et 500 sans annotation par handler.

- [ ] **B8** · Route `/health`
      Statut applicatif et vérification de la base.
      ✓ Test : base indisponible → 503, pas 200.

- [ ] **B9** · Feature flags Cargo
      Déclaration des flags `auth`, `redis`, `mail`, `storage` — sans implémentation.
      Prépare la v0.2 sans anticiper son code.
      ✓ `cargo build --all-features` et `cargo build --no-default-features` passent.

### Lot C — `rbs new`

- [ ] **C1** · Squelette du CLI
      `clap` derive, sous-commandes, `--help` rédigé, sortie colorée via `console`.
      ✓ `rbs --help` liste les commandes prévues avec des descriptions utiles.

- [ ] **C2** · Moteur de rendu
      `minijinja` avec **délimiteurs alternatifs** — Jinja et `format!` Rust utilisent
      tous deux `{{ }}`.
      ✓ Test : une template contenant `format!("{{}}")` se rend sans échappement manuel.

- [ ] **C3** · Templates embarquées
      `include_dir` pour un binaire autonome, plus un flag `--template-dir` de surcharge.
      ✓ Le binaire génère un projet depuis un répertoire vide de tout template.

- [ ] **C4** · Squelette de projet
      `Cargo.toml`, `main.rs`, `router.rs`, `state.rs`, `features/mod.rs`, `features/health/`,
      `migration/`, `config/`, `.env.example`, `.gitignore`, avec toutes les ancres.
      ✓ Revue de lecture : `main.rs` tient en ~25 lignes compréhensibles sans documentation.

- [ ] **C5** · Métadonnées projet
      Écriture de `[package.metadata.rbs]` (version, features installées).
      ✓ Test : relire les métadonnées d'un projet fraîchement généré.

- [ ] **C6** · Prompts interactifs
      `inquire` : nom, base, multi-sélection des features. Chaque question a son flag
      équivalent ; `--yes` prend les défauts.
      ✓ Test : `rbs new x --yes` n'ouvre aucun prompt et réussit sans TTY.

- [ ] **C7** · Commande `rbs new` complète
      Assemblage de C2 → C6, plus `git init` sur le projet créé.
      ✓ Le projet généré démarre et répond 200 sur `/health`.

- [ ] **C8** · Test d'intégration du CLI
      `assert_cmd` + `tempfile` : `rbs new`, puis `cargo build` et `cargo test` du projet
      généré.
      ✓ Le test tourne en CI et échoue si le projet généré ne compile pas.

### Lot D — `rbs generate crud`

- [ ] **D1** · Parseur de champs
      Grammaire `nom:type[:modificateurs]` — types `string`, `int`, `float`, `bool`,
      `uuid`, `datetime`, `text` ; modificateurs `unique`, `optional`, `index`.
      ✓ Tests : chaque type et modificateur, plus les messages d'erreur de syntaxe.

- [ ] **D2** · Génération de l'entité SeaORM
      Clé primaire `id` de type `Uuid`, implicite — jamais déclarée dans `--fields`.
      ✓ L'entité compile et ses types correspondent aux champs demandés.
      ✓ `id` est un `Uuid` sans auto-incrément.

- [ ] **D3** · Génération des DTO
      `Create` / `Update` / `Response`, avec `validator` et `ToSchema`.
      ✓ Un champ `email:string` produit une contrainte de validation d'email.

- [ ] **D4** · Génération du repository
      CRUD complet et liste paginée. Ne connaît que `model.rs`.
      ✓ Revue : aucun import d'Axum dans le fichier.

- [ ] **D5** · Génération du service
      Logique métier, conversions DTO. Ne connaît que `repository.rs` et `dto.rs`.
      ✓ Revue : aucun import de `sea_orm::EntityTrait` dans le fichier.

- [ ] **D6** · Génération du controller
      Handlers Axum, annotations `#[utoipa::path]`, `routes()`. Ne connaît que `service.rs`.
      ✓ Les cinq routes apparaissent dans Swagger UI avec leurs schémas.

- [ ] **D7** · Génération de la migration
      Migration SeaORM correspondant aux champs, horodatée.
      ✓ `rbs migrate up` puis `down` laisse la base dans son état initial.
      ✓ La colonne `id` porte `DEFAULT uuidv7()` ; un `INSERT` sans `id` reçoit un
        UUIDv7 valide, dont l'horodatage de tête est celui de l'insertion.

- [ ] **D8** · Génération des tests
      Tests d'intégration HTTP du CRUD complet contre l'application montée en mémoire.
      ✓ Les tests générés passent immédiatement, sans retouche.

- [ ] **D9** · Insertion dans les ancres
      `<rbs:features>`, `<rbs:routes>`, `<rbs:openapi>`, `<rbs:migrations>`.
      ✓ Test : le contenu existant dans l'ancre n'est ni réordonné ni reformaté.

- [ ] **D10** · `rbs generate feature`
      Squelette à six fichiers, sans champ, pour une feature écrite à la main.
      ✓ Le projet compile après génération d'une feature vide.

- [ ] **D11** · `rbs migrate`
      `up`, `down`, `status`, `new` — enveloppe la crate `migration` du projet avec une
      sortie lisible. Le moteur de SeaORM n'est pas réimplémenté.
      ✓ `status` distingue visuellement appliqué / en attente.

- [ ] **D12** · `rbs doctor`
      Vérifie : ancres présentes, `.env` complet, base joignable, PostgreSQL ≥ 18,
      versions de rbs-core et du CLI cohérentes.
      ✓ Test : une ancre supprimée est signalée avec le bloc à recoller.

- [ ] **D13** · Test d'intégration CRUD
      Extension de C8 : génération d'un CRUD, migration, exécution des tests générés,
      contre PostgreSQL 18 via `testcontainers`.
      ✓ Rouge si l'une des trois étapes échoue.

### Lot E — `rbs add`

- [ ] **E1** · Modèle de plan
      Représentation en mémoire des actions : créer un fichier, insérer dans une ancre,
      patcher un TOML. Aucune écriture pendant la planification.
      ✓ Tests unitaires : construction d'un plan sans effet de bord sur le disque.

- [ ] **E2** · Moteur d'ancres
      Lecture, insertion avant la balise fermante, idempotence.
      ✓ Test : ancre absente → aucune écriture, code de sortie non nul, bloc affiché.
      ✓ Test : insertion déjà présente → aucune modification.

- [ ] **E3** · Patch de `Cargo.toml`
      `toml_edit` : ajout de dépendance, ajout d'une feature à une dépendance existante,
      mise à jour de `metadata.rbs`.
      ✓ Test : commentaires et formatage du fichier préservés à l'octet près hors zone modifiée.

- [ ] **E4** · Vérification du working tree
      Working tree Git sale → avertissement, contournable par `--force`.
      ✓ Test : dépôt sale → refus ; avec `--force` → exécution.

- [ ] **E5** · Affichage du plan et `--dry-run`
      Le plan complet est montré avant toute écriture, fichier par fichier.
      ✓ `--dry-run` ne modifie rien et affiche le même plan que l'exécution réelle.

- [ ] **E6** · Application atomique
      Échec en cours d'application → restauration des fichiers déjà écrits.
      ✓ Test : échec injecté sur la quatrième action → les trois premières sont annulées.

- [ ] **E7** · `rbs add docker`
      `Dockerfile` multi-étapes, `docker-compose.yml` avec PostgreSQL 18, `.dockerignore`.
      ✓ `docker compose up` démarre l'API et sa base.

- [ ] **E8** · `rbs add ci`
      Workflow GitHub Actions pour le projet généré : fmt, clippy, test.
      ✓ Le workflow généré passe sur un projet fraîchement créé.

- [ ] **E9** · Tests du mécanisme `add`
      Idempotence, ancre manquante, dépôt sale, rollback.
      ✓ Les quatre scénarios sont couverts en CI.

> `docker` et `ci` ne touchent pas au code Rust. C'est délibéré : la mécanique d'ancres
> est éprouvée en conditions réelles sur des cas où une erreur ne casse pas la
> compilation, avant que l'auth n'en dépende en v0.2.

### Lot F — Documentation et publication

Démarre dès que le lot C est terminé, en parallèle de D et E. Documenter pendant la
construction, pas après, quand tout paraît évident.

- [ ] **F1** · Docusaurus + i18n
      Initialisation dans `docs/`, locales `fr` et `en`, sélecteur de langue.
      ✓ Le site se construit et bascule entre les deux langues.

- [ ] **F2** · Extraits de code depuis `examples/`
      Les extraits de la documentation sont tirés de projets compilés en CI. Docusaurus
      n'exécute pas le code : c'est la compensation.
      ✓ Un exemple cassé fait échouer la CI.

- [ ] **F3** · Démarrage rapide (FR + EN)
      De l'installation à une API CRUD qui répond.
      ✓ Suivi à la lettre sur une machine vierge, sans intervention extérieure.

- [ ] **F4** · Architecture (FR + EN)
      Frontière noyau/généré, anatomie d'une feature, règle de dépendance.

- [ ] **F5** · Référence du CLI (FR + EN)
      Chaque commande, chaque flag, avec un exemple de sortie réelle.

- [ ] **F6** · Guides transverses (FR + EN)
      Configuration, logs, erreurs, OpenAPI, migrations, tests.

- [ ] **F7** · README FR + EN
      ✓ Mentionne explicitement l'absence de promesse semver avant la v1.0.

- [ ] **F8** · LICENSE
      Double licence `MIT OR Apache-2.0` : `LICENSE-MIT`, `LICENSE-APACHE`, et le champ
      `license` renseigné dans les deux crates.
      ✓ `cargo publish --dry-run` ne signale aucun problème de licence.

- [ ] **F9** · CONTRIBUTING et code de conduite
      ✓ CONTRIBUTING indique comment contribuer au code **sans installer Node**.

- [ ] **F10** · CI complète
      Linux, macOS, Windows. Tests d'intégration du CLI inclus.
      ✓ Les trois plateformes passent au vert.

- [ ] **F11** · Modèles d'issues et de PR

- [ ] **F12** · Publication du site
      GitHub Pages, déploiement automatique.

- [ ] **F13** · Ouverture du dépôt
      ✓ Installation possible par `cargo install --git`.

### Validation du jalon

- [ ] **V1** · Test du critère de sortie
      Une personne extérieure au projet clone, installe, génère une API CRUD qui tourne,
      **sans poser de question**. Chaque question posée devient une tâche de
      documentation avant que la v0.1 ne soit déclarée close.

- [ ] **V2** · Revue de parité FR/EN
      Toute page présente dans une langue existe et est à jour dans l'autre.

- [ ] **V3** · Passe sur les conventions de code
      Suppression des commentaires qui paraphrasent le code ; `missing_docs` sans
      avertissement sur `rbs-core` ; aucun fichier de feature générée au-delà de ~200 lignes.

---

## ⏳ Jalons suivants

Volontairement en grosses mailles. Détailler ces tâches aujourd'hui serait de la
fiction : elles seront réécrites avec ce que la v0.1 aura appris.

### v0.2 — Auth
- [ ] Primitives dans `rbs-core` derrière le flag `auth` : Argon2, JWT, extracteur d'identité
- [ ] `rbs add auth` : DTO, service, controller, migration `users`, guards de rôles
- [ ] Refresh tokens et révocation
- [ ] Documentation FR/EN et exemple compilé

### v0.3 — Intégrations
- [ ] `rbs add redis` — pool, cache, sessions
- [ ] `rbs add mail` — SMTP, templates, envoi asynchrone
- [ ] `rbs add storage` — système de fichiers et S3
- [ ] Vérification : aucune de ces trois features n'a nécessité de modifier `rbs-core`

### v0.4 — Confort
- [ ] Seeds et données de démonstration
- [ ] `rbs dev` — rechargement à chaud
- [ ] Jobs en arrière-plan
- [ ] Support MySQL et SQLite

### v1.0 — Stabilité
- [ ] Gel de l'API publique de `rbs-core`
- [ ] Publication sur crates.io
- [ ] CHANGELOG et engagement semver
- [ ] `rbs upgrade` — migration d'un projet d'une version de rbs à la suivante
