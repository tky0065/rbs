# Webhooks : plus de SSRF par l'URL d'abonnement — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Une URL d'abonnement ne peut plus faire livrer un corps signé sur une adresse interne, ni par un littéral, ni par un nom qui y résout, ni par une redirection.

**Architecture:** Un module `target.rs` du fragment porte une `Policy` (stricte hors profil `development`) qui classe les adresses IP et valide une URL (schéma, `https`, hôte). `service::subscribe` l'applique à l'inscription (400) ; `Sender::post` l'applique à la livraison, résout l'hôte, et refuse avant d'envoyer ; le `reqwest::Client` partagé reçoit `Policy::none()` pour les redirections et un `Resolver` (`reqwest::dns::Resolve`) qui refiltre à la connexion, ce qui ferme le rebinding DNS. Une cible refusée est abandonnée (`Ok(())` + `warn`), pas réessayée.

**Tech Stack:** minijinja (`{@ @}`, mais ces fichiers ne portent aucun tag), reqwest 0.13 (`redirect::Policy`, `dns::{Resolve, Name, Resolving, Addrs}`, `Url`), `tokio::net::lookup_host` (feature `net`), `std::net::IpAddr`, tests unitaires sans base + tests `#[ignore]` joints à la base.

**Spec:** `docs/superpowers/specs/2026-09-12-lot-secu-p2-design.md`, section 1.

## Global Constraints

- Aucun exemple d'`examples/` ne porte `webhooks` : pas de régénération, mais `integration_examples` doit rester vert (aucune template partagée n'est touchée).
- Un commentaire explique le *pourquoi*, jamais le *quoi*. Le code engendré ne commente que ses points d'extension.
- Commits Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution (`CLAUDE.md` racine et global).
- Le fragment n'est compilé par rien avant Docker : `cargo check` sur un projet jetable avant la passe lente (`rbs new` + `add jobs` + `add auth` + `add webhooks`, voir `integration_webhooks.rs::project_with_webhooks_on`).
- La passe lente se lance avec `--no-fail-fast` et sa sortie redirigée vers `$SCRATCHPAD/webhooks-lent.log`.
- Fichiers sous `crates/rbs-cli/templates/features/webhooks/` sauf mention contraire.

---

### Task 1: `target.rs` — la politique et la classification d'adresses

**Files:**
- Create: `target.rs.jinja`
- Modify: `mod.rs.jinja` (déclarer `pub mod target;`)
- Modify: `feature.toml` (nouvelle entrée `[[files]]`, `tokio` feature `net`)
- Test: `tests.rs.jinja` (tests unitaires, sans base)

**Interfaces:**
- Produces: `pub struct Policy { strict: bool }` (`Copy`), `Policy::for_env(env: &str) -> Policy`, `Policy::allows(self, ip: IpAddr) -> bool`, `Policy::check(self, url: &str) -> Result<reqwest::Url, Refusal>`, `pub enum Refusal { Scheme, Https, PrivateHost }` avec `Display` en français, `pub fn is_public(ip: IpAddr) -> bool`.

- [ ] **Step 1: Écrire les tests rouges dans `tests.rs.jinja`** (après les tests de `matches`, avant `table_a_soi`)

