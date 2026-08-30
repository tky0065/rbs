//! Inventaire des entités SeaORM d'un projet, lu sur le disque.
//!
//! Le scan est textuel, non un parseur Rust : un modèle lourdement réécrit le fera
//! échouer en refusant une cible, jamais en écrivant une relation fausse.

use std::fs;
use std::path::Path;

/// Une entité trouvée dans le projet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entity {
    /// Nom de la table, tel que `table_name` le déclare : `users`.
    pub table: String,
    /// Chemin du module portant l'entité : `crate::auth::model::user`.
    pub module_path: String,
    /// Fichier porteur, relatif à la racine du projet : `src/auth/model.rs`.
    pub file: String,
}

/// Parcourt `src/*/model.rs` et relève toute entité déclarée.
pub(crate) fn scan(root: &Path) -> Vec<Entity> {
    let mut found = Vec::new();

    let Ok(entries) = fs::read_dir(root.join("src")) else {
        return found;
    };

    let mut modules: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    // L'ordre de `read_dir` dépend du système de fichiers : sans tri, le message
    // nommant les entités connues changerait d'une machine à l'autre.
    modules.sort();

    for module in modules {
        let file = format!("src/{module}/model.rs");
        let Ok(source) = fs::read_to_string(root.join(&file)) else {
            continue;
        };
        collect(
            &source,
            &format!("crate::{module}::model"),
            &file,
            &mut found,
        );
    }

    found
}

/// Relève les entités d'un seul fichier, en suivant ses modules imbriqués.
///
/// Le suivi est indispensable : la table `users` d'un projet authentifié vit sous
/// `src/auth/model.rs`, dans `pub mod user`, et non dans un `src/users/`.
///
/// La profondeur d'accolades est comptée sur tout le fichier, pas seulement sur les
/// lignes `pub mod` : une `struct` a elle aussi une accolade fermante en début de
/// ligne, et la confondre avec la fermeture du module rattacherait la première entité
/// suivante au mauvais chemin.
fn collect(source: &str, module_path: &str, file: &str, found: &mut Vec<Entity>) {
    let mut current = module_path.to_string();
    // Profondeur à laquelle se referme le module courant ; `None` tant qu'on est à la
    // racine du fichier.
    let mut closes_at: Option<usize> = None;
    let mut depth: usize = 0;

    for line in source.lines() {
        let trimmed = line.trim();

        if closes_at.is_none() {
            if let Some(rest) = strip_module_declaration(trimmed) {
                if let Some(name) = rest.split(['{', ';', ' ']).next().filter(|n| !n.is_empty()) {
                    current = format!("{module_path}::{name}");
                    closes_at = Some(depth);
                }
            }
        }

        if let Some(table) = table_name(trimmed) {
            found.push(Entity {
                table,
                module_path: current.clone(),
                file: file.to_string(),
            });
        }

        depth += line.matches('{').count();
        depth = depth.saturating_sub(line.matches('}').count());

        if closes_at == Some(depth) {
            current = module_path.to_string();
            closes_at = None;
        }
    }
}

/// Reconnaît `mod nom`, sous n'importe laquelle de ses visibilités, et rend ce qui
/// suit `mod `.
///
/// Un `model.rs` retouché à la main peut porter `mod`, `pub(crate) mod` ou
/// `pub(super) mod` aussi bien que `pub mod` : ce sont toutes des déclarations de
/// module valides, et ignorer les trois premières laisserait leurs entités
/// silencieusement rattachées à la racine du fichier.
fn strip_module_declaration(trimmed: &str) -> Option<&str> {
    const VISIBILITIES: [&str; 3] = ["pub(crate) ", "pub(super) ", "pub "];

    let after_visibility = VISIBILITIES
        .iter()
        .find_map(|prefix| trimmed.strip_prefix(prefix))
        .unwrap_or(trimmed);

    after_visibility.strip_prefix("mod ")
}

/// Extrait `users` de `#[sea_orm(table_name = "users")]`.
///
/// N'examine que les lignes d'attribut (`#[` ou `#![`) : un commentaire qui mentionne
/// `table_name` en prose, par exemple pour expliquer un renommage, ne doit jamais être
/// lu comme une déclaration.
fn table_name(line: &str) -> Option<String> {
    if !(line.starts_with("#[") || line.starts_with("#![")) {
        return None;
    }

    let rest = line.split_once("table_name")?.1;
    let rest = rest.split_once('"')?.1;
    let (name, _) = rest.split_once('"')?;

    (!name.is_empty()).then(|| name.to_string())
}

