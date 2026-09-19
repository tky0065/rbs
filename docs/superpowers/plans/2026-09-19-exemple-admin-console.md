# `admin-console`, le sixième exemple — plan

Issue **#24**, sous la spec **#13**.

## Ce que l'exemple doit être

Le seul endroit où la chaîne complète est prouvée de bout en bout, d'une ligne de
`--fields` à l'écran affiché — et la seule source dont la documentation aura le droit de
citer une ligne. Il porte le socle, l'administration, le partage d'origine, et une entité
dont les écrans sont engendrés.

Il est aussi **le seul exemple construit deux fois** : en Rust, par la boucle clippy que la
CI passe sur chaque exemple, et côté client, par un job dédié.

## Les sept critères et ce qui les prouve

- [x] **Engendré en rejouant les commandes réelles, consignées à côté des autres.** — Fait
  le 2026-09-19 : `examples/README.md` et `examples/README.fr.md` portent la section
  `admin-console`, et `integration_examples::EXEMPLES` la même suite de commandes.
- [x] **Le socle, l'administration, le partage d'origine, une entité dont les écrans sont
  engendrés.** — `features: &["cors", "frontend-admin"]`, qui tire `frontend`, `auth`,
  `mail` et `rate-limit` ; `crud: "incidents"`, huit colonnes pour couvrir les huit
  contrôles du formulaire ; `frontend/src/admin/vues/Incidents.vue` versionné, monté dans
  les deux ancres.
- [x] **Il entre dans le jeu rejoué, la comparaison octet à octet passe.** — Fait :
  `cargo test -p rbs-cli --test integration_examples`, 24 passés, dont
  `admin_console_is_what_the_cli_produces_today`.
- [x] **Aucun fichier de verrouillage de dépendances n'apparaît.** — Fait, et gardé : le
  rejeu ne lance jamais l'installateur, et un `frontend/package-lock.json` laissé sur place
  fait échouer la comparaison en le nommant. Mesuré une fois, pour de vrai.
- [x] **Toute exclusion de la comparaison est compensée par un test dédié.** — Une seule
  exclusion, `frontend/src/api/client.ts` : `the_typescript_client_of_admin_console_is_in_place`
  en répond côté rapide, et le job de CI le régénère puis exige qu'il n'ait pas bougé.
- [x] **Un job dédié installe et construit son frontend.** — `admin-console · frontend`
  dans `.github/workflows/ci.yml` : client typé, `npm install`, `npm run typecheck`,
  `npm run build`. Joué à la main : les trois verts, et `Incidents-*.js` dans
  `dist/assets`.
- [x] **Il devient le seul exemple construit deux fois.** — En Rust par la boucle clippy du
  job `linux` (`cargo clippy --workspace --all-targets -- -D warnings` dans l'exemple :
  vert), et côté client par le job ci-dessus.

## Ce que l'exemple a trouvé

Le test que le fragment `frontend` livre dans le projet de l'utilisateur ne passait pas
`clippy -D warnings` : `fn visant(dir: &PathBuf)` contre `clippy::ptr_arg`. Aucune suite ne
le voyait — `integration_frontend` lance `cargo test`, jamais clippy — et la boucle de la CI
sur les exemples ne rencontrait aucun projet portant le socle. Le premier exemple qui en
porte un l'a rendu rouge en vingt secondes. Corrigé dans la template, et dans l'exemple.

## Ce que le plan ne fait pas

La documentation des deux fragments, qui citera cet exemple : elle est sur **#25**.
