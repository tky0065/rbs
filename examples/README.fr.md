# Exemples

Ces projets sont la source de chaque bloc de code de la documentation. Docusaurus n'exécute
pas le code qu'il affiche ; la compensation est que rien n'y est écrit à la main : le site
lit ces fichiers, et la CI les compile.

*[English version](README.md).*

| Projet | Ce qu'il montre |
|---|---|
| `hello-crud` | Un projet créé par `rbs new`, avec une feature CRUD engendrée par `rbs generate crud`. |
| `blog-auth` | Le même, plus `rbs add auth` : des billets que tout compte identifié peut lire, et que seul un administrateur peut écrire. |
| `file-drop` | Les trois features de la v0.3 sur un même projet — `redis`, `mail`, `storage` — câblées dans un CRUD `uploads`. |
| `newsletter-queue` | `jobs`, `mail` et `observability` : une route de diffusion qui enfile une lettre par abonné confirmé, dans la transaction qui les lit — et un listener `/metrics` à lui. |
| `event-hub` | `webhooks`, `scheduler`, `audit`, `cors`, `docker` et `ci` : la création d'une commande écrit sa trace d'audit et émet `order.created` dans la transaction qui l'insère. |

Ils ne sont pas membres du workspace racine — un projet engendré déclare son propre
`[workspace]`, et Cargo interdit l'imbrication. Le manifeste racine les exclut et la CI
compile chacun d'eux dans une étape dédiée.

## Régénérer

### `hello-crud`

Deux commandes, depuis la racine du dépôt :

```bash
cargo run -p rbs-cli --bin rbs -- new hello-crud --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/hello_crud' \
  --lang fr
mv hello-crud examples/hello-crud
cd examples/hello-crud && cargo run --manifest-path ../../Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud articles --fields 'title:string,body:text,published:bool' --force
```

### `blog-auth`

`add` refuse d'écrire dans un arbre de travail sale, et `rbs new` initialise le dépôt sans
commiter — d'où le commit au milieu :

```bash
cargo run -p rbs-cli --bin rbs -- new blog-auth --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/blog_auth' \
  --lang fr
cd blog-auth && git add -A && git commit -q -m 'projet neuf'
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- add auth
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud posts --fields 'title:string,body:text,published:bool' --role admin --force
cd .. && mv blog-auth examples/blog-auth
```

C'est `--role admin` qui fait montrer à cet exemple deux régimes d'un coup. Sur un projet
portant `auth`, `generate crud` ferme au seuil le plus bas toutes les routes qu'il écrit ;
le drapeau relève les trois écritures, et elles seules. Le retirer laisserait l'exemple
protégé, mais protégé pareillement en lecture et en écriture — et sa promesse d'une ligne,
seul un administrateur écrit, cesserait d'être vraie.

`posts` plutôt qu'`articles`, que `hello-crud` porte déjà : ce qui distingue cet exemple est
la protection, pas la ressource. Le nom n'a aucune incidence sur l'ordre de l'ancre
`features` — `insert()` retrie tout le bloc à chaque insertion, quel que soit l'ordre
d'arrivée de `mod auth;` et `mod posts;`.

### `file-drop`

`add` refuse un arbre de travail sale, et chaque feature en laisse un derrière elle : le
commit est pris avant **chacune** d'elles, et non une fois pour les trois.

```bash
cargo run -p rbs-cli --bin rbs -- new file-drop --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/file_drop' \
  --lang fr
cd file-drop
for f in redis mail storage; do
  git add -A && git commit -q -m "before $f"
  cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- add "$f"
done
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud uploads \
  --fields 'title:string,owner_email:string,content_type:string,size:int' \
  --with-upload --force
cd .. && mv file-drop examples/file-drop
```

Les trois fragments se rangent dans l'ancre `modules`, distincte de `features` et triée
indépendamment d'elle — notez que `rbs add redis` y écrit `mod cache;`, et non
`mod redis;`. `owner_email` se termine par `_email`, donc le DTO engendré gagne sa
contrainte d'adresse sans que personne ne l'écrive, et le courriel a un destinataire qui
vient du modèle.

### `newsletter-queue`

Les trois fragments se rangent dans l'ancre `modules` : `jobs`, `mail` et `observability`
s'y trient alphabétiquement, quel que soit l'ordre des `add` ci-dessous. `email` seul
mérite la contrainte de validation du DTO ; la règle reconnaît le nom exact autant que le
suffixe `_email`.

