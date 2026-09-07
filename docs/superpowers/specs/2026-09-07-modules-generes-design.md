# Les modules de `rbs add` sous `src/modules/`

**Date** : 2026-09-07
**Statut** : validé, prêt pour le plan d'implémentation
**Version visée** : 1.3.0 (mineure)

## Le problème

Dans un projet rbs, `src/` mêle deux natures de code que rien ne distingue à l'œil :

```
src/
  articles/     rbs generate crud    — le domaine du développeur
  auth/         rbs add auth
  mail/         rbs add mail
  posts/        rbs generate crud    — le domaine du développeur
  storage/      rbs add storage
  webhooks/     rbs add webhooks
```

Le développeur ouvre `src/` et ne sait pas, sans le lire, si `webhooks/` est son code ou
celui que le CLI a posé. Plus il installe de fragments, plus ses propres features se
noient : un projet portant les onze modules et trois entités montre quatorze répertoires
dont trois seulement lui appartiennent.

`doctor` souffre du même mélange : `written_by_hand` parcourt `src/`, y trouve les
répertoires des fragments, et doit les filtrer par une liste de noms — avec une exception
codée en dur pour `redis`, qui s'installe sous `src/cache/`.

## La décision

Tout ce que `rbs add` dépose dans `src/` va sous `src/modules/`, **sauf `auth`**.

```
src/
  articles/          rbs generate crud      → crate::articles
  posts/             rbs generate crud      → crate::posts
  auth/              rbs add auth           → crate::auth
  health/            squelette
  modules/           rbs add <tout le reste>
    mod.rs             // <rbs:modules>
    audit/  cache/  cors/  jobs/  mail/  observability/
    rate_limit/  scheduler/  storage/  webhooks/
  router.rs  state.rs  lib.rs  openapi.rs
```

### Pourquoi `auth` reste à la racine

`auth` est le seul fragment qui pose une entité métier — `User` — que le développeur étend
comme n'importe laquelle des siennes : il y ajoute des champs, des rôles, des relations
vers ses propres tables. Le ranger parmi les modules d'infrastructure mentirait sur son
usage réel, et casserait le chemin le plus connu de rbs, cité par tous les guides
d'authentification.

La règle tient en une phrase : **`rbs add` écrit sous `src/modules/`, sauf `auth`.**

### Les onze répertoires déplacés

Ce sont des *répertoires*, non des noms de fragments — le fragment `redis` dépose
`src/cache/`, et `rate-limit` dépose `src/rate_limit/` :

| Fragment | Avant | Après |
|---|---|---|
| `audit` | `src/audit/` | `src/modules/audit/` |
| `cors` | `src/cors/` | `src/modules/cors/` |
| `jobs` | `src/jobs/` | `src/modules/jobs/` |
| `mail` | `src/mail/` | `src/modules/mail/` |
| `observability` | `src/observability/` | `src/modules/observability/` |
| `rate-limit` | `src/rate_limit/` | `src/modules/rate_limit/` |
| `redis` | `src/cache/` | `src/modules/cache/` |
| `scheduler` | `src/scheduler/` | `src/modules/scheduler/` |
| `storage` | `src/storage/` | `src/modules/storage/` |
| `webhooks` | `src/webhooks/` | `src/modules/webhooks/` |
| `auth` | `src/auth/` | **inchangé** |

`ci` et `docker` n'écrivent rien dans `src/` — `.github/workflows/ci.yml`, `Dockerfile`,
`docker-compose.yml`, `config/production.toml` — et ne sont donc pas concernés.

## La quatorzième ancre

`// <rbs:modules>`, dans `src/modules/mod.rs`, reçoit les `pub mod <x>;` que l'ancre
`features` recevait jusqu'ici.

```rust
pub(crate) const MODULES: Anchor = Anchor {
    name: Cow::Borrowed("modules"),
    file: Cow::Borrowed("src/modules/mod.rs"),
    comment: "//",
    sorted: true,
    optional: true,
    after: "",
};
```

- `sorted: true` — pour la même raison que `features` : rustfmt trie les `pub mod`, et un
  `add` intercalé entre deux modules ne doit pas faire échouer le `cargo fmt --check` du
  développeur sur une ligne qu'il n'a pas écrite.
