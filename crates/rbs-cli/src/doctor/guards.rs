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
//!
//! Cet appel se cherche dans le corps de chaque handler d'écriture, et non dans le texte
//! du fichier : le mode d'emploi que la template pose en tête de tout contrôleur engendré
//! sous `auth` nomme `require_role` deux fois sans garder quoi que ce soit, et rendrait le
//! contrôle aveugle sur tout projet issu de la 1.3.0. Le corps se délimite sans AST — de
//! la signature à la première accolade fermante en colonne zéro — sur du code que rustfmt
//! a mis en forme.

use std::fs;
use std::path::Path;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "gardes";

/// Signatures des trois handlers qui écrivent, telles que la template les rend.
const ECRITURES: [&str; 3] = [
    "pub async fn create(",
    "pub async fn update(",
    "pub async fn delete(",
];

/// L'appel qui prouve qu'une route est réservée à un rôle.
const GARDE: &str = "require_role";

/// La signature et le corps du handler que `signature` ouvre, ou `None` s'il est absent.
///
/// L'accolade fermante d'une fonction de premier niveau est seule sur sa ligne et en
/// colonne zéro : c'est la borne la moins coûteuse qui reste juste sur du code formaté,
/// et un contrôleur que rustfmt n'aurait jamais vu retomberait au pire sur la fin du
/// fichier, soit le comportement d'avant.
fn corps<'a>(source: &'a str, signature: &str) -> Option<&'a str> {
    let debut = source.find(signature)?;
    let handler = &source[debut..];
    let fin = handler.find("\n}").map_or(handler.len(), |index| index + 2);

    Some(&handler[..fin])
}

/// Vrai si `handler` appelle `require_role` ailleurs que dans un commentaire.
fn garde(handler: &str) -> bool {
    handler
        .lines()
        .filter(|ligne| !ligne.trim_start().starts_with("//"))
        .any(|ligne| ligne.contains(GARDE))
}

/// Signale les features dont `create`, `update` ou `delete` n'exigent aucun rôle.
pub(crate) fn check(root: &Path) -> Check {
    let mut anonymes: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir(root.join("src")) {
        for entry in entries.flatten() {
            let Ok(source) = fs::read_to_string(entry.path().join("controller.rs")) else {
                continue;
            };

            let ouverte = ECRITURES
                .iter()
                .filter_map(|signature| corps(&source, signature))
                .any(|corps| !garde(corps));

            if !ouverte {
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
        "fermez-les à la main : sur chaque handler, ajoutez le paramètre `identite: Identity`, \
         l'appel `identite.require_role(Role::User)?`, l'entrée `security((\"bearer\" = []))` et \
         les réponses 401 et 403 de son annotation — un CRUD engendré sous `auth` les reçoit \
         désormais tout seul ; voir le guide de l'authentification",
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::State;
    use super::*;
    use crate::generate::feature::Feature;
    use crate::generate::{controller as rendu, fields};

    /// Le contrôleur que `generate crud` rend aujourd'hui sous `auth`, bandeau compris.
    ///
    /// Rendu par la template plutôt que recopié : une fixture écrite à la main avait
    /// cessé de ressembler au fichier livré, et c'est ce décalage qui avait laissé passer
    /// un contrôle aveugle.
    fn controller() -> String {
        let champs = fields::parse("titre:string").expect("champs valides");
        rendu::render(&Feature::fresh("articles", champs).authenticated())
            .expect("le contrôleur doit se rendre")
    }

    /// Le même, dont les gardes ont été retirées à la main.
    ///
    /// Le bandeau de tête reste : il explique comment rouvrir une route sans jamais dire
    /// de s'effacer lui-même, et il nomme `require_role` deux fois.
    fn reopened() -> String {
        let source: String = controller()
            .lines()
            .filter(|ligne| !ligne.contains("identite.require_role("))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            source.contains("//! `require_role` 403 en deçà du rôle nommé"),
            "la fixture doit garder le bandeau : c'est lui qui piège le contrôle"
        );

        source
    }

    fn write_feature(root: &Path, name: &str, source: &str) {
        let directory = root.join("src").join(name);
        fs::create_dir_all(&directory).expect("répertoire de feature créable");
        fs::write(directory.join("controller.rs"), source).expect("contrôleur inscriptible");
    }

    /// Le sens qui manquait : le bandeau ne doit pas tenir lieu de garde.
    #[test]
    fn a_feature_writing_without_a_guard_is_only_a_warning() {
        let (_parent, root) = super::super::tests::project(&["health", "auth"]);
        write_feature(&root, "articles", &reopened());

        let check = check(&root);

        assert_eq!(check.state, State::Avertissement, "{}", check.detail);
        assert!(
            check.detail.contains("articles"),
            "le constat doit nommer la feature : {}",
            check.detail
        );
        assert!(
            check.remedy.unwrap_or_default().contains("require_role"),
            "le remède doit nommer l'appel qui ferme la route"
        );
    }

    #[test]
    fn a_guarded_feature_reports_nothing() {
        let (_parent, root) = super::super::tests::project(&["health", "auth"]);
        write_feature(&root, "articles", &controller());

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
            write_feature(&root, name, &reopened());
        }

        let check = check(&root);

        assert!(
            check.detail.ends_with("articles, billets, comments"),
            "les features doivent être triées : {}",
            check.detail
        );
    }
}
