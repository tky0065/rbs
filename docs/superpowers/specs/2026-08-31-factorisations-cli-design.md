# Factorisations de la dette du CLI · Spécification de design

Date : 2026-08-31
Statut : validé, prêt pour le plan d'implémentation
Portée : `crates/rbs-cli/src/`, refactoring à comportement observable strictement inchangé

## 1. Objectif

Trois duplications transverses du CLI se paient à chaque évolution : la fixture de test
d'un projet neuf, recopiée dix-huit fois ; trois variantes d'erreur et leur constructeur,
recopiés dans sept modules ; et le préambule des commandes qui modifient un projet
existant — `canonicalize → project_root → metadata::read`, puis le garde Git —, écrit
trois fois, suivi du rituel afficher-puis-appliquer écrit trois fois de plus.

**Contrainte cardinale** : aucune sortie du CLI, aucun message d'erreur, aucun code de
sortie ne change. Les messages rendus à l'utilisateur ne bougent pas d'un caractère, et la
suite de tests existante passe sans qu'un test soit affaibli ou supprimé.

Une factorisation qui alignerait deux messages voisins pour n'en garder qu'un est donc
refusée d'avance : deux textes différents restent deux textes, même si le code qui les
porte se ressemble.

## 2. Décisions arbitrées

| # | Décision | Retenu | Écarté |
|---|---|---|---|
| A1 | Où vit la fixture de projet | `#[cfg(test)] mod fixtures` à la racine de la crate, un constructeur en chaîne | Une fonction par variante · un `pub(super)` par sous-module |
| A2 | Forme de la fixture | Constructeur en chaîne (`Project::new().database(..).create()`) | Fonction à N paramètres · struct d'options `..Default::default()` |
| A3 | Forme des erreurs communes | Un type porteur par faute, adopté en variante `#[error(transparent)]` | Macro déclarant les variantes · trait d'erreur commun |
| A4 | `PasUnProjet` | Seul le message des quatre modules qui l'ont identique est mis en commun | Aligner les sept messages sur un seul |
| A5 | `From<metadata::RootError>` | Une macro déclarative, sept invocations d'une ligne | Blanket impl · type d'erreur commun |
| A6 | Préambule des commandes | `metadata::cible()`, générique sur l'erreur de l'appelant | Type d'erreur unique pour les trois commandes |
| A7 | Garde Git | `git::garde()` rendant l'erreur commune | Garde intégrée au préambule |
| A8 | Rituel `--dry-run` puis appliquer | Une fonction de `lib.rs` rendant « appliqué ou non » | Trois blocs laissés en place |

## 3. Tâche 37 — la fixture de projet

### 3.1 Ce qui existe

Dix-huit `fn project…()` de test appellent `new::create(&new::Options { … })`. Les sept
champs varient ainsi :

| Champ | Valeur commune | Varie dans |
|---|---|---|
| `name` | `"demo-api"` | nulle part |
| `database_url` | `"postgres://rbs:rbs@localhost:5432/demo_api"` | `add`, `dev`, `doctor/base` |
| `database` | `Database::default()` | `add`, `dev`, `doctor/base` |
| `features` | `Vec::new()` | `agents`, `dev`, `generate/command` |
| `core_path` | `None` | `upgrade` |
| `template_dir` | `None` | nulle part |
| `lang` | `Lang::Fr` | nulle part |

### 3.2 Ce qui est écrit

Un module `crates/rbs-cli/src/fixtures.rs`, déclaré `#[cfg(test)] mod fixtures;` — il ne
part donc dans aucun binaire livré :

```rust
pub(crate) struct Project { options: new::Options }

impl Project {
    pub(crate) fn new() -> Self;                    // les sept valeurs communes
    pub(crate) fn database(self, Database) -> Self; // ne touche pas à l'URL
    pub(crate) fn url(self, &str) -> Self;
    pub(crate) fn features(self, &[&str]) -> Self;
    pub(crate) fn core_path(self, Option<PathBuf>) -> Self;
    pub(crate) fn create(self) -> (TempDir, PathBuf);
}

pub(crate) fn project() -> (TempDir, PathBuf);      // Project::new().create()
```

