# rbs

Un générateur de projets d'API web en Rust. Ce fichier est le glossaire du projet :
il fixe le mot juste pour chaque concept, et rien d'autre. Les décisions vivent dans
`docs/adr/`, l'architecture dans `CLAUDE.md`.

## Language

### Le générateur

**Fragment** :
Une unité installable par `rbs add`, décrite par un `feature.toml` déclaratif et posée
dans un projet existant.
_Avoid_: feature, plugin, extension, module

**Ancre** :
Un marqueur en commentaire, de la forme `<rbs:nom>`, dans un fichier du projet engendré,
où le CLI insère du contenu sans jamais relire l'arbre syntaxique.
_Avoid_: marqueur, point d'injection, hook, balise

**Squelette** :
L'arborescence que produit `rbs new`, avant tout fragment.
_Avoid_: template de projet, boilerplate, starter

**Plan** :
La liste réifiée des actions qu'une commande appliquera, calculée en mémoire et affichée
avant toute écriture sur le disque.
_Avoid_: diff, preview, dry-run

**Noyau** :
Ce que porte la crate `rbs-core` : le code qui n'a aucune raison de varier d'un projet à
l'autre. Son complément est le **généré**, que le CLI dépose chez l'utilisateur et que
celui-ci est censé lire et modifier.
_Avoid_: runtime, framework, librairie commune

**Exemple** :
Un projet réel versionné sous `examples/`, compilé en CI, et seule source des extraits
de la documentation.
_Avoid_: démo, sample, fixture

**Dérive** :
L'écart entre un exemple versionné et ce que le CLI produirait aujourd'hui pour les mêmes
commandes.
_Avoid_: désynchronisation, obsolescence, staleness

### Le frontend

**Socle** :
Ce que dépose le fragment `frontend` : l'application Vue, son routeur, ses stores, son
thème et son client d'API. Il ne suppose aucune authentification.
_Avoid_: starter, boilerplate, scaffold, base

**Accueil** :
La page publique servie à la racine du socle. Son contenu par défaut est vrai au premier
démarrage : elle décrit l'API engendrée à côté d'elle.
_Avoid_: landing, page d'atterrissage, vitrine, home

**Page d'amorçage** :
La page autonome que sert le Rust tant que le frontend n'a pas été construit. Elle nomme
les commandes qui manquent et disparaît dès que le build existe.
_Avoid_: page 404, fallback, page d'erreur, placeholder

**Shell d'administration** :
Ce que dépose le fragment `frontend-admin` : la coquille authentifiée — rail, garde de
route, quatre écrans de compte et de santé — dans laquelle viennent se monter les **écrans
engendrés**. Le shell est posé une fois ; les écrans arrivent ensuite, entité par entité.
_Avoid_: back-office, dashboard, admin panel

**Écran engendré** :
Ce que `rbs generate crud` émet pour une table quand `frontend-admin` est posé : liste
filtrée, formulaire, détail, montés dans le rail et la table de routage par deux ancres.
Il appartient à son auteur dès qu'il est écrit — c'est ce qui le distingue d'un moteur
générique, toujours hors périmètre. Voir ADR-0003, qui abroge ADR-0001.
_Avoid_: écran automatique, scaffold, interface générique

**Écran patron** :
La template unique dont sortent tous les écrans d'administration — celui que le fragment
pose pour son entité de démonstration comme ceux qu'engendre la commande. Une seule forme,
deux producteurs : ce qu'on voit à l'installation est ce qu'on obtiendra ensuite.
_Avoid_: écran d'exemple, template d'écran, page générique

**Bandes de listing** :
Le monde visuel du frontend engendré — papier en continu, marge perforée, encre de ruban
d'impact. Voir ADR-0002.
_Avoid_: thème, skin, charte

**Bande** :
L'unité de composition de ce monde : une rangée pleine largeur qui porte son propre fond.
Aucun contenu ne repose sur du blanc indifférencié.
_Avoid_: section, bloc, row, stripe
