---
sidebar_position: 11.7
title: Webhooks
---

# Webhooks sortants

`rbs add webhooks` donne au projet de quoi dire au dehors ce qui vient d'arriver : dix
fichiers sous `src/webhooks/`, une migration pour la table `webhook_subscriptions`, trois
routes, et un POST HTTP signé vers chaque abonné qui écoute.

**Le fragment livre ; il ne décide pas de ce qui mérite d'être dit.** Rien n'est émis tant
que votre code n'appelle pas `webhooks::emit`. Installée et jamais appelée, la feature n'a
aucun effet observable — et c'est délibéré : lesquelles de vos écritures méritent de sortir
du processus est une question que seul votre domaine tranche.

Elle exige `jobs`, parce que la file sait déjà livrer-avec-réessais — réserver une ligne
sans double dépilage, `attempts`, `available_at`, `last_error` — et qu'un second mécanisme
de réessai n'aurait laissé que deux boucles à maintenir. Elle exige aussi `auth`, qui
entraîne `rate-limit` : une création d'abonnement laissée ouverte permettrait à n'importe
qui de faire livrer chez lui les événements du projet, et `user.created` porte des adresses.
Sur un projet nu, les quatre descendent dans un seul plan :

```text
$ rbs add webhooks
webhooks : webhooks sortants : abonnements, signature HMAC horodatée, livraison par la file
webhooks exige auth, jobs, rate-limit : posée avec elle

plan pour /private/tmp/rbs-demo/blog

  + src/auth/mod.rs                                                  créé
  …
  + src/jobs/mod.rs                                                  créé
  …
  + src/rate_limit/mod.rs                                            créé
  …
  + src/webhooks/mod.rs                                              créé
  + src/webhooks/config.rs                                           créé
  + src/webhooks/model.rs                                            créé
  + src/webhooks/dto.rs                                              créé
  + src/webhooks/repository.rs                                       créé
  + src/webhooks/service.rs                                          créé
  + src/webhooks/controller.rs                                       créé
  + src/webhooks/signature.rs                                        créé
  + src/webhooks/delivery.rs                                         créé
  + src/webhooks/tests.rs                                            créé
  + migration/src/m20260904_160207_create_webhook_subscriptions.rs   créé
  ~ AGENTS.md                                                        modifié

  43 fichiers à écrire
✓ webhooks installée — 32 fichiers

  rbs migrate up, inscrivez un abonné par POST /webhooks/subscriptions — son secret n'est rendu qu'à cet instant — puis appelez webhooks::emit dans vos services
```

Trois migrations l'accompagnent : [`rbs migrate up`](../cli/migrate.md) est la commande
suivante.

## Émettre un événement

```rust
webhooks::emit(&transaction, "user.created", &dto).await?;
```

`emit` prend un `&C: ConnectionTrait` et non une connexion, et c'est tout l'intérêt :
**passez-lui la transaction qui porte votre changement, et les livraisons naissent si et
seulement si elle est committée.** Un `user.created` livré pour une inscription annulée est
un mensonge qu'aucun réessai ne rattrape.

Elle rend le nombre de livraisons enfilées. Aucun abonné concerné n'est pas une erreur : un
projet sans webhook configuré émet dans le vide, ce qui est le cas nominal.

La charge utile est n'importe quoi de `Serialize`, et en pratique c'est le DTO que votre API
rend déjà. Ce qu'elle ne doit *pas* être, c'est votre entité : un corps de webhook est un
contrat public, et le coupler à la table fait de chaque colonne renommée une rupture pour
tous les abonnés.

## S'abonner

Trois routes, toutes exigeant un jeton valide :

| Route | Effet |
|---|---|
| `POST /webhooks/subscriptions` | Inscrit une URL et les motifs qu'elle écoute. **Rend le secret — cette fois-là et jamais plus.** |
| `GET /webhooks/subscriptions` | Liste tous les abonnements, révoqués compris, sans leurs secrets |
| `DELETE /webhooks/subscriptions/{id}` | En révoque un, en datant `revoked_at` |

