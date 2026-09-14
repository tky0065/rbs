//! La planification d'une commande qui modifie un projet, réifiée en valeur.
//!
//! Un plan est une liste d'actions ; chaque action vise un fichier et connaît son contenu
//! avant et son contenu après. Planifier, c'est calculer les « après » sans rien écrire —
//! d'où l'affichage préalable, la restauration en cas d'échec et l'idempotence.

mod action;
pub(crate) mod application;
pub(crate) mod json;
pub(crate) mod render;
mod text;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::anchors::Anchor;

pub(crate) use action::{Action, Effect, PatchToml, Status};

/// Un fichier que le plan touche, avec ses deux états et ce qu'il en coûtera d'y écrire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct File {
    /// Chemin relatif à la racine du projet.
    pub path: String,
    /// Contenu actuel, ou `None` si le fichier n'existe pas encore.
    pub before: Option<String>,
    /// Contenu que l'application écrira.
    pub after: String,
    /// Statut agrégé des actions qui visent ce fichier.
    ///
    /// Sans lui, un appelant qui écrit `files()` tel quel écraserait un fichier en
    /// conflit sans jamais voir le conflit, resté dans `actions()`.
    pub statut: Status,
}

/// Une insertion que le plan ne fera pas, faute de l'endroit qui devait la porter.
///
/// Ne concerne que les ancres optionnelles : un projet SQLite n'a pas de compose, et le
/// service qu'un fragment y aurait ajouté n'a nulle part où aller. Plutôt que d'échouer
/// — ce qui condamnait `mail`, `redis`, `auth` et `webhooks` sur tout projet sans compose
/// — le plan garde le bloc, pour que l'utilisateur sache quoi monter lui-même.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sautee {
    /// L'ancre visée.
    pub anchor: Anchor,
    /// Les lignes qui y seraient allées.
    pub lines: Vec<String>,
    /// Ce qui manquait : le remède n'est pas le même.
    pub cause: CauseSautee,
}

/// Ce qui a fait sauter une insertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CauseSautee {
    /// Le fichier porteur n'existe pas.
    FichierAbsent,
    /// Le fichier est là, sans l'ancre : un projet engendré avant qu'elle n'existe.
    AncreAbsente {
        /// Ce qui empêche `rbs doctor --fix` de la reposer, `None` s'il le sait : promettre
        /// la réparation d'un fichier qui a perdu son accroche enverrait dans une impasse.
        obstacle: Option<crate::anchors::Cause>,
    },
    /// Une insertion précédente du plan a sauté, et celle-ci nomme ce qu'elle devait poser :
    /// écrite seule, elle laisserait le projet hors d'état de compiler, ou de fonctionner.
    Entrainee {
        /// L'ancre de l'insertion sautée dont celle-ci dépend.
        par: Anchor,
    },
}

/// Ce qu'une commande fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    root: PathBuf,
    /// Trace du calcul des statuts, action par action : la vue JSON (`plan::json::plan`)
    /// la lit en production, les tests du modèle la vérifient dans le même ordre.
    actions: Vec<Action>,
    files: Vec<File>,
    sautees: Vec<Sautee>,
}

impl Plan {
    /// Les actions dans l'ordre où elles ont été planifiées.
    ///
    /// L'affichage humain et l'application travaillent par fichier : un fichier peut
    /// recevoir plusieurs actions, et seul son statut agrégé décide de ce qui lui
    /// arrivera. La vue JSON, elle, rend chaque action séparément — d'où cet accesseur.
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    /// Les fichiers touchés, un par chemin, dans l'ordre où ils ont été rencontrés.
    pub fn files(&self) -> &[File] {
        &self.files
    }

    /// Racine du projet, à laquelle les chemins des fichiers sont relatifs.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Les insertions que le plan a sautées, dans l'ordre où elles ont été planifiées.
    pub fn sautees(&self) -> &[Sautee] {
        &self.sautees
    }

    /// Ce que l'application de ce plan écrit, par ce qui arrive aux fichiers.
    ///
    /// La règle est celle d'`application::apply` : un fichier inchangé n'est pas réécrit,
    /// un conflit ne l'est que sous `--force`. Le bilan d'une commande dit ainsi ce que
    /// son plan avait annoncé.
    pub(crate) fn bilan(&self, force: bool) -> Bilan {
        let mut bilan = Bilan {
            crees: 0,
            modifies: 0,
        };
        for file in &self.files {
            let ecrit = match file.statut {
                Status::AFaire => true,
                Status::Conflit => force,
                Status::DejaFait => false,
            };
            if !ecrit {
                continue;
            }
            if file.before.is_some() {
                bilan.modifies += 1;
            } else {
                bilan.crees += 1;
            }
        }
        bilan
    }
}

/// Les fichiers qu'une application écrit, par ce qui leur arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bilan {
    /// Fichiers qui n'existaient pas.
    pub crees: usize,
    /// Fichiers existants réécrits.
    pub modifies: usize,
}

