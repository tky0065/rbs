# Contrôles `doctor` des sept fragments sans contrôle — plan d'implémentation

> **Pour les agents :** sous-skill requis : `superpowers:executing-plans` (ou
> `superpowers:subagent-driven-development`). Les étapes se cochent (`- [ ]`).

**But :** `rbs doctor` juge `cors`, `rate-limit`, `scheduler`, `webhooks`, `audit`, `docker`
et `ci` sur un projet qui les déclare, chacun par un module sur le modèle de `doctor/mail.rs`.

**Architecture :** un module par contrôle sous `crates/rbs-cli/src/doctor/`, chacun exposant
`TITRE` et `check`, inscrit dans `FEATURE_CHECKS` (`doctor/mod.rs`) sous le nom de la feature.
Le contrôle `scheduler` s'appuie sur un validateur partagé, `crates/rbs-cli/src/cron.rs`, qui
reprend la normalisation du fragment et la crate `cron` à la version que le fragment déclare ;
la commande `rbs generate job` le réemploiera.

**Pile :** Rust 2024, `toml_edit`, `thiserror`, crate `cron` 0.17, `tempfile` en test.

**Spec :** `docs/superpowers/specs/2026-09-13-lot-features-p2-design.md`, section « 37 ».

## Contraintes globales

- `cron = "0.17"` : la version que déclare `templates/features/scheduler/feature.toml:49-51`,
  inscrite à `[workspace.dependencies]` et reprise par `rbs-cli` en `cron.workspace = true`.
- Normalisation identique à `normaliser` de `templates/features/scheduler/mod.rs.jinja` :
  5 champs → `0 ` en tête, 6 tels quels, tout autre nombre refusé avec le même message.
- `cors` : section absente ✗, `origins` vide ⚠ (`Check::warned`) — c'est le défaut sûr.
- `ci` vérifie `.github/workflows/ci.yml`, et non `config/production.toml` (écart assumé de
  la spec avec le backlog).
- Le détail d'un constat tient sur une ligne (`doctor::une_ligne`) ; le remède peut en porter
  plusieurs.
- `rbs add` ne rejoue pas une feature déjà installée (`add/mod.rs:229-243`) : un fichier de
  fragment disparu se restaure depuis Git, et le remède le dit.
- Commits : Conventional Commits en français, sans identifiant de tâche, sans mention du
  backlog ni d'un outil, sans `Co-Authored-By` ni `Claude-Session` ; corps avec le pourquoi
  et `Vérifications :`.
