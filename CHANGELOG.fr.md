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

- **`rbs routes` et `rbs generate client` prennent un `--from <FICHIER>`**, un contrat déjà
  figé par `rbs openapi export --out openapi.json`, lu au lieu de compiler le projet. `rbs
  routes --from` n'exige même pas un projet rbs autour de lui. C'est ce qu'il faut à une CI
  qui commite son contrat pour le relire ou pour construire son client sans chaîne de
  compilation Rust, et l'échappatoire quand le contrat mémorisé ci-dessous se trompe. `rbs
  openapi export` n'en a délibérément pas : c'est la commande qui *produit* le contrat, et
  lire un fichier pour en écrire un autre la réduirait à une copie.

- **Un projet engendré porte désormais un `Makefile`, et ne dépend plus de son générateur.**
  Jusqu'ici, les seuls gestes documentés passaient par le CLI — `rbs dev`, `rbs migrate` — si
  bien que le collègue qui clonait le dépôt devait installer `rbs-cli` avant de taper quoi que
  ce soit. Le squelette écrit à la place treize raccourcis : `dev`, `back`, `build`, `test`,
  `lint`, `fmt`, `migrate`, `seed`, `up`, `down`, `openapi`, `clean`, et un `help` qui se lit
  dans le fichier lui-même et que rend un `make` nu. Chaque recette enveloppe `cargo`, `npm`
  ou `docker compose` ; aucune n'appelle `rbs`. `make dev` mène de front tout ce que le projet
  porte, dans un même groupe de processus — un seul Ctrl-C arrête l'ensemble — sans aucune
  dépendance nouvelle : un `trap 'kill 0' INT TERM EXIT` et un `wait`. Les noms de cibles sont
  les mêmes dans toutes les langues ; seules les descriptions qu'affiche `make help` suivent
  `--lang`.

- **Une ancre `# <rbs:make>`**, la vingt-deuxième du registre et la troisième au marqueur `#`
  de Git, est là où un fragment pose un raccourci à lui — `rbs add frontend` y écrit `front`,
  `front-build`, `typecheck` et sa moitié de `make dev`, `rbs add docker` y écrit `image`. Un
  fragment n'en pose un que s'il apporte un exécutable de plus à lancer : `jobs` et
  `observability` n'en posent aucun. L'ancre est optionnelle, pour la même raison que
  `# <rbs:ignore>` : le squelette écrit bien le fichier, mais celui-ci appartient au
  développeur, qui peut l'avoir supprimé. Le registre passe de vingt-et-une ancres à
  vingt-deux, dont onze optionnelles.

### Modifié

- **Le contrat est mémorisé, et les trois commandes qui le lisent ne recompilent plus le
  projet à chaque fois.** `rbs routes`, `rbs openapi export` et `rbs generate client` lancent
  toutes le binaire `openapi` du projet — une compilation complète en profil dev d'un projet
  Axum + SeaORM + utoipa, de l'ordre de la minute sur cible froide — et se tapent d'ordinaire
  l'une après l'autre. Le document atterrit désormais dans `target/rbs/openapi.json`, indexé
  par le condensat SHA-256 du chemin **et** du contenu de chaque fichier de `src/` et de
  `migration/src/`, plus `Cargo.lock` : un fichier retouché, renommé ou supprimé l'invalide,
  et une montée de dépendance aussi, qui déplace le contrat sans toucher une ligne du projet.
  Sur un projet neuf, un premier `rbs routes` prend 30,2 s et le suivant 12 ms. `target/` est
  déjà ignoré par git et déjà effacé par `cargo clean`, et c'est le motif de cet emplacement
  plutôt qu'un répertoire à soi. Le cache ne fait jamais échouer une commande : un `target/`
  non inscriptible, un document tronqué ou un condensat illisible se soldent tous par une
  simple recompilation.

- **`rbs generate crud` réécrit le client engendré à sa suite**, sur un projet qui en porte
  déjà un. L'écran d'administration qu'il émet importe `@/api/client` : jusqu'ici `npm run
  typecheck` était cassé par construction après chaque génération, et la commande se
  contentait d'afficher le geste à faire. Le client est réécrit depuis le **contrat**, jamais
  depuis la spec d'entité que la commande vient de produire : le déduire donnerait au client
  deux producteurs tirant de deux sources, et rien ne dirait laquelle ment le jour où elles
  divergent. La génération paie donc une recompilation incrémentale, par construction — le
  module vient d'être ajouté, et le contrat mémorisé est invalide à l'instant où la commande
  en a besoin. Cela reste conditionnel : un projet sans client garde l'ancien comportement,
  la commande affichant la ligne `rbs generate client` plutôt que d'inventer un
  `frontend/src/api` que personne n'a demandé. Un projet qui ne compile pas reçoit un
  avertissement et le geste à relancer ; l'entité et sa migration sont sur le disque dans les
  deux cas.

- **La section `[auth]` est désormais dédoublée dans `config/development.toml`.**
  `login_requires_verification` reste à `true` dans `config/default.toml`, qui gouverne la
  production, et vaut `false` sur un poste de travail : un compte inscrit par l'API n'est
  jamais vérifié, et aucun écran engendré n'appelle `/auth/verify-email`. `app_url` passe
  de `http://localhost:3000`, un port où rien n'écoute, à `http://localhost:8080` par
  défaut — le port sur lequel le binaire sert le client construit — et
  `http://localhost:5173`, celui de Vite, en développement.

### Corrigé

- **Un projet engendré avec `auth` n'avait aucun compte capable d'entrer dans son espace
  d'administration.** `register` ne fixe aucun rôle et la colonne `users.role` défaut à
  `"user"` : aucun chemin de l'API ne produisait d'administrateur — quand l'écran de
  connexion affirmait à l'utilisateur que les identifiants étaient ceux de la table des
  comptes que le projet porte, une table vide. `rbs add auth` dépose désormais
  `src/seeds/admin.rs` et le déclare dans le binaire des seeds du projet : `rbs seed` écrit
  un compte portant `Role::Admin`, son adresse déjà datée comme vérifiée. Ses identifiants
  sont les nouvelles `ADMIN_EMAIL` et `ADMIN_PASSWORD` — l'adresse déduite du nom du projet,
  le mot de passe tiré à l'installation — écrites dans le `.env` que git ignore, des repères
  restant dans le `.env.example` versionné, et nommées par les gestes de suite du fragment.
  Le seed n'écrit rien si le compte existe, rien si l'une des deux variables manque ou est
  vide, et refuse de tourner sous `RBS_ENV=production` ; ce dernier refus vit dans le seed
  et non dans `rbs seed`, par où `cargo run --bin seed` ne passe jamais.

- **Le serveur de développement relaie désormais les préfixes qu'ajoute `rbs generate crud`.**
  `frontend/vite.config.ts` relayait une liste figée — `/health`, `/docs`, `/api-docs`, plus
  `/auth` quand le shell d'administration est là — quand chaque table engendrée ajoute son
  propre préfixe de route : sous `npm run dev`, l'écran d'administration que la commande venait
  d'écrire appelait `/articles` sur le port de Vite, qui lui rendait l'application au lieu d'une
  page de lignes. Une ancre `// <rbs:vite_proxy>` vit dans cette liste, et `rbs generate crud` y
  inscrit le préfixe de la table, de façon idempotente, sur tout projet portant `frontend` —
  `--no-admin` ne la retire pas, le relais visant l'API et non l'écran. Le registre passe de
  vingt ancres à vingt-et-une, dont dix optionnelles. Un projet engendré avant cette version
  porte la liste figée sans l'ancre : `rbs doctor` la nomme et affiche le bloc, et `rbs doctor
  --fix` la repose sous `'/api-docs',`.

## [1.7.0] — 2026-09-19

### Ajouté

- **`rbs add frontend`** pose une application Vue 3 dans un projet existant : l'application,
  son routeur, un thème Tailwind v4, quatorze composants shadcn-vue vendorisés, et un accueil
  public dont le contenu par défaut est vrai au premier démarrage — il nomme le projet réel,
  interroge sa sonde de santé, renvoie à la documentation OpenAPI que le service sert déjà, et
  montre une commande `curl` qui marche. Un module dédié sert le build en production et
  retombe, tant que ce build n'existe pas, sur une page d'amorçage autonome qui nomme les
  commandes restant à taper ; la page s'efface dès que le build a écrit. Le repli
  n'intercepte ni l'API ni le document OpenAPI, et un rechargement en route profonde rend
  l'application plutôt qu'un 404. `cargo build` réussit toujours sur une machine sans Node :
  le mécanisme des fragments reste déclaratif, si bien que le CLI ne lance jamais `npm`. Le
  `.gitignore` reçoit les répertoires de dépendances et de build du client, par une ancre
  neuve `# <rbs:ignore>`.

