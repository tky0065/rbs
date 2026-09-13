# Cinquième exemple : `event-hub`

**Date** : 2026-09-13 · **Backlog** : `IMPROVE.md`, tâche 31 (P2, Medium)

## Problème

Les guides `webhooks`, `scheduler` et `audit` sont écrits à la main : aucun bloc
`file=… region=…` (14/10/12 fences manuelles, contre 13 extraits pour `auth.md`). Aucun
exemple ne porte ces trois fragments, ni `cors`, `docker`, `ci` : leur code n'est compilé
par aucune CI du dépôt, et la documentation en cite des lignes que rien ne vérifie — ce que
`CLAUDE.md` interdit (« aucune ligne écrite à la main »).

## Décision (validée le 2026-09-13)

**Un cinquième exemple, `examples/event-hub`, porte les six fragments manquants** :
`webhooks` (qui tire `jobs` et `auth`, donc `rate-limit`), `scheduler`, `audit`, `cors`,
`docker`, `ci`. Les trois guides sont réécrits sur des extraits de ce projet.

## Conception

### Le projet

- Construit par des commandes, comme les quatre autres : `rbs new event-hub --yes
  --core-path ./crates/rbs-core --database-url 'postgres://rbs:rbs@localhost:5432/event_hub'
  --lang fr`, puis un commit avant **chaque** `rbs add` (`add` refuse un arbre sale), pour
  `webhooks scheduler audit cors docker ci`, puis un `rbs generate crud` d'une ressource
  métier (`orders`, champs courts : `reference:string,amount:integer`), puis
  `rbs generate client --lang ts` si les autres exemples le font.
- Éditions manuelles, minimales et chacune citée par un guide :
  - le service de `orders` enregistre une entrée d'audit (`audit::Entry`) et émet
    `order.created` par `webhooks::emit` **dans la transaction** de la création ;
  - `scheduler::schedules()` inscrit un job du projet à côté de la démo, si le guide en
    a besoin pour montrer `Schedule::every`.
- Chaque bloc de code **de projet** des trois guides devient `file=examples/event-hub/…
  region=…` ; les marqueurs `// region:` sont posés dans les fichiers de l'exemple.
  Les blocs de commandes restent, ou deviennent des `rbs:transcript` quand la sortie est
  montrée.

### Outillage

- `crates/rbs-cli/tests/integration_examples.rs` : une entrée `Exemple` (commandes,
  fichiers édités à la main) et un test `event_hub_is_what_the_cli_produces_today`, plus
  un test des éditions manuelles sur le modèle de `newsletter-queue`.
- `examples/README.md` et `README.fr.md` : une ligne au tableau, une section
  « Regenerating » avec les commandes exactes.
- La CI compile déjà `examples/*/` par une boucle : rien à ajouter, à vérifier.
- `.github/` et `Dockerfile` engendrés dans l'exemple ne sont pas lus par la CI du dépôt
  (GitHub ne lit que la racine) : ils sont là pour être cités et restés sans dérive.
- Les mentions « quatre exemples / four example projects » de la documentation vivante
  (`CLAUDE.md`, `examples/README*`, pages `docs/`) passent à cinq ; les plans et specs
  archivés ne sont pas réécrits, `CHANGELOG` des versions passées non plus.

### Ordre

L'exemple est engendré **après** la fusion des tâches 23 (langue dans `[server]`),
25 (contrôleurs `auth`), 26 (`AGENTS.md`) et 32 (versions `redis`/`aws-sdk-s3`) : il
n'est ainsi engendré qu'une fois, par le CLI final du lot.

## Preuves attendues

1. `cargo test -p rbs-cli --test integration_examples` vert, cinq projets reconnus octet à
   octet, éditions manuelles en place.
2. `cargo check` puis `cargo clippy --all-targets -- -D warnings` dans `examples/event-hub`.
3. Les tests de l'exemple passent contre un PostgreSQL monté à la main
   (`cargo test -- --include-ignored`), puisque la suite ne les lance jamais.
4. `grep -c 'file=' docs/docs/guides/{webhooks,scheduler,audit}.md` non nul, FR idem ;
   `npm run build` et `npm test` dans `docs/` verts ; `node scripts/parite.mjs` sans écart.
