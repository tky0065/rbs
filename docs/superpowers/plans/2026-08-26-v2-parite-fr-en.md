# V2 — Revue de parité FR/EN

## Décision

La parité se mesure, elle ne s'apprécie pas. Trois contrôles exécutables plutôt qu'une
lecture comparée : l'inventaire des pages, leur profil structurel (titres, blocs de code,
encarts, liens relatifs), et le dernier commit qui a touché chaque paire — ce dernier
étant le plus révélateur, puisque la règle du projet veut que les deux langues voyagent
dans le même commit.

## Constat

Les 14 paires de pages du site sont intactes : aucun écart structurel, et chacune a pour
dernier commit celui de sa jumelle. Les 89 entrées des fichiers de traduction JSON sont
toutes renseignées. `README` et `CONTRIBUTING` sont à jour de part et d'autre.

Une seule brèche : aucune version française du code de conduite, et
`CONTRIBUTING.fr.md` renvoyait le lecteur francophone vers le texte anglais.

## Étapes

1. `CODE_OF_CONDUCT.fr.md` depuis la **traduction officielle** du Contributor Covenant
   2.1, front-matter TOML du site d'origine retiré et adresse de signalement reprise de
   la version anglaise. Traduire soi-même ferait diverger le français de la référence,
   ce que la version anglaise ne fait pas.
2. Faire pointer `CONTRIBUTING.fr.md` vers elle.

## Preuve

Les trois contrôles rejoués après correction, plus la résolution de tous les liens `.md`
relatifs et une construction du site sur les deux locales.
