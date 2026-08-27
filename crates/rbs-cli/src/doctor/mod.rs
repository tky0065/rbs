//! `rbs doctor` : diagnostic d'un projet généré.
//!
//! Chaque contrôle est indépendant et rend son verdict sans interrompre les autres : un
//! diagnostic qui s'arrête au premier problème oblige à le relancer autant de fois qu'il
//! y a de problèmes.

pub mod ancres;
pub mod auth;
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

    let mut controles = vec![
        ancres::controler(&racine),
        env::controler(&racine),
        versions::controler(&racine),
        base::controler(&racine),
    ];

    // Un projet qui n'a pas installé `auth` n'a pas à lire une ligne à son sujet : le
    // rapport ne porte que des contrôles dont le verdict le concerne.
    if feature_installee(&racine, "auth") {
        controles.push(auth::controler(&racine));
    }

    Ok(Rapport { controles })
}

/// Vrai si `nom` figure dans `[package.metadata.rbs].features`.
fn feature_installee(racine: &Path, nom: &str) -> bool {
    metadata::lire(&racine.join("Cargo.toml"))
        .is_ok_and(|metadonnees| metadonnees.features.iter().any(|feature| feature == nom))
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

    /// Un projet neuf, dont les features sont celles passées.
    fn projet(features: &[&str]) -> (TempDir, std::path::PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let projet = crate::new::creer(
            &crate::new::Options {
                nom: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let manifeste = projet.racine.join("Cargo.toml");
        let source = std::fs::read_to_string(&manifeste).expect("manifeste lisible");
        let declarees = features
            .iter()
            .map(|feature| format!("\"{feature}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            &manifeste,
            source.replace(
                "features = [\"health\"]",
                &format!("features = [{declarees}]"),
            ),
        )
        .expect("manifeste inscriptible");

        (parent, projet.racine)
    }

    fn titres(rapport: &Rapport) -> Vec<&'static str> {
        rapport.controles.iter().map(|c| c.titre).collect()
    }

    #[test]
    fn un_projet_sans_auth_n_a_pas_de_controle_auth() {
        let (_parent, racine) = projet(&["health"]);

        let rapport = executer(&racine).expect("c'est un projet rbs");

        assert!(
            !titres(&rapport).contains(&"auth"),
            "un projet sans auth n'a pas à lire une ligne à son sujet : {:?}",
            titres(&rapport)
        );
    }

    #[test]
    fn un_projet_portant_auth_recoit_son_controle() {
        let (_parent, racine) = projet(&["health", "auth"]);

        let rapport = executer(&racine).expect("c'est un projet rbs");

        assert!(
            titres(&rapport).contains(&"auth"),
            "la feature est déclarée, son contrôle doit figurer : {:?}",
            titres(&rapport)
        );
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
