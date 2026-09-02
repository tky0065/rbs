//! Route de santé de l'application.
//!
//! Le noyau porte le contrôle ; c'est le projet généré qui décide où le monter et ce
//! qu'il y ajoute. Le handler est générique sur [`HasCoreState`], donc utilisable aussi
//! bien avec l'`AppState` d'un projet qu'avec un [`CoreState`](crate::state::CoreState)
//! nu.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use sea_orm::{DatabaseConnection, DbErr, RuntimeErr};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::HasCoreState;

/// Délai au terme duquel le ping de la base est abandonné.
///
/// Bien plus court que la borne des requêtes ordinaires : un contrôle de santé qui pend
/// laisse l'orchestrateur décider à la place du service, et un 503 rendu vite vaut mieux
/// qu'un verdict juste rendu trop tard.
const PING_TIMEOUT: Duration = Duration::from_secs(2);

/// Santé de l'application et de ses dépendances.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[non_exhaustive]
pub struct Health {
    /// Verdict d'ensemble.
    pub status: Status,
    /// Détail par dépendance.
    pub checks: Checks,
}

/// Verdict d'ensemble d'un contrôle de santé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Status {
    /// Toutes les dépendances répondent.
    Ok,
    /// Au moins une dépendance ne répond pas.
    Unavailable,
}

/// État de chaque dépendance contrôlée.
///
/// Les contrôles sont imbriqués sous `checks` plutôt qu'à la racine du corps, et les
/// dépendances du projet s'y ajoutent à plat, à côté de `database` : une supervision qui
/// lit `checks.database` ne change pas de chemin quand un cache est installé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[non_exhaustive]
pub struct Checks {
    /// État de la base de données.
    pub database: Check,
    /// État de chaque dépendance sondée par le projet, sous le nom qu'il lui a donné.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Check>,
}

/// Résultat d'un contrôle de dépendance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Check {
    /// La dépendance répond.
    Ok,
    /// La dépendance ne répond pas.
    Unreachable,
}

/// Monte `GET /health` sur un routeur.
pub fn routes<S>() -> Router<S>
where
    S: HasCoreState,
{
    Router::new().route("/health", get(handler::<S>))
}

/// Rend la santé de l'application, `503` dès que la base manque à l'appel.
///
/// N'interroge que la base : le squelette engendré délègue à [`report`], seul à savoir
/// quelles dépendances le projet a installées.
pub async fn handler<S>(State(state): State<S>) -> Response
where
    S: HasCoreState,
{
    report(state.core().db(), Vec::new()).await
}

/// Ce que [`report`] attend d'une base : savoir dire si elle répond.
///
/// Le trait existe pour que la borne du ping soit éprouvable sur une base joignable mais
/// muette, cas que `DatabaseConnection` ne sait pas représenter — son pool répond tout de
/// suite, connecté ou non.
pub trait Ping {
    /// Interroge la base et rend `Ok(())` quand elle répond.
    fn ping(&self) -> impl Future<Output = Result<(), DbErr>> + Send;
}

impl Ping for DatabaseConnection {
    /// Appel qualifié : la méthode inhérente porte le même nom, et `self.ping()` la
    /// choisirait ici sans que rien ne le dise.
    fn ping(&self) -> impl Future<Output = Result<(), DbErr>> + Send {
        DatabaseConnection::ping(self)
    }
}

/// Une dépendance à contrôler, sous le nom qu'elle portera dans le corps de la réponse.
pub struct Probe<'a> {
    name: &'static str,
    check: Pin<Box<dyn Future<Output = bool> + Send + 'a>>,
}

impl<'a> Probe<'a> {
    /// Nomme une dépendance et la façon de la joindre.
    ///
    /// Le futur rend `true` quand la dépendance répond. Ce qu'elle a répondu ne regarde
    /// pas le contrôle de santé : une erreur applicative n'est pas une panne.
    pub fn new<F>(name: &'static str, check: F) -> Self
    where
        F: Future<Output = bool> + Send + 'a,
    {
        Self {
            name,
            check: Box::pin(check),
        }
    }
}

impl std::fmt::Debug for Probe<'_> {
    /// Le futur d'une sonde n'est pas inspectable : seul son nom l'est.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Probe").field("name", &self.name).finish()
    }
}

