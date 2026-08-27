//! Le même programme que `rbs`, sous un nom que rien ne dispute.
//!
//! `rbs` est aussi celui de l'outil de signatures de Ruby, que Homebrew installe dans un
//! répertoire placé devant `~/.cargo/bin`. Là où il gagne, `rbs` ne nous atteint jamais :
//! la collision ne peut être ni détectée ni signalée depuis le code, et il ne reste qu'à
//! offrir un second nom.

fn main() {
    rbs_cli::executer();
}
