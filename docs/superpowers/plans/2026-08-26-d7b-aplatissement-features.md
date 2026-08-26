# Aplatissement des features — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Une feature d'un projet généré vit en `src/<nom>/` et non plus en
`src/features/<nom>/`, et le CLI refuse un nom de feature qui entrerait en collision avec
un module du squelette ou un mot-clé Rust.

**Architecture:** Le niveau `features/` disparaît du squelette : les modules de features
sont déclarés dans `src/main.rs`, qui porte désormais l'ancre `<rbs:features>`, et le
contenu inséré dans les ancres désigne les features par chemin absolu (`crate::users::…`).
En contrepartie de l'aplatissement, un module `generate/nom.rs` valide le nom d'une feature
avant toute écriture, sur le modèle de la validation des champs déjà en place.

**Tech Stack:** Rust 2024, minijinja (délimiteurs `{@ @}`), include_dir, assert_cmd,
tempfile, testcontainers.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` (§3.3, §3.4, §4.5) et
`docs/superpowers/specs/2026-08-26-squelette-projet-design.md` (§2, §4, §5), tous deux
amendés en préalable à ce plan.

## Global Constraints

- Édition Rust 2024 ; `cargo clippy --workspace --all-targets -- -D warnings` et
  `cargo fmt --all --check` sont bloquants.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- Les templates minijinja utilisent les délimiteurs de variable `{@ @}`, jamais `{{ }}`.
- Le CLI n'insère que dans des ancres `// <rbs:nom>` / `// </rbs:nom>` ; l'insertion se
  fait juste avant la balise fermante.
- Les tests marqués `#[ignore]` compilent un projet réel et exigent Docker ; ils se lancent
  par `-- --include-ignored`.
- Aucun identifiant de tâche ni renvoi à un fichier de suivi dans les messages de commit.

---

### Task 1 : Squelette aplati

Le squelette perd `src/features/`. `src/main.rs` accueille l'ancre `<rbs:features>`,
`src/health/` remplace `src/features/health/`, et les deux autres ancres passent au chemin
absolu.

**Files:**
- Create: `templates/project/src/health/mod.rs.jinja` (déplacement de
  `templates/project/src/features/health/mod.rs.jinja`)
- Create: `templates/project/src/health/controller.rs.jinja` (déplacement de
  `templates/project/src/features/health/controller.rs.jinja`)
- Delete: `templates/project/src/features/mod.rs.jinja`
- Modify: `templates/project/src/main.rs.jinja`, `templates/project/src/router.rs.jinja`,
  `templates/project/src/openapi.rs.jinja`
- Test: `crates/rbs-cli/src/templates.rs` (module `tests`, constantes `ANCRES` et
  `DESTINATIONS`), `crates/rbs-cli/src/new.rs` (module `tests`)

**Interfaces:**
- Consumes: rien.
- Produces: le squelette dont les tâches 2 et 3 dépendent — l'ancre `<rbs:features>` dans
  `src/main.rs` recevant `mod <nom>;`, l'ancre `<rbs:routes>` recevant
  `.merge(crate::<nom>::routes())`, l'ancre `<rbs:openapi>` recevant
  `crate::<nom>::controller::<handler>,`.

- [ ] **Step 1 : Écrire les tests qui échouent**

Dans `crates/rbs-cli/src/templates.rs`, module `tests`, remplacer les deux constantes :

```rust
    /// Les quatre points d'insertion, chacun avec le fichier qui le porte.
    const ANCRES: [(&str, &str); 4] = [
        ("features", "src/main.rs.jinja"),
        ("routes", "src/router.rs.jinja"),
        ("openapi", "src/openapi.rs.jinja"),
        ("migrations", "migration/src/lib.rs.jinja"),
    ];

    /// Les chemins de sortie attendus du squelette, tels que `rbs new` les écrira.
    const DESTINATIONS: [&str; 14] = [
        ".env",
        ".env.example",
        ".gitignore",
        "Cargo.toml",
        "config/default.toml",
        "config/development.toml",
        "migration/Cargo.toml",
        "migration/src/lib.rs",
        "src/health/controller.rs",
        "src/health/mod.rs",
        "src/main.rs",
        "src/openapi.rs",
        "src/router.rs",
        "src/state.rs",
    ];
```

