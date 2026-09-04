# `rbs generate client --lang ts`

**Date** : 2026-09-04
**Tâche** : `IMPROVE.md` 79 — *[Feature] `rbs generate client --lang ts`* (Hard)

Un client TypeScript typé, engendré depuis le document OpenAPI que le projet produit
lui-même. Le contrat est déjà écrit — `#[utoipa::path]` sur chaque handler, `ApiDoc` qui
les rassemble — et rien ne le lit hors de Swagger UI. Cette commande le lit.

## Ce que le mainteneur a tranché

Deux décisions sont posées avant ce document et ne se rediscutent pas :

1. **La source du document.** La template du projet gagne un troisième `[[bin]]`, nommé
   `openapi`, qui imprime `ApiDoc::openapi()` en JSON sur la sortie standard.
   `rbs generate client` lance `cargo run --bin openapi` dans le projet et lit sa sortie.
   Pas de `--from` : aucun fichier à tenir à jour à la main, aucun serveur à démarrer.
2. **La forme du client.** Une classe `ApiClient` configurable — `baseUrl`, en-têtes,
   `fetch` injectable —, une méthode par opération OpenAPI, les interfaces des schémas.
   Le jeton se pose une fois, à la construction.

## Le document, tel qu'il est réellement

Relevé le 2026-09-04 sur `examples/blog-auth`, augmenté du binaire ci-dessus
(`cargo run --bin openapi`, 28 816 octets). Ce qui suit n'est pas déduit de la
documentation d'utoipa mais lu dans sa sortie :

- `openapi: "3.1.0"`. Les champs nullables prennent la forme **`"type": ["string", "null"]`**
  et non `nullable: true` — c'est la forme 3.1, et c'est celle qu'il faut traduire.
- `operationId` vaut le **nom nu de la fonction handler** : `list`, `create`, `find`,
  `update`, `delete`, `login`, `health`. Voir « Deux défauts corrigés en chemin ».
- Le `tag` du handler de santé vaut `crate::health::controller` : utoipa retombe sur le
  chemin de module faute de `tag =`. Même section.
- `Page<T>` se sérialise en un schéma nommé **`Page_PostResponse`** dont le `data.items`
  est **inliné** — l'objet est répété, il n'y a pas de `$ref` vers `PostResponse`. Un
  générateur qui ne saurait traiter qu'un `$ref` n'en produirait rien.
- `ProblemDetails.errors` porte `additionalProperties: {type: array, items: {type: string}}`
  **et** `propertyNames`, sous un `type: ["object", "null"]`.
- Les opérations protégées portent `security: [{ "bearer": [] }]` ; les autres n'ont pas
  la clé du tout.
- Une réponse 204 n'a pas de `content`.
- `components.responses` porte six réponses nommées (`BadRequest`, `Unauthorized`…) qu'**aucune
  opération ne référence** : `CommonResponses` les enregistre pour un handler écrit à la
  main. Le générateur les ignore — il ne traduit que ce qui est atteignable depuis une
  opération.

## Décisions de conception

### La commande

```
rbs generate client --lang ts [--out <DIR>] [--force] [--dry-run]
```

- **`--lang` est requis**, `ValueEnum` à une seule variante `ts`. Requis plutôt qu'à
  défaut : le jour où `--lang python` s'ajoute, aucune invocation existante ne change de
  sens. Le type s'appelle `client::Lang` ; il n'a rien à voir avec `lang::Lang`, qui est
  la langue du guide `AGENTS.md`, et son doc-commentaire le dit.
- **`--out`, par défaut `clients/ts`**, relatif à la racine du projet. Pluriel : un second
  langage prendra `clients/python` sans déménager le premier. Hors de `src/`, que Cargo
  compile.
- **`--force` et `--dry-run`** comme les autres sous-commandes de `generate` : `--force`
  lève la garde Git *et* le conflit d'un fichier déjà écrit ; `--dry-run` affiche le plan
  et n'écrit rien.

### Un seul fichier de sortie

`<out>/client.ts`, et rien d'autre. Pas de `types.ts` séparé : un `import` entre deux
fichiers TypeScript exige une extension dont la forme dépend du `moduleResolution` du
consommateur (`./types`, `./types.js`), et il n'existe pas de réponse qui vaille pour
tous. Un fichier unique, sans `import`, se dépose dans n'importe quel projet sans rien
régler. Il est long — trois cents lignes pour `blog-auth` — mais la règle des ~200 lignes
du dépôt vise un fichier de feature écrit à la main, pas un artefact engendré.

