//! `rbs migrate` : pilotage des migrations d'un projet généré.
//!
//! `up`, `down` et `status` enveloppent le binaire de la crate `migration` du projet :
//! le moteur de SeaORM n'est pas réimplémenté, seulement rendu lisible. `new` n'a besoin
//! de personne — ni de cargo, ni d'une base démarrée.

pub mod etat;
pub mod nouvelle;
pub mod rendu;

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::generate::migration::horodatage_courant;
use crate::{dotenv, metadata};

/// La variable qui porte l'URL de la base, telle que le projet la nomme.
///
/// C'est celle de la configuration du noyau — `RBS_DATABASE__URL` alimente
/// `database.url` — et non un `DATABASE_URL` que rbs serait seul à connaître.
const URL: &str = "RBS_DATABASE__URL";

/// Ce que `rbs migrate` peut faire.
#[derive(Debug)]
pub(crate) enum Action {
    /// Applique les migrations en attente.
    Up,
    /// Annule la dernière migration appliquée.
    Down,
    /// Inventorie les migrations et leur état.
    Status,
    /// Crée un fichier de migration vide.
    Nouvelle(String),
}

/// Ce qu'une action a produit, à afficher.
#[derive(Debug)]
pub(crate) enum Sortie {
    /// Les migrations en attente ont été appliquées.
    Appliquees,
    /// La dernière migration appliquée a été annulée.
    Annulee,
    /// L'inventaire, déjà mis en forme.
    Inventaire(String),
    /// La migration créée.
    Creee(nouvelle::Nouvelle),
}

