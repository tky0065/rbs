//! Ce qu'une feature ajoute aux ancres du projet.
//!
//! Chaque ligne insérée désigne la feature par un chemin absolu — `crate::users::routes()`
//! — pour qu'une insertion se suffise à elle-même : aucune seconde écriture dans un bloc
//! `use` ne l'accompagne.

use crate::ancres::{self, Ancre};

/// Les handlers que le controller généré expose, dans l'ordre où ils y sont écrits.
const HANDLERS: [&str; 5] = ["list", "create", "find", "update", "delete"];

/// Une insertion à faire : l'ancre visée, et les lignes à y ajouter.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Montage {
    pub ancre: Ancre,
    pub lignes: Vec<String>,
}

/// Ce que la feature `module` ajoute au binaire du projet.
pub(crate) fn pour(module: &str) -> Vec<Montage> {
    vec![
        Montage {
            ancre: ancres::FEATURES,
            lignes: vec![format!("mod {module};")],
        },
        Montage {
            ancre: ancres::ROUTES,
            lignes: vec![format!(".merge(crate::{module}::routes())")],
        },
        Montage {
            ancre: ancres::OPENAPI,
            lignes: HANDLERS
                .iter()
                .map(|handler| format!("crate::{module}::controller::{handler},"))
                .collect(),
        },
    ]
}

/// Ce que la migration `module` ajoute à la crate `migration`.
///
/// Elle se déclare et s'inscrit séparément : une feature écrite à la main n'a pas de
/// migration générée, et le `Migrator` ne doit alors rien apprendre.
pub(crate) fn pour_migration(module: &str) -> Vec<Montage> {
    vec![
        Montage {
            ancre: ancres::MIGRATION_MODULES,
            lignes: vec![format!("mod {module};")],
        },
        Montage {
            ancre: ancres::MIGRATIONS,
            lignes: vec![format!("Box::new({module}::Migration),")],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lignes(montages: &[Montage], ancre: Ancre) -> &[String] {
        &montages
            .iter()
            .find(|montage| montage.ancre == ancre)
            .unwrap_or_else(|| panic!("aucun montage pour `{}`", ancre.nom))
            .lignes
    }

    #[test]
    fn le_module_de_la_feature_est_declare_dans_main() {
        let montages = pour("users");

        assert_eq!(lignes(&montages, ancres::FEATURES), ["mod users;"]);
    }

    #[test]
    fn les_routes_sont_montees_par_un_chemin_absolu() {
        let montages = pour("blog_posts");

        assert_eq!(
            lignes(&montages, ancres::ROUTES),
            [".merge(crate::blog_posts::routes())"]
        );
    }

    #[test]
    fn les_cinq_handlers_entrent_dans_le_document_openapi() {
        let montages = pour("users");

        assert_eq!(
            lignes(&montages, ancres::OPENAPI),
            [
                "crate::users::controller::list,",
                "crate::users::controller::create,",
                "crate::users::controller::find,",
                "crate::users::controller::update,",
                "crate::users::controller::delete,",
            ]
        );
    }

    #[test]
    fn la_migration_est_declaree_puis_inscrite_dans_le_migrator() {
        let montages = pour_migration("m20260826_143000_create_users");

        assert_eq!(
            lignes(&montages, ancres::MIGRATION_MODULES),
            ["mod m20260826_143000_create_users;"]
        );
        assert_eq!(
            lignes(&montages, ancres::MIGRATIONS),
            ["Box::new(m20260826_143000_create_users::Migration),"]
        );
    }

    /// Une feature écrite à la main porte sa propre migration, ou n'en a pas : le CLI
    /// n'inscrit dans le `Migrator` que ce qu'il a lui-même généré.
    #[test]
    fn le_montage_d_une_feature_ne_touche_pas_a_la_crate_migration() {
        let montages = pour("users");

        assert!(
            !montages
                .iter()
                .any(|montage| montage.ancre.fichier == ancres::MIGRATIONS.fichier),
            "la crate migration ne doit pas être touchée : {montages:?}"
        );
    }

    #[test]
    fn chaque_montage_vise_une_ancre_du_squelette() {
        let mut montages = pour("users");
        montages.extend(pour_migration("m20260826_143000_create_users"));

        assert_eq!(montages.len(), 5, "{montages:?}");
        for montage in &montages {
            assert!(
                ancres::ANCRES.contains(&montage.ancre),
                "`{}` n'est pas une ancre du squelette",
                montage.ancre.nom
            );
        }
    }
}
