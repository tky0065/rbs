# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## État du dépôt

Le workspace est en place et les cinq jalons de la feuille de route, de la v0.1 à la v1.1,
sont livrés. La racine porte deux crates publiables — `crates/rbs-core` et `crates/rbs-cli`,
publiées séparément sur crates.io — quatre projets d'exemple compilés en CI, et le site
Docusaurus sous `docs/`. Le nom `rbs` étant déjà pris sur crates.io, le binaire s'installe
par `cargo install rbs-cli`.

Quatre fichiers portent la décision :

| Fichier | Rôle |
|---|---|
| `docs/superpowers/specs/2026-08-25-rbs-design.md` | La spec. Autorité sur toute décision d'architecture. |
| `IMPROVE.md` | Le backlog ouvert. Les tâches restantes, force-rankées de `P0` à `P3`, chacune avec le `fichier:ligne` qui l'atteste. |
| `TODO.md` | Journal clos : les 147 tâches des cinq jalons livrés, chacune avec sa preuve datée. Ce n'est plus un backlog, on n'y coche plus rien — et il n'est pas déprécié pour autant. |
| `ROADMAP.md` | Les jalons et le hors-périmètre. |

## Exécuter une tâche du backlog

Le backlog ouvert est `IMPROVE.md`. **Utiliser le skill `improve-execute`** pour en
implémenter une tâche. `TODO.md` étant clos, le skill `rbs-task`
(`.claude/skills/rbs-task/SKILL.md`) ne sert plus qu'à relire comment un jalon a été mené.

Trois règles, que des agents ont violées en test malgré des rapports honnêtes :

1. **L'ordre des priorités est contraignant** : pas de `P2` tant qu'un `P1` sélectionné
   reste ouvert, sauf demande explicite. Le contournement « je pose ça en attendant » écrit
   du code que la tâche amont supprimera.
2. **Une case se coche sur une preuve exécutée**, consignée sur la ligne, pas parce que
   le fichier est écrit.
3. Un critère non prouvé → la case reste `- [ ]` avec une annotation `PARTIEL` ou
   `BLOQUÉ : [raison]`.

## Commandes

```bash
cargo build --workspace
cargo test --workspace
cargo test -p rbs-core                      # tests d'une crate
cargo test -p rbs-core error::tests         # un module de tests
cargo test -p rbs-core -- --exact <chemin::du::test>   # un test précis
cargo test -p rbs-cli --lib                 # les tests de rendu, sans Docker
cargo clippy --workspace --all-targets -- -D warnings  # bloquant en CI
cargo fmt --all --check                                # bloquant en CI
```

Le CLI se lance par `cargo run -p rbs-cli --bin rbs -- <commande>` pendant le développement.

Les tests d'intégration du CLI (`assert_cmd`, `crates/rbs-cli/tests/integration_*.rs`)
génèrent un projet dans un répertoire temporaire **puis le compilent** : ils sont lents et
nécessitent Docker (`testcontainers` lance un PostgreSQL). C'est le seul test qui prouve
réellement que rbs fonctionne.

`integration_examples.rs` est à part : il régénère les quatre projets d'`examples/` et les
compare octet à octet à ce qui est versionné. Toute template modifiée le fait échouer tant
que les exemples n'ont pas suivi — `examples/README.md` donne, projet par projet, les
commandes exactes qui les reconstruisent. Un exemple périmé fait mentir la documentation,
qui n'en cite aucune ligne écrite à la main.

## Architecture

Deux crates publiables, plus des templates embarquées dans le binaire :

```
crates/rbs-core/            runtime : Error/Result, config, logs, AppState, middlewares, helpers OpenAPI
crates/rbs-cli/             binaire `rbs` : new, add, generate, migrate, seed, dev, doctor, upgrade
crates/rbs-cli/templates/   squelette de projet et fragments de features (include_dir)
examples/                   projets réels compilés en CI, source des extraits de documentation
docs/                       site Docusaurus (toolchain Node isolée ici)
```

Les templates vivent **dans** la crate qui les embarque, et non à la racine : `cargo
package` n'emporte aucun fichier extérieur au paquet, et `include = [...]` ne lève pas
cette règle. À la racine, `rbs-cli` compilait en local et échouait à se publier.

Les deux crates sont indépendantes — `rbs-cli` ne dépend pas de `rbs-core`, qu'il ne fait
qu'inscrire dans les manifestes qu'il génère. Elles se publient séparément.

**La frontière noyau / généré est la décision structurante du projet** : `rbs-core` porte
ce qui n'a aucune raison de varier d'un projet à l'autre ; le CLI génère dans le projet de
l'utilisateur tout ce qu'il voudra lire ou modifier. Avant d'ajouter quoi que ce soit à
`rbs-core`, vérifier que ce n'est pas du code qui devrait être généré.

**Architecture par feature, avec dépendance unidirectionnelle stricte :**

