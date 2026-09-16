//! Les colonnes `decimal` que SQLite ne sait pas lire.
//!
//! `generate crud` et `generate migration` refusent tous deux un champ `decimal` sur un
//! projet SQLite. Restait le chemin inverse, que rien ne gardait : un projet né sous
//! PostgreSQL, basculé vers SQLite après coup, garde ses colonnes `DECIMAL(19,4)` et son
//! modèle en `Decimal`. Le pilote ne dira rien avant l'exécution — sqlx-sqlite écarte
//! délibérément le décimal exact —, et c'est à froid que le diagnostic peut le dire.
//!
//! Le verdict est rouge, là où `guards` se contente d'orange : une écriture anonyme reste
//! un choix qu'on peut avoir fait, quand une colonne `decimal` sous SQLite n'en est pas
//! un. Le dépôt tient déjà la combinaison pour une faute en deux endroits, où elle arrête
//! la commande ; un troisième verdict qui se contenterait d'avertir les contredirait.
//!
//! Le contrôle ne s'exécute que sous SQLite : c'est `plan` qui l'y borne, sur le moteur
//! que déclare `[package.metadata.rbs]`. Un projet dont seule la feature de `sea-orm`
//! aurait basculé échappe donc à ce contrôle-ci — c'est `base` qui parle de cet écart-là,
//! en nommant le pilote compilé et l'URL visée.
//!
//! Le scan est textuel, à la ligne, et ne délimite aucun corps de `struct` : un
//! `src/*/model.rs` engendré ne porte que des entités, et un champ `Decimal` y est une
//! colonne par construction. Un modèle où l'utilisateur aurait ajouté à la main une
//! structure auxiliaire portant un `Decimal` serait signalé à tort ; le cas ne s'est pas
//! présenté, et le taire coûterait de rater les vraies colonnes.

use std::fs;
use std::path::Path;

use super::Check;

/// Ce que ce contrôle vérifie, tel qu'il paraît au rapport.
pub(crate) const TITRE: &str = "decimal";

/// Les colonnes `decimal` d'un seul fichier, dans l'ordre où elles y sont déclarées.
///
/// L'ordre de déclaration est gardé plutôt que trié : il est celui du `--fields` qui a
/// engendré l'entité, et c'est sous cette forme que le lecteur la relira.
fn colonnes(source: &str) -> Vec<String> {
    let mut trouvees: Vec<String> = Vec::new();

    for ligne in source.lines() {
        let ligne = ligne.trim();

        // Le mode d'emploi que les templates posent en tête d'un modèle nomme des types
        // sans rien déclarer : un commentaire n'est pas une colonne.
        if ligne.starts_with("//") {
            continue;
        }

        let Some(nom) = colonne(ligne) else {
            continue;
        };

        // Deux entités d'un même fichier peuvent porter le même nom de colonne : le
        // répéter n'apprendrait rien.
        if !trouvees.iter().any(|connue| connue == nom) {
            trouvees.push(nom.to_string());
        }
    }

    trouvees
}

/// Le nom de la colonne que `ligne` déclare, si c'est un champ `Decimal`.
fn colonne(ligne: &str) -> Option<&str> {
    let (nom, type_) = ligne.strip_prefix("pub ")?.split_once(':')?;

    est_decimal(type_.trim().trim_end_matches(',')).then(|| nom.trim())
}

/// Le type déclaré est-il un décimal exact, nu ou optionnel ?
///
/// Le chemin qualifié est accepté autant que le nom nu : les templates écrivent
/// `Decimal`, mais un modèle repris à la main peut porter le `sea_orm::prelude::Decimal`
/// que l'import abrège.
fn est_decimal(type_: &str) -> bool {
    let nu = type_
        .strip_prefix("Option<")
        .and_then(|reste| reste.strip_suffix('>'))
        .unwrap_or(type_)
        .trim();

    nu == "Decimal" || nu.ends_with("::Decimal")
}

