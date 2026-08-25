//! Runtime partagé des projets générés par rbs.
//!
//! Cette crate porte ce qui n'a pas de raison de varier d'un projet à l'autre. Tout ce
//! qu'un développeur voudra lire ou modifier est généré dans son projet par `rbs-cli`.

#![warn(missing_docs)]

/// Démonstration jetable : cette fonction porte un warning clippy volontaire.
pub fn warning_volontaire() {
    let inutilise = 1;
}
