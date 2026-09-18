//! Contrôle de la feature `api-keys`.
//!
//! Le noyau ne juge une clé que si le projet le lui dit : sans la délégation posée dans
//! `impl HasAuth for AppState`, le défaut du trait refuse, et toute clé rend 401 sur un
//! projet qui compile et démarre. Un projet engendré avant que l'ancre n'existe reçoit le
//! bloc à coller au lieu de l'insertion — et c'est précisément le cas que ce contrôle voit.

use std::path::Path;

use super::Check;

pub(crate) const TITRE: &str = "api-keys";
const FICHIER: &str = "src/auth/mod.rs";
const DELEGATION: &str = "crate::modules::api_keys::service::accept(self, key, extensions).await";

pub(crate) fn check(root: &Path) -> Check {
    let source = match super::lire(root, TITRE, FICHIER) {
        Ok(source) => source,
        Err(constat) => return constat,
    };

    // Ligne entière, indentation ôtée : une délégation en commentaire ne délègue rien.
    if source.lines().any(|ligne| ligne.trim() == DELEGATION) {
        return Check::ok(TITRE, "le projet juge les clés d'API qu'on lui présente");
    }

    Check::failed(
        TITRE,
        "la délégation des clés d'API n'est pas posée : toute clé rendra 401",
        format!("dans {FICHIER}, entre les balises de `// <rbs:auth_impl>` :\n{DELEGATION}"),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// L'`impl HasAuth` tel que le fragment `auth` le livre, l'ancre portant `ligne`.
    fn implementation(ligne: &str) -> String {
        format!(
            "impl HasAuth for AppState {{\n    async fn accept(&self, claims: &Claims) \
             -> rbs_core::Result<()> {{\n        admit(self, claims).await.map(drop)\n    }}\n\
             \n    // <rbs:auth_impl>\n{ligne}    // </rbs:auth_impl>\n}}\n"
        )
    }

    fn projet(source: Option<&str>) -> TempDir {
        let racine = TempDir::new().expect("répertoire temporaire créable");

        if let Some(source) = source {
            let fichier = racine.path().join(FICHIER);
            fs::create_dir_all(fichier.parent().expect("le fichier a un parent"))
                .expect("répertoire du module auth créable");
            fs::write(&fichier, source).expect("module inscriptible");
        }

        racine
    }

    #[test]
    fn a_posted_delegation_reports_nothing() {
        let racine = projet(Some(&implementation(&format!("    {DELEGATION}\n"))));

        let check = check(racine.path());

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// Le défaut que rien d'autre ne signale : le projet compile, démarre, et rend 401 à
    /// toute clé.
    #[test]
    fn a_missing_delegation_names_its_consequence_and_the_line_to_paste() {
        let racine = projet(Some(&implementation("")));

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("401"), "{}", check.detail);
        let remede = check.remedy.expect("un échec porte son remède");
        assert!(remede.contains(DELEGATION), "{remede}");
        assert!(remede.contains("<rbs:auth_impl>"), "{remede}");
        assert!(remede.contains(FICHIER), "{remede}");
    }

    #[test]
    fn a_delegation_in_a_comment_does_not_count() {
        let racine = projet(Some(&implementation(&format!("    // {DELEGATION}\n"))));

        assert_eq!(check(racine.path()).state, State::Echec);
    }

    #[test]
    fn a_missing_auth_module_is_named() {
        let racine = projet(None);

        let check = check(racine.path());

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains(FICHIER), "{}", check.detail);
    }

    /// La ligne cherchée est celle que le fragment insère : l'ancre `auth_impl` de son
    /// manifeste. Sans ce test, les deux dériveraient en silence et le contrôle
    /// signalerait une délégation pourtant posée.
    #[test]
    fn the_line_is_the_one_the_fragment_inserts() {
        let manifeste: toml_edit::DocumentMut = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/api-keys/feature.toml"
        ))
        .expect("le manifeste du fragment se lit")
        .parse()
        .expect("le manifeste du fragment s'analyse");

        let inseree = manifeste
            .get("anchors")
            .and_then(toml_edit::Item::as_array_of_tables)
            .and_then(|ancres| {
                ancres.iter().find(|ancre| {
                    ancre.get("anchor").and_then(toml_edit::Item::as_str) == Some("auth_impl")
                })
            })
            .and_then(|ancre| ancre.get("content"))
            .and_then(toml_edit::Item::as_str)
            .expect("le fragment insère dans l'ancre `auth_impl`");

        assert!(
            inseree.lines().any(|ligne| ligne.trim() == DELEGATION),
            "le manifeste n'insère pas la ligne cherchée :\n{inseree}"
        );
    }
}
