# `rbs generate crud` dépose le seed de son entité

## Le fichier rendu

`src/seeds/<feature>.rs`, deux lignes de démonstration, du Rust typé :

```rust
#[path = "../articles/model.rs"]
mod model;

pub async fn seed(db: &DatabaseConnection) -> Result<(), DbErr> {
    model::ActiveModel { title: Set("title-1".to_owned()), ..Default::default() }
        .insert(db).await?;
    …
}
```

- `#[path]` plutôt qu'un `use crate::…` : le binaire des seeds est une autre racine de
  crate que l'application, et n'en compile que l'entité. Un `allow(dead_code)` semblait
  nécessaire — l'entité expose plus d'items qu'un seed n'en appelle — mais un
  `clippy -D warnings` sur un projet généré le dément : ce que dérive SeaORM n'éveille pas
  la lint. L'allow est donc retiré, contrôle à l'appui.
- `id`, `created_at` et `updated_at` restent `NotSet` : leur valeur vient du défaut de la
  colonne, que la migration générée pose déjà (`uuidv7()`, `current_timestamp()`).
- Les valeurs suivent celles des tests HTTP générés — `42`/`43`, `4.2`/`8.4`,
  `true`/`false` — pour que deux lignes ne se heurtent pas sur un champ `unique`. `uuid`
  passe par `Uuid::from_u128`, `new_v4` demandant une feature que le projet n'active qu'en
  dev-dependency, donc absente du binaire.

Seul `generate crud` le dépose : une feature écrite à la main n'a pas d'entité, comme elle
n'a pas de migration. `mount::for_seed` se tient donc à côté de `mount::for_migration`.

## L'ordre dans l'ancre

L'ancre empile dans l'ordre de génération, comme les sept autres. `anchors::insert` laisse
au développeur ce qu'il a écrit dans une ancre — le réordonner serait contraire à la règle
du module — et l'ordre de génération est aussi celui des migrations, donc celui qui
survivra le jour où un seed dépendra d'un autre.

## Preuves

- `src/seeds/articles.rs` existe et l'ancre porte `articles,`.
- Deux `generate crud` → deux fichiers, deux lignes dans l'ancre, chacune une fois.
- Lourde : projet neuf, `generate crud`, `migrate up`, `rbs seed`, puis `GET /<feature>`
  rend les deux lignes.
