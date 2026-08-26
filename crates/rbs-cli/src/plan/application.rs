//! Écriture d'un plan sur le disque, en entier ou pas du tout.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{Fichier, Plan, Statut};

/// Ce qui peut empêcher d'appliquer un plan.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// Une écriture a échoué ; ce que le plan avait déjà écrit a été défait.
    #[error("{chemin} n'a pu être écrit : {source} — le projet a été laissé intact")]
    Ecriture {
        /// Chemin fautif, relatif à la racine.
        chemin: String,
        /// Cause système.
        source: std::io::Error,
    },
    /// Le plan écraserait des fichiers que le projet ne lui doit pas.
    #[error("{chemins} — relancer avec --force pour les écraser")]
    Conflit {
        /// Chemins en conflit, relatifs à la racine.
        chemins: String,
    },
}

/// Écrit les fichiers du plan, ou n'en laisse aucun.
///
/// Les conflits s'arbitrent avant la première écriture : un plan refusé à mi-chemin aurait
/// à être défait, alors qu'il n'aurait jamais dû commencer.
pub(crate) fn appliquer(plan: &Plan, force: bool) -> Result<Vec<String>, Erreur> {
    if !force {
        let bloquants: Vec<&str> = plan
            .fichiers()
            .iter()
            .filter(|fichier| fichier.statut == Statut::Conflit)
            .map(|fichier| fichier.chemin.as_str())
            .collect();

        if !bloquants.is_empty() {
            return Err(Erreur::Conflit {
                chemins: bloquants.join(", "),
            });
        }
    }

    let mut journal = Journal::default();

    for fichier in plan.fichiers() {
        if fichier.statut == Statut::DejaFait {
            continue;
        }

        if let Err(source) = journal.ecrire(plan.racine(), fichier) {
            journal.defaire(plan.racine());
            return Err(Erreur::Ecriture {
                chemin: fichier.chemin.clone(),
                source,
            });
        }
    }

    Ok(journal.ecrits)
}

/// Ce que l'application a fait, dans l'ordre, pour pouvoir le défaire.
#[derive(Default)]
struct Journal {
    /// Chemins écrits, relatifs à la racine.
    ecrits: Vec<String>,
    /// Contenu d'origine de chaque chemin écrit : `None` s'il n'existait pas.
    origines: Vec<Option<String>>,
    /// Répertoires que l'application a créés, du plus haut au plus profond.
    repertoires: Vec<PathBuf>,
}

impl Journal {
    /// Écrit un fichier après avoir noté de quoi le défaire.
    fn ecrire(&mut self, racine: &Path, fichier: &Fichier) -> io::Result<()> {
        let chemin = racine.join(&fichier.chemin);

        if let Some(parent) = chemin.parent() {
            self.creer_repertoires(parent)?;
        }

        fs::write(&chemin, &fichier.apres)?;
        self.ecrits.push(fichier.chemin.clone());
        self.origines.push(fichier.avant.clone());

        Ok(())
    }

    /// Crée les répertoires manquants, en notant lesquels sont nés ici.
    ///
    /// `create_dir_all` ne dit pas ce qu'il a créé : sans cet inventaire, un rollback
    /// laisserait derrière lui des répertoires vides que le projet ne connaissait pas.
    fn creer_repertoires(&mut self, parent: &Path) -> io::Result<()> {
        let mut a_creer = Vec::new();
        for ancetre in parent.ancestors() {
            if ancetre.exists() {
                break;
            }
            a_creer.push(ancetre.to_path_buf());
        }

        fs::create_dir_all(parent)?;
        self.repertoires.extend(a_creer.into_iter().rev());

        Ok(())
    }