- Bloquants : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`.

---

## Fichiers

| Fichier | Rôle |
|---|---|
| `Cargo.toml` (racine) | `cron = "0.17"` dans `[workspace.dependencies]` |
| `crates/rbs-cli/Cargo.toml` | `cron.workspace = true` |
| `crates/rbs-cli/src/lib.rs` | `mod cron;` |
| `crates/rbs-cli/src/cron.rs` | créé : `valider`, `Erreur`, tests croisés avec le fragment |
| `crates/rbs-cli/src/doctor/mod.rs` | `pub mod` des sept, `FEATURE_CHECKS` 7 → 14, `Config::array_len`, aide de test `reglages_du_fragment` |
| `crates/rbs-cli/src/doctor/{cors,rate_limit,scheduler,webhooks,audit,docker,ci}.rs` | créés, un contrôle chacun |
| `docs/docs/cli/doctor.md` + `docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/doctor.md` | section « Installed features » |

---

### Tâche 1 : validateur cron partagé et contrôle `scheduler`

Ensemble : `valider` sans appelant serait du code mort sous `clippy -D warnings`.

**Fichiers :** créer `src/cron.rs`, `src/doctor/scheduler.rs` ; modifier `Cargo.toml`,
`crates/rbs-cli/Cargo.toml`, `src/lib.rs`, `src/doctor/mod.rs`.

**Interfaces :**
- Produit : `crate::cron::valider(expression: &str) -> Result<String, crate::cron::Erreur>`
  (forme normalisée à six champs) ; `Erreur::{NombreDeChamps { expression, champs }, Illisible { expression, cause }}`, `Display` = message du fragment.
- Produit : `doctor::scheduler::{TITRE, check(root: &Path) -> Check}`.

- [ ] **Étape 1 : dépendance.** `cron = "0.17"` sous `[workspace.dependencies]` (ordre
  alphabétique), `cron.workspace = true` dans `crates/rbs-cli/Cargo.toml`, `mod cron;` dans
  `src/lib.rs` (ordre alphabétique). `cargo build -p rbs-cli` résout.

- [ ] **Étape 2 : tests de `cron.rs` (rouges).**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const FRAGMENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/features/scheduler");

    #[test]
    fn five_fields_gain_the_zero_second_the_crate_expects() {
        assert_eq!(valider("0 3 * * *").as_deref(), Ok("0 0 3 * * *"));
    }

    #[test]
    fn six_fields_pass_through() {
        assert_eq!(valider("0 0 3 * * *").as_deref(), Ok("0 0 3 * * *"));
    }

    #[test]
    fn another_field_count_is_refused_naming_the_expression() {
        let erreur = valider("0 3 * *").expect_err("quatre champs");
        assert!(matches!(erreur, Erreur::NombreDeChamps { champs: 4, .. }));
        assert!(erreur.to_string().contains("`0 3 * *`"), "{erreur}");
    }

    #[test]
    fn an_out_of_range_value_is_refused_by_the_crate() {
        assert!(matches!(valider("0 99 * * *"), Err(Erreur::Illisible { .. })));
    }

    /// Le message du nombre de champs est celui que le démarrage rend : lu dans la
    /// template, continuation de ligne ôtée.
    #[test]
    fn the_field_count_message_is_the_one_the_fragment_prints() { /* extraire le littéral
        qui suit `autre => anyhow::bail!(` dans mod.rs.jinja, recoller les morceaux coupés
        par `\` + saut + indentation, remplacer `{expression}` par `0 3 * *` et `{autre}`
        par `4`, comparer à `valider("0 3 * *").unwrap_err().to_string()` */ }

    /// Les expressions que les tests du fragment tiennent pour valides le sont ici, et
    /// réciproquement : `normaliser("…").expect(` et `normaliser("…").expect_err(`.
    #[test]
    fn the_expressions_the_fragment_tests_judge_are_judged_alike() { /* au moins quatre
        expressions trouvées, sinon le test ne prouve rien */ }

    /// Même crate, même version : la déclaration du fragment et celle du CLI coïncident.
    #[test]
    fn the_cli_parses_with_the_version_the_fragment_declares() { /* `[[dependencies]]`
        name = "cron" de feature.toml contre `workspace.dependencies.cron` du Cargo.toml racine */ }
}
```

Lancer `cargo test -p rbs-cli --lib cron::` : échec de compilation (`valider` absent).

- [ ] **Étape 3 : `cron.rs`.**

```rust
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Erreur {
    #[error(
        "`{expression}` porte {champs} champ(s) : une expression cron en compte cinq \
         (minute heure jour mois jour-de-semaine) ou six, la seconde en tête"
    )]
    NombreDeChamps { expression: String, champs: usize },
    #[error("`{expression}` : {cause}")]
    Illisible { expression: String, cause: String },
}

pub(crate) fn valider(expression: &str) -> Result<String, Erreur> {
    let normalisee = match expression.split_whitespace().count() {
        5 => format!("0 {expression}"),
        6 => expression.to_string(),
        champs => return Err(Erreur::NombreDeChamps { expression: expression.to_string(), champs }),
    };
    ::cron::Schedule::from_str(&normalisee).map_err(|cause| Erreur::Illisible {
        expression: expression.to_string(),
        cause: cause.to_string(),
    })?;
    Ok(normalisee)
}
```

`::cron::` : le module du CLI porte le nom de la crate. Doc-comments sur le module, l'enum,
chaque champ et la fonction. Lancer `cargo test -p rbs-cli --lib cron::` : vert.

- [ ] **Étape 4 : tests de `doctor/scheduler.rs` (rouges).** Dans un `TempDir` avec
  `src/modules/scheduler/mod.rs` :
  - la template du fragment copiée telle quelle (elle ne porte aucune directive) → `Bon` ;
  - `"0 3 * * *"` remplacée par `"0 99 * * *"` → `Echec`, détail portant `0 99 * * *` ;
  - un fichier absent → `Echec`, détail portant `src/modules/scheduler/mod.rs` ;
  - une expression en commentaire (`// Schedule::every::<X>("n'importe quoi", …)`) → ignorée ;
  - l'expression sur la ligne qui suit l'appel (forme que rustfmt produit) → lue ;
  - un type à chevrons (`Schedule::every::<Enveloppe<Job>>("0 99 * * *", …)`) → lu, `Echec` ;
  - deux fautes → les deux nommées, séparées par ` ; `, sur une ligne.

- [ ] **Étape 5 : `doctor/scheduler.rs`.** `TITRE = "scheduler"`, `FICHIER =
  "src/modules/scheduler/mod.rs"`. Lecture : `NotFound` → échec « {FICHIER} est absent »,
  remède « restaurez-le depuis Git (`git checkout -- {FICHIER}`) : `rbs add` ne rejoue pas
  une feature déjà installée » ; autre erreur → échec « {FICHIER} est inaccessible : … ».
  `expressions(source) -> Vec<String>` : lignes de commentaire (`//` en tête après
  indentation) écartées, puis pour chaque `Schedule::every::<` les chevrons comptés jusqu'à
  fermer le premier, `(` attendu, blancs sautés, littéral `"…"` lu s'il y en a un.
  Chaque expression passe par `crate::cron::valider` ; fautes → `Check::failed(TITRE,
  fautes.join(" ; "), "corrigez-les dans {FICHIER} : cinq champs (minute heure jour mois
  jour-de-semaine) ou six, la seconde en tête — le démarrage s'arrête sur la première qu'il
  ne lit pas")`, chaque faute passée par `super::une_ligne`. Sain → « 1 expression du
  calendrier se lit » / « N expressions du calendrier se lisent » / « aucune échéance
  déclarée ».

- [ ] **Étape 6 : inscription.** `pub mod scheduler;` dans `doctor/mod.rs`, entrée
  `("scheduler", Controle { titre: scheduler::TITRE, executer: |projet, _| scheduler::check(&projet.root) })`
  ajoutée à la fin de `FEATURE_CHECKS` (taille 8). Test de `doctor/mod.rs` :
  `project(&["health", "scheduler"])` reçoit `scheduler`, `project(&["health"])` non.

- [ ] **Étape 7 : vérifier.** `cargo test -p rbs-cli --lib`, clippy, fmt.

- [ ] **Étape 8 : commit** `feat(doctor): juge le calendrier de scheduler avec le validateur cron du démarrage`.

### Tâche 2 : `cors` et `rate-limit`

**Fichiers :** créer `src/doctor/cors.rs`, `src/doctor/rate_limit.rs` ; modifier
`src/doctor/mod.rs`.

**Interfaces :**
- Produit : `Config::array_len(&self, section: &str, key: &str) -> Option<usize>`.
- Produit (test) : `doctor::tests::reglages_du_fragment(feature: &str, section: &str) -> String`
  — le contenu du `[[config]]` du fragment, lignes vides et commentaires ôtés, joint par `\n`.

- [ ] **Étape 1 : tests (rouges).**
  - `cors` : section absente → `Echec` et remède portant `[cors]` ; `origins = []` →
    `Avertissement`, détail portant `origins` ; `origins = ["http://localhost:5173"]` → `Bon`,
    « 1 origine autorisée » ; `REGLAGES == reglages_du_fragment("cors", "cors")`.
  - `rate-limit` : section absente → `Echec` ; présente → `Bon` ;
    `REGLAGES == reglages_du_fragment("rate-limit", "rate_limit")` (routes comprises).

- [ ] **Étape 2 : `Config::array_len`** (`Item::as_array` puis `Array::len`), et l'aide de
  test `reglages_du_fragment` (analyse de `templates/features/<feature>/feature.toml` par
  `toml_edit`, `[[config]]` dont `section` correspond).

- [ ] **Étape 3 : `cors.rs`.** `TITRE = "cors"`. Section absente (ou config absente /
  fautive) → `super::section_check`. Sinon `array_len("cors", "origins")` : `Some(0) | None`
  (défaut serde : vide) → `Check::warned(TITRE, "`cors.origins` est vide : aucun front ne
  peut appeler l'API depuis un navigateur", "énumérez les origines de votre front dans
  config/default.toml — ou dans le profil de l'environnement qui les sert :\n[cors]\norigins =
  [\"http://localhost:5173\"]")` ; `Some(n)` → `Check::ok`, « n origine(s) autorisée(s) »
  accordé.

- [ ] **Étape 4 : `rate_limit.rs`.** `TITRE = "rate-limit"`, `SECTION = "rate_limit"`,
  `super::section_check(config, TITRE, SECTION, "la limite de débit a ses réglages", REGLAGES)`.

- [ ] **Étape 5 : inscription** dans `FEATURE_CHECKS` (taille 10), test d'ordre du rapport.

- [ ] **Étape 6 : vérifier, commit** `feat(doctor): contrôle les sections de cors et de rate-limit`.

### Tâche 3 : `webhooks` et `audit`

**Fichiers :** créer `src/doctor/webhooks.rs`, `src/doctor/audit.rs` ; modifier `src/doctor/mod.rs`.

- [ ] **Étape 1 : tests (rouges).**
  - `webhooks` : `src/modules/jobs/mod.rs` portant la ligne → `Bon` ; sans elle → `Echec`,
    remède portant la ligne et `<rbs:jobs>` ; fichier absent → `Echec` ;
    `INSCRIPTION` égale au contenu de l'ancre `jobs` de `webhooks/feature.toml`.
  - `audit` : `migration/src/lib.rs` déclarant `mod m20260913_000000_create_audit_log;` et
    `Box::new(m20260913_000000_create_audit_log::Migration),` → `Bon` ; déclarée sans
    `Box::new` → `Echec`, remède `Box::new(<module>::Migration),` ; ni l'un ni l'autre mais
    `migration/src/m…_create_audit_log.rs` présent → `Echec` nommant le module et les deux
    lignes ; aucun fichier de migration → `Echec`, remède Git.

- [ ] **Étape 2 : `webhooks.rs`.** `TITRE = "webhooks"`, `INSCRIPTION =
  "registre = registre.register::<crate::modules::webhooks::delivery::Delivery>();"`. Présente
  si une ligne, indentation ôtée, lui est égale. Absente → `Check::failed(TITRE, "la
  livraison des webhooks n'est pas inscrite au registre de la file : chaque livraison
  partira en échec", "dans src/modules/jobs/mod.rs, entre les balises de `// <rbs:jobs>` :\n{INSCRIPTION}")`.

- [ ] **Étape 3 : `audit.rs`.** `TITRE = "audit"`. Module déclaré : ligne `mod m…_create_audit_log;`
  hors commentaire ; inscrit : ligne `Box::new(m…_create_audit_log::Migration),`. Fichier de
  migration cherché dans `migration/src/` par suffixe `_create_audit_log.rs`.

- [ ] **Étape 4 : inscription** (taille 12), **vérifier, commit**
  `feat(doctor): vérifie la livraison des webhooks et la migration d'audit`.

### Tâche 4 : `docker` et `ci`, puis le rapport réel

**Fichiers :** créer `src/doctor/docker.rs`, `src/doctor/ci.rs` ; modifier `src/doctor/mod.rs`,
les deux pages `cli/doctor.md`.

- [ ] **Étape 1 : tests (rouges).** `docker` : `config/production.toml` présent → `Bon`,
  absent → `Echec` dont le remède porte `REGLAGES`, égal aux lignes non commentées de
  `templates/features/docker/config/production.toml.jinja`. `ci` : `.github/workflows/ci.yml`
  présent → `Bon`, absent → `Echec`, remède Git.

- [ ] **Étape 2 : `docker.rs`** (`TITRE = "docker"`, détail d'échec « config/production.toml
  est absent : le service `api` du compose pose RBS_ENV=production », remède « créez
  config/production.toml avec :\n[docs]\nswagger_ui = false\nopenapi_json = false ») et
  **`ci.rs`** (`TITRE = "ci"`, remède « restaurez-le depuis Git (`git checkout --
  .github/workflows/ci.yml`) : `rbs add` ne rejoue pas une feature déjà installée »).

- [ ] **Étape 3 : inscription** (taille 14) ; test : un projet déclarant les sept reçoit les
  sept titres dans l'ordre `cors, rate-limit, scheduler, webhooks, audit, docker, ci`.

- [ ] **Étape 4 : rapport réel.** Construire le CLI, puis dans le scratchpad :
  `rbs new demo --yes --with cors,rate-limit,scheduler,webhooks,audit,docker,ci --core-path
  <dépôt>/crates/rbs-core --database-url postgres://rbs:rbs@127.0.0.1:1/demo`, puis
  `rbs doctor --json` dans `demo`. La sortie réelle, `base` en échec (rien n'écoute sur le
  port 1), est citée dans le commit ; puis `origins` renseignée à la main et
  `config/production.toml` retiré, pour citer un avertissement et un échec.

- [ ] **Étape 5 : documentation.** `cli/doctor.md` EN+FR, section « Installed features » :
  un tableau des contrôles de feature (les sept nouveaux et les cinq existants), une ligne
  par fragment, et l'avertissement de `cors` rendu tel que la commande l'écrit.
  `cd docs && npm run build`.

- [ ] **Étape 6 : vérifier** (fmt, clippy, `cargo test --workspace`, `cargo test -p rbs-cli
  doctor::`), **commit** `feat(doctor): contrôle les fichiers que posent docker et ci` puis
  `docs(doctor): liste les contrôles des fragments`.

## Auto-relecture

- Couverture de la spec : sept contrôles (T1-T4), `src/cron.rs` et sa dépendance (T1), test
  croisé fragment/CLI (T1), `doctor --json` réel (T4), doc EN+FR (T4).
- Aucun type n'est employé avant sa tâche : `Config::array_len` (T2) ne sert qu'à `cors`.
