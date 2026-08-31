//! L'`AGENTS.md` du projet engendré : ses zones, son inventaire, son guide.
//!
//! Deux zones seulement appartiennent à rbs, délimitées par des commentaires HTML. Le
//! reste du fichier est au développeur, et n'est jamais relu. Chaque zone est régénérée
//! en entier plutôt que complétée ligne à ligne : l'idempotence est alors acquise par
//! construction, là où l'insertion incrémentale demande un dédoublonnage qui a déjà coûté
//! plusieurs correctifs aux ancres du code.

use std::path::Path;

use crate::anchors;
use crate::lang::Lang;
use crate::metadata;

/// Zone du mode d'emploi, propriété de rbs, versionnée.
pub(crate) const GUIDE: &str = "guide";

/// Zone de l'état du projet, recalculée à chaque écriture.
pub(crate) const INVENTORY: &str = "inventory";

/// Nom du fichier, à la racine du projet.
pub(crate) const FICHIER: &str = "AGENTS.md";

/// Version du CLI qui écrit le guide.
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Ce qui peut empêcher de rendre l'`AGENTS.md` d'un projet.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Les templates du guide n'ont pas pu être lues.
    #[error("les guides AGENTS.md sont illisibles : {0}")]
    Templates(#[source] std::io::Error),

    /// Une template ne s'est pas rendue.
    #[error("le guide AGENTS.md ne se rend pas : {0}")]
    Rendu(#[source] minijinja::Error),

    /// Le manifeste du projet n'a pas pu être lu.
    #[error(transparent)]
    Metadonnees(#[from] metadata::Error),
}

/// Une zone que le document ne porte pas.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("AGENTS.md ne porte pas la zone `rbs:{zone}`")]
pub(crate) struct MissingZone {
    /// Nom de la zone, tel qu'il paraît dans les marqueurs.
    pub zone: String,
}

impl MissingZone {
    /// Le bloc à recoller, prêt à être collé tel quel.
    pub(crate) fn block(&self) -> String {
        format!("{}\n{}", opening(&self.zone, None), closing(&self.zone))
    }
}

/// Marqueur d'ouverture d'une zone, la version comprise quand elle en porte une.
pub(crate) fn opening(zone: &str, version: Option<&str>) -> String {
    match version {
        Some(version) => format!("<!-- rbs:{zone} {version} -->"),
        None => format!("<!-- rbs:{zone} -->"),
    }
}

/// Marqueur de fermeture d'une zone.
pub(crate) fn closing(zone: &str) -> String {
    format!("<!-- /rbs:{zone} -->")
}

/// Remplace le corps de `zone` par `content`, le marqueur d'ouverture inchangé.
pub(crate) fn replace(source: &str, zone: &str, content: &str) -> Result<String, MissingZone> {
    let (debut, fin) = bounds(source, zone)?;
    let ouverture = &source[debut.0..debut.1];

    Ok(splice(source, debut.0, fin.1, ouverture, content, zone))
}

/// Le même, le marqueur d'ouverture réécrit avec `version`.
pub(crate) fn replace_versioned(
    source: &str,
    zone: &str,
    content: &str,
    version: &str,
) -> Result<String, MissingZone> {
    let (debut, fin) = bounds(source, zone)?;

    Ok(splice(
        source,
        debut.0,
        fin.1,
        &opening(zone, Some(version)),
        content,
        zone,
    ))
}

/// Corps de la zone, marqueurs exclus et retours à la ligne bordants retirés.
pub(crate) fn body<'a>(source: &'a str, zone: &str) -> Option<&'a str> {
    let (debut, fin) = bounds(source, zone).ok()?;

    Some(source[debut.1..fin.0].trim_matches('\n'))
}

/// Version que porte le marqueur d'ouverture de `zone`, s'il en porte une.
pub(crate) fn version(source: &str, zone: &str) -> Option<String> {
    let (debut, _) = bounds(source, zone).ok()?;

    let marqueur = &source[debut.0..debut.1];
    let version = marqueur
        .trim_start_matches(&format!("<!-- rbs:{zone}"))
        .trim_end_matches("-->")
        .trim();

    (!version.is_empty()).then(|| version.to_string())
}

/// Position d'un marqueur dans le document, en octets : `(début, fin)`.
type Span = (usize, usize);

