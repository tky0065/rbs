# Modèle de plan — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Réifier en une valeur la planification des commandes qui modifient un projet, pour que l'affichage, la vérification et la restauration du lot E s'appuient toutes sur le même objet.

**Architecture:** Un module `plan`, neutre vis-à-vis des commandes : il ne connaît ni `add` ni `generate`, seulement des actions. Un `Constructeur` accumule les actions en lisant le disque pour calculer, pour chaque fichier touché, son contenu avant et son contenu après. Rien ne s'écrit pendant la construction. Le `Plan` produit expose deux vues : les actions, que l'utilisateur lit, et les fichiers, que l'application écrira.

**Tech Stack:** Rust, `toml_edit` (déjà en dépendance de `rbs-cli`), `tempfile` (déjà en dev-dependency).

**Spec:** `docs/superpowers/specs/2026-08-26-plan-add-design.md`

## Global Constraints

- Les `-> N passed` des messages de commit ci-dessous sont à remplacer par le compte réellement affiché par la commande. Un chiffre recopié du plan sans avoir lu la sortie n'est pas une vérification.
- Le plan et la spec se commitent avec le premier commit de code, comme les tâches précédentes du dépôt l'ont fait.
- Le module est `pub(crate)` : `rbs-cli` est un binaire, rien n'en sort.
- `#![warn(missing_docs)]` ne porte que sur `rbs-core`, mais le style du dépôt veut un `///` d'une à trois lignes sur chaque item ; un commentaire qui paraphrase la ligne suivante se supprime.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont bloquants. Le module n'ayant pas encore d'appelant, il porte un `#![allow(dead_code)]` justifié en commentaire, comme `crates/rbs-cli/src/metadata.rs:8` le fait déjà.
- Tests inline en `#[cfg(test)] mod tests`, nommés en français à l'indicatif, comme dans `crates/rbs-cli/src/ancres.rs:141`.
- Aucune écriture disque pendant la planification. C'est le critère de la tâche.
- Commits en Conventional Commits, sujet français à l'impératif, aucun identifiant de tâche, aucune mention d'outil ou d'assistant.

## File Structure

| Fichier | Responsabilité |
|---|---|
| `crates/rbs-cli/src/plan/mod.rs` (créer) | `Plan`, `Fichier`, `Constructeur`, `Erreur`. Le calcul des « après » et des statuts. |
| `crates/rbs-cli/src/plan/action.rs` (créer) | `Action`, `Effet`, `PatchToml`, `Statut`. Types seuls, aucune logique d'accès disque. |
| `crates/rbs-cli/src/metadata.rs` (modifier) | Scinder `ajouter_feature` : une fonction pure `inscrire_feature`, et son enrobage lecture/écriture. |
| `crates/rbs-cli/src/main.rs:1-12` (modifier) | Déclarer `mod plan;`. |

---

### Task 1: `metadata::inscrire_feature`, la partie pure

`PatchToml::InscrireFeature` a besoin de calculer le nouveau texte d'un `Cargo.toml` sans l'écrire. `ajouter_feature` (`crates/rbs-cli/src/metadata.rs:97`) fait aujourd'hui lecture, modification et écriture en un bloc.

**Files:**
- Modify: `crates/rbs-cli/src/metadata.rs:93-129`
- Test: `crates/rbs-cli/src/metadata.rs` (module `tests` en fin de fichier)

**Interfaces:**
- Consumes: `metadata::Erreur`, `metadata::charger` (privée, inchangée)
- Produces: `pub fn inscrire_feature(texte: &str, feature: &str, nom: &str) -> Result<Option<String>, Erreur>` — `Ok(None)` si la feature est déjà inscrite, `Ok(Some(texte))` sinon. `nom` ne sert qu'à nommer le fichier dans les messages d'erreur, la fonction ne touchant pas au disque.

- [ ] **Step 1: Write the failing tests**

Ajouter au module `tests` de `crates/rbs-cli/src/metadata.rs` :

