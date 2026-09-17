# `rbs add api-keys` — conception

**Date** : 2026-09-17
**Tâche** : `IMPROVE.md` n° 71 — *[Feature] `rbs add api-keys`*
**Portée** : authentification **machine par clé porteuse**. Hors périmètre, explicitement :
OAuth/OIDC et TOTP (écartés par l'en-tête d'`IMPROVE.md`), les portées par clé — `scopes`
façon GitHub —, et l'émission d'une clé par un tiers pour le compte d'autrui.

---

## Ce que le fragment livre

Une table de clés, quatre routes pour les administrer, et **un en-tête de plus que
`Identity` accepte** — si bien que tout ce qu'un jeton ouvre déjà s'ouvre à une clé, sans
qu'une seule ligne des contrôleurs engendrés ne change.

```
GET /articles
  X-Api-Key: rbs_kJ3…
        │
        ├── rbs-core : Identity ne trouve pas d'Authorization, lit X-Api-Key,
        │              appelle state.accept_key(…)
        │
        └── le projet : src/modules/api_keys/service::accept
                ├── api_keys par empreinte SHA-256, ni révoquée ni périmée
                ├── users par user_id       → le compte vit-il encore
                ├── servi = min(clé.role, porteur.role)
                └── touch(last_used_at) si la dernière trace a plus d'une minute
                        │
                        └── Claims { sub, role: servi, jti: id de la clé, … }
                                │
                                └── Identity { user_id, role } — require_role s'applique
```

La valeur du fragment tient entière dans cette propriété : un CRUD engendré il y a six
mois, un `require_role(Role::Admin)`, une route d'un autre fragment — tous acceptent une
clé le jour de l'installation, sans relecture ni réécriture. C'est aussi ce qui en fait sa
principale surface de risque, et le §5 la borne.

---

## Les décisions

### 1. Une clé vaut `Identity` partout, et non « là où on l'admet »

Arbitré par le mainteneur. L'alternative — un extracteur `ApiIdentity` engendré, que le
développeur substitue à la main dans les handlers qu'il veut ouvrir — laissait le CRUD
engendré fermé aux machines le jour de l'installation, c'est-à-dire ne livrait qu'une
table et trois routes de gestion.

Conséquence assumée : une clé ouvre aussi `/auth/change-password` et `/auth/sessions`. Le
guide le dit en toutes lettres, et c'est le §5 qui rend la chose gouvernable — une clé de
rôle `user` n'administre rien.

### 2. Les deux façons d'éviter de toucher au noyau sont mortes, et c'est vérifié

**Le middleware qui forge un JWT court.** Une couche posée dans `<rbs:layers>` lisant
`X-Api-Key` et réécrivant `Authorization: Bearer <jwt de 60 s>` n'aurait coûté au noyau
aucune ligne. Elle est impossible **dès lors que le rôle est plafonné** (§5) :
`templates/features/auth/mod.rs.jinja:70` refuse tout jeton dont le rôle diffère du rôle
courant du compte —

```rust
if compte.role.to_value() != claims.role {
    return Err(Error::Unauthorized);
}
```

— si bien qu'une clé en lecture seule appartenant à un administrateur, qui est le cas
d'usage même du plafonnement, se ferait rendre 401 par le fragment `auth`. La rescaper
demanderait de retoucher `auth/mod.rs` *et* `guard.rs` : le problème d'AST qu'on cherchait
à éviter, en deux fichiers au lieu d'un.

**Un trait séparé, implémenté dans un fichier que le fragment possède.**
`impl<S: HasAuth> FromRequestParts<S> for Identity` ne peut pas gagner une borne
`+ HasApiKeys` sans rompre tous les projets existants ; et une implémentation générale
`impl<T: HasAuth> HasApiKeys for T` interdirait justement au projet d'écrire la sienne,
deux implémentations entrant en conflit.

Il reste donc une méthode sur `HasAuth`, que le projet surcharge — et le projet ne peut la
surcharger que **dans le bloc `impl` qu'`auth` possède**.

### 3. Une dix-septième ancre : `// <rbs:auth_impl>`

