---
sidebar_position: 4
title: Taking a file
---

# Taking a file

This is the fourth of nine tutorials. It picks up `demo`; this page needs nothing beyond
[Setting up](./setup.md). The case: a client deposits a justificatif — a receipt, a
paper to keep on file — and by the end of this page, `PUT /uploads/{id}/content` stores
it and answers `204`.

## What you need

Nothing beyond [Setting up](./setup.md): the same `demo`, still running with its
database, and `curl` again.

## 1. Install the feature

```bash
rbs add storage
```

{/* rbs:transcript cmd="rbs add storage" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="demo" */}
```text
$ rbs add storage
storage : stockage d'objets : un trait à cinq méthodes, deux backends — fichiers et S3

plan pour …/demo

  + src/modules/storage/mod.rs     créé
  + src/modules/storage/files.rs   créé
  + src/modules/storage/s3.rs      créé
  + src/modules/storage/tests.rs   créé
  + src/modules/mod.rs             créé
  ~ src/lib.rs                     modifié
  ~ src/state.rs                   modifié
  ~ src/health/controller.rs       modifié
  ~ Cargo.toml                     modifié
  ~ config/default.toml            modifié
  ~ .env.example                   modifié
  ~ AGENTS.md                      modifié

  12 fichiers à écrire
✓ storage installée — 4 fichiers

  les objets vont sous ./storage : ajoutez-le à .gitignore, ou passez storage.backend à "s3" et recopiez les RBS_STORAGE__* de .env.example
```

