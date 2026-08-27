---
name: rbs-task
description: Use when asked to implement, execute or continue one or more tasks from the rbs TODO.md backlog — triggers like « fais la tâche B3 », « attaque le lot A », « les tâches A1 à A3 », « les lots C et F en parallèle », « continue le TODO », « la prochaine tâche ». Not for writing the backlog itself or editing ROADMAP.md.
---

# rbs-task

## Overview

Exécute une ou plusieurs tâches de `TODO.md`, identifiées par leur ID de lot (`A1`, `B3`, `D9`…). Plusieurs tâches demandées ensemble peuvent être menées **en parallèle**, un agent par lot dans son propre worktree.

**Trois principes durs, issus de l'observation d'agents réels sur ce backlog :**

1. **Une case ne se coche pas parce que le fichier est écrit.** Elle se coche parce que le critère `✓` a été exécuté et que sa sortie est consignée sur la ligne.
2. **L'ordre des lots `A → B → C → D → E` n'est pas indicatif.** Une tâche dont l'amont est incomplet ne se commence pas — même si on sait la contourner.
3. **`TODO.md` a un seul écrivain : l'orchestrateur.** Un agent parallèle implémente et prouve ; il ne coche pas, et son rapport ne vaut pas preuve.

Les deux premières existent parce que des agents compétents, honnêtes sur leurs limites, les ont violées trois fois sur trois. Écrire du bon code et signaler ce qui manque ne suffit pas.

## Checklist

**Créer un todo par étape avant de commencer.**

- [ ] 1 — Sélectionner et vérifier les dépendances
- [ ] 2 — Décider séquentiel ou parallèle
- [ ] 3 — Concevoir, puis planifier
- [ ] 4 — Implémenter en TDD
- [ ] 5 — Prouver chaque critère ✓
- [ ] 6 — Cocher avec la preuve
- [ ] 7 — Finir

Les étapes 3 à 5 forment la **chaîne d'une tâche** : elle se déroule entière pour chaque tâche, qu'elle soit menée en direct ou par un agent. Le parallélisme fait tourner plusieurs chaînes à la fois, il n'en mutualise aucune.

## Étape 1 — Sélection et dépendances

Lire `TODO.md`. Résoudre les IDs demandés. « Continue » / « la prochaine » → première case non cochée dans l'ordre du fichier.

**Contrôle obligatoire, avant toute écriture de code :**

```bash
grep -n '^- \[ \]' TODO.md    # non faites, dans l'ordre des lots
```

Toute case non cochée **au-dessus** de la tâche visée est une dépendance non satisfaite.

**Amont incomplet → STOP.** Ne pas coder, ne pas contourner. Dire au user quelles tâches bloquent, et proposer soit de les faire d'abord, soit d'obtenir sa confirmation explicite qu'il accepte un travail jetable.

Exception connue : une dépendance amont qui exige une **action humaine hors dépôt** (protection de branche GitHub, secret CI, compte à créer) n'interdit pas d'ouvrir un lot qui n'en dépend pas — mais elle se dit au user, qui tranche.

Le contournement type à refuser : *« le workspace n'existe pas encore, je mets le code à la racine en attendant »*. `A1` exige la suppression de `src/` racine — ce code sera jeté. Un contournement connu d'avance n'est pas un compromis, c'est du travail perdu.

Résumer avant de démarrer : « 3 tâches : C4 (squelette), D1 (grammaire des champs), F2 (garde-fou docs) ».

## Étape 2 — Séquentiel ou parallèle

Une seule tâche → séquentiel, passer à l'étape 3.

Plusieurs tâches → répondre aux trois questions, **puis donner les réponses au user avant de choisir** :

| Question | Ce qui la ferme |
|---|---|
| **Dépendance logique ?** | Une tâche produit ce qu'une autre consomme (un format, une ancre, un type public) → séquentiel, sans discussion. |
| **Fichiers partagés ?** | Mêmes fichiers de templates, même module, même manifeste → séquentiel. Crates disjointes (`rbs-core` d'un côté, `rbs-cli` de l'autre) → candidat sérieux. |
| **Vérification sérialisée ?** | Cible de compilation partagée (`target/rbs-integration`), Docker/testcontainers, port fixe. Le lock de cargo sérialise de toute façon ; donner une cible propre à chaque worktree fait recompiler toute l'arborescence de dépendances, ce que la cible commune existe pour éviter. |

**Présenter l'arbitrage chiffré, jamais « parallèle ou séquentiel ? » à sec.** Formuler : quels lots sont parallélisables, et *où est le goulot*. Exemple réel : « I4+I5 et I6 touchent les mêmes templates, et leur vérification passe par la même cible de compilation — le parallélisme ne gagnerait rien. » Le user tranche vite et bien avec cette information ; il arbitre mal sans elle.

Par défaut : deux lots réellement disjoints → parallèle. Doute non levé → séquentiel.

### Contrat de dispatch

Parallélisme retenu → `superpowers:dispatching-parallel-agents` et `superpowers:using-git-worktrees`. **Un worktree et une branche `<id>-<slug>` par agent** — jamais deux agents dans le même arbre de travail.

Le prompt de chaque agent contient, dans cet ordre :

1. le bloc `TODO.md` intégral de sa tâche : titre, corps, **toutes** les lignes `✓` ;
2. le chemin de son worktree et l'interdiction d'en sortir ;
3. la chaîne à dérouler — étapes 3, 4 et 5 de ce skill, y compris brainstorming et plan ;
4. le format de retour : par critère `✓`, la commande exacte et sa sortie condensée ; puis la branche et le SHA du dernier commit ;
5. la phrase **« Ne touche pas à `TODO.md`. »**

