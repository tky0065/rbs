# AGENTS.md dans le projet engendré · Spécification de design

Date : 2026-08-30
Statut : validé, prêt pour le plan d'implémentation

## 1. Objectif

Un agent lâché dans un projet rbs ne sait pas que le projet est un projet rbs. Il voit
des fichiers Rust, il écrit des fichiers Rust : il recrée à la main les six fichiers d'une
feature, oublie la migration, ignore les ancres, casse la dépendance unidirectionnelle.
Le CLI existe, il ne s'en sert pas — il ne sait pas qu'il existe.

`rbs new` pose donc à la racine du projet un **`AGENTS.md`** : le mode d'emploi de rbs
écrit pour un agent, et non pour un humain. Il énonce la règle — *toute fonctionnalité que
rbs couvre passe par le CLI* — donne les commandes, dit quoi faire pour ce que rbs ne
couvre pas, et nomme l'état réel du projet. `rbs doctor` rend cette règle vérifiable au
lieu de la laisser déclarative.

Le format `AGENTS.md` est retenu pour sa neutralité : Claude Code, Codex, Cursor et Copilot
le lisent tous. Aucun fichier propre à un outil n'est engendré.

## 2. Décisions arbitrées

| # | Décision | Retenu | Écarté |
|---|---|---|---|
| A1 | Fichiers engendrés | Un seul `AGENTS.md`, format neutre | `CLAUDE.md` pointeur · répertoire `.claude/` · flag de choix |
| A2 | Où vit le savoir | Tout dans `AGENTS.md` | Fiches `docs/agents/*.md` · skills `.claude/skills/` · skill publiée à part |
| A3 | Tenue à jour | Zones délimitées, contenu **régénéré en entier** | Insertion incrémentale par ancres · fichier entièrement dérivé |
| A4 | Portée de la mise à jour | `new`, `add`, `generate`, `upgrade` écrivent ; `doctor` constate | Écrit une seule fois à la création |
| A5 | CLI-first | Règle écrite **et** contrôlée par `doctor` | Clé déclarative seule · règle écrite seule |
| A6 | Langue | Deux templates, `fr` et `en`, choisies par `--lang` | Anglais seul · français seul |
| A7 | Nom des marqueurs | En anglais dans les deux langues : `rbs:guide`, `rbs:inventory` | Marqueurs traduits |

**Conséquence assumée de A3** : rbs ne possède que ce qui est entre ses marqueurs. Tout ce
que l'utilisateur écrit hors des zones lui appartient et n'est jamais réécrit. C'est la
même règle que pour le code engendré : *ce fichier est fait pour être modifié*.

**Conséquence assumée de A6** : la clé `lang` entre dans `[package.metadata.rbs]`. Sans
elle, `add` et `upgrade` ne sauraient pas dans quelle langue réécrire un projet existant.

## 3. Le fichier

`AGENTS.md`, à la racine du projet, en trois zones :

```markdown
# <projet> — mode d'emploi pour agents

<!-- rbs:guide 1.2.0 -->
## Le CLI d'abord
## Les commandes
## Recettes
## Architecture imposée
## Les ancres
## Ce que rbs ne couvre pas
## Vérifier avant de conclure
<!-- /rbs:guide -->

<!-- rbs:inventory -->
...
<!-- /rbs:inventory -->

## Notes du projet
```

### 3.1 La zone `rbs:guide`

Propriété de rbs. Le marqueur d'ouverture porte la version du CLI qui l'a écrite — c'est
ce numéro que `upgrade` compare et que `doctor` lit.

**Le CLI d'abord.** La règle en une phrase, et sa raison : le CLI pose la migration, câble
les ancres, inscrit la feature dans les métadonnées et respecte l'architecture. Un agent
qui écrit ces fichiers à la main produit un projet que `rbs doctor` déclarera cassé.

**Les commandes.** Un tableau : la commande, ce qu'elle fait, ce qu'elle dispense
d'écrire. Il couvre `new`, `add`, `generate crud`, `generate feature`, `migrate up|down|status|new`,
`seed`, `dev`, `doctor`, `upgrade`.

**Recettes.** Une ligne de commande par intention courante, avec le résultat attendu :
une entité et son CRUD (`rbs generate crud posts --fields "title:string,body:text"`), une
relation (`--fields "author:references:users"`, `--has-many`), une feature sans champs,
l'authentification (`rbs add auth`), une migration à écrire soi-même (`rbs migrate new`).

**Architecture imposée.** `controller → service → repository → model`, unidirectionnel.
Un service ne touche jamais `DatabaseConnection` ; un controller ne construit jamais de
requête SeaORM. Une feature au-delà de ~200 lignes se scinde.

**Les ancres.** `<rbs:features>`, `<rbs:routes>`, `<rbs:openapi>`, `<rbs:migrations>`,
`<rbs:services>`, `<rbs:seeds>`, `<rbs:startup>`, et les ancres de relation
`<rbs:relations>` et `<rbs:related>` : ce que le CLI alimente et qu'on ne réordonne pas à
la main. La liste est tirée du moteur d'ancres, non recopiée : un test la contrôle (§8).

