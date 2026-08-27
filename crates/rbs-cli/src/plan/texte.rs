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
pub(crate) fn ajouter_section(
    texte: &str,
    section: &str,
    contenu: &str,
) -> Result<Option<String>, TomlError> {
    let document: DocumentMut = texte.parse()?;

    if document.get(section).is_some() {
        return Ok(None);
    }

    let separateur = if texte.is_empty() || texte.ends_with("\n\n") {
        ""
    } else if texte.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let final_de_ligne = if contenu.ends_with('\n') { "" } else { "\n" };

    Ok(Some(format!(
        "{texte}{separateur}[{section}]\n{contenu}{final_de_ligne}"
    )))
}

/// Rend le fichier d'environnement avec `cle` ajoutée, ou `None` s'il la déclare déjà.
///
/// La présence se juge sur la clé et non sur la ligne entière : un développeur qui a
/// changé la valeur n'a pas à en voir apparaître une seconde déclaration.
pub(crate) fn ajouter_variable(
    texte: &str,
    cle: &str,
    valeur: &str,
    commentaire: Option<&str>,
) -> Option<String> {
    if crate::dotenv::valeur(&crate::dotenv::analyser(texte), cle).is_some() {
        return None;
    }

    let separateur = if texte.is_empty() || texte.ends_with("\n\n") {
        ""
    } else if texte.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let commentaire = commentaire.map_or(String::new(), |texte| format!("# {texte}\n"));

    Some(format!("{texte}{separateur}{commentaire}{cle}={valeur}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "[server]\nport = 8080\n";

    #[test]
    fn une_section_absente_est_ajoutee_en_fin_de_document() {
        let rendu = ajouter_section(CONFIG, "auth", "access_ttl_secs = 900\n")
            .expect("le document est valide")
            .expect("la section manque");

        assert_eq!(
            rendu,
            "[server]\nport = 8080\n\n[auth]\naccess_ttl_secs = 900\n"
        );
    }

    #[test]
    fn une_section_deja_presente_ne_se_reecrit_pas() {
        let rendu = ajouter_section(CONFIG, "server", "port = 9090\n").expect("document valide");

        assert!(rendu.is_none(), "{rendu:?}");
    }

    #[test]
    fn le_document_existant_traverse_l_ajout_sans_bouger() {
        let annote = "# la configuration par défaut\n[server]\nport = 8080 # modifiable\n";

        let rendu = ajouter_section(annote, "auth", "secret = \"x\"\n")
            .expect("document valide")
            .expect("la section manque");

        assert!(rendu.starts_with(annote), "{rendu}");
    }

    #[test]
    fn un_document_invalide_est_signale() {
        ajouter_section("[server\n", "auth", "").expect_err("le document ne s'analyse pas");
    }

    #[test]
    fn une_variable_absente_est_ajoutee_avec_son_commentaire() {
        let rendu = ajouter_variable(
            "RBS_DATABASE__URL=postgres://\n",
            "RBS_AUTH__SECRET",
            "changez-moi",
            Some("au moins 32 octets"),
        )
        .expect("la clé manque");

        assert_eq!(
            rendu,
            "RBS_DATABASE__URL=postgres://\n\n# au moins 32 octets\nRBS_AUTH__SECRET=changez-moi\n"
        );
    }

    /// Une valeur changée par le développeur n'appelle pas une seconde déclaration.
    #[test]
    fn une_cle_deja_declaree_ne_se_redeclare_pas_meme_avec_une_autre_valeur() {
        let existant = "RBS_AUTH__SECRET=le-mien\n";

        let rendu = ajouter_variable(existant, "RBS_AUTH__SECRET", "changez-moi", None);

        assert!(rendu.is_none(), "{rendu:?}");
    }

    #[test]
    fn une_variable_sans_commentaire_s_ajoute_seule() {
        let rendu =
            ajouter_variable("", "RBS_AUTH__SECRET", "changez-moi", None).expect("la clé manque");

        assert_eq!(rendu, "RBS_AUTH__SECRET=changez-moi\n");
    }
}
