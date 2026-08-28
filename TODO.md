# TODO — rbs

Tâches actionnables. De la **v0.1** à la **v0.4** les jalons sont détaillés ; la **v1.0**
figure en grosses mailles et sera détaillée à son tour, avec ce que les précédents auront
appris. Détailler un jalon ne l'ouvre pas : l'ordre des lots reste contraignant.

Design de référence : [`docs/superpowers/specs/2026-08-25-rbs-design.md`](docs/superpowers/specs/2026-08-25-rbs-design.md)
Vision et jalons : [`ROADMAP.md`](ROADMAP.md)

Chaque tâche porte son critère de validation (`✓`). Une case ne se coche jamais sur une
impression.

> **Les identifiants du dépôt sont passés à l'anglais le 2026-08-28.** Les lignes de
> preuve antérieures citent les noms qui étaient les leurs le jour où la commande a été
> lancée — `le_backend_fichiers_depose_lit_atteste_puis_supprime` est devenu
> `the_file_backend_puts_gets_attests_then_deletes`, et ainsi de trente-six lignes. Elles
> ne sont pas réécrites : une preuve est un compte rendu daté, et lui donner après coup un
> nom qui n'existait pas ce jour-là la rendrait fausse. Pour rejouer l'une d'elles,
> `docs/superpowers/plans/2026-08-28-glossaire-migration-anglais.md` donne la
> correspondance.

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

