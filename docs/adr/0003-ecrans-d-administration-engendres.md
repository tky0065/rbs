# `generate crud` engendre les écrans d'administration

`rbs generate crud` émet, en plus de l'entité SeaORM et de sa migration, les écrans
d'administration de la table — liste filtrée, formulaire, détail — dès lors que le fragment
`frontend-admin` est posé. Cette décision **abroge [ADR-0001](0001-squelette-admin-plutot-qu-interface-engendree.md)**
et retire « interface d'administration générée » du hors-périmètre de `ROADMAP.md`, où elle
figurait depuis l'origine.

## Context

ADR-0001 tranchait pour un squelette au motif que la roadmap excluait l'interface engendrée,
et l'analyse qu'il en donnait reste juste : le coût est réel, et il tombe précisément là où
il l'annonçait. Le mainteneur a renversé l'arbitrage le 2026-09-19, sur un motif qu'ADR-0001
ne pesait pas — **la vitesse sur le générique**. Un développeur qui décrit sa table en une
ligne de `--fields` obtient déjà son API complète ; lui laisser écrire ses écrans à la main
rend la seconde moitié du travail disproportionnée par rapport à la première.

## Decision

La commande lit les fragments installés et adapte sa sortie, sans drapeau à poser — c'est
exactement ce qu'elle fait depuis la v1.3 pour l'authentification : « sur un projet qui
porte `auth`, `rbs generate crud` écrit des routes fermées ». Exiger un `--with-admin`
alors que `--with-auth` n'existe pas aurait été incohérent. Un `--no-admin` reste la sortie
de secours, pour une entité purement interne.

Deux ancres nouvelles, et deux seulement : la table de routage de l'espace d'administration
et les entrées du rail. Pas de troisième pour déclarer le module — en TypeScript, l'import
dans la table de routage *est* la déclaration, là où Rust demande un `pub mod` distinct du
montage. Le registre passe de dix-huit à vingt.

## Consequences

**Un couplage nouveau**, qu'ADR-0001 refusait et qu'il faut assumer les yeux ouverts :
`generate crud` cesse d'être une commande purement Rust et connaît désormais la disposition
des fichiers du socle. Déplacer un répertoire du fragment `frontend` casse la génération.
C'est le prix, et il se paie à chaque évolution du socle.

**L'écran patron change de nature.** Il n'est plus l'écran câblé en dur qu'on recopie à la
main : il devient la template même du générateur. Une seule forme d'écran, deux producteurs
— le fragment la pose une fois pour une entité de démonstration, la commande la rend pour
chaque entité réelle. Ce qu'on voit à l'installation est donc exactement ce qu'on obtiendra
ensuite, ce qui n'était vrai dans aucune des deux autres options examinées (un shell sans
écran de démonstration laisse un espace vide et muet sur un projet neuf ; un écran de
démonstration distinct du gabarit diverge).

**Ce qui reste hors périmètre.** Le moteur générique piloté par l'OpenAPI au runtime, écarté
par ADR-0001, l'est toujours et pour la même raison : des écrans qu'on ne personnalise pas
sans forker le moteur contredisent la règle « ce code est fait pour être modifié ». Un écran
engendré, lui, appartient à son auteur dès qu'il est écrit — c'est toute la différence.
