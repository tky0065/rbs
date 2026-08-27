# TODO — rbs

Tâches actionnables. La **v0.1** et la **v0.2** sont détaillées ; les jalons plus
lointains figurent en grosses mailles et seront détaillés à leur tour, avec ce que les
précédents auront appris. Détailler un jalon ne l'ouvre pas : l'ordre des lots reste
contraignant.

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

- [x] **A5** · Chargement de configuration — vérifié 2026-08-25 · `cargo test -p rbs-core config` → 6 passed, dont `un_champ_requis_manquant_fait_echouer_le_chargement_en_nommant_le_champ` et `une_variable_d_environnement_ecrase_la_valeur_du_fichier_toml` lancés seuls → 1 passed chacun
      `figment` fusionnant défauts → `config/default.toml` → `config/{RBS_ENV}.toml` →
      `.env` → variables d'environnement, désérialisé dans une struct typée.
      ✓ Test : variable requise manquante → échec au boot, message nommant le champ.
      ✓ Test : une variable d'environnement écrase la valeur du fichier TOML.

- [x] **A6** · Formateur de logs `pretty` — vérifié 2026-08-26 · `cargo test -p rbs-core logs` → 10 passed (dont couleurs absentes hors TTY) · capture régénérée depuis la sortie réelle sous pty par `docs/scripts/capture_logs_pretty.py`, publiée dans la page « Logs » FR + EN, cinq niveaux validés de visu par le user · `npm run build` à froid → deux `[SUCCESS]`, image 1664×270 et régions résolues dans les deux locales · garde-fou de F2 éprouvé sur cette page : région retirée → build sort **1** en la nommant, mais seulement après purge de `node_modules/.cache`, qu'un build incrémental masque.
      `FormatEvent` maison : horodatage court, niveau coloré, cible, champs alignés.
      Le formateur par défaut de `tracing-subscriber` est trop verbeux.
      ✓ Inspection visuelle sur les cinq niveaux + capture dans les docs.
      ✓ Test : couleurs absentes quand la sortie n'est pas un TTY.

- [x] **A7** · Formateur de logs `json` et bascule — vérifié 2026-08-25 · `cargo test -p rbs-core logs` → 10 passed, dont `chaque_ligne_est_un_json_valide_portant_ts_level_et_msg` lancé seul → 1 passed · `RBS_LOG_FORMAT=json cargo run -p rbs-core --example logs_format` → 3 objets JSON, `RUST_LOG=warn` filtre bien l'événement `info`
      `RBS_LOG_FORMAT=pretty|json`, `RUST_LOG` respecté pour le filtrage.
      ✓ Test : chaque ligne de sortie est un JSON valide contenant `ts`, `level`, `msg`.

### Lot B — Noyau HTTP

- [x] **B1** · Connexion base — vérifié 2026-08-25 · `cargo test -p rbs-core db` → 6 passed, dont `une_url_invalide_echoue_avec_un_message_nommant_le_champ` lancé seul → 1 passed
      Initialisation du pool SeaORM depuis la configuration, avec timeouts explicites.
      ✓ Test : URL invalide → erreur au boot, message actionnable.

- [x] **B2** · `AppState` — vérifié 2026-08-25 · `cargo test -p rbs-core state` → 3 passed, dont `un_handler_extrait_l_etat_du_projet_et_repond` (routeur monté, requête réelle)
      Structure partagée portant le pool et la configuration, clonable à coût nul.
      ✓ Un handler d'exemple extrait `State<AppState>` et compile.

- [x] **B3** · Middleware `request_id` — vérifié 2026-08-25 · `cargo test -p rbs-core request_id` → 8 passed, dont `deux_requetes_recoivent_deux_identifiants_distincts` et `un_en_tete_entrant_est_conserve_tel_quel_dans_la_reponse` lancés seuls → 1 passed chacun
      ULID généré, ou repris de l'en-tête `x-request-id` entrant. Injecté dans le span
      `tracing`, renvoyé dans la réponse.
      ✓ Test : deux requêtes → deux identifiants distincts.
      ✓ Test : en-tête entrant fourni → conservé tel quel dans la réponse.

- [x] **B4** · Middleware de trace — vérifié 2026-08-25 · `cargo test -p rbs-core trace` → 3 passed, dont `un_log_emis_dans_un_handler_porte_le_request_id_de_sa_requete` lancé seul → 1 passed
      Un span par requête : méthode, chemin, statut, latence. Le `request_id` est porté
      par tous les logs émis pendant la requête.
      ✓ Test : un log émis dans un handler contient le `request_id` de sa requête.

- [x] **B5** · Extracteur JSON validé — vérifié 2026-08-25 · `cargo test -p rbs-core extract` → 4 passed, dont `un_corps_invalide_repond_422_avec_le_detail_par_champ` et `un_json_malforme_repond_400_pas_500` lancés seuls → 1 passed chacun
      Wrapper autour de `Json` appliquant `validator`, produisant `Error::Validation`.
      ✓ Test : corps invalide → 422 avec le détail par champ.
      ✓ Test : JSON malformé → 400, pas 500.

- [x] **B6** · Pagination — vérifié 2026-08-25 · `cargo test -p rbs-core pagination` → 7 passed, dont `per_page_au_dela_du_maximum_est_plafonne_sans_erreur` lancé seul → 1 passed
      Paramètres de requête `page` / `per_page` avec bornes, et enveloppe de réponse
      paginée.
      ✓ Test : `per_page` au-delà du maximum → plafonné, pas d'erreur.

- [x] **B7** · Helpers OpenAPI — vérifié 2026-08-25 · `cargo test -p rbs-core openapi` → 4 passed, dont `le_document_decrit_422_et_500_sans_annotation_par_handler` lancé seul → 1 passed
      Réponses d'erreur communes déclarées une fois, réutilisables par les features.
      ✓ Le document généré décrit 422 et 500 sans annotation par handler.

- [x] **B8** · Route `/health` — vérifié 2026-08-25 · `cargo test -p rbs-core health` → 4 passed, dont `une_base_indisponible_repond_503_pas_200` lancé seul → 1 passed
      Statut applicatif et vérification de la base.
      ✓ Test : base indisponible → 503, pas 200.

- [x] **B9** · Feature flags Cargo — vérifié 2026-08-25 · `cargo build --all-features` et `cargo build --no-default-features` → Finished, et `--features inexistant` rejeté (preuve que les flags sont déclarés)
      Déclaration des flags `auth`, `redis`, `mail`, `storage` — sans implémentation.
      Prépare la v0.2 sans anticiper son code.
      ✓ `cargo build --all-features` et `cargo build --no-default-features` passent.

- [x] **B10** · Exposition OpenAPI configurable — vérifié 2026-08-26 · `cargo test -p rbs-core config` → 12 passed, dont `couper_swagger_laisse_le_document_json_expose` et `sans_section_docs_swagger_et_le_document_json_sont_exposes` lancés seuls → 1 passed chacun
      Section `[docs]` dans `Config` : `swagger_ui` et `openapi_json`, à `true` par défaut.
      §5.4 les veut désactivables en production ; aucun champ ne le permettait.
      ✓ Test : `docs.swagger_ui = false` dans le TOML est désérialisé à `false`.
      ✓ Test : sans section `[docs]`, les deux valent `true`.

### Lot C — `rbs new`

- [x] **C1** · Squelette du CLI — vérifié 2026-08-26 · `cargo test -p rbs-cli` → 4 passed, dont `le_help_liste_les_commandes_prevues_avec_une_description` · `cargo run -p rbs-cli -- --help` → les cinq commandes avec leur description, validé par le user
      `clap` derive, sous-commandes, `--help` rédigé, sortie colorée via `console`.
      ✓ `rbs --help` liste les commandes prévues avec des descriptions utiles.

- [x] **C2** · Moteur de rendu — vérifié 2026-08-26 · `cargo test -p rbs-cli` → 9 passed, dont `une_template_contenant_un_format_rust_se_rend_intacte` lancé seul → 1 passed · délimiteurs `{@ @}` retenus, seuls à ne collisionner ni avec Rust, ni TOML, YAML, shell ou GitHub Actions
      `minijinja` avec **délimiteurs alternatifs** — Jinja et `format!` Rust utilisent
      tous deux `{{ }}`.
      ✓ Test : une template contenant `format!("{{}}")` se rend sans échappement manuel.