Pas de `package.json`, pas de `README.md`, pas de `tsconfig.json` : le fichier n'a aucune
dépendance et le consommateur a déjà les siens.

### Le plan, et la régénération

Le plan de la commande porte l'unique action `create("<out>/client.ts", …)`. Toute la
sémantique de régénération en découle sans une ligne de plus :

| Situation | Statut du plan | Ce que voit le développeur |
|---|---|---|
| Le fichier n'existe pas | `AFaire` | `+ clients/ts/client.ts créé` |
| Le fichier est déjà celui-là | `DejaFait` | régénération idempotente, rien d'écrit |
| Le fichier a été retouché | `Conflit` | `! conflit — relancer avec --force` |

### La forme du client

```ts
export interface ApiClientOptions {
  /** Racine de l'API : `https://api.exemple.fr` ou `/api`. */
  baseUrl: string;
  /** En-têtes de chaque requête. Une fonction pour un jeton qui tourne. */
  headers?: Record<string, string> | (() => Record<string, string> | Promise<Record<string, string>>);
  /** `fetch` à employer. `globalThis.fetch` par défaut. */
  fetch?: typeof globalThis.fetch;
}
```

`headers` accepte une fonction, et pas seulement un `Record` : le fragment `auth` livre un
`refresh`, donc un jeton qui change en cours de session. Un `Record` figé obligerait à
reconstruire le client à chaque rotation.

`baseUrl` est concaténé, jamais passé à `new URL()` : une racine relative (`/api`) est le
cas normal d'une application servie depuis le même domaine, et `new URL("/api")` jette.

### Les erreurs

Une réponse hors 2xx lève `ApiError`, qui porte `status`, `body` (la charge analysée,
quelle qu'elle soit) et `problem`, typé `ProblemDetails` quand le document en déclare un —
ce qui est le cas de tout projet rbs, `CommonResponses` l'enregistrant. Le générateur
n'en dépend pas pour autant : si le schéma manque, il émet lui-même une interface
`ProblemDetails` minimale, et le champ reste typé.

C'est l'endroit où le client tire un vrai profit d'être engendré *pour ce backend-là* :
`erreur.problem?.errors?.email` est typé de bout en bout, du `validator` de Rust au
`catch` de TypeScript.

### Des schémas aux types

| Schéma OpenAPI | TypeScript |
|---|---|
| `string`, quel que soit le `format` | `string` |
| `integer`, `number` | `number` |
| `boolean` | `boolean` |
| `array` d'items `S` | `S[]` |
| `type: [T, "null"]` | `T \| null` |
| `enum` de chaînes | union de littéraux |
| `object` avec `properties` | objet inline, ou l'identifiant si c'est un composant |
| `object` avec `additionalProperties: S` | `Record<string, S>` |
| `object` sans rien | `Record<string, unknown>` |
| `oneOf` / `anyOf` | union |
| `allOf` | intersection |
| `$ref: #/components/schemas/X` | identifiant de `X` |
| rien de tout cela | `unknown` |

Une propriété citée par `required` est obligatoire, les autres portent `?`.

`unknown` plutôt que `any` : un champ que le générateur ne sait pas décrire doit forcer le
consommateur à se prononcer, non passer silencieusement les vérifications.

**Nom d'un composant → identifiant TypeScript** : les caractères hors `[A-Za-z0-9]`
coupent, chaque tronçon est capitalisé, le tout est recollé. `Page_PostResponse` devient
`PagePostResponse`. Deux composants qui se réduiraient au même identifiant sont une erreur
qui les nomme tous les deux — pas un renommage silencieux qui rendrait le client faux.

Un `$ref` que le document ne résout pas est une erreur nommée, jamais un `unknown` muet :
c'est le seul cas où un document malformé produirait un client qui compile et ment.

### Des opérations aux méthodes

Le nom de la méthode est l'`operationId` en `camelCase` (`articles_list` →
`articlesList`). Deux opérations qui rendraient le même nom sont une erreur qui les
nomme toutes les deux, avec le remède : poser un `operation_id` sur le handler.