Le constructeur en chaîne est préféré à une fonction à cinq paramètres pour la raison que
la contrainte impose : un appelant ne nomme que ce qui le concerne, et aucune valeur de
fixture ne bouge parce qu'un autre module avait besoin d'un paramètre de plus.

`database()` ne touche pas à l'URL : `add/mod.rs` fait aujourd'hui dériver l'une de
l'autre (`database.default_url("demo_api")`) et `doctor/base.rs` les choisit
indépendamment. Les faire dépendre l'une de l'autre dans la fixture changerait la seconde.

Les modules qui posent en plus des fichiers — `doctor/auth`, `mail`, `redis`, `jobs`,
`storage`, `seed::seeded` — gardent leur fonction locale, son doc-commentaire et son nom,
et n'en délèguent que la création du projet.

`lib.rs:183` n'est pas concerné : c'est le code de production de `rbs new`.

## 4. Tâche 38 — les erreurs communes

### 4.1 Ce qui existe

- `Acces { path: String, source: io::Error }`, message `"{path} est inaccessible :
  {source}"`, **au caractère près dans sept énumérations** : `generate::command`, `add`,
  `upgrade`, `seed`, `plan`, `metadata`, `dotenv`.
- `WorkingTreeSale { files: String }`, message identique dans trois : `generate::command`,
  `add`, `upgrade`.
- `PasUnProjet` dans sept, avec **deux messages** : quatre disent « cette commande attend
  un projet rbs : aucun `Cargo.toml` portant `[package.metadata.rbs]` au-dessus d'ici »
  (`seed`, `doctor`, `migrate`, `dev`), trois nomment la commande — « aucun projet rbs
  ici : `rbs add` s'exécute dans un projet créé par `rbs new` » (`add`, `generate`,
  `upgrade`).
- `impl From<metadata::RootError>`, sept fois le même corps de quatre lignes.
- `fn access(path, source) -> Error`, identique au caractère dans `generate::command` et
  `add`.

### 4.2 Ce qui est écrit

Un module `crates/rbs-cli/src/errors.rs` :

```rust
#[derive(Debug, thiserror::Error)]
#[error("{path} est inaccessible : {source}")]
pub(crate) struct Acces { pub path: String, pub source: io::Error }

impl Acces { pub(crate) fn new(path: &Path, source: io::Error) -> Self }

#[derive(Debug, thiserror::Error)]
#[error("le working tree n'est pas propre : {files} — commitez, ou relancez avec --force")]
pub(crate) struct WorkingTreeSale { pub files: String }

pub(crate) const PAS_UN_PROJET: &str = "cette commande attend un projet rbs : …";

macro_rules! depuis_la_racine { … }   // impl From<metadata::RootError> for $Error
```

Chaque énumération concernée remplace sa variante à champs nommés par une variante
porteuse :

```rust
#[error(transparent)]
Acces(#[from] errors::Acces),
```

`#[error(transparent)]` délègue le `Display` au type porté : le texte rendu est celui de
`errors::Acces`, soit exactement l'ancien. Les deux `fn access()` disparaissent au profit
de `errors::Acces::new(...)`, que `?` convertit.

Ce que la variante porteuse coûte : les filtrages `Error::Acces { source, .. }` de
`metadata.rs`, `add/mod.rs` et `plan/mod.rs` deviennent `Error::Acces(faute) if
faute.source.kind() == …`. C'est mécanique et le compilateur les nomme tous.

`PasUnProjet` **n'est pas mise en commun** : quatre modules partagent leur message, trois
nomment leur commande dans le leur. Seul le message des quatre passe par la constante
`errors::PAS_UN_PROJET`, la variante restant déclarée dans chaque énumération — elle est
filtrée nommément par une dizaine de tests, et une variante porteuse ne rendrait pas ces
tests plus lisibles. Les trois messages qui nomment la commande restent littéraux :
« aucun projet rbs ici : `rbs add` … » et « cette commande attend un projet rbs … » sont
deux textes, et l'un n'a pas à devenir l'autre.

La macro `depuis_la_racine!(Error)` remplace les sept `impl From<metadata::RootError>` :
elle suppose seulement que l'énumération porte une variante `PasUnProjet` sans champ et
une variante `Metadata(metadata::Error)`, ce qui est vrai des sept.

