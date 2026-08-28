//! Ce qu'un plan décrit : des actions, leur effet, et ce qu'elles produiront.
//!
//! Types seuls : la lecture du disque et le calcul des statuts appartiennent au
//! builder.

use crate::anchors::Anchor;
use crate::metadata::Dependency;

/// Une action du plan : le fichier qu'elle vise, ce qu'elle y fait, et ce qu'elle
/// produira.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Action {
    /// Chemin du fichier visé, relatif à la racine du projet.
    pub path: String,
    pub effet: Effect,
    pub statut: Status,
}

/// Ce qu'une action fait au fichier qu'elle vise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Effect {
    /// Écrit un fichier dont le contenu est entièrement connu.
    Creer { content: String },
    /// Ajoute des lignes dans une ancre, juste avant sa balise fermante.
    Inserer { anchor: Anchor, lines: Vec<String> },
    /// Modifie un manifeste TOML en préservant sa mise en forme.
    PatcherToml { patch: PatchToml },
    /// Ajoute une section à un document TOML qui n'est pas un manifeste Cargo.
    AjouterSection {
        /// Nom de la section, tel qu'il paraît entre crochets.
        section: String,
        /// Corps de la section, tel que le manifeste du fragment le déclare.
        content: String,
    },
    /// Ajoute une variable à un fichier d'environnement.
    AjouterVariable {
        /// Nom de la variable.
        key: String,
        /// Valeur d'exemple.
        value: String,
        /// Ce que la variable attend, en commentaire au-dessus d'elle.
        comment: Option<String>,
    },
}

/// Les modifications qu'un plan sait faire à un `Cargo.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PatchToml {
    /// Inscrit une feature dans `[package.metadata.rbs]`.
    InscrireFeature(String),
    /// Déclare une dépendance dans `[dependencies]`.
    AjouterDependance(Dependency),
    /// Active une feature sur une dépendance que le manifeste déclare déjà.
    AjouterFeatureADependance {
        /// Nom de la dépendance visée.
        dependency: String,
        /// Feature à y activer.
        feature: String,
    },
}

/// Ce que l'action produira, connu dès la planification.
///
/// Le statut décrit la relation de l'action au projet **tel qu'il a été trouvé**, jamais
/// à ce que les actions précédentes du plan ont projeté : sans cela, un plan pourrait
/// réclamer un forçage sur un fichier que lui seul a écrit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    /// Le contenu final diffère de celui d'origine : l'action aura un effet.
    AFaire,
    /// Le contenu final égale celui d'origine : l'action est sans effet.
    DejaFait,
    /// Le fichier existait déjà, avec un contenu que l'action n'a pas produit. Seule une
    /// exécution forcée l'écrasera.
    Conflit,
}

impl Status {
    /// Statut d'un fichier que plusieurs actions visent.
    ///
    /// Un conflit prime : il ne se dissout pas parce qu'une autre action du plan touche
    /// le même fichier. À l'inverse, un fichier n'est sans effet que si aucune de ses
    /// actions n'en a.
    pub fn merge(self, other: Status) -> Status {
        match (self, other) {
            (Status::Conflit, _) | (_, Status::Conflit) => Status::Conflit,
            (Status::AFaire, _) | (_, Status::AFaire) => Status::AFaire,
            (Status::DejaFait, Status::DejaFait) => Status::DejaFait,
        }
    }
}
