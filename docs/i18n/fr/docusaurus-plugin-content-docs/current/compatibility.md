---
sidebar_position: 6
title: Compatibilité
---

# Compatibilité

rbs suit le [versionnage sémantique](https://semver.org/lang/fr/). À l'intérieur d'une
version majeure, ce qui marchait continue de marcher : écrivez `rbs-core = "1"` dans un
manifeste, montez de version, votre projet compile toujours.

Une promesse de ce genre ne vaut que ce que valent ses frontières. rbs publie deux crates
et écrit du code dans vos propres sources : « l'API » n'y est donc pas une chose, mais
cinq — dont trois sont délibérément laissées dehors. Cette page dit lesquelles, et
pourquoi.

## Ce que la promesse couvre

| Périmètre | Couvert | Pourquoi |
|---|---|---|
| L'API publique de `rbs-core` | **oui** | C'est ce qu'une version majeure gèle : chaque item exporté par la crate, et chaque signature qu'il porte. |
| Le format des ancres et de `[package.metadata.rbs]` | **oui** | Un projet engendré par une version du CLI doit rester lisible par la suivante. |
| Le code engendré dans votre projet | non | Il vous appartient dès qu'il est écrit ; le CLI ne le relit jamais. |
| La bibliothèque de `rbs-cli` | non | `rbs-cli` publie un binaire ; que sa bibliothèque soit visible est un détail de construction, pas une offre. |
| Les features vides `redis`, `mail` et `storage` de `rbs-core` | non | Elles réservent un nom et ne portent aucun code. |

Les deux crates sont publiées depuis un même workspace et partagent leur numéro de
version. Un CLI qui affiche `rbs 1.2.0` est celui qui va avec `rbs-core 1.2.0`.

## L'API publique du noyau

Tout ce que `rbs-core` exporte est couvert : le type `Error` et ses variantes, l'état de
l'application, les middlewares, le chargeur de configuration, les helpers de pagination et
d'OpenAPI, et les items qu'ajoute la feature `auth`. À l'intérieur de la 1.x, aucun ne
disparaît, ne change de signature, ni ne change de sens.

Le gel se mesure au lieu de se déclarer. `cargo-semver-checks` tourne en CI à chaque
commit, en comparant l'arbre de travail à la dernière version publiée sur crates.io, toutes
features activées — une variante retirée ou une fonction renommée fait échouer la
construction et nomme l'item perdu. Une page peut se tromper sur un gel ; une construction,
non.

Deux conséquences méritent d'être connues avant d'écrire du code contre la crate :

- **Les enums et structs publics portent `#[non_exhaustive]`.** Filtrer sur `Error` réclame
  un bras `_ =>`, et ces types se construisent par leurs constructeurs plutôt que
  littéralement. C'est ce qui permet à une version mineure d'ajouter une variante sans
  casser personne, et c'est pourquoi l'attribut a été posé avant la 1.0 plutôt qu'après :
  après, il aurait coûté une majeure.
- **Ajouter est permis, retirer ne l'est pas.** Une version mineure peut apporter des items
  neufs, des variantes neuves, des champs neufs. Un code écrit en supposant une liste close
  en rencontrera une plus longue.

Trois types font exception et restent exhaustifs, parce que le code engendré les construit
ou les déstructure : les claims JWT, l'extracteur JSON validant et le modificateur de
réponses OpenAPI. Leur forme est gelée plus étroitement que le reste, non moins.

## Les ancres et les métadonnées du projet

C'est le périmètre qu'on oublie, et celui dont la perte fait le plus mal.

À côté de votre code, un projet engendré porte deux choses que vous n'appelez jamais : neuf
ancres en commentaires — `// <rbs:features>`, `// <rbs:routes>`, `// <rbs:openapi>`,
`// <rbs:migration_modules>`, `// <rbs:migrations>`, `// <rbs:state_champs>`,
`// <rbs:state_init>`, `// <rbs:startup>`, `// <rbs:seeds>` — et une section
`[package.metadata.rbs]` dans `Cargo.toml`, qui consigne la version de rbs ayant engendré
le projet, les features qui y sont installées et la base qu'il vise. Ni l'une ni l'autre
n'est une API Rust. Une promesse de compatibilité écrite pour les seules API Rust
enjamberait les deux sans les voir.

Elle ne le doit pas, car elles sont tout le canal par lequel un CLI plus récent reconnaît
un projet créé par un plus ancien. [`rbs add`](./cli/add.md) lit `[package.metadata.rbs]`
pour savoir ce qui est déjà installé, ce qui fait son idempotence ;
[`rbs generate`](./cli/generate.md) trouve les ancres pour savoir où va une route, un module
ou une migration ; `rbs upgrade` y réécrit la version consignée. Renommez une ancre,
déplacez la section, changez le sens d'une clé, et tout projet engendré avant ce changement
cesse d'être un projet que les outils comprennent. La panne est silencieuse de la pire
manière : rien ne casse, la commande ne trouve simplement plus où insérer et affiche un
bloc à coller à la main — à chaque commande, indéfiniment.

Le format est donc couvert exactement comme l'API Rust l'est. À l'intérieur de la 1.x :

- les neuf noms d'ancres et leur syntaxe de commentaire ne changent pas, non plus que la
  règle voulant qu'une commande n'écrive rien quand son ancre manque ;
- les clés de `[package.metadata.rbs]` gardent leur nom et leur sens. Une clé peut
  s'ajouter ; une clé absente se lit comme un défaut, jamais comme une erreur.

Ce qui n'est pas promis, c'est qu'un projet engendré en 0.4.0 porte déjà toutes les ancres
qu'une feature ultérieure réclame. Il ne les porte pas, et ne les portera jamais — des
features neuves apportent des ancres neuves. Ce cas est prévu plutôt que subi : la commande
signale l'ancre introuvable et affiche le bloc, et [`rbs doctor`](./cli/doctor.md) les
vérifie toutes les neuf avant que rien n'aille mal.

## Ce que la promesse laisse dehors

**Le code engendré dans votre projet.** Il vous appartient dès l'instant où
`rbs generate` l'écrit. Rien n'y porte la mention « engendré, ne pas modifier », parce que
[toute la conception](./architecture.md) est que vous le lisiez et le modifiiez. rbs ne peut
rien promettre sur la forme d'un fichier que vous avez depuis réécrit, et n'en a pas besoin :
aucune version de rbs ne le réécrira non plus. `rbs upgrade` modifie `Cargo.toml` et rien
d'autre. Si une version ultérieure engendre une couche service différente, c'est une
différence entre deux projets neufs, pas un changement dans le vôtre.

**La bibliothèque de `rbs-cli`.** Le paquet existe pour installer le binaire `rbs`. Sa
bibliothèque est visible parce que c'est ainsi que la crate se construit et se teste, non
parce qu'elle est offerte comme une API. En dépendre, c'est s'exposer à une rupture dès une
version corrective.

**Les features vides `redis`, `mail` et `storage` de `rbs-core`.** Elles réservent des noms
pour un travail qui se fait dans votre projet plutôt que dans le noyau, et ne portent aucun
code. En activer une ne change rien aujourd'hui. Le jour où l'une portera du code, ce sera
un ajout — et la forme qu'il prendra n'est pas promise d'avance.

## Lire un numéro de version

- **Correctif** — de 1.0.0 à 1.0.1 : des corrections. Rien ne bouge dans un périmètre
  couvert.
- **Mineure** — de 1.0 à 1.1 : des ajouts. Commandes neuves, features engendrées neuves,
  items neufs dans le noyau, éventuellement ancres neuves dans les projets engendrés
  ensuite. Les projets existants continuent de marcher.
- **Majeure** — de 1 à 2 : un périmètre couvert peut rompre. Elle vient avec des notes de
  migration, que `rbs upgrade` affiche pour le saut qu'il effectue.

Épinglez selon votre appétit : `rbs-core = "1"` pour prendre les mineures à mesure,
`rbs-core = "1.2"` pour ne prendre que les correctifs. Le journal des modifications dit ce
que chaque version a fait ; cette page dit ce qu'elle avait le droit de faire.