```rust
    const MANIFESTE: &str = r#"[package]
name = "demo"

# les features installées
[package.metadata.rbs]
version = "0.1.0"
features = ["health"]
"#;

    #[test]
    fn une_feature_absente_est_inscrite_sans_toucher_au_reste_du_manifeste() {
        let rendu = inscrire_feature(MANIFESTE, "docker", "Cargo.toml")
            .expect("le manifeste est valide")
            .expect("la feature est absente, le texte change");

        assert!(rendu.contains(r#"features = ["health", "docker"]"#));
        assert!(rendu.contains("# les features installées"));
        assert!(rendu.starts_with("[package]\nname = \"demo\"\n"));
    }

    #[test]
    fn une_feature_deja_inscrite_ne_produit_aucun_texte() {
        let rendu = inscrire_feature(MANIFESTE, "health", "Cargo.toml").expect("le manifeste est valide");

        assert_eq!(rendu, None);
    }

    #[test]
    fn un_manifeste_sans_section_rbs_est_refuse() {
        let erreur = inscrire_feature("[package]\nname = \"demo\"\n", "docker", "Cargo.toml")
            .expect_err("la section manque");

        assert!(matches!(erreur, Erreur::PasUnProjet { .. }));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rbs-cli metadata::tests::une_feature -- --nocapture`
Expected: FAIL — `cannot find function 'inscrire_feature' in this scope`

- [ ] **Step 3: Write the implementation**

Remplacer `ajouter_feature` (`crates/rbs-cli/src/metadata.rs:93-129`) par ces deux fonctions :

```rust
/// Rend le manifeste avec `feature` inscrite, ou `None` si elle y est déjà.
///
/// `nom` ne désigne le fichier que dans les messages d'erreur : rien n'est lu ni écrit ici.
pub fn inscrire_feature(texte: &str, feature: &str, nom: &str) -> Result<Option<String>, Erreur> {
    let mut document = texte.parse::<DocumentMut>().map_err(|source| Erreur::Syntaxe {
        chemin: nom.to_string(),
        source,
    })?;

    let rbs = document
        .get_mut("package")
        .and_then(|package| package.get_mut("metadata"))
        .and_then(|metadata| metadata.get_mut("rbs"))
        .ok_or_else(|| Erreur::PasUnProjet {
            chemin: nom.to_string(),
        })?;

    let installees = rbs
        .get_mut("features")
        .and_then(Item::as_array_mut)
        .ok_or_else(|| Erreur::Champ {
            chemin: nom.to_string(),
            cle: "features",
        })?;

    if installees
        .iter()
        .any(|valeur| valeur.as_str() == Some(feature))
    {
        return Ok(None);
    }

    installees.push(feature);

    Ok(Some(document.to_string()))
}

/// Inscrit `feature` dans les features installées, sans effet si elle y est déjà.
///
/// Ne réécrit pas le manifeste dans ce cas : une commande relancée ne doit pas salir le
/// working tree.
pub fn ajouter_feature(cargo_toml: &Path, feature: &str) -> Result<(), Erreur> {
    let nom = nommer(cargo_toml);

    let texte = fs::read_to_string(cargo_toml).map_err(|source| Erreur::Acces {
        chemin: nom.clone(),
        source,
    })?;

    let Some(rendu) = inscrire_feature(&texte, feature, &nom)? else {
        return Ok(());
    };

    fs::write(cargo_toml, rendu).map_err(|source| Erreur::Acces {
        chemin: nom,
        source,
    })
}
```

- [ ] **Step 4: Run the whole crate's tests**

Run: `cargo test -p rbs-cli --bins` puis `cargo test -p rbs-cli --bins`
Expected: PASS, y compris les tests existants de `metadata` — `ajouter_feature` garde son contrat pour `generate/commande.rs:124`.

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-cli/src/metadata.rs
git commit -m "refactor(cli): dégage du patch de manifeste sa partie sans effet de bord

L'inscription d'une feature lisait, modifiait et écrivait en un bloc. La
planification a besoin de la modification seule : elle calcule le texte
final d'un fichier avant de décider s'il faut l'écrire.

