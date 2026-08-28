# Journal des modifications

Tout ce qui arrive de notable à rbs s'écrit ici, pour qui l'installe — et non pour qui lit
le dépôt, ce à quoi sert le journal des commits.

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Les versions
suivent le [versionnage sémantique](https://semver.org/lang/fr/spec/v2.0.0.html) de forme
seulement : **aucune promesse de compatibilité n'est faite avant la 1.0**, et l'API
publique de `rbs-core` peut changer entre deux versions mineures sans cycle de
dépréciation.

*[English version](CHANGELOG.md).*

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

[0.4.0]: https://github.com/tky0065/rbs/releases/tag/v0.4.0
