//! Mise en forme du rapport de diagnostic.
//!
//! Un remède se lit sous le constat qui l'appelle, indenté : un diagnostic qui renvoie
//! ses remèdes en bas de page oblige à faire l'aller-retour.

use crate::ui;

use super::{Etat, Rapport};

/// Retrait des remèdes, aligné sous le détail des constats.
const RETRAIT: &str = "      ";

/// Rend le rapport, un contrôle par ligne, remèdes compris.
pub(crate) fn rapport(rapport: &Rapport) -> String {
    let largeur = rapport
        .controles
        .iter()
        .map(|controle| controle.titre.chars().count())
        .max()
        .unwrap_or(0);

    let mut lignes = Vec::new();

    for controle in &rapport.controles {
        let titre = format!("{:largeur$}", controle.titre);

        lignes.push(match controle.etat {
            Etat::Bon => format!("  {} {titre}   {}", ui::vert("✓"), controle.detail),
            Etat::Echec => format!("  {} {titre}   {}", ui::rouge("✗"), controle.detail),
        });

        if let Some(remede) = &controle.remede {
            lignes.extend(
                remede
                    .lines()
                    .map(|ligne| format!("{RETRAIT}{}", ui::attenue(ligne))),
            );
        }
    }

    lignes.join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::Controle;
    use super::*;

    fn rapport_de(controles: Vec<Controle>) -> Rapport {
        Rapport { controles }
    }

    #[test]
    fn les_deux_verdicts_portent_des_marqueurs_distincts_sans_couleur() {
        let rendu = rapport(&rapport_de(vec![
            Controle::bon("ancres", "les 5 sont en place"),
            Controle::echec(".env", "RBS_ENV manque", "ajoutez RBS_ENV=development"),
        ]));
        let mut lignes = rendu.lines();

        let bon = lignes.next().expect("le premier contrôle est rendu");
        let echec = lignes.next().expect("le second contrôle est rendu");

        assert!(bon.contains('✓') && bon.contains("ancres"));
        assert!(echec.contains('✗') && echec.contains(".env"));
        assert!(!bon.contains('✗'));
    }

    #[test]
    fn le_remede_suit_son_constat_en_retrait() {
        let rendu = rapport(&rapport_de(vec![Controle::echec(
            ".env",
            "RBS_ENV manque",
            "ajoutez RBS_ENV=development",
        )]));

        let remede = rendu
            .lines()
            .find(|ligne| ligne.contains("ajoutez RBS_ENV=development"))
            .expect("le remède est rendu");

        assert!(
            remede.starts_with("      "),
            "le remède est en retrait du constat : « {remede} »"
        );
    }

    #[test]
    fn un_remede_sur_plusieurs_lignes_est_entierement_en_retrait() {
        let rendu = rapport(&rapport_de(vec![Controle::echec(
            "ancres",
            "routes manque",
            "dans src/router.rs :\n// <rbs:routes>\n// </rbs:routes>",
        )]));

        for ligne in rendu.lines().skip(1).filter(|l| !l.trim().is_empty()) {
            assert!(
                ligne.starts_with("      "),
                "chaque ligne du remède est en retrait : « {ligne} »"
            );
        }
    }

    #[test]
    fn un_controle_sans_reproche_n_ajoute_aucune_ligne() {
        let rendu = rapport(&rapport_de(vec![Controle::bon("ancres", "les 5 sont là")]));

        assert_eq!(rendu.lines().count(), 1);
    }

    #[test]
    fn les_details_sont_alignes_sur_le_titre_le_plus_long() {
        let rendu = rapport(&rapport_de(vec![
            Controle::bon("base", "PostgreSQL 18.1"),
            Controle::bon("versions", "alignées"),
        ]));

        let colonne = |ligne: &str, detail: &str| {
            let octets = ligne.find(detail).expect("le détail est présent");
            ligne[..octets].chars().count()
        };

        let mut lignes = rendu.lines();
        let premiere = lignes.next().expect("première ligne");
        let seconde = lignes.next().expect("seconde ligne");

        assert_eq!(
            colonne(premiere, "PostgreSQL 18.1"),
            colonne(seconde, "alignées")
        );
    }
}
