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

- [x] **A5** · Chargement de configuration — vérifié 2026-08-25 · `cargo test -p rbs-core config` → 6 passed, dont `un_champ_requis_manquant_fait_echouer_le_chargement_en_nommant_le_champ` et `une_variable_d_environnement_ecrase_la_valeur_du_fichier_toml` lancés seuls → 1 passed chacun
      `figment` fusionnant défauts → `config/default.toml` → `config/{RBS_ENV}.toml` →
      `.env` → variables d'environnement, désérialisé dans une struct typée.
      ✓ Test : variable requise manquante → échec au boot, message nommant le champ.
      ✓ Test : une variable d'environnement écrase la valeur du fichier TOML.

- [ ] **A6** · Formateur de logs `pretty` — PARTIEL 2026-08-25 : `cargo test -p rbs-core logs` → 5 passed (dont couleurs absentes hors TTY) · rendu des cinq niveaux validé de visu sur `cargo run -p rbs-core --example logs_pretty`. Reste la capture dans les docs, qui attend le site (F1).
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

- [ ] **E8** · `rbs add ci` — PARTIEL 2026-08-26 : `cargo test -p rbs-cli --bins templates::` → 20 passed · sous `act`, sur un projet neuf : `cargo fmt` **passé**, `cargo clippy` **passé**, migrations **appliquées** contre le service PostgreSQL 18 du workflow ; `cargo test` échoue au build de `utoipa-swagger-ui`, qui télécharge son archive et bute sur la résolution DNS de la machine (`curl exit status 6`, `github.com` irrésolu jusque dans le conteneur). Trois étapes sur quatre : le critère demande le run entier, à rejouer sur un réseau stable ou sur un vrai runner.
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

- [x] **F8** · LICENSE — vérifié 2026-08-26 · `cargo publish --dry-run -p rbs-core` → packagé et vérifié, aucun avertissement ; le contrôle mord (champ retiré → `manifest has no license or license-file`). Le dry-run de `rbs-cli` échoue en aval, sur `include_dir!` : les templates vivent hors de la crate et `cargo package` ne les emporte pas — motif étranger à la licence, à traiter avant toute publication.
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
