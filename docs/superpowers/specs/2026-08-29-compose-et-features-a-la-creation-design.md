# Un compose dès la création, et un `--with` qui installe

**Date** : 2026-08-29
**Portée** : `rbs-cli` — `new`, `add`, `dev`, `doctor`, le squelette et les fragments
**Version visée** : `1.1.0` du workspace

## 1. Le problème

Un projet neuf ne démarre pas sans qu'on lui monte une base à la main. La documentation
l'enseigne ainsi (`docs/docs/getting-started.md:69`) :

```bash
docker run --rm -d --name rbs-demo \
  -e POSTGRES_USER=rbs -e POSTGRES_PASSWORD=rbs -e POSTGRES_DB=demo \
  -p 5432:5432 postgres:18
```

Cette ligne dit trois choses que rbs sait déjà : le moteur, les identifiants, le nom de la
base. Elle est écrite à la main parce que le squelette n'écrit aucun compose, alors que le
CLI connaît l'URL qu'il vient d'inscrire dans le `.env`.

Trois défauts s'y ajoutent, découverts en suivant le parcours d'un utilisateur.

**La question des features est une impasse.** `new::validate_features`
(`crates/rbs-cli/src/new.rs:212`) refuse *toute* feature, connue ou non. La question
« Features à installer ? » et le flag `--with` proposent donc des choix dont aucun
n'aboutit :

```
> Features à installer ? docker, ci
erreur : `docker` ne s'installe pas à la création : créez le projet sans `--with`,
         puis `rbs add docker`
```

**Trois listes de features cohabitent, et aucune n'est juste.** `prompts.rs:11` en propose
deux sur un commentaire périmé (« les autres arrivent en v0.2 ») ; `new.rs:24` en connaît
six et oublie `jobs`, si bien que `--with jobs` répond « n'est pas une feature rbs » alors
que `rbs add jobs` fonctionne ; `cli.rs:51` en annonce sept dans l'aide. Les fragments
embarqués sont sept.

**Le compose de la feature `docker` est employé à contre-emploi.** C'est un compose de
déploiement : il bâtit l'image de l'API, lance `migrate`, puis `api`, et **ne publie pas**
le port de la base — décision délibérée, épinglée par
`templates.rs:733` `the_docker_compose_publishes_only_the_api_port`. Or `rbs dev` fait
`docker compose up -d` sur ce fichier, puis attend une réponse sur `localhost:5432` lue
dans le `.env`. Ce port n'est pas publié : l'attente ne peut pas aboutir. Et le `up`
rebâtit au passage l'image de l'API pour un serveur qui, lui, tourne en `cargo watch` sur
l'hôte.

## 2. Ce que la solution vise

`rbs new demo` suivi de `docker compose up -d` et de `cargo run` doit donner une API qui
répond, sans qu'une seule valeur soit recopiée d'un fichier à l'autre.

`rbs new demo --with auth,redis` doit livrer un projet qui contient `auth` et `redis`.

`rbs dev` doit tenir la promesse de son aide — « services, migrations, serveur » — sur un
projet neuf, sans `rbs add docker` préalable.

## 3. Le fichier engendré

`crates/rbs-cli/templates/project/docker-compose.yml.jinja` est écrit à la racine du
projet créé. Il devient le **17ᵉ** fichier du squelette.

```yaml
name: demo

services:
  db:
    image: postgres:18-alpine
    environment:
      POSTGRES_USER: rbs
      POSTGRES_PASSWORD: rbs
      POSTGRES_DB: demo
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U rbs -d demo"]
      interval: 2s
      timeout: 3s
      retries: 30

  # <rbs:services>
  # </rbs:services>

volumes:
  pgdata:
```

### 3.1 L'identité vient de l'URL

`POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB` et le port publié sont tirés de
`RBS_DATABASE__URL`, la même valeur que le `.env` reçoit. C'est ce qui rend
`docker compose up -d && cargo run` vrai sans rien éditer. La template de la feature écrit
aujourd'hui `postgres/postgres` en dur, ce qui ne correspond au `.env` que par accident.

Le contexte de rendu de `new::render` gagne quatre variables : `database_user`,
`database_password`, `database_name`, `database_port`. Elles sont dérivées par l'analyseur
d'URL qui existe déjà dans `doctor/base.rs` — étendu, non dupliqué : deux analyseurs
divergents feraient publier un port que l'application ne joint pas.

