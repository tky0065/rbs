use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{Cache, decode, encode, pattern, to_delete};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Profile {
    nom: String,
    age: u8,
}

#[test]
fn a_cached_value_reads_back_deserialized() {
    let profile = Profile {
        nom: "ada".to_string(),
        age: 36,
    };

    let encode = encode(&profile).expect("la valeur est sérialisable");
    let read_back: Option<Profile> = decode(Some(encode)).expect("la valeur est désérialisable");

    assert_eq!(read_back, Some(profile));
}

/// Une clé absente est le cas courant d'un cache, pas une panne : elle rend `None`, et
/// l'appelant enchaîne sur la source de vérité.
#[test]
fn a_missing_key_returns_none_and_not_an_error() {
    let read_back: Option<Profile> =
        decode(None).expect("l'absence d'une clé n'est pas une erreur");

    assert_eq!(read_back, None);
}

#[test]
fn invalidate_prefix_only_removes_the_keys_of_the_targeted_prefix() {
    let rendues = ["session:1", "session:2", "sessions:1", "user:1"]
        .map(str::to_string)
        .to_vec();

    assert_eq!(to_delete("session:", rendues), ["session:1", "session:2"]);
}

/// Le motif de `SCAN` est un glob que le serveur interprète : sans échappement,
/// `invalidate_prefix("a*")` emporterait `abc`, que le préfixe ne désigne pas.
#[test]
fn a_prefix_carrying_a_glob_metacharacter_is_escaped() {
    assert_eq!(pattern("session:"), "session:*");
    assert_eq!(pattern("a*b"), r"a\*b*");
    assert_eq!(pattern("user:[1]"), r"user:\[1\]*");
}

// Les deux tests qui suivent joignent le Redis que décrit la section `[cache]`, et sont
// donc `#[ignore]` : `cargo test` ne les lance pas, `cargo test -- --ignored` les lance
// contre le serveur du projet. `RBS_CACHE__URL` en surcharge l'adresse au besoin.

fn profile() -> Profile {
    Profile {
        nom: "ada".to_string(),
        age: 36,
    }
}

#[tokio::test]
#[ignore = "joint le Redis de la section [cache]"]
async fn the_full_run_plays_against_a_server() {
    let cache = Cache::from_config().expect("la section [cache] doit être lisible");

    cache
        .set("parcours:profil", &profile())
        .await
        .expect("l'écriture doit aboutir");

    let read_back: Option<Profile> = cache
        .get("parcours:profil")
        .await
        .expect("la lecture doit aboutir");
    assert_eq!(read_back, Some(profile()));

    cache
        .invalidate("parcours:profil")
        .await
        .expect("la suppression doit aboutir");

    let read_back: Option<Profile> = cache
        .get("parcours:profil")
        .await
        .expect("la lecture d'une clé absente n'est pas une erreur");
    assert_eq!(
        read_back, None,
        "la clé invalidée ne doit plus être lisible"
    );

    for key in [
        "parcours:session:1",
        "parcours:session:2",
        "parcours:session:3",
    ] {
        cache
            .set(key, &profile())
            .await
            .expect("l'écriture doit aboutir");
    }
    // Cette clé ressemble au préfixe visé sans être dessous : c'est elle qui distingue
    // une invalidation exacte d'un balayage approximatif.
    cache
        .set("parcours:sessions:temoin", &profile())
        .await
        .expect("l'écriture doit aboutir");

    let removed = cache
        .invalidate_prefix("parcours:session:")
        .await
        .expect("le balayage doit aboutir");
    assert_eq!(removed, 3);

    let temoin: Option<Profile> = cache
        .get("parcours:sessions:temoin")
        .await
        .expect("la lecture doit aboutir");
    assert_eq!(
        temoin,
        Some(profile()),
        "`parcours:sessions:temoin` n'est pas sous le préfixe invalidé"
    );

    cache
        .invalidate("parcours:sessions:temoin")
        .await
        .expect("la suppression doit aboutir");
}

/// Un préfixe portant un métacaractère de glob, éprouvé contre l'interpréteur du serveur.
///
/// `pattern` l'échappe et `to_delete` refiltre ce que le serveur a rendu : deux gardes
/// pour la même faute, dont aucun test hors serveur ne peut montrer qu'elles se
/// complètent. Sans échappement, `SCAN MATCH parcours:a*b:*` emporte `parcours:ab:temoin`,
/// que le préfixe ne désigne pas — et une suppression ne se défait pas.
#[tokio::test]
#[ignore = "joint le Redis de la section [cache]"]
async fn a_prefix_with_a_metacharacter_only_removes_what_it_designates() {
    let cache = Cache::from_config().expect("la section [cache] doit être lisible");

    for key in ["parcours:a*b:1", "parcours:ab:temoin"] {
        cache
            .set(key, &profile())
            .await
            .expect("l'écriture doit aboutir");
    }

    let removed = cache
        .invalidate_prefix("parcours:a*b:")
        .await
        .expect("le balayage doit aboutir");
    assert_eq!(removed, 1);

    let temoin: Option<Profile> = cache
        .get("parcours:ab:temoin")
        .await
        .expect("la lecture doit aboutir");
    assert_eq!(
        temoin,
        Some(profile()),
        "`parcours:ab:temoin` n'est emporté que si le `*` du préfixe a été interprété"
    );

    cache
        .invalidate("parcours:ab:temoin")
        .await
        .expect("la suppression doit aboutir");
}

#[tokio::test]
#[ignore = "joint le Redis de la section [cache]"]
async fn a_value_with_a_one_second_ttl_is_gone_after_the_wait() {
    let cache = Cache::from_config().expect("la section [cache] doit être lisible");

    cache
        .set_ttl("ttl:ephemere", &profile(), Duration::from_secs(1))
        .await
        .expect("l'écriture doit aboutir");

    // Sans cette lecture, un `get` qui rendrait toujours `None` passerait le test au
    // vert : c'est l'expiration qui est prouvée ici, pas l'illisibilité.
    let avant: Option<Profile> = cache
        .get("ttl:ephemere")
        .await
        .expect("la lecture doit aboutir");
    assert_eq!(
        avant,
        Some(profile()),
        "la valeur doit être lisible avant son expiration"
    );

    // L'horloge du serveur, jamais une horloge simulée. Redis expire à la seconde près :
    // la marge couvre son arrondi.
    tokio::time::sleep(Duration::from_millis(1_500)).await;

    let apres: Option<Profile> = cache
        .get("ttl:ephemere")
        .await
        .expect("la lecture d'une clé expirée n'est pas une erreur");
    assert_eq!(apres, None, "la valeur devait avoir expiré côté serveur");
}
