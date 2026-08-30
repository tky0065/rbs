# file-drop — mode d'emploi pour agents

<!-- rbs:guide 1.1.0 -->
## Le CLI d'abord

Ce projet est engendré par rbs. **Toute fonctionnalité que rbs couvre passe par le CLI**,
jamais par l'écriture à la main des fichiers : le CLI pose la migration, câble les ancres,
inscrit la feature dans `[package.metadata.rbs]` et respecte l'architecture. Six fichiers
écrits à la main donnent un projet que `rbs doctor` déclare incomplet.

## Les commandes

| Commande | Ce qu'elle fait | Ce qu'elle dispense d'écrire |
|---|---|---|
| `rbs new <nom>` | crée un projet | tout le squelette |
| `rbs add <feature>` | installe auth, ci, docker, jobs, mail, redis, storage | le câblage de la feature |
| `rbs generate crud <nom> --fields "..."` | une feature CRUD complète | six fichiers, l'entité et sa migration |
| `rbs generate feature <nom>` | une feature vide | six fichiers |
| `rbs migrate up\|down\|status` | pilote les migrations | — |
| `rbs migrate new <nom>` | un fichier de migration vide | le squelette de la migration |
| `rbs seed` | insère les données de démonstration | — |
| `rbs dev` | services, migrations, serveur rechargé | — |
| `rbs doctor` | diagnostique le projet | — |
| `rbs upgrade` | aligne le projet sur la version du CLI | — |

`rbs generate` accepte `--dry-run` : le plan s'affiche, rien ne s'écrit.

## Recettes

- Une entité et son CRUD : `rbs generate crud posts --fields "title:string,body:text"`
- Une référence vers une autre entité : `--fields "author:references:users"`
- Le côté inverse d'une relation : `rbs generate crud users --has-many posts`
- Une feature sans champs : `rbs generate feature webhooks`
- L'authentification JWT : `rbs add auth`, puis recopier `RBS_AUTH__SECRET`
- Une migration écrite à la main : `rbs migrate new ajoute_index_sur_slug`

## Architecture imposée

```
features/<nom>/  mod · model · dto · repository · service · controller
controller → service → repository → model
```

La dépendance est unidirectionnelle et stricte : un `service` n'accède jamais à
`DatabaseConnection`, un `controller` ne construit jamais de requête SeaORM. Un fichier de
feature au-delà de ~200 lignes signale une feature à scinder.

## Les ancres

Le CLI ne réécrit jamais d'AST : il insère dans des ancres en commentaires. Ne pas les
retirer, ne pas les réordonner, ne pas écrire à leur place quand une commande peut le
faire.

- `<rbs:features>` dans `src/main.rs`
- `<rbs:routes>` dans `src/router.rs`
- `<rbs:openapi>` dans `src/openapi.rs`
- `<rbs:migration_modules>` dans `migration/src/lib.rs`
- `<rbs:migrations>` dans `migration/src/lib.rs`
- `<rbs:state_champs>` dans `src/state.rs`
- `<rbs:state_init>` dans `src/state.rs`
- `<rbs:startup>` dans `src/main.rs`
- `<rbs:seeds>` dans `src/seeds/main.rs`
- `<rbs:services>` dans `docker-compose.yml`
- `<rbs:relations:<table>>` et `<rbs:related:<table>>` dans le modèle de chaque entité

## Ce que rbs ne couvre pas

Un endpoint qui n'est pas un CRUD, un client HTTP externe, une règle métier : rbs ne les
engendre pas, et il est légitime de les écrire à la main. Alors :

- écrire dans la feature existante, jamais dans un module parallèle ;
- respecter les couches — le controller appelle le service, le service appelle le
  repository ;
- si le code ajoute une route, l'inscrire dans `<rbs:routes>` et `<rbs:openapi>` à la
  main ;
- `rbs doctor` signalera un module écrit hors du CLI en avertissement. C'est un constat,
  pas une erreur.

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
- Entités engendrées : aucune
- Ancres du projet : features (src/lib.rs), routes (src/router.rs), openapi (src/openapi.rs), migration_modules (migration/src/lib.rs), migrations (migration/src/lib.rs), state_champs (src/state.rs), state_init (src/state.rs), startup (src/main.rs), seeds (src/seeds/main.rs), services (docker-compose.yml)
<!-- /rbs:inventory -->

## Notes du projet
