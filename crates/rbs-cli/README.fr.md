# rbs-cli

La commande `rbs` : elle engendre et entretient des projets d'API web en Rust, bâtis sur Axum
et SeaORM. Elle fait partie de [rbs](https://github.com/tky0065/rbs).

*[English version](README.md).*

## Installation

```bash
cargo install rbs-cli
```

Le paquet s'appelle `rbs-cli` ; le binaire installé s'appelle `rbs`.

> **`cargo install rbs` vous donne autre chose.** Le nom `rbs` sur crates.io appartient à une
> crate de sérialisation sans rapport. Installez `rbs-cli`.

La même installation dépose aussi un second binaire, `rbs-cli`, à côté de `rbs`. L'écosystème
Ruby publie un outil sans rapport, lui aussi nommé `rbs`, et les gestionnaires de paquets le
placent souvent devant `~/.cargo/bin`. Si `rbs --version` affiche quelque chose comme
`rbs 3.10.0`, c'est celui-là qui l'emporte sur votre `PATH` — utilisez `rbs-cli`, que
personne d'autre ne revendique.

Rust 1.94 ou plus. Un projet engendré tourne sur PostgreSQL 14 ou plus, MySQL 8.0 ou plus, ou
SQLite 3.35 ou plus.

## Commandes

| Commande | Ce qu'elle fait |
|---|---|
| `rbs new <nom>` | Crée un projet prêt à démarrer : base de données, migrations, route `/health` |
| `rbs add <feature>` | Installe une feature : `audit`, `auth`, `ci`, `cors`, `docker`, `jobs`, `mail`, `observability`, `rate-limit`, `redis`, `scheduler`, `storage`, `webhooks` |
| `rbs generate crud <nom>` | Engendre une feature CRUD complète — entité et migration comprises |
| `rbs generate feature <nom>` | Engendre une feature vide : six fichiers, aucun champ |
| `rbs migrate up\|down\|status\|new` | Pilote les migrations du projet |
| `rbs seed` | Insère les données de démonstration du projet |
| `rbs dev` | Démarre les services et les migrations, et relance le serveur à chaque changement |
| `rbs doctor` | Diagnostique le projet : ancres, `.env`, base joignable, versions |
| `rbs upgrade` | Aligne le manifeste du projet sur la version du CLI |
| `rbs completions <shell>` | Écrit le script de complétion du shell sur la sortie standard |

`generate` répond à `g`. `rbs new` prend `--yes`, qui accepte les valeurs par défaut sans
rien demander pour que le CLI reste scriptable ; `rbs new` et `rbs add` prennent
`--template-dir`, qui remplace les templates embarquées dans le binaire par les vôtres.
Aucune autre commande n'accepte l'un ou l'autre.

## Ce qu'elle écrit

D'un répertoire vide à une API CRUD, avec son entité, sa migration, son document OpenAPI et
ses tests :

```bash
rbs new blog-api
cd blog-api
rbs generate crud articles --fields 'title:string,body:text,published:bool'
rbs migrate up
```

C'est la silhouette de la chose, pas une transcription à coller — le
[guide de démarrage](https://tky0065.github.io/rbs/fr/getting-started) en donne la version
exécutable, avec la base de données que les commandes attendent et la sortie de chacune
d'elles.

`rbs generate crud` produit l'entité SeaORM *et* sa migration depuis `--fields`, sans aucune
base de données démarrée — l'inverse de `sea-orm-cli generate entity`.

## La frontière qu'elle trace

[`rbs-core`](https://crates.io/crates/rbs-core) — que `rbs new` inscrit dans le manifeste
engendré — porte ce qui n'a aucune raison de varier d'un projet à l'autre : erreurs, logs,
configuration, état de l'application. Le CLI engendre dans vos propres sources tout ce que
vous voudrez lire ou modifier : modèle, DTO, repository, service, controller, migration,
tests, en Rust clair, sans macro à déplier. Ce code engendré vous appartient dès qu'il est
écrit, et aucune version de rbs ne le réécrit.

C'est aussi pourquoi le CLI ne réécrit jamais d'AST. Il insère dans des ancres en
commentaires (`// <rbs:features>`, `<rbs:routes>`, `<rbs:openapi>`, `<rbs:migrations>`) ; si
une ancre manque, il n'écrit rien et affiche le bloc à coller. Toute commande qui touche à un
projet existant montre son plan avant de l'appliquer, reste idempotente, et restaure ce
qu'elle a touché si une étape échoue.

## Documentation

Le site est à l'adresse <https://tky0065.github.io/rbs/fr/> — démarrage, architecture, une
page de référence par commande. La version anglaise vit à <https://tky0065.github.io/rbs/>.

## Licence

Sous double licence MIT ou Apache-2.0, à votre choix.
