# S4 — Le noyau cesse de nommer PostgreSQL

**Conception.** Le message de `ConnectError` est le seul endroit du noyau qui suppose un
moteur, et c'est un type public : le corriger est la raison d'ouvrir `rbs-core`, seule
tâche du jalon à y être autorisée. Le moteur se déduit du schéma de l'URL — le noyau ne
lit pas `[package.metadata.rbs]`, qui appartient au projet et non à la bibliothèque. Le
helper reste privé : `missing_docs` n'a alors rien à exiger, et rien de public n'est ajouté
à une API que `v1.0` gèlera.

SQLite change la nature de la phrase et non seulement son nom : il n'a pas de serveur à
démarrer, mais un fichier à rendre accessible en écriture.

Le portage traverse ensuite deux endroits que le critère « `doctor` ne suppose plus
PostgreSQL » impose : le binaire de migration engendré, dont la commande `version` exécute
`SHOW server_version_num`, et `doctor` lui-même, qui sonde un port TCP et compare à un
plancher unique.

Le plancher devient propre à chaque moteur, et chacun a une cause vraie plutôt qu'un
chiffre repris d'ailleurs :

| Moteur | Plancher | Cause |
|---|---|---|
| PostgreSQL | 14 | La plus ancienne encore maintenue, tranché en `S2` |
| MySQL | 8.0 | `FOR UPDATE SKIP LOCKED`, dont `S3` dépend |
| SQLite | 3.35 | `UPDATE … RETURNING`, dont le dépilage de `S3` dépend |

## Étapes

1. `rbs-core/src/db.rs` : le moteur déduit du schéma, le message qui le nomme, et la
   phrase propre à SQLite. Tests : un message par moteur.
2. `rbs-core/src/config.rs` : le commentaire de `url` cesse de dire « PostgreSQL ».
3. `templates/project/migration/src/main.rs.jinja` : `version` à trois branches.
4. `doctor/base.rs` : moteur lu des métadonnées, `host_and_port` acceptant `mysql://`,
   SQLite sondé par son fichier, plancher par moteur.
5. Preuves : les trois messages du noyau ; `rbs doctor` vert sur un projet de chaque
   moteur ; morsure d'un plancher abaissé faisant virer le `✓` au `✗`.
