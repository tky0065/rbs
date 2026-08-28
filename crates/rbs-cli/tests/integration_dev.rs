//! `rbs dev` vu depuis la ligne de commande.
//!
//! Ce que ces tests tiennent est le second critère de la commande : ce qui manque se dit
//! en une phrase et se solde par un code de sortie, jamais par une trace de panique. La
//! preuve doit rester légère — ni Docker, ni base, ni projet compilé — pour tourner sur
//! les trois plateformes de la CI.

use assert_cmd::Command;
use tempfile::TempDir;

/// Ce que `rbs dev` écrit et rend, lancé depuis `directory`.
fn dev(directory: &std::path::Path) -> (i32, String) {
    let rendu = Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .arg("dev")
        .current_dir(directory)
        .output()
        .expect("le binaire doit s'exécuter");

    let sortie = format!(
        "{}{}",
        String::from_utf8_lossy(&rendu.stdout),
        String::from_utf8_lossy(&rendu.stderr)
    );

    (rendu.status.code().unwrap_or(-1), sortie)
}

#[test]
fn outside_a_project_the_command_says_so_and_fails() {
    let ailleurs = TempDir::new().expect("répertoire temporaire créable");

    let (code, sortie) = dev(ailleurs.path());

    assert_eq!(code, 1, "code de sortie inattendu :\n{sortie}");
    assert!(
        sortie.contains("projet rbs"),
        "le message ne dit pas ce qui manque :\n{sortie}"
    );
    assert!(
        !sortie.contains("panicked"),
        "une trace de panique est remontée :\n{sortie}"
    );
}

#[test]
fn a_project_pointing_at_a_dead_port_names_the_host_and_port() {
    // Un port lié puis relâché : personne n'écoute derrière, et le démarrage doit s'y
    // arrêter avant même de compiler quoi que ce soit.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("un port éphémère est libre")
        .local_addr()
        .expect("le socket porte son adresse")
        .port();

    let parent = TempDir::new().expect("répertoire temporaire créable");
    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .args(["new", "demo-api", "--yes", "--database-url"])
        .arg(format!("postgres://rbs:rbs@127.0.0.1:{port}/demo"))
        .current_dir(parent.path())
        .assert()
        .success();

    let (code, sortie) = dev(&parent.path().join("demo-api"));

    assert_eq!(code, 1, "code de sortie inattendu :\n{sortie}");
    assert!(
        sortie.contains("127.0.0.1") && sortie.contains(&port.to_string()),
        "le message ne nomme pas la base injoignable :\n{sortie}"
    );
    assert!(
        !sortie.contains("panicked"),
        "une trace de panique est remontée :\n{sortie}"
    );
}
