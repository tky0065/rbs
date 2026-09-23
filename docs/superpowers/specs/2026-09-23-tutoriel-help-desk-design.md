# Tutoriel « gestionnaire de tickets » — conception

**Date** : 2026-09-23
**Portée** : un tutoriel de bout en bout, de `rbs new` à une application de tickets servie
avec son frontend d'administration, adossé à un septième exemple, `examples/help-desk`.
Hors périmètre, explicitement : un job CI pour le frontend de l'exemple, un sélecteur de
référence dans l'écran Vue, un assigné, des règles de transition de statut, des captures
d'écran.

---

## Pourquoi

Neuf tutoriels existent, tous sur le même projet `demo`, et aucun ne touche au frontend : la
seule page qui en parle est le guide `guides/frontend.md`. Aucune page ne montre non plus
comment les pièces s'assemblent — `auth`, les relations, `frontend-admin`, le client typé —
ni où le code engendré se modifie. Ce tutoriel est le « capstone » qui manque : un projet
neuf, un vrai besoin, une retouche à la main justifiée par une faille réelle.

## Décisions

| Question | Décision |
|---|---|
| Forme | Une page unique, `tutorials/help-desk.md`, plutôt qu'une mini-série : « de bout en bout » se lit d'un tenant, et une page reste simple à tenir en parité fr/en. |
| Projet support | Un nouvel `examples/help-desk`, source de tous les extraits de la page. |
| CI | Aucun workflow modifié. La boucle `examples/*/` de `.github/workflows/ci.yml:74` lui applique déjà `clippy -D warnings` ; son frontend n'est pas construit en CI. |
| Point de départ | Un projet neuf, `help-desk`, et non le `demo` des autres pages : `demo` porte `articles` et n'a ni `auth` ni frontend. |
| Code à la main | Une seule retouche, enseignée pas à pas : l'auteur est lu dans le jeton plutôt que reçu dans le corps. |

## Le modèle de données

Sur un projet portant `frontend-admin` — qui tire `frontend`, `auth`, `mail` et
`rate-limit` :

```bash
rbs add frontend-admin
rbs generate crud tickets \
  --fields 'sujet:string,detail:text,statut:enum(ouvert,en_cours,resolu,ferme),priorite:enum(basse,normale,haute),auteur:references:users'
rbs generate crud commentaires \
  --fields 'corps:text,ticket:references:tickets:cascade,auteur:references:users'
```

- `statut` et `priorite` en `enum` : l'écran les rend en liste déroulante, la base les
  garde par un `CHECK`.
- `ticket:…:cascade` : supprimer un ticket emporte ses commentaires.
- `auteur:references:users` sans politique : le `Restrict` par défaut interdit de supprimer
  un utilisateur qui a écrit.
- Colonnes en français, comme `incidents` dans `admin-console` et `--lang fr`. La page
  anglaise tape les mêmes commandes.
- L'ordre des migrations est sûr : `next_timestamp` (`crates/rbs-cli/src/generate/migration.rs:44`)
  date la seconde migration au moins une seconde après la première.

## La retouche : l'auteur pris dans le jeton

**Le défaut montré au lecteur.** Tel qu'engendré, `POST /tickets` attend `auteur_id` dans
le corps : tout appelant connecté ouvre un ticket au nom de qui il veut, et l'écran
demande un UUID collé à la main.

**Côté Rust, sur `tickets`** — la chaîne `controller → service → repository` respectée :

- `dto.rs` : `auteur_id` sort de `CreateTicket` et d'`UpdateTicket` — un auteur ne se
  réécrit pas. `TicketResponse` le garde.
- `controller.rs` : `create` lit `identite.user_uuid()?` (`crates/rbs-core/src/extract.rs:50`)
  et le passe au service. Seul le contrôleur connaît le jeton.
- `service.rs` : `create(db, auteur, input)` pose `auteur_id: Set(auteur)`, sans savoir
  d'où vient l'auteur.

**Sur `commentaires`, le même geste**, résumé en un paragraphe dans la page. `ticket_id`
reste dans le corps : choisir le ticket commenté est légitime.

**Côté Vue.** `rbs generate client --lang ts --out frontend/src/api` régénère le client.
Rien n'échoue alors, et la page le dit franchement : `corps()` n'a pas de type de retour
déclaré (`Incidents.vue:148` dans `admin-console`), si bien que TypeScript ne contrôle pas
ses propriétés en trop, et le serveur ignore un champ inconnu — aucun DTO ne porte
`deny_unknown_fields`. L'écran continuerait donc d'afficher un champ « auteur » que le
serveur jette. La page retire `auteur_id` du type `Formulaire`, de `corps()` et du
formulaire, dans `Tickets.vue` et `Commentaires.vue` ; la colonne reste affichée dans la
liste. Ce silence est la leçon : le client typé suit le contrat, mais un écran ne
signale un champ disparu que là où il type ce qu'il envoie.

**La limite assumée.** `ticket_id` se saisit encore en UUID dans l'écran des commentaires
(`crates/rbs-cli/src/ecran.rs:675`). La page le dit en une phrase et renvoie au guide
frontend.