- [x] **C3** · Templates embarquées — vérifié 2026-08-26 · binaire copié hors du dépôt et `templates/` écartée du disque → `rbs new sans-templates --yes` → 15 fichiers, code 0 · `cargo test -p rbs-cli templates::` → 8 passed
      `include_dir` pour un binaire autonome, plus un flag `--template-dir` de surcharge.
      ✓ Le binaire génère un projet depuis un répertoire vide de tout template.

- [x] **C4** · Squelette de projet — vérifié 2026-08-26 · `cargo test -p rbs-cli` → 13 passed, dont `chaque_ancre_est_ouverte_puis_refermee_dans_son_fichier` et `chaque_template_se_rend_avec_les_cinq_variables` · revue de lecture du `main.rs` généré (25 lignes) validée par le user
      `Cargo.toml`, `main.rs`, `router.rs`, `state.rs`, `features/mod.rs`, `features/health/`,
      `migration/`, `config/`, `.env.example`, `.gitignore`, avec toutes les ancres.
      ✓ Revue de lecture : `main.rs` tient en ~25 lignes compréhensibles sans documentation.

- [x] **C5** · Métadonnées projet — vérifié 2026-08-26 · `cargo test -p rbs-cli -- --exact new::tests::les_metadonnees_du_projet_cree_se_relisent` → 1 passed : version et features relues sur un projet déroulé par `rbs new`, la version venant du CLI et non plus d'une constante figée dans la template
      Écriture de `[package.metadata.rbs]` (version, features installées).
      ✓ Test : relire les métadonnées d'un projet fraîchement généré.

- [x] **C6** · Prompts interactifs — vérifié 2026-08-26 · `rbs new sans-templates --yes < /dev/null` sans TTY → projet créé, code 0, aucun prompt ouvert · `cargo test -p rbs-cli prompts::` → 8 passed
      `inquire` : nom, base, multi-sélection des features. Chaque question a son flag
      équivalent ; `--yes` prend les défauts.
      ✓ Test : `rbs new x --yes` n'ouvre aucun prompt et réussit sans TTY.

- [x] **C7** · Commande `rbs new` complète — vérifié 2026-08-26 · projet généré puis lancé contre PostgreSQL 18.4 en conteneur : `curl -i /health` → `200 OK`, `{"status":"ok","checks":{"database":"ok"}}` · `cargo test -p rbs-cli new::` → 13 passed
      Assemblage de C2 → C6, plus `git init` sur le projet créé.
      ✓ Le projet généré démarre et répond 200 sur `/health`.

- [x] **C8** · Test d'intégration du CLI — vérifié 2026-08-26 · PR #5, étape « cargo test (intégration) » → 1 passed en 76,56 s ; template cassée → FAILED sur `cargo build` (E0308, code 101)
      `assert_cmd` + `tempfile` : `rbs new`, puis `cargo build` et `cargo test` du projet
      généré.
      ✓ Le test tourne en CI et échoue si le projet généré ne compile pas.

### Lot D — `rbs generate crud`

- [x] **D1** · Parseur de champs — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::champs` → 49 passed
      Grammaire `nom:type[:modificateurs]` — types `string`, `int`, `float`, `bool`,
      `uuid`, `datetime`, `text` ; modificateurs `unique`, `optional`, `index`.
      ✓ Tests : chaque type et modificateur, plus les messages d'erreur de syntaxe.

- [x] **D2** · Génération de l'entité SeaORM — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::entite -- --include-ignored` → 13 passed, dont la compilation d'un projet neuf portant l'entité
      Clé primaire `id` de type `Uuid`, implicite — jamais déclarée dans `--fields`.
      ✓ L'entité compile et ses types correspondent aux champs demandés.
      ✓ `id` est un `Uuid` sans auto-incrément.

- [x] **D3** · Génération des DTO — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::dto -- --include-ignored` → 13 passed, dont la compilation des trois DTO dans un projet neuf
      `Create` / `Update` / `Response`, avec `validator` et `ToSchema`.
      ✓ Un champ `email:string` produit une contrainte de validation d'email.

- [x] **D4** · Génération du repository — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::repository -- --include-ignored` → 11 passed, dont la compilation du repository dans un projet neuf et sa stabilité sous rustfmt
      CRUD complet et liste paginée. Ne connaît que `model.rs`.
      ✓ Revue : aucun import d'Axum dans le fichier.

- [x] **D5** · Génération du service — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::service -- --include-ignored` → 12 passed, dont la compilation de model + dto + repository + service dans un projet neuf
      Logique métier, conversions DTO. Ne connaît que `repository.rs` et `dto.rs`.
      ✓ Revue : aucun import de `sea_orm::EntityTrait` dans le fichier.

- [x] **D6** · Génération du controller — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::controller -- --include-ignored` → 12 passed, dont les cinq chemins et leurs schémas dans le document OpenAPI du projet compilé ; rendu de Swagger UI validé de visu par le porteur du projet sur `target/atelier`
      Handlers Axum, annotations `#[utoipa::path]`, `routes()`. Ne connaît que `service.rs`.
      ✓ Les cinq routes apparaissent dans Swagger UI avec leurs schémas.

- [x] **D7** · Génération de la migration — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::migration -- --include-ignored` → 14 passed, dont montée/insertion/descente contre PostgreSQL 18 en conteneur. Le ✓ de réversibilité est prouvé par `Migrator::up`/`down` du projet généré, `rbs migrate` (D11) n'existant pas encore — substitution validée par le porteur du projet
      Migration SeaORM correspondant aux champs, horodatée.
      ✓ `rbs migrate up` puis `down` laisse la base dans son état initial.
      ✓ La colonne `id` porte `DEFAULT uuidv7()` ; un `INSERT` sans `id` reçoit un
        UUIDv7 valide, dont l'horodatage de tête est celui de l'insertion.

- [x] **D7b** · Aplatissement des features — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::nom` → 5 passed · `cargo test -p rbs-cli -- --exact generate::commande::tests::la_commande_refuse_un_nom_en_conflit_en_le_nommant` → 1 passed, et à la main `rbs g crud state` → « ✗ « state » est un module du squelette du projet », `rbs g crud match` → « ✗ « match » est un mot-clé Rust », code 1 dans les deux cas · `cargo test --workspace -- --include-ignored` → 231 + 1 + 72 passed, 0 ignored
      Une feature vit en `src/<nom>/`, non plus en `src/features/<nom>/` : l'ancre
      `<rbs:features>` descend dans `src/main.rs` et les insertions passent au chemin
      absolu. En contrepartie, le nom d'une feature est validé — il entre désormais en
      concurrence avec les modules du squelette.
      ✓ `cargo test --workspace` vert, tests de générateurs compris.
      ✓ Un projet créé par `rbs new` puis portant une feature compile, `src/` sans
        `features/`.
      ✓ `rbs g crud state` et `rbs g crud match` échouent en nommant le conflit.

