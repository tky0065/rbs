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

## Oublier et réinitialiser un mot de passe

Deux routes de plus ferment la boucle que la connexion ouvre, publiques toutes deux — sans
jeton porteur :

| Route | Ce qu'elle fait |
|---|---|
| `POST /auth/forgot-password` | Envoie un lien de réinitialisation si l'adresse est inscrite. Toujours 202. |
| `POST /auth/reset-password` | Consomme le jeton de ce lien et pose un nouveau mot de passe. 204, toutes les sessions du compte révoquées. |

`forgot-password` rend 202 que l'adresse porte un compte ou non, et le corps ne diffère pas
davantage — le même risque d'énumération que le hash témoin de la connexion écarte de
l'autre côté :

```rust file=examples/blog-auth/src/auth/controller/password.rs region=forgot_password
```

L'envoi du courriel lui-même passe par `mail().send_template_detached`, qui rend le
gabarit tout de suite — un gabarit absent fait échouer la requête — puis confie l'envoi à
une tâche détachée plutôt que de l'attendre : attendre le SMTP ferait dire au temps de
réponse ce que le code de statut refuse de dire.

Une seconde demande ferme la première : un seul jeton de réinitialisation reste vivant par
compte, si bien qu'un lien parti dans une boîte qu'on ne contrôle plus cesse de valoir dès
qu'on en redemande un. `reset-password` rend la même 401 pour un jeton inconnu, périmé, ou
déjà consommé — les distinguer renseignerait sur l'état d'une demande en cours à qui n'en
détient aucun des trois :

```rust file=examples/blog-auth/src/auth/controller/password.rs region=reset_password
```

`auth` tire `mail` pour cela — cette route et celle qui vérifie une adresse ont toutes deux
besoin d'un endroit où envoyer un lien, et la dépendance est déclarée plutôt que laissée
facultative. Sa durée et sa destination viennent de la section `[auth]` montrée plus haut :
`reset_ttl_secs` fixe la durée de vie du lien de réinitialisation, `verification_ttl_secs`
fait de même pour l'autre parcours, et `app_url` est la racine que `FlowConfig::link`
préfixe au chemin — l'adresse de votre client, pas de ce serveur.

Les deux routes sont limitées à trois requêtes par heure et par client, aux côtés de
`/auth/login` : elles envoient un courriel à une adresse que l'appelant choisit, et sans
cette borne le projet devient un relais de harcèlement dont le coût retombe sur le
titulaire de l'adresse.

## Protéger une route

La feature livre une garde, non un middleware. C'est un trait d'extension sur `Identity`,
l'extracteur qui change un jeton porteur en appelant :

```rust file=examples/blog-auth/src/auth/guard.rs region=require_role
```

Un trait plutôt qu'un layer, parce que `from_fn_with_state` n'accepte pas de paramètre
supplémentaire : un layer par rôle figerait l'enum `Role` que la migration a justement
laissée ouverte.

L'appeler tient en une ligne, en tête d'un handler — ici le `create` de `blog-auth`,
engendré avec `--role admin`, d'où le `Role::Admin` plutôt que le `Role::User` par défaut :

```rust file=examples/blog-auth/src/posts/controller.rs region=create
```

Deux choses méritent l'attention. `Identity` s'exécute **avant** le corps du handler : une
requête sans aucun jeton reçoit 401 sans que `require_role` soit jamais atteinte — on dit à
l'appelant de s'identifier, non qu'il manque de droits. Et c'est la ligne
`security(("bearer" = []))` qui pose le cadenas sur cette opération dans
`/api-docs/openapi.json` ; une route laissée ouverte ne doit pas la porter.

### Fermées par défaut à la génération

Sur un projet portant `auth`, [`rbs generate crud`](../cli/generate.md) écrit cette ligne
sur chacune des routes qu'il monte. Aucun drapeau ne la demande :

```bash
rbs generate crud articles --fields title:string
```

