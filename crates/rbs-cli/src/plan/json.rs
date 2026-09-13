//! Vue JSON d'un plan et d'une erreur.
//!
//! Sur le modèle de `doctor/json.rs` : ses propres types de sortie, pour qu'un
//! changement de représentation des types internes du plan (`Action`, `Effect`…) ne
//! rejaillisse pas furtivement sur le format qu'un agent analyse.

use serde::Serialize;

use super::{Action, Effect, File, PatchToml, Plan, Sautee, Status};

/// Le plan tel qu'un script le lit.
#[derive(Serialize)]
struct Document<'a> {
    commande: &'a str,
    racine: String,
    applique: bool,
    actions: Vec<ActionJson<'a>>,
    sautees: Vec<SauteeJson<'a>>,
    fichiers: Fichiers,
}

/// Une action, telle qu'un script la lit : son fichier, ce qu'elle y a trouvé, ce
/// qu'elle y fait.
#[derive(Serialize)]
struct ActionJson<'a> {
    chemin: &'a str,
    statut: StatutJson,
    effet: EffectJson<'a>,
}

impl<'a> From<&'a Action> for ActionJson<'a> {
    fn from(action: &'a Action) -> Self {
        ActionJson {
            chemin: &action.path,
            statut: action.statut.into(),
            effet: (&action.effet).into(),
        }
    }
}

/// `Status`, sous les trois libellés qu'un script compare.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum StatutJson {
    AFaire,
    DejaFait,
    Conflit,
}

impl From<Status> for StatutJson {
    fn from(statut: Status) -> Self {
        match statut {
            Status::AFaire => StatutJson::AFaire,
            Status::DejaFait => StatutJson::DejaFait,
            Status::Conflit => StatutJson::Conflit,
        }
    }
}

/// `Effect`, une forme par variante : c'est elle, pas le type interne, que le format
/// JSON expose — un renommage de champ côté `plan::action` ne le fait pas bouger.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EffectJson<'a> {
    Creer {
        contenu: &'a str,
    },
    Inserer {
        ancre: &'a str,
        lignes: &'a [String],
    },
    ReposerAncre {
        ancre: &'a str,
    },
    PatcherToml {
        patch: PatchTomlJson<'a>,
    },
    AjouterSection {
        section: &'a str,
        contenu: &'a str,
    },
    AjouterVariable {
        cle: &'a str,
        valeur: &'a str,
        commentaire: Option<&'a str>,
    },
    RemplacerZone {
        zone: &'a str,
        contenu: &'a str,
    },
}

impl<'a> From<&'a Effect> for EffectJson<'a> {
    fn from(effet: &'a Effect) -> Self {
        match effet {
            Effect::Creer { content } => EffectJson::Creer { contenu: content },
            Effect::Inserer { anchor, lines } => EffectJson::Inserer {
                ancre: anchor.name.as_ref(),
                lignes: lines,
            },
            Effect::ReposerAncre { anchor } => EffectJson::ReposerAncre {
                ancre: anchor.name.as_ref(),
            },
            Effect::PatcherToml { patch } => EffectJson::PatcherToml {
                patch: patch.into(),
            },
            Effect::AjouterSection { section, content } => EffectJson::AjouterSection {
                section,
                contenu: content,
            },
            Effect::AjouterVariable {
                key,
                value,
                comment,
            } => EffectJson::AjouterVariable {
                cle: key,
                valeur: value,
                commentaire: comment.as_deref(),
            },
            Effect::RemplacerZone { zone, content } => EffectJson::RemplacerZone {
                zone,
                contenu: content,
            },
        }
    }
}

/// `PatchToml`, une forme par variante — nichée sous `effet.patch` plutôt qu'aplatie
/// dans `EffectJson`, pour qu'un lecteur retrouve le même embranchement que le code.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PatchTomlJson<'a> {
    InscrireFeature {
        feature: &'a str,
    },
    AjouterDependance {
        nom: &'a str,
        version: &'a str,
        features: &'a [String],
        features_par_defaut: bool,
    },
    AjouterFeatureADependance {
        dependance: &'a str,
        feature: &'a str,
    },
    AlignerSurVersion {
        dependance: &'a str,
        version: &'a str,
    },
}