**Ce que rbs ne couvre pas.** La partie qui évite le pire. Pour un endpoint qui n'est pas
un CRUD, un client HTTP externe, une règle métier : où poser le code (dans la feature
existante, jamais dans un module parallèle), quelles couches respecter, quelle ancre
alimenter à la main, et le fait que ce code est légitime — `rbs doctor` le signalera comme
écrit à la main, ce qui est un constat et non une erreur.

**Vérifier avant de conclure.** `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace`, `rbs doctor`. Avec la mention que
les tests demandent la base démarrée.

### 3.2 La zone `rbs:inventory`

Propriété de rbs, recalculée intégralement à chaque écriture. Elle est courte et factuelle,
pour éviter à l'agent d'explorer le projet afin de savoir ce qu'il contient :

- version de rbs qui a produit le projet et moteur de base ;
- fragments installés (`auth`, `redis`, `mail`, `storage`, `jobs`, `docker`, `ci`) ;
- entités engendrées, distinguées des fragments en croisant `metadata.features` avec la
  liste des fragments embarqués ;
- ancres présentes dans le projet, avec leur fichier.

### 3.3 Le reste du fichier

Le titre, la zone « Notes du projet » et tout ce que l'utilisateur ajoute. Écrit une fois
par `rbs new`, jamais relu ni réécrit.

## 4. Rendu

Un module `crates/rbs-cli/src/agents.rs` porte : le rendu du guide, le calcul de
l'inventaire, la localisation d'une zone dans un fichier existant, le remplacement d'une
zone, la comparaison d'une zone rendue à une zone présente.

Deux templates, `crates/rbs-cli/templates/agents/fr.md.jinja` et `en.md.jinja`. Elles vivent
hors de `templates/project/`, qui est rendue en bloc par `rbs new` : ce fichier-ci est
aussi rendu par `add`, `generate` et `upgrade`.

La langue vient de `rbs new --lang fr|en`, dont le défaut se déduit de `LANG` / `LC_ALL`
(`fr*` → français, tout le reste → anglais) et s'inscrit dans `[package.metadata.rbs]`.
Un projet antérieur, sans la clé, est traité comme français — la langue du dépôt.

Toute écriture passe par le `plan` existant : lire → planifier → vérifier → afficher →
appliquer, avec restauration en cas d'échec partiel.

## 5. Cycle de vie

| Commande | Effet |
|---|---|
| `new` | Écrit le fichier entier : guide, inventaire, titre et section de notes vide. |
| `add <feature>` | Régénère la zone inventaire. |
| `generate crud\|feature` | Régénère la zone inventaire. |
| `upgrade` | Régénère guide et inventaire ; recrée le fichier s'il manque. |
| `doctor` | Ne modifie rien. |
| `migrate`, `seed`, `dev` | Aucun effet. |

**Zone absente.** La règle des ancres s'applique telle quelle : le CLI n'écrit rien et
affiche le bloc rendu, à coller. Un utilisateur qui supprime une zone garde son fichier.

**Fichier absent.** Même règle pour `add` et `generate`. `upgrade` fait exception : sa
mission est d'aligner le projet sur la version du CLI, il recrée donc le fichier.

**Idempotence.** Deux exécutions successives d'une même commande laissent un fichier
identique. Elle est acquise par construction : la zone est régénérée, jamais complétée.
C'est la raison d'être de A3 — l'insertion incrémentale a coûté au dépôt une série de
correctifs sur le dédoublonnage et les insertions concurrentes, que ce mécanisme n'a pas
lieu de rejouer.

**Contrat d'`upgrade`.** La commande n'écrivait jusqu'ici que dans `Cargo.toml`, et son
doc-comment le déclare. Elle écrira désormais aussi dans les zones réservées d'`AGENTS.md`.
Le doc-comment est réécrit en conséquence : laisser cette affirmation en place la
rendrait fausse.

## 6. `doctor` et la règle CLI-first

Un module `crates/rbs-cli/src/doctor/agents.rs`, sur le modèle des contrôles existants :
indépendants, tous exécutés, chacun assorti de son remède.

| Constat | Verdict | Remède |
|---|---|---|
| `AGENTS.md` absent | échec | `rbs upgrade` |
| Zone `rbs:guide` ou `rbs:inventory` absente | échec | le bloc rendu, à coller |
| Version du guide ≠ version du CLI | échec | `rbs upgrade` |
| Inventaire rendu ≠ inventaire présent | échec | `rbs upgrade` |
| `src/<nom>/` ni fragment connu ni inscrit dans `metadata.features` | avertissement | « écrit à la main : légitime si rbs ne le couvre pas, sinon `rbs generate` » |
| Feature inscrite dans `metadata.features` sans répertoire `src/<nom>/` | échec | `rbs add <feature>`, ou retirer la ligne des métadonnées |

