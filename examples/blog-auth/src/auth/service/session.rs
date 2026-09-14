use std::sync::LazyLock;

use chrono::Utc;
use rbs_core::config::AuthConfig;
use rbs_core::{Error, Result, hash, token};
use sea_orm::prelude::Uuid;
use sea_orm::{DatabaseConnection, TransactionTrait};

use super::super::config::FlowConfig;
use super::super::dto::{
    LoginRequest, RefreshRequest, RegisterRequest, SessionResponse, TokenPair, UserResponse,
};
use super::super::repository::{self, refresh_token::Rotation};
use super::{close_every_session, detach, issue, normalise, notify, profile, session_view};
use crate::modules::mail::Mailer;

/// Le hash vérifié quand l'adresse est inconnue.
///
/// Sans lui, la connexion d'une adresse jamais inscrite répond en une fraction de
/// milliseconde là où Argon2 en coûte des dizaines : ce seul écart énumère les comptes.
/// Le mot de passe qu'il couvre n'ouvre rien — seule sa vérification compte, pas son
/// résultat.
static HASH_DE_COMPARAISON: LazyLock<String> = LazyLock::new(|| {
    hash::hash_password("aucun compte ne porte ce mot de passe").expect("hachage du hash témoin")
});

/// Inscrit une adresse, sans jamais dire si elle l'était déjà.
///
/// Neuve, le compte est écrit ici — un client doit pouvoir se connecter aussitôt — et le
/// lien de vérification part détaché. Prise, le compte n'est pas touché et son titulaire
/// est prévenu de la tentative, en détaché aussi. L'appelant reçoit la même chose dans
/// les deux cas.
pub async fn register(
    db: &DatabaseConnection,
    mail: &Mailer,
    flows: &FlowConfig,
    input: RegisterRequest,
) -> Result<()> {
    let email = normalise(&input.email);

    // Argon2 avant tout, dans les deux branches : c'est lui qui coûte, et ne le calculer
    // que pour une adresse neuve ferait répondre une adresse prise des dizaines de
    // millisecondes plus tôt.
    let hash = hash::hash_password(&input.password)?;

    let titulaire = match repository::find_by_email(db, &email).await? {
        Some(titulaire) => Some(titulaire),
        None => match repository::create(db, &email, &hash).await? {
            Some(cree) => {
                super::verification::send_link_detached(db, mail, flows, cree);
                return Ok(());
            }
            // Une inscription concurrente de la même adresse a gagné la course entre la
            // lecture et l'écriture : la contrainte d'unicité l'a tranchée, et l'adresse
            // est prise comme au-dessus.
            None => repository::find_by_email(db, &email).await?,
        },
    };

    if let Some(titulaire) = titulaire {
        warn_taken(mail, flows, titulaire);
    }

    Ok(())
}

/// Prévient le titulaire d'une adresse qu'on a tenté de l'inscrire.
///
/// Détaché comme le lien d'une adresse neuve : les deux branches de `register` répondent
/// dans le même temps.
fn warn_taken(mail: &Mailer, flows: &FlowConfig, titulaire: repository::Model) {
    let (mail, flows) = (mail.clone(), flows.clone());
    detach(titulaire.id, "inscription", async move {
        notify(
            &mail,
            &titulaire,
            "Tentative d'inscription avec votre adresse",
            "inscription.html",
            minijinja::context! { forgot_url => flows.page("forgot-password") },
        );

        Ok(())
    });
}

pub async fn login(
    db: &DatabaseConnection,
    auth: &AuthConfig,
    input: LoginRequest,
) -> Result<TokenPair> {
    let utilisateur = repository::find_by_email(db, &normalise(&input.email)).await?;

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

    // Jeton inconnu ou périmé : la même erreur pour les deux. Les distinguer
    // renseignerait sur l'état des sessions ; ce qu'un jeton connu mais fermé ou tourné
    // vaut se décide juste en dessous.
    let session = repository::find_refresh_token(db, &fingerprint)
        .await?
        .filter(|session| session.expires_at > maintenant)
        .ok_or(Error::Unauthorized)?;

    // Tout ou rien : une session tournée sans paire rendue laisserait le client avec un
    // jeton mort et rien pour le remplacer — et son prochain essai serait un rejeu.
    let transaction = db.begin().await?;

    // Rien ici ne relit les deux colonnes : c'est `rotate` qui porte la condition, et
    // elle seule peut la porter sans laisser passer deux rafraîchissements concurrents.
    match repository::refresh_token::rotate(&transaction, session.id).await? {
        Rotation::Done => {}
        // La ligne avait tourné : ce jeton a servi deux fois. L'un de ses deux porteurs
        // n'est pas le titulaire du compte, et rien ne dit lequel — un jeton volé et joué
        // avant la rotation légitime laisserait sinon le voleur avec une paire valide,
        // renouvelée indéfiniment. Tout le compte se reconnecte.
        Rotation::Replayed => {
            let fermees = close_every_session(&transaction, session.user_id).await?;

            // La révocation est ce qu'un rejeu doit laisser derrière lui : elle se
            // committe avant que l'erreur ne sorte.
            transaction.commit().await?;

            // Ni l'adresse ni le jeton : le journal ne porte pas ce que la réponse tait,
            // et l'identifiant du compte suffit à retrouver ce qui s'est passé.
            tracing::warn!(
                user_id = %session.user_id,
                sessions_revoquees = fermees,
                "jeton de rafraîchissement rejoué : les sessions du compte sont révoquées"
            );

            return Err(Error::Unauthorized);
        }
        // Fermée par une déconnexion, une révocation, ou un rejeu déjà instruit : un
        // client qui réessaie, pas un jeton qui circule pour la première fois. Le même
        // 401 qu'un jeton inconnu, et rien d'autre — rien n'a été écrit, et la
        // transaction abandonnée s'annule d'elle-même.
        Rotation::Closed => {
            tracing::debug!(
                user_id = %session.user_id,
                "jeton de rafraîchissement fermé rejoué"
            );

            return Err(Error::Unauthorized);
        }
    }

    let utilisateur = repository::find(&transaction, session.user_id)
        .await?
        .ok_or(Error::Unauthorized)?;

    let paire = issue(&transaction, auth, &utilisateur).await?;
    transaction.commit().await?;

    Ok(paire)
}

pub async fn logout(db: &DatabaseConnection, input: RefreshRequest) -> Result<()> {
    let fingerprint = token::fingerprint(&input.refresh_token);

    let session = repository::find_refresh_token(db, &fingerprint)
        .await?
        .ok_or(Error::Unauthorized)?;

    // La session ferme la ligne présentée, et elle seule : les autres appareils du même
    // compte gardent la leur. Un jeton déjà fermé ne l'est pas deux fois.
    if !repository::refresh_token::close(db, session.id).await? {
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
    // Les deux écritures de `close_every_session` ou aucune : fermer les
    // rafraîchissements en laissant vivre les accès est ce qu'elle existe pour empêcher.
    let transaction = db.begin().await?;
    close_every_session(&transaction, user_id).await?;
    transaction.commit().await?;

    Ok(())
}
