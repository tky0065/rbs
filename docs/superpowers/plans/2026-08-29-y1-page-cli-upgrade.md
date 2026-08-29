# Y1 — Page `cli/upgrade`, FR et EN

**Conception.** La dernière commande du CLI est la seule sans page. Elle en a d'autant plus
besoin qu'elle est celle qu'on cherche au pire moment : quand un projet ne compile plus
après une montée de version.

La page suit le plan de `cli/doctor.md`, qui est le modèle du répertoire : synopsis avec le
`--help` capturé, ce que la commande touche, la séquence qu'elle déroule, puis les refus.

**Aucun extrait ne s'écrit à la main** — c'est le second critère, et c'est la règle du
dépôt depuis `J3` : les blocs de terminal sont capturés en lançant la commande. Cinq
sorties à prendre sur le binaire :

1. `rbs upgrade --help` ;
2. un saut portant une note — celui vers 1.0.0, qui affiche le gel ;
3. un saut sans note — message neutre, sortie 0 ;
4. le refus d'un projet postérieur au binaire, qui nomme les deux versions ;
5. le refus d'un arbre de travail sale, qui nomme les fichiers.

Plus le `git diff --name-only` d'un projet mis à niveau, qui ne rend que `Cargo.toml` —
c'est la propriété que la page doit rendre vérifiable par son lecteur.

**Les deux premières sorties exigent un binaire en 1.0.0**, que le workspace ne porte pas
encore. Elles se capturent en montant temporairement le numéro, puis en le remettant à
`0.4.0` : `Y2` le montera pour de bon. Vérifier que `Cargo.toml` **et** `Cargo.lock`
reviennent à l'identique.

## Le plancher PostgreSQL, corrigé au passage

Trois affirmations du site sont fausses, dans les deux langues, et aucune tâche du backlog
ne les couvre. `S2` a fait poser l'identifiant v7 par les modèles ; `uuidv7()` n'est donc
plus demandé au serveur, et `doctor` retient depuis un plancher de **14**, motivé par le
support communautaire (`doctor/base.rs:283`).

| Fichier | Ce qu'il dit | Ce qui est vrai |
|---|---|---|
| `getting-started.md:21` | « PostgreSQL 18 or later » | 14 ou plus |
| `guides/migrations.md:54` | « PostgreSQL 18 is the floor, since `uuidv7()` became native » | La raison est caduque : le modèle pose l'identifiant |
| `guides/testing.md:68,83` | « a PostgreSQL 18 container », « not negotiable » | `tests/common/mod.rs:160` déclare `IMAGE = ("postgres", "17")` |

**Les sorties capturées ne changent pas.** `getting-started.md:337` montre
`✓ base PostgreSQL 18.6 répond sur localhost:5432` : la machine de capture portait bien un
18.6, et cette ligne ne prétend pas être une exigence.

## Étapes

1. Capturer les cinq sorties sur le binaire, dont deux avec un workspace temporairement en
   1.0.0. Remettre le numéro et vérifier `Cargo.toml` et `Cargo.lock` à l'identique.
2. Écrire `docs/docs/cli/upgrade.md` (`sidebar_position: 8`) et son miroir français, dans
   le même commit.
3. Corriger les trois affirmations de plancher, dans les deux langues, sans toucher aux
   sorties capturées.
4. Preuves : `npm run parite` sur les paires du site et de la racine, l'instrument éprouvé
   d'abord ; `npm run clear && npm run build` → deux `[SUCCESS]` ; et la confirmation que
   chaque bloc de terminal de la page neuve vient d'une capture.
