use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::database::Database;

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

        /// Crate `rbs-core` locale à utiliser au lieu de la version publiée.
        #[arg(long, value_name = "CHEMIN")]
        core_path: Option<PathBuf>,

        /// Langue de l'`AGENTS.md` engendré. À défaut, celle de l'environnement.
        #[arg(long, value_name = "LANGUE")]
        lang: Option<crate::lang::Lang>,

        /// Répertoire de templates remplaçant celles embarquées dans le binaire.
        #[arg(long, value_name = "CHEMIN")]
        template_dir: Option<PathBuf>,

        /// Prend les valeurs par défaut sans rien demander : le CLI reste scriptable.
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Ajoute une feature : audit, auth, ci, cors, docker, jobs, mail, observability, rate-limit, redis, scheduler, storage.
    Add {
        /// Feature à installer.
        feature: String,

        /// Applique les modifications même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

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
    Dev,

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
    },
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum GenerateCommands {
    /// Génère une feature CRUD complète, entité et migration comprises.
    Crud {
        /// Nom de la feature, au pluriel.
        name: String,

        /// Champs de l'entité, ex. "name:string,email:string:unique".
        #[arg(long, value_name = "CHAMPS")]
        fields: Option<String>,

        /// Écrit même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,

        /// Entité enfant dont ce modèle doit porter la variante inverse, répétable.
        #[arg(long = "has-many", value_name = "ENTITE")]
        has_many: Vec<String>,

        /// Réserve les écritures à ce rôle ; exige la feature auth.
        #[arg(long, value_name = "ROLE")]
        role: Option<String>,

        /// Rend le DELETE logique : la ligne reste, marquée d'une date de suppression.
        #[arg(long)]
        soft_delete: bool,

        /// Ajoute trois routes de contenu binaire ; exige la feature storage.
        #[arg(long)]
        with_upload: bool,
    },

    /// Génère une feature vide : six fichiers, aucun champ.
    Feature {
        /// Nom de la feature.
        name: String,

        /// Écrit même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

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
            "doctor",
            "upgrade",
            "completions",
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
    fn the_generate_help_lists_crud_and_feature() {
        let help = Cli::command()
            .find_subcommand_mut("generate")
            .expect("`generate` absente du CLI")
            .render_long_help()
            .to_string();

        assert!(help.contains("crud"), "`crud` absente :\n{help}");
        assert!(help.contains("feature"), "`feature` absente :\n{help}");
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
}
