# Frictions relevées à la répétition à blanc du critère de sortie

Deux parcours menés au pied de la lettre le 2026-08-27, dans des répertoires isolés, avec
un `cargo install --root` dédié et un PostgreSQL 18 en conteneur sur un port non standard.

- **Parcours A — le lecteur du `README.md`** : **échec**, à l'avant-dernière commande.
- **Parcours B — le lecteur de `docs/docs/getting-started.md`** : **succès**, jusqu'à
  `POST /articles` → 201 et `GET /articles` → 200.

L'écart entre les deux est le résultat : la page qui mène au but n'est pas celle sur
laquelle un tiers atterrit.

## Bloquantes — le parcours du README ne peut pas aboutir

**D1 et D2 corrigés le 2026-08-27**, par la voie retenue : le « Quick look » cesse de se
présenter comme une transcription exécutable et renvoie au guide, qui porte seul la
séquence complète. D3 est traité dans le même mouvement — l'encart sur le `rbs` de Ruby
rejoint la section d'installation, là où le piège se referme. **D4 corrigée le
2026-08-27** — les quatre frictions sont closes.

**D1 · Le « Quick look » du README génère un projet qui ne compile pas.**
`rbs new blog-api` écrit `rbs-core = "0.1.0"` dans le manifeste ; la crate n'est pas
publiée, et `rbs migrate up` meurt sur `no matching package named 'rbs-core' found`.
Le flag `--core-path` répare cela, mais il exige un clone de `crates/rbs-core` — or le
README installe par `cargo install --git`, qui ne laisse aucun clone sur le disque. Le
parcours est donc *structurellement* sans issue, non pas mal documenté.
Correctif : le Quick look doit reproduire la séquence clone + `--core-path`, ou renvoyer
au guide avant la première commande.

**D2 · Le README ne fait jamais démarrer de base de données.**
Il enchaîne `rbs new` → `rbs generate crud` → `rbs migrate up` sans dire qu'un PostgreSQL
doit répondre. Seul le prérequis « les projets générés visent PostgreSQL 18 ou plus »
l'évoque, dix lignes plus haut et sans commande.
Correctif : la ligne `docker run` du guide, ou un renvoi explicite.

## Non bloquantes

**D3 · Le conflit de nom `rbs` n'est signalé que sur le site.**
Reproduit sur la machine de test : `rbs` résout vers le RBS de Ruby (`rbs 3.10.0`). Le
guide porte l'encart, le README non — alors que c'est le README qui porte la ligne
`cargo install`, donc le premier endroit où le piège se referme.

**D4 · `rbs doctor` déclare saine une dépendance introuvable.**
Sur le projet bloqué du parcours A : `✓ versions   projet et rbs-core 0.1.0 alignés sur le
CLI 0.1.0`, quand `cargo` ne résout pas cette crate. Le seul `✗` porte sur la base. Un
tiers bloqué lance `doctor` — la commande que le guide recommande justement quand
« something looks wrong » — et n'apprend rien de ce qui le bloque.
Correctif retenu et appliqué le 2026-08-27 : le contrôle `versions` ne peut pas savoir
si une version de crates.io est résoluble sans requête réseau ; il sait en revanche, à la
compilation du CLI, si son noyau est publié. Une dépendance de registre déclarée dans cet
intervalle rend désormais `✗ versions   rbs-core 0.1.0 déclaré depuis crates.io, où rbs
n'est pas encore publié`, avec la ligne de manifeste à remplacer — le constat primant sur
l'écart de numéros, qui n'apprend rien quand la résolution échoue. Un noyau pris d'un
chemin local reste `✓` : le parcours B n'est pas touché.

## Corrigé pendant la répétition

**PostgreSQL 14 → 18 dans le guide de démarrage.** `getting-started.md` posait
« PostgreSQL 14 or later » et invitait à pointer un serveur existant, quand le reste du
projet tient 18 pour un plancher dur — `uuidv7()`, défaut de clé primaire des migrations
générées, n'existe pas avant. Corrigé dans les deux langues, avec la raison.

## Ce que cette répétition ne prouve pas

Elle ne trouve que les frictions mécaniques : commande qui échoue, prérequis absent,
contradiction entre deux pages. Les frictions cognitives — ce qu'un tiers ne *comprend*
pas — restent hors d'atteinte de qui connaît déjà les réponses. Le critère de `V1` nomme
une personne extérieure au projet, et rien ici ne la remplace.
