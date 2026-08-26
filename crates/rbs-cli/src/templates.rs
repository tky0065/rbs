//! Provenance et lecture des templates : le squelette de projet, et les fragments de
//! feature qu'`rbs add` y dépose.
//!
//! Le binaire porte les deux arborescences en lui, pour qu'une installation depuis
//! crates.io n'ait besoin d'aucun fichier externe ; `--template-dir` leur substitue un
//! répertoire du disque, ce dont le développement de rbs a besoin à chaque retouche d'une
//! template.

use std::io;
use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

/// Suffixe que porte toute template, et que ne porte aucune destination.
const SUFFIXE: &str = "jinja";

/// Le squelette de projet, embarqué au moment de la compilation du binaire.
static PROJET: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/project");

/// Les fragments de feature, un sous-répertoire par feature installable.
static FEATURES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/features");

/// Provenance des templates.
#[derive(Debug)]
pub enum Source {
    /// Une arborescence embarquée dans le binaire.
    Embarquees(&'static Dir<'static>),
    /// Un répertoire du disque, donné par `--template-dir`.
    Repertoire(PathBuf),
}

/// Une feature dont aucun fragment n'existe, ni embarqué ni sur le disque.
#[derive(Debug, thiserror::Error)]
#[error("`{feature}` n'est pas une feature installable : {connues}")]
pub struct Inconnue {
    /// Nom demandé.
    pub feature: String,
    /// Les features que la source propose, énumérées.
    pub connues: String,
}

/// Une template et le chemin auquel son rendu sera écrit.
#[derive(Debug)]
pub struct Fichier {
    /// Chemin de sortie relatif à la racine du projet, suffixe `.jinja` retiré.
    pub destination: PathBuf,
    /// Source de la template, telle quelle : le rendu est l'affaire de l'appelant.
    pub source: String,
}

impl Source {
    /// Retient le répertoire donné par `--template-dir`, ou l'embarqué à défaut.
    pub fn nouvelle(repertoire: Option<&Path>) -> Self {
        match repertoire {
            Some(chemin) => Self::Repertoire(chemin.to_path_buf()),
            None => Self::Embarquees(&PROJET),
        }
    }

    /// S'ouvre sur le fragment d'une feature, sous le répertoire donné ou dans l'embarqué.
    ///
    /// Une feature sans fragment est refusée ici plutôt qu'au rendu : un catalogue vide
    /// produirait un plan vide, donc une commande qui réussit sans rien faire.
    pub fn feature(repertoire: Option<&Path>, feature: &str) -> Result<Self, Inconnue> {
        match repertoire {
            Some(chemin) => {
                let fragment = chemin.join(feature);

                if fragment.is_dir() {
                    Ok(Self::Repertoire(fragment))
                } else {
                    Err(Inconnue {
                        feature: feature.to_owned(),
                        connues: enumerer(noms_du_disque(chemin)),
                    })
                }
            }
            None => FEATURES
                .get_dir(feature)
                .map(Self::Embarquees)
                .ok_or_else(|| Inconnue {
                    feature: feature.to_owned(),
                    connues: enumerer(noms_embarques()),
                }),
        }
    }

    /// Lit toutes les templates, triées par destination.
    ///
    /// Le tri n'est pas cosmétique : `include_dir` et `fs::read_dir` ne rendent pas leurs
    /// entrées dans le même ordre, et le second n'en garantit aucun.
    pub fn fichiers(&self) -> io::Result<Vec<Fichier>> {
        let mut fichiers = Vec::new();

        match self {
            Self::Embarquees(racine) => lire_embarquees(racine, racine.path(), &mut fichiers)?,
            Self::Repertoire(racine) => lire_repertoire(racine, racine, &mut fichiers)?,
        }

        fichiers.sort_by(|gauche, droite| gauche.destination.cmp(&droite.destination));

        Ok(fichiers)
    }
}

/// Les features dont le binaire porte un fragment, triées.
fn noms_embarques() -> Vec<String> {
    let mut noms: Vec<String> = FEATURES
        .dirs()
        .filter_map(|dir| dir.path().file_name())
        .map(|nom| nom.to_string_lossy().into_owned())
        .collect();

    noms.sort();
    noms
}

