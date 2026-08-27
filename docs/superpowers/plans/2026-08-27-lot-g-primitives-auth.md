# Lot G — Primitives d'auth dans le noyau : plan d'implémentation

> **Pour un agent exécutant :** SOUS-SKILL REQUIS — `superpowers:test-driven-development`
> pour chaque tâche. Les étapes sont cochables (`- [ ]`).

**But :** doter `rbs-core`, sous le flag `auth`, des cinq primitives sans logique
applicative dont la feature auth générée (lot I) aura besoin.

**Architecture :** modules de premier niveau `hash`, `jwt`, `token`, compilés sous
`#[cfg(feature = "auth")]`, plus une extension de trois modules existants (`config`,
`extract`, `state`). Aucune logique de connexion, aucune entité, aucun enum de rôle :
tout cela est généré dans le projet par le lot I.

**Pile :** `argon2 0.5.3`, `jsonwebtoken 10.3.0`, `rand 0.10.2`, `sha2 0.11.0`,
`base64 0.23.1` — toutes **optionnelles**, tirées par le flag `auth`.

**Spec :** `docs/superpowers/specs/2026-08-27-v0.2-auth-design.md` (§2.1, §2.2, §2.5, §3).
**Backlog :** `TODO.md`, tâches `G1` à `G5`. Les lignes `✓` y sont la liste des tests.

## Contraintes globales

- **Ce lot ne touche que `crates/rbs-core/` et `Cargo.toml` racine.** Le lot H travaille
  en parallèle dans `crates/rbs-cli/`. Ne modifier aucun fichier de `rbs-cli`,
  d'`examples/`, de `docs/` — ni **`TODO.md`**, qui est coché par l'orchestrateur.
- Rust edition 2024, `rust-version = 1.85`, cargo 1.96.
- `#![warn(missing_docs)]` est armé sur la crate : tout item public porte un `///` d'une
  à trois lignes. C'est la seule exception à « un commentaire dit le pourquoi ».
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`
  sont bloquants.
- Les dépendances passent par `[workspace.dependencies]` du `Cargo.toml` racine, puis
  `nom = { workspace = true, optional = true }` dans `crates/rbs-core/Cargo.toml`.
  Suivre la disposition déjà en place, ne pas la réordonner.
- **Ne jamais deviner l'API d'une crate.** `rand 0.10`, `sha2 0.11` et `base64 0.23` sont
  des majeures récentes dont les API ont changé. Écrire l'appel, compiler, lire l'erreur
  du compilateur — ou lire la source dans `~/.cargo/registry/src/`. Pas de code écrit de
  mémoire sur ces trois-là.
- Commits Conventional Commits, sujet français à l'impératif. **Aucun identifiant de
  tâche (`G1`…), aucun renvoi à `TODO.md` ou à ce plan, aucune ligne `Co-Authored-By`.**
  Le corps porte le pourquoi technique et un intertitre `Vérifications :` avec les
  commandes réellement lancées et leur sortie.
- Un commit par tâche, au minimum.

## Structure de fichiers

| Fichier | Responsabilité |
|---|---|
| `crates/rbs-core/src/hash.rs` | **créé** — Argon2 : `hacher`, `verifier` |
| `crates/rbs-core/src/jwt.rs` | **créé** — `Claims`, `signer`, `verifier`, `ErreurJwt` |
| `crates/rbs-core/src/token.rs` | **créé** — `aleatoire`, `empreinte` |
| `crates/rbs-core/src/config.rs` | **modifié** — `AuthConfig`, champ `auth`, validation |
| `crates/rbs-core/src/extract.rs` | **modifié** — extracteur `Identity` |
| `crates/rbs-core/src/state.rs` | **modifié** — trait `HasAuth` |
| `crates/rbs-core/src/lib.rs` | **modifié** — déclaration et ré-export des modules |
| `crates/rbs-core/Cargo.toml` | **modifié** — cinq deps optionnelles sous le flag |
| `Cargo.toml` (racine) | **modifié** — cinq entrées `[workspace.dependencies]` |

Les tests vivent dans un `#[cfg(test)] mod tests` en fin de fichier, comme partout dans
la crate. Aucun fichier de `tests/` n'est créé.

