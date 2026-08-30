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

#[cfg(test)]
mod tests {
    use super::*;

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
