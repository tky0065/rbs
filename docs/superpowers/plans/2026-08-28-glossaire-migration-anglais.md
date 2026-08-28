# Glossaire de la migration des identifiants vers l'anglais

Correspondance unique, appliquée partout. Une racine traduite deux fois différemment
selon le fichier produit exactement le dépôt bâtard que cette migration existe pour
supprimer.

**Périmètre.** Tout identifiant Rust : fonctions, méthodes, types, variantes, champs,
constantes, noms de tests. **Hors périmètre** : commentaires, doc-comments, messages
d'erreur et sortie du CLI (du texte destiné à l'utilisateur francophone, non des
identifiants), messages de commit, et la prose des pages de documentation.

---

## 1. API livrée à l'utilisateur

Ce que `rbs add` écrit dans le projet de l'utilisateur, et ce que `rbs-core` expose.
Ces noms-ci sont la raison d'être de la migration : ce sont les seuls qu'un utilisateur
de rbs lit sans avoir choisi de contribuer au dépôt.

### Trait `Storage` et ses backends

| Français | Anglais |
|---|---|
| `deposer` | `put` |
| `lire` | `get` |
| `supprimer` | `delete` |
| `existe` | `exists` |
| `normaliser` | `normalize` |
| `racine` / `racine_par_defaut` | `root` / `default_root` |
| `depuis_config` | `from_config` |
| `StockageFichiers` | `FileStorage` |
| `StockageS3` | `S3Storage` |
| `StorageError::Introuvable` | `StorageError::NotFound` |
| `StorageError::CleRefusee` | `StorageError::RejectedKey` |
| `StorageError::Indisponible` | `StorageError::Unavailable` |
| `backend_par_defaut` | `default_backend` |

`get` plutôt que `read` pour faire pendant à `put`, et parce que le trait est un magasin
d'objets, non un système de fichiers.

### `Mailer` et gabarits

