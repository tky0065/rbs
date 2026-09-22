---
sidebar_position: 5
title: Envoyer un mail
---

# Envoyer un mail

C'est le cinquième des neuf tutoriels. Il reprend `demo` juste après [Recevoir un
fichier](./storage.md) — en cours d'exécution, avec la ressource `uploads` et ses routes
de contenu de cette page. Le cas : dès que `create` réussit, quiconque possède
`owner_email` reçoit un mail lui disant que son fichier est enregistré — un accusé de
réception pour le dépôt que la page précédente a câblé.

## Ce qu'il vous faut

Rien de plus qu'à [Recevoir un fichier](./storage.md) : le même `demo` en cours
d'exécution, avec `uploads` et ses routes de contenu montées.

## 1. Installer la fonctionnalité

```bash
rbs add mail
```

{/* rbs:transcript cmd="rbs add mail" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add mail
mail : envoi de courriels par SMTP : transport partagé, gabarits minijinja

plan pour …/demo

  + src/modules/mail/mod.rs         créé
  + src/modules/mail/config.rs      créé
  + src/modules/mail/template.rs    créé
  + src/modules/mail/service.rs     créé
  + src/modules/mail/tests.rs       créé
  + templates/mail/bienvenue.html   créé
  + src/modules/mod.rs              créé
  ~ src/lib.rs                      modifié
  ~ src/state.rs                    modifié
  ~ docker-compose.yml              modifié
  ~ Cargo.toml                      modifié
  ~ config/default.toml             modifié
  ~ .env.example                    modifié
  ~ AGENTS.md                       modifié

  7 à créer, 7 à modifier
✓ mail installée — 7 créés, 7 modifiés

  réglez [mail] dans config/default.toml — un SMTP local par défaut
```

Comme `storage` avant elle, `mail` ne monte aucune route : ce que vous obtenez, c'est un
`Mailer` sur `AppState`, et l'appeler — à la création, à la confirmation, ou pour
n'importe quelle autre décision de votre domaine — reste du code que vous écrivez.
`add storage` et `add redis` ne sont pas relancées ici ;
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop) porte
les deux à côté de `mail`, câblées dans le service unique que cette page lit plus bas.

L'installation écrit aussi `RBS_MAIL__SMTP_PASSWORD=` dans `.env.example` — vide, et là
seulement, jamais dans `.env`. Ce n'est pas le secret tiré au hasard qu'`auth` dépose pour
sa clé de signature ; c'est un espace à remplir, laissé pour le jour où vous aurez un
vrai compte à partir duquel envoyer. Ce que la nouvelle section `[mail]` de
`config/default.toml` porte déjà, c'est un défaut qui fonctionne en développement :
`smtp_host = "localhost"`, le port `1025`, aucun identifiant — l'adresse du service
`mailpit` que l'installation vient d'ajouter à `docker-compose.yml`. `docker compose up
-d` le démarre désormais aux côtés de la base, et sa boîte répond sur `:8025`.

**Lisez le compromis avant de câbler un envoi.** Le `create` de `file-drop` envoie
l'accusé détaché : la réponse HTTP ne l'attend pas, et un envoi manqué ne laisse qu'une
ligne au journal — ni file ni réessai, le message est perdu pour de bon. C'est un prix
raisonnable pour une notification que personne n'attend. Ce n'en est pas un pour une
réinitialisation de mot de passe, ou tout autre courriel que l'appelant attend activement
et ne peut obtenir autrement — un courriel dont la perte doit être réparable a sa place
dans une file qui survit au processus, une autre fonctionnalité et un autre compromis que
celui que cette page installe.

## Vérifier

`generate crud` n'a aucune raison de savoir qu'une brique de mail existe, donc rien ne
câble `Mailer` dans un gestionnaire à votre place — l'appel ci-dessous, `notify`, est du
code de `file-drop`, lu plus bas sur cette page. Ce que `cargo test` peut vérifier sans
une seule ligne à vous, c'est la propriété dont cette page parle vraiment : un envoi
détaché rend-il la main avant que le message ne soit parti ?

```bash
cargo test modules::mail::
```

{/* rbs:libre raison="cargo test compile le projet entier, plusieurs minutes, et l'ordre de ses lignes suit l'ordonnanceur des threads" */}
```text
running 8 tests
test modules::mail::tests::an_invalid_sender_stops_the_build_naming_it ... ok
test modules::mail::tests::the_message_carries_the_configured_sender_and_its_recipient ... ok
test modules::mail::tests::a_missing_template_names_the_file_without_panicking ... ok
test modules::mail::tests::send_template_detached_fails_on_a_missing_template_before_sending_anything ... ok
test modules::mail::tests::the_rendered_template_carries_the_variables_passed_to_it ... ok
test modules::mail::tests::send_detached_returns_without_awaiting_the_send ... ok
test modules::mail::tests::the_three_encryption_modes_build_a_transport ... ok
test modules::mail::tests::a_templated_message_goes_out_to_the_smtp_server ... ignored, joint le serveur SMTP de la section [mail]

test result: ok. 7 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

La preuve du compromis lui-même : `send_detached_returns_without_awaiting_the_send` ouvre
une écoute qui accepte une connexion et ne répond jamais, puis vérifie à la fois que
l'appel rend la main en moins de 200ms *et* que la connexion a bien eu lieu — un envoi qui
attendrait vraiment resterait bloqué sur ce faux serveur, et un corps de méthode vide
n'aurait jamais rien connecté. Le test ignoré est la seule vérification réellement en
conditions réelles : il lui faut le Mailpit que `docker compose up -d` vient de
démarrer, et c'est `cargo test -- --ignored` qui le lance.

## Ce qui a été installé

Trois fichiers, lus depuis
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop) — la
seule commande `add mail` ci-dessus, lancée sur un projet compilé en CI.

:::note
`file-drop` porte les trois briques v0.3 à la fois — `storage`, `mail` et `redis` — sur un
même projet, si bien que son `src/uploads/service.rs` dépose et lit aussi du contenu, et
met la liste en cache, ce que cette page n'a installé ni l'un ni l'autre. L'extrait
`notify` ci-dessous est la part de ce fichier que le mail seul explique.
:::

### Le cas courant

`send_template` rend un gabarit et envoie le résultat en un seul appel — voici à quoi
ressemble un envoi attendu, celui qu'un appelant peut se permettre d'attendre.

```rust file=examples/file-drop/src/modules/mail/service.rs region=send_template
```

### L'envoi détaché

`send_detached` applique le même compromis à un message déjà construit : il lance sa
propre tâche et rend la main avant même que l'envoi commence, si bien que rien n'attend
dessus — ni file, ni réessai, et un envoi manqué n'atteint que le journal. `notify`
ci-dessous fait le même choix à la main, un cran plus tôt, pour un gabarit plutôt qu'un
`Message` déjà prêt.

```rust file=examples/file-drop/src/modules/mail/service.rs region=send_detached
```

### Le point d'appel

`notify` est ce que le `create` de `file-drop` appelle une fois la ligne écrite : il
lance sa propre tâche, et une erreur d'envoi va au journal — jamais vers la réponse HTTP,
déjà repartie avec un `201`.

```rust file=examples/file-drop/src/uploads/service.rs region=notify
```

## Pour aller plus loin

- [Mail](../guides/mail.md) couvre le transport, les gabarits, et le déplacement d'un
  envoi vers une file quand le compromis ci-dessus n'est pas le bon.
- [`rbs add`](../cli/add.md) couvre les onze autres features que `demo` pourrait encore
  installer, `storage` et `mail` désormais sur lui.
- [Tests](../guides/testing.md) est le harnais contre lequel `mail/tests.rs` engendré
  tourne, et ce que `-- --ignored` atteint qu'un simple `cargo test` n'atteint pas.
- [Ne pas recalculer deux fois](./cache.md) est le tutoriel suivant : un `COUNT(*)` lu
  mille fois par minute, mis en cache plutôt que recalculé à chaque appel.