```rust
// ── Cibles ───────────────────────────────────────────────────────────────────

use std::net::IpAddr;

use super::target::{Policy, Refusal, is_public};

#[test]
fn private_loopback_and_link_local_addresses_are_not_public() {
    for adresse in [
        "127.0.0.1", "10.0.0.5", "172.16.3.4", "172.31.255.255", "192.168.1.1",
        "169.254.169.254", "100.64.0.1", "0.0.0.0", "224.0.0.1",
        "::1", "::", "fc00::1", "fd12::1", "fe80::1", "::ffff:10.0.0.1", "ff02::1",
    ] {
        let ip: IpAddr = adresse.parse().expect("adresse lisible");
        assert!(!is_public(ip), "{adresse} devrait être refusée");
    }
}

#[test]
fn public_addresses_are_public() {
    for adresse in ["93.184.216.34", "8.8.8.8", "172.32.0.1", "2606:2800:220:1:248:1893:25c8:1946"] {
        let ip: IpAddr = adresse.parse().expect("adresse lisible");
        assert!(is_public(ip), "{adresse} devrait être acceptée");
    }
}

#[test]
fn outside_development_only_https_to_a_public_host_passes() {
    let stricte = Policy::for_env("production");

    assert!(stricte.check("https://example.test/hooks").is_ok());
    assert_eq!(stricte.check("http://example.test/hooks").unwrap_err(), Refusal::Https);
    assert_eq!(stricte.check("ftp://example.test/hooks").unwrap_err(), Refusal::Scheme);
    assert_eq!(stricte.check("https://10.0.0.5/hooks").unwrap_err(), Refusal::PrivateHost);
    assert_eq!(stricte.check("https://[::1]/hooks").unwrap_err(), Refusal::PrivateHost);
    assert_eq!(stricte.check("https://localhost/hooks").unwrap_err(), Refusal::PrivateHost);
    assert_eq!(stricte.check("https://169.254.169.254/latest").unwrap_err(), Refusal::PrivateHost);
}

#[test]
fn in_development_http_and_private_hosts_pass_but_not_other_schemes() {
    let souple = Policy::for_env("development");

    assert!(souple.check("http://localhost:4000/hooks").is_ok());
    assert!(souple.check("http://127.0.0.1:4000/hooks").is_ok());
    assert_eq!(souple.check("file:///etc/passwd").unwrap_err(), Refusal::Scheme);
}
```

- [ ] **Step 2: Écrire `target.rs.jinja`**

```rust
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

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
            Self::Scheme => "une URL de webhook est en http ou en https",
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

        match url.host() {
            Some(url::Host::Ipv4(ip)) if !is_public(IpAddr::V4(ip)) => Err(Refusal::PrivateHost),
            Some(url::Host::Ipv6(ip)) if !is_public(IpAddr::V6(ip)) => Err(Refusal::PrivateHost),
            Some(url::Host::Domain(nom)) if nom.eq_ignore_ascii_case("localhost") => {
                Err(Refusal::PrivateHost)
            }
            Some(_) => Ok(url),
            None => Err(Refusal::Scheme),
        }
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

        Ok(tokio::net::lookup_host((hote.trim_matches(['[', ']']), port))
            .await?
            .filter(|adresse| self.allows(adresse.ip()))
            .collect())
    }
}

/// Une adresse joignable depuis l'Internet public, et rien d'autre.
///
/// Les plages sont énumérées à la main plutôt que par `IpAddr::is_global`, encore
/// instable : loopback, privées (10/8, 172.16/12, 192.168/16), lien local (169.254/16, où
/// vivent les métadonnées des nuages), CGNAT (100.64/10), non spécifiée, multicast, et
/// leurs équivalents IPv6 — ULA `fc00::/7` et lien local `fe80::/10` — ainsi que les IPv4
/// mappées, jugées par l'adresse qu'elles enveloppent.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_public_v4(v4),
            None => {
                !(v6.is_loopback()
                    || v6.is_unspecified()
                    || v6.is_multicast()
                    || v6.segments()[0] & 0xfe00 == 0xfc00
                    || v6.segments()[0] & 0xffc0 == 0xfe80)
            }
        },
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();

    !(ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || (a == 100 && (64..=127).contains(&b)))
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
                return Err(Box::new(Refusal::PrivateHost)
                    as Box<dyn std::error::Error + Send + Sync>);
            }

            Ok(Box::new(adresses.into_iter()) as Addrs)
        })
    }
}
```

`Ipv6Addr` n'est employé que si le compilateur le réclame ; retirer l'import s'il est inutile (`clippy -D warnings`).

