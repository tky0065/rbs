# Y2 — Publication de 1.0.0

**Conception.** Le même geste qu'à `U3`, avec une différence qui change tout : la 0.4.0
n'engageait rien, la 1.0.0 engage la compatibilité à vie.

**Trois choses basculent avec le numéro**, et aucune n'est cosmétique :

1. **`semver-checks` redevient vert.** Il est rouge depuis la pose des `#[non_exhaustive]`,
   qui est une rupture réelle mesurée contre la 0.4.0 parue. En montée majeure, la rupture
   est déclarée : le verdict passe à `major change` et l'outil cesse de vérifier — ce qui
   est correct, tout étant permis d'une majeure à l'autre. Il reprendra son office contre
   la 1.0.0 dès qu'elle sera parue.
2. **Le test de complétude des notes cesse de passer à vide.** `PUBLIEE` vaut 0.4.0, `CLI`
   vaudra 1.0.0 : le saut existe, et `notes/1.0.0.md` doit être là. Elle l'est — écrite par
   avance et éprouvée en la retirant.
3. **La garde du workflow change de verdict** : `v1.0.0` devient concordant, `v0.4.0` cesse
   de l'être.

**Le numéro se pose aux mêmes cinq endroits qu'à `U3`**, plus les quatre exemples. Ceux-ci
portent `[package.metadata.rbs].version` — la version qui les a engendrés —, et le contrôle
de non-dérive les signalera sinon. C'est le second passage de ce qui est, dans un projet
d'utilisateur, le travail de `rbs upgrade`.

**La leçon d'`U3` est retenue** : ce contrôle a rattrapé les exemples **avant** qu'aucun tag
ne soit posé. Rien ne part tant que `cargo test --workspace` n'est pas vert.

**Le CHANGELOG date son entrée 1.0.0.** Elle dit la promesse qui commence et la rupture qui
la paie — les mêmes que la note de migration, pour un lecteur qui n'aura pas lancé
`upgrade`.

## Étapes

1. `workspace.package.version` à `1.0.0`, les cinq `Cargo.lock` régénérés, et
   `[package.metadata.rbs].version` des quatre exemples aligné.
2. Entrée `## [1.0.0] — 2026-08-29` dans `CHANGELOG.md` et `CHANGELOG.fr.md`, avec le lien
   de bas de page.
3. Vérifier avant tout geste irréversible : `cargo test --workspace --all-features`,
   `clippy`, `fmt`, `npm run parite`, `garde-version.sh v1.0.0` qui doit passer et
   `v0.4.0` qui doit être refusé, et `cargo semver-checks` qui doit redevenir vert.
4. Commit, puis **reconfirmation du mainteneur** avant le tag.
5. Merge dans `main`, push, tag `v1.0.0`, push du tag, suivi du run.
6. Preuves : `cargo install rbs-cli` depuis le registre → 1.0.0 ; `rbs new` sans
   `--core-path` puis `cargo build` → vert sur `rbs-core 1.0.0` résolu depuis le registre.
