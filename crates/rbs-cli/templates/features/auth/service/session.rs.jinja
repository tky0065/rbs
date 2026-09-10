use std::sync::LazyLock;

use chrono::Utc;
use rbs_core::config::AuthConfig;
use rbs_core::{Error, Result, hash, token};
use sea_orm::DatabaseConnection;
use sea_orm::prelude::Uuid;

use super::super::dto::{
    LoginRequest, RefreshRequest, RegisterRequest, SessionResponse, TokenPair, UserResponse,
};
use super::super::repository::{self, ADRESSE_PRISE};
use super::{issue, profile};

/// Le hash vérifié quand l'adresse est inconnue.
///
/// Sans lui, la connexion d'une adresse jamais inscrite répond en une fraction de
/// milliseconde là où Argon2 en coûte des dizaines : ce seul écart énumère les comptes.
/// Le mot de passe qu'il couvre n'ouvre rien — seule sa vérification compte, pas son
/// résultat.
static HASH_DE_COMPARAISON: LazyLock<String> = LazyLock::new(|| {
    hash::hash_password("aucun compte ne porte ce mot de passe").expect("hachage du hash témoin")
});

pub async fn register(db: &DatabaseConnection, input: RegisterRequest) -> Result<UserResponse> {
    if repository::find_by_email(db, &input.email).await?.is_some() {
        return Err(Error::Conflict(ADRESSE_PRISE.to_owned()));
    }

    let hash = hash::hash_password(&input.password)?;
    let cree = repository::create(db, &input.email, &hash).await?;

    Ok(profile(cree))
}

pub async fn login(
    db: &DatabaseConnection,
    auth: &AuthConfig,
    input: LoginRequest,
) -> Result<TokenPair> {
    let utilisateur = repository::find_by_email(db, &input.email).await?;

    // Le mot de passe est vérifié même lorsqu'aucun compte ne répond : sortir plus tôt
    // ici distinguerait une adresse inscrite d'une autre par le seul temps de réponse.
    let hash = utilisateur
        .as_ref()
        .map_or(HASH_DE_COMPARAISON.as_str(), |trouve| {
            trouve.password_hash.as_str()
        });

    let correspond = hash::verify_password(&input.password, hash)?;

    match utilisateur {
        Some(utilisateur) if correspond => issue(db, auth, &utilisateur).await,
        // Mot de passe faux et adresse inconnue rendent la même erreur : les distinguer
        // dirait à un attaquant quelles adresses sont inscrites.
        _ => Err(Error::Unauthorized),
    }
}

pub async fn refresh(
    db: &DatabaseConnection,
    auth: &AuthConfig,
    input: RefreshRequest,
) -> Result<TokenPair> {
    let fingerprint = token::fingerprint(&input.refresh_token);
    let maintenant = Utc::now().fixed_offset();

    // Jeton inconnu, déjà tourné ou périmé : la même erreur pour les trois. Les
    // distinguer renseignerait sur l'état des sessions.
    let session = repository::find_refresh_token(db, &fingerprint)
        .await?
        .filter(|session| session.expires_at > maintenant)
        .ok_or(Error::Unauthorized)?;

    // Rien ici ne relit `revoked_at` : c'est `consume` qui porte la condition, et elle
    // seule peut la porter sans laisser passer deux rafraîchissements concurrents.
    if !repository::consume(db, session.id).await? {
        // La ligne existe et était déjà fermée : ce jeton a servi deux fois. L'un de ses
        // deux porteurs n'est pas le titulaire du compte, et rien ne dit lequel — un
        // jeton volé et joué avant la rotation légitime laisserait sinon le voleur avec
        // une paire valide, renouvelée indéfiniment. Tout le compte se reconnecte.
        let fermees = repository::revoke_sessions_of(db, session.user_id).await?;

        // Ni l'adresse ni le jeton : le journal ne porte pas ce que la réponse tait, et
        // l'identifiant du compte suffit à retrouver ce qui s'est passé.
        tracing::warn!(
            user_id = %session.user_id,
            sessions_revoquees = fermees,
            "jeton de rafraîchissement rejoué : les sessions du compte sont révoquées"
        );

        return Err(Error::Unauthorized);
    }

    let utilisateur = repository::find(db, session.user_id)
        .await?
        .ok_or(Error::Unauthorized)?;

    issue(db, auth, &utilisateur).await
}

pub async fn logout(db: &DatabaseConnection, input: RefreshRequest) -> Result<()> {
    let fingerprint = token::fingerprint(&input.refresh_token);

    let session = repository::find_refresh_token(db, &fingerprint)
        .await?
        .ok_or(Error::Unauthorized)?;

    // La session ferme la ligne présentée, et elle seule : les autres appareils du même
    // compte gardent la leur. Un jeton déjà fermé ne l'est pas deux fois, et ne fait pas
    // tomber le compte comme un rafraîchissement rejoué : redemander une déconnexion est
    // le fait d'un client qui réessaie, non celui d'un jeton qui circule.
    if !repository::consume(db, session.id).await? {
        return Err(Error::Unauthorized);
    }

    Ok(())
}

pub async fn me(db: &DatabaseConnection, id: Uuid) -> Result<UserResponse> {
    // Un jeton valide dont le compte a disparu ne vaut pas mieux qu'un jeton invalide :
    // `NotFound` laisserait entendre que la session, elle, tient encore.
    repository::find(db, id)
        .await?
        .map(profile)
        .ok_or(Error::Unauthorized)
}

/// Les sessions ouvertes du compte, sans jamais l'empreinte du jeton qu'elles portent.
pub async fn sessions(db: &DatabaseConnection, user_id: Uuid) -> Result<Vec<SessionResponse>> {
    Ok(repository::open_sessions_of(db, user_id)
        .await?
        .into_iter()
        .map(session_view)
        .collect())
}

/// La vue publique d'une session.
///
/// Même règle que `profile()` pour `UserResponse` : `SessionResponse` ne porte pas
/// `token_hash`, et cette fonction est le seul passage du modèle vers la réponse.
fn session_view(session: repository::refresh_token::Model) -> SessionResponse {
    SessionResponse {
        id: session.id,
        created_at: session.created_at,
        expires_at: session.expires_at,
    }
}

/// Ferme une session nommée du compte appelant.
///
/// `NotFound` et non `Forbidden` : un identifiant qui n'est pas le vôtre ne désigne, de
/// votre côté, aucune session — un 403 confirmerait qu'elle existe chez quelqu'un d'autre.
pub async fn revoke_session(db: &DatabaseConnection, id: Uuid, user_id: Uuid) -> Result<()> {
    if !repository::revoke_session(db, id, user_id).await? {
        return Err(Error::NotFound("session"));
    }

    Ok(())
}

pub async fn revoke_sessions(db: &DatabaseConnection, user_id: Uuid) -> Result<()> {
    repository::revoke_sessions_of(db, user_id).await?;

    Ok(())
}
