//! Extracteurs de requête du runtime.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::Error;

#[cfg(feature = "auth")]
use crate::state::HasAuth;
#[cfg(feature = "auth")]
use axum::extract::FromRequestParts;
#[cfg(feature = "auth")]
use axum::http::header::AUTHORIZATION;
#[cfg(feature = "auth")]
use axum::http::request::Parts;

/// Schéma d'autorisation attendu, casse comprise dans la comparaison.
#[cfg(feature = "auth")]
const SCHEMA: &str = "bearer";

/// Identité authentifiée, extraite du jeton porté par la requête.
///
/// L'extracteur lit les en-têtes et ne touche pas au corps : un extracteur qui le
/// consommerait interdirait à [`ValidatedJson`] de le lire ensuite.
#[cfg(feature = "auth")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Identifiant de l'utilisateur, tel que porté par `sub`.
    pub user_id: String,
    /// Rôle en clair. L'enum `Role` est généré dans le projet, hors de portée du noyau.
    pub role: String,
}

#[cfg(feature = "auth")]
impl<S: HasAuth> FromRequestParts<S> for Identity {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jeton = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|valeur| valeur.to_str().ok())
            .and_then(porteur)
            .ok_or(Error::Unauthorized)?;

        let claims = crate::jwt::verifier(jeton, &state.auth().secret)?;

        Ok(Self {
            user_id: claims.sub,
            role: claims.role,
        })
    }
}

/// Isole le jeton d'un en-tête `Authorization: Bearer <jeton>`.
///
/// La RFC 7235 déclare le schéma insensible à la casse ; un client qui envoie `bearer`
/// est conforme, et le refuser serait un bug difficile à diagnostiquer côté appelant.
#[cfg(feature = "auth")]
fn porteur(en_tete: &str) -> Option<&str> {
    let (schema, jeton) = en_tete.split_once(' ')?;

    schema.eq_ignore_ascii_case(SCHEMA).then(|| jeton.trim())
}

/// Corps JSON désérialisé **puis** validé.
///
/// Un controller qui l'extrait reçoit un DTO déjà conforme à ses annotations
/// `validator` : il n'a plus à s'en soucier, et l'échec est rendu au client en
/// `application/problem+json` sans passe-plat.
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Extraire d'abord : un corps illisible ne peut pas être validé.
        let Json(recu) = Json::<T>::from_request(request, state)
            .await
            .map_err(corps_illisible)?;

        recu.validate()?;

        Ok(Self(recu))
    }
}