/// Rend la santé de l'application, `503` dès qu'une dépendance manque à l'appel.
///
/// La base est toujours contrôlée ; `probes` ajoute les dépendances que le projet a
/// installées, chacune bornée par le même délai.
pub async fn report(db: &impl Ping, probes: Vec<Probe<'_>>) -> Response {
    let (ping, extras) = run_all(db.ping(), probes).await;
    let (status, health) = verdict(ping, extras);

    (status, axum::Json(health)).into_response()
}

/// Mène le ping de la base et toutes les sondes de front, chacun borné séparément.
///
/// Un `join` écrit à la main plutôt qu'une dépendance à `futures` : le noyau n'a besoin
/// que de cette seule combinaison, et la faire entrer ne vaut pas une crate de plus dans
/// tout projet engendré. Les contrôles déjà rendus ne sont plus interrogés ; les autres
/// sont repollés à chaque réveil, ce qui reste sans effet à cette échelle — un projet
/// compte ses dépendances sur les doigts d'une main.
async fn run_all<'a>(
    ping: impl Future<Output = Result<(), DbErr>> + Send + 'a,
    probes: Vec<Probe<'a>>,
) -> (Result<(), DbErr>, BTreeMap<String, Check>) {
    let mut ping = Box::pin(bounded_ping(ping));
    let mut sondes: Vec<_> = probes
        .into_iter()
        .map(|probe| (probe.name, Some(Box::pin(bounded_probe(probe.check)))))
        .collect();

    let mut rendu = None;
    let mut extras = BTreeMap::new();

    std::future::poll_fn(|cx| {
        if rendu.is_none()
            && let Poll::Ready(resultat) = ping.as_mut().poll(cx)
        {
            rendu = Some(resultat);
        }

        for (name, sonde) in &mut sondes {
            let Some(future) = sonde else { continue };
            if let Poll::Ready(check) = future.as_mut().poll(cx) {
                extras.insert((*name).to_owned(), check);
                *sonde = None;
            }
        }

        if rendu.is_some() && sondes.iter().all(|(_, sonde)| sonde.is_none()) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;

    (
        rendu.expect("le ping est rendu quand la boucle s'achève"),
        extras,
    )
}

/// Borne le ping dans le temps.
///
/// L'expiration prend la forme d'une erreur de connexion plutôt qu'un troisième cas :
/// une base qui ne répond pas dans le délai est injoignable, et [`verdict`] reste seul
/// juge du statut.
async fn bounded_ping<F>(ping: F) -> Result<(), DbErr>
where
    F: Future<Output = Result<(), DbErr>>,
{
    tokio::time::timeout(PING_TIMEOUT, ping)
        .await
        .unwrap_or_else(|_| {
            Err(DbErr::Conn(RuntimeErr::Internal(format!(
                "aucune réponse au ping en {} s",
                PING_TIMEOUT.as_secs()
            ))))
        })
}

/// Borne une sonde dans le temps.
///
/// Une dépendance qui n'a pas répondu dans le délai est injoignable : l'expiration n'est
/// pas un troisième cas, pour la même raison que sur le ping de la base.
async fn bounded_probe(check: Pin<Box<dyn Future<Output = bool> + Send + '_>>) -> Check {
    match tokio::time::timeout(PING_TIMEOUT, check).await {
        Ok(true) => Check::Ok,
        _ => Check::Unreachable,
    }
}

/// Traduit le résultat du ping en verdict.
///
/// Séparée du transport pour que la branche « base saine » reste couverte : sans base
/// démarrée, seule la branche 503 est atteignable par une requête réelle.
fn verdict(ping: Result<(), DbErr>, extras: BTreeMap<String, Check>) -> (StatusCode, Health) {
    let database = match ping {
        Ok(()) => Check::Ok,
        Err(error) => {
            // La cause part au journal et nulle part ailleurs : un contrôle de santé est
            // souvent exposé sans authentification.
            tracing::error!(error = %error, "base de données injoignable");
            Check::Unreachable
        }
    };

    let sain = database == Check::Ok && extras.values().all(|check| *check == Check::Ok);
    let (code, status) = if sain {
        (StatusCode::OK, Status::Ok)
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Status::Unavailable)
    };

    (
        code,
        Health {
            status,
            checks: Checks { database, extras },
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseConfig, DocsConfig, ServerConfig};
    use crate::state::CoreState;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use sea_orm::DatabaseConnection;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    fn config() -> Config {
        Config {
            env: "development".to_owned(),
            server: ServerConfig {
                host: "127.0.0.1".to_owned(),
                port: 8080,
                timeout_secs: 30,
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
            #[cfg(feature = "auth")]
            auth: crate::config::AuthConfig {
                secret: "un secret de test qui porte au moins trente-deux octets".to_owned(),
                access_ttl_secs: 900,
                refresh_ttl_secs: 2_592_000,
            },
        }
    }

    #[tokio::test]
    async fn an_unavailable_database_answers_503_not_200() {
        // `DatabaseConnection::default()` est un pool déconnecté : `ping` y échoue sans
        // qu'aucune base n'ait à tourner.
        let verdict = CoreState::new(DatabaseConnection::default(), config());

        let response = routes()
            .with_state(verdict)
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("requête valide"),
            )
            .await
            .expect("le router doit répondre");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        let body: Value = serde_json::from_slice(&bytes).expect("corps JSON");
        assert_eq!(
            body,
            json!({ "status": "unavailable", "checks": { "database": "unreachable" } })
        );
    }

    /// `report` sans aucune sonde doit rendre, à l'octet près, ce que `handler` rendait :
    /// un projet engendré avant ce jalon expose le même corps qu'hier.
    #[tokio::test]
    async fn report_without_probes_renders_the_body_it_rendered_before() {
        let db = DatabaseConnection::default();

        let response = report(&db, Vec::new()).await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        assert_eq!(
            std::str::from_utf8(&bytes).expect("corps UTF-8"),
            r#"{"status":"unavailable","checks":{"database":"unreachable"}}"#
        );
    }

    /// Une dépendance muette vaut 503 comme la base : le verdict est binaire, et une
    /// application dont le cache est injoignable n'a rien à faire dans la rotation.
    #[test]
    fn a_failing_probe_drops_the_verdict_though_the_database_answers() {
        let extras = BTreeMap::from([("cache".to_owned(), Check::Unreachable)]);

        let (status, health) = verdict(Ok(()), extras);

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            serde_json::to_value(health).expect("sérialisable"),
            json!({
                "status": "unavailable",
                "checks": { "database": "ok", "cache": "unreachable" }
            })
        );
    }

    /// Les sondes s'ajoutent au corps sans le réorganiser : `database` garde sa place, et
    /// une supervision qui la lit ne change pas de chemin.
    #[test]
    fn probes_that_all_answer_keep_the_verdict_at_200() {
        let extras = BTreeMap::from([
            ("cache".to_owned(), Check::Ok),
            ("storage".to_owned(), Check::Ok),
        ]);

        let (status, health) = verdict(Ok(()), extras);

        assert_eq!(status, StatusCode::OK);
        assert_eq!(health.status, Status::Ok);
    }

    /// Les sondes s'exécutent de front, et non l'une après l'autre : sans cela, chaque
    /// dépendance ajoutée allongerait `/health` d'une borne pour tout le monde, et quatre
    /// dépendances muettes tiendraient l'orchestrateur huit secondes.
    ///
    /// L'horloge est en pause : ce n'est pas une mesure de durée réelle mais une lecture
    /// du temps que les bornes ont fait avancer.
    #[tokio::test(start_paused = true)]
    async fn silent_probes_answer_within_one_bound_and_not_one_each() {
        let db = DatabaseConnection::default();
        let depart = tokio::time::Instant::now();

        let response = report(
            &db,
            vec![
                Probe::new("cache", std::future::pending()),
                Probe::new("storage", std::future::pending()),
            ],
        )
        .await;

        let ecoule = depart.elapsed();
        assert!(
            ecoule >= PING_TIMEOUT && ecoule < PING_TIMEOUT * 2,
            "les sondes ne sont pas menées de front : {ecoule:?}"
        );
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        let body: Value = serde_json::from_slice(&bytes).expect("corps JSON");
        assert_eq!(
            body,
            json!({
                "status": "unavailable",
                "checks": {
                    "database": "unreachable",
                    "cache": "unreachable",
                    "storage": "unreachable"
                }
            })
        );
    }

    /// L'ordre des clés du corps est celui du `BTreeMap`, et non celui de l'installation :
    /// deux réponses successives se comparent alors ligne à ligne, et un test
    /// d'intégration peut asserter le corps entier.
    #[test]
    fn the_body_orders_the_probes_by_name_and_keeps_database_first() {
        let extras = BTreeMap::from([
            ("storage".to_owned(), Check::Ok),
            ("cache".to_owned(), Check::Unreachable),
        ]);

        let (_, health) = verdict(Ok(()), extras);

        assert_eq!(
            serde_json::to_string(&health).expect("sérialisable"),
            r#"{"status":"unavailable","checks":{"database":"ok","cache":"unreachable","storage":"ok"}}"#
        );
    }

    /// Ce que `utoipa` fait d'un `#[serde(flatten)]` sur une carte se lit sur le document
    /// engendré, jamais sur une intuition : le schéma doit décrire le corps réel, où
    /// `database` est une propriété nommée et les sondes des clés libres.
    #[test]
    fn the_schema_declares_the_probes_as_free_keys_beside_database() {
        use utoipa::PartialSchema;

        let schema = serde_json::to_value(Checks::schema()).expect("schéma sérialisable");

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["required"], json!(["database"]));
        assert_eq!(
            schema["properties"]["database"]["$ref"],
            "#/components/schemas/Check"
        );
        assert_eq!(
            schema["additionalProperties"]["$ref"],
            "#/components/schemas/Check"
        );
    }

    #[test]
    fn a_healthy_database_gives_200_and_an_ok_status() {
        let (status, health) = verdict(Ok(()), BTreeMap::new());

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            serde_json::to_value(health).expect("sérialisable"),
            json!({ "status": "ok", "checks": { "database": "ok" } })
        );
    }

    #[test]
    fn an_unreachable_database_gives_503_and_names_the_failed_check() {
        let (status, health) = verdict(
            Err(DbErr::Conn(sea_orm::RuntimeErr::Internal(
                "connexion refusée".to_owned(),
            ))),
            BTreeMap::new(),
        );

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(health.checks.database, Check::Unreachable);
    }

    /// Un `/health` qui pend est pire qu'un `/health` qui répond 503 : l'orchestrateur
    /// qui l'interroge attend, et son propre délai décide à la place du service.
    ///
    /// La borne est éprouvée sur le futur du ping, et non à travers une requête :
    /// `DatabaseConnection` ne sait rendre qu'un pool connecté ou déconnecté, dont le
    /// ping répond immédiatement dans les deux cas.
    #[tokio::test(start_paused = true)]
    async fn a_ping_that_never_answers_becomes_a_503_rather_than_a_wait() {
        let depart = tokio::time::Instant::now();

        let (status, health) = verdict(bounded_ping(std::future::pending()).await, BTreeMap::new());

        assert!(
            depart.elapsed() >= PING_TIMEOUT,
            "le ping a été abandonné avant sa borne : {:?}",
            depart.elapsed()
        );
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(health.checks.database, Check::Unreachable);
    }

    /// Un ping qui répond dans la borne passe intact : l'enveloppe ne doit pas
    /// transformer le cas courant.
    #[tokio::test(start_paused = true)]
    async fn a_ping_that_answers_within_the_bound_keeps_its_verdict() {
        let (status, _) = verdict(
            bounded_ping(std::future::ready(Ok(()))).await,
            BTreeMap::new(),
        );

        assert_eq!(status, StatusCode::OK);
    }

    /// Une base joignable mais muette est le cas que `DatabaseConnection` ne sait pas
    /// représenter : son pool répond tout de suite, connecté ou non.
    struct BaseMuette;

    impl Ping for BaseMuette {
        async fn ping(&self) -> Result<(), DbErr> {
            std::future::pending().await
        }
    }

    /// La borne du ping ne vaut que si elle tient sur le chemin réel : c'est le statut de
    /// la réponse HTTP, et non le verdict interne, qui décide de la rotation.
    #[tokio::test(start_paused = true)]
    async fn a_database_that_never_answers_gives_a_503_over_http() {
        let app = Router::new().route(
            "/health",
            axum::routing::get(|| async { report(&BaseMuette, Vec::new()).await }),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("requête valide"),
            )
            .await
            .expect("le router doit répondre");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("corps lisible");
        let body: Value = serde_json::from_slice(&bytes).expect("corps JSON");
        assert_eq!(
            body,
            json!({ "status": "unavailable", "checks": { "database": "unreachable" } })
        );
    }

    #[test]
    fn the_database_error_detail_does_not_leak_into_the_response() {
        let (_, health) = verdict(
            Err(DbErr::Conn(sea_orm::RuntimeErr::Internal(
                "postgres://alice:s3cr3t@localhost/app injoignable".to_owned(),
            ))),
            BTreeMap::new(),
        );

        let rendered = serde_json::to_string(&health).expect("sérialisable");

        assert!(
            !rendered.contains("s3cr3t") && !rendered.contains("injoignable"),
            "le détail de l'erreur ne doit pas atteindre le client : {rendered}"
        );
    }
}
