//! Le script de complétion du shell, engendré depuis la déclaration clap elle-même.
//!
//! Un script écrit à la main aurait divergé du CLI au premier drapeau ajouté ; celui-ci
//! naît de la même `Command` que le parseur.

use std::io;

use clap::CommandFactory;
use clap::builder::PossibleValuesParser;
use clap_complete::Shell;

use crate::cli::Cli;

/// Écrit sur `buffer` le script de complétion de `shell`.
pub(crate) fn render(shell: Shell, buffer: &mut impl io::Write) {
    clap_complete::generate(shell, &mut command(), "rbs", buffer);
}

/// La déclaration du CLI, augmentée de ce qui n'aide qu'à compléter.
///
/// Le catalogue des fragments ne descend pas dans le parseur réel : posé là, il ferait
/// refuser `rbs add ma-feature --template-dir ./mes-templates`, dont le nom ne vient
/// justement pas du binaire. Proposer les noms embarqués reste juste, puisque c'est ce
/// qu'un shell ne peut pas deviner ; les refuser ne le serait pas.
fn command() -> clap::Command {
    let fragments = PossibleValuesParser::new(crate::templates::embedded_names());

    Cli::command().mut_subcommand("add", |add| {
        add.mut_arg("feature", |feature| feature.value_parser(fragments))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les quatre shells que la commande annonce, et sur lesquels le rendu est exercé.
    const SHELLS: [Shell; 4] = [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell];

    /// Le script d'un shell, en clair.
    fn script(shell: Shell) -> String {
        let mut script = Vec::new();
        render(shell, &mut script);

        String::from_utf8(script).expect("le script est de l'UTF-8")
    }

    #[test]
    fn every_shell_gets_a_script_naming_the_subcommands() {
        for shell in SHELLS {
            let script = script(shell);

            assert!(!script.is_empty(), "{shell} : script vide");
            assert!(script.contains("doctor"), "{shell} : `doctor` absente");
            assert!(script.contains("generate"), "{shell} : `generate` absente");
        }
    }

    /// Compléter `rbs add ` sans proposer de feature laisserait l'utilisateur retaper de
    /// mémoire les noms que le binaire est seul à connaître.
    ///
    /// L'assertion porte sur la `Command` et non sur les scripts : des quatre
    /// générateurs, seuls ceux de bash et de zsh écrivent les valeurs d'un argument
    /// positionnel — chercher ces noms dans un script fish se satisferait de la
    /// description d'`add`, qui les énumère déjà en prose.
    #[test]
    fn the_command_rendered_proposes_the_embedded_fragments_after_add() {
        let fragments = crate::templates::embedded_names();
        assert_eq!(fragments.len(), 10, "le catalogue embarqué a changé");
        assert!(
            fragments.contains(&"observability".to_string()),
            "{fragments:?}"
        );

        let proposees: Vec<String> = command()
            .find_subcommand_mut("add")
            .expect("`add` absente du CLI")
            .get_arguments()
            .find(|argument| argument.get_id() == "feature")
            .expect("`feature` absent d'`add`")
            .get_possible_values()
            .iter()
            .map(|valeur| valeur.get_name().to_string())
            .collect();

        assert_eq!(proposees, fragments);
    }

    /// Les deux shells dont le générateur descend jusqu'aux valeurs d'un positionnel :
    /// c'est là que l'enrichissement se voit vraiment dans le script livré.
    ///
    /// La liste est cherchée séparée par des espaces, telle que les deux générateurs
    /// l'écrivent : la description d'`add` énumère les mêmes noms séparés par des
    /// virgules, et un nom cherché seul y répondrait sans qu'aucune valeur ne soit
    /// proposée.
    #[test]
    fn the_bash_and_zsh_scripts_carry_the_fragment_names() {
        let liste = crate::templates::embedded_names().join(" ");

        for shell in [Shell::Bash, Shell::Zsh] {
            assert!(
                script(shell).contains(&liste),
                "{shell} : les fragments ne sont pas proposés après `add`"
            );
        }
    }

    /// Le catalogue proposé à la complétion ne doit pas être descendu dans le parseur :
    /// `rbs add ma-feature --template-dir ./mes-templates` installe un fragment dont
    /// aucun binaire ne porte le nom, et clap le refuserait avant qu'`add` ne le cherche.
    #[test]
    fn the_real_parser_still_accepts_a_feature_that_no_fragment_carries() {
        use clap::Parser;

        let ajout = Cli::try_parse_from(["rbs", "add", "un-nom-inconnu"])
            .expect("le parseur réel n'a pas de catalogue");
        let crate::cli::Commands::Add { feature, .. } = ajout.command else {
            panic!("`add` attendue");
        };

        assert_eq!(feature, "un-nom-inconnu");
    }
}