Vérifications :
- cargo test -p rbs-cli --bins -> <compte réel> passed"
```

---

### Task 2: Les types d'action, et la création de fichier

**Files:**
- Create: `crates/rbs-cli/src/plan/action.rs`
- Create: `crates/rbs-cli/src/plan/mod.rs`
- Modify: `crates/rbs-cli/src/main.rs:1-12`

**Interfaces:**
- Consumes: `crate::ancres::Ancre`, `crate::metadata`
- Produces: `plan::action::{Action, Effet, PatchToml, Statut}`, `plan::{Plan, Fichier, Constructeur, Erreur}`, `Constructeur::nouveau(racine: PathBuf) -> Constructeur`, `Constructeur::creer(&mut self, chemin: &str, contenu: &str) -> Result<(), Erreur>`, `Constructeur::finir(self) -> Plan`, `Plan::actions(&self) -> &[Action]`, `Plan::fichiers(&self) -> &[Fichier]`, `Plan::racine(&self) -> &Path`

- [ ] **Step 1: Write `action.rs`**

```rust
//! Ce qu'un plan décrit : des actions, leur effet, et ce qu'elles produiront.
//!
//! Types seuls : la lecture du disque et le calcul des statuts appartiennent au
//! constructeur.

use crate::ancres::Ancre;

/// Une action du plan : le fichier qu'elle vise, ce qu'elle y fait, et ce qu'elle
/// produira.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Action {
    /// Chemin du fichier visé, relatif à la racine du projet.
    pub chemin: String,
    pub effet: Effet,
    pub statut: Statut,
}

/// Ce qu'une action fait au fichier qu'elle vise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Effet {
    /// Écrit un fichier dont le contenu est entièrement connu.
    Creer { contenu: String },
    /// Ajoute des lignes dans une ancre, juste avant sa balise fermante.
    Inserer { ancre: Ancre, lignes: Vec<String> },
    /// Modifie un manifeste TOML en préservant sa mise en forme.
    PatcherToml { patch: PatchToml },
}

/// Les modifications qu'un plan sait faire à un `Cargo.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PatchToml {
    /// Inscrit une feature dans `[package.metadata.rbs]`.
    InscrireFeature(String),
}

/// Ce que l'action produira, connu dès la planification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Statut {
    /// Le contenu final diffère de l'actuel : l'action aura un effet.
    AFaire,
    /// Le contenu final égale l'actuel : l'action est sans effet.
    DejaFait,
    /// Le fichier existe, avec un contenu que l'action n'a pas produit. Seule une
    /// exécution forcée l'écrasera.
    Conflit,
}
```

- [ ] **Step 2: Write the failing tests in `mod.rs`**

Créer `crates/rbs-cli/src/plan/mod.rs` avec l'en-tête, `mod action;`, et ce module de tests — la déclaration des types et fonctions vient à l'étape 4 :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn projet() -> TempDir {
        TempDir::new().expect("le répertoire temporaire se crée")
    }

    #[test]
    fn creer_un_fichier_absent_est_a_faire() {
        let projet = projet();
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .creer("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent, rien ne s'y oppose");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::AFaire);
        assert_eq!(plan.fichiers()[0].avant, None);
        assert_eq!(plan.fichiers()[0].apres, "FROM rust\n");
    }

    #[test]
    fn creer_un_fichier_deja_identique_est_deja_fait() {
        let projet = projet();
        fs::write(projet.path().join("Dockerfile"), "FROM rust\n").expect("l'écriture aboutit");
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur.creer("Dockerfile", "FROM rust\n").expect("le fichier se lit");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::DejaFait);
        assert_eq!(plan.fichiers()[0].avant.as_deref(), Some("FROM rust\n"));
    }

    #[test]
    fn creer_par_dessus_un_contenu_different_est_un_conflit() {
        let projet = projet();
        fs::write(projet.path().join("Dockerfile"), "FROM alpine\n").expect("l'écriture aboutit");
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur.creer("Dockerfile", "FROM rust\n").expect("le fichier se lit");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::Conflit);
        assert_eq!(plan.fichiers()[0].avant.as_deref(), Some("FROM alpine\n"));
        assert_eq!(plan.fichiers()[0].apres, "FROM rust\n");
    }

    #[test]
    fn planifier_une_creation_n_ecrit_pas_le_fichier() {
        let projet = projet();
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur.creer("Dockerfile", "FROM rust\n").expect("le fichier est absent");
        constructeur.finir();

        assert!(!projet.path().join("Dockerfile").exists());
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Ajouter `mod plan;` à `crates/rbs-cli/src/main.rs`, juste après `mod new;` (ligne 8) — les modules y sont dans l'ordre alphabétique.

Run: `cargo test -p rbs-cli plan::`
Expected: FAIL — `cannot find struct 'Constructeur'`

- [ ] **Step 4: Write the implementation**

En tête de `crates/rbs-cli/src/plan/mod.rs`, avant le module de tests :

```rust
//! La planification d'une commande qui modifie un projet, réifiée en valeur.
//!
//! Un plan est une liste d'actions ; chaque action vise un fichier et connaît son contenu
//! avant et son contenu après. Planifier, c'est calculer les « après » sans rien écrire —
//! d'où l'affichage préalable, la restauration en cas d'échec et l'idempotence.

