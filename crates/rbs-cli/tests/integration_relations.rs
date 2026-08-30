//! Ce que l'écriture du côté inverse fait, et ne fait pas, au modèle de la cible.
//!
//! Le test vit ici et non dans le module du générateur : `CARGO_BIN_EXE_rbs`, dont
//! `assert_cmd` a besoin, n'est défini que pour les tests d'intégration.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Lance `rbs` dans `root` et rend sa sortie, sans exiger qu'elle aboutisse.
fn rbs(root: &Path, arguments: &[&str]) -> std::process::Output {
    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(root)
        .args(arguments)
        .output()
        .expect("le binaire doit être lançable")
}

/// Un projet portant déjà `users`, cible de toutes les relations de ce fichier.
fn project_with_users(parent: &Path) -> std::path::PathBuf {
    let root = common::projet(parent);
    let output = rbs(
        &root,
        &["g", "crud", "users", "--fields", "email:string:unique"],
    );
    assert!(
        output.status.success(),
        "users doit se générer :\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    root
}

#[test]
fn a_reference_writes_the_inverse_into_the_target_model() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let root = project_with_users(parent.path());

    let output = rbs(
        &root,
        &[
            "g",
            "crud",
            "posts",
            "--fields",
            "title:string,author:references:users",
        ],
    );
    assert!(
        output.status.success(),
        "posts doit se générer :\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let target = fs::read_to_string(root.join("src/users/model.rs")).expect("le modèle se lit");
    assert!(
        target.contains(r#"has_many = "crate::posts::model::Entity""#),
        "la variante inverse est absente :\n{target}"
    );
    assert_eq!(
        target.matches("    Posts,").count(),
        1,
        "la variante inverse est écrite plus d'une fois :\n{target}"
    );

    let carrier = fs::read_to_string(root.join("src/posts/model.rs")).expect("le modèle se lit");
    assert!(carrier.contains("    Author,"), "{carrier}");
}

/// Le §4.4 impose l'idempotence : une seconde génération identique n'écrit pas une
/// seconde variante homonyme dans la cible.
#[test]
fn generating_the_same_relation_twice_leaves_a_single_inverse() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let root = project_with_users(parent.path());
    let arguments = [
        "g",
        "crud",
        "posts",
        "--fields",
        "title:string,author:references:users",
    ];

    rbs(&root, &arguments);
    let after_the_first =
        fs::read_to_string(root.join("src/users/model.rs")).expect("le modèle se lit");

    let second = rbs(&root, &arguments);
    assert!(
        !second.status.success(),
        "la seconde génération doit échouer : la feature est déjà là"
    );
    assert_eq!(
        fs::read_to_string(root.join("src/users/model.rs")).expect("le modèle se lit"),
        after_the_first,
        "la seconde génération a retouché le modèle de la cible"
    );
}

/// Ancre absente : le CLI n'écrit rien dans ce fichier et affiche le bloc à coller.
///
/// Le bloc à coller vit sur la sortie standard (`ui::info`), le refus lui-même sur la
/// sortie d'erreur (`ui::error`) : le test lit les deux concaténées, sans quoi une
/// assertion sur `stderr` seul échouerait à tort sur un comportement pourtant juste.
#[test]
fn a_missing_anchor_in_the_target_writes_nothing_and_shows_the_block() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let root = project_with_users(parent.path());
    common::commiter(&root, "projet et users");

    let model = root.join("src/users/model.rs");
    let source = fs::read_to_string(&model).expect("le modèle se lit");
    fs::write(&model, source.replace("    // <rbs:relations:users>\n", ""))
        .expect("l'écriture aboutit");

    let before = common::empreinte(&root);
    let output = rbs(
        &root,
        &[
            "g",
            "crud",
            "posts",
            "--fields",
            "title:string,author:references:users",
            "--force",
        ],
    );

    let output_text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "la génération devait refuser :\n{output_text}"
    );
    assert!(
        output_text.contains("<rbs:relations:users>") && output_text.contains("src/users/model.rs"),
        "le bloc à coller et son fichier doivent être affichés :\n{output_text}"
    );
    common::assert_intact(&before, &root, "une ancre absente laisse le projet intact");
}

/// Le trou trouvé en relecture d'une tâche antérieure : `entities::scan` reconnaît une
/// cible à son `model.rs`, pas à sa migration. `rbs generate feature` écrit l'un sans
/// l'autre, et une relation vers une telle cible poserait une clé étrangère vers une
/// table qu'aucune migration ne crée.
#[test]
fn a_reference_to_a_feature_without_a_migration_is_refused() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let root = common::projet(parent.path());

    let without_migration = rbs(&root, &["g", "feature", "users"]);
    assert!(
        without_migration.status.success(),
        "la feature vide doit se générer :\n{}",
        String::from_utf8_lossy(&without_migration.stderr)
    );

    let output = rbs(
        &root,
        &["g", "crud", "posts", "--fields", "author:references:users"],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "users n'a pas de migration :\n{stderr}"
    );
    assert!(stderr.contains("users"), "{stderr}");
    assert!(
        !root.join("src/posts").is_dir(),
        "des fichiers ont été écrits malgré la migration absente"
    );
}