`impl HasAuth for AppState` vit dans `src/auth/mod.rs:24`, déposé par le fragment `auth`,
avec des corps de méthode. Le CLI ne réécrit jamais d'AST : il faut une ancre **dans** le
bloc.

```rust
pub(crate) const AUTH_IMPL: Anchor = Anchor {
    name: Cow::Borrowed("auth_impl"),
    file: Cow::Borrowed("src/auth/mod.rs"),
    comment: "//",
    sorted: false,
    optional: true,
    after: "impl HasAuth for AppState {",
};
```

`optional: true` : un projet sans `auth` n'a pas ce fichier, et `doctor` ne doit pas le
tenir pour incomplet — la même raison que `modules`, `services`, `jobs` et `schedules`.
`sorted: false` : le contenu est une méthode, que rustfmt ne réordonne pas. L'accroche est
la ligne d'ouverture de l'`impl`, stable et unique dans le fichier.

Ce que le fragment y insère est une délégation, et rien d'autre — le corps vit dans le
module que le fragment possède :

```rust
async fn accept_key(
    &self,
    key: &str,
    extensions: &mut Extensions,
) -> rbs_core::Result<Claims> {
    crate::modules::api_keys::service::accept(self, key, extensions).await
}
```

**Un projet engendré avant cette version n'a pas l'ancre.** `rbs add api-keys` n'y écrit
alors rien et affiche le bloc à coller : la convention du dépôt, déjà éprouvée sous le code
d'erreur `ancre_absente`.

### 4. `HasAuth::accept_key` : additive, refusant par défaut

```rust
/// Ce que le projet fait d'une clé d'API portée par `X-Api-Key`.
///
/// Le défaut refuse : un projet sans le fragment `api-keys` ne connaît aucune clé, et
/// l'en-tête n'y ouvre rien.
fn accept_key(
    &self,
    key: &str,
    extensions: &mut axum::http::Extensions,
) -> impl std::future::Future<Output = crate::Result<crate::jwt::Claims>> + Send {
    let _ = (key, extensions);
    async { Err(crate::Error::Unauthorized) }
}
```

`Identity::from_request_parts` lit `Authorization: Bearer` **puis**, à défaut seulement,
`X-Api-Key`. Le jeton l'emporte quand les deux sont présents : c'est le justificatif le
plus spécifique, et un mandataire qui injecterait une clé de service ne doit pas
supplanter celui que l'appelant a présenté.

Les `extensions` sont dans la signature pour la même raison qu'`accept_in` les porte : le
projet vient de lire le compte pour juger la clé, et laisse ce qu'il en sait à
l'extracteur suivant, qui n'a pas à relire la même ligne.

**Pourquoi un `Claims` et non un type neuf.** `Identity` n'en lit que `sub` et `role`,
mais les trois autres champs reçoivent une valeur qui a un sens, et non un remplissage :
`jti` porte l'identifiant de la clé — ce qui met dans les journaux *laquelle* a servi —,
`iat` sa création, `exp` son échéance ou une échéance lointaine quand elle n'en a pas. Le
noyau ne revérifie pas `exp` sur ce chemin : la péremption est dans la condition de la
requête SQL (§7), et une seconde garde à un autre endroit divergerait un jour.

### 5. Le rôle : propre à la clé, plafonné par le porteur

Arbitré par le mainteneur, contre l'héritage pur du rôle du porteur.

- Une colonne `role` sur `api_keys`.
- À la création, un rôle supérieur à celui du créateur est refusé — **403 avant toute
  écriture**, comme tous les refus du dépôt.
- À l'acceptation, le rôle servi est `min(clé.role, porteur.role)` : une clé
  administratrice cesse d'administrer le jour où son porteur est rétrogradé, sans qu'il
  faille penser à la révoquer.

Le plafond et la garde reposent sur le **même** `Ord` que `require_role` : celui que
l'ordre de déclaration de `Role` porte, documenté dans `model.rs.jinja:8-10`. Un rôle
ajouté à la main dans `src/auth/model.rs` devient donc utilisable par les clés sans
migration — la propriété qu'a déjà la garde.

Ce que ça achète : la clé en lecture seule créée par un administrateur, qui est le cas
d'usage machine le plus courant, et que l'héritage pur rendait impossible autrement qu'en
ouvrant un second compte humain pour la machine.