Signature, dans cet ordre :

1. les paramètres de chemin, positionnels, dans l'ordre où le gabarit les cite ;
2. le corps de requête, s'il y en a un ;
3. les paramètres de query, réunis en un objet — optionnel avec `= {}` si aucun n'est
   requis.

L'objet de query est une interface exportée nommée d'après la méthode
(`articlesList` → `ArticlesListQuery`), pas un type anonyme : un appelant doit pouvoir
nommer ce qu'il construit. Elle entre dans le même contrôle de collision que les schémas.

Le type de retour est l'union des corps distincts des réponses 2xx, ou `void` si aucune
n'a de contenu. Un 204 seul rend donc `Promise<void>`.

Une opération qui porte `security` reçoit dans son doc-commentaire la ligne « requiert un
jeton » : c'est ce que le développeur cherche en lisant la signature, et le typage ne peut
pas le dire.

### La template

L'invariant du client — `ApiError`, `ApiClientOptions`, la méthode privée `request`,
l'analyse de la réponse — vit dans **`crates/rbs-cli/templates/client/ts/client.ts.jinja`**,
lue par `include_str!` comme les templates de `templates/feature/`. Du TypeScript se lit,
se relit et se corrige dans un fichier `.ts.jinja` ; concaténé depuis Rust, il ne se lit
plus. Rust ne calcule que ce qui varie : les interfaces et les méthodes, rendues en texte
puis passées au contexte.

Les délimiteurs alternatifs de minijinja (`{@ @}`) ne rencontrent rien en TypeScript : les
littéraux de gabarit y écrivent `${…}`, jamais `{@`.

### Le cas du projet sans bibliothèque

`crates/rbs-cli/src/anchors.rs:283-320` l'énonce : un projet engendré avant rbs 1.0 n'a pas
de `src/lib.rs`, et ses modules vivent dans le binaire. Un `[[bin]]` séparé ne peut alors
pas atteindre `ApiDoc`. **La commande refuse**, par une erreur qui nomme `src/lib.rs` et
dit pourquoi. Le contourner supposerait de recoller par `#[path]` tout l'arbre de modules
que `src/openapi.rs` traverse — `crate::state::AppState`, chaque contrôleur cité par
`paths(...)` — pour un parc de projets antérieurs à un jalon déjà livré.

### Le cas du projet sans le binaire `openapi`

Même sur un projet à bibliothèque, un projet engendré avant *ce* jalon n'a pas
`src/bin/openapi.rs`. La commande refuse en affichant le fichier à créer et le bloc
`[[bin]]` à coller — **exactement le geste de `rbs seed`** sur un projet antérieur aux
seeds (`crates/rbs-cli/src/seed.rs:98-108`), qui affiche déjà son binaire et sa section de
manifeste.

L'alternative — écrire le binaire soi-même, puis lancer cargo, puis écrire le client —
demanderait deux plans successifs dans une seule commande, ce qu'aucune commande du dépôt
ne fait. `rbs upgrade` n'est pas non plus le bon endroit : son module ouvre en disant
qu'il n'écrit que dans `Cargo.toml` et dans les zones réservées d'`AGENTS.md`.

## Deux défauts corrigés en chemin

Le document OpenAPI cesse, avec ce jalon, d'être un sous-produit de Swagger UI pour
devenir un artefact que le CLI lit. Deux défauts qui y dormaient deviennent visibles.

### `operationId` n'est pas unique

utoipa prend le nom nu de la fonction. Sur un projet à deux features CRUD,
`articles::controller::list` et `comments::controller::list` produisent **deux opérations
d'`operationId` `list`**, ce que la spécification OpenAPI interdit. Aucun exemple ne le
montre — chacun n'a qu'un CRUD — et rien ne le signalait.

Correction : `templates/feature/controller.rs.jinja` pose un
`operation_id = "<module>_<action>"` sur chacun de ses cinq handlers. Le client y gagne
`articlesList()` là où il aurait eu `list()`.

Le générateur ne s'y repose pas pour autant : il détecte la collision et la nomme, un
handler écrit à la main pouvant toujours en produire une.

### Le handler de santé n'a pas de `tag`

`templates/project/src/health/controller.rs.jinja` n'écrit pas de `tag =`, et utoipa
retombe sur le chemin de module : la sortie porte `"tags": ["crate::health::controller"]`.
Une ligne à ajouter, `tag = "health"`.

