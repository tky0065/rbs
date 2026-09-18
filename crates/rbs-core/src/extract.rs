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
#[cfg(feature = "auth")]
use sea_orm::prelude::Uuid;

/// Schéma d'autorisation attendu, casse comprise dans la comparaison.
#[cfg(feature = "auth")]
const SCHEMA: &str = "bearer";

/// En-tête portant une clé d'API. Insensible à la casse, comme tout nom d'en-tête HTTP.
#[cfg(feature = "auth")]
const API_KEY: &str = "x-api-key";

/// Identité authentifiée, extraite du jeton porté par la requête.
///
/// L'extracteur lit les en-têtes et ne touche pas au corps : un extracteur qui le
/// consommerait interdirait à [`ValidatedJson`] de le lire ensuite.
#[cfg(feature = "auth")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Identity {
    /// Identifiant de l'utilisateur, tel que porté par `sub`.
    pub user_id: String,
    /// Rôle en clair. L'enum `Role` est généré dans le projet, hors de portée du noyau.
    pub role: String,
}

#[cfg(feature = "auth")]
impl Identity {
    /// L'identifiant de l'appelant, lu comme UUID.
    ///
    /// `sub` est signé, mais rien ne garantit qu'un jeton émis par une version antérieure
    /// du service y ait mis un UUID : un `sub` illisible vaut un jeton invalide.
    pub fn user_uuid(&self) -> crate::Result<Uuid> {
        Uuid::parse_str(&self.user_id).map_err(|_| Error::Unauthorized)
    }
}

#[cfg(feature = "auth")]
impl<S: HasAuth> FromRequestParts<S> for Identity {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Le jeton d'abord : c'est le justificatif le plus spécifique, et un mandataire qui
        // injecterait une clé de service ne doit pas supplanter celui que l'appelant a
        // présenté.
        if let Some(token) = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(bearer)
        {
            let claims = crate::jwt::verify(token, &state.auth().secret)?;
            state.accept_in(&claims, &mut parts.extensions).await?;

            return Ok(Self {
                user_id: claims.sub,
                role: claims.role,
            });
        }

        let key = parts
            .headers
            .get(API_KEY)
            .and_then(|value| value.to_str().ok())
            .ok_or(Error::Unauthorized)?
            .to_owned();

        // Le projet est seul à savoir ce qu'est une clé : le noyau n'a ni la table ni la
        // règle. Le défaut du trait refuse, un projet sans le fragment n'ouvre donc rien.
        let claims = state.accept_key(&key, &mut parts.extensions).await?;

        Ok(Self {
            user_id: claims.sub,
            role: claims.role,
        })
    }
}

