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
fn output(binaire: &str, args: &[&str]) -> String {
    let rendu = Command::cargo_bin(binaire)
        .unwrap_or_else(|_| panic!("le binaire {binaire} doit être compilé"))
        .args(args)
        .output()
        .expect("le binaire doit s'exécuter");
    assert!(rendu.status.success(), "{binaire} {args:?} a échoué");
    String::from_utf8(rendu.stdout).expect("sortie UTF-8")
}

/// L'aide amputée de sa ligne `Usage:`, seule à porter le nom de l'invocation.
fn help_without_usage(binaire: &str) -> String {
    output(binaire, &["--help"])
        .lines()
        .filter(|ligne| !ligne.starts_with("Usage:"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn both_names_return_the_same_version() {
    assert_eq!(
        output("rbs-cli", &["--version"]),
        output("rbs", &["--version"])
    );
}

#[test]
fn both_names_expose_the_same_commands() {
    assert_eq!(help_without_usage("rbs-cli"), help_without_usage("rbs"));
}

/// Le nom que la ligne `Usage:` donne à la commande, extension de plateforme comprise.
fn name_in_usage(binaire: &str) -> String {
    output(binaire, &["--help"])
        .lines()
        .find_map(|ligne| ligne.strip_prefix("Usage: "))
        .expect("l'aide porte une ligne Usage")
        .split_whitespace()
        .next()
        .expect("la ligne Usage nomme la commande")
        .to_string()
}

#[test]
fn each_name_announces_itself_under_the_one_typed() {
    // Windows livre `rbs-cli.exe`, dont `argv[0]` porte l'extension : comparer au nom nu
    // ferait échouer là où le comportement est pourtant le bon.
    let attendu = |nom: &str| format!("{nom}{}", std::env::consts::EXE_SUFFIX);

    assert_eq!(name_in_usage("rbs-cli"), attendu("rbs-cli"));
    assert_eq!(name_in_usage("rbs"), attendu("rbs"));
}
