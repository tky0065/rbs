//! Les features CRUD dont les écritures restent ouvertes à qui passe.
//!
//! Le contrôle ne s'exécute que sur un projet portant `auth` : sans elle, une API sans
//! authentification est le régime normal, et le signaler serait un reproche adressé à un
//! choix qu'on n'a pas fait. Avec elle, une route d'écriture anonyme reste légitime — un
//! catalogue public en expose — mais mérite d'être vue : c'est un avertissement, jamais un
//! échec.
//!
//! La garde se reconnaît à l'appel de `require_role`. Un projet qui protégerait ses
//! écritures autrement — un middleware posé sur `<rbs:layers>`, un extracteur maison — est
//! signalé à tort ; c'est l'autre raison pour laquelle le verdict n'est qu'orange.

use std::fs;
use std::path::Path;

use super::Check;

const TITRE: &str = "gardes";

/// Signatures des trois handlers qui écrivent, telles que la template les rend.
const ECRITURES: [&str; 3] = [
    "pub async fn create(",
    "pub async fn update(",
    "pub async fn delete(",
];

/// L'appel qui prouve qu'une route est réservée à un rôle.
const GARDE: &str = "require_role";

/// Signale les features dont `create`, `update` ou `delete` n'exigent aucun rôle.
pub(crate) fn check(root: &Path) -> Check {
    let mut anonymes: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir(root.join("src")) {
        for entry in entries.flatten() {
            let Ok(source) = fs::read_to_string(entry.path().join("controller.rs")) else {
                continue;
            };

            if !ECRITURES.iter().any(|verbe| source.contains(verbe)) || source.contains(GARDE) {
                continue;
            }

            anonymes.push(entry.file_name().to_string_lossy().into_owned());
        }
    }

    if anonymes.is_empty() {
        return Check::ok(TITRE, "aucune écriture anonyme parmi les features");
    }

    // L'ordre du disque n'en est pas un : deux diagnostics du même projet doivent se lire
    // pareil.
    anonymes.sort();

    Check::warned(
        TITRE,
        format!("écritures anonymes : {}", anonymes.join(", ")),
        "réservez-les à un rôle : `rbs generate crud <nom> --fields … --role admin` pose le \
         garde à la génération, et `identite.require_role(Role::Admin)?` le pose à la main — \
         voir le guide de l'authentification",
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::State;
    use super::*;

    /// Un contrôleur CRUD tel que la template le rend, garde compris ou non.
    fn controller(guarded: bool) -> String {
        let garde = if guarded {
            "    identite.require_role(Role::Admin)?;\n"
        } else {
            ""
        };

        format!(
            "pub async fn list(State(state): State<AppState>) {{}}\n\n\
             pub async fn create(State(state): State<AppState>) {{\n{garde}}}\n\n\
             pub async fn update(State(state): State<AppState>) {{\n{garde}}}\n\n\
             pub async fn delete(State(state): State<AppState>) {{\n{garde}}}\n"
        )
    }

    fn write_feature(root: &Path, name: &str, source: &str) {
        let directory = root.join("src").join(name);
        fs::create_dir_all(&directory).expect("répertoire de feature créable");
        fs::write(directory.join("controller.rs"), source).expect("contrôleur inscriptible");
    }

    #[test]
    fn a_feature_writing_without_a_guard_is_only_a_warning() {
        let (_parent, root) = super::super::tests::project(&["health", "auth"]);
        write_feature(&root, "articles", &controller(false));

        let check = check(&root);

        assert_eq!(check.state, State::Avertissement, "{}", check.detail);
        assert!(
            check.detail.contains("articles"),
            "le constat doit nommer la feature : {}",
            check.detail
        );
        assert!(
            check.remedy.unwrap_or_default().contains("--role"),
            "le remède doit nommer l'option qui pose le garde"
        );
    }

    #[test]
    fn a_guarded_feature_reports_nothing() {
        let (_parent, root) = super::super::tests::project(&["health", "auth"]);
        write_feature(&root, "articles", &controller(true));

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// Le contrôleur de `health` n'expose aucune écriture : un projet neuf est net.
    #[test]
    fn a_brand_new_project_reports_nothing() {
        let (_parent, root) = super::super::tests::project(&["health", "auth"]);

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    #[test]
    fn the_features_are_named_in_a_stable_order() {
        let (_parent, root) = super::super::tests::project(&["health", "auth"]);
        for name in ["comments", "articles", "billets"] {
            write_feature(&root, name, &controller(false));
        }

        let check = check(&root);

        assert!(
            check.detail.ends_with("articles, billets, comments"),
            "les features doivent être triées : {}",
            check.detail
        );
    }
}
