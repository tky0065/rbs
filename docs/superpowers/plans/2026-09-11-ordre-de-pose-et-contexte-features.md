# Ordre de pose des fragments et contexte `features` — plan d'implémentation

**Goal :** `rbs new --with rate-limit,redis` (et `--preset full`) rend le compteur Redis, et
`--database sqlite --with docker,auth` aboutit : ce qu'un fragment sait des autres est ce que
le plan laissera, non ce que le disque portait avant lui, et l'ordre de pose suit les
dépendances plutôt que l'alphabet.

**Architecture :** `add::Options.features` remplace `feature` (la CLI `rbs add <feature>` en
passe une, `rbs new` toutes) ; `plan_for` rend `installées ∪ à poser` sous `features` ;
`resoudre` ordonne par tri topologique déterministe (Kahn, plus petit nom d'abord) sur deux
arêtes — `requires`, et « écrit le fichier porteur d'une ancre que l'autre vise » ; `Planned`
expose `poses` (nom, fichiers, migration) pour que `new` rapporte chaque fragment posé.

## Étapes (TDD)

- [x] 1. Tests rouges dans `add/mod.rs` : `rate-limit`+`redis` planifiés ensemble rendent le
      compteur Redis quel que soit l'ordre ; `docker` passe avant `mail`/`auth` sur SQLite ;
      `auth` seule pose `mail`, `rate-limit` avant elle ; deux demandes équivalentes rendent
      le même `state.rs`. — quatre tests, verts après l'étape 3.
- [x] 2. `Options.features: Vec<String>`, `Planned.poses`, contexte `features` élargi ;
      appelants (`lib.rs`, `new.rs`, tests de `dev` et `doctor`) suivent.
- [x] 3. `resoudre` : DFS des `requires` puis `ordonner` topologique ; commentaire réécrit.
- [x] 4. `new.rs::install` en un seul pipeline ; `installed` = un `InstalledFeature` par
      fragment posé ; test rouge `--with rate-limit,redis` → compteur Redis sur le disque,
      plus SQLite + `docker,auth` — deux tests verts.
- [x] 5. `cargo test -p rbs-cli --lib` : 1143 passés ; `cargo fmt --all --check` muet ;
      `cargo clippy --workspace --all-targets -- -D warnings` : Finished.
- [x] 6. `docs/docs/cli/new.md`, `docs/docs/tutorials/auth.md` et leurs jumeaux FR :
      transcripts `--with auth` et `rbs add auth`, paragraphe sur l'ordre ;
      `integration_docs` : 13 passés.
- [x] 7. `examples/blog-auth` régénéré par diff (`.env.example`, `AGENTS.md`, `Cargo.toml`,
      `config/default.toml`, `src/state.rs` — réordonnancements seuls) ;
      `integration_examples` : 19 passés ; `cargo check --tests` dans l'exemple : Finished.
- [x] 8. `cargo test -p rbs-cli --test integration_new --no-fail-fast -- --ignored` (Docker) :
      5 passés en 231 s.
- [x] 9. Commit `fix(add): rend aux fragments ce que le plan pose, et ordonne la pose par
      dépendances`.
