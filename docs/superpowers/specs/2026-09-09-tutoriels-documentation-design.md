# Tutoriels de la documentation · Spécification de design

Date : 2026-09-09
Statut : validé, prêt pour le plan d'implémentation

## 1. Objectif

La documentation compte aujourd'hui une page de mise en route linéaire
(`getting-started.md`) et dix-neuf guides thématiques. Entre les deux, rien : un lecteur
qui a fait répondre son premier CRUD et veut installer un module n'a que le guide, écrit
pour expliquer **pourquoi** la brique est conçue ainsi, pas pour l'accompagner pendant
qu'il tape.

Ce chantier ajoute une section **Tutoriels** : neuf pages pas à pas, un module à la fois,
chacune bâtie sur un cas concret, écrites pour quelqu'un qui découvre rbs.

Il n'écrit **aucune ligne de Rust** dans les crates. Ni `rbs-core`, ni le CLI, ni les
templates ne sont touchés. Le seul code modifié est l'insertion de commentaires-marqueurs
`// region:` dans `examples/`, que le test de non-dérive ignore par construction.

## 2. Décisions arbitrées

Cinq décisions ont été prises en amont. Elles ne sont pas rouvertes par l'implémentation.

| # | Décision | Retenu | Écarté |
|---|---|---|---|
| T1 | Socle de code cité | Les quatre exemples existants, sans en créer | Un cinquième exemple fil rouge · Un exemple par tutoriel · Existants + un nouveau |
| T2 | Découpe | Un tutoriel par module | Un tutoriel par exemple (4) · Une progression cumulative en 4 étapes |
| T3 | Rapport aux guides | Séparation nette, aucun guide modifié | Les tutoriels absorbent le pas-à-pas des guides · Un tutoriel en tête de chaque guide |
| T4 | Point de départ | Un prologue commun, puis les modules | Chaque page repart d'un projet neuf · Amorçage en bloc repliable |
| T5 | Niveau de preuve | La sortie décisive de chaque page, marquée `rbs:transcript` | Toutes les sorties marquées · Aucune, aligné sur les guides |

**Conséquence assumée de T1** : les modules `scheduler`, `webhooks`, `audit`, `cors`,
`rate-limit`, `ci` et `docker` ne sont portés par aucun exemple. Ils n'ont donc pas de
tutoriel : un tutoriel sans code réel à citer violerait la règle du dépôt. Leur guide reste
leur seule page, et c'est la limite explicite de cette section.

**Conséquence assumée de T4** : le lecteur construit dans le prologue un projet à lui,
tandis que le code cité vient de `file-drop` ou `newsletter-queue` — d'autres noms, et des
projets qui portent des modules voisins. Chaque page porte donc en tête une admonition de
provenance, rigoureusement identique d'une page à l'autre, plutôt que de laisser croire
qu'il s'agit du même dossier.

## 3. Sommaire

Une catégorie `tutorials`, en position 3, entre `getting-started` et `architecture`.

| Page | Cas concret | Module | Exemple cité |
|---|---|---|---|
| 0. Préparer le terrain | le projet vide qui sert aux huit suivants | `new` | `hello-crud` |
| 1. Ma première ressource | des articles qu'on crée, liste et modifie | `generate crud` | `hello-crud` |
| 2. Fermer l'API aux inconnus | tout le monde lit, seul un admin écrit | `auth` | `blog-auth` |
| 3. Recevoir un fichier | un justificatif déposé par un client | `storage` | `file-drop` |
| 4. Envoyer un mail | l'accusé de réception du dépôt | `mail` | `file-drop` |
| 5. Ne pas recalculer deux fois | une liste lue mille fois par minute | `redis` | `file-drop` |
| 6. Sortir le travail long de la requête | 5 000 lettres sans faire attendre l'appelant | `jobs` | `newsletter-queue` |
| 7. Voir ce que fait l'API | pourquoi cette route est lente | `observability` | `newsletter-queue` |
| 8. Appeler l'API en TypeScript | un front qui consomme le CRUD | `generate client` | `hello-crud` |

Le prologue s'arrête au projet qui démarre et à la base qui répond ; le CRUD devient le
tutoriel 1, pour qu'un débutant obtienne une réussite visible dès la première page de
contenu.

