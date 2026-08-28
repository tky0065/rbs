# X1 — `rbs upgrade`

**Conception.** La commande la plus étroite des trois qui étaient ouvertes : elle met à jour
le manifeste et rien d'autre.

**Elle n'écrit que dans `Cargo.toml`.** Deux valeurs : la version de `rbs-core` en
`[dependencies]`, et `[package.metadata.rbs].version`. Ni re-rendu des fichiers restés
identiques à leur génération, ni insertion dans les ancres — c'est la frontière du projet,
« le CLI génère tout ce que le développeur voudra lire ou modifier » : ce code lui
appartient dès qu'il est écrit, et une commande qui le réécrirait détruirait son travail
sans qu'il l'ait demandé nommément.

**La séquence obligatoire s'appuie sur ce qui existe** — lire → planifier → vérifier →
afficher → appliquer : `metadata::read` lit, `plan::Builder` planifie, `git::modified_files`
exige l'arbre propre, `plan::render` affiche. Rien de tout cela n'est à réécrire.

**La version du CLI se paramètre**, `upgrade_with(root, version)` avec `CARGO_PKG_VERSION`
pour seul défaut. Sans quoi le premier critère — « projet en 0.4.0 lu par un binaire
1.0.0 » — ne serait prouvable qu'après la publication de la 1.0.0, et `X2` comme `X3` en
dépendent. C'est le motif de `doctor/versions.rs`, dont il a permis de couvrir les deux
chemins de part et d'autre du basculement de `NOYAU_PUBLIE`.

**Une extension de `plan/action.rs` est nécessaire et doit rester une extension.**
`PatchToml` sait inscrire une feature, déclarer une dépendance, activer une feature sur une
dépendance existante — pas changer la version d'une dépendance déjà déclarée. Ce module est
partagé avec `rbs add` : **ajouter une variante, ne modifier aucune existante.**

**Le refus est aussi important que la mise à jour.** Un projet dont la version est
postérieure à celle du binaire n'est pas un cas d'erreur exotique : c'est l'utilisateur qui
a deux CLI installés. Le message nomme les deux numéros, faute de quoi il ne saura pas
lequel des deux corriger.

## Étapes

1. TDD : les quatre lignes `✓` sont les quatre tests. Les écrire d'abord, les voir échouer.
2. Ajouter à `PatchToml` la variante qui change la version d'une dépendance déclarée, et
   son application. Ne toucher à aucune variante existante.
3. Écrire `crates/rbs-cli/src/upgrade.rs` : `upgrade_with(root, version)` déroulant la
   séquence, `upgrade(root)` n'étant que le défaut sur `CARGO_PKG_VERSION`.
4. Déclarer la variante `Upgrade` dans `cli.rs` et la brancher.
5. Preuves : les quatre critères joués séparément, dont le `git diff` d'un projet réel après
   coup, qui ne doit montrer que `Cargo.toml`.
