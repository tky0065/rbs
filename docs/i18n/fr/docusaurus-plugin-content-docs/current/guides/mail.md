---
sidebar_position: 9
title: Courriel
---

# Courriel

`rbs add mail` installe l'envoi par SMTP dans un projet existant : cinq fichiers sous
`src/mail/`, un répertoire de gabarits, et un `Mailer` sur votre `AppState`. Comme les
autres briques, elle ne monte aucune route — le moment où un message part est une décision
que seul votre domaine peut prendre.

Tous les extraits de cette page viennent de
[`examples/newsletter-queue`](https://github.com/tky0065/rbs/tree/main/examples/newsletter-queue),
un
projet engendré par le CLI et compilé en CI. Rien ici n'est écrit à la main pour la
documentation.

## Ce qui est installé

```text
$ rbs add mail
mail : envoi de courriels par SMTP : transport partagé, gabarits minijinja

plan pour /private/tmp/rbs-demo/depot

  + src/mail/mod.rs                 créé
  + src/mail/config.rs              créé
  + src/mail/template.rs            créé
  + src/mail/service.rs             créé
  + src/mail/tests.rs               créé
  + templates/mail/bienvenue.html   créé
  ~ src/lib.rs                      modifié
  ~ src/state.rs                    modifié
  ~ docker-compose.yml              modifié
  ~ Cargo.toml                      modifié
  ~ config/default.toml             modifié
  ~ .env.example                    modifié
  ~ AGENTS.md                       modifié

  13 fichiers à écrire
✓ mail installée — 6 fichiers

  réglez [mail] dans config/default.toml — un SMTP local par défaut
```

Un gabarit d'exemple, `templates/mail/bienvenue.html`, l'accompagne — un point de départ
qui fonctionne, et que vous êtes censé remplacer. La ligne `docker-compose.yml` ci-dessus,
c'est un service `mailpit` sans profil : `docker compose up -d`, tel quel, celui que lance
[`rbs dev`](../cli/dev.md), le monte avec la base, et son interface web répond sur
`:8025`.

## Configuration

```rust file=examples/newsletter-queue/src/mail/config.rs
```

Les défauts décrivent un serveur de développement : le port 1025 en clair, celui sur lequel
écoutent [Mailpit](https://mailpit.axllent.org) et MailHog. La production, c'est
`tls = "starttls"` sur le port 587, ou `tls = "wrapper"` sur le port 465, et un
`smtp_user`.

**Le mot de passe est la seule valeur qu'aucun fichier versionné ne porte.** `rbs add mail`
écrit `RBS_MAIL__SMTP_PASSWORD=` dans `.env.example` et rien d'autre — `config/default.toml`
est versionné, et un mot de passe qui y figure est un mot de passe à changer.
[`rbs doctor`](../cli/doctor.md) diagnostique le couple, non la variable seule :

```text
  ✗ mail       RBS_MAIL__SMTP_PASSWORD n'est renseignée ni dans le .env ni dans l'environnement
      ajoutez au .env la ligne que mail y attend, vide tant que smtp_user l'est :
      RBS_MAIL__SMTP_PASSWORD=
```

Un mot de passe vide est légitime tant que `smtp_user` l'est aussi : un relais local sans
authentification n'a besoin ni de l'un ni de l'autre.

## Le transport

```rust file=examples/newsletter-queue/src/mail/service.rs region=construction
```

Deux choses se décident ici, et toutes deux portent sur le *moment* de l'échec.

Le bâtisseur est faillible mais synchrone : aucune socket n'est ouverte, la première
connexion attend le premier message. C'est ce qui garde `AppState::new` synchrone, comme
pour le [cache](./cache.md).

L'adresse de l'expéditeur, en revanche, est analysée **maintenant**. Une faute de frappe
dans `from` arrête le projet au démarrage plutôt qu'au premier envoi — le seul moment où
personne ne regarde les journaux. Notez l'exigence que porte le commentaire : bâtir depuis
un runtime Tokio, car le pool de `lettre` y inscrit une tâche d'entretien à sa création.

Le transport est bâti une fois et cloné avec l'état. `lettre` tient son propre pool de
connexions, qu'un transport par message rendrait inutile.

## Les gabarits

Les corps sont des gabarits [minijinja](https://docs.rs/minijinja) lus dans le répertoire
que nomme `templates` — `templates/mail` par défaut :

```html file=examples/newsletter-queue/templates/mail/newsletter.html
```

`Templates` enveloppe l'environnement dans un `Arc`, parce qu'`AppState` se clone à chaque
requête et que les gabarits doivent être chargés une fois, non une fois par appel. Il
emploie le `path_loader` de minijinja, qui refuse un nom absolu ou remontant : un nom de
gabarit venu d'une entrée utilisateur ne peut pas sortir du répertoire.

Quand un gabarit manque, l'erreur nomme le chemin complet — `templates/mail/absent.html` —
plutôt que le seul `absent.html` que connaît minijinja, car un nom seul n'oriente vers
aucun répertoire lorsqu'on cherche ce qui ne va pas.

## Envoyer

Le cas courant rend et envoie en un appel :

```rust file=examples/newsletter-queue/src/mail/service.rs region=send_template
```

`message()` reste disponible si vous préférez bâtir le `Message` vous-même, et `send()` en
prend un. Les corps partent en `text/html`.

Les pannes du transport reviennent en `Error::Internal` : un relais rompu est un fait
côté serveur, et n'apprend rien au client sur quoi il pourrait agir. Il part au journal, et
le client reçoit un 500 — voir le [guide des erreurs](./errors.md).

## Ne pas faire attendre le client

Envoyer dans un handler fait attendre la réponse HTTP après le serveur SMTP. Quand le
message ne conditionne pas la réponse, il peut partir détaché :

```rust file=examples/newsletter-queue/src/mail/service.rs region=send_detached
```

**Lisez le compromis avant d'y recourir.** Ni file ni réessai : un message perdu l'est pour
de bon, et le journal en est la seule trace. Ni le serveur SMTP indisponible une minute, ni
une résolution DNS qui bronche, ni un redémarrage entre le détachement et l'envoi — rien de
tout cela n'est rattrapable, puisque rien ne se souvient que le message a existé.

C'est le prix d'un envoi qui ne retient pas la réponse, et c'est un prix juste pour une
notification. Ce n'en est pas un pour une réinitialisation de mot de passe, où l'utilisateur
attend ce courriel et où rien d'autre ne lui dira qu'il a échoué.

## Le passage à un job

Quand il vous faut le réessai, le message doit survivre au processus. Cela veut dire
l'écrire quelque part, et c'est ce qu'est la [feature jobs](./jobs.md) :

```rust file=examples/newsletter-queue/src/jobs/newsletter.rs region=job
```

Lisez-le face au `send_detached` ci-dessus. Rien de l'envoi n'a changé — même `Mailer`, même
`send_template`, même gabarit. Ce qui a changé, c'est qui tient l'échec. `send_detached`
l'avale dans une ligne de journal ; ici l'erreur est *rendue*, et la rendre est ce qui
demande le réessai.

L'enfilage est l'autre moitié, et il a lieu dans la transaction qui a motivé le courriel :

```rust file=examples/newsletter-queue/src/subscribers/service.rs region=broadcast
```

Ce qui veut dire que la lettre existe si et seulement si le travail qui la justifiait a été
committé. Un `send_detached` deux lignes plus haut serait parti quoi qu'il arrive — y
compris pour une transaction annulée.

`send_detached` est toujours là, sous une permission qui ne vaut que pour lui : le fragment
doit rester utilisable seul, et un message dont la perte est sans conséquence n'a pas besoin
d'une ligne en base. Installer `jobs` est une décision, non un prérequis.

## Les tests

Le `src/mail/tests.rs` engendré n'a besoin d'aucun serveur pour six de ses sept tests : les
trois modes de chiffrement bâtissent chacun un transport, un expéditeur invalide est refusé
en le nommant, un message bâti porte son expéditeur et son destinataire, un gabarit rend
ses variables, et un gabarit absent nomme son fichier sans panique.

Le sixième mérite un regard. `send_detached` est prouvé contre un faux serveur qui accepte
la connexion et ne répond jamais, si bien que `lettre` reste suspendu sur la bannière SMTP :
un envoi attendu bloquerait le test. Deux assertions ensemble — l'appel rend la main, et la
connexion est tout de même établie — disent ce qu'un corps de méthode vide ne tiendrait pas.

Le septième est `#[ignore]` et sort vers le serveur SMTP de la section `[mail]` — Mailpit ou
MailHog en développement. `cargo test -- --ignored` le lance, et `RBS_MAIL__SMTP_PORT` en
surcharge le port. Voir le [guide des tests](./testing.md).

## Ce qu'elle vous laisse

- **quand envoyer** — aucune route, aucun crochet, aucun événement. L'exemple envoie à la
  création parce que c'est ce dont son domaine parle ;
- **la file et les réessais** — ni l'une ni les autres dans cette feature. `rbs add jobs`
  est ce qui les ajoute, et la section ci-dessus dit comment les deux s'emboîtent ;
- **les corps en texte simple et les pièces jointes** — les corps partent en HTML ;
  `lettre::Message` sait bâtir du multipart si vous en avez besoin ;
- **les retours d'erreur, les liens de désinscription, les gabarits par langue** — hors de
  cette feature ;
- **les limites de débit** — un fournisseur qui bride se manifestera en erreurs de
  transport, et rien ici ne ralentit.

Le code est dans votre arborescence, sans bandeau vous disant de ne pas y toucher.
