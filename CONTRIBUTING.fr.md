# Contribuer à rbs

Merci d'y consacrer du temps. Cette page dit ce qu'il faut installer, comment lancer les
vérifications que la CI lancera, et les conventions que le dépôt s'impose.

*[English version](CONTRIBUTING.md).*

## Contribuer sans installer Node

**Travailler sur le code Rust ne demande jamais Node.** Le site de documentation vit
entièrement sous `docs/`, avec sa propre toolchain et son propre workflow ; rien dans
`cargo build`, `cargo test`, `cargo clippy` ou `cargo fmt` n'y touche, et le job Rust de la
CI ne lance aucune commande `npm`. Si vous ne modifiez que les crates ou les templates,
ignorez `docs/` entièrement.

L'inverse vaut aussi : corriger une coquille dans la documentation ne demande pas de
toolchain Rust.

## Prérequis

| | |
|---|---|
| Rust | 1.94 ou plus — la MSRV est déclarée dans `Cargo.toml` et tenue par la CI |
| Docker | tests d'intégration seulement ; ils démarrent un conteneur PostgreSQL |
| Node | site de documentation seulement — voir ci-dessus |

## Commandes courantes

```bash
cargo build --workspace
cargo test --workspace                                  # tests rapides, sans Docker
cargo clippy --workspace --all-targets -- -D warnings    # bloquant en CI
cargo fmt --all --check                                  # bloquant en CI
```

Le CLI se lance depuis le dépôt par `cargo run -p rbs-cli -- <commande>`, par exemple
`cargo run -p rbs-cli -- new demo-api`.

Pour resserrer la boucle pendant le développement :

```bash
cargo test -p rbs-core                              # une crate
cargo test -p rbs-core error::tests                 # un module de tests
cargo test -p rbs-core -- --exact <chemin::du::test>  # un test précis
```

## Les trois niveaux de test

1. **`rbs-core`** — tests unitaires, dont les conversions erreur → réponse HTTP.
2. **Le projet généré** — `rbs generate crud` produit des tests d'intégration HTTP
   couvrant le CRUD complet contre l'application montée en mémoire. Un starter qui génère
   du code sans tests enseigne à ne pas en écrire.
3. **Le CLI, bout en bout** — `assert_cmd` enchaîne `rbs new`, `rbs generate crud` et
   `rbs add` sur un projet jetable, le compile et lance ses tests contre un vrai
   PostgreSQL. C'est le seul test qui prouve que rbs fonctionne.

Le troisième niveau est lent et demande Docker : il est marqué `#[ignore]` et se lance à
la demande.

```bash
cargo test --workspace -- --ignored        # 17 tests, plusieurs minutes, Docker requis
```

La CI lance les deux, sur chaque pull request. Si vous ne pouvez pas jouer les tests
ignorés en local, dites-le dans la pull request — ne les faites pas taire.

## Conventions

**Les commits suivent les [Conventional Commits](https://www.conventionalcommits.org), et
rien d'autre :** `type(portée): sujet`. Le sujet est en **français**, à l'impératif, sans
majuscule initiale ni point final. Types employés : `feat`, `fix`, `docs`, `test`,
`refactor`, `perf`, `ci`, `build`, `chore`.

Le corps porte le *pourquoi* technique du changement et, sous un intertitre
`Vérifications :`, les commandes lancées avec leur résultat réel.

Travaillez sur une branche dédiée, jamais sur `main`.

**Style de code :**

- Un commentaire explique le *pourquoi*, jamais le *quoi*. Un commentaire qui paraphrase
  la ligne suivante se supprime.
- `rbs-core` porte `#![warn(missing_docs)]` : les items publics portent un `///` d'une à
  trois lignes. C'est la seule exception à la règle ci-dessus.
- Le code généré par le CLI ne commente que ses points d'extension. Pas de bandeau
  « généré, ne pas modifier » : ce code est fait pour être modifié.
- Un fichier de feature au-delà de ~200 lignes signale une feature à scinder.
- La documentation est bilingue : une page modifiée en anglais l'est aussi en français,
  dans le même commit.

**Architecture :** les features ont une dépendance unidirectionnelle stricte —
`controller → service → repository → model`. Un service n'accède jamais à
`DatabaseConnection` ; un controller ne construit jamais de requête SeaORM.

## Pull requests

Dites ce que le changement fait et pourquoi, et énumérez les commandes lancées avec leur
résultat. Si un critère n'a pas pu être vérifié, écrivez-le plutôt que de le laisser
entendre — un manque annoncé se traite mieux qu'une affirmation qui ne tient pas.

En contribuant, vous acceptez que votre travail soit publié sous les deux licences
[MIT](LICENSE-MIT) et [Apache 2.0](LICENSE-APACHE), comme le reste du dépôt.

## Code de conduite

Ce projet suit le [Contributor Covenant](CODE_OF_CONDUCT.fr.md). Signalez tout comportement
inacceptable à tky0065@gmail.com.
