---
sidebar_position: 8
title: rbs et Loco
---

# rbs et Loco

[Loco](https://loco.rs) se tient sur le même terrain que rbs : Axum pour le HTTP, SeaORM
pour la base, un outil en ligne de commande qui engendre modèles, contrôleurs et
migrations. Qui découvre l'un rencontre l'autre dans l'heure, et pose la question juste :
pourquoi deux ?

Cette page n'y répond pas par un tableau de fonctionnalités. Un tableau compare deux
listes le jour où on l'écrit, et ment dès la version suivante de l'autre projet. Ce qui ne
bouge pas d'une version à l'autre, c'est l'endroit où chacun trace ses frontières — ce que
le cadre garde, ce qu'il cède, la façon dont le générateur touche vos fichiers — et c'est
cela que la page compare.

:::info[Comment les affirmations sur Loco ont été vérifiées]
Chaque affirmation sur Loco ci-dessous a été vérifiée le **2026-09-22** sur **Loco 1.1.0**,
dernière version de [`loco-rs` sur crates.io](https://crates.io/crates/loco-rs), publiée le
2026-08-16. Chacune renvoie à la page de documentation ou au fichier source qui la fonde ;
les liens vers le dépôt visent l'étiquette `v1.1.0`, et disent donc toujours ce qu'ils
disaient ce jour-là. Loco avance : si un lien dit aujourd'hui autre chose, c'est le lien
qui a raison, et cette page qui est en retard.
:::

## Deux réponses à la même question

Tout cadre de travail décide ce qu'il garde et ce qu'il cède.

Loco se décrit comme [« fortement inspiré de Rails »](https://github.com/loco-rs/loco/blob/v1.1.0/README.md),
avec la convention plutôt que la configuration pour premier principe. Sa documentation
énonce une [directive première](https://loco.rs/docs/explanation/why-batteries-included/) :
recourir d'abord à un composant intégré, ensuite à un générateur, et ne câbler à la main
qu'en dernier recours. Les composants intégrés — accès à la base, tâches de fond,
planificateur, envoi de courriel, stockage, cache, vues, authentification JWT — sont
compilés dans la crate `loco-rs` derrière des features Cargo, et quand l'un ne convient
pas, Loco offre des points d'extension typés pour le remplacer (même page).

rbs répond par un seul test, appliqué à chaque morceau de code :
[un développeur voudra-t-il le relire ?](./architecture.md#la-frontière-noyau--généré) Si
non, il va dans `rbs-core`, une dépendance mise à jour comme n'importe quelle autre — le
pool de connexions, le type d'erreur et son corps RFC 9457, le formateur de logs. Si oui,
le CLI l'écrit dans votre `src/`, et il vous appartient. Le noyau est petit à dessein, et
ne se dote pas d'un composant intégré là où du code engendré suffit : une capacité
optionnelle comme la file de tâches ou la connexion arrive sous forme de
[fragment](./cli/add.md), écrit dans votre projet ; quand le noyau doit en porter une
primitive, comme pour la connexion, cette primitive reste derrière une feature Cargo.

Aucune des deux réponses n'est la meilleure dans l'absolu. Loco met davantage dans le
cadre, si bien qu'une plus grande part de votre application progresse à chaque montée de
version. rbs met davantage dans votre dépôt, si bien qu'une plus grande part de votre
application se lit, et se modifie, sans connaître le cadre.

## À qui appartient le cycle de vie

La différence se voit le mieux au démarrage de l'application.

Dans une application Loco, le démarrage est conduit par le cadre. Votre `App` implémente
le [trait `Hooks`](https://loco.rs/docs/explanation/architecture/), et la séquence de
démarrage de Loco en appelle les méthodes dans l'ordre — configuration, contexte,
initialiseurs, routes, middlewares. La plupart de ces méthodes ont une implémentation par
défaut, si bien qu'une `App` minimale n'en implémente qu'une poignée ; on surcharge les
autres le jour où il faut en changer le comportement (même page). Le
[`main.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/src/bin/main.rs.t)
engendré tient en un appel : `cli::main::<App, Migrator>()`.

Dans un projet rbs, le [`main.rs`](https://github.com/tky0065/rbs/blob/main/examples/hello-crud/src/main.rs)
*est* la séquence de démarrage : charger la configuration, ouvrir le pool, construire
l'état, écouter sur le port, servir, vider les requêtes en vol à l'arrêt. Chaque étape
appelle `rbs-core`, mais l'ordre est écrit dans votre fichier, et le changer revient à
modifier une ligne plutôt qu'à chercher le hook qui le gouverne. Le routeur, l'état et le
document OpenAPI sont des fichiers engendrés de la même sorte.

La manière de Loco est plus courte à lire et moins chère à mettre à jour. Celle de rbs est
plus longue à lire, et n'a rien derrière elle à découvrir.

## Comment le générateur touche vos fichiers

Sur cet axe, les deux projets sont plus proches qu'on ne le croirait, et il faut le dire.

**Aucun des deux ne réécrit d'arbre syntaxique.** Les gabarits du générateur de Loco
déclarent des *injections* : une ligne ajoutée en fin de fichier, ou insérée avant ou
après une ligne qui répond à une expression régulière. Le gabarit de contrôleur, par exemple,
insère son `.add_route(...)` après la ligne qui contient `AppRoutes::` dans `src/app.rs`
([gabarit](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/templates/controller/api/controller.t)) ;
le gabarit de migration insère avant un commentaire, `// inject-above (do not remove this comment)`,
dans `migration/src/lib.rs`
([gabarit](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/templates/model/model.t),
[squelette](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/migration/src/lib.rs.t)).
Le moteur d'injection, [`rrgen` 0.6](https://crates.io/crates/rrgen), calcule toutes les
injections avant d'écrire quoi que ce soit, et échoue en nommant le motif qu'il n'a pas
trouvé ([source](https://docs.rs/crate/rrgen/0.6.0/source/src/lib.rs)).

rbs fait le même genre d'insertion textuelle, avec une règle à lui : **il n'insère jamais
qu'entre des ancres en commentaire**, des marqueurs appariés comme `// <rbs:startup>` et
`// </rbs:startup>`, posés par le squelette ou par un fragment et recensés en un seul
endroit, que parcourt [`rbs doctor`](./cli/doctor.md). Une ligne de vrai code n'est
jamais un repère. La raison tient à la propriété : une ligne de code, vous avez le droit
de la renommer, de la reformater, de la déplacer, et un générateur qui la cherche fait de
vos modifications ce qui casse la génération suivante. Une ancre est une ligne dont le
seul office est d'être trouvée. Quand elle manque, rbs n'écrit rien, et affiche le bloc
que vous collerez où vous voudrez.

Le format de ces ancres fait partie de la [promesse de compatibilité](./compatibility.md#les-ancres-et-les-métadonnées-du-projet)
de rbs : un projet engendré par une version du CLI reste lisible par la suivante.

## Un code qui appartient à son auteur

La documentation de Loco dit du code engendré qu'il est
[« un point de départ que vous possédez et modifiez »](https://loco.rs/docs/explanation/why-batteries-included/).
rbs dit la même chose, et va un pas plus loin : une fois écrit, le projet ne doit plus du
tout avoir besoin de rbs.

Chez Loco, le générateur fait partie de l'application. `cargo loco` est un alias Cargo de
`cargo run --`
([squelette](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/.cargo/config.toml.t)) :
engendrer un contrôleur lance votre propre binaire, dont la ligne de commande vient de
`loco-rs`. La sous-commande `generate` n'est compilée que dans les builds de débogage
([`src/cli.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/src/cli.rs)), et `loco-gen`, la
crate du générateur, est une dépendance de `loco-rs`
([`Cargo.toml`](https://github.com/loco-rs/loco/blob/v1.1.0/Cargo.toml)). Ce couplage a
son avantage : le générateur correspond toujours à la version du cadre contre laquelle votre
projet compile, et ses gabarits se surchargent projet par projet en les copiant dans un
répertoire `.loco-templates/`
([documentation](https://loco.rs/docs/how-to/override-templates/)).

Chez rbs, le générateur est un outil à part, installé séparément par
`cargo install rbs-cli`. Un projet engendré dépend de `rbs-core` et jamais de `rbs-cli` ;
son `Makefile` appelle `cargo`, `npm` et `docker compose`, jamais `rbs`. Clonez-le sur une
machine sans le générateur : tout se compile, se teste et tourne encore.
[`rbs upgrade`](./cli/upgrade.md) écrit dans `Cargo.toml` et dans les zones réservées du
guide des agents, et crée les fichiers qui manquent à un projet plus ancien sans jamais en
réécrire un qui existe : le code que le CLI a écrit n'est jamais relu ni re-rendu, et
c'est pourquoi aucun fichier engendré ne porte de bandeau « généré, ne pas modifier ».

C'est à la montée de version que les deux choix paient leur prix. Une nouvelle version de
Loco atteint votre code par le cadre : son
[guide de mise à jour](https://loco.rs/docs/extras/upgrades/) consiste à relever la
version dans `Cargo.toml`, lire le changelog pour y trouver les ruptures, et lancer
`cargo loco doctor`. Une nouvelle version de rbs atteint `rbs-core` de la même façon, mais
aucun code déjà engendré : l'amélioration d'un gabarit arrive dans la prochaine feature
que vous engendrez, pas dans celles que vous avez déjà.

## Engendrer depuis la ligne de commande

Les deux générateurs prennent une liste de champs `nom:type`. Ce qu'ils exigent de voir
tourner diffère.

Le `generate model` de Loco écrit une migration, **l'applique à votre base de
développement**, puis régénère les entités SeaORM dans `src/models/_entities/` en appelant
`sea-orm-cli` sur cette base
([documentation](https://loco.rs/docs/how-to/add-model/),
[`loco-gen/src/model.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/model.rs),
[`src/db/entities.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/src/db/entities.rs)).
Avec `SKIP_MIGRATION`, il n'écrit que la migration ; les entités viennent alors d'un
`cargo loco db entities` ultérieur (même page de documentation). Le schéma de la base est
la source de l'entité.

rbs prend l'autre sens : `rbs generate crud` écrit l'entité SeaORM **et** sa migration
depuis `--fields`, sans aucune base démarrée. La liste des champs est la source des deux,
et un clone tout frais peut engendrer une ressource avant que rien ne tourne. C'est
l'inverse de `sea-orm-cli generate entity`, et à dessein ; voir la référence de
[`generate`](./cli/generate.md).

## Quand préférer Loco

Honnêtement : souvent.

Préférez Loco si vous voulez un cadre plutôt qu'un générateur — un cadre qui possède le
cycle de vie, dont les composants intégrés s'améliorent sous vos pieds à chaque version,
et dont on apprend une fois les conventions pour les retrouver dans tout projet Loco.
Préférez-le si vous venez de Rails, qu'il
[suit ouvertement](https://github.com/loco-rs/loco/blob/v1.1.0/README.md), ou si vous
voulez des vues rendues côté serveur, qu'il [fournit](https://loco.rs/docs/explanation/why-batteries-included/)
et que rbs ne fournit pas : rbs engendre une API HTTP, avec un client Vue optionnel à côté.
Préférez-le si les fonctionnalités dont vous avez besoin figurent déjà parmi ses
[composants intégrés](https://loco.rs/docs/explanation/why-batteries-included/) et que vous
aimez mieux les configurer que lire leur code. Et préférez-le si l'ancienneté d'un projet
compte pour vous : `loco-rs` est publié sur
[crates.io](https://crates.io/crates/loco-rs/versions) depuis novembre 2023, et la
première ligne de rbs date d'août 2026.

Préférez rbs si vous voulez que chaque ligne qui fait tourner votre application vive dans
votre propre dépôt, lisible sans connaître de cadre, et un générateur que vous pourrez
désinstaller dès le lendemain.

## Sources

Consultées le 2026-09-22, Loco 1.1.0 :

- [`loco-rs` sur crates.io](https://crates.io/crates/loco-rs) — version, date de publication, et [première version](https://crates.io/crates/loco-rs/versions), 0.1.0, le 2023-11-23
- [README](https://github.com/loco-rs/loco/blob/v1.1.0/README.md) — l'inspiration de Rails, la convention plutôt que la configuration
- [Why "batteries included"?](https://loco.rs/docs/explanation/why-batteries-included/) — la directive première, les composants intégrés derrière des features, le code engendré comme point de départ
- [Architecture: the request lifecycle](https://loco.rs/docs/explanation/architecture/) — le trait `Hooks` et la séquence de démarrage
- [Add a model](https://loco.rs/docs/how-to/add-model/) — migration appliquée, entités régénérées, `SKIP_MIGRATION`
- [Override templates](https://loco.rs/docs/how-to/override-templates/) — `.loco-templates/`
- [Upgrades](https://loco.rs/docs/extras/upgrades/) — la procédure de mise à jour
- Source à `v1.1.0` : [`Cargo.toml`](https://github.com/loco-rs/loco/blob/v1.1.0/Cargo.toml), [`src/cli.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/src/cli.rs), [`src/db/entities.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/src/db/entities.rs), [`loco-gen/src/model.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/model.rs), les gabarits de [contrôleur](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/templates/controller/api/controller.t) et de [modèle](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/src/templates/model/model.t), le [`main.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/src/bin/main.rs.t), le [`.cargo/config.toml`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/.cargo/config.toml.t) et le [`migration/src/lib.rs`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-new/base_template/migration/src/lib.rs.t) du squelette
- [`rrgen` 0.6.0](https://docs.rs/crate/rrgen/0.6.0/source/src/lib.rs), le moteur d'injection de Loco, dépendance de `loco-gen` ([`Cargo.toml`](https://github.com/loco-rs/loco/blob/v1.1.0/loco-gen/Cargo.toml))