- [x] **D8** · Génération des tests — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::essais` → 13 passed · `cargo test -p rbs-cli -- --exact generate::essais::tests::les_tests_generes_passent_sans_retouche --include-ignored` → 1 passed, dont « 3 passed » dans le projet généré · `cargo test --workspace -- --include-ignored` → 202 + 1 + 72 passed, 0 ignored
      Tests d'intégration HTTP du CRUD complet contre l'application montée en mémoire.
      ✓ Les tests générés passent immédiatement, sans retouche.

- [x] **D9** · Insertion dans les ancres — vérifié 2026-08-26 · `cargo test -p rbs-cli ancres` → 10 passed, dont `le_contenu_existant_n_est_ni_reordonne_ni_reformate` · `cargo test -p rbs-cli generate::montage` → 6 passed · `cargo test --workspace -- --include-ignored` → 218 + 1 + 72 passed, 0 ignored : les tests lourds passent désormais par le moteur et leurs projets compilent
      `<rbs:features>`, `<rbs:routes>`, `<rbs:openapi>`, `<rbs:migrations>`, plus
      `<rbs:migration_modules>` : Rust interdit un `mod` non-inline dans un bloc, la
      déclaration du fichier de migration ne peut donc pas tenir dans le `vec!`.
      ✓ Test : le contenu existant dans l'ancre n'est ni réordonné ni reformaté.

- [x] **D10** · `rbs generate feature` — vérifié 2026-08-26 · `cargo test -p rbs-cli generate::commande` → 12 passed · `cargo test -p rbs-cli -- --exact generate::commande::tests::le_projet_compile_apres_generation_d_une_feature_vide --include-ignored` → 1 passed : `rbs g feature notes` puis `rbs g crud carnets` sur un projet neuf, `cargo build --workspace` vert · `cargo test --workspace -- --include-ignored` → 231 + 1 + 72 passed, 0 ignored
      Squelette à six fichiers, sans champ, pour une feature écrite à la main. `crud` est
      câblée avec elle : sept fichiers, la migration et les cinq ancres. `--force` reste
      sans effet et le signale — la vérification du working tree est E4.
      ✓ Le projet compile après génération d'une feature vide.

- [x] **D11** · `rbs migrate` — vérifié 2026-08-26 · `cargo test -p rbs-cli migrate` → 22 passed · `rbs migrate status` sur un projet neuf contre PostgreSQL 18 : « ✓ … appliquée » / « · … en attente », validé de visu · `cargo test --workspace -- --include-ignored` → 262 + 1 + 72 passed, 0 ignored
      `up`, `down`, `status`, `new` — enveloppe la crate `migration` du projet, qui gagne
      un binaire : elle n'était qu'une lib. Le moteur de SeaORM n'est pas réimplémenté ;
      `new` ne délègue rien.
      ✓ `status` distingue visuellement appliqué / en attente.

- [x] **D12** · `rbs doctor` — vérifié 2026-08-26 · `cargo test -p rbs-cli doctor` → 33 passed, dont `une_ancre_supprimee_est_signalee_avec_le_bloc_a_recoller` · bout en bout : ancre retirée de `router.rs` et variable retirée du `.env` → blocs à recoller affichés, code de sortie 1 ; projet sain → « PostgreSQL 18.6 répond », code 0 · `cargo test --workspace -- --include-ignored` → 295 + 1 + 72 passed, 0 ignored
      Vérifie : ancres présentes, `.env` complet, base joignable, PostgreSQL ≥ 18,
      versions de rbs-core et du CLI cohérentes. Cinq ancres depuis D9, et non quatre :
      la liste fait foi dans `ancres::ANCRES`. La version du serveur vient du binaire de
      la crate `migration`, qui gagne une commande `version` : rbs ne parle pas SQL.
      ✓ Test : une ancre supprimée est signalée avec le bloc à recoller.

- [x] **D13** · Test d'intégration CRUD — vérifié 2026-08-26 · `cargo test -p rbs-cli --test integration_crud -- --ignored` → 1 passed · rouge prouvé sur deux étapes : `--fields "titre:type_inexistant"` → échec à la génération, `migrate up` retirée → `articles::tests::le_cycle_de_vie_complet_passe_par_l_api` échoue dans le projet généré · `cargo test --workspace -- --include-ignored` → 295 + 1 + 1 + 72 passed, 0 ignored
      Extension de C8 : génération d'un CRUD, migration, exécution des tests générés,
      contre PostgreSQL 18 via `testcontainers`. L'attente porte sur la **seconde**
      annonce de disponibilité : la première a lieu pendant l'initialisation, où le
      serveur n'écoute que sur son socket local.
      ✓ Rouge si l'une des trois étapes échoue.

### Lot E — `rbs add`

- [x] **E1** · Modèle de plan — vérifié 2026-08-26 · `cargo test -p rbs-cli --bins plan::` → 20 passed (dont `planifier_ne_modifie_pas_le_repertoire_du_projet`, empreinte du répertoire inchangée)

- [x] **E2** · Moteur d'ancres — vérifié 2026-08-26 · `cargo test -p rbs-cli plan::` → 25 passed (ancre absente : répertoire intact et `fichiers()` vide ; réinsertion : statut `DejaFait`) · bout en bout : `<rbs:routes>` retirée → bloc à recoller affiché, code de sortie 1, rien d'écrit
      Lecture, insertion avant la balise fermante, idempotence.
      ✓ Test : ancre absente → aucune écriture, code de sortie non nul, bloc affiché.
      ✓ Test : insertion déjà présente → aucune modification.

- [x] **E3** · Patch de `Cargo.toml` — vérifié 2026-08-26 · `cargo test -p rbs-cli ne_modifie_que` → 3 passed (manifeste témoin comparé ligne à ligne après chacun des trois patchs), `metadata::` → 21 passed
      `toml_edit` : ajout de dépendance, ajout d'une feature à une dépendance existante,
      mise à jour de `metadata.rbs`.
      ✓ Test : commentaires et formatage du fichier préservés à l'octet près hors zone modifiée.

- [x] **E4** · Vérification du working tree — vérifié 2026-08-26 · `cargo test -p rbs-cli projet_sale` → 2 passed, `git::` → 5 passed · bout en bout : projet sale → refus code 1, aucun `src/notes` ; avec `--force` → 6 fichiers générés
      Working tree Git sale → avertissement, contournable par `--force`. Vaut aussi pour
      `rbs generate`, dont le `--force` est déclaré depuis D10 mais sans effet.
      ✓ Test : dépôt sale → refus ; avec `--force` → exécution.

- [x] **E5** · Affichage du plan et `--dry-run` — vérifié 2026-08-26 · `cargo test -p rbs-cli --bins rendu::tests` → 18 passed, `le_plan_affiche` → 1 passed (mutation de l'application → FAILED, le test mord) · bout en bout : `generate crud --dry-run` puis sans → plans identiques au diff, rien d'écrit par le premier
      Le plan complet est montré avant toute écriture, fichier par fichier.
      ✓ `--dry-run` ne modifie rien et affiche le même plan que l'exécution réelle.

- [x] **E6** · Application atomique — vérifié 2026-08-26 · `cargo test -p rbs-cli --bins application::` → 6 passed, dont l'échec injecté sur la quatrième action : empreinte du répertoire identique à l'origine · `generate` migrée vers le plan, `allow(dead_code)` de `plan/mod.rs` retiré
      Échec en cours d'application → restauration des fichiers déjà écrits.
      ✓ Test : échec injecté sur la quatrième action → les trois premières sont annulées.

- [x] **E7** · `rbs add docker` — vérifié 2026-08-26 · `cargo test -p rbs-cli --bins templates::` → 17 passed, `add::` → 7 passed · bout en bout : `docker compose up` sur un projet neuf → db healthy, migrate exited(0), api Up ; `curl localhost:8080/health` → 200 `{"status":"ok","checks":{"database":"ok"}}`

- [x] **E8** · `rbs add ci` — vérifié 2026-08-26 · `cargo test -p rbs-cli --bins templates::` → 20 passed · workflow généré rejoué en entier sous `act` sur un projet neuf (`rbs new` + `add ci` + `generate crud articles`) : `🏁 Job succeeded`, les quatre étapes vertes — `cargo fmt`, `cargo clippy`, `migrations`, `cargo test` → **3 passed / 0 failed**, dont `le_cycle_de_vie_complet_passe_par_l_api`, qui ne réussit que si la migration a créé la table contre le service PostgreSQL du workflow · le noyau non publié est monté dans le conteneur à son chemin absolu d'hôte, le projet testé est donc celui que `rbs new` a écrit, sans retouche · deux pièges relevés : forcer `linux/amd64` sur une image arm64 rend `node` introuvable et fait échouer toute action JavaScript, et sans CRUD généré `cargo test` passe à vide (0 test) — le workflow « passait » sans rien éprouver
      Workflow GitHub Actions pour le projet généré : fmt, clippy, test.
      ✓ Le workflow généré passe sur un projet fraîchement créé.

- [x] **E9** · Tests du mécanisme `add` — vérifié 2026-08-26 · `cargo test -p rbs-cli --test integration_add` → 4 passed, chacun mis au rouge par une mutation du code de production (déduplication des features, ancre absente ignorée, garde du working tree, rollback retiré) · l'ancre absente est éprouvée sur `generate crud`, `add` n'écrivant dans aucune ancre
      Idempotence, ancre manquante, dépôt sale, rollback.
      ✓ Les quatre scénarios sont couverts en CI.

> `docker` et `ci` ne touchent pas au code Rust. C'est délibéré : la mécanique d'ancres
> est éprouvée en conditions réelles sur des cas où une erreur ne casse pas la
> compilation, avant que l'auth n'en dépende en v0.2.

### Lot F — Documentation et publication

Démarre dès que le lot C est terminé, en parallèle de D et E. Documenter pendant la
construction, pas après, quand tout paraît évident.

- [x] **F1** · Docusaurus + i18n — vérifié 2026-08-26 · `npm run build` → `en` et `fr`, deux `[SUCCESS]` · `<html lang=en>` contre `<html lang=fr>`, dropdown `href=/rbs/fr/ …>Français` et retour `href=/rbs/ …>English` · `grep -rl superpowers build/ | wc -l` → 0 · `grep -c 'npm\|node\|yarn' ci.yml` → 0, la preuve de F9 tient · bascule validée de visu par le user sur `npm run serve`
      Initialisation dans `docs/`, locales `fr` et `en`, sélecteur de langue.
      ✓ Le site se construit et bascule entre les deux langues.

- [x] **F2** · Extraits de code depuis `examples/` — vérifié 2026-08-26 · les trois garde-fous éprouvés en cassant l'exemple pour de bon : code illégal → `cargo clippy` de l'étape CI sort **101** (`error[E0308]`) ; dérive d'une template → `integration_examples` sort **101** en nommant la ligne ; région ou fichier cité disparu → `npm run build` sort **1** avec le message du plugin · réparé à chaque fois, retour au vert constaté · 16 tests du plugin, 5 de non-dérive, 454 du dépôt, site construit à froid sur 2 locales, extrait rendu en FR et EN · `grep -c 'npm\|node\|yarn' ci.yml` → 0 · le run GitHub lui-même n'est pas rejoué (github.com injoignable ici, cf. E8) : c'est le périmètre de F10
      Les extraits de la documentation sont tirés de projets compilés en CI. Docusaurus
      n'exécute pas le code : c'est la compensation.
      ✓ Un exemple cassé fait échouer la CI.

- [x] **F3** · Démarrage rapide (FR + EN) — vérifié 2026-08-26 · parcours rejoué **deux fois** à la lettre dans des répertoires vierges, depuis un `git clone https://github.com/tky0065/rbs` public (`01b4bb5`) sur un PostgreSQL 18 neuf : `cargo install --path` → `rbs 0.1.0`, `rbs new` 15 fichiers, `migrate up`, `generate crud articles` 8 fichiers, `migrate up` + `migrate status` → migration appliquée, `cargo run`, `/health` → 200 `{"status":"ok","checks":{"database":"ok"}}`, `POST /articles` → 201, `GET /articles` → 200 paginé (`total_pages: 1`), `openapi.json` → `/articles`, `/articles/{id}`, `/health`, `doctor` → quatre contrôles verts, sortie identique ligne pour ligne à celle que la page annonce · deux défauts trouvés en suivant et corrigés dans les deux langues : l'encart renvoyant vers un « arbre de développement de la 0.1 » inexistant (F13 prouve que `main` porte les cinq commandes), et le répertoire de travail jamais explicité — `--core-path ../rbs/…` échouait depuis le parent du clone, remplacé par un `cd ..` et un chemin `rbs/crates/rbs-core` · `npm run clear && npm run build` → deux `[SUCCESS]` · une réserve : le CLI a été installé dans un `--root` isolé pour ne pas écraser le `~/.cargo/bin/rbs` de la machine, et la machine n'était pas vierge de Rust ni de Docker, que la page liste en prérequis
      De l'installation à une API CRUD qui répond.
      ✓ Suivi à la lettre sur une machine vierge, sans intervention extérieure.