- `optional: true` — le fichier porteur n'existe pas sur un projet sans fragment. `doctor`
  ne la réclame donc que si `src/modules/mod.rs` est là, exactement comme il traite déjà
  `jobs` et `services`.

`ANCRES` passe de 13 à 14 entrées. L'ancre `features` de `src/lib.rs` ne reçoit plus, de la
part des fragments, qu'une seule ligne : `pub mod modules;` — et continue de recevoir
`pub mod auth;` du fragment `auth`, ainsi que les `pub mod <entité>;` de `generate crud`.

Une seule ancre change de fichier : `jobs`, qui suit son fragment de `src/jobs/mod.rs` vers
`src/modules/jobs/mod.rs`.

## Le point de montage

`add` gagne une étape, entre « planifier » et « vérifier ». Quand un fragment déclare
l'ancre `modules`, le plan s'assure que le point de montage existe :

- **`src/modules/mod.rs` absent** → le plan ajoute sa création et l'insertion de
  `pub mod modules;` dans `<rbs:features>`.
- **`src/modules/mod.rs` présent** → rien ; l'insertion dans `features` est idempotente
  comme toutes les autres.

Le fichier créé tient en quatre lignes :

```rust
//! Les modules d'infrastructure que `rbs add` installe.

// <rbs:modules>
// </rbs:modules>
```

Le squelette de `rbs new` ne pose rien : un projet sans fragment n'a pas de `src/modules/`,
et `hello-crud` reste exactement ce qu'il est aujourd'hui.

Cette étape ne contourne pas la garde d'ancre absente : un projet dont on a effacé
`<rbs:features>` reçoit toujours le bloc à coller, sans que rien ne soit écrit.

## Les projets déjà générés

`upgrade` ne déplace aucun fichier — il n'écrit que dans `Cargo.toml` et dans les zones
réservées d'`AGENTS.md`, parce que le code posé appartient au développeur dès que
`rbs new` l'a écrit. Cette spec ne change pas ce principe.

Un projet portant `src/mail/` qui lance `rbs add audit` après la mise à jour reçoit donc
`src/modules/audit/`, à côté de son `src/mail/` inchangé. La disposition devient mixte, et
c'est assumé : le déplacement des anciens modules appartient au développeur, qui seul sait
ce qu'il a modifié.

Deux choses lui sont dues :

1. **`doctor` le signale**, en `Check::warned` : si au moins un des onze répertoires existe
   à la racine de `src/` *et* que `src/modules/` existe, le diagnostic nomme les
   répertoires concernés et dit que rbs n'y touchera pas. Un projet entièrement à
   l'ancienne disposition — sans `src/modules/` — ne déclenche rien : il est cohérent,
   simplement antérieur.
2. **`AGENTS.md` est corrigé par `upgrade`**, qui réécrit ses zones réservées. La carte des
   ancres qu'un agent y lit citera `<rbs:modules>` et le nouveau chemin de `<rbs:jobs>`.

Ce diagnostic ne deviendra pas un `--fix`. Déplacer `src/mail/` exigerait de réécrire les
`use crate::mail::` du développeur, c'est-à-dire de toucher à l'AST — ce que rbs refuse par
construction, et ce qui l'exposerait à casser du code qu'il n'a pas écrit.

## Les templates

Les manifestes disent la destination en toutes lettres. Aucun préfixe implicite, aucune clé
`placement` : lire un `feature.toml` continue de dire exactement où chaque fichier
atterrit, comme les ancres du projet sont énumérées et jamais déduites.

```toml
# features/storage/feature.toml

[[files]]
source      = "mod.rs.jinja"
destination = "src/modules/storage/mod.rs"

[[anchors]]
anchor  = "modules"                 # était "features"
content = "pub mod storage;"

[[anchors]]
anchor  = "state_init"
content = "storage: crate::modules::storage::from_config()?,"
```

Trois familles de modifications :

1. **Destinations** — 50 lignes `destination` dans 10 `feature.toml`.
2. **Contenus d'ancres** — les 14 lignes `content` qui citent un chemin de crate passent à
   `crate::modules::…` ; les 10 lignes `anchor = "features"` deviennent
   `anchor = "modules"`, leur `content` restant le même `pub mod <x>;`. `auth` garde
   `features`. S'y ajoutent les commentaires de ces mêmes manifestes, qui citent les
   chemins qu'ils expliquent.
