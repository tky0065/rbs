# `rbs seed` : la commande

Conception retenue (tâche bornée, §2.4 de la conception du jalon).

## Ce que la commande fait, dans l'ordre

1. Remonte à la racine du projet (`metadata::project_root`) — sinon `PasUnProjet`.
2. **Garde-fou de production** : `RBS_ENV=production` sans `--force` → refus. L'environnement
   du processus l'emporte sur le `.env` du projet, comme pour `rbs migrate`. Le refus est
   posé **avant** toute lecture et tout lancement : c'est ce que le critère exige.
3. `src/seeds/main.rs` absent → message disant comment le créer, avec le bloc `[[bin]]` à
   coller. Jamais une erreur de cargo.
4. Ancre `<rbs:seeds>` présente et vide → « rien à insérer », code 0, aucun cargo lancé.
   Une ancre absente ne bloque pas : un binaire de seeds écrit à la main reste lançable.
5. Sinon, `.env` lu puis `cargo run --bin seed`, sur le motif de `rbs migrate`.

## Découpage

- `crates/rbs-cli/src/cargo.rs` — le lancement d'un binaire du projet, extrait de
  `migrate::launch` pour que les deux commandes ne dupliquent pas le spawn et gardent
  chacune son message d'échec.
- `crates/rbs-cli/src/seed.rs` — la commande. `execute()` prend l'accès à l'environnement
  et le lanceur en paramètres : c'est ce qui rend « le binaire n'a pas été lancé »
  vérifiable sans compiler un projet.
- `crates/rbs-cli/src/anchors.rs` — `body()`, le contenu d'une ancre, pour savoir si elle
  est vide.
- `cli.rs`, `lib.rs` — la variante `Seed { force }` et son affichage.

## Preuves

- unitaire : sous `RBS_ENV=production`, `execute` rend `Production` et le lanceur injecté
  n'est pas appelé ; avec `--force`, il l'est.
- intégration (`tests/integration_seed.rs`, rapide) : `RBS_ENV=production rbs seed` sort
  non nul, nomme `--force`, et le projet n'a pas de `target/` — cargo n'a donc pas tourné.
- intégration : projet sans `src/seeds/` → sortie non nulle nommant `src/seeds`, et pas de
  `target/`.