- [x] **F4** · Architecture (FR + EN) — vérifié 2026-08-26 · `npx docusaurus clear && npm run build` → deux `[SUCCESS]`, garde-fou éprouvé en renommant la région `entite` → build interrompu en nommant le fichier · `cargo test -p rbs-cli --test integration_examples` → 5 passed · 9 directives d'extrait et 13 titres identiques FR/EN · règle de dépendance établie sur les imports réels : `grep -l 'Entity::'` ne remonte que `repository.rs`
      Frontière noyau/généré, anatomie d'une feature, règle de dépendance.

- [x] **F5** · Référence du CLI (FR + EN) — vérifié 2026-08-26 · 10 flags relevés sur la surface rendue par clap = 10 documentés en FR et en EN, aucun manquant · les 5 commandes, leurs sous-commandes et les cas d'échec cités capturés en exécution réelle contre un PostgreSQL 18 en conteneur · `npm run build` → deux `[SUCCESS]` · trois écarts au comportement déclaré consignés dans les pages : `--with docker` refusé par la 0.1.0, `--template-dir` ignoré par `generate`, `--yes` lu par `new` seul
      Chaque commande, chaque flag, avec un exemple de sortie réelle.

- [x] **F6** · Guides transverses (FR + EN) — vérifié 2026-08-26 · six guides livrés FR + EN · `npm run clear && npm run build` → deux `[SUCCESS]`, garde-fou éprouvé en renommant la région `montage` → build interrompu · `cargo test -p rbs-cli --test integration_examples` → 5 passed · deux erreurs de la conception corrigées sur le code : `Error` compte 9 variantes (`Domain` incluse) et l'ancre `<rbs:openapi>` vit dans `paths(...)`, elle liste des handlers et non des schémas
      Configuration, logs, erreurs, OpenAPI, migrations, tests.

- [x] **F7** · README FR + EN — vérifié 2026-08-26 · `grep -in semver README.md README.fr.md` → `README.md:14` et `README.fr.md:14`, section « Status » placée avant l'installation · 14 liens relatifs résolus par `test -f`, 0 mort · 8 titres de part et d'autre · l'unique extrait de code `diff`é identique à `examples/hello-crud/src/articles/controller.rs:29-46`
      ✓ Mentionne explicitement l'absence de promesse semver avant la v1.0.

- [x] **F8** · LICENSE — vérifié 2026-08-26 · `cargo publish --dry-run -p rbs-core` → packagé et vérifié, aucun avertissement ; le contrôle mord (champ retiré → `manifest has no license or license-file`). `cargo publish --dry-run -p rbs-cli` échouait alors sur `include_dir!`, motif étranger à la licence ; les templates ont depuis rejoint la crate et les deux dry-runs passent.
      Double licence `MIT OR Apache-2.0` : `LICENSE-MIT`, `LICENSE-APACHE`, et le champ
      `license` renseigné dans les deux crates.
      ✓ `cargo publish --dry-run` ne signale aucun problème de licence.

- [x] **F9** · CONTRIBUTING et code de conduite — vérifié 2026-08-26 · section « Contributing without Node » / « Contribuer sans installer Node » en tête des deux versions, appuyée sur un fait vérifié : `grep -c 'npm\|node\|yarn' .github/workflows/ci.yml` → 0 · chaque commande citée lancée avant d'être écrite (`cargo test -p rbs-core error::tests` → 15 passed, `-- --exact` → 1 passed, `rbs --help` → sortie réelle) · Contributor Covenant 2.1, texte officiel · parité EN/FR : 7 sections de part et d'autre

- [x] **F10** · CI complète — vérifié 2026-08-26 · matrice à deux jobs (`linux` : fmt, clippy, test, `--ignored`, exemples, `publish --dry-run` ; `portabilite` sur `[macos-latest, windows-latest]`, `fail-fast: false`) · run réel sur `main` → **les trois plateformes `success`** (`fmt · clippy · test · intégration`, `macos-latest`, `windows-latest`), run `33023554339` · les tests `--ignored` restent sur Linux : les runners macOS n'exposent aucun démon Docker et `windows-latest` ne fait tourner que des conteneurs Windows · `.gitattributes` `eol=lf` ajouté contre le `core.autocrlf` des runners Windows, `git add --renormalize .` ne touche aucun fichier · **la matrice a payé au premier run** : Windows a révélé deux tests qui présumaient le séparateur `/` — `new.rs` refabriquait le TOML attendu sans l'échappement que `toml_edit` applique, `templates.rs` comparait à `"config/default.toml"` — le code de production étant sain, les deux assertions comparent désormais des chemins analysés et non leur rendu · protection de branche mise à jour pour exiger les trois contrôles, sans quoi un échec Windows ne bloquerait rien
      Linux, macOS, Windows. Tests d'intégration du CLI inclus.
      ✓ Les trois plateformes passent au vert.