### 3.2 Le port publié est celui du `.env`

C'est le renversement d'une décision existante, et il est délibéré. Le compose de
déploiement ne publiait pas 5432 parce que l'API l'atteignait par le réseau du compose.
Ici l'application tourne sur l'hôte : le port **doit** être publié, sans quoi le fichier ne
sert à rien.

Le défaut reste 5432, qui est déjà ce que la documentation fait taper à la main. Un
conflit avec un PostgreSQL local se voit immédiatement au `up`, dans les mots de Docker.

### 3.3 Quand aucun compose n'est écrit

**SQLite** n'a pas de serveur : aucun fichier n'est engendré. La condition est
`Database::a_un_serveur()`, celle que la template de la feature emploie déjà.

**Une URL distante** — un hôte qui n'est ni `localhost`, ni `127.0.0.1`, ni `::1` — ne
donne pas lieu à un compose non plus. Engendrer une base locale que `rbs dev` monterait
pendant que l'application en interroge une autre serait pire que ne rien écrire.

Dans ces deux cas, le squelette compte 16 fichiers, comme aujourd'hui.

### 3.4 L'image

`postgres:18-alpine` reste le défaut, épinglé par `templates.rs:868`
`the_docker_compose_targets_the_latest_stable_postgres`. C'est un choix de défaut pour un
projet neuf, non un plancher de support : le plancher reste PostgreSQL 14, ce que
`prompts.rs:109` annonce et ce que les tests d'intégration exercent.

Le montage porte sur `/var/lib/postgresql` et non sur `/var/lib/postgresql/data` : la 18 y
place ses données sous `18/docker`, et le montage des versions antérieures ne persisterait
rien.

### 3.5 Ce que la commande affiche

```
✓ demo créé — 17 fichiers

  cd demo
  docker compose up -d   # PostgreSQL 18 sur 5432, base « demo » créée
  cargo run              # ou `rbs dev`, qui enchaîne les deux
```

## 4. L'ancre `services`

### 4.1 Une ancre en commentaire YAML

Le CLI ne réécrit jamais d'AST : il insère dans des ancres en commentaires. C'est ce
mécanisme qu'on emploie, et non une fusion de YAML.

`Anchor::opening()` (`crates/rbs-cli/src/anchors.rs:19`) code en dur `// <rbs:{name}>`. La
structure gagne un champ :

```rust
pub(crate) struct Anchor {
    pub name: &'static str,
    pub file: &'static str,
    /// Marqueur de commentaire du langage porteur : `//` en Rust, `#` en YAML.
    pub comment: &'static str,
}
```

Les huit ancres existantes portent `"//"`, la nouvelle `"#"`. `ANCRES` passe de 9 à 10
entrées — c'est cette liste que `rbs doctor` parcourt pour vérifier qu'un projet les porte
toutes.

```rust
pub(crate) const SERVICES: Anchor = Anchor {
    name: "services",
    file: "docker-compose.yml",
    comment: "#",
};
```

### 4.2 Le groupement des lignes insérées

`anchors::groups` (`anchors.rs:192`) tient une ligne commençant par `//` ou `#[` pour une
ligne qui en qualifie une autre, et la rend indivisible de la suivante. Un commentaire YAML
est `# ` : il ne correspond ni à l'un ni à l'autre, et compterait comme une ligne autonome,
dédupliquée seule. Un commentaire inséré par un fragment se retrouverait orphelin de son
service dès qu'un autre fragment aurait posé la même ligne.

`groups` reconnaît donc `# ` en plus des deux marqueurs actuels.

### 4.3 Les profils Compose

Un fichier unique sert deux usages. Les services de développement n'ont pas de profil : ils
montent au `docker compose up -d` nu, donc à chaque `rbs dev`. Les services de déploiement
que `rbs add docker` insère portent `profiles: ["app"]` : ils ne montent que sur
`docker compose --profile app up`.

C'est un mécanisme natif de Compose. Rien n'est à coder dans le CLI, et le fichier reste
lisible par quelqu'un qui n'a jamais lu cette spec.

### 4.4 Ce que `rbs add docker` devient

