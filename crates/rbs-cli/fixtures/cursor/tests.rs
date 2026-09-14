use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use std::collections::HashSet;
use tower::ServiceExt;
use uuid::Uuid;

use crate::router::router;
use crate::state::AppState;

// Les tests de ce fichier joignent la base que décrit `.env`, et sont donc `#[ignore]` :
// `cargo test` ne les lance pas, `cargo test -- --ignored` les lance contre la base du
// projet, migrations appliquées.

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

/// Fait traverser le routeur à `request`, et rend son statut avec son corps.
async fn call(api: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = api
        .clone()
        .oneshot(request)
        .await
        .expect("l'application doit répondre");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("corps de réponse lisible");

    // La suppression ne rend aucun corps : il se lit `null` plutôt que d'arrêter le test.
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

    (status, body)
}

fn request(method: &str, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("requête bien formée")
}

fn without_body(method: &str, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .expect("requête bien formée")
}

/// Compare à la réponse la valeur envoyée pour `champ`.
fn compare(rendered: &Value, sent: &Value, champ: &str) {
    assert_eq!(rendered[champ], sent[champ], "« {champ} » mal rendu");
}

/// Corps de création dont les valeurs textuelles portent un suffixe tiré au sort.
///
/// Les champs uniques interdisent de rejouer deux fois la même valeur : le suffixe rend
/// chaque exécution indépendante des précédentes.
fn creation() -> Value {
    let suffix = Uuid::new_v4();

    json!({
        "title": format!("title-{suffix}"),
    })
}

fn modification() -> Value {
    let suffix = Uuid::new_v4();

    json!({
        "title": format!("title-modifie-{suffix}"),
    })
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_full_lifecycle_goes_through_the_api() {
    let api = application().await;
    let collection = "/articles";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    compare(&created, &sent, "title");

    let id = created["id"].as_str().expect("identifiant rendu");
    let resource = format!("{collection}/{id}");

    let (status, read) = call(&api, without_body("GET", &resource)).await;
    assert_eq!(status, StatusCode::OK, "relecture refusée : {read}");
    assert_eq!(read["id"], created["id"], "l'identifiant doit être stable");

    // L'`id` est un UUIDv7 et la liste trie du plus récent au plus ancien. Ce qui se
    // vérifie est que la ligne créée est sur la première page et que la page est
    // ordonnée — non qu'elle en occupe la première place : les tests tournent en
    // parallèle, et un autre peut écrire entre la création et la lecture.
    let premiere = format!("{collection}?per_page=50");
    let (status, page) = call(&api, without_body("GET", &premiere)).await;
    assert_eq!(status, StatusCode::OK, "liste refusée : {page}");

    let ids: Vec<&str> = page["data"]
        .as_array()
        .expect("la liste rend un tableau")
        .iter()
        .map(|ligne| ligne["id"].as_str().expect("identifiant rendu"))
        .collect();

    assert!(
        ids.contains(&created["id"].as_str().expect("identifiant rendu")),
        "la ligne créée est absente de la première page : {page}"
    );

    let mut decroissants = ids.clone();
    decroissants.sort_unstable_by(|gauche, droite| droite.cmp(gauche));
    assert_eq!(ids, decroissants, "la liste n'est pas triée : {page}");

    assert_eq!(
        page["meta"]["per_page"], 50,
        "la page doit annoncer la taille demandée : {page}"
    );

    let sent = modification();
    let mise_a_jour = request("PATCH", &resource, sent.clone());
    let (status, updated) = call(&api, mise_a_jour).await;
    assert_eq!(status, StatusCode::OK, "mise à jour refusée : {updated}");
    compare(&updated, &sent, "title");

    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");

    let (status, _) = call(&api, without_body("GET", &resource)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "elle répond encore");

    // Une seconde suppression ne trouve plus rien à supprimer. L'assertion vaut des deux
    // côtés de `--soft-delete` : c'est elle qui attrape une suppression logique dont la
    // condition de garde manquerait, et qui rendrait alors 204 indéfiniment.
    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "elle se supprime deux fois");
}