| Français | Anglais |
|---|---|
| `envoyer` | `send` |
| `envoyer_gabarit` | `send_template` |
| `envoyer_detache` | `send_detached` |
| `message` | `message` (inchangé) |
| `Gabarits` | `Templates` |
| `src/mail/gabarit.rs` | `src/mail/template.rs` |
| `src/storage/fichiers.rs` | `src/storage/files.rs` |
| clé `[mail].gabarits` | `[mail].templates` |
| clé `[storage].racine` | `[storage].root` (répertoire `./storage`) |
| `gabarits` (champ) | `templates` |
| `rendre` | `render` |
| `nouveau` / `nouveaux` | `new` |
| `interne` (conversion d'erreur) | `internal` |
| `url_par_defaut` | `default_url` |

### `Cache`

| Français | Anglais |
|---|---|
| `invalider` | `invalidate` |
| `invalider_prefixe` | `invalidate_prefix` |
| `set` / `get` / `set_ttl` | inchangés |
| `ttl_par_defaut` | `default_ttl` |
| `connexion` | `connection` |
| `motif` (glob) | `pattern` |

### Auth

| Français | Anglais |
|---|---|
| `ouvrir_session` | `login` |
| `ouvrir_session_admin` | `admin_login` |
| `deconnecter` | `logout` |
| `rafraichir` | `refresh` |
| `consommer` | `consume` |
| `emettre` | `issue` |
| `encoder` / `decoder` | `encode` / `decode` |
| `hacher` | `hash_password` |
| `verifier` (mot de passe) | `verify_password` |
| `verifier` (JWT) | `verify` |
| `signer` | `sign` |
| `empreinte` | `fingerprint` |
| `aleatoire` | `random` |
| `Profil` | `Profile` |
| `ligne_de_session` | `session_row` |
| `inscrire` | `register` |
| `RequireRole`, `Role`, `Claims` | inchangés |

### `rbs-core` — divers publics

| Français | Anglais |
|---|---|
| `Sante` / `sante` | `Health` / `health` |
| `Statut` | `Status` |
| `ErreurJwt` | `JwtError` |
| `ReponsesCommunes` | `CommonResponses` |
| `section` | `section` (inchangé) |
| `charger` | `load` |
| `construire` | `build` |
| `connecter` | `connect` |

---

## 2. Interne `rbs-cli`

Invisible pour l'utilisateur de rbs, visible pour qui contribue.

| Français | Anglais |
|---|---|
| `Ancre` / `ancre` | `Anchor` / `anchor` |
| `Plan` / `planifier` / `Planifiee` | `Plan` / `plan` / `Planned` |
| `Etat` / `Etats` | `State` / `States` |
| `Controle` / `Controles` / `controler` | `Check` / `Checks` / `check` |
| `Rapport` / `rapport_de` | `Report` / `report_of` |
| `remede` | `remedy` |
| `Echec` | `Failed` |
| `Manifeste` / `lire_manifeste` | `Manifest` / `read_manifest` |
| `Noyau` / `noyau` / `noyau_local` | `Core` / `core` / `local_core` |
| `Projet` / `projet` / `creer_projet` | `Project` / `project` / `create_project` |
| `Fichier` / `fichiers` | `File` / `files` |
| `Fragment` / `fragment_a_code` | `Fragment` / `fragment_has_code` |
| `Champ` / `champs` / `TypeChamp` | `Field` / `fields` / `FieldType` |
| `Sortie` | `Output` |
| `Journal` | `Log` |
| `Erreur*` (préfixe) | `*Error` (suffixe, idiome Rust) |
| `Dependance` / `DependanceDeclaree` | `Dependency` / `DeclaredDependency` |
| `MigrationDeclaree`, `SectionDeclaree`, … | `DeclaredMigration`, `DeclaredSection`, … |
| `Metadonnees` | `Metadata` |
| `Espion` (test double) | `Spy` |
| `Tampon` | `Buffer` |
| `engendrer` | `generate` |
| `verifier_non_derive` | `assert_no_drift` |
| `masquer_horodatage` / `masquer_chemin_du_noyau` | `mask_timestamp` / `mask_core_path` |
| `commiter` | `commit` |
| `depot` | `repo` |
| `cible` | `target` |
| `empreinte` (arborescence) | `fingerprint` |
| `est_marqueur` | `is_marker` |
| `edite_a_la_main` | `hand_edited` |
| `en_snake_case` / `en_pascal_case` | `to_snake_case` / `to_pascal_case` |
| `au_singulier` | `to_singular` |
| `horodatage_courant` | `current_timestamp` |
| `poser_*` | `write_*` |
| `inserer` | `insert` |
| `patcher` | `patch` |
| `expurger` | `strip` |
| `deguillemeter` | `unquote` |
| `relever` | `collect` |
| `salir` (working tree) | `dirty` |
| `reussi` | `succeeded` |
| `diagnostiquer` | `diagnose` |
| `variables_du_projet` | `project_variables` |

## 3. Règle pour les noms de tests

~400 noms de tests sont des phrases françaises. Ils ne se traduisent pas mot à mot :
la phrase anglaise se réécrit, en gardant **ce que le test affirme**, jamais sa
tournure.

| Français | Anglais |
|---|---|
| `le_cycle_de_vie_complet_passe_par_l_api` | `the_full_lifecycle_goes_through_the_api` |
| `une_cle_remontant_hors_de_la_racine_est_refusee` | `a_key_escaping_the_root_is_rejected` |
| `un_identifiant_inconnu_rend_404` | `an_unknown_id_returns_404` |
| `hello_crud_est_celui_que_le_cli_produit_aujourd_hui` | `hello_crud_is_what_the_cli_produces_today` |

Le nom reste une phrase — c'est la convention du dépôt, et elle vaut dans les deux
langues. Un test renommé `test_storage_1` serait une régression.

## 4. Faux amis — ne pas toucher

Identiques dans les deux langues, ou déjà anglais. Un `sed` sur ces racines casserait
du code juste :

`config`, `section`, `message`, `format`, `service`, `page`, `port`, `version`,
`migration`, `template` (dans `templates/`), `document`, `plan`, `role`, `token`,
`cache`, `mail`, `storage`, `router`, `routes`, `handler`, `middleware`, `init`,
`load`, `new`, `from`, `get`, `set`, `list`, `find`, `create`, `update`, `delete`,
`up`, `down`, `main`, `scope`, `offset`, `per_page`, `current`.

---

## 5. Ce que la migration a appris

Consigné après coup, parce que rien de tout cela ne se voyait avant de le rencontrer.

- **Trois régimes de remplacement, pas un.** Le code se traduit intégralement ; un
  commentaire seulement entre backticks ; une chaîne littérale seulement dans ses
  interpolations `{…}`. Sans cette distinction, « le nom n'est pas rendu » devient « le
  nom n'est pas rendered ». Le premier outil ne faisait pas la troisième distinction : la
  couche des templates a dû être rejouée depuis un dépôt propre.
- **Un traitement ligne à ligne rate les chaînes multi-lignes.** Les messages écrits sur
  plusieurs lignes avec `\` échappent au régime « chaîne » et repassent en régime
  « code ». Ils se retrouvent à la relecture, par un grep cherchant un mot anglais entouré
  de français.
- **serde nomme des fonctions par des chaînes.** `#[serde(default = "url_par_defaut")]`
  est un identifiant malgré ses guillemets. Le protéger comme une chaîne casse la
  compilation des projets générés — et seule une génération réelle le montre.
- **Les variables de rendu vivent des deux côtés.** Ce que `Feature` et `Field`
  sérialisent à la main pour minijinja, et ce que les templates lisent. Renommer un seul
  des deux rend la variable indéfinie, ce que ni `cargo build` ni `cargo test --lib` ne
  révèlent.
- **Les clés d'un `feature.toml` ne sont pas du code.** Les renommer avec les
  identifiants Rust casse le parsing ; elles ne suivent que lorsque leurs structures
  serde bougent, et alors les six fragments doivent suivre ensemble.
- **`integration_examples` ne protège pas les marqueurs `// region:`** : il les filtre
  par construction. Un exemple aligné par copie depuis un projet généré les perd sans que
  rien n'échoue — jusqu'à ce que le site de documentation ne trouve plus ses extraits.
  C'est le seul dégât que la suite de tests n'a pas signalé.
