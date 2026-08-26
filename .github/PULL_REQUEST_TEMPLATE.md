## What this changes, and why

<!-- The technical *why* — what the change makes possible, or what it stops going wrong.
     The *what* is already in the diff. -->

## Verification

<!-- The commands you ran, with their real output. This is the same rule commit bodies
     follow in this repository, and it does not stop at outside contributions. -->

```
$ cargo test --workspace
$ cargo clippy --workspace --all-targets -- -D warnings
$ cargo fmt --all --check
```

Integration tests (`cargo test --workspace -- --ignored`) need Docker and take several
minutes. If you could not run them, say so here — an announced gap is easier to work with
than a silent one.

## Checklist

- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org),
      subject in French, imperative, no trailing period
- [ ] Documentation changed in English is changed in French too, in the same commit
- [ ] No comment that paraphrases the line below it
- [ ] Public items of `rbs-core` carry a `///`

See [CONTRIBUTING.md](../CONTRIBUTING.md) for the full conventions.
