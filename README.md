# rbs

A web API framework for Rust, built on Axum and SeaORM. It gives a project the parts that
have no reason to differ from one API to the next — errors, logging, configuration,
database access, OpenAPI — and generates the rest into your own source tree, where you can
read it and change it.

*[Version française](README.fr.md).*

## Status

Version 0.1 is under construction.

**There is no semver promise before 1.0.** The public API of `rbs-core` may change between
minor versions, with no deprecation cycle and no migration path. Pin an exact version, read
the commit log before upgrading, and expect to adjust code. Freezing that API is what 1.0
means here; until then, nothing is frozen.

rbs is not published on crates.io yet either — that also comes with 1.0.

## Installation

Rust 1.85 or later. The projects you generate target PostgreSQL 18 or later.

```bash
cargo install --git https://github.com/tky0065/rbs rbs-cli
```

The package is `rbs-cli`; the binary it installs is `rbs`.

That command gives you the binary, and that is all it gives you. Until `rbs-core` is on
crates.io, a generated project needs a local copy of it to build against, which means the
repository has to be cloned rather than installed from. The
**[getting started guide](https://tky0065.github.io/rbs/getting-started)** carries that
sequence, along with the database the commands expect; follow it rather than the sketch
below, which leaves both out.

> The Ruby ecosystem ships an unrelated tool also called `rbs`. If `rbs --version` prints
> something like `rbs 3.10.0`, that one is winning on your `PATH` — use `rbs-cli`, which
> the same install put there under a name nobody else claims.

## Quick look

What rbs does, in four commands — from an empty directory to a CRUD API, with its entity,
its migration, its OpenAPI document and its integration tests:

```bash
rbs new blog-api
cd blog-api
rbs generate crud articles --fields 'title:string,body:text,published:bool'
rbs migrate up
```

This is the shape of the thing, not a transcript to paste: 0.1 needs two more arguments
than are shown here, and a database answering. The
[getting started guide](https://tky0065.github.io/rbs/getting-started) has the runnable
version, with the output of every command.

## What rbs gives you

`rbs-core` is the runtime: typed errors rendered as RFC 9457 problem documents, a log
formatter that stays readable in development and turns to JSON in production, configuration
validated at boot, pagination, application state. It carries what has no reason to vary
from one project to the next.

The `rbs` command writes everything else into your own project — model, DTO, repository,
service, controller, migration, tests — as plain Rust, with no macro to unfold.

That generated code is meant to be edited. Nothing in it carries a "generated, do not edit"
banner, because nothing regenerates over your changes.

Features depend in one direction only: `controller → service → repository → model`. Here is
the whole of the `POST /articles` handler, as `rbs generate crud` writes it, read from
[`examples/hello-crud/src/articles/controller.rs`](examples/hello-crud/src/articles/controller.rs):

```rust
#[utoipa::path(
    post,
    path = "/articles",
    tag = "articles",
    request_body = CreateArticle,
    responses(
        (status = 201, description = "article créé", body = ArticleResponse),
        (status = 400, description = "corps illisible", body = ProblemDetails, content_type = "application/problem+json")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ValidatedJson(entree): ValidatedJson<CreateArticle>,
) -> Result<(StatusCode, Json<ArticleResponse>)> {
    let article = service::create(state.core().db(), entree).await?;

    Ok((StatusCode::CREATED, Json(article)))
}
```

The controller hands the request to the service and maps the result to a status code — the
service, in turn, never sees a `DatabaseConnection`.

## Documentation

The site is at **<https://tky0065.github.io/rbs/>**: getting started, architecture, CLI
reference, guides. The binary carries five commands — `new`, `add`, `generate`, `migrate`,
`doctor` — and the site documents each one.

[`ROADMAP.md`](ROADMAP.md) lists what 0.1 covers, what the later milestones add, and what is
deliberately left out.

## Contributing

Start with [`CONTRIBUTING.md`](CONTRIBUTING.md): what you need installed, the checks CI runs,
and the conventions the repository holds itself to. Working on the Rust code never requires
Node. The project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