`docker-compose.yml` cesse d'être un fichier déposé et devient une insertion, comme
`auth`, `jobs`, `mail`, `redis` et `storage` en font déjà. Le fragment conserve
`Dockerfile` et `.dockerignore` en `[[files]]`.

| État du projet | Ce que fait `rbs add docker` |
|---|---|
| Compose présent, ancre présente | insère `api` et `migrate` sous `profiles: ["app"]` |
| Aucun compose (projet SQLite, ou créé avant la 1.1.0) | écrit le fichier entier, ancre comprise |
| Compose présent, ancre absente | n'écrit rien, affiche le bloc à coller |

La troisième ligne est la convention du projet, pas une règle nouvelle. La deuxième
conserve la template d'aujourd'hui comme repli — un projet SQLite n'ayant jamais de
compose engendré, c'est le seul chemin par lequel il peut en obtenir un.

### 4.5 Les services que `redis` et `mail` déposent

Ces deux fragments annoncent aujourd'hui une adresse que rien ne sert : `redis` écrit
`url = "redis://127.0.0.1:6379"` dans `config/default.toml`, et `mail` y écrit
`smtp_port = 1025` sous le commentaire « Mailpit et MailHog écoutent sur 1025 ». Le
service était à la charge de l'utilisateur, faute d'un fichier où le déposer.

Chacun gagne un `[[anchors]] anchor = "services"` dans son `feature.toml` :

- `redis` → un service `redis:8-alpine`, port publié celui de sa configuration ;
- `mail` → un service `axllent/mailpit`, SMTP sur 1025 et interface web sur 8025.

Sans profil : ce sont des dépendances de développement, elles doivent monter au `up` nu.

## 5. `rbs new` installe les features

### 5.1 Une seule liste

`prompts::FEATURES_DISPONIBLES` et `new::FEATURES_CONNUES` sont supprimées au profit de
`templates::embedded_names()` (`templates.rs:136`), qui lit les répertoires de fragments
embarqués. Une feature ajoutée au binaire ne peut plus être oubliée d'une liste. La
fonction passe `pub(crate)`.

### 5.2 La séquence

```
valider  →  rendre  →  écrire le squelette  →  installer les features  →  git init
```

Chaque feature passe par `add::plan_for` puis l'application du plan — le pipeline existant,
et non un second chemin d'écriture.

**Les noms sont validés avant que rien ne s'écrive**, comme le nom du projet et l'URL le
sont déjà. `Error::FeatureAVenir` disparaît ; `Error::FeatureInconnue` reste, sa liste
dérivée d'`embedded_names()`.

**L'ordre d'installation est celui d'`embedded_names()`, donc alphabétique**, quel que soit
l'ordre de frappe. Les insertions dans le `Migrator` et dans `# <rbs:services>` suivent
l'ordre d'installation : deux `--with` équivalents doivent produire deux projets
identiques au bit près.

**Le projet est entier, ou il n'existe pas.** L'échec de l'installation d'une feature
retire toute la racine — la même garantie qu'aujourd'hui pour l'échec d'écriture, sûre pour
la même raison : le répertoire n'existait pas avant la commande.

**`git init` reste en dernier.** `add` refuse un working tree sale, mais `git.rs:29` écarte
les fichiers non suivis : la séquence n'est pas dictée par cette contrainte. Elle l'est par
le résultat — un dépôt initialisé après coup a le projet complet dans son premier
`git add`, features comprises, plutôt qu'un squelette suivi de sept modifications non
commises.

### 5.3 Ce que la commande rend compte

```
✓ demo créé — 17 fichiers
  + auth   9 fichiers, 1 migration
  + redis  3 fichiers

  cd demo
  docker compose up -d
  cargo run
```

Le compte de la première ligne est celui du squelette seul ; chaque feature rend le sien,
sur sa propre ligne. Additionner les trois nombres en un seul effacerait la distinction
entre ce que `new` a écrit et ce qu'`add` y a posé — distinction que `rbs doctor` et
`[package.metadata.rbs]` maintiennent par ailleurs.

## 6. `rbs dev` et `rbs doctor`

`dev::plan_with` (`dev/mod.rs:147`) ne cherche le compose que si `[package.metadata.rbs]`
déclare la feature `docker`. Cette condition perd son objet : le compose existe désormais
sans la feature. Elle devient « le fichier existe ».