- [x] **F11** · Modèles d'issues et de PR — vérifié 2026-08-26 · quatre fichiers (bug, évolution, `config.yml` sans issue vierge, modèle de PR) · front-matter validé par `yaml.safe_load` sur les trois · les commandes demandées existent (`rbs --version` → `rbs 0.1.0`, `rbs doctor` → « Diagnostique le projet : ancres, .env, base joignable, versions ») et les liens relatifs `../../ROADMAP.md` et `../CONTRIBUTING.md` résolvent

- [x] **F12** · Publication du site — vérifié 2026-08-26 · job `deploy` dans `docs.yml` (`needs: build`, conditionné à un push sur `main`, `environment: github-pages`, `upload-pages-artifact@v5` sur `docs/build` puis `deploy-pages@v5`), permissions `pages`/`id-token` portées par ce seul job, concurrence `pages` en `cancel-in-progress: false` · `examples/**` ajouté aux filtres `paths:` : le site tire ses extraits de `examples/` et se serait publié périmé sans cela · source Pages activée en `build_type: workflow`, puis run réel sur `main` → **les deux jobs `success`** · site en ligne et servi : `https://tky0065.github.io/rbs/` → 200 « Introduction | rbs », `/fr/` → 200, `cli/new` et `fr/architecture` → 200 après redirection, bascule `href=/rbs/fr/` présente dans le HTML servi · le premier run avait échoué en `404 Creating Pages deployment failed`, faute de source activée — c'était bien le seul manque
      GitHub Pages, déploiement automatique.

- [x] **F13** · Ouverture du dépôt — vérifié 2026-08-26 · dépôt `PUBLIC`, `origin/main` à `01b4bb5` · `cargo install --git https://github.com/tky0065/rbs rbs-cli --root <tmp>` → exit 0, « Installed package `rbs-cli v0.1.0 (…#01b4bb54)` (executable `rbs`) » · binaire exercé : `--version` → `rbs 0.1.0`, les cinq commandes exposées (`new`, `add`, `generate`, `migrate`, `doctor`), `rbs new demo-f13 --yes` → exit 0, 15 fichiers · installé dans une racine isolée pour ne pas confondre le dépôt public avec le `~/.cargo/bin/rbs` déjà présent · deux constats renvoyés à F3 : la forme nue sans `rbs-cli` échoue sur les binaires de `examples/`, et un projet généré déclare `rbs-core = "0.1.0"` absent de crates.io (`cargo fetch` → `no matching package named 'rbs-core' found`), ce que le quickstart traite par `--core-path` mais que le README ne mentionne pas
      ✓ Installation possible par `cargo install --git`.

### Validation du jalon

- [x] **V1** · Test du critère de sortie — PARTIEL 2026-08-27 : répétition à blanc menée,
      pas le test lui-même. Deux parcours joués au pied de la lettre en environnement isolé
      — le lecteur du `README` **échoue** (`rbs migrate up` → `no matching package named
      'rbs-core' found`, et le README n'ayant fait cloner aucun dépôt, `--core-path` est
      hors d'atteinte), le lecteur du `getting-started` **aboutit** (`POST /articles` → 201,
      `GET /articles` → 200, OpenAPI à trois chemins). Quatre frictions consignées dans
      `docs/superpowers/plans/2026-08-27-v1-frictions.md`, une contradiction corrigée au
      passage (le guide annonçait PostgreSQL 14 quand `uuidv7()` en exige 18). Le critère
      nomme **une personne extérieure au projet** : une répétition par qui connaît les
      réponses ne trouve que les frictions mécaniques, jamais les cognitives. D1, D2 et D3
      corrigés depuis — le README renvoie au guide, qui porte seul le parcours exécutable.
      D4 arbitrée et corrigée le 2026-08-27 : `doctor` rend `✗ versions` sur un noyau
      déclaré depuis crates.io tant que rbs n'y est pas publié · `cargo test -p rbs-cli
      doctor::versions` → 9 passed · `integration_crud -- --ignored` → 1 passed, le
      parcours `--core-path` restant `✓`. Reste à faire : faire jouer le parcours par un
      tiers — seul geste qui coche cette case. Protocole d'observation et consigne
      bilingue prêts : `docs/superpowers/plans/2026-08-27-v1-protocole-test-tiers.md`.
      Une personne extérieure au projet clone, installe, génère une API CRUD qui tourne,
      **sans poser de question**. Chaque question posée devient une tâche de
      documentation avant que la v0.1 ne soit déclarée close.

- [x] **V2** · Revue de parité FR/EN — vérifié 2026-08-26 · parité mesurée, pas appréciée : sur les 14 paires de pages du site, **0 écart structurel** (titres, blocs de code, encarts, liens) et **14/14 avec le même dernier commit** — la règle « les deux langues dans le même commit » a tenu sans exception · 89 entrées de traduction JSON, **0 vide** ; `docusaurus write-translations` ne produit qu'une dérive de champs `description`, aucune traduction manquante · une seule brèche trouvée et comblée : aucune version française du code de conduite, et `CONTRIBUTING.fr.md:106` renvoyait vers le texte anglais — `CODE_OF_CONDUCT.fr.md` ajouté depuis la **traduction officielle** du Contributor Covenant 2.1 (front-matter TOML retiré, adresse de signalement reprise de la version anglaise), 12 titres de part et d'autre, et le renvoi corrigé · 62 liens `.md` relatifs vérifiés, **0 mort** · `npm run clear && npm run build` → deux `[SUCCESS]`
      Toute page présente dans une langue existe et est à jour dans l'autre.

- [x] **V3** · Passe sur les conventions de code — vérifié 2026-08-26 · `cargo build -p rbs-core` et `cargo clippy -p rbs-core --all-targets` → **0 avertissement `missing_docs`**, le lint étant bien armé (`crates/rbs-core/src/lib.rs:18`) — le zéro ne vient donc pas d'un lint absent · feature générée : 7 fichiers de 19 à 158 lignes (`tests.rs` le plus gros), **aucun au-delà de ~200** · commentaires : 175 non-doc passés en deux temps — les 21 isolés lus un par un (tous porteurs d'un pourquoi), puis les 175 blocs mesurés au recouvrement lexical avec la ligne suivante, **1 seul au-dessus de 40 %** et c'est l'ancre `// <rbs:routes>` d'une fixture de test, pas de la prose · **0 paraphrase, aucune suppression à faire** · critère subjectif validé par le user
      Suppression des commentaires qui paraphrasent le code ; `missing_docs` sans
      avertissement sur `rbs-core` ; aucun fichier de feature générée au-delà de ~200 lignes.

---

## 📐 v0.2 — Auth

**Détaillé le 2026-08-27. Conception et décisions :**
[`docs/superpowers/specs/2026-08-27-v0.2-auth-design.md`](docs/superpowers/specs/2026-08-27-v0.2-auth-design.md).

> **Ouvert le 2026-08-27 alors que `V1` n'est pas clos**, sur décision explicite du
> mainteneur. Le jalon devait attendre la clôture de la v0.1 ; `V1` exige une personne
> extérieure au projet, que rien dans le dépôt ne peut produire, et l'attente aurait
> bloqué les lots `G` et `H` sans rien leur apprendre — la conception l'avait prévu
> (§5 : « la contradiction est assumée pour les lots G, H et I »). **La réserve porte
> toujours sur le lot `J`** : sa documentation se révise après le retour du tiers, et
> `J5` ne se coche pas avant que `V1` ne soit coché.

Ordre : `G ∥ H → I → J`. `G` ne touche que `rbs-core`, `H` que `rbs-cli` : aucun fichier
partagé, les deux lots se mènent en parallèle sur deux branches. `I` consomme les deux.

Frontière retenue : le noyau porte des primitives sans logique applicative ; le flux de
connexion, l'entité `User`, l'enum `Role` et les guards sont générés dans le projet, donc
lisibles et modifiables par son auteur.

### Lot G — Primitives d'auth dans le noyau

Sous le flag `auth` de `rbs-core`, déjà réservé et vide. Les cinq dépendances — `argon2`,
`jsonwebtoken`, `rand`, `sha2`, `base64` — sont **optionnelles** et tirées par le flag :
un projet sans auth ne les compile pas.

