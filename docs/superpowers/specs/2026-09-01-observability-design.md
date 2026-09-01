# `rbs add observability` — traces OTLP et `/metrics`

**Tâche** : `IMPROVE.md` #56. **Date** : 2026-09-01. **Statut** : validé, prêt à planifier.

## Le problème

Une API engendrée par rbs est muette en production au-delà de ses logs. `trace.rs` pose
bien un span par requête, mais il ne sort jamais du processus : rien ne l'exporte, et
aucune métrique n'est publiée. Le premier réflexe d'exploitation — « quelle route est
lente, depuis quand, et sur quel appel en aval » — n'a aucune réponse.

## Le point dur, et sa résolution

`rbs_core::logs::init()` (`logs/mod.rs:69`) **pose l'abonné global lui-même**, et c'est la
première ligne de `main`. L'ancre `startup` s'exécute après. Or `set_global_default` ne
s'appelle qu'une fois : un fragment installé ne peut pas y greffer la couche
`tracing-opentelemetry` depuis `startup`, et le CLI ne réécrit jamais une ligne existante
de `main.rs` — il n'insère que dans des ancres.

**La greffe se fait donc dans le noyau, derrière une feature cargo.** `rbs-core` gagne une
feature `observability`, désactivée par défaut, que le fragment ajoute au manifeste du
projet comme `auth` le fait déjà. Sous `#[cfg(feature = "observability")]`, `logs::init()`
compose sa couche d'export en même temps que son formateur.

C'est cohérent avec la frontière du projet plutôt qu'une entorse : le critère du noyau est
« ce qui n'a aucune raison de varier d'un projet à l'autre ». Un exportateur OTLP ne varie
pas — il lit un endpoint et pousse des spans — et le span qu'il exporte est déjà celui que
`rbs-core::trace` construit. À l'inverse, les métriques métier varient à chaque projet :
elles restent dans le code engendré.

## Configuration : deux sources, et pourquoi

| Réglage | Source | Raison |
|---|---|---|
| Endpoint OTLP | `OTEL_EXPORTER_OTLP_ENDPOINT` | `logs::init()` s'exécute **avant** `Config::load()` : il n'a pas de configuration à lire. Et ce nom est celui que tout collecteur et tout opérateur connaissent déjà — l'inventer autrement obligerait à traduire. |
| Nom du service | `OTEL_SERVICE_NAME` | Idem. À défaut, le nom du paquet. |
| Port de `/metrics` | `config/default.toml`, section `[observability]` | Le serveur de métriques démarre à l'ancre `startup`, la configuration en main. C'est une décision du projet, pas de son environnement. |

**Variable absente → aucun export.** Le fragment installé reste inerte tant que personne
n'a nommé de collecteur : un développeur qui lance `cargo run` sur son poste ne doit pas
voir son démarrage ralenti par un endpoint injoignable.

## Ce que le noyau apporte

Feature cargo `observability = ["dep:opentelemetry", "dep:opentelemetry_sdk",
"dep:opentelemetry-otlp", "dep:tracing-opentelemetry"]`. Les versions se relèvent par
`cargo add --dry-run` à l'implémentation ; l'écosystème OpenTelemetry bouge vite et
aucune n'est à deviner ici.

```rust
/// Vide les lots de spans encore en attente, et rend la main quand ils sont partis.
///
/// Sans feature `observability`, ne fait rien. Un arrêt brutal sans cet appel perd le
/// dernier lot : le processus meurt avant que l'exportateur ne l'ait poussé.
pub fn shutdown();
```

**Point ouvert, assumé** : rien n'appelle `shutdown()` d'office. `main.rs` n'a pas d'ancre
de fin, et lui en ajouter une pour ce seul usage la ferait réclamer par `doctor` sur tous
les projets existants. Le fragment documente l'appel dans le `README` qu'il dépose ; le
coût d'un oubli est le dernier lot de spans, pas une panne. Une ancre de fin de `main` est
une tâche à part entière, à ouvrir si un second usage se présente.

## Ce que le fragment engendre

`crates/rbs-cli/templates/features/observability/`, sur le moule des neuf autres :

- `mod.rs.jinja` → `src/observability/mod.rs` : le montage, et le `serve` du second
  listener.
