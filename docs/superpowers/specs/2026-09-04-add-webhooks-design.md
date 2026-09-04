# `rbs add webhooks` — conception

**Date** : 2026-09-04
**Tâche** : `IMPROVE.md` n° 80 — *[Feature] `rbs add webhooks`*
**Portée** : webhooks **sortants** seulement. Les entrants — un extracteur qui vérifie une
signature reçue — sont explicitement hors périmètre.

---

## Ce que le fragment livre

Une table d'abonnements, trois routes pour l'administrer, une fonction d'émission
appelable depuis le code du projet, et une livraison signée qui passe **par la file
`jobs`** : `webhooks` n'a ni boucle, ni compteur de tentatives, ni horloge à lui.

```
webhooks::emit(&transaction, "user.created", &payload).await?
        │
        ├── lit les abonnements actifs qui écoutent `user.created`
        └── enfile un job `webhooks::deliver` par abonné, dans la transaction reçue
                │
                └── le worker de `jobs` le dépile, le signe et le POSTe
                        échec → réessai de `jobs`, épuisé → `failed`
```

La table `jobs` porte déjà tout ce qui fait la livraison-avec-réessais : réservation sans
double dépilage sur trois moteurs, `attempts`, `available_at`, `last_error`,
`max_attempts`. Y ajouter un second mécanisme n'aurait laissé que deux boucles à
maintenir. `requires = ["jobs"]`, donc, comme `scheduler`.

---

## Les décisions

### 1. La table : `webhook_subscriptions`

| Colonne | Type | Rôle |
|---|---|---|
| `id` | `uuid` PK | UUIDv7, posé par `ActiveModelBehavior::new()` comme partout ailleurs |
| `url` | `string` | où POSTer |
| `events` | `json` | les motifs écoutés, tableau de chaînes |
| `secret` | `string` | le secret de signature **propre à cet abonnement** |
| `revoked_at` | `timestamptz` null | la révocation, datée |
| `created_at` / `updated_at` | `timestamptz` | |

Aucun index supplémentaire, et ce n'est pas un oubli : l'émission lit tous les abonnements
non révoqués — le filtrage par événement se fait en Rust, voir §4 — et un index sur
`revoked_at`, colonne nulle dans l'immense majorité des lignes, n'épargnerait pas le
parcours. La table compte le nombre d'abonnés du projet, pas le nombre d'événements émis.

### 2. Le secret appartient à l'abonnement, pas au projet

**Aucun `[[env]]`.** Le fragment ne dépose aucun secret global dans le `.env`, à la
différence d'`auth`.

La raison est que la signature n'a de sens qu'entre deux parties : un secret unique pour
tout le projet donnerait à chaque abonné de quoi contrefaire les événements livrés à tous
les autres. Chaque abonnement tire donc le sien à sa création, par
`rbs_core::token::random()` — 32 octets, base64url sans remplissage, la même fonction que
les jetons de rafraîchissement d'`auth`.

Il est stocké **en clair**, et ne peut pas ne pas l'être : la livraison doit le relire pour
signer, là où un mot de passe n'a jamais à être relu. La table est donc aussi sensible que
les données qu'elle protège, et la documentation le dit.

Le secret n'est rendu **qu'à la création**, jamais par la liste : une seule lecture de
`GET /webhooks/subscriptions` livrerait sinon les secrets de tous les abonnés d'un coup.

### 3. La signature : `X-Rbs-Signature: t=<timestamp>,v1=<hex>`

Le schéma de Stripe, repris tel quel :

```
signé   = HMAC-SHA256(secret, "<t>.<corps JSON exact>")
en-tête = X-Rbs-Signature: t=1757000000,v1=3f9a…
```

L'horodatage entre dans la signature, ce qui ferme le rejeu : un tiers qui capte une
livraison ne peut pas la resservir plus tard sous une date fraîche sans invalider le `v1`.
Le préfixe `v1=` nomme le schéma, pour qu'un `v2` puisse un jour cohabiter avec lui dans
le même en-tête.

Le corps est signé **octet pour octet tel qu'il part** : le job sérialise une fois, signe
ces octets-là et les envoie. Sérialiser deux fois exposerait à ce que l'ordre des clés
change entre la signature et l'envoi.

Deux autres en-têtes accompagnent la livraison :

- `X-Rbs-Event: user.created` — le type, lisible sans ouvrir le corps ;
- `X-Rbs-Delivery: <uuid>` — l'identifiant de l'**événement**, stable d'un réessai à
  l'autre. C'est ce qui permet au receveur de dédupliquer : la file peut livrer deux fois
  (une réponse perdue après traitement), et sans cet identifiant il n'aurait aucun moyen
  de s'en apercevoir.

### 4. Le filtrage par événement : en Rust, trois formes de motif