- [x] **J2** · `integration_auth` sous testcontainers — vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_auth -- --ignored` → 6 passed en 56,79 s, dont les huit étapes jouées contre PostgreSQL 18 et le binaire du projet **réellement lancé sur un port libre**, là où les 21 tests d'auth du projet généré montent le `Router` en mémoire · trois morsures : garde `revoked_at IS NULL` retirée de `consommer` → `le refresh déjà consommé doit être refusé` FAILED (200) ; `consommer` retirée de `logout` seul → `le refresh révoqué doit être refusé` FAILED (200), **une étape distincte de la précédente**, ce qui montre que rotation et révocation sont tenues séparément ; `require_role(Role::Admin)` retirée de `posts::create` → 201 au lieu de 403, **l'étape « 401 sans jeton » restant verte** — le test distingue bien les deux statuts · **écart assumé** : le parcours est joué en deux séquences et non une. `add auth` livre la garde mais aucune route qui la porte — la seule du projet généré est montée dans son module de tests et ne répond jamais sur le réseau ; le 403 se joue donc sur `examples/blog-auth`, dont c'est la **première exécution**, J1 n'en prouvant que la compilation · **deux étapes ajoutées hors critère** : `me` avec le jeton de `login`, puis avec celui rendu par `refresh` — sans elles, un `login` qui émettrait un jeton que la garde refuse passerait toutes les autres étapes · **corrigé hors périmètre** : les tests se partagent `debug/migration` par `CARGO_TARGET_DIR`, et `cargo run` relâche son verrou avant d'exécuter — un `No such file or directory (os error 2)` observé une fois sur huit, et surtout un faux vert possible quand la course se gagne au lieu de se perdre (le test exécute alors les migrations d'un autre projet) ; verrou posé après le conteneur, 18,71 → 56,79 s, le total valant désormais la somme des durées et non leur maximum · **à connaître** : `refresh` et `logout` portent le **même** `if !repository::consommer(db, session.id).await? {` — une morsure posée par recherche de texte frappe `refresh` en croyant frapper `logout`, et fait échouer une étape qui n'est pas celle visée
      `#[ignore]` par défaut, comme `integration_crud`.
      ✓ Le parcours entier joué contre un PostgreSQL réel : register → login → 401 sans
      jeton → 403 en `user` → refresh → ancien refresh 401 → logout → refresh 401.

- [x] **J3** · Page de documentation FR et EN — vérifié 2026-08-27 · parité mesurée page par page comme en `V2` (titres et niveaux, blocs de code avec leur langue et leur méta `file=`/`region=`, encarts, liens relatifs) → **15 paires, 0 écart structurel**, et **15/15 au même dernier commit** · `npm run clear && npm run build` → deux `[SUCCESS]` · l'instrument de mesure éprouvé avant de servir de preuve : un titre retiré du FR, une ligne changée dans une sortie de terminal, une page FR absente → un écart signalé à chaque fois · deux morsures sur le build, qui est ce qui tient le second critère : `region=require_rolle` → « La région « require_rolle » est introuvable dans examples/blog-auth/src/auth/guard.rs », et `config/defaut.toml` → « introuvable. Le fichier a-t-il été déplacé ou l'exemple régénéré ? » · **inventaire des blocs** : 9 par page, dont **7 de code portant tous `file=examples/blog-auth/…`** — zéro extrait écrit à la main ; les 2 restants sont une sortie de `rbs add auth` et une invocation de `rbs doctor`, non du code du projet · **aucune ligne de `examples/blog-auth` modifiée** : `integration_examples.rs:53` n'autorise que trois fichiers retouchés à la main, et le plugin acceptant `file=` sans `region=`, `.env.example` et `config/default.toml` sont cités entiers plutôt que d'y poser des régions neuves · `npm run write-translations --locale fr` → 0 entrée vide, aucune traduction manquante, seule reparaît la dérive de champs `description` que `V2` avait déjà constatée · **quatre défauts du CLI corrigés en passant**, tous la même désynchronisation entre une liste écrite à la main et le catalogue tiré des fragments : `FEATURES_CONNUES` ignorait `auth`, le message des features connues annonçait une commande `add` « pas encore exposée », l'aide de `rbs add` listait « docker, ci », et `suite()` ne disait rien après `add auth` — la feature dont l'étape suivante compte le plus était la seule muette ; deux tests comparent désormais ces listes au catalogue · **cinquième défaut, de documentation** : la section d'idempotence de `cli/add.md` montrait un plan « inchangé » que la commande ne rend plus depuis qu'elle court-circuite sur `[package.metadata.rbs]`, et les sorties de `add docker`/`add ci` avaient perdu la ligne de description que chaque installation affiche — recapturées sur le binaire · **réserve maintenue** : la page se révise après `V1`, dont les frictions cognitives la toucheront
      **À réviser après `V1`** : le test par un tiers n'ayant pas été joué, les frictions
      cognitives qu'il révélera toucheront cette page.
      ✓ Parité stricte FR/EN mesurée comme en `V2`.
      ✓ Aucun extrait de code non issu de `examples/blog-auth`.

- [x] **J4** · `doctor` diagnostique l'auth — vérifié 2026-08-27 · `cargo test -p rbs-cli doctor::` → 46 passed, dont 9 neufs, et les trois critères relus sur la **sortie réelle** du binaire dans un projet doté par `rbs add auth` : `✗ auth  RBS_AUTH__SECRET n'est renseignée ni dans le .env ni dans l'environnement`, `✗ auth  RBS_AUTH__SECRET porte 10 octets, il en faut 32`, `✗ auth  config/default.toml ne porte pas de section [auth]`, et `✓ auth  le secret et la configuration sont en place` une fois corrigé · cinq morsures, une par constat : chacune n'a rendu rouge que son propre test, sauf la section réputée toujours présente qui en emporte deux — celui du commentaire compris · **quatrième constat ajouté hors critères** : la ligne d'exemple fait 61 octets, donc recopiée telle quelle depuis `.env.example` — ce que le remède du contrôle `.env` **suggère déjà de faire** — elle passe les trois critères et le projet signe ses jetons avec une clé publiée dans Git ; elle se reconnaît en comparant `.env` à `.env.example`, sans chaîne écrite dans le CLI · **un seul `Controle` qui agrège**, comme `env` agrège ses variables manquantes : quatre lignes `auth` dans un rapport de cinq contrôles seraient du bruit · le contrôle n'entre au rapport que si la feature est déclarée dans `[package.metadata.rbs]` · l'environnement l'emporte sur le `.env`, comme dans `migrate::variables_du_projet` · le seuil de 32 octets **duplique** `SECRET_MINIMUM` de `rbs-core`, que `rbs-cli` ne peut pas lire — les deux crates sont indépendantes par construction · **écart de méthode assumé** : l'implémentation a été écrite avant ses tests, contrairement au plan ; ce sont les morsures qui en tiennent lieu de preuve, et non un échec de compilation · **corrigé en passant** : les remèdes ajoutaient six espaces à ceux que `rendu` pose déjà, les blocs à coller partant douze colonnes à droite de leur phrase
      Leçon directe de la friction `D4` : un utilisateur bloqué lance `doctor`, la
      commande doit lui apprendre ce qui le bloque.
      ✓ Secret absent → `✗` nommant la variable d'environnement.
      ✓ Secret trop court → `✗`.
      ✓ Feature `auth` déclarée sans section `[auth]` dans la configuration → `✗`.

- [x] **J5** · Critère de sortie du jalon — PARTIEL 2026-08-27 : son unique critère est
      tenu — `J2` joue le parcours complet contre un PostgreSQL réel, et `J1` compile
      l'exemple en CI. Ce qui retient la case est la réserve inscrite en tête de ce jalon :
      « `J5` ne se coche pas avant que `V1` ne soit coché », et `V1` attend une personne
      extérieure au projet, que rien dans le dépôt ne peut produire. Protocole prêt :
      `docs/superpowers/plans/2026-08-27-v1-protocole-test-tiers.md`.
      ✓ Une API protégée, générée de bout en bout, prouvée par `J2`.

---

## 🔌 v0.3 — Intégrations

**Détaillé le 2026-08-27. Conception et décisions :**
[`docs/superpowers/specs/2026-08-27-v0.3-integrations-design.md`](docs/superpowers/specs/2026-08-27-v0.3-integrations-design.md).

> Le critère de sortie annoncé — trois features ajoutées **sans toucher au noyau** — n'était
> pas atteignable en l'état : la configuration d'une feature n'a pas de place hors de
> `Config`, le manifeste de fragment ne sait pas ajouter de dépendance, et aucune ancre
> n'atteint `state.rs`. C'est ce que le `ROADMAP` prévoyait — « si l'une d'elles oblige à
> modifier `rbs-core`, c'est le moule qui est à revoir ». Le lot `K` le revoit, **et il est
> le seul lot du jalon autorisé à modifier le noyau**. C'est ce bornage qui rend le critère
> mesurable, par le diff que vérifie `O4`.

Ordre : `K1 ∥ K2 → K3 → (L ∥ M ∥ N) → O`. `K1` ne touche que `rbs-core`, `K2` que `rbs-cli`.
`L`, `M` et `N` sont disjointes à l'écriture — un répertoire de fragment et un test
d'intégration chacune, `FEATURES_CONNUES` (`new.rs:23`) pour seul fichier commun — mais
**leur vérification ne l'est pas** : cible de compilation partagée, verrou de cargo, un
conteneur par test. Les `--ignored` se passent en fin de parcours, une par une.

### Lot K — Le moule, deuxième tour

Ne livre aucune feature. Lève les trois verrous que la conception a relevés.

- [x] **K1** · `config::section::<T>` — le noyau ouvre sa cascade — vérifié 2026-08-27 · `cargo test -p rbs-core --lib -- config::tests` → les trois tests neufs passent, et `cargo test -p rbs-core` → 75 passed, 0 failed · `cargo build -p rbs-core --no-default-features` → Finished, la fonction ne portant aucun `#[cfg]` et `default = []` rendant les deux builds identiques · trois morsures, une par critère : garde `figment.contains` retirée → `SectionAbsente` FAILED ; couche `Env::prefixed` retirée de `surcharges` → la cascade FAILED (`left: "depuis-default"`) ; `Serialized::default("externe.ttl_secs", 999)` opposé par le noyau → les défauts de l'appelant FAILED (`left: 999, right: 300`) · **faux vert corrigé** : le premier test ne lisait que le message, or figment nomme lui aussi la section absente (`missing field \`externe\``) — la morsure passait alors, le test ne prouvant ni la garde ni la variante ; l'assertion porte désormais sur `ConfigError::SectionAbsente` · `clippy --all-features --all-targets -D warnings` et `fmt --all --check` propres · la section de test s'appelle `externe` : aucun nom de feature du jalon n'entre dans le code ajouté · **réserve pour `O4`** : `redis`, `mail` et `storage` sont déjà nommées dans `rbs-core` avant ce lot — features vides réservées dans `Cargo.toml`, documentées dans `lib.rs` — le critère de sortie se mesure donc sur le diff, non sur l'absence des noms

- [x] **K2** · `[[dependances]]` au manifeste de fragment — vérifié 2026-08-27 · `cargo test -p rbs-cli --lib` → 419 passed, dont les trois tests des critères : `la_dependance_declaree_arrive_avec_sa_version_ses_features_et_son_default_features` (`lettre = { version = "0.11", default-features = false, features = ["smtp-transport", "builder"] }`), `les_commentaires_et_la_mise_en_forme_survivent_a_l_ajout_d_une_dependance` (chaque ligne du témoin retrouvée verbatim, exactement une de plus) et `une_dependance_deja_declaree_dans_le_projet_n_est_pas_dupliquee` · `cargo test -p rbs-cli --test integration_add` → 11 passed, dont `les_dependances_du_fragment_arrivent_dans_le_manifeste_du_projet`, qui l'éprouve au niveau de la commande réelle · deux morsures : report du décor retiré d'`etaler` → 4 FAILED dont `les_commentaires_du_developpeur_survivent_au_patch` ; déclaration existante effacée puis réinsérée → 5 FAILED dont `une_dependance_deja_declaree_ne_produit_aucun_texte` · **observation sur la seconde morsure** : le test que le critère nomme reste vert, une réinsertion ne dupliquant pas — c'est `metadata::tests` qui tient l'idempotence, non le test qui porte son nom · `default_features` vaut `true` par défaut dans le schéma, un fragment muet obtenant ce que fait `cargo add` ; seule la valeur `false` s'écrit, sous la clé Cargo `default-features` · la déclaration reste une chaîne nue tant qu'il n'y a rien d'autre à dire qu'une version, et la conversion chaîne → table inline est extraite en `etaler`, partagée avec `ajouter_feature_a_dependance` · les `[[dependances]]` sont patchées **avant** les `[cargo.<crate>]`

- [x] **K3** · Ancres `state_champs` et `state_init` — vérifié 2026-08-27 · `cargo test -p rbs-cli --test integration_add` → 11 passed, dont `les_deux_ancres_d_etat_recoivent_le_contenu_declare` et `une_ancre_d_etat_absente_arrete_l_installation_sans_rien_ecrire`, ce dernier nommant `<rbs:state_champs>` et `src/state.rs` sur stderr, portant le bloc à coller sur stdout et laissant le projet intact · `cargo test -p rbs-cli --test integration_new -- --ignored` → 1 passed, le test **étendu au niveau `I1`** : `clippy --workspace --all-targets -- -D warnings` puis `rustfmt --edition 2024 --check` sur `src/main.rs` et `migration/src/lib.rs` · `cargo test -p rbs-cli --test integration_examples` → 12 passed après régénération du squelette et des deux exemples · deux morsures : `STATE_INIT` visant `state_champs` → 4 FAILED dont le test des deux ancres ; indentation de `state.rs.jinja` désalignée → `Diff in …/src/state.rs:4`, **ce qui prouve que le `rustfmt` lancé depuis `main.rs` traverse le fichier que ce lot modifie** — `mod state;` y est déclaré ligne 4 · `AppState::new` rend `anyhow::Result<Self>` et **reste synchrone** : le squelette dépend déjà d'`anyhow` et son `main` remonte la panne par un `?` · `state_init` est posée **dans le littéral `Ok(Self { … })`** et non dans un bloc de statements : un champ s'y nomme une fois au lieu de deux, un fragment qui a besoin de plusieurs lignes appelant un constructeur de son module · **hors critère** : sept pages FR et EN annonçaient « cinq ancres », que `doctor` contredisait dès ce commit — corrigées dans les deux langues, sortie de `doctor` recapturée (`les 7 points d'insertion sont en place`), et `npm run clear && npm run build` → deux `[SUCCESS]`, ce que l'agent avait laissé en réserve · workspace après intégration : 544 passed, 0 failed, `clippy --all-features -D warnings` et `fmt --all --check` propres

### Lot L — `rbs add redis`

Dépose `src/cache/` et non `src/redis/` : le squelette insère `mod redis;` en tête de
`main.rs`, où `use redis::Client` deviendrait ambigu avec la crate du même nom (E0659).
Le module porte ce qu'il fait, non la techno qui le sert.

- [x] **L1** · Manifeste, section `[cache]`, pool dans l'état — vérifié 2026-08-27 · `rbs new` puis `rbs add redis` → clippy `--workspace --all-targets -- -D warnings` Finished et `rustfmt --check` propre · relu sur le projet réel : `src/state.rs` porte `pub cache: crate::cache::Cache,` dans `<rbs:state_champs>` et `cache: crate::cache::Cache::depuis_config()?,` dans `<rbs:state_init>`, `Cargo.toml` porte `redis = { version = "1.6", features = ["tokio-comp"] }` et `deadpool-redis = "0.23"`, `config/default.toml` se termine par `[cache]` · `cargo test -p rbs-cli --test integration_add` → 13 passed, dont `installer_redis_deux_fois_n_ecrit_rien_la_seconde` ; à la main, le second `add` rend `✓ redis est déjà installée — rien à faire`, code 0, `git status` vide · morsure de l'orchestrateur : section `[[config]]` retirée du manifeste de fragment → `le_fragment_redis_ecrit_les_ancres_d_etat_les_dependances_et_la_section_cache` FAILED · **morsure la plus instructive de l'agent** : le court-circuit `[package.metadata.rbs]` neutralisé **seul** laisse le test vert, et la déduplication d'`inserer` neutralisée seule aussi — il faut les deux pour le faire tomber : l'inertie du second `add` ne tient pas au court-circuit mais à l'idempotence propre de chaque patch · **troisième dépendance hors backlog** : `serde_json 1.0`, « sérialisation par serde » exigeant un format que `redis 1.6` n'offre pas (son `json` vise RedisJSON, module serveur) ; elle paraît en `[dependencies]` et en `[dev-dependencies]` du projet, ce que cargo accepte, la redondance se levant en `O` · **écart d'outillage** : `cargo add --dry-run` lancé depuis le dépôt rabat sur `redis 1.2.2` pour tenir la `rust-version 1.85` de `rbs-cli` ; sans effet, ces crates n'étant jamais ses dépendances et le projet généré ne déclarant pas de MSRV

- [x] **L2** · Le cache typé — vérifié 2026-08-28 · les trois critères traversent désormais `Cache::set`, `Cache::get` et `Cache::invalider_prefixe` contre un `redis:8-alpine` réel, ce que la note du 2026-08-27 attendait de `L3` · `cargo test -p rbs-cli --test integration_redis -- --ignored` → 1 passed, les trois tests serveur exécutés · morsure `get` rendant toujours `None` → les trois FAILED, dont l'aller-retour typé et la clé absente ; morsure du `retain` d'`a_supprimer` **et** de l'échappement de `motif` → le préfixe FAILED (`left: 2, right: 1`), chacune retirée seule laissant le test vert : les deux gardes visent la même faute et se rattrapent, ce qu'aucun test hors serveur ne pouvait montrer

- [x] **L3** · `integration_redis` sous conteneur — vérifié 2026-08-28 · `cargo test -p rbs-cli --test integration_redis -- --ignored` → 1 passed, les trois tests serveur du fragment exécutés contre `redis:8-alpine` sous `GenericImage`, aucune dépendance de développement ajoutée · **faux vert refermé** : `cargo test -- --ignored` sort en 0 même quand il ne filtre aucun test — le test assert que chacun des trois a bien tourné · morsure `set_ex` remplacé par `set` → le seul test de TTL FAILED, « la valeur devait avoir expiré côté serveur » ; morsure `get` rendant toujours `None` → les trois FAILED, le TTL sur son assertion de présence, qui existe pour ça · les tests sont **livrés** dans tout projet qui fait `rbs add redis` : `cargo test` du projet → 4 passed, 3 ignored · `[cargo.tokio] features = ["time"]` déclarée plutôt que prise par unification depuis `redis`/`tokio-comp` · `cargo test --workspace` → 528 passed, total de `main` inchangé ; clippy et fmt propres sur le dépôt et sur un projet réel doté du fragment

### Lot M — `rbs add mail`

- [x] **M1** · Manifeste, section `[mail]`, transport dans l'état — vérifié 2026-08-27 · `rbs new` puis `rbs add mail` → clippy `--workspace --all-targets -- -D warnings` Finished et `rustfmt --check` propre · secret relu sur le projet réel : `.env.example` porte `RBS_MAIL__SMTP_PASSWORD=` et `grep -rn` sur `config/` ne rend **aucune** clé de configuration le portant · `cargo test -p rbs-cli --lib` → 420 passed, dont `le_mot_de_passe_smtp_est_dans_l_environnement_et_dans_aucune_configuration`, qui planifie un vrai `add mail` et lit les fichiers **projetés**, non le manifeste de fragment · morsures : `smtp_password` ajouté à la section `[mail]` du fragment → FAILED « config/default.toml porte le secret en clé de configuration » ; bloc `[[env]]` retiré → FAILED « .env.example absent du plan » · le test ignore les lignes de commentaire, celui de `[mail]` renvoyant exprès vers la variable — sans quoi il tombait sur son propre commentaire · **écart au backlog** : la liste de features de `lettre` qu'il fige **ne compile pas** — `rustls` réclame en plus un fournisseur de chiffrement et une source de certificats ; `ring` et `webpki-roots` ajoutés, seuls choix ne demandant rien au système, donc les seuls fidèles au motif du design (éviter OpenSSL sur les trois plateformes)

- [x] **M2** · Gabarits et rendu — vérifié 2026-08-27 · `cargo test` du projet généré → 6 passed, dont `le_gabarit_rendu_porte_les_variables_qui_lui_sont_passees`, `un_gabarit_introuvable_nomme_le_fichier_sans_paniquer` et `envoyer_detache_rend_la_main_sans_attendre_l_envoi` · **faux vert évité, et c'est le point du lot** : le message natif de minijinja est `template "absent.html" does not exist` — il nomme le *gabarit*, jamais le *fichier* ; le test assert sur `templates/mail/absent.html`, chaîne que seul le code du fragment produit · morsure relancée par l'orchestrateur : `tokio::spawn` remplacé par un `block_on` → `envoyer_detache a attendu 1.003189417s`, FAILED, les 5 autres verts · la preuve d'`envoyer_detache` tient sur un `TcpListener` de boucle locale qui accepte et ne répond **jamais** : l'appel doit rendre la main sous 200 ms, **puis** un `mpsc` atteste que la connexion est arrivée quand même — un corps vide échoue à la seconde assertion, une attente à la première · `minijinja 2.24` avec la feature `loader`, absente des défauts, dont `path_loader` dépend

- [x] **M3** · `integration_mail` sous Mailpit — vérifié 2026-08-28 · `cargo test -p rbs-cli --test integration_mail -- --ignored` → 1 passed : le message relu par l'API porte `ada@example.org`, « Bienvenue chez nous » et un corps **rendu par le gabarit** · **partage des rôles inverse de `L3`** : le fragment livre le parcours d'envoi, le dépôt seul relit l'API — aucun projet n'a de raison d'hériter d'un test qui interroge un serveur de développement · l'API se lit sur un `TcpStream` en **HTTP/1.0**, sans dépendance de développement ni binaire externe : le serveur Go de Mailpit annonce alors la fin par la fermeture de la connexion, au lieu du chunked qu'il faudrait décoder · **constat sur le gabarit** : l'URL n'est pas cherchée entière dans le corps reçu, l'autoéchappement de minijinja rendant ses `/` en `&#x2f;` — ce qu'on attend d'un gabarit HTML dont le contexte peut venir d'une entrée utilisateur ; l'hôte suffit, il n'apparaît nulle part ailleurs · quatre morsures, chacune sur son assertion : corps littéral au lieu du gabarit → seul le corps FAILED, destinataire et sujet passant ; sujet altéré → `left: "Autre sujet"` ; envoi retiré → `messages_count` à 0 contre 1 ; nom du test d'envoi changé → « n'a pas été exécuté » alors que le `cargo test` interne rend « ok », ce qui prouve le garde-fou contre un `-- --ignored` sortant en 0 sur zéro test filtré · `cargo test` du projet généré → 6 passed, 1 ignored · `cargo test --workspace` → 528 passed, total de `main` inchangé ; clippy et fmt propres sur le dépôt et sur un projet réel doté du fragment

### Lot N — `rbs add storage`

Un trait `Storage` d'une quinzaine de lignes, deux implémentations. `object_store` le
fournirait tout fait ; il est écarté parce que ce trait a vocation à être lu et remplacé
par l'auteur du projet, et qu'une crate tierce le lui retirerait.

- [x] **N1** · Trait `Storage` et backend fichiers — vérifié 2026-08-27 · `rbs new` puis `rbs add storage` → `cargo clippy --workspace --all-targets -- -D warnings` Finished et `rustfmt --edition 2024 --check` propre sur `src/main.rs` et `migration/src/lib.rs` · `cargo test` du projet généré → `le_backend_fichiers_depose_lit_atteste_puis_supprime` et `une_cle_remontant_hors_de_la_racine_est_refusee` ok · le test de traversée éprouve quatre clés (`../vole.txt`, `../../vole.txt`, `sous/../../../vole.txt`, un chemin absolu), assertion sur la variante `CleRefusee` **et** sur l'absence des fichiers témoins hors racine, et vérifie que `sous/../recu.txt` reste acceptée — le refus porte sur l'évasion, non sur la présence d'un `..` · morsure de sécurité relancée par l'orchestrateur : `self.racine.join(normaliser(cle)?)` ramené à `self.racine.join(cle)` → le test de traversée FAILED et **lui seul** (3 passed / 1 failed) · le trait est dyn-compatible via `async-trait`, `AppState` portant `Arc<dyn Storage>` : le rendre générique aurait contaminé toute signature de handler · `normaliser` sert aux **deux** backends et `supprimer` est idempotent des deux côtés, deux conditions de la substituabilité qu'exigera `N3` · **deux dépendances hors backlog** : `async-trait 0.1` (sans quoi le trait n'est pas dyn-compatible) et `thiserror 2.0`, plus `[cargo.tokio] features = ["fs"]`

