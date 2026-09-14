# `--json` sur les plans et les erreurs — plan d'implémentation

> **Pour les agents :** sous-skill requis : `superpowers:subagent-driven-development`.
> Les étapes se suivent en cases `- [ ]`.

**But :** sous `--json`, `add`, `generate crud|feature|client` et `upgrade` n'écrivent sur
la sortie standard qu'un document JSON — leur plan, contenu complet des fichiers créés
compris, ou leur erreur avec un code stable — pour qu'un agent cesse de parser de l'ANSI.

**Architecture :** une vue dédiée `plan/json.rs`, sur le modèle de `doctor/json.rs` : des
structures `Serialize` propres à la sortie, construites depuis `Plan`, sans aucun
`Serialize` sur les types internes. `Plan::actions()` sort de `#[cfg(test)]` : c'est la
vue JSON qui la lit. Les erreurs gagnent un trait `errors::Codee` (`code`, `remede`,
`bloc`), implémenté par chaque énumération d'erreur des quatre commandes ; les erreurs
partagées (`plan::Error`, `plan::application::Error`) portent leur propre `code()`/`bloc()`
auxquels les autres délèguent. `lib.rs` choisit le rendu : texte inchangé, ou JSON.

**Spec :** `docs/superpowers/specs/2026-09-13-lot-features-p2-design.md`, section « 40 ».

## Contraintes globales

- Document (clés en français, comme `doctor --json`) :
  `{ "commande", "racine", "applique", "actions": [{ "chemin", "statut", "effet": { "type", … } }],
  "sautees": [{ "fichier", "ancre", "bloc" }], "fichiers": { "crees", "modifies" } }`.
- `applique` = `false` sous `--dry-run` (et quand rien n'a été écrit), `true` après
  application. **Contenu complet** des fichiers créés.
- `statut` : `a_faire` | `deja_fait` | `conflit`. Une forme par variante d'`Effect`
  (`plan/action.rs:20-80`) : `creer {contenu}`, `inserer {ancre, lignes}`,
  `reposer_ancre {ancre}`, `patcher_toml {patch}` (`inscrire_feature {feature}`,
  `ajouter_dependance {nom, version, features, features_par_defaut}`,
  `ajouter_feature_a_dependance {dependance, feature}`, `aligner_sur_version {dependance, version}`),
  `ajouter_section {section, contenu}`, `ajouter_variable {cle, valeur, commentaire|null}`,
  `remplacer_zone {zone, contenu}`.
- Erreurs sous `--json` : `{ "erreur": { "code", "message", "remede": "…"|null, "bloc": "…"|null } }`
  sur la sortie standard, **code de sortie inchangé** (1). Codes snake_case stables,
  `match` exhaustif (sans `_`) pour qu'une variante ajoutée ne compile pas sans son code.
- Sous `--json`, la sortie standard ne porte **qu'un** document ; tout message humain va
  sur stderr ou se tait. Le rendu humain sans `--json` ne change pas d'un caractère.
- Pas de `generate job` (absent de la branche).
- Doc bilingue ; l'exemple de document est tiré d'une exécution réelle.

## Table des codes

| Code | Erreur |
|---|---|
| `pas_un_projet` | `PasUnProjet` de chaque commande |
| `arbre_sale` | `WorkingTreeSale` |
| `fichier_inaccessible` | `Acces`, `plan::Error::Acces` |
| `manifeste_illisible` | `Metadata`, `plan::Error::Metadata` |
| `ancre_absente` | `plan::Error::Anchor` — `bloc` = `anchor.block()` |
| `ancre_mal_placee` | `plan::Error::MalPlacee` — `bloc` = le bloc à remonter |
| `fichier_absent` | `plan::Error::FichierAbsent` |
| `manifeste_absent` | `plan::Error::ManifesteAbsent` |
| `toml_invalide` | `plan::Error::Toml` |
| `zone_absente` | `plan::Error::ZoneAbsente` — `bloc` = `zone.block()` |
| `plan_incoherent` | `plan::Error::DejaProjete` |
| `conflit` | `application::Error::Conflit` |
| `ecriture_impossible` | `application::Error::Ecriture` |
| `feature_inconnue`, `fragment_sans_manifeste`, `fragment_invalide`, `template_absente`, `ancre_inconnue`, `rendu_impossible`, `env_illisible`, `url_indecomposable` | `add` |
| `nom_invalide`, `champs_invalides`, `feature_deja_presente`, `rendu_impossible`, `relation_invalide`, `migration_absente`, `homonyme`, `feature_absente`, `role_sans_auth`, `role_inconnu`, `upload_sans_storage`, `storage_hors_modules`, `colonne_reservee`, `enfant_sans_cle`, … | `generate crud|feature` (une par variante) |
| `sans_bibliotheque`, `sans_binaire_openapi`, `cargo_introuvable`, `projet_ne_compile_pas` | `generate client` — `client::Error::Openapi(openapi::Obtention::…)` depuis la factorisation OpenAPI |
| `document_illisible`, `client_irrendable` | `generate client` — `Document`, `Rendu` |
| `cli_anterieur`, `agents_illisible` | `upgrade` |

---

### Tâche 1 : la vue `plan/json.rs`

**Fichiers :** Créer `crates/rbs-cli/src/plan/json.rs` ; modifier `crates/rbs-cli/src/plan/mod.rs`
(`pub(crate) mod json;`, `actions()` hors `cfg(test)`, champ `actions` sans `expect(dead_code)`).

**Interfaces :** `pub(crate) fn plan(commande: &str, plan: &Plan, applique: bool) -> String` ;
`pub(crate) fn erreur(code: &str, message: &str, remede: Option<&str>, bloc: Option<&str>) -> String`.

- [ ] Tests rouges, un par variante d'`Effect` et de `PatchToml` (plan construit par
  `Builder` sur un répertoire temporaire, ou `Plan` littéral dans le module de tests),
  plus : `statut` des trois valeurs, `sautees` (fichier, ancre, bloc = lignes jointes par
  `\n`), `fichiers` (créés = `before` absent, modifiés = `before` présent, `deja_fait`
  exclus), `racine`, `applique`, et `erreur` avec `remede`/`bloc` à `null`.