```bash
cargo run -p rbs-cli --bin rbs -- new newsletter-queue --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/newsletter_queue' \
  --lang fr
cd newsletter-queue
for f in jobs mail observability; do
  git add -A && git commit -q -m "before $f"
  cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- add "$f"
done
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud subscribers --fields 'email:string:unique,name:string,confirmed:bool' --force
cd .. && mv newsletter-queue examples/newsletter-queue
```

### `event-hub`

`add` refuse un arbre de travail sale, et chaque feature en laisse un derrière elle : le
commit est pris avant **chacune** des six, et non une fois pour toutes.

```bash
cargo run -p rbs-cli --bin rbs -- new event-hub --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/event_hub' \
  --lang fr
cd event-hub
for f in webhooks scheduler audit cors docker ci; do
  git add -A && git commit -q -m "before $f"
  cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- add "$f"
done
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud orders --fields 'reference:string,amount:int' --force
cd .. && mv event-hub examples/event-hub
```

`add webhooks` tire `jobs` et `auth`, et par elle `mail` et `rate-limit` : les cinq
descendent d'un seul plan, et ses trois migrations — `create_auth_tables`,
`create_jobs`, `create_webhook_subscriptions` — naissent dans la même seconde. L'ancre
`migration_modules` trie leurs déclarations `mod` comme le ferait rustfmt ; l'ordre
d'exécution, lui, ne bouge pas à ce tri et reste l'ordre d'installation, dans le `vec!`
du `Migrator`. `.github/workflows/ci.yml` et `Dockerfile` ne sont lus par aucune CI de ce
dépôt — GitHub ne lit les workflows qu'à la racine — et restent là pour être cités par la
documentation et pour ne pas dériver.

## Les retouches que le CLI ne produit pas

Trois s'appliquent aux cinq projets :

- supprimer le `.git` que `rbs new` initialise — un dépôt imbriqué n'a rien à faire ici ;
- réécrire la dépendance `rbs-core` en `{ path = "../../crates/rbs-core" }`, puisque
  `--core-path` est canonicalisé en un chemin absolu qui ne survivrait pas à une autre
  machine ;
- restaurer les marqueurs `// region:` que la documentation cite — `# region:` dans un
  TOML ou un YAML, la syntaxe qu'y lit le plugin du site.

`blog-auth` en porte **une** de plus, celle qui reste une fois que `generate crud` écrit
le garde lui-même. La phrase qui tenait ici — aucune commande ne câble un garde sur une
route que vous avez engendrée — a cessé d'être vraie en 1.3.0 : sur un projet portant
`auth`, chaque route que la commande écrit prend une `Identity` et appelle `require_role`,
et `--role` relève le seuil des écritures. Le contrôleur est donc le fichier engendré tel
quel, et `src/auth/guard.rs` aussi.

- `src/posts/tests.rs` : un test s'ajoute, `a_non_admin_write_returns_403`. Le fichier
  engendré refuse déjà une écriture anonyme et une lecture anonyme — 401 toutes deux, de
  l'extracteur — mais il signe un jeton `admin` pour tout le reste de ce qu'il envoie, et
  ne dit donc rien du seuil lui-même. C'est de présenter un jeton `user` qui sépare les
  deux refus : 403 sur `POST /posts`, 200 sur `GET /posts` avec ce même jeton. Rien
  d'autre n'est écrit à la main — le harnais, le cycle de vie, le filtre et le 404 sont
  tels qu'ils ont été engendrés, seuls les marqueurs `// region:` se posent par-dessus.

Deux entrées qui figuraient ici ont disparu, et c'est leur absence qui compte :

- le garde posé sur le contrôleur, qu'écrit désormais `generate crud` — voir le bloc plus
  haut ;
- le retrait du `#[allow(dead_code)]` sur `RequireRole`, dans `src/auth/guard.rs`. Le
  fragment le porte toujours, délibérément : un projet qui installe `auth` sans engendrer
  le moindre CRUD compile sous `clippy -D warnings`, et le trait y serait du code mort.
  Garder ce retrait coûtait à la comparaison de non-dérive sa surveillance du fichier
  entier — c'est exactement ainsi que la réécriture du garde en seuil est passée sous le
  test sans être vue — en échange d'une ligne dont aucune route ne dépend. L'exemple prend
  désormais le fichier tel qu'il est engendré, et le commentaire posé au-dessus de
  l'attribut le dit : votre premier CRUD engendré appelle la garde, si bien que la ligne
  ne masque plus rien et peut partir.

