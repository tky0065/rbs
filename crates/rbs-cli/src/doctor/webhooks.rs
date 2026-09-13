//! Contrôle de la feature `webhooks`.
//!
//! Une livraison est un job de la file, et le worker n'exécute que ce que `registry()` lui
//! déclare. Sans l'inscription que `add webhooks` pose dans `<rbs:jobs>`, le projet compile,
//! l'événement s'enfile, et chaque livraison part en réessai puis en échec sous « aucun job
//! n'est inscrit » — ce qu'aucun autre contrôle ne dit avant qu'un abonné ne s'en plaigne.

use std::path::Path;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "webhooks";
const FICHIER: &str = "src/modules/jobs/mod.rs";
const INSCRIPTION: &str =
    "registre = registre.register::<crate::modules::webhooks::delivery::Delivery>();";

/// Vérifie que le registre de la file porte la livraison des webhooks.
pub(crate) fn check(root: &Path) -> Check {
    let source = match super::lire(root, TITRE, FICHIER) {
        Ok(source) => source,
        Err(constat) => return constat,
    };

    // Ligne entière, indentation ôtée : une inscription en commentaire n'inscrit rien.
    if source.lines().any(|ligne| ligne.trim() == INSCRIPTION) {
        return Check::ok(TITRE, "la livraison est inscrite au registre de la file");
    }

    Check::failed(
        TITRE,
        "la livraison des webhooks n'est pas inscrite au registre de la file : chaque \
         livraison partira en échec",
        format!("dans {FICHIER}, entre les balises de `// <rbs:jobs>` :\n{INSCRIPTION}"),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Un projet réduit au registre de la file, quand il en porte un.
    fn projet(registre: Option<&str>) -> TempDir {
        let racine = TempDir::new().expect("répertoire temporaire créable");

        if let Some(source) = registre {
            let fichier = racine.path().join(FICHIER);
            fs::create_dir_all(fichier.parent().expect("le fichier a un parent"))
                .expect("répertoire de la file créable");
            fs::write(&fichier, source).expect("registre inscriptible");
        }

        racine
    }

    /// `registry()` tel que la file le livre, l'ancre portant `ligne` si elle en porte une.
    fn registre(ligne: &str) -> String {
        format!(
            "pub fn registry() -> Registry {{\n    let mut registre = Registry::new();\n    \
             registre = registre.register::<demo::Log>();\n    // <rbs:jobs>\n{ligne}    \
             // </rbs:jobs>\n    registre\n}}\n"
        )
    }

    #[test]
    fn a_registered_delivery_reports_nothing() {
        let racine = projet(Some(&registre(&format!("    {INSCRIPTION}\n"))));

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// Le défaut que rien ne signale : le projet compile, et chaque livraison part en
    /// réessai puis en échec sous « aucun job n'est inscrit ».
    #[test]
    fn a_missing_registration_names_its_consequence_and_the_line_to_paste() {
        let racine = projet(Some(&registre("")));

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("échec"), "{}", check.detail);
        let remede = check.remedy.expect("un échec porte son remède");
        assert!(remede.contains(INSCRIPTION), "{remede}");
        assert!(remede.contains("<rbs:jobs>"), "{remede}");
    }

    #[test]
    fn a_registration_in_a_comment_does_not_count() {
        let racine = projet(Some(&registre(&format!("    // {INSCRIPTION}\n"))));

        assert_eq!(check(racine.path()).state, State::Echec);
    }

    #[test]
    fn a_missing_queue_registry_is_named() {
        let racine = projet(None);

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains(FICHIER), "{}", check.detail);
    }

    /// La ligne cherchée est celle que le fragment insère : l'ancre `jobs` de son manifeste.
    #[test]
    fn the_line_is_the_one_the_fragment_inserts() {
        let manifeste: toml_edit::DocumentMut = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/webhooks/feature.toml"
        ))
        .expect("le manifeste du fragment se lit")
        .parse()
        .expect("le manifeste du fragment s'analyse");

        let inseree = manifeste
            .get("anchors")
            .and_then(toml_edit::Item::as_array_of_tables)
            .and_then(|ancres| {
                ancres.iter().find(|ancre| {
                    ancre.get("anchor").and_then(toml_edit::Item::as_str) == Some("jobs")
                })
            })
            .and_then(|ancre| ancre.get("content"))
            .and_then(toml_edit::Item::as_str)
            .expect("le fragment insère dans l'ancre `jobs`");

        assert_eq!(inseree.trim(), INSCRIPTION);
    }
}