// Le module précède ses appelants : `rbs add` n'est pas encore implémentée.
#![allow(dead_code)]

mod action;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) use action::{Action, Effet, PatchToml, Statut};

/// Un fichier que le plan touche, avec ses deux états.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fichier {
    /// Chemin relatif à la racine du projet.
    pub chemin: String,
    /// Contenu actuel, ou `None` si le fichier n'existe pas encore.
    pub avant: Option<String>,
    /// Contenu que l'application écrira.
    pub apres: String,
}

/// Ce qu'une commande fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    racine: PathBuf,
    actions: Vec<Action>,
    fichiers: Vec<Fichier>,
}

impl Plan {
    /// Les actions dans l'ordre où elles ont été planifiées.
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    /// Les fichiers touchés, un par chemin, dans l'ordre où ils ont été rencontrés.
    pub fn fichiers(&self) -> &[Fichier] {
        &self.fichiers
    }

    /// Racine du projet, à laquelle les chemins des fichiers sont relatifs.
    pub fn racine(&self) -> &Path {
        &self.racine
    }
}

/// Ce qui peut empêcher de planifier.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// Un fichier du projet n'a pas pu être lu.
    #[error("{chemin} est inaccessible : {source}")]
    Acces {
        /// Chemin fautif, relatif à la racine.
        chemin: String,
        /// Cause système.
        source: io::Error,
    },
}

/// Accumule les actions d'un plan en calculant, pour chaque fichier, son contenu final.
pub(crate) struct Constructeur {
    racine: PathBuf,
    actions: Vec<Action>,
    fichiers: Vec<Fichier>,
}

impl Constructeur {
    /// Ouvre un plan vide sur le projet enraciné en `racine`.
    pub fn nouveau(racine: PathBuf) -> Self {
        Self {
            racine,
            actions: Vec::new(),
            fichiers: Vec::new(),
        }
    }

    /// Planifie l'écriture de `chemin` avec `contenu`.
    pub fn creer(&mut self, chemin: &str, contenu: &str) -> Result<(), Erreur> {
        let avant = self.etat_courant(chemin)?;

        let statut = match avant.as_deref() {
            None => Statut::AFaire,
            Some(actuel) if actuel == contenu => Statut::DejaFait,
            Some(_) => Statut::Conflit,
        };

        self.projeter(chemin, avant, contenu.to_string());
        self.actions.push(Action {
            chemin: chemin.to_string(),
            effet: Effet::Creer {
                contenu: contenu.to_string(),
            },
            statut,
        });

        Ok(())
    }