`file-drop` en porte huit de plus. `--with-upload` sur `generate crud` écrit désormais les
trois handlers de contenu eux-mêmes — `PUT`, `GET` et `HEAD` sur `/uploads/{id}/content`,
et la route qui les monte — si bien que ce qui reste à la main est ce qu'aucun drapeau
n'engendre : les trois fragments qui livrent une brique et aucune route, et une brique que
rien n'appelle ne prouve rien du câblage.

- `src/uploads/service.rs` : le service orchestre les trois briques. Le stockage reçoit le
  contenu sous `uploads/{id}` ; le courriel part dans sa propre tâche, en rendant
  `depot.html` ; le cache tient le `COUNT(*)` que les trois écritures invalident. Le total
  plutôt que la page — `Page` n'est que `Serialize`, et le relire depuis le cache imposerait
  de le rendre désérialisable dans le noyau.
- `src/uploads/controller.rs` : `list`, `create`, `update` et `delete` passent les briques
  au service ; les trois handlers de contenu sont engendrés tels que `--with-upload` les
  produit.
- `src/uploads/repository.rs` : `page` se détache de `list`, pour qu'un appelant qui tient
  déjà le compte ne refasse pas le `COUNT(*)`.
- `src/modules/{cache,storage}/mod.rs` et `src/modules/mail/service.rs` :
  `src/modules/storage/mod.rs` abandonne l'`allow(dead_code)` que le fragment pose sur le
  trait `Storage`, dont ce projet appelle les cinq méthodes. `src/modules/cache/mod.rs` et
  `src/modules/mail/service.rs` en gardent un, ciblé, sur `invalidate` et `send_detached`,
  qu'aucun projet n'est tenu d'appeler.
- `templates/mail/depot.html` : une seconde template, ajoutée à la main — celle que le
  fragment livre dit « votre compte est ouvert », ce qu'aucun dépôt de fichier ne réutilise.

`the_hand_edits_of_file_drop_are_in_place` les atteste toutes. Sans lui, ces huit chemins
resteraient hors de toute surveillance, et le câblage pourrait disparaître en silence.

`newsletter-queue` en porte quatorze de plus, et elles sont tout l'intérêt de l'exemple — les
fragments livrent une file, un expéditeur et un module de métriques, et aucun d'eux une
route ; un job que rien n'enfile ne prouve rien de la file.

- `src/modules/jobs/newsletter.rs` : `SendNewsletter` implémente `Job`. Il attend l'envoi
  plutôt que de le détacher, et c'est toute la différence : une erreur rendue ici est un
  réessai, là où `send_detached` ne laisse qu'une ligne de log. La charge porte l'`id` de
  l'abonné et non son adresse — entre l'enfilage et l'exécution, une adresse peut changer.
- `src/modules/jobs/mod.rs` : `registry()` enregistre `SendNewsletter`, et `demo.rs` part
  avec son enregistrement. `enqueue` perd sa permission `unused_imports` — `broadcast`
  l'appelle — quand `enqueue_at` en garde une pour elle, aucune lettre n'étant ici
  programmée.
- `src/subscribers/service.rs` : `broadcast` ouvre une transaction, lit les abonnés
  confirmés et enfile une lettre pour chacun **à l'intérieur** de celle-ci. Sur `db` plutôt
  que `&transaction`, les lettres survivraient au rollback qui les annule, ce qu'une file en
  table existe précisément pour éviter.
- `src/subscribers/repository.rs` : `confirmed` est générique sur la connexion, les autres
  portes prenant la connexion elle-même — une transaction n'est pas une
  `DatabaseConnection`.
- `src/subscribers/{dto,controller,mod}.rs` et `src/openapi.rs` : l'entrée `Broadcast`, le
  handler qui répond `202` (les lettres sont enfilées, pas envoyées), la route montée avant
  `/subscribers/{id}` pour que `broadcast` ne soit pas lu comme un id, et le chemin déclaré
  à OpenAPI.
- `src/modules/mail/mod.rs` et `src/modules/mail/service.rs` : la permission `dead_code`
  de module tombe avec le premier appel, et `send_detached` en garde une pour elle seule —
  la fonction est conservée, un message dont la perte ne coûte rien n'ayant pas besoin
  d'une ligne en base.
- `prometheus.yml` : aucune commande ne l'écrit, et un `/metrics` que personne ne scrute ne
  prouve rien non plus. Il vise `metrics_port`, et non `server.port` ; le test de non-dérive
  relit le port dans `config/default.toml` plutôt que de faire confiance à un second
  littéral.
- `src/seeds/subscribers.rs` : quatre abonnés, dont un qui n'a jamais confirmé — sans eux,
  le filtre de `confirmed` ne se verrait pas.
