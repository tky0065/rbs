---
sidebar_position: 7
title: Authentification
---

# Authentification

`rbs add auth` installe une authentification qui fonctionne dans un projet existant :
des fichiers sous `src/auth/`, trois gabarits de courriel, un seed, une migration, et les
routes [décrites plus bas](#ce-qui-sinstalle), montées sur le routeur. Ce qu'elle dépose est du code ordinaire dans votre
arborescence — une entité, un service, un controller, une garde — et il est fait pour être
lu et modifié.

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
  + src/auth/config.rs                                     créé
  + src/auth/model.rs                                      créé
  + src/auth/dto.rs                                        créé
  + src/auth/repository/mod.rs                             créé
  + src/auth/repository/user.rs                            créé
  + src/auth/repository/refresh_token.rs                   créé
  + src/auth/repository/one_time_token.rs                  créé
  + src/auth/service/mod.rs                                créé
  + src/auth/service/session.rs                            créé
  + src/auth/service/account.rs                            créé
  + src/auth/service/password.rs                           créé
  + src/auth/service/verification.rs                       créé
  + src/auth/controller/mod.rs                             créé
  + src/auth/controller/session.rs                         créé
  + src/auth/controller/account.rs                         créé
  + src/auth/controller/password.rs                        créé
  + src/auth/controller/verification.rs                    créé
  + templates/mail/reinitialisation.html                   créé
  + templates/mail/verification.html                       créé
  + templates/mail/inscription.html                        créé
  + src/auth/guard.rs                                      créé
  + src/seeds/admin.rs                                     créé
  + src/auth/tests/mod.rs                                  créé
  + src/auth/tests/account.rs                              créé
  + src/auth/tests/change.rs                               créé
  + src/auth/tests/guard.rs                                créé
  + src/auth/tests/http.rs                                 créé
  + src/auth/tests/login.rs                                créé
  + src/auth/tests/logout.rs                               créé
  + src/auth/tests/openapi.rs                              créé
  + src/auth/tests/refresh.rs                              créé
  + src/auth/tests/registration.rs                         créé
  + src/auth/tests/replay.rs                               créé
  + src/auth/tests/reset.rs                                créé
  + src/auth/tests/roles.rs                                créé
  + src/auth/tests/sessions.rs                             créé
  + src/auth/tests/tokens.rs                               créé
  + src/auth/tests/verification.rs                         créé
  + migration/src/m20260910_162209_create_auth_tables.rs   créé
  ~ migration/src/lib.rs                                   modifié
  ~ src/lib.rs                                             modifié
  ~ src/router.rs                                          modifié
  ~ src/openapi.rs                                         modifié
  ~ src/seeds/main.rs                                      modifié
  ~ src/state.rs                                           modifié
  ~ Cargo.toml                                             modifié
  ~ config/default.toml                                    modifié
  ~ config/development.toml                                modifié
  ~ .env.example                                           modifié
  ~ .env                                                   modifié
  ~ AGENTS.md                                              modifié

  40 à créer, 12 à modifier
✓ auth installée — 40 créés, 12 modifiés

  rbs migrate up
```

Quinze routes viennent avec, sur treize chemins — `/auth/sessions` porte à la fois la liste
et la révocation globale, `/auth/me` à la fois le profil et la seule écriture qu'il accepte.
Six ouvrent le cycle central :

| Route | Ce qu'elle fait |
|---|---|
| `POST /auth/register` | Crée un compte. Toujours 202, sans corps — adresse déjà prise comprise. |
| `POST /auth/login` | Échange les identifiants contre une paire accès/rafraîchissement. |
| `POST /auth/refresh` | Fait tourner la paire : le jeton présenté est marqué remplacé. |
| `POST /auth/logout` | Révoque une session. 204. |
| `GET /auth/me` | Le profil de l'appelant. |
| `PATCH /auth/me` | Change l'adresse de l'appelant, et rien d'autre. Toujours 202, sans corps. |

`register` rend le même 202 que l'adresse soit neuve ou porte déjà un compte, et hache le
mot de passe dans les deux cas — un 409, un profil rendu à la seule adresse neuve, ou une
réponse qui aurait sauté Argon2 diraient chacun à qui essaie plusieurs adresses lesquelles
sont inscrites. Une adresse neuve voit son compte écrit avant la réponse, et son lien de vérification part
dans une tâche détachée ; une adresse prise n'est pas touchée, et son titulaire reçoit un
courriel, `templates/mail/inscription.html`, qui le prévient de la tentative et le renvoie
vers `forgot-password`.

Une connexion avec le mot de passe tout juste soumis ne les distingue pas davantage. Un
compte ne se connecte qu'une fois son adresse vérifiée — `login_requires_verification`,
`true` par défaut dans `[auth]` — et reçoit sinon le 401 d'un mauvais mot de passe, après
le même Argon2 : l'adresse neuve n'est pas encore vérifiée, la prise ne porte pas ce mot de
passe, et les deux répondent pareil. Mettre la clé à `false` connecte un compte dès son
inscription, et rouvre cet écart — `register` suivi de `login` distingue alors les deux, en
deux requêtes, au rythme que la limite de débit autorise (`/auth/register` en accepte 10
par heure et par client). Un projet qui fait ce choix pose `VerifiedIdentity` sur les
routes qui ne doivent pas servir une adresse non prouvée.

Une septième, `GET /auth/registration`, dit si `POST /auth/register` est ouverte — la
section ci-dessous explique pourquoi un écran doit le demander. Une huitième,
`POST /auth/change-password`, laisse un appelant qui porte déjà un jeton changer son mot de
passe sans lien courriel — couverte juste en dessous. Les sept autres portent sur un mot de
passe oublié, une adresse non confirmée, ou les sessions de l'appelant, chacune dans sa
propre section plus bas sur cette page.

`PATCH /auth/me` prend une adresse et rien d'autre : le corps est l'`EmailRequest` que
lisent déjà `forgot-password` et `resend-verification`, si bien qu'un champ ajouté là
serait visible aux trois endroits à la fois. Elle écrit la nouvelle adresse, remet
`email_verified_at` à `NULL` — la preuve portait sur l'ancienne — et envoie un lien de
vérification neuf, dans la même tâche détachée qu'une inscription. La réponse est le même
202 que l'adresse ait été libre ou qu'elle porte déjà un compte, et dans le second cas rien
n'est écrit et son titulaire reçoit le courriel d'avertissement qu'une inscription lui
aurait envoyé : sans cela, un seul compte suffirait à savoir quelles adresses le service
connaît.

La migration crée `users`, `refresh_tokens` et `one_time_tokens`, avec une contrainte
d'unicité sur l'adresse courriel et un `email_verified_at` nullable sur `users`.
`rbs migrate down` les remporte toutes trois : les tables arrivent et repartent avec la
feature.

### Fermer les inscriptions

`registration_enabled`, dans `[auth]`, vaut `true` par défaut. Mise à `false`, elle fait
refuser `POST /auth/register` avant même la lecture de l'adresse — un 403 dont le document
de problème porte le code stable `registration_closed` — et non pas seulement masquer un
bouton : la route est ouverte à qui poste, et un interrupteur qui n'aurait atteint que les
navigateurs n'aurait rien fermé. `GET /auth/registration` rend alors `{"enabled": false}`,
qui est le seul moyen pour une application servie en fichiers statiques de connaître un
réglage que le serveur lit à son démarrage ; l'écran d'inscription du shell
d'administration le demande, et affiche le refus au lieu du formulaire.

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

Deux de ses clés sont dédoublées dans `config/development.toml`, que le profil
`development` pose par-dessus les défauts. `app_url` vaut `http://localhost:8080` par
défaut — le port sur lequel le binaire sert le client construit — et
`http://localhost:5173` sur un poste de travail, qui est celui de Vite.
`login_requires_verification` vaut `true` par défaut et `false` sur un poste de travail :
un compte inscrit par l'API n'est pas vérifié, le lien qui le prouve part vers le client et
non vers ce serveur, et l'écart que `false` rouvre est un écart qu'un poste de travail peut
porter quand la production ne le peut pas. Cet écart se resserre dès que le shell
d'administration est posé : il porte l'écran public qui poste sur `/auth/verify-email`, et
un compte inscrit par l'API peut alors prouver son adresse sur un poste de travail comme en
production.

## Le cycle des jetons

Deux jetons, deux métiers différents.

Le **jeton d'accès** est un JWT signé (HS256). Il porte l'identifiant du compte et son
rôle, et n'est stocké nulle part. Sa signature est vérifiée d'abord ; puis la ligne du
compte est relue — une requête par appel authentifié — et le jeton est refusé si le compte
a disparu, si toutes les sessions ont été fermées après son émission
(`users.sessions_revoked_at`), ou si le rôle qu'il porte n'est plus celui du compte.
Fermer une *seule* session n'atteint pas son jeton d'accès : rien ne relie les deux, et
le jeton vit jusqu'à `exp`. C'est pourquoi il reste de courte durée.

Le **jeton de rafraîchissement** est fait de 256 bits tirés au hasard, opaque, sans
structure à lire. Il est stocké dans `refresh_tokens` sous forme d'empreinte SHA-256,
jamais en clair : un vol de cette table ne remet rien d'utilisable à un attaquant. Il n'est
délibérément pas haché par Argon2 — un jeton aléatoire n'offre rien à une recherche
exhaustive, et un KDF lent à chaque rafraîchissement ne s'achèterait rien.

Rafraîchir fait **tourner** la paire : le jeton présenté est marqué remplacé
(`replaced_at`) par le même `UPDATE` conditionnel qui le lit, si bien que deux
rafraîchissements concurrents ne peuvent pas gagner tous les deux. Un jeton remplacé
présenté à nouveau a servi deux fois — un de ses deux détenteurs n'est pas le titulaire du
compte — et toutes les sessions du compte sont fermées. Se déconnecter, révoquer une
session, réinitialiser ou changer le mot de passe ferment un jeton à la place, dans une
colonne séparée (`revoked_at`) : un jeton fermé présenté à nouveau vaut 401 et rien de
plus, parce qu'un client qui rejoue une déconnexion n'est pas un jeton volé qui circule.
Réinitialiser ou changer le mot de passe et `DELETE /auth/sessions` estampillent aussi
`users.sessions_revoked_at` : tout jeton d'accès émis avant cette seconde meurt avec les
sessions.

Un client qui resoumet le même jeton de rafraîchissement — un retry après un délai, un
double envoi — est indiscernable d'un rejeu et paie le même prix : toutes les sessions se
ferment. C'est le prix de la détection du rejeu : ne rejouer un rafraîchissement que si
aucune réponse n'a été reçue.

Les mots de passe sont hachés par Argon2id, avec un sel tiré à chaque appel. Ni le hash ni
le mot de passe n'apparaissent dans une réponse ou dans les logs.

Les adresses sont débarrassées de leurs blancs et passées en minuscules avant d'atteindre
la table : `Alice@Exemple.test` et `alice@exemple.test` sont un seul compte, à
l'inscription comme à la connexion, et `/auth/me` montre la forme en minuscules. Le DTO
valide toujours ce que le client a envoyé.

La connexion répond **la même 401** que l'adresse soit inconnue ou le mot de passe erroné,
et elle hache une valeur de comparaison même pour une adresse inconnue. Sauter cette
comparaison répondrait aux adresses inconnues en deux millisecondes et aux autres en deux
cent quarante — un oracle d'énumération mesurable de l'extérieur.

## Changer son propre mot de passe

Une route, protégée, pour un appelant qui connaît déjà son mot de passe actuel et en veut
un neuf sans passer par un courriel :

| Route | Ce qu'elle fait |
|---|---|
| `POST /auth/change-password` | Vérifie le mot de passe actuel, pose le nouveau, et révoque toutes les sessions du compte — la sienne comprise. 200 avec une paire neuve. |

La révocation est inconditionnelle : rien dans la requête ne relie le jeton d'accès
présenté à la ligne de session qui l'a émis, épargner « cette » session demanderait un
identifiant que la route n'a pas — elles tombent donc toutes, et la réponse rend une paire
pour se reconnecter aussitôt. Le service revérifie le mot de passe avant d'écrire quoi que
ce soit — un `current_password` erroné rend **403, et non 401** : l'appelant *est*
identifié, son Bearer *est* bon, et un 401 pousserait un client vers un rafraîchissement
qui ne réglerait rien.

## Voir et fermer ses sessions

Trois routes de plus, protégées celles-ci — elles portent sur le compte de l'appelant, et
jamais sur un autre :

| Route | Ce qu'elle fait |
|---|---|
| `GET /auth/sessions` | Les sessions encore ouvertes du compte, la plus récente d'abord. |
| `DELETE /auth/sessions/{id}` | Ferme une session nommée. 204, ou 404 si l'identifiant ne désigne aucune session de l'appelant. |
| `DELETE /auth/sessions` | Ferme toutes les sessions du compte, la sienne comprise. 204. |

`GET /auth/sessions` rend `SessionResponse`, qui ne porte jamais `token_hash` — la même
règle qui tient `UserResponse` à l'écart du hash du mot de passe : la vue d'une session n'a
aucune raison de porter de quoi la présenter.

`DELETE /auth/sessions/{id}` rend **404, et non 403**, quand l'identifiant ne désigne
aucune session de l'appelant :

```rust file=examples/blog-auth/src/auth/controller/session.rs region=revoke_session
```

Un identifiant qui n'est pas le vôtre ne désigne, de votre côté, aucune session — un 403
confirmerait qu'elle existe chez quelqu'un d'autre. La fermeture porte le propriétaire dans
la condition de l'`UPDATE` du repository, et non dans une lecture qui le précède : lire la
ligne puis comparer laisserait la révocation de la session d'autrui à portée d'une course —
le même motif que suit déjà la rotation d'un jeton de rafraîchissement.

`DELETE /auth/sessions` ferme tout, y compris la session qui porte le jeton présenté à la
requête : rien ne la distingue des autres, comme pour `change-password`.

## Oublier et réinitialiser un mot de passe

Deux routes de plus ferment la boucle que la connexion ouvre, publiques toutes deux — sans
jeton porteur :

| Route | Ce qu'elle fait |
|---|---|
| `POST /auth/forgot-password` | Envoie un lien de réinitialisation si l'adresse est inscrite. Toujours 202. |
| `POST /auth/reset-password` | Consomme le jeton de ce lien et pose un nouveau mot de passe. 204, toutes les sessions du compte révoquées, et l'adresse vérifiée. |

`forgot-password` rend 202 que l'adresse porte un compte ou non, et le corps ne diffère pas
davantage — le même risque d'énumération que le hash témoin de la connexion écarte de
l'autre côté :

```rust file=examples/blog-auth/src/auth/controller/password.rs region=forgot_password
```

Le handler passe l'adresse à `service::password::send_reset_link`, qui lit le compte et
rien de plus : l'ouverture du jeton et le rendu du courriel partent dans une tâche
détachée, dont le courriel passe par `notify` — le seul endroit de la feature qui en
envoie un :

```rust file=examples/blog-auth/src/auth/service/mod.rs region=notify
```

Seule la lecture est attendue. Fermer le jeton précédent, écrire le neuf, rendre et
envoyer le courriel prennent un temps qu'une adresse inconnue ne dépense jamais : attendus,
ils feraient dire au temps de réponse ce que le code de statut refuse de dire. La même
tâche purge d'abord les jetons échus de tous les comptes — c'est à l'émission que
`one_time_tokens` grossit, et y purger la borne sans tâche planifiée. Ce qui y échoue — la
base, un gabarit absent, une adresse que `lettre` ne sait pas analyser — est journalisé
avec l'identifiant du compte et n'atteint jamais la réponse, déjà partie.

Une seconde demande ferme la première : un seul jeton de réinitialisation reste vivant par
compte, si bien qu'un lien parti dans une boîte qu'on ne contrôle plus cesse de valoir dès
qu'on en redemande un. `reset-password` rend la même 401 pour un jeton inconnu, périmé, ou
déjà consommé — les distinguer renseignerait sur l'état d'une demande en cours à qui n'en
détient aucun des trois :

```rust file=examples/blog-auth/src/auth/controller/password.rs region=reset_password
```

Une réinitialisation vérifie aussi l'adresse, si elle ne l'était pas encore, dans la même
transaction que le nouveau mot de passe : le jeton est arrivé dans la boîte exactement
comme un lien de vérification. Sans cela, un compte qui n'a jamais cliqué son lien de
vérification recevrait le 401 d'un mauvais mot de passe sous `login_requires_verification`,
passerait par `forgot-password` comme tout le monde, et retrouverait le même 401 avec son
nouveau mot de passe. Une adresse déjà vérifiée garde sa date.

`auth` tire `mail` pour cela — cette route et celle qui vérifie une adresse ont toutes deux
besoin d'un endroit où envoyer un lien, et la dépendance est déclarée plutôt que laissée
facultative. Sa durée et sa destination viennent de la section `[auth]` montrée plus haut :
`reset_ttl_secs` fixe la durée de vie du lien de réinitialisation, `verification_ttl_secs`
fait de même pour l'autre parcours, et `app_url` est la racine que `FlowConfig::link`
préfixe au chemin — l'adresse de votre client, pas de ce serveur. Le jeton voyage dans le
fragment du lien, `…/reset-password#token=…` : un navigateur n'envoie jamais le fragment
à un serveur, si bien que le jeton reste hors des journaux d'accès et des en-têtes
`Referer` — votre client le lit dans `location.hash` avant de le poster.

Les deux routes sont limitées à trois requêtes par heure et par client, aux côtés de
`/auth/login` : elles envoient un courriel à une adresse que l'appelant choisit, et sans
cette borne le projet devient un relais de harcèlement dont le coût retombe sur le
titulaire de l'adresse.

## Confirmer une adresse

`register` ouvre un jeton de vérification pour chaque compte qu'elle crée — dans une
tâche détachée, une fois le compte écrit, si bien qu'un échec à cet instant ne peut pas
défaire l'inscription : l'appelant a toujours un compte, il ne lui manque que
`resend-verification` pour rattraper le courriel. Deux routes ferment cette boucle,
publiques toutes deux — sans jeton porteur :

| Route | Ce qu'elle fait |
|---|---|
| `POST /auth/resend-verification` | Envoie un lien de vérification neuf. Toujours 202, exactement comme `forgot-password`. |
| `POST /auth/verify-email` | Consomme le jeton de ce lien et date `email_verified_at`. 204. |

`resend-verification` rend le même 202 que l'adresse porte un compte ou non, et une
adresse déjà vérifiée ne reçoit rien — une seconde preuve ne ferait que rajeunir sa date,
et c'est cette date que lira une revérification des plus anciennes adresses. Le handler
n'attend que la lecture du compte : l'écriture du jeton et le courriel partent dans la même
sorte de tâche détachée que ceux de `forgot-password`, attendre l'un ou l'autre laissant
le temps de réponse dire ce que le code de statut refuse de dire :

```rust file=examples/blog-auth/src/auth/controller/verification.rs region=resend_verification
```

`verify-email` rend le même 401 pour quatre causes distinctes : un jeton inconnu, périmé,
déjà consommé, ou — ce quatrième cas est ce qui fait d'une table de jetons partagée une
économie plutôt qu'une faille — émis pour l'autre parcours. La recherche filtre sur l'usage
autant que sur l'empreinte, si bien qu'un lien de réinitialisation ne se consomme jamais
ici — seule `reset-password` le consomme, et vérifie l'adresse avec le nouveau mot de
passe :

```rust file=examples/blog-auth/src/auth/controller/verification.rs region=verify_email
```

L'inscription et le renvoi partagent une seule fonction de service plutôt que deux,
`verification::send_link_detached`, parce que les deux ont le compte en main au moment
d'émettre. `send_link` lit le compte derrière une adresse et écarte celui qui est déjà
vérifié ; `register` appelle `send_link_detached` directement, une fois le compte écrit, si
bien que le compte tient quoi qu'il advienne du courriel :

```rust file=examples/blog-auth/src/auth/service/verification.rs region=send_link
```

**`login` exige une adresse vérifiée, sauf si vous coupez la clé.** Sous le défaut
`login_requires_verification = true`, un compte qui ne clique jamais son lien — ni ne
réinitialise son mot de passe — reçoit le 401 d'un mauvais mot de passe ; un client qui
vient d'appeler `register` doit dire « vérifiez votre boîte », puisque ce 401 ne le dira
pas. La clé à `false`, le compte se connecte aussitôt, et celles de vos routes qui refusent
un appelant non vérifié relèvent d'une garde, couverte [plus bas](#exiger-une-adresse-vérifiée).

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

### Exiger une adresse vérifiée

Un second extracteur, `VerifiedIdentity`, enveloppe `Identity` plutôt que de se poser à
côté : un handler qui le prend à la place reçoit la même 401 pour un jeton absent ou
invalide, puis une 403 par-dessus quand `email_verified_at` est vide. La date qu'elle
examine vient du compte qu'`Identity` vient de lire pour accepter le jeton : `accept_in`
laisse cette date dans la requête — la date seule, et non la ligne avec son hash de mot de
passe —, et la garde ne relit pas la même ligne.

L'état vient de la base et non du jeton, délibérément : le jeton d'accès porte `sub` et
`role` pour ses quinze minutes entières, et lire la vérification dessus continuerait de
répondre faux pour ce qu'il reste de cette fenêtre après que `verify-email` l'a levée.
Aucune route du fragment ne prend `VerifiedIdentity` — sous le défaut
`login_requires_verification = true`, seul un compte vérifié se connecte, et la garde sert
le projet qui a mis la clé à `false` — si bien qu'elle démarre en code mort, derrière
`#[allow(dead_code)]`, dans `src/auth/guard.rs`, de la même façon que `require_role` le
serait si aucune route engendrée ne l'appelait. Prendre `VerifiedIdentity` au lieu
d'`Identity` sur la signature d'un handler est ce qui met une route derrière elle.

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

Le `tests/` engendré suit. Il signe le jeton qu'il présente par `rbs_core::jwt::sign` —
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
`admin` est une route que quelqu'un finira par atteindre. Le `src/auth/tests/roles.rs`
engendré promeut un compte exactement ainsi, et se connecte seulement après — un jeton émis
avant la promotion porterait l'ancien rôle :

```rust file=examples/blog-auth/src/auth/tests/roles.rs region=jeton_admin
```

**Le premier administrateur vient d'un seed.** `auth` dépose `src/seeds/admin.rs` et le
déclare dans le binaire des seeds du projet : `rbs seed` écrit un compte portant
`Role::Admin`, son adresse datée comme vérifiée — sans cette date,
`login_requires_verification` le refuserait. Ses identifiants sont `ADMIN_EMAIL` et
`ADMIN_PASSWORD` : `rbs add auth` pose les deux dans votre `.env`, que git ignore —
l'adresse déduite du nom du projet, le mot de passe tiré à l'installation — et laisse des
repères dans le `.env.example` versionné. Le seed n'écrit rien si le compte existe déjà,
rien si l'une des deux variables manque ou est vide, et rien du tout sous
`RBS_ENV=production` — ce dernier refus vit dans le seed et non dans la commande, parce que
`cargo run --bin seed` ne passe jamais par la garde que porte
[`rbs seed`](../cli/seed.md).

## Tester une route protégée

Les tests d'une feature créent un compte. `Identity` vérifie la signature, puis relit la
ligne du compte : un jeton signé pour un `sub` inventé est refusé. Le `tests/mod.rs` engendré
inscrit donc un compte au rôle que ses routes exigent, à la première montée de
`application()`, signe un jeton pour lui, et chaque requête le porte :

```rust file=examples/blog-auth/src/posts/tests/mod.rs region=jeton
```

Trois tests tiennent ensuite les refus, et il ne faut pas les laisser se confondre. Deux
sont engendrés — une écriture et une lecture, anonymes toutes deux, auxquelles l'extracteur
répond 401 avant que le handler s'exécute. Le troisième appartient à l'exemple : un appelant
bien identifié mais d'un rôle trop court, à qui la garde répond 403 dans le handler.

```rust file=examples/blog-auth/src/posts/tests/access.rs region=refus
```

Les routes de la feature elle-même sont couvertes de la même façon, réparties par sujet
sous `src/auth/tests/` — `registration.rs`, `login.rs`, `refresh.rs`, `replay.rs`,
`logout.rs`, `sessions.rs`, `roles.rs`, `tokens.rs`, `change.rs`, `account.rs`, `reset.rs`,
`verification.rs`, `guard.rs` et `openapi.rs`, autour du harnais partagé de `mod.rs` et des
aides de requête de `http.rs` — l'inscription et son interrupteur, les 401 identiques, la
rotation, le rejeu, la révocation, le changement d'adresse, les parcours de mot de passe et
de vérification ci-dessus, la garde d'adresse vérifiée et le document OpenAPI. Tous passent par HTTP contre une vraie base, et tous
portent donc `#[ignore]` : le `cargo test` d'un projet neuf réussit sans serveur démarré,
et `cargo test -- --ignored` les lance contre la base que nomme votre `.env`, migrations
appliquées. Un test échappe à la règle et reste un `#[test]` ordinaire, dans `mod.rs` : il
vérifie qu'un lien porte son jeton dans son fragment, ce qui tient à la façon dont l'URL
est construite et ne demande rien de démarré. Voir le [guide des tests](./testing.md).

## Ce qu'elle vous laisse

Tout ce qui est propre à votre domaine :

- **qui a le droit de quoi** — la garde pèse un seuil de rôle contre une route ; tout ce
  qui est plus fin, tel un propriétaire modifiant sa propre ressource, est à écrire dans le
  service ;
- **la politique de mot de passe** — le DTO valide une longueur de 12 à 128 caractères,
  rien de plus ;
- **fournisseurs tiers** — hors de cette feature ;
- **la rotation du secret** — changer `RBS_AUTH__SECRET` invalide tous les jetons d'accès
  en circulation, ce qui est une fonctionnalité le jour où vous en avez besoin, et une
  panne le jour où vous ne l'attendez pas.

Le code est dans votre arborescence, sans bandeau vous interdisant d'y toucher.
