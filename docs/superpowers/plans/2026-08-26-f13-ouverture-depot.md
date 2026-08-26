# F13 — Ouverture du dépôt

## Décision

Aucun code à écrire : le dépôt est déjà public et `origin/main` porte le CLI complet. La
tâche est une vérification, et son critère — « installation possible par `cargo install
--git` » — se prouve en installant depuis l'URL publique dans une racine isolée, puis en
exerçant le binaire obtenu.

La racine isolée n'est pas un détail : `~/.cargo/bin/rbs` existe déjà sur la machine, et
installer par-dessus effacerait la distinction entre « le dépôt public est installable »
et « une version locale traînait là ».

## Étapes

1. `cargo install --git <url> rbs-cli --root <tmp>` — la forme que le README documente.
2. Exercer le binaire produit : version, commandes exposées, une génération réelle.

## Preuve

Le critère porte sur l'installation, et elle réussit. Deux constats la débordent et
appartiennent à F3, dont c'est le sujet :

- la forme *nue* `cargo install --git <url>`, que le README ne donne pas, échoue :
  `cargo` fouille tout le dépôt et bute sur les binaires de `examples/` ;
- un projet généré déclare `rbs-core = "0.1.0"`, absent de crates.io — le quickstart
  documente le `--core-path` qui répond à cela, le README ne le mentionne pas.
