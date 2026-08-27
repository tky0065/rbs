use std::path::PathBuf;

use clap::{Parser, Subcommand};

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

    /// Répertoire de templates remplaçant celles embarquées dans le binaire.
    #[arg(long, global = true, value_name = "CHEMIN")]
    pub template_dir: Option<PathBuf>,

    /// Prend les valeurs par défaut sans rien demander : le CLI reste scriptable.
    #[arg(long, short = 'y', global = true)]
    pub yes: bool,
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum Commands {
    /// Crée un projet prêt à démarrer, avec sa base, ses migrations et sa route /health.
    New {
        /// Nom du projet, qui est aussi celui du répertoire créé.
        nom: String,

        /// URL de la base PostgreSQL, à défaut de quoi la question est posée.
        #[arg(long, value_name = "URL")]
        database_url: Option<String>,

        /// Features à installer sans passer par les questions, séparées par des virgules.
        #[arg(long, value_name = "FEATURES", value_delimiter = ',')]
        with: Vec<String>,

        /// Crate `rbs-core` locale à utiliser au lieu de la version publiée.
        #[arg(long, value_name = "CHEMIN")]
        core_path: Option<PathBuf>,
    },

    /// Ajoute une feature à un projet existant : auth, ci, docker, mail, redis, storage.
    Add {
        /// Feature à installer.
        feature: String,

        /// Applique les modifications même si le working tree Git est sale.
        #[arg(long)]
        force: bool,
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

    /// Diagnostique le projet : ancres, .env, base joignable, versions.
    Doctor,
}

#[derive(Debug, PartialEq, Subcommand)]
pub enum GenerateCommands {
    /// Génère une feature CRUD complète, entité et migration comprises.
    Crud {
        /// Nom de la feature, au pluriel.
        nom: String,

        /// Champs de l'entité, ex. "name:string,email:string:unique".
        #[arg(long, value_name = "CHAMPS")]
        fields: Option<String>,

        /// Écrit même si le working tree Git est sale.
        #[arg(long)]
        force: bool,

        /// Affiche le plan sans rien écrire.
        #[arg(long)]
        dry_run: bool,
    },

    /// Génère une feature vide : six fichiers, aucun champ.
    Feature {
        /// Nom de la feature.
        nom: String,

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
        nom: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn la_declaration_clap_est_coherente() {
        Cli::command().debug_assert();
    }

    #[test]
    fn le_help_liste_les_commandes_prevues_avec_une_description() {
        let commande = Cli::command();
        let help = commande.clone().render_long_help().to_string();

        for attendue in ["new", "add", "generate", "migrate", "doctor"] {
            let sous_commande = commande
                .get_subcommands()
                .find(|s| s.get_name() == attendue)
                .unwrap_or_else(|| panic!("`{attendue}` absente du CLI"));

            assert!(
                sous_commande.get_about().is_some(),
                "`{attendue}` n'a pas de description"
            );
            assert!(
                help.contains(attendue),
                "`{attendue}` absente du help :\n{help}"
            );
        }
    }

    #[test]
    fn le_help_d_add_nomme_toutes_les_features_installables() {
        // La description est écrite à la main quand la liste, elle, vient des fragments
        // embarqués : `auth` a été livrée sans que cette phrase la mentionne.
        let installables = crate::templates::Source::feature(None, "_aucune_feature_de_ce_nom_")
            .expect_err("ce nom ne doit désigner aucun fragment")
            .connues;

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
    fn le_help_de_generate_liste_crud_et_feature() {
        let help = Cli::command()
            .find_subcommand_mut("generate")
            .expect("`generate` absente du CLI")
            .render_long_help()
            .to_string();

        assert!(help.contains("crud"), "`crud` absente :\n{help}");
        assert!(help.contains("feature"), "`feature` absente :\n{help}");
    }

    #[test]
    fn l_alias_g_parse_comme_generate() {
        let court = Cli::try_parse_from(["rbs", "g", "crud", "users"]).unwrap();
        let long = Cli::try_parse_from(["rbs", "generate", "crud", "users"]).unwrap();

        assert_eq!(court, long);
    }
}
