# Câblage de `rbs generate` — plan

**But :** `rbs g crud <nom> --fields …` et `rbs g feature <nom>` écrivent une feature dans
un projet existant et la montent dans ses ancres.

**Approche :** `generate/commande.rs`, sur la séquence du §4.4 que `new.rs` suit déjà :
tout ce qui peut être vérifié l'est, tout le rendu aboutit, et la première écriture
n'arrive qu'ensuite. La racine du projet se trouve en remontant jusqu'au `Cargo.toml`
porteur de `[package.metadata.rbs]`.

`crud` pose sept fichiers, une migration et cinq ancres. `feature` en pose six et trois
ancres, sans migration ni tests : le critère ne demande que la compilation, et une feature
écrite à la main porte sa propre migration ou n'en a pas.

**Hors périmètre, laissé tel quel :** `--force` reste inerte — la vérification du working
tree est E4 ; l'affichage du plan et `--dry-run` sont E5 ; la restauration après échec
partiel est E6. D10 s'en tient à ne rien écrire avant d'avoir tout vérifié et tout rendu.

## Étapes

- [x] Racine du projet, et refus hors d'un projet rbs.
- [x] `generate/commande.rs` : nom validé (D7b), champs analysés (D1), feature déjà
      présente refusée, rendu complet, ancres montées (D9), écriture, inscription dans
      `[package.metadata.rbs]`.
- [x] Câblage de `main.rs` : messages d'erreur et code de sortie.
- [x] Tests unitaires sur un projet déroulé par `new::creer`, sans compilation ; test
      lourd `assert_cmd` pour la compilation.

## Preuves attendues

- ✓ *Le projet compile après génération d'une feature vide* — `rbs new`, `rbs g feature`,
  puis `cargo build` du projet.
- ✓ *(D7b)* *`rbs g crud state` et `rbs g crud match` échouent en nommant le conflit* — la
  commande existant enfin, le troisième critère de D7b se prouve avec elle.
