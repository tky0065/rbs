# `generate crud` compile le projet pour n'avoir qu'une source du client

`rbs generate crud` relance le binaire `openapi` du projet et réécrit le **client engendré**
en entier, au lieu d'y insérer les méthodes qu'il pourrait déduire de l'entité qu'il vient
d'écrire. La commande perd ainsi sa propriété la plus visible — ne jamais compiler — pour en
garder une autre, moins visible et plus coûteuse à retrouver : le client n'a qu'une source,
le **contrat**.

## Context

`generate crud` n'a jamais lancé cargo : tout y passe par minijinja et `rustfmt`, et la
justification est écrite dans le code (`lib.rs:1005-1008`, « la commande compile le projet,
ce qu'une génération n'a jamais fait »). En contrepartie, l'écran d'administration qu'elle
émet importe `@/api/client` (`Patron.vue.jinja:3`) : **`npm run typecheck` est cassé après
chaque génération** tant que `rbs generate client` n'a pas été relancé à la main. La
commande se contente d'afficher le geste à faire.

Le mainteneur a demandé le 2026-09-20 que les deux gestes n'en fassent plus qu'un.

## Considered Options

**Déduire le client de la spec d'entité.** La commande tient déjà les DTO qu'elle vient
d'écrire ; elle pourrait en rendre les interfaces et les méthodes et les insérer par une
ancre. Instantané, et cohérent avec le refus de compiler. Écarté : le client aurait alors
deux producteurs tirant de **deux sources différentes** — le contrat pour `generate client`,
la spec d'entité pour `generate crud`. Le jour où utoipa et le rendu TypeScript divergent
d'un champ, rien ne dit lequel ment. C'est exactement la **dérive** que le projet nomme
ailleurs, et la condition à laquelle l'**écran patron** avait été accepté avec deux
producteurs — *une seule forme* — ne serait pas tenue ici.

**Ne rien changer.** Écarté : laisse le typecheck cassé par construction après chaque
entité.

## Consequences

**Chaque `rbs generate crud` paie une recompilation incrémentale.** Par construction : le
module vient d'être ajouté, donc le cache du contrat (`target/rbs/`) est invalide au moment
où la commande en a besoin. Le coût est payé une fois par entité créée — et non à chaque
`rbs routes`, que le cache, lui, rend instantané.

**La règle « une génération ne compile pas » tombe**, et avec elle la possibilité d'engendrer
un CRUD sur un projet qui ne compile pas. C'est acceptable parce qu'un projet qui ne compile
pas n'a de toute façon pas de contrat à offrir à son frontend — mais c'est un cas où la
commande refusait auparavant de dépendre de l'état du code, et où elle en dépend désormais.

**La régénération reste conditionnelle.** Le client n'est réécrit que si le projet en porte
déjà un : `generate crud` n'invente pas un répertoire `frontend/src/api` que l'utilisateur
n'a pas demandé. C'est la règle déjà appliquée aux ancres `admin_routes` et `admin_rail`,
qu'on saute plutôt que d'exiger.
