# V3 — Passe sur les conventions de code

## Décision

Trois critères, dont deux se mesurent et un se juge. Le troisième — « aucun commentaire
qui paraphrase le code » — ne se décerne pas soi-même : il se présente au user avec sa
méthode et son résultat.

La méthode ne peut pas être une lecture d'ensemble : 175 commentaires non-doc, l'attention
retombe avant la fin. Deux passes ciblées à la place.

1. Les commentaires **isolés** — une seule ligne, sans voisin commenté — lus un par un.
   C'est la forme qu'un commentaire paraphrasant prend presque toujours : le bloc de
   plusieurs lignes, lui, sert à porter un raisonnement.
2. Le **recouvrement lexical** de chaque bloc avec la ligne de code qui le suit, sur les
   175. Un commentaire qui redit le code en réemploie forcément les identifiants ; celui
   qui explique un pourquoi parle d'autre chose que de la ligne.

## Résultat

- `missing_docs` : 0 avertissement, et le lint est bien armé (`lib.rs:18`) — le zéro ne
  vient pas d'un lint absent.
- Feature générée : 7 fichiers de 19 à 158 lignes, aucun au-delà de ~200.
- Commentaires : 0 paraphrase. Le seul dépassement du seuil est `// <rbs:routes>`, une
  ancre citée dans une fixture de test, pas de la prose.

Aucune correction : la passe constate, elle n'avait rien à supprimer.
