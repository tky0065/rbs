---
sidebar_position: 13
title: AGENTS.md
---

# AGENTS.md

Un agent lâché dans un projet rbs n'a aucun moyen de savoir que rbs existe. Il voit des
fichiers Rust, il écrit des fichiers Rust : il recrée à la main les six fichiers d'une
feature, oublie la migration, ignore les ancres, et casse la dépendance unidirectionnelle
sur laquelle repose l'architecture. Le CLI est juste là, et rien ne dit à l'agent de s'en
servir.

`rbs new` répond à cela en écrivant `AGENTS.md` à la racine du projet — le mode d'emploi
de rbs, écrit pour un agent plutôt que pour un humain. `AGENTS.md` est un format neutre,
que Codex, Cursor et Copilot lisent tel quel. Claude Code, lui, ne le lit pas : il lit
`CLAUDE.md`, et n'atteint `AGENTS.md` que par un import qui y est déclaré. `rbs new` écrit
donc aussi `CLAUDE.md`, le seul fichier qu'il engendre pour un outil en particulier, et
long d'une ligne — `@AGENTS.md` — pour que le mode d'emploi garde une seule source et que
rien dans `CLAUDE.md` ne puisse s'en écarter. Ce fichier vous appartient dès qu'il existe :
ajoutez vos propres consignes sous l'import, aucune commande ne les réécrira.

## Les deux zones que rbs possède

Seules deux parties d'`AGENTS.md` appartiennent à rbs, chacune délimitée par un
commentaire HTML :

```text
# <projet> — mode d'emploi pour agents

<!-- rbs:guide 1.2.0 -->
… le mode d'emploi …
<!-- /rbs:guide -->

<!-- rbs:inventory -->
… l'état du projet …
<!-- /rbs:inventory -->

## Notes du projet
```

`rbs:guide` est le mode d'emploi proprement dit — la règle du CLI d'abord, un mot sur les
zones de ce fichier, le tableau des commandes, des recettes, l'architecture imposée, la
liste des ancres, ce que rbs ne couvre pas, et les commandes à lancer avant de conclure.
Son marqueur d'ouverture porte la version du CLI qui l'a écrit, et c'est ce numéro que
[`rbs upgrade`](../cli/upgrade.md) compare et réécrit.

`rbs:inventory` est l'état du projet lui-même, recalculé en entier à chaque écriture : la
version de rbs et le moteur de base, les fragments installés, les entités engendrées, et
les ancres que le projet porte réellement. Il reste court et factuel exprès, pour qu'un
agent n'ait pas à explorer l'arborescence pour savoir ce qu'elle contient déjà.

**Tout ce qui vit hors de ces deux zones vous appartient, et rbs ne le réécrit jamais** —
ni le titre, ni la section `## Notes du projet` que `rbs new` laisse vide, ni un titre que
vous ajoutez de votre côté. C'est la même règle que pour le code que rbs engendre : ce
fichier est fait pour être modifié, et les marqueurs sont la seule promesse que rbs fait
sur ce qu'il touchera.

Les zones d'un vrai projet, engendré en français, se lisent ainsi :

```text
# blog — mode d'emploi pour agents

<!-- rbs:guide 1.2.0 -->
## Le CLI d'abord
## Ce fichier
## Les commandes
## Recettes
## Architecture imposée
## Les ancres
## Ce que rbs ne couvre pas
## Vérifier avant de conclure
<!-- /rbs:guide -->

<!-- rbs:inventory -->
- rbs 1.2.0 · base postgres
- Fragments installés : aucun
- Entités engendrées : aucune
- Ancres du projet : features (src/lib.rs), routes (src/router.rs), openapi (src/openapi.rs), migration_modules (migration/src/lib.rs), migrations (migration/src/lib.rs), state_champs (src/state.rs), state_init (src/state.rs), startup (src/main.rs), seeds (src/seeds/main.rs), services (docker-compose.yml)
<!-- /rbs:inventory -->

## Notes du projet
```

## Qui écrit quoi

| Commande | Effet sur `AGENTS.md` |
|---|---|
| `rbs new` | Écrit le fichier entier : guide, inventaire, titre et une section de notes vide. |
| `rbs add <feature>` | Régénère la zone d'inventaire. |
| `rbs generate crud\|feature` | Régénère la zone d'inventaire. |
| `rbs upgrade` | Régénère le guide et l'inventaire ; recrée le fichier s'il a disparu. |
| `rbs doctor` | Ne change rien — il ne fait que constater. |
| `rbs generate job`, `rbs generate client`, `rbs migrate`, `rbs seed`, `rbs dev`, `rbs test`, `rbs routes`, `rbs openapi export` | Aucun effet. |

