//! Lancement d'un binaire du projet par cargo.
//!
//! Ni `rbs migrate` ni `rbs seed` ne parlent à la base : ils lancent un binaire du projet,
//! qui, lui, en a le droit. Le spawn tient ici ; le message d'échec reste à chacune, une
//! migration qui échoue et un seed qui échoue n'ayant rien à se dire.

use std::io::{self, Write};
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
/// Quand la sortie standard est capturée, c'est qu'un appelant va la lire pour en tirer
/// un verdict — `doctor`, `migrate status` — et la progression de cargo (`Updating`,
/// `Locking`, `Compiling`) n'a pas à précéder ce verdict de cinquante lignes : `stderr`
/// est capturée aussi, et rejouée seulement si le binaire échoue, pour qu'une erreur de
/// compilation reste lisible. Sans capture, les deux flux restent hérités : `migrate up`,
/// `seed` et `dev` montrent leur compilation comme avant.
pub(crate) fn run(
    root: &Path,
    arguments: &[&str],
    variables: &[(String, String)],
    capturer: bool,
) -> Result<String, Error> {
    run_with(
        "cargo",
        root,
        arguments,
        variables,
        capturer,
        &mut io::stderr().lock(),
    )
}

/// Le programme et le puits de rejeu sont les deux points qu'un test remplace : un faux
/// `cargo` en script shell, et un tampon à la place de la sortie d'erreur du terminal.
fn run_with(
    programme: &str,
    root: &Path,
    arguments: &[&str],
    variables: &[(String, String)],
    capturer: bool,
    rejeu: &mut dyn Write,
) -> Result<String, Error> {
    let flux = || {
        if capturer {
            Stdio::piped()
        } else {
            Stdio::inherit()
        }
    };
    let mut processus = Command::new(programme);
    processus
        .current_dir(root)
        .args(arguments)
        .envs(variables.iter().map(|(key, value)| (key, value)))
        .stdout(flux())
        .stderr(flux());

    let output = processus
        .spawn()
        .map_err(Error::Lancement)?
        .wait_with_output()
        .map_err(Error::Lancement)?;

    if !output.status.success() {
        let _ = rejeu.write_all(&output.stderr);
        return Err(Error::Statut(output.status.code().unwrap_or(1)));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    /// Un faux `cargo` : du bruit sur stderr, la sortie utile sur stdout, et le code de
    /// sortie que lui passe son premier argument.
    fn faux_cargo(dossier: &TempDir) -> PathBuf {
        let chemin = dossier.path().join("cargo");
        fs::write(
            &chemin,
            "#!/bin/sh\necho 'Compiling faux' >&2\necho utile\nexit \"$1\"\n",
        )
        .expect("script écrit");
        fs::set_permissions(&chemin, fs::Permissions::from_mode(0o755)).expect("exécutable");
        chemin
    }

    fn lancer(code: &str) -> (Result<String, Error>, String) {
        let dossier = TempDir::new().expect("répertoire temporaire");
        let programme = faux_cargo(&dossier);
        let mut puits = Vec::new();
        let resultat = run_with(
            programme.to_str().expect("chemin UTF-8"),
            dossier.path(),
            &[code],
            &[],
            true,
            &mut puits,
        );
        (resultat, String::from_utf8_lossy(&puits).into_owned())
    }

    #[test]
    fn a_captured_success_yields_stdout_and_replays_nothing() {
        let (resultat, rejeu) = lancer("0");

        assert_eq!(resultat.expect("succès").as_str(), "utile\n");
        assert!(rejeu.is_empty(), "rien ne doit être rejoué : {rejeu:?}");
    }

    #[test]
    fn a_captured_failure_replays_stderr_before_the_status() {
        let (resultat, rejeu) = lancer("1");

        assert!(
            matches!(resultat, Err(Error::Statut(1))),
            "attendu Statut(1), obtenu {resultat:?}"
        );
        assert!(
            rejeu.contains("Compiling faux"),
            "la sortie d'erreur de cargo doit être rejouée : {rejeu:?}"
        );
    }
}