Dans `crates/rbs-cli/src/new.rs`, module `tests`, la liste du test
`creer_ecrit_l_arborescence_attendue` (vers la ligne 320) perd `src/features/mod.rs` et
voit ses deux fichiers de `health` remonter :

```rust
            "src/health/controller.rs",
            "src/health/mod.rs",
            "src/main.rs",
```

Ajouter dans le module `tests` de `crates/rbs-cli/src/templates.rs` un test qui fixe la
place de l'ancre — c'est elle qui change, et rien d'autre ne la vérifie :

```rust
    #[test]
    fn l_ancre_des_features_suit_les_modules_du_squelette_dans_main() {
        let source = fs::read_to_string(Path::new(RACINE).join("src/main.rs.jinja"))
            .expect("main.rs.jinja lisible");

        let modules = source
            .find("mod state;")
            .expect("les modules du squelette doivent être déclarés");
        let ancre = source
            .find("// <rbs:features>")
            .expect("main.rs doit porter l'ancre des features");

        assert!(
            modules < ancre,
            "l'ancre doit suivre les modules du squelette :\n{source}"
        );
    }
```

- [ ] **Step 2 : Lancer les tests et les voir échouer**

Run: `cargo test -p rbs-cli templates:: new::tests`
Expected: FAIL — `ANCRES` pointe sur un fichier qui porte encore l'ancre ailleurs, et
`src/health/mod.rs` n'existe pas parmi les destinations rendues.

- [ ] **Step 3 : Déplacer les templates de health**

```bash
git mv templates/project/src/features/health templates/project/src/health
git rm templates/project/src/features/mod.rs.jinja
```

- [ ] **Step 4 : Déclarer les modules et l'ancre dans main.rs.jinja**

`templates/project/src/main.rs.jinja` — remplacer les quatre premières lignes :

```rust
mod health;
mod openapi;
mod router;
mod state;
// <rbs:features>
// </rbs:features>
```

Le reste du fichier est inchangé.

- [ ] **Step 5 : Monter health par son chemin dans router.rs.jinja**

`templates/project/src/router.rs.jinja` — `use crate::features;` devient
`use crate::health;`, et le montage perd son préfixe :

```rust
use axum::Router;
use axum::middleware::from_fn;
use rbs_core::HasCoreState;

use crate::health;
use crate::openapi;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let docs = openapi::routes(state.core().config());

    Router::new()
        .merge(health::routes())
        // <rbs:routes>
        // </rbs:routes>
        .merge(docs)
        .layer(from_fn(rbs_core::trace::middleware))
        .layer(from_fn(rbs_core::request_id::middleware))
        .with_state(state)
}
```

- [ ] **Step 6 : Raccourcir le chemin du handler dans openapi.rs.jinja**

`templates/project/src/openapi.rs.jinja` — dans `paths(...)` :

```rust
        crate::health::controller::sante,
        // <rbs:openapi>
        // </rbs:openapi>
```

- [ ] **Step 7 : Lancer les tests et les voir passer**

Run: `cargo test -p rbs-cli templates:: new::tests`
Expected: PASS

- [ ] **Step 8 : Vérifier qu'un projet neuf compile encore**

Run: `cargo test -p rbs-cli --test integration_new -- --include-ignored`
Expected: PASS — le projet créé par le binaire compile avec `src/` aplati.

- [ ] **Step 9 : Commit**

```bash
git add templates crates/rbs-cli/src/templates.rs crates/rbs-cli/src/new.rs
git commit -m "refactor(templates): pose les features à la racine de src

Le niveau `features/` ne portait aucune information : le seul contenu de
`src/` est le démarrage, le routage, l'état et des features. L'ancre
<rbs:features> descend dans main.rs, qui déclare déjà les modules, et le
contenu inséré dans les ancres désigne les features par chemin absolu.

Vérifications :
- cargo test -p rbs-cli templates:: new::tests → <à remplir>
- cargo test -p rbs-cli --test integration_new -- --include-ignored → <à remplir>"
```

---

