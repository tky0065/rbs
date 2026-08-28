use std::time::Instant;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use chrono::Utc;
use rbs_core::Identity;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::auth::guard::RequireRole;
use crate::auth::model::{Role, refresh_token};
use crate::router::router;
use crate::state::AppState;

/// Un mot de passe qui satisfait la validation du DTO, partagé par les tests.
const MOT_DE_PASSE: &str = "un mot de passe assez long";

/// Monte l'application sur la base décrite par `.env`, sans écouter sur le réseau.
///
/// Les migrations sont supposées appliquées : elles précèdent `cargo test`.
async fn application() -> Router {
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable — les migrations doivent avoir été appliquées");

    router(AppState::new(db, config).expect("état partagé constructible"))
}

/// Ouvre une connexion à la même base que l'application.
///
/// Deux garanties de la rotation ne s'observent que dans la table : l'empreinte qui y est
/// stockée, et le refus d'un jeton dont la date est passée — qu'il faut y fabriquer.
async fn connexion() -> DatabaseConnection {
    let config = rbs_core::Config::load().expect("configuration lisible");

    rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable")
}

/// Fait traverser le routeur à `requete`, et rend son statut avec son corps.
async fn appeler(api: &Router, requete: Request<Body>) -> (StatusCode, Value) {
    let reponse = api
        .clone()
        .oneshot(requete)
        .await
        .expect("l'application doit répondre");
    let statut = reponse.status();
    let octets = to_bytes(reponse.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    // Une réponse sans corps se lit `null` plutôt que d'arrêter le test.
    let corps = serde_json::from_slice(&octets).unwrap_or(Value::Null);

    (statut, corps)
}

fn sans_corps(methode: &str, chemin: &str) -> Request<Body> {
    Request::builder()
        .method(methode)
        .uri(chemin)
        .body(Body::empty())
        .expect("requête bien formée")
}

fn poster(chemin: &str, corps: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(chemin)
        .header("content-type", "application/json")
        .body(Body::from(corps.to_string()))
        .expect("requête bien formée")
}

/// Une adresse jamais inscrite : les tests partagent une base qu'ils ne vident pas.
fn email_neuf() -> String {
    format!("{}@exemple.test", Uuid::new_v4())
}

/// Inscrit `email` et rend le corps de la réponse.
async fn inscrire(api: &Router, email: &str) -> (StatusCode, Value) {
    appeler(
        api,
        poster(
            "/auth/register",
            json!({ "email": email, "password": MOT_DE_PASSE }),
        ),
    )
    .await
}

/// Tente une connexion et rend statut et corps.
async fn connecter(api: &Router, email: &str, mot_de_passe: &str) -> (StatusCode, Value) {
    appeler(
        api,
        poster(
            "/auth/login",
            json!({ "email": email, "password": mot_de_passe }),
        ),
    )
    .await
}

/// La garde tient avant même que le service existe : `Identity` refuse la requête sans
/// jamais atteindre le controller.
#[tokio::test]
async fn me_sans_jeton_rend_401() {
    let api = application().await;

    let (statut, corps) = appeler(&api, sans_corps("GET", "/auth/me")).await;

    assert_eq!(statut, StatusCode::UNAUTHORIZED);
    assert_eq!(corps["status"], 401, "{corps}");
}

/// Un jeton que le service n'a pas signé ne vaut pas mieux qu'aucun jeton.
#[tokio::test]
async fn me_avec_un_jeton_illisible_rend_401() {
    let api = application().await;
    let requete = Request::builder()
        .method("GET")
        .uri("/auth/me")
        .header("authorization", "Bearer pas-un-jeton")
        .body(Body::empty())
        .expect("requête bien formée");

    let (statut, corps) = appeler(&api, requete).await;

    assert_eq!(statut, StatusCode::UNAUTHORIZED);
    assert_eq!(corps["status"], 401, "{corps}");
}

#[tokio::test]
async fn inscription_rend_201_et_le_profil_cree() {
    let api = application().await;
    let email = email_neuf();

    let (statut, corps) = inscrire(&api, &email).await;

    assert_eq!(statut, StatusCode::CREATED, "{corps}");
    assert_eq!(corps["email"], email, "{corps}");
    assert_eq!(corps["role"], "user", "{corps}");
    assert!(corps["id"].is_string(), "{corps}");
}

/// Ni le hash ni le mot de passe reçu ne repartent vers le client.
#[tokio::test]
async fn le_hash_n_apparait_pas_dans_la_reponse() {
    let api = application().await;

    let (_, corps) = inscrire(&api, &email_neuf()).await;

    let texte = corps.to_string();
    assert!(
        !texte.contains("$argon2"),
        "hash dans la réponse :\n{texte}"
    );
    assert!(
        !texte.contains(MOT_DE_PASSE),
        "mot de passe dans la réponse :\n{texte}"
    );
    assert!(
        !texte.contains("password"),
        "champ de mot de passe dans la réponse :\n{texte}"
    );
}

#[tokio::test]
async fn un_email_deja_pris_rend_409() {
    let api = application().await;
    let email = email_neuf();

    let (premier, corps) = inscrire(&api, &email).await;
    assert_eq!(premier, StatusCode::CREATED, "{corps}");

    let (second, corps) = inscrire(&api, &email).await;

    assert_eq!(second, StatusCode::CONFLICT, "{corps}");
}

/// Les deux échecs sont indiscernables : un corps qui différerait dirait à un attaquant
/// quelles adresses sont inscrites.
///
/// `request_id` est retiré avant la comparaison — il change à chaque requête par
/// construction, et corréler une réponse à une ligne de journal ne renseigne personne sur
/// l'existence d'un compte.
#[tokio::test]
async fn mot_de_passe_errone_et_email_inconnu_rendent_la_meme_401() {
    let api = application().await;
    let inscrit = email_neuf();
    inscrire(&api, &inscrit).await;

    let (statut_faux, mut corps_faux) =
        connecter(&api, &inscrit, "un tout autre mot de passe").await;
    let (statut_inconnu, mut corps_inconnu) = connecter(&api, &email_neuf(), MOT_DE_PASSE).await;

    assert_eq!(statut_faux, StatusCode::UNAUTHORIZED, "{corps_faux}");
    assert_eq!(statut_inconnu, StatusCode::UNAUTHORIZED, "{corps_inconnu}");

    for corps in [&mut corps_faux, &mut corps_inconnu] {
        if let Some(objet) = corps.as_object_mut() {
            objet.remove("request_id");
        }
    }

    assert_eq!(
        corps_faux, corps_inconnu,
        "les deux échecs doivent être indiscernables"
    );
}

/// Le temps de réponse ne renseigne pas davantage que le corps.
///
/// Sans vérification sur le chemin « adresse inconnue », la réponse tombe en une fraction
/// de milliseconde là où Argon2 en coûte plusieurs dizaines : cet écart seul énumère les
/// comptes. Le rapport toléré est large — il sépare l'absence de vérification, d'un ordre
/// de grandeur, d'un hachage à vide, qui coûte le même temps.
#[tokio::test]
async fn un_email_inconnu_coute_le_meme_temps_qu_un_mot_de_passe_faux() {
    let api = application().await;
    let inscrit = email_neuf();
    inscrire(&api, &inscrit).await;

    // Un tour à vide : le hash de comparaison se calcule au premier passage, et son coût
    // ne doit pas être imputé à la mesure.
    connecter(&api, &email_neuf(), MOT_DE_PASSE).await;

    let depart = Instant::now();
    connecter(&api, &inscrit, "un tout autre mot de passe").await;
    let mot_de_passe_faux = depart.elapsed();

    let depart = Instant::now();
    connecter(&api, &email_neuf(), MOT_DE_PASSE).await;
    let email_inconnu = depart.elapsed();

    assert!(
        email_inconnu * 5 >= mot_de_passe_faux,
        "une adresse inconnue répond en {email_inconnu:?} contre {mot_de_passe_faux:?} \
         pour un mot de passe faux : l'écart énumère les comptes"
    );
}

/// Inscrit une adresse neuve et ouvre une session : l'identifiant du compte et sa paire.
async fn ouvrir_session(api: &Router) -> (Uuid, Value) {
    let email = email_neuf();
    let (_, profil) = inscrire(api, &email).await;
    let id = Uuid::parse_str(
        profil["id"]
            .as_str()
            .expect("le profil porte un identifiant"),
    )
    .expect("identifiant lisible");

    let (statut, paire) = connecter(api, &email, MOT_DE_PASSE).await;
    assert_eq!(statut, StatusCode::OK, "{paire}");

    (id, paire)
}

/// Le jeton de rafraîchissement d'une paire.
fn refresh_de(paire: &Value) -> String {
    paire["refresh_token"]
        .as_str()
        .expect("la paire doit porter un jeton de rafraîchissement")
        .to_owned()
}

async fn rafraichir(api: &Router, jeton: &str) -> (StatusCode, Value) {
    appeler(
        api,
        poster("/auth/refresh", json!({ "refresh_token": jeton })),
    )
    .await
}

/// L'unique ligne de `refresh_tokens` ouverte pour `user_id`.
///
/// La recherche porte sur le compte et non sur l'empreinte : chercher par ce qu'on veut
/// vérifier ferait échouer le test à la lecture, sans jamais atteindre l'assertion.
async fn ligne_de_session(db: &DatabaseConnection, user_id: Uuid) -> refresh_token::Model {
    refresh_token::Entity::find()
        .filter(refresh_token::Column::UserId.eq(user_id))
        .one(db)
        .await
        .expect("la table doit être interrogeable")
        .expect("la session ouverte doit avoir sa ligne")
}

#[tokio::test]
async fn un_refresh_valide_rend_une_nouvelle_paire() {
    let api = application().await;
    let (_, paire) = ouvrir_session(&api).await;

    let (statut, nouvelle) = rafraichir(&api, &refresh_de(&paire)).await;

    assert_eq!(statut, StatusCode::OK, "{nouvelle}");
    assert_eq!(nouvelle["token_type"], "Bearer", "{nouvelle}");
    assert_ne!(
        nouvelle["refresh_token"], paire["refresh_token"],
        "le jeton de rafraîchissement doit tourner"
    );
    assert_ne!(
        nouvelle["access_token"], paire["access_token"],
        "le jeton d'accès doit être réémis"
    );
}

#[tokio::test]
async fn l_ancien_refresh_est_ensuite_refuse() {
    let api = application().await;
    let ancien = refresh_de(&ouvrir_session(&api).await.1);

    let (statut, corps) = rafraichir(&api, &ancien).await;
    assert_eq!(statut, StatusCode::OK, "{corps}");

    let (rejeu, corps) = rafraichir(&api, &ancien).await;

    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{corps}");
}

/// La ligne est reculée dans le passé plutôt que forgée : elle garde ainsi tout ce que
/// `login` y a mis, et seule son expiration la disqualifie.
#[tokio::test]
async fn un_refresh_expire_rend_401() {
    let api = application().await;
    let db = connexion().await;
    let (compte, paire) = ouvrir_session(&api).await;
    let jeton = refresh_de(&paire);

    let mut ligne: refresh_token::ActiveModel = ligne_de_session(&db, compte).await.into();
    ligne.expires_at = Set((Utc::now() - chrono::Duration::seconds(1)).fixed_offset());
    ligne.update(&db).await.expect("expiration reculée");

    let (statut, corps) = rafraichir(&api, &jeton).await;

    assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
}

/// Une base lue par un tiers ne lui donne aucune session utilisable.
#[tokio::test]
async fn la_table_porte_l_empreinte_et_jamais_le_jeton() {
    let api = application().await;
    let db = connexion().await;
    let (compte, paire) = ouvrir_session(&api).await;
    let jeton = refresh_de(&paire);

    let ligne = ligne_de_session(&db, compte).await;

    assert_eq!(
        ligne.token_hash,
        rbs_core::token::fingerprint(&jeton),
        "la colonne ne porte pas l'empreinte du jeton"
    );
    assert_ne!(ligne.token_hash, jeton, "le jeton lui-même est stocké");

    let en_clair = refresh_token::Entity::find()
        .filter(refresh_token::Column::TokenHash.eq(jeton.clone()))
        .one(&db)
        .await
        .expect("la table doit être interrogeable");

    assert!(
        en_clair.is_none(),
        "une ligne porte le jeton remis au client"
    );
}

async fn deconnecter(api: &Router, jeton: &str) -> (StatusCode, Value) {
    appeler(
        api,
        poster("/auth/logout", json!({ "refresh_token": jeton })),
    )
    .await
}

#[tokio::test]
async fn logout_rend_204() {
    let api = application().await;
    let (_, paire) = ouvrir_session(&api).await;

    let (statut, corps) = deconnecter(&api, &refresh_de(&paire)).await;

    assert_eq!(statut, StatusCode::NO_CONTENT, "{corps}");
    assert_eq!(corps, Value::Null, "un 204 ne porte pas de corps");
}

#[tokio::test]
async fn le_refresh_revoque_rend_401() {
    let api = application().await;
    let (_, paire) = ouvrir_session(&api).await;
    let jeton = refresh_de(&paire);

    let (statut, corps) = deconnecter(&api, &jeton).await;
    assert_eq!(statut, StatusCode::NO_CONTENT, "{corps}");

    let (statut, corps) = rafraichir(&api, &jeton).await;

    assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
}

/// Se déconnecter d'un appareil ne déconnecte pas les autres.
///
/// C'est la garantie que ce lot ajoute : la révocation porte sur la ligne présentée, et
/// non sur le compte qui la détient.
#[tokio::test]
async fn les_autres_sessions_du_meme_compte_restent_valides() {
    let api = application().await;
    let email = email_neuf();
    inscrire(&api, &email).await;

    let (_, premiere) = connecter(&api, &email, MOT_DE_PASSE).await;
    let (_, seconde) = connecter(&api, &email, MOT_DE_PASSE).await;

    let (statut, corps) = deconnecter(&api, &refresh_de(&premiere)).await;
    assert_eq!(statut, StatusCode::NO_CONTENT, "{corps}");

    let (statut, corps) = rafraichir(&api, &refresh_de(&seconde)).await;

    assert_eq!(
        statut,
        StatusCode::OK,
        "la seconde session a été fermée avec la première : {corps}"
    );
}

/// Une route protégée, montée pour les tests seuls.
///
/// Le fragment n'en livre aucune : c'est à vous d'en poser sur vos propres routes, et
/// voici comment. Le handler se contente d'exiger le rôle avant de répondre.
async fn route_reservee_aux_admins() -> Router {
    async fn reservee(identite: Identity) -> rbs_core::Result<StatusCode> {
        identite.require_role(Role::Admin)?;

        Ok(StatusCode::OK)
    }

    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");

    Router::new()
        .route("/reserve", get(reservee))
        .with_state(AppState::new(db, config).expect("état partagé constructible"))
}

fn avec_jeton(methode: &str, chemin: &str, jeton: &str) -> Request<Body> {
    Request::builder()
        .method(methode)
        .uri(chemin)
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::empty())
        .expect("requête bien formée")
}

