# `integration_redis` sous conteneur

## Ce qui s'ajoute

- `crates/rbs-cli/tests/integration_redis.rs` : `GenericImage("redis", "8-alpine")`,
  `rbs new` + `rbs add redis`, puis `cargo test --workspace -- --ignored` dans le projet
  généré, `RBS_CACHE__URL` pointant sur le port publié. Aucune dépendance de
  développement ajoutée — `testcontainers` est déjà en `[dev-dependencies]`.
- `templates/features/redis/tests.rs.jinja` : deux tests `#[ignore]` qui joignent le
  serveur, livrés dans tout projet qui fait `rbs add redis`.
- `templates/features/redis/feature.toml` : `[cargo.tokio] features = ["time"]`.

## Décisions

- **Les tests serveur sont livrés à l'utilisateur**, et non écrits par le test
  d'intégration dans un projet jetable : le code généré est fait pour être lu et repris,
  et un projet doté d'un cache mérite la suite qui le prouve contre son propre Redis.
- **`Cache::depuis_config()` et non un `Config` construit à la main** : c'est le parcours
  qu'un handler suit, et il prouve au passage la section `[cache]` et sa surcharge par
  `RBS_CACHE__*`. Un `Config` littéral court-circuiterait la moitié de ce que `L1` a posé.
- **`[cargo.tokio] features = ["time"]` est déclarée**, alors que `redis`/`tokio-comp`
  l'active probablement par unification : une feature dont le projet a besoin ne se prend
  pas dans la dépendance d'un tiers, qui peut la retirer sans prévenir.
- **Pas de PostgreSQL dans ce test** : `-- --ignored` ne lance que les tests serveur, et
  les tests de santé du squelette restent hors du lot. Le projet doit compiler, pas se
  migrer.
- Chaque test porte **son propre préfixe de clés** : ils partagent un serveur et cargo les
  lance en parallèle.

## Le faux vert à éviter

Le test de TTL **assert la présence avant l'attente**. Sans cette assertion, un `get`
cassé rendant toujours `None` passerait au vert — et c'est exactement le défaut que `L2`
n'a pas pu exclure faute de serveur.

## Ordre

1. Les deux tests du fragment + `integration_redis.rs`, lancés → échec.
2. `[cargo.tokio] features = ["time"]` si la compilation le réclame → vert.
3. clippy et rustfmt sur le dépôt et sur le projet généré.
4. Morsures : `get` rendant toujours `None` (doit faire tomber le TTL *et* le parcours) ;
   `set_ex` remplacé par `set` sans expiration (doit ne faire tomber que le TTL) ;
   `a_supprimer` sans son `retain` (la clé témoin `sessions:1` doit disparaître à tort).