    /// Clôt le plan.
    pub fn finir(self) -> Plan {
        Plan {
            racine: self.racine,
            actions: self.actions,
            fichiers: self.fichiers,
        }
    }

    /// Contenu du fichier tel que les actions déjà planifiées le laisseront.
    ///
    /// Une action qui suit une autre sur le même fichier part de ce que la première
    /// produit, et non de ce que le disque contient encore.
    fn etat_courant(&self, chemin: &str) -> Result<Option<String>, Erreur> {
        if let Some(fichier) = self.fichiers.iter().find(|f| f.chemin == chemin) {
            return Ok(Some(fichier.apres.clone()));
        }

        self.lire(chemin)
    }

    /// Contenu du fichier sur le disque, ou `None` s'il n'existe pas.
    fn lire(&self, chemin: &str) -> Result<Option<String>, Erreur> {
        match fs::read_to_string(self.racine.join(chemin)) {
            Ok(contenu) => Ok(Some(contenu)),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(Erreur::Acces {
                chemin: chemin.to_string(),
                source,
            }),
        }
    }

    /// Enregistre le contenu final du fichier, en conservant son état d'origine.
    fn projeter(&mut self, chemin: &str, avant: Option<String>, apres: String) {
        match self.fichiers.iter_mut().find(|f| f.chemin == chemin) {
            Some(fichier) => fichier.apres = apres,
            None => self.fichiers.push(Fichier {
                chemin: chemin.to_string(),
                avant,
                apres,
            }),
        }
    }
}
```

Note : `projeter` reçoit l'`avant` que `etat_courant` a calculé ; quand le fichier est déjà projeté, cet `avant` est le contenu projeté et non l'original — mais la branche `Some` de `projeter` ne s'en sert pas, et l'`avant` d'origine reste celui du premier passage. C'est l'invariant qui rend le rollback correct.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p rbs-cli plan::`
Expected: PASS, 4 tests

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/src/plan crates/rbs-cli/src/main.rs
git commit -m "feat(cli): réifie en valeur ce qu'une commande fera à un projet

Une action vise un fichier et connaît son contenu avant et après elle. La
construction lit le disque pour calculer les « après » et n'y écrit rien,
ce dont dépendent l'affichage préalable du plan et sa restauration.

