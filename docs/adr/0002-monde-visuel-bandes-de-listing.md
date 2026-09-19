# Le monde visuel du frontend engendré : « Bandes de listing »

Le frontend par défaut est composé comme une sortie d'imprimante ligne sur papier en
continu : marge perforée, bandes alternées pleine largeur, encre de ruban d'impact, une
seule seconde couleur pour l'alerte. Il n'emprunte rien au standard de la catégorie — fond
charbon, dégradé maillé, accent néon — que cette décision écarte explicitement.

## Context

Le projet n'avait aucune identité visuelle : son site de documentation est un Docusaurus de
série, palette par défaut jamais modifiée. Le premier écran du frontend engendré est donc
aussi la première proposition d'identité de `rbs`, et il part chez tous les utilisateurs.

## Considered Options

**Le standard de la catégorie**, exécuté impeccablement. Écarté parce qu'il est, par
construction, indiscernable de tout autre outil pour développeurs — et parce qu'une page
neutre est une page que chaque utilisateur jette, ce qui rendrait la vendorisation de
shadcn-vue gratuite.

**Fiche technique de composant** (couverture de datasheet, deux encres, schéma
fonctionnel). La plus immédiatement lisible pour ce public, et pour cette raison la plus
prévisible ; une couverture de fiche ne vit pas, alors qu'une page d'accueil doit le faire.

**Panneau d'instruments de vol de nuit.** Excellent pour l'administration, où le statut a
trois états et non deux, mais un cadran ne porte ni prose, ni `curl`, ni démarrage rapide —
et sa palette frôle précisément l'ornière refusée.

## Consequences

Trois disciplines contraignent toute retouche future : la bande est un matériau et non un
fond, donc aucun contenu ne repose sur du blanc indifférencié ; la hiérarchie se fait par
contraste d'échelle seul, sans niveau intermédiaire ; et tout élément graphique est un
caractère, un filet ou une perforation.

Cette dernière règle a été assouplie une fois, sur instruction du mainteneur : les emoji
sont proscrits, mais les icônes vectorielles sont admises **là où elles sont une
affordance** — chevron d'un menu, croix d'une boîte de dialogue, flèche de tri — jamais
comme ornement.

Le monde est entièrement tokenisé : le remplacer revient à réécrire le bloc `@theme`, sans
toucher à un seul composant.
