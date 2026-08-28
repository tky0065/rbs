//! Écriture d'un plan sur le disque, en entier ou pas du tout.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{File, Plan, Status};

/// Ce qui peut empêcher d'appliquer un plan.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Une écriture a échoué ; ce que le plan avait déjà écrit a été défait.
    #[error("{path} n'a pu être écrit : {source} — le projet a été laissé intact")]
    Ecriture {
        /// Chemin fautif, relatif à la racine.
        path: String,
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
pub(crate) fn apply(plan: &Plan, force: bool) -> Result<Vec<String>, Error> {
    if !force {
        let bloquants: Vec<&str> = plan
            .files()
            .iter()
            .filter(|file| file.statut == Status::Conflit)
            .map(|file| file.path.as_str())
            .collect();

        if !bloquants.is_empty() {
            return Err(Error::Conflit {
                chemins: bloquants.join(", "),
            });
        }
    }

    let mut journal = Log::default();

    for file in plan.files() {
        if file.statut == Status::DejaFait {
            continue;
        }

        if let Err(source) = journal.write(plan.root(), file) {
            journal.undo(plan.root());
            return Err(Error::Ecriture {
                path: file.path.clone(),
                source,
            });
        }
    }

    Ok(journal.ecrits)
}

/// Ce que l'application a fait, dans l'ordre, pour pouvoir le défaire.
#[derive(Default)]
struct Log {
    /// Chemins écrits, relatifs à la racine.
    ecrits: Vec<String>,
    /// Contenu d'origine de chaque chemin écrit : `None` s'il n'existait pas.
    origines: Vec<Option<String>>,
    /// Répertoires que l'application a créés, du plus haut au plus profond.
    repertoires: Vec<PathBuf>,
}

impl Log {
    /// Écrit un fichier après avoir noté de quoi le défaire.
    fn write(&mut self, root: &Path, file: &File) -> io::Result<()> {
        let path = root.join(&file.path);

        if let Some(parent) = path.parent() {
            self.create_directories(parent)?;
        }

        fs::write(&path, &file.after)?;
        self.ecrits.push(file.path.clone());
        self.origines.push(file.before.clone());

        Ok(())
    }

    /// Crée les répertoires manquants, en notant lesquels sont nés ici.
    ///
    /// `create_dir_all` ne dit pas ce qu'il a créé : sans cet inventaire, un rollback
    /// laisserait derrière lui des répertoires vides que le projet ne connaissait pas.
    fn create_directories(&mut self, parent: &Path) -> io::Result<()> {
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
    fn undo(&self, root: &Path) {
        for (path, origin) in self.ecrits.iter().zip(&self.origines).rev() {
            let path = root.join(path);
            let _ = match origin {
                Some(content) => fs::write(&path, content),
                None => fs::remove_file(&path),
            };
        }

        for directory in self.repertoires.iter().rev() {
            let _ = fs::remove_dir(directory);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::super::{File, Status};
    use super::*;

    fn file(path: &str, before: Option<&str>, after: &str, statut: Status) -> File {
        File {
            path: path.to_string(),
            before: before.map(str::to_string),
            after: after.to_string(),
            statut,
        }
    }

    fn plan_of(root: &Path, files: Vec<File>) -> Plan {
        Plan {
            root: root.to_path_buf(),
            actions: Vec::new(),
            files,
        }
    }

    /// Empreinte récursive d'un répertoire : chemin relatif -> contenu.
    ///
    /// Plus forte qu'une vérification d'absence : elle attrape aussi ce qu'on n'aurait pas
    /// pensé à chercher, un répertoire vide laissé derrière compris.
    fn fingerprint(root: &Path) -> BTreeMap<PathBuf, Option<String>> {
        let mut vue = BTreeMap::new();
        let mut a_visiter = vec![root.to_path_buf()];

        while let Some(directory) = a_visiter.pop() {
            for input in fs::read_dir(&directory).expect("le répertoire se lit") {
                let path = input.expect("l'entrée se lit").path();
                let relatif = path
                    .strip_prefix(root)
                    .expect("le chemin est sous la racine")
                    .to_path_buf();

                if path.is_dir() {
                    vue.insert(relatif, None);
                    a_visiter.push(path);
                } else {
                    vue.insert(relatif, Some(fs::read_to_string(&path).unwrap_or_default()));
                }
            }
        }

        vue
    }

    fn project() -> TempDir {
        TempDir::new().expect("le répertoire temporaire se crée")
    }

    #[test]
    fn a_conflict_free_plan_writes_all_its_files() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), "[package]\n").expect("l'écriture aboutit");

        let plan = plan_of(
            project.path(),
            vec![
                file("Dockerfile", None, "FROM rust\n", Status::AFaire),
                file("src/notes/mod.rs", None, "pub mod dto;\n", Status::AFaire),
                file(
                    "Cargo.toml",
                    Some("[package]\n"),
                    "[package]\nname = \"demo\"\n",
                    Status::AFaire,
                ),
            ],
        );

        let ecrits = apply(&plan, false).expect("rien ne s'oppose à l'écriture");

        assert_eq!(ecrits.len(), 3, "{ecrits:?}");
        assert_eq!(
            fs::read_to_string(project.path().join("Dockerfile")).expect("le fichier existe"),
            "FROM rust\n"
        );
        assert_eq!(
            fs::read_to_string(project.path().join("src/notes/mod.rs")).expect("le fichier existe"),
            "pub mod dto;\n"
        );
        assert_eq!(
            fs::read_to_string(project.path().join("Cargo.toml")).expect("le fichier existe"),
            "[package]\nname = \"demo\"\n"
        );
    }

    #[test]
    fn an_already_conforming_file_is_not_rewritten() {
        let project = project();
        fs::write(project.path().join("Dockerfile"), "FROM rust\n").expect("l'écriture aboutit");

        let plan = plan_of(
            project.path(),
            vec![file(
                "Dockerfile",
                Some("FROM rust\n"),
                "FROM rust\n",
                Status::DejaFait,
            )],
        );

        let ecrits = apply(&plan, false).expect("il n'y a rien à faire");

        assert!(ecrits.is_empty(), "{ecrits:?}");
    }

    #[test]
    fn a_conflict_rejects_the_plan_before_the_first_write() {
        let project = project();
        fs::write(project.path().join("src.rs"), "écrit à la main\n").expect("l'écriture aboutit");
        let before = fingerprint(project.path());

        let plan = plan_of(
            project.path(),
            vec![
                file("Dockerfile", None, "FROM rust\n", Status::AFaire),
                file(
                    "src.rs",
                    Some("écrit à la main\n"),
                    "écrasé\n",
                    Status::Conflit,
                ),
            ],
        );

        let error = apply(&plan, false).expect_err("le conflit doit arrêter le plan");

        assert!(matches!(error, Error::Conflit { .. }), "{error:?}");
        assert!(error.to_string().contains("src.rs"), "{error}");
        assert_eq!(
            fingerprint(project.path()),
            before,
            "rien ne doit avoir été écrit"
        );
    }

    #[test]
    fn a_forced_conflict_is_overwritten() {
        let project = project();
        fs::write(project.path().join("src.rs"), "écrit à la main\n").expect("l'écriture aboutit");

        let plan = plan_of(
            project.path(),
            vec![file(
                "src.rs",
                Some("écrit à la main\n"),
                "écrasé\n",
                Status::Conflit,
            )],
        );

        apply(&plan, true).expect("--force écrase");

        assert_eq!(
            fs::read_to_string(project.path().join("src.rs")).expect("le fichier existe"),
            "écrasé\n"
        );
    }

    /// Le critère de la tâche : un échec en cours d'application ne laisse rien derrière.
    #[test]
    fn a_failure_on_the_fourth_action_rolls_back_the_first_three() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), "[package]\n").expect("l'écriture aboutit");
        // Le parent du quatrième fichier est un fichier régulier : `create_dir_all` y
        // échouera pour de vrai, sans point d'injection dans le code de production.
        fs::write(
            project.path().join("obstacle"),
            "je ne suis pas un répertoire\n",
        )
        .expect("l'écriture aboutit");
        let before = fingerprint(project.path());

        let plan = plan_of(
            project.path(),
            vec![
                file("Dockerfile", None, "FROM rust\n", Status::AFaire),
                file("src/notes/mod.rs", None, "pub mod dto;\n", Status::AFaire),
                file(
                    "Cargo.toml",
                    Some("[package]\n"),
                    "[package]\nname = \"demo\"\n",
                    Status::AFaire,
                ),
                file("obstacle/x.rs", None, "jamais écrit\n", Status::AFaire),
            ],
        );

        let error = apply(&plan, false).expect_err("la quatrième action doit échouer");

        assert!(matches!(error, Error::Ecriture { .. }), "{error:?}");
        assert_eq!(
            fingerprint(project.path()),
            before,
            "les trois premières actions doivent avoir été annulées"
        );
    }

    #[test]
    fn a_directory_created_then_rolled_back_does_not_linger() {
        let project = project();
        fs::write(project.path().join("obstacle"), "pas un répertoire\n")
            .expect("l'écriture aboutit");

        let plan = plan_of(
            project.path(),
            vec![
                file("src/notes/mod.rs", None, "pub mod dto;\n", Status::AFaire),
                file("obstacle/x.rs", None, "jamais écrit\n", Status::AFaire),
            ],
        );

        apply(&plan, false).expect_err("la seconde action doit échouer");

        assert!(
            !project.path().join("src").exists(),
            "`src/` a été créé par le plan : il doit disparaître avec lui"
        );
    }
}
