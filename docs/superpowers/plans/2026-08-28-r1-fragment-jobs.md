# R1 — Manifeste du fragment `jobs`, table et section `[jobs]`

**Conception.** `jobs` est un fragment ordinaire — `rbs add jobs` — donc un répertoire
sous `templates/features/` et un `feature.toml` qui ne déclare rien de neuf : des
`[[files]]`, une `[migration]`, des `[[dependencies]]`, un `[cargo.sea-orm]` pour
`with-json`, une `[[config]]` `[jobs]`. La table naît avec `status`, `attempts`,
`available_at` et `payload`, plus l'index qui sert le dépilage. `serde_json` quitte
`[dev-dependencies]` du squelette pour `[dependencies]` : le payload est du code de
production, pas de test.

Le worker tourne **dans le processus de l'API** : c'est la seule façon qu'un job accède au
`Mailer` ou au cache que les autres fragments posent dans `AppState`. Il lui faut donc un
point d'appel dans `main.rs`, d'où une huitième ancre, `startup`.

## Étapes

1. Ancre `startup` : `anchors.rs`, `main.rs.jinja` (et le `let state` qu'elle suppose),
   les trois exemples. Test : `templates.rs` boucle déjà sur `ANCRES`.
2. `serde_json` de `[dev-dependencies]` vers `[dependencies]` dans `Cargo.toml.jinja`.
3. `templates/features/jobs/` : manifeste, `config.rs`, `model.rs`, `queue.rs`,
   `worker.rs`, `mod.rs`, `demo.rs`, `tests.rs`, `migration.rs`.
4. Test unitaire `add` : le plan de `jobs` dépose ses fichiers, sa migration, sa section.
5. Preuve : `rbs new` + `rbs add jobs` → `cargo clippy`/`fmt` propres ; `rbs migrate up`
   crée la table ; `rbs add jobs` deux fois n'écrit rien la seconde.
