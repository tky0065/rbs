---
sidebar_position: 8
title: Seeing what the API does
---

# Seeing what the API does

This is the eighth of nine tutorials. It picks up `demo`; this page needs nothing
beyond [Setting up](./setup.md). The case: one route has gone slow, and there is no way
yet to say since when, or which one.

## What you need

Nothing beyond [Setting up](./setup.md): the same running `demo`, and `curl` again.

## 1. Install the feature

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

  5 à créer, 6 à modifier
✓ observability installée — 5 créés, 6 modifiés

  les métriques sont sur http://localhost:9090/metrics ; pour les traces, nommez un collecteur dans OTEL_EXPORTER_OTLP_ENDPOINT
```

Unlike the bricks before it, `observability` does not wait for you to wire anything: a
counting middleware lands in `// <rbs:layers>`, positioned after `trace` and
`request_id` so a request it rejects still carries a request id and reaches the log; and
a second HTTP listener, spawned from `// <rbs:startup>`, starts serving one route —
`/metrics` — before the API's own listener has even bound. `config/default.toml` gained
an `[observability]` section carrying that listener's port: `metrics_port = 9090`,
distinct from `server.port`'s `8080` on purpose, which the next section is about. Traces
are the install's other half — they leave through the core the moment
`OTEL_EXPORTER_OTLP_ENDPOINT` names a collector — but nothing below reaches that far;
this page is metrics end to end.

The separation is deliberate, and it is the whole point of the port in
`config/default.toml` not matching `server.port`: metrics publish the service's internal
topology — its routes, their volume, its version — to whoever can reach them, and
exposing that on the API's own port would mean a reverse-proxy rule at every single
deployment to keep it from leaking past the edge. Give it a listener of its own instead,
and the rule to keep it internal is a firewall one, written once.

## 2. Run the server

```bash
cargo run
```

{/* rbs:libre raison="journal d'un serveur qui tourne : le rejeu ne lance aucun serveur, et l'heure change à chaque démarrage" */}
```text
INFO   demo::modules::observability  métriques  adresse=127.0.0.1:9090
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

Proof both listeners are up before anything asks either of them anything: the metrics
line prints first — `// <rbs:startup>` runs before `router()` even builds the API's own
`Router` — and the second confirms the API itself, on the port every earlier tutorial
already used.

## Verify

From the second terminal, the app's own port first:

```bash
curl -i http://127.0.0.1:8080/metrics
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
```text
HTTP/1.1 404 Not Found
x-request-id: 01M22V0E32T6TBZPBRYZNR0NRW
content-length: 0
date: Wed, 09 Sep 2026 10:21:53 GMT
```

Proof the route was never mounted on `server.port` at all — not a permission the API
refuses, a path it does not carry. And the listener that does carry it:

```bash
curl -i http://127.0.0.1:9090/metrics
```

{/* rbs:libre raison="réponse HTTP d'un serveur lancé sur une base vivante : le rejeu ne démarre ni l'un ni l'autre, et identifiants, dates et id changent à chaque appel" */}
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

Proof of the same separation from the other side: this listener answers, in the format
Prometheus scrapes, and the response is unrelated to `curl` reading it — nothing here
counts this very request, because the middleware above is layered onto the API's
`Router` alone, never onto this one. The single series present is the `404` from a
moment ago: `<hors route>`, a fixed label rather than the literal path a scanner tried,
is what keeps an unmatched request from opening a series of its own — the property the
next section's test guards on purpose, for the routes that do exist.

## What was installed

Three extracts, read from
[`examples/newsletter-queue`](https://github.com/tky0065/rbs/tree/main/examples/newsletter-queue) —
the same command shown above, run on a project compiled in CI. Nothing here is
domain-specific: a project carrying `observability` alone writes the same three files.

### The config

`add observability` writes one new key, `metrics_port`, deliberately apart from
`server.port`: metrics expose the service's internal topology — its routes, its traffic,
its versions — and putting that on the API's own port would ask every deployment for a
reverse-proxy rule to hide it.

```toml file=examples/newsletter-queue/config/default.toml region=metriques
```

### The listener

```rust file=examples/newsletter-queue/src/modules/observability/mod.rs region=exposition
```

`bind` is the only fallible step in the whole install: a `metrics_port` that collides
with `server.port`, or with anything else already listening, fails here — before the
API's own listener has bound, and with a message naming the address that would not open.

### The cardinality guard

Every series above carries `path`, and every one of them read `<hors route>` rather than
`/metrics` or `/health` verbatim — that discipline is what a route with a parameter
depends on just as much, and it is not optional: label a series by the address a caller
actually typed, and `/articles/{id}` opens one series per article ever requested rather
than one for the route. A collector has a hard limit on how many series it can hold, and
production traffic reaches it in hours, not months, once that discipline slips.

```rust file=examples/newsletter-queue/src/modules/observability/tests.rs region=cardinalite
```

## Going further

- [Observability](../guides/observability.md) covers traces in full — the export this
  page's config never turned on, and how it composes with the subscriber the core
  installs.
- [Logs](../guides/logs.md) covers the subscriber traces graft onto, and the `pretty`
  formatter behind every line this page's server printed.
- [`rbs add`](../cli/add.md) covers the twelve other features `demo` could still
  install, `observability` now on it.
- [Calling the API from TypeScript](./typescript-client.md) is the last tutorial: a
  front end that calls `articles` through a client read from the API's own OpenAPI
  document, rather than a second copy of its types.
