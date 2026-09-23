//! Les générateurs, et le peu qu'ils partagent au-delà de leurs modules.

use crate::metadata::Dependency;
use crate::plan::PatchToml;

/// Ce qu'un champ `decimal` demande au manifeste du projet.
///
/// `sea_orm::prelude::Decimal` n'existe que sous `with-rust_decimal` ; `serde-str` épingle
/// la représentation JSON du décimal — une chaîne — plutôt que de la laisser au défaut de
/// la crate, qu'une feature activée ailleurs dans le graphe pourrait changer.
///
/// Rendu ici plutôt qu'écrit dans chaque commande : `generate crud` crée la colonne,
/// `generate migration` l'ajoute, et une épingle qui divergerait entre les deux ferait
/// dépendre le manifeste de celle qui l'a touché en dernier.
pub(crate) fn patches_decimal() -> [PatchToml; 2] {
    [
        PatchToml::AjouterDependance(Dependency {
            name: "rust_decimal".to_string(),
            version: "1.43".to_string(),
            features: vec!["serde-str".to_string()],
            default_features: true,
        }),
        PatchToml::AjouterFeatureADependance {
            dependency: "sea-orm".to_string(),
            feature: "with-rust_decimal".to_string(),
        },
    ]
}

pub(crate) mod alter;
#[cfg(test)]
pub(crate) mod bench;
pub(crate) mod command;
pub(crate) mod controller;
pub(crate) mod dto;
pub(crate) mod entities;
pub(crate) mod entity;
pub(crate) mod feature;
pub(crate) mod fields;
pub(crate) mod filter;
pub(crate) mod format;
pub(crate) mod job;
pub(crate) mod migration;
pub(crate) mod mount;
pub(crate) mod name;
pub(crate) mod reference;
pub(crate) mod relations;
pub(crate) mod repository;
pub(crate) mod seed;
pub(crate) mod service;
pub(crate) mod tests_http;
