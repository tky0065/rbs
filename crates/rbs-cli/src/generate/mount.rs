//! Ce qu'une feature ajoute aux ancres du projet.
//!
//! Chaque ligne insérée désigne la feature par un chemin absolu — `crate::users::routes()`
//! — pour qu'une insertion se suffise à elle-même : aucune seconde écriture dans un bloc
//! `use` ne l'accompagne.

use crate::anchors::{self, Anchor};

use super::relations;

/// Les handlers que le controller généré expose, dans l'ordre où ils y sont écrits.
const HANDLERS: [&str; 6] = ["list", "filter", "create", "find", "update", "delete"];

/// Les trois handlers que `--with-upload` ajoute, dans le même ordre.
///
/// Ils s'inscrivent à l'ancre `openapi` comme les six autres : montées sans y figurer,
/// les routes existeraient hors du document, et aucune compilation ne le dirait.
const HANDLERS_CONTENU: [&str; 3] = ["put_content", "get_content", "head_content"];

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
pub(crate) fn pour(module: &str, features: Anchor, with_upload: bool) -> Vec<Mount> {
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
                .chain(if with_upload {
                    HANDLERS_CONTENU.iter()
                } else {
                    [].iter()
                })
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
/// Related` ne le peut pas. Toutes deux portent le nom de l'entité visée : un fichier de
/// modèle peut en décrire plusieurs.
///
/// Une cible visée par plusieurs relations n'a pas d'`impl Related` à recevoir, et le
/// montage se réduit alors au commentaire qui l'explique.
pub(crate) fn for_inverse(inverse: &relations::Inverse) -> Vec<Mount> {
    let mut montages = vec![Mount {
        anchor: anchors::RELATIONS.for_entity(&inverse.file, &inverse.entity),
        lines: inverse.variant.clone(),
    }];

    if !inverse.related.is_empty() {
        montages.push(Mount {
            anchor: anchors::RELATED.for_entity(&inverse.file, &inverse.entity),
            lines: inverse.related.clone(),
        });
    }

    montages
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
        let montages = pour("users", anchors::FEATURES, false);

        assert_eq!(lines(&montages, anchors::FEATURES), ["mod users;"]);
    }

    #[test]
    fn the_feature_module_is_declared_public_in_the_library() {
        let lib = anchors::FEATURES.in_file("src/lib.rs");
        let montages = pour("users", lib.clone(), false);

        assert_eq!(lines(&montages, lib), ["pub mod users;"]);
    }

    #[test]
    fn the_routes_are_mounted_by_an_absolute_path() {
        let montages = pour("blog_posts", anchors::FEATURES, false);

        assert_eq!(
            lines(&montages, anchors::ROUTES),
            [".merge(crate::blog_posts::routes())"]
        );
    }

    #[test]
    fn the_six_handlers_enter_the_openapi_document() {
        let montages = pour("users", anchors::FEATURES, false);

        assert_eq!(
            lines(&montages, anchors::OPENAPI),
            [
                "crate::users::controller::list,",
                "crate::users::controller::filter,",
                "crate::users::controller::create,",
                "crate::users::controller::find,",
                "crate::users::controller::update,",
                "crate::users::controller::delete,",
            ]
        );
    }

    #[test]
    fn the_three_content_handlers_reach_the_openapi_anchor() {
        let mounts = pour("articles", anchors::FEATURES, true);
        let openapi = mounts
            .iter()
            .find(|mount| mount.anchor == anchors::OPENAPI)
            .expect("l'ancre openapi doit être visée");

        assert_eq!(
            openapi.lines.len(),
            9,
            "trois handlers de plus ; sans eux les routes existent hors du document, \
             et rien ne le signale : {:?}",
            openapi.lines
        );
        assert!(
            openapi
                .lines
                .iter()
                .any(|line| line.contains("controller::put_content")),
            "{:?}",
            openapi.lines
        );
    }

    #[test]
    fn an_ordinary_feature_mounts_six_handlers() {
        let mounts = pour("articles", anchors::FEATURES, false);
        let openapi = mounts
            .iter()
            .find(|mount| mount.anchor == anchors::OPENAPI)
            .expect("l'ancre openapi doit être visée");

        assert_eq!(openapi.lines.len(), 6, "témoin : {:?}", openapi.lines);
    }

    fn inverse_towards_users() -> relations::Inverse {
        relations::Inverse {
            file: "src/auth/model.rs".to_string(),
            entity: "users".to_string(),
            variant: vec![
                r#"#[sea_orm(has_many = "crate::posts::model::Entity")]"#.to_string(),
                "Posts,".to_string(),
            ],
            related: relations::related_impl("crate::posts::model::Entity", "Posts"),
        }
    }

    /// Les deux ancres visées portent le nom de l'entité, et non celui du seul fichier :
    /// `src/auth/model.rs` en porte deux paires, une par entité nichée.
    #[test]
    fn the_inverse_targets_the_two_anchors_of_its_entity() {
        let inverse = inverse_towards_users();

        let montages = for_inverse(&inverse);

        assert_eq!(montages.len(), 2, "{montages:?}");
        let relations_anchor = anchors::RELATIONS.for_entity("src/auth/model.rs", "users");
        let related_anchor = anchors::RELATED.for_entity("src/auth/model.rs", "users");
        assert_eq!(
            lines(&montages, relations_anchor),
            inverse.variant.as_slice()
        );
        assert_eq!(lines(&montages, related_anchor), inverse.related.as_slice());
    }

    /// Une cible visée plusieurs fois ne reçoit qu'un commentaire : rien à monter dans
    /// l'ancre des `impl Related`, qui n'en recevra pas.
    #[test]
    fn an_inverse_without_a_related_impl_mounts_a_single_anchor() {
        let inverse = relations::Inverse {
            related: Vec::new(),
            ..inverse_towards_users()
        };

        let montages = for_inverse(&inverse);

        assert_eq!(montages.len(), 1, "{montages:?}");
        assert_eq!(
            montages[0].anchor,
            anchors::RELATIONS.for_entity("src/auth/model.rs", "users")
        );
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
        let montages = pour("users", anchors::FEATURES, false);

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
        let montages = pour("users", anchors::FEATURES, false);

        assert!(
            !montages.iter().any(|mount| mount.anchor == anchors::SEEDS),
            "le binaire des seeds ne doit pas être touché : {montages:?}"
        );
    }

    #[test]
    fn each_mount_targets_a_skeleton_anchor() {
        let mut montages = pour("users", anchors::FEATURES, false);
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
