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

Le client atterrit dans `clients/ts/client.ts` — ou dans `frontend/src/api/client.ts` sur
un projet qui porte le fragment [`frontend`](./add.md), là où le socle l'importe et là où
le serveur de développement peut l'atteindre. Régénérez-le après chaque changement de
contrat plutôt que de le retoucher : la commande refuse d'écraser un fichier modifié, et
`--force` lève ce refus.

## D'où vient le document

**Aucun serveur ne tourne.** `rbs new` écrit un troisième binaire, `src/bin/openapi.rs`, qui
imprime ce que rend `ApiDoc::openapi()` ; `generate client` lance `cargo run --bin openapi`
dans le projet et lit sa sortie standard.

Le document est mémorisé sous `target/rbs/`, indexé par un condensat des sources du
projet : une seconde exécution sur un projet inchangé répond sans rien compiler.
[`rbs openapi export`](./openapi.md#le-contrat-mémorisé) dit ce que le condensat couvre.

C'est ce qui fait suivre le code au client, et non une lecture approximative des sources :
le document porte les routes que vos fragments ont montées, les DTO que vos `--fields` ont
produits, et l'`operationId` de chaque handler — y compris ceux que vous avez écrits à la
main.

Le binaire vaut par lui-même. Figer le contrat en CI, c'est un
[`rbs openapi export --out openapi.json`](./openapi.md) suivi d'un `git diff` qui doit
rester vide.

## Les drapeaux

| Drapeau | Effet |
|---|---|
| `--lang <LANGAGE>` | **Requis.** `ts` en est aujourd'hui la seule valeur. Aucun défaut : le jour où un second langage arrive, aucune invocation existante ne change de sens. |
| `--out <DIR>` | Répertoire de sortie, relatif à la racine du projet, à la place du défaut que le projet dicte. Le nom du fichier ne change pas — c'est celui que le client porte dans un import. |
| `--from <FICHIER>` | Lit un contrat déjà exporté par [`rbs openapi export`](./openapi.md), relatif au répertoire d'où la commande est lancée, au lieu de compiler le projet. Le client s'écrit alors sans aucune chaîne de compilation Rust — ce qu'il faut à une CI qui commite son `openapi.json`, et l'échappatoire quand le contrat mémorisé se trompe. |
| `--force` | Écrit même si le working tree Git est sale, et écrase un client signalé en conflit. |
| `--dry-run` | Affiche le plan et s'arrête. rbs n'écrit rien — mais le projet est tout de même compilé, puisque c'est ainsi que le document se lit, à moins que `--from` ne le fournisse. |
| `--json` | Rend le plan — ou l'erreur — en un seul document JSON sur la sortie standard, contenu complet du client compris ; la compilation du projet reste sur la sortie d'erreur. Indépendant de `--dry-run`. [Le guide des agents](../guides/agents.md#lire-un-plan-en-json) donne le document et les codes d'erreur. |

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

{/* rbs:libre raison="exige un client engendré puis édité à la main, et l'engendrer compile le projet" */}
```text
  ! clients/ts/client.ts   conflit — relancer avec --force
```

C'est le moment de sortir votre propre code du fichier engendré, plutôt que d'attraper
`--force`.

## Appelée après `rbs generate crud`

Sur un projet qui porte déjà un client engendré, vous n'avez pas à le faire : `rbs generate
crud` le réécrit juste après avoir écrit l'entité, depuis le contrat et jamais depuis
l'entité qu'il vient de produire — [l'ADR-0004](https://github.com/tky0065/rbs/blob/main/docs/adr/0004-une-seule-source-pour-le-client-engendre.md)
dit pourquoi le client n'a qu'une source. La génération paie donc une recompilation
incrémentale, par construction : le module vient d'être ajouté, et le contrat mémorisé est
invalide à l'instant où la commande en a besoin.

Cela reste conditionnel. Un projet sans client garde l'ancien comportement — la commande
affiche la ligne `rbs generate client` à relancer, et n'invente pas un `frontend/src/api`
que personne n'a demandé.

La réécriture est un écrasement, non un conflit : un contrat qui vient de gagner une table
rend tout client existant différent, et refuser reviendrait à refuser chaque fois. Ce qui
protège un client que vous avez retouché est la garde de `generate crud` elle-même — elle ne
tourne pas sur un working tree sale sans `--force`, et la version précédente est donc à un
`git checkout` de là.

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