`events` porte un tableau de motifs. Un motif vaut pour un événement si :

| Motif | Signification |
|---|---|
| `*` | tous les événements |
| `user.*` | tout événement dont le nom commence par `user.` |
| `user.created` | celui-là exactement |

Le filtrage est fait **en Rust et non en SQL**, et c'est délibéré : la recherche dans un
tableau JSON n'a pas de forme commune à PostgreSQL, MySQL et SQLite, et les motifs à
préfixe l'auraient de toute façon interdite. L'émission lit les abonnements actifs et les
trie elle-même. Le coût est une lecture de table par événement émis — acceptable tant que
les abonnés se comptent en centaines, ce que la documentation dit franchement.

### 5. Le statut : `revoked_at`, une date et non un booléen

Une date porte le booléen *et* le moment. `DELETE /webhooks/subscriptions/{id}` la pose,
il ne supprime pas la ligne : l'historique d'un abonnement révoqué reste lisible, et une
livraison déjà en file trouve encore sa ligne.

La révocation est relue **à la livraison**, pas seulement à l'émission : un abonnement
révoqué entre l'enfilage et le dépilage ne reçoit rien, et le job se termine en succès
plutôt qu'en échec — il n'y a rien à réessayer.

### 6. Le contenu livré

```json
{
  "id": "0199a1f2-…",
  "event": "user.created",
  "created_at": "2026-09-04T10:00:00+00:00",
  "data": { … ce que l'appelant a passé à `emit` … }
}
```

`id` est tiré à l'émission et voyage dans la charge utile du job : il est donc le même à
chaque réessai, ce qui est toute sa raison d'être. `data` est la valeur sérialisée telle
quelle — `emit` est générique sur `T: Serialize`, l'appelant y met son DTO.

### 7. Les routes, et pourquoi elles exigent `auth`

| Route | Réponse |
|---|---|
| `POST /webhooks/subscriptions` | 201 — l'abonnement **et son secret**, rendu cette seule fois |
| `GET /webhooks/subscriptions` | 200 — la liste, sans les secrets |
| `DELETE /webhooks/subscriptions/{id}` | 204 — révoqué ; 404 si inconnu |

**`requires = ["auth"]`, et les trois handlers prennent l'extracteur `Identity`.**

C'est la décision la plus discutable du lot, et elle mérite son paragraphe. Une route de
création laissée ouverte permet à n'importe qui de faire livrer chez lui les événements du
projet — `user.created` porte des adresses. Ce n'est pas une faiblesse théorique, c'est une
fuite de données offerte au premier venu. Le projet a déjà tranché ce genre d'arbitrage
dans le même sens : `auth` exige `rate-limit` parce que sa protection contre l'énumération
serait sinon un déni de service.

Le prix est réel : `rbs add webhooks` sur un projet nu pose quatre fragments —
`rate-limit`, `auth`, `jobs`, puis `webhooks`. La documentation le montre.

C'est aussi ce qui rend `rbs_core::token::random()` disponible : il vit derrière la feature
`auth` de `rbs-core`, qu'`auth` active déjà et que le manifeste de `webhooks` redéclare
pour son propre compte.

### 8. Une treizième ancre : `// <rbs:jobs>` dans `src/jobs/mod.rs`

Le worker n'exécute que ce que `jobs::registry()` connaît. Le job de livraison doit donc
s'y inscrire — et un fragment ne peut écrire que dans une ancre du registre `ANCRES`
(`add/installation.rs:212` refuse tout autre nom). Il n'existe aujourd'hui aucune ancre
dans `src/jobs/mod.rs` : le commentaire « Inscrivez les vôtres ici » ne s'adresse qu'à un
humain, et aucun fragment ne peut poser un job.

```rust
pub fn registry() -> Registry {
    Registry::new()
        .register::<demo::Log>()
    // <rbs:jobs>
    // </rbs:jobs>
}
```

L'ancre est **optionnelle**, au sens de `Anchor::optional` : son fichier porteur est
déposé par un fragment et non par le squelette, si bien qu'un projet sans `jobs` n'a
aucune raison de la porter. C'est la deuxième après `# <rbs:services>`, et pour la même
raison exactement.

Le corps est un maillon de chaîne d'appels, comme `<rbs:layers>` : un commentaire entre
deux `.register::<…>()` est du Rust valide et rustfmt le laisse en place.

**Conséquence documentaire** : le compte passe de douze ancres à treize, et la
compatibilité annoncée — « les douze noms d'ancres ne changent pas » — reste tenue,
l'ajout étant purement additif. Toutes les pages qui comptent (`compatibility.md`,
`cli/doctor.md`, `cli/add.md`, `cli/generate.md`, `getting-started.md`) sont reprises dans
le même lot, anglais et français.

