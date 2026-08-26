# Génération du repository — plan

**But :** produire `features/<nom>/repository.rs` — CRUD complet et liste paginée —
depuis le même `Feature` que D2, D3 et D7.

**Approche :** template `templates/feature/repository.rs.jinja` et `generate/repository.rs`,
sur le patron de `dto.rs`. Cinq fonctions libres, pas de struct : le module *est* le
repository.

```rust
pub async fn list(db, pagination: &Pagination) -> Result<(Vec<Model>, u64)>
pub async fn find(db, id: Uuid)               -> Result<Option<Model>>
pub async fn create(db, modele: ActiveModel)  -> Result<Model>
pub async fn update(db, modele: ActiveModel)  -> Result<Model>
pub async fn delete(db, id: Uuid)             -> Result<bool>
```

**Frontière :** le fichier ne connaît que `model.rs`. Il ne voit ni DTO, ni Axum, ni
`Page<T>` : `list` rend le couple `(page, total)` et laisse le service l'assembler.

**Tri :** `id` décroissant. L'`id` est un UUIDv7, son ordre *est* l'ordre d'insertion —
seul critère à la fois déterministe et sans colonne supplémentaire.

**Porte du modèle :** `pub use super::model::{ActiveModel, Model};` en tête. C'est ce qui
permettra au service (D5) de ne jamais nommer `model.rs`, comme l'exige la spec §3.4.

## Étapes

- [ ] Test de rendu rouge : les cinq signatures, la réexportation, l'absence d'Axum.
- [ ] `templates/feature/repository.rs.jinja` — indépendant des champs : seuls le nom du
      module et celui de l'entité varient.
- [ ] `generate/repository.rs` : `rendre(&Feature)`, déclaré dans `generate/mod.rs`.
- [ ] Test `#[ignore]` de compilation : `model.rs` + `repository.rs` dans un projet neuf.

## Preuve attendue

- ✓ *Revue : aucun import d'Axum dans le fichier* — automatisé en test de rendu
  (`assert!(!rendu.contains("axum"))`), pas laissé à une lecture.
- La compilation du couple `model` + `repository` dans un projet issu de `rbs new`.
