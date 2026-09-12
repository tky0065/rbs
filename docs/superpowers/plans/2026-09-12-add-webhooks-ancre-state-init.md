# Plan — `rbs add webhooks` refuse une ancre `state_init` posée sous `core:`

Tâche 73 d'`IMPROVE.md` (Easy). Un projet d'avant 1.5.0 porte `// <rbs:state_init>`
après `core: CoreState::new(db, config)` ; le fragment `webhooks` y écrit
`Sender::from_config(&config)?` et le projet ne compile plus. Le CLI doit le refuser à
la planification, avec le bloc à remonter affiché.

## Design

- `manifest::DeclaredInsertion` gagne `before: Option<String>` — la ligne (préfixe, après
  `trim`) que l'insertion doit précéder dans le fichier de l'ancre. Seul
  `webhooks/feature.toml` le pose : `before = "core: CoreState::new("`.
- `anchors::precedes(source, &anchor, before) -> Result<(), Misplaced>` : `Err` quand une
  ligne commençant par `before` précède la balise ouvrante ; `Ok` si l'ancre ou la ligne
  est absente (l'ancre absente est déjà l'affaire d'`insert`, la ligne absente est un
  `state.rs` réécrit que rien ici ne sait juger). `Misplaced` porte l'ancre, la ligne et
  le bloc tel qu'il est dans le fichier.
- `plan::Builder::require_before(&anchor, before)` lit le contenu courant projeté et rend
  `plan::Error::MalPlacee(Misplaced)`.
- `add::Error::remedy()` affiche « dans src/state.rs, remontez ce bloc au-dessus de
  `core: CoreState::new(` : … » avec le bloc.
- Hors périmètre : relocalisation par `upgrade` ou `doctor --fix` (candidat backlog).

## Étapes (TDD)

1. `anchors.rs` : tests rouges de `precedes` (dessous → Err avec le bloc ; dessus → Ok ;
   ligne absente → Ok ; ancre absente → Ok), puis `Misplaced` + `precedes`.
2. `plan/mod.rs` : `Error::MalPlacee`, `Builder::require_before`.
3. `manifest.rs` : `before` facultatif, test d'analyse.
4. `add/installation.rs` : appel avant `insert` ; `add/mod.rs` : `remedy`, test unitaire
   rouge (projet dont `state.rs` est remis à la disposition d'avant 1.5.0, plan de
   `webhooks` refusé, remède nommant la ligne et le bloc) puis vert.
5. `webhooks/feature.toml` : `before`.
6. `tests/integration_add.rs` : `rbs add webhooks` sur le même projet, sortie 1, stderr
   nomme l'ancre et la ligne, stdout porte le bloc, arbre intact (sans Docker).
7. `CHANGELOG.md` / `.fr.md` (1.5.0), `docs/docs/cli/add.md` EN + FR : une phrase.
8. `cargo fmt --all --check`, `clippy --workspace --all-targets -D warnings`,
   `cargo test -p rbs-cli --lib`, `cargo test -p rbs-cli --test integration_add`,
   `integration_docs`.
