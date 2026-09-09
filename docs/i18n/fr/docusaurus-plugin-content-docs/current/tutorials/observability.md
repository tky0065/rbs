---
sidebar_position: 8
title: Voir ce que fait l'API
---

# Voir ce que fait l'API

C'est le huitième des neuf tutoriels. Il reprend `demo` exactement là où [Préparer le
terrain](./setup.md) l'a laissé : en cours d'exécution, sans rien de monté hormis un
contrôle de santé. Le cas : une route est devenue lente, et rien ne permet encore de
dire depuis quand, ni laquelle.

## Ce qu'il vous faut

Rien de plus qu'à [Préparer le terrain](./setup.md) : le même `demo` en cours
d'exécution, et `curl` de nouveau.

## 1. Installer la fonctionnalité

```bash
rbs add observability
```

{/* rbs:transcript cmd="rbs add observability" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add observability
observability : observabilité : traces OTLP vers un collecteur, et un /metrics Prometheus sur son propre port

plan pour …/demo

  + src/modules/observability/mod.rs       créé
  + src/modules/observability/config.rs    créé
  + src/modules/observability/metrics.rs   créé
  + src/modules/observability/tests.rs     créé
  + src/modules/mod.rs                     créé
  ~ src/lib.rs                             modifié
  ~ src/router.rs                          modifié
  ~ src/main.rs                            modifié
  ~ Cargo.toml                             modifié
  ~ config/default.toml                    modifié
  ~ AGENTS.md                              modifié

  11 fichiers à écrire
✓ observability installée — 4 fichiers

  les métriques sont sur http://localhost:9090/metrics ; pour les traces, nommez un collecteur dans OTEL_EXPORTER_OTLP_ENDPOINT et videz le dernier lot par rbs_core::logs::shutdown() avant la fin du processus
```

À la différence des briques précédentes, `observability` n'attend pas que vous câbliez
quoi que ce soit : un middleware de comptage se pose dans `// <rbs:layers>`, à une
position qui suit `trace` et `request_id`, si bien qu'une requête qu'il refuse porte
tout de même un identifiant de requête et atteint le journal ; et un second listener
HTTP, lancé depuis `// <rbs:startup>`, se met à servir une route — `/metrics` — avant
même que le listener de l'API ne se soit lié. `config/default.toml` a gagné une section
`[observability]` qui porte le port de ce listener : `metrics_port = 9090`, distinct du
`8080` de `server.port` à dessein, ce dont traite la section suivante. Les traces sont
l'autre moitié de l'installation — elles quittent le processus par le noyau dès que
`OTEL_EXPORTER_OTLP_ENDPOINT` nomme un collecteur — mais rien plus bas n'y touche ;
cette page traite des métriques de bout en bout.

La séparation est délibérée, et c'est tout le sens du port de `config/default.toml` qui
ne rejoint pas celui de `server.port` : les métriques publient la topologie interne du
service — ses routes, leur volumétrie, sa version — à quiconque peut les atteindre, et
les exposer sur le port de l'API demanderait une règle de reverse-proxy à chaque
déploiement pour les empêcher de fuir au-delà de la lisière. Donnez-leur plutôt un
listener à elles, et la règle qui les garde internes devient une règle de pare-feu,
écrite une seule fois.

## 2. Lancer le serveur

```bash
cargo run
```

```text
INFO   demo::modules::observability  métriques  adresse=127.0.0.1:9090
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

La preuve que les deux listeners sont levés avant que quoi que ce soit ne leur demande
quoi que ce soit : la ligne des métriques s'affiche en premier — `// <rbs:startup>`
s'exécute avant même que `router()` ne construise le `Router` de l'API — et la seconde
confirme l'API elle-même, sur le port que chaque tutoriel précédent a déjà employé.

## Vérifier

Depuis le second terminal, le port de l'API d'abord :

```bash
curl -i http://127.0.0.1:8080/metrics
```

```text
HTTP/1.1 404 Not Found
x-request-id: 01M22V0E32T6TBZPBRYZNR0NRW
content-length: 0
date: Wed, 09 Sep 2026 10:21:53 GMT
```

La preuve que la route n'a jamais été montée sur `server.port` : ce n'est pas un droit
que l'API refuse, c'est un chemin qu'elle ne porte pas. Et le listener qui le porte,
lui :

```bash
curl -i http://127.0.0.1:9090/metrics
```

```text
HTTP/1.1 200 OK
content-type: text/plain; charset=utf-8
content-length: 1356
date: Wed, 09 Sep 2026 10:21:53 GMT

# TYPE http_requests_total counter
http_requests_total{method="GET",path="<hors route>",status="404"} 1

# TYPE http_requests_in_flight gauge
http_requests_in_flight 0

# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="0.005"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="0.01"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="0.025"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="0.05"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="0.1"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="0.25"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="0.5"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="1"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="2.5"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="5"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="10"} 1
http_request_duration_seconds_bucket{method="GET",path="<hors route>",le="+Inf"} 1
http_request_duration_seconds_sum{method="GET",path="<hors route>"} 0.000048875
http_request_duration_seconds_count{method="GET",path="<hors route>"} 1
```

La preuve de la même séparation, vue de l'autre côté : ce listener répond, dans le
format que scrute Prometheus, et la réponse ne doit rien à `curl` qui la lit — rien ici
ne compte cette requête même, parce que le middleware ci-dessus n'est posé que sur le
`Router` de l'API, jamais sur celui-ci. L'unique série présente est le `404` d'un
instant plus tôt : `<hors route>`, une étiquette fixe plutôt que le chemin littéral
qu'un scanner aurait tenté, est ce qui empêche une requête sans route d'ouvrir une série
qui lui soit propre — la propriété que garde à dessein le test de la section suivante,
pour les routes qui, elles, existent.

## Ce qui a été installé

Trois extraits, lus depuis
[`examples/newsletter-queue`](https://github.com/tky0065/rbs/tree/main/examples/newsletter-queue) —
la même commande montrée plus haut, lancée sur un projet compilé en CI. Rien ici n'est
propre au domaine : un projet ne portant qu'`observability` écrit les trois mêmes
fichiers.

### La configuration

```toml file=examples/newsletter-queue/config/default.toml region=metriques
```

### Le listener

```rust file=examples/newsletter-queue/src/modules/observability/mod.rs region=exposition
```

`bind` est la seule étape faillible de toute l'installation : un `metrics_port` qui
entre en collision avec `server.port`, ou avec tout autre écoute déjà prise, échoue
ici — avant que le listener de l'API lui-même ne se lie, et avec un message qui nomme
l'adresse qui n'aurait pas pu s'ouvrir.

### Le garde-fou de la cardinalité

Chaque série ci-dessus porte `path`, et chacune d'elles a lu `<hors route>` plutôt que
`/metrics` ou `/health` littéralement — cette discipline, une route à paramètre en
dépend tout autant, et elle n'est pas optionnelle : étiquetez une série par l'adresse
qu'un appelant a réellement tapée, et `/articles/{id}` ouvre une série par article jamais
demandé plutôt qu'une seule pour la route. Un collecteur a une limite dure au nombre de
séries qu'il peut tenir, et le trafic de production l'atteint en quelques heures, pas en
quelques mois, dès que cette discipline se relâche.

```rust file=examples/newsletter-queue/src/modules/observability/tests.rs region=cardinalite
```

## Pour aller plus loin

- [Observabilité](../guides/observability.md) couvre les traces en entier — l'export
  que la configuration de cette page n'a jamais activé, et comment il se compose avec
  l'abonné que le noyau installe.
- [Logs](../guides/logs.md) couvre l'abonné sur lequel les traces se greffent, et le
  formateur `pretty` derrière chaque ligne que le serveur de cette page a affichée.
- [`rbs add`](../cli/add.md) couvre les douze autres features que `demo` pourrait
  encore installer, `observability` désormais dessus.
- [Appeler l'API en TypeScript](./typescript-client.md) est le dernier tutoriel : un
  front qui appelle `articles` à travers un client lu depuis le document OpenAPI de
  l'API elle-même, plutôt qu'une seconde copie de ses types.