/// Les features qu'un `--template-dir` propose, triées, ou rien s'il est illisible.
fn noms_du_disque(repertoire: &Path) -> Vec<String> {
    let Ok(entrees) = std::fs::read_dir(repertoire) else {
        return Vec::new();
    };

    let mut noms: Vec<String> = entrees
        .flatten()
        .filter(|entree| entree.path().is_dir())
        .map(|entree| entree.file_name().to_string_lossy().into_owned())
        .collect();

    noms.sort();
    noms
}

/// Rend une liste de features lisible dans un message d'erreur.
fn enumerer(noms: Vec<String>) -> String {
    if noms.is_empty() {
        "aucune n'est disponible".to_string()
    } else {
        noms.join(", ")
    }
}

fn lire_embarquees(
    repertoire: &Dir<'static>,
    base: &Path,
    fichiers: &mut Vec<Fichier>,
) -> io::Result<()> {
    for sous_repertoire in repertoire.dirs() {
        lire_embarquees(sous_repertoire, base, fichiers)?;
    }

    for fichier in repertoire.files() {
        // Une template non-UTF-8 est une template qu'aucun rendu ne traversera : la
        // laisser passer déplacerait l'échec dans l'écriture du projet.
        let source = fichier.contents_utf8().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} n'est pas de l'UTF-8", fichier.path().display()),
            )
        })?;

        // Le chemin d'un fichier embarqué est relatif à la racine de l'`include_dir!`, et
        // non au fragment ouvert : sans ce retrait, `add docker` viserait `docker/Dockerfile`.
        let relatif = fichier.path().strip_prefix(base).unwrap_or(fichier.path());

        fichiers.push(Fichier {
            destination: destination(relatif),
            source: source.to_owned(),
        });
    }

    Ok(())
}

fn lire_repertoire(
    racine: &Path,
    repertoire: &Path,
    fichiers: &mut Vec<Fichier>,
) -> io::Result<()> {
    let entrees = std::fs::read_dir(repertoire).map_err(|erreur| nommer(repertoire, erreur))?;

    for entree in entrees {
        let chemin = entree.map_err(|erreur| nommer(repertoire, erreur))?.path();

        if chemin.is_dir() {
            lire_repertoire(racine, &chemin, fichiers)?;
            continue;
        }

        let source = std::fs::read_to_string(&chemin).map_err(|erreur| nommer(&chemin, erreur))?;
        let relatif = chemin.strip_prefix(racine).unwrap_or(&chemin);

        fichiers.push(Fichier {
            destination: destination(relatif),
            source,
        });
    }

    Ok(())
}

/// Retire le suffixe `.jinja` du chemin d'une template.
///
/// C'est l'unique endroit du CLI où la convention du §1 du design du squelette
/// s'applique : tout le reste du code ne voit que des destinations. Un chemin sans
/// suffixe traverse intact — le refuser transformerait la faute de frappe d'un
/// `--template-dir` en erreur incompréhensible.
fn destination(template: &Path) -> PathBuf {
    if template
        .extension()
        .is_some_and(|suffixe| suffixe == SUFFIXE)
    {
        template.with_extension("")
    } else {
        template.to_path_buf()
    }
}

/// Rejoue une erreur d'entrée-sortie en nommant le chemin en cause.
///
/// Un `--template-dir` mal saisi est l'erreur la plus probable de ce flag, et
/// « No such file or directory » seul ne la corrige pas.
fn nommer(chemin: &Path, erreur: io::Error) -> io::Error {
    io::Error::new(erreur.kind(), format!("{} : {erreur}", chemin.display()))
}

#[cfg(test)]
mod tests {
    //! Ces templates sont une interface : les commandes de génération écrivent dans leurs
    //! ancres, et un projet déjà déroulé ne bénéficie d'aucune correction faite après
    //! coup. On vérifie donc en permanence ce qui ne dépend pas d'un rendu complet — la
    //! convention de nommage, les ancres, et l'absence de variable non déclarée.

    use std::fs;
    use std::path::{Path, PathBuf};