- [ ] Implémenter (`serde` `tag = "type"`, `rename_all = "snake_case"`,
  `serde_json::to_string_pretty(..).expect(..)` comme `doctor/json.rs`).
- [ ] `cargo test -p rbs-cli --lib`, clippy, fmt ; commit
  `feat(plan): rend un plan et une erreur en JSON`.

### Tâche 2 : codes d'erreur

**Fichiers :** `crates/rbs-cli/src/errors.rs` (trait `Codee { fn code(&self) -> &'static str;
fn remede(&self) -> Option<String>; fn bloc(&self) -> Option<String>; }`),
`plan/mod.rs` (`Error::code`, `Error::bloc`), `plan/application.rs` (`Error::code`),
`add/mod.rs` (+ `add/installation.rs`), `generate/command.rs`, `client/mod.rs`,
`upgrade.rs`.

- [ ] Tests rouges : `plan::Error::Anchor` → `ancre_absente` et `bloc` = `anchor.block()` ;
  `add::Error` d'ancre absente (par `Installation(Plan(Anchor))`) → idem ;
  `generate::command::Error::Plan(Anchor)` → idem ; `WorkingTreeSale` → `arbre_sale` ;
  `client::Error::Openapi(Obtention::SansBinaire)` → `sans_binaire_openapi` ; `upgrade::Error::PasUnProjet` → `pas_un_projet` ; tous les
  codes rendus sont en snake_case ASCII.
- [ ] Implémenter, `match` exhaustifs ; `remede` délègue aux `remedy()` existants
  (`None` pour `upgrade`, qui n'en a pas).

### Tâche 3 : drapeaux et branchement

**Fichiers :** `crates/rbs-cli/src/cli.rs`, `crates/rbs-cli/src/lib.rs`, tests
`crates/rbs-cli/tests/integration_json.rs`.

- [ ] `#[arg(long)] json: bool` — « Rend le plan, ou l'erreur, en un document JSON sur la
  sortie standard. » — sur `Add`, `Crud`, `Feature`, `Client`, `Upgrade` ; tests de parsing.
- [ ] `lib.rs` : `commande` = `add`, `generate crud`, `generate feature`, `generate client`,
  `upgrade`. Sous `--json` : ni plan texte ni lignes de succès ni conseils ; les
  avertissements (zone d'`AGENTS.md` manquante et son bloc, CRUD laissés ouverts par
  `auth`, notes de migration d'`upgrade`, référence requise) passent sur stderr ; à la fin,
  `plan::json::plan(commande, &plan, applique)`. Un projet déjà à jour / feature déjà
  installée rend un document à `actions` vides et `applique: false`. Erreurs :
  `plan::json::erreur(code, message, remede, bloc)` sur stdout, exit 1.
- [ ] Tests `assert_cmd` (sans Docker, sans `#[ignore]`) : `rbs new demo --yes`, puis
  `rbs add cors --dry-run --json` → la sortie standard **entière** s'analyse par
  `serde_json`, `commande == "add"`, `applique == false`, une action `creer` à `contenu`
  non vide, au moins une `inserer` sur l'ancre `layers` ; le projet est intact ;
  une ancre `layers` retirée de `src/router.rs` puis `rbs add cors --json --force` →
  code 1, stdout = `{"erreur": {"code": "ancre_absente", "bloc": "// <rbs:layers>\n// </rbs:layers>", …}}` ;
  `rbs generate crud articles --fields title:string --dry-run --json --force` et
  `rbs upgrade --dry-run --json` → document analysable ; `rbs add cors --json` hors
  projet → `pas_un_projet`.
- [ ] `cargo test --workspace` (dont `integration_docs` rapide), clippy, fmt ; commit
  `feat(cli): ajoute --json aux plans d'add, generate et upgrade et à leurs erreurs`.

### Tâche 4 : documentation

- [ ] `cli/add.md`, `cli/generate.md`, `cli/upgrade.md` : section `--json` (EN+FR) ;
  `guides/agents.md` (EN+FR) : le document, la table des codes, un exemple tiré d'une
  exécution réelle (`rbs add cors --dry-run --json`, tronqué avec mention).
- [ ] `cli/generate.md` (EN+FR) : les blocs `rbs:transcript` de `rbs generate crud --help` et `rbs generate feature --help` gagnent la ligne `--json`, recopiée de la sortie réelle — `integration_docs` les compare au caractère ; `cargo test -p rbs-cli --test integration_docs --no-fail-fast` vert.
- [ ] `cd docs && npm run build` → 0 ; commit `docs(cli): documente la sortie --json des plans`.
