use super::super::{matches, signature};

/// Le vecteur de signature, calculé hors de Rust :
///
/// ```text
/// $ printf '1757000000.{}' | openssl dgst -sha256 -hmac "secret" -hex
/// ```
///
/// C'est ce qui lui donne sa valeur : un test qui comparerait `sign` à `sign` ne prouverait
/// que la stabilité du condensat, jamais sa justesse. Un receveur écrit en Python ou en Go
/// doit retrouver celui-ci, octet pour octet.
const VECTEUR: &str = "a5dba1becd39e5b001811ec483ca620bdfc84f2167017c06f8a0fc0ca2953b75";

#[test]
fn the_signature_matches_an_independently_computed_vector() {
    assert_eq!(signature::sign("secret", 1_757_000_000, b"{}"), VECTEUR);
}

#[test]
fn the_signature_changes_with_the_timestamp() {
    // L'horodatage entre dans le condensat, et c'est ce qui ferme le rejeu : un tiers qui
    // capte une livraison ne peut pas la resservir plus tard sous une date fraîche.
    assert_ne!(
        signature::sign("secret", 1_757_000_000, b"{}"),
        signature::sign("secret", 1_757_000_001, b"{}"),
        "la date ne participe pas à la signature"
    );
}

#[test]
fn the_signature_header_carries_the_timestamp_and_the_v1_digest() {
    let entete = signature::header("secret", 1_757_000_000, b"{}");

    assert_eq!(entete, format!("t=1757000000,v1={VECTEUR}"));

    // La forme est ce que le receveur découpe : elle ne peut pas dériver sans casser tous
    // les receveurs déjà écrits.
    let (horodatage, condensat) = entete.split_once(',').expect("deux champs séparés par ,");
    assert_eq!(horodatage, "t=1757000000");

    let condensat = condensat.strip_prefix("v1=").expect("le schéma est nommé");
    assert_eq!(condensat.len(), 64, "un SHA-256 fait 32 octets");
    assert!(
        condensat
            .chars()
            .all(|caractere| caractere.is_ascii_digit() || ('a'..='f').contains(&caractere)),
        "l'hexadécimal doit être minuscule : {condensat}"
    );
}

#[test]
fn an_exact_pattern_matches_only_its_own_event() {
    let motifs = vec!["user.created".to_string()];

    assert!(matches(&motifs, "user.created"));
    assert!(!matches(&motifs, "user.deleted"));
    // Un motif exact n'est pas un préfixe : sans quoi `user.created` prendrait aussi
    // `user.created.retry`, un événement que l'abonné n'a jamais demandé.
    assert!(!matches(&motifs, "user.created.retry"));
}

#[test]
fn a_prefix_pattern_matches_every_event_of_its_family() {
    let motifs = vec!["user.*".to_string()];

    assert!(matches(&motifs, "user.created"));
    assert!(matches(&motifs, "user.deleted"));
    assert!(!matches(&motifs, "order.created"));
}

#[test]
fn the_star_pattern_matches_every_event() {
    let motifs = vec!["*".to_string()];

    assert!(matches(&motifs, "user.created"));
    assert!(matches(&motifs, "order.paid"));
}