- `metrics.rs.jinja` → `src/observability/metrics.rs` : le registre et le middleware.
- `config.rs.jinja` → `src/observability/config.rs` : la section `[observability]`.
- `tests.rs.jinja` → `src/observability/tests.rs`, `#[ignore]` sur ce qui exige un
  service externe, comme `jobs`, `redis` et `storage`.

Ancres employées :

| Ancre | Contenu |
|---|---|
| `features` | `pub mod observability;` |
| `layers` | le middleware de comptage |
| `startup` | `observability::serve(&config);` — le second listener, dans un `tokio::spawn` |

Le middleware va à l'ancre `layers`, donc **à l'intérieur** de `trace` et `request_id` :
il voit l'identifiant de la requête, et les réponses courtes des couches posées avant lui
— un 429 de `rate-limit`, un préflight refusé par `cors` — entrent dans ses compteurs. Une
couche posée plus bas les manquerait.

### Les métriques

Trois séries, celles qui répondent aux questions d'exploitation sans en poser de
nouvelles :

- `http_requests_total{method, path, status}` — un compteur.
- `http_request_duration_seconds{method, path}` — un histogramme.
- `http_requests_in_flight` — une jauge.

**`path` est le gabarit de route, jamais l'URL demandée** : il vient de `MatchedPath`
d'axum. `/articles/{id}` engendre une série ; `/articles/0192f3…` en engendrerait une par
article, et ferait tomber le collecteur au bout de quelques heures. Une requête qui ne
correspond à aucune route est comptée sous un `path` constant, faute de gabarit.

Le choix de la crate de registre — `metrics` + `metrics-exporter-prometheus`, ou
`prometheus-client` — se tranche à l'implémentation sur ce que `cargo add --dry-run`
montre de leur maintenance, et se justifie en une ligne dans le commit.

### Le second listener

`/metrics` n'est **jamais** monté sur le routeur public. Un serveur à part, sur
`observability.metrics_port` (défaut 9090), lancé à l'ancre `startup`. Les métriques
publient la topologie interne du service — routes, volumétrie, versions ; les exposer sur
le port de l'API demanderait à chaque déploiement une règle de reverse-proxy pour les
cacher, et un déploiement qui l'oublie fuit sans le savoir.

Le port par défaut diffère de `server.port` ; `doctor` refuse la configuration où les deux
coïncident, un `bind` qui échoue au démarrage étant plus cher à diagnostiquer qu'une
configuration refusée.

## `doctor`

Un contrôle de plus dans `FEATURE_CHECKS`, sous le nom `observability` : la section
`[observability]` est présente dans `config/default.toml`, et son port diffère de celui du
serveur. L'endpoint OTLP n'est **pas** contrôlé — son absence est un mode de
fonctionnement légitime, pas une faute.

## Tests

- **Noyau** : sans la feature, `logs::init()` se comporte exactement comme aujourd'hui —
  les tests existants de `logs` sont la non-régression. Avec la feature et sans variable
  d'environnement, `init()` réussit et n'installe aucun exportateur ; `shutdown()` sans
  `init()` ne panique pas.
- **Fragment, rendu** : le manifeste énumère ses fichiers — contrairement à `ci`, dont
  `IMPROVE.md` #69 reproche le silence ; les trois ancres reçoivent leur contenu ; la
  section de config est déposée.
- **Métriques, unitaires** : une requête sur une route à paramètre compte sous le gabarit
  et non sous l'URL. C'est le test qui garde la cardinalité.
- **Intégration** : `rbs add observability` sur un projet neuf, puis compilation. Le
  fragment tire une dizaine de crates : c'est le seul test qui prouve qu'elles
  s'accordent.
- **Documentation bilingue** : une page `docs/docs/features/observability.md` et sa
  jumelle française, dans le même commit ; la liste des features de `rbs add` est à
  reprendre partout où les neuf sont énumérées — `cli.rs`, le site, les `README`.

## Hors périmètre

Les métriques métier engendrées avec un CRUD, l'export OTLP des métriques (le fragment
publie en Prometheus, pas en OTLP), les logs corrélés au span côté collecteur, et tout
tableau de bord livré avec. Le fragment pose la matière ; ce qui la consomme appartient au
projet.
