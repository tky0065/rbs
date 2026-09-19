# La documentation bilingue des deux fragments — plan

Issue **#25**, sous la spec **#13**. Dernière tranche du jalon v1.7.

## Les sept critères et ce qui les prouve

- [x] **Un guide en anglais et son homologue français, dans le même commit.** — Fait le
  2026-09-19 : `docs/docs/guides/frontend.md` et
  `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/frontend.md`, en
  `sidebar_position: 11.9`.
- [x] **Les deux entrées de catalogue des fragments.** — `frontend` y était déjà,
  `frontend-admin` manquait, et le titre annonçait quinze features quand le CLI en livre
  seize. Corrigé dans les deux langues (`cli/add.md`), avec les deux ancres de titre qui y
  renvoyaient.
- [x] **La page de la commande de génération dit ce qu'elle émet, et ce que le drapeau de
  refus supprime.** — La ligne de transcription porte `--no-admin`, la table des options en
  donne l'effet complet, et la section des ancres gagne les deux du frontend — les seules
  que la commande vise sans que le squelette les porte, et les seules qu'elle saute plutôt
  que de refuser.
- [x] **Le renversement de portée est mentionné là où un lecteur pourrait encore croire
  l'ancienne règle.** — `ROADMAP.md` le portait déjà. Le guide le porte à son tour, en
  nommant ADR-0003 et ADR-0001. Aucune autre page du site n'énonçait l'ancienne règle —
  vérifié par balayage sur « hors périmètre » et « out of scope ».
- [x] **Aucun extrait de code écrit à la main : tout vient de l'exemple.** — Les huit
  extraits du guide sont des blocs `file=examples/admin-console/…`, six par `region=` posée
  dans l'exemple ; les deux transcripts sont gardés par le marqueur `rbs:transcript` et
  rejoués. Le build du site est le seul endroit d'où une région disparue se verrait.
- [x] **Le contrôle de parité rend zéro écart.** — `npm run parite` : 48 paires de pages,
  48/48 au même dernier commit, 269 liens relatifs, 0 écart structurel.
- [x] **La construction du site passe.** — `npm run typecheck`, `npm test` (16 passés) et
  `npm run build` : les deux locales construites, aucun lien mort, aucune région
  introuvable.

## Ce que la tranche corrige au passage

Le jalon avait laissé **« dix-huit ancres » sur cinq pages dans les deux langues**, dont la
promesse de compatibilité, qui porte précisément sur les noms d'ancres et leur syntaxe. Le
registre en porte vingt depuis la tranche du shell. Corrigé, et désormais **gardé** :
`anchors::tests::the_documentation_names_every_anchor_and_no_other` compare le registre aux
`<rbs:…>` que nomment les quatre pages qui en dressent la liste, dans les deux langues. Une
ancre neuve non documentée fait maintenant échouer la suite — c'est ce qui manquait pour que
l'oubli ne se reproduise pas.

Le `[Unreleased]` des deux changelogs était resté vide pendant les douze tranches du jalon.
Il porte désormais les quatre entrées du jalon et le correctif du client typé.

## Ce que le plan ne fait pas

Aucun tutoriel : les tutoriels du site partent d'un projet neuf et enchaînent des commandes,
et le frontend demande une chaîne npm qu'un tutoriel ne peut pas garder à jour. Le guide
porte les gestes ; l'exemple porte la preuve.
