# `rbs migrate` — plan

**But :** `rbs migrate up|down|status|new <nom>` pilote les migrations d'un projet généré,
avec une sortie que rbs contrôle de bout en bout.

**Approche :** deux mécaniques distinctes derrière une même commande.

`new` ne parle à personne : il pose un fichier de migration vide et le monte dans
`<rbs:migration_modules>` et `<rbs:migrations>` par le moteur d'ancres (D9), avec
l'horodatage de `generate/migration.rs`. Ni compilation ni base.

`up`, `down` et `status` enveloppent la crate `migration` du projet. Celle-ci est une lib
pure aujourd'hui : le template gagne donc un binaire, qui applique `MigratorTrait` et écrit
un état machine sur sa sortie standard — une ligne `applied|pending\t<nom>` par migration.
rbs lit le `.env` du projet, transmet `RBS_DATABASE__URL` au sous-processus et rend.

La lisibilité est le travail de rbs, pas celui du projet généré. C'est ce qui rend le
critère visuel testable sans base : le rendu est une fonction pure sur des états déjà lus.

**Écarté :** la feature `cli` de `sea-orm-migration` — clap et dotenvy dans le projet de
l'utilisateur pour une sortie verbeuse à parser. L'interrogation directe de
`seaql_migrations` par rbs — sqlx et tokio dans le CLI, et deux sources de vérité sur la
liste des migrations.

**Hors périmètre :** le working tree sale (E4), l'affichage du plan et `--dry-run` (E5).
`migrate new` écrit directement, comme `generate` le fait depuis D10.

## Fichiers

| Chemin | Rôle |
|---|---|
| `crates/rbs-cli/src/dotenv.rs` | lecture d'un `.env` en `Vec<(String, String)>` — D12 le réutilise |
| `crates/rbs-cli/src/migrate/mod.rs` | `Options`, aiguillage des quatre sous-commandes |
| `crates/rbs-cli/src/migrate/etat.rs` | `Etat { nom, appliquee }`, analyse du TSV du sous-processus |
| `crates/rbs-cli/src/migrate/rendu.rs` | `status(&[Etat]) -> String`, colonnes alignées |
| `crates/rbs-cli/src/migrate/nouvelle.rs` | `migrate new` : fichier vide et double ancre |
| `templates/project/migration/src/main.rs.jinja` | binaire de la crate `migration` |
| `templates/migration/vide.rs.jinja` | migration sans table, `up`/`down` à remplir |

## Étapes

- [x] `dotenv.rs` : lecture, commentaires, `export`, guillemets, ligne sans `=` ignorée.
- [x] `migrate/etat.rs` : analyse du TSV, ligne inconnue → erreur nommant la ligne.
- [x] `migrate/rendu.rs` : les deux états portent des marqueurs distincts, y compris hors
      TTY où `console` retire la couleur — c'est le critère ✓ de la tâche.
- [x] `templates/project/migration/` : `[[bin]]`, dépendance `tokio`, `main.rs` qui
      exécute `up`, `down` ou `status` selon son argument.
- [x] `templates/migration/vide.rs.jinja` et `migrate/nouvelle.rs` : écriture et montage,
      sur un projet déroulé par `new::creer`.
- [x] `migrate/mod.rs` : racine du projet, `.env` lu, `cargo run -p migration`, erreur du
      sous-processus relayée sans être avalée. `stderr` reste hérité pour que la
      progression de cargo, qui compile la crate au premier appel, reste visible.
- [x] Câblage de `main.rs` : les quatre bras, code de sortie 1 sur échec.
- [ ] Test lourd `#[ignore]` via testcontainers — **non fait ici** : D13 monte
      `testcontainers`, et son test d'intégration CRUD exerce déjà `migrate up`. La
      preuve de bout en bout a été faite à la main contre un PostgreSQL 18 en conteneur.

## Preuves attendues

- ✓ *`status` distingue visuellement appliqué / en attente* — test unitaire sur `rendu`
  (marqueurs distincts sans couleur, libellés alignés), et la sortie réelle contre
  PostgreSQL 18, validée de visu.
- Non régression : `cargo test --workspace -- --include-ignored`, `clippy -D warnings`,
  `fmt --check`.
