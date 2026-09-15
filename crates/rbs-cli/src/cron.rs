//! Les expressions cron du fragment `scheduler`, jugées comme le démarrage les jugera.
//!
//! Le CLI lit les expressions qu'un projet engendré déclare — `rbs doctor` dans son
//! calendrier, `rbs generate job` sur sa ligne de commande — et doit rendre le verdict que
//! le projet rendra en démarrant. Il emploie donc la crate `cron` sous la requête de version
//! que le fragment déclare, et la normalisation de sa fonction `normaliser`, à l'identique :
//! un désaccord ferait accepter par l'un ce que l'autre refuse, et un `doctor` vert
//! annoncerait sain un projet qui ne démarre pas.

use std::str::FromStr;

/// Ce qui rend une expression cron illisible.
///
/// Le message du nombre de champs est celui du fragment, mot pour mot : c'est la même
/// expression, et l'utilisateur ne doit pas en lire deux descriptions. Le refus de la crate,
/// lui, est réduit à sa dernière ligne — les deux premières ne font que répéter
/// l'expression, et un rapport n'écrit qu'une ligne par constat.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Erreur {
    /// Ni cinq champs ni six.
    #[error(
        "`{expression}` porte {champs} champ(s) : une expression cron en compte cinq \
         (minute heure jour mois jour-de-semaine) ou six, la seconde en tête"
    )]
    NombreDeChamps {
        /// L'expression, telle qu'elle a été écrite.
        expression: String,
        /// Le nombre de champs qu'elle porte.
        champs: usize,
    },
    /// Le bon nombre de champs, mais une valeur que la crate refuse.
    #[error("`{expression}` : {}", explication(.cause))]
    Illisible {
        /// L'expression, telle qu'elle a été écrite.
        expression: String,
        /// Le refus de la crate, tel qu'elle le rend.
        cause: String,
    },
}

/// Ce que la crate `cron` dit d'une expression qu'elle refuse, sur une ligne.
///
/// Elle rend trois lignes — la forme soumise, un curseur sous la faute, puis l'explication —
/// quand un rapport n'en écrit qu'une par constat. La première n'est qu'un écho de ce qu'on
/// lui a soumis : c'est la dernière qui dit ce qui ne va pas.
fn explication(cause: &str) -> &str {
    cause
        .lines()
        .map(str::trim)
        .rfind(|ligne| !ligne.is_empty())
        .unwrap_or(cause)
}

