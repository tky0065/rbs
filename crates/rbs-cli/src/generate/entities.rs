//! Inventaire des entités SeaORM d'un projet, lu sur le disque.
//!
//! Le scan est textuel, non un parseur Rust : un modèle lourdement réécrit le fera
//! échouer en refusant une cible, jamais en écrivant une relation fausse.

use std::fs;
use std::path::Path;

use super::fields::to_pascal_case;

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
    let mut lecture = Reading::default();

    for line in source.lines() {
        let Line {
            code,
            opens,
            closes,
        } = lecture.read(line);
        let trimmed = code.trim();

        if closes_at.is_none()
            && let Some(rest) = strip_module_declaration(trimmed)
            && let Some(name) = rest.split(['{', ';', ' ']).next().filter(|n| !n.is_empty())
        {
            current = format!("{module_path}::{name}");
            closes_at = Some(depth);
        }

        if let Some(table) = table_name(trimmed) {
            found.push(Entity {
                table,
                module_path: current.clone(),
                file: file.to_string(),
            });
        }

        depth += opens;
        depth = depth.saturating_sub(closes);

        if closes_at == Some(depth) {
            current = module_path.to_string();
            closes_at = None;
        }
    }
}

/// Ce qu'une ligne apporte au scan, une fois son texte mis de côté.
struct Line {
    /// La ligne privée de ses commentaires, chaînes conservées.
    ///
    /// `table_name` lit le contenu d'une chaîne — c'est là que vit le nom de la table —
    /// et ne peut donc pas travailler sur un code dont les chaînes auraient été retirées.
    code: String,
    /// Accolades ouvrantes de la ligne, hors chaîne et hors commentaire.
    opens: usize,
    /// Ses fermantes, à la même condition.
    closes: usize,
}

/// L'état lexical du scan, qui se poursuit d'une ligne à la suivante.
///
/// Un commentaire de bloc et une chaîne peuvent tous deux franchir une fin de ligne : le
/// suivi ne peut pas se faire ligne par ligne.
#[derive(Default)]
enum Reading {
    /// Du code.
    #[default]
    Code,
    /// Dans une `"chaîne"`, où `\"` n'en sort pas.
    Text,
    /// Dans une chaîne brute `r#"…"#`, que ferme un guillemet suivi d'autant de dièses.
    Raw(usize),
    /// Dans un `/* commentaire */`, dont Rust autorise l'imbrication.
    Block(usize),
}