```json
{ "url": "https://example.test/hooks", "events": ["user.*"] }
```

**Le secret appartient à l'abonnement, non au projet.** Un secret commun donnerait à chaque
abonné de quoi contrefaire les événements livrés à tous les autres. Il n'est rendu que par
la réponse de création : une seule lecture de la liste livrerait sinon les secrets de tout
le monde d'un coup.

`Identity` ne dit que « le jeton est valide ». Un projet qui réserve l'administration des
abonnements à un rôle remplace l'extracteur par sa propre garde — voir `src/auth/guard.rs`
et le [guide auth](./auth.md).

## Les motifs d'événement

Trois formes, et pas une de plus :

| Motif | Correspond à |
|---|---|
| `*` | tout événement |
| `user.*` | tous ceux de la famille |
| `user.created` | lui-même, et rien d'autre |

Le tri se fait en Rust et non en SQL, délibérément : la recherche dans un tableau JSON n'a
pas de forme commune à PostgreSQL, MySQL et SQLite, et les motifs à préfixe l'auraient de
toute façon interdite. Le prix est une lecture de la table des abonnements par événement
émis — sans conséquence tant que les abonnés se comptent en centaines, et ce qu'il faudra
reprendre s'ils se comptent un jour en millions.

## Ce que lit le receveur

Le corps est l'enveloppe, non votre charge utile seule :

```json
{
  "id": "0192f3a0-…",
  "event": "user.created",
  "created_at": "2026-09-04T16:02:07+00:00",
  "data": { "…": "ce que vous avez passé à emit" }
}
```

Trois en-têtes l'accompagnent :

| En-tête | Porte |
|---|---|
| `X-Rbs-Signature` | `t=<secondes unix>,v1=<HMAC-SHA256 hexadécimal>` |
| `X-Rbs-Event` | le nom de l'événement, lisible sans ouvrir le corps |
| `X-Rbs-Delivery` | l'`id` de l'enveloppe, **stable d'un réessai à l'autre** |

C'est par le dernier qu'un receveur déduplique. La file peut livrer deux fois — une réponse
perdue après que le travail a été fait y suffit — et sans cet identifiant, rien ne le lui
dirait.

## Vérifier la signature

Les octets signés sont l'horodatage, un point, puis le corps verbatim :

```text
HMAC-SHA256(secret, "<horodatage>.<corps brut>")
```

**L'horodatage entre dans la signature**, et c'est ce qui ferme le rejeu : un tiers qui capte
une livraison ne peut pas la resservir plus tard sous une date fraîche sans invalider le
condensat. Un receveur doit refuser un horodatage trop éloigné de sa propre horloge — cinq
minutes est la tolérance d'usage — et comparer les condensats en temps constant.

```js
const [t, v1] = header.split(',').map((part) => part.split('=')[1]);
const attendu = crypto.createHmac('sha256', secret).update(`${t}.${corpsBrut}`).digest('hex');
const ok = crypto.timingSafeEqual(Buffer.from(v1), Buffer.from(attendu));
```

`v1=` nomme le schéma plutôt que de laisser le condensat nu : le jour où un second arrive,
les deux cohabitent dans le même en-tête et un receveur à jour choisit.

Vérifiez sur le **corps brut**, avant tout parsing JSON. Re-sérialiser un objet parsé peut
réordonner les clés, et le condensat d'un corps re-sérialisé n'est pas celui qui a été
signé. L'émetteur a la même contrainte et la respecte : l'enveloppe est sérialisée une fois,
et ce sont ces octets-là qui sont signés puis envoyés.

## Livraison, réessais et doublons

Une émission enfile un job par abonné qui écoute ; le worker fait le reste. Les réessais,
l'attente entre deux tentatives, `attempts` et `last_error` sont ceux de la file, inchangés
— voir le [guide jobs](./jobs.md).