/// Isole le jeton d'un en-tête `Authorization: Bearer <token>`.
///
/// La RFC 7235 déclare le schéma insensible à la casse ; un client qui envoie `bearer`
/// est conforme, et le refuser serait un bug difficile à diagnostiquer côté appelant.
#[cfg(feature = "auth")]
fn bearer(header: &str) -> Option<&str> {
    let (schema, token) = header.split_once(' ')?;

    schema.eq_ignore_ascii_case(SCHEMA).then(|| token.trim())
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
    struct Registration {
        #[validate(email(message = "adresse électronique invalide"))]
        email: String,
        #[validate(range(min = 18, message = "âge minimum : 18 ans"))]
        age: u8,
    }

    /// Poste `body` sur un handler qui exige un [`Registration`] validé.
    async fn post_json(body: &'static str, content_type: Option<&str>) -> (StatusCode, Value) {
        async fn handler(ValidatedJson(recu): ValidatedJson<Registration>) -> String {
            recu.email
        }

        let mut requete = Request::builder().method("POST").uri("/");
        if let Some(content_type) = content_type {
            requete = requete.header(header::CONTENT_TYPE, content_type);
        }

        let response = Router::new()
            .route("/", post(handler))
            .oneshot(requete.body(Body::from(body)).expect("requête valide"))
            .await
            .expect("le router doit répondre");

        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        let body = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));

        (status, body)
    }

    #[tokio::test]
    async fn a_valid_body_is_extracted_as_is() {
        let (status, body) = post_json(
            r#"{"email":"alice@exemple.fr","age":30}"#,
            Some("application/json"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, Value::String("alice@exemple.fr".to_owned()));
    }

    #[tokio::test]
    async fn an_invalid_body_answers_422_with_the_per_field_detail() {
        let (status, body) = post_json(
            r#"{"email":"pas-une-adresse","age":12}"#,
            Some("application/json"),
        )
        .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["status"], 422);
        assert_eq!(body["errors"]["email"][0], "adresse électronique invalide");
        assert_eq!(body["errors"]["age"][0], "âge minimum : 18 ans");
    }

    #[tokio::test]
    async fn malformed_json_answers_400_not_500() {
        let (status, body) =
            post_json(r#"{"email":"alice@exemple.fr","#, Some("application/json")).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "obtenu : {body}");
        assert_eq!(body["status"], 400);
        assert!(
            body["detail"].is_string(),
            "la cause doit rester lisible : {body}"
        );
    }

    #[tokio::test]
    async fn a_missing_content_type_answers_400_not_500() {
        let (status, body) = post_json(r#"{"email":"alice@exemple.fr","age":30}"#, None).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "obtenu : {body}");
        assert_eq!(body["status"], 400);
    }

    #[cfg(feature = "auth")]
    mod identite {
        use crate::Error;
        use crate::config::{AuthConfig, Config, DatabaseConfig, DocsConfig, ServerConfig};
        use crate::extract::Identity;
        use crate::jwt::{self, Claims};
        use crate::state::{CoreState, HasAuth, HasCoreState};
        use axum::Router;
        use axum::body::{Body, to_bytes};
        use axum::http::{Request, StatusCode, header};
        use axum::routing::get;
        use sea_orm::DatabaseConnection;
        use sea_orm::prelude::Uuid;
        use tower::ServiceExt;

        const SECRET: &str = "un secret de test qui porte au moins trente-deux octets";

        /// Expiration lointaine, pour les cas où la validité temporelle n'est pas le sujet.
        const LATER: i64 = 4_102_444_800;

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

        fn state() -> AppState {
            let config = Config {
                env: "development".to_owned(),
                server: ServerConfig {
                    host: "127.0.0.1".to_owned(),
                    port: 8080,
                    timeout_secs: 30,
                    shutdown_timeout_secs: 30,
                    lang: crate::lang::Lang::Fr,
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

        fn token(exp: i64, secret: &str) -> String {
            jwt::sign(
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
        async fn call(autorisation: Option<&str>) -> (StatusCode, Option<String>, String) {
            async fn handler(identite: Identity) -> String {
                format!("{} {}", identite.user_id, identite.role)
            }

            let mut requete = Request::builder().uri("/");
            if let Some(autorisation) = autorisation {
                requete = requete.header(header::AUTHORIZATION, autorisation);
            }

            let response = Router::new()
                .route("/", get(handler))
                .with_state(state())
                .oneshot(requete.body(Body::empty()).expect("requête valide"))
                .await
                .expect("le router doit répondre");

            let status = response.status();
            let content_type = response
                .headers()
                .get(header::CONTENT_TYPE)
                .map(|value| value.to_str().expect("content-type ASCII").to_owned());
            let bytes = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("corps lisible");

            (
                status,
                content_type,
                String::from_utf8_lossy(&bytes).into_owned(),
            )
        }

        #[tokio::test]
        async fn without_an_authorization_header_the_response_is_401_in_problem_json() {
            let (status, content_type, body) = call(None).await;

            assert_eq!(status, StatusCode::UNAUTHORIZED, "obtenu : {body}");
            assert_eq!(content_type.as_deref(), Some("application/problem+json"));
        }

        #[tokio::test]
        async fn an_invalid_or_expired_token_returns_401() {
            let expire = format!("Bearer {}", token(0, SECRET));
            let (status, _, body) = call(Some(&expire)).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "expiré, obtenu : {body}");

            let autre_secret = format!(
                "Bearer {}",
                token(LATER, "un other secret tout aussi long ici")
            );
            let (status, _, body) = call(Some(&autre_secret)).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "signé ailleurs : {body}");

            let (status, _, body) = call(Some("Bearer pas-un-token")).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "malformé : {body}");
        }

        #[tokio::test]
        async fn a_valid_token_populates_the_identity_from_the_claims() {
            let bearer = format!("Bearer {}", token(LATER, SECRET));

            let (status, _, body) = call(Some(&bearer)).await;

            assert_eq!(status, StatusCode::OK, "obtenu : {body}");
            assert_eq!(body, "u1 admin");
        }

        /// Un état qui refuse tout jeton, quelle que soit sa signature : ce que le projet
        /// écrit quand il relit le compte et que la révocation l'a vidé.
        #[derive(Clone)]
        struct Refusant(AppState);

        impl HasCoreState for Refusant {
            fn core(&self) -> &CoreState {
                self.0.core()
            }
        }

        impl HasAuth for Refusant {
            async fn accept(&self, _: &Claims) -> Result<(), crate::Error> {
                Err(crate::Error::Unauthorized)
            }
        }

        #[tokio::test]
        async fn a_state_that_refuses_a_claim_turns_a_signed_token_into_401() {
            async fn handler(identite: Identity) -> String {
                identite.user_id
            }

            let response = Router::new()
                .route("/", get(handler))
                .with_state(Refusant(state()))
                .oneshot(
                    Request::builder()
                        .uri("/")
                        .header(
                            header::AUTHORIZATION,
                            format!("Bearer {}", token(LATER, SECRET)),
                        )
                        .body(Body::empty())
                        .expect("requête valide"),
                )
                .await
                .expect("le router doit répondre");

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        /// Ce qu'un état dépose depuis `accept_in` : le projet y met ce qu'il a relu du
        /// compte.
        #[derive(Clone)]
        struct Relu(String);

        #[derive(Clone)]
        struct Deposant(AppState);

        impl HasCoreState for Deposant {
            fn core(&self) -> &CoreState {
                self.0.core()
            }
        }

        impl HasAuth for Deposant {
            async fn accept_in(
                &self,
                claims: &Claims,
                extensions: &mut axum::http::Extensions,
            ) -> Result<(), crate::Error> {
                extensions.insert(Relu(claims.sub.clone()));
                Ok(())
            }
        }

        /// Ce qu'`accept_in` dépose, l'extracteur qui suit `Identity` le trouve : c'est ce
        /// qui épargne à une garde de relire le compte que le projet vient de lire.
        #[tokio::test]
        async fn what_accept_in_leaves_in_the_extensions_reaches_the_next_extractor() {
            async fn handler(_: Identity, axum::Extension(relu): axum::Extension<Relu>) -> String {
                relu.0
            }

            let response = Router::new()
                .route("/", get(handler))
                .with_state(Deposant(state()))
                .oneshot(
                    Request::builder()
                        .uri("/")
                        .header(
                            header::AUTHORIZATION,
                            format!("Bearer {}", token(LATER, SECRET)),
                        )
                        .body(Body::empty())
                        .expect("requête valide"),
                )
                .await
                .expect("le router doit répondre");

            assert_eq!(response.status(), StatusCode::OK);
            let corps = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("corps lisible");
            assert_eq!(&corps[..], b"u1");
        }

        #[test]
        fn a_uuid_subject_reads_back_as_the_user_uuid() {
            let id = Uuid::from_u128(0x0192_1f5e_7a3b_7c4d_8e9f_a0b1_c2d3_e4f5);
            let identite = Identity {
                user_id: id.to_string(),
                role: "user".into(),
            };

            assert_eq!(identite.user_uuid().expect("un UUID se lit"), id);
        }

        #[test]
        fn a_subject_that_is_not_a_uuid_is_unauthorized() {
            let identite = Identity {
                user_id: "42".into(),
                role: "user".into(),
            };

            assert!(matches!(identite.user_uuid(), Err(Error::Unauthorized)));
        }

        #[tokio::test]
        async fn a_header_without_the_bearer_scheme_is_rejected() {
            let (status, _, body) = call(Some(&token(LATER, SECRET))).await;

            assert_eq!(
                status,
                StatusCode::UNAUTHORIZED,
                "un token nu, hors du schéma `Bearer`, n'est pas une autorisation : {body}"
            );
        }

        /// Un état qui accepte une clé unique et lui donne un rôle : ce que le fragment
        /// `api-keys` écrira, réduit à ce que l'extracteur doit en voir.
        #[derive(Clone)]
        struct AvecCles(AppState);

        impl HasCoreState for AvecCles {
            fn core(&self) -> &CoreState {
                self.0.core()
            }
        }

        impl HasAuth for AvecCles {
            async fn accept_key(
                &self,
                key: &str,
                extensions: &mut axum::http::Extensions,
            ) -> Result<Claims, crate::Error> {
                if key != "rbs_la_bonne" {
                    return Err(crate::Error::Unauthorized);
                }
                extensions.insert(Relu("par la clé".to_owned()));
                Ok(Claims {
                    sub: "u7".to_owned(),
                    role: "user".to_owned(),
                    exp: LATER,
                    iat: 0,
                    jti: "cle-1".to_owned(),
                })
            }
        }

        /// Appelle un handler protégé en présentant les en-têtes donnés.
        async fn call_with(autorisation: Option<&str>, cle: Option<&str>) -> (StatusCode, String) {
            async fn handler(identite: Identity) -> String {
                format!("{} {}", identite.user_id, identite.role)
            }

            let mut requete = Request::builder().uri("/");
            if let Some(autorisation) = autorisation {
                requete = requete.header(header::AUTHORIZATION, autorisation);
            }
            if let Some(cle) = cle {
                requete = requete.header("x-api-key", cle);
            }

            let response = Router::new()
                .route("/", get(handler))
                .with_state(AvecCles(state()))
                .oneshot(requete.body(Body::empty()).expect("requête valide"))
                .await
                .expect("le router doit répondre");

            let status = response.status();
            let bytes = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("corps lisible");

            (status, String::from_utf8_lossy(&bytes).into_owned())
        }

        #[tokio::test]
        async fn a_valid_key_alone_identifies_the_caller() {
            let (status, body) = call_with(None, Some("rbs_la_bonne")).await;

            assert_eq!(status, StatusCode::OK, "obtenu : {body}");
            assert_eq!(body, "u7 user");
        }

        #[tokio::test]
        async fn an_unknown_key_is_unauthorized() {
            let (status, _) = call_with(None, Some("rbs_pas_celle_la")).await;

            assert_eq!(status, StatusCode::UNAUTHORIZED);
        }

        /// Le jeton l'emporte : c'est le justificatif le plus spécifique, et un mandataire qui
        /// injecterait une clé de service ne doit pas supplanter celui que l'appelant présente.
        #[tokio::test]
        async fn a_bearer_token_wins_over_a_key_presented_at_the_same_time() {
            let bearer = format!("Bearer {}", token(LATER, SECRET));

            let (status, body) = call_with(Some(&bearer), Some("rbs_la_bonne")).await;

            assert_eq!(status, StatusCode::OK, "obtenu : {body}");
            assert_eq!(body, "u1 admin", "le jeton doit gouverner : {body}");
        }

        /// Ce que `accept_key` dépose est à portée de l'extracteur suivant, comme pour
        /// `accept_in` : sans quoi une garde relirait le compte que le service vient de lire.
        #[tokio::test]
        async fn what_accept_key_leaves_in_the_extensions_reaches_the_next_extractor() {
            async fn handler(_: Identity, axum::Extension(relu): axum::Extension<Relu>) -> String {
                relu.0
            }

            let response = Router::new()
                .route("/", get(handler))
                .with_state(AvecCles(state()))
                .oneshot(
                    Request::builder()
                        .uri("/")
                        .header("x-api-key", "rbs_la_bonne")
                        .body(Body::empty())
                        .expect("requête valide"),
                )
                .await
                .expect("le router doit répondre");

            assert_eq!(response.status(), StatusCode::OK);
            let corps = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("corps lisible");
            assert_eq!(&corps[..], b"par la cl\xc3\xa9");
        }

        /// Un état qui accepte **n'importe quelle** clé, la vide comprise : ce qu'écrirait un
        /// projet dont le jugement des clés est permissif, ou simplement bogué.
        #[derive(Clone)]
        struct ToutAccepter(AppState);

        impl HasCoreState for ToutAccepter {
            fn core(&self) -> &CoreState {
                self.0.core()
            }
        }

        impl HasAuth for ToutAccepter {
            async fn accept_key(
                &self,
                _: &str,
                _: &mut axum::http::Extensions,
            ) -> Result<Claims, crate::Error> {
                Ok(Claims {
                    sub: "u9".to_owned(),
                    role: "user".to_owned(),
                    exp: LATER,
                    iat: 0,
                    jti: "permissive".to_owned(),
                })
            }
        }

        /// L'absence de justificatif est tranchée par le noyau, et non déléguée au projet : un
        /// `accept_key` permissif ne doit pas pouvoir ouvrir une requête qui ne présente rien.
        #[tokio::test]
        async fn a_request_without_any_credential_is_refused_even_by_a_permissive_state() {
            async fn handler(identite: Identity) -> String {
                identite.user_id
            }

            let response = Router::new()
                .route("/", get(handler))
                .with_state(ToutAccepter(state()))
                .oneshot(
                    Request::builder()
                        .uri("/")
                        .body(Body::empty())
                        .expect("requête valide"),
                )
                .await
                .expect("le router doit répondre");

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }
}
