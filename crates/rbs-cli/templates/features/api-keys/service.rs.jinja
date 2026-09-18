use axum::http::Extensions;
use chrono::Utc;
use rbs_core::jwt::Claims;
use rbs_core::{Error, HasCoreState, Result};
use sea_orm::ActiveEnum;
use sea_orm::prelude::Uuid;

use super::dto::{ApiKeyCreated, ApiKeyResponse, CreateApiKey};
use super::repository;
use crate::auth::model::Role;
use crate::state::AppState;

/// Préfixe de toute clé émise. Reconnaissable d'un coup d'œil dans un fichier de secrets.
const PREFIXE: &str = "rbs_";

/// Ce qui reparaît d'une clé après sa création : `rbs_` et huit caractères du tirage.
const LONGUEUR_DU_PREFIXE: usize = 12;

/// En deçà, la trace d'usage n'est pas réécrite.
///
/// Une clé martelée à mille requêtes par seconde coûte **une** écriture par minute, et non
/// mille par seconde. La minute suffit à la seule question qu'on pose à cette colonne :
/// cette clé sert-elle encore ?
///
/// En secondes, comme tous les réglages de durée du projet : `TimeDelta::minutes` n'est
/// pas constructible dans un `const`.
const TRACE_SECS: i64 = 60;

/// Le même seuil, sous la forme qu'attendent les comparaisons de dates.
fn seuil() -> chrono::TimeDelta {
    chrono::TimeDelta::seconds(TRACE_SECS)
}

/// Juge une clé présentée en `X-Api-Key`, et rend les claims de son porteur.
///
/// Deux lectures et jamais de jointure, comme partout ailleurs dans un projet engendré :
/// la clé, puis le compte. La seconde n'est pas facultative — c'est elle qui fait qu'une
/// clé cesse d'administrer le jour où son porteur est rétrogradé.
pub async fn accept(state: &AppState, key: &str, extensions: &mut Extensions) -> Result<Claims> {
    let maintenant = Utc::now().fixed_offset();
    let empreinte = rbs_core::token::fingerprint(key);

    let cle = repository::active(state.core().db(), &empreinte, maintenant)
        .await?
        // Clé inconnue, révoquée ou périmée : la même réponse pour les trois. Les
        // distinguer renseignerait sur l'état d'une clé qu'on n'a pas.
        .ok_or(Error::Unauthorized)?;

    let compte = crate::auth::repository::find(state.core().db(), cle.user_id)
        .await?
        .ok_or(Error::Unauthorized)?;

    // Le moindre des deux : une clé ne vaut jamais plus que le compte qui la porte, et le
    // plafond se recalcule à chaque requête plutôt que de se figer à la création.
    let servi = cle.role.clone().min(compte.role.clone());

    // La borne est tenue ici, avant tout aller-retour : la ligne vient d'être lue, et une
    // condition portée par le seul `UPDATE` coûterait une écriture par requête pour ne
    // rien changer cinquante-neuf fois sur soixante.
    if cle
        .last_used_at
        .is_none_or(|trace| maintenant - trace >= seuil())
    {
        let db = state.core().db().clone();
        let id = cle.id;
        // Détachée : la réponse n'attend pas l'écriture, et une trace manquée vaut mieux
        // qu'un appel refusé.
        tokio::spawn(async move {
            let _ = repository::touch(db, id, maintenant, seuil()).await;
        });
    }

    // Ce que la garde `VerifiedIdentity` cherchera : sans ce dépôt, elle relirait le compte
    // que l'on vient de lire.
    extensions.insert(crate::auth::guard::Accepted(compte.email_verified_at));

    Ok(Claims {
        sub: compte.id.to_string(),
        role: servi.to_value(),
        // L'échéance de la clé, ou une échéance qui n'arrive pas. Le noyau ne la revérifie
        // pas : la péremption est dans la condition de la lecture ci-dessus, et une
        // seconde garde ailleurs divergerait un jour.
        exp: cle.expires_at.map_or(i64::MAX, |date| date.timestamp()),
        iat: cle.created_at.timestamp(),
        // L'identifiant de la clé : c'est ce qui met dans les journaux *laquelle* a servi.
        jti: cle.id.to_string(),
    })
}

/// Tire une clé pour `porteur`, au rôle demandé si son créateur peut le donner.
///
/// Rend la clé en clair. C'est la seule fois qu'elle existe hors du processus de l'appelant.
pub async fn create(
    state: &AppState,
    porteur: Uuid,
    role_du_createur: Role,
    input: CreateApiKey,
) -> Result<ApiKeyCreated> {
    let demande = match input.role.as_deref() {
        // Le défaut est le rôle le moins ouvert, et non celui du créateur : une clé qui
        // administre doit être demandée, jamais obtenue par omission.
        None => Role::User,
        Some(nom) => Role::try_from_value(&nom.to_owned())
            .map_err(|_| Error::BadRequest(format!("rôle inconnu : {nom}")))?,
    };

    // Le plafond, à la création : nul ne délègue plus qu'il ne détient.
    if demande > role_du_createur {
        return Err(Error::Forbidden);
    }

    let clair = format!("{PREFIXE}{}", rbs_core::token::random());
    let prefixe: String = clair.chars().take(LONGUEUR_DU_PREFIXE).collect();
    let expires_at = input
        .expires_in_days
        .map(|jours| Utc::now().fixed_offset() + chrono::TimeDelta::days(i64::from(jours)));

    let cle = repository::create(
        state.core().db(),
        porteur,
        &input.name,
        &prefixe,
        &rbs_core::token::fingerprint(&clair),
        demande,
        expires_at,
    )
    .await?;

    Ok(ApiKeyCreated {
        id: cle.id,
        name: cle.name,
        prefix: cle.prefix,
        role: cle.role.to_value(),
        expires_at: cle.expires_at,
        key: clair,
    })
}

/// Les clés du porteur, sans rien qui permette de les présenter.
pub async fn list(state: &AppState, porteur: Uuid) -> Result<Vec<ApiKeyResponse>> {
    Ok(repository::list_for_user(state.core().db(), porteur)
        .await?
        .into_iter()
        .map(ApiKeyResponse::from)
        .collect())
}

/// Révoque une clé du porteur. `false` si elle n'est pas à lui, ou déjà révoquée.
pub async fn revoke(state: &AppState, id: Uuid, porteur: Uuid) -> Result<bool> {
    repository::revoke(state.core().db(), id, porteur).await
}

/// Révoque toutes les clés vivantes du porteur, et dit combien.
pub async fn revoke_all(state: &AppState, porteur: Uuid) -> Result<u64> {
    repository::revoke_all(state.core().db(), porteur).await
}
