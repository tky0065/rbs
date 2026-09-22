---
sidebar_position: 2.7
title: rbs openapi export
---

# `rbs openapi export`

Writes the project's OpenAPI document, without starting a server: on standard output, or
into a file with `--out`.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

{/* rbs:transcript cmd="rbs openapi export --help" */}
```text
$ rbs openapi export --help
Écrit le document OpenAPI du projet sur la sortie standard, ou dans un fichier

Utilisation : rbs openapi export [OPTIONS]

Options :
      --out <FICHIER>  Fichier à écrire, relatif au répertoire courant, au lieu de la sortie standard
  -h, --help           Affiche l'aide
  -V, --version        Affiche la version
```

| Flag | Effect |
|---|---|
| `--out <FICHIER>` | Writes the document into this file, relative to the directory the command runs from, and prints a one-line confirmation instead. Without it, the document goes to standard output and nothing else does. |

## Where the document comes from

The same place [`rbs generate client`](./client.md) reads it from, through the same code:
`rbs new` writes a third binary, `src/bin/openapi.rs`, which prints what
`ApiDoc::openapi()` returns. `rbs openapi export` runs `cargo run --quiet --bin openapi` at
the project root. The project's compilation goes to standard error, so that standard
output carries the document alone — `rbs openapi export > openapi.json` and `--out
openapi.json` write the same bytes.

The text is parsed before it is written. A binary edited to print something else would
otherwise leave a file named like a contract that the first tool to read it rejects.

## The memorised contract

That `cargo run` is a full debug build of an Axum + SeaORM + utoipa project — around a
minute on a cold target, and the three commands that read the contract are usually typed
one after another. So the document is memorised under `target/rbs/openapi.json`, next to
the SHA-256 digest of the sources that produced it, `target/rbs/openapi.sha256`. Unchanged
sources, and the answer is immediate.

The digest covers the path **and** the content of every file under `src/` and
`migration/src/`, plus `Cargo.lock`. A file edited, renamed or deleted changes it, and so
does a dependency bump that moves the contract without touching a line of the project.
`target/` is already ignored by git and already erased by `cargo clean`, which is the
reason for that location rather than a directory of its own — there is nothing new to learn
to clean up.

The cache never fails a command: an unwritable `target/`, a truncated document, a source
that cannot be read all end in a plain recompilation.

`rbs openapi export` deliberately has no `--from`: it is the command that *produces* the
contract, and reading one file to write another would reduce it to a copy. When the
memorised contract has to be bypassed, what is needed is a rebuild — `rm -rf target/rbs`,
or `cargo clean` — not a file to read.

## Freezing the contract

The document is what a client, a gateway or another team relies on. Committing it and
checking it in CI turns an accidental contract change into a red build:

```bash
rbs openapi export --out openapi.json
git diff --exit-code openapi.json
```

## Failures

The two refusals of [`rbs generate client`](./client.md), word for word, with the same
remedies: a project without `src/lib.rs`, whose `ApiDoc` lives in the main binary where a
second binary cannot reach it; a project without `src/bin/openapi.rs`, for which the
remedy prints the file to create and the `[[bin]]` entry to declare. Both are refused before
cargo runs. A project that does not compile stops on
`` `cargo run --bin openapi` a échoué (code …) : le projet ne compile pas ``, with the
compiler's own errors above it.

[`rbs routes`](./routes.md) reads the same document and lists its operations, and both
it and [`rbs generate client`](./client.md) take `--from <FILE>` to read a contract this
command has already frozen.
