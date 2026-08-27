//! Traduit ce qu'un manifeste de fragment déclare en actions de plan.
//!
//! C'est ici, et nulle part ailleurs, qu'`add` cesse d'être générique : le manifeste est
//! lu, ses sections sont interprétées, et le CLI ne connaît toujours aucune feature par
//! son nom.

use std::path::Path;

use minijinja::Value;

use crate::ancres::{self, Ancre};
use crate::generate::montage;
use crate::manifeste::Manifeste;
use crate::plan;
use crate::template::Renderer;
use crate::templates;

/// Où les variables d'environnement d'un fragment sont déclarées.
///
/// C'est `.env.example` et non `.env` : le second porte les secrets réels du développeur,
/// que rbs n'a pas à toucher.
const FICHIER_ENV: &str = ".env.example";

/// Le fragment tel que l'installation le voit.
pub(crate) struct Fragment<'a> {
    /// Nom de la feature, pour les messages d'erreur.
    pub nom: &'a str,
    /// Ce que son manifeste déclare.
    pub manifeste: &'a Manifeste,
    /// Ses templates, telles que la source les a lues.
    pub templates: &'a [templates::Fichier],
    /// Contexte de rendu, déduit du projet visé.
    pub contexte: Value,
    /// Horodatage que portera la migration du fragment.
    ///
    /// Il est reçu et non lu de l'horloge : une planification doit être reproductible.
    pub horodatage: &'a str,
}

/// Ce qui peut empêcher d'interpréter un manifeste.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// Le manifeste désigne une template que le fragment ne porte pas.
    #[error("{feature}/feature.toml déclare `{template}`, absente du fragment")]
    TemplateAbsente {
        /// Feature en cours d'installation.
        feature: String,
        /// Template introuvable, telle que le manifeste la nomme.
        template: String,
    },

    /// Le manifeste vise une ancre que le squelette ne porte pas.
    #[error("{feature}/feature.toml vise l'ancre `{ancre}`, qui n'existe pas : {connues}")]
    AncreInconnue {
        /// Feature en cours d'installation.
        feature: String,
        /// Nom d'ancre refusé.
        ancre: String,
        /// Les ancres du squelette, énumérées.
        connues: String,
    },

    /// Une template ne s'est pas rendue.
    #[error("{fichier} ne se rend pas : {source}")]
    Rendu {
        /// Fichier fautif.
        fichier: String,
        /// Cause du moteur de rendu.
        source: minijinja::Error,
    },

    /// L'action n'a pas pu être planifiée.
    #[error("{0}")]
    Plan(#[from] plan::Erreur),
}

/// Ajoute au plan ce que le manifeste déclare, et rend les chemins déposés.
pub(crate) fn actions(
    fragment: &Fragment,
    constructeur: &mut plan::Constructeur,
) -> Result<Vec<String>, Erreur> {
    let renderer = Renderer::new();
    let mut deposes = Vec::new();

    for (destination, source) in a_deposer(fragment)? {
        let contenu = rendre(&renderer, fragment, source, &destination)?;

        constructeur.creer(&destination, &contenu)?;
        deposes.push(destination);
    }

    if let Some(declaree) = &fragment.manifeste.migration {
        // Le format est celui de `generate crud` et de `migrate new` : deux formats
        // d'horodatage dans un même projet, ce sont deux ordres de migration possibles.
        let module = format!("m{}_{}", fragment.horodatage, declaree.nom);
        let chemin = format!("migration/src/{module}.rs");
        let contenu = rendre(
            &renderer,
            fragment,
            template(fragment, &declaree.source)?,
            &chemin,
        )?;

        constructeur.creer(&chemin, &contenu)?;
        deposes.push(chemin);

        for montage in montage::pour_migration(&module) {
            constructeur.inserer(montage.ancre, &montage.lignes)?;
        }
    }

    for insertion in &fragment.manifeste.ancres {
        let ancre = ancre(fragment, &insertion.ancre)?;
        constructeur.inserer(ancre, &lignes(&insertion.contenu))?;
    }

    for (crate_, patch) in &fragment.manifeste.cargo {
        for feature in &patch.features {
            constructeur.patcher(plan::PatchToml::AjouterFeatureADependance {
                dependance: crate_.clone(),
                feature: feature.clone(),
            })?;
        }
    }

    for section in &fragment.manifeste.config {
        constructeur.ajouter_section(&section.fichier, &section.section, &section.contenu)?;
    }

    for variable in &fragment.manifeste.env {
        constructeur.ajouter_variable(
            FICHIER_ENV,
            &variable.cle,
            &variable.valeur,
            variable.commentaire.as_deref(),
        )?;
    }

    Ok(deposes)
}

