//! Mise en forme de l'inventaire des migrations.
//!
//! C'est ici que vit la lisibilité de `rbs migrate status` : la crate `migration` du
//! projet ne rapporte qu'un état par ligne, sans se soucier de l'affichage.

use super::etat::Etat;
use crate::ui;

const APPLIQUEE: &str = "appliquée";
const EN_ATTENTE: &str = "en attente";

/// Rend l'inventaire des migrations, une par ligne.
///
/// Les deux états se distinguent par leur puce et par leur libellé, jamais par la seule
/// couleur : la sortie reste lisible dans un `less`, un fichier de log ou une CI.
pub fn status(etats: &[Etat]) -> String {
    if etats.is_empty() {
        return "  aucune migration déclarée".to_string();
    }

    let largeur = etats
        .iter()
        .map(|etat| etat.nom.chars().count())
        .max()
        .unwrap_or(0);

    etats
        .iter()
        .map(|etat| ligne(etat, largeur))
        .collect::<Vec<_>>()
        .join("\n")
}

fn ligne(etat: &Etat, largeur: usize) -> String {
    let nom = format!("{:largeur$}", etat.nom);

    if etat.appliquee {
        format!("  {} {nom}   {}", ui::vert("✓"), ui::vert(APPLIQUEE))
    } else {
        format!("  {} {nom}   {}", ui::attenue("·"), ui::attenue(EN_ATTENTE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Colonne d'affichage d'un libellé, comptée en caractères : `find` rend des octets,
    /// et les deux puces n'en occupent pas le même nombre.
    fn colonne(ligne: &str, libelle: &str) -> usize {
        let octets = ligne.find(libelle).expect("le libellé est présent");
        ligne[..octets].chars().count()
    }

    fn etats() -> Vec<Etat> {
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
    }

    #[test]
    fn les_deux_etats_portent_des_marqueurs_distincts_sans_couleur() {
        let rendu = status(&etats());
        let mut lignes = rendu.lines();

        let appliquee = lignes.next().expect("la première migration est rendue");
        let en_attente = lignes.next().expect("la seconde migration est rendue");

        assert!(appliquee.contains('✓') && appliquee.contains("appliquée"));
        assert!(en_attente.contains('·') && en_attente.contains("en attente"));
        assert!(!appliquee.contains("en attente"));
        assert!(!en_attente.contains('✓'));
    }

    #[test]
    fn chaque_migration_est_nommee() {
        let rendu = status(&etats());

        assert!(rendu.contains("m20260826_120000_creer_carnets"));
        assert!(rendu.contains("m20260826_133000_creer_notes"));
    }

    #[test]
    fn les_libelles_sont_alignes_sur_le_nom_le_plus_long() {
        let rendu = status(&[
            Etat {
                nom: "m1_court".to_string(),
                appliquee: true,
            },
            Etat {
                nom: "m2_un_nom_nettement_plus_long".to_string(),
                appliquee: false,
            },
        ]);

        let mut lignes = rendu.lines();
        let appliquee = lignes.next().expect("la première ligne est rendue");
        let en_attente = lignes.next().expect("la seconde ligne est rendue");

        assert_eq!(
            colonne(appliquee, APPLIQUEE),
            colonne(en_attente, EN_ATTENTE),
            "les libellés commencent à la même colonne"
        );
    }

    #[test]
    fn un_projet_sans_migration_le_dit_plutot_que_de_rendre_le_vide() {
        let rendu = status(&[]);

        assert!(rendu.contains("aucune migration"));
    }
}