### Task 2 : Générateurs et banc d'essai sur la structure aplatie

Les générateurs rendent des chaînes ; ce sont leur banc d'essai et leurs docs de module qui
portent encore `features/`. Le banc pose les fichiers au nouvel emplacement et remplit les
ancres avec les chemins absolus.

**Files:**
- Modify: `crates/rbs-cli/src/generate/banc.rs:102-172` (`poser_feature`, `monter_feature`)
- Modify: `crates/rbs-cli/src/generate/{entite,dto,repository,service,controller}.rs`
  (ligne 1, doc de module)
- Test: les modules `tests` des cinq générateurs, déjà écrits, servent de test de
  non-régression.

**Interfaces:**
- Consumes: le squelette de la tâche 1 — ancre `<rbs:features>` dans `src/main.rs`.
- Produces: `Projet::poser_feature(&self, module: &str, fichiers: &[(&str, &str)])` écrivant
  dans `src/<module>/` et déclarant `mod <module>;` dans `src/main.rs` ;
  `Projet::monter_feature(&self, module: &str, handlers: &[&str])` inchangée de signature.

- [ ] **Step 1 : Rediriger `poser_feature` vers `src/<module>/`**

`crates/rbs-cli/src/generate/banc.rs` — dans `poser_feature`, le répertoire perd son
niveau intermédiaire :

```rust
        let repertoire = self.racine.join("src").join(module);
```

et la déclaration du module passe de `src/features/mod.rs` à `src/main.rs`, avec la forme
qu'y prend une feature :

```rust
        let main = self.racine.join("src/main.rs");
        let source = fs::read_to_string(&main).expect("main.rs lisible");

        fs::write(
            &main,
            source.replace(
                "// <rbs:features>",
                &format!("// <rbs:features>\nmod {module};"),
            ),
        )
        .expect("main.rs écrivable");
```

Mettre à jour la doc de la méthode : « Écrit `src/<module>/` avec les fichiers donnés, et
déclare le module. »

- [ ] **Step 2 : Passer `monter_feature` aux chemins absolus**

Toujours dans `banc.rs`, les deux insertions :

```rust
                &format!("// <rbs:routes>\n        .merge(crate::{module}::routes())"),
```

```rust
            .map(|handler| format!("\n        crate::{module}::controller::{handler},"))
```

- [ ] **Step 3 : Corriger les docs de module des générateurs**

Ligne 1 de chacun des cinq fichiers, `features/<nom>/` devient `<nom>/` :

```rust
//! Rendu de `<nom>/model.rs` : l'entité SeaORM d'une feature.
//! Rendu de `<nom>/dto.rs` : les trois formes que la feature expose en HTTP.
//! Rendu de `<nom>/repository.rs` : le seul fichier qui parle à la base.
//! Rendu de `<nom>/service.rs` : les décisions métier de la feature.
//! Rendu de `<nom>/controller.rs` et du `mod.rs` qui monte ses routes.
```

- [ ] **Step 4 : Lancer les tests de rendu**

Run: `cargo test -p rbs-cli generate::`
Expected: PASS — les tests de rendu ne dépendent pas de l'emplacement.

- [ ] **Step 5 : Lancer les tests qui compilent un projet réel**

Run: `cargo test -p rbs-cli generate:: -- --include-ignored`
Expected: PASS — c'est le seul test qui prouve que la structure aplatie compile avec une
feature complète, ses routes montées et son document OpenAPI. Nécessite Docker.

- [ ] **Step 6 : Commit**

```bash
git add crates/rbs-cli/src/generate
git commit -m "refactor(cli): pose les features générées à la racine de src

Le banc d'essai écrit désormais dans src/<nom>/ et déclare la feature dans
l'ancre de main.rs. Les insertions dans <rbs:routes> et <rbs:openapi>
passent au chemin absolu, ce qui évite d'avoir à écrire un `use` par
feature dans un second endroit du fichier.

Vérifications :
- cargo test -p rbs-cli generate:: → <à remplir>
- cargo test -p rbs-cli generate:: -- --include-ignored → <à remplir>"
```

---

### Task 3 : Validation du nom de feature

