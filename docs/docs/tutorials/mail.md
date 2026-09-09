---
sidebar_position: 5
title: Sending mail
---

# Sending mail

This is the fifth of nine tutorials. It picks up `demo` right after [Taking a
file](./storage.md) — running, with the `uploads` resource and its content routes from
that page. The case: the moment `create` succeeds, whoever owns `owner_email` gets a mail
telling them their file is on file — an accusé de réception for the deposit the previous
page wired.

## What you need

Nothing beyond [Taking a file](./storage.md): the same running `demo`, with `uploads`
and its content routes mounted.

## 1. Install the feature

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

  14 fichiers à écrire
✓ mail installée — 6 fichiers

  réglez [mail] dans config/default.toml — un SMTP local par défaut
```

Like `storage` before it, `mail` mounts no route: what you get is a `Mailer` on
`AppState`, and calling it — on creation, on confirmation, on whatever else your domain
decides — is work you write yourself. `add storage` and `add redis` are not run again
here; [`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop)
carries both alongside `mail`, wired into the one service this page reads from below.

The install also writes `RBS_MAIL__SMTP_PASSWORD=` into `.env.example` — empty, and only
there, never into `.env`. That is not the random secret `auth` draws for its signing key;
it is a placeholder, left for you to fill in once you have a real account to send from.
What the new `[mail]` section of `config/default.toml` already carries is a working
default for development: `smtp_host = "localhost"`, port `1025`, no credentials — the
address of the `mailpit` service the install just added to `docker-compose.yml`.
`docker compose up -d` now brings it up alongside the database, and its inbox answers on
`:8025`.

**Read the trade-off before wiring a send.** `file-drop`'s `create` sends the accusé
detached: the HTTP response does not wait for it, and a failed send leaves nothing but a
line in the log — no queue, no retry, the message is simply gone. That is a fair price
for a notification nobody is blocked on. It is not a fair price for a password reset, or
anything else the caller is actively waiting to receive and cannot get any other way —
mail whose loss must be recoverable belongs in a queue that survives the process, which
is a different feature and a different trade-off than the one this page installs.

## Verify

`generate crud` has no reason to know a mail brick exists, so nothing wires `Mailer`
into a handler for you — the call below, `notify`, is `file-drop` code, read further
down this page. What `cargo test` can check without a line of your own is the property
this page is actually about: does a detached send return before the message is sent?

```bash
cargo test modules::mail::
```

```text
running 7 tests
test modules::mail::tests::an_invalid_sender_stops_the_build_naming_it ... ok
test modules::mail::tests::the_message_carries_the_configured_sender_and_its_recipient ... ok
test modules::mail::tests::a_missing_template_names_the_file_without_panicking ... ok
test modules::mail::tests::the_rendered_template_carries_the_variables_passed_to_it ... ok
test modules::mail::tests::send_detached_returns_without_awaiting_the_send ... ok
test modules::mail::tests::the_three_encryption_modes_build_a_transport ... ok
test modules::mail::tests::a_templated_message_goes_out_to_the_smtp_server ... ignored, joint le serveur SMTP de la section [mail]

test result: ok. 6 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Proof of the trade-off itself: `send_detached_returns_without_awaiting_the_send` opens a
listener that accepts a connection and never answers, then asserts the call returns in
under 200ms *and* that the connection was made — a send that actually waited would hang
on that fake server, and an empty method body would never have connected at all. The
ignored test is the one honest live check: it needs the Mailpit `docker compose up -d`
just started, and `cargo test -- --ignored` is what runs it.

## What was installed

Three files, read from
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop) — the
one `add mail` command above, run on a project compiled in CI.

:::note
`file-drop` carries all three v0.3 bricks — `storage`, `mail` and `redis` — on one
project, so its `src/uploads/service.rs` also deposits and reads content, and caches the
list, neither of which this page installed. The `notify` extract below is the part of
that file mail alone accounts for.
:::

### The common case

`send_template` renders a gabarit and sends the result in one call — this is what an
awaited send looks like, the one a caller can afford to wait for.

```rust file=examples/file-drop/src/modules/mail/service.rs region=send_template
```

### The detached send

```rust file=examples/file-drop/src/modules/mail/service.rs region=send_detached
```

### The call site

`notify` is what `file-drop`'s `create` calls after the row is written: it spawns its
own task, and an error from the send goes to the log — never back to the HTTP response
that has already answered `201`.

```rust file=examples/file-drop/src/uploads/service.rs region=notify
```

## Going further

- [Mail](../guides/mail.md) covers the transport, the templates, and moving a send into
  a queue when the trade-off above is the wrong one.
- [`rbs add`](../cli/add.md) covers the eleven other features `demo` could still
  install, `storage` and `mail` now both on it.
- [Testing](../guides/testing.md) is the harness the generated `mail/tests.rs` runs
  against, and what `-- --ignored` reaches that a plain `cargo test` does not.
