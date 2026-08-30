# Journal des modifications

Tout ce qui arrive de notable à rbs s'écrit ici, pour qui l'installe — et non pour qui lit
le dépôt, ce à quoi sert le journal des commits.

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Les versions
suivent le [versionnage sémantique](https://semver.org/lang/fr/spec/v2.0.0.html) de forme
seulement : **aucune promesse de compatibilité n'est faite avant la 1.0**, et l'API
publique de `rbs-core` peut changer entre deux versions mineures sans cycle de
dépréciation.

*[English version](CHANGELOG.md).*

## [1.1.0] — 2026-08-29

### Ajouté

- `rbs new` écrit un `docker-compose.yml` portant la base du projet, avec les
  identifiants, le nom de base et le port publié tous tirés de l'URL qui lui a été
  donnée. `docker compose up -d` puis `cargo run` suffisent — rien n'est retapé. Rien
  n'est écrit pour un projet SQLite ni pour une URL dont l'hôte n'est pas local, faute
  d'avoir quoi que ce soit à monter dans les deux cas.
- `rbs add docker` insère désormais ses services `api` et `migrate` dans le compose du
  projet, sous le profil `app`, au lieu de déposer un fichier entier — sauf s'il n'y a
  aucun compose où insérer, auquel cas il en écrit toujours un entier, services de
  déploiement compris. Un compose ayant perdu son ancre `# <rbs:services>` n'est pas
  touché, le bloc s'affichant à coller.
- `rbs add redis` et `rbs add mail` insèrent chacun leur propre service —
  `redis:8-alpine`, `axllent/mailpit` — dans le compose du projet, hors de tout profil :
  `docker compose up -d` seul les monte.
- `rbs dev` monte la pile du compose dès que le projet en porte un, que `docker` soit
  installée ou non — le compose est celui du squelette depuis `rbs new`, pas une marque
  du fragment ci-dessus.
- `rbs new` écrit un `AGENTS.md` à la racine du projet : le mode d'emploi de rbs, écrit
  pour un agent plutôt que pour un lecteur. Deux zones y appartiennent à rbs — le guide,
  qui porte la version du CLI l'ayant écrit, et un inventaire du projet — et tout ce qui
  est hors d'elles vous appartient et n'est jamais réécrit. `rbs add` et `rbs generate`
  rafraîchissent l'inventaire ; `rbs upgrade` rafraîchit les deux zones et réécrit le
  fichier s'il manque. La langue suit `rbs new --lang fr|en`, ou la locale à défaut de
  flag, et s'inscrit dans `[package.metadata.rbs].lang`.
- `rbs doctor` contrôle ce fichier — présent, entier, à jour — et nomme, **en
  avertissement**, tout répertoire de `src/` que rien ne déclare. Écrire à la main ce que
  rbs ne couvre pas reste légitime : l'avertissement le dit, et ne change jamais le code
  de sortie de la commande.

### Modifié

- `--with` installe les features qu'il nomme au lieu de les refuser toutes : `rbs new
  mon-api --with auth` échouait auparavant avec une erreur explicite et un code de sortie
  1 ; elle installe `auth` désormais, dans la même passe qui écrit le projet. L'ordre
  d'installation est dérivé des noms — alphabétique — plutôt que de l'ordre où ils ont
  été tapés.
- `--with jobs` est accepté : il était refusé par une liste que l'ajout du fragment avait
  laissée incomplète.

## [1.0.1] — 2026-08-29

### Corrigé

- Les deux crates paraissaient sans README : aucun manifeste n'en déclarait, et les
  fichiers du dépôt vivent hors du paquet — `cargo package` n'emporte rien d'extérieur à
  la crate. Chacune porte désormais le sien.
- La documentation faisait encore passer les nouveaux venus par `--core-path`, le
  contournement d'un noyau absent de crates.io. Il y est publié depuis la 0.4.0. Le flag
  garde sa vraie raison d'être — bâtir un projet contre un noyau local, ce qui est le mode
  de développement de rbs — et le parcours de démarrage ne le mentionne plus.
- `rbs add` documentait six features quand le binaire en livre sept : `jobs` manquait à la
  page comme à sa capture d'aide.
- La page d'architecture décrivait quatre feature flags « vides » du noyau. `auth` porte du
  code depuis la v0.2 ; seules `redis`, `mail` et `storage` réservent encore un nom.

## [1.0.0] — 2026-08-29

L'API publique de `rbs-core` est figée. À partir d'ici, le versionnage sémantique est une
promesse et non une forme : à l'intérieur de la 1.x, rien n'est retiré, renommé ni doté
d'un autre sens, et `cargo-semver-checks` fait échouer la construction plutôt que de
laisser passer. La promesse couvre aussi le format des ancres en commentaires et de
`[package.metadata.rbs]` : un projet engendré par une version du CLI reste lisible par la
suivante. La [page de compatibilité](https://tky0065.github.io/rbs/fr/compatibility) énonce
les cinq périmètres.

### Ajouté

- `rbs upgrade` aligne le manifeste d'un projet existant sur la version du CLI, et affiche
  les notes de migration du saut. Elle n'écrit que dans `Cargo.toml` : le code engendré
  dans vos sources vous appartient dès qu'il est écrit.
- `rbs doctor` nomme désormais cette commande quand il trouve un projet en retard sur le
  CLI, au lieu de décrire un alignement à faire à la main.
- Les notes de migration sont embarquées dans le binaire, une par version qui introduit une
  rupture.

### Modifié

- **Rupture.** 22 types publics de `rbs-core` portent `#[non_exhaustive]` : les 7 enums
  (`Error`, `ConfigError`, `JwtError`, `LogError`, `Status`, `Check`, `LogFormat`) et 15
  structs. Un `match` exhaustif sur un de ces enums réclame désormais un bras `_ =>`, et
  ces structs ne se construisent plus par un littéral hors de la crate — passez par leur
  constructeur, ou par la configuration désérialisée. C'est le prix du gel, et il se paie
  ici parce qu'après la 1.0 il aurait coûté une 2.0.
- `Claims`, `ValidatedJson<T>` et `CommonResponses` en sont délibérément exclus : le code
  qu'écrivent `rbs new` et `rbs generate` les construit ou les déstructure. **Un projet
  engendré traverse cette version sans une ligne à changer.**

### Corrigé

- Le plancher PostgreSQL documenté était 18, exigence tombée depuis que les modèles
  engendrés posent eux-mêmes l'identifiant v7. `rbs doctor` fait respecter 14, et les
  guides le disent désormais.

## [0.4.0] — 2026-08-28

Cette première entrée est la première version publiée : elle ne fait donc qu'ajouter. Elle rassemble
les quatre jalons livrés par le dépôt — le socle, l'authentification, les intégrations et
le confort — en ce qu'une seule installation donne aujourd'hui.

### Ajouté

**La commande `rbs`.** Sept commandes : `new` crée un projet qui démarre, avec sa base, ses
migrations et sa route `/health` ; `generate crud` et `generate feature` écrivent une
feature dans un projet existant ; `add` installe un fragment de feature ; `migrate` pilote
les migrations, `seed` insère les données de démonstration, `dev` relance le serveur à
chaque changement, et `doctor` diagnostique un projet.

**`rbs generate crud`, le CLI d'abord.** Du seul `--fields 'title:string,body:text'`, et
sans base démarrée, il écrit l'entité SeaORM, les DTO, le repository, le service, le
controller, la migration, le seed et les tests d'intégration. C'est l'inverse de
`sea-orm-cli generate entity`, qui exige que les tables existent d'abord.

**Un code généré qui vous appartient.** Chaque feature suit le même moule —
`model · dto · repository · service · controller` — avec une dépendance à sens unique :
`controller → service → repository → model`. C'est du Rust clair, sans macro à déplier et
sans bandeau « généré, ne pas modifier », parce que rien ne vient réécrire vos changements.

**Des ancres plutôt qu'une réécriture d'AST.** Le CLI insère dans des ancres en
commentaires que vous pouvez voir et déplacer (`// <rbs:features>`, `<rbs:routes>`,
`<rbs:openapi>`, `<rbs:migrations>`, `<rbs:state_champs>`, `<rbs:state_init>`). Une ancre
absente n'écrit rien et affiche le bloc à coller. Les commandes qui touchent un projet
existant lisent, planifient, vérifient, affichent, puis appliquent — en tout ou rien, avec
restauration en cas d'échec partiel, et idempotence portée par `[package.metadata.rbs]`.

**`rbs-core`, le runtime.** Des erreurs typées rendues en documents de problème RFC 9457 ;
une configuration chargée depuis `config/*.toml` et l'environnement, validée au démarrage ;
un formateur de logs qui reste lisible en développement et devient du JSON en production ;
la connexion à la base et l'état de l'application ; les middlewares `request_id` et de
trace ; un extracteur JSON validé ; la pagination ; les helpers OpenAPI et une Swagger UI
configurable.

**`rbs add auth`.** Inscription, connexion, rafraîchissement avec rotation du jeton,
déconnexion et révocation, un guard `require_role`, les migrations `users` et
`refresh_tokens`, une énumération `Role`, et les routes inscrites au document OpenAPI.
Derrière, dans `rbs-core` sous la feature `auth` : le hachage Argon2, la signature et la
vérification des JWT, un extracteur `Identity`, et des jetons opaques stockés en empreinte.

**`rbs add redis`, `rbs add mail`, `rbs add storage`.** Un cache typé sur un pool de
connexions ; un transport de courriel avec ses gabarits ; un trait `Storage` avec un
backend fichiers et un backend S3. Les trois se sont ajoutés sans toucher au noyau : un
fragment déclare ses dépendances, sa section de configuration et ses champs d'état dans son
propre `feature.toml`.

**`rbs add jobs`.** Des jobs en arrière-plan, enfilés dans la même transaction que
l'écriture métier qui les déclenche, et un worker qui réserve, réessaie, puis renonce — un
job survit au redémarrage du processus qui l'exécutait.

**`rbs dev`.** Démarre les services dont le projet a besoin, applique les migrations en
attente, puis lance le serveur et le relance à chaque changement de source.

**`rbs seed`.** Les données de démonstration, dans `src/seeds/` avec leur propre binaire.
`generate crud` y dépose le seed de l'entité qu'il vient de créer, et la commande refuse de
s'exécuter sous `RBS_ENV=production` sauf mention contraire.

**Trois moteurs de base.** `rbs new --database postgres|mysql|sqlite`. Les identifiants
sont des UUID v7 posés par l'application et non par la base, ce qui fait se comporter les
trois moteurs de la même façon ; `rbs-core` ne nomme plus PostgreSQL nulle part.

**`rbs doctor`.** Vérifie les ancres, le `.env`, si la base répond, les versions employées,
et la configuration de chaque feature installée.

**Quatre projets d'exemple**, compilés en CI sur Linux, macOS et Windows, et servant de
source à chaque extrait de code de la documentation : `hello-crud`, `blog-auth`,
`file-drop` et `newsletter-queue`.

**Un site de documentation bilingue**, à l'adresse <https://tky0065.github.io/rbs/fr/> :
démarrage, architecture, référence du CLI et guides, en français et en anglais.

### Prérequis

Rust 1.85 ou plus, édition 2024. Un projet généré tourne sur PostgreSQL 14 ou plus,
MySQL 8.0 ou plus, ou SQLite 3.35 ou plus — `rbs doctor` refuse tout ce qui est en dessous.

[1.0.1]: https://github.com/tky0065/rbs/releases/tag/v1.0.1
[1.0.0]: https://github.com/tky0065/rbs/releases/tag/v1.0.0
[0.4.0]: https://github.com/tky0065/rbs/releases/tag/v0.4.0