3. **Chemins dans le code rendu** — 20 occurrences de `crate::<module>::` sur 7 fichiers :
   `scheduler/tests.rs.jinja` (7), `webhooks/tests.rs.jinja` (6), `scheduler/mod.rs.jinja`
   (3), `webhooks/delivery.rs.jinja`, `webhooks/service.rs.jinja`,
   `rate-limit/counter.rs.jinja`, et **`templates/feature/service.rs.jinja`**, le gabarit
   de `generate crud`, qui importe `use crate::storage::{Storage, StorageError};`.

Ce troisième point est le seul endroit où le renommage traverse la frontière dans l'autre
sens : une feature utilisateur consomme un module. C'est légitime et ça reste — seul le
chemin s'allonge.

Le prix de ce choix est connu : renommer `modules` un jour coûterait une rectification de
ces 50 lignes. C'est le prix de manifestes qui ne mentent pas.

## `doctor`

- **`ANCRES` passe à 14.** L'ancre `modules` y entre, `optional: true`.
- **`HORS_FEATURES` accueille `modules`.** `cache` y reste, non plus pour les projets neufs
  — qui n'ont plus de `src/cache/` — mais pour le parc existant, qui en porte encore un.
- **`written_by_hand` se simplifie** : `src/` ne contient plus, hors squelette, que les
  features du développeur et `auth`. Le contrôle ne descend pas dans `src/modules/` : le
  contenu de ce répertoire appartient au CLI, l'inventorier n'apprendrait rien.
- **Un contrôle neuf** signale la disposition mixte, selon la règle ci-dessus.

## Exemples, tests, documentation

**Les quatre exemples** se régénèrent par diff entre deux générations, jamais par
écrasement : `file-drop` et `newsletter-queue` portent des éditions manuelles.
`integration_examples` est l'oracle — il échoue tant que les quatre n'ont pas suivi.

| Exemple | Fichiers touchés |
|---|---|
| `blog-auth` | 3 — `state.rs`, `router.rs`, `AGENTS.md` |
| `file-drop` | 4 |
| `newsletter-queue` | 4 |
| `hello-crud` | 1 — la carte des ancres dans `AGENTS.md` |

**Les tests** :

- `crates/rbs-cli/src/add/mod.rs` — 23 occurrences dans les tests unitaires, dont ceux qui
  vérifient qu'une sonde atterrit dans `health_probes` ou un layer dans `layers`. Ces
  chaînes sont figées des deux côtés de la frontière : elles se corrigent avec les
  manifestes, dans le même commit.
- Tests lents, sous Docker : `integration_examples` (17), `integration_webhooks` (5),
  `integration_scheduler` (3), `integration_doctor` (2), `integration_add` (2),
  `integration_new` (2).
- **Trois tests neufs** : `add` crée le point de montage sur un projet vierge ; un second
  fragment n'insère pas `pub mod modules;` une seconde fois ; `doctor` avertit sur une
  disposition mixte et se tait sur une disposition ancienne cohérente.

**La documentation** — 9 pages × 2 langues, ~296 occurrences : `cli/add.md` et les guides
`scheduler`, `jobs`, `webhooks`, `storage`, `mail`, `cache`, `audit`, `observability`.
S'y ajoute un paragraphe neuf, en anglais et en français dans le même commit, sur la
disposition `src/modules/` dans la page de structure du projet : c'est le changement qu'un
lecteur de la 1.3 doit comprendre en premier.

## Versionnage

Aucune API publique de `rbs-core` ni de `rbs-cli` ne change ; ce qui bouge est ce que le
CLI *écrit*. La rupture est réelle pour l'utilisateur et inobservable par le compilateur :
**version mineure**, 1.2.0 → 1.3.0, avec sa note de version au `CHANGELOG` bilingue,
disant ce que `add` pose désormais, ce que les projets existants gardent, et que `doctor`
signale sans corriger.

## Hors périmètre

- Déplacer automatiquement les modules d'un projet existant, par `doctor --fix` ou
  autrement.
- Une clé `placement` dans le format de manifeste : `auth` est la seule exception, une
  liste de deux cas ne mérite pas un concept.
- Descendre dans `src/modules/` pour y inventorier ce qui a été écrit à la main.
- Déplacer `ci` et `docker`, qui n'écrivent rien dans `src/`.