/// Ce qui peut empêcher de planifier.
///
/// Chaque variante nomme son fichier relativement à la racine, comme `Action::path` :
/// l'emplacement complet du projet est porté une seule fois, par l'en-tête de l'affichage
/// du plan.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Un fichier du projet n'a pas pu être lu.
    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),
    /// Deux actions du plan prétendent écrire le même fichier de bout en bout.
    ///
    /// Erreur de programmation de l'appelant : deux contenus complets ne se composent
    /// pas, et le second effacerait silencieusement ce que le premier a projeté.
    #[error("{path} est déjà projeté par une action précédente du plan")]
    DejaProjete {
        /// Chemin fautif, relatif à la racine.
        path: String,
    },
    /// Une ancre attendue a disparu du projet.
    #[error("{0}")]
    Anchor(#[source] crate::anchors::Missing),
    /// Une ancre est là, mais sous une ligne que l'insertion doit précéder.
    ///
    /// Distincte d'`Anchor` : l'ancre existe, et la reposer n'y changerait rien. C'est sa
    /// place qui est en cause, et le remède est un bloc à remonter, non à coller.
    #[error("{0}")]
    MalPlacee(#[source] Box<crate::anchors::Misplaced>),
    /// Le fichier qui porte l'ancre visée n'existe pas.
    ///
    /// Distincte d'`Anchor`, qui suppose au contraire un fichier présent mais dépourvu de
    /// ses balises : ici c'est le fichier entier qui manque, et chercher une balise
    /// dedans n'aurait aucun sens.
    #[error("{path} est introuvable")]
    FichierAbsent {
        /// Chemin du fichier porteur, relatif à la racine.
        path: String,
    },
    /// Le manifeste du projet n'a pas pu être patché.
    #[error("{0}")]
    Metadata(#[source] crate::metadata::Error),
    /// Un document TOML du projet ne s'analyse pas.
    ///
    /// Distincte de `Metadata` : celle-ci vise les documents de configuration, dont
    /// rien ne dit qu'ils portent une section `[package]`.
    #[error("{path} n'est pas un TOML valide : {source}")]
    Toml {
        /// Chemin fautif, relatif à la racine.
        path: String,
        /// Cause de l'analyse.
        source: toml_edit::TomlError,
    },
    /// Le `Cargo.toml` visé par un patch n'existe pas à l'emplacement attendu.
    ///
    /// Distincte de `Metadata(PasUnProjet)`, qui suppose au contraire un fichier
    /// présent mais dépourvu de la section `[package.metadata.rbs]`.
    #[error("{path} est introuvable")]
    ManifesteAbsent {
        /// Chemin du manifeste, relatif à la racine.
        path: String,
    },
    /// Le fichier ne porte pas la zone que l'action visait.
    #[error("{path} : {zone}")]
    ZoneAbsente {
        /// Chemin du fichier visé.
        path: String,
        /// La zone manquante, et le bloc qui la rétablit.
        zone: crate::agents::MissingZone,
    },
}

impl Error {
    /// Code stable de la faute, en snake_case ASCII.
    ///
    /// `match` exhaustif, sans bras `_` : une variante ajoutée à `Error` ne compile plus
    /// tant qu'elle n'a pas le sien.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Error::Acces(_) => "fichier_inaccessible",
            Error::DejaProjete { .. } => "plan_incoherent",
            Error::Anchor(_) => "ancre_absente",
            Error::MalPlacee(_) => "ancre_mal_placee",
            Error::FichierAbsent { .. } => "fichier_absent",
            Error::Metadata(_) => "manifeste_illisible",
            Error::Toml { .. } => "toml_invalide",
            Error::ManifesteAbsent { .. } => "manifeste_absent",
            Error::ZoneAbsente { .. } => "zone_absente",
        }
    }

    /// Le bloc à coller, quand la faute est une ancre disparue, mal placée, ou une zone
    /// absente d'`AGENTS.md` — les seules dont le remède tient dans un extrait de
    /// fichier plutôt que dans une décision du développeur.
    pub(crate) fn bloc(&self) -> Option<String> {
        match self {
            Error::Anchor(absente) => Some(absente.anchor.block()),
            Error::MalPlacee(placee) => Some(placee.block.clone()),
            Error::ZoneAbsente { zone, .. } => Some(zone.block()),
            _ => None,
        }
    }

    /// Le texte du remède, porté une seule fois : chaque commande qui affiche un remède
    /// humain pour une ancre disparue ou mal placée délègue ici plutôt que de recopier
    /// ce texte, qui divergerait au premier changement.
    ///
    /// `Some` exactement quand [`Self::bloc`] l'est : un bloc à coller sans le dire où le
    /// coller laisserait un agent deviner.
    pub(crate) fn remede(&self) -> Option<String> {
        match self {
            Error::Anchor(absente) => Some(format!(
                "dans {} :\n{}",
                absente.anchor.file,
                absente.anchor.block()
            )),
            Error::MalPlacee(placee) => Some(format!(
                "dans {}, remontez ce bloc au-dessus de `{}` :\n{}",
                placee.anchor.file, placee.before, placee.block
            )),
            Error::ZoneAbsente { path, zone } => Some(format!(
                "dans {path}, collez ce bloc pour rétablir la zone `rbs:{}` :\n{}",
                zone.zone,
                zone.block()
            )),
            _ => None,
        }
    }
}

/// Accumule les actions d'un plan en calculant, pour chaque fichier, son contenu final.
pub(crate) struct Builder {
    root: PathBuf,
    actions: Vec<Action>,
    files: Vec<File>,
    sautees: Vec<Sautee>,
}

