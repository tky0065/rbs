# U3 — Publication réelle de 0.4.0

**Conception.** Cinq changements, puis un geste qui ne se reprend pas.

Le lot précédent a livré la mécanique ; celui-ci ne fait que la déclencher, à ceci près
qu'il porte le premier acte irrévocable du dépôt. crates.io ne retire pas une version : ce
qui part reste, et `yank` ne fait que la masquer aux résolutions neuves.

**Le numéro se pose à cinq endroits, non à un.** `Cargo.toml` porte
`workspace.package.version`, mais les cinq `Cargo.lock` du dépôt inscrivent le numéro
résolu — celui de la racine pour les deux crates, ceux des quatre exemples parce qu'ils
déclarent `rbs-core` par `path`. Les régénérer n'est pas cosmétique : `release.yml` publie
en `--locked`, et un verrou resté en `0.1.0` ferait échouer la publication après le tag,
c'est-à-dire au pire moment.

**`NOYAU_PUBLIE` bascule dans le commit taggé, non après.** La constante vaut pour le
binaire qui la porte : `rbs-cli` 0.4.0 est installé dans un monde où `rbs-core` 0.4.0
existe. La laisser à `false` publierait un `doctor` qui diagnostique en échec un projet
correct. Le basculement ne casse rien, `check_with` étant paramétré et ses deux chemins
couverts par les tests de part et d'autre (`doctor/versions.rs:191` et `:232`).

**Le CHANGELOG se date ici.** Le lot précédent a titré son entrée `[Unreleased]` /
`[Non publié]` pour satisfaire « aucune entrée pour une version non publiée ». La
publication est ce qui autorise le numéro.

**Le tag part de `main`**, sur arbitrage : une version publiée doit correspondre à l'état
de la branche principale, sans quoi `W`, `X` et `Y` partiraient d'une base qui diverge de
ce qui est sur crates.io. Conséquence assumée : le push expose d'un coup les commits que
`main` local avait d'avance sur `origin/main`.

**Vérifié avant d'engager quoi que ce soit** : `rbs-core` et `rbs-cli` sont libres au
registre (`cargo info` les y déclare introuvables). Le nom court `rbs` est pris par un
tiers sans rapport — d'où `cargo install rbs-cli`, jamais `cargo install rbs`.

## Étapes

1. `workspace.package.version` à `0.4.0`, puis régénérer les cinq `Cargo.lock`.
2. `## [Unreleased]` / `## [Non publié]` → `## [0.4.0] — 2026-08-28`, dans les deux langues.
3. `NOYAU_PUBLIE` à `true` dans `crates/rbs-cli/src/doctor/versions.rs`.
4. Vérifier avant de pousser : `cargo test --workspace --all-features`, `clippy`, `fmt`,
   `npm run parite`, et surtout `garde-version.sh v0.4.0` qui doit **passer** là où il
   refusait, et `v0.1.0` qui doit désormais être refusé — la garde change de verdict, ce
   qui est la preuve qu'elle lit vraiment le dépôt.
5. Commit, merge dans `main`, push, tag `v0.4.0`, push du tag. Suivre le run.
6. Preuves des trois critères, après publication : `cargo install rbs-cli` depuis le
   registre sans `--git` ; `rbs new` sans `--core-path` puis `cargo build` ; `doctor` sur ce
   projet, qui ne doit plus le diagnostiquer en échec.