Sans `features/`, un nom de feature entre en concurrence avec les modules du squelette :
`rbs g crud state` écraserait `src/state.rs`. Aucune validation ne portait jusqu'ici sur le
nom d'une feature — seulement sur ses champs.

**Files:**
- Create: `crates/rbs-cli/src/generate/nom.rs`
- Modify: `crates/rbs-cli/src/generate/mod.rs` (déclaration du module)
- Modify: `crates/rbs-cli/src/generate/champs.rs:307` et `:293` (visibilité de
  `est_en_snake_case` et `MOTS_CLES_RUST`)
- Test: module `tests` de `crates/rbs-cli/src/generate/nom.rs`

**Interfaces:**
- Consumes: `champs::est_en_snake_case(&str) -> bool`, `champs::MOTS_CLES_RUST: [&str; 51]`,
  `champs::erreur::en_snake_case(&str) -> String` (déjà `pub(crate)`).
- Produces: `nom::valider(nom: &str) -> Result<(), ErreurNom>`, appelé par la commande
  `generate` (tâche D10) avant toute écriture. `ErreurNom` implémente `Display` et rend un
  message suivi d'un indice, comme `ErreurChamp`.

- [ ] **Step 1 : Écrire les tests qui échouent**

Créer `crates/rbs-cli/src/generate/nom.rs` avec, pour l'instant, seulement son module de
tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_nom_en_snake_case_est_accepte() {
        for nom in ["users", "blog_posts", "categories", "v2_items"] {
            assert!(valider(nom).is_ok(), "« {nom} » doit être accepté");
        }
    }

    #[test]
    fn un_nom_mal_casse_suggere_sa_forme_snake_case() {
        let erreur = valider("BlogPosts").expect_err("un nom en PascalCase doit être refusé");

        let rendu = erreur.to_string();
        assert!(rendu.contains("snake_case"), "{rendu}");
        assert!(rendu.contains("blog_posts"), "la recasse doit être proposée : {rendu}");
    }

    #[test]
    fn un_mot_cle_rust_est_refuse() {
        let erreur = valider("match").expect_err("un mot-clé Rust doit être refusé");

        assert!(erreur.to_string().contains("mot-clé Rust"), "{erreur}");
    }

    #[test]
    fn un_module_du_squelette_est_refuse_en_le_nommant() {
        for nom in ["main", "router", "openapi", "state", "health"] {
            let Err(erreur) = valider(nom) else {
                panic!("« {nom} » doit être refusé");
            };
            let rendu = erreur.to_string();

            assert!(
                rendu.contains(nom),
                "le message doit nommer le module en cause : {rendu}"
            );
            assert!(
                rendu.contains("squelette"),
                "le message doit dire d'où vient la collision : {rendu}"
            );
        }
    }

    #[test]
    fn un_nom_vide_est_refuse() {
        assert!(valider("").is_err());
    }
}
```

- [ ] **Step 2 : Lancer les tests et les voir échouer**

Run: `cargo test -p rbs-cli generate::nom`
Expected: FAIL à la compilation — `valider` n'existe pas.

- [ ] **Step 3 : Implémenter la validation**

Corps de `crates/rbs-cli/src/generate/nom.rs`, avant le module `tests` :

```rust
//! Validation du nom d'une feature, avant qu'un seul fichier ne soit écrit.
//!
//! Une feature occupe `src/<nom>/` : son nom entre donc en concurrence avec les modules
//! que `rbs new` a posés. Le diagnostic suit celui des champs — un message, puis un
//! indice qui donne l'issue.

use std::fmt;

use super::champs::erreur::{en_snake_case, suggestions_mot_cle};
use super::champs::{MOTS_CLES_RUST, est_en_snake_case};

/// Ce qui rend un nom de feature inutilisable.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ErreurNom {
    libelle: String,
    nature: Nature,
}

#[derive(Debug, PartialEq, Eq)]
enum Nature {
    Vide,
    PasEnSnakeCase { suggestion: Option<String> },
    MotCleRust,
    ModuleDuSquelette,
}

/// Modules que `rbs new` pose à la racine de `src/` : une feature qui en porte le nom
/// écraserait le fichier.
const MODULES_DU_SQUELETTE: [&str; 5] = ["main", "router", "openapi", "state", "health"];