`url::Host` : `reqwest` réexporte `Url` mais pas `Host`. Deux options, au choix de l'implémenteur après `cargo check` : ajouter `url = "2"` aux `[[dependencies]]` du `feature.toml` (dépendance déjà dans l'arbre via reqwest), ou tester `url.host_str()` puis `parse::<IpAddr>()` sur la chaîne débarrassée des crochets. La seconde évite une dépendance ; la préférer :

```rust
        let Some(hote) = url.host_str() else {
            return Err(Refusal::Scheme);
        };
        let nu = hote.trim_matches(['[', ']']);

        if nu.eq_ignore_ascii_case("localhost") {
            return Err(Refusal::PrivateHost);
        }
        if let Ok(ip) = nu.parse::<IpAddr>()
            && !is_public(ip)
        {
            return Err(Refusal::PrivateHost);
        }

        Ok(url)
```

- [ ] **Step 3: Déclarer le module et le fichier**

`mod.rs.jinja` : ajouter `pub mod target;` après `pub mod signature;`.

`feature.toml` : ajouter après l'entrée `signature.rs.jinja` :

```toml
[[files]]
source      = "target.rs.jinja"
destination = "src/modules/webhooks/target.rs"
```

et remplacer la section tokio :

```toml
# `sync` : les tests livrés se relaient sur la table des abonnements et sur la file, et leur
# verrou traverse un `await`. `net` : la résolution DNS de la cible, avant l'envoi et à la
# connexion.
[cargo.tokio]
features = ["sync", "net"]
```

- [ ] **Step 4: Vérifier sur un projet jetable**

```bash
S=/private/tmp/claude-501/-Users-yacoubakone-dev-rs/*/scratchpad; cd $S && rm -rf wh && \
cargo run -q --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- new wh --yes --core-path ~/dev/rs/crates/rbs-core --database-url 'postgres://rbs:rbs@localhost:5432/wh' && \
cd wh && git add -A && git commit -qm init && \
for f in jobs auth webhooks; do cargo run -q --manifest-path ~/dev/rs/Cargo.toml -p rbs-cli --bin rbs -- add $f && git add -A && git commit -qm "add $f"; done && \
cargo test --lib -- modules::webhooks::tests 2>&1 | tail -20
```

Attendu : les quatre tests neufs passent, les six anciens aussi (les `#[ignore]` sont ignorés). Si `add auth` refuse faute de compose, jouer `add` avec l'option que `integration_webhooks.rs::project_with_webhooks_on` emploie.

- [ ] **Step 5: Commit**

```bash
git add crates/rbs-cli/templates/features/webhooks/
git commit -m "feat(webhooks): classe les cibles de livraison par une politique liée au profil"
```

---

### Task 2: Refus à l'abonnement

**Files:**
- Modify: `service.rs.jinja` (`subscribe`)
- Modify: `controller.rs.jinja` (`subscribe`)
- Modify: `delivery.rs.jinja` (`Sender` porte la `Policy`, `from_config(&config)`)
- Modify: `feature.toml` (ancre `state_init`)
- Modify: `dto.rs.jinja` (le commentaire de `url`)
- Test: `tests.rs.jinja`

**Interfaces:**
- Consumes: `Policy`, `Refusal` de la tâche 1.
- Produces: `Sender::from_config(config: &rbs_core::Config) -> anyhow::Result<Self>`, `Sender::policy(&self) -> Policy`, `service::subscribe(db, policy: Policy, input) -> Result<SubscriptionCreated>`.

- [ ] **Step 1: Test rouge (sous conteneur) dans `tests.rs.jinja`**

```rust
/// Le profil des tests est `development`, qui tolère tout : le refus s'observe sur un
/// service appelé avec la politique stricte, pas sur la route.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_private_url_is_refused_outside_development() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let refus = super::service::subscribe(
        db,
        Policy::for_env("production"),
        super::dto::CreateSubscription {
            url: "http://169.254.169.254/latest/meta-data".to_string(),
            events: vec!["*".to_string()],
        },
    )
    .await;

    assert!(
        matches!(refus, Err(rbs_core::Error::BadRequest(_))),
        "une URL interne doit être refusée : {refus:?}"
    );
    assert_eq!(livraisons(db).await, 0);
}
```