- [x] **N2** · Backend S3 — vérifié 2026-08-27 · `cargo test` du projet généré → `le_backend_s3_se_construit_sans_joindre_le_reseau` et `un_backend_inconnu_echoue_en_nommant_les_valeurs_admises` ok (4 passed) · **preuve d'absence de réseau à trois couches** : le test est un `#[test]` sans boucle Tokio (`Handle::try_current().is_err()` asserté, donc le SDK ne peut attendre aucune réponse), l'endpoint est `http://127.0.0.1:1` avec des identifiants faux, et la construction doit tenir en 20 ms — mesurée `118,9 ms` à la première puis `0,78 / 0,63 / 0,63`, les 119 ms étant l'initialisation paresseuse du client HTTPS et non du réseau, ce qui a fait porter la mesure sur la seconde · morsures : un `head_bucket().send()` bloquant ajouté au constructeur → le test tombe et la suite passe de 0,11 s à 2,06 s ; le bras d'erreur remplacé par un repli silencieux sur `fs` → le second test tombe · le message des valeurs admises est **écrit à la main** et non délégué à une énumération serde, dont le message nommerait déjà les variantes et ne prouverait rien du fragment · **`aws-config` n'a pas été nécessaire** : `Credentials`, `Region` et `BehaviorVersion` sont réexportés par `aws_sdk_s3::config`, et le fournisseur explicite dispense de la chaîne par défaut — c'est ce qui garde la construction synchrone, `aws-config` imposant un `await` donc un `AppState::new` async, contre §2.4

