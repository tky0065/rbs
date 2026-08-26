# Insertion dans les ancres — plan

**But :** poser le moteur qui écrit dans les ancres du projet, et les lignes qu'une
feature y ajoute.

**Approche :** `crates/rbs-cli/src/ancres.rs` — fonctions pures, de source à source.
L'insertion se fait juste avant la balise fermante, avec son indentation ; le contenu
déjà présent n'est ni relu ni réordonné, et une ligne déjà là n'est pas réécrite. Aucune
écriture sur disque : elle appartient à D10, et son atomicité à E6.

**Une cinquième ancre.** `migration/src/lib.rs` a besoin de deux insertions — le
`Box::new(…::Migration)` dans le `vec!`, et le `mod …;` qui déclare le fichier. Rust
interdit un `mod` non-inline dans un bloc : les deux ne peuvent pas tenir dans la même
ancre. `<rbs:migration_modules>` s'ajoute donc en tête du fichier. **D12 (`rbs doctor`)
vérifiera cinq ancres, pas quatre.**

**Frontière avec E2.** Le moteur d'ancres de E2 est celui-ci, étendu : E2 lui ajoutera le
message d'erreur de la spec §4.6 — le bloc à recoller — et le code de sortie qui va avec.
D9 s'arrête au type d'erreur, qui porte l'ancre et son fichier.

## Étapes

- [x] `<rbs:migration_modules>` dans la template de la crate `migration`, et la liste des
      ancres devient celle de `ancres.rs` — `templates.rs` la dupliquait.
- [x] `ancres::inserer`, avec ses tests : place, indentation, idempotence, ancre absente,
      et le critère du lot — le contenu existant traverse l'insertion à l'octet près.
- [x] `generate/montage.rs` : les lignes qu'une feature ajoute à chacune des cinq ancres,
      en chemins absolus (`crate::users::routes()`), conformément à la spec §4.5. Le banc
      cesse de simuler les ancres à la main et passe par le moteur : les tests lourds du
      lot le prouvent alors contre de vrais projets, compilés.

## Preuves attendues

- ✓ *Le contenu existant dans l'ancre n'est ni réordonné ni reformaté* — test unitaire sur
  une ancre déjà peuplée de lignes désordonnées et diversement indentées.