**Commande de preuve du lot :** `cargo test -p rbs-core --features auth`.
Vérifier **aussi** `cargo test -p rbs-core` (sans le flag) à chaque tâche : le noyau doit
continuer à compiler et à passer sans auth. C'est la moitié de l'intérêt du flag.

---

### Tâche G1 — Hachage Argon2

**Fichiers :** créer `crates/rbs-core/src/hash.rs` ; modifier `lib.rs`,
`crates/rbs-core/Cargo.toml`, `Cargo.toml` racine.

**Produit** (ce dont G4 et le lot I dépendront) :

```rust
/// Hache `mot_de_passe` avec Argon2id et un sel tiré pour cet appel.
pub fn hacher(mot_de_passe: &str) -> crate::Result<String>;
/// Vérifie `mot_de_passe` contre un hash au format PHC.
pub fn verifier(mot_de_passe: &str, hash: &str) -> crate::Result<bool>;
```

L'échec passe par `Error::Internal(anyhow::Error)` : un hash illisible vient de la base
ou d'un bug, jamais du client. **`verifier` ne renvoie pas `Err` sur mot de passe
faux** — c'est `Ok(false)`. Confondre les deux ferait répondre 500 à une faute de frappe.

- [ ] **Étape 1 — Ajouter la dépendance.** `cargo add --package rbs-core argon2@0.5.3
      --optional`, puis basculer l'entrée sur `[workspace.dependencies]` à la manière des
      autres. Ajouter `argon2` à la liste du flag : `auth = ["dep:argon2"]`.
      Vérifier : `cargo build -p rbs-core` **et** `cargo build -p rbs-core --features auth`.

- [ ] **Étape 2 — Écrire les trois tests d'abord**, dans `hash.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deux_hachages_du_meme_mot_de_passe_different() {
        let a = hacher("correct horse battery staple").expect("hachage");
        let b = hacher("correct horse battery staple").expect("hachage");
        assert_ne!(a, b, "le sel doit être tiré à chaque appel");
    }

    #[test]
    fn verifier_accepte_le_bon_mot_de_passe_et_rejette_un_autre() {
        let hash = hacher("s3cr3t").expect("hachage");
        assert!(verifier("s3cr3t", &hash).expect("vérification"));
        assert!(!verifier("s3cr3T", &hash).expect("vérification"));
    }

    #[test]
    fn un_hash_malforme_rend_une_erreur_sans_paniquer() {
        assert!(verifier("s3cr3t", "pas un hash PHC").is_err());
    }
}
```

- [ ] **Étape 3 — Les voir échouer.** `cargo test -p rbs-core --features auth hash::`
      Attendu : échec de compilation, `hacher` n'existe pas.

- [ ] **Étape 4 — Implémenter.** `Argon2::default()` (Argon2id, paramètres par défaut de
      la crate), `SaltString::generate(&mut OsRng)` pour le sel, `PasswordHash::new` puis
      `verify_password` pour la vérification. Mapper les erreurs vers `Error::Internal`
      via `anyhow::anyhow!`. `PasswordVerifyError::Password` (mot de passe faux) doit
      devenir `Ok(false)`, pas une erreur — c'est le piège de cette tâche.

- [ ] **Étape 5 — Les voir passer.** `cargo test -p rbs-core --features auth hash::`
      Attendu : 3 passed. Puis `cargo test -p rbs-core` → la crate compile sans le flag.

- [ ] **Étape 6 — Éprouver que les tests mordent.** Retirer le sel aléatoire (sel
      constant) → `deux_hachages_du_meme_mot_de_passe_different` doit **échouer**.
      Remettre. Consigner ce résultat, il fait partie de la preuve.

- [ ] **Étape 7 — Qualité et commit.** `cargo fmt --all` puis
      `cargo clippy -p rbs-core --all-targets --features auth -- -D warnings`.
      `git commit -m "feat(core): hache et vérifie les mots de passe avec Argon2"`

---

### Tâche G2 — Jetons JWT

**Fichiers :** créer `crates/rbs-core/src/jwt.rs` ; modifier `lib.rs` et les manifestes.

**Produit :**

```rust
/// Charge utile d'un jeton d'accès.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // identifiant de l'utilisateur
    pub role: String,  // rôle en clair : l'enum `Role` est généré, invisible au noyau
    pub exp: i64,      // expiration, secondes epoch
    pub iat: i64,      // émission, secondes epoch
    pub jti: String,   // identifiant du jeton
}

