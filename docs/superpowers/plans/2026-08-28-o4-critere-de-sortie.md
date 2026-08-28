# O4 — Critère de sortie du jalon

## Décision

Le premier critère, pris à la lettre, n'est pas rempli : `git diff` depuis la fin du
second tour du moule rend 18 fichiers et 666 lignes sur `crates/rbs-core/`. Un seul commit
les porte — la migration des identifiants vers l'anglais du 2026-08-28, qui n'appartient à
aucun lot et que l'encadré de tête du backlog acte déjà.

**Le repère est donc déplacé à ce commit, sur arbitrage demandé.** C'est un changement de
critère et non une preuve, et il se consigne comme tel, à la manière du premier critère de
l'exemple compilé en CI. Ce que le critère voulait établir — qu'aucun lot d'intégration n'a
touché le noyau — reste mesuré, et vaut `0 ligne`.

## Constat

Le second critère a trouvé ce qu'il était fait pour trouver. Sur un projet fraîchement
engendré portant les trois features, `cargo test` échoue :
`the_s3_backend_builds_without_touching_the_network` cherche `StockageS3` dans la
représentation de debug, quand la migration a renommé le type `S3Storage`. Le fragment
livre donc à tout utilisateur de `rbs add storage` un test qui tombe.

Il est passé au travers parce que la CI compile les exemples sans lancer leurs tests, qui
demandent une base. Aucun `cargo test` n'avait été joué sur un projet doté du stockage
depuis la migration.

## Étapes

1. Corriger l'assertion dans le fragment et dans l'exemple, qui doivent rester identiques
   sous peine de faire dériver la comparaison.
2. Rejouer `cargo test` sur un projet engendré des trois features, puis sur l'exemple
   contre une base réelle.

## Preuve

`git diff --stat <migration>..HEAD -- crates/rbs-core/` vide ; clippy, fmt et `cargo test`
verts sur un projet des trois features ; `integration_examples` inchangé ; suite du dépôt
inchangée hors le test corrigé.
