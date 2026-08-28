use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{Cache, a_supprimer, decoder, encoder, motif};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Profil {
    nom: String,
    age: u8,
}

#[test]
fn une_valeur_mise_en_cache_se_relit_deserialisee() {
    let profil = Profil {
        nom: "ada".to_string(),
        age: 36,
    };

    let encode = encoder(&profil).expect("la valeur est sérialisable");
    let relu: Option<Profil> = decoder(Some(encode)).expect("la valeur est désérialisable");

    assert_eq!(relu, Some(profil));
}

/// Une clé absente est le cas courant d'un cache, pas une panne : elle rend `None`, et
/// l'appelant enchaîne sur la source de vérité.
#[test]
fn une_cle_absente_rend_none_et_non_une_erreur() {
    let relu: Option<Profil> = decoder(None).expect("l'absence d'une clé n'est pas une erreur");

    assert_eq!(relu, None);
}

#[test]
fn invalider_prefixe_n_emporte_que_les_cles_du_prefixe_vise() {
    let rendues = ["session:1", "session:2", "sessions:1", "user:1"]
        .map(str::to_string)
        .to_vec();

    assert_eq!(a_supprimer("session:", rendues), ["session:1", "session:2"]);
}

/// Le motif de `SCAN` est un glob que le serveur interprète : sans échappement,
/// `invalider_prefixe("a*")` emporterait `abc`, que le préfixe ne désigne pas.
#[test]
fn un_prefixe_portant_un_metacaractere_de_glob_est_echappe() {
    assert_eq!(motif("session:"), "session:*");
    assert_eq!(motif("a*b"), r"a\*b*");
    assert_eq!(motif("user:[1]"), r"user:\[1\]*");
}

// Les deux tests qui suivent joignent le Redis que décrit la section `[cache]`, et sont
// donc `#[ignore]` : `cargo test` ne les lance pas, `cargo test -- --ignored` les lance
// contre le serveur du projet. `RBS_CACHE__URL` en surcharge l'adresse au besoin.

fn profil() -> Profil {
    Profil {
        nom: "ada".to_string(),
        age: 36,
    }
}

#[tokio::test]
#[ignore = "joint le Redis de la section [cache]"]
async fn le_parcours_complet_se_joue_contre_un_serveur() {
    let cache = Cache::depuis_config().expect("la section [cache] doit être lisible");

    cache
        .set("parcours:profil", &profil())
        .await
        .expect("l'écriture doit aboutir");

    let relu: Option<Profil> = cache
        .get("parcours:profil")
        .await
        .expect("la lecture doit aboutir");
    assert_eq!(relu, Some(profil()));

    cache
        .invalider("parcours:profil")
        .await
        .expect("la suppression doit aboutir");

    let relu: Option<Profil> = cache
        .get("parcours:profil")
        .await
        .expect("la lecture d'une clé absente n'est pas une erreur");
    assert_eq!(relu, None, "la clé invalidée ne doit plus être lisible");

    for cle in [
        "parcours:session:1",
        "parcours:session:2",
        "parcours:session:3",
    ] {
        cache
            .set(cle, &profil())
            .await
            .expect("l'écriture doit aboutir");
    }
    // Cette clé ressemble au préfixe visé sans être dessous : c'est elle qui distingue
    // une invalidation exacte d'un balayage approximatif.
    cache
        .set("parcours:sessions:temoin", &profil())
        .await
        .expect("l'écriture doit aboutir");

    let emportees = cache
        .invalider_prefixe("parcours:session:")
        .await
        .expect("le balayage doit aboutir");
    assert_eq!(emportees, 3);

    let temoin: Option<Profil> = cache
        .get("parcours:sessions:temoin")
        .await
        .expect("la lecture doit aboutir");
    assert_eq!(
        temoin,
        Some(profil()),
        "`parcours:sessions:temoin` n'est pas sous le préfixe invalidé"
    );

    cache
        .invalider("parcours:sessions:temoin")
        .await
        .expect("la suppression doit aboutir");
}

/// Un préfixe portant un métacaractère de glob, éprouvé contre l'interpréteur du serveur.
///
/// `motif` l'échappe et `a_supprimer` refiltre ce que le serveur a rendu : deux gardes
/// pour la même faute, dont aucun test hors serveur ne peut montrer qu'elles se
/// complètent. Sans échappement, `SCAN MATCH parcours:a*b:*` emporte `parcours:ab:temoin`,
/// que le préfixe ne désigne pas — et une suppression ne se défait pas.
#[tokio::test]
#[ignore = "joint le Redis de la section [cache]"]
async fn un_prefixe_a_metacaractere_n_emporte_que_ce_qu_il_designe() {
    let cache = Cache::depuis_config().expect("la section [cache] doit être lisible");

    for cle in ["parcours:a*b:1", "parcours:ab:temoin"] {
        cache
            .set(cle, &profil())
            .await
            .expect("l'écriture doit aboutir");
    }

    let emportees = cache
        .invalider_prefixe("parcours:a*b:")
        .await
        .expect("le balayage doit aboutir");
    assert_eq!(emportees, 1);

    let temoin: Option<Profil> = cache
        .get("parcours:ab:temoin")
        .await
        .expect("la lecture doit aboutir");
    assert_eq!(
        temoin,
        Some(profil()),
        "`parcours:ab:temoin` n'est emporté que si le `*` du préfixe a été interprété"
    );

    cache
        .invalider("parcours:ab:temoin")
        .await
        .expect("la suppression doit aboutir");
}

#[tokio::test]
#[ignore = "joint le Redis de la section [cache]"]
async fn une_valeur_a_ttl_d_une_seconde_a_disparu_apres_l_attente() {
    let cache = Cache::depuis_config().expect("la section [cache] doit être lisible");

    cache
        .set_ttl("ttl:ephemere", &profil(), Duration::from_secs(1))
        .await
        .expect("l'écriture doit aboutir");

    // Sans cette lecture, un `get` qui rendrait toujours `None` passerait le test au
    // vert : c'est l'expiration qui est prouvée ici, pas l'illisibilité.
    let avant: Option<Profil> = cache
        .get("ttl:ephemere")
        .await
        .expect("la lecture doit aboutir");
    assert_eq!(
        avant,
        Some(profil()),
        "la valeur doit être lisible avant son expiration"
    );

    // L'horloge du serveur, jamais une horloge simulée. Redis expire à la seconde près :
    // la marge couvre son arrondi.
    tokio::time::sleep(Duration::from_millis(1_500)).await;

    let apres: Option<Profil> = cache
        .get("ttl:ephemere")
        .await
        .expect("la lecture d'une clé expirée n'est pas une erreur");
    assert_eq!(apres, None, "la valeur devait avoir expiré côté serveur");
}
