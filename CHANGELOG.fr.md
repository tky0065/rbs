# Journal des modifications

Tout ce qui arrive de notable à rbs s'écrit ici, pour qui l'installe — et non pour qui lit
le dépôt, ce à quoi sert le journal des commits.

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/). Les versions
suivent le [versionnage sémantique](https://semver.org/lang/fr/spec/v2.0.0.html) de forme
seulement : **aucune promesse de compatibilité n'est faite avant la 1.0**, et l'API
publique de `rbs-core` peut changer entre deux versions mineures sans cycle de
dépréciation.

*[English version](CHANGELOG.md).*

## [Non publié]

### Ajouté

- `rbs generate client --lang ts` écrit un client TypeScript typé depuis le document
  OpenAPI du projet lui-même — une méthode par opération, une interface par schéma, aucune
  dépendance à installer côté TypeScript. Aucun serveur ne tourne : `rbs new` écrit
  désormais un troisième binaire, `src/bin/openapi.rs`, qui imprime ce que rend
  `ApiDoc::openapi()`, et la commande le lance. Le client est une classe `ApiClient`
  configurable plutôt que des fonctions libres, si bien qu'un jeton se pose une fois à la
  construction au lieu d'être enfilé dans chaque appel ; `headers` accepte une fonction
  autant qu'un objet, pour un jeton qui tourne. Il est projeté comme une création :
  régénérer un contrat inchangé n'écrit rien, et un client que vous avez modifié revient en
  conflit plutôt que d'être écrasé. Le binaire vaut par lui-même : `cargo run --bin openapi
  > openapi.json` suivi d'un `git diff` fige le contrat en CI.
- `rbs add webhooks` installe les webhooks sortants : une table `webhook_subscriptions`,
  trois routes pour inscrire, lister et révoquer un abonné, et une fonction `emit` qui enfile
  une livraison signée par abonnement qui écoute. `emit` prend un `&C: ConnectionTrait` et non
  une connexion, pour la raison qui vaut déjà pour `audit::record` : passez-lui la transaction
  qui porte votre changement, et les livraisons naissent si et seulement si elle est committée
  — un événement annonçant une inscription annulée est un mensonge qu'aucun réessai ne
  rattrape. La livraison passe par la file de `jobs` inchangée, et c'est pourquoi le fragment
  l'exige : les réessais, l'attente entre deux tentatives et `last_error` étaient déjà
  prouvés, et une seconde boucle de réessai aurait fait une seconde chose à maintenir. Il
  exige aussi `auth` — une création d'abonnement laissée ouverte permet à n'importe qui de
  faire livrer chez lui les événements du projet. Le corps est signé en HMAC-SHA256 sur
  `<horodatage>.<corps brut>` et porté par `X-Rbs-Signature: t=…,v1=…` : l'horodatage entre
  dans le condensat, ce qui ferme le rejeu. Chaque abonnement a son propre secret, rendu une
  seule fois à la création et jamais par la liste — un secret commun donnerait à chaque abonné
  de quoi contrefaire les événements livrés à tous les autres. L'abonnement est désigné dans
  le job par son identifiant plutôt que recopié, si bien qu'un secret tourné s'applique aux
  livraisons déjà en file et qu'une révocation les arrête.
- `rbs add audit` installe un journal des écritures : une table `audit_log`, un type
  `Entry` et une fonction `record` sous `src/audit/`. `record` prend un
  `&C: ConnectionTrait` et non une connexion, ce qui est toute la raison de mettre le
  journal en base : passez-lui la transaction qui porte votre changement, et la trace naît
  si et seulement si ce changement est committé. `actor_id` est nullable et prend une
  `String` plutôt que l'`Identity` de la feature `auth`, si bien que le fragment s'installe
  sur un service sans JWT et garde traçables les écritures hors requête — un job, un seed,
  une commande d'administration. `action` est une chaîne et non un enum, avec `CREATE`,
  `UPDATE` et `DELETE` pour constantes : `login` et `export` sont des actions légitimes
  qu'un enum fermé ne ferait que forcer à contourner. Le fragment ne monte aucune route et
  ne se câble sur aucun CRUD engendré — quelles écritures méritent une trace est une
  question à laquelle seul votre domaine répond.
- `rbs add scheduler` installe le déclenchement calendaire : une échéance due enfile un
  job dans la file existante, une seule fois quel que soit le nombre de réplicas. Il
  entraîne `jobs` — le scheduler déclenche, il n'exécute pas — et le calendrier se déclare
  en code, dans `src/scheduler/mod.rs`, où `Schedule::every::<J>` tire le `kind` de
  `J::KIND` : une échéance qui viserait un job non inscrit est inécrivable. La réservation
  est un `UPDATE` conditionnel sur `next_run_at`, qui partage sa transaction avec
  l'enfilage — aucun arrêt ne peut donc avancer une échéance sans créer son job. Les
  expressions acceptent cinq champs comme six — une ligne collée d'un crontab est servie,
  pas punie — et s'évaluent en UTC ; une seule illisible arrête le démarrage en la
  nommant.
- `rbs generate crud --with-upload` monte trois routes de contenu sur la ressource
  engendrée — `PUT`, `GET` et `HEAD` sur `/<ressource>/{id}/content` — contre le trait du
  fragment `storage`. Le corps voyage en `application/octet-stream`, pas en JSON : le
  base64 chargerait le fichier deux fois en mémoire. La clé de stockage se dérive de
  l'`id`, si bien qu'aucune colonne ne la porte. Sans la feature `storage`, le drapeau est
  refusé avant tout écrit, en nommant `rbs add storage`. Une borne de taille s'applique à
  la seule route de dépôt, sous forme d'une constante à relever.
- `rbs_core::Cursor` et `CursorPage<T>` paginent sur l'`id` plutôt que sur un offset, pour
  les listes où `OFFSET n` fait parcourir au moteur les lignes qu'il va jeter. La borne
  `after` est exclusive, et la réponse ne porte pas de `total` — le `COUNT(*)` qu'il
  exigerait est le coût que le curseur évite. Le CRUD engendré ne change pas et garde
  `Pagination` : basculer retirerait `total` de toutes les réponses déjà servies.
- `rbs add observability` installe des traces OTLP et un `/metrics` Prometheus. Les traces
  sortent par `rbs-core`, derrière sa nouvelle feature cargo `observability` : c'est
  `logs::init()` qui pose l'abonné global, et rien d'ajouté à l'ancre `// <rbs:startup>`
  ne pourrait ensuite y greffer une couche d'export. `OTEL_EXPORTER_OTLP_ENDPOINT` nomme
  le collecteur — absente, rien n'est exporté — et `rbs_core::logs::shutdown()` vide le
  dernier lot. Les métriques comptent sous le gabarit de route pris du `MatchedPath`
  d'axum, et jamais sous l'URL demandée ; elles sont servies sur un listener à elles, pour
  qu'aucun déploiement n'ait à les cacher derrière une règle de reverse-proxy. `rbs
  doctor` refuse une configuration où ce port égale `server.port`.
- `--fields` accepte un modificateur `max=<n>`, qui borne la longueur d'un champ textuel
  dans les DTO engendrés. Il est refusé sur tout autre type.
- `rbs add cors` installe une couche CORS dont les origines autorisées se lisent dans la
  configuration du projet, et qui n'est jamais grande ouverte par défaut.
- `rbs add rate-limit` installe une limite de débit. Le compteur est un pipeline Redis
  quand le fragment `redis` est là — atomique entre processus — et une fenêtre fixe en
  mémoire sinon ; le fichier engendré dit lequel il porte et pourquoi. Le 429 qu'il rend
  suit le format d'erreur du projet et porte un `Retry-After`.
- `rbs add auth` installe désormais `rate-limit` avec lui, et l'annonce dans le plan avant
  d'écrire quoi que ce soit. `/auth/login` hache un Argon2 même pour une adresse inconnue,
  délibérément : sans limite, cette protection est aussi un moyen d'épuiser la mémoire du
  serveur. La connexion est bornée à 5 tentatives par minute contre 120 en global.
- Une ancre `// <rbs:layers>` dans `src/router.rs`, où un fragment empile un middleware.
  Elle est intérieure à `trace` et `request_id` : une couche ajoutée voit l'identifiant de
  la requête, et ses propres réponses courtes restent dans la trace.
- `rbs new` écrit un `config/production.toml` qui coupe Swagger UI et le document OpenAPI,
  et le service `api` du compose pose `RBS_ENV=production`. Tout déploiement Docker
  publiait les deux jusqu'ici.
- `rbs-core` enregistre une réponse `TooManyRequests` sous `components/responses`.
- `rbs generate crud --soft-delete` rend le `DELETE` logique : la ligne reste, sa colonne
  `deleted_at` datée, et toute lecture l'écarte. Le contrat HTTP ne change pas — 204 à la
  suppression, 404 pour une seconde, 404 en lisant une ligne supprimée — si bien qu'aucun
  client ne le voit. Un champ `unique` fait quitter sa contrainte pour un index restreint
  aux lignes vivantes, ce qui permet de se réinscrire avec une adresse qu'on avait avant.
  **MySQL n'a pas d'index partiel** : la migration engendrée branche à l'exécution et y
  garde une unicité globale, si bien qu'une valeur supprimée y reste réservée. Le contrat
  inchangé vaut pour la feature qui porte le drapeau, non pour celles qui la référencent :
  l'`ON DELETE` d'une clé étrangère ne se déclenche jamais sur une suppression logique, si
  bien que les enfants survivent à un parent supprimé et qu'un `Restrict` ne refuse plus
  rien.

### Modifié

- `rbs new` avertit quand il n'a pas su décomposer l'URL de la base. Aucun
  `docker-compose.yml` n'est écrit dans ce cas, et le projet naissait jusqu'ici sans
  compose et sans un mot — l'absence ne se découvrait qu'à un fichier manquant. Un
  avertissement et non un refus : une socket Unix comme `postgres:///demo` est légitime et
  ne se décompose pas davantage.
- Un champ `string` est borné à 255 caractères dans les DTO engendrés, sans qu'on le
  demande. Rien d'autre ne le bornait — `ColumnDef::string()` rend un `varchar` sans
  longueur sur PostgreSQL — si bien que chaque route publique acceptait une chaîne de
  longueur arbitraire. `text` n'en reçoit aucune par défaut : c'est le type qu'on choisit
  pour dépasser cette borne.
- **La version minimale de Rust passe de 1.85 à 1.94.** La 1.85 ne résolvait déjà plus :
  `sea-orm` 2.0.2 et `sqlx` 0.9.0 exigent la 1.94.0, et Cargo refuse de compiler en
  dessous. Le plancher déclaré décrivait une chaîne d'outils qu'aucune installation ne
  pouvait employer. Un job de CI épinglé sur la 1.94 tient désormais la promesse.
- Le CRUD engendré rend **409** au lieu de 500 sur une violation de contrainte `unique`,
  sur `create` comme sur `update`, et son contrat OpenAPI déclare le statut. Le fragment
  `auth` le faisait déjà ; le gabarit générique faisait l'inverse.
- Le `list` engendré lance sa page et son `COUNT(*)` ensemble par `tokio::try_join!`,
  plutôt que l'un après l'autre.
- `POST /auth/register` ne répète plus l'adresse soumise dans son 409. Le statut dit
  toujours que l'adresse est prise, mais le corps ne la renvoie plus dans les journaux et
  les réponses.
- Un jeton de rafraîchissement présenté deux fois révoque désormais toutes les sessions du
  compte et journalise un avertissement sans donnée personnelle. Jusqu'ici le rejeu se
  contentait d'un 401, laissant une paire volée valide indéfiniment et en silence.
- `rbs dev` annonce l'attente de la base — « en attente de la base (host:port) » — puis un
  point par seconde, au lieu de rester muet jusqu'à trente secondes. Rien ne s'affiche
  quand la base répond du premier coup.
- L'ancre `features` maintient son bloc trié au lieu d'empiler dans l'ordre d'arrivée : un
  projet dont la CI lance `cargo fmt --check` n'échoue plus sur une ligne qu'il n'a pas
  écrite.
- Les modules de feature engendrés ne portent plus de `#![allow(dead_code)]` sur le module
  entier. Un projet engendré aujourd'hui a un `src/lib.rs`, et un item public d'un module
  public reste joignable de l'extérieur du crate : la permission ne masquait plus qu'un
  appel oublié.
- Le courrielleur et le stockage d'objets s'atteignent par `state.mail()` et
  `state.storage()`, comme le cache par `state.cache()`. Leur champ abandonne le
  `#[allow(dead_code)]` qui tenait lieu d'accesseur.

### Corrigé

- Chaque handler qu'engendre le CLI porte un `operation_id`, et la sonde de santé porte son
  `tag`. Sans eux, utoipa dérive l'identifiant du seul nom de fonction, si bien que le
  `list` de deux features entrait en collision dans le document — et qu'un client engendré
  ne pouvait plus nommer ses méthodes. Les cinq routes du fragment `auth`, celle du filtre
  et les trois routes de contenu n'en portaient aucun.

- `rbs dev` nomme le fichier qui a déclenché un redémarrage. Rien à l'écran ne
  distinguait un redémarrage voulu d'un serveur qui venait de mourir de lui-même.
- `POST /auth/login` et `POST /auth/refresh` répondent avec `Cache-Control: no-store` et
  `Pragma: no-cache`, comme la RFC 6749 §5.1 l'exige d'une réponse portant des jetons.
  L'en-tête tient au type `TokenPair` et non aux deux handlers : un troisième, ajouté plus
  tard, le reçoit sans y penser.
- L'écriture dans une ancre suit les fins de ligne du fichier hôte. Sur un dépôt en
  `core.autocrlf=true`, le CLI posait des lignes LF au milieu d'un fichier CRLF, ce que le
  `cargo fmt --check` du workflow `ci` engendré pouvait ensuite refuser. La réparation
  d'une ancre, elle, réécrivait le fichier entier en LF.
- Une zone de l'`AGENTS.md` ne s'ouvre que sur un marqueur seul sur sa ligne. Citer
  `<!-- rbs:inventory -->` dans sa propre prose faisait effacer par `rbs upgrade` tout ce
  qui séparait la citation du vrai marqueur de fermeture.
- L'inventaire des entités ne compte plus les accolades des chaînes et des commentaires.
  Un `format!("{{")` décalait la profondeur et rattachait les entités suivantes au mauvais
  module, et une entité mise au rebut en commentaire de bloc était inventoriée comme
  réelle — l'un et l'autre finissant en `belongs_to` faux.
- `--fields "author_id:uuid,author:references:users"` est refusé au lieu d'engendrer un
  projet qui ne compile pas : les deux champs donnent la même colonne `author_id`, et la
  déduplication porte désormais sur le nom de colonne et non sur le nom déclaré.
- Deux références qui se singularisent pareil — `author` et `authors` — n'émettent plus
  deux fois la même variante `Relation`.
- Un mot de passe de base contenant un `/` est masqué dans l'erreur de connexion.
  L'autorité était coupée au premier `/`, ce qui mettait l'arobase hors d'atteinte et
  laissait le secret dans les journaux.
- Le test engendré pour un champ `references` requis ne viole plus la clé étrangère à sa
  première exécution. Les scénarios qui créent ne sont pas engendrés, un bandeau nomme la
  référence bloquante, et une référence optionnelle part en `null` plutôt qu'en UUID tiré
  au hasard.
- Les corps d'erreur des 500, les descriptions OpenAPI et plusieurs messages de
  configuration sont de nouveau en français : un renommage vers des identifiants anglais
  avait atteint les littéraux.

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