/// Bornes des deux marqueurs d'une zone : celles de l'ouverture, puis celles de la
/// fermeture.
///
/// Une ouverture sans fermeture n'est pas une zone : la traiter comme telle emporterait
/// tout ce que le développeur a écrit en dessous.
fn bounds(source: &str, zone: &str) -> Result<(Span, Span), MissingZone> {
    let manquante = || MissingZone {
        zone: zone.to_string(),
    };

    let prefixe = format!("<!-- rbs:{zone}");
    let debut = source.find(&prefixe).ok_or_else(manquante)?;
    let apres = source[debut..].find("-->").ok_or_else(manquante)? + debut + "-->".len();

    let fermeture = closing(zone);
    let fin = source[apres..].find(&fermeture).ok_or_else(manquante)? + apres;

    Ok(((debut, apres), (fin, fin + fermeture.len())))
}

/// Recompose le document autour d'une zone réécrite.
fn splice(
    source: &str,
    debut: usize,
    fin: usize,
    ouverture: &str,
    content: &str,
    zone: &str,
) -> String {
    format!(
        "{}{ouverture}\n{}\n{}{}",
        &source[..debut],
        content.trim_matches('\n'),
        closing(zone),
        &source[fin..]
    )
}

/// Rend le corps de la zone du guide, dans la langue et pour le projet donnés.
///
/// `root` ne sert qu'à résoudre l'ancre des features par
/// [`anchors::resolve_features`] : aucun gabarit n'emploie le nom du projet, seul le
/// titre que compose `document` en a besoin.
pub(crate) fn guide(lang: Lang, root: &Path) -> Result<String, Error> {
    let attendue = format!("{}.md", lang.name());
    let files = crate::templates::Source::agents()
        .files()
        .map_err(Error::Templates)?;

    let template = files
        .iter()
        .find(|file| file.destination == Path::new(&attendue))
        .ok_or_else(|| {
            Error::Templates(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{attendue} manque aux guides"),
            ))
        })?;

    crate::template::Renderer::new()
        .render(
            &template.source,
            minijinja::context! {
                ancres => anchor_list(lang, root),
            },
        )
        .map(|rendu| rendu.trim_matches('\n').to_string())
        .map_err(Error::Rendu)
}

/// Le fichier complet, prêt à être écrit : titre, guide, inventaire, place du développeur.
///
/// `version` est celle à inscrire dans les deux zones — celle du CLI pour un projet neuf
/// ([`new`](crate::new::create)), celle visée par la mise à niveau pour
/// [`upgrade`](crate::upgrade) : jamais celle que le manifeste porte encore sur le disque,
/// qui resterait celle d'avant la mise à niveau le temps d'une seconde passe.
pub(crate) fn document(
    root: &Path,
    lang: Lang,
    project: &str,
    version: &str,
) -> Result<String, Error> {
    let (mode_d_emploi, notes) = match lang {
        Lang::Fr => ("mode d'emploi pour agents", "## Notes du projet"),
        Lang::En => ("agent handbook", "## Project notes"),
    };

    let metadonnees = metadata::read(&root.join("Cargo.toml"))?;
    let inventaire = inventory_of(
        &metadonnees.features,
        version,
        metadonnees.database,
        root,
        lang,
    );

    Ok(format!(
        "# {project} — {mode_d_emploi}\n\n\
         {}\n{}\n{}\n\n\
         {}\n{}\n{}\n\n\
         {notes}\n",
        opening(GUIDE, Some(version)),
        guide(lang, root)?,
        closing(GUIDE),
        opening(INVENTORY, None),
        inventaire,
        closing(INVENTORY),
    ))
}

/// La liste des ancres du registre, rendue en markdown pour la template.
///
/// Calculée et non recopiée : une ancre ajoutée au registre doit apparaître dans le guide
/// sans que personne ait à y penser. Seuls les mots de liaison varient avec `lang` : un
/// guide anglais truffé de « dans » et de « chaque entité » ferait douter de tout le
/// reste du document.
///
/// L'ancre des features est résolue par [`anchors::resolve_features`], comme le fait déjà
/// `present_anchors` pour l'inventaire : la prendre telle quelle dans le registre citerait
/// `src/main.rs` sur un projet qui porte une bibliothèque, en contradiction avec
/// l'inventaire du même document, qui lui dit vrai.
fn anchor_list(lang: Lang, root: &Path) -> String {
    let relie = |anchor: &anchors::Anchor| match lang {
        Lang::Fr => format!("- `<rbs:{}>` dans `{}`", anchor.name, anchor.file),
        Lang::En => format!("- `<rbs:{}>` in `{}`", anchor.name, anchor.file),
    };

    let registre = anchors::resolved(root)
        .iter()
        .map(relie)
        .collect::<Vec<_>>()
        .join("\n");

    let relations = match lang {
        Lang::Fr => {
            "- `<rbs:relations:<table>>` et `<rbs:related:<table>>` dans le modèle de \
             chaque entité"
        }
        Lang::En => {
            "- `<rbs:relations:<table>>` and `<rbs:related:<table>>` in each entity's \
             model"
        }
    };

    format!("{registre}\n{relations}")
}