/// L'ancre du squelette que le manifeste désigne par `nom`.
///
/// Un nom inconnu est une faute du manifeste : l'ignorer installerait une feature dont
/// le montage manquerait, ce que seule la compilation du projet révélerait.
fn ancre(fragment: &Fragment, nom: &str) -> Result<Ancre, Erreur> {
    ancres::ANCRES
        .into_iter()
        .find(|ancre| ancre.nom == nom)
        .ok_or_else(|| Erreur::AncreInconnue {
            feature: fragment.nom.to_string(),
            ancre: nom.to_string(),
            connues: ancres::ANCRES
                .iter()
                .map(|ancre| ancre.nom)
                .collect::<Vec<_>>()
                .join(", "),
        })
}

/// Découpe le contenu déclaré en lignes à insérer.
///
/// Une ancre en reçoit souvent plusieurs — les cinq chemins OpenAPI d'une feature — et
/// une chaîne TOML multiligne est la façon naturelle de les écrire.
fn lignes(contenu: &str) -> Vec<String> {
    contenu
        .lines()
        .filter(|ligne| !ligne.trim().is_empty())
        .map(|ligne| ligne.trim_end().to_string())
        .collect()
}

/// Rend `source` dans le contexte du fragment, en nommant `destination` si elle échoue.
fn rendre(
    renderer: &Renderer,
    fragment: &Fragment,
    source: &str,
    destination: &str,
) -> Result<String, Erreur> {
    renderer
        .rendre(source, fragment.contexte.clone())
        .map_err(|source| Erreur::Rendu {
            fichier: destination.to_string(),
            source,
        })
}

/// Les templates à déposer, avec leur chemin dans le projet.
///
/// Sans `[[fichiers]]`, le fragment est copié tel quel : un fragment qui n'apporte pas de
/// code Rust n'a rien à déclarer pour que ses fichiers arrivent où leur arborescence les
/// place déjà.
fn a_deposer<'a>(fragment: &'a Fragment) -> Result<Vec<(String, &'a str)>, Erreur> {
    if fragment.manifeste.fichiers.is_empty() {
        return Ok(fragment
            .templates
            .iter()
            .map(|template| {
                (
                    template.destination.to_string_lossy().into_owned(),
                    template.source.as_str(),
                )
            })
            .collect());
    }

    fragment
        .manifeste
        .fichiers
        .iter()
        .map(|declare| {
            let source = template(fragment, &declare.source)?;
            Ok((declare.cible.clone(), source))
        })
        .collect()
}

