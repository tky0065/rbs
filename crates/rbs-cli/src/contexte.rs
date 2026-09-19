//! Le contexte de rendu d'un projet existant, partagé par `add` et `remove`.
//!
//! Les deux commandes rendent les mêmes templates de fragment : la pose écrit ce rendu,
//! le retrait le rejoue pour comparer au disque ce que l'installation avait écrit. Deux
//! constructeurs recopiés l'un sur l'autre finissent par diverger, et celui du retrait
//! avait déjà perdu huit clés. Le moteur étant en [`minijinja::UndefinedBehavior::Strict`],
//! le premier fragment à en interpoler une dans un fichier non `if_absent`, dans le contenu
//! d'une ancre ou dans le `when` d'un `[[env]]` aurait fait échouer `rbs remove` chez
//! l'utilisateur, où les fragments se rendent, et non en CI, où personne ne les retire.
//!
//! `rbs new` garde le sien : il rend le squelette et non un fragment, avec des clés que
//! seul le squelette connaît — `rbs_core_dep`, `sea_orm_feature`, `compose` — et sans
//! projet d'où déduire quoi que ce soit.

use std::io;
use std::path::Path;

use minijinja::{Value, context};

use crate::database::Database;
use crate::dotenv;
use crate::migrate;

/// Ce qui peut empêcher de déduire le contexte de rendu d'un projet.
///
/// Deux variantes, et non une erreur partagée : chaque commande garde son énumération et
/// ses messages, qui nomment la commande que l'utilisateur a tapée.
#[derive(Debug)]
pub(crate) enum Erreur {
    /// Le `.env` du projet est là, mais illisible.
    Env(migrate::Error),
    /// L'URL du projet ne se décompose pas, et le rendu s'appuie dessus.
    UrlIndecomposable {
        /// L'URL, telle que le `.env` du projet la porte.
        url: String,
    },
}

