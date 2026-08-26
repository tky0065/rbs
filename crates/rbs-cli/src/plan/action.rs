//! Ce qu'un plan décrit : des actions, leur effet, et ce qu'elles produiront.
//!
//! Types seuls : la lecture du disque et le calcul des statuts appartiennent au
//! constructeur.

use crate::ancres::Ancre;

/// Une action du plan : le fichier qu'elle vise, ce qu'elle y fait, et ce qu'elle
/// produira.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Action {
    /// Chemin du fichier visé, relatif à la racine du projet.
    pub chemin: String,
    pub effet: Effet,
    pub statut: Statut,
}

/// Ce qu'une action fait au fichier qu'elle vise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Effet {
    /// Écrit un fichier dont le contenu est entièrement connu.
    Creer { contenu: String },
    /// Ajoute des lignes dans une ancre, juste avant sa balise fermante.
    Inserer { ancre: Ancre, lignes: Vec<String> },
    /// Modifie un manifeste TOML en préservant sa mise en forme.
    PatcherToml { patch: PatchToml },
}

/// Les modifications qu'un plan sait faire à un `Cargo.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PatchToml {
    /// Inscrit une feature dans `[package.metadata.rbs]`.
    InscrireFeature(String),
}

/// Ce que l'action produira, connu dès la planification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Statut {
    /// Le contenu final diffère de l'actuel : l'action aura un effet.
    AFaire,
    /// Le contenu final égale l'actuel : l'action est sans effet.
    DejaFait,
    /// Le fichier existe, avec un contenu que l'action n'a pas produit. Seule une
    /// exécution forcée l'écrasera.
    Conflit,
}
