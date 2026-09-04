---
sidebar_position: 1.5
title: Observabilité
---

# Observabilité

Les logs disent *ce qui s'est passé*. `rbs add observability` répond aux deux questions
suivantes : *quelle route est lente, et depuis quand* — par les métriques — et *sur quel
appel en aval* — par les traces. La feature installe quatre fichiers sous
`src/observability/`, une section `[observability]`, un middleware de comptage, et un
second listener HTTP qui sert `/metrics`.

```bash
rbs add observability
```

Tous les extraits de cette page sont tirés de
[`examples/newsletter-queue`](https://github.com/tky0065/rbs/tree/main/examples/newsletter-queue),
un projet engendré par le CLI et compilé en CI. Rien ici n'est écrit à la main pour la
documentation.

## Les traces sortent par le noyau, pas par le fragment

`rbs_core::logs::init()` est la première ligne du `main` engendré, et c'est lui qui pose
l'abonné global. Le `set_global_default` de `tracing` ne s'appelle qu'une fois par
processus, et l'ancre `// <rbs:startup>` s'exécute après lui : rien de ce qu'installe un
fragment ne pourrait donc greffer une couche d'export sur cet abonné.

La greffe vit par conséquent dans le noyau, derrière une feature cargo que le fragment
active. Voici la ligne de manifeste que laisse `rbs add observability` :

```toml file=examples/newsletter-queue/Cargo.toml region=noyau
```

L'exemple dépend du noyau par `path` parce qu'il vit dans ce dépôt et compile contre la
crate d'à côté ; votre projet, lui, porte un `version`. Ce que la commande a changé est la
dernière entrée de `features`.

Cette feature levée, `logs::init()` compose une couche d'export OTLP en même temps que le
formateur qu'il pose déjà. Le span exporté est celui que `rbs_core::trace` construit par
requête — celui-là même sur lequel vos logs se corrèlent.

Deux variables d'environnement le règlent, et ce sont celles que tout collecteur connaît
déjà :

| Variable | Effet |
|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Le collecteur, en OTLP/gRPC — `http://localhost:4317` pour un collecteur local. |
| `OTEL_SERVICE_NAME` | Le nom de service que portent les traces exportées. À défaut, le nom du binaire en cours. |

Elles viennent de l'environnement plutôt que de `config/default.toml` parce que
`logs::init()` s'exécute *avant* `Config::load()` : à cet instant, il n'y a aucune
configuration à lire.

**Pas d'endpoint, pas d'export.** Le fragment reste inerte tant que personne n'a nommé de
collecteur : un `cargo run` sur un poste de développement n'est pas ralenti par un
exportateur qui compose une adresse où rien ne répond.

### Vider le dernier lot

Les spans partent par lots. Un processus qui meurt entre deux lots emporte le dernier :
appelez donc ceci avant de sortir de `main` — c'est ce que fait l'exemple, à la toute fin
du sien :

```rust file=examples/newsletter-queue/src/main.rs region=arret
```

Sans la feature `observability`, l'appel ne fait rien, et l'appeler alors que rien n'a
jamais été installé n'est pas une faute. Le coût d'un oubli est ce dernier lot — pas une
panne, et c'est pourquoi rien dans le squelette ne l'appelle à votre place.

## Les métriques : trois séries, et une étiquette qui décide de tout

Le middleware va dans `// <rbs:layers>`, à l'intérieur de `trace` et de `request_id`.
C'est cette position qui lui donne l'identifiant de la requête, et qui fait entrer dans
ses compteurs les réponses courtes des couches posées avant lui — un 429 de `rate-limit`,
un préflight refusé par `cors`. Une couche posée plus bas les manquerait, et le taux
d'erreur publié serait faux.

| Série | Type | Étiquettes |
|---|---|---|
| `http_requests_total` | compteur | `method`, `path`, `status` |
| `http_request_duration_seconds` | histogramme | `method`, `path` |
| `http_requests_in_flight` | jauge | — |

**`path` est le gabarit de route, jamais l'URL demandée.** Il vient du `MatchedPath`
d'axum : `/articles/{id}` fait une série, quand l'URL demandée en ferait une par article —
et un collecteur tombe au bout de quelques heures sous ce régime. Une requête qui ne
correspond à aucune route est comptée sous une constante unique, pour la même raison : un
scanner qui frappe mille adresses inventées ouvre une série, et non mille.

C'est la contrainte autour de laquelle tout le module est bâti, et le
`src/observability/tests.rs` engendré la garde :

```rust file=examples/newsletter-queue/src/observability/tests.rs region=cardinalite
```

Un second test en fait autant pour un chemin qui ne correspond à aucune route. Les deux
passent sous `cargo test observability`, dans votre projet comme dans l'exemple.

## `/metrics` a son propre port

```toml file=examples/newsletter-queue/config/default.toml region=metriques
```

`/metrics` n'est jamais monté sur le routeur public. Les métriques publient la topologie
interne d'un service — ses routes, sa volumétrie, ses versions ; les poser sur le port de
l'API demanderait à chaque déploiement une règle de reverse-proxy pour les cacher, et un
déploiement qui oublie cette règle fuit sans le savoir. Un second listener, sur un port à
lui, n'a rien à oublier.

Il écoute sur le même hôte que l'API — `server.host` — pour qu'une interface se choisisse
là où l'API choisit la sienne, et non dans une seconde clé qui la contredirait. Le port
par défaut, 9090, diffère de `server.port` ; [`rbs doctor`](../cli/doctor.md) refuse une
configuration où les deux coïncident, un `bind` qui échoue au démarrage coûtant plus cher
à diagnostiquer qu'une configuration refusée avant.

Pointez Prometheus dessus, et il n'y a rien d'autre à faire — voici l'intégralité du
`prometheus.yml` de l'exemple :

```yaml file=examples/newsletter-queue/prometheus.yml
```

## Ce que le fragment ne fait pas

Les métriques métier sont les vôtres : la façade `metrics` est une dépendance de votre
projet, donc `metrics::counter!("orders_total").increment(1)` fonctionne depuis n'importe
quelle couche, sans registre à faire traverser `AppState`. Les métriques sont publiées au
format Prometheus, et non exportées en OTLP. Aucun tableau de bord n'est livré. Le
fragment pose la matière ; ce qui la consomme appartient au projet.