`upgrade` est la seule commande qui a mandat de remettre le projet en accord avec le CLI,
et c'est pourquoi elle est aussi la seule à recréer un fichier supprimé. `add` et
`generate` ne régénèrent que l'inventaire : elles connaissent la feature ou l'entité
qu'elles viennent d'installer, pas si le CLI lui-même a changé de version — cette
comparaison n'appartient qu'à `upgrade`.

`CLAUDE.md` suit une règle à lui, plus courte. `rbs new` l'écrit ; `rbs upgrade` ne le
réécrit que s'il manque — le cas de tout projet engendré avant que rbs ne l'écrive — et
aucune commande ne réécrit jamais celui qui existe, quoi qu'il contienne.

## Choisir la langue

`rbs new --lang fr|en` choisit la langue dans laquelle le mode d'emploi est écrit. Sans le
flag, rbs se rabat sur l'environnement : `LC_ALL` d'abord, puis `LANG` — une valeur qui
commence par `fr` donne le français, toute autre valeur non vide donne l'anglais, et
l'absence de valeur donne le français, la langue du dépôt rbs lui-même.

Le choix s'inscrit dans le manifeste plutôt que d'être redéduit à chaque commande :

```toml
[package.metadata.rbs]
lang = "en"
```

Sans cette clé, [`rbs add`](../cli/add.md) et [`rbs upgrade`](../cli/upgrade.md) devraient
deviner la langue du projet depuis l'environnement de celui qui les lance — réécrivant un
guide anglais en français le jour où quelqu'un de l'équipe lance la commande depuis une
locale française. La lire depuis le manifeste fait au contraire que le fichier reste dans
la langue où le projet a été créé, indépendamment de qui le touche ensuite.

## Ce que vérifie `rbs doctor`

[`rbs doctor`](../cli/doctor.md) lance un contrôle `agents` parmi les autres :

| Ce qu'il constate | Verdict |
|---|---|
| `AGENTS.md` absent | échec — `rbs upgrade` le recrée |
| La zone `rbs:guide` ou `rbs:inventory` manque | échec — le bloc à coller s'affiche |
| Le guide est d'une version différente de celle du CLI | échec — `rbs upgrade` réécrit le guide |
| L'inventaire rendu diffère de celui du disque | échec — `rbs upgrade` le recalcule |
| Une feature déclarée dans le manifeste sans son `src/<nom>/` | échec — `rbs add <nom>`, ou retirez la ligne du manifeste |
| Un répertoire de `src/` qu'aucun fragment ni aucune feature déclarée n'explique | **avertissement** |

Cette dernière ligne rend vérifiable la règle du CLI d'abord : un répertoire que rien dans
le manifeste n'explique est du code que personne n'a engendré. Il reste un avertissement
plutôt qu'un échec, et c'est voulu — écrire à la main ce que rbs ne couvre pas est
légitime et prévu, c'est même tout l'objet de la section « ce que rbs ne couvre pas » du
guide. En faire un échec rendrait `rbs doctor` rouge sur un projet parfaitement sain dès
qu'on ajoute à la main un gestionnaire de webhook ou un client HTTP externe — exactement
le genre de code que cet outil n'a pas vocation à engendrer.

Un avertissement ne change ni le code de sortie ni le verdict final : un projet qui ne
porte qu'un avertissement continue de sortir en 0 et d'être rapporté comme sain dans
l'ensemble — seul un échec véritable change cela.

```text
$ rbs doctor
  ✓ ancres        les 12 points d'insertion sont en place
  ! agents        écrit hors du CLI : webhooks
      légitime si rbs ne couvre pas ce code ; sinon, rbs generate le reprend
  ✓ relations     les modèles portent leurs ancres de relation
  ✓ .env          les 7 variables de .env.example sont renseignées
  ✓ versions      projet et rbs-core pris d'un chemin local alignés sur le CLI 1.2.0
  … base          compilation de la crate migration, peut prendre
                  une minute au premier lancement…
  ✓ base          postgres 18.6 répond sur localhost:55502
  ✓ disposition   aucun module ne mélange les deux dispositions
✓ le projet est sain
```

## Quand une zone a disparu

Supprimer un marqueur est traité comme la suppression d'une ancre de code : la commande
qui aurait dû y écrire n'écrit rien, et affiche à la place le bloc exact à coller.