/// La source de la template que le manifeste désigne par `nom`.
fn template<'a>(fragment: &'a Fragment, nom: &str) -> Result<&'a str, Erreur> {
    fragment
        .templates
        .iter()
        .find(|template| template.origine == Path::new(nom))
        .map(|template| template.source.as_str())
        .ok_or_else(|| Erreur::TemplateAbsente {
            feature: fragment.nom.to_string(),
            template: nom.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use minijinja::context;
    use tempfile::TempDir;

    use super::*;
    use crate::manifeste;

    /// Une template du fragment, telle que la source la restituerait.
    fn template_de(origine: &str, source: &str) -> templates::Fichier {
        let origine = PathBuf::from(origine);
        let destination = if origine
            .extension()
            .is_some_and(|suffixe| suffixe == "jinja")
        {
            origine.with_extension("")
        } else {
            origine.clone()
        };

        templates::Fichier {
            destination,
            origine,
            source: source.to_string(),
        }
    }

    /// Planifie l'installation d'un fragment décrit par son manifeste et ses templates.
    fn planifier(
        racine: &Path,
        manifeste: &str,
        templates: &[templates::Fichier],
    ) -> Result<(Vec<String>, plan::Plan), Erreur> {
        let manifeste = manifeste::lire(manifeste, "essai/feature.toml")
            .expect("le manifeste du test doit être valide");
        let mut constructeur = plan::Constructeur::nouveau(racine.to_path_buf());

        let deposes = actions(
            &Fragment {
                nom: "essai",
                manifeste: &manifeste,
                templates,
                contexte: context! { nom_projet => "demo-api", nom_crate => "demo_api" },
                horodatage: "20260827_120000",
            },
            &mut constructeur,
        )?;

        Ok((deposes, constructeur.finir()))
    }

    #[test]
    fn sans_section_fichiers_le_fragment_est_copie_tel_quel() {
        let projet = TempDir::new().expect("répertoire temporaire créable");
        let templates = [
            template_de("Dockerfile.jinja", "FROM rust\n"),
            template_de(".dockerignore", "target\n"),
        ];

        let (deposes, plan) = planifier(
            projet.path(),
            "[feature]\ndescription = \"docker\"\n",
            &templates,
        )
        .expect("le plan doit se calculer");

        assert_eq!(deposes, ["Dockerfile", ".dockerignore"]);
        assert_eq!(plan.fichiers().len(), 2);
        assert_eq!(plan.fichiers()[0].apres, "FROM rust\n");
    }

    #[test]
    fn une_template_declaree_est_deposee_a_la_cible_qui_l_accompagne() {
        let projet = TempDir::new().expect("répertoire temporaire créable");
        let templates = [template_de(
            "model.rs.jinja",
            "// {@ nom_crate @}\npub struct User;\n",
        )];

        let (deposes, plan) = planifier(
            projet.path(),
            "[feature]\ndescription = \"auth\"\n\n\
             [[fichiers]]\nsource = \"model.rs.jinja\"\ncible = \"src/auth/model.rs\"\n",
            &templates,
        )
        .expect("le plan doit se calculer");

        assert_eq!(deposes, ["src/auth/model.rs"]);
        assert_eq!(plan.fichiers()[0].chemin, "src/auth/model.rs");
        assert_eq!(plan.fichiers()[0].apres, "// demo_api\npub struct User;\n");
    }

    /// Un manifeste de projet réaliste : une dépendance nue, une commentée en fin de
    /// ligne, et un commentaire de bloc.
    const CARGO: &str = "\
[package]
name = \"demo-api\"

[dependencies]
# le noyau, épinglé par `rbs new`
rbs-core = \"0.1.0\"   # ne pas remonter sans relire le CHANGELOG
axum = \"0.8\"
";

    /// Un fragment qui n'exerce que les patchs : manifeste, configuration, environnement.
    const PATCHS: &str = "[feature]\ndescription = \"auth\"\n\n\
         [cargo.rbs-core]\nfeatures = [\"auth\"]\n\n\
         [[config]]\nfichier = \"config/default.toml\"\nsection = \"auth\"\n\
         contenu = \"\"\"\naccess_ttl_secs = 900\nrefresh_ttl_secs = 2592000\n\"\"\"\n\n\
         [[env]]\ncle = \"RBS_AUTH__SECRET\"\nvaleur = \"changez-moi\"\n\
         commentaire = \"Secret de signature HS256, au moins 32 octets\"\n";

    /// Pose les fichiers du projet que les patchs viseront.
    fn avec(racine: &Path, fichiers: &[(&str, &str)]) {
        for (chemin, contenu) in fichiers {
            let cible = racine.join(chemin);
            if let Some(parent) = cible.parent() {
                std::fs::create_dir_all(parent).expect("le répertoire se crée");
            }
            std::fs::write(cible, contenu).expect("le fichier s'écrit");
        }
    }

    /// Le contenu que le plan projette pour `chemin`.
    fn projete<'plan>(plan: &'plan plan::Plan, chemin: &str) -> &'plan str {
        &plan
            .fichiers()
            .iter()
            .find(|fichier| fichier.chemin == chemin)
            .unwrap_or_else(|| panic!("{chemin} absent du plan"))
            .apres
    }

    /// Le critère de la tâche : le patch touche une ligne et laisse les autres intactes.
    #[test]
    fn rbs_core_gagne_la_feature_sans_que_le_reste_soit_reformate() {
        let projet = TempDir::new().expect("répertoire temporaire créable");
        avec(
            projet.path(),
            &[
                ("Cargo.toml", CARGO),
                ("config/default.toml", "[server]\nport = 8080\n"),
                (".env.example", "RBS_DATABASE__URL=postgres://\n"),
            ],
        );

        let (_, plan) = planifier(projet.path(), PATCHS, &[]).expect("le plan doit se calculer");
        let apres = projete(&plan, "Cargo.toml");

        let attendues = CARGO.lines().count();
        assert_eq!(apres.lines().count(), attendues, "{apres}");

        for (rang, (avant, apres)) in CARGO.lines().zip(apres.lines()).enumerate() {
            if avant.starts_with("rbs-core") {
                continue;
            }
            assert_eq!(avant, apres, "la ligne {} a été reformatée", rang + 1);
        }
    }

    /// Le critère de la tâche : ce que le développeur a annoté lui appartient.
    #[test]
    fn les_commentaires_du_developpeur_survivent_au_patch() {
        let projet = TempDir::new().expect("répertoire temporaire créable");
        avec(
            projet.path(),
            &[
                ("Cargo.toml", CARGO),
                ("config/default.toml", "[server]\nport = 8080\n"),
                (".env.example", "RBS_DATABASE__URL=postgres://\n"),
            ],
        );

        let (_, plan) = planifier(projet.path(), PATCHS, &[]).expect("le plan doit se calculer");
        let apres = projete(&plan, "Cargo.toml");

        assert!(
            apres.contains("# le noyau, épinglé par `rbs new`"),
            "le commentaire de bloc a disparu :\n{apres}"
        );
        assert!(
            apres.contains(
                "rbs-core = { version = \"0.1.0\", features = [\"auth\"] }   \
                 # ne pas remonter sans relire le CHANGELOG"
            ),
            "le commentaire de fin de ligne a disparu :\n{apres}"
        );
    }

    /// Le critère de la tâche : la configuration et l'environnement du fragment arrivent.
    #[test]
    fn la_section_de_configuration_et_la_variable_d_environnement_sont_ajoutees() {
        let projet = TempDir::new().expect("répertoire temporaire créable");
        avec(
            projet.path(),
            &[
                ("Cargo.toml", CARGO),
                ("config/default.toml", "[server]\nport = 8080\n"),
                (".env.example", "RBS_DATABASE__URL=postgres://\n"),
            ],
        );

        let (_, plan) = planifier(projet.path(), PATCHS, &[]).expect("le plan doit se calculer");

        let config = projete(&plan, "config/default.toml");
        assert!(config.starts_with("[server]\nport = 8080\n"), "{config}");
        assert!(config.contains("[auth]"), "{config}");
        assert!(config.contains("access_ttl_secs = 900"), "{config}");
        assert!(config.contains("refresh_ttl_secs = 2592000"), "{config}");

        let env = projete(&plan, ".env.example");
        assert!(env.starts_with("RBS_DATABASE__URL=postgres://\n"), "{env}");
        assert!(
            env.contains(
                "# Secret de signature HS256, au moins 32 octets\nRBS_AUTH__SECRET=changez-moi"
            ),
            "{env}"
        );
    }

    /// Le critère de la tâche : un patch déjà posé ne se repose pas.
    #[test]
    fn les_trois_patchs_sont_sans_effet_la_seconde_fois() {
        let projet = TempDir::new().expect("répertoire temporaire créable");
        avec(
            projet.path(),
            &[
                ("Cargo.toml", CARGO),
                ("config/default.toml", "[server]\nport = 8080\n"),
                (".env.example", "RBS_DATABASE__URL=postgres://\n"),
            ],
        );

        let (_, premier) = planifier(projet.path(), PATCHS, &[]).expect("le plan doit se calculer");
        for fichier in premier.fichiers() {
            avec(projet.path(), &[(&fichier.chemin, &fichier.apres)]);
        }

        let (_, second) = planifier(projet.path(), PATCHS, &[]).expect("le plan se recalcule");

        for fichier in second.fichiers() {
            assert_eq!(
                fichier.statut,
                plan::Statut::DejaFait,
                "{} n'est pas sans effet :\n{}",
                fichier.chemin,
                fichier.apres
            );
        }
    }

    /// Une template déclarée mais absente est une faute du manifeste, pas un silence.
    #[test]
    fn une_template_declaree_et_absente_est_signalee_par_son_nom() {
        let projet = TempDir::new().expect("répertoire temporaire créable");

        let erreur = planifier(
            projet.path(),
            "[feature]\ndescription = \"auth\"\n\n\
             [[fichiers]]\nsource = \"absente.rs.jinja\"\ncible = \"src/auth/model.rs\"\n",
            &[],
        )
        .expect_err("la template n'existe pas");

        assert!(matches!(erreur, Erreur::TemplateAbsente { .. }), "{erreur}");
        assert!(erreur.to_string().contains("absente.rs.jinja"), "{erreur}");
    }
}