/// Le jeton d'accès d'une paire.
fn acces_de(paire: &Value) -> String {
    paire["access_token"]
        .as_str()
        .expect("la paire doit porter un jeton d'accès")
        .to_owned()
}

/// Inscrit un compte, le promeut administrateur, et ouvre une session à ce titre.
///
/// La promotion passe par la base : l'inscription rend toujours un `user`, par défaut de
/// la table, et le rôle ne voyage que dans un jeton émis après coup.
async fn ouvrir_session_admin(api: &Router, db: &DatabaseConnection) -> Value {
    let email = email_neuf();
    let (_, profil) = inscrire(api, &email).await;
    let id = Uuid::parse_str(
        profil["id"]
            .as_str()
            .expect("le profil porte un identifiant"),
    )
    .expect("identifiant lisible");

    let compte = crate::auth::model::user::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("la table doit être interrogeable")
        .expect("le compte inscrit doit exister");

    let mut promu: crate::auth::model::user::ActiveModel = compte.into();
    promu.role = Set(Role::Admin);
    promu.update(db).await.expect("compte promu");

    let (statut, paire) = connecter(api, &email, MOT_DE_PASSE).await;
    assert_eq!(statut, StatusCode::OK, "{paire}");

    paire
}

/// Sans jeton, la réponse dit « identifie-toi », et non « tu n'as pas le droit ».
///
/// Les deux se confondent aisément : ici c'est l'extractor `Identity` qui tranche, avant
/// que le corps du handler — et donc la garde — s'exécute.
#[tokio::test]
async fn sans_jeton_la_route_admin_rend_401() {
    let api = route_reservee_aux_admins().await;

    let (statut, corps) = appeler(&api, sans_corps("GET", "/reserve")).await;

    assert_eq!(statut, StatusCode::UNAUTHORIZED, "{corps}");
    assert_ne!(statut, StatusCode::FORBIDDEN, "{corps}");
}

