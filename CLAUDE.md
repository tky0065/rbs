# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## État du dépôt

**Aucun code applicatif n'existe encore.** La racine contient le squelette `cargo new`
d'origine (paquet `rs`, `src/main.rs` avec un `Hello, world!`) — la tâche `A1` du backlog
le supprime pour le remplacer par un workspace. Ne pas supposer l'existence de
`crates/`, de `rbs-core` ou d'un binaire `rbs` : les vérifier avant d'y référer.

Le projet est aujourd'hui à l'état de conception, entièrement décrit dans trois fichiers :

| Fichier | Rôle |
|---|---|
| `docs/superpowers/specs/2026-08-25-rbs-design.md` | La spec. Autorité sur toute décision d'architecture. |
| `TODO.md` | Les 52 tâches de la v0.1, avec ordre et critères de validation. Source de vérité de l'avancement. |
| `ROADMAP.md` | Les jalons et le hors-périmètre. |

## Exécuter une tâche du backlog

**Utiliser le skill `rbs-task`** (`.claude/skills/rbs-task/SKILL.md`) dès qu'il s'agit
d'implémenter une tâche de `TODO.md`. Il encode trois règles que des agents ont violées
en test, malgré des rapports honnêtes :

1. **L'ordre des lots `A → B → C → D → E` est contraignant.** Une tâche dont l'amont est
   incomplet ne se commence pas. Le contournement « je mets le code à la racine en
   attendant que `A1` soit fait » produit du code que `A1` supprimera.
2. **Une case se coche sur une preuve exécutée**, consignée sur la ligne, pas parce que
   le fichier est écrit.
3. Un critère `✓` non prouvé → la case reste `- [ ]` avec une annotation `PARTIEL`.

## Commandes

Le workspace n'existant pas encore, ces commandes ne fonctionneront qu'une fois `A1` faite :

```bash
cargo build --workspace
cargo test --workspace
cargo test -p rbs-core                      # tests d'une crate
cargo test -p rbs-core error::tests         # un module de tests
cargo test -p rbs-core -- --exact <chemin::du::test>   # un test précis
cargo clippy --workspace --all-targets -- -D warnings  # bloquant en CI
cargo fmt --all --check                                # bloquant en CI
```

Le CLI se lance par `cargo run -p rbs-cli -- <commande>` pendant le développement.

Les tests d'intégration du CLI (`assert_cmd`) génèrent un projet dans un répertoire
temporaire **puis le compilent** : ils sont lents et nécessitent Docker (`testcontainers`
lance un PostgreSQL). C'est le seul test qui prouve réellement que rbs fonctionne.

## Architecture

Deux crates publiables, plus des templates embarquées dans le binaire :

```
crates/rbs-core/            runtime : Error/Result, config, logs, AppState, middlewares, helpers OpenAPI
crates/rbs-cli/             binaire `rbs` : new, add, generate, migrate, doctor
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

Un `service` n'accède jamais à `DatabaseConnection` ; un `controller` ne construit jamais
de requête SeaORM. Cette règle rend chaque fichier lisible isolément.

**Le CLI ne réécrit jamais d'AST.** Il insère dans des ancres en commentaires
(`// <rbs:features>`, `<rbs:routes>`, `<rbs:openapi>`, `<rbs:migrations>`). Ancre absente →
le CLI n'écrit rien et affiche le bloc à coller. Toute commande modifiant un projet
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