    use minijinja::{Value, context};

    use super::*;
    use crate::template::Renderer;

    /// Racine des templates du squelette, résolue depuis la crate plutôt que depuis le
    /// répertoire courant, que `cargo test` ne garantit pas.
    const RACINE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/project");

    /// Les chemins de sortie attendus du squelette, tels que `rbs new` les écrira.
    const DESTINATIONS: [&str; 15] = [
        ".env",
        ".env.example",
        ".gitignore",
        "Cargo.toml",
        "config/default.toml",
        "config/development.toml",
        "migration/Cargo.toml",
        "migration/src/lib.rs",
        "migration/src/main.rs",
        "src/health/controller.rs",
        "src/health/mod.rs",
        "src/main.rs",
        "src/openapi.rs",
        "src/router.rs",
        "src/state.rs",
    ];

    /// Contexte de rendu minimal : les cinq variables que `rbs new` fournira.
    fn contexte() -> Value {
        context! {
            nom_projet => "mon-api",
            nom_crate => "mon_api",
            rbs_core_dep => "\"0.1\"",
            rbs_version => "0.1.0",
            database_url => "postgres://postgres:postgres@localhost:5432/mon_api",
        }
    }

    /// Toutes les templates du squelette, répertoires imbriqués compris.
    fn templates() -> Vec<PathBuf> {
        let mut trouvees = Vec::new();
        parcourir(Path::new(RACINE), &mut trouvees);

        assert!(
            !trouvees.is_empty(),
            "aucune template trouvée sous {RACINE}"
        );

        trouvees
    }

    fn parcourir(repertoire: &Path, trouvees: &mut Vec<PathBuf>) {
        let entrees = fs::read_dir(repertoire).unwrap_or_else(|erreur| {
            panic!("{} illisible : {erreur}", repertoire.display());
        });

        for entree in entrees {
            let chemin = entree.expect("entrée de répertoire lisible").path();
            if chemin.is_dir() {
                parcourir(&chemin, trouvees);
            } else {
                trouvees.push(chemin);
            }
        }
    }

    fn lire(chemin: &Path) -> String {
        fs::read_to_string(chemin).unwrap_or_else(|erreur| {
            panic!("{} illisible : {erreur}", chemin.display());
        })
    }

    #[test]
    fn chaque_template_porte_le_suffixe_jinja() {
        for chemin in templates() {
            assert_eq!(
                chemin.extension().and_then(|suffixe| suffixe.to_str()),
                Some("jinja"),
                "{} ne porte pas le suffixe `.jinja`",
                chemin.display()
            );
        }
    }

    #[test]
    fn l_ancre_des_features_suit_les_modules_du_squelette_dans_main() {
        let source = lire(&Path::new(RACINE).join("src/main.rs.jinja"));

        let modules = source
            .find("mod state;")
            .expect("les modules du squelette doivent être déclarés");
        let ancre = source
            .find("// <rbs:features>")
            .expect("main.rs doit porter l'ancre des features");

        assert!(
            modules < ancre,
            "l'ancre doit suivre les modules du squelette :\n{source}"
        );
    }

    #[test]
    fn chaque_ancre_est_ouverte_puis_refermee_dans_son_fichier() {
        for ancre in crate::ancres::ANCRES {
            let relatif = format!("{}.jinja", ancre.fichier);
            let source = lire(&Path::new(RACINE).join(&relatif));

            let ouverture = ancre.ouverture();
            let fermeture = ancre.fermeture();

            assert_eq!(
                source.matches(&ouverture).count(),
                1,
                "{relatif} doit porter une fois `{ouverture}`"
            );
            assert_eq!(
                source.matches(&fermeture).count(),
                1,
                "{relatif} doit porter une fois `{fermeture}`"
            );
            assert!(
                source.find(&ouverture) < source.find(&fermeture),
                "{relatif} referme `{}` avant de l'ouvrir",
                ancre.nom
            );
        }
    }