Deux agents qui écrivent `TODO.md` produisent un conflit et des cases fausses. L'orchestrateur coche, à l'étape 6, sur des preuves qu'il a lui-même relancées.

## Étape 3 — Concevoir, puis planifier

Ces deux étapes sont sautées par défaut. Elles ne le sont pas ici.

**A — `superpowers:brainstorming`.** Presque toutes les tâches du TODO sont *bounded* : design de quelques phrases en chat, approbation, puis code. Comptent comme *architectural* (design doc complet) les tâches qui figent un format dont tout le reste dépend : `C4` (squelette de projet), `D1` (grammaire des champs), `E1` (modèle de plan).

**B — `superpowers:writing-plans`** → `docs/superpowers/plans/YYYY-MM-DD-<id>-<slug>.md`. Pour une tâche de 30 lignes, le plan tient en 5 lignes — mais il existe.

## Étape 4 — Implémenter

**Sur une branche dédiée, jamais sur `main`.** `git switch -c <id>-<slug>` avant la première écriture — l'étape 7 en dépend, et un lot raté doit pouvoir être abandonné sans réécrire l'historique.

`superpowers:test-driven-development`. Les lignes `✓ Test :` de la tâche **sont** les tests à écrire : les écrire d'abord, les voir échouer, puis implémenter.

Conventions du projet (voir `docs/superpowers/specs/2026-08-25-rbs-design.md` §5.6) : pas de commentaire qui paraphrase le code, `missing_docs` sur `rbs-core`, `clippy -D warnings` et `fmt --check` propres.

## Étape 5 — Prouver chaque critère ✓

**Chaque ligne `✓` est une obligation distincte.** Une tâche à trois `✓` a trois preuves à produire.

| Type de critère | Ce qui vaut preuve |
|---|---|
| Exécutable (`cargo test`, `cargo build`, une commande) | La commande lancée + sa sortie réelle, lue avant toute affirmation |
| Non exécutable (« inspection visuelle », « revue de lecture ») | L'artefact produit + **la validation du user**. Ne jamais s'auto-décerner un critère subjectif. |
| Dépendant d'une tâche non faite | **Non rempli.** Pas de substitution, pas d'équivalent approchant. |

`superpowers:verification-before-completion` s'applique : la sortie se lit, elle ne se suppose pas.

**Retour d'un agent parallèle :** son rapport est une déclaration, pas une preuve. Avant de cocher, relancer soi-même chaque commande dans son worktree et lire la sortie. Un agent qui annonce « 3 tests passent » a peut-être lancé autre chose, ou lancé dans le mauvais arbre.

## Étape 6 — Cocher avec la preuve

Format **obligatoire** — le slot preuve est requis, et tient **sur une seule ligne** :

```markdown
- [x] **B3** · Middleware `request_id` — vérifié 2026-08-25 · `cargo test -p rbs-core request_id` → 2 passed
```

Commande + résultat condensé. Ne pas recopier la sortie complète : le détail vit dans le message de commit, pas dans `TODO.md`. Ce fichier compte 52 tâches — quatre lignes de preuve chacune le rendent illisible.

Impossible de remplir le slot = impossible de cocher.

**Un seul `✓` non prouvé et la case reste `- [ ]`**, annotée :

```markdown
- [ ] **A2** · CI minimale — PARTIEL 2026-08-25 : workflow écrit, mais le critère
      « un PR avec un warning clippy est bloqué » exige une protection de branche
      GitHub non activée. Reste à faire côté dépôt.
```

Signaler la limite dans le rapport final ne remplace pas cette annotation. Le `TODO.md` est la source de vérité, pas la conversation.

Un lot parallèle qui échoue ne bloque pas les autres : cocher ceux qui sont prouvés, annoter celui-là, continuer.

## Étape 7 — Finir

`superpowers:finishing-a-development-branch` (tests finaux, décision commit/merge/PR). Plusieurs branches parallèles → les intégrer une par une, en relançant la vérification après chaque intégration : deux lots verts séparément peuvent casser une fois réunis.

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
| « L'agent rapporte 3 tests verts, je coche » | Un rapport est une déclaration. Relancer la commande dans son worktree, lire la sortie, puis cocher. |
| « Je laisse chaque agent cocher sa propre ligne » | Deux écrivains sur `TODO.md` = conflit et cases fausses. Un seul écrivain : l'orchestrateur. |
| « Les fichiers sont disjoints, donc je parallélise » | L'indépendance des fichiers ne suffit pas : cible de compilation partagée, Docker et lock de cargo sérialisent la vérification. Vérifier les trois questions. |
| « Je demande au user : parallèle ou séquentiel ? » | Question à sec = arbitrage à l'aveugle. Donner les lots parallélisables **et** le goulot chiffré. |
| « Deux tâches en parallèle, un seul brainstorming pour les deux » | Le parallélisme fait tourner deux chaînes, il n'en fusionne aucune. Chaque tâche a son design, son plan, ses preuves. |
| « Deux worktrees, je leur donne le même dossier de build pour aller vite » | Ils se bloqueront sur le lock de cargo. Si la cible doit être partagée, c'est que le lot n'était pas parallélisable. |
| « Pas de remote, je commite sur `main` » | Observé en test. Branche dédiée toujours : l'étape 7 en dépend. |
| « Je recopie toute la sortie, c'est plus rigoureux » | Observé en test. Une ligne par preuve. Le détail va dans le commit. |