impl<'a> From<&'a PatchToml> for PatchTomlJson<'a> {
    fn from(patch: &'a PatchToml) -> Self {
        match patch {
            PatchToml::InscrireFeature(feature) => PatchTomlJson::InscrireFeature { feature },
            PatchToml::AjouterDependance(dependency) => PatchTomlJson::AjouterDependance {
                nom: &dependency.name,
                version: &dependency.version,
                features: &dependency.features,
                features_par_defaut: dependency.default_features,
            },
            PatchToml::AjouterFeatureADependance {
                dependency,
                feature,
            } => PatchTomlJson::AjouterFeatureADependance {
                dependance: dependency,
                feature,
            },
            PatchToml::AlignerSurVersion {
                dependency,
                version,
            } => PatchTomlJson::AlignerSurVersion {
                dependance: dependency,
                version,
            },
        }
    }
}

/// Une insertion sautée, telle qu'un script la lit : le fichier qui manquait, l'ancre
/// visée, et le bloc à recoller lui-même — sans forcer un second aller-retour pour le
/// reconstruire depuis l'ancre.
#[derive(Serialize)]
struct SauteeJson<'a> {
    fichier: &'a str,
    ancre: &'a str,
    bloc: String,
}

impl<'a> From<&'a Sautee> for SauteeJson<'a> {
    fn from(sautee: &'a Sautee) -> Self {
        SauteeJson {
            fichier: sautee.anchor.file.as_ref(),
            ancre: sautee.anchor.name.as_ref(),
            bloc: sautee.lines.join("\n"),
        }
    }
}

/// Compte des fichiers touchés, `deja_fait` exclus : un script qui veut le détail des
/// chemins le trouve dans `actions`, groupé par fichier dans le rendu humain.
#[derive(Serialize)]
struct Fichiers {
    crees: usize,
    modifies: usize,
}

impl From<&[File]> for Fichiers {
    fn from(files: &[File]) -> Self {
        let mut fichiers = Fichiers {
            crees: 0,
            modifies: 0,
        };

        for file in files {
            if file.statut == Status::DejaFait {
                continue;
            }

            match file.before {
                None => fichiers.crees += 1,
                Some(_) => fichiers.modifies += 1,
            }
        }

        fichiers
    }
}

/// Rend le plan en JSON, seul document de la sortie standard sous `--json`.
pub(crate) fn plan(commande: &str, plan: &Plan, applique: bool) -> String {
    let document = Document {
        commande,
        racine: plan.root().display().to_string(),
        applique,
        actions: plan.actions().iter().map(ActionJson::from).collect(),
        sautees: plan.sautees().iter().map(SauteeJson::from).collect(),
        fichiers: plan.files().into(),
    };

    // Ni carte à clés non textuelles ni flottant : la sérialisation ne peut échouer que
    // sur un défaut de programmation, qu'il vaut mieux voir tomber ici.
    serde_json::to_string_pretty(&document).expect("le plan se sérialise")
}