impl Reading {
    /// Lit une ligne et rend ce que le scan doit en retenir, l'état reporté à la suivante.
    ///
    /// Sans cette lecture, le scan comptait les accolades des chaînes et des
    /// commentaires : un `format!("{{")` décalait la profondeur et rattachait les entités
    /// suivantes au mauvais module, quand le fichier promet de refuser une cible plutôt
    /// que d'écrire une relation fausse.
    fn read(&mut self, line: &str) -> Line {
        let mut code = String::with_capacity(line.len());
        let (mut opens, mut closes) = (0, 0);
        let octets: Vec<char> = line.chars().collect();
        let mut rang = 0;

        while rang < octets.len() {
            let caractere = octets[rang];

            match self {
                Self::Block(niveau) => {
                    if caractere == '*' && octets.get(rang + 1) == Some(&'/') {
                        *niveau -= 1;
                        if *niveau == 0 {
                            *self = Self::Code;
                        }
                        rang += 2;
                        continue;
                    }
                    if caractere == '/' && octets.get(rang + 1) == Some(&'*') {
                        *niveau += 1;
                        rang += 2;
                        continue;
                    }
                }
                Self::Text => {
                    code.push(caractere);
                    if caractere == '\\' {
                        // L'échappement emporte le caractère suivant, guillemet compris.
                        if let Some(suivant) = octets.get(rang + 1) {
                            code.push(*suivant);
                        }
                        rang += 2;
                        continue;
                    }
                    if caractere == '"' {
                        *self = Self::Code;
                    }
                }
                Self::Raw(dieses) => {
                    code.push(caractere);
                    // La chaîne ne se ferme que sur un guillemet suivi d'autant de dièses
                    // qu'elle en a ouvert : moins n'y suffit pas.
                    let suivants = &octets[rang + 1..];
                    if caractere == '"'
                        && suivants.len() >= *dieses
                        && suivants[..*dieses].iter().all(|c| *c == '#')
                    {
                        *self = Self::Code;
                    }
                }
                Self::Code => {
                    if caractere == '/' && octets.get(rang + 1) == Some(&'/') {
                        // Le reste de la ligne est du commentaire : rien n'en est retenu.
                        break;
                    }
                    if caractere == '/' && octets.get(rang + 1) == Some(&'*') {
                        *self = Self::Block(1);
                        rang += 2;
                        continue;
                    }
                    if let Some(dieses) = raw_string_at(&octets, rang) {
                        // `r#"` ouvre la chaîne : le préfixe et ses dièses sont du code.
                        for c in &octets[rang..=rang + dieses + 1] {
                            code.push(*c);
                        }
                        *self = Self::Raw(dieses);
                        rang += dieses + 2;
                        continue;
                    }

                    code.push(caractere);

                    match caractere {
                        '"' => *self = Self::Text,
                        '{' => opens += 1,
                        '}' => closes += 1,
                        // Un caractère littéral peut porter une accolade — `'{'` — ou un
                        // guillemet, et sa quote ne doit pas ouvrir de chaîne. La forme
                        // est bornée, donc reconnue ici plutôt que suivie par un état.
                        '\'' => {
                            if let Some(saut) = char_literal_at(&octets, rang) {
                                for c in &octets[rang + 1..rang + saut] {
                                    code.push(*c);
                                }
                                rang += saut;
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
            }

            rang += 1;
        }

        Line {
            code,
            opens,
            closes,
        }
    }
}

/// Nombre de dièses d'une chaîne brute ouvrant au rang donné, ou `None` si ce n'en est pas
/// une. `r"…"` en compte zéro, `r#"…"#` un.
fn raw_string_at(octets: &[char], rang: usize) -> Option<usize> {
    if octets[rang] != 'r' {
        return None;
    }
    // `r` n'ouvre une chaîne brute que s'il n'est pas la fin d'un identifiant.
    if rang > 0 && (octets[rang - 1].is_alphanumeric() || octets[rang - 1] == '_') {
        return None;
    }

    let dieses = octets[rang + 1..].iter().take_while(|c| **c == '#').count();

    (octets.get(rang + 1 + dieses) == Some(&'"')).then_some(dieses)
}

/// Longueur d'un caractère littéral ouvrant au rang donné, quote fermante comprise, ou
/// `None` si la quote est en fait celle d'une durée de vie — `&'a str`.
fn char_literal_at(octets: &[char], rang: usize) -> Option<usize> {
    let echappe = octets.get(rang + 1) == Some(&'\\');
    let fin = if echappe { rang + 3 } else { rang + 2 };

    (octets.get(fin) == Some(&'\'')).then_some(fin - rang)
}

/// Reconnaît `mod nom`, sous n'importe laquelle de ses visibilités, et rend ce qui
/// suit `mod `.
///
/// Un `model.rs` retouché à la main peut porter `mod`, `pub(crate) mod` ou
/// `pub(super) mod` aussi bien que `pub mod` : ce sont toutes des déclarations de
/// module valides, et ignorer les trois premières laisserait leurs entités
/// silencieusement rattachées à la racine du fichier.
fn strip_module_declaration(trimmed: &str) -> Option<&str> {
    strip_visibility(trimmed).strip_prefix("mod ")
}

/// Retire le préfixe de visibilité d'une déclaration, s'il y en a un.
fn strip_visibility(trimmed: &str) -> &str {
    const VISIBILITIES: [&str; 3] = ["pub(crate) ", "pub(super) ", "pub "];

    VISIBILITIES
        .iter()
        .find_map(|prefix| trimmed.strip_prefix(prefix))
        .unwrap_or(trimmed)
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

/// `table` a-t-elle une migration dans le projet ?
///
/// `rbs generate feature` écrit un `model.rs` sans migration : une entité qu'`scan` trouve
/// n'a donc pas forcément de table en base.
///
/// La recherche porte sur le **contenu** des migrations, non sur leur nom : celui-ci est
/// libre, et une migration qui crée plusieurs tables n'en nomme aucune — le fragment
/// `auth` crée `users` et `refresh_tokens` sous `create_auth_tables`. Ce qu'une migration
/// créant une table écrit toujours, en revanche, est le couple `.table(<Iden>::Table)` et
/// `enum <Iden>`, avec `<Iden>` le nom de la table en PascalCase.
///
/// Textuel comme le scan, et faillible du même côté : une migration écrite autrement — du
/// SQL brut, un identifiant renommé — n'est pas reconnue, et la relation qui la visait est
/// refusée. C'est le sens du refus qu'il faut préserver en y touchant : mieux vaut
/// redemander une migration qui existe que poser une clé étrangère vers une table absente.
pub(crate) fn has_migration(root: &Path, table: &str) -> bool {
    let iden = to_pascal_case(table);

    let Ok(entries) = fs::read_dir(root.join("migration/src")) else {
        return false;
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|suffixe| suffixe == "rs"))
        .filter_map(|path| fs::read_to_string(path).ok())
        .any(|source| creates_table(&source, &iden))
}

/// Cette source de migration crée-t-elle la table dont l'identifiant SeaORM est `iden` ?
fn creates_table(source: &str, iden: &str) -> bool {
    source.contains(&format!(".table({iden}::Table)")) && declares_iden(source, iden)
}

/// La source déclare-t-elle `enum <iden>`, sous n'importe quelle visibilité ?
///
/// Le nom est comparé en entier, et non par préfixe : `enum Users` et `enum UserSessions`
/// commencent pareil, et les confondre déclarerait `users` créée par une migration qui ne
/// la touche pas.
fn declares_iden(source: &str, iden: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.trim_start();
        let declaration = strip_visibility(trimmed);

        declaration.strip_prefix("enum ").is_some_and(|rest| {
            let name = rest
                .trim_start()
                .split(|letter: char| !letter.is_alphanumeric() && letter != '_')
                .next()
                .unwrap_or_default();

            name == iden
        })
    })
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

    /// Le défaut que le module promet de ne pas avoir : une accolade qui n'ouvre rien.
    ///
    /// `format!("{{")` est du texte, pas un bloc ; la compter décale la profondeur et
    /// referme le module courant une entité trop tôt.
    const BRACE_IN_A_STRING: &str = r#"
pub mod user {
    #[sea_orm(table_name = "users")]
    pub struct Model { pub id: Uuid }

    impl Model {
        /// Une accolade ouvrante en prose, que rien ne referme : `{`.
        pub fn label(&self) -> String {
            format!("{{")
        }
    }
}

pub mod refresh_token {
    #[sea_orm(table_name = "refresh_tokens")]
    pub struct Model { pub id: Uuid }
}
"#;

    /// Une entité et un module mis au rebut en commentaire de bloc, comme on le fait en
    /// retouchant un modèle à la main.
    const COMMENTED_OUT: &str = r#"
/*
pub mod ancien {
    #[sea_orm(table_name = "anciennes_tables")]
    pub struct Model { pub id: Uuid }
}
*/

pub mod user {
    // #[sea_orm(table_name = "commentee")]
    #[sea_orm(table_name = "users")]
    pub struct Model { pub id: Uuid }
}
"#;

    #[test]
    fn a_brace_inside_a_string_does_not_close_the_module() {
        let root = project(&[("auth", BRACE_IN_A_STRING)]);
        let found = scan(root.path());

        let jeton = find(&found, "refresh_tokens").expect("les deux entités sont relevées");
        assert_eq!(
            jeton.module_path, "crate::auth::model::refresh_token",
            "l'accolade d'une chaîne a refermé le module trop tôt : {found:?}"
        );
    }

    #[test]
    fn an_entity_commented_out_is_not_inventoried() {
        let root = project(&[("auth", COMMENTED_OUT)]);
        let found = scan(root.path());

        assert_eq!(
            tables(&found),
            vec!["users".to_string()],
            "une entité en commentaire a été relevée : {found:?}"
        );
        assert_eq!(
            found[0].module_path, "crate::auth::model::user",
            "le module en commentaire a été suivi : {found:?}"
        );
    }

    /// Une chaîne brute ne s'échappe pas : ses accolades et ses guillemets sont du texte.
    #[test]
    fn a_raw_string_is_read_as_text() {
        let source = r###"
pub mod user {
    pub const REQUETE: &str = r#"select "{" from t"#;
    #[sea_orm(table_name = "users")]
    pub struct Model { pub id: Uuid }
}

pub mod session {
    #[sea_orm(table_name = "sessions")]
    pub struct Model { pub id: Uuid }
}
"###;
        let root = project(&[("auth", source)]);
        let found = scan(root.path());

        let session = find(&found, "sessions").expect("les deux entités sont relevées");
        assert_eq!(
            session.module_path, "crate::auth::model::session",
            "la chaîne brute a déréglé la profondeur : {found:?}"
        );
    }

    /// Un caractère littéral peut porter une accolade, et sa quote ne doit pas être prise
    /// pour l'ouverture d'une chaîne — une durée de vie en porte une aussi.
    #[test]
    fn a_char_literal_and_a_lifetime_are_not_strings() {
        let source = r#"
pub mod user {
    pub fn ouvre<'a>(t: &'a str) -> bool { t.starts_with('{') }
    #[sea_orm(table_name = "users")]
    pub struct Model { pub id: Uuid }
}

pub mod session {
    #[sea_orm(table_name = "sessions")]
    pub struct Model { pub id: Uuid }
}
"#;
        let root = project(&[("auth", source)]);
        let found = scan(root.path());

        let session = find(&found, "sessions").expect("les deux entités sont relevées");
        assert_eq!(
            session.module_path, "crate::auth::model::session",
            "l'accolade d'un caractère littéral a été comptée : {found:?}"
        );
    }

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

    /// Une migration, telle que le CLI et les fragments l'écrivent : la table se nomme
    /// dans le corps, jamais dans le nom du fichier.
    fn migration(root: &Path, module: &str, idens: &[&str]) {
        fs::create_dir_all(root.join("migration/src")).expect("le répertoire se crée");

        let mut source = String::from("use sea_orm_migration::prelude::*;\n");
        for iden in idens {
            source.push_str(&format!(
                "\nmanager.create_table(Table::create().table({iden}::Table).to_owned()).await?;\n\
                 \n#[derive(DeriveIden)]\nenum {iden} {{\n    Table,\n    Id,\n}}\n"
            ));
        }

        fs::write(root.join(format!("migration/src/{module}.rs")), source)
            .expect("l'écriture aboutit");
    }

    #[test]
    fn a_table_created_by_a_migration_is_recognized() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        migration(root.path(), "m20260826_143000_create_users", &["Users"]);

        assert!(has_migration(root.path(), "users"));
    }