/// Échec de vérification d'un jeton.
#[derive(Debug, thiserror::Error)]
pub enum ErreurJwt {
    #[error("jeton expiré")]
    Expire,
    #[error("signature invalide")]
    Signature,
    #[error("jeton malformé : {0}")]
    Malforme(String),
}

pub fn signer(claims: &Claims, secret: &str) -> crate::Result<String>;
pub fn verifier(jeton: &str, secret: &str) -> Result<Claims, ErreurJwt>;
```

Plus `impl From<ErreurJwt> for Error` rendant `Error::Unauthorized` : le lot I et G4 en
dépendent pour répondre 401 sans réécrire la traduction.

**Pourquoi une erreur typée ici et pas dans G1 :** le `✓` de la tâche l'exige — un jeton
expiré doit se distinguer d'une signature invalide. Le distinguo sert au client (« ton
jeton a expiré, rafraîchis-le ») et non au serveur, qui répond 401 dans les deux cas.

- [ ] **Étape 1 — Dépendance.** `jsonwebtoken@10.3.0`, optionnelle, ajoutée au flag.
      Vérifier la version résolue : `cargo add --dry-run jsonwebtoken -p rbs-core` doit
      annoncer `v10.3.0`. **L'API de la 10 diffère de la 9** — la lire, ne pas la deviner.

- [ ] **Étape 2 — Écrire les quatre tests d'abord :**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "un secret de test qui porte au moins trente-deux octets";

    fn claims(exp: i64) -> Claims { /* sub "u1", role "user", iat 0, jti "j1", exp */ }

    #[test]
    fn signer_puis_verifier_restitue_les_claims() { /* aller-retour, assert_eq complet */ }

    #[test]
    fn un_jeton_expire_rend_une_erreur_distincte_de_la_signature() {
        // exp dans le passé, signé avec le bon secret
        assert!(matches!(verifier(&jeton, SECRET), Err(ErreurJwt::Expire)));
    }

    #[test]
    fn une_signature_invalide_est_rejetee() {
        assert!(matches!(verifier(&jeton, "un autre secret tout aussi long ici"),
                         Err(ErreurJwt::Signature)));
    }

    #[test]
    fn un_jeton_alg_none_est_rejete() {
        // Forger l'en-tête {"alg":"none","typ":"JWT"} en base64url, payload valide,
        // signature vide. Doit être rejeté, jamais accepté.
    }
}
```

Le test `alg: none` est le seul de ce lot qui teste une **vulnérabilité** et non un
comportement : un vérificateur qui fait confiance à l'en-tête du jeton accepte n'importe
qui. `jsonwebtoken` s'en protège via `Validation::new(Algorithm::HS256)` ; le test prouve
que la protection est bien armée dans **notre** appel, pas seulement disponible.

- [ ] **Étape 3 — Les voir échouer.** `cargo test -p rbs-core --features auth jwt::`

- [ ] **Étape 4 — Implémenter.** `encode` avec `Header::new(Algorithm::HS256)` ;
      `decode::<Claims>` avec `Validation::new(Algorithm::HS256)`. Traduire
      `ErrorKind::ExpiredSignature` → `Expire`, `ErrorKind::InvalidSignature` →
      `Signature`, le reste → `Malforme`.

