//! `rbs doctor` : diagnostic d'un projet généré.
//!
//! Chaque contrôle est indépendant et rend son verdict sans interrompre les autres : un
//! diagnostic qui s'arrête au premier problème oblige à le relancer autant de fois qu'il
//! y a de problèmes.

pub mod ancres;
pub mod base;
pub mod env;
pub mod rendu;
pub mod versions;

use std::path::Path;

use crate::metadata;

/// Verdict d'un contrôle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Etat {
    /// Rien à signaler.
    Bon,
    /// Ce qui empêche le projet de fonctionner.
    Echec,
}

/// Ce qu'un contrôle a constaté.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Controle {
    /// Ce qui est vérifié, en un mot : `ancres`, `.env`, `versions`, `base`.
    pub titre: &'static str,
    /// Verdict.
    pub etat: Etat,
    /// Ce qui a été constaté, en une ligne.
    pub detail: String,
    /// Quoi faire, quand il y a quelque chose à faire.
    pub remede: Option<String>,
}

impl Controle {
    /// Un contrôle sans rien à signaler.
    pub(crate) fn bon(titre: &'static str, detail: impl Into<String>) -> Self {
        Self {
            titre,
            etat: Etat::Bon,
            detail: detail.into(),
            remede: None,
        }
    }

    /// Un contrôle en échec, et le geste qui le corrige.
    pub(crate) fn echec(
        titre: &'static str,
        detail: impl Into<String>,
        remede: impl Into<String>,
    ) -> Self {
        Self {
            titre,
            etat: Etat::Echec,
            detail: detail.into(),
            remede: Some(remede.into()),
        }
    }
}

/// L'ensemble des constats, dans l'ordre où ils ont été faits.
#[derive(Debug)]
pub(crate) struct Rapport {
    /// Les contrôles, tous exécutés.
    pub controles: Vec<Controle>,
}

impl Rapport {
    /// Vrai si aucun contrôle n'a échoué.
    pub(crate) fn reussi(&self) -> bool {
        self.controles.iter().all(|c| c.etat == Etat::Bon)
    }
}

/// Ce qui peut empêcher de diagnostiquer.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error(
        "cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici"
    )]
    PasUnProjet,
}

/// Diagnostique le projet qui contient `repertoire`.
pub(crate) fn executer(repertoire: &Path) -> Result<Rapport, Erreur> {
    let racine = metadata::racine_du_projet(repertoire).ok_or(Erreur::PasUnProjet)?;

    let controles = vec![
        ancres::controler(&racine),
        env::controler(&racine),
        versions::controler(&racine),
        base::controler(&racine),
    ];

    Ok(Rapport { controles })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn hors_d_un_projet_rbs_rien_n_est_diagnostique() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let erreur = executer(ailleurs.path()).expect_err("ce n'est pas un projet");

        assert!(matches!(erreur, Erreur::PasUnProjet));
    }

    #[test]
    fn un_rapport_sans_echec_est_reussi() {
        let rapport = Rapport {
            controles: vec![Controle::bon("ancres", "les 5 sont en place")],
        };

        assert!(rapport.reussi());
    }

    #[test]
    fn un_seul_echec_fait_echouer_le_rapport() {
        let rapport = Rapport {
            controles: vec![
                Controle::bon("ancres", "les 5 sont en place"),
                Controle::echec(".env", "RBS_ENV manque", "ajoutez RBS_ENV"),
            ],
        };

        assert!(!rapport.reussi());
    }
}