- [x] **N3** · `integration_storage` sous MinIO — vérifié 2026-08-28 · `cargo test -p rbs-cli --test integration_storage -- --ignored` → 1 passed, les deux tests du fragment exécutés contre MinIO · **le premier critère ne demandait aucune ligne neuve** : `ronde(&dyn Storage)` existe depuis `N1`, annotée « rejouable telle quelle contre S3 » — elle est appelée sans une ligne de différence, un jeu réécrit pour S3 n'aurait prouvé que S3 · **morsure décisive** : la ronde mordue dans un projet généré, lancée en `-- --include-ignored`, fait tomber `le_backend_fichiers_depose_lit_atteste_puis_supprime` et `le_backend_s3_passe_la_meme_ronde_que_le_backend_fichiers` **ensemble**, les quatre autres restant verts : c'est là que se prouve le partage du jeu · la relecture hors du trait bâtit son propre `aws_sdk_s3::Client`, le champ de `StockageS3` étant privé au module · morsure `lire` rendant un contenu constant → la ronde FAILED et la relecture hors trait **passe**, son indépendance étant ainsi observée et non supposée ; morsure `deposer` sans effet → les deux FAILED, le dépôt traversant bien MinIO ; morsure `existe` toujours `true` → la seule ronde FAILED · le bucket est créé par le conteneur (`mkdir -p /data/demo && minio server /data`, un répertoire de premier niveau de `/data` étant un bucket) : **aucun test livré à l'utilisateur ne crée de ressource** · `cargo test` du projet généré → 4 passed, 2 ignored · `cargo test --workspace` → 528 passed, total de `main` inchangé ; clippy et fmt propres sur le dépôt et sur un projet réel doté du fragment

### Lot O — Documentation et sortie du jalon

