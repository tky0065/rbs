//! Le second nom du binaire, `rbs-cli`.
//!
//! `rbs` est déjà porté par l'outil de signatures de Ruby, que Homebrew installe dans un
//! répertoire placé devant `~/.cargo/bin` sur macOS. Là où il gagne, aucune ligne de ce
//! CLI ne s'exécute : la collision ne peut être ni détectée ni signalée depuis le code,
//! et `doctor` n'y peut rien. Un second nom, libre, est la seule réponse qui ne demande à
//! l'utilisateur qu'un autre mot à taper.
//!
//! Ces tests existent pour qu'une cible `[[bin]]` supprimée par mégarde fasse rougir la
//! CI, et pour tenir la garantie qui rend ce second nom utilisable : l'aide qu'il affiche
//! nomme la commande qu'on vient de taper, non celle qui est inaccessible.

use assert_cmd::Command;

/// La sortie standard d'un binaire livré, en exigeant qu'il réussisse.
fn sortie(binaire: &str, args: &[&str]) -> String {
    let rendu = Command::cargo_bin(binaire)
        .unwrap_or_else(|_| panic!("le binaire {binaire} doit être compilé"))
        .args(args)
        .output()
        .expect("le binaire doit s'exécuter");
    assert!(rendu.status.success(), "{binaire} {args:?} a échoué");
    String::from_utf8(rendu.stdout).expect("sortie UTF-8")
}

/// L'aide amputée de sa ligne `Usage:`, seule à porter le nom de l'invocation.
fn aide_sans_usage(binaire: &str) -> String {
    sortie(binaire, &["--help"])
        .lines()
        .filter(|ligne| !ligne.starts_with("Usage:"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn les_deux_noms_rendent_la_meme_version() {
    assert_eq!(
        sortie("rbs-cli", &["--version"]),
        sortie("rbs", &["--version"])
    );
}

#[test]
fn les_deux_noms_exposent_les_memes_commandes() {
    assert_eq!(aide_sans_usage("rbs-cli"), aide_sans_usage("rbs"));
}

#[test]
fn chaque_nom_s_annonce_sous_celui_qu_on_a_tape() {
    assert!(sortie("rbs-cli", &["--help"]).contains("Usage: rbs-cli "));
    assert!(sortie("rbs", &["--help"]).contains("Usage: rbs "));
}