## 5. Tâche 39 — le préambule et le rituel

### 5.1 Ce qui existe

`generate::command::plan_for`, `add::plan_for` et `upgrade::plan_for_with` ouvrent sur les
mêmes quatre gestes : canonicaliser le répertoire de lancement, remonter à la racine du
projet, lire le manifeste, garder le working tree. Les deux premiers portent le même
commentaire, au mot près. `lib.rs` répète trois fois les cinq lignes qui, `--dry-run`
posé, disent que rien n'a été écrit et sortent, ou appliquent le plan.

### 5.2 Ce qui est écrit

Dans `metadata.rs`, le préambule, générique sur l'erreur de l'appelant :

```rust
/// Le projet visé depuis `directory`, et son manifeste, lus une seule fois.
pub struct Cible { pub root: PathBuf, pub metadonnees: Metadata }

pub fn cible<E>(directory: &Path) -> Result<Cible, E>
where E: From<crate::errors::Acces> + From<RootError> + From<Error>;
```

Le paramètre de type est ce qui permet aux trois commandes de garder **leur** énumération
d'erreur et **leurs** messages : `?` convertit, et rien du texte rendu ne dépend de la
fonction commune. Les trois bornes sont exactement les trois fautes que le préambule peut
rencontrer — un appelant qui ne les porte pas toutes ne compile pas, ce qui est la
propriété recherchée.

Dans `git.rs`, le garde :

```rust
/// Le working tree, ou la faute qui énumère ce qu'il porte de non commité.
pub(crate) fn garde(root: &Path) -> Result<(), crate::errors::WorkingTreeSale>;
```

Les trois appelants gardent leur condition, qui n'est pas la même — `upgrade` ne garde que
si la mise à niveau a quelque chose à écrire :

```rust
if !options.force { git::garde(&root)?; }                  // generate, add
if !deja_a_jour && !options.force { git::garde(&root)?; }  // upgrade
```

Dans `lib.rs`, le rituel :

```rust
/// Applique le plan, ou dit que `--dry-run` l'a laissé sur le papier.
///
/// Rend `false` quand rien n'a été écrit : l'appelant sort alors sans annoncer une
/// écriture qui n'a pas eu lieu.
fn appliquer(plan: &plan::Plan, force: bool, dry_run: bool) -> Result<bool, plan::application::Error>
```

### 5.3 Ce que la factorisation doit rendre plus visible

La séquence **lire → planifier → vérifier → afficher → appliquer** est une invariante
d'architecture du projet. Après ce lot, elle se lit dans trois noms au lieu de trois
copies : `metadata::cible` (lire), `plan::Builder` (planifier), `git::garde` (vérifier),
`plan::render::plan` (afficher), `appliquer` (appliquer). Aucune étape n'est absorbée dans
une autre, et aucune commande ne perd la maîtrise de l'ordre où elle les enchaîne :
`add` juge son idempotence entre la lecture et le garde, `upgrade` calcule son plan avant
de garder. Une fonction d'entrée qui enchaînerait elle-même lecture *et* garde diluerait
justement cela, et est écartée pour cette raison.

## 6. Ce qui n'est pas fait

- Les messages ne sont pas alignés : deux textes voisins restent deux textes.
- Les signatures des sous-commandes et `cli.rs` ne bougent pas — trois tâches en cours
  travaillent la même zone.
- `doctor/mail.rs` et `doctor/storage.rs` ne rejoignent pas le contrôle de section partagé
  de la tâche 41 : ils accumulent plusieurs défauts, lisent le `.env` en plus de la
  configuration, et leur section n'est qu'un défaut parmi d'autres.
- Rien n'est touché sous `templates/` : `examples/` ne doit pas bouger.

## 7. Preuves attendues

- `cargo test --workspace` vert, **et le nombre de tests inchangé ou supérieur** : c'est la
  preuve qu'aucune fixture, aucun filtrage d'erreur et aucun contrôle n'a disparu dans la
  factorisation.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`.
- `cargo test --workspace --no-fail-fast -- --ignored` : la suite Docker, seule à prouver
  que le CLI fonctionne encore de bout en bout.
- `git status` sans une ligne sous `examples/`.
