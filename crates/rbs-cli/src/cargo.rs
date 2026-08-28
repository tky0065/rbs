//! Lancement d'un binaire du projet par cargo.
//!
//! Ni `rbs migrate` ni `rbs seed` ne parlent à la base : ils lancent un binaire du projet,
//! qui, lui, en a le droit. Le spawn tient ici ; le message d'échec reste à chacune, une
//! migration qui échoue et un seed qui échoue n'ayant rien à se dire.

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// Ce qui peut empêcher un binaire du projet de rendre son travail.
#[derive(Debug)]
pub(crate) enum Error {
    /// `cargo` lui-même n'a pas pu être lancé.
    Lancement(io::Error),
    /// Le binaire a rendu un code non nul.
    Statut(i32),
}

/// Lance `cargo <arguments>` à la racine du projet et rend sa sortie standard.
///
/// `stderr` reste hérité : la progression de cargo, qui compile la crate au premier appel,
/// doit rester visible pendant que la sortie utile est capturée.
pub(crate) fn run(
    root: &Path,
    arguments: &[&str],
    variables: &[(String, String)],
    capturer: bool,
) -> Result<String, Error> {
    let mut processus = Command::new("cargo");
    processus
        .current_dir(root)
        .args(arguments)
        .envs(variables.iter().map(|(key, value)| (key, value)))
        .stdout(if capturer {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });

    let output = processus
        .spawn()
        .map_err(Error::Lancement)?
        .wait_with_output()
        .map_err(Error::Lancement)?;

    if !output.status.success() {
        return Err(Error::Statut(output.status.code().unwrap_or(1)));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
