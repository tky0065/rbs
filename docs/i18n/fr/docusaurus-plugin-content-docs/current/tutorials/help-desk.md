---
sidebar_position: 10
title: Construire un gestionnaire de tickets
---

# Construire un gestionnaire de tickets

Les autres tutoriels ajoutent chacun une fonctionnalité à `demo`. Celui-ci part de rien et
aboutit à une application : un gestionnaire de tickets où des personnes connectées ouvrent
des tickets et les commentent, depuis un espace d'administration que le binaire sert
lui-même. En chemin, il assemble ce que les autres pages montraient une à une — `auth`, les
relations entre tables, `frontend-admin`, le client typé — et montre la seule chose
qu'aucune ne fait : où le code engendré se retouche, et pourquoi.

C'est un projet neuf, `help-desk`, et non `demo`, qui porte `articles` et n'a ni `auth`
ni frontend. Chaque extrait ci-dessous est lu dans
[`examples/help-desk`](https://github.com/tky0065/rbs/tree/main/examples/help-desk), un
projet engendré par les mêmes commandes et compilé en CI.

## Ce qu'il vous faut

Ce que demandait [Préparer le terrain](./setup.md) — Rust, `rbs`, Docker pour
PostgreSQL —, plus Node.js pour le frontend, et un navigateur.

## 1. Créer le projet

```bash
rbs new help-desk --lang fr
cd help-desk
git add -A && git commit -q -m "projet neuf"
rbs add frontend-admin
```

`add` refuse d'écrire dans un working tree sale, et `rbs new` initialise le dépôt sans
rien commiter — d'où le commit qui le précède. Le plan est long ; ses lignes de fichiers
sont coupées ci-dessous, et il en reste la tête et la queue.

{/* rbs:transcript cmd="rbs add frontend-admin" setup="rbs new help-desk --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/help_desk && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="help-desk" extrait="oui" */}
```text
frontend-admin : le shell d'administration : connexion, jetons, garde de route et les deux stores de l'application
frontend-admin exige frontend, mail, rate-limit, auth : posée avec elle

plan pour …/help-desk

  + src/modules/frontend/mod.rs                                             créé
  + frontend/src/admin/vues/Demonstration.vue                               créé
  ~ AGENTS.md                                                               modifié

  175 à créer, 15 à modifier
✓ frontend-admin installée — 175 créés, 15 modifiés
```

Un fragment demandé, cinq installés. Le shell d'administration exige qu'on soit connecté,
il tire donc `auth` ; `auth` envoie des liens de vérification et de réinitialisation, il
tire donc `mail`, et il freine les tentatives de connexion, il tire donc `rate-limit`. Le
shell est une application Vue 3 montée sous `/admin`, bâtie sur le socle que pose
`frontend`. Dès lors, le projet se sait doté d'un frontend, et les deux commandes suivantes
en tiennent compte.

## 2. Modéliser les tickets

```bash
git add -A && git commit -q -m "frontend-admin installée"
rbs generate crud tickets \
  --fields "sujet:string,detail:text,statut:enum(ouvert,en_cours,resolu,ferme),priorite:enum(basse,normale,haute),auteur:references:users"
```

`generate` refuse lui aussi un arbre sale : chaque étape de cette page commence donc par
un commit.

{/* rbs:transcript cmd="rbs generate crud tickets --fields sujet:string,detail:text,statut:enum(ouvert,en_cours,resolu,ferme),priorite:enum(basse,normale,haute),auteur:references:users" setup="rbs new help-desk --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/help_desk && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add frontend-admin && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="help-desk" extrait="oui" */}
```text
  + src/tickets/controller.rs                          créé
  + migration/src/m20260923_134225_create_tickets.rs   créé
  + frontend/src/admin/vues/Tickets.vue                créé
  ~ src/auth/model.rs                                  modifié
  ~ frontend/vite.config.ts                            modifié
  ~ frontend/src/admin/montage.ts                      modifié
  ~ frontend/src/admin/rail.ts                         modifié

  12 à créer, 10 à modifier

  la référence « auteur » est requise : ni le seed de tickets ni ses scénarios de création ne peuvent deviner vers quelle ligne pointer — le seed n'est pas engendré, et les tests s'arrêtent aux cas qui ne créent rien
✓ tickets générée — 12 créés, 10 modifiés
```

Trois sortes de champ, chacune choisie pour ce qu'elle apporte d'elle-même :

- `statut` et `priorite` sont des **énumérations**. La base les garde par un `CHECK`, le
  Rust par un enum, et l'écran les rend en listes déroulantes.
- `auteur:references:users` est une **référence** vers une table que le projet a déjà —
  les comptes qu'a créés `auth`. Le CLI inventorie les entités du projet avant de lire ce
  que vous avez tapé : c'est ainsi qu'il sait que `users` vit dans `src/auth/model.rs`, et
  qu'il y écrit la relation inverse.
- Le reste, ce sont des colonnes ordinaires.

Les lignes `frontend/` sont propres à cette commande : parce que le projet porte
`frontend-admin`, elle a écrit un écran `Tickets.vue`, l'a monté dans le routeur et le
rail, et a dit au serveur de développement de Vite de relayer `/tickets` au binaire.

L'avertissement qui précède la dernière ligne compte pour la suite. Une référence requise
est une référence qu'aucun test engendré ne sait remplir — il ignore vers quel compte
pointer —, si bien que les tests de `tickets` s'arrêtent aux cas qui ne créent rien. La
section 4 est celle qui comble ce manque.

## 3. Y accrocher des commentaires

Un commentaire appartient à un ticket : `commentaires` référence donc `tickets`. L'ordre
des deux commandes est celui de la clé étrangère, et le CLI l'impose. Lancez la seconde
en premier, et rien n'est écrit :

{/* rbs:transcript cmd="rbs generate crud commentaires --fields corps:text,ticket:references:tickets:cascade,auteur:references:users --dry-run" setup="rbs new help-desk --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/help_desk && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add frontend-admin && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="help-desk" */}
```text
erreur : relation « ticket » — « tickets » est introuvable dans ce projet
        → entités connues : commentaires, one_time_tokens, refresh_tokens, users
```

Dans le bon ordre :

```bash
git add -A && git commit -q -m "tickets"
rbs generate crud commentaires \
  --fields "corps:text,ticket:references:tickets:cascade,auteur:references:users"
git add -A && git commit -q -m "commentaires"
```

{/* rbs:transcript cmd="rbs generate crud commentaires --fields corps:text,ticket:references:tickets:cascade,auteur:references:users" setup="rbs new help-desk --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/help_desk && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add frontend-admin && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs generate crud tickets --fields sujet:string,detail:text,statut:enum(ouvert,en_cours,resolu,ferme),priorite:enum(basse,normale,haute),auteur:references:users && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="help-desk" extrait="oui" */}
```text
  + migration/src/m20260923_134226_create_commentaires.rs   créé
  + frontend/src/admin/vues/Commentaires.vue                créé
  ~ src/tickets/model.rs                                    modifié

  12 à créer, 11 à modifier
✓ commentaires générée — 12 créés, 11 modifiés
```

`cascade` fait qu'un ticket supprimé emporte ses commentaires. `auteur` n'a pas de
politique, il prend donc celle par défaut, `Restrict` : un compte qui a écrit ne peut pas
être supprimé sous ce qu'il a écrit. `src/tickets/model.rs` paraît modifié parce qu'un
ticket *a* désormais des commentaires, et SeaORM veut aussi ce côté de la relation. Les
deux migrations sont nées à une seconde d'écart, et c'est ce qui garantit que `tickets` est
créée avant la table qui la vise.

## 4. Lire l'auteur dans le jeton

Tout ce qui précède est engendré, et présente un trou. Tel qu'engendré, créer un ticket
lit `auteur_id` dans le corps de la requête. Tout compte connecté peut donc ouvrir un
ticket au nom d'un autre, et l'écran demande à qui remplit le formulaire de coller l'UUID
d'un compte. L'auteur n'est pas à choisir par l'appelant : c'est l'appelant.

Trois fichiers changent, un par couche, dans l'ordre où une requête les traverse. D'abord,
les types d'entrée cessent d'accepter le champ, à la création comme à la mise à jour — un
auteur ne se réécrit pas. La réponse le garde.

```rust file=examples/help-desk/src/tickets/dto.rs region=entree
```

Le contrôleur est la seule couche qui voit le jeton. `Identity` l'a déjà vérifié avant que
le handler ne s'exécute ; `user_uuid()` lit le compte auquel il appartient, et le transmet.

```rust file=examples/help-desk/src/tickets/controller.rs region=create
```

Le service reçoit l'auteur en paramètre et le pose, sans savoir d'où il vient. C'est la
dépendance à sens unique sur laquelle ce projet repose : le contrôleur connaît HTTP et les
jetons, le service connaît les tickets, et aucun ne se mêle des affaires de l'autre.

```rust file=examples/help-desk/src/tickets/service.rs region=create
```

`update`, dans le même fichier, perd simplement les trois lignes qui recopiaient
`auteur_id`.

Faites de même sur `commentaires` — son `dto.rs`, son `controller.rs` et son `service.rs`
changent de la même façon. `ticket_id` y reste dans le corps : à quel ticket un
commentaire appartient, c'est à l'appelant d'en décider ; qui l'a écrit, non.

Puis commitez, car la commande suivante ne tourne pas sur un arbre sale :

```bash
git add -A && git commit -q -m "l'auteur est lu dans le jeton"
```

## 5. Porter le changement jusqu'à l'écran

Les écrans appellent l'API par un client typé, engendré depuis le document OpenAPI du
serveur. Le contrat vient de changer : régénérez-le.

```bash
rbs generate client --lang ts --out frontend/src/api
```

{/* rbs:transcript cmd="rbs generate client --lang ts --out frontend/src/api" setup="rbs new help-desk --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/help_desk && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs add frontend-admin && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs generate crud tickets --fields sujet:string,detail:text,statut:enum(ouvert,en_cours,resolu,ferme),priorite:enum(basse,normale,haute),auteur:references:users && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init && rbs generate crud commentaires --fields corps:text,ticket:references:tickets:cascade,auteur:references:users && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="help-desk" */}
```text
plan pour …/help-desk

  + frontend/src/api/client.ts   créé

  1 à créer
✓ client engendré — frontend/src/api/client.ts porte 29 opérations
```

`CreateTicket`, dans `client.ts`, n'a plus d'`auteur_id`. On s'attendrait à ce que
l'écran cesse de passer la vérification des types, puisqu'il envoie encore ce champ. Il
n'en est rien : `npm run typecheck` passe. La fonction qui construit le corps de la
requête n'a pas de type de retour déclaré, si bien que TypeScript n'en contrôle pas les
propriétés en trop, et le serveur jette sans un mot un champ qu'il ne connaît pas. L'écran
continuerait d'afficher un champ « Auteur id » que personne ne lit.

Retouchez-le donc à la main. Dans `frontend/src/admin/vues/Tickets.vue`, retirez
`auteur_id` de l'interface `Formulaire`, du formulaire vierge `VIERGE`, de `corps()` et de
`saisie()`, et supprimez le bloc `champ-auteur_id` du formulaire. Laissez-le dans `Ligne`,
dans `depuis()` et dans la liste des colonnes — la table montre toujours qui a ouvert
chaque ticket.

```ts file=examples/help-desk/frontend/src/admin/vues/Tickets.vue region=corps
```

`Commentaires.vue` reçoit le même traitement. La leçon générale : le client typé suit le
contrat, mais un écran ne remarque la disparition d'un champ que là où il type ce qu'il
envoie.

Une limite reste en l'état. Dans `Commentaires.vue`, `ticket_id` est encore un champ texte
qui attend un UUID. Le remplacer par une liste de tickets est un travail Vue ordinaire, et
[Frontend](../guides/frontend.md) décrit l'écran que vous retoucheriez.

## 6. Le lancer

Démarrez PostgreSQL et Mailpit, appliquez les deux migrations, et créez le compte
d'administration que décrivait `add frontend-admin` :

```bash
docker compose up -d
rbs migrate up
rbs seed
```

Puis construisez le frontend et lancez le binaire, qui le sert :

```bash
cd frontend && npm install && npm run build && cd ..
cargo run
```

## Vérifier

Ouvrez [`http://127.0.0.1:8080/admin`](http://127.0.0.1:8080/admin) et connectez-vous avec
`ADMIN_EMAIL` et `ADMIN_PASSWORD`, tirés de votre `.env`. Le rail compte deux entrées de
plus, **Tickets** et **Commentaires**. Ouvrez un ticket : le formulaire demande un sujet,
un détail, un statut et une priorité, et pas d'auteur. Une fois enregistré, la liste
affiche l'identifiant de votre compte dans la colonne de l'auteur.

La même garantie est épinglée par un test, et c'est elle qui tient encore quand on ne
regarde plus. Il poste un ticket dont le corps nomme *un autre compte, réel* — un
identifiant inventé serait refusé par la clé étrangère, et le test passerait sans rien
prouver — et exige que le ticket revienne au nom de l'appelant :

```rust file=examples/help-desk/src/tickets/tests/auteur.rs region=auteur
```

L'exemple porte ce fichier et un second test, qui vérifie qu'un `PATCH` nommant autrui
laisse l'auteur en place. Tous deux lisent la base, comme tout test qui crée une ligne, et
sont donc `#[ignore]` par défaut :

```bash
cargo test -- --include-ignored
```

## Ce qui a été installé

Cinq fragments, deux tables, deux écrans et un client, tous venus du CLI ; une retouche à
la main, en deux endroits. Dans
[`examples/help-desk`](https://github.com/tky0065/rbs/tree/main/examples/help-desk) :

- `src/tickets/` et `src/commentaires/` — les deux features, `controller → service →
  repository → model`, la retouche portant sur `dto.rs`, `controller.rs` et `service.rs`.
- `src/tickets/tests/auteur.rs` — les deux tests écrits à la main.
- `frontend/src/admin/vues/Tickets.vue` et `Commentaires.vue` — les écrans, sans champ
  d'auteur.
- `frontend/src/api/client.ts` — le client typé, régénéré après la retouche.

## Pour aller plus loin

- [Relations](../guides/relations.md) couvre toute la grammaire de `references` : la forme
  un-à-un, `nullify`, et pourquoi toute clé étrangère reçoit un index.
- [Frontend](../guides/frontend.md) décrit les deux fragments et l'écran qu'écrit
  `generate crud` — le fichier à ouvrir pour un sélecteur de ticket.
- [Authentification](../guides/auth.md) couvre `Identity`, les rôles, et les routes qui
  gèrent les comptes.
- [Appeler l'API en TypeScript](./typescript-client.md) explique le client que cette page a
  régénéré.