```
src/<nom>/  mod · model · dto · repository · service · controller
controller → service → repository → model
```

Chaque couche ne voit que la suivante. Un `service` n'accède jamais *directement* à
`DatabaseConnection` — il la reçoit et la passe au `repository`, seul à construire une
requête SeaORM ; un `controller` n'en construit jamais. Cette règle rend chaque fichier
lisible isolément.

**Le CLI ne réécrit jamais d'AST.** Il insère dans des ancres en commentaires, treize au
total, énumérées par `ANCRES` dans `crates/rbs-cli/src/anchors.rs` — c'est cette liste que
`rbs doctor` parcourt, et non celle-ci :

| Ancre | Fichier |
|---|---|
| `// <rbs:features>` | `src/lib.rs`, ou `src/main.rs` sur un projet sans bibliothèque |
| `// <rbs:routes>` | `src/router.rs` |
| `// <rbs:layers>` | `src/router.rs` |
| `// <rbs:openapi>` | `src/openapi.rs` |
| `// <rbs:migration_modules>` | `migration/src/lib.rs` |
| `// <rbs:migrations>` | `migration/src/lib.rs` |
| `// <rbs:state_champs>` | `src/state.rs` |
| `// <rbs:state_init>` | `src/state.rs` |
| `// <rbs:startup>` | `src/main.rs` |
| `// <rbs:seeds>` | `src/seeds/main.rs` |
| `# <rbs:services>` | `docker-compose.yml` — la seule en YAML, et l'une des deux optionnelles |
| `// <rbs:health_probes>` | `src/health/controller.rs` |
| `// <rbs:jobs>` | `src/jobs/mod.rs` — l'autre optionnelle : le registre que pose le fragment `jobs` |

`generate` en emploie six ; les autres appartiennent aux fragments qu'installe `add`. Une
ancre insérée dans `<rbs:layers>` est *intérieure* à `trace` et `request_id` : un `.layer()`
enveloppe ce qui le précède, si bien qu'un middleware posé là voit le `request_id` et que
ses propres réponses courtes — un 429, un préflight refusé — restent dans la trace.

Ancre absente → le CLI n'écrit rien et affiche le bloc à coller. Toute commande modifiant un projet
existant suit la séquence lire → planifier → vérifier → afficher → appliquer, avec
idempotence (via `[package.metadata.rbs]`) et restauration en cas d'échec partiel.

**Scaffolding « CLI d'abord »** : `rbs generate crud` produit l'entité SeaORM *et* sa
migration depuis `--fields`, sans base de données démarrée. L'inverse de
`sea-orm-cli generate entity`.

Deux contraintes techniques à connaître avant de toucher aux templates ou aux logs :
minijinja utilise des **délimiteurs alternatifs** (Jinja et `format!` se disputent `{{ }}`),
et le formateur de logs `pretty` est un `FormatEvent` maison — celui de
`tracing-subscriber` est trop verbeux pour l'objectif de lisibilité du projet.

## Conventions de code

- **Un commentaire explique le *pourquoi*, jamais le *quoi*.** Un commentaire qui
  paraphrase la ligne suivante se supprime.
- `#![warn(missing_docs)]` sur `rbs-core` : les items publics portent un `///` d'une à
  trois lignes — seule exception à la règle ci-dessus.
- Le code généré par le CLI ne commente que ses points d'extension. Pas de bandeau
  « généré, ne pas modifier » : ce code est fait pour être modifié.
- Un fichier de feature au-delà de ~200 lignes signale une feature à scinder.
- Documentation bilingue : toute page modifiée en anglais l'est aussi en français, dans le
  même commit.

## Commits

**Conventional Commits, et rien d'autre.** `type(portée): sujet` — sujet en français, à
l'impératif, sans majuscule initiale ni point final. Types employés : `feat`, `fix`,
`docs`, `test`, `refactor`, `perf`, `ci`, `build`, `chore`.

Un message décrit **ce qui est fait dans le dépôt**, jamais le processus qui y a mené :

- **Aucun identifiant de tâche** (`A1`, `B3`, `D9`…) ni dans le sujet ni dans le corps.
  Le message dit « convertit la racine en workspace Cargo », pas « (A1) ».
- **Aucun renvoi à un fichier de suivi** : pas de ligne `Plan : docs/superpowers/plans/…`,
  pas de mention de `TODO.md`, `ROADMAP.md`, d'un lot, d'un jalon ou d'un backlog.
- **Jamais de ligne `Co-Authored-By`, `Claude-Session`**, ni aucune mention d'un assistant
  IA, d'une session ou d'un outil de génération.

Le corps porte le *pourquoi* technique du changement et, sous un intertitre
`Vérifications :`, les commandes lancées avec leur résultat réel. C'est là que vit le
détail des preuves — `TODO.md` n'en garde qu'une ligne.

Travailler sur une branche dédiée, jamais directement sur `main`.