- [x] **G1** · Hachage Argon2 — `hash::{hacher, verifier}` · vérifié 2026-08-27 · `cargo test -p rbs-core --features auth hash::` → 3 passed, sel figé en constante → `deux_hachages_du_meme_mot_de_passe_different` FAILED · `verifier` rend `Ok(false)` sur mot de passe faux et `Err` sur hash illisible, jamais l'inverse · `argon2` a exigé sa feature `std` : `password-hash` n'active `rand_core/getrandom` que par elle
      `argon2 0.5.3`, paramètres par défaut de la crate, sel tiré par appel.
      ✓ Test : deux hachages du même mot de passe diffèrent.
      ✓ Test : `verifier` accepte le mot de passe correct et rejette un autre.
      ✓ Test : un hash malformé renvoie `Err`, sans panique.

- [x] **G2** · Jetons — `jwt::{Claims, signer, verifier}` · vérifié 2026-08-27 · `cargo test -p rbs-core --features auth jwt::` → 5 passed, validation élargie à HS384/HS512 → `un_jeton_signe_avec_un_autre_algorithme_est_rejete` FAILED · **réserve sur le `✓` `alg: none`** : le test passe, mais aucune mutation de notre code ne le fait tomber — `jsonwebtoken 10.3` n'a pas de variante `Algorithm::None`, l'en-tête échoue à la désérialisation avant toute validation ; la preuve de morsure est portée par le test de confusion d'algorithme, ajouté pour cela · backend `rust_crypto` et non `aws-lc-rs`, qui imposerait cmake et un compilateur C à tout projet généré
      `jsonwebtoken 10.3.0`. `Claims { sub, role, exp, iat, jti }`, HS256.
      ✓ Test : aller-retour `signer` puis `verifier` restitue les claims.
      ✓ Test : un jeton expiré renvoie une erreur typée distincte de la signature invalide.
      ✓ Test : une signature invalide est rejetée.
      ✓ Test : un jeton portant `alg: none` est rejeté.

- [x] **G3** · `AuthConfig` branchée sur figment · vérifié 2026-08-27 · `cargo test -p rbs-core --features auth config::` → 14 passed, seuil abaissé de 32 à 0 → `un_secret_de_moins_de_32_octets_est_refuse_au_chargement` FAILED · `cargo build -p rbs-core` sans le flag → Finished, le champ n'est pas compilé et aucune section `auth` n'est requise · message réel du secret absent : ``missing field `secret` for key "default.auth"`` — il nomme le champ, sans la forme littérale `auth.secret` · les onze cas préexistants reçoivent le secret par l'environnement, le compte sans flag reste celui de `main` (72 = 72)
      Champ `auth` de `Config`, compilé sous `#[cfg(feature = "auth")]` : secret et durées
      de vie de l'accès et du rafraîchissement. Le chargement en cascade de `A5` est
      réutilisé tel quel, et non doublé par une lecture directe de l'environnement.
      ✓ Test : secret absent → échec au boot, message nommant le champ.
      ✓ Test : secret de moins de 32 octets → refus au chargement.
      ✓ Test : `cargo build -p rbs-core` sans le flag `auth` ne compile pas le champ.

- [x] **G4** · Extracteur `Identity` et trait `HasAuth` · vérifié 2026-08-27 · `cargo test -p rbs-core --features auth extract::` → 8 passed, préfixe `Bearer` rendu facultatif → `un_en_tete_sans_le_schema_bearer_est_refuse` FAILED (200 et corps `u1 admin` au lieu de 401) · 401 rendu en `application/problem+json`, et les trois échecs — expiré, signé ailleurs, malformé — rendent 401 · un en-tête absent donne 401 quel que soit le traitement du préfixe : c'est le test du schéma qui éprouve la garde, pas celui de l'en-tête manquant
      Lit l'en-tête `Authorization: Bearer`, vérifie le jeton, expose
      `Identity { user_id, role: String }`. Le rôle reste une chaîne : l'enum `Role` est
      généré, donc hors de portée du noyau.
      ✓ Test : en-tête absent → 401 en `application/problem+json`.
      ✓ Test : jeton invalide ou expiré → 401.
      ✓ Test : jeton valide → identité peuplée depuis les claims.

- [x] **G5** · Jetons opaques — `token::{aleatoire, empreinte}` · vérifié 2026-08-27 · `cargo test -p rbs-core --features auth token::` → 3 passed, tirage figé en `[7u8; 32]` → `deux_tirages_successifs_different` et `l_empreinte_est_deterministe_et_ne_rend_pas_le_jeton` FAILED · dans `rand 0.10` `OsRng` n'existe plus : c'est `SysRng`, dont `try_fill_bytes` rend un `Result` — l'échec du générateur système est un `expect`, aucun appelant ne saurait le traiter
      Tirage de 32 octets par `OsRng` encodés en base64url, et empreinte SHA-256 de ce
      jeton pour le stockage. Volontairement **pas** d'Argon2 ici : un jeton de 256 bits
      tirés au hasard n'est pas devinable par force brute, et un KDF lent se paierait à
      chaque rafraîchissement sans rien acheter. La primitive vit dans le noyau pour que
      le projet généré n'ait pas à choisir lui-même entre un générateur cryptographique
      et un générateur ordinaire.
      ✓ Test : deux tirages successifs diffèrent.
      ✓ Test : le jeton décodé porte au moins 32 octets.
      ✓ Test : `empreinte` est déterministe et ne permet pas de retrouver le jeton.

### Lot H — Le moule des fragments

`rbs add` ne sait installer que des fragments sans code Rust. Ce lot lui apprend à en
installer qui en apportent, sans que le CLI connaisse aucune feature par son nom.

- [x] **H1** · Format `feature.toml` et son parseur · vérifié 2026-08-27 · `cargo test -p rbs-cli --lib manifeste::` → 3 passed, `deny_unknown_fields` retiré → `un_champ_inconnu_nomme_le_champ_et_le_fichier` FAILED (`unwrap_err() on an Ok value`) · écart au plan : la désérialisation passe par `toml_edit::de`, déjà présent, plutôt que par la crate `toml` — une dépendance de moins, et le message nomme toujours le champ et le fichier · trois points que la conception ne montrait pas sont comblés : nom de la migration, contenu d'une section de configuration, variable d'environnement
      Un manifeste par répertoire de `templates/features` : fichiers, insertions d'ancres,
      migration, features Cargo, sections de configuration.
      ✓ Test : un manifeste valide se désérialise dans la structure attendue.
      ✓ Test : un champ inconnu → erreur nommant le champ et le fichier fautif.

- [x] **H2** · `add` interprète le manifeste ; `docker` et `ci` migrés · vérifié 2026-08-27 · `git diff crates/rbs-cli/tests/integration_add.rs` → **0 suppression**, un seul hunk en fin de fichier (vide au moment du commit ; les 144 lignes sont les ajouts ultérieurs) : les quatre cas d'origine sont intacts · `cargo test -p rbs-cli --test integration_add` → 4 passed · le piège s'est bien manifesté — créer les deux `feature.toml` a fait tomber 3 tests, le manifeste étant copié chez l'utilisateur ; exclusion posée dans `Source::fichiers()`, son retrait fait tomber `le_manifeste_du_fragment_n_est_pas_copie_dans_le_projet` · un fragment sans manifeste rend `Erreur::SansManifeste`, pas un panic
      Migration à comportement constant : les deux fragments existants reçoivent un
      manifeste trivial et s'installent exactement comme avant.
      ✓ Les tests actuels de `add` passent **sans être modifiés**.

- [x] **H3** · Insertions dans les ancres déclarées · vérifié 2026-08-27 · `cargo test -p rbs-cli --lib add::` → 20 passed et `--test integration_add` → 8 passed, le cas de l'ancre absente étant désormais éprouvé **sur la commande elle-même** et non plus seulement par `generate` : insertion rendue silencieuse dans l'interprète → `une_ancre_absente_arrete_l_installation_sans_rien_ecrire` FAILED, l'installation aboutissant et déposant ses fichiers · code de sortie 1, ancre et fichier nommés sur stderr, bloc sur stdout, répertoire intact · **limite relevée, préexistante** : le bloc affiché porte l'ancre à recréer, vide, et non le contenu que le fragment voulait y insérer — le développeur doit le deviner ; `generate` a le même défaut, il n'est pas né ici
      Réutilise `ancres.rs`. Insertion juste avant la balise fermante, sans réordonner
      l'existant.
      ✓ Test : le contenu déclaré est inséré dans chacune des quatre ancres.
      ✓ Test : ancre absente → **rien n'est écrit**, le bloc à coller est affiché, sortie en erreur.

