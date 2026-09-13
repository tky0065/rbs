# `CLAUDE.md` engendré — plan d'implémentation

> **Pour les agents :** SOUS-SKILL REQUIS : superpowers:executing-plans. Étapes à cocher (`- [ ]`).

**But :** `rbs new` écrit un `CLAUDE.md` d'une ligne, `@AGENTS.md` ; `rbs upgrade` le crée
s'il manque et ne réécrit jamais un existant.

**Architecture :** deux constantes dans `crates/rbs-cli/src/agents.rs` — le nom du fichier
et son contenu — lues par `new.rs` (écriture après `AGENTS.md`) et par `upgrade.rs`
(`builder.create` seulement si `!builder.exists`). Pas de template du squelette : un
`--template-dir` qui ne porte que le squelette doit recevoir le fichier comme il reçoit
`AGENTS.md`, et `upgrade` a besoin du même contenu.

**Spec :** `docs/superpowers/specs/2026-09-13-lot-features-p2-design.md`, section « 42 ».

## Contraintes globales

- Contenu exact : `@AGENTS.md\n`.
- `upgrade` ne réécrit **jamais** un `CLAUDE.md` présent, quel que soit son contenu.
- Les cinq exemples reçoivent leur `CLAUDE.md`.
- Doc EN + FR dans le même commit ; `npm run build` vert.

---

### Tâche 1 : `rbs new`

**Fichiers :** `crates/rbs-cli/src/agents.rs`, `crates/rbs-cli/src/new.rs`.

- [ ] **Étape 1 : test rouge** (dans `new.rs`, près de `a_new_project_carries_its_agents_file`) :

```rust
    /// Claude Code ne lit pas `AGENTS.md` de lui-même : il suit l'import d'un `CLAUDE.md`.
    #[test]
    fn a_new_project_carries_a_claude_file_importing_its_agents_file() {
        let parent = parent();

        let project = create(&options("mon-api"), parent.path()).expect("le projet doit se créer");

        assert_eq!(read(&project.root.join("CLAUDE.md")), "@AGENTS.md\n");
    }
```

  et, dans `a_template_dir_holding_only_the_skeleton_still_gets_its_agents_file`, une
  assertion que `CLAUDE.md` existe aussi.
- [ ] **Étape 2 :** `cargo test -p rbs-cli --lib new::tests::a_new_project_carries_a_claude` → FAIL.
- [ ] **Étape 3 :** dans `agents.rs` :

```rust
/// Le fichier que Claude Code lit, à la racine du projet : il ne lit `AGENTS.md` que par
/// l'import que celui-ci déclare.
pub(crate) const CLAUDE: &str = "CLAUDE.md";

/// Le contenu entier de [`CLAUDE`] : l'import, et rien qui puisse diverger du guide.
pub(crate) const CLAUDE_CONTENU: &str = "@AGENTS.md\n";
```

  dans `new.rs::create`, après l'écriture d'`AGENTS.md`, la même écriture pour `CLAUDE`
  (même nettoyage sur échec), et `files: rendus.len() + 2`.
- [ ] **Étape 4 :** `cargo test -p rbs-cli --lib` entier → vert.

### Tâche 2 : `rbs upgrade`

**Fichier :** `crates/rbs-cli/src/upgrade.rs`.

- [ ] **Étape 1 : tests rouges**

```rust
    /// Le parc déjà engendré n'a pas de `CLAUDE.md` : la mise à niveau le lui donne.
    #[test]
    fn upgrading_creates_a_missing_claude_file() {
        let (_parent, root) = project(None);
        fs::remove_file(root.join("CLAUDE.md")).expect("le fichier existe");

        let planned = plan_for_with(&Options { directory: root.clone(), force: true }, "2.0.0")
            .expect("le plan doit se calculer");

        let projete = planned.plan.files().iter()
            .find(|file| file.path == "CLAUDE.md")
            .expect("le plan crée CLAUDE.md");
        assert_eq!(projete.after, "@AGENTS.md\n");
    }

    /// Un `CLAUDE.md` présent appartient au développeur, même s'il n'importe plus rien.
    #[test]
    fn upgrading_never_rewrites_an_existing_claude_file() {
        let (_parent, root) = project(None);
        fs::write(root.join("CLAUDE.md"), "# nos règles\n").expect("l'écriture aboutit");

        upgrade(&root, futur());

        assert_eq!(fs::read_to_string(root.join("CLAUDE.md")).expect("lisible"), "# nos règles\n");
    }
```

- [ ] **Étape 2 :** `cargo test -p rbs-cli --lib upgrade::tests` → le premier FAIL.
- [ ] **Étape 3 :** après le bloc `AGENTS.md` de `plan_for_with` :
  `if !builder.exists(crate::agents::CLAUDE)? { builder.create(crate::agents::CLAUDE, crate::agents::CLAUDE_CONTENU)?; }` ;
  en-tête du module mis à jour (la commande écrit aussi un `CLAUDE.md` absent).
- [ ] **Étape 4 :** `cargo test -p rbs-cli --lib` entier → vert.

### Tâche 3 : exemples

- [ ] `cargo test -p rbs-cli --test integration_examples` → FAIL (5 × `CLAUDE.md` absent).
- [ ] `printf '@AGENTS.md\n' > examples/<ex>/CLAUDE.md` pour les cinq, `git add`.
- [ ] `cargo test -p rbs-cli --test integration_examples -- --include-ignored` → vert.

### Tâche 4 : documentation (EN + FR)

- `guides/agents.md` : la phrase « rbs n'engendre aucun fichier propre à un outil » tombe ;
  une section courte dit pourquoi `CLAUDE.md` existe (Claude Code ne lit `AGENTS.md` que
  par l'import) et qu'il appartient au développeur ; tableau « Qui écrit quoi » : `new`
  l'écrit, `upgrade` le crée s'il manque sans jamais le réécrire.
- `cli/new.md` : transcriptions 21 → 22 fichiers (20 → 21 sous SQLite), liste des fichiers
  (`blog/CLAUDE.md`), « vingt » → « vingt-deux ».
- `getting-started.md`, `tutorials/setup.md` : 21 → 22 fichiers.
- `cli/upgrade.md` : ce que la commande écrit — plus un `CLAUDE.md` absent.
- [ ] `integration_docs` (rapide) vert ; `cd docs && npm run clear && npm run build` → 0.

### Tâche 5 : vérification et commit

- [ ] fmt, clippy, `cargo test --workspace`, `integration_upgrade` et `integration_new` (lents).
- [ ] Commit `feat(new): …`, corps : pourquoi + `Vérifications :`.