- [ ] **Étape 5 — Les voir passer.** `cargo test -p rbs-core --features auth jwt::`
      Attendu : 4 passed.

- [ ] **Étape 6 — Éprouver.** Remplacer `Validation::new(Algorithm::HS256)` par une
      validation acceptant `none` → le test `alg: none` doit **échouer**. Remettre.

- [ ] **Étape 7 — fmt, clippy, commit.**
      `git commit -m "feat(core): signe et vérifie les jetons d'accès en HS256"`

---

### Tâche G3 — `AuthConfig` branchée sur figment

**Fichiers :** modifier `crates/rbs-core/src/config.rs`.

**Consomme :** rien. **Produit :** `config::AuthConfig`, et `Config::auth`.

```rust
/// Secret de signature et durées de vie des jetons.
#[cfg(feature = "auth")]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AuthConfig {
    /// Secret de signature HS256. Aucune valeur par défaut.
    pub secret: String,
    /// Durée de vie du jeton d'accès, en secondes.
    pub access_ttl_secs: u64,
    /// Durée de vie du jeton de rafraîchissement, en secondes.
    pub refresh_ttl_secs: u64,
}
```

Champ `#[cfg(feature = "auth")] pub auth: AuthConfig` sur `Config`. Défauts
`access_ttl_secs = 900` et `refresh_ttl_secs = 2_592_000` (quinze minutes, trente jours,
§2.2 de la spec), posés par `Serialized::default` dans `figment()` sous le même `cfg`.
**Pas de défaut pour `secret`** : son absence doit faire échouer le démarrage.

Nouvelle variante d'erreur, sous `cfg` :

```rust
#[cfg(feature = "auth")]
#[error("configuration invalide : `auth.secret` doit porter au moins 32 octets, {0} fournis")]
SecretTropCourt(usize),
```

`Config::load()` extrait puis valide. Un secret court n'est pas rattrapable au runtime :
il se refuse au boot, où le développeur le lit, et non à la première requête.

- [ ] **Étape 1 — Écrire les trois tests d'abord**, dans le `mod tests` existant, en
      suivant le motif `Jail::expect_with` déjà utilisé par les seize tests du fichier
      (`jail.clear_env()`, `jail.create_dir("config")`, `jail.create_file(...)`) :

```rust
#[cfg(feature = "auth")]
#[test]
fn un_secret_absent_fait_echouer_le_chargement_en_nommant_le_champ() {
    // DEFAULT_TOML sans section [auth] → Err, message contenant "auth.secret"
}

#[cfg(feature = "auth")]
#[test]
fn un_secret_de_moins_de_32_octets_est_refuse_au_chargement() {
    // secret = "trop court" → Err(ConfigError::SecretTropCourt(_))
}
```

Le troisième `✓` — « `cargo build -p rbs-core` sans le flag ne compile pas le champ » —
n'est pas un test unitaire mais une commande : `cargo build -p rbs-core` doit réussir
alors qu'aucune configuration ne porte de section `auth`. Le prouver en le lançant.

- [ ] **Étape 2 — Les voir échouer.** `cargo test -p rbs-core --features auth config::`

- [ ] **Étape 3 — Implémenter** la struct, le champ, les défauts et la validation.

- [ ] **Étape 4 — Réparer les constructions littérales de `Config`.** Ajouter le champ
      casse tout site qui construit `Config { .. }` à la main sous le flag — au moins
      `state.rs`. Les corriger avec un champ sous `#[cfg(feature = "auth")]`.
      **Chercher exhaustivement :** `grep -rn "Config {" crates/rbs-core/src/`.

- [ ] **Étape 5 — Les voir passer.** Les trois commandes, dans cet ordre :
      `cargo test -p rbs-core --features auth` (tout vert, pas seulement `config::`),
      `cargo test -p rbs-core` (sans flag, tout vert),
      `cargo build -p rbs-core` (compile sans le champ).

