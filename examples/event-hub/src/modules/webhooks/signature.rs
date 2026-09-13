// `KeyInit` porte `new_from_slice` depuis hmac 0.13 : la 0.12 le réexportait par `Mac`, et
// l'omettre ne se voit qu'à la compilation, sur une erreur qui ne nomme pas la version.
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// L'en-tête qui porte la signature.
pub const HEADER: &str = "x-rbs-signature";

/// L'en-tête qui nomme l'événement, lisible sans ouvrir le corps.
pub const HEADER_EVENT: &str = "x-rbs-event";

/// L'en-tête qui identifie l'événement, stable d'un réessai à l'autre.
///
/// C'est par lui que le receveur déduplique : la file peut livrer deux fois — une réponse
/// perdue après traitement — et sans cet identifiant, rien ne le lui dirait.
pub const HEADER_DELIVERY: &str = "x-rbs-delivery";

/// Signe un corps daté, et rend le condensat en hexadécimal minuscule.
///
/// **L'horodatage entre dans la signature**, et c'est ce qui ferme le rejeu : un tiers qui
/// capte une livraison ne peut pas la resservir plus tard sous une date fraîche sans
/// invalider le condensat.
pub fn sign(secret: &str, timestamp: i64, body: &[u8]) -> String {
    // `new_from_slice` n'échoue que sur une longueur de clé impossible, et HMAC en accepte
    // toutes : le `expect` est ici la façon de dire qu'il n'y a pas de cas d'échec.
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepte une clé de n'importe quelle longueur");

    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);

    mac.finalize()
        .into_bytes()
        .iter()
        .map(|octet| format!("{octet:02x}"))
        .collect()
}

/// La valeur de l'en-tête `X-Rbs-Signature`, prête à être posée.
///
/// `v1=` nomme le schéma plutôt que de laisser le condensat nu : le jour où un second
/// arrive, les deux cohabitent dans le même en-tête et un receveur à jour choisit.
pub fn header(secret: &str, timestamp: i64, body: &[u8]) -> String {
    format!("t={timestamp},v1={}", sign(secret, timestamp, body))
}
