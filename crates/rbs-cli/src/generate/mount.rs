//! Ce qu'une feature ajoute aux ancres du projet.
//!
//! Chaque ligne insérée désigne la feature par un chemin absolu — `crate::users::routes()`
//! — pour qu'une insertion se suffise à elle-même : aucune seconde écriture dans un bloc
//! `use` ne l'accompagne.

use crate::anchors::{self, Anchor};

use super::relations;

/// Les handlers que le controller généré expose, dans l'ordre où ils y sont écrits.
const HANDLERS: [&str; 5] = ["list", "create", "find", "update", "delete"];

/// Une insertion à faire : l'ancre visée, et les lignes à y ajouter.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Mount {
    pub anchor: Anchor,
    pub lines: Vec<String>,
}

/// Ce que la feature `module` ajoute au projet.
///
/// `features` est déjà résolue par [`anchors::resolve_features`] : la déclaration du
/// module se pose en `pub mod` quand la cible est `src/lib.rs`, faute de quoi le binaire
/// des seeds ne pourrait pas atteindre l'entité depuis l'autre côté de la frontière de
/// crate ; elle reste un `mod` privé sur un projet antérieur, encore réduit à `src/main.rs`.
pub(crate) fn pour(module: &str, features: Anchor) -> Vec<Mount> {
    let visibility = if features.file == "src/lib.rs" {
        "pub "
    } else {
        ""
    };

    vec![
        Mount {
            anchor: features,
            lines: vec![format!("{visibility}mod {module};")],
        },
        Mount {
            anchor: anchors::ROUTES,
            lines: vec![format!(".merge(crate::{module}::routes())")],
        },
        Mount {
            anchor: anchors::OPENAPI,
            lines: HANDLERS
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
pub(crate) fn for_migration(module: &str) -> Vec<Mount> {
    vec![
        Mount {
            anchor: anchors::MIGRATION_MODULES,
            lines: vec![format!("mod {module};")],
        },
        Mount {
            anchor: anchors::MIGRATIONS,
            lines: vec![format!("Box::new({module}::Migration),")],
        },
    ]
}

/// Ce que le côté inverse d'une relation ajoute au modèle de sa cible.
///
/// Deux ancres et non une : la variante vit dans les accolades de l'énumération, l'`impl
/// Related` ne le peut pas.
pub(crate) fn for_inverse(inverse: &relations::Inverse) -> Vec<Mount> {
    vec![
        Mount {
            anchor: anchors::RELATIONS.in_file(&inverse.file),
            lines: inverse.variant.clone(),
        },
        Mount {
            anchor: anchors::RELATED.in_file(&inverse.file),
            lines: inverse.related.clone(),
        },
    ]
}

/// Ce que le seed de `module` ajoute au binaire des seeds.
///
/// Séparé de [`pour`] pour la même raison que [`for_migration`] : une feature écrite à la
/// main n'a pas d'entité, donc rien à semer.
///
/// L'ancre empile dans l'ordre de génération, qui est aussi celui des migrations : le jour
/// où un seed dépendra d'un autre, c'est cet ordre-là qui tiendra, non l'alphabet.
pub(crate) fn for_seed(module: &str) -> Vec<Mount> {
    vec![Mount {
        anchor: anchors::SEEDS,
        lines: vec![format!("{module},")],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(montages: &[Mount], anchor: Anchor) -> &[String] {
        &montages
            .iter()
            .find(|mount| mount.anchor == anchor)
            .unwrap_or_else(|| panic!("aucun montage pour `{}`", anchor.name))
            .lines
    }

    #[test]
    fn the_feature_module_is_declared_in_main_on_a_project_without_a_library() {
        let montages = pour("users", anchors::FEATURES);

        assert_eq!(lines(&montages, anchors::FEATURES), ["mod users;"]);
    }

    #[test]
    fn the_feature_module_is_declared_public_in_the_library() {
        let lib = anchors::FEATURES.in_file("src/lib.rs");
        let montages = pour("users", lib.clone());

        assert_eq!(lines(&montages, lib), ["pub mod users;"]);
    }

    #[test]
    fn the_routes_are_mounted_by_an_absolute_path() {
        let montages = pour("blog_posts", anchors::FEATURES);

        assert_eq!(
            lines(&montages, anchors::ROUTES),
            [".merge(crate::blog_posts::routes())"]
        );
    }

    #[test]
    fn the_five_handlers_enter_the_openapi_document() {
        let montages = pour("users", anchors::FEATURES);

        assert_eq!(
            lines(&montages, anchors::OPENAPI),
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
    fn the_inverse_targets_the_two_anchors_of_the_computed_file() {
        let inverse = relations::Inverse {
            file: "src/auth/model.rs".to_string(),
            variant: vec![
                r#"    #[sea_orm(has_many = "crate::posts::model::Entity")]"#.to_string(),
                "    Posts,".to_string(),
            ],
            related: vec![
                "impl Related<crate::posts::model::Entity> for Entity {".to_string(),
                "    fn to() -> RelationDef { Relation::Posts.def() }".to_string(),
                "}".to_string(),
            ],
        };

        let montages = for_inverse(&inverse);

        assert_eq!(montages.len(), 2, "{montages:?}");
        let relations_anchor = anchors::RELATIONS.in_file("src/auth/model.rs");
        let related_anchor = anchors::RELATED.in_file("src/auth/model.rs");
        assert_eq!(
            lines(&montages, relations_anchor),
            inverse.variant.as_slice()
        );
        assert_eq!(lines(&montages, related_anchor), inverse.related.as_slice());
    }

    #[test]
    fn the_migration_is_declared_then_recorded_in_the_migrator() {
        let montages = for_migration("m20260826_143000_create_users");

        assert_eq!(
            lines(&montages, anchors::MIGRATION_MODULES),
            ["mod m20260826_143000_create_users;"]
        );
        assert_eq!(
            lines(&montages, anchors::MIGRATIONS),
            ["Box::new(m20260826_143000_create_users::Migration),"]
        );
    }

    /// Une feature écrite à la main porte sa propre migration, ou n'en a pas : le CLI
    /// n'inscrit dans le `Migrator` que ce qu'il a lui-même généré.
    #[test]
    fn mounting_a_feature_does_not_touch_the_migration_crate() {
        let montages = pour("users", anchors::FEATURES);

        assert!(
            !montages
                .iter()
                .any(|mount| mount.anchor.file == anchors::MIGRATIONS.file),
            "la crate migration ne doit pas être touchée : {montages:?}"
        );
    }

    #[test]
    fn the_seed_is_declared_by_its_module_name() {
        let montages = for_seed("blog_posts");

        assert_eq!(lines(&montages, anchors::SEEDS), ["blog_posts,"]);
    }

    /// Une feature écrite à la main n'a pas d'entité : rien à semer, donc rien à déclarer.
    #[test]
    fn mounting_a_feature_declares_no_seed() {
        let montages = pour("users", anchors::FEATURES);

        assert!(
            !montages.iter().any(|mount| mount.anchor == anchors::SEEDS),
            "le binaire des seeds ne doit pas être touché : {montages:?}"
        );
    }

    #[test]
    fn each_mount_targets_a_skeleton_anchor() {
        let mut montages = pour("users", anchors::FEATURES);
        montages.extend(for_migration("m20260826_143000_create_users"));
        montages.extend(for_seed("users"));

        assert_eq!(montages.len(), 6, "{montages:?}");
        for mount in &montages {
            assert!(
                anchors::ANCRES.contains(&mount.anchor),
                "`{}` n'est pas une ancre du squelette",
                mount.anchor.name
            );
        }
    }
}
