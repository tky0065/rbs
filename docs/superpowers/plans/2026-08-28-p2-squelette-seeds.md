# `src/seeds/` et son binaire au squelette

## La forme retenue

`src/seeds/main.rs` est la racine d'un second binaire du paquet, déclaré dans le manifeste :

```toml
default-run = "<projet>"

[[bin]]
name = "<projet>"
path = "src/main.rs"

[[bin]]
name = "seed"
path = "src/seeds/main.rs"
```

`default-run` n'est pas un détail : deux binaires rendent `cargo run` ambigu, et c'est la
commande que `rbs new` affiche à la fin.

Le binaire des seeds est une racine de crate distincte de celle de l'application. Il
n'atteint donc pas `crate::<feature>::model` : chaque seed rejoint l'entité de sa feature
par un `#[path]`, ce que `P3` rend. Le squelette de l'application n'est pas touché — ni
`main.rs`, ni ses ancres.

## Une ancre, et un `mod` qui ne peut pas vivre dans un bloc

`<rbs:seeds>` est la huitième ancre. Elle porte des identifiants, non des instructions :

```rust
seeds! {
    // <rbs:seeds>
    // </rbs:seeds>
}
```

Un `mod` non inline ne s'écrit pas dans un bloc — c'est déjà pourquoi la crate `migration`
porte deux ancres. Une invocation de `macro_rules!` à hauteur d'item lève la contrainte :
la macro déclare les modules **et** construit leur enchaînement d'un seul geste, ce qui
tient en une ancre au lieu de deux. Elle a l'effet second d'échapper à `reorder_modules` de
rustfmt, qui trierait des `mod` posés dans l'ordre de génération.

## Preuves

- `rbs new` puis `rbs seed` sur un projet vierge → code 0, « rien à insérer ».
- `cargo clippy --workspace --all-targets -- -D warnings` et `rustfmt --check` sur le
  projet généré, `src/seeds/main.rs` compris dans les racines de modules vérifiées.
- Les trois exemples régénérés : ils gagnent le binaire et les deux sections du manifeste.
- `doctor` compte huit points d'insertion — la page `cli/doctor` suit, en anglais et en
  français.