- [ ] **Étape 6 — Éprouver.** Abaisser le seuil de 32 à 0 → le test du secret court doit
      **échouer**. Remettre.

- [ ] **Étape 7 — fmt, clippy, commit.**
      `git commit -m "feat(core): charge le secret et les durées de vie des jetons"`

---

### Tâche G4 — Extracteur `Identity` et trait `HasAuth`

**Fichiers :** modifier `crates/rbs-core/src/extract.rs` et `crates/rbs-core/src/state.rs`.

**Consomme :** `jwt::{Claims, verifier, ErreurJwt}` (G2), `config::AuthConfig` (G3).

```rust
// state.rs
/// État applicatif donnant accès à la configuration d'authentification.
#[cfg(feature = "auth")]
pub trait HasAuth: HasCoreState {
    /// Configuration d'authentification portée par cet état.
    fn auth(&self) -> &AuthConfig {
        &self.core().config().auth
    }
}

#[cfg(feature = "auth")]
impl HasAuth for CoreState {}
```

Méthode par défaut, et **pas** d'implémentation générale (`impl<T: HasCoreState> HasAuth
for T`) : une implémentation générale interdirait à un projet de tirer son secret
d'ailleurs — d'un gestionnaire de secrets, par exemple. Le projet généré écrit
`impl HasAuth for AppState {}`, une ligne.

```rust
// extract.rs
/// Identité authentifiée, extraite du jeton porté par la requête.
#[cfg(feature = "auth")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Identifiant de l'utilisateur, tel que porté par `sub`.
    pub user_id: String,
    /// Rôle en clair. L'enum `Role` est généré dans le projet, hors de portée du noyau.
    pub role: String,
}

impl<S: HasAuth> FromRequestParts<S> for Identity { type Rejection = Error; /* ... */ }
```

`FromRequestParts` et non `FromRequest` : l'identité se lit dans les en-têtes, et un
extracteur qui consommerait le corps interdirait à `ValidatedJson` de le lire ensuite.

- [ ] **Étape 1 — Écrire les trois tests d'abord.** Reprendre le motif du `mod tests` de
      `state.rs` : un `AppState` de test qui implémente `HasCoreState` **et** `HasAuth`,
      un routeur monté par `Router::new().route(...).with_state(...)`, joué par
      `tower::ServiceExt::oneshot`.

```rust
#[tokio::test] async fn sans_en_tete_authorization_la_reponse_est_401_en_problem_json() {
    // assert status 401 ET content-type "application/problem+json"
}
#[tokio::test] async fn un_jeton_invalide_ou_expire_rend_401() { /* les deux cas */ }
#[tokio::test] async fn un_jeton_valide_peuple_l_identite_depuis_les_claims() {
    // handler renvoyant format!("{} {}", id.user_id, id.role) → "u1 admin"
}
```

- [ ] **Étape 2 — Les voir échouer.** `cargo test -p rbs-core --features auth extract::`

- [ ] **Étape 3 — Implémenter.** Lire `Authorization`, exiger le préfixe `Bearer `
      (insensible à la casse du schéma), appeler `jwt::verifier` avec
      `state.auth().secret`, peupler depuis `sub` et `role`. Tout échec →
      `Error::Unauthorized`, dont `IntoResponse` rend déjà le `problem+json`.

- [ ] **Étape 4 — Les voir passer.** `cargo test -p rbs-core --features auth` → tout vert.

- [ ] **Étape 5 — Éprouver.** Faire accepter un en-tête sans préfixe `Bearer` → le
      premier test doit **échouer**. Remettre.

- [ ] **Étape 6 — fmt, clippy, commit.**
      `git commit -m "feat(core): extrait l'identité authentifiée d'une requête"`

---

### Tâche G5 — Jetons opaques

**Fichiers :** créer `crates/rbs-core/src/token.rs` ; modifier `lib.rs` et les manifestes.

**Produit :**

```rust
/// Tire un jeton opaque de 32 octets, encodé en base64url sans remplissage.
pub fn aleatoire() -> String;
/// Empreinte SHA-256 d'un jeton, en hexadécimal minuscule, pour le stockage.
pub fn empreinte(jeton: &str) -> String;
```

**Pas d'Argon2 ici, et c'est délibéré** (§2.2 de la spec) : un jeton de 256 bits tirés au
hasard n'est pas atteignable par force brute, et un KDF lent se paierait à chaque
rafraîchissement sans rien acheter.

- [ ] **Étape 1 — Dépendances.** `rand@0.10.2`, `sha2@0.11.0`, `base64@0.23.1`,
      optionnelles, ajoutées au flag. **Ces trois majeures ont des API récentes :**
      compiler et lire les erreurs plutôt qu'écrire de mémoire. En particulier, le
      tirage par `OsRng` peut renvoyer un `Result` selon la version — si c'est le cas,
      un échec du générateur du système est irrécupérable : `expect` avec un message
      explicite, plutôt que propager une erreur qu'aucun appelant ne saura traiter.

- [ ] **Étape 2 — Écrire les trois tests d'abord :**

```rust
#[test] fn deux_tirages_successifs_different() { assert_ne!(aleatoire(), aleatoire()); }