/// Deux créations à la suite portent des identifiants croissants.
///
/// C'est ce qui sépare un UUIDv7 d'un v4, et ce dont dépend la liste : elle trie sur
/// l'`id` pour rendre le plus récent en tête. Un test qui se contenterait de constater
/// la présence d'un UUID laisserait passer la régression.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn two_creations_in_a_row_carry_increasing_ids() {
    let api = application().await;
    let collection = "/articles";

    let (status, premier) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {premier}");
    let (status, second) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {second}");

    let lire = |rendered: &Value| {
        Uuid::parse_str(rendered["id"].as_str().expect("identifiant rendu"))
            .expect("identifiant lisible")
    };
    let (premier, second) = (lire(&premier), lire(&second));

    // Les tests jouent sur la base du `.env`, hors transaction : sans ces suppressions, la
    // table enfle de deux lignes à chaque exécution.
    for identifiant in [premier, second] {
        let resource = format!("{collection}/{identifiant}");
        let (status, _) = call(&api, without_body("DELETE", &resource)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
    }

    assert_eq!(
        premier.get_version_num(),
        7,
        "{premier} n'est pas un UUIDv7"
    );
    assert!(second > premier, "{second} ne suit pas {premier}");
}

/// La marche par curseur rend chaque ligne une fois, et s'éteint d'elle-même.
///
/// Trois créations lues par pages de deux franchissent au moins une frontière de page, là
/// où une borne inclusive rendrait deux fois la même ligne ; la dernière page, plus
/// courte, doit rendre `next` à `null`.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_cursor_walks_every_page_without_duplicates() {
    let api = application().await;
    let collection = "/articles";

    let mut crees = Vec::new();
    for _ in 0..3 {
        let (status, created) = call(&api, request("POST", collection, creation())).await;
        assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
        let id = created["id"].as_str().expect("identifiant rendu");
        crees.push(id.to_owned());
    }

    // Les tests tournent en parallèle sur la même base : une ligne qu'ils créent pendant
    // la marche porte un `id` plus grand, et ne paraît au plus que sur la première page ;
    // une ligne qu'ils suppriment manque, sans créer de doublon. Rien ne se conclut donc du
    // nombre de lignes parcourues.
    let mut vus = HashSet::new();
    let mut doublons = Vec::new();
    let mut marche = Vec::new();
    let mut chemin = format!("{collection}?per_page=2");
    let mut eteinte = false;

    // La borne change en échec un `next` qui ne s'éteindrait jamais, là où le test
    // tournerait sans fin.
    for _ in 0..10_000 {
        let (status, page) = call(&api, without_body("GET", &chemin)).await;
        assert_eq!(status, StatusCode::OK, "page refusée : {page}");

        for ligne in page["data"].as_array().expect("la liste rend un tableau") {
            let id = ligne["id"].as_str().expect("identifiant rendu").to_owned();
            if !vus.insert(id.clone()) {
                doublons.push(id.clone());
            }
            marche.push(id);
        }

        match page["meta"]["next"].as_str() {
            Some(next) => chemin = format!("{collection}?per_page=2&after={next}"),
            None => {
                eteinte = true;
                break;
            }
        }
    }

    for id in &crees {
        let resource = format!("{collection}/{id}");
        let (status, _) = call(&api, without_body("DELETE", &resource)).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
    }

    assert!(
        eteinte,
        "`next` ne s'est pas éteint en 10 000 pages : le curseur n'avance plus, ou la table \
         dépasse 20 000 lignes"
    );
    assert!(
        doublons.is_empty(),
        "lignes rendues deux fois : {doublons:?}"
    );
    for id in &crees {
        assert!(vus.contains(id), "« {id} » manque à la marche");
    }

    let mut decroissants = marche.clone();
    decroissants.sort_unstable_by(|gauche, droite| droite.cmp(gauche));
    assert_eq!(marche, decroissants, "la marche n'est pas décroissante");
}