/// Le module du squelette, qui n'est pas une entité engendrée.
const SQUELETTE: &str = "health";

/// L'état du projet, tel que la zone `rbs:inventory` le porte.
///
/// Les fragments et les entités partagent une seule liste dans le manifeste : c'est le
/// catalogue des fragments qui les sépare, et non une marque dans les métadonnées — un
/// projet dont le CLI apprendrait un nouveau fragment doit reclasser les anciens sans
/// qu'on ait à réécrire son manifeste.
pub(crate) fn inventory(root: &Path, lang: Lang) -> Result<String, metadata::Error> {
    let metadonnees = metadata::read(&root.join("Cargo.toml"))?;

    Ok(inventory_of(
        &metadonnees.features,
        &metadonnees.version,
        metadonnees.database,
        root,
        lang,
    ))
}

/// L'inventaire d'un état donné du projet, le manifeste déjà lu.
///
/// Séparée d'[`inventory`] pour `add` et `generate`, qui doivent décrire le projet tel que
/// leur plan le laissera, non tel que le manifeste du disque le décrit encore.
pub(crate) fn inventory_of(
    features: &[String],
    version: &str,
    database: crate::database::Database,
    root: &Path,
    lang: Lang,
) -> String {
    let ancres = present_anchors(root, |file| root.join(file).exists());

    inventory_with(features, version, database, lang, &ancres)
}

/// Le même, la liste des ancres déjà résolue.
///
/// Séparée pour `refresh`, dont les ancres se lisent dans le plan et non sur le disque.
fn inventory_with(
    features: &[String],
    version: &str,
    database: crate::database::Database,
    lang: Lang,
    ancres: &[String],
) -> String {
    let catalogue = crate::templates::feature_names(None);

    let (fragments, entites): (Vec<&String>, Vec<&String>) = features
        .iter()
        .filter(|feature| feature.as_str() != SQUELETTE)
        // `partition` sur un itérateur de `&String` passe un `&&String` : le
        // déréférencement est ce que `Vec::contains` attend.
        .partition(|feature| catalogue.contains(*feature));

    match lang {
        Lang::Fr => format!(
            "- rbs {} · base {}\n\
             - Fragments installés : {}\n\
             - Entités engendrées : {}\n\
             - Ancres du projet : {}",
            version,
            database.name(),
            enumerate(&fragments, "aucun"),
            enumerate(&entites, "aucune"),
            ancres.join(", "),
        ),
        Lang::En => format!(
            "- rbs {} · {} database\n\
             - Fragments installed: {}\n\
             - Generated entities: {}\n\
             - Project anchors: {}",
            version,
            database.name(),
            enumerate(&fragments, "none"),
            enumerate(&entites, "none"),
            ancres.join(", "),
        ),
    }
}

/// Réécrit la zone d'inventaire de l'`AGENTS.md`, les features ou entités qu'`ajoutees`
/// nomme comprises.
///
/// Une zone ou un fichier absents ne sont pas une erreur : le développeur a pu retirer
/// l'un ou l'autre, et une installation ne se refuse pas pour un fichier de
/// documentation. La zone est alors simplement laissée de côté — `rbs doctor` la
/// réclamera.
pub(crate) fn refresh(
    builder: &mut crate::plan::Builder,
    root: &Path,
    metadonnees: &metadata::Metadata,
    ajoutees: &[String],
) -> Result<Option<MissingZone>, crate::plan::Error> {
    let mut features = metadonnees.features.clone();
    for ajoutee in ajoutees {
        if !features.contains(ajoutee) {
            features.push(ajoutee.clone());
        }
    }

    // Les ancres se lisent dans le plan et non sur le disque : `rbs add docker` écrit le
    // `docker-compose.yml` d'un projet qui n'en avait pas, et l'inventaire doit nommer
    // l'ancre `services` que ce fichier apportera. Interrogé sur le disque, il l'omettait,
    // et `rbs doctor` — qui relit le disque après écriture — la réclamait aussitôt.
    let ancres = present_anchors(root, |file| builder.exists(file).unwrap_or(false));

    let content = inventory_with(
        &features,
        &metadonnees.version,
        metadonnees.database,
        metadonnees.lang,
        &ancres,
    );

    match builder.replace_zone(FICHIER, INVENTORY, &content, None) {
        Ok(()) => Ok(None),
        // Le fichier entier absent ne se signale pas ici : il n'y a pas de bloc à coller
        // dans un fichier qui n'existe pas, et `rbs upgrade` sait le recréer.
        Err(crate::plan::Error::FichierAbsent { .. }) => Ok(None),
        Err(crate::plan::Error::ZoneAbsente { zone, .. }) => Ok(Some(zone)),
        Err(autre) => Err(autre),
    }
}

