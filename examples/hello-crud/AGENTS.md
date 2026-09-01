# hello-crud — mode d'emploi pour agents

<!-- rbs:guide 1.1.0 -->
## Le CLI d'abord

Ce projet est engendré par rbs. **Toute fonctionnalité que rbs couvre passe par le CLI**,
jamais par l'écriture à la main des fichiers : le CLI pose la migration, câble les ancres,
inscrit la feature dans `[package.metadata.rbs]` et respecte l'architecture. Le même CRUD
écrit à la main, ce sont sept fichiers, une migration et toutes les ancres à tenir
soi-même, pour un module que `[package.metadata.rbs]` continuera d'ignorer.

## Ce fichier

Deux zones de ce fichier appartiennent à rbs et sont régénérées en entier : `rbs:guide`,
le mode d'emploi que vous lisez, et `rbs:inventory`, l'état du projet. Ce qui est écrit
entre leurs marqueurs disparaît au prochain `rbs upgrade`, `rbs add` ou `rbs generate`,
sans avertissement. Tout le reste du fichier appartient au projet, et la section « Notes
du projet », en bas, est faite pour l'accueillir.

## Les commandes

| Commande | Ce qu'elle fait | Ce qu'elle dispense d'écrire |
|---|---|---|
| `rbs new <nom>` | crée un projet ; `--lang fr\|en` fixe la langue de ce fichier | tout le squelette |
| `rbs add <feature>` | installe auth, ci, docker, jobs, mail, redis, storage | le câblage de la feature |
| `rbs generate crud <nom> --fields "..."` | une feature CRUD complète | sept fichiers, le seed et la migration |
| `rbs generate feature <nom>` | une feature vide | six fichiers |
| `rbs migrate up\|down\|status` | pilote les migrations | — |
| `rbs migrate new <nom>` | un fichier de migration vide | le squelette de la migration |
| `rbs seed` | insère les données de démonstration | — |
| `rbs dev` | services, migrations, serveur rechargé | — |
| `rbs doctor` | diagnostique le projet | — |
| `rbs upgrade` | aligne le projet sur la version du CLI | — |

`rbs generate`, `rbs add` et `rbs upgrade` acceptent `--dry-run` : le plan s'affiche, rien ne s'écrit.

## Recettes

- Une entité et son CRUD : `rbs generate crud posts --fields "title:string,body:text"`
- Une référence vers une autre entité : `--fields "author:references:users"`
- Le côté inverse d'une relation : `rbs generate crud users --has-many posts`
- Une feature sans champs : `rbs generate feature webhooks`
- L'authentification JWT : `rbs add auth`, puis recopier `RBS_AUTH__SECRET`
- Une migration écrite à la main : `rbs migrate new ajoute_index_sur_slug`

## Architecture imposée

```
src/<nom>/  mod · model · dto · repository · service · controller
controller → service → repository → model
```

La dépendance est unidirectionnelle et stricte : un `service` n'accède jamais
*directement* à `DatabaseConnection` — il la reçoit et la passe au `repository`, seul à
construire une requête SeaORM ; un `controller` n'en construit jamais. `rbs generate
crud` ajoute à ces six fichiers un `tests.rs` et, quand l'entité est semable, le seed
`src/seeds/<nom>.rs`. Un fichier de feature au-delà de ~200 lignes signale une feature à
scinder.

## Les ancres

Le CLI ne réécrit jamais d'AST : il insère dans des ancres en commentaires. Ne pas les
retirer, ne pas les réordonner, ne pas écrire à leur place quand une commande peut le
faire.

- `<rbs:features>` dans `src/lib.rs`
- `<rbs:routes>` dans `src/router.rs`
- `<rbs:layers>` dans `src/router.rs`
- `<rbs:openapi>` dans `src/openapi.rs`
- `<rbs:migration_modules>` dans `migration/src/lib.rs`
- `<rbs:migrations>` dans `migration/src/lib.rs`
- `<rbs:state_champs>` dans `src/state.rs`
- `<rbs:state_init>` dans `src/state.rs`
- `<rbs:startup>` dans `src/main.rs`
- `<rbs:seeds>` dans `src/seeds/main.rs`
- `<rbs:services>` dans `docker-compose.yml`
- `<rbs:health_probes>` dans `src/health/controller.rs`
- `<rbs:relations:<table>>` et `<rbs:related:<table>>` dans le modèle de chaque entité

## Ce que rbs ne couvre pas

Un endpoint qui n'est pas un CRUD, un client HTTP externe, une règle métier : rbs ne les
engendre pas, et il est légitime de les écrire à la main. Alors :

- écrire dans la feature existante, jamais dans un module parallèle ;
- respecter les couches — le controller appelle le service, le service appelle le
  repository ;
- si le code ajoute une route, l'inscrire dans `<rbs:routes>` et `<rbs:openapi>` à la
  main ;
- `rbs doctor` signale un module écrit hors du CLI par un avertissement `!`, jamais par un
  échec : c'est un constat, et la commande reste bonne.

## Vérifier avant de conclure

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
rbs doctor
```

Les tests demandent la base du `.env` démarrée — `docker compose up -d`, ou `rbs dev` qui
l'enchaîne.
<!-- /rbs:guide -->

<!-- rbs:inventory -->
- rbs 1.1.0 · base postgres
- Fragments installés : aucun
- Entités engendrées : articles
- Ancres du projet : features (src/lib.rs), routes (src/router.rs), layers (src/router.rs), openapi (src/openapi.rs), migration_modules (migration/src/lib.rs), migrations (migration/src/lib.rs), state_champs (src/state.rs), state_init (src/state.rs), startup (src/main.rs), seeds (src/seeds/main.rs), services (docker-compose.yml), health_probes (src/health/controller.rs)
<!-- /rbs:inventory -->

## Notes du projet