### 9. Le client HTTP : `reqwest` 0.13, défauts inchangés

Aucun fragment n'apporte encore de client HTTP — `mail` a `lettre`, `storage` a
`aws-sdk-s3`. `reqwest` 0.13 a fait de `rustls` son TLS par défaut : les défauts suffisent
et n'appellent aucun OpenSSL, contrairement à ce qu'imposait la 0.12.

Le client vit dans l'`AppState`, par les ancres `state_champs` et `state_init`, avec un
accesseur `AppState::webhooks()` — la forme que `redis` a retenue, et non le champ public
nu de `mail` et `storage`. Un client par processus, donc un pool de connexions partagé par
toutes les livraisons ; le construire à chaque job rouvrirait une session TLS par
tentative.

---

## Les fichiers

```
crates/rbs-cli/templates/features/webhooks/
  feature.toml
  mod.rs.jinja          → src/webhooks/mod.rs         modules, routes(), matches(), réexports
  config.rs.jinja       → src/webhooks/config.rs      section [webhooks]
  model.rs.jinja        → src/webhooks/model.rs       entité webhook_subscriptions
  dto.rs.jinja          → src/webhooks/dto.rs         requêtes et réponses des trois routes
  repository.rs.jinja   → src/webhooks/repository.rs  seul à parler à SeaORM
  service.rs.jinja      → src/webhooks/service.rs     emit(), création, liste, révocation
  controller.rs.jinja   → src/webhooks/controller.rs  les trois handlers, annotés utoipa
  signature.rs.jinja    → src/webhooks/signature.rs   HMAC-SHA256 et forme de l'en-tête
  delivery.rs.jinja     → src/webhooks/delivery.rs    le Job, l'enveloppe, le Sender
  migration.rs.jinja    → migration/src/m…_create_webhook_subscriptions.rs
  tests.rs.jinja        → src/webhooks/tests.rs
```

Ancres visées : `features`, `routes`, `openapi`, `state_champs`, `state_init`, `jobs`.

Hors du fragment :

- `crates/rbs-cli/src/anchors.rs` — la constante `JOBS`, `ANCRES` à treize, les tests ;
- `crates/rbs-cli/templates/features/jobs/mod.rs.jinja` — le bloc d'ancre ;
- `crates/rbs-cli/src/lib.rs` — le conseil de fin d'installation ;
- `crates/rbs-cli/src/cli.rs`, `templates/agents/{en,fr}.md.jinja` — l'énumération ;
- `crates/rbs-cli/tests/integration_webhooks.rs` ;
- `docs/` en deux langues, `CHANGELOG.md` et `CHANGELOG.fr.md`.

---

## Ce qui se prouve, et où

**Tests livrés au projet** (`src/webhooks/tests.rs`), sans base :

1. la signature couvre l'horodatage — deux `t` donnent deux `v1` ;
2. l'en-tête a la forme `t=…,v1=…`, le `v1` en hexadécimal minuscule ;
3. un motif exact ne vaut que pour lui-même ;
4. `user.*` vaut pour `user.created` et pas pour `order.created` ;
5. `*` vaut pour tout.

Avec base (`#[ignore]`, comme partout) :

6. une émission enfile un job par abonné concerné ;
7. un abonné révoqué n'en reçoit aucun ;
8. un abonné qui n'écoute pas l'événement n'en reçoit aucun ;
9. la livraison d'un abonnement révoqué est un succès sans requête HTTP ;
10. une émission dans une transaction annulée n'enfile rien.

**`crates/rbs-cli/tests/integration_webhooks.rs`**, sur le moule d'`integration_scheduler` :
un projet neuf, `rbs add webhooks`, `rbs migrate up`, puis les deux flux — `cargo test` et
`cargo test -- --ignored` — dont chaque nom de test est exigé nommément. Plus le contrôle
que tout fichier livré est déclaré au manifeste.

Pas de test sur les trois moteurs : ce que webhooks ajoute au schéma est une colonne JSON
et une colonne de date nullable, or `integration_jobs::the_dequeue_never_hands_the_same_job_twice_on_the_three_engines`
prouve déjà la colonne JSON de `jobs` sur les trois. Un troisième test à trois moteurs
coûterait dix minutes de CI pour reprouver la même chose.

---

## Ce qui reste ouvert pour le mainteneur

1. **`requires = ["auth"]`** (§7). C'est le choix qui ferme le trou plutôt que celui qui
   lit le critère au minimum. Le revenir en arrière coûte trois lignes : retirer le
   `requires`, retirer `Identity` des trois handlers.
2. **La treizième ancre** (§8). Elle est la conséquence mécanique de « l'émission enfile
   un job dans la file existante » ; il n'y a pas de conception sans elle.