**L'abonnement est désigné par son identifiant, non recopié dans le job.** L'URL et le secret
sont relus au dépilage, si bien qu'un secret tourné s'applique aux livraisons déjà en file
et qu'une révocation les arrête. Une livraison dont l'abonnement a été révoqué — ou supprimé
— se termine en succès sans requête HTTP : il n'y a rien à livrer et rien à réessayer, et
rendre une erreur dépenserait cinq tentatives sur une ligne qui ne compte plus.

Toute réponse hors 2xx vaut échec, **4xx comprises**. Un receveur qui répond 400 à une
livraison bien formée est en panne, et le distinguer d'un 503 demanderait de deviner
laquelle des deux parties a tort.

La livraison est au-moins-une-fois, jamais exactement-une-fois. Ce n'est pas un manque à
combler plus tard : un receveur qui répond après avoir fait le travail mais avant que sa
réponse n'arrive doit être livré de nouveau, et `X-Rbs-Delivery` est ce qui rend cela sans
conséquence.

## La table

| Colonne | Type | Note |
|---|---|---|
| `id` | `uuid`, clé primaire | UUIDv7, posé par l'application |
| `url` | `varchar(191)` | Où la livraison est POSTée |
| `events` | `json` | Les motifs écoutés |
| `secret` | `varchar(191)` | Le secret de signature de cet abonnement |
| `revoked_at` | `timestamptz`, nullable | La révocation, datée. Nulle tant que l'abonnement sert |
| `created_at` | `timestamptz` | |
| `updated_at` | `timestamptz` | |

Aucun index de plus, et ce n'est pas un oubli : l'émission lit tous les abonnements non
révoqués pour les trier elle-même, et un index sur une colonne toujours lue en entier ne
rapporte rien tout en coûtant à chaque écriture.

## Configuration

```toml
[webhooks]
timeout_secs = 10
```

Un seul réglage : le temps laissé au receveur pour répondre. Au-delà, la livraison compte
pour un échec et repart en réessai — un endpoint lent ne doit pas retenir un worker.
`config/{env}.toml` et `RBS_WEBHOOKS__TIMEOUT_SECS` le surchargent comme pour toute autre
section — voir le [guide de configuration](./configuration.md).

Le client HTTP vit dans l'`AppState` et se construit une fois au démarrage : un
`reqwest::Client` porte son pool de connexions, et le reconstruire à chaque livraison
rouvrirait une session TLS par tentative. Un délai d'expiration illisible arrête donc le
démarrage plutôt que de se découvrir six heures plus tard dans le journal d'un worker.

## Les tests

Le `src/webhooks/tests.rs` engendré couvre les deux moitiés séparément. La signature est
prouvée contre **un vecteur calculé hors de Rust**, si bien que le test survivrait à une
réécriture du code de signature et attraperait un changement de schéma ; la correspondance
des motifs est prouvée sur ses trois formes.

La moitié qui touche la base est celle qui justifie la conception :

- une émission enfile une livraison par abonnement qui écoute, et aucune pour un abonné qui
  n'écoute pas ;
- un abonnement révoqué ne reçoit rien ;
- une livraison dont l'abonnement a été révoqué entre l'émission et le dépilage **réussit
  sans requête HTTP** ;
- **une émission annulée avec sa transaction n'enfile rien** — le test qui fait de la
  signature `&C: ConnectionTrait` autre chose qu'une coquetterie.

## Ce qu'elle vous laisse

- **ce qui mérite d'être émis** — `emit` est appelée par votre code ou par personne ;
- **la forme de vos charges utiles** — aucun versionnage de l'enveloppe au-delà du `v1` de
  la signature, et aucun schéma publié aux abonnés ;
- **les webhooks entrants** — le fragment signe ce qu'il envoie et ne vérifie rien de ce
  qu'il reçoit ;
- **le disjoncteur** — un endpoint mort depuis une semaine est réessayé comme un autre, et
  rien ne désactive un abonnement qui ne répond jamais ;
- **un journal des livraisons** — le `last_error` du job dit pourquoi la dernière tentative
  a échoué, et le job parti, la trace l'est aussi. [`rbs add audit`](./audit.md) est le
  fragment qui en garde un.