- **`rbs add frontend-admin`** ajoute le shell d'administration dans l'arbre que le socle a
  posé : quinze fichiers, un espace authentifié de quatre écrans de compte et de santé — la
  connexion avec sa demande de réinitialisation, un tableau de bord des sondes réelles, les
  sessions ouvertes avec leur révocation unitaire et globale, le profil avec son changement de
  mot de passe — chacun adossé à une route qu'`auth` expose réellement. Une application, deux
  régimes de route : l'espace d'administration est un morceau paresseux derrière une garde de
  route. Deux stores Pinia et pas plus ; toute ressource passe par le client typé qu'écrit
  [`rbs generate client`](https://tky0065.github.io/rbs/fr/cli/client) depuis le document OpenAPI
  du projet, que le shell importe et qui est donc la commande à lancer avant le premier
  `npm run build`. Il exige `frontend` et `auth`, et par `auth`, `mail` et `rate-limit`.

- **`rbs generate crud` émet les écrans d'administration de la table** sur un projet portant
  `frontend-admin` : la liste filtrée, le formulaire et le détail, montés dans le rail et la
  table de routage par deux ancres neuves — `// <rbs:admin_routes>` et
  `// <rbs:admin_rail>`, le registre passant de dix-huit à vingt. Aucun drapeau ne le
  demande, pas plus qu'aucun ne demande les routes fermées que vaut `auth` ; `--no-admin` est
  la sortie de secours, pour une table que personne ne doit administrer depuis l'interface.
  Sans le fragment, la commande se comporte exactement comme avant. L'écran de démonstration
  que le fragment dépose et chaque écran engendré sortent de la même template, si bien que ce
  qu'on voit à l'installation est ce qu'on obtiendra ensuite. C'est un renversement de
  portée : « interface d'administration générée » quitte le hors-périmètre de la feuille de
  route, où elle figurait depuis l'origine.

- **Un champ `next_steps`** dans la section `[feature]` du manifeste d'un fragment : une
  liste de lignes, rendues comme le reste et affichées après l'application du plan. Rien n'est
  exécuté — le CLI reste hors-ligne et déterministe.

### Corrigé

- **`rbs generate client --lang ts`** rendait tout composant non-objet en
  `export interface X <union>`, qui n'est pas du TypeScript : toute entité portant un champ
  `enum` déclare un tel composant, et le client entier cessait de s'analyser à cause de lui.
  La panne était invisible au CLI et ne paraissait qu'à `npm run typecheck`. Ces composants se
  déclarent désormais en alias de type.

## [1.6.0] — 2026-09-19

### Ajouté

- **`rbs remove <feature>`** retire une feature qu'`rbs add` avait installée : ses
  fichiers, les lignes qu'elle avait insérées dans chaque ancre, sa migration et — quand
  plus aucun autre fragment installé ne les réclame — ses dépendances. Elle suit la même
  séquence lire → planifier → vérifier → afficher → appliquer que toute autre commande
  qui touche un projet, dans l'ordre inverse de celui qui avait posé le fragment, et
  refuse avant toute écriture sur quatre points : un nom qui n'a jamais été un fragment ;
  un dépendant, trouvé par fermeture transitive, qui l'exige encore — retirer `mail`
  quand `auth` et `webhooks` sont installées nomme les deux d'un coup ; un fichier qui a
  divergé d'un rendu neuf du fragment, `--force` passant outre ; et, comme `add`, un
  working tree Git sale. Retirer un fragment déjà absent est un succès, pas une erreur —
  la même idempotence qu'`add`, lue sur `[package.metadata.rbs]` plutôt que sur les
  fichiers du disque. La commande n'ouvre jamais de connexion à la base : une migration
  qu'elle retire laisse ses tables en place, et `rbs migrate down` doit s'exécuter
  avant. Les variables d'environnement, les fichiers qu'un fragment ne pose que s'ils
  manquent, et les dépendances que le squelette ou un autre fragment installé déclarent
  encore ne sont jamais retirés, seulement nommés dans le rapport — `.env` est gitignoré,
  la seule écriture qu'aucun `git checkout` ne défera jamais. Ce que le code du
  développeur appelle encore n'est pas non plus recherché : `cargo build` est l'oracle, et
  le rapport le dit. `--json` rend le même document qu'`add`, `fichiers` portant un
  troisième compteur, `supprimes`, à côté de `crees` et `modifies`.

- **`rbs add api-keys`** authentifie des machines plutôt que des personnes. Une clé
  présentée en `X-Api-Key` vaut `Identity` partout où vaut un jeton de session — un CRUD
  engendré six mois plus tôt l'accepte sans une ligne à récrire, parce que les deux moyens
  se rejoignent dans l'unique extracteur que traverse déjà chaque handler. Le noyau a reçu
  pour cela une seule méthode additive, `HasAuth::accept_key`, dont le défaut refuse : un
  projet qui n'installe jamais le fragment continue d'écarter toute clé, et aucune
  implémentation 1.x du trait ne rompt. Ce qui borne cette portée, c'est le rôle. Chaque
  clé porte le sien, plafonné à chaque requête par celui de son porteur — `min(clé,
  porteur)` recalculé plutôt que figé à l'émission, si bien que déclasser un compte
  déclasse ses clés du même geste, et qu'une clé en lecture seule d'un administrateur
  devient quelque chose qu'on peut réellement confier. La clé elle-même n'est rendue
  qu'une fois, à la création, et n'est conservée que hachée ; `expires_at` est facultative
  et `expires_in_days` est bornée à 1..3650, si bien qu'aucun appelant n'atteint
  l'arithmétique qui déborderait. `last_used_at` est tracée à la minute, et la décision de
  l'écrire se prend en mémoire avant tout aller-retour — c'est ce qui évite qu'une clé
  employée mille fois par seconde ne fasse mille écritures. La révocation est un geste à
  part : `DELETE /api-keys/{id}` laisse les sessions intactes et `DELETE /auth/sessions`
  laisse les clés intactes, parce qu'une clé fuitée et un portable volé ne sont pas le
  même incident. Quatre routes, un schéma de sécurité `api_key` dans le document OpenAPI,
  et une dix-septième ancre — `<rbs:auth_impl>`, la seule à vivre dans un bloc `impl`, que
  `rbs doctor` contrôle et propose à coller quand la délégation manque. Le client
  TypeScript engendré ne présente pas de clé : il est écrit pour la session du navigateur.

## [1.5.0] — 2026-09-12

### Ajouté

- **`Identity::user_uuid()` et `Claims::user_uuid()`** lisent l'identifiant de l'appelant
  en `Uuid`, et rendent `Error::Unauthorized` quand `sub` n'en est pas un. Le fragment
  `auth` appelait `Uuid::parse_str` avec la même conversion d'erreur à sept endroits ;
  chacun appelle désormais la méthode, comme peut le faire un CRUD engendré qui veut
  l'auteur d'une écriture.
- **`rbs_core::db::redact_url`** rend une URL de connexion au mot de passe remplacé par
  `***` — le masquage que `db::connect` appliquait déjà à ses propres erreurs. Les
  fragments `redis` et `rate-limit` l'appellent pour citer `[cache] url` dans un journal ;
  toute autre URL d'un projet qui porte un secret peut en faire autant.
- **`rbs generate crud` et `rbs generate feature` prennent `--singular <NOM>`** pour les
  cas où l'heuristique de singularisation se trompe : `rbs generate crud news` nommait
  ses types `CreateNew` et ses liaisons `new`, schémas OpenAPI et interfaces TypeScript
  compris. `news`, `series` et `species` sont désormais reconnus invariables sans le
  drapeau ; tout autre cas se règle par `--singular news_item`. La valeur doit être en
  snake_case, comme le nom de la feature.
- **Un projet engendré s'arrête proprement sur Ctrl-C ou SIGTERM.** `main.rs` sert
  désormais avec `with_graceful_shutdown` : l'écoute cesse d'accepter, les requêtes en vol
  finissent, le worker de `jobs` achève le job qu'il exécute et le ticker de `scheduler`
  son tour, `main` les attend au plus la nouvelle clé `server.shutdown_timeout_secs`
  (défaut `30`), puis appelle lui-même `rbs_core::logs::shutdown()` — plus de dernier lot
  de spans perdu à `docker stop`, ni de job laissé `running` jusqu'à l'échéance du bail.
  Le signal vient de `rbs-core` : `CoreState::shutdown()` rend un `Shutdown` sous lequel
  toute tâche de fond se détache et qu'elle écoute, si bien qu'aucune ancre n'est ajoutée
  et que le contenu de l'ancre `startup` ne change pas. Le message de `rbs add
  observability` ne demande plus d'appeler `logs::shutdown()` soi-même. Un projet engendré
  avant 1.5.0 garde son ancien `main.rs`, que `rbs upgrade` ne réécrit pas ; la note de
  montée de version dit quoi coller.
- **Le worker de `jobs` exécute plusieurs jobs de front, et un job raté attend plus
  longtemps à chaque fois.** `[jobs] concurrency` (défaut `4`) borne le nombre de jobs
  qu'un worker exécute à la fois — une livraison webhook qui attend un receveur lent ne
  retient plus toute la file — et chaque job tourne dans une tâche à lui, si bien qu'un
  job qui panique ne tue plus le worker. Le délai de reprise vaut désormais
  `retry_delay_secs × 2^(tentative − 1)`, plafonné par la nouvelle clé
  `retry_max_delay_secs` (défaut `3600`). Les deux clés ont un défaut : un projet engendré
  avant 1.5.0 continue de fonctionner sans elles, et reprend `src/modules/jobs/worker.rs`
  et `queue.rs` du fragment quand il veut le comportement. `rbs doctor` propose les deux
  clés dans le bloc qu'il imprime quand la section manque.
- **Chaque release GitHub porte des binaires précompilés, que `cargo binstall rbs-cli`
  trouve.** Un tag joint désormais `rbs` et `rbs-cli` pour Linux (x86_64 et aarch64,
  compilés contre la glibc d'Ubuntu 22.04), macOS (Intel et Apple silicon) et Windows
  (x86_64), chaque archive avec sa somme SHA-256, à une release GitHub dont les notes
  sont la section de ce fichier pour la version. `rbs-cli` déclare
  `[package.metadata.binstall]` : `cargo binstall rbs-cli` télécharge l'archive de votre
  plateforme au lieu de compiler — installer rbs sur un runner de CI ne coûte plus une
  compilation d'axum et de sea-orm.
- **`rbs generate job <nom>` écrit un job de la file, et `--every "<cron>"` son
  échéance.** Un seul plan crée `src/modules/jobs/<nom>.rs`, déclare le module entre les
  nouvelles balises `// <rbs:job_modules>`, l'inscrit dans `// <rbs:jobs>` et, sous
  `--every`, le pousse dans le calendrier par la nouvelle `// <rbs:schedules>`.
  L'expression est jugée avec la crate et la normalisation du démarrage du projet, avant
  toute écriture. La commande exige `jobs` (et `scheduler` sous `--every`), et refuse un
  nom qui est un mot-clé Rust, un module de la file, `jobs`, ou une crate que nomme le code
  de la file — déclaré dans `src/modules/jobs/mod.rs`, il masquerait cette crate. Sur
  un projet engendré avant 1.5.0, les ancres manquent : le
  fichier du job s'écrit, et la déclaration, l'inscription et l'échéance — chacune
  suppose la précédente — s'affichent à reporter plutôt que de s'écrire sans ce qu'elles
  nomment. Un projet qui a reçu `jobs` ou `scheduler` avant 1.3.0 est refusé, avec le
  déplacement à faire à la main.
- **`rbs generate migration <nom> --add-column <table> --fields "…"` écrit une migration
  d'évolution du schéma.** Une cinquième sous-commande de `generate`, à côté de `crud`,
  `feature`, `client` et `job`. Elle écrit un fichier,
  `migration/src/m<horodatage>_<nom>.rs`, déclaré et inscrit dans les deux ancres de toute
  migration engendrée : `up` empile un `alter_table().add_column()` par champ — une
  instruction par colonne, SQLite n'acceptant qu'une modification par `ALTER TABLE` — et
  `down` les défait dans l'ordre inverse, les index avant les colonnes qu'ils nomment. Le
  fichier déclare son propre `Iden` minimal : la table, les colonnes qu'il ajoute, et rien
  d'autre. Une colonne ajoutée doit être `optional` — la table porte déjà des lignes, qui
  n'ont pas de valeur pour elle. `unique` et `references` sont refusés sur les trois
  moteurs, SQLite ne sachant ajouter après coup ni contrainte d'unicité ni clé étrangère,
  et un `decimal` sous SQLite reçoit le refus que `generate crud` prononce déjà ; un
  `decimal` demande toujours au manifeste ce que le type exige. Comme `model.rs` et
  `dto.rs` ne portent pas d'ancre et que le CLI ne réécrit pas d'AST, les lignes qui leur
  reviennent sont affichées plutôt qu'écrites — y compris, pour un champ `enum(a,b,c)`, le
  type `DeriveActiveEnum` à coller, tel que `generate crud` le rend.
- **`rbs doctor` contrôle sept fragments de plus.** `cors` avertit d'un `origins` vide,
  `rate-limit` veut sa section, `scheduler` lit chaque expression littérale du calendrier
  comme le démarrage la lira, `webhooks` veut la livraison inscrite à la file, `audit` sa
  migration déclarée et dans le `Migrator`, `docker` le `config/production.toml` que son
  compose sélectionne, `ci` son workflow. Chacun nomme un projet qui compile puis se
  comporte mal. `scheduler` et `webhooks` lisent un projet qui les a reçus avant 1.3.0 là
  où il les porte encore, sous `src/`. Après la montée, un projet qui porte `cors` voit un
  nouvel avertissement tant qu'il n'a pas énuméré les origines de son front.
- **Un projet engendré répond à `GET /health/live`, et son image Docker le sonde.** La
  nouvelle route rend `200` sans rien interroger : une sonde de vie liée à la base ferait
  redémarrer l'API en boucle par un orchestrateur, pendant une panne de base qu'aucun
  redémarrage ne répare. `/health` ne change pas et répond toujours à la question de la
  disponibilité, base et sondes comprises. L'image que construit `rbs add docker` déclare
  un `HEALTHCHECK` sur la nouvelle route, parlé par bash et `/dev/tcp` puisque l'image ne
  porte ni curl ni wget. Un projet engendré avant 1.5.0 garde son module de santé et son
  `Dockerfile`, qu'aucune mise à niveau ne réécrit ; la note de mise à niveau donne les
  lignes à coller.
- **`rbs new` écrit un `CLAUDE.md` d'une ligne qui importe `AGENTS.md`.** Claude Code lit
  `CLAUDE.md`, et n'atteint le guide que par son import `@AGENTS.md`. `rbs upgrade` crée
  le fichier sur un projet qui ne l'a pas, et ne réécrit jamais un fichier existant.
- **`rbs test` lance toute la suite de tests d'un projet comme le fait sa CI.** Il monte les
  services du compose, attend la base, applique les migrations, puis lance
  `cargo test --workspace --no-fail-fast -- --include-ignored` — la commande même du
  workflow de `rbs add ci`. Un filtre et des arguments pour libtest passent tels quels
  (`rbs test articles -- --nocapture`), et le code de sortie de `cargo test` revient
  inchangé : un script distingue un test rouge d'un échec du CLI. Le guide des tests part
  désormais de lui.
- **`rbs routes` énumère les routes d'un projet, et `rbs openapi export` imprime son
  document OpenAPI**, sans démarrer de serveur : tous deux lisent ce qu'imprime le binaire
  `openapi` du projet, comme `rbs generate client`. `routes` montre méthode, chemin,
  `operation_id` et garde — `bearer` ou `public` —, avec `--json` pour un script ;
  `openapi export` écrit sur la sortie standard, ou dans le fichier que nomme `--out`,
  relatif au répertoire de lancement.
- **`rbs generate crud --cursor` pagine la liste par curseur.** `GET /<ressource>` prend
  `after` et `per_page` et rend un `rbs_core::CursorPage` : pas de `COUNT(*)`, et une ligne
  insérée entre deux requêtes ne décale plus la fenêtre. La route de filtre garde ses pages
  numérotées, un curseur sur l'`id` étant faux dès que le tri porte sur une autre colonne.
  Les tests engendrés parcourent les pages jusqu'à l'extinction de `next` ;
  `--soft-delete`, `--role`, `--with-upload` et `--has-many` s'y combinent.
- **Toute commande qui planifie prend `--json`.** `rbs add`, `rbs generate crud`, `feature`,
  `client` et `job`, et `rbs upgrade` impriment alors un seul document JSON sur la sortie
  standard au lieu du plan en couleurs : chaque action avec son contenu complet, les
  insertions à reporter avec leur `bloc` et leur `cause`, et le compte des fichiers créés
  et modifiés, `applique` disant si quelque chose a été écrit. Un refus devient un document
  `erreur` portant `code`, `message`, `remede` et `bloc`, code de sortie inchangé ; les
  codes sont stables, et énumérés dans le guide des agents. Un argument que l'analyseur
  refuse reste du texte, code 2.

- **Un projet engendré compresse ses réponses.** Le bloc `<rbs:layers>` du squelette porte
  désormais une `CompressionLayer`, et `tower-http` gagne la feature `compression-gzip` :
  `/api-docs/openapi.json`, qui grossit à chaque CRUD, et chaque liste partent compressés
  en gzip vers tout client qui l'accepte. Le prédicat par défaut épargne les petits
  corps, les images et les flux SSE, et le squelette y ajoute `application/octet-stream` :
  un fichier servi tel quel garde son `content-length`, et une archive déjà compressée ne
  l'est pas une seconde fois. Un projet engendré avant garde son routeur ; la note de
  montée donne les lignes à coller.
- **`HasAuth::accept_in(&claims, &mut extensions)`**, une méthode fournie qu'`Identity`
  appelle désormais à la place d'`accept`, les extensions de la requête à portée. Son
  défaut appelle `accept` : un projet qui n'implémente qu'`accept` se comporte comme avant.
  Le fragment `auth` implémente les deux : `accept_in` laisse dans la requête la date de
  vérification du compte qu'il lit — cette date seule, et non la ligne avec son hash de mot
  de passe — et `VerifiedIdentity` l'y reprend : une lecture de `users` par requête sur une
  route derrière la garde, au lieu de deux. Un projet engendré plus tôt garde sa garde, qui
  relit toujours le compte elle-même.
- **`--fields` prend trois types de plus : `date`, `enum(a,b,c)` et `decimal`.**
  `due:date` donne une colonne `Date` — un `chrono::NaiveDate`, un `date()` dans la
  migration, un `"2026-09-15"` en JSON — là où `datetime` imposait une heure que personne
  n'avait. `status:enum(draft,published)` déclare dans le `model.rs` de la feature une
  énumération `DeriveActiveEnum` nommée d'après le champ, une variante par valeur, et
  borne la colonne à la plus longue d'entre elles sous un
  `CHECK (status IN ('draft', 'published'))` que tiennent PostgreSQL, MySQL 8.0.16+ et
  SQLite ; les valeurs sont en snake_case, distinctes et au moins une, et l'analyseur ne
  coupe plus `--fields` sur une virgule placée entre parenthèses. `price:decimal` donne un
  `rust_decimal::Decimal` et une colonne `DECIMAL(19, 4)` — écrite en toutes lettres,
  MySQL ramenant un `DECIMAL` nu à `DECIMAL(10, 0)` — portée en JSON par une chaîne
  (`"12.5000"`), pour qu'aucun centime ne se perde dans un flottant : en déclarer un
  ajoute au manifeste du projet `rust_decimal` (feature `serde-str`) et la feature
  `with-rust_decimal` de sea-orm, et un nombre JSON est dès lors refusé plutôt qu'arrondi
  en silence. **SQLite refuse `decimal`** avant toute écriture, sqlx-sqlite écartant
  délibérément le décimal exact ; le message propose `float` ou un entier en centimes. Le
  noyau gagne ce qu'exigent les filtres engendrés : l'opérateur `OneOf<T>` — `eq`, `in`,
  `is_null`, lu d'une valeur nue ou d'un objet, qu'une colonne énumérée prend à la place
  de `Comparison` — et les schémas de documentation `OneOfSchema<T>`,
  `DateComparisonSchema` et `DecimalComparisonSchema`. `OneOfSchema` prend en paramètre
  l'énumération engendrée : le document nomme dès lors les valeurs acceptées du côté du
  filtre autant que dans le corps de la réponse.

### Modifié

- **Les tests engendrés se rangent par préoccupation.** Les fichiers de tests que reçoit
  un projet ne courent plus sur des centaines de lignes — `add auth` posait un
  `session.rs` de 1300. Chaque fragment qui livre de longs tests — `auth`, `jobs`,
  `scheduler`, `storage`, `webhooks` — les range désormais dans un répertoire `tests/` :
  un `mod.rs` qui porte le harnais partagé, et un fichier par route ou par mécanisme,
  autour de 250 lignes au plus. `rbs generate crud` fait de même : la feature reçoit
  `src/<nom>/tests/` au lieu de `src/<nom>/tests.rs`, ses scénarios de cycle de vie,
  d'erreurs, de filtre, d'accès et de contenu chacun dans son fichier, écrit seulement si
  les options lui donnent de quoi éprouver. `jobs/queue.rs` devient `jobs/queue/`, dépôt,
  réservation et issue d'un job dans trois fichiers ; chaque chemin qu'appelle le projet
  reste le même. Un projet engendré plus tôt garde ses fichiers.
- **Un compte ne se connecte qu'une fois son adresse vérifiée.** `register` rend le même
  202 à une adresse neuve et à une prise, mais une connexion avec le mot de passe tout
  juste soumis les distinguait encore : le compte neuf se connectait, la prise non. La
  section `[auth]` du fragment `auth` gagne `login_requires_verification`, `true` par
  défaut : un compte non vérifié reçoit désormais le 401 d'un mauvais mot de passe, après
  le même Argon2, et le lien de vérification — dans Mailpit en développement — précède la
  première connexion. Une réinitialisation du mot de passe vérifie aussi l'adresse : son
  jeton est arrivé dans la boîte comme un lien de vérification, et un compte non vérifié
  que ce 401 envoie vers `forgot-password` le retrouverait sinon avec son nouveau mot de
  passe. `false` rend la connexion dès l'inscription, et l'écart avec elle. Un projet
  engendré plus tôt garde son `login` ; la note de montée de version donne les lignes à
  changer.
- **`rbs` parle français de bout en bout dans son aide et ses erreurs d'usage.** clap
  écrivait en anglais ce qui lui revient — `Usage:`, `Commands:`, `Options:`,
  `Print help`, `[default: …]`, `[possible values: …]`, et chaque erreur d'usage
  (`error:`, `tip:`, `For more information, try '--help'`) — autour de descriptions
  françaises. Les en-têtes, les drapeaux `-h` et `-V`, la sous-commande `help`, les
  valeurs par défaut et possibles, et les erreurs d'usage courantes — argument inconnu,
  valeur invalide, commande inconnue, argument manquant, conflit — sont désormais en
  français ; une erreur d'usage sort toujours avec le code de clap, 2. Les complétions
  du shell décrivent elles aussi les options en français.

- **`rbs add jobs` et `rbs add scheduler` portent chacun une ancre de plus, et
  `schedules()` s'écrit en instructions.** `// <rbs:job_modules>` se tient sous
  `pub mod worker;`, et `// <rbs:schedules>` sous `let mut calendrier = Vec::new();` — le
  calendrier a quitté son littéral `vec![]`, où une ancre ne survit pas à rustfmt dès
  qu'un second élément s'y ajoute. Sur un projet engendré plus tôt, `rbs doctor` échoue sur
  `job_modules` absente, que `rbs doctor --fix` repose, et n'avertit que pour `schedules` :
  `schedules()` doit d'abord être réécrite à la main, et un projet sain ne doit pas faire
  échouer une CI entre-temps. La note de montée donne la forme à coller.

- **Un abonnement webhook ne peut plus atteindre le réseau du projet.** Hors du profil
  `development`, `POST /webhooks/subscriptions` rend 400 à une URL qui n'est pas en
  `https` et à tout hôte qui est une adresse de boucle locale, privée, de lien local ou de
  CGNAT, ou `localhost`. À la livraison, l'hôte est résolu et filtré de nouveau, dans le
  résolveur du client HTTP aussi, et les redirections ne sont jamais suivies. Une
  livraison interdite est abandonnée plutôt que réessayée. Les abonnements inscrits avant
  cette version sont jugés à la livraison par la même règle. La politique lit le profil
  dans la configuration que reçoit `AppState::new` : le squelette joue donc l'ancre
  `// <rbs:state_init>` avant que `core: CoreState::new(db, config)` ne la consomme, et
  `rbs doctor --fix` repose cette ancre à sa place quand le fichier ne la porte plus du
  tout.

- **`rbs-core` passe à argon2 0.6 et jsonwebtoken 11**, derrière sa feature `auth`, sans
  changement d'API publique. Un mot de passe haché avant la montée reste vérifiable et un
  jeton émis avant reste accepté ; un jeton signé après est identique octet pour octet à
  celui d'avant, si bien qu'un déploiement progressif ou un retour arrière garde toutes
  les sessions ouvertes. `rsa` entre toujours dans le verrou par le backend `rust_crypto`
  de jsonwebtoken : l'exception `RUSTSEC-2023-0071` demeure.
- **`rbs add redis` écrit `redis = "1.7"` et `rbs add storage` `aws-sdk-s3 = "1.146"`**
  (au lieu de `"1.6"` et `"1.144"`). Un projet engendré auparavant résout déjà ces
  versions par sa propre exigence ; relever le plancher dans son `Cargo.toml` la rend
  explicite.
- **Les contrôleurs d'`auth` n'envoient plus eux-mêmes de courriel.** Un helper unique
  `notify`, dans la couche service, rend et expédie chaque message ;
  `verification::send_link` et `password::send_reset_link` portent les émissions, et
  `service::register` reçoit désormais le client mail et les réglages des parcours. Objets
  et gabarits ne changent pas. Un rendu en
  échec est journalisé une fois, sous « préparation du courriel échouée » avec un champ
  `gabarit`, au lieu d'un message par parcours. Seul un `rbs add auth` neuf écrit cette
  disposition ; un projet existant garde la sienne.
- **Les réponses d'erreur parlent la langue du projet, et `--lang` les couvre désormais.**
  Un projet engendré répondait en deux langues à la fois — un `title` anglais
  (`"Not Found"`) à côté d'un `detail` français (`"article introuvable"`) — quoi que dise
  `rbs new --lang`, qui ne choisissait que la langue d'`AGENTS.md`. La nouvelle clé
  `[server] lang` de `config/default.toml` (`"fr"` par défaut, ou `"en"`) est désormais la
  langue du projet pour tout ce que voit un client : à l'exécution, `rbs-core` y écrit le
  `title` et le `detail` fixe de chaque corps `application/problem+json` et les
  descriptions communes du document OpenAPI (`RBS_SERVER__LANG` la surcharge là), et
  `rbs add` et `rbs generate crud` la lisent — dans `config/default.toml` seul, jamais
  dans l'environnement — pour écrire les messages qu'ils adressent au client
  (`"too many requests: try again later"`, `"this value is already taken"`…). `rbs new --lang` l'écrit à côté de
  `[package.metadata.rbs] lang`, qui ne décide plus que de la langue d'`AGENTS.md`.
  `Error::Domain` garde son `code` pour `title`, les codes de validation restent ceux de
  `validator` ; les journaux, les commentaires, les courriels et les textes OpenAPI par
  opération restent en français. **Les `title` d'un projet français passent eux aussi en
  français** (`"Introuvable"`, `"Conflit"`, `"Validation échouée"`…) : un client qui
  compare le `title` plutôt que le `status` doit suivre. Un projet engendré avant cette
  version n'a pas la clé et reste en français ; `rbs upgrade` ne réécrit rien. Pour le
  passer en anglais, poser `lang = "en"` sous `[server]` — l'exécution et tout `add` ou
  `generate` ultérieur la suivent —, puis traduire à la main les messages déjà engendrés
  dans `src/`.

- **`POST /auth/register` rend 202 sans corps, que l'adresse soit neuve ou prise.** Elle
  rendait 201 et le profil, et 409 — avant de hacher — pour une adresse prise : le statut,
  et le temps de réponse, disaient à qui essayait plusieurs adresses lesquelles étaient
  inscrites. Argon2 tourne désormais dans les deux branches. Une adresse neuve voit
  toujours son compte écrit avant la réponse, si bien qu'un client se connecte aussitôt ;
  une adresse prise n'est pas touchée, et son titulaire reçoit
  `templates/mail/inscription.html`. Cela vaut pour les projets neufs : un projet engendré
  plus tôt garde son code, et la note de montée liste les fichiers à reprendre du fragment.
  La réponse seule ne dit plus rien ; une connexion avec le mot de passe soumis le dit
  encore, puisqu'un compte non vérifié se connecte — le guide auth dit ce qui le fermerait.

- **`forgot-password`, `resend-verification` et l'inscription émettent leurs jetons dans
  une tâche détachée.** La requête ne fait plus que lire le compte ; la purge,
  l'invalidation, l'écriture du jeton et le rendu du courriel ont lieu après la réponse, et
  leurs échecs vont au journal avec l'identifiant du compte — attendre ces écritures
  laissait le temps de réponse dire si une adresse était inscrite.

- **Les liens de réinitialisation et de vérification portent leur jeton dans le
  fragment.** `…/reset-password#token=…` plutôt que `?token=…` : un navigateur n'envoie
  jamais le fragment à un serveur, si bien que le jeton reste hors des journaux d'accès et
  des en-têtes `Referer`. Le client le lit dans `location.hash`.

- **Le code de sortie dit à un script de quelle nature est l'échec.** `1` : le projet
  porte une faute que la commande a trouvée ou qui l'arrête — `rbs doctor` qui trouve
  quelque chose, une ancre absente ou mal placée, une migration qui échoue. `2` : l'appel
  est à corriger — hors d'un projet, une feature inconnue, un nom déjà pris, un working
  tree sale, un conflit que `--force` lèverait — comme les erreurs d'usage que clap
  rendait déjà en `2`. `3` : l'environnement a manqué — un fichier illisible, `docker`
  ou `cargo` impossibles à lancer, une base qui ne répond pas, un CLI plus ancien que le
  projet. Tout échec sortait en `1` : une CI ne distinguait pas `rbs doctor` qui trouve
  une faute de `rbs doctor` qui n'a pas pu tourner. `rbs test` garde le code de
  `cargo test` quand un test échoue, et un script qui ne teste qu'un statut non nul ne
  voit aucune différence.

- **Le plan et le bilan comptent à part les fichiers créés et modifiés.** Le pied du plan
  dit `3 à créer, 6 à modifier, 2 inchangés`, et le bilan `✓ cors installée — 3 créés,
  6 modifiés` : `rbs add cors` annonçait neuf fichiers à écrire, puis se disait installé
  en trois, ne comptant que ceux qu'il avait créés. `rbs generate` et `rbs generate job`
  suivent ; la sortie `--json` ne change pas.

- **`rbs doctor` ne signale qu'une fois une clé absente du `.env`.** Le contrôle `.env`
  nomme déjà chaque clé que `.env.example` déclare et que le `.env` n'a pas, avec la
  ligne à ajouter. Les contrôles `auth` et `mail` n'échouent plus une seconde fois sur le
  même `RBS_AUTH__SECRET` ou `RBS_MAIL__SMTP_PASSWORD`, avec un autre remède, et `base`
  avertit qu'il n'a pas pu vérifier la base au lieu d'échouer sur `RBS_DATABASE__URL`
  manquante. Une clé absente aussi de `.env.example` reste signalée par le contrôle de
  sa feature.

- **La file `jobs` inscrit le sort d'un job sans relire sa ligne.** `mark_done` et
  `retry_or_fail` émettent un `UPDATE` ciblé : `ActiveModel::update` rendait la ligne
  entière, payload compris — par `RETURNING` sur PostgreSQL et SQLite, par un `SELECT`
  de plus sur MySQL — pour un modèle que personne ne lisait.

- **Un motif d'événement vide reçoit un 422, comme une URL invalide.**
  `POST /webhooks/subscriptions` vérifiait les motifs blancs dans le service et répondait
  400 ; la vérification se tient désormais sur le DTO, à côté de `#[validate(url)]`, si
  bien que le refus est une erreur de validation qui nomme `events` dans les `errors` du
  problème. Un projet engendré plus tôt garde son 400 tant qu'il ne réengendre pas le
  fragment.

- **Un projet engendré déclare sa MSRV, un profil release et un build Docker en cache.**
  Le manifeste gagne `rust-version = "1.94.1"` — un patch au-dessus de celle de
  `rbs-core`, qu'exige la famille `aws-sdk` que tire `storage` : cargo refuse lui-même une
  toolchain trop vieille pour le projet, au lieu de laisser le
  build échouer sur une édition ou une syntaxe qu'elle ne connaît pas. Une table
  `[profile.release]` pose `lto = "thin"`, `codegen-units = 1` et `strip = true`. Le
  `Dockerfile` qu'écrit `rbs add docker` épingle la mineure, `rust:1.94-slim-trixie`, au lieu de
  `rust:1` flottante, et construit sous des montages de cache BuildKit pour le registre et
  `target/` : un commit ne recompile plus toutes les dépendances. Un projet engendré plus
  tôt garde son manifeste et son `Dockerfile` ; les deux changements se recopient à la
  main.

- **`storage` lit ses objets en flux, et dépose des `Bytes` sans copie.** Le `get` du
  trait chargeait l'objet entier en mémoire — `fs::read` côté fichiers, `collect()` puis
  `to_vec()` côté S3 —, et la route de contenu engendrée copiait chaque dépôt par
  `Bytes::to_vec()`. `get` rend désormais un `Object`, sa taille quand le backend la
  connaît et son contenu en flux lu à mesure que le client le consomme, et
  `GET /<module>/{id}/content` sert ce flux avec son `content-length`. `put` prend des
  `bytes::Bytes`, transmis par l'extracteur tels quels. Le fragment gagne `bytes`,
  `futures-util` et `tokio-util`. Un projet engendré avant garde son trait ; la note de
  mise à jour 1.5.0 dit les retouches qui adoptent le flux.

### Retiré

- **`rbs-core` perd ses features vides `redis`, `mail` et `storage`.** Elles
  n'activaient rien depuis la v0.3 : les trois vivent en fragments engendrés dans le
  projet, et aucun `feature.toml` ni aucun exemple ne les nommait. Un manifeste qui en
  active une sur `rbs-core` ne se résout plus tant qu'elle y figure — voir la note de
  mise à jour.

### Corrigé

- **Le mot de passe Redis n'atteint plus les journaux.** Les fragments `redis` et
  `rate-limit` citaient `[cache] url` telle quelle dans l'erreur d'un pool
  inconstructible, mot de passe compris. Tous deux passent désormais par
  `rbs_core::db::redact_url`.

- **Tout `.env` qu'écrit rbs est en `0600` sous Unix.** `rbs new`, `rbs add` et tout
  autre plan qui écrit le fichier — sa restauration comprise — le laissaient aux droits
  du umask, `0644` d'ordinaire : lisible de tout compte de la machine, mot de passe de la
  base et secret de signature avec lui. Les droits se posent désormais sur le descripteur
  avant l'écriture du contenu, ce qui referme aussi le `.env` d'un projet antérieur au
  prochain plan qui le touche — `rbs add auth`, par exemple. `.env.example` garde des
  droits ordinaires.

- **La CI engendrée épingle ses actions par SHA.** `rbs add ci` écrivait
  `actions/checkout@v7`, `dtolnay/rust-toolchain@stable` et `Swatinem/rust-cache@v2`,
  et un tag peut être déplacé vers un autre commit dans le dos du projet. Chaque action
  est désormais épinglée par son SHA, la version en commentaire, avec `toolchain: stable`
  écrit en toutes lettres puisque le SHA ne le porte plus ; le fragment dépose aussi
  `.github/dependabot.yml`, qui en propose les montées chaque semaine.

- **`rbs generate crud --with-upload` écrit les tests de ses trois routes de contenu.** Le
  drapeau montait `PUT`, `GET` et `HEAD` sur `/<nom>/{id}/content` et laissait `tests.rs`
  sans un seul `/content` : une régression dans l'un des trois handlers échappait au
  `cargo test -- --include-ignored` du projet. Le fichier engendré porte désormais le
  cycle — un corps binaire déposé, relu octet pour octet en `application/octet-stream`,
  `HEAD` avant et après, remplacé par un second `PUT` —, le 404 d'un identifiant inconnu
  sur les trois verbes, le 413 un octet au-delà de `TAILLE_MAX`, et sous `auth` le 401
  d'une requête sans jeton.

- **Le backend `fs` de `storage` n'écrit plus un objet en place.** `put` faisait un
  `fs::write` sur le chemin final, qui le tronque avant de le remplir : un `GET` concurrent
  recevait un corps vide ou tronqué, et un crash en pleine écriture laissait le fichier
  tronqué sous le nom final. Les octets vont désormais dans un fichier temporaire à côté
  de la cible, un UUID par dépôt, synchronisé sur le disque puis `rename` sur elle. La
  racine est créée à la construction du stockage — une racine qui ne se crée pas échoue au
  démarrage, en nommant le chemin — et la sonde de `/health` vérifie seulement qu'elle est
  toujours un répertoire, au lieu d'un `create_dir_all` qui recréait en silence une racine
  disparue et gardait la sonde verte sur un magasin vide. Un projet qui porte déjà
  `storage` reçoit la règle en recopiant `files.rs` depuis le fragment et en ajoutant `?` à
  `FileStorage::new` dans `mod.rs`.

- **`rbs generate crud --with-upload` refuse un projet dont le fragment `storage` vit
  encore en `src/storage/`** — reçu avant la 1.3.0 et jamais déplacé sous
  `src/modules/`. Le service engendré importe `crate::modules::storage` : la génération
  aboutissait et le projet ne compilait plus. Le refus nomme le chemin attendu et le
  déplacement à faire ; rien n'est écrit.
- **Une expression cron modifiée prend effet au démarrage suivant, et non après la
  prochaine occurrence de l'ancienne.** `reconcilier` gardait le `next_run_at` stocké de
  toute échéance connue : passer de `0 3 1 * *` à `*/5 * * * *` laissait le prochain tick
  au premier du mois, sans un mot. Une occurrence stockée déjà échue reste au ticker, une
  occurrence égale à celle fraîchement calculée ne bouge pas, une occurrence qui diverge
  est remplacée — avec un log `info` qui nomme le kind, l'ancienne et la nouvelle
  échéance. Ni colonne ni migration : un projet qui porte déjà `scheduler` reçoit la
  règle en recopiant `sync.rs` depuis le fragment.
- **Le client TypeScript type une structure imbriquée optionnelle en `null | T`, et non
  `unknown | T`.** utoipa rend `Option<Struct>` en `oneOf: [{type: "null"}, {$ref}]`, et
  la variante `null` tombait dans le repli `unknown`, qui avalait toute l'union.
- **`modules`, `seeds`, `bin` et `lib` sont refusés comme noms de feature**, comme `main`,
  `router`, `openapi`, `state` et `health` avant eux. `rbs generate crud modules`
  réussissait, faisait de `src/modules/mod.rs` un CRUD, et cassait tout `rbs add`
  suivant sur une ancre `<rbs:modules>` introuvable.
- **`rbs doctor` et `rbs migrate status` n'affichent plus les lignes `Compiling` de cargo
  avant leur verdict**, `doctor --json` compris. Quand le CLI capture la sortie standard
  de cargo pour la lire, il capture aussi sa sortie d'erreur et ne la rejoue qu'en cas
  d'échec de compilation. `migrate up`, `seed` et `dev` montrent toujours la progression.
- **`rbs add webhooks` sur un projet engendré avant cette version ne le laisse plus
  incompilable.** Un tel projet porte `// <rbs:state_init>` sous
  `core: CoreState::new(db, config)`, qui a déjà consommé `config` quand
  `Sender::from_config(&config)` le lit. La commande refuse désormais à la planification,
  n'écrit rien, nomme la ligne et affiche le bloc à remonter au-dessus. Un fragment
  déclare la ligne que son insertion doit précéder par `before` sur son entrée
  `[[anchors]]` ; `webhooks` est le seul à le faire.
- **Un jeton de rafraîchissement fermé par une déconnexion, rejoué, ne ferme plus tout le
  compte.** `refresh_tokens` gagne `replaced_at` : la rotation le pose, fermer (`logout`,
  `DELETE /auth/sessions`, réinitialisation ou changement de mot de passe) pose
  `revoked_at`, et seul un jeton *remplacé* présenté à nouveau déclenche la révocation de
  la famille — une fois : la ligne rejouée est fermée à son tour, et la présenter encore
  vaut 401 et rien de plus. Un attaquant chassé par une réinitialisation pouvait sinon
  déconnecter la victime à volonté pendant trente jours en rejouant un jeton mort.
- **Un jeton d'accès ne survit plus à la révocation de ses sessions.** `HasAuth`, dans
  `rbs-core`, gagne une méthode fournie `accept(&claims)` qu'`Identity` appelle après la
  vérification de signature ; le fragment `auth` l'implémente en relisant le compte :
  disparu, sessions fermées après `iat` (`users.sessions_revoked_at`, estampillée par la
  réinitialisation, le changement de mot de passe et `DELETE /auth/sessions`), ou rôle
  changé → 401. Les jetons sont émis après la seconde de la révocation, si bien que la
  paire rendue par `change-password` sert aussitôt. Les tests engendrés qui signaient un
  jeton pour un `sub` tiré au hasard créent désormais un compte.
- **Les adresses sont débarrassées de leurs blancs et passées en minuscules** avant
  `register`, `login`, `forgot-password` et `resend-verification`. Deux inscriptions ne
  différant que par la casse faisaient deux comptes — et le lien de vérification de l'un
  arrivait dans la boîte de l'autre.
- **`change-password`, `reset-password`, `refresh`, `verify-email` et
  `DELETE /auth/sessions` écrivent tout ou rien.** Chacun enchaînait ses écritures sur des
  connexions distinctes du pool : un échec entre la consommation d'un jeton de
  réinitialisation et la pose du mot de passe brûlait le jeton pour rien ; un échec entre
  le nouveau mot de passe et la révocation laissait ouvertes les sessions d'un compte
  peut-être compromis ; un échec entre la rotation d'un jeton de rafraîchissement et
  l'émission de la paire laissait le client avec un jeton mort et rien pour le remplacer
  — et son essai suivant comptait pour un rejeu. Chaque dépôt d'`auth` prend désormais
  `&impl ConnectionTrait`, comme `jobs::enqueue`, et les cinq services ouvrent chacun une
  transaction, committée après la dernière écriture.
- **Le guide `AGENTS.md` ne se trompe plus de compte, et dit comment fermer une route à la
  main.** Il annonçait six fichiers pour `rbs generate feature` et sept pour
  `rbs generate crud`, un de moins chacun depuis `filter.rs` ; il proposait
  `rbs generate feature webhooks`, un nom qu'installe désormais `rbs add` ; il désignait
  `clippy` comme la ligne de sa liste qui compte au lieu de `cargo test -- --ignored` ; et
  sur un projet qui porte `auth` il taisait l'argument `Identity`, `require_role`,
  `security(("bearer" = []))` et les réponses 401 et 403 qu'exige une route écrite à la
  main. `rbs upgrade` réécrit la zone du guide d'un projet existant.
- **`rbs add webhooks` ne laisse plus un projet que `cargo fmt --check` refuse.** Il écrit
  trois migrations sous un seul horodatage, et leurs lignes `mod` se déclaraient dans
  l'ordre d'installation, que rustfmt réécrit : la CI qu'engendre `rbs add ci` échouait
  dès son premier push. L'ancre `migration_modules` garde désormais son bloc trié ;
  l'ordre d'exécution vit toujours dans le `vec!` du `Migrator`, que rien ne réordonne. Un
  projet déjà touché lance `cargo fmt` une fois.

#### Projets déjà générés

La migration `create_auth_tables` ne change que pour un `rbs add auth` neuf ; elle
n'altère toujours rien. Un projet déjà migré joue lui-même les ordres — `timestamptz` se
lit `timestamp` sur MySQL et `timestamp_with_timezone_text` sur SQLite, ce que
`rbs migrate` écrit pour ce type de colonne — puis reprend du fragment les fichiers nommés :

- `ALTER TABLE refresh_tokens ADD COLUMN replaced_at timestamptz NULL;`, puis `model.rs`
  (le champ `replaced_at`), le fichier entier `repository/refresh_token.rs` (`rotate`,
  `close`, et le filtre `replaced_at IS NULL` de `open_sessions_of`, `revoke_sessions_of`
  et `revoke_session` — sans lui, `GET /auth/sessions` gagne une ligne à chaque
  rafraîchissement) et `service/session.rs` (`refresh` et `logout`).
- `ALTER TABLE users ADD COLUMN sessions_revoked_at timestamptz NULL;`, puis
  `impl HasAuth for AppState` du `mod.rs` du fragment, `stamp_sessions_revoked` de
  `repository/user.rs` et `close_every_session` de `service/mod.rs`. Sans cela,
  `rbs-core` 1.5.0 compile tel quel et rien ne change.
- `UPDATE users SET email = lower(trim(email));` — la clé unique le refuse si deux comptes
  ne diffèrent que par la casse, et c'est le cas à trancher à la main — puis `normalise`
  de `service/mod.rs` et ses quatre appels (`register`, `login`,
  `password::request_reset`, `verification::request`).
- Aucun ordre : recopier en entier les répertoires `repository/` et `service/` du fragment
  (et les deux tests neufs de `tests/password.rs` et `tests/session.rs`) pour que les cinq
  parcours écrivent tout ou rien. Un appelant qui passait la connexion à un dépôt compile
  tel quel.
- `scheduler` : recopier `sync.rs` depuis le fragment (et les deux tests neufs de
  `tests.rs`) pour qu'une expression cron modifiée prenne effet au démarrage suivant.
  Rien d'autre ne change ; la table garde sa forme.
- Avant `rbs add webhooks` : remonter le bloc `// <rbs:state_init>` … `// </rbs:state_init>`
  au-dessus de `core: CoreState::new(db, config),` dans `src/state.rs`. La commande refuse
  et affiche ce bloc tant qu'il reste sous la ligne ; `rbs doctor --fix` ne fait que
  restaurer une ancre absente, il n'en déplace jamais une déjà présente.

- **Une adresse vérifiée ne se revérifie plus.** `resend-verification` émettait un jeton
  neuf pour un compte déjà vérifié, et chaque jeton consommé réécrivait
  `email_verified_at`, rajeunissant l'adresse au-delà de sa première preuve. Un compte
  vérifié ne reçoit plus rien, et `mark_verified` n'écrit qu'une date encore nulle.

- **`one_time_tokens` se purge.** La table croissait d'une ligne par demande sans jamais en
  perdre : chaque émission supprime désormais les jetons échus de tous les comptes, et la
  migration ajoute `idx_one_time_tokens_expires_at` pour que cette purge ne parcoure pas la
  table. Un projet déjà migré crée l'index à la main, comme le montre la note de montée.

- **`rbs dev` sur un projet MySQL nomme MySQL** quand l'URL de la base est illisible, et
  son remède donne une URL `mysql://` au lieu d'une URL PostgreSQL.

- **`contains` cherche la valeur à la lettre dans un filtre engendré.** `%` et `_`
  partaient dans le `LIKE` comme jokers : `{"title": {"contains": "%"}}` rendait toutes
  les lignes, alors que le commentaire au-dessus affirmait la valeur échappée. `%`, `_`
  et `!` sont désormais échappés, avec un `ESCAPE '!'` explicite qui se lit de même sur
  les trois moteurs. Une feature engendrée avant garde son `filter.rs` ; la note de montée
  donne la fonction à coller.

- **Le compteur mémoire du rate-limit ne parcourt plus sa table à chaque requête sous
  charge.** Dès que la table atteignait 10 000 clés, le balayage tournait à chaque coup
  et ne retirait que les fenêtres échues : avec 10 000 clients actifs à la fois, chaque
  requête parcourait toute la table sous le verrou. Le balayage suivant attend désormais
  que la table ait doublé.

- **La révocation d'un abonnement webhook est un seul `UPDATE` conditionnel.** `revoke`
  lisait la ligne, testait `revoked_at`, puis la réécrivait : deux révocations
  concurrentes pouvaient écrire chacune sa date, la seconde écrasant la première. Elle
  passe désormais par `UPDATE … WHERE revoked_at IS NULL`, comme `auth` pour ses
  sessions — la première gagne, la seconde ne touche aucune ligne. `emit` cesse aussi de
  cloner la liste des motifs de chaque abonnement.

## [1.4.0] — 2026-09-11

### Ajouté

- **Le fragment `auth` passe de cinq routes à treize.** `/auth/change-password`
  (authentifié) rend 200 et une paire de jetons neuve plutôt que 204 — changer le mot de
  passe révoque toutes les sessions, celle de l'appelant comprise, et la réémettre évite
  de déconnecter quelqu'un qui vient de faire la bonne chose ; un mot de passe courant
  faux rend 403, pas 401, l'appelant étant déjà identifié — un 401 le pousserait vers un
  rafraîchissement inutile. `/auth/forgot-password` et `/auth/resend-verification`
  (anonymes) rendent toutes deux 202 qu'un compte porte l'adresse ou non, et l'envoi du
  courriel part détaché de la réponse — distinguer les deux cas, ou attendre le SMTP,
  énumérerait les comptes. `/auth/reset-password` et `/auth/verify-email` (anonymes)
  consomment un jeton à usage unique et rendent la même 401 pour chacune de ses façons
  d'échouer — inconnu, périmé, déjà consommé, ou émis pour l'autre usage.
  `/auth/sessions` (GET, DELETE) liste ou ferme toutes les sessions ouvertes de
  l'appelant, sans jamais exposer l'empreinte d'un jeton dans la vue, et
  `/auth/sessions/{id}` (DELETE) rend 404 plutôt que 403 quand l'identifiant ne désigne
  aucune session de l'appelant — un 403 confirmerait qu'elle existe ailleurs.
- **`register` envoie désormais un courriel de vérification.** Son contrat ne change pas —
  toujours 201, toujours un `UserResponse` — avec un champ de plus, `email_verified_at`,
  nul à l'inscription. L'échec d'un envoi est journalisé, jamais remonté à l'appelant.
- **Une garde `VerifiedIdentity`**, livrée dans `src/auth/guard.rs`, que le projet pose
  lui-même sur les routes qu'il juge sensibles. `login` continue d'accepter un compte non
  vérifié sans changement ; la garde relit l'état de vérification en base à chaque
  requête plutôt que dans le jeton, ce qui le figerait pour la durée du jeton.
- **Une table `one_time_tokens`**, partagée entre réinitialisation de mot de passe et
  vérification d'adresse et distinguée par une colonne `purpose`, plus une colonne
  `email_verified_at` sur `users` — toutes deux ajoutées par la migration
  `create_auth_tables`, qui ne fait que créer des tables et n'en altère aucune.
- **Trois clés sous `[auth]`** : `reset_ttl_secs` (3600), `verification_ttl_secs` (86400)
  et `app_url` (`http://localhost:3000`), lues par une `FlowConfig` que le projet porte
  lui-même, et non `rbs-core`.
- **Deux limites de débit de plus**, sur `/auth/forgot-password` et
  `/auth/resend-verification`, à 3 requêtes par heure, et une sur `/auth/register`, à
  10 — ces trois routes envoient un courriel à une adresse que l'appelant choisit.

### Modifié

- **`auth` exige désormais `mail` en plus de `rate-limit`.** `rbs add auth` sur un projet
  qui ne porte pas encore `mail` l'installe avec lui : `lettre` dans `Cargo.toml`, la
  section `[mail]`, un service `mailpit` dans `docker-compose.yml`, `templates/mail/` et
  `RBS_MAIL__SMTP_PASSWORD` dans `.env.example`.
- **`src/auth/` devient un répertoire par couche.** `repository/`, `service/`,
  `controller/` et `tests/` portent chacun plusieurs fichiers ; le fragment dépose
  désormais 21 fichiers sous `src/auth/` au lieu de 8. Un projet déjà engendré n'est pas
  touché — `rbs` ne réécrit aucun fichier qu'il a déjà écrit, et `rbs upgrade` ne retouche
  jamais le code d'une feature installée — mais un `rbs add auth` neuf rend une
  arborescence différente.
- `rbs-core` n'a pas changé dans cette version : tout le changement vit dans le fragment
  `auth` du CLI et ses gabarits.

## [1.3.1] — 2026-09-09

### Modifié

- **Le filtre engendré se décrit désormais par le type de ses colonnes.** `filter.rs` cite
  le schéma du type de sa colonne — `rbs_core::BoolComparisonSchema` pour un `bool`,
  `TextMatchSchema` pour un texte — là où chaque colonne citait le seul
  `ComparisonSchema`, qui portait une valeur libre. Le document OpenAPI décrit maintenant
  les deux formes qu'une condition a toujours acceptées : un `oneOf` entre la valeur nue,
  typée par la colonne, et l'objet qui nomme ses opérateurs. Et plus aucune colonne n'y est
  exigée — le document les réclamait toutes, si bien qu'un client qui le validait devait
  envoyer le filtre entier pour restreindre une liste sur un seul champ. Swagger propose
  désormais `{ "published": true }` au lieu d'un objet de `"string"` sur chaque colonne.
  Rien ne change à l'exécution : un corps accepté hier l'est aujourd'hui. Un projet déjà
  engendré compile sans retouche — `ComparisonSchema` reste exporté — et gagne le nouveau
  document en régénérant son filtre.

### Ajouté

- **Six schémas dans `rbs-core`**, un par type de colonne que `--fields` peut nommer :
  `BoolComparisonSchema`, `IntComparisonSchema`, `FloatComparisonSchema`,
  `UuidComparisonSchema`, `DateTimeComparisonSchema` et `TextMatchSchema`, chacun nommant
  ses opérateurs par un type compagnon. Ils n'existent que pour être cités par
  `#[schema(value_type = ...)]` ; rien ne les construit.

## [1.3.0] — 2026-09-08

### Modifié

- **Sur un projet portant `auth`, `rbs generate crud` engendre désormais des routes
  fermées** — toutes, et non les seules écritures. Les six routes du CRUD, neuf lorsque
  `--with-upload` ajoute la route de contenu, prennent chacune un paramètre
  `identite: Identity`, ouvrent leur corps par `identite.require_role(Role::User)?`,
  portent `security(("bearer" = []))` et déclarent une 401 et une 403 dans leur
  `#[utoipa::path]` ; le contrôleur engendré s'ouvre sur un bandeau de quatre lignes qui le
  dit. `--role admin` ne décide plus *si* les routes sont gardées, mais jusqu'où : il relève
  le seuil de `create`, `update`, `delete` et du `PUT` de la route de contenu au rôle nommé,
  les lectures — `list`, `filter`, `find`, `GET` et `HEAD` — gardant le `Role::User` par
  défaut. Ouvrir une route au public devient l'édition, et non le défaut : sur le handler
  concerné, retirez le paramètre `identite`, l'appel à `require_role`, l'entrée `security`
  et les réponses 401 et 403 de son annotation. Le `tests.rs` engendré suit — il signe le
  jeton qu'il présente par `rbs_core::jwt::sign`, sans avoir besoin d'un compte, exerce avec
  lui le cycle d'écriture complet, et ajoute deux tests qui ne présentent rien du tout, une
  écriture et une lecture, pour tenir la 401 que l'une et l'autre reçoivent. Un projet
  **sans** `auth` engendre exactement ce qu'il engendrait, octet pour octet ; tout ce qui
  compare la sortie de `rbs generate crud` à une référence enregistrée passe au rouge sur un
  projet portant `auth`, et là seulement. Deux conséquences pour un projet existant, puisque
  `rbs` ne réécrit aucun fichier qu'il a déjà écrit : un CRUD engendré avant cette version
  reste grand ouvert, y compris sur un projet qui installe `auth` ensuite, et `rbs add auth`
  nomme donc en fin de sortie, une fois la feature installée, les features restées
  publiques, pour qu'on sache lesquelles fermer à la main.
- **`require_role` compare un seuil plutôt qu'une égalité.** Elle laisse passer dès que le
  rôle porté est supérieur ou égal au rôle exigé (`porte >= minimum`), si bien qu'un `Admin`
  satisfait un `require_role(Role::User)` ; sans quoi le `Role::User` qu'un CRUD engendré
  nomme sur ses lectures en fermerait la porte aux administrateurs du projet. L'enum `Role`
  dérive en conséquence `PartialOrd, Ord`, et **l'ordre dans lequel elle déclare ses
  variantes porte désormais une hiérarchie** : un rôle inséré entre deux autres déplace d'un
  coup le seuil de toutes les gardes du projet, si bien qu'un rôle plus étendu s'ajoute en
  fin d'énumération. Seul un projet engendré à partir de cette version reçoit cette garde ;
  un projet plus ancien conserve l'égalité stricte qu'il a reçue, car `rbs` ne réécrit pas
  `src/auth/guard.rs` une fois qu'il l'a posé, et les deux sémantiques coexistent donc selon
  la date du projet. La note de version 1.3.0, qu'affiche `rbs upgrade`, porte les lignes
  exactes à remplacer dans `src/auth/guard.rs` et dans le `derive` de `src/auth/model.rs`
  pour y faire passer un projet existant.
- **`rbs add` range désormais dix de ses modules sous `src/modules/`, au lieu de la
  racine de `src/`** — `audit`, `cache` (le répertoire qu'écrit `redis`), `cors`, `jobs`,
  `mail`, `observability`, `rate_limit` (ce qu'écrit `rate-limit`), `scheduler`, `storage`
  et `webhooks`. Le point de montage est `src/modules/mod.rs`, ouvert au premier fragment
  qui en a besoin, puis inséré à l'ancre `<rbs:modules>` par la suite — la quatorzième,
  aux côtés des treize que le CLI connaissait déjà. `auth` est le seul fragment laissé de
  côté : il pose l'entité `User` que vous étendez comme n'importe laquelle de vos propres
  features, et reste donc là où vit votre propre code. Un projet engendré avant cette
  version garde ses modules exactement là où il les a reçus — les déplacer reviendrait à
  réécrire des `use` que le CLI ne possède pas — mais `rbs doctor` gagne un contrôle
  `disposition` qui avertit, sans faire échouer, le jour où un tel projet reçoit un module
  rangé de la nouvelle façon aux côtés de modules restés à la racine. Cette garantie tient
  pour un fragment installé seul, mais pas pour trois d'entre eux, qui visent un autre
  module par son nouveau chemin : `webhooks` vise l'ancre `<rbs:jobs>` dans
  `src/modules/jobs/mod.rs`, et sur un projet portant encore `src/jobs/` d'une version
  antérieure, l'installation échoue tout simplement — `src/modules/jobs/mod.rs est
  introuvable`, sans bloc à coller, mais sans rien écrire non plus. `scheduler` dépose un
  `use crate::modules::jobs::{self, Job};`, et `rate-limit` un appel à
  `crate::modules::cache::Config::load()?` quand le projet porte aussi `redis` ; les deux
  s'installent avec succès et laissent du code qui ne compile pas — `rbs doctor` le
  signale après coup, mais l'installation aura déjà déclaré sa réussite. Sur un projet
  antérieur à la 1.3.0, déplacez `src/jobs/` (et, avant d'ajouter `rate-limit`,
  `src/cache/`) sous `src/modules/` et corrigez leurs `use` avant d'installer `webhooks`,
  `scheduler` ou `rate-limit`.

## [1.2.0] — 2026-09-04

### Ajouté

- `rbs new --preset api|worker|full` installe un jeu de features nommé : `api` vaut `auth`,
  `cors`, `docker` et `rate-limit` ; `worker` vaut `docker`, `jobs`, `redis` et
  `scheduler` ; `full` vaut tout ce que le binaire sait installer, dérivé de ce qu'il porte
  plutôt qu'écrit — une liste figée périmerait au premier fragment ajouté. Il s'ajoute à
  `--with` plutôt que de le remplacer, sans répéter ce que les deux nomment, et dans l'ordre
  des features plutôt que dans celui de la frappe : deux invocations équivalentes rendent
  deux projets identiques.
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

[Non publié]: https://github.com/tky0065/rbs/compare/v1.7.0...HEAD
[1.7.0]: https://github.com/tky0065/rbs/compare/v1.6.0...v1.7.0
[1.6.0]: https://github.com/tky0065/rbs/releases/tag/v1.6.0
[1.5.0]: https://github.com/tky0065/rbs/releases/tag/v1.5.0
[1.4.0]: https://github.com/tky0065/rbs/releases/tag/v1.4.0
[1.3.1]: https://github.com/tky0065/rbs/releases/tag/v1.3.1
[1.3.0]: https://github.com/tky0065/rbs/releases/tag/v1.3.0
[1.2.0]: https://github.com/tky0065/rbs/releases/tag/v1.2.0
[1.1.0]: https://github.com/tky0065/rbs/releases/tag/v1.1.0
[1.0.1]: https://github.com/tky0065/rbs/releases/tag/v1.0.1
[1.0.0]: https://github.com/tky0065/rbs/releases/tag/v1.0.0
[0.4.0]: https://github.com/tky0065/rbs/releases/tag/v0.4.0
