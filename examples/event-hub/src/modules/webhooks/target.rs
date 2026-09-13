use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use reqwest::Url;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

/// Où une livraison a le droit d'aller.
///
/// Stricte partout sauf en `development` : un receveur sur `localhost:4000` est le cas
/// nominal d'un poste de travail, et le seul endroit où un `http` en clair ne livre rien à
/// personne d'autre. Pas de réglage pour rouvrir la porte ailleurs — une clé de
/// configuration serait le SSRF que cette politique ferme, remis à portée d'un fichier.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    strict: bool,
}

/// Pourquoi une URL est refusée. Le message nomme la règle, jamais la plage : dire
/// « 10.0.0.0/8 » à qui vient de sonder confirmerait qu'il a visé juste.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Scheme,
    Https,
    PrivateHost,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Scheme => "une URL de webhook doit être en http ou en https",
            Self::Https => "une URL de webhook doit être en https",
            Self::PrivateHost => "l'hôte de l'URL n'est pas une adresse publique",
        })
    }
}

impl std::error::Error for Refusal {}

impl Policy {
    pub fn for_env(env: &str) -> Self {
        Self {
            strict: env != "development",
        }
    }

    /// Dit si une livraison peut se connecter à cette adresse.
    pub fn allows(self, ip: IpAddr) -> bool {
        !self.strict || is_public(ip)
    }

    /// Valide l'URL d'un abonnement, et la rend analysée.
    ///
    /// La même fonction sert à l'inscription et à la livraison : un abonnement inscrit
    /// avant cette règle, ou sous un autre profil, est jugé à la livraison comme s'il
    /// venait d'être posé.
    pub fn check(self, url: &str) -> Result<Url, Refusal> {
        let url = Url::parse(url).map_err(|_| Refusal::Scheme)?;

        match url.scheme() {
            "https" => {}
            "http" if !self.strict => {}
            "http" => return Err(Refusal::Https),
            _ => return Err(Refusal::Scheme),
        }

        if !self.strict {
            return Ok(url);
        }

        let Some(hote) = url.host_str() else {
            return Err(Refusal::Scheme);
        };
        let nu = hote.trim_matches(['[', ']']);
        // RFC 6761 : un nom pleinement qualifié garde un `.` final, et tout sous-domaine
        // de `localhost` reste local quel que soit le nom qui précède.
        let nu_min = nu.strip_suffix('.').unwrap_or(nu).to_ascii_lowercase();

        if nu_min == "localhost" || nu_min.ends_with(".localhost") {
            return Err(Refusal::PrivateHost);
        }
        if let Ok(ip) = nu.parse::<IpAddr>()
            && !is_public(ip)
        {
            return Err(Refusal::PrivateHost);
        }

        Ok(url)
    }

    /// Résout l'hôte de `url` et ne garde que les adresses permises.
    ///
    /// Vide quand tout a été filtré : c'est ce que la livraison lit pour abandonner plutôt
    /// que réessayer. Le client refiltre à la connexion par [`Resolver`] — deux
    /// résolutions, parce qu'un nom peut changer de réponse entre les deux, et c'est
    /// précisément l'attaque que la seconde ferme.
    pub async fn resolve(self, url: &Url) -> std::io::Result<Vec<SocketAddr>> {
        let Some(hote) = url.host_str() else {
            return Ok(Vec::new());
        };
        let port = url.port_or_known_default().unwrap_or(443);

        Ok(
            tokio::net::lookup_host((hote.trim_matches(['[', ']']), port))
                .await?
                .filter(|adresse| self.allows(adresse.ip()))
                .collect(),
        )
    }
}

