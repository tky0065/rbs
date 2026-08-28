# T3 — `doctor` diagnostique les jobs et la base

**Conception.** Les deux constats ne sont pas de même nature, et ne vivent donc pas au
même endroit.

Le premier est un contrôle *de feature* : il n'a de sens que sur un projet qui a installé
`jobs`, exactement comme `redis`, `mail` et `storage`. Il prend donc la forme déjà en
place — un fichier `doctor/jobs.rs`, une entrée de plus dans `FEATURE_CHECKS`. La
déclaration se lit dans `[package.metadata.rbs].features` (c'est le gardiennage que
`mod.rs` fait déjà) et la section dans `config/default.toml`, par `toml_edit` : une
section en commentaire n'est pas une section, et `rbs add jobs` écrit là ses trois clés.
Le manifeste dit ce qui est installé, la configuration ce qui est réglé — l'écart entre
les deux *est* la faute à diagnostiquer, ce qui interdit de lire une seule des deux
sources.

Le second n'est pas un contrôle de feature : il porte sur la base, et rejoint `base.rs`
plutôt qu'un fichier à lui. Il s'y place **avant** la tentative de connexion : un serveur
qui répond ne prouve rien quand le pilote compilé ne sait pas parler son protocole, et
sonder le port avant de dire l'écart ferait payer trois secondes de délai à un diagnostic
qui tient dans deux lectures de fichier. Le moteur retenu est celui de la feature
`sea-orm` de `[dependencies]` — `sqlx-postgres` — et non la clé
`[package.metadata.rbs].database` : la feature est ce qui est réellement compilé dans le
binaire, la clé n'est qu'une écriture de suivi que `doctor` ne doit pas croire sur parole.

Le message nomme les deux valeurs en conflit, jamais leur conclusion :
« le manifeste compile `sqlx-postgres` et RBS_DATABASE__URL est une URL `mysql://` ».
Un « configuration invalide » obligerait à rouvrir les deux fichiers pour savoir lequel
corriger.

## Étapes

1. `doctor/jobs.rs` : section `[jobs]` de `config/default.toml`, sur le modèle de
   `redis.rs` ; entrée `("jobs", jobs::check)` dans `FEATURE_CHECKS`.
2. `doctor/base.rs` : `pilote()` lit la feature `sea-orm` du manifeste ; l'écart avec le
   schéma de l'URL est rendu avant `joignable()`.
3. `tests/integration_doctor.rs` : `rbs new`, édition de fichiers, `rbs doctor` — sans
   Docker et sans compiler le projet, les deux constats étant des lectures de fichiers.
   L'assertion porte sur la ligne rendue : `doctor` sort en 0 même en échec.
4. Preuves : les deux lignes `✗`, et une morsure par critère (section `[jobs]` remise,
   comparaison pilote/URL retirée).
