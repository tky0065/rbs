//! Les deux modifications qu'un plan sait faire à un fichier qui n'est pas un manifeste
//! Cargo : ajouter une section à un document TOML, ajouter une variable à un `.env`.
//!
//! Aucune des deux ne re-sérialise le fichier. Ce que le développeur y a écrit — son
//! ordre, ses commentaires, sa mise en forme — traverse l'ajout tel quel, parce que le
//! texte d'origine n'est jamais relu autrement que pour décider s'il faut ajouter.

use toml_edit::{DocumentMut, TomlError};

/// Rend le document avec `section` ajoutée en fin de fichier, ou `None` s'il la porte
/// déjà.
///
/// Une section déjà présente n'est pas complétée : le développeur a pu en retirer une clé
/// dont il ne veut pas, et la lui réinscrire à chaque commande serait une correction
/// contre son gré.
pub(crate) fn add_section(
    text: &str,
    section: &str,
    content: &str,
) -> Result<Option<String>, TomlError> {
    let document: DocumentMut = text.parse()?;

    if document.get(section).is_some() {
        return Ok(None);
    }

    let separator = if text.is_empty() || text.ends_with("\n\n") {
        ""
    } else if text.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let line_end = if content.ends_with('\n') { "" } else { "\n" };

    Ok(Some(format!(
        "{text}{separator}[{section}]\n{content}{line_end}"
    )))
}

/// Rend le fichier d'environnement avec `key` ajoutée, ou `None` s'il la déclare déjà.
///
/// La présence se juge sur la clé et non sur la ligne entière : un développeur qui a
/// changé la valeur n'a pas à en voir apparaître une seconde déclaration.
pub(crate) fn add_variable(
    text: &str,
    key: &str,
    value: &str,
    comment: Option<&str>,
) -> Option<String> {
    if crate::dotenv::value(&crate::dotenv::parse(text), key).is_some() {
        return None;
    }

    let separator = if text.is_empty() || text.ends_with("\n\n") {
        ""
    } else if text.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let comment = comment.map_or(String::new(), |text| format!("# {text}\n"));

    Some(format!("{text}{separator}{comment}{key}={value}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "[server]\nport = 8080\n";

    #[test]
    fn a_missing_section_is_appended_at_the_end_of_the_document() {
        let rendered = add_section(CONFIG, "auth", "access_ttl_secs = 900\n")
            .expect("le document est valide")
            .expect("la section manque");

        assert_eq!(
            rendered,
            "[server]\nport = 8080\n\n[auth]\naccess_ttl_secs = 900\n"
        );
    }

    #[test]
    fn an_already_present_section_is_not_rewritten() {
        let rendered = add_section(CONFIG, "server", "port = 9090\n").expect("document valide");

        assert!(rendered.is_none(), "{rendered:?}");
    }

    #[test]
    fn the_existing_document_survives_the_addition_unchanged() {
        let annote = "# la configuration par défaut\n[server]\nport = 8080 # modifiable\n";

        let rendered = add_section(annote, "auth", "secret = \"x\"\n")
            .expect("document valide")
            .expect("la section manque");

        assert!(rendered.starts_with(annote), "{rendered}");
    }

    #[test]
    fn an_invalid_document_is_reported() {
        add_section("[server\n", "auth", "").expect_err("le document ne s'analyse pas");
    }

    #[test]
    fn a_missing_variable_is_added_with_its_comment() {
        let rendered = add_variable(
            "RBS_DATABASE__URL=postgres://\n",
            "RBS_AUTH__SECRET",
            "changez-moi",
            Some("au moins 32 octets"),
        )
        .expect("la clé manque");

        assert_eq!(
            rendered,
            "RBS_DATABASE__URL=postgres://\n\n# au moins 32 octets\nRBS_AUTH__SECRET=changez-moi\n"
        );
    }

    /// Une valeur changée par le développeur n'appelle pas une seconde déclaration.
    #[test]
    fn an_already_declared_key_is_not_redeclared_even_with_another_value() {
        let existant = "RBS_AUTH__SECRET=le-mien\n";

        let rendered = add_variable(existant, "RBS_AUTH__SECRET", "changez-moi", None);

        assert!(rendered.is_none(), "{rendered:?}");
    }

    #[test]
    fn a_variable_without_a_comment_is_added_on_its_own() {
        let rendered =
            add_variable("", "RBS_AUTH__SECRET", "changez-moi", None).expect("la clé manque");

        assert_eq!(rendered, "RBS_AUTH__SECRET=changez-moi\n");
    }
}
