//! Contrôle des points d'insertion du projet.
//!
//! Une ancre disparue ne casse rien tant qu'aucune génération n'a lieu : c'est
//! précisément pourquoi `doctor` la cherche avant que `rbs generate` ne bute dessus.

use std::fs;
use std::path::Path;

use crate::ancres::{ANCRES, Ancre};

use super::Controle;

const TITRE: &str = "ancres";

/// Vérifie que le projet porte toutes ses ancres, et dit comment recoller les absentes.
pub(crate) fn controler(racine: &Path) -> Controle {
    let absentes: Vec<&Ancre> = ANCRES.iter().filter(|a| !presente(racine, a)).collect();

    if absentes.is_empty() {
        return Controle::bon(
            TITRE,
            format!("les {} points d'insertion sont en place", ANCRES.len()),
        );
    }

    let detail = absentes
        .iter()
        .map(|a| format!("{} manque dans {}", a.nom, a.fichier))
        .collect::<Vec<_>>()
        .join(", ");

    let remede = absentes
        .iter()
        .map(|a| format!("dans {} :\n{}", a.fichier, a.bloc()))
        .collect::<Vec<_>>()
        .join("\n\n");

    Controle::echec(TITRE, detail, remede)
}

/// Vrai si le fichier porteur existe et contient les deux balises de l'ancre.
///
/// Un fichier illisible vaut ancre absente : le diagnostic le signale par le nom du
/// fichier plutôt que de s'interrompre.
fn presente(racine: &Path, ancre: &Ancre) -> bool {
    fs::read_to_string(racine.join(ancre.fichier)).is_ok_and(|source| {
        source.contains(&ancre.ouverture()) && source.contains(&ancre.fermeture())
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::Etat;
    use super::*;

    fn projet() -> (TempDir, PathBuf) {
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

        (parent, projet.racine)
    }

    /// Retire du projet la ligne portant `motif`.
    fn retirer(racine: &Path, fichier: &str, motif: &str) {
        let chemin = racine.join(fichier);
        let source = fs::read_to_string(&chemin).expect("le fichier est lisible");
        let ampute: Vec<_> = source.lines().filter(|l| !l.contains(motif)).collect();
        fs::write(&chemin, ampute.join("\n")).expect("le fichier est réécrivable");
    }

    #[test]
    fn un_projet_neuf_porte_toutes_ses_ancres() {
        let (_parent, racine) = projet();

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Bon);
        assert!(
            controle.detail.contains(&ANCRES.len().to_string()),
            "{}",
            controle.detail
        );
        assert!(controle.remede.is_none());
    }

    #[test]
    fn une_ancre_supprimee_est_signalee_avec_le_bloc_a_recoller() {
        let (_parent, racine) = projet();
        retirer(&racine, "src/router.rs", "<rbs:routes>");
        retirer(&racine, "src/router.rs", "</rbs:routes>");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains("routes"));
        assert!(controle.detail.contains("src/router.rs"));

        let remede = controle.remede.expect("un échec porte son remède");
        assert!(remede.contains("// <rbs:routes>"));
        assert!(remede.contains("// </rbs:routes>"));
        assert!(
            remede.contains("src/router.rs"),
            "le remède dit où coller le bloc"
        );
    }

    #[test]
    fn une_ancre_dont_la_fermeture_manque_compte_pour_absente() {
        let (_parent, racine) = projet();
        retirer(&racine, "src/router.rs", "</rbs:routes>");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains("routes"));
    }

    #[test]
    fn les_deux_ancres_d_un_meme_fichier_sont_verifiees_separement() {
        let (_parent, racine) = projet();
        retirer(&racine, "migration/src/lib.rs", "<rbs:migrations>");
        retirer(&racine, "migration/src/lib.rs", "</rbs:migrations>");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains("migrations"));
        assert!(
            !controle.detail.contains("migration_modules"),
            "l'autre ancre du fichier est intacte"
        );
    }

    #[test]
    fn un_fichier_disparu_est_signale_plutot_que_de_faire_paniquer_le_diagnostic() {
        let (_parent, racine) = projet();
        fs::remove_file(racine.join("src/openapi.rs")).expect("le fichier existe");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains("src/openapi.rs"));
    }
}
