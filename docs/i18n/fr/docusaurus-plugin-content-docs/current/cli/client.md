---
sidebar_position: 2.5
title: rbs generate client
---

# `rbs generate client`

Écrit un client TypeScript typé depuis le document OpenAPI du projet lui-même. Une méthode
par opération, une interface par schéma, et aucune dépendance à installer côté TypeScript.

```bash
rbs generate client --lang ts
```

Le client atterrit dans `clients/ts/client.ts`. Régénérez-le après chaque changement de
contrat plutôt que de le retoucher : la commande refuse d'écraser un fichier modifié, et
`--force` lève ce refus.

## D'où vient le document

**Aucun serveur ne tourne.** `rbs new` écrit un troisième binaire, `src/bin/openapi.rs`, qui
imprime ce que rend `ApiDoc::openapi()` ; `generate client` lance `cargo run --bin openapi`
dans le projet et lit sa sortie standard.

C'est ce qui fait suivre le code au client, et non une lecture approximative des sources :
le document porte les routes que vos fragments ont montées, les DTO que vos `--fields` ont
produits, et l'`operationId` de chaque handler — y compris ceux que vous avez écrits à la
main.

Le binaire vaut par lui-même. Figer le contrat en CI, c'est un `cargo run --bin openapi >
openapi.json` suivi d'un `git diff` qui doit rester vide.

## Les drapeaux

| Drapeau | Effet |
|---|---|
| `--lang <LANGAGE>` | **Requis.** `ts` en est aujourd'hui la seule valeur. Aucun défaut : le jour où un second langage arrive, aucune invocation existante ne change de sens. |
| `--out <DIR>` | Répertoire de sortie, relatif à la racine du projet. Le nom du fichier ne change pas — c'est celui que le client porte dans un import. |
| `--force` | Écrit même si le working tree Git est sale, et écrase un client signalé en conflit. |
| `--dry-run` | Affiche le plan et s'arrête. rbs n'écrit rien — mais le projet est tout de même compilé, puisque c'est ainsi que le document se lit. |

## À quoi ressemble le client

Une classe configurable plutôt que des fonctions libres : le jeton se pose une fois, à la
construction, au lieu d'être enfilé dans chaque appel.

```ts file=examples/hello-crud/clients/ts/client.ts region=options
```

`headers` accepte une fonction autant qu'un objet, et c'est ce qui rend un jeton tournant
praticable — elle est appelée à chaque requête. `fetch` est injectable pour la raison qu'un
test en a besoin.

```ts file=examples/hello-crud/clients/ts/client.ts region=classe
```

Puis une méthode par opération, nommée d'après son `operationId` en camelCase :

```ts file=examples/hello-crud/clients/ts/client.ts region=methodes
```

Les paramètres de chemin viennent en premier, puis le corps, puis la query — et une query
dont tous les champs sont optionnels reçoit un défaut, si bien qu'`articlesList()` se passe
d'argument.

## Les erreurs

Toute réponse hors 2xx jette une `ApiError` portant le statut, le corps analysé et — quand
ce corps est un problème RFC 9457 — un `problem` typé. `rbs-core` rend toutes ses erreurs
sous cette forme, donc `error.problem?.title` est le message que votre API a réellement
envoyé.

```ts
try {
  await api.articlesCreate({ title: "", body: "…", published: false });
} catch (error) {
  if (error instanceof ApiError && error.status === 422) {
    console.error(error.problem?.errors);
  }
}
```

Cet exemple-là est écrit à la main : il montre comment *employer* le client, et aucun
fichier d'`examples/` ne l'appelle.

## Régénérer

Le client est projeté comme une création : une seconde passe sur un contrat inchangé rend
`· clients/ts/client.ts inchangé` et n'écrit rien. Un client que vous avez modifié revient
en conflit plutôt que d'être écrasé en silence :

```text
  ! clients/ts/client.ts   conflit — relancer avec --force
```

C'est le moment de sortir votre propre code du fichier engendré, plutôt que d'attraper
`--force`.

## Les deux refus

Tous deux arrivent **avant** que cargo ne soit lancé, et dans l'ordre où ils se réparent.

Un projet sans `src/lib.rs` — créé avant rbs 1.0 — est refusé en le nommant : `ApiDoc` y vit
dans le binaire principal, où un second binaire ne peut pas l'atteindre. Annoncer d'abord le
binaire manquant enverrait écrire un fichier qui ne compilerait pas.

Un projet sans `src/bin/openapi.rs` est refusé avec le bloc à coller — le fichier, et la
section `[[bin]]` qui le déclare. Un projet créé par `rbs new` porte déjà les deux.

## Ce qu'elle vous laisse

- **les langages** — `ts` seul aujourd'hui ;
- **l'empaquetage** — le fichier est écrit, et rien n'en fait un paquet npm ;
- **les opérations sans `operationId`** — la commande refuse le document entier plutôt que
  d'engendrer un client partiel. Chaque handler qu'rbs engendre en porte un ; un handler
  que vous avez écrit à la main a besoin du sien, comme le montre `broadcast` dans
  `examples/newsletter-queue`.