#[tokio::test]
async fn un_user_sur_une_route_admin_rend_403() {
    let api = application().await;
    let reservee = route_reservee_aux_admins().await;
    let (_, paire) = ouvrir_session(&api).await;

    let (statut, corps) =
        appeler(&reservee, avec_jeton("GET", "/reserve", &acces_de(&paire))).await;

    assert_eq!(statut, StatusCode::FORBIDDEN, "{corps}");
}

#[tokio::test]
async fn un_admin_sur_la_meme_route_rend_200() {
    let api = application().await;
    let db = connexion().await;
    let reservee = route_reservee_aux_admins().await;
    let paire = ouvrir_session_admin(&api, &db).await;

    let (statut, corps) =
        appeler(&reservee, avec_jeton("GET", "/reserve", &acces_de(&paire))).await;

    assert_eq!(statut, StatusCode::OK, "{corps}");
}

#[tokio::test]
async fn me_rend_le_profil_de_l_appelant() {
    let api = application().await;
    let email = email_neuf();
    let (_, inscrit) = inscrire(&api, &email).await;
    let (_, paire) = connecter(&api, &email, MOT_DE_PASSE).await;

    let (statut, profil) = appeler(&api, avec_jeton("GET", "/auth/me", &acces_de(&paire))).await;

    assert_eq!(statut, StatusCode::OK, "{profil}");
    assert_eq!(profil["id"], inscrit["id"], "{profil}");
    assert_eq!(profil["email"], email, "{profil}");
    assert_eq!(profil["role"], "user", "{profil}");
}

