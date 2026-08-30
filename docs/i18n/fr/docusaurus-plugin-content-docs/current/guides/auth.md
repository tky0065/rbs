---
sidebar_position: 7
title: Authentification
---

# Authentification

`rbs add auth` installe une authentification qui fonctionne dans un projet existant : huit
fichiers sous `src/auth/`, une migration, et cinq routes montées sur le routeur. Ce qu'elle
dépose est du code ordinaire dans votre arborescence — une entité, un service, un
controller, une garde — et il est fait pour être lu et modifié.

Tous les extraits de cette page sont tirés de
[`examples/blog-auth`](https://github.com/tky0065/rbs/tree/main/examples/blog-auth), un
projet généré par le CLI et compilé en CI. Rien ici n'est écrit à la main pour la
documentation.

## Ce qui s'installe

```text
$ rbs add auth
auth : authentification JWT : Argon2, jetons d'accès et de rafraîchissement, rôles

plan pour /private/tmp/rbs-demo/blog

  + src/auth/mod.rs                                        créé
  + src/auth/model.rs                                      créé
  + src/auth/dto.rs                                        créé
  + src/auth/repository.rs                                 créé
  + src/auth/service.rs                                    créé
  + src/auth/controller.rs                                 créé
  + src/auth/guard.rs                                      créé
  + src/auth/tests.rs                                      créé
  + migration/src/m20260830_111428_create_auth_tables.rs   créé
  ~ migration/src/lib.rs                                   modifié
  ~ src/lib.rs                                             modifié
  ~ src/router.rs                                          modifié
  ~ src/openapi.rs                                         modifié
  ~ Cargo.toml                                             modifié
  ~ config/default.toml                                    modifié
  ~ .env.example                                           modifié
  ~ .env                                                   modifié
  ~ AGENTS.md                                              modifié

  18 fichiers à écrire
✓ auth installée — 9 fichiers

  rbs migrate up
```

Cinq routes viennent avec :

| Route | Ce qu'elle fait |
|---|---|
| `POST /auth/register` | Crée un compte. 201 avec le profil, 409 si l'adresse est prise. |
| `POST /auth/login` | Échange les identifiants contre une paire accès/rafraîchissement. |
| `POST /auth/refresh` | Fait tourner la paire. Le jeton de rafraîchissement présenté est consommé. |
| `POST /auth/logout` | Révoque une session. 204. |
| `GET /auth/me` | Le profil de l'appelant. La seule route que la feature protège. |

La migration crée `users` et `refresh_tokens`, avec une contrainte d'unicité sur l'adresse
courriel. `rbs migrate down` les remporte toutes deux : les tables arrivent et repartent
avec la feature.

## Le secret, et où il vit

`add auth` tire `RBS_AUTH__SECRET` à l'installation et l'écrit dans votre `.env`, qui est
gitignoré : chaque projet signe ses jetons avec une valeur que personne d'autre ne détient,
et il n'y a rien à recopier avant le premier lancement. Ce qui atterrit dans
`.env.example`, versionné, est un placeholder — le fichier documente la variable sans en
livrer une clé utilisable :

```bash file=examples/blog-auth/.env.example
```

Un déploiement fournit sa propre valeur par l'environnement plutôt que par un fichier.
`Config::load` refuse un secret de moins de 32 octets plutôt que de signer des jetons avec
une clé faible, et l'échec a lieu au démarrage plutôt qu'à la première connexion.

Si vous ne savez pas ce que porte votre `.env`, demandez :

```bash
rbs doctor
```

Il signale le secret absent, trop court, ou portant encore la valeur d'exemple publiée —
ce dernier cas est celui d'un `.env` recopié à la main depuis `.env.example`, et un projet
qui signe ses jetons avec une valeur commitée dans Git est plus mal loti qu'un projet qui
ne démarre pas. Voir [`rbs doctor`](../cli/doctor.md).

Les durées de vie vivent dans la configuration, où elles se lisent et se changent sans
toucher au code :

```toml file=examples/blog-auth/config/default.toml
```

`access_ttl_secs` fait quinze minutes, `refresh_ttl_secs` trente jours. La section `[auth]`
est ajoutée par `rbs add auth` ; tout ce qui la précède était déjà là.

## Le cycle des jetons

Deux jetons, deux métiers différents.

Le **jeton d'accès** est un JWT signé (HS256). Il porte l'identifiant du compte et son
rôle, il n'est stocké nulle part, et il se vérifie par sa seule signature — ce qui le rend
peu coûteux. Il est de courte durée parce qu'il ne peut pas être révoqué.

Le **jeton de rafraîchissement** est fait de 256 bits tirés au hasard, opaque, sans
structure à lire. Il est stocké dans `refresh_tokens` sous forme d'empreinte SHA-256,
jamais en clair : un vol de cette table ne remet rien d'utilisable à un attaquant. Il n'est
délibérément pas haché par Argon2 — un jeton aléatoire n'offre rien à une recherche
exhaustive, et un KDF lent à chaque rafraîchissement ne s'achèterait rien.

Rafraîchir fait **tourner** la paire : le jeton présenté est marqué consommé par l'`UPDATE`
conditionnel qui le lit, si bien que le rejouer une seconde fois vaut 401. Se déconnecter
le consomme de la même façon, sans en réémettre — c'est pourquoi les deux opérations
partagent leur appel au repository.

Les mots de passe sont hachés par Argon2id, avec un sel tiré à chaque appel. Ni le hash ni
le mot de passe n'apparaissent dans une réponse ou dans les logs.

La connexion répond **la même 401** que l'adresse soit inconnue ou le mot de passe erroné,
et elle hache une valeur de comparaison même pour une adresse inconnue. Sauter cette
comparaison répondrait aux adresses inconnues en deux millisecondes et aux autres en deux
cent quarante — un oracle d'énumération mesurable de l'extérieur.

## Protéger une route

La feature livre une garde, non un middleware. C'est un trait d'extension sur `Identity`,
l'extracteur qui change un jeton porteur en appelant :

```rust file=examples/blog-auth/src/auth/guard.rs region=require_role
```

Un trait plutôt qu'un layer, parce que `from_fn_with_state` n'accepte pas de paramètre
supplémentaire : un layer par rôle figerait l'enum `Role` que la migration a justement
laissée ouverte.

L'appeler tient en une ligne, en tête d'un handler :

```rust file=examples/blog-auth/src/posts/controller.rs region=create
```

Deux choses méritent l'attention. `Identity` s'exécute **avant** le corps du handler : une
requête sans aucun jeton reçoit 401 sans que `require_role` soit jamais atteinte — on dit à
l'appelant de s'identifier, non qu'il manque de droits. Et c'est la ligne
`security(("bearer" = []))` qui pose le cadenas sur cette opération dans
`/api-docs/openapi.json` ; une route laissée ouverte ne doit pas la porter.

## Les rôles

`Role` est une enum Rust stockée en chaîne :

- un rôle de plus s'ajoute à l'enum et ne demande aucune migration ;
- un jeton signé par une version antérieure du projet, portant un rôle que l'enum ne
  connaît plus, n'ouvre rien et ne fait pas tomber le serveur.

**Aucune route ne donne un rôle.** L'inscription rend toujours un `user`, par défaut de la
table, et la promotion passe par la base. C'est délibéré : une route HTTP qui distribue
`admin` est une route que quelqu'un finira par atteindre. Les tests de l'exemple
promeuvent un compte exactement ainsi, et se connectent seulement après — un jeton émis
avant la promotion porterait l'ancien rôle :

```rust file=examples/blog-auth/src/posts/tests.rs region=jeton_admin
```

## Tester une route protégée

Les tests présentent un jeton plutôt que de construire une requête signée de zéro. Signer
une requête déjà construite garde les deux formes sous les yeux — ce qui n'est *pas* signé
dans le fichier est ce que l'API laisse ouvert :

```rust file=examples/blog-auth/src/posts/tests.rs region=signee
```

Trois tests suffisent ensuite à tenir le contrat, et il ne faut pas laisser les deux
premiers se confondre :

```rust file=examples/blog-auth/src/posts/tests.rs region=refus
```

Le `src/auth/tests.rs` généré couvre les routes de la feature elle-même — l'inscription,
les 401 identiques, la rotation, la révocation. Ils passent par HTTP contre une vraie base,
comme tous les tests que rbs génère ; voir le [guide des tests](./testing.md).

## Ce qu'elle vous laisse

Tout ce qui est propre à votre domaine :

- **qui a le droit de quoi** — la garde compare un rôle à une route ; tout ce qui est plus
  fin, tel un propriétaire modifiant sa propre ressource, est à écrire dans le service ;
- **la politique de mot de passe** — le DTO valide une longueur minimale, rien de plus ;
- **vérification d'adresse, réinitialisation, fournisseurs tiers** — hors de cette feature ;
- **la rotation du secret** — changer `RBS_AUTH__SECRET` invalide tous les jetons d'accès
  en circulation, ce qui est une fonctionnalité le jour où vous en avez besoin, et une
  panne le jour où vous ne l'attendez pas.

Le code est dans votre arborescence, sans bandeau vous interdisant d'y toucher.
