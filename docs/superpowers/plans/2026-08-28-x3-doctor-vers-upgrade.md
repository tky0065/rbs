# X3 — `doctor` renvoie vers `rbs upgrade`

**Conception.** Pas un contrôle de plus : le contrôle `versions` sait déjà diagnostiquer
l'écart. Ce qu'il ne sait pas, c'est **nommer le geste**.

Son conseil actuel — « alignez le projet sur rbs 1.0.0, ou relancez la commande avec le CLI
qui l'a généré » (`doctor/versions.rs:90`) — a été écrit quand le geste n'existait pas. Il
décrit un alignement à la main, ce qui était la seule voie. `rbs upgrade` existe désormais,
et un écart qui ne nomme pas sa commande est un utilisateur qui ne la lancera jamais.

**La tâche est étroite et doit le rester** : c'est le texte d'un conseil, pas un contrôle
neuf, pas une logique de diagnostic nouvelle. Le contrôle `versions` compare déjà les trois
numéros et rend le bon verdict ; seul son conseil change.

**Le second critère est une garde contre l'excès de zèle** : un projet aligné rend une ligne
`✓` qui ne doit pas bouger. Nommer une commande de mise à niveau à qui n'a rien à mettre à
niveau serait un bruit, et la sortie de `doctor` vit de sa concision.

**La direction de l'écart compte.** `upgrade` ne redescend pas un projet : `X1` le refuse
explicitement, en nommant les deux versions. Le conseil ne doit donc nommer `rbs upgrade`
que dans le sens où la commande peut quelque chose — projet en retard sur le CLI. Un projet
en avance garde un conseil qui parle du CLI, non de la commande.

## Étapes

1. Lire `crates/rbs-cli/src/doctor/versions.rs` en entier, et les tests qui couvrent déjà
   les cas d'écart. Les lignes `✓` sont deux tests : les écrire d'abord.
2. Changer le conseil du cas « projet en retard sur le CLI » pour qu'il nomme
   `rbs upgrade`. Ne pas toucher au verdict, ni aux autres cas.
3. Vérifier que le cas « projet en avance » garde un conseil cohérent avec le refus
   qu'`upgrade` oppose à ce sens-là.
4. Preuves : les deux critères joués, dont la sortie réelle de `rbs doctor` sur un projet
   en retard — capturée sur le binaire, non recopiée à la main ; et la ligne `✓` d'un
   projet aligné, inchangée.
