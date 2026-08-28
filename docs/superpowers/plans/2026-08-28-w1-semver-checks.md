# W1 — `cargo-semver-checks` en CI

**Conception.** L'outil répond à une question qu'aucun instantané textuel ne pose : *ce
changement est-il une rupture ?* Un diff d'API répond « l'API a-t-elle changé ? », question
à laquelle un ajout compatible répond « oui » pour rien.

Il n'était pas utilisable avant aujourd'hui : il lui faut une version publiée à laquelle se
comparer, et `rbs-core 0.4.0` vient de paraître. C'est la seconde raison de l'ordre
`U → W`, indépendante de la première.

**Portée : `rbs-core` seule.** `rbs-cli` publie un binaire ; que sa bibliothèque soit
visible est un détail de construction et non une offre (conception §2.4). L'y soumettre
ferait échouer la CI sur des changements que la promesse ne couvre pas.

**L'outil arrive par `obi1kenobi/cargo-semver-checks-action`**, sur arbitrage : elle
télécharge un binaire précompilé quand `cargo install cargo-semver-checks` coûte deux à
trois minutes à chaque run, sur le job déjà le plus long. Le prix est une action tierce de
plus.

**Le point à trancher sur pièce est le jeu de features.** `rbs-core` porte quatre pilotes
plus `auth`, et sa surface publique change avec eux : les features par défaut ratent `auth`,
`--all-features` analyse des combinaisons qu'aucun projet réel n'active. Le manifeste
déclare déjà `[package.metadata.docs.rs] all-features = true`, ce qui penche pour la
surface complète — à confirmer en lançant les deux et en comparant ce que chacun voit.

**L'instrument se prouve en le mordant**, comme `V2`, `J3`, `O2` et la garde de version :
une variante retirée d'`Error` doit rendre un verdict rouge **nommant l'item**. Un
instrument dont on n'a jamais observé l'échec ne se distingue pas d'un instrument absent.

## Étapes

1. Installer `cargo-semver-checks` en local et le lancer sur `main` tel quel : le verdict
   doit être vert. Comparer ce que voient les features par défaut et `--all-features`,
   et retenir le jeu qui couvre la surface publique réelle.
2. Mordre : retirer une variante d'`Error`, relancer, **lire la sortie** et vérifier
   qu'elle nomme l'item. Restaurer.
3. Ajouter le step à `ci.yml`, dans le job Linux, après `cargo test`. Ne pas toucher aux
   autres steps.
4. Preuves : le verdict vert sur l'état courant ; la sortie de la morsure, citée mot pour
   mot ; `actionlint` sur les workflows.