/// Vérifie qu'une feature peut porter ce nom sans casser le projet.
pub(crate) fn valider(nom: &str) -> Result<(), ErreurNom> {
    let erreur = |nature| {
        Err(ErreurNom {
            libelle: nom.to_string(),
            nature,
        })
    };

    if nom.is_empty() {
        return erreur(Nature::Vide);
    }

    if !est_en_snake_case(nom) {
        let recasse = en_snake_case(nom);
        let suggestion = (recasse != nom && est_en_snake_case(&recasse)).then_some(recasse);

        return erreur(Nature::PasEnSnakeCase { suggestion });
    }

    if MOTS_CLES_RUST.contains(&nom) {
        return erreur(Nature::MotCleRust);
    }

    if MODULES_DU_SQUELETTE.contains(&nom) {
        return erreur(Nature::ModuleDuSquelette);
    }

    Ok(())
}

impl fmt::Display for ErreurNom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let libelle = &self.libelle;

        let (message, indice) = match &self.nature {
            Nature::Vide => (
                "le nom de la feature est vide".to_string(),
                Some("exemple : « rbs generate crud users »".to_string()),
            ),
            Nature::PasEnSnakeCase { suggestion } => (
                "le nom doit être en snake_case : minuscules ASCII, chiffres et souligné"
                    .to_string(),
                suggestion
                    .as_ref()
                    .map(|valeur| format!("essayez « {valeur} »")),
            ),
            Nature::MotCleRust => {
                let liste: Vec<String> = suggestions_mot_cle(libelle)
                    .iter()
                    .map(|suggestion| format!("« {suggestion} »"))
                    .collect();

                (
                    format!("« {libelle} » est un mot-clé Rust"),
                    Some(format!("essayez {}", liste.join(" ou "))),
                )
            }
            Nature::ModuleDuSquelette => (
                format!("« {libelle} » est un module du squelette du projet"),
                Some(format!(
                    "src/ porte déjà ce module — essayez « {libelle}s »"
                )),
            ),
        };

        write!(f, "✗ {message}")?;
        if let Some(indice) = indice {
            write!(f, "\n  {indice}")?;
        }

        Ok(())
    }
}
```

Rendre accessibles les deux items de `champs.rs` :

```rust
pub(crate) fn est_en_snake_case(nom: &str) -> bool {
```

```rust
pub(crate) const MOTS_CLES_RUST: [&str; 51] = [
```

Le module `erreur` de `champs.rs` passe de `mod erreur;` à `pub(crate) mod erreur;`.

Déclarer le module dans `crates/rbs-cli/src/generate/mod.rs`, à sa place alphabétique :

```rust
pub(crate) mod migration;
pub(crate) mod nom;
pub(crate) mod repository;
```

- [ ] **Step 4 : Lancer les tests et les voir passer**

Run: `cargo test -p rbs-cli generate::nom`
Expected: PASS — 5 tests.

- [ ] **Step 5 : Vérifier que le reste du CLI est intact**

Run: `cargo test -p rbs-cli && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: PASS, aucun warning.

- [ ] **Step 6 : Commit**

```bash
git add crates/rbs-cli/src/generate
git commit -m "feat(cli): refuse un nom de feature qui casserait le projet

Une feature occupe désormais src/<nom>/ : son nom entre en concurrence
avec les modules posés par `rbs new`. Un nom mal cassé, un mot-clé Rust
ou un module du squelette sont refusés avec l'issue à prendre, sur le
modèle du diagnostic des champs.

Vérifications :
- cargo test -p rbs-cli generate::nom → <à remplir>
- cargo clippy --workspace --all-targets -- -D warnings → <à remplir>"
```

---

## Vérification finale du lot

- [ ] `cargo test --workspace` → tout vert
- [ ] `cargo test -p rbs-cli -- --include-ignored` → tout vert (Docker requis)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` → aucun warning
- [ ] `cargo fmt --all --check` → aucun diff
- [ ] `grep -rn 'features/' crates templates` ne rend plus que des occurrences légitimes
      (`features = [...]` de Cargo, `Options::features` de `rbs new`)