/// Traduit un rejet d'extraction en [`Error::BadRequest`].
///
/// Tout rejet devient 400, là où axum distingue 400, 415 et 422. La frontière est alors
/// lisible pour qui débogue une API générée : 400 « je n'ai pas pu lire ton corps »,
/// 422 « je l'ai lu, il ne respecte pas les règles ». Seul `body_text()` est repris, pour
/// garder `JsonRejection` hors de la signature d'[`Error`] : une mise à jour d'axum ne
/// doit pas rompre le noyau.
fn corps_illisible(rejet: JsonRejection) -> Error {
    Error::BadRequest(rejet.body_text())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode, header};
    use axum::routing::post;
    use serde::Deserialize;
    use serde_json::Value;
    use tower::ServiceExt;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate)]
    struct Inscription {
        #[validate(email(message = "adresse électronique invalide"))]
        email: String,
        #[validate(range(min = 18, message = "âge minimum : 18 ans"))]
        age: u8,
    }

    /// Poste `corps` sur un handler qui exige un [`Inscription`] validé.
    async fn poster(corps: &'static str, content_type: Option<&str>) -> (StatusCode, Value) {
        async fn handler(ValidatedJson(recu): ValidatedJson<Inscription>) -> String {
            recu.email
        }

        let mut requete = Request::builder().method("POST").uri("/");
        if let Some(content_type) = content_type {
            requete = requete.header(header::CONTENT_TYPE, content_type);
        }

        let reponse = Router::new()
            .route("/", post(handler))
            .oneshot(requete.body(Body::from(corps)).expect("requête valide"))
            .await
            .expect("le routeur doit répondre");

        let status = reponse.status();
        let octets = to_bytes(reponse.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        let corps = serde_json::from_slice(&octets)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&octets).into_owned()));

        (status, corps)
    }

    #[tokio::test]
    async fn un_corps_valide_est_extrait_tel_quel() {
        let (status, corps) = poster(
            r#"{"email":"alice@exemple.fr","age":30}"#,
            Some("application/json"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(corps, Value::String("alice@exemple.fr".to_owned()));
    }

    #[tokio::test]
    async fn un_corps_invalide_repond_422_avec_le_detail_par_champ() {
        let (status, corps) = poster(
            r#"{"email":"pas-une-adresse","age":12}"#,
            Some("application/json"),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(corps["status"], 422);
        assert_eq!(corps["errors"]["email"][0], "adresse électronique invalide");
        assert_eq!(corps["errors"]["age"][0], "âge minimum : 18 ans");
    }

    #[tokio::test]
    async fn un_json_malforme_repond_400_pas_500() {
        let (status, corps) =
            poster(r#"{"email":"alice@exemple.fr","#, Some("application/json")).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "obtenu : {corps}");
        assert_eq!(corps["status"], 400);
        assert!(
            corps["detail"].is_string(),
            "la cause doit rester lisible : {corps}"
        );
    }

    #[tokio::test]
    async fn un_content_type_absent_repond_400_pas_500() {
        let (status, corps) = poster(r#"{"email":"alice@exemple.fr","age":30}"#, None).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "obtenu : {corps}");
        assert_eq!(corps["status"], 400);
    }

    #[cfg(feature = "auth")]
    mod identite {
        use crate::config::{AuthConfig, Config, DatabaseConfig, DocsConfig, ServerConfig};
        use crate::extract::Identity;
        use crate::jwt::{self, Claims};
        use crate::state::{CoreState, HasAuth, HasCoreState};
        use axum::Router;
        use axum::body::{Body, to_bytes};
        use axum::http::{Request, StatusCode, header};
        use axum::routing::get;
        use sea_orm::DatabaseConnection;
        use tower::ServiceExt;

        const SECRET: &str = "un secret de test qui porte au moins trente-deux octets";

        /// Expiration lointaine, pour les cas où la validité temporelle n'est pas le sujet.
        const PLUS_TARD: i64 = 4_102_444_800;

        /// Ce que `state.rs` générera dans le projet : le `CoreState` composé, plus la
        /// ligne d'`impl HasAuth` qui donne au noyau l'accès au secret.
        #[derive(Clone)]
        struct AppState {
            core: CoreState,
        }

        impl HasCoreState for AppState {
            fn core(&self) -> &CoreState {
                &self.core
            }
        }

        impl HasAuth for AppState {}

        fn etat() -> AppState {
            let config = Config {
                env: "development".to_owned(),
                server: ServerConfig {
                    host: "127.0.0.1".to_owned(),
                    port: 8080,
                },
                database: DatabaseConfig {
                    url: "postgres://localhost/app".to_owned(),
                    max_connections: 10,
                    min_connections: 0,
                    connect_timeout_secs: 5,
                    acquire_timeout_secs: 5,
                    idle_timeout_secs: 600,
                    max_lifetime_secs: 1800,
                },
                docs: DocsConfig {
                    swagger_ui: true,
                    openapi_json: true,
                },
                auth: AuthConfig {
                    secret: SECRET.to_owned(),
                    access_ttl_secs: 900,
                    refresh_ttl_secs: 2_592_000,
                },
            };

            AppState {
                core: CoreState::new(DatabaseConnection::default(), config),
            }
        }

        fn jeton(exp: i64, secret: &str) -> String {
            jwt::signer(
                &Claims {
                    sub: "u1".to_owned(),
                    role: "admin".to_owned(),
                    exp,
                    iat: 0,
                    jti: "j1".to_owned(),
                },
                secret,
            )
            .expect("signature")
        }

        /// Appelle un handler protégé, avec ou sans en-tête `Authorization`.
        async fn appeler(autorisation: Option<&str>) -> (StatusCode, Option<String>, String) {
            async fn handler(identite: Identity) -> String {
                format!("{} {}", identite.user_id, identite.role)
            }

            let mut requete = Request::builder().uri("/");
            if let Some(autorisation) = autorisation {
                requete = requete.header(header::AUTHORIZATION, autorisation);
            }

            let reponse = Router::new()
                .route("/", get(handler))
                .with_state(etat())
                .oneshot(requete.body(Body::empty()).expect("requête valide"))
                .await
                .expect("le routeur doit répondre");

            let status = reponse.status();
            let content_type = reponse
                .headers()
                .get(header::CONTENT_TYPE)
                .map(|valeur| valeur.to_str().expect("content-type ASCII").to_owned());
            let octets = to_bytes(reponse.into_body(), usize::MAX)
                .await
                .expect("corps lisible");

            (
                status,
                content_type,
                String::from_utf8_lossy(&octets).into_owned(),
            )
        }

        #[tokio::test]
        async fn sans_en_tete_authorization_la_reponse_est_401_en_problem_json() {
            let (status, content_type, corps) = appeler(None).await;

            assert_eq!(status, StatusCode::UNAUTHORIZED, "obtenu : {corps}");
            assert_eq!(content_type.as_deref(), Some("application/problem+json"));
        }

        #[tokio::test]
        async fn un_jeton_invalide_ou_expire_rend_401() {
            let expire = format!("Bearer {}", jeton(0, SECRET));
            let (status, _, corps) = appeler(Some(&expire)).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "expiré, obtenu : {corps}");

            let autre_secret = format!(
                "Bearer {}",
                jeton(PLUS_TARD, "un autre secret tout aussi long ici")
            );
            let (status, _, corps) = appeler(Some(&autre_secret)).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "signé ailleurs : {corps}");

            let (status, _, corps) = appeler(Some("Bearer pas-un-jeton")).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "malformé : {corps}");
        }

        #[tokio::test]
        async fn un_jeton_valide_peuple_l_identite_depuis_les_claims() {
            let porteur = format!("Bearer {}", jeton(PLUS_TARD, SECRET));

            let (status, _, corps) = appeler(Some(&porteur)).await;

            assert_eq!(status, StatusCode::OK, "obtenu : {corps}");
            assert_eq!(corps, "u1 admin");
        }

        #[tokio::test]
        async fn un_en_tete_sans_le_schema_bearer_est_refuse() {
            let (status, _, corps) = appeler(Some(&jeton(PLUS_TARD, SECRET))).await;

            assert_eq!(
                status,
                StatusCode::UNAUTHORIZED,
                "un jeton nu, hors du schéma `Bearer`, n'est pas une autorisation : {corps}"
            );
        }
    }
}