/// Retrouve l'entité portant cette table.
pub(crate) fn find<'a>(entities: &'a [Entity], table: &str) -> Option<&'a Entity> {
    entities.iter().find(|entity| entity.table == table)
}

/// Les tables connues, triées : c'est ce que le refus d'une cible inconnue énumère.
pub(crate) fn tables(entities: &[Entity]) -> Vec<String> {
    let mut names: Vec<String> = entities.iter().map(|e| e.table.clone()).collect();
    names.sort();
    names.dedup();

    names
}

/// `table` a-t-elle une migration inscrite dans le projet ?
///
/// `rbs generate feature` écrit un `model.rs` sans migration : une entité qu'`scan` trouve
/// n'a donc pas forcément de table en base. Une migration s'inscrit dans
/// `migration/src/lib.rs` sous `mod m..._create_<table>;` — c'est ce texte qu'on cherche,
/// plutôt que de rouvrir chaque fichier de migration pour y lire la table qu'il crée.
pub(crate) fn has_migration(root: &Path, table: &str) -> bool {
    let Ok(source) = fs::read_to_string(root.join("migration/src/lib.rs")) else {
        return false;
    };

    source.contains(&format!("_create_{table};"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn project(features: &[(&str, &str)]) -> TempDir {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        for (module, source) in features {
            let directory = root.path().join("src").join(module);
            fs::create_dir_all(&directory).expect("le répertoire se crée");
            fs::write(directory.join("model.rs"), source).expect("l'écriture aboutit");
        }
        root
    }

    const PLAIN: &str = r#"
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model { pub id: Uuid }
"#;

    // `auth` déclare deux entités dans des modules imbriqués. La table `users` est la
    // cible la plus probable de toute relation : un scan qui ne lirait que les
    // répertoires la déclarerait introuvable.
    const NESTED: &str = r#"
pub mod user {
    #[sea_orm(table_name = "users")]
    pub struct Model { pub id: Uuid }
}

pub mod refresh_token {
    #[sea_orm(table_name = "refresh_tokens")]
    pub struct Model { pub id: Uuid }
}
"#;

    #[test]
    fn a_flat_feature_yields_one_entity_at_its_module_root() {
        let root = project(&[("posts", PLAIN)]);
        let found = scan(root.path());

        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].table, "posts");
        assert_eq!(found[0].module_path, "crate::posts::model");
        assert_eq!(found[0].file, "src/posts/model.rs");
    }

    #[test]
    fn nested_modules_are_followed_so_auth_tables_are_visible() {
        let root = project(&[("auth", NESTED)]);
        let found = scan(root.path());
        let users = find(&found, "users").expect("la table users doit être trouvée");

        assert_eq!(users.module_path, "crate::auth::model::user");
        assert_eq!(users.file, "src/auth/model.rs");
        assert!(find(&found, "refresh_tokens").is_some(), "{found:?}");
    }

    #[test]
    fn the_tables_are_listed_sorted_for_a_stable_error_message() {
        let root = project(&[("posts", PLAIN), ("auth", NESTED)]);

        assert_eq!(
            tables(&scan(root.path())),
            ["posts", "refresh_tokens", "users"]
        );
    }

    #[test]
    fn a_project_without_a_src_directory_yields_nothing() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");

        assert!(scan(root.path()).is_empty());
    }

    #[test]
    fn a_table_created_by_a_migration_is_recognized() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        fs::create_dir_all(root.path().join("migration/src")).expect("le répertoire se crée");
        fs::write(
            root.path().join("migration/src/lib.rs"),
            "mod m20260826_143000_create_users;\n",
        )
        .expect("l'écriture aboutit");

        assert!(has_migration(root.path(), "users"));
    }

    // Le trou que le scan laissait ouvert : un `model.rs` sans migration existe pour de
    // vrai, `rbs generate feature` en écrit un.
    #[test]
    fn a_table_without_a_migration_is_not_recognized() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        fs::create_dir_all(root.path().join("migration/src")).expect("le répertoire se crée");
        fs::write(
            root.path().join("migration/src/lib.rs"),
            "mod m20260826_143000_create_tags;\n",
        )
        .expect("l'écriture aboutit");

        assert!(!has_migration(root.path(), "users"));
    }

    #[test]
    fn a_project_without_a_migration_crate_has_no_migration_for_any_table() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");

        assert!(!has_migration(root.path(), "users"));
    }

    #[test]
    fn a_module_without_a_table_name_is_ignored() {
        let root = project(&[("health", "pub fn ok() {}\n")]);

        assert!(scan(root.path()).is_empty());
    }

    // Une `struct` referme aussi son accolade en début de ligne : un compteur de
    // profondeur naïf, remis à zéro par `}` seule sans vrai comptage, la confond avec
    // la fermeture du module. Une seconde entité, toujours nichée dans `user` après
    // cette accolade de trop, se retrouverait alors faussement au module racine ; et
    // la troisième, réellement au niveau racine du fichier, doit y rester elle aussi.
    #[test]
    fn a_struct_closing_inside_a_module_does_not_confuse_a_later_nested_entity() {
        let source = r#"
pub mod user {
    #[sea_orm(table_name = "users")]
    pub struct Model {
        pub id: Uuid,
    }

    #[sea_orm(table_name = "user_profiles")]
    pub struct Profile { pub id: Uuid }
}

#[sea_orm(table_name = "sessions")]
pub struct Model { pub id: Uuid }
"#;
        let root = project(&[("auth", source)]);
        let found = scan(root.path());

        let users = find(&found, "users").expect("la table users doit être trouvée");
        assert_eq!(users.module_path, "crate::auth::model::user", "{found:?}");

        let profiles =
            find(&found, "user_profiles").expect("la table user_profiles doit être trouvée");
        assert_eq!(profiles.module_path, "crate::auth::model::user");

        let sessions = find(&found, "sessions").expect("la table sessions doit être trouvée");
        assert_eq!(sessions.module_path, "crate::auth::model");
    }

    // `table_name` cherchait la sous-chaîne n'importe où sur la ligne : un commentaire
    // qui la mentionne produirait une entité fantôme, table qui n'existe pas.
    #[test]
    fn a_comment_mentioning_table_name_is_not_read_as_a_declaration() {
        let source = r#"
// renommée depuis table_name "old_posts"
#[sea_orm(table_name = "posts")]
pub struct Model { pub id: Uuid }
"#;
        let root = project(&[("posts", source)]);
        let found = scan(root.path());

        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].table, "posts");
    }

    // Un `model.rs` retouché à la main peut nicher une entité sous `mod`,
    // `pub(crate) mod` ou `pub(super) mod` : ce sont des syntaxes Rust valides, pas
    // seulement `pub mod`. Ne pas les reconnaître laisse le `module_path` à la racine
    // du fichier, silencieusement faux, alors que ce chemin sert ensuite à écrire du
    // code.
    #[test]
    fn a_module_declared_with_any_visibility_is_still_recognized_as_nested() {
        let source = r#"
mod bare {
    #[sea_orm(table_name = "bares")]
    pub struct Model { pub id: Uuid }
}

pub(crate) mod crate_scoped {
    #[sea_orm(table_name = "crate_scoped_items")]
    pub struct Model { pub id: Uuid }
}

pub(super) mod super_scoped {
    #[sea_orm(table_name = "super_scoped_items")]
    pub struct Model { pub id: Uuid }
}
"#;
        let root = project(&[("mixed", source)]);
        let found = scan(root.path());

        let bare = find(&found, "bares").expect("la table bares doit être trouvée");
        assert_eq!(bare.module_path, "crate::mixed::model::bare", "{found:?}");

        let crate_scoped = find(&found, "crate_scoped_items")
            .expect("la table crate_scoped_items doit être trouvée");
        assert_eq!(
            crate_scoped.module_path,
            "crate::mixed::model::crate_scoped"
        );

        let super_scoped = find(&found, "super_scoped_items")
            .expect("la table super_scoped_items doit être trouvée");
        assert_eq!(
            super_scoped.module_path,
            "crate::mixed::model::super_scoped"
        );
    }
}