    /// Remet le projet dans l'état où l'application l'a trouvé.
    ///
    /// Les échecs de restauration sont tus : on est déjà sur un chemin d'erreur, et
    /// l'erreur qui a tout déclenché est plus utile que celle du nettoyage.
    fn defaire(&self, racine: &Path) {
        for (chemin, origine) in self.ecrits.iter().zip(&self.origines).rev() {
            let chemin = racine.join(chemin);
            let _ = match origine {
                Some(contenu) => fs::write(&chemin, contenu),
                None => fs::remove_file(&chemin),
            };
        }

        for repertoire in self.repertoires.iter().rev() {
            let _ = fs::remove_dir(repertoire);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::super::{Fichier, Statut};
    use super::*;

    fn fichier(chemin: &str, avant: Option<&str>, apres: &str, statut: Statut) -> Fichier {
        Fichier {
            chemin: chemin.to_string(),
            avant: avant.map(str::to_string),
            apres: apres.to_string(),
            statut,
        }
    }

    fn plan_de(racine: &Path, fichiers: Vec<Fichier>) -> Plan {
        Plan {
            racine: racine.to_path_buf(),
            actions: Vec::new(),
            fichiers,
        }
    }

    /// Empreinte récursive d'un répertoire : chemin relatif -> contenu.
    ///
    /// Plus forte qu'une vérification d'absence : elle attrape aussi ce qu'on n'aurait pas
    /// pensé à chercher, un répertoire vide laissé derrière compris.
    fn empreinte(racine: &Path) -> BTreeMap<PathBuf, Option<String>> {
        let mut vue = BTreeMap::new();
        let mut a_visiter = vec![racine.to_path_buf()];

        while let Some(repertoire) = a_visiter.pop() {
            for entree in fs::read_dir(&repertoire).expect("le répertoire se lit") {
                let chemin = entree.expect("l'entrée se lit").path();
                let relatif = chemin
                    .strip_prefix(racine)
                    .expect("le chemin est sous la racine")
                    .to_path_buf();

                if chemin.is_dir() {
                    vue.insert(relatif, None);
                    a_visiter.push(chemin);
                } else {
                    vue.insert(
                        relatif,
                        Some(fs::read_to_string(&chemin).unwrap_or_default()),
                    );
                }
            }
        }

        vue
    }

    fn projet() -> TempDir {
        TempDir::new().expect("le répertoire temporaire se crée")
    }

    #[test]
    fn un_plan_sans_conflit_ecrit_tous_ses_fichiers() {
        let projet = projet();
        fs::write(projet.path().join("Cargo.toml"), "[package]\n").expect("l'écriture aboutit");

        let plan = plan_de(
            projet.path(),
            vec![
                fichier("Dockerfile", None, "FROM rust\n", Statut::AFaire),
                fichier("src/notes/mod.rs", None, "pub mod dto;\n", Statut::AFaire),
                fichier(
                    "Cargo.toml",
                    Some("[package]\n"),
                    "[package]\nname = \"demo\"\n",
                    Statut::AFaire,
                ),
            ],
        );

        let ecrits = appliquer(&plan, false).expect("rien ne s'oppose à l'écriture");

        assert_eq!(ecrits.len(), 3, "{ecrits:?}");
        assert_eq!(
            fs::read_to_string(projet.path().join("Dockerfile")).expect("le fichier existe"),
            "FROM rust\n"
        );
        assert_eq!(
            fs::read_to_string(projet.path().join("src/notes/mod.rs")).expect("le fichier existe"),
            "pub mod dto;\n"
        );
        assert_eq!(
            fs::read_to_string(projet.path().join("Cargo.toml")).expect("le fichier existe"),
            "[package]\nname = \"demo\"\n"
        );
    }

    #[test]
    fn un_fichier_deja_conforme_n_est_pas_reecrit() {
        let projet = projet();
        fs::write(projet.path().join("Dockerfile"), "FROM rust\n").expect("l'écriture aboutit");

        let plan = plan_de(
            projet.path(),
            vec![fichier(
                "Dockerfile",
                Some("FROM rust\n"),
                "FROM rust\n",
                Statut::DejaFait,
            )],
        );

        let ecrits = appliquer(&plan, false).expect("il n'y a rien à faire");

        assert!(ecrits.is_empty(), "{ecrits:?}");
    }

    #[test]
    fn un_conflit_fait_refuser_le_plan_avant_la_premiere_ecriture() {
        let projet = projet();
        fs::write(projet.path().join("src.rs"), "écrit à la main\n").expect("l'écriture aboutit");
        let avant = empreinte(projet.path());

        let plan = plan_de(
            projet.path(),
            vec![
                fichier("Dockerfile", None, "FROM rust\n", Statut::AFaire),
                fichier(
                    "src.rs",
                    Some("écrit à la main\n"),
                    "écrasé\n",
                    Statut::Conflit,
                ),
            ],
        );

        let erreur = appliquer(&plan, false).expect_err("le conflit doit arrêter le plan");

        assert!(matches!(erreur, Erreur::Conflit { .. }), "{erreur:?}");
        assert!(erreur.to_string().contains("src.rs"), "{erreur}");
        assert_eq!(
            empreinte(projet.path()),
            avant,
            "rien ne doit avoir été écrit"
        );
    }

    #[test]
    fn un_conflit_force_est_ecrase() {
        let projet = projet();
        fs::write(projet.path().join("src.rs"), "écrit à la main\n").expect("l'écriture aboutit");

        let plan = plan_de(
            projet.path(),
            vec![fichier(
                "src.rs",
                Some("écrit à la main\n"),
                "écrasé\n",
                Statut::Conflit,
            )],
        );

        appliquer(&plan, true).expect("--force écrase");

        assert_eq!(
            fs::read_to_string(projet.path().join("src.rs")).expect("le fichier existe"),
            "écrasé\n"
        );
    }

    /// Le critère de la tâche : un échec en cours d'application ne laisse rien derrière.
    #[test]
    fn un_echec_sur_la_quatrieme_action_annule_les_trois_premieres() {
        let projet = projet();
        fs::write(projet.path().join("Cargo.toml"), "[package]\n").expect("l'écriture aboutit");
        // Le parent du quatrième fichier est un fichier régulier : `create_dir_all` y
        // échouera pour de vrai, sans point d'injection dans le code de production.
        fs::write(
            projet.path().join("obstacle"),
            "je ne suis pas un répertoire\n",
        )
        .expect("l'écriture aboutit");
        let avant = empreinte(projet.path());

        let plan = plan_de(
            projet.path(),
            vec![
                fichier("Dockerfile", None, "FROM rust\n", Statut::AFaire),
                fichier("src/notes/mod.rs", None, "pub mod dto;\n", Statut::AFaire),
                fichier(
                    "Cargo.toml",
                    Some("[package]\n"),
                    "[package]\nname = \"demo\"\n",
                    Statut::AFaire,
                ),
                fichier("obstacle/x.rs", None, "jamais écrit\n", Statut::AFaire),
            ],
        );

        let erreur = appliquer(&plan, false).expect_err("la quatrième action doit échouer");

        assert!(matches!(erreur, Erreur::Ecriture { .. }), "{erreur:?}");
        assert_eq!(
            empreinte(projet.path()),
            avant,
            "les trois premières actions doivent avoir été annulées"
        );
    }

    #[test]
    fn un_repertoire_cree_puis_annule_ne_reste_pas_derriere() {
        let projet = projet();
        fs::write(projet.path().join("obstacle"), "pas un répertoire\n")
            .expect("l'écriture aboutit");

        let plan = plan_de(
            projet.path(),
            vec![
                fichier("src/notes/mod.rs", None, "pub mod dto;\n", Statut::AFaire),
                fichier("obstacle/x.rs", None, "jamais écrit\n", Statut::AFaire),
            ],
        );

        appliquer(&plan, false).expect_err("la seconde action doit échouer");

        assert!(
            !projet.path().join("src").exists(),
            "`src/` a été créé par le plan : il doit disparaître avec lui"
        );
    }
}
