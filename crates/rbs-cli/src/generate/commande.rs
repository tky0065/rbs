//! `rbs generate crud` et `rbs generate feature` : la feature écrite dans un projet.
//!
//! La séquence est celle du §4.4 de la spec, dans l'ordre où les échecs restent
//! inoffensifs : le nom, les champs et les ancres sont vérifiés, le rendu aboutit
//! entièrement, et le premier fichier n'est écrit qu'ensuite. Un nom refusé, une ancre
//! disparue ou une feature déjà présente laissent le disque tel qu'ils l'ont trouvé.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::ancres;
use crate::git;
use crate::metadata;

use super::feature::Feature;
use super::{
    champs, controller, dto, entite, essais, migration, montage, nom, repository, service,
};

/// Ce qu'il faut savoir pour générer une feature.
pub(crate) struct Options {
    /// Nom de la feature, au pluriel.
    pub nom: String,
    /// Champs de l'entité, tels que `--fields` les donne.
    pub fields: Option<String>,
    /// `crud` génère l'entité, la migration et les tests ; `feature` s'arrête au squelette.
    pub complete: bool,
    /// Répertoire d'où la commande est lancée.
    pub repertoire: PathBuf,
    /// Génère même si le projet porte des modifications non commitées.
    pub force: bool,
}

/// Un fichier à écrire : son chemin, relatif à la racine du projet, et son contenu.
type Fichier = (String, String);

/// Ce qu'une génération rapporte à son appelant.
#[derive(Debug)]
pub(crate) struct Generee {
    /// Chemins écrits, relatifs à la racine du projet.
    pub fichiers: Vec<String>,
    /// Module de la migration générée, s'il y en a une.
    pub migration: Option<String>,
}

