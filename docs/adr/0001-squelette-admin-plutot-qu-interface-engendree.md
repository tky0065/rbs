---
status: superseded by ADR-0003
---

# Un squelette d'administration, jamais une interface engendrée

> **Abrogé le 2026-09-19 par [ADR-0003](0003-ecrans-d-administration-engendres.md).**
> Le corps ci-dessous est conservé tel qu'il a été écrit : il porte le raisonnement qui
> valait à ce moment-là, et les deux alternatives qu'il écarte restent des lectures utiles.
> Ce qui a changé n'est pas l'analyse mais l'arbitrage — voir ADR-0003.

`ROADMAP.md` range « interface d'administration générée » dans un hors-périmètre annoncé
comme définitif et non comme un « plus tard ». Le fragment `frontend-admin` livre donc une
coquille authentifiée — rail, garde de route, cinq écrans adossés à des routes que `auth`
expose réellement — et s'arrête là : `rbs generate crud` n'émet aucun fichier Vue, et rien
dans le shell n'est dérivé d'une entité.

## Considered Options

**Écrans engendrés par entité.** `rbs generate crud post --fields …` aurait aussi produit
`frontend/src/pages/admin/posts/`. C'est la promesse la plus forte, et elle tombe
frontalement sous la ligne du hors-périmètre. Elle aurait de surcroît exigé une famille
d'ancres côté TypeScript — registre de routes, entrées du rail — doublant les dix-sept
ancres que `rbs doctor` parcourt, dans un langage où le CLI n'a aujourd'hui aucun outillage.

**Moteur générique piloté par l'OpenAPI.** Une application unique lisant
`/api-docs/openapi.json` au démarrage et déduisant tables et formulaires. Zéro code
engendré, mais un moteur à maintenir et des écrans qu'on ne personnalise pas sans le
forker — ce qui contredit la règle « ce code est fait pour être modifié ».

## Consequences

Le cinquième écran du shell, l'**écran patron**, est câblé en dur sur une entité réelle
plutôt que paramétré. C'est délibéré : un écran paramétrable serait le premier pas vers le
moteur générique écarté ci-dessus. Sa valeur est d'être recopié, pas réutilisé.

Rouvrir cette décision suppose d'amender `ROADMAP.md` d'abord, pas d'ajouter discrètement
un `--with-admin` à `generate crud`.