#[test] fn le_jeton_decode_porte_au_moins_32_octets() {
    // décoder en base64url sans padding, assert >= 32
}

#[test] fn l_empreinte_est_deterministe_et_ne_rend_pas_le_jeton() {
    let jeton = aleatoire();
    assert_eq!(empreinte(&jeton), empreinte(&jeton));
    assert_ne!(empreinte(&jeton), jeton);
    assert_eq!(empreinte(&jeton).len(), 64);      // SHA-256 en hexadécimal
    assert_ne!(empreinte(&jeton), empreinte(&aleatoire()));
}
```

- [ ] **Étape 3 — Les voir échouer.** `cargo test -p rbs-core --features auth token::`

- [ ] **Étape 4 — Implémenter.** `OsRng` → 32 octets, `URL_SAFE_NO_PAD` pour l'encodage,
      `Sha256::digest` puis rendu hexadécimal.

- [ ] **Étape 5 — Les voir passer.** `cargo test -p rbs-core --features auth token::`
      Attendu : 3 passed.

- [ ] **Étape 6 — Éprouver.** Remplacer le tirage par une constante → le premier test
      doit **échouer**. Remettre.

- [ ] **Étape 7 — fmt, clippy, commit.**
      `git commit -m "feat(core): tire et empreinte les jetons de rafraîchissement"`

---

## Vérification finale du lot

Lancer, lire la sortie, et la consigner :

```bash
cargo test -p rbs-core --features auth        # tout le noyau, flag armé
cargo test -p rbs-core                        # tout le noyau, flag absent
cargo build -p rbs-core                       # compile sans les cinq deps
cargo clippy -p rbs-core --all-targets --features auth -- -D warnings
cargo clippy -p rbs-core --all-targets -- -D warnings
cargo fmt --all --check
```

Vérifier enfin qu'un projet sans auth **ne compile pas** les cinq crates :
`cargo tree -p rbs-core | grep -c argon2` → attendu `0`, et
`cargo tree -p rbs-core --features auth | grep -c argon2` → attendu au moins `1`.

## À rapporter

Pour **chaque** tâche `G1`…`G5`, et séparément : la commande exacte, son résultat réel
(nombre de tests passés), et le résultat de l'étape « éprouver » (quelle mutation, quel
test est tombé). Signaler toute tâche dont un `✓` n'a **pas** pu être prouvé — elle
restera `- [ ]` avec une annotation `PARTIEL`, ce qui est un résultat acceptable ;
un `✓` déclaré sans preuve ne l'est pas.