Le test qui vérifie qu'un projet sans la feature ne fait pas même de `stat` sur le disque
perd sa prémisse et est supprimé. C'est une suppression assumée : ce qu'il protégeait
n'existe plus.

La commande lancée ne change pas d'une lettre — `docker compose -f <file> up -d`. Avec les
profils, elle monte `db`, plus `redis` et `mailpit` s'ils ont été insérés, et ne bâtit
jamais l'image de l'API.

Le conseil de `dev/mod.rs:102` — « démarrez-la — `docker compose up -d` si la feature
docker est installée » — cesse d'être conditionnel.

`doctor` gagne la dixième ancre par simple parcours d'`ANCRES`. Son contrôle « base
joignable » nomme le compose du projet quand il en existe un.

## 7. Ce qui doit être prouvé

### 7.1 Les tests qui changent de sens

Ils encodent des décisions ; en retourner un en silence est la façon dont un projet perd sa
mémoire.

| Test | Aujourd'hui | Demain |
|---|---|---|
| `templates.rs:733` `…publishes_only_the_api_port` | le compose ne publie que 8080 | le compose du squelette publie le port du `.env` |
| `templates.rs:868` `…targets_the_latest_stable_postgres` | vise la template de la feature | vise celle du squelette |
| `templates.rs:584` `DESTINATIONS_DOCKER` | trois fichiers | deux : le compose est inséré, non déposé |
| `dev/mod.rs` — le projet sans feature ne sonde pas le disque | prémisse vivante | supprimé |
| `new.rs` — les tests du refus de `--with` | prouvent le refus | prouvent l'installation |

### 7.2 Les preuves nouvelles

- Un test d'intégration `assert_cmd` : `rbs new demo --with auth,redis --yes`, puis
  `cargo build` du projet engendré. C'est le seul test qui prouve que les insertions de
  deux fragments dans un projet neuf compilent ensemble.
- `docker compose config` sur le fichier produit, dans les trois états de §4.4 : un compose
  syntaxiquement invalide ne se voit pas autrement.
- Le rendu du squelette pour une URL distante et pour SQLite : aucun compose, 16 fichiers.
- Le parcours nominal joué à la main : `rbs new`, `docker compose up -d`, `cargo run`,
  `/health` qui répond — sans un `docker run` tapé.

### 7.3 La documentation

Bilingue dans le même commit, et `npm run parite` à 0 écart sur les 24 paires de pages.

- `getting-started.md` — la section « Starting a database » et son `docker run` tombent :
  le fichier engendré la remplace. Le compte « 16 fichiers » passe à 17.
- `cli/new.md` — la section « `--with` in this version » est fausse d'un bout à l'autre et
  se récrit ; le tableau des flags aussi.
- `cli/dev.md` — le compose n'est plus conditionné à la feature.
- `cli/add.md` — `add docker` insère au lieu de déposer, et ses trois cas.
- `guides/cache.md`, `guides/mail.md` — le service est monté par `rbs dev`.
- `README.md` et `README.fr.md`, si leur parcours de démarrage montre le `docker run`.

Toutes les sorties de terminal recapturées sur le binaire recompilé. Aucun extrait écrit à
la main.

### 7.4 Version et note de migration

Le gel porte sur l'API publique de `rbs-core`, que rien ici ne touche : c'est un `1.1.0` du
workspace.

`crates/rbs-cli/notes/1.1.0.md` est **obligatoire** — le contrôle de complétude de
`notes.rs:69` échoue s'il manque une note pour le saut depuis la dernière version publiée.
Elle dit les deux choses qu'un utilisateur doit savoir :

- `--with` installe désormais au lieu de refuser ; un script qui comptait sur le code de
  sortie 1 change de comportement ;
- un projet créé avant la 1.1.0 n'a pas de compose. `rbs add docker` lui en écrit un
  entier ; `rbs upgrade` ne le lui ajoutera pas, cette commande n'écrivant que dans un
  manifeste.

## 8. Hors périmètre

- Un `README.md` dans le squelette engendré.
- Un service pour la feature `storage` — MinIO servirait le backend `s3`, alors que le
  défaut du fragment est `backend = "fs"`, qui n'a besoin de rien.
- La fusion de YAML. Le CLI insère dans une ancre ou n'écrit pas : il ne lit pas un
  document YAML pour le récrire.
- `rbs upgrade` ne rétro-installe rien dans un projet existant.
