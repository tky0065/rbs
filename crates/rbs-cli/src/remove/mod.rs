//! `rbs remove <feature>` : défait ce que `rbs add` a posé.
//!
//! Le cœur de la commande, le parcours inverse du manifeste, vit dans
//! [`desinstallation`]. Le reste — options, garde Git, application du plan — appartient
//! à une tâche ultérieure : ce module ne déclare pour l'instant que celui-ci.

pub(crate) mod desinstallation;
