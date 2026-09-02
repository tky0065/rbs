# Exemples

Ces projets sont la source de chaque bloc de code de la documentation. Docusaurus n'exécute
pas le code qu'il affiche ; la compensation est que rien n'y est écrit à la main : le site
lit ces fichiers, et la CI les compile.

*[English version](README.md).*

| Projet | Ce qu'il montre |
|---|---|
| `hello-crud` | Un projet créé par `rbs new`, avec une feature CRUD engendrée par `rbs generate crud`. |
| `blog-auth` | Le même, plus `rbs add auth` : des billets que tout le monde peut lire, et que seul un administrateur peut écrire. |
| `file-drop` | Les trois features de la v0.3 sur un même projet — `redis`, `mail`, `storage` — câblées dans un CRUD `uploads`. |
| `newsletter-queue` | `jobs` et `mail` : une route de diffusion qui enfile une lettre par abonné confirmé, dans la transaction qui les lit. |

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
  generate crud posts --fields 'title:string,body:text,published:bool' --force
cd .. && mv blog-auth examples/blog-auth
```

`posts` plutôt qu'`articles`, que `hello-crud` porte déjà : ce qui distingue cet exemple est
la protection, pas la ressource. Le nom laisse aussi l'ancre `features` triée — elle empile
les déclarations `mod` dans l'ordre d'installation, et `mod auth; mod articles;` ferait
tiquer un `cargo fmt` à l'intérieur du projet.

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
  --fields 'title:string,owner_email:string,content_type:string,size:int' --force
cd .. && mv file-drop examples/file-drop
```

`uploads` est le seul nom de ressource qui laisse l'ancre `features` triée derrière
`storage` — et notez que `rbs add redis` écrit `mod cache;`, et non `mod redis;`.
`owner_email` se termine par `_email`, donc le DTO engendré gagne sa contrainte d'adresse
sans que personne ne l'écrive, et le courriel a un destinataire qui vient du modèle.

### `newsletter-queue`

L'ordre d'installation n'est pas libre : l'ancre `features` empile les déclarations `mod`
dans l'ordre d'arrivée des features et doit rester triée, donc `jobs`, puis `mail`, puis une
ressource qui suit les deux — ce qui écarte `newsletter` comme nom de ressource. `email`
seul mérite la contrainte de validation du DTO ; la règle reconnaît le nom exact autant que
le suffixe `_email`.

```bash
cargo run -p rbs-cli --bin rbs -- new newsletter-queue --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/newsletter_queue' \
  --lang fr
cd newsletter-queue
for f in jobs mail; do
  git add -A && git commit -q -m "before $f"
  cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- add "$f"
done
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- \
  generate crud subscribers --fields 'email:string:unique,name:string,confirmed:bool' --force
cd .. && mv newsletter-queue examples/newsletter-queue
```

## Les retouches que le CLI ne produit pas

Trois s'appliquent aux quatre projets :

- supprimer le `.git` que `rbs new` initialise — un dépôt imbriqué n'a rien à faire ici ;
- réécrire la dépendance `rbs-core` en `{ path = "../../crates/rbs-core" }`, puisque
  `--core-path` est canonicalisé en un chemin absolu qui ne survivrait pas à une autre
  machine ;
- restaurer les marqueurs `// region:` que la documentation cite.

`blog-auth` en porte trois de plus, une par fichier, et elles sont tout l'intérêt de
l'exemple — aucune commande ne câble un garde sur une route que vous avez engendrée :

- `src/posts/controller.rs` : `create`, `update` et `delete` prennent un extracteur
  `Identity` et appellent `identite.require_role(Role::Admin)?` ; leur `#[utoipa::path]`
  gagne `security(("bearer" = []))` et les réponses 401 et 403. `list` et `find` sont
  intactes — la lecture reste ouverte.
- `src/auth/guard.rs` : le `#[allow(dead_code)]` posé sur `RequireRole` a disparu. Le
  fragment prescrit de le retirer dès qu'une de vos routes appelle le garde, ce qui est
  exactement le cas ici.
- `src/posts/tests.rs` : le harnais inscrit un compte, le promeut par la base —
  l'inscription rend toujours un `user`, et le rôle ne voyage que dans un jeton frappé
  ensuite — et signe les requêtes qui écrivent. Trois tests s'ajoutent : sans jeton → 401,
  avec un `user` → 403, et la route de filtrage qui répond sans jeton sur une ressource
  dont la création en exige un d'`admin`.

`file-drop` en porte neuf de plus, et elles sont tout l'intérêt de l'exemple — les trois
fragments livrent une brique et aucune route, et une brique que rien n'appelle ne prouve
rien du câblage.

- `src/uploads/service.rs` : le service orchestre les trois briques. Le stockage reçoit le
  contenu sous `uploads/{id}` ; le courriel part dans sa propre tâche, en rendant
  `depot.html` ; le cache tient le `COUNT(*)` que les trois écritures invalident. Le total
  plutôt que la page — `Page` n'est que `Serialize`, et le relire depuis le cache imposerait
  de le rendre désérialisable dans le noyau.
