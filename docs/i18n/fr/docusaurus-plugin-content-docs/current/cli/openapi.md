---
sidebar_position: 2.7
title: rbs openapi export
---

# `rbs openapi export`

Écrit le document OpenAPI du projet, sans démarrer de serveur : sur la sortie standard, ou
dans un fichier avec `--out`.

:::note
rbs parle français dans ses écrans d'aide et dans ses sorties. Tous les blocs de terminal
de cette page sont verbatim, capturés en lançant la commande.
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

| Drapeau | Effet |
|---|---|
| `--out <FICHIER>` | Écrit le document dans ce fichier, relatif au répertoire d'où la commande est lancée, et affiche à la place une ligne de confirmation. Sans lui, le document part sur la sortie standard, et rien d'autre. |

## D'où vient le document

Du même endroit que pour [`rbs generate client`](./client.md), par le même code :
`rbs new` écrit un troisième binaire, `src/bin/openapi.rs`, qui imprime ce que
`ApiDoc::openapi()` rend. `rbs openapi export` lance `cargo run --quiet --bin openapi` à la
racine du projet. La compilation du projet part sur la sortie d'erreur, pour que la sortie
standard ne porte que le document — `rbs openapi export > openapi.json` et `--out
openapi.json` écrivent les mêmes octets.

Le texte est analysé avant d'être écrit. Un binaire retouché pour imprimer autre chose
laisserait sinon un fichier nommé comme un contrat, que le premier outil à le lire
refuserait.

## Le contrat mémorisé

Ce `cargo run` est une compilation complète en profil dev d'un projet Axum + SeaORM +
utoipa — de l'ordre de la minute sur cible froide, et les trois commandes qui lisent le
contrat se tapent d'ordinaire l'une après l'autre. Le document est donc mémorisé sous
`target/rbs/openapi.json`, à côté du condensat SHA-256 des sources qui l'ont produit,
`target/rbs/openapi.sha256`. Sources inchangées, réponse immédiate.

Le condensat couvre le chemin **et** le contenu de chaque fichier de `src/` et de
`migration/src/`, plus `Cargo.lock`. Un fichier retouché, renommé ou supprimé le change,
et une montée de dépendance aussi — elle déplace le contrat sans toucher une ligne du
projet. `target/` est déjà ignoré par git et déjà effacé par `cargo clean`, et c'est le
motif de cet emplacement plutôt qu'un répertoire à soi : il n'y a rien de nouveau à
apprendre à nettoyer.

Le cache ne fait jamais échouer une commande : un `target/` non inscriptible, un document
tronqué, une source illisible se soldent tous par une simple recompilation.

`rbs openapi export` n'a délibérément pas de `--from` : c'est la commande qui *produit* le
contrat, et lire un fichier pour en écrire un autre la réduirait à une copie. Quand il faut
contourner le contrat mémorisé, ce qu'on veut est une recompilation — `rm -rf target/rbs`,
ou `cargo clean` — et non un fichier à lire.

## Figer le contrat

Le document est ce sur quoi s'appuient un client, une passerelle ou une autre équipe. Le
commiter et le vérifier en CI fait d'un changement de contrat involontaire un build rouge :

```bash
rbs openapi export --out openapi.json
git diff --exit-code openapi.json
```

## Échecs

Les deux refus de [`rbs generate client`](./client.md), mot pour mot, avec les mêmes
remèdes : un projet sans `src/lib.rs`, dont l'`ApiDoc` vit dans le binaire principal où un
second binaire ne peut pas l'atteindre ; un projet sans `src/bin/openapi.rs`, pour lequel
le remède imprime le fichier à créer et l'entrée `[[bin]]` à déclarer. Tous deux sont
refusés avant que cargo ne soit lancé. Un projet qui ne compile pas s'arrête sur
`` `cargo run --bin openapi` a échoué (code …) : le projet ne compile pas ``, les erreurs du
compilateur au-dessus.

[`rbs routes`](./routes.md) lit le même document et en énumère les opérations ; elle
comme [`rbs generate client`](./client.md) prennent un `--from <FICHIER>` qui relit un
contrat que cette commande-ci a déjà figé.
