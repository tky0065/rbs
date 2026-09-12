# Quatre trous de sécurité des fragments `webhooks` et `auth`

Date : 2026-09-12
Portée : `crates/rbs-cli/templates/features/webhooks/`, `crates/rbs-cli/templates/features/auth/`,
`crates/rbs-core/src/{state,extract}.rs`, `examples/blog-auth/`, guides `auth` et `webhooks`
en deux langues, `CHANGELOG` en deux langues, version 1.5.0 des deux crates.
Hors portée : la révocation d'une session *nommée* qui tuerait son jeton d'accès (rien ne
relie un `jti` à une ligne de `refresh_tokens`), une liste de refus de `jti` dans Redis,
la normalisation Unicode des adresses (NFKC, points de Gmail).

## Le problème

Quatre tâches du backlog, toutes de sécurité, toutes vérifiées dans le code du 12
septembre :

1. **SSRF par l'URL d'abonnement webhook.** `dto.rs.jinja` ne pose que `#[validate(url)]`,
   `delivery.rs.jinja` construit le `reqwest::Client` sans politique de redirection et
   POSTe vers l'URL telle quelle. Un admin — ou l'attaquant qui tient son jeton — fait
   livrer un corps JSON signé sur `http://169.254.169.254/…`, sur `http://10.0.0.5:9200`,
   ou sur une URL publique qui répond 302 vers l'interne, et la file le rejoue cinq fois.
2. **Un jeton de rafraîchissement fermé permet de fermer le compte à volonté.** Une seule
   colonne, `revoked_at`, dit qu'une ligne n'est plus vivante ; rien ne dit *pourquoi*.
   `refresh` traite donc un jeton fermé par `logout` ou par un reset comme un jeton rejoué
   après rotation, et révoque tout le compte. L'attaquant expulsé par un reset rejoue son
   jeton mort à chaque reconnexion de la victime, pendant trente jours.
3. **Le jeton d'accès survit à toute révocation.** `Identity` ne fait que `jwt::verify` ;
   après un reset « sessions révoquées », l'attaquant garde `access_ttl_secs` d'accès, et un
   admin rétrogradé reste admin jusqu'à l'expiration.
4. **L'adresse n'est jamais normalisée.** `find_by_email` compare à l'octet près et la
   clé unique est sensible à la casse sur PostgreSQL et SQLite : l'attaquant inscrit
   `Victime@ex.fr`, la victime clique le lien de vérification, le compte de l'attaquant
   porte son adresse vérifiée.

## 1. SSRF — `webhooks`

### À l'abonnement

`service::subscribe` valide l'URL après `#[validate(url)]`, dans un module `target.rs` du
fragment :

- schéma `http` ou `https`, tout autre refusé (`file`, `gopher`, `ftp`) ;
- `https` obligatoire quand le profil n'est pas `development` ;
- hôte littéral refusé s'il n'est pas public : loopback, privé (10/8, 172.16/12,
  192.168/16), link-local (169.254/16, dont le point de métadonnées), CGNAT (100.64/10),
  non spécifié, multicast, `::1`, ULA `fc00::/7`, IPv4 mappées en IPv6, et le nom
  `localhost` — sauf en `development`.

Le refus est un `Error::BadRequest` dont le message nomme la règle enfreinte, pas la
plage : « une URL de webhook doit être en https » ou « l'hôte de l'URL n'est pas une
adresse publique ».

### À la livraison

Le `reqwest::Client` partagé reçoit deux choses :

- `redirect(reqwest::redirect::Policy::none())` : un 3xx est une réponse hors 2xx, donc un
  échec, donc un réessai — le receveur qui redirige est en panne du point de vue de
  l'émetteur ;
- un résolveur DNS maison, `target::Resolver`, implémentant `reqwest::dns::Resolve` :
  il résout par `tokio::net::lookup_host` puis **filtre** les adresses rendues avec la même
  règle que l'abonnement. Une adresse privée retirée à la résolution n'est jamais
  connectée : c'est ce qui ferme le DNS rebinding, que la validation à l'abonnement ne
  voit pas. Quand rien ne reste, la résolution échoue avec un message qui dit que l'hôte ne
  résout que vers du privé.

En profil `development`, le résolveur ne filtre rien : un receveur sur `localhost:4000`
est le cas nominal d'un poste de travail.