```text
$ rbs add redis
[…]
attention : AGENTS.md ne porte pas la zone `rbs:inventory` — collez ce bloc pour la rétablir :

<!-- rbs:inventory -->
<!-- /rbs:inventory -->
✓ redis installée — 4 créés, 6 modifiés
```

Le reste de la commande va tout de même à son terme — une zone manquante dans un fichier
de documentation n'est jamais une raison de refuser d'installer une feature. Collez le
bloc, et la prochaine commande qui touche `AGENTS.md` le remplira à nouveau.

Supprimer le fichier entier va plus loin encore : `rbs add` et `rbs generate` aboutissent
sans même le mentionner. La seule commande qui le repose est
[`rbs upgrade`](../cli/upgrade.md), puisque remettre le projet en accord avec le CLI
courant est précisément son rôle :

```text
$ rbs upgrade
rbs 1.2.0 → 1.2.0

plan pour /private/tmp/rbs-demo/blog2

  · Cargo.toml   inchangé
  + AGENTS.md    créé

  1 à créer, 1 inchangé
✓ manifeste aligné sur rbs 1.2.0
```

## Lire un plan en JSON

Le plan qu'une commande affiche avant d'écrire est fait pour un humain : couleurs, puces, un
décompte en bas. `rbs add`, `rbs generate crud`, `feature`, `client` et `job`, et
`rbs upgrade` prennent `--json`, et la sortie standard porte alors
un seul document JSON à la place — le plan, ou l'erreur. C'est lui qu'un agent doit lire.

```text
rbs add cors --dry-run --json
```

`--json` et `--dry-run` sont indépendants. Avec `--dry-run`, rien n'est écrit et `applique`
vaut `false` ; sans lui, le plan s'applique d'abord et le document annonce
`applique: true`. Une feature déjà installée, ou un projet déjà à jour, rend un document
dont les `actions` sont vides et dont `applique` vaut `false` : rien n'a été écrit.

Ce qu'un humain aurait encore besoin de voir part sur la sortie d'erreur : une zone
d'`AGENTS.md` manquante et son bloc, l'avertissement de rustfmt, une référence requise, les
CRUD engendrés qu'`auth` laisse ouverts, les notes de migration. Les lignes de succès et les
conseils se taisent. La sortie standard reste un seul document du premier octet au
dernier, même pendant que `generate client` compile le projet.

Sur le projet que crée `rbs new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo`,
`rbs add cors --dry-run --json` rend ceci — tronqué : le document réel porte douze actions et
le contenu complet de chaque fichier qu'il crée, et la racine est coupée :

```json
{
  "commande": "add",
  "racine": "/…/demo",
  "applique": false,
  "actions": [
    {
      "chemin": "src/modules/cors/mod.rs",
      "statut": "a_faire",
      "effet": {
        "type": "creer",
        "contenu": "use std::time::Duration;\n\nuse axum::http::{HeaderName, HeaderValue, Method};\n…"
      }
    },
    {
      "chemin": "src/router.rs",
      "statut": "a_faire",
      "effet": {
        "type": "inserer",
        "ancre": "layers",
        "lignes": [
          ".layer(crate::modules::cors::layer())"
        ]
      }
    },
    {
      "chemin": "Cargo.toml",
      "statut": "a_faire",
      "effet": {
        "type": "patcher_toml",
        "patch": {
          "type": "ajouter_dependance",
          "nom": "tower-http",
          "version": "0.7",
          "features": [
            "cors"
          ],
          "features_par_defaut": true
        }
      }
    }
  ],
  "sautees": [],
  "fichiers": {
    "crees": 4,
    "modifies": 5
  }
}
```

Le document énumère des actions, non des fichiers : un fichier que deux actions touchent
paraît deux fois — `src/modules/mod.rs` est créé, puis reçoit `pub mod cors;` dans son
ancre. `fichiers` compte les fichiers, une fois chacun : créés quand ils n'existaient pas,
modifiés sinon, les inchangés laissés dehors. `statut` vaut `a_faire` quand l'action change
quelque chose, `deja_fait` quand le projet la porte déjà, `conflit` quand le fichier existe
avec un contenu que rbs n'a pas écrit — seul `--force` l'écrase. `sautees` énumère les
insertions que rbs vous laisse écrire, chacune avec son `bloc` et sa `cause`. `cause.type`
vaut `fichier_absent` quand manque le fichier optionnel qui porte l'ancre — un projet sans
compose, par exemple ; `ancre_absente` quand le fichier est là sans l'ancre — un projet
engendré avant qu'elle n'existe —, avec `reparable` vrai quand `rbs doctor --fix` sait la
reposer ; `entrainee` quand l'insertion nomme ce qu'une autre, sautée elle aussi, devait
déclarer, dont `par` donne l'ancre : c'est celle-là qu'on reporte d'abord.