/// Valide `expression` et la rend sous la forme que la crate `cron` attend.
///
/// Le crontab Unix compte cinq champs et ne dit rien des secondes : ils valent « à la
/// seconde zéro », que la forme rendue porte en tête. Six champs passent tels quels.
pub(crate) fn valider(expression: &str) -> Result<String, Erreur> {
    let normalisee = match expression.split_whitespace().count() {
        5 => format!("0 {expression}"),
        6 => expression.to_string(),
        champs => {
            return Err(Erreur::NombreDeChamps {
                expression: expression.to_string(),
                champs,
            });
        }
    };

    // `::cron` : ce module porte le nom de la crate.
    ::cron::Schedule::from_str(&normalisee).map_err(|cause| Erreur::Illisible {
        expression: expression.to_string(),
        cause: cause.to_string(),
    })?;

    Ok(normalisee)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un fichier du fragment `scheduler`, lu depuis les sources de la crate.
    fn fragment(fichier: &str) -> String {
        let chemin = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/features/scheduler/");
        std::fs::read_to_string(format!("{chemin}{fichier}"))
            .unwrap_or_else(|faute| panic!("{fichier} illisible : {faute}"))
    }

    #[test]
    fn five_fields_gain_the_zero_second_the_crate_expects() {
        assert_eq!(valider("0 3 * * *").as_deref(), Ok("0 0 3 * * *"));
    }

    #[test]
    fn six_fields_pass_through() {
        assert_eq!(valider("0 0 3 * * *").as_deref(), Ok("0 0 3 * * *"));
    }

    #[test]
    fn another_field_count_is_refused_naming_the_expression() {
        let erreur = valider("0 3 * *").expect_err("quatre champs ne veulent rien dire");

        assert!(
            matches!(erreur, Erreur::NombreDeChamps { champs: 4, .. }),
            "{erreur:?}"
        );
        assert!(erreur.to_string().contains("`0 3 * *`"), "{erreur}");
    }

    #[test]
    fn an_out_of_range_value_is_refused_by_the_crate() {
        let erreur = valider("0 99 * * *").expect_err("99 n'est pas une heure");

        assert!(matches!(erreur, Erreur::Illisible { .. }), "{erreur:?}");
        assert!(erreur.to_string().contains("`0 99 * * *`"), "{erreur}");
    }

    /// La crate rend trois lignes — la forme soumise, un curseur sous la faute, puis
    /// l'explication — quand un rapport n'en écrit qu'une par constat : c'est l'explication
    /// qui y va, et non l'écho de la forme soumise.
    #[test]
    fn a_refusal_keeps_the_explanation_of_the_crate_on_one_line() {
        for expression in ["0 99 * * *", "0 0 99 * * *"] {
            let message = valider(expression)
                .expect_err("99 n'est pas une heure")
                .to_string();

            assert!(
                message.starts_with(&format!("`{expression}` : ")),
                "{message}"
            );
            assert!(
                message.contains("'99'"),
                "l'explication doit nommer la valeur fautive : {message}"
            );
            assert!(!message.contains('\n'), "{message}");
        }
    }

    /// Le refus d'un mauvais nombre de champs dit ce que dit le démarrage : le message est
    /// lu dans la template, sa continuation de ligne recollée.
    #[test]
    fn the_field_count_message_is_the_one_the_fragment_prints() {
        let source = fragment("mod.rs.jinja");
        let apres = source
            .split("autre => anyhow::bail!(")
            .nth(1)
            .expect("le fragment refuse par `anyhow::bail!`");
        let litteral = apres.split('"').nth(1).expect("le message est un littéral");

        // Une chaîne coupée par `\` en fin de ligne reprend au premier caractère non blanc
        // de la ligne suivante.
        let message: String = litteral
            .split("\\\n")
            .enumerate()
            .map(|(rang, morceau)| {
                if rang == 0 {
                    morceau
                } else {
                    morceau.trim_start()
                }
            })
            .collect();
        let attendu = message
            .replace("{expression}", "0 3 * *")
            .replace("{autre}", "4");

        assert_eq!(
            valider("0 3 * *")
                .expect_err("quatre champs ne veulent rien dire")
                .to_string(),
            attendu
        );
    }

    /// Ce que les tests du fragment tiennent pour valide l'est ici, et réciproquement : un
    /// désaccord ferait dire au CLI l'inverse du démarrage.
    #[test]
    fn the_expressions_the_fragment_tests_judge_are_judged_alike() {
        let source = fragment("tests/expression.rs.jinja");
        let mut jugees = 0;

        for morceau in source.split("normaliser(\"").skip(1) {
            let Some((expression, suite)) = morceau.split_once("\")") else {
                continue;
            };

            if suite.starts_with(".expect_err(") {
                assert!(
                    valider(expression).is_err(),
                    "le fragment refuse `{expression}`, le CLI l'accepte"
                );
            } else if suite.starts_with(".expect(") {
                assert!(
                    valider(expression).is_ok(),
                    "le fragment accepte `{expression}`, le CLI la refuse"
                );
            } else {
                continue;
            }

            jugees += 1;
        }

        assert!(
            jugees >= 4,
            "{jugees} expression(s) jugée(s) seulement : le test ne croise plus rien"
        );
    }

    /// Même crate, même requête de version : la déclaration du fragment et celle du CLI
    /// ne peuvent pas diverger sans que ce test le dise.
    #[test]
    fn the_cli_parses_with_the_version_the_fragment_declares() {
        let manifeste: toml_edit::DocumentMut = fragment("feature.toml")
            .parse()
            .expect("le manifeste du fragment s'analyse");
        let du_fragment = manifeste
            .get("dependencies")
            .and_then(toml_edit::Item::as_array_of_tables)
            .and_then(|dependances| {
                dependances
                    .iter()
                    .find(|dependance| {
                        dependance.get("name").and_then(toml_edit::Item::as_str) == Some("cron")
                    })
                    .and_then(|dependance| dependance.get("version"))
                    .and_then(toml_edit::Item::as_str)
                    .map(str::to_owned)
            })
            .expect("le fragment déclare la crate cron");

        let racine: toml_edit::DocumentMut =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"))
                .expect("le manifeste racine se lit")
                .parse()
                .expect("le manifeste racine s'analyse");
        let du_cli = racine
            .get("workspace")
            .and_then(|espace| espace.get("dependencies"))
            .and_then(|dependances| dependances.get("cron"))
            .and_then(toml_edit::Item::as_str)
            .expect("le workspace déclare la crate cron");

        assert_eq!(du_cli, du_fragment);
    }
}