/// Les ancres que le projet porte réellement, chacune avec son fichier.
///
/// L'ancre des features se résout par repli — `src/lib.rs` ou `src/main.rs` selon l'âge du
/// projet — et une ancre optionnelle dont le fichier est absent n'est pas listée : un
/// projet SQLite n'a pas de compose, et n'a pas à passer pour incomplet.
///
/// `porte` dit si le projet a le fichier voulu : le disque pour un inventaire constaté,
/// le plan pour celui qu'une commande s'apprête à écrire.
fn present_anchors(root: &Path, porte: impl Fn(&str) -> bool) -> Vec<String> {
    anchors::resolved(root)
        .into_iter()
        .filter(|anchor| !anchor.optional || porte(anchor.file.as_ref()))
        .map(|anchor| format!("{} ({})", anchor.name, anchor.file))
        .collect()
}

/// Une liste, ou le mot qui dit qu'elle est vide.
fn enumerate(noms: &[&String], vide: &str) -> String {
    if noms.is_empty() {
        return vide.to_string();
    }

    noms.iter()
        .map(|nom| nom.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::Lang;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Un projet neuf, créé sans passer par le binaire.
    fn project(features: Vec<String>) -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo_api".to_string(),
                database: Default::default(),
                features,
                core_path: None,
                template_dir: None,
                lang: Lang::Fr,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, project.root)
    }

    #[test]
    fn the_inventory_names_the_version_and_the_database() {
        let (_parent, root) = project(Vec::new());

        let rendu = inventory(&root, Lang::Fr).expect("le projet est lisible");

        assert!(rendu.contains(env!("CARGO_PKG_VERSION")), "{rendu}");
        assert!(rendu.contains("postgres"), "{rendu}");
    }

    /// `add` doit rendre l'inventaire du projet *après* installation, alors que le
    /// manifeste du disque décrit encore celui d'avant.
    #[test]
    fn the_inventory_can_be_computed_from_a_projected_feature_list() {
        let (_parent, root) = project(Vec::new());

        let rendu = inventory_of(
            &["health".to_string(), "auth".to_string()],
            "1.2.0",
            Default::default(),
            &root,
            Lang::Fr,
        );

        assert!(rendu.contains("auth"), "{rendu}");
        assert!(rendu.contains("1.2.0"), "{rendu}");
    }

    /// Un fragment et une entité se distinguent par le catalogue des fragments : le
    /// manifeste, lui, les met dans une seule liste.
    #[test]
    fn a_fragment_and_an_entity_are_told_apart() {
        let (_parent, root) = project(vec!["redis".to_string()]);
        let manifest = root.join("Cargo.toml");
        let source = std::fs::read_to_string(&manifest).expect("manifeste lisible");
        let patched = crate::metadata::record_feature(&source, "articles", "Cargo.toml")
            .expect("le manifeste accepte la feature")
            .expect("la feature n'y est pas encore");
        std::fs::write(&manifest, patched).expect("manifeste réécrit");

        let rendu = inventory(&root, Lang::Fr).expect("le projet est lisible");

        let fragments = rendu
            .lines()
            .find(|line| line.contains("Fragments"))
            .expect("la ligne des fragments est rendue");
        let entites = rendu
            .lines()
            .find(|line| line.contains("Entités"))
            .expect("la ligne des entités est rendue");

        assert!(
            fragments.contains("redis") && !fragments.contains("articles"),
            "{fragments}"
        );
        assert!(
            entites.contains("articles") && !entites.contains("redis"),
            "{entites}"
        );
    }

    /// `health` est le module du squelette, non une entité engendrée : le compter parmi
    /// elles ferait croire à un CRUD que personne n'a demandé.
    #[test]
    fn the_health_module_is_not_counted_as_an_entity() {
        let (_parent, root) = project(Vec::new());

        let rendu = inventory(&root, Lang::Fr).expect("le projet est lisible");

        assert!(!rendu.contains("health"), "{rendu}");
    }

    /// Une liste vide se dit, elle ne se tait pas : une ligne absente se lit comme une
    /// information manquante.
    #[test]
    fn an_empty_list_is_said_rather_than_omitted() {
        let (_parent, root) = project(Vec::new());

        let rendu = inventory(&root, Lang::Fr).expect("le projet est lisible");

        assert!(rendu.contains("aucun"), "{rendu}");
    }

    /// L'ancre des features vit dans `src/lib.rs` depuis que le projet engendré porte une
    /// bibliothèque : l'inventaire doit nommer le fichier réel, non celui du registre.
    #[test]
    fn the_features_anchor_is_named_where_the_project_actually_carries_it() {
        let (_parent, root) = project(Vec::new());

        let rendu = inventory(&root, Lang::Fr).expect("le projet est lisible");

        assert!(rendu.contains("features (src/lib.rs)"), "{rendu}");
    }

    /// Un projet SQLite n'a pas de compose : réclamer l'ancre `services` le ferait passer
    /// pour incomplet.
    #[test]
    fn an_optional_anchor_whose_file_is_absent_is_not_listed() {
        let (_parent, root) = project(Vec::new());
        std::fs::remove_file(root.join("docker-compose.yml")).ok();

        let rendu = inventory(&root, Lang::Fr).expect("le projet est lisible");

        assert!(!rendu.contains("services"), "{rendu}");
    }

    #[test]
    fn the_english_inventory_uses_english_labels() {
        let (_parent, root) = project(Vec::new());

        let rendu = inventory(&root, Lang::En).expect("le projet est lisible");

        assert!(rendu.contains("Fragments installed"), "{rendu}");
        assert!(!rendu.contains("Fragments installés"), "{rendu}");
    }

    const DOCUMENT: &str = "# blog\n\n\
        <!-- rbs:guide 1.1.0 -->\nancien guide\n<!-- /rbs:guide -->\n\n\
        <!-- rbs:inventory -->\nancien inventaire\n<!-- /rbs:inventory -->\n\n\
        ## Notes du projet\n\nà moi\n";

    #[test]
    fn the_opening_marker_carries_the_version_when_there_is_one() {
        assert_eq!(opening(GUIDE, Some("1.2.0")), "<!-- rbs:guide 1.2.0 -->");
        assert_eq!(opening(INVENTORY, None), "<!-- rbs:inventory -->");
        assert_eq!(closing(GUIDE), "<!-- /rbs:guide -->");
    }

    #[test]
    fn a_zone_is_replaced_by_its_new_content() {
        let rendu =
            replace(DOCUMENT, INVENTORY, "nouvel inventaire").expect("la zone est présente");

        assert!(
            rendu.contains("<!-- rbs:inventory -->\nnouvel inventaire\n<!-- /rbs:inventory -->")
        );
        assert!(!rendu.contains("ancien inventaire"));
    }

    /// La raison d'être des zones : le développeur écrit dans ce fichier, et rbs ne doit
    /// jamais lui reprendre ce qu'il y met.
    #[test]
    fn everything_outside_the_zone_survives_the_replacement() {
        let rendu =
            replace(DOCUMENT, INVENTORY, "nouvel inventaire").expect("la zone est présente");

        assert!(rendu.starts_with("# blog\n"));
        assert!(rendu.contains("## Notes du projet\n\nà moi\n"));
        assert!(rendu.contains("ancien guide"), "l'autre zone est intacte");
    }

    /// Deux écritures successives doivent laisser un fichier identique : c'est ce qui
    /// permet à `doctor` de comparer un rendu au fichier pour dire « périmé ».
    #[test]
    fn replacing_twice_gives_the_same_document() {
        let une = replace(DOCUMENT, INVENTORY, "nouvel inventaire").expect("zone présente");
        let deux = replace(&une, INVENTORY, "nouvel inventaire").expect("zone présente");

        assert_eq!(une, deux);
    }

    /// Le marqueur d'ouverture du guide porte sa version : la remplacer ne doit pas la
    /// laisser derrière.
    #[test]
    fn replacing_the_guide_rewrites_its_version() {
        let rendu = replace_versioned(DOCUMENT, GUIDE, "nouveau guide", "1.2.0")
            .expect("la zone est présente");

        assert!(rendu.contains("<!-- rbs:guide 1.2.0 -->"));
        assert!(!rendu.contains("1.1.0"));
    }

    #[test]
    fn a_missing_zone_is_reported_with_the_block_to_paste() {
        let absente = replace("# blog\n", INVENTORY, "x").expect_err("la zone manque");

        assert_eq!(absente.zone, INVENTORY);
        assert_eq!(
            absente.block(),
            "<!-- rbs:inventory -->\n<!-- /rbs:inventory -->"
        );
    }

    /// Une ouverture sans fermeture n'est pas une zone : la traiter comme telle
    /// emporterait tout le reste du fichier.
    #[test]
    fn an_unclosed_zone_is_a_missing_zone() {
        let tronque = "# blog\n<!-- rbs:inventory -->\nrien ne ferme\n";

        assert!(replace(tronque, INVENTORY, "x").is_err());
    }

    #[test]
    fn the_body_of_a_zone_is_read_back() {
        assert_eq!(body(DOCUMENT, INVENTORY), Some("ancien inventaire"));
        assert_eq!(body(DOCUMENT, "absente"), None);
    }

    #[test]
    fn the_version_of_the_guide_is_read_from_its_opening_marker() {
        assert_eq!(version(DOCUMENT, GUIDE).as_deref(), Some("1.1.0"));
        assert_eq!(version(DOCUMENT, INVENTORY), None);
    }

    /// Un guide périmé n'induit pas un développeur en erreur : il induit tous les agents
    /// en erreur. `add` fut livrée avec une description qui ne nommait pas `auth` — même
    /// piège, en plus coûteux.
    ///
    /// L'assertion porte sur la ligne du tableau des commandes, jamais sur le nom nu : sur
    /// des noms de trois lettres, un `contains` cru se satisfait de n'importe quoi —
    /// `add_index_on_slug` répondait pour `add`, `rbs migrate new` pour `new`, et une
    /// simple mention en prose pour le reste. Retirer la ligne `| rbs add <feature> | … |`
    /// du tableau ne faisait alors rougir personne, ce qui est pourtant le seul défaut que
    /// ce test existe pour attraper.
    #[test]
    fn the_guide_names_every_subcommand_of_the_cli() {
        use clap::CommandFactory;

        let racine = Path::new("/aucun-projet-ici");

        for lang in [Lang::Fr, Lang::En] {
            let rendu = guide(lang, racine).expect("le guide se rend");

            for sous_commande in crate::cli::Cli::command().get_subcommands() {
                // `help` est engendrée par clap lui-même : aucun guide n'a à la documenter.
                if sous_commande.get_name() == "help" {
                    continue;
                }

                let nom = sous_commande.get_name();
                // `| `rbs seed` |` se clôt par l'accent grave, `| `rbs generate crud …` |`
                // par l'espace qui précède son premier argument : ce qui suit le nom
                // départage une commande citée d'une autre qui la préfixe.
                let cellule = format!("| `rbs {nom}");

                assert!(
                    rendu.lines().any(|ligne| ligne
                        .strip_prefix(&cellule)
                        .is_some_and(|reste| reste.starts_with(['`', ' ']))),
                    "`rbs {nom}` absente du tableau des commandes du guide {lang} :\n{rendu}"
                );
            }
        }
    }

    /// La liste des ancres se calcule, elle ne se recopie pas : une ancre ajoutée au
    /// registre sans être écrite ici laisserait l'agent la piétiner.
    #[test]
    fn the_guide_names_every_anchor_of_the_registry() {
        let racine = Path::new("/aucun-projet-ici");

        for lang in [Lang::Fr, Lang::En] {
            let rendu = guide(lang, racine).expect("le guide se rend");

            for anchor in crate::anchors::ANCRES.iter() {
                assert!(
                    rendu.contains(anchor.name.as_ref()),
                    "l'ancre `{}` est absente du guide {lang}",
                    anchor.name
                );
            }
        }
    }

    /// Huit sections, dans cet ordre, de chaque côté : un compte seul laisserait passer
    /// une section renommée ou déplacée d'une seule langue.
    #[test]
    fn each_language_carries_its_sections_in_order() {
        let racine = Path::new("/aucun-projet-ici");
        let titres = |lang| {
            guide(lang, racine)
                .expect("le guide se rend")
                .lines()
                .filter(|line| line.starts_with("## "))
                .map(str::to_string)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            titres(Lang::Fr),
            [
                "## Le CLI d'abord",
                "## Ce fichier",
                "## Les commandes",
                "## Recettes",
                "## Architecture imposée",
                "## Les ancres",
                "## Ce que rbs ne couvre pas",
                "## Vérifier avant de conclure",
            ]
        );
        assert_eq!(
            titres(Lang::En),
            [
                "## CLI first",
                "## This file",
                "## Commands",
                "## Recipes",
                "## Enforced architecture",
                "## Anchors",
                "## What rbs does not cover",
                "## Check before you conclude",
            ]
        );
    }

    #[test]
    fn the_document_carries_both_zones_and_a_place_for_the_developer() {
        let (_parent, root) = project(Vec::new());

        let rendu = document(&root, Lang::Fr, "demo-api", VERSION).expect("le document se rend");

        assert!(rendu.contains(&opening(GUIDE, Some(VERSION))), "{rendu}");
        assert!(rendu.contains(&closing(GUIDE)), "{rendu}");
        assert!(rendu.contains(&opening(INVENTORY, None)), "{rendu}");
        assert!(rendu.contains(&closing(INVENTORY)), "{rendu}");
        assert!(rendu.contains("## Notes du projet"), "{rendu}");
        assert!(rendu.starts_with("# demo-api"), "{rendu}");
    }

    /// Le guide décrit l'ancre `features` par son propre calcul, l'inventaire par
    /// `present_anchors` : deux chemins vers la même information, qui doivent s'accorder.
    /// Un projet engendré depuis le jalon de la bibliothèque porte `src/lib.rs`, et c'est
    /// ce nom que les deux moitiés du document doivent citer — jamais `src/main.rs`, où
    /// l'ancre a vécu avant ce jalon.
    #[test]
    fn the_guide_and_the_inventory_agree_on_the_features_anchor_file() {
        let (_parent, root) = project(Vec::new());

        let rendu = document(&root, Lang::Fr, "demo-api", VERSION).expect("le document se rend");

        let dans_le_guide = fichier_apres(&rendu, "<rbs:features>` dans `", '`');
        let dans_linventaire = fichier_apres(&rendu, "features (", ')');

        assert_eq!(
            dans_le_guide, dans_linventaire,
            "le guide et l'inventaire ne s'accordent pas sur l'ancre features :\n{rendu}"
        );
        assert_eq!(dans_le_guide, "src/lib.rs", "{rendu}");
    }

    /// Le chemin cité juste après `motif`, jusqu'au prochain `fin`.
    fn fichier_apres<'a>(texte: &'a str, motif: &str, fin: char) -> &'a str {
        let debut = texte
            .find(motif)
            .unwrap_or_else(|| panic!("`{motif}` absent du document :\n{texte}"))
            + motif.len();
        let longueur = texte[debut..]
            .find(fin)
            .unwrap_or_else(|| panic!("`{fin}` ne referme pas `{motif}` :\n{texte}"));

        &texte[debut..debut + longueur]
    }

    /// Le fichier est du markdown lu par des humains autant que par des agents : un
    /// document qui ne finit pas par une ligne vide fâche Git et les éditeurs.
    #[test]
    fn the_document_ends_with_a_single_newline() {
        let (_parent, root) = project(Vec::new());

        let rendu = document(&root, Lang::Fr, "demo-api", VERSION).expect("le document se rend");

        assert!(
            rendu.ends_with('\n') && !rendu.ends_with("\n\n"),
            "{rendu:?}"
        );
    }

    /// Un guide à moitié traduit fait douter de tout le reste du document : la section
    /// des ancres est calculée, et c'est par là que le français s'y était glissé.
    #[test]
    fn the_english_guide_carries_no_french_in_its_anchor_list() {
        let rendu = guide(Lang::En, Path::new("/aucun-projet-ici")).expect("le guide se rend");

        for francais in [" dans `", " et `", "de chaque entité"] {
            assert!(
                !rendu.contains(francais),
                "« {francais} » subsiste dans le guide anglais :\n{rendu}"
            );
        }
    }
}
