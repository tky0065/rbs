//! L'`AGENTS.md` du projet engendré : ses zones, son inventaire, son guide.
//!
//! Deux zones seulement appartiennent à rbs, délimitées par des commentaires HTML. Le
//! reste du fichier est au développeur, et n'est jamais relu. Chaque zone est régénérée
//! en entier plutôt que complétée ligne à ligne : l'idempotence est alors acquise par
//! construction, là où l'insertion incrémentale demande un dédoublonnage qui a déjà coûté
//! plusieurs correctifs aux ancres du code.

// Aucun consommateur n'existe encore : le calcul de l'inventaire et le rendu du guide
// arrivent aux tâches suivantes, qui appelleront ces fonctions.
#![allow(dead_code)]

use std::path::Path;

use crate::anchors;
use crate::lang::Lang;
use crate::metadata;

/// Zone du mode d'emploi, propriété de rbs, versionnée.
pub(crate) const GUIDE: &str = "guide";

/// Zone de l'état du projet, recalculée à chaque écriture.
pub(crate) const INVENTORY: &str = "inventory";

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
    let catalogue = crate::templates::feature_names(None);

    let (fragments, entites): (Vec<&String>, Vec<&String>) = metadonnees
        .features
        .iter()
        .filter(|feature| feature.as_str() != SQUELETTE)
        // `partition` sur un itérateur de `&String` passe un `&&String` : le
        // déréférencement est ce que `Vec::contains` attend.
        .partition(|feature| catalogue.contains(*feature));

    let ancres = present_anchors(root);

    Ok(match lang {
        Lang::Fr => format!(
            "- rbs {} · base {}\n\
             - Fragments installés : {}\n\
             - Entités engendrées : {}\n\
             - Ancres du projet : {}",
            metadonnees.version,
            metadonnees.database.name(),
            enumerate(&fragments, "aucun"),
            enumerate(&entites, "aucune"),
            ancres.join(", "),
        ),
        Lang::En => format!(
            "- rbs {} · {} database\n\
             - Fragments installed: {}\n\
             - Generated entities: {}\n\
             - Project anchors: {}",
            metadonnees.version,
            metadonnees.database.name(),
            enumerate(&fragments, "none"),
            enumerate(&entites, "none"),
            ancres.join(", "),
        ),
    })
}

/// Les ancres que le projet porte réellement, chacune avec son fichier.
///
/// L'ancre des features se résout par repli — `src/lib.rs` ou `src/main.rs` selon l'âge du
/// projet — et une ancre optionnelle dont le fichier est absent n'est pas listée : un
/// projet SQLite n'a pas de compose, et n'a pas à passer pour incomplet.
fn present_anchors(root: &Path) -> Vec<String> {
    anchors::ANCRES
        .iter()
        .map(|anchor| {
            if anchor.name == anchors::FEATURES.name {
                anchors::resolve_features(root)
            } else {
                anchor.clone()
            }
        })
        .filter(|anchor| !anchor.optional || root.join(anchor.file.as_ref()).exists())
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
}