- `templates/mail/newsletter.html` : la template que le fragment livre annonce un compte
  ouvert, ce qu'aucune infolettre ne peut réutiliser.

`the_hand_edits_of_newsletter_queue_are_in_place` les atteste toutes.

`event-hub` en porte trois de plus, une par fichier, et ensemble elles forment la
création de commande tracée et transactionnelle que trois guides citent :

- `src/orders/repository.rs` : `create` est générique sur `ConnectionTrait` plutôt que
  sur `DatabaseConnection` — une transaction n'en est pas une.
- `src/orders/service.rs` : `create` ouvre une transaction, insère la commande,
  enregistre sa trace d'audit et émet `order.created` via `webhooks::emit`, puis
  commite — un rollback emporte les trois avec lui.
- `src/orders/controller.rs` : passe `&identite.user_id` au service comme acteur de la
  trace d'audit.

`the_hand_edits_of_event_hub_are_in_place` atteste les trois, dans cet ordre : une trace
ou une émission enregistrée après le commit survivrait au rollback qu'elle doit suivre.

Un fichier est suivi contre son propre `.gitignore`. `rbs new` écrit un `.env`, et le
`.gitignore` qu'il pose à côté l'ignore — correct pour un vrai projet, fatal pour une
fixture. Laissé hors du suivi, `.env` reste sur la machine qui a engendré l'exemple et
manque à tout clone, si bien que la comparaison passe en local et échoue en CI. Il est donc
ajouté de force (`git add -f`). Le `.gitignore`, lui, n'est pas touché : il est identique
octet pour octet à la template, et l'éditer s'enregistrerait comme la dérive même que cette
comparaison traque.

Sur les deux exemples qui portent `auth` — `blog-auth` directement, `event-hub` par
`webhooks` — ce `.env` porte **bien** `RBS_AUTH__SECRET`. Le fragment marque la variable
comme secrète, ce qui lui tire une valeur au hasard à chaque génération ; `.env.example`
garde à la place la valeur de substitution qu'`add auth` y écrit, et recopier la vraie
valeur est la première chose que l'on fait dans un vrai projet. Le test de non-dérive
masque cette valeur plutôt que de la comparer, deux générations n'en tirant jamais la
même.

## Le test de non-dérive

`cargo test -p rbs-cli --test integration_examples` régénère les cinq projets et les
compare aux versions versionnées ici, en ignorant exactement les différences énumérées
ci-dessus. Il échoue quand une template change sans que l'exemple ait suivi — et c'est tout
son intérêt : un exemple périmé fait mentir la documentation, et rien d'autre ne s'en
apercevrait.

La comparaison ignore aussi l'ordre des déclarations `mod` de migration d'`event-hub`,
une fois leur horodatage masqué : cet ordre est fonction de l'horodatage, et deux
générations ne tombent jamais dans la même seconde — une régénération rapide y met
plusieurs commandes séparées, et `create_audit_log` échange sa place avec
`create_schedules` selon la seconde tombée. Ce qui reste comparé est l'ordre
d'exécution, que ce tri ne touche pas : les appels `Box::new` dans le `vec!` du
`Migrator`, dans l'ordre d'installation. Ce qui garde le tri committé lui-même — celui que
cette comparaison ignore — c'est `each_example_passes_cargo_fmt`, qui lance `cargo fmt
--check` sur chaque exemple et échouerait si les lignes `mod` qu'il committe n'étaient
pas ce que produit rustfmt.

Le seul fichier de `blog-auth` retouché à la main est exclu de cette comparaison octet à
octet, qui signalerait sinon la retouche elle-même. Ce qu'il porte est attesté à part, par
`the_hand_edits_of_blog_auth_are_in_place` — sans lui, la liste d'exclusion serait une
porte ouverte sur la dérive même qu'elle existe pour déclarer.

La comparaison a son propre angle mort sur les arguments qu'elle rejoue : retirez `--role
admin` d'`EXEMPLES` et les deux côtés régénèrent des écritures au seuil par défaut, sans un
mot. C'est `blog_auth_carries_the_two_regimes_of_the_guard` qui en répond — il compte trois
`Role::Admin` et trois `Role::User` dans le contrôleur engendré.

## Un piège à connaître

Docusaurus met en cache par fichier Markdown. Changez une source sous `examples/` sans
toucher à la page qui la cite, et un `npm run build` local servira sans broncher l'ancien
extrait — il ignore que la page dépend de ce fichier. Lancez `npm run clear` d'abord quand
vous avez édité un exemple. La CI le fait explicitement plutôt que de compter sur un
checkout neuf.