/// Rend une erreur en JSON, sur la sortie standard : le code de sortie reste inchangé.
pub(crate) fn erreur(
    code: &str,
    message: &str,
    remede: Option<&str>,
    bloc: Option<&str>,
) -> String {
    #[derive(Serialize)]
    struct ErreurDocument<'a> {
        erreur: ErreurJson<'a>,
    }

    #[derive(Serialize)]
    struct ErreurJson<'a> {
        code: &'a str,
        message: &'a str,
        remede: Option<&'a str>,
        bloc: Option<&'a str>,
    }

    let document = ErreurDocument {
        erreur: ErreurJson {
            code,
            message,
            remede,
            bloc,
        },
    };

    serde_json::to_string_pretty(&document).expect("l'erreur se sérialise")
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::path::PathBuf;

    use super::*;
    use crate::anchors::Anchor;
    use crate::metadata::Dependency;

    /// Une ancre minimale, pour des tests qui n'observent que son nom et son fichier.
    fn anchor(name: &'static str, file: &'static str) -> Anchor {
        Anchor {
            name: Cow::Borrowed(name),
            file: Cow::Borrowed(file),
            comment: "//",
            sorted: false,
            optional: false,
            after: "",
        }
    }

    /// Un plan à une seule action, pour isoler le rendu d'un effet ou d'un statut.
    fn minimal_plan(effet: Effect, statut: Status) -> Plan {
        Plan {
            root: PathBuf::from("/projet"),
            actions: vec![Action {
                path: "src/fichier.rs".to_string(),
                effet,
                statut,
            }],
            files: Vec::new(),
            sautees: Vec::new(),
        }
    }

    /// Le document, analysé comme un script l'analyserait.
    fn document(plan: &Plan) -> serde_json::Value {
        let rendu = super::plan("add", plan, false);

        serde_json::from_str(&rendu)
            .unwrap_or_else(|faute| panic!("le rendu doit être un JSON valide ({faute}) : {rendu}"))
    }

    #[test]
    fn the_creer_effect_carries_its_full_content() {
        let plan = minimal_plan(
            Effect::Creer {
                content: "fn main() {}\n".to_string(),
            },
            Status::AFaire,
        );
        let effet = &document(&plan)["actions"][0]["effet"];

        assert_eq!(effet["type"], "creer");
        assert_eq!(effet["contenu"], "fn main() {}\n");
    }

    #[test]
    fn the_inserer_effect_carries_the_anchor_name_and_lines() {
        let plan = minimal_plan(
            Effect::Inserer {
                anchor: anchor("layers", "src/router.rs"),
                lines: vec!["    .layer(cors)".to_string()],
            },
            Status::AFaire,
        );
        let effet = &document(&plan)["actions"][0]["effet"];

        assert_eq!(effet["type"], "inserer");
        assert_eq!(effet["ancre"], "layers");
        assert_eq!(effet["lignes"][0], "    .layer(cors)");
    }

    #[test]
    fn the_reposer_ancre_effect_carries_the_anchor_name() {
        let plan = minimal_plan(
            Effect::ReposerAncre {
                anchor: anchor("routes", "src/router.rs"),
            },
            Status::AFaire,
        );
        let effet = &document(&plan)["actions"][0]["effet"];

        assert_eq!(effet["type"], "reposer_ancre");
        assert_eq!(effet["ancre"], "routes");
    }

    #[test]
    fn the_inscrire_feature_patch_carries_its_feature_name() {
        let plan = minimal_plan(
            Effect::PatcherToml {
                patch: PatchToml::InscrireFeature("cors".to_string()),
            },
            Status::AFaire,
        );
        let effet = &document(&plan)["actions"][0]["effet"];

        assert_eq!(effet["type"], "patcher_toml");
        assert_eq!(effet["patch"]["type"], "inscrire_feature");
        assert_eq!(effet["patch"]["feature"], "cors");
    }

    #[test]
    fn the_ajouter_dependance_patch_carries_the_dependency_fields() {
        let plan = minimal_plan(
            Effect::PatcherToml {
                patch: PatchToml::AjouterDependance(Dependency {
                    name: "tower-http".to_string(),
                    version: "0.6".to_string(),
                    features: vec!["cors".to_string()],
                    default_features: false,
                }),
            },
            Status::AFaire,
        );
        let patch = &document(&plan)["actions"][0]["effet"]["patch"];

        assert_eq!(patch["type"], "ajouter_dependance");
        assert_eq!(patch["nom"], "tower-http");
        assert_eq!(patch["version"], "0.6");
        assert_eq!(patch["features"][0], "cors");
        assert_eq!(patch["features_par_defaut"], false);
    }

    #[test]
    fn the_ajouter_feature_a_dependance_patch_carries_the_dependency_and_feature() {
        let plan = minimal_plan(
            Effect::PatcherToml {
                patch: PatchToml::AjouterFeatureADependance {
                    dependency: "tokio".to_string(),
                    feature: "macros".to_string(),
                },
            },
            Status::AFaire,
        );
        let patch = &document(&plan)["actions"][0]["effet"]["patch"];

        assert_eq!(patch["type"], "ajouter_feature_a_dependance");
        assert_eq!(patch["dependance"], "tokio");
        assert_eq!(patch["feature"], "macros");
    }

    #[test]
    fn the_aligner_sur_version_patch_carries_the_dependency_and_version() {
        let plan = minimal_plan(
            Effect::PatcherToml {
                patch: PatchToml::AlignerSurVersion {
                    dependency: "rbs-core".to_string(),
                    version: "1.5.0".to_string(),
                },
            },
            Status::AFaire,
        );
        let patch = &document(&plan)["actions"][0]["effet"]["patch"];

        assert_eq!(patch["type"], "aligner_sur_version");
        assert_eq!(patch["dependance"], "rbs-core");
        assert_eq!(patch["version"], "1.5.0");
    }

    #[test]
    fn the_ajouter_section_effect_carries_the_section_name_and_content() {
        let plan = minimal_plan(
            Effect::AjouterSection {
                section: "storage".to_string(),
                content: "driver = \"s3\"\n".to_string(),
            },
            Status::AFaire,
        );
        let effet = &document(&plan)["actions"][0]["effet"];

        assert_eq!(effet["type"], "ajouter_section");
        assert_eq!(effet["section"], "storage");
        assert_eq!(effet["contenu"], "driver = \"s3\"\n");
    }

    #[test]
    fn the_ajouter_variable_effect_carries_its_comment_when_present() {
        let plan = minimal_plan(
            Effect::AjouterVariable {
                key: "REDIS_URL".to_string(),
                value: "redis://localhost:6379".to_string(),
                comment: Some("URL du serveur Redis".to_string()),
            },
            Status::AFaire,
        );
        let effet = &document(&plan)["actions"][0]["effet"];

        assert_eq!(effet["type"], "ajouter_variable");
        assert_eq!(effet["cle"], "REDIS_URL");
        assert_eq!(effet["valeur"], "redis://localhost:6379");
        assert_eq!(effet["commentaire"], "URL du serveur Redis");
    }

    /// Un commentaire absent se rend en `null`, et non en champ omis : contrairement à
    /// `doctor --json`, un lecteur de la vue du plan ne doit jamais filtrer une clé.
    #[test]
    fn the_ajouter_variable_effect_renders_a_missing_comment_as_null() {
        let plan = minimal_plan(
            Effect::AjouterVariable {
                key: "REDIS_URL".to_string(),
                value: "redis://localhost:6379".to_string(),
                comment: None,
            },
            Status::AFaire,
        );
        let effet = &document(&plan)["actions"][0]["effet"];

        assert!(effet.get("commentaire").is_some(), "{effet}");
        assert!(effet["commentaire"].is_null(), "{effet}");
    }

    #[test]
    fn the_remplacer_zone_effect_carries_the_zone_name_and_content() {
        let plan = minimal_plan(
            Effect::RemplacerZone {
                zone: "guide".to_string(),
                content: "# Guide\n".to_string(),
            },
            Status::AFaire,
        );
        let effet = &document(&plan)["actions"][0]["effet"];

        assert_eq!(effet["type"], "remplacer_zone");
        assert_eq!(effet["zone"], "guide");
        assert_eq!(effet["contenu"], "# Guide\n");
    }

    #[test]
    fn the_three_statuses_render_distinct_labels() {
        let statuts: Vec<serde_json::Value> = [Status::AFaire, Status::DejaFait, Status::Conflit]
            .into_iter()
            .map(|statut| {
                let plan = minimal_plan(
                    Effect::Creer {
                        content: String::new(),
                    },
                    statut,
                );
                document(&plan)["actions"][0]["statut"].clone()
            })
            .collect();

        assert_eq!(statuts, vec!["a_faire", "deja_fait", "conflit"]);
    }

    #[test]
    fn a_skipped_insertion_carries_its_file_anchor_and_joined_lines() {
        let plan = Plan {
            root: PathBuf::from("/projet"),
            actions: Vec::new(),
            files: Vec::new(),
            sautees: vec![Sautee {
                anchor: anchor("services", "docker-compose.yml"),
                lines: vec!["  redis:".to_string(), "    image: redis:7".to_string()],
            }],
        };
        let sautee = &document(&plan)["sautees"][0];

        assert_eq!(sautee["fichier"], "docker-compose.yml");
        assert_eq!(sautee["ancre"], "services");
        assert_eq!(sautee["bloc"], "  redis:\n    image: redis:7");
    }

    #[test]
    fn fichiers_counts_creations_and_modifications_and_excludes_untouched_files() {
        let plan = Plan {
            root: PathBuf::from("/projet"),
            actions: Vec::new(),
            files: vec![
                File {
                    path: "src/nouveau.rs".to_string(),
                    before: None,
                    after: "// nouveau\n".to_string(),
                    statut: Status::AFaire,
                },
                File {
                    path: "src/router.rs".to_string(),
                    before: Some("ancien".to_string()),
                    after: "nouveau".to_string(),
                    statut: Status::AFaire,
                },
                File {
                    path: "Cargo.toml".to_string(),
                    before: Some("inchangé".to_string()),
                    after: "inchangé".to_string(),
                    statut: Status::DejaFait,
                },
            ],
            sautees: Vec::new(),
        };
        let fichiers = &document(&plan)["fichiers"];

        assert_eq!(fichiers["crees"], 1);
        assert_eq!(fichiers["modifies"], 1);
    }

    #[test]
    fn racine_carries_the_plan_root_as_displayed() {
        let plan = minimal_plan(
            Effect::Creer {
                content: String::new(),
            },
            Status::AFaire,
        );

        assert_eq!(document(&plan)["racine"], "/projet");
    }

    #[test]
    fn commande_is_carried_through_unchanged() {
        let plan = minimal_plan(
            Effect::Creer {
                content: String::new(),
            },
            Status::AFaire,
        );

        assert_eq!(document(&plan)["commande"], "add");
    }

    #[test]
    fn applique_reflects_whether_the_plan_was_written() {
        let plan = minimal_plan(
            Effect::Creer {
                content: String::new(),
            },
            Status::AFaire,
        );
        let brouillon: serde_json::Value =
            serde_json::from_str(&super::plan("add", &plan, false)).expect("JSON valide");
        let ecrit: serde_json::Value =
            serde_json::from_str(&super::plan("add", &plan, true)).expect("JSON valide");

        assert_eq!(brouillon["applique"], false);
        assert_eq!(ecrit["applique"], true);
    }

    #[test]
    fn an_error_with_a_remedy_and_a_block_carries_both() {
        let rendu = super::erreur(
            "ancre_absente",
            "l'ancre `layers` est absente",
            Some("recollez le bloc affiché"),
            Some("// <rbs:layers>\n// </rbs:layers>"),
        );
        let document: serde_json::Value = serde_json::from_str(&rendu).expect("JSON valide");

        assert_eq!(document["erreur"]["code"], "ancre_absente");
        assert_eq!(
            document["erreur"]["message"],
            "l'ancre `layers` est absente"
        );
        assert_eq!(document["erreur"]["remede"], "recollez le bloc affiché");
        assert_eq!(
            document["erreur"]["bloc"],
            "// <rbs:layers>\n// </rbs:layers>"
        );
    }

    /// `remede` et `bloc` restent des clés présentes, valant `null` : la forme du
    /// document ne varie pas selon ce que l'erreur sait dire.
    #[test]
    fn an_error_without_a_remedy_or_block_renders_them_as_null() {
        let rendu = super::erreur("arbre_sale", "l'arbre git n'est pas propre", None, None);
        let document: serde_json::Value = serde_json::from_str(&rendu).expect("JSON valide");

        assert!(document["erreur"].get("remede").is_some(), "{document}");
        assert!(document["erreur"]["remede"].is_null(), "{document}");
        assert!(document["erreur"].get("bloc").is_some(), "{document}");
        assert!(document["erreur"]["bloc"].is_null(), "{document}");
    }
}