/// Le document tel que le serveur le publie.
async fn document_openapi(api: &Router) -> Value {
    let (statut, document) = appeler(api, sans_corps("GET", "/api-docs/openapi.json")).await;

    assert_eq!(statut, StatusCode::OK, "le document doit être exposé");

    document
}

#[tokio::test]
async fn le_document_openapi_porte_les_cinq_chemins_d_auth() {
    let api = application().await;

    let document = document_openapi(&api).await;

    for chemin in [
        "/auth/register",
        "/auth/login",
        "/auth/refresh",
        "/auth/logout",
        "/auth/me",
    ] {
        assert!(
            document["paths"][chemin].is_object(),
            "`{chemin}` ne figure pas dans le document"
        );
    }
}

/// Un client généré depuis ce document doit savoir comment s'authentifier, et sur quelles
/// routes le faire.
#[tokio::test]
async fn le_schema_bearer_est_declare_et_me_le_porte() {
    let api = application().await;

    let document = document_openapi(&api).await;

    let schema = &document["components"]["securitySchemes"]["bearer"];
    assert_eq!(schema["type"], "http", "{schema}");
    assert_eq!(schema["scheme"], "bearer", "{schema}");
    assert_eq!(schema["bearerFormat"], "JWT", "{schema}");

    let securite = &document["paths"]["/auth/me"]["get"]["security"];
    assert!(
        securite.is_array() && !securite.as_array().expect("tableau").is_empty(),
        "`/auth/me` ne déclare pas exiger de jeton : {securite}"
    );
    assert!(
        securite[0]["bearer"].is_array(),
        "`/auth/me` n'exige pas le schéma `bearer` : {securite}"
    );
}

/// `refresh` et `logout` s'authentifient par leur corps : leur apposer le schéma
/// décrirait une exigence que le serveur ne pose pas.
#[tokio::test]
async fn les_routes_sans_en_tete_ne_declarent_pas_le_schema() {
    let api = application().await;

    let document = document_openapi(&api).await;

    for chemin in [
        "/auth/register",
        "/auth/login",
        "/auth/refresh",
        "/auth/logout",
    ] {
        assert!(
            document["paths"][chemin]["post"]["security"].is_null(),
            "`{chemin}` déclare exiger un jeton alors qu'il n'en lit aucun"
        );
    }
}
