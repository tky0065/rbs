---
name: rbs-task
description: Use when asked to implement, execute or continue one or more tasks from the rbs TODO.md backlog — triggers like « fais la tâche B3 », « attaque le lot A », « les tâches A1 à A3 », « continue le TODO », « la prochaine tâche ». Not for writing the backlog itself or editing ROADMAP.md.
---

# rbs-task

## Overview

Exécute une ou plusieurs tâches de `TODO.md`, identifiées par leur ID de lot (`A1`, `B3`, `D9`…).

**Deux principes durs, issus de l'observation d'agents réels sur ce backlog :**

1. **Une case ne se coche pas parce que le fichier est écrit.** Elle se coche parce que le critère `✓` a été exécuté et que sa sortie est consignée sur la ligne.
2. **L'ordre des lots `A → B → C → D → E` n'est pas indicatif.** Une tâche dont l'amont est incomplet ne se commence pas — même si on sait la contourner.

Ces deux règles existent parce que des agents compétents, honnêtes sur leurs limites, les ont violées trois fois sur trois. Écrire du bon code et signaler ce qui manque ne suffit pas.

## Checklist

**Créer un todo par étape avant de commencer.**

- [ ] 1 — Sélectionner et vérifier les dépendances
- [ ] 2 — Concevoir, puis planifier
- [ ] 3 — Implémenter en TDD
- [ ] 4 — Prouver chaque critère ✓
- [ ] 5 — Cocher avec la preuve
- [ ] 6 — Finir

## Étape 1 — Sélection et dépendances

Lire `TODO.md`. Résoudre les IDs demandés. « Continue » / « la prochaine » → première case non cochée dans l'ordre du fichier.

**Contrôle obligatoire, avant toute écriture de code :**

```bash
grep -n '^- \[ \]' TODO.md    # non faites, dans l'ordre des lots
```

Toute case non cochée **au-dessus** de la tâche visée est une dépendance non satisfaite.

**Amont incomplet → STOP.** Ne pas coder, ne pas contourner. Dire au user quelles tâches bloquent, et proposer soit de les faire d'abord, soit d'obtenir sa confirmation explicite qu'il accepte un travail jetable.

Le contournement type à refuser : *« le workspace n'existe pas encore, je mets le code à la racine en attendant »*. `A1` exige la suppression de `src/` racine — ce code sera jeté. Un contournement connu d'avance n'est pas un compromis, c'est du travail perdu.

## Étape 2 — Concevoir, puis planifier

Ces deux étapes sont sautées par défaut. Elles ne le sont pas ici.

**A — `superpowers:brainstorming`.** Presque toutes les tâches du TODO sont *bounded* : design de quelques phrases en chat, approbation, puis code. Comptent comme *architectural* (design doc complet) les tâches qui figent un format dont tout le reste dépend : `C4` (squelette de projet), `D1` (grammaire des champs), `E1` (modèle de plan).

**B — `superpowers:writing-plans`** → `docs/superpowers/plans/YYYY-MM-DD-<id>-<slug>.md`. Pour une tâche de 30 lignes, le plan tient en 5 lignes — mais il existe.

## Étape 3 — Implémenter

**Sur une branche dédiée, jamais sur `main`.** `git switch -c <id>-<slug>` avant la
première écriture — l'étape 6 en dépend, et un lot raté doit pouvoir être abandonné
sans réécrire l'historique.

`superpowers:test-driven-development`. Les lignes `✓ Test :` de la tâche **sont** les tests à écrire : les écrire d'abord, les voir échouer, puis implémenter.

Conventions du projet (voir `docs/superpowers/specs/2026-08-25-rbs-design.md` §5.6) : pas de commentaire qui paraphrase le code, `missing_docs` sur `rbs-core`, `clippy -D warnings` et `fmt --check` propres.

## Étape 4 — Prouver chaque critère ✓

**Chaque ligne `✓` est une obligation distincte.** Une tâche à trois `✓` a trois preuves à produire.

| Type de critère | Ce qui vaut preuve |
|---|---|
| Exécutable (`cargo test`, `cargo build`, une commande) | La commande lancée + sa sortie réelle, lue avant toute affirmation |
| Non exécutable (« inspection visuelle », « revue de lecture ») | L'artefact produit + **la validation du user**. Ne jamais s'auto-décerner un critère subjectif. |
| Dépendant d'une tâche non faite | **Non rempli.** Pas de substitution, pas d'équivalent approchant. |

`superpowers:verification-before-completion` s'applique : la sortie se lit, elle ne se suppose pas.

## Étape 5 — Cocher avec la preuve

Format **obligatoire** — le slot preuve est requis, et tient **sur une seule ligne** :

```markdown
- [x] **B3** · Middleware `request_id` — vérifié 2026-08-25 · `cargo test -p rbs-core request_id` → 2 passed
```

Commande + résultat condensé. Ne pas recopier la sortie complète : le détail vit dans
le message de commit, pas dans `TODO.md`. Ce fichier compte 52 tâches — quatre lignes
de preuve chacune le rendent illisible.

Impossible de remplir le slot = impossible de cocher.

**Un seul `✓` non prouvé et la case reste `- [ ]`**, annotée :

```markdown
- [ ] **A2** · CI minimale — PARTIEL 2026-08-25 : workflow écrit, mais le critère
      « un PR avec un warning clippy est bloqué » exige une protection de branche
      GitHub non activée. Reste à faire côté dépôt.
```

Signaler la limite dans le rapport final ne remplace pas cette annotation. Le `TODO.md` est la source de vérité, pas la conversation.

## Étape 6 — Finir

`superpowers:finishing-a-development-branch` (tests finaux, décision commit/merge/PR).

Rapport : tâches cochées avec leur preuve, tâches laissées partielles avec ce qui manque, blocages rencontrés.

## Red Flags — Stop et corrige

| Pensée | Réalité |
|---|---|
| « A1 n'est pas fait, je mets ça à la racine en attendant » | Observé 2 fois sur 3. `A1` supprime `src/` racine : ce code est déjà mort. STOP et demander. |
| « Je signale la limite dans mon résumé, donc je peux cocher » | Observé 3 fois sur 3. L'honnêteté du rapport ne remplace pas la preuve. Case non prouvée = case non cochée. |
| « Le critère dépend d'une tâche future, je coche quand même » | Le critère n'est pas rempli. `- [ ]` + annotation PARTIEL. |
| « Pas de site de docs, le rustdoc fera office de capture » | Substituer un artefact à un autre, c'est décider seul que le critère a changé. Demander au user. |
| « C'est visuel, je juge moi-même que c'est lisible » | Un critère subjectif se valide auprès du user. Toujours. |
| « La tâche fait 30 lignes, le plan est superflu » | Observé 3 fois sur 3 : zéro plan écrit. 5 lignes suffisent, mais elles existent. |
| « Le user est pressé, je vais au plus direct » | La pression au temps était la consigne exacte du scénario S1. Elle a produit du code à jeter. |
| « J'enchaîne les tâches du même lot, ça ira plus vite » | Chaque tâche a sa chaîne complète et son cochage prouvé. |
| « Pas de remote, je commite sur `main` » | Observé en test. Branche dédiée toujours : l'étape 6 en dépend. |
| « Je recopie toute la sortie, c'est plus rigoureux » | Observé en test. Une ligne par preuve. Le détail va dans le commit. |