La dernière ligne est le contrôle CLI-first proprement dit : elle nomme le code qui n'est
pas passé par le CLI. Elle ne peut pas être un échec — le guide autorise explicitement
d'écrire à la main ce que rbs ne couvre pas, et faire échouer `doctor` là-dessus rendrait
le contrôle inutilisable en CI.

**Élargissement nécessaire** : `doctor::State` ne connaît que `Bon` et `Echec`. Un
troisième état, l'avertissement, s'y ajoute : affiché dans le rapport, sans effet sur
`Report::succeeded` ni sur le code de sortie. `doctor/render.rs` lui donne sa marque
visuelle.

La dernière ligne du tableau est le cas symétrique, et il est bien un échec : aucun
contrôle existant ne le couvre — `doctor/anchors.rs` ne regarde que les ancres, et les
contrôles par feature (`auth`, `redis`, `mail`, `storage`, `jobs`) ne vérifient que leur
configuration, jamais la présence de leurs fichiers. Un projet qui déclare `auth` sans
porter `src/auth/` ne compile pas, et `doctor` reste vert aujourd'hui.

## 7. Impacts sur le code existant

| Fichier | Changement |
|---|---|
| `src/agents.rs` | Nouveau : rendu, inventaire, zones. |
| `templates/agents/{fr,en}.md.jinja` | Nouveaux. |
| `templates/project/Cargo.toml.jinja` | Clé `lang` dans `[package.metadata.rbs]`. |
| `src/metadata.rs` | `Metadata::lang`, lecture et défaut pour les projets antérieurs. |
| `src/cli.rs` | `new --lang fr\|en`. |
| `src/new.rs` | Écrit `AGENTS.md`. |
| `src/add/mod.rs` | Régénère l'inventaire. |
| `src/generate/command.rs` | Régénère l'inventaire. |
| `src/upgrade.rs` | Régénère les deux zones ; doc-comment réécrit. |
| `src/doctor/mod.rs` | Troisième état ; branchement du contrôle. |
| `src/doctor/render.rs` | Rendu de l'avertissement. |
| `src/doctor/agents.rs` | Nouveau. |

## 8. Tests

**Le guide ne ment pas.** Le test qui compte le plus : un guide périmé n'induit pas un
développeur en erreur, il induit tous les agents en erreur. Un test croise
`Cli::command().get_subcommands()` avec le contenu des deux templates — chaque sous-commande
y est nommée. Le précédent existe déjà dans `cli.rs`, où
`the_add_help_names_every_installable_feature` a été écrit parce qu'« `auth` a été livrée
sans que cette phrase la mentionne ». Un deuxième test croise de la même façon la liste
des ancres du moteur avec celle que le guide énumère. Un troisième contrôle la parité
fr/en sur les titres de sections.

**`agents.rs`** : rendu dans les deux langues ; inventaire calculé depuis un manifeste ;
le hors-zone survit au remplacement ; deux rendus successifs donnent un fichier identique ;
zone absente → rien écrit et bloc affiché.

**`new`** : le projet porte `AGENTS.md`, dans la langue demandée, nommant le moteur de base
et les features de `--with`.

**`add` et `generate`** : après la commande, l'inventaire nomme la feature ou l'entité.

**`upgrade`** : un guide de version antérieure est remplacé ; un fichier supprimé est
recréé ; les notes de l'utilisateur survivent.

**`doctor/agents`** : un test par ligne du tableau du §6, y compris le fait que
l'avertissement ne change pas le code de sortie.

**Intégration `assert_cmd`** : `rbs new` puis assertions sur le fichier. Sans compilation
ni Docker : `AGENTS.md` ne change rien au build du projet engendré.

## 9. Documentation

Bilingue dans le même commit, selon la règle du dépôt :

- une page « Développer avec un agent » dans `docs/docs/guides/` et son homologue
  `docs/i18n/` : ce qu'est `AGENTS.md`, ce qu'il contient, comment le modifier sans le
  perdre, ce que `doctor` vérifie ;
- `docs/docs/cli/` : `new --lang`, le nouveau contrat d'`upgrade`, le nouveau contrôle de
  `doctor`.

## 10. Suivi

Un lot de tâches est ajouté à `TODO.md`, avec ses critères de validation, exécutable par le
skill `rbs-task`. `ROADMAP.md` reçoit un jalon **v1.2 — Agents**, dont le critère de sortie
est qu'un agent partant d'un projet fraîchement engendré produise une feature complète en
passant par le CLI, `rbs doctor` restant vert.

## 11. Hors périmètre

Explicitement, et non « plus tard » :

- `CLAUDE.md`, `.cursorrules` ou tout fichier propre à un outil ;
- des skills ou commandes engendrées dans le projet (`.claude/skills/`) ;
- un `doctor --fix` : le remède existe et s'appelle `rbs upgrade` ;
- un serveur MCP rbs ;
- la traduction du guide au-delà du français et de l'anglais.