/// Ce qui peut empêcher de piloter les migrations.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// La commande n'a pas été lancée depuis un projet rbs.
    #[error(
        "cette commande attend un projet rbs : aucun Cargo.toml portant [package.metadata.rbs] au-dessus d'ici"
    )]
    PasUnProjet,

    /// Le `.env` du projet est absent ou illisible.
    #[error("{0}")]
    Env(#[from] dotenv::Erreur),

    /// Le `.env` ne dit pas quelle base viser.
    #[error("{URL} est absente du .env : rbs ne sait pas quelle base migrer")]
    SansUrl,

    /// `cargo` n'a pas pu être lancé.
    #[error("cargo n'a pas pu être lancé : {0}")]
    Cargo(#[source] io::Error),

    /// Le binaire de migration a échoué.
    #[error("la crate migration a échoué (code {code})")]
    Migration {
        /// Code de sortie du sous-processus.
        code: i32,
    },

    /// La sortie du binaire de migration n'a pas pu être analysée.
    #[error("{0}")]
    Etat(#[from] etat::Erreur),

    /// La migration n'a pas pu être créée.
    #[error("{0}")]
    Nouvelle(#[from] nouvelle::Erreur),
}

/// Exécute `action` dans le projet qui contient `repertoire`.
pub(crate) fn executer(action: Action, repertoire: &Path) -> Result<Sortie, Erreur> {
    let racine = metadata::racine_du_projet(repertoire).ok_or(Erreur::PasUnProjet)?;

    if let Action::Nouvelle(nom) = action {
        return Ok(Sortie::Creee(nouvelle::executer(
            &racine,
            &nom,
            &horodatage_courant(),
        )?));
    }

    let paires = dotenv::lire(&racine.join(".env"))?;
    let variables = preparer(paires, |cle| std::env::var_os(cle).is_some())?;

    match action {
        Action::Up => {
            lancer(&racine, "up", &variables, false)?;
            Ok(Sortie::Appliquees)
        }
        Action::Down => {
            lancer(&racine, "down", &variables, false)?;
            Ok(Sortie::Annulee)
        }
        Action::Status => {
            let sortie = lancer(&racine, "status", &variables, true)?;
            Ok(Sortie::Inventaire(rendu::status(&etat::analyser(&sortie)?)))
        }
        Action::Nouvelle(_) => unreachable!("traitée avant la lecture du .env"),
    }
}

/// Retient du `.env` ce que le sous-processus n'a pas déjà, et exige de savoir quelle
/// base viser.
///
/// L'environnement de l'appelant l'emporte : `RBS_DATABASE__URL=… rbs migrate up` doit
/// pouvoir viser une autre base sans toucher au fichier du projet.
fn preparer(
    paires: Vec<(String, String)>,
    definie: impl Fn(&str) -> bool,
) -> Result<Vec<(String, String)>, Erreur> {
    if !definie(URL) && dotenv::valeur(&paires, URL).is_none() {
        return Err(Erreur::SansUrl);
    }

    Ok(variables(paires, definie))
}

fn variables(
    paires: Vec<(String, String)>,
    definie: impl Fn(&str) -> bool,
) -> Vec<(String, String)> {
    paires
        .into_iter()
        .filter(|(cle, _)| !definie(cle))
        .collect()
}

/// Lance le binaire de la crate `migration` du projet.
///
/// `stderr` reste hérité : la progression de cargo, qui compile la crate au premier
/// appel, doit rester visible pendant que la sortie utile est capturée.
fn lancer(
    racine: &Path,
    commande: &str,
    variables: &[(String, String)],
    capturer: bool,
) -> Result<String, Erreur> {
    let mut processus = Command::new("cargo");
    processus
        .current_dir(racine)
        .args(["run", "-p", "migration", "--", commande])
        .envs(variables.iter().map(|(cle, valeur)| (cle, valeur)))
        .stdout(if capturer {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });

    let sortie = processus
        .spawn()
        .map_err(Erreur::Cargo)?
        .wait_with_output()
        .map_err(Erreur::Cargo)?;

    if !sortie.status.success() {
        return Err(Erreur::Migration {
            code: sortie.status.code().unwrap_or(1),
        });
    }

    Ok(String::from_utf8_lossy(&sortie.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    /// Un projet déroulé par `rbs new`, sans passer par le binaire ni par cargo.
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

    #[test]
    fn hors_d_un_projet_rbs_rien_n_est_lance() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let erreur = executer(Action::Status, ailleurs.path()).expect_err("ce n'est pas un projet");

        assert!(matches!(erreur, Erreur::PasUnProjet));
    }

    #[test]
    fn la_variable_attendue_est_celle_qu_un_projet_neuf_ecrit_dans_son_env() {
        let (_parent, racine) = projet();

        let paires = dotenv::lire(&racine.join(".env")).expect("le .env est lisible");

        assert!(
            dotenv::valeur(&paires, URL).is_some(),
            "migrate cherche {URL}, absente du .env généré"
        );
    }

    #[test]
    fn un_env_absent_est_signale_avec_son_chemin() {
        let (_parent, racine) = projet();
        std::fs::remove_file(racine.join(".env")).expect("le .env existe");

        let erreur = executer(Action::Status, &racine).expect_err("le .env manque");

        assert!(erreur.to_string().contains(".env"));
    }

    #[test]
    fn sans_url_nulle_part_la_base_visee_est_inconnue() {
        let paires = vec![("RUST_LOG".to_string(), "info".to_string())];

        let erreur = preparer(paires, |_| false).expect_err("aucune URL n'est connue");

        assert!(erreur.to_string().contains(URL));
    }

    #[test]
    fn une_url_heritee_de_l_environnement_suffit() {
        let paires = vec![("RUST_LOG".to_string(), "info".to_string())];

        preparer(paires, |cle| cle == URL).expect("l'appelant fournit l'URL");
    }

    #[test]
    fn une_migration_creee_depuis_un_sous_repertoire_vise_la_racine_du_projet() {
        let (_parent, racine) = projet();

        let sortie = executer(
            Action::Nouvelle("ajout_index".to_string()),
            &racine.join("migration/src"),
        )
        .expect("la migration se crée");

        let Sortie::Creee(nouvelle) = sortie else {
            panic!("une création rend la migration créée");
        };
        assert!(racine.join(&nouvelle.fichier).is_file());
    }

    #[test]
    fn une_variable_deja_definie_prime_sur_celle_du_fichier() {
        let paires = vec![
            (URL.to_string(), "postgres://du-fichier".to_string()),
            ("RUST_LOG".to_string(), "info".to_string()),
        ];

        let transmises = variables(paires, |cle| cle == URL);

        assert_eq!(
            transmises,
            vec![("RUST_LOG".to_string(), "info".to_string())],
            "l'environnement de l'appelant l'emporte sur le .env"
        );
    }
}