L'ordre 3 → 4 → 5 suit `file-drop` : le dépôt du fichier, l'accusé qu'il déclenche, puis
le cache que ses écritures invalident. Chaque page s'appuie sur ce que la précédente a
posé, sans jamais l'exiger — les trois modules s'installent indépendamment.

## 4. Anatomie d'une page

Sept blocs, dans cet ordre, sur les neuf pages. La régularité est le service rendu au
débutant : à la deuxième page il sait où regarder.

1. **« Ce qu'on va construire »** — deux ou trois phrases nommant le cas et le résultat
   final. Pas d'abstraction : « à la fin, `PUT /uploads/{id}/content` range le fichier et
   répond `204` ».
2. **L'admonition de provenance** — identique sur les neuf pages : le code montré est lu
   dans `examples/<projet>`, que la CI compile ; chez le lecteur, les mêmes fichiers
   portent le nom de sa ressource.
3. **« Il te faut »** — une ligne, un lien vers le prologue.
4. **Les étapes numérotées** (`## 1.`, `## 2.`…, trois à cinq par page) — chacune est un
   triplet strict : la commande, sa sortie, puis **une phrase disant ce que la sortie
   prouve**. Cette phrase est ce qui fait le niveau débutant ; sans elle, quinze lignes de
   `+ src/modules/…` défilent sans qu'on sache ce qui vient de se passer.
5. **« Vérifier »** — un `curl` et sa réponse. Le tutoriel se termine sur quelque chose qui
   répond, pas sur un fichier écrit.
6. **« Ce qui a été installé »** — deux ou trois extraits `file=`/`region=`, chacun sous un
   titre qui dit le rôle du fichier. Pas d'exhaustivité : le guide est là pour ça.
7. **« Pour aller plus loin »** — le guide du module, la page CLI de la commande, le
   tutoriel suivant.

### Extraits pressentis

Les régions nécessaires existent déjà, à une exception près. Liste indicative, que
l'implémentation confirme page par page :

| Page | Extraits |
|---|---|
| 1 | `hello-crud` : `model.rs` `entite`, `dto.rs` `entree`, `controller.rs` `create`, `repository.rs` `list` |
| 2 | `blog-auth` : `posts/controller.rs` `create`, `auth/guard.rs` `require_role`, `posts/tests.rs` `refus` |
| 3 | `file-drop` : `modules/storage/mod.rs` `trait`, `uploads/controller.rs` `put_content`, `uploads/service.rs` `contenu` |
| 4 | `file-drop` : `modules/mail/service.rs` `send_template` et `send_detached`, `uploads/service.rs` `notify` |
| 5 | `file-drop` : `modules/cache/mod.rs` `lecture` et `invalidate_prefix`, `uploads/service.rs` `list` |
| 6 | `newsletter-queue` : `modules/jobs/newsletter.rs` `job`, `subscribers/service.rs` `broadcast`, `modules/jobs/mod.rs` `registry` |
| 7 | `newsletter-queue` : `config/default.toml` `metriques`, `modules/observability/tests.rs` `cardinalite` — **la seule page susceptible d'exiger une région neuve**, l'exposition du listener n'en portant aucune |
| 8 | `hello-crud` : `clients/ts/client.ts` `classe` et `methodes` |

Le prologue ne cite pas de code : il ne montre que des commandes et leurs sorties.

Toute région ajoutée l'est dans le fichier d'`examples/` concerné. `integration_examples`
ignore les lignes de marqueur (`the_region_markers_are_ignored`), et
`examples/README.md` couvre déjà ces éditions par une ligne générique — rien n'y est à
ajouter.

## 5. Mécanisme de preuve

Une sortie citée qui n'est jamais rejouée se périme sans bruit : c'est la raison d'être
d'`integration_docs.rs`, et quatre blocs de la documentation avaient vécu faux avant lui.

Sur chaque page, le bloc marqué est celui de l'étape décisive : le `rbs new` pour le
prologue, le `generate` pour les pages 1 et 8, le `rbs add` pour les six autres. Son
`setup=` porte l'amorçage complet, de sorte que le prologue reste vrai même si personne ne
l'a lu :

```
{/* rbs:transcript cmd="rbs add jobs" setup="rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo && git add -A && git commit -q -m init" dans="demo" */}
```