Chaque `effet` a une forme par `type` :

| `type` | Champs | Effet |
|---|---|---|
| `creer` | `contenu` | Écrit un fichier dont le contenu est connu en entier — tout est dans `contenu`. |
| `inserer` | `ancre`, `lignes` | Ajoute des lignes dans une ancre, avant sa balise fermante. |
| `reposer_ancre` | `ancre` | Repose une ancre disparue sous sa ligne d'accroche. |
| `patcher_toml` | `patch` | Modifie `Cargo.toml`, mise en forme préservée. `patch.type` vaut `inscrire_feature` (`feature`), `ajouter_dependance` (`nom`, `version`, `features`, `features_par_defaut`), `ajouter_feature_a_dependance` (`dependance`, `feature`) ou `aligner_sur_version` (`dependance`, `version`). |
| `ajouter_section` | `section`, `contenu` | Ajoute une section à un document TOML qui n'est pas le manifeste. |
| `ajouter_variable` | `cle`, `valeur`, `commentaire` | Ajoute une variable à un fichier d'environnement ; `commentaire` vaut `null` s'il n'y en a pas. |
| `remplacer_zone` | `zone`, `contenu` | Remplace le corps d'une zone d'`AGENTS.md`, le reste du fichier intact. |

Une ligne de commande mal formée n'est pas un document : un drapeau inconnu est refusé par
l'analyseur d'arguments, en texte sur la sortie d'erreur, avec le code de sortie 2.

### Codes d'erreur

Sous `--json`, un refus est un seul document sur la sortie standard, et le code de sortie
reste 1. rbs n'ajoute rien sur la sortie d'erreur pour le refus lui-même ; ce qui l'a
précédé y reste — un avertissement affiché plus tôt dans l'exécution, ou la compilation du
projet sous `generate client`. Ici, l'ancre `layers` a été retirée de
`src/router.rs` avant `rbs add cors --json --force` :

```json
{
  "erreur": {
    "code": "ancre_absente",
    "message": "ancre // <rbs:layers> introuvable dans src/router.rs",
    "remede": "dans src/router.rs :\n// <rbs:layers>\n// </rbs:layers>",
    "bloc": "// <rbs:layers>\n// </rbs:layers>"
  }
}
```

`message` est la phrase qu'un humain lirait, et peut changer d'une version à l'autre ;
`code` ne change pas, et c'est sur lui qu'un script décide. `remede` et `bloc` valent `null`
quand il n'y a rien à faire ou rien à coller ; dès que `bloc` n'est pas `null`, `remede` dit
où il va. Un code partagé par plusieurs commandes a le même sens dans toutes.

