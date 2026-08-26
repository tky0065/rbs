//! État d'une migration, tel que rapporté par la crate `migration` du projet.
//!
//! Le binaire du projet écrit un état par ligne, `applied` ou `pending` suivi du nom.
//! Ce format existe pour être analysé : la mise en forme lisible est le travail de rbs,
//! qui la contrôle ainsi sans dépendre de ce que le projet imprime.

/// Une migration et son état vis-à-vis de la base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etat {
    /// Nom du module de migration, tel que déclaré dans `migration/src/lib.rs`.
    pub nom: String,
    /// Vrai si la migration est déjà appliquée sur la base visée.
    pub appliquee: bool,
}

/// Ce qui peut empêcher de comprendre la sortie du sous-processus.
#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    /// Une ligne ne suit pas le format attendu.
    #[error("la crate migration a répondu une ligne incomprise : {ligne}")]
    Ligne {
        /// La ligne fautive, telle que reçue.
        ligne: String,
    },
}

/// Analyse la sortie du binaire de migration du projet.
pub fn analyser(sortie: &str) -> Result<Vec<Etat>, Erreur> {
    sortie
        .lines()
        .filter(|ligne| !ligne.trim().is_empty())
        .map(analyser_ligne)
        .collect()
}

fn analyser_ligne(ligne: &str) -> Result<Etat, Erreur> {
    let incomprise = || Erreur::Ligne {
        ligne: ligne.to_string(),
    };

    let (etat, nom) = ligne.trim_end().split_once('\t').ok_or_else(incomprise)?;

    let appliquee = match etat {
        "applied" => true,
        "pending" => false,
        _ => return Err(incomprise()),
    };

    Ok(Etat {
        nom: nom.to_string(),
        appliquee,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_deux_etats_sont_reconnus_dans_l_ordre() {
        let etats = analyser(
            "applied\tm20260826_120000_creer_carnets\npending\tm20260826_133000_creer_notes\n",
        )
        .expect("la sortie est bien formée");

        assert_eq!(
            etats,
            vec![
                Etat {
                    nom: "m20260826_120000_creer_carnets".to_string(),
                    appliquee: true,
                },
                Etat {
                    nom: "m20260826_133000_creer_notes".to_string(),
                    appliquee: false,
                },
            ]
        );
    }

    #[test]
    fn une_sortie_vide_donne_aucune_migration() {
        assert_eq!(analyser("\n").expect("le vide est valide"), vec![]);
    }

    #[test]
    fn une_ligne_incomprise_est_signalee_avec_son_contenu() {
        let erreur = analyser("error: connexion refusée\n").expect_err("la ligne est invalide");

        assert!(erreur.to_string().contains("error: connexion refusée"));
    }

    #[test]
    fn un_etat_inconnu_est_refuse_plutot_que_suppose_en_attente() {
        let erreur = analyser("skipped\tm20260826_120000_creer_carnets\n")
            .expect_err("`skipped` n'est pas un état connu");

        assert!(erreur.to_string().contains("skipped"));
    }
}
