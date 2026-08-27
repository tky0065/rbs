# Ancres `state_champs` et `state_init`

Design : `2026-08-27-v0.3-integrations-design.md` §2.4.

## Décisions internes

- **`AppState::new` rend `anyhow::Result<Self>`.** Le squelette dépend déjà d'`anyhow`, son
  `main` rend `anyhow::Result<()>` et remonte donc l'erreur par un seul `?`. Un type
  d'erreur maison obligerait chaque fragment à s'y convertir pour une panne de démarrage
  que personne ne rattrape.
- **`state_init` est posée dans le littéral `Self { … }`**, et non dans un bloc de
  statements avant lui : un champ se nomme alors une fois et non deux, et un fragment qui
  a besoin de plusieurs lignes appelle un constructeur de son propre module
  (`cache: crate::cache::pool()?,`), ce qui laisse `state.rs` lisible.

## Étapes

1. `ancres.rs` : `STATE_CHAMPS` et `STATE_INIT`, toutes deux sur `src/state.rs` ; `ANCRES`
   passe à sept.
2. `state.rs.jinja` : les deux ancres, `new` faillible et toujours synchrone.
3. `main.rs.jinja` : `AppState::new(db, config)?`.
4. `doctor` : la formulation « cinq » et son test comptent désormais sept.
5. Les deux exemples reçoivent le même squelette, ce que la comparaison de non-dérive
   arbitre.

## Preuves

| Critère | Commande |
|---|---|
| Le contenu déclaré est inséré dans chacune des deux ancres | `cargo test -p rbs-cli --test integration_add -- les_deux_ancres_d_etat` |
| Ancre absente → rien d'écrit, bloc affiché, sortie en erreur | `cargo test -p rbs-cli --test integration_add -- une_ancre_d_etat_absente` |
| `rbs new` puis `clippy -D warnings` et `rustfmt --check` du projet généré | `cargo test -p rbs-cli --test integration_new -- --ignored` |
| Non-dérive des exemples | `cargo test -p rbs-cli --test integration_examples` |
| Qualité | `cargo clippy --workspace --all-targets --all-features -- -D warnings` · `cargo fmt --all --check` |
