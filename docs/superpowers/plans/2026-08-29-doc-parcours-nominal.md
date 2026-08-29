# La documentation sort du contournement

**Constat.** Le parcours principal de `getting-started` impose `--core-path` et l'explique
par « la seconde conséquence de la 0.1 non publiée : sans lui le manifeste réclame
`rbs-core = "0.1.0"` sur crates.io, où il n'existe pas ». Deux versions sont désormais
parues, et cette phrase est fausse depuis hier.

La conception du jalon l'avait prévu (§5 : « le flag garde une raison d'être — développer
rbs lui-même contre un noyau non publié — mais la documentation qui le présentait comme le
contournement d'un défaut doit cesser de le faire »), et l'avait renvoyé à `U2` ou `W3`.
Ni l'une ni l'autre ne l'a fait. C'est ce lot qui le règle.

**Ce qui change, page par page :**

| Page | Ce qui est périmé |
|---|---|
| `getting-started` | Le parcours principal passe par un clone du dépôt et `--core-path`. Il doit partir de `cargo install rbs-cli` et de `rbs new` tout court |
| `cli/new` | « `--core-path` is what the walkthrough on these pages uses, because `rbs-core` 0.1.0 is not on crates.io yet » — et les invocations d'exemple portent le flag |
| `cli/new`, `cli/add`, `cli/generate` | Les blocs `[package.metadata.rbs] version = "0.1.0"` : capturés quand rbs était en 0.1.0, ils annoncent aujourd'hui une version qui n'a jamais été publiée |

**`--core-path` ne disparaît pas, il retrouve sa place.** C'est le mode dans lequel rbs se
développe lui-même : un projet engendré contre un noyau local, pour éprouver une
modification du noyau avant qu'elle soit publiée. Il cesse d'être présenté comme le
contournement d'un défaut, et devient ce qu'il est — un outil de contributeur, non une
étape du parcours d'un utilisateur.

**Les sorties se recapturent, elles ne se retouchent pas.** C'est la règle du dépôt depuis
`J3`, et le second critère de `Y1` l'a rappelée : un bloc de terminal vient du binaire. Le
binaire installé est en 1.0.0 ; toute sortie recapturée portera donc les bons numéros.

**La parité se mesure**, et l'instrument a une limite connue : `parite.mjs` ne compare pas
le contenu des tableaux. Si une page en porte un, sa correspondance se vérifie à la main.

## Étapes

1. Recapturer, sur le binaire 1.0.0 installé depuis crates.io, les sorties du parcours
   nominal : `rbs new` sans `--core-path`, `rbs add`, `rbs generate crud`, et les blocs de
   manifeste qui en découlent.
2. Réécrire le parcours de `getting-started` dans les deux langues : installation par
   `cargo install rbs-cli`, création sans flag, aucune mention d'un noyau non publié.
3. Réécrire ce que `cli/new` dit de `--core-path` : le mode de développement de rbs, non un
   contournement.
4. Remplacer les blocs de manifeste périmés de `cli/new`, `cli/add` et `cli/generate`.
5. Preuves : `npm run parite` avec l'instrument éprouvé d'abord ; `npm run clear && npm run
   build` → deux `[SUCCESS]` ; et la liste des blocs recapturés avec la commande qui les a
   produits.