- [x] **O1** · Exemple compilé en CI — vérifié 2026-08-28 · `examples/file-drop` : `rbs new`, les trois features, puis un CRUD `uploads` · **le nom de la ressource n'est pas libre** — l'ancre `features` empile les `mod` dans l'ordre d'installation et doit rester triée, or `rbs add redis` inscrit `mod cache;` : `uploads` est ce qui la ferme derrière `storage` · `cargo test -p rbs-cli --test integration_examples` → 14 passed, dont `file_drop_is_what_the_cli_produces_today` et `the_hand_edits_of_file_drop_are_in_place` · la boucle `for exemple in examples/*/` de `ci.yml` jouée en local → les trois exemples à 0, sans modification du workflow (la boucle plutôt qu'un step par exemple était faite pour ça) · **les trois `#![allow(dead_code)]` de module retirés**, ce qui fait de `clippy -D warnings` la preuve du câblage : un appel oublié ne compile plus · trois morsures, chacune sur son assertion : une invalidation sur trois retirée → « toutes trois invalider » FAILED ; `#![allow(dead_code)]` remis sur le cache → « la permission de module tombe » FAILED ; `href="{{ lien }}"` → « le href doit porter la variable » FAILED · le README rejoué à la lettre dans un répertoire vierge reproduit l'exemple, aux deux fichiers de migration près, identiques à l'horodatage · **deux découvertes** : `engendrer` ne commitait qu'une fois avant la première feature, ce qu'aucun exemple à une seule feature ne pouvait montrer ; et le total est mis en cache plutôt que la page, `Page` n'étant que `Serialize` — la relire du cache exigerait de toucher au noyau, que `K` seul peut faire
      **Premier critère réécrit le 2026-08-28, sur arbitrage.** Il exigeait les trois
      plateformes ; `ci.yml` ne compile les exemples que sous Linux, et l'argumente :
      « les contrôles de dépôt — format, exemples, dry-run — ne dépendent pas de la
      plateforme ». `hello-crud` et `blog-auth` sont dans ce cas depuis toujours. C'est
      un changement de critère, non une preuve, et il a été demandé.
      Un exemple portant les trois features. S'il s'avère illisible, il se scinde :
      l'installation isolée est de toute façon prouvée par `L3`, `M3` et `N3`.
      ✓ Compilé par le step `examples/` de la CI.
      ✓ `integration_examples` le compare à une génération fraîche.

- [x] **O2** · Pages de documentation FR et EN — vérifié 2026-08-28 · trois guides — `cache`, `mail`, `storage` — plutôt qu'une page commune, chacun sur le plan d'`auth.md` · `npm run parite` → **18 paires, 18/18 au même dernier commit**, 41 liens relatifs résolus, 0 écart structurel · l'instrument éprouvé **avant** de servir de preuve, par cinq morsures rendant chacune son propre diagnostic : titre retiré du FR, méta de bloc changée, encart retiré, lien détourné, page FR absente · **25 blocs sur les trois pages, dont 21 portant `file=examples/file-drop/…`** — zéro extrait écrit à la main ; les 4 restants sont des sorties de `rbs add` et de `doctor` capturées sur le binaire, non du code du projet · `npm run clear && npm run build` → deux `[SUCCESS]`, et c'est lui qui tient le second critère : morsures `region=to_deletee` → « La région « to_deletee » est introuvable » et backend fichiers renommé → « introuvable. Le fichier a-t-il été déplacé ou l'exemple régénéré ? » · **la contrainte n'était pas rédactionnelle** : `edite_a_la_main` est la liste des fichiers *exclus* de la comparaison de non-dérive, donc les régions ne sont posées que là, les courts non édités cités entiers, et les quatre longs — trois suites de tests et le backend S3 — décrits en prose, comme `auth.md` le fait de `src/auth/tests.rs` ; `cargo test -p rbs-cli --test integration_examples` → 14 passed, aucun exemple n'a dérivé · `rustfmt --edition 2024 --check` propre sur les six fichiers touchés, ayant attrapé un `endregion` glissé avant l'accolade fermante de `to_delete` · `write-translations --locale fr` → 89 entrées, 0 vide, seule reparaissant la dérive de champs `description` que `V2` avait déjà constatée · **deux corrections hors critères** : `cli/add.md` annonçait trois features quand le binaire en livre six — page, `--help` et message de refus remis sur la sortie réelle — et `scripts/parite.mjs` fige l'instrument que `V2` et `J3` avaient dû réécrire chacune de leur côté
      ✓ Parité stricte FR/EN mesurée comme en `V2` et `J3`.
      ✓ Aucun extrait de code non issu de l'exemple d'`O1`.

- [x] **O3** · `doctor` diagnostique les trois features — vérifié 2026-08-28 · `cargo test -p rbs-cli --lib doctor::` → 62 passed, dont 16 neufs, et `cargo test --workspace` → 546 contre 530 sur `main`, soit ces 16 et rien d'autre · les trois critères relus sur la **sortie réelle** du binaire dans un projet doté par `rbs add redis`, `rbs add mail` et `rbs add storage` : `✗ redis  config/default.toml ne porte pas de section [cache]`, `✗ mail  RBS_MAIL__SMTP_PASSWORD n'est renseignée ni dans le .env ni dans l'environnement`, `✗ storage  backend = "s3" sans bucket : ni config/default.toml ni RBS_STORAGE__BUCKET n'en nomment un` · **le titre est `redis` et non `cache`**, sur arbitrage : le critère nomme la section et le module, le manifeste déclare la crate, et le rapport porte le nom déclaré comme `auth` le fait · **constat hors critères** : la variable du courriel étant déjà posée vide par le fragment, `env` rend `✓ les 8 variables sont renseignées` là où `mail` voit `✗ smtp_user vaut envoi@exemple.fr et RBS_MAIL__SMTP_PASSWORD est vide` — c'est le couple qui se diagnostique, la variable seule étant du ressort d'`env` · deuxième constat hors critères, calqué sur celui de `J4` : les identifiants S3 restés à la valeur de `.env.example`, comparés à ce fichier et non à une chaîne du CLI · le backend `fs` ne réclame rien, tout le contrôle du stockage ne vaut que pour `s3` · cinq morsures, une par constat : la section réputée toujours présente fait tomber les deux tests de `redis` et eux seuls, les quatre autres un test chacune · `section` et `field` remontées dans `mod.rs`, `auth::auth_section` en était le premier exemplaire ; la cascade `config/<env>.toml` reste hors de portée, le CLI ne sachant pas quel `RBS_ENV` sera employé · clippy `-D warnings` et fmt propres
      Leçon de `J4` : un utilisateur bloqué lance `doctor`, la commande doit lui apprendre
      ce qui le bloque. Le contrôle n'entre au rapport que si la feature est déclarée dans
      `[package.metadata.rbs]`.
      ✓ `cache` déclarée sans section `[cache]` → `✗`.
      ✓ `mail` déclarée et `RBS_MAIL__SMTP_PASSWORD` absente → `✗` nommant la variable.
      ✓ `storage` en `backend = "s3"` sans bucket → `✗`.

- [x] **O4** · Critère de sortie du jalon — vérifié 2026-08-28 · `git diff --stat d29b311..HEAD -- crates/rbs-core/` → **0 ligne**, et `git log d29b311..HEAD -- crates/rbs-core/` → **aucun commit** : aucun lot d'intégration n'a touché le noyau · second critère prouvé **deux fois** — un projet fraîchement engendré des trois features (`rbs new` puis `add redis`, `add mail`, `add storage`) → `cargo fmt --check` propre, `clippy --all-targets -- -D warnings` Finished, `cargo test` → 14 passed / 6 ignored ; et `examples/file-drop`, où les trois briques sont réellement appelées, contre **PostgreSQL 18 et Redis réels** → 17 passed / 6 ignored, dont `the_full_lifecycle_goes_through_the_api` qui traverse le cache, le stockage et le courriel · **le second critère a trouvé ce qu'il était fait pour trouver** : `the_s3_backend_builds_without_touching_the_network` cherchait `StockageS3` dans la représentation de debug quand la migration a renommé le type `S3Storage` — le fragment livrait à tout utilisateur de `rbs add storage` un test qui tombe au premier `cargo test`, passé au travers parce que la CI compile les exemples sans lancer leurs tests, qui demandent une base · corrigé dans le fragment **et** dans l'exemple, qui doivent rester identiques : `integration_examples` → 14 passed et `cargo test --workspace` → 546 passed, total inchangé · morsure faisant construire le backend fichiers sous `backend = "s3"` → le seul test corrigé FAILED, les treize autres verts · `uuidv7()` de la migration engendrée exige **PostgreSQL 18** : un 17 fait échouer `rbs migrate up` avant tout test
      **Premier critère réécrit le 2026-08-28, sur arbitrage.** Il prenait pour repère la
      fin de `K`, d'où `git diff` rend 18 fichiers et 666 lignes : la migration des
      identifiants vers l'anglais, postérieure à `K`, a touché tout le noyau. Elle
      n'appartient à aucun lot et l'encadré de tête l'acte déjà. Le repère est donc le
      commit de cette migration, `d29b311`, et ce que le critère voulait établir — aucun
      lot d'intégration n'a touché le noyau — reste mesuré. C'est un changement de critère,
      non une preuve, et il a été demandé.
      ✓ `git diff --stat d29b311..HEAD -- crates/rbs-core/` → 0 ligne.
      ✓ Les trois features installées sur un même projet cohabitent : clippy, fmt et
      `cargo test` du projet généré passent.

---

## 🧰 v0.4 — Confort

**Détaillé le 2026-08-28. Conception et décisions :**
[`docs/superpowers/specs/2026-08-28-v0.4-confort-design.md`](docs/superpowers/specs/2026-08-28-v0.4-confort-design.md).

> **Le support de MySQL et SQLite reste dans ce jalon, en dernier lot**, sur arbitrage du
> mainteneur : il aurait pu devenir un jalon v0.5, intercalé avant le gel de l'API publique
> de `rbs-core` que v1.0 promet — geler `ConnectError`, dont le message nomme PostgreSQL,
> graverait le couplage dans le contrat de compatibilité. La conséquence est assumée et
> bornée : `R` pose du SQL que `S` devra porter, ce que `R3` amortit en isolant le dépilage
> dans une fonction unique dès le premier jour.

Ordre : `(P ∥ Q ∥ R) → S → T`. Les trois premiers lots sont indépendants — trois chantiers
sans fichier de fond commun, qui se rejoignent sur l'enum `Commands` de `cli.rs` à deux
lignes chacun. **Le goulot est entier à la vérification**, comme en v0.3 : cible de
compilation partagée, verrou de cargo, un conteneur par test ; les `--ignored` se passent
en fin de parcours, une par une.

`S` est **le seul lot autorisé à modifier `rbs-core`** — dispositif repris de `K`, et c'est
lui qui rend le critère de sortie mesurable, par les deux diffs de `T4`.

### Lot P — Seeds

- [x] **P1** · Commande `rbs seed` — vérifié 2026-08-28 · `cargo test -p rbs-cli --lib seed::` → 20 passed et `cargo test -p rbs-cli --test integration_seed` → 3 passed, dont `under_production_the_command_refuses_without_launching_the_project_binary` — code non nul, stderr nommant `--force` et `production`, et **`projet/target` absent** : cargo n'a pas tourné · `a_project_without_seeds_says_how_to_create_one` nomme `src/seeds/main.rs` et donne le bloc `[[bin]]`, là où cargo aurait rendu une erreur de manifeste · **le garde-fou vit dans la commande** et non dans le seed, qui est fait pour être modifié · morsure `if false && !force && production(…)` → les 3 tests de production tombent, eux seuls ; morsure `ensure_binary` rendant toujours `Ok` → l'intégration passe de 0,7 s à **7,3 s**, cargo ayant enfin tourné : c'est la durée qui atteste que le binaire du projet n'est pas lancé
      Enveloppe un binaire du projet, sur le motif de `rbs migrate` (`migrate/mod.rs:163`) :
      le CLI ne parle jamais à la base et ne gagne aucun client SQL. Le garde-fou de
      production vit dans la commande et non dans le code généré — un seed est fait pour
      être modifié, le refus ne doit pas pouvoir être retiré par mégarde.
      ✓ Sous `RBS_ENV=production` → refus nommant `--force`, code non nul, et le binaire
      du projet n'est **pas** lancé.
      ✓ Projet sans `src/seeds/` → message disant comment en créer un, non une erreur de cargo.

- [x] **P2** · `src/seeds/` et son binaire au squelette — vérifié 2026-08-28 · `integration_seed::on_a_fresh_project_the_command_says_there_is_nothing_to_insert` → exit 0, message « rien à insérer », pas de `target/` ; `cargo test -p rbs-cli --test integration_new -- --ignored` → 1 passed en **62,84 s**, qui joue `build`, `test`, `clippy --workspace --all-targets -- -D warnings` et `rustfmt --check` sur `src/main.rs`, **`src/seeds/main.rs`** et `migration/src/lib.rs` · **le binaire des seeds est une seconde racine de crate** (`[[bin]]` plus `default-run`, deux binaires rendant `cargo run` ambigu) et non un module : donner une lib au projet aurait déplacé l'ancre `<rbs:features>` hors de `src/main.rs`, et avec elle tout le squelette, les trois exemples et la documentation · **une seule ancre grâce à `macro_rules!`** — un `mod` non inline ne s'écrit pas dans un bloc, ce qui vaut deux ancres à `migration` · morsure `fn jamais_appelee() {}` déposée dans le seed d'un projet généré → `clippy -D warnings` échoue, prouvant que clippy inspecte bien `src/seeds/<feature>.rs` ; morsure `SEEDS` retirée d'`ANCRES` → seul `the_seeds_anchor_is_one_of_those_counted`, test ajouté pour cela — sans lui rien ne tombait · **neuvième point d'insertion et non huitième** : `R` a posé `startup` dans le même jalon, `ANCRES` passe de 7 à 9 — conflit d'intégration résolu en gardant les deux, et la documentation des deux langues portée à neuf
      Un module et son binaire, derrière une ancre `<rbs:seeds>` — huitième point
      d'insertion, que `doctor` compte.
      ✓ `rbs new` puis `rbs seed` sur un projet vierge → exit 0, message disant qu'il n'y a
      rien à insérer.
      ✓ `clippy --workspace --all-targets -- -D warnings` et `rustfmt --check` propres sur
      le projet généré.

- [x] **P3** · `rbs generate crud` dépose le seed de son entité — vérifié 2026-08-28 · `cargo test -p rbs-cli --lib generate::command` → 20 passed, dont `a_crud_drops_its_seed_and_declares_it_in_the_anchor` (`src/seeds/articles.rs` présent, corps d'ancre `== "articles,"`) et `two_generations_leave_two_seeds_and_an_orderly_anchor` ; `generate::seed::tests::the_seeded_rows_come_back_from_the_api -- --ignored` → 1 passed en **29,30 s** contre PostgreSQL 18 : `generate crud` → `migrate up` → `seed`, puis `GET /semis` rend les deux lignes · **« une ancre toujours triée » lu comme « ordonnée, une entrée par ligne », sur arbitrage demandé** : le cas discriminant — générer `notes` *puis* `articles` — rend `notes, articles,`, donc l'ordre de génération, qui est aussi celui des migrations et le seul sûr le jour où un seed dépendra d'un autre ; **aucune ancre du CLI n'a jamais été triée**, `anchors::insert` apposant avant la balise fermante, et son commentaire pose que le contenu appartient au développeur · morsures `mount::for_seed` non appelé, puis fichier seed non écrit → les deux tests d'ancre tombent chaque fois
      Du Rust typé passant par l'entité générée : un champ renommé casse à la compilation,
      et non en silence à l'exécution. C'est ce qui met ce lot hors d'atteinte de `S`.
      ✓ Le fichier existe et l'ancre `<rbs:seeds>` porte son appel.
      ✓ `rbs seed` puis `GET /<entité>` rend les lignes insérées.
      ✓ Deux `generate crud` successifs → deux seeds, une ancre toujours triée.

### Lot Q — `rbs dev`

- [x] **Q1** · Orchestration du démarrage — vérifié 2026-08-28 · `cargo test -p rbs-cli --lib dev::tests::without_the_docker_feature_no_compose_is_looked_for` → 1 passed, la sonde d'existence de fichier étant **injectée et jamais appelée** : le test constate l'absence d'appel, non un plan vide, et vérifie que `Step::Server` est là quand même ; `cargo test -p rbs-cli --test integration_dev` → 2 passed, la base injoignable rendant code 1, un message nommant `127.0.0.1` et le port, et l'assertion explicite `!sortie.contains("panicked")` · la feature `docker` du test est posée par le vrai chemin `add::plan_for` puis `apply`, donc le nom du compose vient du fragment et non d'une constante recopiée · **patience à deux vitesses, non dictée par le backlog** : 30 s quand `rbs dev` vient de remonter le compose, 3 s quand la base était censée déjà tourner — trente secondes de silence pour apprendre qu'on a oublié PostgreSQL sont trente secondes perdues · morsure conditions inversées → seul le test du compose tombe ; morsure message amputé de `{host}:{port}` → seul celui de l'injoignabilité
      Compose remonté si `docker` est déclarée dans `[package.metadata.rbs]`, attente de la
      base, `migrate up`, puis le serveur. C'est le démarrage en une commande, qui est la
      moitié de la valeur de `rbs dev`.
      ✓ Projet sans la feature `docker` → aucun compose cherché, démarrage quand même.
      ✓ Base injoignable → message nommant ce qui manque, non une trace de panique.

- [x] **Q2** · Le watch, `watchexec 8.4` — vérifié 2026-08-28 · `cargo test -p rbs-cli --lib dev::watch` → 5 passed, dont `target_is_not_even_watched` qui écarte `target/` **à la source** par `Filterer::check_dir` et non au tri des événements, et le cas `target/debug/build/…/out/genere.rs` qui est celui qui boucle ; morsure `SpawnOptions { grouped: false }` → « le petit-fils a survécu à la coupure : 60674 n'est pas libre » · **les trois plateformes tranchées par le run CI 33188943379**, où `the_child_server_dies_with_its_group_and_frees_the_port ... ok` paraît dans les trois jobs — `fmt · clippy · test · intégration` (ubuntu), `windows-latest` et `macos-latest` : le test est normal, sans `#[ignore]`, sans Docker et sans `#[cfg]` de plateforme, donc joué tel quel partout · **le push a révélé une régression étrangère au lot** : `generate::format::a_badly_formatted_source_is_straightened` tombait sur `windows-latest` seul, `newline_style` valant « Auto » dans rustfmt — corrigé à part, la morsure `newline_style=Windows` reproduisant l'échec du runner à la ligne près
      Le point dur n'est ni le debounce ni le filtrage, tous deux faciles, mais la coupure
      du serveur enfant : un `cargo run` tué sans son enfant laisse le port occupé, et le
      geste diffère sur les trois plateformes.
      ✓ Fichier de `src/` touché → redémarrage ; fichier de `target/` touché → rien.
      ✓ Le serveur enfant meurt avec son groupe : le port est libre au redémarrage suivant,
      vérifié sur les trois plateformes de la CI.

### Lot R — Jobs en arrière-plan

Une table, et non Redis : le manifeste de fragment n'a aucun champ pour exiger une autre
feature, et un job poussé dans Redis survivrait au rollback de la transaction qui le
motivait. `jobs` est un fragment, `rbs add jobs`, comme `redis`, `mail` et `storage`.

- [x] **R1** · Manifeste du fragment, table et section `[jobs]` — vérifié 2026-08-28 · `rbs new` puis `rbs add jobs` (8 fichiers) → `cargo fmt --all --check` exit 0 et `cargo clippy --workspace --all-targets -- -D warnings` Finished **sur le projet engendré** ; `rbs migrate up` puis `\d jobs` contre PostgreSQL 18 → `status`, `attempts`, `available_at`, `payload`, plus `last_error`, `created_at`, `updated_at`, et l'index `idx_jobs_status_available_at (status, available_at)` ; second `rbs add jobs` → « ✓ jobs est déjà installée — rien à faire », **empreinte du projet identique** (`d5b5044d64231292` avant et après) · **colonnes en anglais là où le critère écrivait `disponible_a`** : le glossaire du dépôt et les tables `users`/`refresh_tokens` de la même base l'imposent, comme `O3` avait tranché `redis` contre `cache` · une table plutôt que Redis, le manifeste de fragment n'ayant aucun champ pour exiger une autre feature
      `serde_json` monte de `[dev-dependencies]` en dépendance de production pour le payload.
      ✓ `rbs new` puis `rbs add jobs` → clippy et fmt propres sur le projet généré.
      ✓ `rbs migrate up` crée la table avec statut, tentatives, `disponible_a` et payload.
      ✓ `rbs add jobs` deux fois → rien écrit la seconde fois.

- [x] **R2** · Enfilage typé et atomicité avec le métier — vérifié 2026-08-28 · les deux tests vivent **dans le projet engendré** et sont joués par `cargo test -p rbs-cli --test integration_jobs -- --ignored` → 2 passed en 38,07 s · ce test **exige nommément** que `jobs::tests::a_job_enqueued_in_a_rolled_back_transaction_does_not_exist` et `…_in_a_committed_transaction_is_visible_to_the_worker` paraissent en `... ok` : sans ces quatre lignes, un fragment cessant de livrer ses tests le laisserait au vert, `cargo test -- --ignored` sortant en 0 **même quand il ne filtre aucun test** · morsure enfiler sur `db` au lieu de `&transaction` → seul le test de rollback tombe, les trois autres `ok` — c'est ce qui distingue la table d'une file Redis, qui survivrait au rollback qui la motivait
      Le seul critère qui justifie d'avoir choisi la base contre Redis. S'il ne passe pas,
      le support n'a pas d'intérêt sur une file en mémoire.
      ✓ Un job enfilé dans une transaction **annulée** n'existe pas après le rollback.
      ✓ Un job enfilé dans une transaction committée est visible du worker.

- [x] **R3** · Le worker : réservation, réessai, échec définitif — vérifié 2026-08-28 · mêmes deux tests exigés nommément par `integration_jobs` en `... ok` : `two_concurrent_workers_never_reserve_the_same_job` (200 jobs, 8 workers) et `a_failing_job_is_retried_then_marked_failed_after_the_last_attempt` · `grep -rn "FOR UPDATE SKIP LOCKED" crates/rbs-cli/templates/features/jobs/` → **un seul fichier**, `queue.rs.jinja`, où `reserver_prochain_job` est défini une fois, ses six autres occurrences étant des appels ; `templates::tests::the_dequeue_appears_in_a_single_place_of_the_jobs_fragment` fige cette unicité, dont dépend `S3` · morsure clause `SKIP LOCKED` retirée → « 214 job(s) réservé(s) deux fois » ; morsure `UPDATE … RETURNING` remplacé par un `SELECT` puis un `UPDATE` → « 856 » · **découverte : à 40 jobs et 4 workers, la morsure passait** — le test ne mordait pas ; le test livré porte 200/8
      Le dépilage est isolé dans `reserver_prochain_job` **dès maintenant**, pour que `S3`
      n'ait qu'un corps de fonction à trois branches à écrire au lieu d'une chasse à la
      requête. Le nombre de tentatives et l'attente entre deux viennent de `[jobs]`.
      ✓ Deux workers concurrents ne réservent jamais le même job.
      ✓ Un job qui échoue est réessayé, puis marqué en échec après N tentatives.
      ✓ Le dépilage n'apparaît qu'à un seul endroit du fragment, mesuré au `grep`.

- [x] **R4** · `integration_jobs` sous conteneur — la survie au redémarrage — vérifié 2026-08-28 · `cargo test -p rbs-cli --test integration_jobs -- --ignored --test-threads=1` → **2 passed en 38,07 s**, dont `a_job_enqueued_before_the_process_is_killed_runs_after_the_restart` · **l'intervalle de scrutation sert de pince** : 3600 s pour le premier processus, qui ne verra donc jamais le job de son vivant, 1 s pour le second, qui le dépile — sans elle le premier l'exécuterait aussitôt et le test ne prouverait rien de sa survie ; le statut est constaté `pending` avant et après la mise à mort · morsure file passée en mémoire du processus (`Mutex<Vec<Model>>`) → « le job n'a pas survécu au redémarrage — statut "pending" », ce qui sépare ce jalon du `tokio::spawn` détaché de `M2`
      C'est ce qui distingue ce jalon du `tokio::spawn` détaché de `M2`, et cela se prouve
      plutôt que cela ne s'affirme.
      ✓ Processus tué entre l'enfilage et l'exécution, puis relancé → le job s'exécute.
      ✓ Les tests livrés au projet tournent : `cargo test` du projet généré.

### Lot S — Portabilité MySQL et SQLite

**Seul lot du jalon autorisé à modifier `rbs-core`.**

- [x] **S1** · `rbs new --database postgres|mysql|sqlite` — vérifié 2026-08-28 · `cargo test -p rbs-cli --test integration_new -- --ignored` → 2 passed : `the_generated_project_compiles_and_passes_its_tests` (PostgreSQL, build + test + clippy + rustfmt) et `each_engine_produces_a_project_that_compiles` (MySQL puis SQLite, `cargo build --workspace`, 76,94 s) ; `rbs new --database oracle` → « invalid value 'oracle' for `--database <MOTEUR>` [possible values: postgres, mysql, sqlite] », aucun répertoire écrit ; `without_the_flag_the_manifest_stays_on_postgres` et `a_manifest_without_a_database_key_reads_back_as_postgres` → un manifeste sans la clé reste un projet PostgreSQL · **une cible de compilation par moteur**, sur arbitrage : les trois activent des features `sea-orm` différentes, et une cible commune ferait recompiler `sea-orm` et `sqlx` à chaque bascule, y compris pour les tests d'intégration qui n'ont rien demandé · **la contradiction `--database mysql` avec une URL `postgres://` est un refus**, levé dans la phase de vérification donc avant le premier rendu — c'est la faute que `T3` fera diagnostiquer après coup · **découverte : `rbs dev` échouait en `UrlIllisible` sur SQLite**, dont l'URL n'a ni hôte ni port ; l'attente de la base est désormais conditionnée par `a_un_serveur` · les fixtures de rendu des tests recopiaient le contexte de `new::render` et d'`add::plan_for`, et leur dérive a fait tomber cinq tests étrangers au lot — le test qui promettait « les cinq variables » ne promet plus de décompte · `cargo test --workspace --all-features -- --ignored` → 30 passed, dont `integration_add`, `integration_dev` et `integration_jobs`, qui dépendent du compose et du contexte de `add`
      Manifestes, `.env.example`, compose et configuration suivent la valeur choisie.
      ✓ Les trois valeurs produisent un projet qui compile.
      ✓ Une valeur inconnue → refus nommant les trois admises.
      ✓ Sans le flag, `postgres` reste le défaut : aucun projet existant ne change.

- [x] **S2** · Identifiant v7 posé par l'application — vérifié 2026-08-28 · `cargo test --workspace --all-features -- --ignored` → **30 passed, 0 failed, exit 0**, dont `a_generated_crud_migrates_and_passes_its_tests_against_postgresql` **sur PostgreSQL 17**, l'image des trois tests sous conteneur étant descendue de 18 à 17 — c'est la façon la moins contestable de prouver que l'exigence de la 18 est tombée ; `integration_crud` **exige nommément** que `articles::tests::two_creations_in_a_row_carry_increasing_ids` paraisse en `... ok` dans le projet engendré, faute de quoi un gabarit cessant de livrer ce test laisserait la suite au vert ; `integration_examples` → 14 passed, les trois exemples régénérés · **un seul point d'écriture par entité** : tous les chemins d'insertion du projet passent par `..Default::default()` — service d'un CRUD, seed, `enfiler`, création d'un utilisateur et d'un refresh token — et `DeriveEntityModel` fait déléguer `Default::default()` à `ActiveModelBehavior::new()`, vérifié dans la macro et non sur la foi de son commentaire ; `new()` est synchrone, donc pas d'`async_trait` dans un fichier fait pour être lu, et `update`, qui passe par `From<Model>`, garde son `id` en `Unchanged` · **la monotonie devient garantie par processus** là où celle de PostgreSQL l'était par serveur, `Uuid::now_v7()` partageant un contexte — noté dans le commentaire du modèle · **plancher de `doctor` à PostgreSQL 14** et non 17, sur arbitrage : 14 est la plus ancienne version encore maintenue, quand 17 n'aurait été justifié que par le test qui le cite · **deux régressions révélées par le conteneur, absentes de la suite rapide** : le test des identifiants croissants écrivait en parallèle du cycle de vie et lui volait la première place de la liste — l'assertion porte désormais sur l'ordre de la page, non sur une position ; et l'insert brut de `integration_jobs`, qui enfilait sans `id`, ne tenait que par le défaut de colonne
      Le gabarit de migration perd `Expr::cust("uuidv7()")`, qui n'a pas d'équivalent à
      écrire en MySQL ni en SQLite. `uuid` monte en dépendance de production avec la feature
      `v7` ; le commentaire du squelette qui explique pourquoi elle était en dev se réécrit.
      ✓ Deux entités créées à la suite ont des identifiants **croissants** — ce qu'un `v4`
      ne donnerait pas, et ce qu'un test vérifiant seulement la présence d'un UUID ne
      prouverait pas.
      ✓ `rbs migrate up` passe sur **PostgreSQL 17** : l'exigence 18 relevée par `V1` tombe.
      ✓ Les trois exemples régénérés, `integration_examples` vert.

- [x] **S3** · `reserver_prochain_job` à trois branches — vérifié 2026-08-28 · `cargo test -p rbs-cli --test integration_jobs -- --ignored` → **3 passed, exit 0**, dont `the_dequeue_never_hands_the_same_job_twice_on_the_three_engines` qui engendre un projet par moteur, chacun dans **sa cible de compilation**, et exige nommément `jobs::tests::two_concurrent_workers_never_reserve_the_same_job ... ok` (200 jobs, 8 workers) · **les trois branches vivent dans la fonction**, triées à l'exécution sur `db.get_database_backend()` : un branchement de gabarit obligerait `rbs add jobs` à connaître le moteur et rendrait faux le test d'unicité posé par `R3` · morsure **commit de la transaction MySQL juste après l'élection** → « 213 job(s) réservé(s) deux fois », **sur MySQL seul** ; morsure **requête PostgreSQL donnée à SQLite** → « near "FOR": syntax error », **sur SQLite seul** · **morsure `SKIP LOCKED` retiré : ne mord pas** — `FOR UPDATE` fait attendre le second worker au lieu de lui donner la ligne, et les deux assertions du test restent vraies ; la clause est ici de performance, non de correction, contrairement à ce que la ligne de `R3` laisse entendre · **deux défauts antérieurs mis au jour par le portage** : l'instant de comparaison venait de `now()` de la base quand `available_at` est posé par l'application, d'où une instabilité une exécution sur deux sur PostgreSQL — il est désormais lié en paramètre, ce qui ôte des trois requêtes toute fonction de date propre à un moteur ; et MySQL **arrondit** un `timestamp` sans précision, plaçant `available_at` jusqu'à une demi-seconde dans le futur — les écritures tronquent à la seconde · **MySQL tourne en CI comme les autres `#[ignore]`, sur arbitrage** : le job Linux joue déjà `cargo test -- --ignored`, et un test qui ne tourne jamais pourrit
      `FOR UPDATE SKIP LOCKED` en PostgreSQL et MySQL 8 ; SQLite n'en a pas besoin, n'ayant
      qu'un seul écrivain — une transaction immédiate y suffit.
      ✓ Le test de concurrence de `R3` passe sur les trois bases.
      ✓ La CI joue le bout-en-bout sur PostgreSQL et SQLite ; MySQL reçoit un `#[ignore]`
      sous conteneur, lancé à la main comme `L3`, `M3` et `N3`.

- [ ] **S4** · Le noyau cesse de nommer PostgreSQL
      `ConnectError` est un type public dont le message dit aujourd'hui « vérifiez que le
      serveur PostgreSQL est démarré », sur un projet qui peut désormais tourner sur trois
      moteurs.
      ✓ Le message nomme le moteur réellement configuré.
      ✓ `doctor` ne suppose plus PostgreSQL.

### Lot T — Documentation et sortie du jalon

- [ ] **T1** · Exemple `newsletter-queue` compilé en CI
      Les seeds peuplent les abonnés, les jobs envoient. S'il faut y ajouter `mail` pour
      qu'il soit parlant, cela se tranche sur pièce — au prix d'un conteneur de plus en CI.
      ✓ Compilé par le step `examples/` de la CI.
      ✓ `integration_examples` le compare à une génération fraîche.

- [ ] **T2** · Pages de documentation FR et EN
      ✓ Parité stricte FR/EN mesurée comme en `V2`, `J3` et `O2`.
      ✓ Aucun extrait de code non issu de `newsletter-queue`.
      ✓ La page de `mail` dit ce qu'`envoyer_detache` ne garantit pas et montre le passage
      à un job — la fonction est conservée, le fragment devant rester utilisable seul.

- [ ] **T3** · `doctor` diagnostique les jobs et la base
      ✓ `jobs` déclarée sans section `[jobs]` → `✗`.
      ✓ URL `mysql://` avec `sqlx-postgres` au manifeste → `✗` nommant l'écart.

- [ ] **T4** · Critère de sortie du jalon
      Le `ROADMAP` n'en énonce pas pour v0.4 ; celui-ci est proposé par la conception §2.9.
      ✓ `rbs new --database` pour les trois valeurs, puis `cargo test` → vert sur chacun.
      ✓ Un job survit au redémarrage du processus, rejoué sur le projet livré.
      ✓ `git diff --stat` de `crates/rbs-core/` → 0 ligne sur `P`, `Q`, `R` réunis, et
      0 ligne après `S` : seul `S` a ouvert le noyau.

---

## ⏳ Jalons suivants

Volontairement en grosses mailles. Détailler ces tâches aujourd'hui serait de la
fiction : elles seront réécrites avec ce que les jalons précédents auront appris.

### v1.0 — Stabilité
- [ ] Gel de l'API publique de `rbs-core`
- [ ] Publication sur crates.io
- [ ] CHANGELOG et engagement semver
- [ ] `rbs upgrade` — migration d'un projet d'une version de rbs à la suivante