### 6. `last_used_at`, borné à une écriture par minute et par clé

Une clé qu'on n'ose pas révoquer parce que personne ne sait si elle sert encore est une
clé éternelle. La colonne répond à cette question, et c'est la seule qu'on lui pose : la
précision de la minute suffit.

La borne est tenue **en Rust, avant tout aller-retour** — la ligne vient d'être lue, son
`last_used_at` est sous la main :

```rust
if cle.last_used_at.is_none_or(|trace| maintenant - trace >= SEUIL) {
    // détaché : la réponse n'attend pas l'écriture
    tokio::spawn(repository::touch(db.clone(), cle.id, maintenant));
}
```

Sans ce test, un `UPDATE … WHERE … AND last_used_at < now - 60s` conditionnel coûterait un
aller-retour par requête pour ne rien changer 59 fois sur 60. La condition reste **aussi**
dans l'`UPDATE`, où elle ferme la concurrence entre deux requêtes simultanées de la même
clé — la règle qu'`one_time_token::consume` a posée : ce qui décide va dans la condition,
pas dans la lecture qui précède.

L'écriture est détachée et son échec ne refuse pas la requête : une trace manquée vaut
mieux qu'un appel refusé.

`touch` prend la connexion **par valeur**, là où toutes ses voisines prennent
`&impl ConnectionTrait` : une tâche détachée est `'static` et ne peut rien emprunter.
`DatabaseConnection` se clone à coût nul — un `Arc` interne. C'est le seul endroit du
fragment qui s'écarte de la signature des repositories, et le commentaire le dit.

### 7. La table `api_keys`

| Colonne | Type | Rôle |
|---|---|---|
| `id` | `uuid` PK | UUIDv7 posé par `ActiveModelBehavior::new()`, comme partout |
| `user_id` | `uuid` not null, indexé | le porteur |
| `name` | `string` not null | « ci », « script de facturation » — validé 1..80 |
| `prefix` | `string` not null | les 12 premiers caractères de la clé, pour la reconnaître dans la liste |
| `token_hash` | `string` not null, **index unique** | SHA-256 de la clé entière |
| `role` | `string` not null | `crate::auth::model::Role` |
| `last_used_at` | `timestamptz` null | §6 |
| `expires_at` | `timestamptz` null | facultatif ; `null` = ne périme pas |
| `revoked_at` | `timestamptz` null | la révocation, datée — comme `refresh_tokens` |
| `created_at` / `updated_at` | `timestamptz` | |

La clé rendue est `rbs_` suivi des 43 caractères de `rbs_core::token::random()` — 32
octets, base64url sans remplissage, la fonction même des jetons de rafraîchissement. En
base, `token::fingerprint` de la clé **entière**, et elle seule : une base lue par un tiers
ne lui donne aucune clé utilisable, exactement comme pour `one_time_tokens`.

**La clé en clair ne quitte le processus qu'une fois**, dans la réponse de création. La
liste ne rend que `prefix`. C'est la règle que le secret d'abonnement webhooks a posée
(`webhooks/dto.rs.jinja:31`) : une seule lecture de la liste livrerait sinon tout.

L'index unique sur `token_hash` n'est pas cosmétique — c'est le chemin de lecture de
**chaque requête machine**.

Péremption et révocation vivent dans la condition de la recherche, jamais après elle :

```rust
.filter(api_key::Column::TokenHash.eq(empreinte))
.filter(api_key::Column::RevokedAt.is_null())
.filter(
    Condition::any()
        .add(api_key::Column::ExpiresAt.is_null())
        .add(api_key::Column::ExpiresAt.gt(maintenant)),
)
```

**Deux lectures, jamais de jointure.** Aucun repository engendré ne joint — pas un
`find_also_related`, pas un `JoinType` dans tout `templates/` — et tous les `Relation` des
fragments sont des ancres vides. La lecture de `users` qui suit est celle qu'`admit` fait
déjà pour un jeton : une requête authentifiée coûte une lecture de compte, par clé comme
par jeton.

### 8. Les routes, et ce que « je révoque tout » veut dire

Calquées sur `/auth/sessions`, qui administre le même genre d'objet.

