//! Contrôle du `.env` du projet.
//!
//! `.env.example` sert de référence : il est versionné, généré par le squelette et mis à
//! jour en même temps que lui. Comparer à une liste tenue dans le CLI aurait fait deux
//! vérités à synchroniser.

use std::path::Path;

use crate::dotenv;

use super::Controle;

const TITRE: &str = ".env";
const FICHIER: &str = ".env";
const EXEMPLE: &str = ".env.example";

/// Vérifie que le `.env` porte tout ce que `.env.example` déclare.
pub(crate) fn controler(racine: &Path) -> Controle {
    let attendues = match dotenv::lire(&racine.join(EXEMPLE)) {
        Ok(paires) => paires,
        Err(erreur) => {
            return Controle::echec(
                TITRE,
                erreur.to_string(),
                format!("{EXEMPLE} est la référence du diagnostic : restaurez-le depuis Git"),
            );
        }
    };

    let presentes = match dotenv::lire(&racine.join(FICHIER)) {
        Ok(paires) => paires,
        Err(erreur) => {
            return Controle::echec(
                TITRE,
                erreur.to_string(),
                format!("cp {EXEMPLE} {FICHIER}, puis renseignez l'URL de votre base"),
            );
        }
    };

    // Une variable propre au projet est légitime : seule l'absence est un défaut.
    let manquantes: Vec<&str> = attendues
        .iter()
        .map(|(cle, _)| cle.as_str())
        .filter(|cle| dotenv::valeur(&presentes, cle).is_none())
        .collect();

    if manquantes.is_empty() {
        return Controle::bon(
            TITRE,
            format!(
                "les {} variables de {EXEMPLE} sont renseignées",
                attendues.len()
            ),
        );
    }

    Controle::echec(
        TITRE,
        format!(
            "{} absente{} du {FICHIER}",
            manquantes.join(", "),
            if manquantes.len() > 1 { "s" } else { "" }
        ),
        format!(
            "ajoutez au {FICHIER} :\n{}",
            manquantes
                .iter()
                .map(|cle| format!("{cle}={}", dotenv::valeur(&attendues, cle).unwrap_or("")))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
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

    /// Retire du `.env` la ligne portant `cle`.
    fn retirer(racine: &Path, cle: &str) {
        let chemin = racine.join(FICHIER);
        let source = fs::read_to_string(&chemin).expect("le .env est lisible");
        let ampute: Vec<_> = source
            .lines()
            .filter(|ligne| !ligne.starts_with(cle))
            .collect();
        fs::write(&chemin, ampute.join("\n")).expect("le .env est réécrivable");
    }

    #[test]
    fn un_projet_neuf_a_un_env_complet() {
        let (_parent, racine) = projet();

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Bon, "{}", controle.detail);
        assert!(controle.remede.is_none());
    }

    #[test]
    fn une_variable_de_l_exemple_absente_du_env_est_nommee() {
        let (_parent, racine) = projet();
        retirer(&racine, "RBS_LOG_FORMAT");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains("RBS_LOG_FORMAT"));
        assert!(
            controle
                .remede
                .expect("un échec porte son remède")
                .contains("RBS_LOG_FORMAT")
        );
    }

    #[test]
    fn le_constat_s_accorde_avec_le_nombre_de_variables_manquantes() {
        let (_parent, racine) = projet();
        retirer(&racine, "RBS_LOG_FORMAT");

        assert!(controler(&racine).detail.contains("absente du"));

        retirer(&racine, "RUST_LOG");

        assert!(controler(&racine).detail.contains("absentes du"));
    }

    #[test]
    fn une_variable_propre_au_projet_ne_derange_pas() {
        let (_parent, racine) = projet();
        let chemin = racine.join(FICHIER);
        let source = fs::read_to_string(&chemin).expect("le .env est lisible");
        fs::write(&chemin, format!("{source}\nSTRIPE_KEY=sk_test\n")).expect("écriture");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Bon, "{}", controle.detail);
    }

    #[test]
    fn un_env_absent_renvoie_a_l_exemple_qui_le_reconstitue() {
        let (_parent, racine) = projet();
        fs::remove_file(racine.join(FICHIER)).expect("le .env existe");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(
            controle
                .remede
                .expect("un échec porte son remède")
                .contains(EXEMPLE)
        );
    }

    #[test]
    fn sans_exemple_le_controle_le_dit_plutot_que_de_conclure_au_vert() {
        let (_parent, racine) = projet();
        fs::remove_file(racine.join(EXEMPLE)).expect("l'exemple existe");

        let controle = controler(&racine);

        assert_eq!(controle.etat, Etat::Echec);
        assert!(controle.detail.contains(EXEMPLE));
    }
}