    #[test]
    fn chaque_template_se_rend_avec_les_cinq_variables() {
        let renderer = Renderer::new();

        for chemin in templates() {
            let source = lire(&chemin);
            renderer
                .rendre(&source, contexte())
                .unwrap_or_else(|erreur| {
                    panic!("{} ne se rend pas : {erreur}", chemin.display());
                });
        }
    }

    #[test]
    fn chaque_template_rust_du_squelette_est_conforme_a_rustfmt() {
        // Le workflow d'`rbs add ci` lance `cargo fmt --check` sur le projet généré : un
        // squelette non conforme le fait échouer au premier pas, sur du code que le
        // développeur n'a pas écrit.
        //
        // Le squelette est déroulé en entier plutôt que fichier par fichier : rustfmt suit
        // les déclarations de modules, et un `main.rs` seul ne résout pas ses `mod`.
        let renderer = Renderer::new();
        let temporaire = tempfile::tempdir().expect("répertoire temporaire créable");
        let racine = temporaire.path();

        let fichiers = Source::nouvelle(None)
            .fichiers()
            .expect("les templates embarquées doivent se lire");

        let mut sources = Vec::new();
        for fichier in &fichiers {
            let destination = racine.join(&fichier.destination);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).expect("le répertoire est créable");
            }

            let rendu = renderer
                .rendre(&fichier.source, contexte())
                .unwrap_or_else(|erreur| {
                    panic!(
                        "{} ne se rend pas : {erreur}",
                        fichier.destination.display()
                    )
                });
            fs::write(&destination, rendu).expect("le rendu est écrivable");

