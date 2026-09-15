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

[`rbs routes`](./routes.md) lit le même document et en énumère les opérations.
