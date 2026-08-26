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

use crate::ancres::Ancre;

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
///
/// Chaque variante nomme son fichier relativement à la racine, comme `Action::chemin` :
/// l'emplacement complet du projet est porté une seule fois, par l'en-tête de l'affichage
/// du plan.
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
    /// Une ancre attendue a disparu du projet.
    #[error("{0}")]
    Ancre(#[source] crate::ancres::Absente),
    /// Le manifeste du projet n'a pas pu être patché.
    #[error("{0}")]
    Metadonnees(#[source] crate::metadata::Erreur),
    /// Le `Cargo.toml` visé par un patch n'existe pas à l'emplacement attendu.
    ///
    /// Distincte de `Metadonnees(PasUnProjet)`, qui suppose au contraire un fichier
    /// présent mais dépourvu de la section `[package.metadata.rbs]`.
    #[error("{chemin} est introuvable")]
    ManifesteAbsent {
        /// Chemin du manifeste, relatif à la racine.
        chemin: String,
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

    /// Planifie l'ajout de `lignes` dans `ancre`, juste avant sa balise fermante.
    ///
    /// Le fichier visé est celui que l'ancre désigne : une ancre ne se déplace pas.
    pub fn inserer(&mut self, ancre: Ancre, lignes: &[String]) -> Result<(), Erreur> {
        let chemin = ancre.fichier;

        let avant = self
            .etat_courant(chemin)?
            .ok_or(Erreur::Ancre(crate::ancres::Absente { ancre }))?;

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

    /// Planifie une modification du `Cargo.toml` de la racine.
    pub fn patcher(&mut self, patch: PatchToml) -> Result<(), Erreur> {
        let chemin = "Cargo.toml";

        let avant = self
            .etat_courant(chemin)?
            .ok_or_else(|| Erreur::ManifesteAbsent {
                chemin: chemin.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ancres;
    use std::fs;
    use tempfile::TempDir;

    fn projet() -> TempDir {
        TempDir::new().expect("le répertoire temporaire se crée")
    }

    const ROUTER: &str = "pub fn router() -> Router {\n    Router::new()\n        // <rbs:routes>\n        // </rbs:routes>\n}\n";

    fn avec_router(projet: &TempDir, source: &str) {
        fs::create_dir_all(projet.path().join("src")).expect("le répertoire se crée");
        fs::write(projet.path().join("src/router.rs"), source).expect("l'écriture aboutit");
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

        constructeur
            .creer("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::DejaFait);
        assert_eq!(plan.fichiers()[0].avant.as_deref(), Some("FROM rust\n"));
    }

    #[test]
    fn creer_par_dessus_un_contenu_different_est_un_conflit() {
        let projet = projet();
        fs::write(projet.path().join("Dockerfile"), "FROM alpine\n").expect("l'écriture aboutit");
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .creer("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::Conflit);
        assert_eq!(plan.fichiers()[0].avant.as_deref(), Some("FROM alpine\n"));
        assert_eq!(plan.fichiers()[0].apres, "FROM rust\n");
    }

    #[test]
    fn planifier_une_creation_n_ecrit_pas_le_fichier() {
        let projet = projet();
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .creer("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent");
        constructeur.finir();

        assert!(!projet.path().join("Dockerfile").exists());
    }

    #[test]
    fn inserer_dans_une_ancre_vide_est_a_faire() {
        let projet = projet();
        avec_router(&projet, ROUTER);
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .inserer(
                ancres::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::AFaire);
        assert_eq!(plan.actions()[0].chemin, "src/router.rs");
        assert!(
            plan.fichiers()[0]
                .apres
                .contains(".merge(crate::users::routes())")
        );
    }

    #[test]
    fn inserer_une_ligne_deja_presente_est_deja_fait() {
        let projet = projet();
        avec_router(
            &projet,
            &ROUTER.replace(
                "        // </rbs:routes>",
                "        .merge(crate::users::routes())\n        // </rbs:routes>",
            ),
        );
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .inserer(
                ancres::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = constructeur.finir();

        assert_eq!(plan.actions()[0].statut, Statut::DejaFait);
        assert_eq!(
            plan.fichiers()[0].avant.as_deref(),
            Some(plan.fichiers()[0].apres.as_str())
        );
    }

    #[test]
    fn deux_insertions_dans_un_meme_fichier_se_chainent_sur_un_seul_fichier() {
        let projet = projet();
        let lib = "// <rbs:migration_modules>\n// </rbs:migration_modules>\nvec![\n    // <rbs:migrations>\n    // </rbs:migrations>\n]\n";
        fs::create_dir_all(projet.path().join("migration/src")).expect("le répertoire se crée");
        fs::write(projet.path().join("migration/src/lib.rs"), lib).expect("l'écriture aboutit");
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        constructeur
            .inserer(
                ancres::MIGRATION_MODULES,
                &["mod m20260826_creer_users;".to_string()],
            )
            .expect("l'ancre est présente");
        constructeur
            .inserer(
                ancres::MIGRATIONS,
                &["Box::new(m20260826_creer_users::Migration),".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = constructeur.finir();

        assert_eq!(plan.actions().len(), 2);
        assert_eq!(plan.fichiers().len(), 1);
        assert!(
            plan.fichiers()[0]
                .apres
                .contains("mod m20260826_creer_users;")
        );
        assert!(
            plan.fichiers()[0]
                .apres
                .contains("Box::new(m20260826_creer_users::Migration),")
        );
        assert_eq!(plan.fichiers()[0].avant.as_deref(), Some(lib));
    }

    #[test]
    fn une_ancre_absente_interrompt_la_planification() {
        let projet = projet();
        avec_router(
            &projet,
            "pub fn router() -> Router {\n    Router::new()\n}\n",
        );
        let mut constructeur = Constructeur::nouveau(projet.path().to_path_buf());

        let erreur = constructeur
            .inserer(
                ancres::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("l'ancre manque");

        assert!(matches!(erreur, Erreur::Ancre(_)));
    }

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

        assert!(matches!(erreur, Erreur::ManifesteAbsent { .. }));
        let message = erreur.to_string();
        assert!(
            message.starts_with("Cargo.toml"),
            "le message ne nomme pas le manifeste comme `Action::chemin` : {message}"
        );
        assert!(
            !message.contains(&projet.path().display().to_string()),
            "le message porte un chemin absolu : {message}"
        );
        assert!(
            message.contains("introuvable"),
            "le message ne dit pas que le fichier manque : {message}"
        );
    }

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
        constructeur
            .creer("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        constructeur
            .creer("docker-compose.yml", "services:\n")
            .expect("le fichier est absent");
        constructeur
            .inserer(
                ancres::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        constructeur
            .patcher(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        let plan = constructeur.finir();

        assert_eq!(
            empreinte(projet.path()),
            avant,
            "la planification a touché au disque"
        );
        assert_eq!(plan.actions().len(), 4);
        assert_eq!(plan.fichiers().len(), 4);
    }
}
