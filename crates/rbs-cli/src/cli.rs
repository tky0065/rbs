use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::database::Database;

mod aide;
mod usage;

/// La commande `rbs`, aide comprise en français.
pub fn command() -> clap::Command {
    use clap::CommandFactory;

    aide::francise(Cli::command())
}

/// Parse les arguments du processus ; une erreur d'usage s'affiche en français et sort en 2.
pub fn parse() -> Cli {
    use clap::{CommandFactory, FromArgMatches};

    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    // `francise` construit l'arborescence, et clap y nomme chaque sous-commande d'après le
    // binaire : il doit le connaître avant, faute de quoi `rbs-cli new --help` se
    // présenterait en `rbs new`.
    let mut declaree = Cli::command();
    if let Some(nom) = args
        .first()
        .and_then(|a| std::path::Path::new(a).file_name())
    {
        declaree = declaree.bin_name(nom.to_string_lossy().into_owned());
    }

    aide::francise(declaree)
        .try_get_matches_from(args)
        .and_then(|matches| Cli::from_arg_matches(&matches))
        .unwrap_or_else(|error| usage::exit(error))
}

#[derive(Debug, PartialEq, Parser)]
#[command(
    name = "rbs",
    version,
    about = "Génère et maintient des projets Axum + SeaORM.",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum Commands {
    /// Crée un projet prêt à démarrer, avec sa base, ses migrations et sa route /health.
    New {
        /// Nom du projet, qui est aussi celui du répertoire créé, à défaut de quoi la
        /// question est posée.
        name: Option<String>,

        /// URL de connexion, à défaut de quoi la question est posée.
        #[arg(long, value_name = "URL")]
        database_url: Option<String>,

        /// Moteur de base sur lequel le projet tournera.
        #[arg(long, value_name = "MOTEUR", default_value_t = Database::default())]
        database: Database,

        /// Features à installer sans passer par les questions, séparées par des virgules.
        #[arg(long, value_name = "FEATURES", value_delimiter = ',')]
        with: Vec<String>,

        /// Jeu de features nommé, cumulable avec `--with`.
        #[arg(long, value_name = "PRESET")]
        preset: Option<crate::preset::Preset>,

        /// Crate `rbs-core` locale à utiliser au lieu de la version publiée.
        #[arg(long, value_name = "CHEMIN")]
        core_path: Option<PathBuf>,

        /// Langue du projet : `AGENTS.md` et réponses HTTP. À défaut, celle de l'environnement.
        #[arg(long, value_name = "LANGUE")]
        lang: Option<crate::lang::Lang>,

        /// Répertoire de templates remplaçant celles embarquées dans le binaire.
        #[arg(long, value_name = "CHEMIN")]
        template_dir: Option<PathBuf>,

        /// Prend les valeurs par défaut sans rien demander : le CLI reste scriptable.
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Ajoute une feature : audit, auth, ci, cors, docker, jobs, mail, observability, rate-limit, redis, scheduler, storage, webhooks.
    Add {
        /// Feature à installer.
        feature: String,

        /// Applique les modifications même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

        /// Rend le plan, ou l'erreur, en un document JSON sur la sortie standard.
        #[arg(long)]
        json: bool,

        /// Répertoire de templates remplaçant celles embarquées dans le binaire.
        #[arg(long, value_name = "CHEMIN")]
        template_dir: Option<PathBuf>,
    },

    /// Retire une feature installée : ses fichiers, ses ancres, sa migration et ses dépendances.
    Remove {
        /// Feature à retirer.
        feature: String,

        /// Retire même si un fichier a été modifié, ou si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

        /// Rend le plan, ou l'erreur, en un document JSON sur la sortie standard.
        #[arg(long)]
        json: bool,

        /// Répertoire de templates remplaçant celles embarquées dans le binaire.
        #[arg(long, value_name = "CHEMIN")]
        template_dir: Option<PathBuf>,
    },

    /// Génère une feature dans un projet existant.
    #[command(alias = "g")]
    Generate {
        #[command(subcommand)]
        command: GenerateCommands,
    },

    /// Pilote les migrations du projet.
    Migrate {
        #[command(subcommand)]
        command: MigrateCommands,
    },

    /// Insère les données de démonstration du projet.
    Seed {
        /// Insère même sous RBS_ENV=production.
        #[arg(long)]
        force: bool,
    },

    /// Démarre le projet : services, migrations, serveur relancé à chaque changement.
    Dev {
        /// Ne remonte pas les services du compose : ils tournent déjà, ou ailleurs.
        #[arg(long)]
        no_compose: bool,

        /// N'applique pas les migrations en attente.
        #[arg(long)]
        no_migrate: bool,

        /// Arguments passés au binaire du serveur après `--` ; le main engendré n'en lit aucun.
        #[arg(last = true, value_name = "ARGS")]
        server: Vec<String>,
    },

    /// Lance les tests du projet : services, migrations, puis cargo test sur tout le workspace.
    Test {
        /// Ne lance que les tests dont le chemin contient ce motif.
        #[arg(value_name = "FILTRE")]
        filtre: Option<String>,

        /// Ne remonte pas les services du compose : ils tournent déjà, ou ailleurs.
        #[arg(long)]
        no_compose: bool,

        /// N'applique pas les migrations en attente.
        #[arg(long)]
        no_migrate: bool,

        /// Arguments du harnais de test, passés après `--` (ex. --nocapture).
        #[arg(last = true, value_name = "ARGS")]
        libtest: Vec<String>,
    },

    /// Liste les routes du projet : méthode, chemin, operation_id et garde.
    Routes {
        /// Rend les routes en JSON sur la sortie standard, pour un script ou un agent.
        #[arg(long)]
        json: bool,
    },

    /// Lit le document OpenAPI du projet, sans démarrer de serveur.
    Openapi {
        #[command(subcommand)]
        command: OpenapiCommands,
    },

    /// Diagnostique le projet : ancres, .env, base joignable, versions.
    Doctor {
        /// Rend le rapport en JSON sur la sortie standard, pour un script ou une CI.
        #[arg(long)]
        json: bool,

        /// Repose les ancres absentes avant de diagnostiquer.
        #[arg(long)]
        fix: bool,

        // `requires` : seul `--fix` écrit, et hors de lui ce drapeau serait pris puis
        // ignoré — ce que `--template-dir` faisait sur les commandes qui ne le lisent pas.
        /// Repose les ancres même si le working tree Git est sale.
        #[arg(long, requires = "fix")]
        force: bool,
    },

    /// Écrit sur la sortie standard le script de complétion du shell donné.
    Completions {
        /// Shell visé.
        #[arg(value_name = "SHELL")]
        shell: clap_complete::Shell,
    },

    /// Aligne le manifeste du projet sur la version du CLI : rbs-core et les métadonnées.
    Upgrade {
        /// Met à niveau même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

        /// Rend le plan, ou l'erreur, en un document JSON sur la sortie standard.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum GenerateCommands {
    /// Génère une feature CRUD complète, entité et migration comprises.
    Crud {
        /// Nom de la feature, au pluriel.
        name: String,

        /// Champs de l'entité, ex. "name:string,status:enum(draft,published)".
        #[arg(long, value_name = "CHAMPS")]
        fields: Option<String>,

        /// Forme singulière du nom, quand l'heuristique se trompe (ex. news).
        #[arg(long, value_name = "NOM")]
        singular: Option<String>,

        /// Écrit même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

        /// Rend le plan, ou l'erreur, en un document JSON sur la sortie standard.
        #[arg(long)]
        json: bool,

        /// Entité enfant dont ce modèle doit porter la variante inverse, répétable.
        #[arg(long = "has-many", value_name = "ENTITE")]
        has_many: Vec<String>,

        /// Relève à ce rôle le seuil des écritures ; exige la feature auth.
        #[arg(long, value_name = "ROLE")]
        role: Option<String>,

        /// Rend le DELETE logique : la ligne reste, marquée d'une date de suppression.
        #[arg(long)]
        soft_delete: bool,

        /// Ajoute trois routes de contenu binaire ; exige la feature storage.
        #[arg(long)]
        with_upload: bool,

        /// Pagine GET /<ressource> par curseur ; la route de filtre garde ses pages.
        #[arg(long)]
        cursor: bool,
    },

    /// Génère une feature vide : six fichiers, aucun champ.
    Feature {
        /// Nom de la feature.
        name: String,

        /// Forme singulière du nom, quand l'heuristique se trompe (ex. news).
        #[arg(long, value_name = "NOM")]
        singular: Option<String>,

        /// Écrit même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

        /// Rend le plan, ou l'erreur, en un document JSON sur la sortie standard.
        #[arg(long)]
        json: bool,
    },

    /// Engendre un client typé depuis le document OpenAPI du projet.
    Client {
        /// Langage du client.
        #[arg(long, value_name = "LANGAGE")]
        lang: crate::client::Lang,

        /// Répertoire de sortie, relatif à la racine du projet.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,

        /// Écrit même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

        /// Rend le plan, ou l'erreur, en un document JSON sur la sortie standard.
        #[arg(long)]
        json: bool,
    },

    /// Génère un job de la file, et son échéance sous --every ; exige la feature jobs.
    Job {
        /// Nom du job, en snake_case : celui de son module et de son KIND.
        name: String,

        /// Expression cron de l'échéance, à cinq ou six champs, évaluée en UTC ; exige la feature scheduler.
        #[arg(long, value_name = "CRON")]
        every: Option<String>,

        /// Écrit même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

        /// Rend le plan, ou l'erreur, en un document JSON sur la sortie standard.
        #[arg(long)]
        json: bool,
    },

    /// Écrit une migration d'évolution : des colonnes de plus sur une table existante.
    Migration {
        /// Nom de la migration, en snake_case : celui de son module.
        name: String,

        /// Table à modifier, telle que le projet la déclare.
        #[arg(long = "add-column", value_name = "TABLE")]
        add_column: String,

        /// Colonnes à ajouter, toutes optionnelles, ex. "statut:enum(draft,published):optional".
        #[arg(long, value_name = "CHAMPS")]
        fields: String,

        /// Écrit même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

        /// Rend le plan, ou l'erreur, en un document JSON sur la sortie standard.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum MigrateCommands {
    /// Applique les migrations en attente.
    Up,

    /// Annule la dernière migration appliquée.
    Down,

    /// Affiche les migrations appliquées et celles en attente.
    Status,

    /// Crée un fichier de migration vide.
    New {
        /// Nom de la migration.
        name: String,
    },
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum OpenapiCommands {
    /// Écrit le document OpenAPI du projet sur la sortie standard, ou dans un fichier.
    Export {
        /// Fichier à écrire, relatif au répertoire courant, au lieu de la sortie standard.
        #[arg(long, value_name = "FICHIER")]
        out: Option<PathBuf>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use clap::error::ErrorKind;

    /// Ce que clap écrit en anglais dans une aide, et qu'aucune page de `rbs` ne doit
    /// garder.
    const ANGLAIS: [&str; 12] = [
        "Usage:",
        "Commands:",
        "Options:",
        "Arguments:",
        "Print help",
        "Print version",
        "Print this message",
        "see more with",
        "see a summary with",
        "[default:",
        "[possible values:",
        "Possible values:",
    ];

    fn toutes(
        commande: &clap::Command,
        chemin: &str,
        visite: &mut dyn FnMut(&str, &clap::Command),
    ) {
        visite(chemin, commande);
        for sous in commande.get_subcommands() {
            toutes(sous, &format!("{chemin} {}", sous.get_name()), visite);
        }
    }

    #[test]
    fn no_help_page_keeps_an_english_clap_string() {
        let mut racine = command();
        racine.build();
        let mut fautes = Vec::new();
        let mut pages = 0;
        toutes(&racine, "rbs", &mut |chemin, commande| {
            pages += 1;
            let mut c = commande.clone();
            for aide in [
                c.render_help().to_string(),
                c.render_long_help().to_string(),
            ] {
                for mot in ANGLAIS {
                    if aide.contains(mot) {
                        fautes.push(format!("{chemin} : {mot}"));
                    }
                }
            }
        });
        assert!(pages > 20, "le parcours n'a vu que {pages} pages");
        assert!(fautes.is_empty(), "{fautes:#?}");
    }

    #[test]
    fn a_help_page_speaks_french_with_french_typography() {
        let mut racine = command();
        let aide = racine.render_help().to_string();
        for attendu in [
            "Utilisation : rbs",
            "Commandes :",
            "Options :",
            "Affiche l'aide",
        ] {
            assert!(aide.contains(attendu), "`{attendu}` absent :\n{aide}");
        }

        let mut nouveau = command();
        let nouveau = nouveau
            .find_subcommand_mut("new")
            .expect("`new` absente du CLI");
        let courte = nouveau.render_help().to_string();
        for attendu in [
            "Arguments :",
            "[défaut : postgres]",
            "[valeurs : postgres, mysql, sqlite]",
            "plus de détail avec --help",
        ] {
            assert!(courte.contains(attendu), "`{attendu}` absent :\n{courte}");
        }
        let longue = nouveau.render_long_help().to_string();
        for attendu in [
            "Valeurs possibles :",
            "- sqlite : SQLite, sans serveur",
            "résumé avec -h",
        ] {
            assert!(longue.contains(attendu), "`{attendu}` absent :\n{longue}");
        }
    }

    /// L'erreur d'usage que `command()` rend pour `args`, et son rendu français.
    fn refus(args: &[&str]) -> (clap::Error, String) {
        let error = command()
            .try_get_matches_from(args)
            .expect_err("la commande doit être refusée");
        let texte = usage::message(&error)
            .unwrap_or_else(|| panic!("{:?} sans rendu français : {error}", error.kind()));
        (error, texte)
    }

    fn sans_anglais(texte: &str) {
        for mot in ["error:", "tip:", "For more information", "Usage:"] {
            assert!(!texte.contains(mot), "`{mot}` dans :\n{texte}");
        }
        assert!(
            texte.ends_with("Pour plus d'informations, essayez « --help »."),
            "{texte}"
        );
    }

    #[test]
    fn an_unknown_argument_is_refused_in_french() {
        let (error, texte) = refus(&["rbs", "new", "--inconnu"]);
        assert_eq!(error.exit_code(), 2);
        assert!(
            texte.contains("argument inattendu « --inconnu »"),
            "{texte}"
        );
        assert!(texte.contains("Utilisation : rbs new"), "{texte}");
        sans_anglais(&texte);
    }

    #[test]
    fn an_invalid_value_is_refused_with_the_accepted_ones() {
        let (error, texte) = refus(&["rbs", "new", "--database", "oracle"]);
        assert_eq!(error.exit_code(), 2);
        assert!(
            texte.contains("valeur « oracle » invalide pour « --database <MOTEUR> »"),
            "{texte}"
        );
        assert!(
            texte.contains("[valeurs : postgres, mysql, sqlite]"),
            "{texte}"
        );
        sans_anglais(&texte);
    }

    #[test]
    fn an_unknown_subcommand_is_refused_in_french() {
        let (error, texte) = refus(&["rbs", "nouveau"]);
        assert_eq!(error.exit_code(), 2);
        assert!(texte.contains("commande inconnue « nouveau »"), "{texte}");
        sans_anglais(&texte);
    }

    #[test]
    fn a_missing_argument_is_named_in_french() {
        let (error, texte) = refus(&["rbs", "completions"]);
        assert_eq!(error.exit_code(), 2);
        assert!(
            texte.contains("argument obligatoire absent : <SHELL>"),
            "{texte}"
        );
        sans_anglais(&texte);
    }

    #[test]
    fn help_and_version_stay_successful_outputs() {
        for args in [["rbs", "--help"], ["rbs", "-h"]] {
            let aide = command()
                .try_get_matches_from(args)
                .expect_err("l'aide interrompt");
            assert_eq!(aide.kind(), ErrorKind::DisplayHelp, "{args:?}");
            assert_eq!(aide.exit_code(), 0, "{args:?}");
        }

        let version = command()
            .try_get_matches_from(["rbs", "--version"])
            .expect_err("la version interrompt");
        assert_eq!(version.kind(), ErrorKind::DisplayVersion);
        assert_eq!(version.exit_code(), 0);
        assert_eq!(
            version.render().to_string(),
            format!("rbs {}\n", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn the_french_command_parses_like_the_declared_one() {
        let matches = command()
            .try_get_matches_from(["rbs", "dev", "--no-migrate", "--", "--port", "4000"])
            .expect("commande valide");
        let cli = <Cli as clap::FromArgMatches>::from_arg_matches(&matches).expect("Cli lisible");
        assert_eq!(
            cli.command,
            Commands::Dev {
                no_compose: false,
                no_migrate: true,
                server: vec!["--port".to_string(), "4000".to_string()],
            }
        );
    }

    #[test]
    fn the_french_command_is_consistent() {
        command().debug_assert();
    }

    #[test]
    fn the_clap_declaration_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn the_help_lists_the_planned_commands_with_a_description() {
        let command = Cli::command();
        let help = command.clone().render_long_help().to_string();

        for expected in [
            "new",
            "add",
            "generate",
            "migrate",
            "seed",
            "dev",
            "test",
            "doctor",
            "upgrade",
            "completions",
            "routes",
            "openapi",
        ] {
            let sous_commande = command
                .get_subcommands()
                .find(|s| s.get_name() == expected)
                .unwrap_or_else(|| panic!("`{expected}` absente du CLI"));

            assert!(
                sous_commande.get_about().is_some(),
                "`{expected}` n'a pas de description"
            );
            assert!(
                help.contains(expected),
                "`{expected}` absente du help :\n{help}"
            );
        }
    }

    #[test]
    fn the_add_help_names_every_installable_feature() {
        // La description est écrite à la main quand la liste, elle, vient des fragments
        // embarqués : `auth` a été livrée sans que cette phrase la mentionne.
        let installables = crate::templates::Source::feature(None, "_aucune_feature_de_ce_nom_")
            .expect_err("ce nom ne doit désigner aucun fragment")
            .known;

        let description = Cli::command()
            .find_subcommand_mut("add")
            .expect("`add` absente du CLI")
            .get_about()
            .expect("`add` n'a pas de description")
            .to_string();

        for feature in installables.split(", ") {
            assert!(
                description.contains(feature),
                "`{feature}` s'installe mais n'est pas nommée par l'aide : {description}"
            );
        }
    }

    #[test]
    fn the_generate_help_lists_its_five_subcommands() {
        let help = Cli::command()
            .find_subcommand_mut("generate")
            .expect("`generate` absente du CLI")
            .render_long_help()
            .to_string();

        for sous_commande in ["crud", "feature", "client", "job", "migration"] {
            assert!(
                help.contains(sous_commande),
                "`{sous_commande}` absente :\n{help}"
            );
        }
    }

    #[test]
    fn generate_job_reads_its_name_its_schedule_and_dry_run() {
        let cli = Cli::try_parse_from([
            "rbs",
            "generate",
            "job",
            "purge",
            "--every",
            "0 4 * * *",
            "--dry-run",
        ])
        .expect("la ligne doit être acceptée");

        assert_eq!(
            cli.command,
            Commands::Generate {
                command: GenerateCommands::Job {
                    name: "purge".to_string(),
                    every: Some("0 4 * * *".to_string()),
                    force: false,
                    dry_run: true,
                    json: false,
                },
            }
        );
    }

    #[test]
    fn routes_and_openapi_export_parse_their_flags() {
        let routes = Cli::try_parse_from(["rbs", "routes", "--json"]).expect("commande valide");
        assert_eq!(routes.command, Commands::Routes { json: true });

        let export = Cli::try_parse_from(["rbs", "openapi", "export", "--out", "doc.json"])
            .expect("commande valide");
        assert_eq!(
            export.command,
            Commands::Openapi {
                command: OpenapiCommands::Export {
                    out: Some(PathBuf::from("doc.json")),
                },
            }
        );

        let sans_fichier =
            Cli::try_parse_from(["rbs", "openapi", "export"]).expect("commande valide");
        assert_eq!(
            sans_fichier.command,
            Commands::Openapi {
                command: OpenapiCommands::Export { out: None },
            }
        );
    }

    #[test]
    fn dev_reads_its_skips_and_the_server_arguments() {
        let cli = Cli::try_parse_from([
            "rbs",
            "dev",
            "--no-compose",
            "--no-migrate",
            "--",
            "--port",
            "4000",
        ])
        .expect("commande valide");
        assert_eq!(
            cli.command,
            Commands::Dev {
                no_compose: true,
                no_migrate: true,
                server: vec!["--port".to_string(), "4000".to_string()],
            }
        );

        let nue = Cli::try_parse_from(["rbs", "dev"]).expect("commande valide");
        assert_eq!(
            nue.command,
            Commands::Dev {
                no_compose: false,
                no_migrate: false,
                server: vec![],
            }
        );
    }

    #[test]
    fn test_reads_its_skips_between_the_filter_and_the_harness_arguments() {
        let cli = Cli::try_parse_from(["rbs", "test", "--no-migrate", "user", "--", "--nocapture"])
            .expect("commande valide");
        assert_eq!(
            cli.command,
            Commands::Test {
                filtre: Some("user".to_string()),
                no_compose: false,
                no_migrate: true,
                libtest: vec!["--nocapture".to_string()],
            }
        );
    }

    #[test]
    fn the_g_alias_parses_as_generate() {
        let court = Cli::try_parse_from(["rbs", "g", "crud", "users"]).unwrap();
        let long = Cli::try_parse_from(["rbs", "generate", "crud", "users"]).unwrap();

        assert_eq!(court, long);
    }

    #[test]
    fn generate_crud_accepts_soft_delete() {
        let cli = Cli::try_parse_from(["rbs", "generate", "crud", "articles", "--soft-delete"])
            .expect("la ligne doit être acceptée");

        let Commands::Generate {
            command: GenerateCommands::Crud { soft_delete, .. },
        } = cli.command
        else {
            panic!("la sous-commande doit être `generate crud`");
        };

        assert!(soft_delete);
    }

    #[test]
    fn generate_crud_accepts_with_upload() {
        let cli = Cli::try_parse_from(["rbs", "generate", "crud", "articles", "--with-upload"])
            .expect("la ligne doit être acceptée");

        let Commands::Generate {
            command: GenerateCommands::Crud { with_upload, .. },
        } = cli.command
        else {
            panic!("la sous-commande doit être `generate crud`");
        };

        assert!(with_upload);
    }

    #[test]
    fn generate_crud_accepts_cursor() {
        let cli = Cli::try_parse_from(["rbs", "generate", "crud", "articles", "--cursor"])
            .expect("la ligne doit être acceptée");

        let Commands::Generate {
            command: GenerateCommands::Crud { cursor, .. },
        } = cli.command
        else {
            panic!("la sous-commande doit être `generate crud`");
        };

        assert!(cursor);
    }

    #[test]
    fn generate_crud_accepts_singular() {
        let cli =
            Cli::try_parse_from(["rbs", "generate", "crud", "news", "--singular", "news_item"])
                .expect("la ligne doit être acceptée");

        let Commands::Generate {
            command: GenerateCommands::Crud { singular, .. },
        } = cli.command
        else {
            panic!("la sous-commande doit être `generate crud`");
        };

        assert_eq!(singular.as_deref(), Some("news_item"));
    }

    /// `feature` passe par la même dérivation du singulier que `crud` : le flag y a le
    /// même sens.
    #[test]
    fn generate_feature_accepts_singular() {
        let cli = Cli::try_parse_from([
            "rbs",
            "generate",
            "feature",
            "news",
            "--singular",
            "news_item",
        ])
        .expect("la ligne doit être acceptée");

        let Commands::Generate {
            command: GenerateCommands::Feature { singular, .. },
        } = cli.command
        else {
            panic!("la sous-commande doit être `generate feature`");
        };

        assert_eq!(singular.as_deref(), Some("news_item"));
    }

    /// Le flag doit accepter les deux langues et rester absent par défaut : c'est cette
    /// absence qui laisse la détection décider.
    #[test]
    fn the_language_flag_accepts_both_languages_and_defaults_to_none() {
        let sans = Cli::try_parse_from(["rbs", "new", "blog"]).expect("commande valide");
        let Commands::New { lang, .. } = sans.command else {
            panic!("`new` attendue");
        };
        assert_eq!(lang, None);

        let avec =
            Cli::try_parse_from(["rbs", "new", "blog", "--lang", "en"]).expect("commande valide");
        let Commands::New { lang, .. } = avec.command else {
            panic!("`new` attendue");
        };
        assert_eq!(lang, Some(crate::lang::Lang::En));
    }

    /// Le nom absent n'est pas une faute de frappe : c'est ce qui déclenche la question.
    #[test]
    fn new_parses_without_a_name_so_the_question_can_be_asked() {
        let sans = Cli::try_parse_from(["rbs", "new"]).expect("commande valide");
        let Commands::New { name, .. } = sans.command else {
            panic!("`new` attendue");
        };
        assert_eq!(name, None);

        let avec = Cli::try_parse_from(["rbs", "new", "blog"]).expect("commande valide");
        let Commands::New { name, .. } = avec.command else {
            panic!("`new` attendue");
        };
        assert_eq!(name.as_deref(), Some("blog"));
    }

    /// Les deux commandes qui modifient un projet existant doivent pouvoir n'en montrer
    /// que le plan, comme `generate` le fait déjà.
    #[test]
    fn add_and_upgrade_accept_dry_run() {
        let ajout =
            Cli::try_parse_from(["rbs", "add", "cors", "--dry-run"]).expect("commande valide");
        let Commands::Add { dry_run, .. } = ajout.command else {
            panic!("`add` attendue");
        };
        assert!(dry_run);

        let mise_a_niveau =
            Cli::try_parse_from(["rbs", "upgrade", "--dry-run"]).expect("commande valide");
        let Commands::Upgrade { dry_run, .. } = mise_a_niveau.command else {
            panic!("`upgrade` attendue");
        };
        assert!(dry_run);
    }

    /// La commande prend les mêmes drapeaux qu'`add`.
    #[test]
    fn the_removal_takes_the_same_flags_as_the_installation() {
        let parsed =
            Cli::try_parse_from(["rbs", "remove", "mail", "--force", "--dry-run", "--json"])
                .expect("la commande se parse");

        let Commands::Remove {
            feature,
            force,
            dry_run,
            json,
            ..
        } = parsed.command
        else {
            panic!("attendu Remove");
        };
        assert_eq!(feature, "mail");
        assert!(force && dry_run && json);
    }

    #[test]
    fn an_unknown_language_is_refused_by_the_parser() {
        // Le motif du refus est asserté, et pas seulement le refus : sans lui, une faute
        // de frappe dans `new` ou dans `--lang` ferait passer le test pour la mauvaise
        // raison.
        let refus = Cli::try_parse_from(["rbs", "new", "blog", "--lang", "de"])
            .expect_err("une langue hors de la liste doit être refusée");

        assert_eq!(
            refus.kind(),
            clap::error::ErrorKind::InvalidValue,
            "{refus}"
        );
        assert!(
            refus.to_string().contains("--lang"),
            "le refus doit nommer le drapeau — {refus}"
        );
    }

    /// Le drapeau ne descend que sur les deux commandes qui le lisent. Ailleurs, clap
    /// doit le refuser : `rbs generate crud --template-dir ./mes-templates` l'acceptait
    /// et rendait le projet depuis les templates embarquées, sans un mot.
    #[test]
    fn template_dir_is_refused_by_the_commands_that_ignore_it() {
        for commande in [
            vec![
                "rbs",
                "generate",
                "crud",
                "users",
                "--template-dir",
                "/tmp/t",
            ],
            vec!["rbs", "migrate", "up", "--template-dir", "/tmp/t"],
            vec!["rbs", "seed", "--template-dir", "/tmp/t"],
            vec!["rbs", "dev", "--template-dir", "/tmp/t"],
            vec!["rbs", "doctor", "--template-dir", "/tmp/t"],
            vec!["rbs", "upgrade", "--template-dir", "/tmp/t"],
            vec!["rbs", "test", "--template-dir", "/tmp/t"],
            vec!["rbs", "routes", "--template-dir", "/tmp/t"],
            vec!["rbs", "openapi", "export", "--template-dir", "/tmp/t"],
        ] {
            // Le motif du refus est asserté, et pas seulement le refus : sans lui, une
            // faute de frappe dans le nom de la sous-commande ferait passer le test pour
            // la mauvaise raison.
            let refus = Cli::try_parse_from(&commande)
                .expect_err("le drapeau n'y ferait rien : la commande doit être refusée");

            assert_eq!(
                refus.kind(),
                clap::error::ErrorKind::UnknownArgument,
                "{commande:?} : {refus}"
            );
            assert!(
                refus.to_string().contains("--template-dir"),
                "le refus doit nommer le drapeau — {commande:?} : {refus}"
            );
        }
    }

    /// `new` rend le projet depuis ce répertoire, `add` y prend ses fragments : les deux
    /// gardent le drapeau.
    #[test]
    fn template_dir_stays_on_the_two_commands_that_honour_it() {
        let creation = Cli::try_parse_from(["rbs", "new", "blog", "--template-dir", "/tmp/t"])
            .expect("commande valide");
        let Commands::New { template_dir, .. } = creation.command else {
            panic!("`new` attendue");
        };
        assert_eq!(template_dir, Some(PathBuf::from("/tmp/t")));

        let ajout = Cli::try_parse_from(["rbs", "add", "cors", "--template-dir", "/tmp/t"])
            .expect("commande valide");
        let Commands::Add { template_dir, .. } = ajout.command else {
            panic!("`add` attendue");
        };
        assert_eq!(template_dir, Some(PathBuf::from("/tmp/t")));
    }

    /// `--force` ne lève que la garde Git de `--fix`, seul geste de `doctor` qui écrive :
    /// accepté seul, il serait pris puis ignoré.
    #[test]
    fn doctor_force_is_refused_without_fix() {
        let reparation =
            Cli::try_parse_from(["rbs", "doctor", "--fix", "--force"]).expect("commande valide");
        let Commands::Doctor { fix, force, .. } = reparation.command else {
            panic!("`doctor` attendue");
        };
        assert!(fix && force);

        // Le motif du refus est asserté, et pas seulement le refus : sans lui, une faute
        // de frappe dans `doctor` ferait passer le test pour la mauvaise raison.
        let refus = Cli::try_parse_from(["rbs", "doctor", "--force"])
            .expect_err("`--force` seul doit être refusé : rien n'écrirait");

        assert_eq!(
            refus.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        assert!(refus.to_string().contains("--fix"), "{refus}");
    }

    /// `prompts.rs` est le seul module qui pose des questions, et `rbs new` la seule
    /// commande qui l'appelle : `--yes` n'a rien à faire ailleurs.
    #[test]
    fn yes_is_accepted_only_by_new() {
        let creation =
            Cli::try_parse_from(["rbs", "new", "blog", "--yes"]).expect("commande valide");
        let Commands::New { yes, .. } = creation.command else {
            panic!("`new` attendue");
        };
        assert!(yes);

        for commande in [
            vec!["rbs", "add", "cors", "--yes"],
            vec!["rbs", "generate", "crud", "users", "--yes"],
            vec!["rbs", "migrate", "up", "--yes"],
            vec!["rbs", "seed", "--yes"],
            vec!["rbs", "dev", "--yes"],
            vec!["rbs", "doctor", "--yes"],
            vec!["rbs", "upgrade", "--yes"],
            vec!["rbs", "test", "--yes"],
            vec!["rbs", "routes", "--yes"],
            vec!["rbs", "openapi", "export", "--yes"],
        ] {
            // Le motif du refus est asserté, et pas seulement le refus : sans lui, une
            // faute de frappe dans le nom de la sous-commande ferait passer le test pour
            // la mauvaise raison.
            let refus = Cli::try_parse_from(&commande)
                .expect_err("le drapeau n'y ferait rien : la commande doit être refusée");

            assert_eq!(
                refus.kind(),
                clap::error::ErrorKind::UnknownArgument,
                "{commande:?} : {refus}"
            );
            assert!(
                refus.to_string().contains("--yes"),
                "le refus doit nommer le drapeau — {commande:?} : {refus}"
            );
        }
    }

    /// Le filtre précède `--`, les arguments du harnais de test le suivent : la même
    /// convention que `cargo test`.
    #[test]
    fn test_parses_a_filter_and_libtest_arguments() {
        let cli = Cli::try_parse_from(["rbs", "test", "articles", "--", "--nocapture"])
            .expect("commande valide");
        let Commands::Test {
            filtre, libtest, ..
        } = cli.command
        else {
            panic!("`test` attendue");
        };

        assert_eq!(filtre.as_deref(), Some("articles"));
        assert_eq!(libtest, vec!["--nocapture".to_string()]);
    }

    /// Le drapeau d'une commande qui planifie, quelle qu'elle soit.
    fn json_de(commande: &Commands) -> bool {
        match commande {
            Commands::Add { json, .. }
            | Commands::Remove { json, .. }
            | Commands::Upgrade { json, .. }
            | Commands::Generate {
                command:
                    GenerateCommands::Crud { json, .. }
                    | GenerateCommands::Feature { json, .. }
                    | GenerateCommands::Client { json, .. }
                    | GenerateCommands::Job { json, .. }
                    | GenerateCommands::Migration { json, .. },
            } => *json,
            autre => panic!("commande qui ne planifie pas : {autre:?}"),
        }
    }

    /// Les huit commandes qui planifient rendent leur plan en JSON sur demande, et
    /// seulement sur demande : sans le drapeau, le rendu humain que la documentation
    /// transcrit reste celui qui s'affiche.
    #[test]
    fn the_eight_planning_commands_accept_json_and_default_to_the_human_rendering() {
        for commande in [
            vec!["rbs", "add", "cors"],
            vec!["rbs", "remove", "cors"],
            vec!["rbs", "generate", "crud", "articles"],
            vec!["rbs", "generate", "feature", "articles"],
            vec!["rbs", "generate", "client", "--lang", "ts"],
            vec!["rbs", "generate", "job", "purge"],
            vec![
                "rbs",
                "generate",
                "migration",
                "ajoute_statut",
                "--add-column",
                "articles",
                "--fields",
                "statut:string:optional",
            ],
            vec!["rbs", "upgrade"],
        ] {
            let sans = Cli::try_parse_from(&commande)
                .unwrap_or_else(|refus| panic!("{commande:?} : {refus}"));
            assert!(
                !json_de(&sans.command),
                "{commande:?} : JSON sans le drapeau"
            );

            let avec_drapeau: Vec<&str> = commande.iter().copied().chain(["--json"]).collect();
            let avec = Cli::try_parse_from(&avec_drapeau)
                .unwrap_or_else(|refus| panic!("{avec_drapeau:?} : {refus}"));
            assert!(json_de(&avec.command), "{avec_drapeau:?} : drapeau perdu");
        }
    }
}