Like every brick `rbs add` installs, `storage` mounts no route — it hands you an
`Arc<dyn Storage>` on `AppState` and leaves deciding when to call it to you. `add mail`
and `add redis` are not run on this page: `demo` gets storage alone. The example this
page reads from, [`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop),
carries all three — which is why its service holds calls this page never shows.

## 2. Generate the resource

`generate` needs a clean tree, same as `add` did — commit what `storage` just wrote,
then turn the trait above into a resource that uses it:

```bash
git add -A && git commit -q -m "storage installée"
rbs generate crud uploads --fields "title:string,owner_email:string,content_type:string,size:int" --with-upload
```

```text
plan pour …/demo

  + src/uploads/mod.rs                                 créé
  + src/uploads/model.rs                               créé
  + src/uploads/dto.rs                                 créé
  + src/uploads/filter.rs                              créé
  + src/uploads/repository.rs                          créé
  + src/uploads/service.rs                             créé
  + src/uploads/controller.rs                          créé
  + src/uploads/tests.rs                                créé
  + src/seeds/uploads.rs                               créé
  + migration/src/m20260909_094423_create_uploads.rs   créé
  ~ src/lib.rs                                         modifié
  ~ src/router.rs                                      modifié
  ~ src/openapi.rs                                      modifié
  ~ migration/src/lib.rs                               modifié
  ~ src/seeds/main.rs                                   modifié
  ~ Cargo.toml                                          modifié
  ~ AGENTS.md                                           modifié

  17 fichiers à écrire
✓ uploads générée — 10 fichiers

  la migration m20260909_094423_create_uploads reste à appliquer avant de lancer le projet
```

Two things worth naming here. `--with-upload` is what wrote the three routes on
`/uploads/{id}/content` — `PUT`, `GET`, `HEAD` — against the trait `storage` installed a
moment ago; drop the flag and `generate crud` would have produced the same six operations
as [Your first resource](./first-resource.md) and nothing under `/content` at all. And
`owner_email` — an ordinary field in `--fields`, no special syntax — earned an `email`
constraint in the generated DTO for the one reason that its name ends in `_email`;
nothing in the command asked for it.

## 3. Apply the migration and run the server

```bash
rbs migrate up
```

```text
   Compiling migration v0.1.0 (…/demo/migration)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 35.44s
     Running `target/debug/migration up`
✓ migrations appliquées
```

Proof the table now exists and the binary is ready to serve it: the migration
`generate` wrote a moment ago is applied, which is what lets the server below start
against it.

```bash
cargo run
```

```text
INFO   demo                démarrage  adresse=127.0.0.1:8080
```

A clean start is proof the three content handlers `--with-upload` wrote compile against
the storage trait `add storage` installed — if the wiring were off, this line would never
print.

## Verify

From the second terminal, try a bad address first:

```bash
curl -i -X POST http://127.0.0.1:8080/uploads \
  -H 'Content-Type: application/json' \
  -d '{"title":"Justificatif de domicile","owner_email":"pas-un-email","content_type":"application/pdf","size":48213}'
```

```text
HTTP/1.1 422 Unprocessable Entity
content-type: application/problem+json
x-request-id: 01M22SF8V7N4F6RP5DQWPQCBVP
content-length: 143
date: Wed, 09 Sep 2026 09:55:02 GMT

{"type":"about:blank","title":"Validation échouée","status":422,"errors":{"owner_email":["email"]},"request_id":"01M22SF8V7N4F6RP5DQWPQCBVP"}
```

Proof that `_email` earns its constraint without anyone declaring it: nothing in
`--fields` said "validate this column", and the DTO refused it anyway. The same body,
with a real address:

```bash
ID=$(curl -s -X POST http://127.0.0.1:8080/uploads \
  -H 'Content-Type: application/json' \
  -d '{"title":"Justificatif de domicile","owner_email":"alice@example.com","content_type":"application/pdf","size":48213}' \
  | jq -r .id)
```

The row exists; its content does not yet:

```bash
curl -i -I http://127.0.0.1:8080/uploads/$ID/content
```

```text
HTTP/1.1 404 Not Found
content-type: application/problem+json
x-request-id: 01M22SFC21MJ86CC06CZC804JP
content-length: 130
date: Wed, 09 Sep 2026 09:55:05 GMT
```

Proof that a resource and its content are two separate things: `create` above wrote the
row, and nothing under `/content` exists until something is deposited there. Now the
deposit itself, the case this page opened with:

```bash
curl -i -X PUT http://127.0.0.1:8080/uploads/$ID/content \
  -H 'Content-Type: application/octet-stream' \
  --data-binary 'Justificatif de domicile, PDF simulé pour la démo du tutoriel.'
```

```text
HTTP/1.1 204 No Content
x-request-id: 01M22SFC29MWP1REHVJBA2B7F3
date: Wed, 09 Sep 2026 09:55:05 GMT
```

Proof the deposit landed: the same `HEAD` that answered `404` a moment ago now answers
without one:

```bash
curl -i -I http://127.0.0.1:8080/uploads/$ID/content
```

```text
HTTP/1.1 204 No Content
x-request-id: 01M22SFC2PJTX8YFDNPH9QYJY8
content-length: 0
date: Wed, 09 Sep 2026 09:55:05 GMT
```

Proof that presence and absence read differently even through `HEAD` alone:
`content-length` drops from `130` — the size of the JSON problem a `GET` would have
returned a moment ago — to a flat `0`, so a caller can tell the two apart from the
header, without ever fetching a body. And reading it back:

```bash
curl -i http://127.0.0.1:8080/uploads/$ID/content
```

```text
HTTP/1.1 200 OK
content-type: application/octet-stream
x-request-id: 01M22SFC2X607FJFHR3X8A4FB4
content-length: 64
date: Wed, 09 Sep 2026 09:55:05 GMT

Justificatif de domicile, PDF simulé pour la démo du tutoriel.
```

Proof the round trip holds: the bytes `PUT` sent are the bytes `GET` returns, byte for
byte, through the store `storage` chose — `fs` here, `s3` on a different
`storage.backend`, with no other line on this page changing.

## What was installed

Three files, read from
[`examples/file-drop`](https://github.com/tky0065/rbs/tree/main/examples/file-drop) — the
same two commands, run in the same order, on a project compiled in CI.

:::note
`file-drop` carries all three v0.3 bricks — `storage`, `mail` and `redis` — on one
project, so its `src/uploads/service.rs` also sends a mail on creation and caches the
list, neither of which this page installed. The `contenu` extract below is the part of
that file storage alone accounts for.
:::

### The trait

Five methods are the whole contract `add storage` promises, whichever backend answers
them.

```rust file=examples/file-drop/src/modules/storage/mod.rs region=trait
```

### The write handler

`put_content` is what `--with-upload` wrote: the body arrives as raw bytes, never as
JSON, and is handed straight to the service.

```rust file=examples/file-drop/src/uploads/controller.rs region=put_content
```

### The service

The row is read before the deposit, so the store never accumulates an object no
resource claims — and `exists` answers the `HEAD` request rather than a `get` whose body
would be thrown away.

```rust file=examples/file-drop/src/uploads/service.rs region=contenu
```

## Going further

- [Storage](../guides/storage.md) covers both backends, the key-escaping rule this page
  never triggered, and everything `--with-upload` leaves to you — size limits, MIME
  filtering, listing.
- [`rbs add`](../cli/add.md) covers the twelve other features this project could still
  install.
- [`rbs generate`](../cli/generate.md) has the full grammar of `--with-upload`, including
  how it combines with `--role` and `--soft-delete`.
- [Testing](../guides/testing.md) is the harness `uploads/tests.rs` runs against, and
  the `storage` fragment's own `round` test this page's extracts never open.
- [Sending mail](./mail.md) is the next tutorial: the moment this page's deposit
  succeeds, its owner gets a mail confirming it.