- [x] **H4** · Migration horodatée déposée par un fragment · vérifié 2026-08-27 · `cargo test -p rbs-cli --lib -- add::tests::la_migration add::tests::l_ancre_migrations` → 2 passed, horodatage retiré du nom → `la_migration_du_fragment_est_deposee_au_format_horodate` FAILED · réutilise `generate::migration::horodatage_courant()` et `generate::montage::pour_migration()` : aucun second format d'horodatage, donc aucun second ordre de migration possible · les deux ancres distinctes sont complétées, le `mod` et l'entrée du `Migrator`
      Réutilise la génération de migration de `generate crud`.
      ✓ Test : le fichier est créé au format horodaté attendu.
      ✓ Test : l'ancre `migrations` est complétée par l'appel correspondant.

- [x] **H5** · Patchs de `Cargo.toml`, `config/default.toml` et `.env.example` · vérifié 2026-08-27 · `cargo test -p rbs-cli --lib add::installation` → 7 passed et `plan::texte` → 7 passed · re-sérialisation du manifeste patché → `les_commentaires_du_developpeur_survivent_au_patch` FAILED **sans que le test de non-reformatage s'en aperçoive** : c'est bien le test des commentaires qui tient cette garantie · `PatchToml::AjouterFeatureADependance`, écrit sans appelant depuis le lot E, a enfin le sien ; aucun second chemin de patch · les deux nouvelles actions n'écrivent qu'en fin de fichier, le texte d'origine traverse octet pour octet
      `toml_edit` pour activer une feature sur une dépendance déjà présente.
      ✓ Test : `rbs-core` gagne `features = ["auth"]` sans que le reste du manifeste soit reformaté.
      ✓ Test : les commentaires du développeur survivent au patch.
      ✓ Test : la section de configuration et la variable d'environnement sont ajoutées.

- [x] **H6** · Idempotence et tout-ou-rien sur un fragment à code Rust · vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_add` → 8 passed, sur un fragment de test exerçant les six sections du manifeste · `journal.defaire()` supprimé → 2 tests de restauration FAILED ; idempotence portée sur la présence d'un fichier au lieu de `[package.metadata.rbs]` → `un_fichier_supprime_ne_fait_pas_reinstaller_la_feature` FAILED (`+ src/essai/service.rs est apparu`) · **à connaître** : avant cette tâche le test des deux installations passait par chance, les deux exécutions tombant dans la même seconde et la migration horodatée portant le même nom — c'est la garde sur les métadonnées qui le rend solide
      La vérification porte sur `[package.metadata.rbs]`, pas sur la présence des fichiers.
      ✓ Test : deux installations successives — la seconde n'écrit rien.
      ✓ Test : échec à mi-parcours → les fichiers déjà écrits sont restaurés.

### Lot I — La feature auth générée

`src/auth/{mod,model,dto,repository,service,controller,tests}.rs`, une migration, quatre
insertions d'ancres. Dépend de `G` et de `H`.

- [x] **I1** · Manifeste d'auth et squelette des templates — vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_auth` → 3 passed, `-- --ignored` → 1 passed · le critère est pris au niveau qu'exige la CI d'`add ci` — `clippy -D warnings` et `fmt --check` du projet généré, non `cargo check` seul · ancre `openapi` retirée du manifeste → `les_quatre_ancres_du_projet_sont_completees` FAILED ; `#![allow(dead_code)]` retiré de `mod.rs` → 5 erreurs dead_code, ce qui est la raison d'être de cette ligne, que I3 retirera · **écart assumé** : dépose dans `src/auth/` et non `src/features/auth/` — l'ancre `features` insère `mod auth;` en tête de `main.rs`, et un `src/features/mod.rs` partagé entre fragments se heurterait à l'idempotence de H6 · à connaître pour I2 : `Manifeste.migration` est un `Option`, donc **une seule** migration par fragment
      ✓ `rbs new` puis `rbs add auth` → `cargo check` du projet généré passe.
      ✓ Les quatre ancres sont complétées.

- [x] **I2** · Entités et migrations `users`, `refresh_tokens`, enum `Role` — vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_auth -- --ignored` → 2 passed, dont `la_migration_d_auth_cree_le_schema_puis_le_rend_a_son_etat_initial` qui interroge un PostgreSQL 18 réel par `psql` · trois morsures : index déplacé de `token_hash` vers `user_id` → FAILED, `unique_key()` retiré d'`email` → FAILED, `down` privé du `DROP TABLE users` → `` `users` survit à `migrate down` `` · **écart assumé** : une seule migration `create_auth_tables` et non deux — le moule ne pose qu'une migration par fragment, `migrate down` n'en annule qu'une, et les deux tables arrivent et repartent avec la feature ; l'ordre de création s'en trouve garanti par construction · `Role` en VARCHAR via `DeriveActiveEnum` : un rôle de plus ne demandera aucune migration · **corrigé au passage** : le `cargo fmt --check` du projet généré, ajouté en I1, retrouvait le workspace de rbs et signalait ses fichiers — remplacé par `rustfmt` sur les racines de modules, vérifié insensible à un défaut dans le dépôt et sensible à un défaut dans le projet généré
      ✓ `rbs migrate up` puis `down` → schéma créé puis rendu à son état initial.
      ✓ Contrainte d'unicité sur `email`, index sur `token_hash`.

- [x] **I3** · Register et login — vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_auth -- --ignored` → 4 passed, dont les 7 tests d'auth joués dans le projet généré contre un PostgreSQL 18 réel · deux morsures : un `tracing::debug!` du hash ajouté dans `register` → `le hash est journalisé` FAILED ; le hash de comparaison retiré de `login` → `une adresse inconnue répond en 2.012458ms contre 240.162834ms` FAILED, **les quatre autres tests passant malgré la faille** — c'est le test de durée seul qui la tient · **écart assumé** : la moitié « logs » du deuxième critère se prouve côté rbs, sur la sortie réelle du binaire à `RUST_LOG=debug`, le projet généré n'ayant pas `tracing-subscriber` et le moule des fragments ne sachant pas ajouter de dev-dependency — preuve plus large, du reste : elle couvre aussi les middlewares du noyau · **à connaître pour J4** : `add auth` n'écrit `RBS_AUTH__SECRET` que dans `.env.example`, jamais dans `.env` — un projet fraîchement doté d'auth ne démarre pas tant que l'utilisateur ne l'a pas recopié · `#![allow(dead_code)]` **conservé**, contrairement à ce qu'annonçait I1 : `RefreshRequest` et `repository::find` n'auront de lecteur qu'avec I4 et I5, son commentaire les nomme désormais
      ✓ Test : inscription → 201.
      ✓ Test : le hash n'apparaît ni dans la réponse ni dans les logs.
      ✓ Test : email déjà pris → 409.
      ✓ Test : mot de passe erroné et email inconnu renvoient **la même** 401, sans oracle d'énumération.

- [x] **I4** · Refresh avec rotation — vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_auth -- --ignored` → 4 passed, dont les 11 tests d'auth du projet généré contre PostgreSQL 18 · trois morsures : garde `revoked_at IS NULL` retirée de `consommer` → le rejeu rend une paire valide, `l_ancien_refresh_est_ensuite_refuse` FAILED ; filtre d'expiration retiré → `un_refresh_expire_rend_401` FAILED ; `token_hash` empli d'autre chose que l'empreinte → `la colonne ne porte pas l'empreinte du jeton` FAILED · **la rotation tient à un seul `UPDATE` conditionnel** — le `WHERE revoked_at IS NULL` de `consommer`, qui rend le nombre de lignes touchées : la relecture de `revoked_at` écrite d'abord s'est révélée redondante et non tenue par un test, donc retirée ; elle laisserait de toute façon passer deux rafraîchissements concurrents · le test de l'empreinte cherche sa ligne par `user_id` et non par l'empreinte — chercher par ce qu'on vérifie faisait échouer le test à la lecture, sans jamais atteindre l'assertion · `#![allow(dead_code)]` **retiré** de `mod.rs` : ce qu'I1 attendait d'I3, et que I4 permet enfin
      ✓ Test : un refresh valide rend une nouvelle paire de jetons.
      ✓ Test : l'ancien refresh est ensuite refusé (401).
      ✓ Test : un refresh expiré → 401.
      ✓ Test : requête sur la table — la colonne stockée porte l'empreinte `token::empreinte`
      et jamais le jeton remis au client.

