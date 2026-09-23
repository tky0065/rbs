---
sidebar_position: 11.9
title: Frontend
---

# Frontend

Deux fragments posent une application Vue 3 dans un projet existant. `rbs add frontend`
installe le **socle** : l'application, son routeur, un thème Tailwind v4, quatorze
composants shadcn-vue vendorisés, et un accueil public. `rbs add frontend-admin` y ajoute le
**shell d'administration** : quatre écrans de compte et de santé derrière une garde de
route, et les quatre pages publiques qui y mènent — et, dès lors,
[`rbs generate crud`](../cli/generate.md) écrit les écrans d'administration de la table en
même temps que son entité.

Une seule application, deux régimes de route : une racine publique, et l'espace
d'administration dans un morceau paresseux derrière une garde de route. Un build, un service
statique, un repli.

Chaque extrait de cette page est lu dans
[`examples/admin-console`](https://github.com/tky0065/rbs/tree/main/examples/admin-console),
un projet engendré par le CLI, compilé en CI, et dont la CI installe, vérifie et construit
le frontend. Rien ici n'est écrit à la main pour la documentation.

## Ce que le socle installe

Cent six fichiers, dont quatre-vingt-deux sont les composants vendorisés. Les lignes de
fichiers du plan sont coupées ci-dessous ; il n'en reste que la tête et la queue.

{/* rbs:transcript cmd="rbs add frontend" setup="rbs new demo --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" extrait="oui" */}
```text
$ rbs add frontend
frontend : une application Vue 3 servie par le binaire, et une page d'amorçage tant qu'elle n'est pas construite

plan pour …/demo

  + src/modules/frontend/mod.rs                                             créé
  + src/modules/frontend/config.rs                                          créé
  + src/modules/frontend/amorcage.rs                                        créé
  + src/modules/frontend/feuille.rs                                         créé
  + src/modules/frontend/tests.rs                                           créé
  + frontend/package.json                                                   créé
  + frontend/vite.config.ts                                                 créé
  + frontend/src/main.ts                                                    créé
  + frontend/src/router/index.ts                                            créé
  + frontend/src/assets/main.css                                            créé
  + frontend/src/components/Bande.vue                                       créé
  + frontend/src/views/Accueil.vue                                          créé
  + src/modules/mod.rs                                                      créé
  ~ src/lib.rs                                                              modifié
  ~ src/router.rs                                                           modifié
  ~ .gitignore                                                              modifié
  ~ Makefile                                                                modifié
  ~ Cargo.toml                                                              modifié
  ~ config/default.toml                                                     modifié
  ~ AGENTS.md                                                               modifié

  106 à créer, 7 à modifier
✓ frontend installée — 106 créés, 7 modifiés

  cd frontend && npm install

  rbs generate client --lang ts

  celle-ci passe avant npm run build : le socle importe le client engendré, et la vérification des types s'arrête sans lui

  npm run build (ou npm run dev, qui sert le client sur son propre port)

  cargo run : le binaire sert le build, et jusque-là une page qui nomme ce qu'il reste à taper
```

L'ordre de ces gestes n'est pas décoratif. **Le socle importe le client typé, et ce client
n'est pas livré** — il est engendré depuis le document OpenAPI de *votre* projet par
[`rbs generate client`](../cli/client.md), parce que le contrat d'un projet n'est pas celui
d'un autre et qu'un client figé mentirait dès la première route ajoutée. Sautez cette
commande et `npm run typecheck` s'arrête sur un import que rien n'a écrit. Pas de `--out` :
sur un projet qui porte `frontend`, la commande écrit déjà là où le socle lit.

La couche de transport appartient au socle et non au shell d'administration — un socle qui
ne saurait pas appeler sa propre API serait une vitrine. La sonde de l'accueil passe par
elle. Une instance, un endroit ; les seuls `fetch` qui restent dans l'application sont les
deux routes de documentation que l'accueil interroge, qui ne sont pas des opérations du
contrat et qu'un client engendré depuis lui ne saurait décrire :

```ts file=examples/admin-console/frontend/src/api/index.ts
```

Deux autres lignes comptent au-delà du décompte. `.gitignore` reçoit
`frontend/node_modules/` et `frontend/dist/`, si bien que le premier `npm install` ne vous
propose pas trente mille fichiers à commiter. Et `Cargo.toml` ne reçoit rien qui réclame
Node : **`cargo build` réussit sur une machine qui n'en a pas**, ce qui est la contrepartie
d'un mécanisme de fragments purement déclaratif — le CLI ne lance aucune commande, donc
jamais `npm`.

## Le service statique est un module

Le socle ne jette pas un repli dans l'ancre du routeur. Il installe un module — sur la forme
qu'emploie le fragment `cors` — qui porte sa configuration, ses routes et ses tests, et
l'ancre ne reçoit qu'un appel.

```rust file=examples/admin-console/src/modules/frontend/mod.rs region=repli
```

Un repli plutôt que des routes nommées est tout le mécanisme : ce qui est monté ailleurs —
votre API, `/health`, le document OpenAPI — est trouvé avant lui, et il ne peut donc masquer
aucune route du service. C'est aussi ce qui rend l'application sur un rechargement en route
profonde, au lieu d'un 404.

```rust file=examples/admin-console/src/modules/frontend/mod.rs region=servir
```

Le fichier d'index est interrogé à chaque requête et non une fois au démarrage. C'est
voulu : la page d'amorçage s'efface d'elle-même dès que le build a écrit, sans réglage à
basculer ni serveur à relancer. Le coût est un appel système, sur le seul chemin du repli.

## La page d'amorçage

Le mécanisme des fragments est déclaratif : aucun hook, aucune exécution de commande. Le CLI
ne peut donc pas installer les dépendances du client pour vous, et le build est absent au
premier `cargo run`. Plutôt qu'un 404, le module sert alors une page autonome — sans
dépendance, sur des polices système, composée dans le monde visuel du projet — qui imprime le
nom du projet, l'état réel de sa base et les commandes qu'il reste à taper.

C'est la contrainte transformée en premier contact. Elle disparaît d'elle-même.

## Configuration

```toml file=examples/admin-console/config/default.toml region=frontend
```

`dir` est l'endroit où le build atterrit, `index` le fichier que le repli sert pour une route
profonde. Les deux suivent le mécanisme de configuration du projet, si bien que
`RBS_FRONTEND__DIR` remplace `dir` sans toucher au fichier — voir
[Configuration](./configuration.md).

## L'accueil

L'accueil du socle est public, et son contenu par défaut est **vrai au premier démarrage**.
Il nomme le projet réel, interroge réellement sa sonde de santé, renvoie à la documentation
OpenAPI que le service sert déjà, et montre une commande `curl` qui marche. Aucun texte de
remplissage, aucune affirmation inventée : rien sur la page n'a besoin d'être vidé avant
qu'on s'en serve.

Sa structure est celle d'une vitrine — sonde, essai, routes, documentation, composants — pour
qu'on la remplace section par section vers une vraie page produit. Elle reste lisible sur
téléphone, et respecte `prefers-reduced-motion`.

## Le monde visuel

Le monde est celui des **bandes de listing** : papier en continu, marge perforée, encre de
ruban d'impact. C'est un refus délibéré du standard de la catégorie — fond charbon, dégradé
maillé, accent néon — et
[ADR-0002](https://github.com/tky0065/rbs/blob/main/docs/adr/0002-monde-visuel-bandes-de-listing.md)
consigne les trois directions écartées avec lui.

Trois disciplines contraignent toute retouche ultérieure : la **bande** est un matériau et
non un fond, donc aucun contenu ne repose sur du blanc indifférencié ; la hiérarchie se fait
par contraste d'échelle seul, sans niveau intermédiaire ; et tout élément graphique est un
caractère, un filet ou une perforation.

**Aucun emoji.** Les icônes vectorielles sont admises là où elles sont une affordance —
chevron d'un menu, croix d'une boîte de dialogue, flèche de tri — jamais comme ornement. Sur
la page d'amorçage, qui n'a pas accès au registre npm, elles sont en SVG écrit à la main.

Le monde est entièrement tokenisé. Le remplacer revient à réécrire le bloc de thème, `@theme`
dans `frontend/src/assets/main.css` : aucun composant ne nomme une couleur, ils ne
connaissent que les rôles que ce bloc définit.

**Deux jeux de valeurs, un seul interrupteur.** Un second bloc, `:root.sombre`, donne aux
mêmes rôles leurs valeurs de papier carbone ; `frontend/src/lib/theme.ts` pose cette classe
sur la racine du document au démarrage, à partir du choix retenu ou, à défaut, de la
préférence du système. La variante sombre s'allume donc sur un projet à socle seul — le
shell d'administration n'ajoute que l'interrupteur qui la bascule.

## Ce que le shell d'administration ajoute

Dix-neuf fichiers, dans l'arbre que le socle a posé — une application, deux régimes de route.
Le shell exige [`auth`](./auth.md), qui tire à son tour `mail` et `rate-limit` : tous
descendent d'un seul plan, nommés avant que rien ne soit écrit.

{/* rbs:transcript cmd="rbs add frontend-admin" setup="rbs new demo --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" extrait="oui" */}
```text
$ rbs add frontend-admin
frontend-admin : le shell d'administration : connexion, jetons, garde de route et les deux stores de l'application
frontend-admin exige frontend, mail, rate-limit, auth : posée avec elle

plan pour …/demo

  + frontend/src/api/jetons.ts                                              créé
  + frontend/src/api/entetes.ts                                             créé
  + frontend/src/stores/authentification.ts                                 créé
  + frontend/src/stores/interface.ts                                        créé
  + frontend/src/admin/montage.ts                                           créé
  + frontend/src/admin/garde.ts                                             créé
  + frontend/src/admin/rail.ts                                              créé
  + frontend/src/admin/document.ts                                          créé
  + frontend/src/admin/textes.ts                                            créé
  + frontend/src/admin/lien.ts                                              créé
  + frontend/src/admin/Shell.vue                                            créé
  + frontend/src/admin/vues/Connexion.vue                                   créé
  + frontend/src/admin/vues/Inscription.vue                                 créé
  + frontend/src/admin/vues/Reinitialisation.vue                            créé
  + frontend/src/admin/vues/Verification.vue                                créé
  + frontend/src/admin/vues/TableauDeBord.vue                               créé
  + frontend/src/admin/vues/Sessions.vue                                    créé
  + frontend/src/admin/vues/Profil.vue                                      créé
  + frontend/src/admin/vues/Demonstration.vue                               créé

  175 à créer, 15 à modifier
✓ frontend-admin installée — 175 créés, 15 modifiés

  cd frontend && npm install

  rbs generate client --lang ts

  celle-ci passe avant npm run build : le socle importe le client engendré, et la vérification des types s'arrête sans lui

  npm run build (ou npm run dev, qui sert le client sur son propre port)

  cargo run : le binaire sert le build, et jusque-là une page qui nomme ce qu'il reste à taper

  rbs seed pose le compte d'administration dans la table des comptes : ADMIN_EMAIL (admin@demo.test) et ADMIN_PASSWORD, tiré dans votre .env, sont les identifiants que l'écran de connexion demande

  puis cargo run : l'écran de connexion est servi sur /admin
```

Le shell ne dit rien du client typé : ce geste appartient au socle, qui porte le module
l'instanciant, et les gestes du socle s'affichent avant les siens.

Tout ce que le shell ajoute à la couche de transport est un en-tête.
`frontend/src/api/entetes.ts` lit le jeton d'accès et rend un en-tête `authorization`, et
le `src/api/index.ts` du socle le découvre comme le routeur découvre un montage — un
fragment ne peut pas redéposer un fichier qu'un autre a posé.

### Huit écrans, chacun adossé à une vraie route

Quatre vivent derrière la garde : un tableau de bord qui montre les sondes réelles, la
version et le nombre de routes montées lu dans le document OpenAPI ; les sessions ouvertes
avec leur révocation unitaire et globale ; et le profil, qui *écrit* désormais autant qu'il
lit — `PATCH /auth/me` change l'adresse qu'il montre, retire la preuve qui portait sur
l'ancienne, et envoie un lien de vérification neuf.

Quatre sont publiques, parce qu'une garde sans autre page que la connexion enfermerait
dehors qui n'a pas encore de compte, qui a perdu son mot de passe, ou dont l'adresse attend
sa preuve : la connexion avec son dialogue de réinitialisation, l'inscription, l'écran du
nouveau mot de passe, et l'écran de preuve d'adresse avec son renvoi. Les deux derniers
lisent le jeton que porte le lien du courriel — dans le *fragment* de l'URL, qu'un
navigateur n'envoie jamais au serveur — par un seul module qu'ils partagent.

Elles vivent hors du shell, et hors du rail : le rail est la navigation d'un espace
authentifié, et la connexion n'y a jamais figuré non plus. Les trois chemins qu'`auth` met
dans ses courriels — `/forgot-password`, `/reset-password`, `/verify-email` — sont les alias
de trois d'entre elles : le fragment les compose depuis `app_url` sans rien savoir d'un
shell posé à côté, c'est donc au shell de les servir, et un alias les sert sans redirection,
qui aurait perdu le jeton.

L'inscription demande `GET /auth/registration` avant de montrer son formulaire, et affiche
le refus à la place quand `registration_enabled` vaut `false` ; la connexion retire dans le
même cas le lien qui y mène. Chaque écran est adossé à une route qu'`auth` expose réellement
— aucun ne montre un chiffre que personne ne sert, et aucune route qu'`auth` expose ne reste
sans appelant.

### Le transport des jetons, et ce qu'il coûte

`auth` rend une paire en JSON — un jeton d'accès signé de courte durée, un jeton de
rafraîchissement opaque de longue durée — présentée en en-tête `Authorization`, sans cookie
ni session côté serveur. Le client conserve donc **le jeton d'accès en mémoire et le jeton de
rafraîchissement dans le stockage local** :

```ts file=examples/admin-console/frontend/src/api/jetons.ts region=jetons
```

Le prix est réel et il est assumé : un script injecté dans cette application lit le stockage
local et repart avec une session de longue durée, là où le jeton d'accès, lui, n'aurait pas
survécu à l'onglet. L'alternative sûre est un flux par cookie `HttpOnly`, qui demande de
modifier le fragment `auth` côté serveur — un autre chantier, pas un réglage d'ici. En
attendant, `POST /auth/logout` révoque la session côté serveur, et l'écran des sessions coupe
un accès qu'on ne reconnaît pas.

### L'état côté client

**Deux stores Pinia, pas plus** : l'authentification (jetons, utilisateur courant,
renouvellement) et l'interface (thème, état du rail, notifications). Toute ressource métier
passe par un composable au-dessus du client typé. Un store par entité est le réflexe dont
Pinia s'est précisément affranchi, et il doublerait le code à recopier pour chaque écran
nouveau.

## Les écrans engendrés

Sur un projet qui porte le shell, `rbs generate crud` écrit l'écran d'administration de la
table — liste filtrée, formulaire, détail — en plus de l'entité, de sa migration et de ses
routes fermées. **Aucun drapeau ne le demande** : la commande lit les fragments installés et
adapte sa sortie, exactement comme elle écrit des routes fermées dès qu'`auth` est là.
`--no-admin` est la sortie de secours, pour une table que personne ne doit administrer depuis
l'interface.

C'est un renversement de portée daté. « Interface d'administration générée » figurait depuis
l'origine dans le hors-périmètre de la feuille de route ;
[ADR-0003](https://github.com/tky0065/rbs/blob/main/docs/adr/0003-ecrans-d-administration-engendres.md)
le renverse et abroge
[ADR-0001](https://github.com/tky0065/rbs/blob/main/docs/adr/0001-squelette-admin-plutot-qu-interface-engendree.md),
dont l'analyse du coût reste juste — seul l'arbitrage a changé. Ce qui reste hors périmètre
est le moteur générique piloté par le document OpenAPI à l'exécution : des écrans qu'on ne
personnalise pas sans forker le moteur contredisent la règle selon laquelle le code engendré
est fait pour être modifié. Un écran engendré appartient à son auteur dès qu'il est écrit.

L'écran se monte par **deux ancres, et deux seulement**. La table de routage :

```ts file=examples/admin-console/frontend/src/admin/montage.ts region=montage
```

Et le rail, dont le shell parcourt les entrées deux fois — un rail à demeure au-delà de la
largeur d'un ordinateur portable, un panneau latéral en deçà :

```ts file=examples/admin-console/frontend/src/admin/rail.ts
```

Il n'y a pas de troisième ancre pour déclarer le module : en TypeScript, l'import qui donne
son composant à la route *est* la déclaration, là où Rust demande un `pub mod` distinct du
montage. Les deux entrées ci-dessus sont de la même forme — la première est l'écran de
démonstration que le fragment dépose, la seconde celui que la commande a engendré.

### Une seule template, deux producteurs

L'écran de démonstration et chaque écran engendré sortent de la **même template**. Le
fragment la rend une fois pour une entité de démonstration ; la commande la rend pour chaque
table réelle. Ce qu'on voit à l'installation est donc exactement ce qu'on obtiendra ensuite —
ce qui n'était vrai dans aucune des deux autres options : un shell sans écran de
démonstration ouvre sur un espace vide et muet, et un écran de démonstration distinct du
gabarit en diverge.

Ce qui sépare les deux producteurs est une source : quatre fonctions, plus le traducteur de
ligne et la phrase d'erreur. Le fragment les rend sur un tableau écrit dans le fichier ; la
commande les rend sur le client typé :

```ts file=examples/admin-console/frontend/src/admin/vues/Incidents.vue region=source
```

Tout ce qui suit — filtre, tri, pagination, formulaire, détail, rendu — ne connaît que
`Ligne`, `Requete` et `Formulaire`, et ne change pas d'une table à l'autre.

### Ce que porte le formulaire

Un contrôle par colonne déclarée : un champ texte pour une chaîne, un texte long, un UUID ou
une référence ; le champ numérique du navigateur pour un entier ou un flottant ; un champ
texte pour un décimal, que le contrat porte en chaîne afin qu'un nombre JavaScript n'en perde
pas les centimes ; une case à cocher pour un booléen ; les contrôles natifs de date et
d'instant, dont la sortie est exactement le format que le contrat attend — le fuseau de
l'instant se repose à l'envoi ; et une liste pour une énumération, typée comme l'union des
valeurs qu'elle déclare.

Une colonne facultative peut rester vide et envoie alors `null`, la seule lecture qui laisse
la remettre à zéro depuis le formulaire.

Le filtre porte sur la **première colonne textuelle** de la table, et une table qui n'en a
aucune n'affiche pas de champ de filtre plutôt qu'un champ qui ne filtre rien : le filtre du
projet conjugue ses conditions par ET, et chercher un motif dans plusieurs colonnes
demanderait un OU que le contrat n'expose pas.

### Régénérer

L'écran est du code ordinaire, qui vous appartient. Régénérer une table dont les colonnes ont
changé passe par la porte habituelle : le plan signale l'écran en conflit, et seul `--force`
l'écrase — le même contrat qu'offre [`rbs generate client`](../cli/client.md). Les deux
ancres sont idempotentes, si bien qu'une régénération n'ajoute ni seconde route ni seconde
entrée de rail.

Un nom est refusé : une table nommée `demonstration`, sur un projet qui porte le shell,
rendrait son écran sur le fichier et la route de l'écran de démonstration. La commande le dit
et nomme `--no-admin`.

## Développement et production

Le fragment écrit ses raccourcis dans le
[`Makefile`](../cli/new.md#les-raccourcis-du-projet) en s'installant : `make front` pour le
serveur de développement, `make front-build` pour le build que sert le binaire, `make
typecheck` pour `vue-tsc`. Il y ajoute aussi sa moitié de `make dev`, qui mène dès lors le
binaire et Vite de front, dans un même groupe de processus — un seul Ctrl-C arrête les deux.

En production, le binaire sert le build lui-même : ni second serveur, ni reverse proxy. En
développement, `npm run dev` sert le client sur le port de Vite avec son rechargement à
chaud, et relaie au binaire ce qu'il ne sert pas lui-même :

```ts file=examples/admin-console/frontend/vite.config.ts region=relais
```

Une route que votre projet ajoute se déclare là aussi, faute de quoi elle ne répondra qu'une
fois le build en place. `rbs generate crud` y inscrit lui-même la sienne, dans l'ancre
`// <rbs:vite_proxy>` — `'/articles',` pour une table nommée `articles` — si bien qu'un écran
engendré fonctionne sous `npm run dev` sans une ligne à ajouter ; une route que vous écrivez
vous-même se déclare à côté, à la main. Le relais est aussi la raison pour laquelle
l'installation par défaut n'a pas besoin de CORS : le navigateur ne voit qu'une origine. [`rbs add cors`](../cli/add.md#les-features) vise le cas où le client est
servi depuis une *autre* origine, et `admin-console` le porte pour que la configuration soit
sous les yeux.

## Tests

Le module livre ses propres tests dans votre projet, comme le fait `cors` : le routage du
service statique, le repli sur l'application, la non-interception de l'API et du document
OpenAPI, la page d'amorçage servie quand le build est absent et effacée quand il est présent.

```bash
cargo test --workspace modules::frontend
```

Node n'y est pour rien : ce sont des tests Rust sur le repli, et ils passent sur une machine
qui n'a jamais vu `npm`.

Du côté de rbs lui-même, `examples/admin-console` est le seul exemple construit deux fois —
en Rust comme les quatre autres, et côté client par un job de CI dédié qui régénère le client
typé, installe les dépendances, vérifie les types et construit. Sans lui, plusieurs centaines
de lignes de TypeScript ne seraient compilées nulle part.

## Ce que le fragment vous laisse

Les textes suivent la langue choisie à la création du projet et sont rendus à la génération.
Il n'y a **aucune bibliothèque d'internationalisation** : elle ajouterait une dépendance, une
indirection et deux catalogues à maintenir dans un socle dont le premier geste de son
propriétaire sera de réécrire les textes. Les réécrire, c'est éditer les fichiers.

L'embarquement du build dans le binaire Rust est hors périmètre : il rendrait `cargo build`
dépendant d'une construction npm préalable. Tenir à jour les composants vendorisés est une
tâche de maintenance récurrente, et non un mécanisme — le thème est inliné à la main
précisément pour que l'installation ne dépende d'aucun générateur tiers.