| Route | Rend | Notes |
|---|---|---|
| `POST /api-keys` | 201 `{id, name, prefix, role, expires_at, key}` | `key` une seule fois ; **403** si le rôle demandé dépasse celui du créateur |
| `GET /api-keys` | 200, les siennes | jamais `token_hash`, jamais `key` |
| `DELETE /api-keys/{id}` | 204 | **404** si ce n'est pas la sienne — et non 403, qui dirait qu'elle existe |
| `DELETE /api-keys` | 204 | toutes les siennes |

Le corps de création : `{ name, role?, expires_in_days? }`. `role` absent vaut `user` —
le défaut le moins ouvert. `expires_in_days` absent vaut « ne périme pas ».

`ApiKeyCreated` porte son propre `impl IntoResponse` avec `Cache-Control: no-store` et
`Pragma: no-cache`, **sur le type et non dans le handler** : la règle que `TokenPair` a
posée pour qu'un second handler ne puisse pas l'oublier (`auth/dto.rs.jinja:76-91`).

**Une clé peut créer une clé.** Le plafond du §5 interdit l'escalade, et l'interdire
casserait le provisionnement automatisé qui est la raison d'être du fragment.

**`DELETE /auth/sessions` ne touche pas aux clés**, et `DELETE /api-keys` ne touche pas aux
sessions : deux surfaces, deux gestes, chacun nommé dans son guide. Arbitré par le
mainteneur contre un `sessions_revoked_at` qui fermerait tout d'un coup — un humain qui
clique « déconnecter partout » depuis une application ne veut pas arrêter la CI, et rien
dans cette interface ne l'en avertirait.

### 9. Le document OpenAPI : le schéma vient du noyau

Le schéma `bearer` que citent toutes les routes fermées n'est pas déclaré dans le gabarit :
`openapi.rs.jinja` ne porte que `modifiers(&CommonResponses)`, et c'est `CommonResponses`
qui le pose, dans `rbs-core/src/openapi.rs`, sous `#[cfg(feature = "auth")]`.

Déclarer `X-Api-Key` s'y fait donc **additivement dans le noyau**, sans seconde ancre et
sans toucher un fichier du projet — un projet déjà engendré le reçoit à son prochain
`upgrade`. Le noyau gagne un `KEY_SCHEME_NAME` à côté de `SCHEME_NAME`, et les quatre
routes du fragment déclarent `security(("bearer" = []), ("api_key" = []))`.

**Limite assumée, écrite dans le guide** : `rbs generate client` tient une opération pour
protégée dès que sa `security` est non vide (`client/document.rs:194`) mais n'émet qu'un
porteur `Bearer`. Le client TypeScript engendré ne saura pas envoyer une clé. Hors
périmètre de cette tâche.

### 10. Deux retouches au fragment `auth`, et pas une de plus

1. L'ancre `// <rbs:auth_impl>` dans `templates/features/auth/mod.rs.jinja`, à l'intérieur
   de l'`impl HasAuth for AppState`.
2. `guard::Accepted` passe de `pub(super)` à `pub(crate)` : il vit dans `src/auth/guard.rs`
   et le fragment, qui vit dans `src/modules/api_keys/`, doit pouvoir l'y déposer pour que
   `VerifiedIdentity` ne relise pas le compte qu'on vient de lire.

Les deux touchent un gabarit versionné : `blog-auth` et `event-hub` se régénèrent, et
`integration_examples` reste rouge tant qu'ils ne l'ont pas été.

---

## Les fichiers