| Code | Commandes | Sens |
|---|---|---|
| `pas_un_projet` | toutes | Aucun `Cargo.toml` portant `[package.metadata.rbs]` au-dessus du répertoire courant. |
| `arbre_sale` | toutes | Le working tree Git porte des modifications non commitées. Commitez, ou relancez avec `--force`. |
| `fichier_inaccessible` | toutes | Un fichier du projet ou d'une template n'a pu être lu ou écrit. |
| `manifeste_illisible` | toutes | Le `Cargo.toml` du projet n'a pu être lu ou patché. |
| `ancre_absente` | `add`, `generate`, `upgrade` | Une ancre manque à son fichier. `bloc` porte les deux balises à coller, `remede` nomme le fichier. |
| `ancre_mal_placee` | `add`, `generate`, `upgrade` | Une ancre est placée sous la ligne qu'elle doit précéder. `bloc` est le bloc à remonter, `remede` nomme la ligne. |
| `zone_absente` | `add`, `generate`, `upgrade` | Une zone d'`AGENTS.md` manque. `bloc` porte ses marqueurs. |
| `fichier_absent` | `add`, `generate`, `upgrade` | Le fichier qui doit porter une ancre n'existe pas. |
| `manifeste_absent` | `add`, `generate`, `upgrade` | Le `Cargo.toml` visé par une modification n'existe pas. |
| `toml_invalide` | `add`, `generate`, `upgrade` | Un document TOML du projet ne s'analyse pas. |
| `conflit` | `add`, `generate`, `generate client`, `upgrade` | Le plan écraserait des fichiers que rbs n'a pas écrits. Relancez avec `--force` pour les écraser. |
| `ecriture_impossible` | `add`, `generate`, `generate client`, `upgrade` | Une écriture a échoué ; ce que le plan avait déjà écrit a été défait. |
| `plan_incoherent` | `add`, `generate`, `generate client`, `upgrade` | Deux actions prétendent écrire le même fichier de bout en bout — un défaut de rbs, à signaler. |
| `feature_inconnue` | `add` | Aucun fragment ne porte ce nom. |
| `fragment_sans_manifeste` | `add` | Le fragment n'a pas de `feature.toml`. |
| `fragment_invalide` | `add` | Le `feature.toml` du fragment est invalide. |
| `template_absente` | `add` | Le manifeste du fragment déclare une template que le fragment ne porte pas. |
| `ancre_inconnue` | `add` | Le manifeste du fragment vise une ancre que rbs ne connaît pas. |
| `rendu_impossible` | `add`, `generate` | Une template ne se rend pas. |
| `env_illisible` | `add` | Le `.env` du projet est absent, illisible, ou muet sur la base. |
| `url_indecomposable` | `add` | L'URL de la base ne se décompose pas en utilisateur, mot de passe et hôte. |
| `nom_invalide` | `generate` | Le nom de la feature, ou celui donné à `--singular`, est inutilisable. |
| `champs_invalides` | `generate` | `--fields` ne s'analyse pas. |
| `feature_deja_presente` | `generate` | Le répertoire de la feature existe déjà. |
| `relation_invalide` | `generate` | Une référence ne se résout pas : cible introuvable, ou deux relations réclamant la même variante. |
| `migration_absente` | `generate` | Une entité référencée n'a pas de migration dans le projet. |
| `homonyme` | `generate` | Le modèle cible porte déjà, sous ce nom, une variante visant une autre entité. |
| `feature_absente` | `generate` | `--has-many` répare une feature qui doit d'abord exister. |
| `role_sans_auth` | `generate` | `--role` exige la feature `auth`. |
| `role_inconnu` | `generate` | Le rôle n'est pas une variante de `src/auth/model.rs`. |
| `upload_sans_storage` | `generate` | `--with-upload` exige la feature `storage`. |
| `storage_hors_modules` | `generate` | `storage` a été installée avant la 1.3.0, sous `src/storage/`. |
| `colonne_reservee` | `generate` | `--soft-delete` pose lui-même `deleted_at` : retirez-la de `--fields`. |
| `enfant_sans_cle` | `generate` | L'enfant nommé par `--has-many` ne porte aucune colonne référençant cette table. |
| `nom_reserve` | `generate job` | Le nom est pris : un module de la file, `jobs` lui-même, ou une crate que nomme le code de la file. |
| `disposition_anterieure` | `generate job` | `jobs` ou `scheduler` a été installée avant la 1.3.0, hors de `src/modules/` ; le message nomme le déplacement à faire. |
| `jobs_absent` | `generate job` | Le projet n'a pas la feature `jobs` ; `remede` vaut `rbs add jobs`. |
| `scheduler_absent` | `generate job` | `--every` exige la feature `scheduler` ; `remede` vaut `rbs add scheduler`. |
| `cron_invalide` | `generate job` | L'expression de `--every` ne passerait pas le démarrage du projet. |
| `module_deja_declare` | `generate job` | `src/modules/jobs/mod.rs` déclare déjà le module hors de son ancre. |
| `fichier_etranger` | `generate job` | Le fichier du job existe et ne définit pas ce job. |
| `kind_pris` | `generate job` | Un autre job porte déjà ce `KIND`. |
| `echeance_existante` | `generate job` | Le job a déjà une échéance, sous une autre expression. |
| `sans_bibliotheque` | `generate client` | Le projet n'a pas de `src/lib.rs`. |
| `sans_binaire_openapi` | `generate client` | Le projet n'a pas de `src/bin/openapi.rs` ; `remede` donne le fichier à créer. |
| `cargo_introuvable` | `generate client` | `cargo` n'a pas pu être lancé. |
| `projet_ne_compile_pas` | `generate client` | `cargo run --bin openapi` a échoué. |
| `document_illisible` | `generate client` | Ce que le binaire a imprimé n'est pas un document OpenAPI. |
| `client_irrendable` | `generate client` | Le document ne se traduit pas en TypeScript. |
| `cli_anterieur` | `upgrade` | Le projet a été engendré par un rbs plus récent que ce CLI. |
| `agents_illisible` | `upgrade` | `AGENTS.md` n'a pas pu être rendu. |
