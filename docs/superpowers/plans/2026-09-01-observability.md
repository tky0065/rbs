# Plan — `rbs add observability`

Tâche `IMPROVE.md` #56. Spec : `docs/superpowers/specs/2026-09-01-observability-design.md`,
à lire en entier avant la première ligne de code. Le design est validé : l'appliquer, pas
le rouvrir.

Ordre imposé : le noyau d'abord — sans lui le fragment n'a rien où se brancher — puis le
fragment, puis `doctor`.

## 1 — Relever les versions

1. `cargo add --dry-run` pour `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`,
   `tracing-opentelemetry`, et pour les deux candidates du registre de métriques
   (`metrics` + `metrics-exporter-prometheus`, `prometheus-client`). **Les versions se
   relèvent, ne se devinent pas** ; `cargo info` remonte parfois des pré-publications.
2. Trancher entre les deux candidates sur ce que montre leur maintenance, et justifier le
   choix en une ligne dans le message de commit.

## 2 — Le noyau

3. Ajouter la feature cargo `observability` au `Cargo.toml` de `rbs-core`, désactivée par
   défaut, avec ses `dep:` — sur le moule de `auth`, et en remplaçant le commentaire
   « vides et sans dépendances » là où il ne vaut plus.
4. Test d'abord : avec la feature activée et **aucune** variable d'environnement,
   `logs::init()` réussit et n'installe aucun exportateur ; `shutdown()` appelée sans
   `init()` ne panique pas. Les voir échouer.
5. Écrire la greffe dans `logs::init()`, sous `#[cfg(feature = "observability")]` : la
   couche `tracing-opentelemetry` se compose avec le formateur existant quand
   `OTEL_EXPORTER_OTLP_ENDPOINT` est présent. Le nom du service vient de
   `OTEL_SERVICE_NAME`, à défaut du nom du paquet.
6. Écrire `logs::shutdown()`, et son équivalent vide sans la feature.
7. **Non-régression** : les tests existants de `logs` passent sans la feature, à
   l'identique. `#![warn(missing_docs)]` vaut sur `rbs-core` : chaque item public porte
   son `///`.

## 3 — Le fragment

8. Créer `crates/rbs-cli/templates/features/observability/` sur le moule des neuf autres.
   Le `feature.toml` **énumère ses `[[files]]`** — ne pas reproduire le silence de `ci`,
   que le backlog reproche par ailleurs.
9. `metrics.rs.jinja` : le registre et le middleware de comptage. Les trois séries de la
   spec, avec `path` pris de `MatchedPath` d'axum.
10. Test d'abord, de rendu puis dans les tests engendrés : une requête sur une route à
    paramètre compte sous le **gabarit** `/articles/{id}` et non sous l'URL demandée.
    C'est le test qui garde la cardinalité du collecteur ; sans lui, la feature devient
    nuisible en production.
11. `mod.rs.jinja` : le second listener sur `observability.metrics_port`, et `config.rs`
    pour sa section. `/metrics` n'est monté sur le routeur public à aucun moment.
12. Les trois `[[anchors]]` : `features`, `layers` (le middleware), `startup` (le
    listener, dans un `tokio::spawn`). Le middleware va bien à `layers` et pas ailleurs :
    il doit voir le `request_id` et compter les réponses courtes des couches posées avant.
13. Ajouter la feature au manifeste `rbs-core` du projet, comme `auth` ajoute la sienne.
14. `tests.rs.jinja` : `#[ignore]` sur ce qui exige un service externe, comme `jobs`,
    `redis` et `storage` le font déjà.

## 4 — `doctor`

15. Un contrôle de plus dans `FEATURE_CHECKS`, sous `observability` : la section de config
    est présente, et son port diffère de `server.port`. L'endpoint OTLP n'est pas
    contrôlé — son absence est un mode légitime.
16. Test du contrôle, dans les deux verdicts.

## 5 — Le pourtour

17. La liste des features est écrite en toute lettre à plusieurs endroits — le
    doc-commentaire de `Commands::Add` dans `cli.rs`, le site, les `README`, `CLAUDE.md`.
    Les reprendre toutes : `grep -rn "rate-limit" --include='*.rs' --include='*.md'` les
    trouve.
18. Documentation bilingue : `docs/docs/features/observability.md` et sa jumelle
    française, dans le même commit, avec l'appel à `logs::shutdown()` et la raison de
    l'exposer sur un port à part.
19. Régénérer `examples/` si une template du projet a bougé, selon `examples/README.md`,
    **par diff entre deux générations et jamais par écrasement**.

## 6 — Vérification

20. `cargo test --workspace`, `cargo test -p rbs-core --features observability`,
    `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`,
    puis `rbs add observability` sur un projet neuf **et sa compilation** — le seul test
    qui prouve que la dizaine de crates tirées s'accordent. Docker requis, suite
    `--ignored` avec `--no-fail-fast`. Lire chaque sortie avant d'affirmer quoi que ce
    soit.