Le `git commit` n'est pas décoratif : `add` refuse d'écrire dans un arbre de travail sale,
et `rbs new` initialise le dépôt sans committer.

Neuf transcrits de plus, portant le total de dix à dix-neuf. `normalise()` masque déjà
horodatages, versions, durées et adresses, donc le timestamp que `add` met dans le nom de
sa migration ne rend pas le bloc instable. Si un cas résiste, `extrait="oui"` réduit
l'oracle à la portion stable plutôt que de renoncer au marqueur.

**Ce qui n'est pas gardé, et pourquoi** : les `curl` du bloc 6. Le harnais lance une
commande dans un répertoire temporaire ; il ne démarre pas de serveur. Ces blocs restent
non marqués, comme ceux des guides. C'est une limite connue, pas un oubli.

## 6. Fichiers touchés

**Créés — 20.**

- `docs/docs/tutorials/_category_.json` et neuf pages `.md` ;
- leur miroir sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/`.

**Modifiés — une dizaine, tous légers.**

- `docs/i18n/fr/docusaurus-plugin-content-docs/current.json` : la clé
  `sidebar.docsSidebar.category.Tutorials`, sur le modèle des deux qui y sont déjà ;
- les positions : `tutorials` prend 3, et `architecture` (3→4), `cli` (4→5), `guides`
  (5→6), `compatibility` (6→7) glissent d'un cran — du frontmatter et deux
  `_category_.json`, dans les deux langues ;
- `docs/docs/intro.md` et sa traduction : une entrée « Tutoriels » en tête de « Where to go
  next » ;
- `examples/` : les marqueurs `// region:` que la page 7 réclamerait.

Aucun guide n'est modifié (T3). Aucun fichier de `crates/` n'est modifié.

## 7. Vérification

Cinq commandes, exécutées, avec leur sortie consignée dans le corps du commit.

| Ce qu'on vérifie | Commande |
|---|---|
| Les deux locales construisent, aucun lien mort, tout `file=`/`region=` résolu | `npm run clear && npm run build` (dans `docs/`) |
| La charpente FR/EN coïncide — titres, méta des blocs, encarts, liens relatifs | `node scripts/parite.mjs` |
| Les sorties citées sans base sont encore vraies | `cargo test -p rbs-cli --test integration_docs` |
| Les sorties citées qui exigent un PostgreSQL | `cargo test -p rbs-cli --test integration_docs -- --ignored` |
| Les régions ajoutées n'ont pas fait dériver les exemples | `cargo test -p rbs-cli --test integration_examples` |

Le `build` est le garde-fou réel de la parité : la CI ne lance pas `parite.mjs`, et
`onBrokenLinks: 'throw'` fait tomber le site sur un lien mort. `parite.mjs` compare en
outre le dernier commit de chaque paire de pages — les deux langues partent donc dans le
même commit, comme le `CLAUDE.md` l'exige déjà.

## 8. Hors périmètre

- **Les sept modules sans exemple** (`scheduler`, `webhooks`, `audit`, `cors`,
  `rate-limit`, `ci`, `docker`) : pas de tutoriel, faute de code réel à citer. Les couvrir
  supposerait de rouvrir T1.
- **La réécriture des guides** : T3 les laisse intacts.
- **Un cinquième projet dans `examples/`** : écarté par T1, avec sa charge de CI.
- **Les tutoriels vidéo, les captures d'écran, un bac à sable exécutable** : hors sujet.
- **Le marquage des `curl`** : demanderait un harnais capable de démarrer un serveur, ce
  qui est un chantier à soi seul.

## 9. Risques connus

| Risque | Parade |
|---|---|
| Un transcrit de `rbs add` instable malgré `normalise()` | `extrait="oui"` sur la portion stable |
| La page 7 (`observability`) manque de matière citable | Ajouter une région dans `examples/newsletter-queue` ; à défaut, s'appuyer sur `config/default.toml` et le guide |
| Neuf pages neuves × deux langues : dérive de structure entre FR et EN | `parite.mjs` avant chaque commit, pas seulement à la fin |
| La suite lente s'allonge de neuf transcrits sous Docker | Mesurée à l'implémentation ; si le coût est réel, replier les pages 3-5 sur un `setup` partagé |