Ajouter le nom à `TESTS_SOUS_CONTENEUR` dans `crates/rbs-cli/tests/integration_webhooks.rs` (le tableau passe à `[&str; 8]`) et les quatre noms de la tâche 1 à `TESTS_ORDINAIRES` (`[&str; 10]`).

- [ ] **Step 2: `service::subscribe` prend la politique**

```rust
use super::target::Policy;
// ...
pub async fn subscribe(
    db: &DatabaseConnection,
    policy: Policy,
    input: CreateSubscription,
) -> Result<SubscriptionCreated> {
    // `#[validate(url)]` a dit que c'est une URL ; ceci dit où elle a le droit de mener.
    policy
        .check(&input.url)
        .map_err(|refus| Error::BadRequest(refus.to_string()))?;

    for motif in &input.events {
```

- [ ] **Step 3: `Sender` porte la politique**

Dans `delivery.rs.jinja` :

```rust
use super::target::{Policy, Resolver};
// ...
#[derive(Clone, Debug)]
pub struct Sender {
    client: reqwest::Client,
    policy: Policy,
}

impl Sender {
    /// Construit le client d'après la section `[webhooks]` et le profil actif.
    ///
    /// L'échec remonte au démarrage plutôt qu'à la première livraison : un délai
    /// d'expiration illisible est une faute de configuration, et la découvrir six heures
    /// plus tard dans un journal de worker ne sert personne.
    pub fn from_config(config: &rbs_core::Config) -> anyhow::Result<Self> {
        let section = Config::load()?;
        let policy = Policy::for_env(&config.env);

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(section.timeout_secs))
                // Un 3xx est une réponse hors 2xx comme une autre : suivre une
                // redirection livrerait le corps signé là où le receveur — ou qui a pris
                // sa place — l'envoie, hors de toute politique.
                .redirect(reqwest::redirect::Policy::none())
                .dns_resolver(std::sync::Arc::new(Resolver::new(policy)))
                .build()?,
            policy,
        })
    }

    pub fn policy(&self) -> Policy {
        self.policy
    }
```

`feature.toml`, ancre `state_init` : `webhooks: crate::modules::webhooks::Sender::from_config(&config)?,` — `config` est le paramètre d'`AppState::new` (`templates/project/src/state.rs.jinja:13`).

`controller.rs.jinja` : `service::subscribe(state.core().db(), state.webhooks().policy(), input).await?`.

`dto.rs.jinja`, commentaire de `url` :

```rust
    /// L'URL du receveur. `https` obligatoire hors développement, et jamais vers une
    /// adresse interne : la signature authentifie l'émetteur, elle ne chiffre pas ce qui
    /// est livré, et une livraison n'a rien à faire sur le réseau du projet.
```

- [ ] **Step 4: Vérifier** — `cargo test --lib -- modules::webhooks::tests` sur le projet jetable (régénérer le projet ou recopier les fichiers touchés), puis `cargo test --lib -- --ignored modules::webhooks::tests` avec la base du compose démarrée (`docker compose up -d`, `cargo run -p migration`), attendu : le test neuf passe.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat(webhooks): refuse à l'inscription une URL interne, en http hors développement"
```

---

### Task 3: Refus à la livraison, sans réessai

**Files:**
- Modify: `delivery.rs.jinja` (`Sender::post`, `Delivery::run`)
- Test: `tests.rs.jinja`

**Interfaces:**
- Consumes: `Policy::check`, `Policy::resolve`.
- Produces: `pub(super) enum Outcome`? Non : `post` rend `Result<(), PostError>` avec `enum PostError { Blocked(Refusal), Transport(anyhow::Error) }`.

- [ ] **Step 1: Tests rouges (sous conteneur)**

