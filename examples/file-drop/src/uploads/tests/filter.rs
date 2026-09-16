use super::*;

/// La route de filtrage retient la ligne qui satisfait son propre critère.
///
/// Le rendu du filtre ne prouve rien : une condition mal traduite rend une page vide, et
/// seule une requête jouée contre la base le montre.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_filter_narrows_the_list() {
    let api = application().await;
    let collection = "/uploads";
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
    let collection = "/uploads";
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