Les six routes du CRUD — `list`, `filter`, `create`, `find`, `update`, `delete` — prennent
chacune une `identite: Identity`, ouvrent leur corps par
`identite.require_role(Role::User)?`, portent `security(("bearer" = []))`, et déclarent les
deux refus dans leur `#[utoipa::path]` : la 401 que rend l'extracteur, la 403 que rend
`require_role`. Sous `--with-upload`, les `PUT`, `GET` et `HEAD` de la route de contenu les
rejoignent — neuf routes, toutes fermées. Voir
[le guide du stockage](./storage.md#les-routes-de-contenu-engendrées).

`--role admin` ne ferme rien de plus. Il **relève le seuil des écritures** :

```bash
rbs generate crud articles --fields title:string --role admin
```

`create`, `update` et `delete` — et le `PUT` de la route de contenu sous `--with-upload` —
exigent alors `Role::Admin`, tandis que `list`, `filter`, `find` et les `GET` et `HEAD` de
la route de contenu gardent le `Role::User` par défaut. Deux étages d'appelants, et non une
moitié ouverte et une moitié fermée. Le drapeau refuse, avant toute écriture, sur un projet
sans `auth` — le contrôleur importerait un module qui n'existe pas — et sur un rôle que
`src/auth/model.rs` ne déclare pas.

**Ouvrir une route au public est une édition du fichier engendré**, et son en-tête le dit :
sur le handler à ouvrir, retirez le paramètre `identite`, l'appel à `require_role`, l'entrée
`security` et les réponses 401 et 403 de son annotation. Quatre suppressions dans un fichier
de votre propre arborescence. Rien dans le CLI ne les fait pour vous, et rien ne les remet.

Le `tests.rs` engendré suit. Il signe le jeton qu'il présente par `rbs_core::jwt::sign` —
`Identity` ne vérifie qu'une signature, il n'y a donc aucun compte à créer — et exerce avec
lui le cycle d'écriture complet. Deux de ses tests ne présentent aucun jeton, une écriture
et une lecture, et tiennent la 401 que l'une et l'autre reçoivent.

Un CRUD engendré *avant* l'installation d'`auth` reste ouvert, car le CLI ne réécrit aucun
fichier qu'il a déjà écrit. `rbs add auth` nomme donc ces features en fin de sortie, une
fois la feature installée — un `--dry-run` n'écrit rien et n'en affiche rien —, et en fermer
une revient à remettre à la main les quatre mêmes éléments.

[`rbs doctor`](../cli/doctor.md) signale toujours en orange, sur un projet portant `auth`,
toute feature dont `create`, `update` ou `delete` n'appelle aucune garde — mais sur un
projet engendré à partir de la 1.3.0, il n'a plus rien à dire, puisque ce qu'écrit
`generate crud` l'appelle déjà. Ce qu'il trouve désormais, c'est un CRUD engendré avant la
venue d'`auth`, un CRUD engendré par une version antérieure, ou un CRUD dont les écritures
ont été rouvertes à la main — la garde se cherche dans le corps de chaque handler
d'écriture, et non n'importe où dans le fichier : le bandeau qui nomme `require_role` ne
répond donc pas pour elle. Un avertissement et non un échec : un catalogue public est un
choix légitime, et la commande sort toujours en 0.

## Les rôles

`Role` est une enum Rust stockée en chaîne :

- un rôle de plus s'ajoute à l'enum et ne demande aucune migration ;
- un jeton signé par une version antérieure du projet, portant un rôle que l'enum ne
  connaît plus, n'ouvre rien et ne fait pas tomber le serveur.

`require_role` compare un **seuil**, non une égalité : elle laisse passer dès que le rôle
de l'appelant est supérieur ou égal à celui exigé, si bien qu'un `Admin` satisfait un
`require_role(Role::User)`. Sans cela, un CRUD engendré nommant `Role::User` sur ses
lectures en fermerait la porte à ses propres administrateurs. **La hiérarchie, c'est
l'ordre dans lequel l'enum déclare ses variantes** — `User`, puis `Admin`, avec `Ord`
dérivé — si bien qu'un rôle inséré entre deux autres déplace d'un coup le seuil de toutes
les gardes du projet. Un rôle plus étendu que le dernier s'ajoute donc en fin d'énumération.

Un projet engendré avant la 1.3.0 porte la garde antérieure, qui comparait une égalité, et
la conserve : `rbs` ne réécrit aucun fichier qu'il a déjà écrit, si bien que laquelle des
deux sémantiques votre projet porte dépend de la version qui l'a engendré. La note de la
1.3.0 — affichée par [`rbs upgrade`](../cli/upgrade.md), et versionnée sous
`crates/rbs-cli/notes/1.3.0.md` — porte les lignes exactes à remplacer dans
`src/auth/guard.rs` et `src/auth/model.rs` pour y faire passer un projet existant.

**Aucune route ne donne un rôle.** L'inscription rend toujours un `user`, par défaut de la
table, et la promotion passe par la base. C'est délibéré : une route HTTP qui distribue
`admin` est une route que quelqu'un finira par atteindre. Le `src/auth/tests.rs` engendré
promeut un compte exactement ainsi, et se connecte seulement après — un jeton émis avant la
promotion porterait l'ancien rôle :

```rust file=examples/blog-auth/src/auth/tests.rs region=jeton_admin
```

## Tester une route protégée

Les tests d'une feature n'ont besoin d'aucun compte. `Identity` ne vérifie qu'une
signature : le `tests.rs` engendré signe le jeton qu'il présente, et le fichier n'a ni
ligne à créer ni ligne à nettoyer :

```rust file=examples/blog-auth/src/posts/tests.rs region=jeton
```

Trois tests tiennent ensuite les refus, et il ne faut pas les laisser se confondre. Deux
sont engendrés — une écriture et une lecture, anonymes toutes deux, auxquelles l'extracteur
répond 401 avant que le handler s'exécute. Le troisième appartient à l'exemple : un appelant
bien identifié mais d'un rôle trop court, à qui la garde répond 403 dans le handler.

```rust file=examples/blog-auth/src/posts/tests.rs region=refus
```

Ce même `src/auth/tests.rs` couvre les routes de la feature elle-même — l'inscription,
les 401 identiques, la rotation, la révocation. Tous passent par HTTP contre une vraie
base, et tous portent donc `#[ignore]` : le `cargo test` d'un projet neuf réussit sans
serveur démarré, et `cargo test -- --ignored` les lance contre la base que nomme votre
`.env`, migrations appliquées. Voir le [guide des tests](./testing.md).

## Ce qu'elle vous laisse

Tout ce qui est propre à votre domaine :

- **qui a le droit de quoi** — la garde pèse un seuil de rôle contre une route ; tout ce
  qui est plus fin, tel un propriétaire modifiant sa propre ressource, est à écrire dans le
  service ;
- **la politique de mot de passe** — le DTO valide une longueur de 12 à 128 caractères,
  rien de plus ;
- **vérification d'adresse, fournisseurs tiers** — hors de cette feature ;
- **la rotation du secret** — changer `RBS_AUTH__SECRET` invalide tous les jetons d'accès
  en circulation, ce qui est une fonctionnalité le jour où vous en avez besoin, et une
  panne le jour où vous ne l'attendez pas.

Le code est dans votre arborescence, sans bandeau vous interdisant d'y toucher.
