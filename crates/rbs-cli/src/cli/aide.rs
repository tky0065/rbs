//! L'aide de clap, rendue en français.
//!
//! clap n'a aucun réglage de langue : ses en-têtes, ses drapeaux `-h` et `-V`, la
//! sous-commande `help` qu'il engendre et ses mentions `[default: …]` sont écrits en dur.
//! Les drapeaux sont remplacés par les nôtres, les mentions masquées puis réécrites dans
//! l'aide de l'argument, et chaque commande reçoit un gabarit aux en-têtes français.

use clap::builder::PossibleValue;
use clap::{Arg, ArgAction, Command};

/// L'`about` que clap donne à la sous-commande `help` qu'il engendre.
const HELP_ENGENDREE: &str = "Print this message or the help of the given subcommand(s)";

/// `commande`, aide comprise en français.
pub(super) fn francise(commande: Command) -> Command {
    let mut commande = avant_construction(commande);
    // La sous-commande `help` et ses copies naissent à la construction : on ne peut les
    // traduire qu'ensuite.
    commande.build();
    apres_construction(commande)
}

fn noms(commande: &Command) -> Vec<String> {
    commande
        .get_subcommands()
        .map(|sous| sous.get_name().to_string())
        .collect()
}

/// Réécrit les mentions des arguments et remplace `-h` et `-V`, sur toute l'arborescence.
fn avant_construction(mut commande: Command) -> Command {
    for nom in noms(&commande) {
        commande = commande.mut_subcommand(nom, avant_construction);
    }

    let ids: Vec<String> = commande
        .get_arguments()
        .map(|arg| arg.get_id().to_string())
        .collect();
    for id in ids {
        commande = commande.mut_arg(id, mentions);
    }

    let longue = aide_longue(&commande);
    commande
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(drapeau_aide(longue))
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::Version)
                .help("Affiche la version"),
        )
}

/// Traduit la sous-commande `help` engendrée et pose le gabarit français partout.
fn apres_construction(mut commande: Command) -> Command {
    for nom in noms(&commande) {
        commande = commande.mut_subcommand(nom, apres_construction);
    }

    if commande
        .get_about()
        .is_some_and(|about| about.to_string() == HELP_ENGENDREE)
    {
        commande = commande.about("Affiche cette aide, ou celle des commandes données");
    }
    if commande.get_name() == "help"
        && commande
            .get_arguments()
            .any(|arg| arg.get_id() == "subcommand")
    {
        commande = commande.mut_arg("subcommand", |arg| {
            arg.value_name("COMMANDE")
                .help("Commande dont afficher l'aide")
        });
    }
    if commande.has_subcommands() {
        commande = commande.subcommand_value_name("COMMANDE");
    }

    let gabarit = gabarit(&commande);
    commande.help_template(gabarit)
}

/// Le gabarit d'aide de `commande` : celui de clap, en-têtes en français, sans les
/// sections qu'elle n'a pas.
fn gabarit(commande: &Command) -> String {
    let mut gabarit = String::from("{before-help}{about-with-newline}\nUtilisation : {usage}");
    if commande.get_subcommands().any(|sous| !sous.is_hide_set()) {
        gabarit.push_str("\n\nCommandes :\n{subcommands}");
    }
    if commande.get_positionals().any(|arg| !arg.is_hide_set()) {
        gabarit.push_str("\n\nArguments :\n{positionals}");
    }
    if commande
        .get_arguments()
        .any(|arg| !arg.is_positional() && !arg.is_hide_set())
    {
        gabarit.push_str("\n\nOptions :\n{options}");
    }
    gabarit.push_str("{after-help}");
    gabarit
}

/// Le drapeau d'aide, renvoyant à `--help` quand l'aide longue dit davantage.
fn drapeau_aide(longue: bool) -> Arg {
    let drapeau = Arg::new("help")
        .short('h')
        .long("help")
        .action(ArgAction::Help);
    if longue {
        drapeau
            .help("Affiche l'aide (plus de détail avec --help)")
            .long_help("Affiche l'aide (résumé avec -h)")
    } else {
        drapeau.help("Affiche l'aide")
    }
}

/// Vrai si `commande` a une aide longue distincte de la courte : ce que clap calcule pour
/// son propre `-h`, les valeurs possibles étant déjà réécrites.
fn aide_longue(commande: &Command) -> bool {
    commande.get_long_about().is_some()
        || commande.get_before_long_help().is_some()
        || commande.get_after_long_help().is_some()
        || commande.get_arguments().any(|arg| {
            !arg.is_hide_set()
                && (arg.get_long_help().is_some()
                    || arg.is_hide_long_help_set()
                    || arg.is_hide_short_help_set())
        })
}

/// `arg`, ses mentions de défaut et de valeurs possibles réécrites en français.
fn mentions(arg: Arg) -> Arg {
    let prend_valeur = arg.get_action().takes_values();
    let defauts: Vec<String> = if prend_valeur && !arg.is_hide_default_value_set() {
        arg.get_default_values()
            .iter()
            .map(|defaut| defaut.to_string_lossy().into_owned())
            .collect()
    } else {
        Vec::new()
    };
    // Avant la construction, `get_possible_values` rend une liste vide : clap ne sait pas
    // encore que l'argument prend une valeur. Son analyseur, lui, les connaît déjà.
    let valeurs: Vec<PossibleValue> = if prend_valeur && !arg.is_hide_possible_values_set() {
        arg.get_value_parser()
            .possible_values()
            .map(|valeurs| valeurs.filter(|valeur| !valeur.is_hide_set()).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    if defauts.is_empty() && valeurs.is_empty() {
        return arg;
    }

    let aide = arg.get_help().map(ToString::to_string).unwrap_or_default();
    let defaut = (!defauts.is_empty()).then(|| format!("[défaut : {}]", defauts.join(" ")));
    let liste = (!valeurs.is_empty()).then(|| {
        let noms: Vec<&str> = valeurs.iter().map(PossibleValue::get_name).collect();
        format!("[valeurs : {}]", noms.join(", "))
    });

    let courte = [Some(aide.clone()), defaut.clone(), liste.clone()]
        .into_iter()
        .flatten()
        .filter(|morceau| !morceau.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let decrites = valeurs.iter().any(|valeur| valeur.get_help().is_some());
    let longue = (decrites || arg.get_long_help().is_some()).then(|| {
        let mut parties = vec![arg.get_long_help().map_or(aide, ToString::to_string)];
        if decrites {
            let mut bloc = String::from("Valeurs possibles :");
            for valeur in &valeurs {
                match valeur.get_help() {
                    Some(aide) => bloc.push_str(&format!("\n- {} : {aide}", valeur.get_name())),
                    None => bloc.push_str(&format!("\n- {}", valeur.get_name())),
                }
            }
            parties.push(bloc);
        } else {
            parties.extend(liste);
        }
        parties.extend(defaut);
        parties.join("\n\n")
    });

    let arg = arg
        .hide_default_value(true)
        .hide_possible_values(true)
        .help(courte);
    match longue {
        Some(longue) => arg.long_help(longue),
        None => arg,
    }
}