Une livraison dont la résolution a été vidée est **abandonnée** : `Delivery::run` rend
`Ok(())` après un `warn` portant l'identifiant de l'abonnement et l'événement. Cinq
réessais sur une cible interdite ne feraient que cinq lignes de journal de plus. La
distinction se fait sur l'erreur : `Sender::post` rend une erreur typée
`Blocked` que `run` reconnaît, toute autre erreur de transport reste un réessai.

### Le profil

`Sender::from_config(&config)` prend la `rbs_core::Config` que `AppState::new` reçoit
déjà ; l'ancre `state_init` devient `Sender::from_config(&config)?`. La règle est
`config.env == "development"`, sans réglage pour la contourner : une clé
`allow_private_targets` rouvrirait par configuration ce que la tâche ferme.

### Tests

Unitaires sans base : la classification d'adresses (tableau de cas, IPv4 et IPv6) ; la
validation d'URL selon le profil (`https` exigé, `http` toléré en `development`, littéral
privé refusé, `localhost` refusé, hôte public accepté) ; le résolveur qui vide `localhost`
hors `development` et le garde en `development`.

Sous conteneur : `an_admin_subscribing_a_private_url_gets_400` (via la route, profil
forcé hors développement par la construction du `Sender` de test) ; le test existant
`a_delivery_whose_subscription_was_revoked_succeeds_without_a_request` garde son URL
`http://127.0.0.1:1/hook`, insérée sans passer par la route.

`integration_webhooks.rs` : ses deux listes de noms s'allongent des tests neufs.

## 2. Rotation et fermeture distinctes — `auth`

### Schéma

`refresh_tokens` gagne `replaced_at`, `timestamp_with_time_zone` nullable, après
`revoked_at`. Le modèle suit. Une session est **ouverte** quand les deux colonnes sont
nulles ; c'est la condition de `open_sessions_of`, de `revoke_sessions_of` et de
`revoke_session`.

### Dépôt

`consume(db, id)` se scinde :

- `rotate(db, id) -> Result<Rotation>` pose `replaced_at` où les deux colonnes sont nulles.
  Quand l'`UPDATE` ne touche rien, la fonction relit la ligne et rend `Rotation::Replayed`
  si `replaced_at` est posé, `Rotation::Closed` si `revoked_at` l'est — le `SELECT` qui
  suit l'`UPDATE` ne décide de rien, il ne fait que nommer l'état déjà écrit.
- `close(db, id) -> Result<bool>` pose `revoked_at` sous la même condition, pour `logout`.

### Service

`refresh` : `Rotation::Done` → nouvelle paire ; `Rotation::Replayed` → révocation de tout
le compte, `warn`, 401 (le comportement d'aujourd'hui, désormais réservé au vrai rejeu) ;
`Rotation::Closed` → 401 seul, aucune écriture, aucun journal au-delà du `debug`.

`logout` appelle `close`.

### Existant

La migration ne change que pour un `rbs add auth` neuf. Le CHANGELOG donne aux projets
déjà migrés la ligne à jouer :

```sql
ALTER TABLE refresh_tokens ADD COLUMN replaced_at timestamptz NULL;
```

### Tests

`a_refresh_closed_by_logout_when_replayed_does_not_close_the_other_sessions` ; le test
existant `replaying_a_refresh_closes_the_other_sessions_of_the_account` reste vert ;
`the_table_carries_the_fingerprint_and_never_the_token` s'étend à la colonne neuve si le
test lit les colonnes.

## 3. Jeton d'accès révocable — `auth` + `rbs-core`

### Le crochet, dans le noyau

`HasAuth` gagne une méthode fournie :

```rust
/// Dernier mot du projet sur un jeton dont la signature est bonne.
///
/// Le noyau ne connaît ni la table des comptes ni ce qu'une révocation y écrit : il
/// vérifie la signature, puis demande. Le défaut accepte tout, et c'est ce qu'un projet
/// sans révocation obtient sans rien écrire.
fn accept(&self, claims: &Claims) -> impl Future<Output = Result<(), Error>> + Send {
    let _ = claims;
    async { Ok(()) }
}
```

`Identity::from_request_parts` l'appelle entre `jwt::verify` et la construction. Ajout
d'une méthode fournie : non cassant, `impl HasAuth for AppState {}` compile toujours.
`rbs-core` passe en 1.5.0, `Claims` ne change pas.