impl Builder {
    /// Ouvre un plan vide sur le projet enraciné en `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            actions: Vec::new(),
            files: Vec::new(),
            sautees: Vec::new(),
        }
    }

    /// Planifie l'écriture de `path` avec `content`.
    ///
    /// Refuse un chemin qu'une action précédente a déjà projeté : voir
    /// [`Error::DejaProjete`].
    pub fn create(&mut self, path: &str, content: &str) -> Result<(), Error> {
        if self.projected(path) {
            return Err(Error::DejaProjete {
                path: path.to_string(),
            });
        }

        let origin = self.read(path)?;

        let statut = match origin.as_deref() {
            None => Status::AFaire,
            Some(actuel) if actuel == content => Status::DejaFait,
            Some(_) => Status::Conflit,
        };

        self.project_onto(path, origin, content.to_string(), statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::Creer {
                content: content.to_string(),
            },
            statut,
        });

        Ok(())
    }

    /// Planifie l'ajout de `lines` dans `anchor`, juste avant sa balise fermante.
    ///
    /// Le fichier visé est celui que l'ancre désigne : une ancre ne se déplace pas. S'il
    /// manque et que l'ancre est optionnelle, l'insertion est sautée et consignée — voir
    /// [`Sautee`] ; un fichier qu'une action précédente du plan projette compte comme
    /// présent, pour que le fragment qui écrit le compose puis y insère n'y saute rien.
    pub fn insert(&mut self, anchor: Anchor, lines: &[String]) -> Result<(), Error> {
        let path = anchor.file.to_string();

        let states = self.states(&path)?;
        let courant = match states.courant {
            Some(courant) => courant,
            None if anchor.optional => {
                self.sautees.push(Sautee {
                    anchor,
                    lines: lines.to_vec(),
                    cause: CauseSautee::FichierAbsent,
                });
                return Ok(());
            }
            None => {
                return Err(Error::FichierAbsent {
                    path: path.to_string(),
                });
            }
        };

        let after =
            crate::anchors::insert(&courant, anchor.clone(), lines).map_err(Error::Anchor)?;
        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(&path, states.origin, after, statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::Inserer {
                anchor,
                lines: lines.to_vec(),
            },
            statut,
        });

        Ok(())
    }

    /// Comme [`Builder::insert`], mais une ancre optionnelle absente d'un fichier présent
    /// saute l'insertion au lieu d'arrêter le plan.
    ///
    /// Distincte d'`insert`, dont les appelants tiennent au refus : un compose réécrit à la
    /// main sans son ancre est un compose abîmé. Ici le fichier précède l'ancre — un projet
    /// engendré avant elle — et l'y reposer est l'affaire de `rbs doctor --fix`, quand le
    /// fichier porte encore la ligne sous laquelle elle se repose.
    ///
    /// Rend `true` quand l'insertion est planifiée, écrite ou déjà en place, et `false`
    /// quand elle saute : l'appelant dont une insertion suivante en dépend la saute avec.
    pub fn insert_ou_sauter(&mut self, anchor: Anchor, lines: &[String]) -> Result<bool, Error> {
        let avant = self.sautees.len();

        match self.insert(anchor.clone(), lines) {
            Ok(()) => Ok(self.sautees.len() == avant),
            Err(Error::Anchor(_)) if anchor.optional => {
                // Jugée sur le fichier tel que le plan le laisse : c'est lui que `rbs doctor
                // --fix` trouvera, le plan appliqué.
                let obstacle = match self.states(&anchor.file)?.courant {
                    Some(courant) => crate::anchors::repose(&courant, &anchor).err(),
                    None => Some(crate::anchors::Cause::FichierAbsent),
                };
                self.sautees.push(Sautee {
                    anchor,
                    lines: lines.to_vec(),
                    cause: CauseSautee::AncreAbsente { obstacle },
                });
                Ok(false)
            }
            Err(autre) => Err(autre),
        }
    }

    /// Consigne une insertion sans la tenter : `cause` dit pourquoi elle ne s'écrira pas.
    pub fn sauter(&mut self, anchor: Anchor, lines: &[String], cause: CauseSautee) {
        self.sautees.push(Sautee {
            anchor,
            lines: lines.to_vec(),
            cause,
        });
    }

    /// Vérifie que `anchor` précède, dans son fichier, toute ligne commençant par `line`.
    ///
    /// Sur le fichier tel que le plan l'a trouvé, et non tel qu'il le laissera : le bloc
    /// que le remède affiche est celui que le développeur doit déplacer, sans les lignes
    /// que ce plan, refusé, n'écrira pas. Un fichier absent du disque ne dit rien — s'il
    /// naît de ce plan, sa template le rend en place ; sinon c'est l'insertion qui le
    /// signale, ou la saute si l'ancre est optionnelle.
    pub fn require_before(&self, anchor: &Anchor, line: &str) -> Result<(), Error> {
        let Some(origine) = self.states(anchor.file.as_ref())?.origin else {
            return Ok(());
        };

        crate::anchors::precedes(&origine, anchor, line).map_err(Error::MalPlacee)
    }

    /// Planifie la remise en place de `anchor`, disparue de son fichier.
    ///
    /// Rend ce qu'elle a fait plutôt qu'une erreur : une ancre dont l'accroche ne se
    /// retrouve pas laisse le plan intact, et le diagnostic la nomme comme avant.
    pub fn repose(&mut self, anchor: Anchor) -> Result<Repose, Error> {
        let path = anchor.file.to_string();

        let states = self.states(&path)?;
        let Some(courant) = states.courant else {
            return Ok(Repose::Laissee(crate::anchors::Cause::FichierAbsent));
        };

        let after = match crate::anchors::repose(&courant, &anchor) {
            Ok(after) => after,
            Err(cause) => return Ok(Repose::Laissee(cause)),
        };
        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(&path, states.origin, after, statut);
        self.actions.push(Action {
            path,
            effet: Effect::ReposerAncre { anchor },
            statut,
        });

        Ok(Repose::Reposee)
    }

    /// Planifie le remplacement du corps de `zone` dans `path`.
    ///
    /// `version` réécrit le marqueur d'ouverture : c'est ainsi que le guide porte la
    /// version du CLI qui l'a produit.
    ///
    /// Comme une insertion, l'action compose avec ce qu'elle trouve : elle ne remplace
    /// pas le fichier, et n'entre donc jamais en conflit.
    pub fn replace_zone(
        &mut self,
        path: &str,
        zone: &str,
        content: &str,
        version: Option<&str>,
    ) -> Result<(), Error> {
        let states = self.states(path)?;
        let courant = states.courant.ok_or_else(|| Error::FichierAbsent {
            path: path.to_string(),
        })?;

        let after = match version {
            Some(version) => crate::agents::replace_versioned(&courant, zone, content, version),
            None => crate::agents::replace(&courant, zone, content),
        }
        .map_err(|zone| Error::ZoneAbsente {
            path: path.to_string(),
            zone,
        })?;

        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(path, states.origin, after, statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::RemplacerZone {
                zone: zone.to_string(),
                content: content.to_string(),
            },
            statut,
        });

        Ok(())
    }

    /// Planifie une modification du `Cargo.toml` de la racine.
    pub fn patch(&mut self, patch: PatchToml) -> Result<(), Error> {
        let path = "Cargo.toml";

        let states = self.states(path)?;
        let courant = states.courant.ok_or_else(|| Error::ManifesteAbsent {
            path: path.to_string(),
        })?;

        let rendered = match &patch {
            PatchToml::InscrireFeature(feature) => {
                crate::metadata::record_feature(&courant, feature, path)
            }
            PatchToml::AjouterDependance(dependency) => {
                crate::metadata::add_dependency(&courant, dependency, path)
            }
            PatchToml::AjouterFeatureADependance {
                dependency,
                feature,
            } => crate::metadata::add_feature_to_dependency(&courant, dependency, feature, path),
            PatchToml::AlignerSurVersion {
                dependency,
                version,
            } => crate::metadata::align_version(&courant, dependency, version, path),
        }
        .map_err(Error::Metadata)?;

        let after = rendered.unwrap_or(courant);
        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(path, states.origin, after, statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::PatcherToml { patch },
            statut,
        });

        Ok(())
    }

    /// Planifie l'ajout de la section `section` au document TOML `path`.
    pub fn add_section(&mut self, path: &str, section: &str, content: &str) -> Result<(), Error> {
        let states = self.states(path)?;
        let courant = states.courant.ok_or_else(|| Error::FichierAbsent {
            path: path.to_string(),
        })?;

        let rendered =
            text::add_section(&courant, section, content).map_err(|source| Error::Toml {
                path: path.to_string(),
                source,
            })?;

        let after = rendered.unwrap_or(courant);
        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(path, states.origin, after, statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::AjouterSection {
                section: section.to_string(),
                content: content.to_string(),
            },
            statut,
        });

        Ok(())
    }

    /// Planifie l'ajout de la variable `key` au fichier d'environnement `path`.
    pub fn add_variable(
        &mut self,
        path: &str,
        key: &str,
        value: &str,
        comment: Option<&str>,
    ) -> Result<(), Error> {
        let states = self.states(path)?;
        let courant = states.courant.ok_or_else(|| Error::FichierAbsent {
            path: path.to_string(),
        })?;

        let after = text::add_variable(&courant, key, value, comment).unwrap_or(courant);
        let statut = combined_status(states.origin.as_deref(), &after);

        self.project_onto(path, states.origin, after, statut);
        self.actions.push(Action {
            path: path.to_string(),
            effet: Effect::AjouterVariable {
                key: key.to_string(),
                value: value.to_string(),
                comment: comment.map(str::to_string),
            },
            statut,
        });

        Ok(())
    }

    /// Clôt le plan.
    pub fn finir(self) -> Plan {
        Plan {
            root: self.root,
            actions: self.actions,
            files: self.files,
            sautees: self.sautees,
        }
    }

    /// Le projet porte-t-il déjà ce fichier, sur le disque ou par une action projetée ?
    pub fn exists(&self, path: &str) -> Result<bool, Error> {
        Ok(self.states(path)?.courant.is_some())
    }

    /// Une action précédente a-t-elle déjà calculé le contenu final de ce fichier ?
    fn projected(&self, path: &str) -> bool {
        self.files.iter().any(|file| file.path == path)
    }

    /// Ce qu'une action trouve du fichier qu'elle vise.
    ///
    /// Les deux états ne se confondent qu'au premier passage sur un fichier : ensuite,
    /// l'action compose avec ce que la précédente a produit, mais son statut se décide
    /// toujours contre l'origine.
    fn states(&self, path: &str) -> Result<States, Error> {
        if let Some(file) = self.files.iter().find(|f| f.path == path) {
            return Ok(States {
                origin: file.before.clone(),
                courant: Some(file.after.clone()),
            });
        }

        let disque = self.read(path)?;

        Ok(States {
            origin: disque.clone(),
            courant: disque,
        })
    }

    /// Contenu du fichier sur le disque, ou `None` s'il n'existe pas.
    fn read(&self, path: &str) -> Result<Option<String>, Error> {
        match fs::read_to_string(self.root.join(path)) {
            Ok(content) => Ok(Some(content)),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => {
                Err(crate::errors::Acces::new(std::path::Path::new(path), source).into())
            }
        }
    }

    /// Enregistre le contenu final du fichier, en conservant son état d'origine et en
    /// agrégeant le statut des actions qui le visent.
    fn project_onto(&mut self, path: &str, before: Option<String>, after: String, statut: Status) {
        match self.files.iter_mut().find(|f| f.path == path) {
            Some(file) => {
                file.after = after;
                file.statut = file.statut.merge(statut);
            }
            None => self.files.push(File {
                path: path.to_string(),
                before,
                after,
                statut,
            }),
        }
    }
}

