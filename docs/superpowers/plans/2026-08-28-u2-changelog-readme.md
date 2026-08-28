# U2 — `CHANGELOG.md`, et le README remis à jour

**Conception.** Deux livrables de texte et un instrument de mesure.

**Le CHANGELOG est bilingue**, `CHANGELOG.md` et `CHANGELOG.fr.md`, comme le README, le
guide de contribution et le code de conduite : le `.md` nu est l'anglais partout à la racine,
et le critère exige la parité FR/EN des fichiers racine. Un CHANGELOG français seul serait
le seul fichier racine sans homologue — exactement la brèche que `V2` avait comblée pour le
code de conduite.

**Une seule entrée**, couvrant les quatre jalons livrés, et non quatre entrées reconstituées :
la conception §5 pose que reconstituer a un coût sans lecteur pour l'instant. Elle est titrée
`## [Unreleased]` / `## [Non publié]` tant que 0.4.0 n'est pas sur crates.io — c'est ce qui
satisfait « aucune entrée pour une version non publiée ». **`U3` la renommera** en
`## [0.4.0] — <date>` au moment du tag.

Le CHANGELOG s'écrit à la main, et non depuis `git log` : un sujet de commit est écrit pour
qui lit le dépôt, une entrée de CHANGELOG pour qui installe. Les 406 commits de l'historique
sont la matière, pas le résultat.

**Le README passe à son état post-0.4.0** : le numéro de version, `cargo install rbs-cli`,
un renvoi au CHANGELOG. `README.md:12` annonce encore « Version 0.1 is under construction »
quand quatre jalons sont livrés. La mention « aucune promesse semver avant la 1.0 »
(`README.md:14`) **reste en place** : elle est vraie tant que `W2` n'a pas posé les
`#[non_exhaustive]`, et c'est `W3` qui la remplacera par la promesse. Le README annonce donc
une publication qui n'a pas encore eu lieu pendant l'intervalle qui sépare ce lot de `U3` —
arbitrage assumé, borné à la durée du lot.

**L'instrument manque et c'est le point non évident du lot.** Le critère dit « parité mesurée
comme en `V2` », et `V2` a mesuré, pas apprécié. Or `docs/scripts/parite.mjs` — figé par `O2`
après que `V2` et `J3` l'eurent réécrit chacune de son côté — ne compare que les pages du
site, `docs/docs` contre `i18n/fr/…`. Sans extension, ce lot s'auto-décernerait un critère
subjectif. Le script reçoit donc un mode « paires racine » (`X.md` ↔ `X.fr.md`) réutilisant
sa comparaison structurelle : charpente des titres, langue et méta des blocs de code, type
des encarts, cible des liens relatifs. `W3` et `Y1` en auront besoin après ce lot.

## Étapes

1. Étendre `docs/scripts/parite.mjs` d'un mode « paires racine » : découvrir les paires
   `X.md` / `X.fr.md` à la racine du dépôt, leur appliquer la comparaison structurelle
   existante, et les compter dans le verdict. Ne pas toucher au comportement sur le site.
2. Éprouver l'instrument **avant** de s'en servir de preuve, comme `V2`, `J3` et `O2` :
   un titre retiré du fichier français, un lien détourné, un bloc de code dont la langue
   change — un écart signalé, nommant le fichier, à chaque fois.
3. Écrire `CHANGELOG.md` et `CHANGELOG.fr.md` : en-tête Keep a Changelog, une entrée
   `## [Unreleased]` / `## [Non publié]` groupée en `Added` / `Changed` / `Fixed`, couvrant
   les quatre jalons livrés.
4. Réécrire la section « Status » / « Statut » des deux README, et leur ajouter le renvoi au
   CHANGELOG.
5. Preuves : `npm run parite` → les 4 paires racine et les 18 paires du site, 0 écart ;
   les morsures de l'étape 2, une ligne chacune ; un contrôle que le CHANGELOG ne nomme
   aucune version publiée — le seul titre de version est `Unreleased` / `Non publié`.
