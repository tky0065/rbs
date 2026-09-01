# Plan — Sondes de santé par feature installée

Tâche `IMPROVE.md` #55. Spec :
`docs/superpowers/specs/2026-09-01-sondes-de-sante-par-feature-design.md`, à lire en
entier avant la première ligne de code. Le design est validé : l'appliquer, pas le rouvrir.

Ordre imposé : le noyau d'abord, l'ancre ensuite, les fragments en dernier. Chaque étape
part d'un test qu'on voit échouer.

## 1 — Le noyau : `Probe` et `report`

1. Test d'abord, dans `crates/rbs-core/src/health.rs` : `report` sans aucune sonde rend
   le corps d'aujourd'hui, à l'octet près — `{"status":"ok","checks":{"database":"ok"}}`.
   Le voir échouer à la compilation.
2. Écrire `Probe` et `report` selon la spec. `handler` est conservé et délègue à `report`
   avec une liste vide.
3. Test : une sonde qui rend `false` fait tomber le verdict à 503 alors que la base
   répond, et son nom paraît dans `checks`.
4. Test : deux sondes muettes rendent leur verdict au bout d'**une** fois `PING_TIMEOUT`
   et non de deux, sous `tokio::time::pause()` — c'est la preuve que les sondes sont
   concurrentes, et le seul moyen de la tenir sans dépendance réelle.
5. Test : l'ordre des clés du corps est celui du `BTreeMap`, stable d'un appel à l'autre.

## 2 — La forme du corps

6. Ajouter `extras: BTreeMap<String, Check>` à `Checks`, sous `#[serde(flatten)]`.
   `Checks` et `Health` perdent `Copy` et `Eq` : corriger les usages internes que le
   compilateur signalera.
7. **Vérifier sur la sortie réelle** ce que `utoipa` engendre pour ce champ : rendre le
   document OpenAPI dans un test et le lire. S'il est faux ou refusé, replier sur un
   `#[schema(additional_properties)]` explicite — et dire dans le commit ce qui a été
   observé. Ne rien supposer ici.

## 3 — L'ancre

8. Ajouter `HEALTH_PROBES` à `crates/rbs-cli/src/anchors.rs` et à `ANCRES`
   (`sorted = false`, `optional = false`), avec le doc-commentaire qui dit *pourquoi* elle
   existe.
9. Poser l'ancre dans `templates/project/src/health/controller.rs.jinja`, en passant
   l'appel de `rbs_core::health::handler` à `rbs_core::health::report(state.core().db(),
   vec![…])`.
10. Tests du CLI : `ANCRES` en compte douze, `doctor` les réclame toutes, et le
    `controller.rs` engendré porte la sienne. Mettre à jour le tableau des ancres dans
    `CLAUDE.md` — onze y sont énumérées.

## 4 — Les fragments

11. `redis` : une méthode `ping()` sur `Cache` dans `templates/features/redis/mod.rs.jinja`
    — une connexion prise au pool, un `PING` — et une entrée `[[anchors]]` sur
    `health_probes` dans son `feature.toml`.
12. `storage` : une méthode de disponibilité sur le trait `Storage`
    (`templates/features/storage/mod.rs.jinja`), implémentée par `s3.rs.jinja` — un
    `head_bucket` — et par `files.rs.jinja` — l'accessibilité de la racine — plus son
    entrée `[[anchors]]`.
13. Tests de rendu : chaque fragment inscrit sa ligne au bon endroit.

## 5 — Bout en bout

14. Test d'intégration : `rbs new --with redis` puis compilation du projet engendré. Sans
    lui, rien ne prouve que la sonde inscrite compile. Docker requis ; lancer la suite
    `--ignored` avec `--no-fail-fast`, sans quoi elle s'arrête au premier binaire et
    masque les échecs suivants.
15. Régénérer les quatre projets d'`examples/` selon `examples/README.md` — les templates
    ont changé, `integration_examples.rs` compare octet à octet. **Par diff entre deux
    générations, jamais par écrasement** : ces fichiers portent des éditions manuelles.
16. Documentation bilingue : la page qui décrit `/health` et son corps, en anglais et en
    français, dans le même commit.

## 6 — Vérification

17. `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
    `cargo fmt --all --check`, puis la suite `--ignored` sous Docker avec
    `--no-fail-fast`. Lire chaque sortie ; ne rien affirmer qui n'ait été lu.