Vérifications :
- cargo test -p rbs-cli plan:: -> 4 passed"
```

---

### Task 3: L'insertion dans une ancre

**Files:**
- Modify: `crates/rbs-cli/src/plan/mod.rs`
- Test: `crates/rbs-cli/src/plan/mod.rs` (module `tests`)

**Interfaces:**
- Consumes: `crate::ancres::{self, Ancre, Absente, inserer}`, `Constructeur` de la tâche 2
- Produces: `Constructeur::inserer(&mut self, ancre: Ancre, lignes: &[String]) -> Result<(), Erreur>`, variante `Erreur::Ancre(ancres::Absente)`

Le fichier visé n'est pas un paramètre : `Ancre` porte son `fichier` (`crates/rbs-cli/src/ancres.rs:13`).

- [ ] **Step 1: Write the failing tests**

Ajouter au module `tests` de `plan/mod.rs` :

```rust
    use crate::ancres;

    const ROUTER: &str = "pub fn router() -> Router {\n    Router::new()\n        // <rbs:routes>\n        // </rbs:routes>\n}\n";

    fn avec_router(projet: &TempDir, source: &str) {
        fs::create_dir_all(projet.path().join("src")).expect("le répertoire se crée");
        fs::write(projet.path().join("src/router.rs"), source).expect("l'écriture aboutit");
    }

    #[test]
    fn inserer_dans_une_ancre_vide_est_a_faire() {
        let projet = projet();
        avec_router(&projet, ROUTER);
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .inserer(ancres::ROUTES, &[".merge(crate::users::routes())".to_string()])
            .expect("l'ancre est présente");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::AFaire);
        assert_eq!(plan.actions()[0].chemin, "src/router.rs");
        assert!(plan.fichiers()[0].apres.contains(".merge(crate::users::routes())"));
    }

    #[test]
    fn inserer_une_ligne_deja_presente_est_deja_fait() {
        let projet = projet();
        avec_router(&projet, &ROUTER.replace("        // </rbs:routes>", "        .merge(crate::users::routes())\n        // </rbs:routes>"));
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .inserer(ancres::ROUTES, &[".merge(crate::users::routes())".to_string()])
            .expect("l'ancre est présente");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::DejaFait);
        assert_eq!(plan.fichiers()[0].avant.as_deref(), Some(plan.fichiers()[0].apres.as_str()));
    }

    #[test]
    fn deux_insertions_dans_un_meme_fichier_se_chainent_sur_un_seul_fichier() {
        let projet = projet();
        let lib = "// <rbs:migration_modules>\n// </rbs:migration_modules>\nvec![\n    // <rbs:migrations>\n    // </rbs:migrations>\n]\n";
        fs::create_dir_all(projet.path().join("migration/src")).expect("le répertoire se crée");
        fs::write(projet.path().join("migration/src/lib.rs"), lib).expect("l'écriture aboutit");
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .inserer(ancres::MIGRATION_MODULES, &["mod m20260826_creer_users;".to_string()])
            .expect("l'ancre est présente");
        constructeur
            .inserer(ancres::MIGRATIONS, &["Box::new(m20260826_creer_users::Migration),".to_string()])
            .expect("l'ancre est présente");
        let plan = constructeur.finir();

        assert_eq!(plan.actions().len(), 2);
        assert_eq!(plan.fichiers().len(), 1);
        assert!(plan.fichiers()[0].apres.contains("mod m20260826_creer_users;"));
        assert!(plan.fichiers()[0].apres.contains("Box::new(m20260826_creer_users::Migration),"));
        assert_eq!(plan.fichiers()[0].avant.as_deref(), Some(lib));
    }

    #[test]
    fn une_ancre_absente_interrompt_la_planification() {
        let projet = projet();
        avec_router(&projet, "pub fn router() -> Router {\n    Router::new()\n}\n");
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        let erreur = constructeur
            .inserer(ancres::ROUTES, &[".merge(crate::users::routes())".to_string()])
            .expect_err("l'ancre manque");

        assert!(matches!(erreur, Erreur::Ancre(_)));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rbs-cli plan::tests::inserer`
Expected: FAIL — `no method named 'inserer' found`

- [ ] **Step 3: Write the implementation**

Ajouter la variante à `Erreur` :

```rust
    /// Une ancre attendue a disparu du projet.
    #[error("{0}")]
    Ancre(#[source] crate::ancres::Absente),
```

Et la méthode à `Constructeur`, après `creer` :

```rust
    /// Planifie l'ajout de `lignes` dans `ancre`, juste avant sa balise fermante.
    ///
    /// Le fichier visé est celui que l'ancre désigne : une ancre ne se déplace pas.
    pub fn inserer(&mut self, ancre: Ancre, lignes: &[String]) -> Result<(), Erreur> {
        let chemin = ancre.fichier;

        let avant = self.etat_courant(chemin)?.ok_or_else(|| {
            Erreur::Ancre(crate::ancres::Absente { ancre })
        })?;

        let apres = crate::ancres::inserer(&avant, ancre, lignes).map_err(Erreur::Ancre)?;

        let statut = if apres == avant {
            Statut::DejaFait
        } else {
            Statut::AFaire
        };

        self.projeter(chemin, Some(avant), apres);
        self.actions.push(Action {
            chemin: chemin.to_string(),
            effet: Effet::Inserer {
                ancre,
                lignes: lignes.to_vec(),
            },
            statut,
        });

        Ok(())
    }
```

Ajouter `use crate::ancres::Ancre;` aux imports du module.

Un fichier absent est traité comme une ancre absente : le message dit ce que le développeur doit faire, et l'absence du fichier est le cas extrême de l'absence d'ancre.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rbs-cli plan::`
Expected: PASS, 8 tests

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-cli/src/plan/mod.rs
git commit -m "feat(cli): planifie l'insertion dans une ancre

Deux ancres visent le même fichier de la crate migration : la seconde
insertion se calcule contre ce que la première projette, jamais contre ce
que le disque contient encore. Le plan n'en garde qu'un fichier, dont
l'état d'origine reste celui d'avant la première.

Vérifications :
- cargo test -p rbs-cli plan:: -> 8 passed"
```

---

### Task 4: Le patch de manifeste

**Files:**
- Modify: `crates/rbs-cli/src/plan/mod.rs`
- Test: `crates/rbs-cli/src/plan/mod.rs` (module `tests`)

**Interfaces:**
- Consumes: `metadata::inscrire_feature` (tâche 1), `Constructeur` (tâche 2)
- Produces: `Constructeur::patcher(&mut self, patch: PatchToml) -> Result<(), Erreur>`, variante `Erreur::Metadonnees(metadata::Erreur)`

Le fichier visé est toujours le `Cargo.toml` de la racine : `PatchToml` ne décrit aujourd'hui que des modifications de `[package.metadata.rbs]`.

- [ ] **Step 1: Write the failing tests**

```rust
    const CARGO: &str = "[package]\nname = \"demo\"\n\n[package.metadata.rbs]\nversion = \"0.1.0\"\nfeatures = [\"health\"]\n";

    #[test]
    fn patcher_une_feature_absente_est_a_faire() {
        let projet = projet();
        fs::write(projet.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .patcher(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::AFaire);
        assert_eq!(plan.actions()[0].chemin, "Cargo.toml");
        assert!(plan.fichiers()[0].apres.contains("\"docker\""));
    }

    #[test]
    fn patcher_une_feature_deja_inscrite_est_deja_fait() {
        let projet = projet();
        fs::write(projet.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .patcher(PatchToml::InscrireFeature("health".to_string()))
            .expect("le manifeste est valide");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::DejaFait);
        assert_eq!(plan.fichiers()[0].apres, CARGO);
    }

    #[test]
    fn patcher_un_manifeste_absent_est_signale() {
        let projet = projet();
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        let erreur = constructeur
            .patcher(PatchToml::InscrireFeature("docker".to_string()))
            .expect_err("le manifeste manque");

        assert!(matches!(erreur, Erreur::Metadonnees(_)));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rbs-cli plan::tests::patcher`
Expected: FAIL — `no method named 'patcher' found`

- [ ] **Step 3: Write the implementation**

Ajouter la variante à `Erreur` :

```rust
    /// Le manifeste du projet n'a pas pu être patché.
    #[error("{0}")]
    Metadonnees(#[source] crate::metadata::Erreur),
```

Et la méthode :

```rust
    /// Planifie une modification du `Cargo.toml` de la racine.
    pub fn patcher(&mut self, patch: PatchToml) -> Result<(), Erreur> {
        let chemin = "Cargo.toml";

        let avant = self.etat_courant(chemin)?.ok_or_else(|| {
            Erreur::Metadonnees(crate::metadata::Erreur::PasUnProjet {
                chemin: chemin.to_string(),
            })
        })?;

        let PatchToml::InscrireFeature(feature) = &patch;
        let rendu = crate::metadata::inscrire_feature(&avant, feature, chemin)
            .map_err(Erreur::Metadonnees)?;

        let (apres, statut) = match rendu {
            Some(apres) => (apres, Statut::AFaire),
            None => (avant.clone(), Statut::DejaFait),
        };

        self.projeter(chemin, Some(avant), apres);
        self.actions.push(Action {
            chemin: chemin.to_string(),
            effet: Effet::PatcherToml { patch },
            statut,
        });

        Ok(())
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rbs-cli plan::`
Expected: PASS, 11 tests

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-cli/src/plan/mod.rs
git commit -m "feat(cli): planifie le patch du manifeste du projet

L'inscription d'une feature déjà présente se marque sans effet dès la
planification : c'est là que se décide l'idempotence, et non au moment
d'écrire.

Vérifications :
- cargo test -p rbs-cli plan:: -> 11 passed"
```

---

### Task 5: La preuve d'absence d'effet de bord

C'est le critère de la tâche. Un test qui vérifie l'absence d'un fichier attendu ne suffit pas : il n'attrape pas une écriture non prévue. On prend une empreinte du répertoire entier, avant et après.

**Files:**
- Modify: `crates/rbs-cli/src/plan/mod.rs` (module `tests`)

**Interfaces:**
- Consumes: tout le module.
- Produces: rien — c'est un test.

- [ ] **Step 1: Write the failing test**

```rust
    /// Chemin et contenu de chaque fichier du répertoire, trié : deux empreintes égales
    /// valent répertoires identiques.
    fn empreinte(racine: &Path) -> Vec<(String, Vec<u8>)> {
        let mut vus = Vec::new();
        let mut a_parcourir = vec![racine.to_path_buf()];

        while let Some(repertoire) = a_parcourir.pop() {
            for entree in fs::read_dir(&repertoire).expect("le répertoire se lit") {
                let chemin = entree.expect("l'entrée se lit").path();
                if chemin.is_dir() {
                    a_parcourir.push(chemin);
                } else {
                    let relatif = chemin
                        .strip_prefix(racine)
                        .expect("le chemin est sous la racine")
                        .display()
                        .to_string();
                    vus.push((relatif, fs::read(&chemin).expect("le fichier se lit")));
                }
            }
        }

        vus.sort();
        vus
    }

    #[test]
    fn planifier_ne_modifie_pas_le_repertoire_du_projet() {
        let projet = projet();
        fs::write(projet.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        avec_router(&projet, ROUTER);
        fs::write(projet.path().join("Dockerfile"), "FROM alpine\n").expect("l'écriture aboutit");

        let avant = empreinte(projet.path());

        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());
        constructeur.creer("Dockerfile", "FROM rust\n").expect("le fichier se lit");
        constructeur.creer("docker-compose.yml", "services:\n").expect("le fichier est absent");
        constructeur
            .inserer(ancres::ROUTES, &[".merge(crate::users::routes())".to_string()])
            .expect("l'ancre est présente");
        constructeur
            .patcher(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        let plan = constructeur.finir();

        assert_eq!(empreinte(projet.path()), avant, "la planification a touché au disque");
        assert_eq!(plan.actions().len(), 4);
        assert_eq!(plan.fichiers().len(), 4);
    }
```

- [ ] **Step 2: Run test to verify it passes**

Ce test doit passer du premier coup : il ne demande rien de neuf au code, il prouve une propriété. S'il échoue, c'est un vrai défaut du module — corriger le module, pas le test.

Run: `cargo test -p rbs-cli plan::tests::planifier_ne_modifie_pas`
Expected: PASS

Pour se convaincre que le test attrape ce qu'il prétend attraper, y ajouter temporairement une écriture (`fs::write(projet.path().join("intrus"), "x").unwrap();` juste avant l'empreinte finale), vérifier que le test échoue, puis la retirer.

- [ ] **Step 3: Run the crate's full test suite, clippy and fmt**

```bash
cargo test -p rbs-cli --bins
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```
Expected: PASS sur les trois.

- [ ] **Step 4: Commit**

```bash
git add crates/rbs-cli/src/plan/mod.rs
git commit -m "test(cli): prouve qu'une planification laisse le projet intact

L'empreinte du répertoire entier est comparée avant et après une
planification de quatre actions. Vérifier l'absence des fichiers attendus
aurait laissé passer une écriture non prévue.

Vérifications :
- cargo test -p rbs-cli plan:: -> 12 passed
- cargo clippy --workspace --all-targets -- -D warnings -> propre
- cargo fmt --all --check -> propre"
```

---

## Après le plan

Cocher `E1` dans `TODO.md` avec la preuve exécutée, sur une seule ligne, puis
`superpowers:finishing-a-development-branch`.
