---
sidebar_position: 1.5
title: Observability
---

# Observability

Logs answer *what happened*. `rbs add observability` answers the two questions that come
next: *which route is slow, since when* — from metrics — and *on which downstream call* —
from traces. It installs four files under `src/observability/`, an `[observability]`
config section, a counting middleware, and a second HTTP listener that serves `/metrics`.

```bash
rbs add observability
```

## Traces leave through the core, not through the fragment

`rbs_core::logs::init()` is the first line of a generated `main`, and it installs the
global subscriber itself. `tracing`'s `set_global_default` may be called once per process,
and the `// <rbs:startup>` anchor runs after it — so nothing installed by a fragment could
graft an export layer onto that subscriber.

The graft therefore lives in the core, behind a cargo feature the fragment enables:

```toml
rbs-core = { version = "1.1", features = ["observability"] }
```

With that feature on, `logs::init()` composes an OTLP export layer alongside the formatter
it already installs. The span it exports is the one `rbs_core::trace` already builds per
request — the same one your logs correlate on.

Two environment variables drive it, and they are the ones every collector already knows:

| Variable | Effect |
|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | The collector, over OTLP/gRPC — `http://localhost:4317` for a local one. |
| `OTEL_SERVICE_NAME` | The service name carried by exported traces. Defaults to the running binary's name. |

They are read from the environment rather than from `config/default.toml` because
`logs::init()` runs *before* `Config::load()`: at that point there is no configuration to
read.

**No endpoint means no export.** The fragment sits inert until someone names a collector,
so `cargo run` on a laptop is not slowed down by an exporter dialling an address that
answers nothing.

### Flushing the last batch

Spans are exported in batches. A process that dies between two batches takes the last one
with it, so call this before returning from `main`:

```rust
rbs_core::logs::shutdown();
```

Without the `observability` feature it does nothing, and calling it when nothing was ever
installed is not an error. The cost of forgetting it is that last batch — not an outage,
which is why nothing in the skeleton calls it for you.

## Metrics: three series, and one label that decides everything

The middleware goes into `// <rbs:layers>`, inside `trace` and `request_id`. That position
is what lets it see the request id, and what makes the short responses of the layers above
it — a 429 from `rate-limit`, a preflight refused by `cors` — land in its counters. A layer
mounted lower would miss them, and the published error rate would be wrong.

| Series | Type | Labels |
|---|---|---|
| `http_requests_total` | counter | `method`, `path`, `status` |
| `http_request_duration_seconds` | histogram | `method`, `path` |
| `http_requests_in_flight` | gauge | — |

**`path` is the route template, never the URL that was requested.** It comes from axum's
`MatchedPath`, so `/articles/{id}` is one series; the requested URL would be one series per
article, and a collector falls over within hours under that. A request matching no route at
all is counted under a single constant, for the same reason: a scanner hitting a thousand
made-up paths opens one series, not a thousand.

This is the constraint the whole module is built around, and the generated
`src/observability/tests.rs` holds it:

```text
$ cargo test observability
test observability::tests::a_request_on_a_parameterised_route_counts_under_its_template ... ok
test observability::tests::an_unmatched_path_counts_under_a_single_constant ... ok
```

## `/metrics` gets its own port

```toml
[observability]
metrics_port = 9090
```

`/metrics` is never mounted on the public router. Metrics publish the internal topology of
a service — its routes, its volumes, its versions; putting them on the API's port would
require a reverse-proxy rule at every deployment to hide them, and a deployment that
forgets that rule leaks without knowing it. A second listener, on a port of its own, has
nothing to forget.

It listens on the same host as the API — `server.host` — so an interface is chosen where
the API chooses its own, not in a second key that could contradict it. The default port,
9090, differs from `server.port`; [`rbs doctor`](../cli/doctor.md) refuses a configuration
where the two coincide, a `bind` failing at startup being more expensive to diagnose than a
configuration refused before it.

Point Prometheus at it and nothing else is needed:

```yaml
scrape_configs:
  - job_name: mon-api
    static_configs:
      - targets: ["localhost:9090"]
```

## What the fragment does not do

Business metrics are yours: the `metrics` facade is a dependency of your project, so
`metrics::counter!("orders_total").increment(1)` works from any layer, with no registry to
thread through `AppState`. Metrics are published in Prometheus format, not exported over
OTLP. Nothing ships a dashboard. The fragment lays down the material; what consumes it
belongs to the project.
