---
sidebar_position: 9
title: rbs completions
---

# `rbs completions`

Écrit sur la sortie standard le script de complétion du shell donné, et n'y écrit rien
d'autre : le script est fait pour être passé à un `eval`, où une ligne de courtoisie
deviendrait une commande à exécuter.

Le script est engendré depuis la déclaration même sur laquelle le parseur est bâti, si
bien qu'il ne dérive jamais : un drapeau ajouté à [`rbs generate`](./generate.md) se
complète le jour où il est ajouté, sans que personne ait à se souvenir de mettre à jour un
script écrit à la main.

Contrairement à toutes les commandes voisines de cette page, celle-ci ne lit aucun projet.
Elle se lance de n'importe où.

:::note
Les blocs de terminal de cette page sont des sorties réelles, capturées en lançant la
commande. Elles sont identiques à celles de la page anglaise : le CLI parle français, une
sortie de terminal ne se traduit pas.
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

Le shell est le seul argument, et il est obligatoire. Les cinq valeurs sont celles pour
lesquelles `clap_complete` sait engendrer ; les quatre d'usage courant sont documentées
ci-dessous.

## L'installer

Chaque shell lit ses complétions à un endroit qui lui est propre. Prenez votre ligne, puis
ouvrez un nouveau shell.

Bash — à sourcer depuis `~/.bashrc` :

```bash
echo 'eval "$(rbs completions bash)"' >> ~/.bashrc
```

Zsh — le script se dépose dans un répertoire de `$fpath`, sous le nom `_rbs` :

```bash
rbs completions zsh > "${fpath[1]}/_rbs"
```

Fish :

```bash
rbs completions fish > ~/.config/fish/completions/rbs.fish
```

PowerShell — à ajouter au profil que `$PROFILE` désigne :

```powershell
rbs completions powershell | Out-String | Invoke-Expression
```

Engendrer le script à chaque démarrage de shell, comme le fait la ligne Bash ci-dessus,
coûte le lancement d'un processus ; l'écrire dans un fichier, comme font les trois autres,
coûte une régénération après chaque `cargo install rbs-cli`. Aucune des deux n'est fautive
— la seconde est plus rapide, la première ne peut jamais se périmer.

## Ce qu'il complète

Les sous-commandes, leurs drapeaux, et les valeurs de toute option dont les valeurs sont
connues : le `--database` de [`rbs new`](./new.md), le `--lang`, le shell de cette
commande-ci.

Une liste ne figure pas dans la déclaration dont le parseur se sert, et n'est ajoutée que
pour la complétion — les features qu'installe [`rbs add`](./add.md) :

```text
$ rbs completions bash | grep -A1 'rbs__subcmd__add)'
        rbs__subcmd__add)
            opts="-h -V --force --dry-run --template-dir --help --version audit auth ci cors docker jobs mail observability rate-limit redis storage"
```

```text
$ rbs completions zsh | grep "':feature"
':feature -- Feature à installer:(audit auth ci cors docker jobs mail observability rate-limit redis storage)' \
```

Les onze noms viennent des fragments embarqués dans le binaire, et sont ceux qu'un shell
n'a aucun moyen de deviner.

Ils sont proposés, non exigés. `rbs add` accepte lui-même un nom qu'aucun binaire ne
porte — c'est la raison d'être de `--template-dir` — si bien que le parseur ne garde
aucune liste de ce genre, et que seule la `Command` remise au générateur la reçoit. Une
complétion qui refuserait ce que la commande accepte vaudrait moins que pas de complétion
du tout.

Fish et PowerShell sont les deux shells dont le générateur s'arrête avant les valeurs d'un
argument positionnel : là, `rbs add ` complète les drapeaux mais pas les onze noms. C'est
une limite du générateur, non de la déclaration.

## Un shell inconnu

```text
$ rbs completions nushell
error: invalid value 'nushell' for '<SHELL>'
  [possible values: bash, elvish, fish, powershell, zsh]

For more information, try '--help'.
```

Code de sortie 2, et rien sur la sortie standard — un `eval` de la commande n'évalue donc
rien, plutôt qu'un demi-script.
