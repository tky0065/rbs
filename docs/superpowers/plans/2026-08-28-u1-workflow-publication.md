# U1 — Workflow de publication sur tag

**Conception.** Le livrable n'est pas la publication, c'est la **garde**. Publier tient en
deux lignes de YAML ; ce qui demande du soin est le refus d'un tag `v0.4.0` posé sur un
workspace resté à `0.1.0` — crates.io ne reprend rien, et la version fautive resterait.

La garde ne peut pas se prouver en posant un tag. Elle vit donc dans un script,
`.github/scripts/garde-version.sh`, que `release.yml` appelle et qu'un step de `ci.yml`
**mord sur ses deux cas**. C'est la méthode que `V2`, `J3` et `O2` ont employée pour leurs
instruments de mesure : un instrument dont on n'a pas vu l'échec ne se distingue pas d'un
instrument absent.

Elle lit la version par `cargo pkgid`, non par un `grep` sur `Cargo.toml` : l'outil officiel
plutôt qu'un parseur TOML fait main. La sortie est
`path+file:///…/crates/rbs-core#0.1.0`, dont le numéro s'extrait par `sed 's/.*[@#]//'` —
la forme `…#nom@version` apparaissant quand le répertoire ne porte pas le nom du paquet.

Elle compare le tag aux versions des **deux crates publiées**, et non au seul
`[workspace.package]`. Elles en héritent aujourd'hui par `version.workspace = true` ; le
jour où l'une en sortirait, une garde qui ne lit que le workspace deviendrait silencieusement
fausse. Le coût est d'une ligne.

L'ordre `rbs-core` puis `rbs-cli` n'est pas une dépendance Cargo — `rbs-cli` ne dépend pas
de `rbs-core`. C'est que `new.rs:238` engendre `rbs-core = "<version du CLI>"` : un CLI
installable avant que le noyau existe produirait des projets qui ne résolvent pas. Pour la
même raison, aucune attente d'indexation n'est nécessaire entre les deux publications.

La garde est le premier step après la toolchain, **avant le dry-run** : le critère exige un
échec « avant toute publication », et un dry-run est déjà du travail engagé sur un tag faux.

Le secret `CARGO_REGISTRY_TOKEN` est une action hors dépôt (conception §2.8). Tout ce lot
est prouvable sans lui ; la case attend le run réel de `U3`.

**Hors périmètre, à signaler dans le message de commit** : si `rbs-cli` échoue après que
`rbs-core` a publié, le workflow n'est pas rejouable tel quel, crates.io ne reprenant pas une
version. Aucun critère ne le demande, et un `continue-on-error` masquerait l'échec au lieu
de le traiter.

## Étapes

1. Écrire `.github/scripts/garde-version.sh` : un argument, le nom du tag. Refuse un tag mal
   formé (`0.4.0` sans `v`, `v0.4`), puis compare le numéro aux versions de `rbs-core` et
   `rbs-cli`. En cas d'écart, écrit **les deux numéros** sur `stderr` et sort en 1. `set -eu`.
2. Écrire `.github/workflows/release.yml` : `on: push: tags: ['v*']`,
   `permissions: contents: read`, un job — checkout, `dtolnay/rust-toolchain@stable`,
   `Swatinem/rust-cache@v2`, la garde, `cargo publish --dry-run --locked` des deux crates,
   puis `cargo publish --locked` des deux, `CARGO_REGISTRY_TOKEN` en `env`.
   Reprendre les versions d'actions de `ci.yml` (`actions/checkout@v7`).
3. Ajouter à `ci.yml` un step qui mord la garde : le tag concordant passe, un tag discordant
   échoue. Le step échoue si l'un des deux cas ne rend pas le verdict attendu.
4. Preuves : les deux cas de la garde joués en local, la sortie de l'échec lue et vérifiée
   nommante ; `cargo publish --dry-run --locked` des deux crates ; `actionlint` sur les deux
   workflows s'il est disponible, sinon une validation YAML.
