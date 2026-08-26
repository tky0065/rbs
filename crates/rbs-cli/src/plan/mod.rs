//! La planification d'une commande qui modifie un projet, réifiée en valeur.
//!
//! Un plan est une liste d'actions ; chaque action vise un fichier et connaît son contenu
//! avant et son contenu après. Planifier, c'est calculer les « après » sans rien écrire —
//! d'où l'affichage préalable, la restauration en cas d'échec et l'idempotence.

// Le module précède ses appelants : `rbs add` n'est pas encore implémentée. `PatchToml`
// n'est réexportée que pour cette future commande, d'où l'import inutilisé aujourd'hui.
#![allow(dead_code, unused_imports)]

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
}
