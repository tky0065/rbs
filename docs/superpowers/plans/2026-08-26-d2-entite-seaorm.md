# Génération de l'entité SeaORM — plan

**But :** produire `features/<nom>/model.rs` à partir du nom de feature et des champs de D1.

**Approche :** une template minijinja `templates/feature/model.rs.jinja`, rendue par le
`Renderer` existant. Le contexte vient d'un `Feature` commun aux trois générateurs de ce
lot, qui porte le nom pluriel (module et table), le nom d'entité au singulier en
PascalCase, et les champs déjà sérialisés par D1.

**Contraintes :** code du CLI nommé en français, `pub(crate)`, clippy et fmt bloquants.
`id`, `created_at`, `updated_at` sont implicites et ne figurent jamais dans `--fields`.

## Étapes

- [ ] `generate/feature.rs` : `Feature { nom, champs }`, singularisation anglaise minimale
      (`-ies`→`-y`, `-ches|-shes|-xes|-ses`→ retrait `-es`, `-s`→ retrait), `Serialize`
      exposant `module`, `table`, `entite`. Tests unitaires sur la singularisation.
- [ ] `templates/feature/` + second `include_dir!` : les templates de feature ne sont pas
      celles du squelette, que `rbs new` copie intégralement.
- [ ] `templates/feature/model.rs.jinja` : `DeriveEntityModel`, `table_name`, `id: Uuid` en
      `primary_key, auto_increment = false`, `unique` et `column_type = "Text"` d'après les
      modificateurs, `Relation` vide, `ActiveModelBehavior`.
- [ ] `generate/entite.rs` : `rendre(&Feature) -> Result<String, minijinja::Error>`.
      Tests : chaque type projeté, `optional` → `Option<T>`, `unique` → attribut.

## Preuves attendues

- ✓ *L'entité compile et ses types correspondent aux champs demandés* — test d'intégration
  `assert_cmd` : `rbs new`, écriture de l'entité rendue dans le projet, `cargo build`.
- ✓ *`id` est un `Uuid` sans auto-incrément* — test unitaire sur le rendu.
