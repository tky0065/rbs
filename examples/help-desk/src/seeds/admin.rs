//! Le compte qui ouvre l'espace d'administration.
//!
//! `register` n'attribue aucun rôle et la colonne `role` défaut à `"user"` : sans ce
//! seed, aucun chemin de l'API ne produit d'administrateur. Les identifiants viennent de
//! l'environnement plutôt que d'être écrits ici — ce fichier est versionné, le `.env` non.

use std::env;

use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use help_desk::auth::model;

/// La variable qui nomme l'environnement, telle que la configuration du noyau la lit.
const ENV: &str = "RBS_ENV";

/// L'adresse du compte, et le mot de passe qui l'ouvre.
const ADRESSE: &str = "ADMIN_EMAIL";
const MOT_DE_PASSE: &str = "ADMIN_PASSWORD";

/// Pose l'administrateur du projet, ou dit pourquoi il ne l'a pas fait.
///
/// Rien n'échoue ici : un seed qui s'arrête sur une variable absente arrêterait avec lui
/// tous ceux que le binaire enchaîne, et l'absence n'est pas une faute — c'est un projet
/// dont le développeur a retiré la variable.
pub async fn seed(db: &DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    // `rbs seed` porte déjà ce refus, mais `cargo run --bin seed` ne passe pas par lui :
    // ce compte-ci a tous les droits, et son mot de passe traîne dans un fichier du poste
    // de travail. La garde est donc ici, où aucun chemin ne la contourne.
    if env::var(ENV).as_deref() == Ok("production") {
        println!("admin : {ENV}=production, aucun compte d'administration n'est posé");
        return Ok(());
    }

    let (Some(adresse), Some(secret)) = (renseignee(ADRESSE), renseignee(MOT_DE_PASSE)) else {
        println!("admin : {ADRESSE} ou {MOT_DE_PASSE} manque au .env, aucun compte n'est posé");
        return Ok(());
    };

    // Comme `service::normalise` : un compte semé sous une autre casse que celle de la
    // connexion ne serait jamais retrouvé.
    let adresse = adresse.trim().to_lowercase();

    if model::user::Entity::find()
        .filter(model::user::Column::Email.eq(adresse.as_str()))
        .one(db)
        .await?
        .is_some()
    {
        println!("admin : {adresse} existe déjà, rien n'est écrit");
        return Ok(());
    }

    model::user::ActiveModel {
        email: Set(adresse.clone()),
        password_hash: Set(rbs_core::hash::hash_password(&secret)?),
        role: Set(model::Role::Admin),
        // Daté d'emblée : sous `login_requires_verification`, un compte dont l'adresse
        // n'est pas prouvée reçoit le 401 d'un mauvais mot de passe, et aucun écran
        // n'appelle `/auth/verify-email` pour le sortir de là.
        email_verified_at: Set(Some(chrono::Utc::now().into())),
        ..Default::default()
    }
    .insert(db)
    .await?;

    println!("admin : {adresse} peut entrer dans l'espace d'administration");

    Ok(())
}

/// La variable, si elle porte autre chose que du blanc.
///
/// `ADMIN_PASSWORD=` ouvrirait sinon un compte d'administration sur la chaîne vide.
fn renseignee(cle: &str) -> Option<String> {
    env::var(cle)
        .ok()
        .filter(|valeur| !valeur.trim().is_empty())
}