### La règle, dans le fragment

`users` gagne `sessions_revoked_at`, nullable. `mod.rs.jinja` remplace
`impl HasAuth for AppState {}` par une implémentation d'`accept` qui lit la ligne du
compte et refuse en `Unauthorized` :

- si le compte n'existe plus ;
- si `claims.iat <= sessions_revoked_at.timestamp()` — la seconde de la révocation
  comprise, `iat` n'ayant pas mieux que la seconde ;
- si `claims.role != utilisateur.role.to_value()` — le client rafraîchit et repart avec le
  rôle courant.

Coût : une lecture de `users` par requête qui extrait `Identity`, et uniquement celles-là.
Le guide le dit, et dit que la révocation d'une session nommée laisse vivre son jeton
d'accès jusqu'à `exp`.

### L'émission

`issue()` pose `iat = max(maintenant, sessions_revoked_at + 1)` : la paire que
`change-password` rend dans la seconde même de sa révocation n'est pas morte-née. `exp`
suit `iat`. `change` recharge le compte après la révocation, sinon `issue` verrait la
colonne d'avant.

### L'estampille

`repository::user::stamp_sessions_revoked(db, id)` pose `sessions_revoked_at = maintenant`
(instant Rust lié en paramètre, comme toute date de ces dépôts). Le service l'appelle
partout où il appelle `revoke_sessions_of` : rejeu détecté, `change`, `reset`,
`DELETE /auth/sessions`. Un helper de service, `close_every_session`, porte les deux
appels pour qu'aucun des quatre chemins ne puisse en oublier un.

### Existant

```sql
ALTER TABLE users ADD COLUMN sessions_revoked_at timestamptz NULL;
```

### Tests

`an_access_token_issued_before_a_reset_is_refused` (reset, puis l'ancien Bearer sur
`/auth/me` → 401) ; `the_pair_returned_by_change_password_works_at_once` (l'existant
`changing_the_password_returns_a_usable_pair_and_closes_the_others` le couvre s'il rejoue
la paire — à vérifier, sinon l'étendre) ; `a_demoted_admin_is_refused_with_its_old_token`
(rôle changé en base, ancien Bearer sur une route admin → 401) ; dans `rbs-core`, un test
d'`extract.rs` avec un état dont `accept` refuse → 401.

## 4. Adresse normalisée — `auth`

`service/mod.rs.jinja` porte `pub(super) fn normalise(email: &str) -> String` :
`trim()` puis `to_lowercase()`. Appliquée à l'entrée de `register`, `login`,
`password::request_reset`, `verification::request`. Le DTO ne change pas : la validation
`#[validate(email)]` s'applique à ce que le client envoie, la normalisation à ce que la
base voit.

Le profil rendu porte l'adresse normalisée. La partie locale est théoriquement sensible à
la casse (RFC 5321 §2.4) ; aucun fournisseur ne l'honore, et c'est l'attaquant qui en
profiterait.

### Existant

```sql
UPDATE users SET email = lower(trim(email));
```

Un projet dont deux comptes ne diffèrent que par la casse verra la clé unique refuser
cette ligne : le CHANGELOG le dit, et laisse le projet trancher.

### Tests

`registration_lowercases_and_trims_the_address` ; `login_ignores_the_case_of_the_address` ;
`an_address_taken_in_another_case_is_a_conflict`.

## Transverse

- Version `1.5.0` dans `Cargo.toml` (workspace), `CHANGELOG.md` et `CHANGELOG.fr.md` : une
  entrée `Changed` par tâche, les trois `ALTER`/`UPDATE` sous une sous-section « Projets
  déjà générés ».
- Guides `auth.md` (EN et FR) : « It is short-lived because it cannot be revoked » devient
  faux, ainsi que « the two operations share their repository call » ; un paragraphe sur
  `sessions_revoked_at`, le coût, et la limite des sessions nommées. `webhooks.md` : un
  paragraphe « Où une livraison peut aller ».
- `examples/blog-auth` régénéré par diff entre deux générations ; `integration_examples`
  est l'oracle.
- Passes lentes : `integration_auth` et `integration_webhooks` sous `--ignored
  --no-fail-fast`, sorties redirigées vers des fichiers distincts du scratchpad.