```rust
/// Une cible interdite ne se réessaie pas : cinq tentatives de plus n'y changeraient
/// rien, et `run` le dit en rendant `Ok`.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_delivery_to_a_blocked_target_is_abandoned_not_retried() {
    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();
    let abonnement = abonne(db, "http://169.254.169.254/latest", &["*"]).await;

    let sender = super::delivery::Sender::with_policy(Policy::for_env("production"))
        .expect("client constructible");
    let resultat = sender
        .post(&abonnement.url, "t=0,v1=0", "user.created", Uuid::now_v7(), Vec::new())
        .await;

    assert!(
        matches!(resultat, Err(super::delivery::PostError::Blocked(Refusal::Https))),
        "{resultat:?}"
    );
}

/// En développement, un receveur local est joint : c'est le chemin nominal du poste de
/// travail, et il traverse le résolveur filtrant — qui doit laisser passer.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn in_development_a_local_receiver_is_reached() {
    use axum::routing::post;

    let (_garde, state) = table_a_soi().await;
    let db = state.core().db();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port libre");
    let port = listener.local_addr().expect("adresse locale").port();
    let recu = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let compteur = recu.clone();
    tokio::spawn(async move {
        let app = Router::new().route(
            "/hook",
            post(move || {
                compteur.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { StatusCode::NO_CONTENT }
            }),
        );
        axum::serve(listener, app).await.expect("receveur");
    });

    let abonnement = abonne(db, &format!("http://localhost:{port}/hook"), &["*"]).await;
    let livraison = Delivery {
        subscription: abonnement.id,
        event: Event {
            id: Uuid::now_v7(),
            event: "user.created".to_string(),
            created_at: chrono::Utc::now().fixed_offset(),
            data: donnees(),
        },
    };

    livraison.run(&state).await.expect("livraison acceptée");
    assert_eq!(recu.load(std::sync::atomic::Ordering::SeqCst), 1);
}
```

Ajouter les deux noms à `TESTS_SOUS_CONTENEUR` (`[&str; 10]`).

- [ ] **Step 2: `Sender::post` refuse avant d'envoyer**

```rust
/// Ce qui empêche une livraison, et ce que la file doit en faire.
#[derive(Debug)]
pub enum PostError {
    /// La cible est interdite : rien ne changera au prochain essai.
    Blocked(Refusal),
    /// Le transport ou le receveur a échoué : la file réessaie.
    Transport(anyhow::Error),
}

impl Sender {
    /// Un client sous une politique donnée, pour les tests qui veulent la stricte sous
    /// un profil qui ne l'est pas.
    #[cfg(test)]
    pub(super) fn with_policy(policy: Policy) -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(1))
                .redirect(reqwest::redirect::Policy::none())
                .dns_resolver(std::sync::Arc::new(Resolver::new(policy)))
                .build()?,
            policy,
        })
    }

    pub(super) async fn post(
        &self,
        url: &str,
        signature: &str,
        event: &str,
        delivery: Uuid,
        body: Vec<u8>,
    ) -> Result<(), PostError> {
        let cible = self.policy.check(url).map_err(PostError::Blocked)?;

        // Résolue avant l'envoi pour que le refus soit nommé — le résolveur du client
        // refiltre à la connexion, mais son erreur arrive noyée dans celle du transport.
        let adresses = self
            .policy
            .resolve(&cible)
            .await
            .map_err(|source| PostError::Transport(source.into()))?;
        if adresses.is_empty() {
            return Err(PostError::Blocked(Refusal::PrivateHost));
        }

        let reponse = self
            .client
            .post(cible)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(signature::HEADER, signature)
            .header(signature::HEADER_EVENT, event)
            .header(signature::HEADER_DELIVERY, delivery.to_string())
            .body(body)
            .send()
            .await
            .map_err(|source| PostError::Transport(source.into()))?;

        let statut = reponse.status();

        if statut.is_success() {
            tracing::debug!(%url, %event, %delivery, status = statut.as_u16(), "livraison acceptée");
            return Ok(());
        }

        Err(PostError::Transport(anyhow::anyhow!("{url} a répondu {statut}")))
    }
}
```