            if destination
                .extension()
                .is_some_and(|suffixe| suffixe == "rs")
            {
                sources.push((fichier.destination.clone(), destination));
            }
        }

        assert!(
            !sources.is_empty(),
            "le squelette ne porte aucun fichier Rust"
        );

        for (relatif, chemin) in sources {
            let sortie = std::process::Command::new("rustfmt")
                .args(["--edition", "2024", "--check"])
                .arg(&chemin)
                .output()
                .expect("rustfmt doit être lançable");

            assert!(
                sortie.status.success(),
                "{} n'est pas conforme à rustfmt :\n{}{}",
                relatif.display(),
                String::from_utf8_lossy(&sortie.stdout),
                String::from_utf8_lossy(&sortie.stderr)
            );
        }
    }

    #[test]
    fn le_manifeste_rendu_porte_le_nom_du_projet_et_la_dependance_au_noyau() {
        let source = lire(&Path::new(RACINE).join("Cargo.toml.jinja"));

        let rendu = Renderer::new()
            .rendre(&source, contexte())
            .expect("le manifeste doit se rendre");

        assert!(
            rendu.contains("name = \"mon-api\""),
            "nom du paquet absent du manifeste rendu :\n{rendu}"
        );
        assert!(
            rendu.contains("rbs-core = \"0.1\""),
            "dépendance au noyau absente du manifeste rendu :\n{rendu}"
        );
    }

    #[test]
    fn la_source_embarquee_restitue_le_squelette_avec_ses_chemins_de_sortie() {
        let fichiers = Source::nouvelle(None)
            .fichiers()
            .expect("les templates embarquées doivent se lire");

        let destinations: Vec<String> = fichiers
            .iter()
            .map(|fichier| fichier.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, DESTINATIONS);

        for fichier in &fichiers {
            assert!(
                !fichier.source.is_empty(),
                "{} est embarquée vide",
                fichier.destination.display()
            );
        }
    }

    #[test]
    fn aucune_destination_ne_porte_le_suffixe_jinja() {
        let fichiers = Source::nouvelle(None)
            .fichiers()
            .expect("les templates embarquées doivent se lire");

        for fichier in fichiers {
            assert_ne!(
                fichier.destination.extension(),
                Some("jinja".as_ref()),
                "{} garde le suffixe `.jinja`",
                fichier.destination.display()
            );
        }
    }

    #[test]
    fn un_repertoire_de_templates_prend_le_pas_sur_l_embarque() {
        let repertoire = tempfile::tempdir().expect("répertoire temporaire créable");
        fs::create_dir(repertoire.path().join("config")).expect("sous-répertoire créable");
        fs::write(
            repertoire.path().join("Cargo.toml.jinja"),
            "name = \"surcharge\"",
        )
        .expect("template écrivable");
        fs::write(
            repertoire.path().join("config/default.toml.jinja"),
            "port = 1",
        )
        .expect("template écrivable");

        let fichiers = Source::nouvelle(Some(repertoire.path()))
            .fichiers()
            .expect("le répertoire doit se lire");

        let destinations: Vec<&Path> = fichiers
            .iter()
            .map(|fichier| fichier.destination.as_path())
            .collect();

        // Comparer des `Path` et non leur rendu : sous Windows, `config/default.toml`
        // s'affiche `config\default.toml`, et l'assertion parlerait du séparateur au
        // lieu de parler de l'ordre des fichiers.
        assert_eq!(
            destinations,
            [
                Path::new("Cargo.toml"),
                &Path::new("config").join("default.toml")
            ]
        );
        assert_eq!(fichiers[0].source, "name = \"surcharge\"");
    }

    #[test]
    fn un_repertoire_de_templates_inexistant_echoue_en_nommant_le_chemin() {
        let absent = Path::new("/introuvable/templates/rbs");

        let erreur = Source::nouvelle(Some(absent))
            .fichiers()
            .expect_err("un répertoire absent ne doit pas rendre une liste vide");

        assert!(
            erreur.to_string().contains("/introuvable/templates/rbs"),
            "le message ne nomme pas le chemin : {erreur}"
        );
    }

    /// Racine des fragments de feature, résolue comme celle du squelette.
    const RACINE_FEATURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/features");

    /// Les chemins de sortie attendus de `docker`, tels que `rbs add docker` les écrira.
    const DESTINATIONS_DOCKER: [&str; 3] = [".dockerignore", "Dockerfile", "docker-compose.yml"];

    /// Contexte de rendu d'un fragment : les deux variables qu'un projet existant fournit.
    fn contexte_feature() -> Value {
        context! {
            nom_projet => "mon-api",
            nom_crate => "mon_api",
        }
    }

    /// Toutes les templates de tous les fragments de feature.
    fn templates_de_features() -> Vec<PathBuf> {
        let mut trouvees = Vec::new();
        parcourir(Path::new(RACINE_FEATURES), &mut trouvees);

        assert!(
            !trouvees.is_empty(),
            "aucun fragment trouvé sous {RACINE_FEATURES}"
        );

        trouvees
    }

    #[test]
    fn la_source_d_une_feature_restitue_ses_fichiers_embarques() {
        let fichiers = Source::feature(None, "docker")
            .expect("`docker` doit être une feature connue")
            .fichiers()
            .expect("les templates embarquées doivent se lire");

        let destinations: Vec<String> = fichiers
            .iter()
            .map(|fichier| fichier.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, DESTINATIONS_DOCKER);

        for fichier in &fichiers {
            assert!(
                !fichier.source.is_empty(),
                "{} est embarquée vide",
                fichier.destination.display()
            );
        }
    }

    #[test]
    fn une_feature_inconnue_est_signalee_par_son_nom() {
        let erreur = Source::feature(None, "auth")
            .expect_err("`auth` n'existe pas encore : la source ne doit pas être vide");

        assert!(
            erreur.to_string().contains("auth"),
            "le message ne nomme pas la feature : {erreur}"
        );
        assert!(
            erreur.to_string().contains("ci, docker"),
            "le message n'énumère pas les features installables : {erreur}"
        );
    }

    #[test]
    fn un_repertoire_de_templates_prend_le_pas_pour_une_feature() {
        let repertoire = tempfile::tempdir().expect("répertoire temporaire créable");
        fs::create_dir(repertoire.path().join("docker")).expect("sous-répertoire créable");
        fs::write(
            repertoire.path().join("docker/Dockerfile.jinja"),
            "FROM surcharge",
        )
        .expect("template écrivable");

        let fichiers = Source::feature(Some(repertoire.path()), "docker")
            .expect("le répertoire doit fournir la feature")
            .fichiers()
            .expect("le répertoire doit se lire");

        let destinations: Vec<String> = fichiers
            .iter()
            .map(|fichier| fichier.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, ["Dockerfile"]);
        assert_eq!(fichiers[0].source, "FROM surcharge");
    }

    #[test]
    fn chaque_template_de_feature_porte_le_suffixe_jinja() {
        for chemin in templates_de_features() {
            assert_eq!(
                chemin.extension().and_then(|suffixe| suffixe.to_str()),
                Some("jinja"),
                "{} ne porte pas le suffixe `.jinja`",
                chemin.display()
            );
        }
    }

    #[test]
    fn chaque_template_de_feature_se_rend_avec_son_contexte() {
        let renderer = Renderer::new();

        for chemin in templates_de_features() {
            let source = lire(&chemin);
            renderer
                .rendre(&source, contexte_feature())
                .unwrap_or_else(|erreur| {
                    panic!("{} ne se rend pas : {erreur}", chemin.display());
                });
        }
    }

    #[test]
    fn le_compose_de_docker_ne_publie_que_le_port_de_l_api() {
        // Publier 5432 fait échouer `docker compose up` sur toute machine portant déjà un
        // PostgreSQL, et la base n'a pas à être jointe depuis l'hôte : l'API l'atteint par
        // le réseau du compose, et `docker compose exec db psql` reste ouvert.
        let source = lire(&Path::new(RACINE_FEATURES).join("docker/docker-compose.yml.jinja"));

        let publies: Vec<&str> = source
            .lines()
            .map(str::trim)
            .filter(|ligne| ligne.starts_with("- \""))
            .collect();

        assert_eq!(publies, ["- \"8080:8080\""], "ports publiés :\n{source}");
    }

    #[test]
    fn la_source_de_ci_restitue_son_workflow() {
        let fichiers = Source::feature(None, "ci")
            .expect("`ci` doit être une feature connue")
            .fichiers()
            .expect("les templates embarquées doivent se lire");

        let destinations: Vec<String> = fichiers
            .iter()
            .map(|fichier| fichier.destination.to_string_lossy().into_owned())
            .collect();

        assert_eq!(destinations, [".github/workflows/ci.yml"]);
    }

    #[test]
    fn le_workflow_de_ci_amene_une_base_migree_avant_les_tests() {
        // Les tests d'une feature générée montent l'application sur une vraie base et
        // supposent les migrations appliquées : sans elles, la CI échoue sur un schéma
        // absent, loin de sa cause.
        let source = lire(&Path::new(RACINE_FEATURES).join("ci/.github/workflows/ci.yml.jinja"));

        assert!(
            source.contains("postgres:18"),
            "le workflow n'amène pas PostgreSQL 18 :\n{source}"
        );

        let migrations = source
            .find("-p migration")
            .expect("le workflow doit appliquer les migrations");
        let tests = source
            .find("cargo test")
            .expect("le workflow doit lancer les tests");

        assert!(
            migrations < tests,
            "les migrations doivent précéder les tests :\n{source}"
        );
    }

    #[test]
    fn le_builder_de_docker_installe_ce_dont_le_build_a_besoin() {
        // `utoipa-swagger-ui` télécharge son archive pendant la compilation, avec `curl`,
        // que l'image `rust:slim` ne porte pas : sans lui le build casse à la toute fin,
        // après plusieurs minutes de compilation.
        let source = lire(&Path::new(RACINE_FEATURES).join("docker/Dockerfile.jinja"));
        let builder = source
            .split("AS runtime")
            .next()
            .expect("le Dockerfile doit avoir une étape de build");

        assert!(
            builder.contains("curl"),
            "l'étape de build n'installe pas curl :\n{builder}"
        );
    }

    #[test]
    fn le_compose_de_docker_vise_postgres_18() {
        // `uuidv7()` n'est natif qu'à partir de PostgreSQL 18, et toute entité générée en
        // dépend : une image plus ancienne casse le projet sans casser la compilation.
        let source = lire(&Path::new(RACINE_FEATURES).join("docker/docker-compose.yml.jinja"));

        assert!(
            source.contains("postgres:18"),
            "le compose ne vise pas PostgreSQL 18 :\n{source}"
        );
    }
}
