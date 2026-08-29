# Les README des crates publiées

**Constat.** crates.io affiche « rbs-cli v1.0.0 appears to have no README.md file ». C'est
la première chose que lit quiconque découvre la crate, juste après la publication de la
1.0.0.

**La cause est celle qui avait déjà déplacé les templates** : `cargo package` n'emporte
aucun fichier extérieur au paquet, et `include = [...]` ne lève pas cette règle. Les
`README.md` du projet vivent à la racine du dépôt ; ni `rbs-core` ni `rbs-cli` n'en
emporte, et aucun des deux manifestes ne déclare `readme`.

**Un README par crate, et non une copie du README racine.** Celui de la racine présente le
projet entier — le CLI, le runtime, la philosophie. Sur crates.io, deux paquets distincts
sont publiés, et chacun répond à une question différente : *que fait cette bibliothèque ?*
pour `rbs-core`, *que fait ce binaire ?* pour `rbs-cli`. Une copie répondrait deux fois à
la même question, et mal.

**En anglais seul.** La règle bilingue du dépôt porte sur la documentation, dont le lectorat
est celui du site ; ces deux fichiers s'adressent à crates.io. Chacun renvoie vers le site,
dont la version française existe.

**Un README ne se rétro-publie pas** : la page affiche ce qu'embarquait le paquet publié.
Il faut donc une 1.0.1 pour qu'il paraisse — un numéro de correctif, sans rupture, ce à
quoi il sert exactement.

## Étapes

1. Écrire `crates/rbs-core/README.md` : ce que le runtime donne, ce qu'il ne fait pas,
   comment on l'obtient (par `rbs new`, non à la main), et le renvoi au site.
2. Écrire `crates/rbs-cli/README.md` : les commandes, la frontière noyau/engendré, et le
   fait que le binaire s'appelle `rbs` quand la crate s'appelle `rbs-cli`.
3. Déclarer `readme = "README.md"` dans les deux manifestes.
4. Vérifier que le paquet les emporte : `cargo package --list --locked` doit lister
   `README.md` pour chacun.
5. La publication de la 1.0.1 est un geste distinct, fait après.