    /// Le défaut que le nom de fichier laissait passer : le fragment `auth` crée `users`
    /// et `refresh_tokens` sous une migration qu'il nomme `create_auth_tables`. Toute
    /// relation vers `users` s'y voyait refusée, migration à l'appui.
    #[test]
    fn a_migration_creating_several_tables_is_recognized_for_each_of_them() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        migration(
            root.path(),
            "m20260826_143000_create_auth_tables",
            &["Users", "RefreshTokens"],
        );

        assert!(has_migration(root.path(), "users"));
        assert!(has_migration(root.path(), "refresh_tokens"));
    }

    // Le trou que le scan laissait ouvert : un `model.rs` sans migration existe pour de
    // vrai, `rbs generate feature` en écrit un.
    #[test]
    fn a_table_without_a_migration_is_not_recognized() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        migration(root.path(), "m20260826_143000_create_tags", &["Tags"]);

        assert!(!has_migration(root.path(), "users"));
    }

    /// Deux identifiants dont l'un préfixe l'autre : les confondre déclarerait `users`
    /// créée par une migration qui ne la touche pas.
    #[test]
    fn a_table_whose_iden_merely_prefixes_another_is_not_recognized() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        migration(
            root.path(),
            "m20260826_143000_create_user_sessions",
            &["UserSessions"],
        );

        assert!(!has_migration(root.path(), "users"));
    }

    /// Une table nommée dans le corps d'une migration qui ne la crée pas — une clé
    /// étrangère la vise — ne suffit pas : sans son `enum`, elle n'est pas créée ici.
    #[test]
    fn a_table_merely_referenced_by_a_migration_is_not_recognized() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        fs::create_dir_all(root.path().join("migration/src")).expect("le répertoire se crée");
        fs::write(
            root.path()
                .join("migration/src/m20260826_143000_create_posts.rs"),
            "manager.create_table(Table::create().table(Posts::Table)\n\
             .foreign_key(ForeignKey::create().to(Users::Table, Users::Id)).to_owned());\n\
             \n#[derive(DeriveIden)]\nenum Posts {\n    Table,\n}\n",
        )
        .expect("l'écriture aboutit");

        assert!(has_migration(root.path(), "posts"));
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
