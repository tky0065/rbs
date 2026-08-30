//! Ce que l'écriture du côté inverse fait, et ne fait pas, au modèle de la cible.
//!
//! Le test vit ici et non dans le module du générateur : `CARGO_BIN_EXE_rbs`, dont
//! `assert_cmd` a besoin, n'est défini que pour les tests d'intégration.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

mod common;

/// Lance `rbs` dans `racine` et rend sa sortie, sans exiger qu'elle aboutisse.
fn rbs(racine: &Path, arguments: &[&str]) -> std::process::Output {
    Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(racine)
        .args(arguments)
        .output()
        .expect("le binaire doit être lançable")
}

/// Un projet portant déjà `users`, cible de toutes les relations de ce fichier.
fn project_with_users(parent: &Path) -> std::path::PathBuf {
    let racine = common::projet(parent);
    let output = rbs(
        &racine,
        &["g", "crud", "users", "--fields", "email:string:unique"],
    );
    assert!(
        output.status.success(),
        "users doit se générer :\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    racine
}

#[test]
fn a_reference_writes_the_inverse_into_the_target_model() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_users(parent.path());

    let output = rbs(
        &racine,
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

    let cible = fs::read_to_string(racine.join("src/users/model.rs")).expect("le modèle se lit");
    assert!(
        cible.contains(r#"has_many = "crate::posts::model::Entity""#),
        "la variante inverse est absente :\n{cible}"
    );
    assert_eq!(
        cible.matches("    Posts,").count(),
        1,
        "la variante inverse est écrite plus d'une fois :\n{cible}"
    );

    let porteur = fs::read_to_string(racine.join("src/posts/model.rs")).expect("le modèle se lit");
    assert!(porteur.contains("    Author,"), "{porteur}");
}

/// Le §4.4 impose l'idempotence : une seconde génération identique n'écrit pas une
/// seconde variante homonyme dans la cible.
#[test]
fn generating_the_same_relation_twice_leaves_a_single_inverse() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = project_with_users(parent.path());
    let arguments = [
        "g",
        "crud",
        "posts",
        "--fields",
        "title:string,author:references:users",
    ];

    rbs(&racine, &arguments);
    let apres_la_premiere =
        fs::read_to_string(racine.join("src/users/model.rs")).expect("le modèle se lit");

    let seconde = rbs(&racine, &arguments);
    assert!(
        !seconde.status.success(),
        "la seconde génération doit échouer : la feature est déjà là"
    );
    assert_eq!(
        fs::read_to_string(racine.join("src/users/model.rs")).expect("le modèle se lit"),
        apres_la_premiere,
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
    let racine = project_with_users(parent.path());
    common::commiter(&racine, "projet et users");

    let modele = racine.join("src/users/model.rs");
    let source = fs::read_to_string(&modele).expect("le modèle se lit");
    fs::write(&modele, source.replace("    // <rbs:relations>\n", "")).expect("l'écriture aboutit");

    let avant = common::empreinte(&racine);
    let output = rbs(
        &racine,
        &[
            "g",
            "crud",
            "posts",
            "--fields",
            "title:string,author:references:users",
            "--force",
        ],
    );

    let sortie = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "la génération devait refuser :\n{sortie}"
    );
    assert!(
        sortie.contains("<rbs:relations>") && sortie.contains("src/users/model.rs"),
        "le bloc à coller et son fichier doivent être affichés :\n{sortie}"
    );
    common::assert_intact(&avant, &racine, "une ancre absente laisse le projet intact");
}

/// Le trou trouvé en relecture d'une tâche antérieure : `entities::scan` reconnaît une
/// cible à son `model.rs`, pas à sa migration. `rbs generate feature` écrit l'un sans
/// l'autre, et une relation vers une telle cible poserait une clé étrangère vers une
/// table qu'aucune migration ne crée.
#[test]
fn a_reference_to_a_feature_without_a_migration_is_refused() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let racine = common::projet(parent.path());

    let sans_migration = rbs(&racine, &["g", "feature", "users"]);
    assert!(
        sans_migration.status.success(),
        "la feature vide doit se générer :\n{}",
        String::from_utf8_lossy(&sans_migration.stderr)
    );

    let output = rbs(
        &racine,
        &["g", "crud", "posts", "--fields", "author:references:users"],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "users n'a pas de migration :\n{stderr}"
    );
    assert!(stderr.contains("users"), "{stderr}");
    assert!(
        !racine.join("src/posts").is_dir(),
        "des fichiers ont été écrits malgré la migration absente"
    );
}