**La preuve.** Les tests engendrés ne créent rien sur une table à référence requise
(`crates/rbs-cli/templates/feature/tests/mod.rs.jinja:1`). Un test écrit à la main,
`src/tickets/tests/auteur.rs`, crée un utilisateur et son jeton, poste un ticket dont le
corps porte un `auteur_id` étranger, et exige que le ticket créé porte l'auteur du jeton.
C'est aussi l'extrait que la page cite.

## La page

`docs/docs/tutorials/help-desk.md` (« Building a help desk ») et
`docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/help-desk.md` (« Construire
un gestionnaire de tickets »), `sidebar_position: 10`, dans le même commit. La phrase
« This is the last tutorial » de `typescript-client.md` est corrigée dans les deux langues.

| Section | Contenu | Blocs |
|---|---|---|
| Intro | Ce qu'on construit ; un projet neuf, pas `demo` | — |
| What you need | Rust, Node, PostgreSQL 14+ — renvoi à *Setting up* | — |
| 1. Create the project | `rbs new help-desk`, commit, `rbs add frontend-admin` | `rbs:transcript`, `extrait="oui"` |
| 2. Model the tickets | `generate crud tickets` | `rbs:transcript` |
| 3. Hang comments on them | Le refus si `tickets` manque, puis la vraie commande | deux `rbs:transcript`, dont l'erreur |
| 4. Take the author from the token | Le défaut, puis les trois retouches Rust | `file=examples/help-desk/… region=…` |
| 5. Carry the change to the screen | Régénérer le client, constater que rien n'échoue et pourquoi, retoucher les `.vue` | `rbs:transcript` de `generate client` ; extrait `.vue` |
| 6. Run it | `rbs migrate up`, `rbs dev` | `rbs:libre` motivé |
| Verify | Compte, ticket ouvert depuis l'écran, auteur constaté ; le test `auteur` | extrait du test |
| What was installed | Les fichiers clés de l'exemple | extraits |
| Going further | `relations`, `frontend`, `auth` ; la limite du sélecteur | — |

Toute sortie du CLI est un `rbs:transcript` rejoué par `integration_docs.rs` ; tout code
cité vient d'`examples/help-desk` par région ; les réponses HTTP d'un serveur vivant sont
des `rbs:libre` avec leur raison.

## L'exemple et le harnais

**`examples/help-desk`**, rejoué comme la page le fait taper, puis retouché, puis
`generate client`. `examples/README.md` et `README.fr.md` gagnent une ligne au tableau et une
section `### help-desk` : les commandes de régénération et la liste des fichiers retouchés.
Après un `npm install` local, `frontend/package-lock.json`, `node_modules/` et `dist/` sont
supprimés.

**`crates/rbs-cli/tests/integration_examples.rs`** :

- `crud` + `champs` deviennent `cruds: &'static [(&'static str, &'static str)]`, engendrés
  dans l'ordre ; les six exemples existants passent à une liste d'un élément, sans effet
  sur leur sortie.
- L'entrée `help-desk` : `features: &["frontend-admin"]`, `role: None`,
  `with_upload: false`.
- `edite_a_la_main` : `dto.rs`, `service.rs`, `controller.rs` des deux features,
  `frontend/src/admin/vues/Tickets.vue`, `frontend/src/admin/vues/Commentaires.vue`,
  `src/tickets/tests/auteur.rs`, `src/tickets/tests/mod.rs`.
- `engendre_a_part` : `frontend/src/api/client.ts`.
- `help_desk_is_what_the_cli_produces_today`, et
  `the_hand_edits_of_help_desk_are_in_place`, qui exige `user_uuid()` dans les deux
  contrôleurs, aucun `auteur_id` dans les DTO `Create` et `Update` des deux features, et la
  présence du test `auteur`.

## Vérifications avant livraison

Toutes exécutées, résultats consignés sous `Vérifications :` dans les commits :

1. Dans `examples/help-desk` : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`.
2. Les tests de l'exemple, dont `auteur`, sur un PostgreSQL monté à la main, `--include-ignored`.
3. Dans `examples/help-desk/frontend` : `npm install && npm run build` (`vue-tsc` compris) — `npm ci` exige un lockfile que l'exemple ne versionne pas —, puis
   nettoyage du lockfile, de `node_modules/` et de `dist/`.
4. `cargo test -p rbs-cli --test integration_examples` — les sept exemples.
5. `cargo test -p rbs-cli --test integration_docs --no-fail-fast`.
6. `npm run build` sous `docs/` — régions citées et liens, fr et en.
7. Workspace : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test -p rbs-cli --lib`.

## Git

Branche `docs/tutoriel-help-desk`. Commits conventionnels, dans l'ordre :

1. `test(examples): …` — `crud`/`champs` deviennent `cruds`, les six exemples inchangés ;
   `integration_examples` vert seul.
2. `docs(examples): …` — `examples/help-desk`, son entrée dans le harnais, son test des
   retouches, les deux README.
3. `docs(tutorials): …` — la page fr+en et la correction de `typescript-client.md`.
