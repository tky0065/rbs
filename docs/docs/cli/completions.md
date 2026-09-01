---
sidebar_position: 9
title: rbs completions
---

# `rbs completions`

Writes the completion script of the given shell to standard output, and writes nothing
else there: the script is meant to be piped into an `eval`, where a line of courtesy would
become a command to run.

The script is generated from the very declaration the parser is built on, so it never
drifts: a flag added to [`rbs generate`](./generate.md) is completable the day it is
added, without anyone remembering to update a hand-written script.

Unlike every other command on this page's siblings, this one does not read a project. It
can be run from anywhere.

:::note
rbs speaks French in its help screens and in its output. Every terminal block on this page
is verbatim, captured by running the command; only the prose around it is translated.
:::

## Synopsis

```text
$ rbs completions --help
Écrit sur la sortie standard le script de complétion du shell donné

Usage: rbs completions <SHELL>

Arguments:
  <SHELL>  Shell visé [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -h, --help     Print help
  -V, --version  Print version
```

The shell is the only argument, and it is required. The five values are those
`clap_complete` knows how to generate for; the four in daily use are documented below.

## Installing it

Each shell reads its completions from its own place. Pick your line, and open a new shell
afterwards.

Bash — source it from `~/.bashrc`:

```bash
echo 'eval "$(rbs completions bash)"' >> ~/.bashrc
```

Zsh — drop the script in a directory of `$fpath`, under the name `_rbs`:

```bash
rbs completions zsh > "${fpath[1]}/_rbs"
```

Fish:

```bash
rbs completions fish > ~/.config/fish/completions/rbs.fish
```

PowerShell — append to the profile that `$PROFILE` names:

```powershell
rbs completions powershell | Out-String | Invoke-Expression
```

Generating the script on every shell start, as the Bash line above does, costs a process
launch; writing it to a file, as the three others do, costs a regeneration after each
`cargo install rbs-cli`. Neither is wrong — the second is faster, the first can never go
stale.

## What it completes

Subcommands, their flags, and the values of every option whose values are known: the
`--database` of [`rbs new`](./new.md), the `--lang`, the shell of this very command.

One list is not in the declaration the parser uses, and is added for the completion
alone — the features [`rbs add`](./add.md) installs:

```text
$ rbs completions bash | grep -A1 'rbs__subcmd__add)'
        rbs__subcmd__add)
            opts="-h -V --force --dry-run --template-dir --help --version auth ci cors docker jobs mail rate-limit redis storage"
```

```text
$ rbs completions zsh | grep "':feature"
':feature -- Feature à installer:(auth ci cors docker jobs mail rate-limit redis storage)' \
```

The nine names come from the fragments embedded in the binary, and are the ones a shell
has no way of guessing.

They are proposed, not required. `rbs add` itself accepts a name no binary carries — that
is what `--template-dir` exists for — so the parser keeps no such list, and only the
`Command` handed to the generator receives it. A completion that refused what the command
accepts would be worse than no completion at all.

Fish and PowerShell are the two shells whose generator stops short of the values of a
positional argument: there, `rbs add ` completes the flags but not the nine names. That is
a limit of the generator, not of the declaration.

## An unknown shell

```text
$ rbs completions nushell
error: invalid value 'nushell' for '<SHELL>'
  [possible values: bash, elvish, fish, powershell, zsh]

For more information, try '--help'.
```

Exit status 2, and nothing on standard output — so an `eval` of the command evaluates
nothing rather than half a script.