/// Ce qu'une remise en place d'ancre a pu faire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Repose {
    /// L'ancre est reposée : le plan porte le fichier réécrit.
    Reposee,
    /// L'ancre est laissée où elle n'est pas, et voici pourquoi.
    Laissee(crate::anchors::Cause),
}

/// Les deux lectures dont une action a besoin pour se planifier.
struct States {
    /// Contenu du fichier tel que la planification a trouvé le projet, `None` s'il n'y
    /// existait pas.
    origin: Option<String>,
    /// Contenu du fichier tel que les actions déjà planifiées le laisseront.
    courant: Option<String>,
}

/// Statut d'une action qui compose avec ce qu'elle trouve — une insertion, un patch.
///
/// Elle n'est sans effet que si le projet d'origine porte déjà ce qu'elle produit ; elle
/// n'entre jamais en conflit, puisqu'elle ne remplace pas un fichier entier.
fn combined_status(origin: Option<&str>, after: &str) -> Status {
    if origin == Some(after) {
        Status::DejaFait
    } else {
        Status::AFaire
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchors;
    use std::fs;
    use tempfile::TempDir;

    fn project() -> TempDir {
        TempDir::new().expect("le répertoire temporaire se crée")
    }

    const ROUTER: &str = "pub fn router() -> Router {\n    Router::new()\n        // <rbs:routes>\n        // </rbs:routes>\n}\n";

    fn with_router(project: &TempDir, source: &str) {
        fs::create_dir_all(project.path().join("src")).expect("le répertoire se crée");
        fs::write(project.path().join("src/router.rs"), source).expect("l'écriture aboutit");
    }

    fn fichier(path: &str, before: Option<&str>, statut: Status) -> File {
        File {
            path: path.to_string(),
            before: before.map(str::to_string),
            after: "après".to_string(),
            statut,
        }
    }

    /// La règle d'`application::apply` : un fichier inchangé n'est pas réécrit, un
    /// conflit ne l'est que sous `--force`.
    #[test]
    fn the_summary_counts_what_the_application_writes() {
        let plan = Plan {
            root: std::path::PathBuf::from("/projets/demo-api"),
            actions: Vec::new(),
            files: vec![
                fichier("Dockerfile", None, Status::AFaire),
                fichier("src/notes/mod.rs", None, Status::AFaire),
                fichier("src/notes/dto.rs", None, Status::AFaire),
                fichier("src/router.rs", Some("avant"), Status::AFaire),
                fichier("Cargo.toml", Some("avant"), Status::AFaire),
                fichier("src/lib.rs", Some("après"), Status::DejaFait),
                fichier("src/main.rs", Some("autre"), Status::Conflit),
            ],
            sautees: Vec::new(),
        };

        assert_eq!(
            plan.bilan(false),
            Bilan {
                crees: 3,
                modifies: 2
            }
        );
        assert_eq!(
            plan.bilan(true),
            Bilan {
                crees: 3,
                modifies: 3
            }
        );
    }

    #[test]
    fn a_file_written_on_disk_is_reported_as_existing() {
        let root = TempDir::new().expect("répertoire temporaire créable");
        fs::write(root.path().join("present.yml"), "services:\n").expect("écriture possible");
        let builder = Builder::new(root.path());

        assert!(builder.exists("present.yml").expect("lecture possible"));
        assert!(!builder.exists("absent.yml").expect("lecture possible"));
    }

    #[test]
    fn creating_a_missing_file_is_todo() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent, rien ne s'y oppose");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert_eq!(plan.files()[0].before, None);
        assert_eq!(plan.files()[0].after, "FROM rust\n");
    }

    #[test]
    fn creating_an_already_identical_file_is_done() {
        let project = project();
        fs::write(project.path().join("Dockerfile"), "FROM rust\n").expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].before.as_deref(), Some("FROM rust\n"));
    }

    #[test]
    fn creating_over_different_content_is_a_conflict() {
        let project = project();
        fs::write(project.path().join("Dockerfile"), "FROM alpine\n").expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::Conflit);
        assert_eq!(plan.files()[0].before.as_deref(), Some("FROM alpine\n"));
        assert_eq!(plan.files()[0].after, "FROM rust\n");
    }

    #[test]
    fn planning_a_creation_does_not_write_the_file() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent");
        builder.finir();

        assert!(!project.path().join("Dockerfile").exists());
    }

    #[test]
    fn inserting_into_an_empty_anchor_is_todo() {
        let project = project();
        with_router(&project, ROUTER);
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert_eq!(plan.actions()[0].path, "src/router.rs");
        assert!(
            plan.files()[0]
                .after
                .contains(".merge(crate::users::routes())")
        );
    }

    #[test]
    fn inserting_an_already_present_line_is_done() {
        let project = project();
        with_router(
            &project,
            &ROUTER.replace(
                "        // </rbs:routes>",
                "        .merge(crate::users::routes())\n        // </rbs:routes>",
            ),
        );
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(
            plan.files()[0].before.as_deref(),
            Some(plan.files()[0].after.as_str())
        );
    }

    #[test]
    fn two_insertions_in_one_file_chain_onto_a_single_file() {
        let project = project();
        let lib = "// <rbs:migration_modules>\n// </rbs:migration_modules>\nvec![\n    // <rbs:migrations>\n    // </rbs:migrations>\n]\n";
        fs::create_dir_all(project.path().join("migration/src")).expect("le répertoire se crée");
        fs::write(project.path().join("migration/src/lib.rs"), lib).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert(
                anchors::MIGRATION_MODULES,
                &["mod m20260826_creer_users;".to_string()],
            )
            .expect("l'ancre est présente");
        builder
            .insert(
                anchors::MIGRATIONS,
                &["Box::new(m20260826_creer_users::Migration),".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions().len(), 2);
        assert_eq!(plan.files().len(), 1);
        assert!(plan.files()[0].after.contains("mod m20260826_creer_users;"));
        assert!(
            plan.files()[0]
                .after
                .contains("Box::new(m20260826_creer_users::Migration),")
        );
        assert_eq!(plan.files()[0].before.as_deref(), Some(lib));
    }

    #[test]
    fn a_missing_anchor_stops_the_planning() {
        let project = project();
        with_router(
            &project,
            "pub fn router() -> Router {\n    Router::new()\n}\n",
        );
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("l'ancre manque");

        assert!(matches!(error, Error::Anchor(_)));
    }

    #[test]
    fn inserting_into_a_missing_file_names_the_file_not_the_anchor() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("le fichier n'existe pas");

        assert!(matches!(error, Error::FichierAbsent { .. }), "{error:?}");
        let message = error.to_string();
        assert!(message.contains("src/router.rs"), "{message}");
        assert!(
            !message.contains("<rbs:routes>"),
            "le message parle d'une balise alors que le fichier entier manque : {message}"
        );
    }

    /// Un fichier illisible n'est pas un fichier absent : seul `NotFound` vaut absence.
    #[test]
    fn inserting_into_an_unreadable_file_stays_an_access_error() {
        let project = project();
        fs::create_dir_all(project.path().join("src/router.rs")).expect("le répertoire se crée");
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert(anchors::ROUTES, &["peu importe".to_string()])
            .expect_err("le fichier ne se lit pas");

        assert!(matches!(error, Error::Acces(_)), "{error:?}");
    }

    const COMPOSE: &str = "services:\n  # <rbs:services>\n  # </rbs:services>\n";

    fn mailpit() -> Vec<String> {
        vec![
            "mailpit:".to_string(),
            "  image: axllent/mailpit".to_string(),
        ]
    }

    /// Un projet SQLite n'a pas de compose : l'ancre `services` y est optionnelle, et
    /// la refuser condamnait `mail`, `redis`, `auth` et `webhooks` sur tout projet sans
    /// compose. L'insertion est sautée, et le plan la garde pour l'annoncer.
    #[test]
    fn inserting_into_the_missing_file_of_an_optional_anchor_is_skipped_and_recorded() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert(anchors::SERVICES, &mailpit())
            .expect("l'ancre est optionnelle : son fichier peut manquer");
        let plan = builder.finir();

        assert!(plan.files().is_empty(), "{:?}", plan.files());
        assert!(plan.actions().is_empty(), "{:?}", plan.actions());
        assert_eq!(plan.sautees().len(), 1);
        assert_eq!(plan.sautees()[0].anchor, anchors::SERVICES);
        assert_eq!(plan.sautees()[0].lines, mailpit());
        assert_eq!(plan.sautees()[0].cause, CauseSautee::FichierAbsent);
    }

    /// Le fragment `docker` écrit le compose puis y insère ses services, dans le même
    /// plan : un fichier projeté par une action précédente est présent, et rien ne se saute.
    #[test]
    fn an_optional_anchor_in_a_file_projected_earlier_in_the_plan_is_inserted() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("docker-compose.yml", COMPOSE)
            .expect("le fichier est absent");
        builder
            .insert(anchors::SERVICES, &mailpit())
            .expect("le compose vient d'être projeté");
        let plan = builder.finir();

        assert!(plan.sautees().is_empty(), "{:?}", plan.sautees());
        assert!(
            plan.files()[0].after.contains("mailpit:"),
            "{}",
            plan.files()[0].after
        );
    }

    /// Optionnelle ne veut pas dire facultative dans un fichier présent : un compose
    /// réécrit à la main sans son ancre garde le diagnostic et le bloc à coller.
    #[test]
    fn an_optional_anchor_missing_from_a_present_file_stays_an_anchor_error() {
        let project = project();
        fs::write(
            project.path().join("docker-compose.yml"),
            "services:\n  db:\n    image: postgres\n",
        )
        .expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert(anchors::SERVICES, &mailpit())
            .expect_err("le fichier est là, sans son ancre");

        assert!(matches!(error, Error::Anchor(_)), "{error:?}");
    }

    /// Un projet engendré avant l'ancre porte le fichier, sans elle : c'est le cas que
    /// `insert_ou_sauter` sert. L'insertion est sautée, consignée comme telle, et le
    /// fichier n'est pas touché.
    #[test]
    fn insert_or_skip_skips_an_optional_anchor_missing_from_a_present_file() {
        let project = project();
        let compose = "services:\n  db:\n    image: postgres\n";
        fs::write(project.path().join("docker-compose.yml"), compose).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert_ou_sauter(anchors::SERVICES, &mailpit())
            .expect("l'ancre est optionnelle : son absence saute l'insertion");
        let plan = builder.finir();

        assert!(plan.files().is_empty(), "{:?}", plan.files());
        assert!(plan.actions().is_empty(), "{:?}", plan.actions());
        assert_eq!(
            plan.sautees(),
            [Sautee {
                anchor: anchors::SERVICES,
                lines: mailpit(),
                // `services:` est là : `rbs doctor --fix` saurait reposer l'ancre dessous.
                cause: CauseSautee::AncreAbsente { obstacle: None },
            }]
        );
        assert_eq!(
            fs::read_to_string(project.path().join("docker-compose.yml"))
                .expect("le compose se lit"),
            compose
        );
    }

    /// Obligatoire, l'ancre absente reste une erreur : le projet est alors abîmé, et non
    /// antérieur à elle.
    #[test]
    fn insert_or_skip_keeps_a_mandatory_anchor_missing_from_its_file_an_error() {
        let project = project();
        with_router(
            &project,
            "pub fn router() -> Router {\n    Router::new()\n}\n",
        );
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert_ou_sauter(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("l'ancre des routes n'est pas optionnelle");
        let plan = builder.finir();

        assert!(matches!(error, Error::Anchor(_)), "{error:?}");
        assert!(plan.sautees().is_empty(), "{:?}", plan.sautees());
    }

    #[test]
    fn insert_or_skip_names_a_missing_file_as_the_cause() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert_ou_sauter(anchors::SERVICES, &mailpit())
            .expect("l'ancre est optionnelle : son fichier peut manquer");
        let plan = builder.finir();

        assert_eq!(plan.sautees().len(), 1, "{:?}", plan.sautees());
        assert_eq!(plan.sautees()[0].cause, CauseSautee::FichierAbsent);
    }

    #[test]
    fn insert_or_skip_inserts_into_a_present_anchor() {
        let project = project();
        fs::write(project.path().join("docker-compose.yml"), COMPOSE).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert_ou_sauter(anchors::SERVICES, &mailpit())
            .expect("l'ancre est là");
        let plan = builder.finir();

        assert!(plan.sautees().is_empty(), "{:?}", plan.sautees());
        assert!(plan.files()[0].after.contains("mailpit:"));
    }

    #[test]
    fn inserting_into_a_present_file_without_the_anchor_stays_an_anchor_error() {
        let project = project();
        with_router(
            &project,
            "pub fn router() -> Router {\n    Router::new()\n}\n",
        );
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("l'ancre manque");

        assert!(matches!(error, Error::Anchor(_)), "{error:?}");
        assert!(error.to_string().contains("<rbs:routes>"), "{error}");
    }

    const CARGO: &str = "[package]\nname = \"demo\"\n\n[package.metadata.rbs]\nversion = \"0.1.0\"\nfeatures = [\"health\"]\n";

    #[test]
    fn patching_a_missing_feature_is_todo() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert_eq!(plan.actions()[0].path, "Cargo.toml");
        assert!(plan.files()[0].after.contains("\"docker\""));
    }

    #[test]
    fn patching_an_already_recorded_feature_is_done() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .patch(PatchToml::InscrireFeature("health".to_string()))
            .expect("le manifeste est valide");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].after, CARGO);
    }

    #[test]
    fn patching_a_missing_manifest_is_reported() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect_err("le manifeste manque");

        assert!(matches!(error, Error::ManifesteAbsent { .. }));
        let message = error.to_string();
        assert!(
            message.starts_with("Cargo.toml"),
            "le message ne nomme pas le manifeste comme `Action::path` : {message}"
        );
        assert!(
            !message.contains(&project.path().display().to_string()),
            "le message porte un chemin absolu : {message}"
        );
        assert!(
            message.contains("introuvable"),
            "le message ne dit pas que le fichier manque : {message}"
        );
    }

    const CARGO_DEPS: &str = "[package]\nname = \"demo\"\n\n[dependencies]\naxum = \"0.9\"       # le serveur\n\n[package.metadata.rbs]\nversion = \"0.1.0\"\nfeatures = [\"health\"]\n";

    fn redis() -> PatchToml {
        PatchToml::AjouterDependance(crate::metadata::Dependency {
            name: "redis".to_string(),
            version: "0.32".to_string(),
            features: vec!["tokio-comp".to_string()],
            default_features: true,
        })
    }

    /// Écrit `manifest`, applique `patch`, et rend le plan obtenu.
    fn patched_plan(project: &TempDir, manifest: &str, patch: PatchToml) -> Plan {
        fs::write(project.path().join("Cargo.toml"), manifest).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder.patch(patch).expect("le manifeste est valide");

        builder.finir()
    }

    #[test]
    fn patching_a_missing_dependency_is_todo() {
        let project = project();

        let plan = patched_plan(&project, CARGO_DEPS, redis());

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert_eq!(plan.actions()[0].path, "Cargo.toml");
        assert!(
            plan.files()[0]
                .after
                .contains(r#"redis = { version = "0.32", features = ["tokio-comp"] }"#),
            "{}",
            plan.files()[0].after
        );
    }

    #[test]
    fn patching_an_already_declared_dependency_is_done() {
        let project = project();
        let after = patched_plan(&project, CARGO_DEPS, redis()).files()[0]
            .after
            .clone();

        let plan = patched_plan(&project, &after, redis());

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].after, after);
    }

    #[test]
    fn patching_a_missing_dependency_feature_is_todo() {
        let project = project();
        let patch = PatchToml::AjouterFeatureADependance {
            dependency: "axum".to_string(),
            feature: "macros".to_string(),
        };

        let plan = patched_plan(&project, CARGO_DEPS, patch);

        assert_eq!(plan.actions()[0].statut, Status::AFaire);
        assert!(
            plan.files()[0].after.contains(
                r#"axum = { version = "0.9", features = ["macros"] }       # le serveur"#
            ),
            "{}",
            plan.files()[0].after
        );
    }

    #[test]
    fn patching_an_already_enabled_dependency_feature_is_done() {
        let project = project();
        let patch = || PatchToml::AjouterFeatureADependance {
            dependency: "axum".to_string(),
            feature: "macros".to_string(),
        };
        let after = patched_plan(&project, CARGO_DEPS, patch()).files()[0]
            .after
            .clone();

        let plan = patched_plan(&project, &after, patch());

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].after, after);
    }

    #[test]
    fn patching_a_feature_on_a_missing_dependency_is_reported() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO_DEPS).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        let error = builder
            .patch(PatchToml::AjouterFeatureADependance {
                dependency: "sea-orm".to_string(),
                feature: "with-uuid".to_string(),
            })
            .expect_err("la dépendance manque");

        assert!(matches!(error, Error::Metadata(_)), "{error}");
    }

    #[test]
    fn a_second_patch_of_the_same_feature_stays_todo() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        let plan = builder.finir();

        assert_eq!(
            plan.actions()[1].statut,
            Status::AFaire,
            "le manifeste trouvé ne porte pas `docker` : l'action a bien un effet"
        );
    }

    #[test]
    fn a_second_insertion_of_the_same_line_stays_todo() {
        let project = project();
        with_router(&project, ROUTER);
        let mut builder = Builder::new(project.path().to_path_buf());
        let lines = [".merge(crate::users::routes())".to_string()];

        builder
            .insert(anchors::ROUTES, &lines)
            .expect("l'ancre est présente");
        builder
            .insert(anchors::ROUTES, &lines)
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(
            plan.actions()[1].statut,
            Status::AFaire,
            "le routeur trouvé ne porte pas la ligne : l'action a bien un effet"
        );
        assert_eq!(
            plan.files()[0].after.matches("users::routes").count(),
            1,
            "la ligne a été insérée deux fois"
        );
    }

    #[test]
    fn creating_on_an_already_projected_path_is_rejected() {
        let project = project();
        with_router(&project, ROUTER);
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let error = builder
            .create("src/router.rs", ROUTER)
            .expect_err("une action a déjà projeté ce fichier");
        let plan = builder.finir();

        assert!(matches!(error, Error::DejaProjete { .. }));

        assert_eq!(plan.actions().len(), 1);
        assert!(
            plan.files()[0].after.contains("users::routes"),
            "la projection de l'insertion a été écrasée : {}",
            plan.files()[0].after
        );
    }

    #[test]
    fn creating_the_same_file_twice_is_rejected() {
        let project = project();
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent");
        let error = builder
            .create("Dockerfile", "FROM alpine\n")
            .expect_err("le fichier est déjà projeté");
        let plan = builder.finir();

        assert!(matches!(error, Error::DejaProjete { .. }));
        assert_eq!(plan.actions().len(), 1);
        assert_eq!(plan.files()[0].after, "FROM rust\n");
    }

    #[test]
    fn a_conflicting_file_says_so_without_reading_its_actions() {
        let project = project();
        fs::write(project.path().join("Dockerfile"), "FROM alpine\n").expect("l'écriture aboutit");
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        let plan = builder.finir();

        assert_eq!(plan.files()[0].statut, Status::Conflit);
    }

    #[test]
    fn a_file_whose_every_action_is_a_no_op_is_a_no_op() {
        let project = project();
        let peuple = ROUTER.replace(
            "        // </rbs:routes>",
            "        .merge(crate::users::routes())\n        // </rbs:routes>",
        );
        with_router(&project, &peuple);
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("src/router.rs", &peuple)
            .expect("le fichier se lit");
        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.actions()[1].statut, Status::DejaFait);
        assert_eq!(plan.files()[0].statut, Status::DejaFait);
    }

    #[test]
    fn a_file_mixing_a_no_op_and_a_todo_action_is_todo() {
        let project = project();
        with_router(&project, ROUTER);
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("src/router.rs", ROUTER)
            .expect("le fichier se lit");
        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        let plan = builder.finir();

        assert_eq!(plan.actions()[0].statut, Status::DejaFait);
        assert_eq!(plan.actions()[1].statut, Status::AFaire);
        assert_eq!(plan.files()[0].statut, Status::AFaire);
    }

    #[test]
    fn a_failed_action_leaves_neither_action_nor_file_in_the_plan() {
        let project = project();
        with_router(
            &project,
            "pub fn router() -> Router {\n    Router::new()\n}\n",
        );
        let mut builder = Builder::new(project.path().to_path_buf());

        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier est absent");

        builder
            .insert(anchors::ROUTES, &["peu importe".to_string()])
            .expect_err("l'ancre manque");
        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect_err("le manifeste manque");
        builder
            .create("Dockerfile", "FROM alpine\n")
            .expect_err("le fichier est déjà projeté");

        let plan = builder.finir();

        assert_eq!(plan.actions().len(), 1, "une action en échec a été retenue");
        assert_eq!(plan.files().len(), 1, "un fichier en échec a été projeté");
        assert_eq!(plan.files()[0].after, "FROM rust\n");
    }

    /// Chemin et contenu de chaque fichier du répertoire, trié : deux empreintes égales
    /// valent répertoires identiques.
    fn fingerprint(root: &Path) -> Vec<(String, Vec<u8>)> {
        let mut vus = Vec::new();
        let mut a_parcourir = vec![root.to_path_buf()];

        while let Some(directory) = a_parcourir.pop() {
            for input in fs::read_dir(&directory).expect("le répertoire se lit") {
                let path = input.expect("l'entrée se lit").path();
                if path.is_dir() {
                    a_parcourir.push(path);
                } else {
                    let relatif = path
                        .strip_prefix(root)
                        .expect("le chemin est sous la racine")
                        .display()
                        .to_string();
                    vus.push((relatif, fs::read(&path).expect("le fichier se lit")));
                }
            }
        }

        vus.sort();
        vus
    }

    /// Le critère du lot : ancre absente, rien d'écrit, et le bloc à recoller sous la main.
    #[test]
    fn a_missing_anchor_leaves_the_project_intact_and_gives_the_block_to_paste() {
        let project = project();
        with_router(
            &project,
            &ROUTER
                .replace("        // <rbs:routes>\n", "")
                .replace("        // </rbs:routes>\n", ""),
        );
        let before = fingerprint(project.path());

        let mut builder = Builder::new(project.path().to_path_buf());
        let error = builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect_err("l'ancre manque");
        let plan = builder.finir();

        let Error::Anchor(absente) = error else {
            panic!("le fichier est là, seule l'ancre manque : {error:?}");
        };

        assert_eq!(
            fingerprint(project.path()),
            before,
            "l'échec de planification a touché au disque"
        );
        assert!(plan.files().is_empty(), "un fichier a été projeté");

        let block = absente.anchor.block();
        assert!(block.contains("// <rbs:routes>"), "{block}");
        assert!(block.contains("// </rbs:routes>"), "{block}");
    }

    /// Le critère du lot : une ligne déjà montée ne se réécrit pas au plan suivant.
    #[test]
    fn inserting_the_same_line_twice_changes_nothing_the_second_time() {
        let project = project();
        with_router(&project, ROUTER);
        let lines = [".merge(crate::users::routes())".to_string()];

        let mut builder = Builder::new(project.path().to_path_buf());
        builder
            .insert(anchors::ROUTES, &lines)
            .expect("l'ancre est présente");
        let premier = builder.finir();
        assert_eq!(premier.files()[0].statut, Status::AFaire);

        fs::write(
            project.path().join("src/router.rs"),
            &premier.files()[0].after,
        )
        .expect("l'écriture aboutit");

        let mut builder = Builder::new(project.path().to_path_buf());
        builder
            .insert(anchors::ROUTES, &lines)
            .expect("l'ancre est présente");
        let second = builder.finir();

        assert_eq!(second.files()[0].statut, Status::DejaFait);
        assert_eq!(
            second.files()[0].before.as_deref(),
            Some(second.files()[0].after.as_str()),
            "la seconde planification réécrirait le fichier"
        );
    }

    const AGENTS: &str = "# blog\n\n<!-- rbs:inventory -->\nvieux\n<!-- /rbs:inventory -->\n\n\
                          ## Notes du projet\n\nà moi\n";

    fn with_agents(project: &TempDir, source: &str) {
        fs::write(project.path().join("AGENTS.md"), source).expect("AGENTS.md écrit");
    }

    #[test]
    fn replacing_a_zone_is_todo_and_leaves_the_rest_intact() {
        let project = project();
        with_agents(&project, AGENTS);
        let mut builder = Builder::new(project.path());

        builder
            .replace_zone("AGENTS.md", "inventory", "neuf", None)
            .expect("la zone est présente");

        let plan = builder.finir();
        let file = plan.files().first().expect("une action a visé le fichier");

        assert_eq!(file.statut, Status::AFaire);
        assert!(file.after.contains("neuf"));
        assert!(file.after.contains("## Notes du projet\n\nà moi\n"));
    }

    /// Le statut est ce qui permet à `upgrade` de dire « rien à faire » : une zone déjà
    /// conforme ne doit pas se compter parmi les fichiers à écrire.
    #[test]
    fn replacing_a_zone_by_its_own_content_is_done() {
        let project = project();
        with_agents(&project, AGENTS);
        let mut builder = Builder::new(project.path());

        builder
            .replace_zone("AGENTS.md", "inventory", "vieux", None)
            .expect("la zone est présente");

        assert_eq!(
            builder
                .finir()
                .files()
                .first()
                .expect("un fichier visé")
                .statut,
            Status::DejaFait
        );
    }

    /// Une zone remplacée ne remplace pas un fichier : elle ne peut pas entrer en
    /// conflit, et ne doit donc jamais réclamer `--force`.
    #[test]
    fn replacing_a_zone_never_conflicts() {
        let project = project();
        with_agents(&project, AGENTS);
        let mut builder = Builder::new(project.path());

        builder
            .replace_zone("AGENTS.md", "inventory", "neuf", None)
            .expect("la zone est présente");

        assert_ne!(
            builder
                .finir()
                .files()
                .first()
                .expect("un fichier visé")
                .statut,
            Status::Conflit
        );
    }

    #[test]
    fn replacing_a_versioned_zone_rewrites_its_version() {
        let project = project();
        with_agents(
            &project,
            "# blog\n<!-- rbs:guide 1.0.0 -->\nvieux\n<!-- /rbs:guide -->\n",
        );
        let mut builder = Builder::new(project.path());

        builder
            .replace_zone("AGENTS.md", "guide", "neuf", Some("1.2.0"))
            .expect("la zone est présente");

        let plan = builder.finir();
        let after = &plan.files().first().expect("un fichier visé").after;

        assert!(after.contains("<!-- rbs:guide 1.2.0 -->"), "{after}");
    }

    #[test]
    fn a_missing_zone_stops_the_planning_and_names_the_block_to_paste() {
        let project = project();
        with_agents(&project, "# blog\n");
        let mut builder = Builder::new(project.path());

        let error = builder
            .replace_zone("AGENTS.md", "inventory", "neuf", None)
            .expect_err("la zone manque");

        assert!(error.to_string().contains("inventory"), "{error}");

        // Le nom seul ne répare rien : ce que le développeur colle, c'est le bloc, et
        // c'est donc lui que l'erreur doit porter jusqu'à l'affichage.
        let Error::ZoneAbsente { zone, .. } = &error else {
            panic!("une zone absente doit se dire comme telle : {error:?}");
        };
        assert_eq!(
            zone.block(),
            "<!-- rbs:inventory -->\n<!-- /rbs:inventory -->"
        );
    }

    #[test]
    fn replacing_a_zone_of_a_missing_file_names_the_file() {
        let project = project();
        let mut builder = Builder::new(project.path());

        let error = builder
            .replace_zone("AGENTS.md", "inventory", "neuf", None)
            .expect_err("le fichier manque");

        assert!(error.to_string().contains("AGENTS.md"), "{error}");
    }

    #[test]
    fn planning_does_not_modify_the_project_directory() {
        let project = project();
        fs::write(project.path().join("Cargo.toml"), CARGO).expect("l'écriture aboutit");
        with_router(&project, ROUTER);
        fs::write(project.path().join("Dockerfile"), "FROM alpine\n").expect("l'écriture aboutit");

        let before = fingerprint(project.path());

        let mut builder = Builder::new(project.path().to_path_buf());
        builder
            .create("Dockerfile", "FROM rust\n")
            .expect("le fichier se lit");
        builder
            .create("docker-compose.yml", "services:\n")
            .expect("le fichier est absent");
        builder
            .insert(
                anchors::ROUTES,
                &[".merge(crate::users::routes())".to_string()],
            )
            .expect("l'ancre est présente");
        builder
            .patch(PatchToml::InscrireFeature("docker".to_string()))
            .expect("le manifeste est valide");
        let plan = builder.finir();

        assert_eq!(
            fingerprint(project.path()),
            before,
            "la planification a touché au disque"
        );
        assert_eq!(plan.actions().len(), 4);
        assert_eq!(plan.files().len(), 4);
    }

    #[test]
    fn a_vanished_anchor_carries_its_code_and_the_block_to_paste() {
        let error = Error::Anchor(anchors::Missing {
            anchor: anchors::ROUTES,
        });

        assert_eq!(error.code(), "ancre_absente");
        let Error::Anchor(absente) = &error else {
            unreachable!()
        };
        assert_eq!(error.bloc(), Some(absente.anchor.block()));
    }

    #[test]
    fn a_vanished_anchor_names_where_to_paste_the_block() {
        let error = Error::Anchor(anchors::Missing {
            anchor: anchors::ROUTES,
        });

        assert_eq!(
            error.remede(),
            Some(format!(
                "dans {} :\n{}",
                anchors::ROUTES.file,
                anchors::ROUTES.block()
            ))
        );
    }

    #[test]
    fn a_misplaced_anchor_names_the_line_to_move_the_block_above() {
        let error = Error::MalPlacee(Box::new(anchors::Misplaced {
            anchor: anchors::STATE_INIT,
            before: "core: CoreState::new(".to_string(),
            block: "// <rbs:state_init>\n// </rbs:state_init>".to_string(),
        }));

        assert_eq!(
            error.remede(),
            Some(
                "dans src/state.rs, remontez ce bloc au-dessus de `core: CoreState::new(` :\n\
                 // <rbs:state_init>\n// </rbs:state_init>"
                    .to_string()
            )
        );
    }

    #[test]
    fn a_missing_zone_names_the_file_and_the_block_to_paste() {
        let error = Error::ZoneAbsente {
            path: "AGENTS.md".to_string(),
            zone: crate::agents::MissingZone {
                zone: "inventory".to_string(),
            },
        };

        assert_eq!(
            error.remede(),
            Some(
                "dans AGENTS.md, collez ce bloc pour rétablir la zone `rbs:inventory` :\n\
                 <!-- rbs:inventory -->\n<!-- /rbs:inventory -->"
                    .to_string()
            )
        );
    }

    /// La règle que chaque commande doit pouvoir supposer : un bloc à coller sans dire
    /// où le coller n'existe pas, et réciproquement.
    #[test]
    fn remede_is_some_exactly_when_bloc_is_some_for_every_variant() {
        let toml_source: Result<toml_edit::DocumentMut, toml_edit::TomlError> =
            "= invalide".parse();
        let erreurs: Vec<Error> = vec![
            Error::Acces(crate::errors::Acces {
                path: "Cargo.toml".to_string(),
                source: io::Error::other("panne"),
            }),
            Error::DejaProjete {
                path: "src.rs".to_string(),
            },
            Error::Anchor(anchors::Missing {
                anchor: anchors::ROUTES,
            }),
            Error::MalPlacee(Box::new(anchors::Misplaced {
                anchor: anchors::STATE_INIT,
                before: "core: CoreState::new(".to_string(),
                block: "// <rbs:state_init>\n// </rbs:state_init>".to_string(),
            })),
            Error::FichierAbsent {
                path: "src/router.rs".to_string(),
            },
            Error::Metadata(crate::metadata::Error::PasUnProjet {
                path: "Cargo.toml".to_string(),
            }),
            Error::Toml {
                path: "Cargo.toml".to_string(),
                source: toml_source.expect_err("la source est invalide"),
            },
            Error::ManifesteAbsent {
                path: "Cargo.toml".to_string(),
            },
            Error::ZoneAbsente {
                path: "AGENTS.md".to_string(),
                zone: crate::agents::MissingZone {
                    zone: "inventory".to_string(),
                },
            },
        ];

        for erreur in erreurs {
            assert_eq!(
                erreur.remede().is_some(),
                erreur.bloc().is_some(),
                "{erreur:?}"
            );
        }
    }
}