- `src/uploads/controller.rs` : trois handlers sur `/uploads/{id}/content` — `PUT`, `GET` et
  `HEAD` — et les cinq existants passent les briques au service. Le contenu voyage hors du
  DTO : un corps binaire n'a pas sa place dans du JSON.
- `src/uploads/repository.rs` : `page` se détache de `list`, pour qu'un appelant qui tient
  déjà le compte ne refasse pas le `COUNT(*)`.
- `src/uploads/mod.rs` : la route de contenu est montée.
- `src/{cache,storage}/mod.rs` et `src/mail/service.rs` : `src/storage/mod.rs` abandonne
  l'`allow(dead_code)` que le fragment pose sur le trait `Storage`, dont ce projet appelle
  les cinq méthodes. `src/cache/mod.rs` et `src/mail/service.rs` en gardent un, ciblé, sur
  `invalidate` et `send_detached`, qu'aucun projet n'est tenu d'appeler.
- `templates/mail/depot.html` : une seconde template, ajoutée à la main — celle que le
  fragment livre dit « votre compte est ouvert », ce qu'aucun dépôt de fichier ne réutilise.

`the_hand_edits_of_file_drop_are_in_place` les atteste toutes. Sans lui, ces neuf chemins
resteraient hors de toute surveillance, et le câblage pourrait disparaître en silence.

`newsletter-queue` en porte onze de plus, et elles sont tout l'intérêt de l'exemple — le
fragment livre une file et aucune route, et un job que rien n'enfile ne prouve rien de la
file.

- `src/jobs/newsletter.rs` : `SendNewsletter` implémente `Job`. Il attend l'envoi plutôt que
  de le détacher, et c'est toute la différence : une erreur rendue ici est un réessai, là où
  `send_detached` ne laisse qu'une ligne de log. La charge porte l'`id` de l'abonné et non
  son adresse — entre l'enfilage et l'exécution, une adresse peut changer.
- `src/jobs/mod.rs` : `registry()` enregistre `SendNewsletter`, et `demo.rs` part avec son
  enregistrement. `enqueue` perd sa permission `unused_imports` — `broadcast` l'appelle —
  quand `enqueue_at` en garde une pour elle, aucune lettre n'étant ici programmée.
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
- `src/mail/service.rs` : `send_detached` garde un `allow(dead_code)` ciblé — la fonction
  est conservée, un message dont la perte ne coûte rien n'ayant pas besoin d'une ligne en
  base.
- `src/seeds/subscribers.rs` : quatre abonnés, dont un qui n'a jamais confirmé — sans eux,
  le filtre de `confirmed` ne se verrait pas.
- `templates/mail/newsletter.html` : la template que le fragment livre annonce un compte
  ouvert, ce qu'aucune infolettre ne peut réutiliser.

`the_hand_edits_of_newsletter_queue_are_in_place` les atteste toutes.

Un fichier est suivi contre son propre `.gitignore`. `rbs new` écrit un `.env`, et le
`.gitignore` qu'il pose à côté l'ignore — correct pour un vrai projet, fatal pour une
fixture. Laissé hors du suivi, `.env` reste sur la machine qui a engendré l'exemple et
manque à tout clone, si bien que la comparaison passe en local et échoue en CI. Il est donc
ajouté de force (`git add -f`). Le `.gitignore`, lui, n'est pas touché : il est identique
octet pour octet à la template, et l'éditer s'enregistrerait comme la dérive même que cette
comparaison traque.

Ce `.env` ne porte **pas** `RBS_AUTH__SECRET`. `add auth` ne l'écrit que dans
`.env.example`, et l'exemple est gardé exactement tel que le CLI le produit — recopier la
variable est la première chose que l'on fait dans un vrai projet, et rien ici ne démarre le
serveur.

## Le test de non-dérive

`cargo test -p rbs-cli --test integration_examples` régénère les quatre projets et les
compare aux versions versionnées ici, en ignorant exactement les différences énumérées
ci-dessus. Il échoue quand une template change sans que l'exemple ait suivi — et c'est tout
son intérêt : un exemple périmé fait mentir la documentation, et rien d'autre ne s'en
apercevrait.

Les trois fichiers de `blog-auth` retouchés à la main sont exclus de cette comparaison octet
à octet, qui signalerait sinon la retouche elle-même. Ce qu'ils portent est attesté à part,
par `the_hand_edits_of_blog_auth_are_in_place` — sans lui, la liste d'exclusion serait une
porte ouverte sur la dérive même qu'elle existe pour déclarer.

## Un piège à connaître

Docusaurus met en cache par fichier Markdown. Changez une source sous `examples/` sans
toucher à la page qui la cite, et un `npm run build` local servira sans broncher l'ancien
extrait — il ignore que la page dépend de ce fichier. Lancez `npm run clear` d'abord quand
vous avez édité un exemple. La CI le fait explicitement plutôt que de compter sur un
checkout neuf.
