use super::*;

#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_full_lifecycle_goes_through_the_api() {
    let api = application().await;
    let collection = "/subscribers";
    let sent = creation();

    let (status, created) = call(&api, request("POST", collection, sent.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");
    compare(&created, &sent, "email");
    compare(&created, &sent, "name");
    compare(&created, &sent, "confirmed");

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

    assert!(
        page["meta"]["total"].as_u64().unwrap_or_default() >= 1,
        "la page doit compter au moins ce qui vient d'être créé : {page}"
    );

    let sent = modification();
    let mise_a_jour = request("PATCH", &resource, sent.clone());
    let (status, updated) = call(&api, mise_a_jour).await;
    assert_eq!(status, StatusCode::OK, "mise à jour refusée : {updated}");
    compare(&updated, &sent, "email");
    compare(&updated, &sent, "name");
    compare(&updated, &sent, "confirmed");

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

/// Le squelette compresse ce que le client accepte de recevoir compressé.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_list_travels_compressed_when_the_client_accepts_it() {
    let api = application().await;
    let collection = "/subscribers";

    // Une ligne au moins : une liste vide peut tomber sous le seuil en deçà duquel la
    // compression ne s'applique pas.
    let (status, created) = call(&api, request("POST", collection, creation())).await;
    assert_eq!(status, StatusCode::CREATED, "création refusée : {created}");

    let mut demande = without_body("GET", collection);
    demande
        .headers_mut()
        .insert("accept-encoding", "gzip".parse().expect("en-tête valide"));
    let reponse = api.clone().oneshot(demande).await.expect("réponse");
    let encodage = reponse.headers().get("content-encoding");

    assert_eq!(reponse.status(), StatusCode::OK);
    assert_eq!(
        encodage.map(|valeur| valeur.as_bytes()),
        Some(&b"gzip"[..]),
        "le client accepte gzip, la liste doit partir compressée"
    );
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
    let collection = "/subscribers";

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