/// Ce qui peut empêcher de générer une feature.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Erreur {
    /// La commande n'a pas été lancée dans un projet rbs.
    #[error("aucun projet rbs ici : `rbs generate` s'exécute dans un projet créé par `rbs new`")]
    PasUnProjet,

    /// Le nom de la feature est inutilisable.
    #[error("{0}")]
    Nom(nom::ErreurNom),

    /// Les champs ne s'analysent pas.
    #[error("{0}")]
    Champs(champs::ErreurChamps),

    /// La feature occupe déjà son répertoire.
    #[error("{chemin} existe déjà : la feature `{feature}` est déjà là")]
    DejaPresente {
        /// Chemin occupé, relatif à la racine.
        chemin: String,
        /// Feature demandée.
        feature: String,
    },

    /// Une ancre attendue a disparu du projet.
    #[error("{0}")]
    Ancre(#[source] ancres::Absente),

    /// Une template ne s'est pas rendue.
    #[error("{fichier} ne se rend pas : {source}")]
    Rendu {
        /// Fichier fautif.
        fichier: String,
        /// Cause du moteur de rendu.
        source: minijinja::Error,
    },

    /// Un fichier du projet n'a pu être lu ou écrit.
    #[error("{chemin} est inaccessible : {source}")]
    Acces {
        /// Chemin fautif.
        chemin: String,
        /// Cause système.
        source: io::Error,
    },

    /// Le projet porte des modifications non commitées, qu'une génération rendrait
    /// indiscernables des siennes.
    #[error("le working tree n'est pas propre : {fichiers} — commitez, ou relancez avec --force")]
    WorkingTreeSale {
        /// Fichiers suivis modifiés, énumérés.
        fichiers: String,
    },

    /// Les métadonnées du projet n'ont pu être mises à jour.
    #[error("{0}")]
    Metadonnees(#[source] metadata::Erreur),
}

impl Erreur {
    /// Ce que le développeur peut coller pour réparer, quand la panne se répare ainsi.
    ///
    /// Seule une ancre disparue a un remède tenant en un bloc de texte : les autres pannes
    /// se règlent par une décision — commiter, choisir un autre nom, corriger un champ.
    pub(crate) fn remede(&self) -> Option<String> {
        match self {
            Erreur::Ancre(absente) => Some(format!(
                "dans {} :\n{}",
                absente.ancre.fichier,
                absente.ancre.bloc()
            )),
            _ => None,
        }
    }
}

/// Génère la feature décrite par `options` dans le projet qui contient son répertoire.
pub(crate) fn executer(options: &Options) -> Result<Generee, Erreur> {
    let depart = options
        .repertoire
        .canonicalize()
        .map_err(|source| acces(&options.repertoire, source))?;
    let racine = metadata::racine_du_projet(&depart).ok_or(Erreur::PasUnProjet)?;

    if !options.force {
        let modifies = git::fichiers_modifies(&racine);
        if !modifies.is_empty() {
            return Err(Erreur::WorkingTreeSale {
                fichiers: enumerer(&modifies),
            });
        }
    }

    nom::valider(&options.nom).map_err(Erreur::Nom)?;
    let champs =
        champs::analyser(options.fields.as_deref().unwrap_or_default()).map_err(Erreur::Champs)?;

    let feature = Feature::nouvelle(&options.nom, champs);
    let module = feature.module().to_string();

    if racine.join("src").join(&module).exists() {
        return Err(Erreur::DejaPresente {
            chemin: format!("src/{module}"),
            feature: module,
        });
    }

    let (fichiers, migration) = rendre(&feature, options.complete)?;
    let modifies = monter(&racine, &module, migration.as_deref())?;

    for (chemin, contenu) in fichiers.iter().chain(modifies.iter()) {
        ecrire(&racine.join(chemin), contenu)?;
    }

    metadata::ajouter_feature(&racine.join("Cargo.toml"), &module).map_err(Erreur::Metadonnees)?;

    Ok(Generee {
        fichiers: fichiers.into_iter().map(|(chemin, _)| chemin).collect(),
        migration,
    })
}

/// Énumère les fichiers en cause, sans dérouler une liste illisible.
fn enumerer(fichiers: &[String]) -> String {
    const NOMMES: usize = 5;

    let debut = fichiers
        .iter()
        .take(NOMMES)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    match fichiers.len().saturating_sub(NOMMES) {
        0 => debut,
        reste => format!("{debut} … et {reste} autres"),
    }
}

/// Rend les fichiers de la feature, et sa migration si elle est complète.
///
/// Rien n'est écrit ici : une template fautive doit échouer avant la première écriture.
fn rendre(feature: &Feature, complete: bool) -> Result<(Vec<Fichier>, Option<String>), Erreur> {
    let module = feature.module();
    let dans = |nom: &str| format!("src/{module}/{nom}");

    let mut fichiers = vec![
        (dans("mod.rs"), controller::rendre_mod(feature, complete)),
        (dans("model.rs"), entite::rendre(feature)),
        (dans("dto.rs"), dto::rendre(feature)),
        (dans("repository.rs"), repository::rendre(feature)),
        (dans("service.rs"), service::rendre(feature)),
        (dans("controller.rs"), controller::rendre(feature)),
    ];

    if complete {
        fichiers.push((dans("tests.rs"), essais::rendre(feature)));
    }

    let mut rendus = Vec::with_capacity(fichiers.len() + 1);
    for (chemin, rendu) in fichiers {
        let contenu = rendu.map_err(|source| Erreur::Rendu {
            fichier: chemin.clone(),
            source,
        })?;
        rendus.push((chemin, contenu));
    }

    if !complete {
        return Ok((rendus, None));
    }

    let rendue =
        migration::rendre(feature, &migration::horodatage_courant()).map_err(|source| {
            Erreur::Rendu {
                fichier: format!("migration de {module}"),
                source,
            }
        })?;

    rendus.push((
        format!("migration/src/{}.rs", rendue.module),
        rendue.contenu,
    ));

    Ok((rendus, Some(rendue.module)))
}

/// Applique les montages de la feature aux ancres du projet, en mémoire.
///
/// Deux montages visent le même fichier — la crate `migration` en porte deux : le second
/// part donc du résultat du premier, et non de ce que le disque contient encore.
fn monter(racine: &Path, module: &str, migration: Option<&str>) -> Result<Vec<Fichier>, Erreur> {
    let mut montages = montage::pour(module);
    if let Some(migration) = migration {
        montages.extend(montage::pour_migration(migration));
    }

    let mut modifies: Vec<Fichier> = Vec::new();

    for montage in montages {
        let chemin = montage.ancre.fichier;
        let source = match modifies.iter().find(|(vise, _)| vise == chemin) {
            Some((_, contenu)) => contenu.clone(),
            None => lire(&racine.join(chemin))?,
        };

        let rendu =
            ancres::inserer(&source, montage.ancre, &montage.lignes).map_err(Erreur::Ancre)?;

        match modifies.iter_mut().find(|(vise, _)| vise == chemin) {
            Some((_, contenu)) => *contenu = rendu,
            None => modifies.push((chemin.to_string(), rendu)),
        }
    }

    Ok(modifies)
}

fn lire(chemin: &Path) -> Result<String, Erreur> {
    fs::read_to_string(chemin).map_err(|source| acces(chemin, source))
}

fn ecrire(chemin: &Path, contenu: &str) -> Result<(), Erreur> {
    if let Some(parent) = chemin.parent() {
        fs::create_dir_all(parent).map_err(|source| acces(parent, source))?;
    }

    fs::write(chemin, contenu).map_err(|source| acces(chemin, source))
}

fn acces(chemin: &Path, source: io::Error) -> Erreur {
    Erreur::Acces {
        chemin: chemin.display().to_string(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use tempfile::TempDir;

    use super::*;
    use crate::generate::banc;

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

    fn options(racine: &Path, nom: &str, fields: Option<&str>, complete: bool) -> Options {
        Options {
            nom: nom.to_string(),
            fields: fields.map(str::to_string),
            complete,
            repertoire: racine.to_path_buf(),
            force: false,
        }
    }

    /// Fait du projet un dépôt dont tout est commité.
    fn commiter(racine: &Path) {
        for arguments in [
            vec!["config", "user.email", "rbs@example.test"],
            vec!["config", "user.name", "rbs"],
            vec!["add", "-A"],
            vec!["commit", "--quiet", "-m", "projet neuf"],
        ] {
            let sortie = std::process::Command::new("git")
                .args(&arguments)
                .current_dir(racine)
                .output()
                .expect("git doit être lançable");

            assert!(
                sortie.status.success(),
                "git {arguments:?} a échoué :\n{}",
                String::from_utf8_lossy(&sortie.stderr)
            );
        }
    }

    /// Modifie un fichier suivi du projet, sans abîmer ce dont la génération a besoin.
    fn salir(racine: &Path) {
        let main = racine.join("src/main.rs");
        let source = lire(&main);

        fs::write(&main, format!("{source}\n// une modification en cours\n"))
            .expect("main.rs réécrivable");
    }

    fn lire(chemin: &Path) -> String {
        fs::read_to_string(chemin)
            .unwrap_or_else(|erreur| panic!("{} illisible : {erreur}", chemin.display()))
    }

    #[test]
    fn un_crud_ecrit_les_sept_fichiers_de_la_feature_et_sa_migration() {
        let (_parent, racine) = projet();

        let generee = executer(&options(&racine, "articles", Some("titre:string"), true))
            .expect("la génération doit aboutir");

        for fichier in [
            "mod.rs",
            "model.rs",
            "dto.rs",
            "repository.rs",
            "service.rs",
            "controller.rs",
            "tests.rs",
        ] {
            assert!(
                racine.join("src/articles").join(fichier).exists(),
                "src/articles/{fichier} manquant"
            );
        }

        let module = generee.migration.expect("un crud porte une migration");
        assert!(
            racine
                .join("migration/src")
                .join(format!("{module}.rs"))
                .exists(),
            "migration {module} manquante"
        );
    }

    #[test]
    fn un_crud_monte_la_feature_dans_les_cinq_ancres() {
        let (_parent, racine) = projet();

        executer(&options(&racine, "articles", Some("titre:string"), true))
            .expect("la génération doit aboutir");

        assert!(lire(&racine.join("src/main.rs")).contains("mod articles;"));
        assert!(lire(&racine.join("src/router.rs")).contains(".merge(crate::articles::routes())"));
        assert!(
            lire(&racine.join("src/openapi.rs")).contains("crate::articles::controller::list,")
        );

        let lib = lire(&racine.join("migration/src/lib.rs"));
        assert!(lib.contains("_create_articles;"), "{lib}");
        assert!(lib.contains("::Migration),"), "{lib}");
    }

    #[test]
    fn une_feature_vide_n_ecrit_ni_migration_ni_tests() {
        let (_parent, racine) = projet();

        let generee =
            executer(&options(&racine, "notes", None, false)).expect("la génération doit aboutir");

        assert!(racine.join("src/notes/controller.rs").exists());
        assert!(
            !racine.join("src/notes/tests.rs").exists(),
            "une feature écrite à la main ne porte pas de tests générés"
        );
        assert_eq!(generee.migration, None);
        assert!(
            !lire(&racine.join("migration/src/lib.rs")).contains("notes"),
            "la crate migration ne doit pas être touchée"
        );
        assert!(
            !lire(&racine.join("src/notes/mod.rs")).contains("mod tests;"),
            "le module de tests ne doit pas être déclaré"
        );
    }

    /// Le troisième critère de D7b, que la commande rend enfin vérifiable.
    #[test]
    fn un_nom_en_conflit_avec_le_squelette_est_refuse_en_nommant_le_conflit() {
        let (_parent, racine) = projet();
        let avant = lire(&racine.join("src/main.rs"));

        let erreur = executer(&options(&racine, "state", Some("titre:string"), true))
            .expect_err("`state` entre en conflit avec le squelette");

        let message = erreur.to_string();
        assert!(message.contains("state"), "{message}");
        assert!(
            message.contains("module"),
            "le message doit nommer le conflit : {message}"
        );
        assert!(
            !racine.join("src/state").is_dir(),
            "un répertoire a été créé"
        );
        assert_eq!(lire(&racine.join("src/main.rs")), avant, "main.rs a bougé");
    }

    /// Le troisième critère de D7b, second cas.
    #[test]
    fn un_mot_cle_rust_est_refuse_en_nommant_le_conflit() {
        let (_parent, racine) = projet();

        let erreur = executer(&options(&racine, "match", Some("titre:string"), true))
            .expect_err("`match` est un mot-clé");

        let message = erreur.to_string();
        assert!(message.contains("match"), "{message}");
        assert!(
            !racine.join("src/match").is_dir(),
            "un répertoire a été créé"
        );
    }

    #[test]
    fn des_champs_fautifs_sont_refuses_avant_toute_ecriture() {
        let (_parent, racine) = projet();

        let erreur = executer(&options(&racine, "articles", Some("titre:chaine"), true))
            .expect_err("`chaine` n'est pas un type");

        assert!(erreur.to_string().contains("chaine"), "{erreur}");
        assert!(
            !racine.join("src/articles").is_dir(),
            "un répertoire a été créé"
        );
    }

    #[test]
    fn une_feature_deja_presente_est_refusee() {
        let (_parent, racine) = projet();
        executer(&options(&racine, "articles", Some("titre:string"), true))
            .expect("la première génération doit aboutir");

        let erreur = executer(&options(&racine, "articles", Some("titre:string"), true))
            .expect_err("la feature est déjà là");

        assert!(erreur.to_string().contains("articles"), "{erreur}");
    }

    #[test]
    fn la_feature_est_inscrite_dans_les_metadonnees_du_projet() {
        let (_parent, racine) = projet();

        executer(&options(&racine, "articles", Some("titre:string"), true))
            .expect("la génération doit aboutir");

        let metadonnees = metadata::lire(&racine.join("Cargo.toml")).expect("métadonnées lisibles");
        assert!(
            metadonnees.features.contains(&"articles".to_string()),
            "{metadonnees:?}"
        );
    }

    #[test]
    fn une_ancre_disparue_donne_le_bloc_a_recoller() {
        let erreur = Erreur::Ancre(ancres::Absente {
            ancre: ancres::ROUTES,
        });

        let remede = erreur.remede().expect("une ancre disparue se recolle");

        assert!(remede.contains("src/router.rs"), "{remede}");
        assert!(remede.contains("// <rbs:routes>"), "{remede}");
        assert!(remede.contains("// </rbs:routes>"), "{remede}");
    }

    #[test]
    fn une_erreur_sans_remede_connu_n_en_invente_pas() {
        assert_eq!(Erreur::PasUnProjet.remede(), None);
    }

    #[test]
    fn hors_d_un_projet_rbs_rien_n_est_ecrit() {
        let ailleurs = TempDir::new().expect("répertoire temporaire créable");

        let erreur = executer(&options(ailleurs.path(), "articles", None, true))
            .expect_err("il n'y a pas de projet ici");

        assert!(matches!(erreur, Erreur::PasUnProjet), "{erreur}");
        assert!(!ailleurs.path().join("src").exists());
    }

    #[test]
    fn la_commande_se_lance_depuis_un_sous_repertoire_du_projet() {
        let (_parent, racine) = projet();

        executer(&options(
            &racine.join("src"),
            "articles",
            Some("titre:string"),
            true,
        ))
        .expect("la racine se retrouve en remontant");

        assert!(racine.join("src/articles/mod.rs").exists());
    }

    /// L'ancre disparue arrête la commande : le CLI ne réécrit jamais ce qu'il ne
    /// reconnaît pas, et n'écrit rien du reste non plus.
    #[test]
    fn une_ancre_absente_arrete_la_commande_sans_rien_ecrire() {
        let (_parent, racine) = projet();
        let routeur = racine.join("src/router.rs");
        let ampute = lire(&routeur).replace("// <rbs:routes>", "");
        fs::write(&routeur, ampute).expect("routeur écrivable");

        let erreur = executer(&options(&racine, "articles", Some("titre:string"), true))
            .expect_err("l'ancre des routes a disparu");

        assert!(matches!(erreur, Erreur::Ancre(_)), "{erreur}");
        assert!(
            !racine.join("src/articles").is_dir(),
            "des fichiers ont été écrits malgré l'ancre absente"
        );
        assert!(!lire(&racine.join("src/main.rs")).contains("mod articles;"));
    }

    /// Ce que rbs écrit doit rester défaisable par un `git checkout` : il ne peut donc pas
    /// mêler ses fichiers à des modifications que le développeur n'a pas commitées.
    #[test]
    fn un_projet_sale_refuse_la_generation_et_n_ecrit_rien() {
        let (_parent, racine) = projet();
        commiter(&racine);
        salir(&racine);

        let erreur = executer(&options(&racine, "notes", None, false))
            .expect_err("le working tree n'est pas propre");

        assert!(matches!(erreur, Erreur::WorkingTreeSale { .. }), "{erreur}");
        assert!(
            erreur.to_string().contains("src/main.rs"),
            "le fichier en cause doit être nommé : {erreur}"
        );
        assert!(
            !racine.join("src/notes").is_dir(),
            "des fichiers ont été écrits malgré le working tree sale"
        );
    }

    #[test]
    fn un_projet_sale_accepte_la_generation_avec_force() {
        let (_parent, racine) = projet();
        commiter(&racine);
        salir(&racine);

        executer(&Options {
            force: true,
            ..options(&racine, "notes", None, false)
        })
        .expect("`--force` passe outre");

        assert!(racine.join("src/notes/controller.rs").exists());
    }

    #[test]
    fn un_projet_hors_depot_git_genere_sans_force() {
        let (_parent, racine) = projet();
        fs::remove_dir_all(racine.join(".git")).expect("dépôt supprimable");

        executer(&options(&racine, "notes", None, false))
            .expect("hors dépôt, il n'y a rien à protéger");

        assert!(racine.join("src/notes/controller.rs").exists());
    }

    /// Le critère du lot : le projet compile après génération d'une feature vide.
    ///
    /// Un CRUD est généré avec elle : c'est la même commande, et rien ne prouverait
    /// autrement que ce qu'elle écrit dans un vrai projet forme du Rust valide. Le CRUD
    /// exercé contre une base est l'affaire du test d'intégration du lot.
    #[test]
    #[ignore = "compile un projet Axum + SeaORM complet"]
    fn le_projet_compile_apres_generation_d_une_feature_vide() {
        let projet = banc::Projet::neuf();

        for arguments in [
            vec!["generate", "feature", "notes"],
            vec![
                "generate",
                "crud",
                "carnets",
                "--fields",
                "titre:string,email:string:unique,vues:int,publie:bool,publie_le:datetime",
            ],
        ] {
            Command::cargo_bin("rbs")
                .expect("le binaire rbs doit être compilé")
                .current_dir(projet.racine())
                .args(&arguments)
                .assert()
                .success();
        }

        projet.compiler();
    }
}
