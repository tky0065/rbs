//! Les erreurs d'usage de clap, rendues en français.
//!
//! clap compose ses messages en anglais, en dur. Ceux qu'on rencontre en tapant mal une
//! commande sont réécrits ici depuis le contexte que porte l'erreur ; les genres plus
//! rares gardent le rendu de clap plutôt qu'un message appauvri.

use clap::error::{ContextKind, ContextValue, ErrorKind};

/// Sort du processus sur `error` : l'aide et la version telles quelles, une erreur
/// d'usage en français, avec le code de sortie de clap.
pub(crate) fn exit(error: clap::Error) -> ! {
    match message(&error) {
        Some(texte) => {
            crate::ui::error(&texte);
            std::process::exit(error.exit_code());
        }
        None => error.exit(),
    }
}

/// Le rendu français d'une erreur d'usage, ou `None` pour celles que clap rend seul.
pub(crate) fn message(error: &clap::Error) -> Option<String> {
    let mut texte = cause(error)?;

    for (contexte, annonce) in [
        (
            ContextKind::SuggestedSubcommand,
            "une commande voisine existe",
        ),
        (ContextKind::SuggestedArg, "une option voisine existe"),
        (ContextKind::SuggestedValue, "une valeur voisine existe"),
    ] {
        if let Some(voisins) = liste(error.get(contexte)) {
            texte.push_str(&format!("\n\n  {annonce} : {}", guillemets(&voisins)));
        }
    }

    // Le seul conseil que clap donne sur un argument inconnu : le faire passer pour une
    // valeur. Les autres, rares, ne se traduisent pas sans en connaître la forme.
    if error.kind() == ErrorKind::UnknownArgument
        && let Some(ContextValue::StyledStrs(conseils)) = error.get(ContextKind::Suggested)
        && conseils
            .iter()
            .any(|c| c.to_string().starts_with("to pass"))
        && let Some(argument) = chaine(error.get(ContextKind::InvalidArg))
    {
        texte.push_str(&format!(
            "\n\n  pour le passer comme valeur, écrivez « -- {argument} »"
        ));
    }

    if let Some(ContextValue::StyledStr(usage)) = error.get(ContextKind::Usage) {
        let usage = usage.to_string();
        let usage = usage.strip_prefix("Usage: ").unwrap_or(&usage);
        texte.push_str(&format!("\n\nUtilisation : {usage}"));
    }

    texte.push_str("\n\nPour plus d'informations, essayez « --help ».");
    Some(texte)
}

/// La première phrase du message, propre au genre de l'erreur.
fn cause(error: &clap::Error) -> Option<String> {
    let argument = chaine(error.get(ContextKind::InvalidArg));
    let valeur = chaine(error.get(ContextKind::InvalidValue));

    match error.kind() {
        ErrorKind::UnknownArgument => Some(format!("argument inattendu « {} »", argument?)),
        ErrorKind::InvalidValue => {
            let (argument, valeur) = (argument?, valeur?);
            let mut texte = if valeur.is_empty() {
                format!("« {argument} » attend une valeur, et n'en a reçu aucune")
            } else {
                format!("valeur « {valeur} » invalide pour « {argument} »")
            };
            if let Some(valides) = liste(error.get(ContextKind::ValidValue)) {
                texte.push_str(&format!("\n  [valeurs : {}]", valides.join(", ")));
            }
            Some(texte)
        }
        ErrorKind::ValueValidation => {
            let (argument, valeur) = (argument?, valeur?);
            let raison = std::error::Error::source(error)
                .map(|source| format!(" : {source}"))
                .unwrap_or_default();
            Some(format!(
                "valeur « {valeur} » invalide pour « {argument} »{raison}"
            ))
        }
        ErrorKind::TooManyValues => {
            let (argument, valeur) = (argument?, valeur?);
            Some(format!(
                "valeur inattendue « {valeur} » pour « {argument} » : aucune autre n'était attendue"
            ))
        }
        ErrorKind::InvalidSubcommand => Some(format!(
            "commande inconnue « {} »",
            chaine(error.get(ContextKind::InvalidSubcommand))?
        )),
        ErrorKind::MissingSubcommand => {
            let commande = chaine(error.get(ContextKind::InvalidSubcommand))?;
            let mut texte = format!("« {commande} » attend une commande");
            if let Some(valides) = liste(error.get(ContextKind::ValidSubcommand)) {
                texte.push_str(&format!("\n  [commandes : {}]", valides.join(", ")));
            }
            Some(texte)
        }
        ErrorKind::MissingRequiredArgument => {
            let manquants = liste(error.get(ContextKind::InvalidArg))?;
            Some(match manquants.as_slice() {
                [seul] => format!("argument obligatoire absent : {seul}"),
                plusieurs => format!("arguments obligatoires absents : {}", plusieurs.join(", ")),
            })
        }
        ErrorKind::ArgumentConflict => {
            let argument = argument?;
            Some(match liste(error.get(ContextKind::PriorArg)) {
                Some(anterieurs) if anterieurs == [argument.clone()] => {
                    format!("l'argument « {argument} » ne se donne qu'une fois")
                }
                Some(anterieurs) => format!(
                    "l'argument « {argument} » ne s'emploie pas avec {}",
                    guillemets(&anterieurs)
                ),
                None => format!(
                    "l'argument « {argument} » ne s'emploie pas avec les autres arguments donnés"
                ),
            })
        }
        _ => None,
    }
}

fn chaine(valeur: Option<&ContextValue>) -> Option<String> {
    match valeur? {
        ContextValue::String(texte) => Some(texte.clone()),
        ContextValue::StyledStr(texte) => Some(texte.to_string()),
        _ => None,
    }
}

fn liste(valeur: Option<&ContextValue>) -> Option<Vec<String>> {
    match valeur? {
        ContextValue::Strings(textes) if !textes.is_empty() => Some(textes.clone()),
        ContextValue::String(texte) => Some(vec![texte.clone()]),
        _ => None,
    }
}

fn guillemets(textes: &[String]) -> String {
    textes
        .iter()
        .map(|texte| format!("« {texte} »"))
        .collect::<Vec<_>>()
        .join(", ")
}