/// La route de filtrage retient la ligne qui satisfait son propre critère.
///
/// Le rendu du filtre ne prouve rien : une condition mal traduite rend une page vide, et
/// seule une requête jouée contre la base le montre.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_filter_narrows_the_list() {
    let api = application().await;
    let collection = "/articles";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");

    let critere = json!({ "title": sent["title"] });
    let chemin = format!("{collection}/filter");
    let (status, page) = call(&api, request("POST", &chemin, critere)).await;
    assert_eq!(status, StatusCode::OK, "filtre refusé : {page}");

    let ids: Vec<&str> = page["data"]
        .as_array()
        .expect("la liste rend un tableau")
        .iter()
        .map(|ligne| ligne["id"].as_str().expect("identifiant rendu"))
        .collect();

    let id = created["id"].as_str().expect("identifiant rendu");
    assert!(
        ids.contains(&id),
        "la ligne créée doit satisfaire son propre critère : {page}"
    );

    let resource = format!("{collection}/{id}");
    let (status, _) = call(&api, without_body("DELETE", &resource)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "suppression refusée");
}

/// `%` et `_` sont des jokers de LIKE, et `!` le caractère qui les échappe : lus tels
/// quels, `contains: "%"` rendrait toute la table.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn contains_reads_percent_and_underscore_literally() {
    let api = application().await;
    let collection = "/articles";
    let champ = "title";
    let marque = Uuid::new_v4().simple().to_string()[..10].to_string();
    let pourcent = format!("{marque}%");
    let souligne = format!("{marque}_");
    let exclamation = format!("{marque}!");
    let temoin = format!("{marque}x");

    for valeur in [&pourcent, &souligne, &exclamation, &temoin] {
        let mut sent = creation();
        sent[champ] = json!(valeur);
        let (status, created) = call(&api, request("POST", collection, sent)).await;
        assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    }

    // Lu comme un joker, chacun de ces motifs retiendrait aussi les trois autres lignes.
    let chemin = format!("{collection}/filter");
    let mut ecarts = Vec::new();
    for attendue in [&pourcent, &souligne, &exclamation] {
        let critere = json!({ champ: { "contains": attendue } });
        let (status, page) = call(&api, request("POST", &chemin, critere)).await;
        assert_eq!(status, StatusCode::OK, "filtre refusé : {page}");

        let valeurs: Vec<&str> = page["data"]
            .as_array()
            .expect("la liste rend un tableau")
            .iter()
            .map(|ligne| ligne[champ].as_str().expect("texte rendu"))
            .collect();
        if valeurs != [attendue.as_str()] {
            ecarts.push(format!("« {attendue} » rend {valeurs:?}"));
        }
    }
    assert!(
        ecarts.is_empty(),
        "`contains` doit se lire à la lettre : {ecarts:?}"
    );
}

/// Une colonne de tri inconnue est une faute du client, et le refus la nomme.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_sort_column_returns_400() {
    let api = application().await;
    let critere = json!({ "sort": ["-inconnue"] });

    let (status, body) = call(&api, request("POST", "/articles/filter", critere)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["status"], 400, "{body}");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unknown_id_returns_404() {
    let api = application().await;
    let inconnu = Uuid::new_v4();
    let resource = format!("/articles/{inconnu}");

    let (status, body) = call(&api, without_body("GET", &resource)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404, "{body}");
}

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn an_unreadable_body_returns_400() {
    let api = application().await;
    let truncated = Request::builder()
        .method("POST")
        .uri("/articles")
        .header("content-type", "application/json")
        .body(Body::from("{"))
        .expect("requête bien formée");

    let (status, body) = call(&api, truncated).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}