/// Une adresse joignable depuis l'Internet public, et rien d'autre.
///
/// Les plages sont énumérées à la main plutôt que par `IpAddr::is_global`, encore
/// instable : loopback, privées (10/8, 172.16/12, 192.168/16), lien local (169.254/16, où
/// vivent les métadonnées des nuages), CGNAT (100.64/10), non spécifiée, multicast,
/// réservées (0.0.0.0/8, 240.0.0.0/4, 192.0.0.0/24, 198.18.0.0/15, 192.88.99.0/24), et
/// leurs équivalents IPv6 — ULA `fc00::/7`, lien local `fe80::/10` et site-local déprécié
/// `fec0::/10` — ainsi que les IPv4 mappées et les adresses de transition qui enveloppent
/// une IPv4 : NAT64 `64:ff9b::/96`, IPv4-compatible dépréciée `::/96` et 6to4 `2002::/16`
/// sont jugées par l'IPv4 qu'elles portent, tandis que Teredo (`2001:0::/32`) est refusée
/// en bloc plutôt que décodée — aucun service public n'y vit.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_public_v4(v4),
            None => {
                let segs = v6.segments();

                // NAT64 (RFC 6052) et le compatible IPv4 déprécié (RFC 4291) logent une
                // IPv4 dans les 32 bits de poids faible : les ignorer laisserait ce
                // détour joindre une cible privée sans jamais toucher le résolveur DNS.
                if segs[..6] == [0, 0, 0, 0, 0, 0] || segs[..6] == [0x0064, 0xff9b, 0, 0, 0, 0] {
                    return is_public_v4(Ipv4Addr::new(
                        (segs[6] >> 8) as u8,
                        segs[6] as u8,
                        (segs[7] >> 8) as u8,
                        segs[7] as u8,
                    ));
                }

                // 6to4 (RFC 3056) loge l'IPv4 du relais dans les deux segments qui
                // suivent le préfixe, sur le même principe.
                if segs[0] == 0x2002 {
                    return is_public_v4(Ipv4Addr::new(
                        (segs[1] >> 8) as u8,
                        segs[1] as u8,
                        (segs[2] >> 8) as u8,
                        segs[2] as u8,
                    ));
                }

                // Teredo (RFC 4380) loge aussi une IPv4, complémentée bit à bit — mais
                // aucun service public ne vit sur `2001:0::/32`, donc autant refuser la
                // plage entière plutôt que décoder un client pour rien.
                if segs[0] == 0x2001 && segs[1] == 0 {
                    return false;
                }

                !(v6.is_loopback()
                    || v6.is_unspecified()
                    || v6.is_multicast()
                    || segs[0] & 0xfe00 == 0xfc00
                    || segs[0] & 0xffc0 == 0xfe80
                    || segs[0] & 0xffc0 == 0xfec0)
            }
        },
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();

    !(ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_multicast()
        || a == 0
        || a >= 240
        || (a == 100 && (64..=127).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 192 && b == 88 && c == 99))
}

/// Le résolveur du client de livraison : celui du système, filtré par la politique.
///
/// Une adresse retirée ici n'est jamais connectée, quoi qu'ait répondu la résolution
/// faite avant l'envoi.
#[derive(Clone, Debug)]
pub struct Resolver {
    policy: Policy,
}

impl Resolver {
    pub fn new(policy: Policy) -> Self {
        Self { policy }
    }
}

impl Resolve for Resolver {
    fn resolve(&self, name: Name) -> Resolving {
        let policy = self.policy;
        let hote = name.as_str().to_owned();

        Box::pin(async move {
            // Le port n'a pas d'importance : le connecteur pose celui de l'URL sur chaque
            // adresse rendue.
            let adresses: Vec<SocketAddr> = tokio::net::lookup_host((hote.as_str(), 0))
                .await?
                .filter(|adresse| policy.allows(adresse.ip()))
                .collect();

            if adresses.is_empty() {
                return Err(
                    Box::new(Refusal::PrivateHost) as Box<dyn std::error::Error + Send + Sync>
                );
            }

            Ok(Box::new(adresses.into_iter()) as Addrs)
        })
    }
}