- [x] **I5** · Logout et révocation — vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_auth -- --ignored` → 4 passed, dont les 14 tests d'auth du projet généré · morsure : `consommer` élargie du `id` au `user_id` par sous-requête → `les_autres_sessions_du_meme_compte_restent_valides` FAILED, **et aussi** `l_ancien_refresh_est_ensuite_refuse`, la révocation par compte emportant la ligne que `refresh` venait de créer · **aucune ligne nouvelle dans `repository.rs`** : `logout` n'est que la moitié de `refresh` — même empreinte, même `consommer`, sans réémission, ce qui confirme la granularité choisie en I4 · le contrat 204 / 401-jeton-inconnu est celui que le controller d'I1 publie déjà dans le document OpenAPI, un logout idempotent l'aurait contredit · **trou repéré dans le backlog** : le corps de `me` n'est écrit par aucune tâche d'I3 à I7, alors que la route est montée et sera enregistrée dans OpenAPI par I7 — à trancher au design d'I6
      ✓ Test : logout → 204.
      ✓ Test : le refresh révoqué → 401.
      ✓ Test : les autres sessions du même utilisateur restent valides.

- [x] **I6** · Guard `require_role` — vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_auth -- --ignored` → 4 passed, dont les 18 tests d'auth du projet généré · deux morsures : comparaison du rôle neutralisée → `un_user_sur_une_route_admin_rend_403` FAILED ; route rendant 403 sans regarder le jeton → `sans_jeton_la_route_admin_rend_401` FAILED, ce qui prouve que le test distingue bien les deux statuts · **trait d'extension sur `Identity` et non layer** : `from_fn_with_state` n'accepte pas de paramètre supplémentaire, il faudrait une closure au type de retour imprononçable ou une fonction par rôle — ce qui figerait l'enum que I2 a rendu extensible sans migration · **à connaître** : le projet généré est un binaire, donc n'exporte rien — un point d'extension que l'utilisateur n'appelle pas encore est du code mort pour `clippy -D warnings`, d'où un `#[allow(dead_code)]` ciblé sur le trait, commenté comme tel · **écart assumé** : le corps de `me` est écrit ici, aucune tâche d'I3 à I7 ne le prévoyant alors que I7 s'apprête à publier un contrat annonçant 200 sur une route qui rendait 501 ; `a_ecrire` perd son dernier appelant et disparaît — le fragment ne livre plus aucune route non implémentée
      Généré dans le projet, à partir de l'enum `Role` qu'il y trouve.
      ✓ Test : un `user` sur une route admin → 403.
      ✓ Test : un `admin` sur la même route → 200.
      ✓ Test : sans jeton → **401 et non 403**.

- [x] **I7** · Enregistrement OpenAPI — vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_auth -- --ignored` → 4 passed, dont les 21 tests d'auth du projet généré, et `cargo test --workspace --all-features` → 514 passed · morsure : schéma retiré de `ReponsesCommunes` → `le_schema_de_securite_bearer_est_declare` FAILED dans le noyau **et** `le_schema_bearer_est_declare_et_me_le_porte` dans le projet généré · le premier critère était déjà tenu par l'ancre `openapi` d'I1 : le test le prouve désormais au lieu de le supposer · **écart assumé** : le critère nomme `/openapi.json`, le document vit sur `/api-docs/openapi.json` depuis C4 — c'est l'URL que Swagger UI charge, le test le lit là où il est · seul `me` porte `security` : `refresh` et `logout` s'authentifient par leur corps, et un test interdit qu'on leur appose le schéma · **corrigé hors périmètre** : la CI ne compilait `rbs-core` avec aucune feature — tout le lot G n'était vérifié par aucune exécution automatique, et le schéma ajouté ici serait tombé dans le même angle mort ; `--all-features` posé sur les étapes clippy et test des deux jobs du workspace, mesuré propre avant d'être écrit, 72 → 90 tests couverts dans le noyau (l'étape `examples/`, qui porte sur un projet généré, est laissée telle quelle)
      ✓ Les cinq chemins d'auth figurent dans `/openapi.json`.
      ✓ Le schéma de sécurité `bearer` est déclaré et les routes protégées le portent.

### Lot J — Documentation et sortie du jalon

- [x] **J1** · `examples/blog-auth`, compilé en CI — vérifié 2026-08-27 · run CI `33080485433` **vert sur les trois plateformes**, step `cargo clippy (examples)` traitant `blog-auth` puis `hello-crud` · `cargo test -p rbs-cli --test integration_examples` → 11 passed, les deux exemples comparés à une génération fraîche · trois morsures : une garde retirée du controller → `les trois mutations doivent porter la garde` FAILED ; `features = ["auth"]` retiré du manifeste → dérive signalée `Cargo.toml` ligne 15, **ce que l'ancien masquage laissait passer** ; `list` renommée dans un fichier généré → dérive signalée `src/posts/service.rs` ligne 9 · le step d'exemples **boucle sur `examples/*/`** au lieu de nommer un répertoire : un exemple ajouté sans être inscrit ici ferait mentir sa page sans que rien n'échoue · les trois fichiers retouchés à la main sortent de la comparaison octet à octet et reçoivent des assertions de contenu — sans quoi la liste d'exclusion serait une porte ouverte à la dérive qu'elle déclare · **écart assumé** : le CRUD s'appelle `posts` et non `articles` — ce qui distingue l'exemple est la protection, pas la ressource, et le nom laisse l'ancre `features` triée · **écart assumé** : le `.env` de l'exemple ne porte pas `RBS_AUTH__SECRET`, `add auth` ne l'écrivant que dans `.env.example` — l'exemple reste exactement ce que le CLI produit, et c'est la friction que `J4` doit diagnostiquer · **la matrice a repayé** : `windows-latest` a signalé les deux non-dérives, `toml_edit` y écrivant le chemin UNC en chaîne littérale à guillemets simples que le masquage restreint ne reconnaissait pas — corrigé, deux tests neufs portant la ligne exacte du runner · **à connaître pour J3** : régions posées `create` (controller), `require_role` (guard), `harnais`, `signee`, `jeton_admin`, `cycle_de_vie`, `refus`, `erreur_404`, `corps_illisible` (tests) · **deux défauts du CLI découverts, non corrigés** : `rbs new --with auth` répond `disponibles : docker, ci`, `FEATURES_CONNUES` (`new.rs:20`) n'ayant pas suivi le lot I ; et `generate crud` rend un `service.rs` que rustfmt reformate dès que le nom est court — `articles` est le seul nom sur lequel `le_rendu_traverse_rustfmt_sans_diff` l'éprouve, or `rbs add ci` pose un `cargo fmt --check` dans le projet généré
      Articles protégés par `require_role(Role::Admin)`. Le site tirant ses extraits de
      `examples/`, cet exemple est la source de la page de documentation.
      ✓ Le job d'exemples passe au vert.

- [ ] **J2** · `integration_auth` sous testcontainers
      `#[ignore]` par défaut, comme `integration_crud`.
      ✓ Le parcours entier joué contre un PostgreSQL réel : register → login → 401 sans
      jeton → 403 en `user` → refresh → ancien refresh 401 → logout → refresh 401.

- [ ] **J3** · Page de documentation FR et EN
      **À réviser après `V1`** : le test par un tiers n'ayant pas été joué, les frictions
      cognitives qu'il révélera toucheront cette page.
      ✓ Parité stricte FR/EN mesurée comme en `V2`.
      ✓ Aucun extrait de code non issu de `examples/blog-auth`.

- [ ] **J4** · `doctor` diagnostique l'auth
      Leçon directe de la friction `D4` : un utilisateur bloqué lance `doctor`, la
      commande doit lui apprendre ce qui le bloque.
      ✓ Secret absent → `✗` nommant la variable d'environnement.
      ✓ Secret trop court → `✗`.
      ✓ Feature `auth` déclarée sans section `[auth]` dans la configuration → `✗`.

- [ ] **J5** · Critère de sortie du jalon
      ✓ Une API protégée, générée de bout en bout, prouvée par `J2`.

---

## ⏳ Jalons suivants

Volontairement en grosses mailles. Détailler ces tâches aujourd'hui serait de la
fiction : elles seront réécrites avec ce que les jalons précédents auront appris.

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