/// Signale les colonnes `decimal` que le moteur du projet ne saura pas lire.
pub(crate) fn check(root: &Path) -> Check {
    let mut porteurs: Vec<(String, Vec<String>)> = Vec::new();

    if let Ok(entries) = fs::read_dir(root.join("src")) {
        for entry in entries.flatten() {
            let module = entry.file_name().to_string_lossy().into_owned();
            let file = format!("src/{module}/model.rs");

            let Ok(source) = fs::read_to_string(root.join(&file)) else {
                continue;
            };

            let colonnes = colonnes(&source);
            if !colonnes.is_empty() {
                porteurs.push((file, colonnes));
            }
        }
    }

    if porteurs.is_empty() {
        return Check::ok(TITRE, "aucune colonne `decimal`");
    }

    // L'ordre du disque n'en est pas un : deux diagnostics du même projet doivent se lire
    // pareil.
    porteurs.sort();

    let total: usize = porteurs.iter().map(|(_, colonnes)| colonnes.len()).sum();
    let accord = if total > 1 { "colonnes" } else { "colonne" };

    // Une ligne par fichier plutôt qu'une énumération d'un seul tenant : le remède se
    // porte fichier par fichier, et c'est dans cet ordre qu'il se conduit.
    let mut detail = format!("{total} {accord} `decimal` que sqlx-sqlite ne lira pas :");
    for (file, colonnes) in &porteurs {
        detail.push_str(&format!("\n{file} : {}", colonnes.join(", ")));
    }

    Check::failed(
        TITRE,
        detail,
        "sqlx-sqlite refuse délibérément de lier un décimal exact, et l'affinité NUMERIC de \
         SQLite ne garde que quinze chiffres significatifs : revenez à PostgreSQL ou MySQL — \
         feature `sqlx-postgres` ou `sqlx-mysql` de sea-orm, URL du .env et \
         `[package.metadata.rbs] database` alignés —, ou portez chaque colonne en entier de \
         centimes ou en texte, dans le modèle comme dans une migration nouvelle",
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::super::State;
    use super::*;
    use crate::database::Database;

    /// Un projet vide, sans modèle : les tests y déposent les leurs.
    ///
    /// Le moteur n'y change rien — le contrôle reçoit une racine, et c'est `plan` qui
    /// décide de l'exécuter ou non.
    fn projet() -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let root = parent.path().join("demo-api");
        fs::create_dir_all(root.join("src")).expect("racine créable");

        (parent, root)
    }

    /// Dépose un `src/<name>/model.rs` portant `source`.
    fn write_model(root: &Path, name: &str, source: &str) {
        let directory = root.join("src").join(name);
        fs::create_dir_all(&directory).expect("répertoire de feature créable");
        fs::write(directory.join("model.rs"), source).expect("modèle inscriptible");
    }

    /// Le modèle que `generate crud --fields prix:decimal` rend, réduit à ce que le
    /// contrôle lit.
    const AVEC_DECIMAL: &str = "\
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = \"produits\")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub libelle: String,
    pub prix: Decimal,
    pub created_at: DateTimeWithTimeZone,
}
";

    /// Le même, dont la colonne monétaire est un entier de centimes.
    const SANS_DECIMAL: &str = "\
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = \"produits\")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub libelle: String,
    pub prix_centimes: i64,
    pub created_at: DateTimeWithTimeZone,
}
";

    /// Le sens de la tâche : la colonne fatale se nomme, et le verdict arrête.
    #[test]
    fn a_decimal_column_is_an_error_naming_the_field_and_its_file() {
        let (_parent, root) = projet();
        write_model(&root, "produits", AVEC_DECIMAL);

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("prix"),
            "le constat doit nommer le champ : {}",
            check.detail
        );
        assert!(
            check.detail.contains("src/produits/model.rs"),
            "le constat doit nommer le fichier porteur : {}",
            check.detail
        );
    }

    /// Le remède doit nommer les deux gestes qui lèvent le problème : il n'y en a pas de
    /// bon marché, et un message qui n'en nommerait aucun laisserait le lecteur devant
    /// une colonne qu'il ne sait pas corriger.
    #[test]
    fn the_remedy_names_both_ways_out() {
        let (_parent, root) = projet();
        write_model(&root, "produits", AVEC_DECIMAL);

        let remede = check(&root).remedy.unwrap_or_default();

        assert!(
            remede.contains("PostgreSQL") && remede.contains("MySQL"),
            "le remède doit nommer le retour à un moteur qui porte le décimal : {remede}"
        );
        assert!(
            remede.contains("centimes"),
            "le remède doit nommer le portage de la colonne : {remede}"
        );
    }

    /// Une colonne optionnelle est une colonne : `Option<Decimal>` se lie aussi mal.
    #[test]
    fn an_optional_decimal_column_is_named_too() {
        let (_parent, root) = projet();
        write_model(
            &root,
            "factures",
            "pub struct Model {\n    pub remise: Option<Decimal>,\n}\n",
        );

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(
            check.detail.contains("remise"),
            "le constat doit nommer le champ optionnel : {}",
            check.detail
        );
    }

    #[test]
    fn a_model_without_any_decimal_reports_nothing() {
        let (_parent, root) = projet();
        write_model(&root, "produits", SANS_DECIMAL);

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// Un projet neuf n'a pas de feature : il est net, et le contrôle ne doit pas
    /// trébucher sur l'absence de tout modèle.
    #[test]
    fn a_project_without_any_model_reports_nothing() {
        let (_parent, root) = projet();

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// Un commentaire qui nomme un `Decimal` n'est pas une colonne : c'est le piège où
    /// `guards` était tombé, le mode d'emploi d'un fichier engendré nommant ce qu'il
    /// décrit.
    #[test]
    fn a_commented_field_is_not_a_column() {
        let (_parent, root) = projet();
        write_model(
            &root,
            "produits",
            "pub struct Model {\n    // pub prix: Decimal,\n    pub prix_centimes: i64,\n}\n",
        );

        let check = check(&root);

        assert_eq!(check.state, State::Bon, "{}", check.detail);
    }

    /// L'ordre du disque n'en est pas un : deux diagnostics du même projet doivent se
    /// lire pareil.
    #[test]
    fn the_files_are_named_in_a_stable_order() {
        let (_parent, root) = projet();
        for name in ["produits", "factures", "avoirs"] {
            write_model(&root, name, AVEC_DECIMAL);
        }

        let detail = check(&root).detail;
        let avoirs = detail.find("avoirs").expect("avoirs nommé");
        let factures = detail.find("factures").expect("factures nommé");
        let produits = detail.find("produits").expect("produits nommé");

        assert!(
            avoirs < factures && factures < produits,
            "les fichiers doivent être triés : {detail}"
        );
    }

    /// Le contrôle ne coûte rien à un projet qui ne tourne pas sous SQLite : il n'y est
    /// pas même prévu.
    #[test]
    fn only_a_sqlite_project_receives_the_check() {
        for (moteur, url, attendu) in [
            (Database::Sqlite, "sqlite://demo_api.db?mode=rwc", true),
            (
                Database::Postgres,
                "postgres://rbs:rbs@127.0.0.1:1/demo_api",
                false,
            ),
        ] {
            let (_parent, root) = crate::fixtures::Project::new()
                .database(moteur)
                .url(url)
                .create();

            let controles = super::super::plan(&super::super::manifeste(&root));
            let prevu = controles.iter().any(|controle| controle.titre == TITRE);

            assert_eq!(prevu, attendu, "moteur {moteur}");
        }
    }
}