/// Déduit du projet enraciné en `root` le contexte où se rendent ses fragments.
///
/// `features` est ce que le projet portera une fois le plan appliqué, et non ce que le
/// disque porte : `add` y ajoute les fragments que la même passe pose, `remove` passe la
/// liste du manifeste. Un fragment qui sait qu'un autre est là s'appuie dessus — la limite
/// de débit compte dans Redis quand le cache existe, dans sa mémoire sinon.
pub(crate) fn projet(
    root: &Path,
    nom_projet: &str,
    database: Database,
    features: Vec<String>,
) -> Result<Value, Erreur> {
    let crate_name = nom_projet.replace('-', "_");
    // `[server] lang` de `config/default.toml`, non la métadonnée : celle-ci ne gouverne
    // plus que `AGENTS.md`, et pouvait diverger de la langue des réponses HTTP avant
    // 1.5.0, quand elle se remplissait de la locale.
    let lang = crate::lang::Lang::of_project(root);

    // L'URL du projet, non une valeur par défaut : le compose qu'un fragment engendre doit
    // se connecter à la base que le projet interroge, avec ses identifiants, et un retrait
    // comparerait sinon le disque à un rendu que l'installation n'a jamais écrit. Un
    // `.env` qu'on ne sait pas ouvrir en porte peut-être d'autres, et les remplacer en
    // silence poserait un compose qui ne se connecte à rien : seule l'absence se replie,
    // parce qu'un projet neuf n'a rien encore à contredire.
    let url = match migrate::project_variables(root) {
        Ok(variables) => dotenv::value(&variables, migrate::URL).map(str::to_string),
        Err(migrate::Error::SansUrl) => None,
        Err(migrate::Error::Env(dotenv::Error::Acces(faute)))
            if faute.source.kind() == io::ErrorKind::NotFound =>
        {
            None
        }
        Err(faute) => return Err(Erreur::Env(faute)),
    }
    .unwrap_or_else(|| database.default_url(&crate_name));
    let connexion = crate::url::parse(&url);

    // Refuser plutôt que se replier : des identifiants vides posent un `POSTGRES_USER=`
    // que Compose substitue par rien, et le service `db` ne monte jamais — panne à
    // l'exécution, pour une URL que la commande avait sous les yeux. SQLite n'a pas
    // d'autorité à décomposer, et n'est donc pas concerné.
    if database.a_un_serveur() && connexion.is_none() {
        return Err(Erreur::UrlIndecomposable { url });
    }

    // Une URL sans chemin rend un nom de base vide, que le repli ne rattraperait pas s'il
    // ne guettait que `None` : le compose porterait un `POSTGRES_DB:` vide, et le service
    // ne deviendrait jamais sain.
    let nom_base = connexion
        .as_ref()
        .map(|c| c.database.clone())
        .filter(|base| !base.is_empty())
        .unwrap_or_else(|| crate_name.clone());
    let utilisateur = connexion
        .as_ref()
        .map(|c| c.user.clone())
        .unwrap_or_default();

    // Les identifiants que `.env.example` documente sont ceux de l'URL de démonstration du
    // moteur, comme le squelette les écrit : les recopier à la main dans le fragment les
    // ferait diverger de `default_url`.
    let demonstration = crate::url::parse(&database.default_url(&crate_name));

    Ok(context! {
        project_name => nom_projet,
        crate_name => crate_name.clone(),
        rust_version => crate::templates::RUST_VERSION,
        rust_image => crate::templates::rust_image(),
        features => features,
        // Par où le binaire principal atteint un module de feature : la bibliothèque du
        // projet, ou `crate::` sur un projet engendré avant qu'elle n'existe, où ces
        // modules vivent dans le binaire lui-même.
        crate_path => if root.join("src/lib.rs").exists() {
            crate_name.clone()
        } else {
            "crate".to_string()
        },
        database => database.name(),
        database_a_un_serveur => database.a_un_serveur(),
        // Le moteur décide, et non la lecture de l'URL : une URL que rien ne décompose
        // ferait sinon tomber un moteur à serveur sur `compose_url`, dont les identifiants
        // en dur partiraient dans un fichier versionné.
        database_url_compose => crate::url::interne(database, &utilisateur)
            .unwrap_or_else(|| database.compose_url(&crate_name)),
        database_url_par_defaut => database.default_url(&crate_name),
        database_user => utilisateur.clone(),
        database_password => connexion.as_ref().map(|c| c.password.clone()).unwrap_or_default(),
        database_name => nom_base,
        database_port => connexion.as_ref().map(|c| c.port).unwrap_or_default(),
        database_user_par_defaut => demonstration.as_ref().map(|c| c.user.clone()).unwrap_or_default(),
        database_password_par_defaut => demonstration.as_ref().map(|c| c.password.clone()).unwrap_or_default(),
        database_name_par_defaut => demonstration.as_ref().map(|c| c.database.clone()).unwrap_or_default(),
        lang => lang.name(),
        // L'écran de démonstration que le shell d'administration dépose. Dans le contexte
        // commun et non dans celui du seul fragment qui le lit : un fragment n'a pas de
        // contexte à lui, et c'est la même clé que `rbs generate crud` remplira pour une
        // table réelle — même template, même nom de variable, deux producteurs.
        ecran => Value::from_serialize(crate::ecran::Ecran::demonstration(lang)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::add::installation;
    use crate::manifest;
    use crate::template::Renderer;
    use crate::templates;

    /// Tout ce qu'un fragment embarqué interpole se résout dans ce contexte.
    ///
    /// Le moteur est en `UndefinedBehavior::Strict` : une clé absente n'est pas une chaîne
    /// vide, c'est un `rbs add` ou un `rbs remove` qui s'arrête chez l'utilisateur. La liste
    /// des clés à porter ne s'écrit donc pas à la main — elle se dérive des fragments, seuls
    /// à savoir ce qu'ils réclament. C'est le contrôle qui manquait quand le retrait portait
    /// sa propre copie du contexte, amputée de huit clés : rien ne les réclamait sur le
    /// chemin du retrait ce jour-là, et rien n'aurait dit que le premier à le faire
    /// échouerait.
    #[test]
    fn every_variable_an_embedded_fragment_interpolates_resolves_in_the_context() {
        let (_parent, root) = crate::fixtures::Project::new().create();
        let contexte = projet(&root, "demo-api", Database::default(), Vec::new())
            .expect("le contexte du projet de test se déduit");
        let renderer = Renderer::new();

        let rendre = |source: &str, quoi: &str| {
            renderer
                .render(source, contexte.clone())
                .unwrap_or_else(|faute| panic!("{quoi} ne se rend pas : {faute}"));
        };

        // `feature_names`, et non `feature_names_with_manifest` : sur la source embarquée,
        // la seconde rend une liste vide — mesuré — et ce garde ne parcourrait rien. Un
        // fragment sans manifeste se saute donc ici, ce qu'aucun fragment embarqué n'est.
        let mut vus = 0;
        for nom in templates::feature_names(None) {
            let source = templates::Source::feature(None, &nom).expect("le fragment s'ouvre");
            let (manifeste, fichiers) = source.manifest_and_files().expect("le fragment se lit");
            let Some(manifeste) = manifeste else {
                continue;
            };
            let manifest = manifest::read(&manifeste, &format!("{nom}/feature.toml"))
                .expect("le manifeste embarqué est valide");
            vus += 1;

            for (destination, template, _) in
                installation::a_deposer(&nom, &manifest, &fichiers).expect("les templates sont là")
            {
                rendre(template, &format!("{nom} : {destination}"));
            }

            if let Some(migration) = &manifest.migration {
                let template = installation::template(&nom, &fichiers, &migration.source)
                    .expect("la migration déclarée est portée par le fragment");
                rendre(template, &format!("{nom} : {}", migration.source));
            }

            for insertion in &manifest.anchors {
                rendre(
                    &insertion.content,
                    &format!("{nom} : ancre {}", insertion.anchor),
                );
            }

            for variable in &manifest.env {
                for valeur in [Some(&variable.value), variable.project_value.as_ref()]
                    .into_iter()
                    .flatten()
                {
                    rendre(valeur, &format!("{nom} : [[env]] {}", variable.key));
                }

                if let Some(condition) = &variable.when {
                    renderer
                        .condition(condition, contexte.clone())
                        .unwrap_or_else(|faute| {
                            panic!(
                                "{nom} : le `when` de {} ne s'évalue pas : {faute}",
                                variable.key
                            )
                        });
                }
            }
        }

        // Un garde qui ne parcourt rien passe au vert sans rien prouver : c'est exactement
        // ce qu'il a fait tant qu'il lisait `feature_names_with_manifest`.
        assert_eq!(
            vus, 16,
            "les seize fragments embarqués doivent être parcourus"
        );
    }
}
