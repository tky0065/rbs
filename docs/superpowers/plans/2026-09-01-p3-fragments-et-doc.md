# Lot P3 67-73 — dette des fragments et de la documentation

Branche d'intégration `improve/p3-fragments-et-doc`. Quatre lots en worktrees isolés,
regroupés par zone de fichiers : aucun ne touche un fichier qu'un autre touche.

Décisions arbitrées avant le code : accesseur d'état partout (68), `ci` déclare ses
`[[files]]` (69), les six `preview()` sont supprimés (71), les trois README sont traduits
et `parite.mjs` étendu (72).

## Lot A — `improve/p3-lot-a` — tâches 67 et 68

Les deux touchent `templates/features/{jobs,redis,mail,storage}`, donc ils partent ensemble.

1. Descendre le `#![allow(dead_code)]` de `jobs/mod.rs.jinja:3`, `redis/mod.rs.jinja:3` et
   `mail/mod.rs.jinja:11` au niveau de l'item, sur le modèle d'`auth/guard.rs.jinja:24` :
   un `#[allow(dead_code)]` par item réellement mort, avec sa mention de retrait.
2. Ajouter à `mail/mod.rs.jinja` et `storage/mod.rs.jinja` l'`impl AppState` que
   `redis/mod.rs.jinja:187-191` porte déjà ; retirer les `#[allow(dead_code)]` des
   `state_champs` de `mail/feature.toml:36-40` et `storage/feature.toml:26-29`.
3. La sonde `health_probes` de `storage` passe par l'accesseur.
4. Régénérer `examples/file-drop` et `examples/newsletter-queue` par diff entre deux
   générations, jamais par écrasement.

Preuve : `cargo test -p rbs-cli --lib`, `clippy`, `fmt`, puis `cargo check` sur les deux
exemples porteurs.

## Lot B — `improve/p3-lot-b` — tâches 69 et 70

1. `ci/feature.toml` déclare `.github/workflows/ci.yml.jinja → .github/workflows/ci.yml`.
2. `templates/feature/seed.rs.jinja:21` cesse d'attribuer l'`id` au défaut de la colonne :
   il vient d'`ActiveModelBehavior::new()` (`model.rs.jinja:63`). Seuls `created_at` et
   `updated_at` gardent leur `.default(Expr::current_timestamp())`.
3. Régénérer les quatre exemples pour les seeds, par diff.

Preuve : `cargo test -p rbs-cli --lib`, `clippy`, `fmt`.

## Lot C — `improve/p3-lot-c` — tâches 71 et 73

1. Supprimer les six `preview()` `#[ignore = "affichage pour revue humaine"]` de
   `generate/{entity,dto,repository,migration,service,controller}.rs`.
2. `Plan::actions()` (`plan/mod.rs:50`) passe sous `#[cfg(test)]`, son `allow(dead_code)`
   disparaît.
3. Retirer `with-chrono` et `with-uuid` de `sea-orm` dans `Cargo.toml:38`.

Preuve : `cargo test --workspace`, `clippy --workspace --all-targets -- -D warnings`, `fmt`.

## Lot D — `improve/p3-lot-d` — tâche 72

1. Traduire `examples/README.md` (214 l.), `crates/rbs-cli/README.md` (85 l.) et
   `crates/rbs-core/README.md` (69 l.) en `README.fr.md`, convention déjà en place à la
   racine du dépôt.
2. Étendre `docs/scripts/parite.mjs` d'un troisième jeu de paires, à côté des pages du
   site et des fichiers racine.

Preuve : `node docs/scripts/parite.mjs` en sortie propre.

## Intégration

Merge des quatre branches dans `improve/p3-fragments-et-doc`, puis une seule passe Docker
sur la branche intégrée : `integration_examples.rs` et le lot `--ignored`, avec
`--no-fail-fast`. Aucune case cochée dans `IMPROVE.md` avant que cette passe soit lue.