- [ ] **Step 3: `Delivery::run` abandonne une cible refusée**

Remplacer la fin de `run` :

```rust
        match state
            .webhooks()
            .post(
                &abonnement.url,
                &signature::header(&abonnement.secret, horodatage, &corps),
                &self.event.event,
                self.event.id,
                corps,
            )
            .await
        {
            Ok(()) => Ok(()),
            // Une cible interdite le restera : réessayer ne ferait que cinq lignes de
            // journal de plus. Le job se termine, et l'abonnement reste à révoquer.
            Err(PostError::Blocked(refus)) => {
                tracing::warn!(
                    subscription = %abonnement.id,
                    event = %self.event.event,
                    %refus,
                    "livraison vers une cible interdite : abandonnée"
                );
                Ok(())
            }
            Err(PostError::Transport(source)) => Err(source),
        }
```

- [ ] **Step 4: `cargo check` + tests sur le projet jetable**, ordinaires puis `--ignored`. Attendu : 10 ordinaires, 10 sous conteneur, tous verts.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat(webhooks): interdit redirections et cibles internes à la livraison, sans réessai"
```

---

### Task 4: Documentation, notes de version, passe lente

**Files:**
- Modify: `docs/docs/guides/webhooks.md`, `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/webhooks.md`
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md` (sous `## [1.5.0]`, section `Changed`/`Modifié` — créer l'entrée `[1.5.0] — 2026-09-12` si l'autre branche ne l'a pas encore posée ; au merge, les deux entrées fusionnent)
- Modify: `IMPROVE.md` ligne de la tâche 9 — **non** : le cochage est fait par l'orchestrateur après vérification.

- [ ] **Step 1: Guide EN** — après le tableau des routes (ligne ~91, l'exemple `{ "url": "https://example.test/hooks" ... }`), ajouter :

```markdown
### Where a delivery may go

A subscription URL is checked twice. At registration, outside the `development` profile,
it must be `https` and its host must not be a loopback, private, link-local or
carrier-grade NAT address — nor `localhost`; the request gets a 400 naming the rule. At
delivery, the host is resolved and every non-public address is dropped, both before the
request is sent and again inside the HTTP client's resolver, so a name that changes its
answer between the two never reaches an internal service. Redirects are never followed: a
3xx is a failed delivery like any other non-2xx. A delivery whose target is blocked is
abandoned, not retried — nothing would change on the fifth attempt.

In `development` every rule is lifted: a receiver on `http://localhost:4000` is the normal
case on a workstation.
```

FR équivalent dans le fichier i18n, même emplacement.

- [ ] **Step 2: CHANGELOG** (EN puis FR) :

```markdown
## [1.5.0] — 2026-09-12

### Changed

- **A webhook subscription can no longer reach the project's own network.** Outside the
  `development` profile, `POST /webhooks/subscriptions` answers 400 to a non-`https` URL
  and to any host that is a loopback, private, link-local or CGNAT address, or
  `localhost`. At delivery the host is resolved and filtered again, inside the HTTP
  client's resolver as well, and redirects are never followed. A blocked delivery is
  abandoned rather than retried. Subscriptions registered before this version are judged
  at delivery by the same rule.
```

- [ ] **Step 3: Passe lente**

```bash
cargo test -p rbs-cli --test integration_webhooks --no-fail-fast -- --ignored > $SCRATCHPAD/webhooks-lent.log 2>&1; tail -5 $SCRATCHPAD/webhooks-lent.log
```

Attendu : `1 passed`. Puis `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p rbs-cli --lib`, `cargo test -p rbs-cli --test integration_docs` (les transcriptions), `cargo test -p rbs-cli --test integration_examples`.

- [ ] **Step 4: Commit**

```bash
git commit -am "docs(webhooks): dit où une livraison a le droit d'aller"
```
