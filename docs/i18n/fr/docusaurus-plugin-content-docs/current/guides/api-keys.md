---
sidebar_position: 11.8
title: Clés d'API
---

# Authentification machine par clés d'API

`rbs add api-keys` donne au projet des justificatifs qu'une machine peut porter : une table,
quatre routes pour les administrer, et un en-tête de plus qu'`Identity` accepte — de sorte
que **tout ce qu'un jeton ouvre déjà, une clé l'ouvre aussi**, sans qu'un seul contrôleur
engendré ne change.

C'est toute la valeur du fragment, et c'est aussi son tranchant. Lisez
[Ce que vaut une clé](#ce-que-vaut-une-clé) avant de l'installer.

Il exige `auth`, qui entraîne à son tour `mail` et `rate-limit` : une clé appartient à un
compte, porte un rôle, et les deux viennent de là. Sur un projet nu, les quatre descendent
dans un seul plan — en voici un extrait :

{/* rbs:transcript cmd="rbs add api-keys" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" extrait="oui" */}
```text
$ rbs add api-keys
api-keys : clés d'API : authentification machine, rôle plafonné par le porteur, trace d'usage
api-keys exige mail, rate-limit, auth : posée avec elle

plan pour …/demo

  + src/modules/mail/mod.rs                                créé
  + src/modules/rate_limit/mod.rs                          créé
  + src/auth/mod.rs                                        créé
  + src/modules/api_keys/mod.rs                            créé
  + src/modules/api_keys/model.rs                          créé
  + src/modules/api_keys/repository.rs                     créé
  + src/modules/api_keys/service.rs                        créé
  + src/modules/api_keys/dto.rs                            créé
  + src/modules/api_keys/controller.rs                     créé
  + src/modules/api_keys/tests/mod.rs                      créé
  + src/modules/api_keys/tests/accept.rs                   créé
  + src/modules/api_keys/tests/routes.rs                   créé
  + migration/src/m20260918_144456_create_api_keys.rs      créé
  ~ AGENTS.md                                              modifié

  57 à créer, 11 à modifier
✓ api-keys installée — 57 créés, 11 modifiés

  rbs migrate up, puis POST /api-keys pour tirer une clé — elle n'est rendue qu'à cet instant — et présentez-la en X-Api-Key
```

Trois migrations l'accompagnent : [`rbs migrate up`](../cli/migrate.md) est la commande
suivante.

## Ce que vaut une clé

Une clé est un second justificatif qu'`Identity` accepte. Présentez-la dans `X-Api-Key`, et
l'extracteur construit la même `Identity` qu'aurait construite un jeton porteur :

```http
GET /articles
X-Api-Key: rbs_kJ3…
```

**Le jeton l'emporte quand les deux sont présentés.** C'est le justificatif le plus
spécifique, et un mandataire qui injecterait une clé de service ne doit pas supplanter celui
que l'appelant a offert.

La conséquence est délibérée et mérite d'être dite sans détour : une clé ouvre **toutes** les
routes que garde `Identity` — y compris `/auth/change-password` et `/auth/sessions`. Rien
dans le code engendré ne distingue une requête faite à la clé d'une requête faite au jeton,
et c'est précisément ce qui rend le fragment utile : un CRUD que vous avez engendré il y a
six mois accepte une clé le jour où vous installez celui-ci, sans une ligne à récrire.

Si c'est plus que ce que vous voulez, le plafond ci-dessous est ce qui le borne.

## Le plafond

Chaque clé porte son propre rôle, et le rôle qui lui est **servi** est le moindre du sien et
de celui de son porteur, recalculé à chaque requête :

- à la création, un rôle supérieur à celui du créateur est refusé par un `403` — nul ne
  délègue plus qu'il ne détient ;
- à chaque requête, le compte est relu, si bien qu'une clé cesse d'administrer le jour où son
  porteur est rétrogradé. Vous n'avez pas à penser à la révoquer.

C'est ce qui achète la clé en lecture seule : un administrateur tire une clé `user` pour un
script, et ce script n'écrira jamais sur une route que le générateur de CRUD a fermée par
`--role admin`.

L'ordre vient de l'énumération `Role` de `src/auth/model.rs` — celle-là même que lit
`require_role`. Un rôle que vous y ajoutez devient utilisable par les clés sans migration.

## Tirer, lister, révoquer

Quatre routes, calquées sur `/auth/sessions`, qui administre le même genre d'objet.

| Route | Rend |
|---|---|
| `POST /api-keys` | `201` avec la clé en clair — **cette seule fois** |
| `GET /api-keys` | les clés de l'appelant, jamais la clé, jamais son empreinte |
| `DELETE /api-keys/{id}` | `204`, ou `404` si elle n'est pas la sienne |
| `DELETE /api-keys` | `204`, toutes les clés de l'appelant |

```http
POST /api-keys
{ "name": "ci", "role": "user", "expires_in_days": 90 }

201 Created
Cache-Control: no-store
{ "id": "…", "name": "ci", "prefix": "rbs_kJ3aB7c", "role": "user",
  "expires_at": "2026-12-17T…", "key": "rbs_kJ3aB7c…" }
```

La clé en clair ne quitte le processus qu'une fois. Ensuite, seul `prefix` revient : assez
pour reconnaître une clé dans une liste, jamais assez pour la présenter. La réponse porte
`Cache-Control: no-store` sur le **type**, non sur le handler, de sorte qu'un second handler
rendant ce corps ne puisse pas l'oublier.

Révoquer la clé d'autrui rend `404`, non `403` : un `403` confirmerait qu'elle existe.

**Une clé peut en tirer une autre.** Le plafond interdit l'escalade, et l'interdire
casserait le provisionnement automatisé, qui est la raison d'être du fragment.

## Tout révoquer

`DELETE /auth/sessions` ferme les sessions du porteur. **Il ne touche pas à ses clés**, et
`DELETE /api-keys` ne touche pas à ses sessions. Deux surfaces, deux gestes.

C'est une décision, non un oubli : un humain qui clique « me déconnecter partout » depuis une
application ne veut pas dire « arrêter la CI ». Si vous voulez les deux, appelez les deux.

## La trace d'usage

`last_used_at` répond à la seule question qu'on pose à un justificatif dormant : cette clé
sert-elle encore ? Une clé que personne n'ose révoquer parce que personne ne sait est une clé
éternelle.

L'écriture est bornée à **une par minute et par clé**, décidée en mémoire avant tout
aller-retour — la ligne vient d'être lue, sa trace précédente est donc déjà sous la main. La
mise à jour part détachée : une trace manquée vaut mieux qu'un appel refusé.

## La table

| Colonne | Rôle |
|---|---|
| `user_id` | le porteur ; une clé ne vaut jamais plus que le compte |
| `name` | ce qu'elle sert — « ci », « script de facturation » |
| `prefix` | les douze premiers caractères, pour la reconnaître dans une liste |
| `token_hash` | SHA-256 de la clé entière, index unique — le chemin de lecture de chaque requête machine |
| `role` | le rôle propre à la clé, plafonné à la lecture |
| `last_used_at`, `expires_at`, `revoked_at` | la trace, l'échéance facultative, la révocation datée |

La clé elle-même n'est jamais stockée. Une base lue par un tiers ne livre aucun justificatif
utilisable — la règle même que suivent les jetons à usage unique d'`auth`.

Péremption et révocation vivent **dans** la condition de la recherche, et non dans une
vérification qui la suivrait : une clé révoquée entre la lecture et le contrôle ne doit pas
ouvrir la requête en cours.

## Ce qui vous reste à faire

- **Le client TypeScript engendré ne sait pas présenter une clé.** `rbs generate client`
  marque une opération comme protégée dès que sa `security` est non vide, mais il n'émet
  qu'un porteur `Bearer`. Une intégration machine écrite contre ce client a donc encore
  besoin d'un jeton, ou d'un en-tête posé à la main.
- **Choisir qui peut tirer une clé.** Les quatre routes n'exigent aucun rôle : chacun
  administre les siennes, comme chacun administre ses sessions. Si votre projet veut réserver
  l'émission aux administrateurs, ajoutez `identite.require_role(Role::Admin)?` à
  `controller::create`.

## Les tests

Onze tests accompagnent le fragment, sous `src/modules/api_keys/tests/`. Ils joignent la base
que décrit votre `.env`, et portent donc `#[ignore]` : `cargo test -- --ignored` les lance
une fois `rbs migrate up` appliquée.

Ce sont eux qui prouvent que le fragment fonctionne — le plafond recalculé quand un porteur
est rétrogradé, une clé inconnue, révoquée ou périmée rendant la même chose, la trace écrite
une seule fois sur des appels rapprochés, et une clé qui ouvre une route gardée par
`Identity`.
