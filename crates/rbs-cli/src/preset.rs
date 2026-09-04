//! Les presets de `rbs new` : une liste de features sous un nom.

/// Un jeu de features nommé, proposé par `rbs new --preset`.
///
/// Sans rapport avec `--with`, qui nomme les features une à une : un preset dit une
/// intention — servir une API, faire tourner un worker — et les deux se cumulent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Preset {
    /// Une API exposée : authentification, CORS, limite de débit, conteneur.
    Api,
    /// Un service de traitement : file, calendrier, cache, conteneur.
    Worker,
    /// Tout ce que le CLI sait installer.
    Full,
}

impl Preset {
    /// Les features du preset, parmi celles que le CLI sait installer.
    ///
    /// `Full` est dérivé de `disponibles` plutôt qu'écrit en dur : une liste figée ici
    /// périmerait au premier fragment ajouté, et « tout » cesserait de vouloir dire tout
    /// sans que rien ne le signale.
    ///
    /// Les deux autres sont nommés, et une feature qu'ils citent sans qu'elle existe est
    /// simplement ignorée — un `--template-dir` peut ne porter qu'une partie des
    /// fragments, et refuser le preset y serait un refus sans remède.
    pub(crate) fn features(self, disponibles: &[String]) -> Vec<String> {
        let nommees: &[&str] = match self {
            // `auth` entraîne déjà `rate-limit` ; le citer dit ce que le preset garantit,
            // plutôt que de le laisser dépendre d'une implication qui pourrait changer.
            Preset::Api => &["auth", "cors", "docker", "rate-limit"],
            Preset::Worker => &["docker", "jobs", "redis", "scheduler"],
            Preset::Full => return disponibles.to_vec(),
        };

        disponibles
            .iter()
            .filter(|feature| nommees.contains(&feature.as_str()))
            .cloned()
            .collect()
    }
}

/// Réunit le preset et les features nommées à la main, sans doublon.
///
/// L'ordre est celui de `disponibles`, comme pour `--with` : deux invocations équivalentes
/// doivent rendre deux projets identiques, et l'ordre d'installation décide de l'ordre des
/// insertions dans les ancres.
pub(crate) fn reunir(
    preset: Option<Preset>,
    with: &[String],
    disponibles: &[String],
) -> Vec<String> {
    let du_preset = preset
        .map(|preset| preset.features(disponibles))
        .unwrap_or_default();

    disponibles
        .iter()
        .filter(|feature| du_preset.contains(feature) || with.contains(feature))
        .cloned()
        // Une feature de `--with` que le CLI ne connaît pas doit rester dans la liste :
        // c'est `new::validate_features` qui la refuse, en la nommant.
        .chain(
            with.iter()
                .filter(|feature| !disponibles.contains(feature))
                .cloned(),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disponibles() -> Vec<String> {
        [
            "audit",
            "auth",
            "ci",
            "cors",
            "docker",
            "jobs",
            "mail",
            "observability",
            "rate-limit",
            "redis",
            "scheduler",
            "storage",
            "webhooks",
        ]
        .iter()
        .map(|nom| nom.to_string())
        .collect()
    }

    #[test]
    fn the_api_preset_serves_an_exposed_api() {
        assert_eq!(
            Preset::Api.features(&disponibles()),
            ["auth", "cors", "docker", "rate-limit"]
        );
    }

    #[test]
    fn the_worker_preset_runs_work_rather_than_serving_it() {
        assert_eq!(
            Preset::Worker.features(&disponibles()),
            ["docker", "jobs", "redis", "scheduler"]
        );
    }

    /// `full` se dérive, et c'est ce qui le garde vrai : un fragment ajouté au CLI y entre
    /// sans que personne n'y pense.
    #[test]
    fn the_full_preset_is_everything_the_cli_can_install() {
        assert_eq!(Preset::Full.features(&disponibles()), disponibles());
    }

    #[test]
    fn a_preset_and_named_features_add_up_without_repeating_one() {
        let features = reunir(
            Some(Preset::Api),
            &["mail".to_string(), "auth".to_string()],
            &disponibles(),
        );

        assert_eq!(features, ["auth", "cors", "docker", "mail", "rate-limit"]);
    }

    /// L'ordre est celui des features disponibles, non celui de la frappe : deux
    /// invocations équivalentes doivent rendre deux projets identiques.
    #[test]
    fn the_order_is_the_one_of_the_available_features() {
        let features = reunir(
            None,
            &[
                "storage".to_string(),
                "auth".to_string(),
                "cors".to_string(),
            ],
            &disponibles(),
        );

        assert_eq!(features, ["auth", "cors", "storage"]);
    }

    /// Une feature inconnue survit à la réunion : c'est `new` qui la refuse, en la
    /// nommant, et l'écarter ici rendrait ce refus muet.
    #[test]
    fn an_unknown_feature_reaches_the_refusal_that_names_it() {
        let features = reunir(None, &["graphql".to_string()], &disponibles());

        assert_eq!(features, ["graphql"]);
    }

    /// Sans preset ni `--with`, la liste est vide et la question reste ouverte : c'est
    /// elle qui distingue « aucune feature » de « on ne m'a rien dit ».
    #[test]
    fn nothing_asked_leaves_nothing_chosen() {
        assert!(reunir(None, &[], &disponibles()).is_empty());
    }
}