## Découpage

```
crates/rbs-cli/src/client/mod.rs          la commande : options, erreurs, plan
crates/rbs-cli/src/client/document.rs     le document OpenAPI lu en modèle serde
crates/rbs-cli/src/client/ts.rs           schémas et opérations → TypeScript
crates/rbs-cli/templates/client/ts/client.ts.jinja   l'invariant du client
crates/rbs-cli/templates/project/src/bin/openapi.rs.jinja   le binaire qui imprime
```

`document.rs` ne connaît que JSON, `ts.rs` ne connaît que le modèle de `document.rs` et
ne lance aucun processus, `mod.rs` est le seul à toucher au disque et à cargo. Chacun se
teste seul : `ts.rs` sur un document construit en mémoire, `mod.rs` sur un projet
temporaire.

## `examples/` et la documentation

Les quatre exemples versionnés gagnent `src/bin/openapi.rs` et sa section `[[bin]]`, la
template du projet ayant changé : `integration_examples` le réclame, et c'est lui
l'oracle. Leurs contrôleurs gagnent en même temps l'`operation_id`, et leur
`src/health/controller.rs` son `tag`.

`examples/hello-crud` gagne en plus **`clients/ts/client.ts`**, d'où la documentation lira
ses extraits — le site ne cite aucune ligne écrite à la main.

Ce fichier ne peut pas entrer dans la comparaison de `assert_no_drift` : ce test rejoue
`new`, `add` et `generate crud`, qui ne compilent rien, et y ajouter `generate client`
ajouterait une compilation complète de projet à la suite rapide. Il sort donc de la
comparaison par un champ nouveau et distinct d'`edite_a_la_main` — il n'est pas écrit à la
main, il est engendré par une commande que ce test-là ne rejoue pas — et un test
`#[ignore]` répond de lui en rejouant vraiment `rbs generate client`. C'est la structure
qu'`edite_a_la_main` a déjà : une liste d'exclusions, et un test qui en répond.

## Ce que les tests doivent prouver

Rendu (rapide, `cargo test -p rbs-cli --lib`) :

- chaque ligne du tableau des types, y compris `["string","null"]` et
  `additionalProperties` ;
- `Page_PostResponse` → `PagePostResponse`, avec son objet inline ;
- deux composants qui se réduisent au même identifiant → erreur nommant les deux ;
- deux opérations de même `operationId` → erreur nommant les deux ;
- un `$ref` non résolu → erreur ;
- l'ordre des arguments : chemin, corps, query ;
- `204` seul → `Promise<void>` ;
- l'union des 2xx quand il y en a plusieurs ;
- une opération `security` → la mention du jeton dans le doc-commentaire.

Chaque test de rendu porte son propre document JSON, court et écrit dans le test. Pas de
grande fixture relevée sur un exemple : elle deviendrait un second exemplaire des
templates, à régénérer à chaque retouche, et rien ne le signalerait. Ce que les morceaux
donnent une fois assemblés est prouvé plus loin, sur le client versionné de
`examples/hello-crud`.

Commande (lent, `--ignored`) :

- `rbs new` puis `rbs generate crud` puis `rbs generate client --lang ts` écrit
  `clients/ts/client.ts` ;
- relancée, la commande ne réécrit rien ;
- le fichier retouché puis la commande relancée : conflit, puis `--force` qui l'emporte ;
- `--dry-run` n'écrit rien ;
- un projet sans `src/bin/openapi.rs` : refus, et le remède affiche le bloc `[[bin]]` ;
- `examples/hello-crud/clients/ts/client.ts` est encore ce que le CLI produit.

## Ce que ce jalon ne fait pas

- Un autre langage que TypeScript. `--lang` existe pour que l'ajout soit additif.
- Un client par tag (`client.articles.list()`) : la décision 2 pose une classe plate.
- Les corps `multipart`, qu'aucune template rbs ne produit aujourd'hui.
- Les paramètres d'en-tête et de cookie : aucune opération engendrée n'en déclare, et un
  handler écrit à la main qui en déclarerait verrait le paramètre ignoré. Le générateur
  refuse plutôt que d'ignorer — un paramètre requis silencieusement omis produirait un
  appel qui échoue à l'exécution.
