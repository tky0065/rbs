---
sidebar_position: 1
title: Logs
---

# Logs

rbs ships two log formatters: `pretty`, meant to be read by a human during development,
and `json`, meant to be parsed by a log collector. `RBS_LOG_FORMAT` picks between them —
`pretty` is the default — and `RUST_LOG` filters, as it does anywhere else in the Rust
ecosystem.

## The `pretty` formatter

`tracing-subscriber` ships its own formatter. rbs does not use it: it prints more than a
developer reads. `pretty` gives one line per event, with columns that stay put from one
line to the next, so the eye finds the level and the message without scanning.

![Output of the pretty formatter across the five levels](/img/logs-pretty.png)

Left to right: a short timestamp, the level, the target, the message, and the event's
fields — followed by the fields of any enclosing span, as on the `ERROR` line above,
which carries the `request_id` of the span it was emitted in.

Levels are colored: `TRACE` grey, `DEBUG` blue, `INFO` green, `WARN` yellow, `ERROR` red.
Fields and target are dimmed, so the message keeps the reader's attention. **Colors are
dropped when the output is not a terminal** — a redirected log file holds no escape
sequences.

## Emitting events

Nothing about the formatter is specific to rbs: you emit with the `tracing` macros.

```rust file=crates/rbs-core/examples/logs_pretty.rs region=niveaux
```

## Installing it by hand

`rbs_core::logs::init()` reads `RBS_LOG_FORMAT` and installs the right formatter — a
generated project calls it at startup and needs nothing else. To install `pretty`
unconditionally, on a test harness or a one-off binary, build the subscriber yourself:

```rust file=crates/rbs-core/examples/logs_pretty.rs region=installation
```

Run this example to judge the rendering on your own terminal:

```bash
cargo run -p rbs-core --example logs_pretty
```

The image above is regenerated from that very command, never edited by hand:

```bash
python3 docs/scripts/capture_logs_pretty.py
```