```
crates/rbs-core/src/state.rs         + HasAuth::accept_key, défaut refusant
crates/rbs-core/src/extract.rs       + le chemin X-Api-Key dans Identity
crates/rbs-core/src/openapi.rs       + le schéma de sécurité api_key
crates/rbs-cli/src/anchors.rs        + AUTH_IMPL, ANCRES passe à 17
crates/rbs-cli/src/doctor/api_keys.rs        le contrôle de la délégation
crates/rbs-cli/src/doctor/mod.rs             son inscription au tableau
crates/rbs-cli/src/lib.rs                    le conseil de suite (migrate up, créez une clé)
crates/rbs-cli/templates/features/auth/mod.rs.jinja    l'ancre
crates/rbs-cli/templates/features/auth/guard.rs.jinja  pub(crate)

crates/rbs-cli/templates/features/api-keys/
    feature.toml          requires = ["auth"] ; ancres modules, routes, openapi, auth_impl
                          [cargo.rbs-core] features = ["auth"] — le fragment en dépend pour son propre compte
    mod.rs.jinja          routes()
    model.rs.jinja        l'entité
    dto.rs.jinja          ApiKeyCreated (no-store), ApiKeyResponse, CreateApiKey
    repository.rs.jinja   find_by_fingerprint, insert, touch, revoke, revoke_all, list
    service.rs.jinja      accept, create (le plafond), les quatre gestes
    controller.rs.jinja   les quatre routes
    migration.rs.jinja    create_api_keys
    tests/mod.rs.jinja    tests/accept.rs, tests/cap.rs, tests/routes.rs
```

`examples/event-hub` gagne le fragment : c'est ce qui le fait **compiler en CI**, un
fragment n'étant compilé nulle part ailleurs.

---

## Ce qui se prouve, et où

| Critère | Preuve |
|---|---|
| Le plan, les ancres, l'idempotence, le refus sur ancre absente | `cargo test -p rbs-cli --lib -- add::` |
| `ANCRES` tient 17 entrées, sans doublon, accroche vérifiée contre le gabarit | `--lib -- anchors::` |
| La délégation manquante est nommée avec sa ligne à coller | `--lib -- doctor::api_keys` |
| La constante du doctor est bien celle que le `feature.toml` insère | idem, sur le modèle de `doctor::webhooks::the_line_is_the_one_the_fragment_inserts` |
| Le noyau : `X-Api-Key` accepté, refusé par défaut, `Bearer` prioritaire | `cargo test -p rbs-core --all-features -- extract::` |
| Une clé ouvre un CRUD engendré ; le plafond ferme un `--role admin` | `integration_api_keys -- --ignored` (Docker) |
| Péremption, révocation, 404 sur la clé d'autrui, `no-store` | idem |
| `last_used_at` : une écriture, pas trois, sur trois appels rapprochés | tests engendrés du fragment |
| Les exemples ne dérivent pas | `integration_examples` |
| La documentation ne ment pas | `integration_docs`, `npm run build`, `npm run parite` |

Les tests engendrés (`src/modules/api_keys/tests/`) sont le seul endroit où le code du
fragment est **exécuté** ; `cargo check` sur `event-hub` est le seul endroit où il est
compilé avant la passe Docker.

---

## Écarté, et pourquoi

- **Des portées (`scopes`) par clé.** Le projet a déjà une hiérarchie d'autorisation,
  `Role`, sur laquelle reposent toutes les gardes engendrées. Une seconde, orthogonale,
  demanderait de réécrire `require_role` et n'aurait de sens que si les routes engendrées
  savaient la citer.
- **Le préfixe `rbs_live_` / `rbs_test_`.** Le projet engendré n'a pas de notion
  d'environnement dans ses jetons ; l'introduire ici la rendrait fausse ailleurs.
- **Une clé sans porteur** — un principal machine autonome. `Identity.user_id` est lu
  comme UUID par `user_uuid()`, et tout ce qui filtre par propriétaire s'appuie dessus.
- **Un `[[env]]`.** Rien de global à tirer : chaque clé porte sa propre valeur, comme
  chaque abonnement webhook porte son propre secret.

---

## Ce qui reste ouvert pour le mainteneur

1. **La version.** `v1.4.0` est le dernier tag et le `CHANGELOG` porte une section
   `[Unreleased]` qui est la 1.5.0. Le fragment y entre, sauf si la 1.5.0 est gelée — et
   il faudrait alors ouvrir une 1.6.0. La rupture reste **mineure dans les deux cas** : une
   méthode à corps par défaut ajoutée à un trait public, un schéma de sécurité de plus.
2. **`api-keys` dans le preset `api`.** `Preset::Full` le prendra automatiquement, étant
   dérivé de la liste des fragments disponibles. `Preset::Api` est nommé, lui : l'y ajouter
   est un choix, et je ne l'ai pas fait.
