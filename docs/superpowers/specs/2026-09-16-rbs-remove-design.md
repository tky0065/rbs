# `rbs remove <fragment>` — design

**2026-09-16.** Tâche 72 d'`IMPROVE.md` (P3, Hard). Autorité sur l'architecture de la
commande ; complète §4.4 de `2026-08-25-rbs-design.md`, qui ne connaît aujourd'hui qu'un
seul chemin de retour — `git checkout`, « plus fiable qu'un système de backup maison ».

`remove` ne remplace pas ce filet, il en ajoute un second, plus étroit et plus sûr : là où
`git checkout` défait *tout* ce qui n'est pas commité, `remove` défait *un fragment*, y
compris commité, et refuse plutôt que de deviner.

## 1. Objet et portée

Désinstaller un fragment posé par `rbs add` : ses fichiers, ses lignes d'ancres, sa
migration, ses dépendances, ses sections de configuration et son inscription dans
`[package.metadata.rbs]`.

**Hors périmètre, délibérément :**

- **Les CRUD engendrés.** `[package.metadata.rbs] features` mêle fragments et modules
  engendrés — `["health", "mail", "rate-limit", "auth", "posts"]` dans `blog-auth`. `remove`
  distingue les deux par l'existence d'un `feature.toml` embarqué et refuse `posts` en
  nommant les treize fragments valides. Retirer un CRUD est une autre commande, avec
  d'autres questions : rien n'enregistre ses options de génération.
- **Plusieurs fragments en un appel.** La commande en prend un, comme `rbs add`.
- **Toute conversation avec la base.** Voir §3.2.

## 2. Surface

```
rbs remove <feature> [--force] [--dry-run] [--json] [--template-dir <dir>]
```

Calquée trait pour trait sur `Commands::Add` (`cli.rs:93`), jusqu'aux noms des drapeaux.
Séquence de §4.4 sans exception : lire → planifier → vérifier → afficher → appliquer, avec
restauration tout-ou-rien.

```rust
pub(crate) struct Options {
    pub feature: String,
    pub directory: PathBuf,
    pub force: bool,
    pub template_dir: Option<PathBuf>,
}

pub(crate) struct Planned {
    pub plan: plan::Plan,
    pub description: String,
    pub migration: bool,
    pub laissees: Vec<String>,
    pub deja_absente: bool,
}
```

`laissees` porte ce que la commande a sciemment laissé en place et que le rapport doit
nommer : variables d'environnement, dépendances partagées.

## 3. Les contrats

### 3.1 Quatre refus, tous avant la moindre écriture

1. **Fragment non installé** — `[package.metadata.rbs] features` ne l'inscrit pas. Sortie
   nette, symétrique du `deja_installee` d'`add`.
2. **Nom qui n'est pas un fragment** — un CRUD, un inconnu. L'erreur nomme les treize
   fragments valides.
3. **Un dépendant installé l'exige** — `auth` déclare `requires = ["rate-limit", "mail"]`,
   `webhooks` déclare `["jobs", "auth"]`, `scheduler` déclare `["jobs"]`. `rbs remove mail`
   sur un projet portant `auth` refuse en nommant `auth`. L'utilisateur enchaîne les
   commandes dans l'ordre qu'il choisit.
4. **Divergence** — un fichier du fragment existe et diffère de ce qu'un rendu neuf
   produirait. L'erreur liste les fichiers ; `--force` passe outre.

Le quatrième est le garde-fou central, et il n'est pas une réutilisation gratuite :
`combined_status` ne rend jamais `Conflit`, qui naît dans `Builder::create`
(`plan/mod.rs:310`) d'un fichier présent que le plan n'a pas produit. `remove` a besoin du
test inverse — *présent et différent du rendu* — porté par une méthode neuve,
`Builder::supprimer(path, rendu_attendu)`.

Rejouer ce rendu exige le contexte qu'`add` construit : nom du paquet, `crate_name`,
moteur, langue, URL du `.env`. Le contenu d'une migration, lui, ne porte pas son
horodatage — `generate::migration::render` ne le passe qu'au nom du module — donc elle se
compare comme les autres fichiers.

### 3.2 Aucune conversation avec la base

`migrate status` lance un sous-processus contre la base (`migrate/mod.rs:118`) : savoir
si une migration est appliquée exigerait une base joignable *et* la crate `migration`
compilée, pour une commande qui ne touche par ailleurs qu'à des fichiers. `remove` retire
la migration et ses deux lignes d'ancres sans rien demander à personne, et **le rapport
dit en toutes lettres** que le schéma garde ses tables et que `rbs migrate down` devait
passer avant.

Conséquence assumée : sur un projet où la migration avait tourné, `seaql_migrations` citera
une migration sans fichier. C'est le prix d'une commande qui marche hors ligne.

### 3.3 Le compilateur est l'oracle des références

Un CRUD peut dépendre d'un fragment : `generate crud --with-upload` écrit
`use crate::modules::storage::{Object, Storage, StorageError};` dans son `service.rs`, et
`--role` écrit `use crate::auth::guard::RequireRole;` dans son `controller.rs`. Rien
n'enregistre ces options.

`remove` **ne cherche pas** ces références. Un `cargo build` les nomme toutes, exactement,
là où un grep les approxime ; le rapport prévient que le projet peut ne plus compiler et
invite à construire. C'est le seul endroit où le diagnostic arrive après l'écriture, et
c'est délibéré : l'alternative était une heuristique textuelle à maintenir, moins fiable
que rustc.

### 3.4 Ce qui n'est jamais retiré

- **Une dépendance qu'un autre fragment installé déclare encore.** `async-trait` est
  déclarée par `jobs`, `storage` et `webhooks` ; `thiserror` par `cors` et `storage`. Un
  inverse naïf de `[[dependencies]]` casserait la compilation.
- **Une feature cargo qu'un autre fragment installé demande encore.** `[cargo.tokio]` est
  déclarée par huit fragments, `[cargo.sea-orm]` par trois, `[cargo.rbs-core]` par trois.
  Même règle d'union.
- **Les dépendances du squelette**, *toutes*, et la liste se **dérive** de
  `templates/project/Cargo.toml.jinja` plutôt que de s'écrire à la main : un fragment leur
  ajoute des features, il ne les apporte pas. Le squelette en déclare treize, et deux
  fragments en redéclarent une pour y ajouter un flag — `tower-http` par `cors`,
  `serde_json` par `redis`. Une liste écrite à la main les avait manqués, et
  `rbs remove cors` vidait `tower-http` d'un manifeste dont `router.rs` dépend
  inconditionnellement : le projet ne compilait plus. Une liste dérivée reste juste le jour
  où le squelette gagne une dépendance ; une liste écrite à la main rouvre le trou en
  silence.
- **Les fichiers déclarés `if_absent`.** Le fragment ne les pose que s'ils manquent, et
  désavoue donc leur paternité quand ils préexistent : le retrait n'a aucun moyen de savoir
  lequel des deux cas s'est produit. Ils sont signalés, jamais supprimés.
- **Les variables d'environnement.** `RBS_AUTH__SECRET` est tirée au hasard à
  l'installation, et `.env` est gitignoré : la supprimer est la seule écriture de `remove`
  qu'aucun `git checkout` ne répare. La garder a de plus une vertu — un `rbs add auth`
  ultérieur retrouve le secret au lieu d'en tirer un neuf, qui invaliderait tous les jetons
  en circulation. Elle reste dans `.env` comme dans `.env.example`, et le rapport la nomme.

## 4. L'inverse, section par section du `feature.toml`

| Section | Inverse |
|---|---|
| `[[files]]` | suppression, après le contrôle de divergence de §3.1 — **sauf `if_absent`, jamais supprimé** (voir ci-dessous) |
| `[migration]` | `migration/src/m*_{name}.rs`, retrouvé **par suffixe** — l'horodatage n'est mémorisé nulle part — et les deux lignes de `mount::for_migration` |
| `[[anchors]]` | retrait des lignes exactes, `services` du compose compris |
| `[[dependencies]]` | retirée seulement sous la règle d'union de §3.4 |
| `[cargo.X]` | features retirées sous la même règle ; la dépendance elle-même reste |
| `[[config]]` | section retirée de `config/default.toml` — une par fragment, sans collision |
| `[[env]]` | **laissée**, et signalée |
| `requires` | ne sert qu'au refus 3 |
| métadonnées | `PatchToml::RetirerFeature` |

**Un fichier déclaré `if_absent` n'est jamais supprimé.** Il est signalé, comme les
variables d'environnement. La raison est que `if_absent` signifie « ne le pose que s'il
manque » : le fragment y **désavoue explicitement la paternité** du fichier quand celui-ci
préexiste, et le retrait n'a aucun moyen de savoir lequel des deux cas s'est produit. Le
contrôle de divergence ne le sauve pas : `templates/features/docker/config/production.toml.jinja`
et `templates/project/config/production.toml.jinja` sont octet pour octet identiques et sans
la moindre expression Jinja, si bien que le rendu coïncide avec le disque et qu'un
`rbs remove docker` effacerait sans `--force` un fichier écrit par le squelette. Le second
`if_absent`, `docker-compose.yml`, est plus grave encore : `mail` et `redis` y insèrent
leurs services par l'ancre `services`, et le supprimer emporterait le travail d'autres
fragments encore installés.

Les deux lignes de la migration sont littérales et connues : `mod {module};` dans
`<rbs:migration_modules>`, `Box::new({module}::Migration),` dans `<rbs:migrations>`
(`generate/mount.rs:68-79`). Le retrait les apparie telles quelles.

Une ancre optionnelle dont le fichier porteur manque — `services` sur un projet sans
compose — n'est pas une erreur : il n'y a rien à retirer.

## 5. La bascule

`File.after` devient `Option<String>` : `None` signifie *ce fichier n'existera plus*.
C'est la décision structurante, et elle est prise en une fois plutôt qu'étalée.

- `Log::write` (`plan/application.rs:90`) acquiert sa branche `None => fs::remove_file`,
  en notant `before` comme il le fait déjà ; `Log::undo` sait restaurer depuis la ligne 132,
  la restauration ayant toujours eu besoin de supprimer.
- `combined_status` rend `DejaFait` quand `origin` et `after` valent tous deux `None` :
  supprimer un fichier déjà absent ne fait rien, et **l'idempotence tombe gratuitement**.
- La porte des conflits d'`apply`, le `--dry-run`, la restauration tout-ou-rien et
  l'affichage traversent la bascule sans être élargis : ils travaillent tous sur `File`.

`plan/render.rs` affiche le plan **depuis `files()` et non depuis `actions()`** — « un
fichier par ligne, et non une action par ligne ». La bascule suffit donc à faire afficher
les suppressions ; seul `plan/json.rs`, qui sérialise les actions, reçoit les variantes
neuves.

Vocabulaire ajouté, chaque nom à côté de sa jumelle :

```rust
Effect::Supprimer
Effect::RetirerLignes { anchor: Anchor, lines: Vec<String> }
Effect::RetirerSection { section: String }

PatchToml::RetirerFeature(String)
PatchToml::RetirerDependance(String)
PatchToml::RetirerFeatureADependance { dependency: String, feature: String }
```

Fonctions neuves, chacune posée contre celle qu'elle inverse :

| Neuve | Jumelle |
|---|---|
| `anchors::retire` — opère sur `body` | `anchors::insert` |
| `metadata::remove_feature` | `metadata::record_feature` |
| `metadata::remove_dependency` | `metadata::add_dependency` |
| `metadata::remove_feature_from_dependency` | `metadata::add_feature_to_dependency` |
| `plan::text::remove_section` | `plan::text::add_section` |
| `plan::Builder::supprimer` | `plan::Builder::create` |

Coût de la bascule, **mesuré après exécution** et non estimé : neuf fichiers, 228 lignes
ajoutées et 103 retirées. Les quatre de `plan/` portent le gros du changement, mais cinq
autres consomment `File.after` et ont dû suivre — `add/mod.rs`, `add/installation.rs`,
`generate/command.rs`, `generate/job.rs` et `upgrade.rs`.

L'estimation initiale — onze constructions de `File`, « le rayon se concentrant sur
`plan/` » — était fausse, et sa cause mérite d'être écrite : un `grep .after` rend deux
champs homonymes et sans rapport, `File.after` et le `Anchor.after` qui porte la ligne
d'accroche d'une ancre. Écarter le bruit en écartant tout ce qui vivait hors de `plan/`
a écarté cinq vrais appelants du même coup. Le compilateur, lui, ne s'y est pas trompé.

## 6. Découpage

`src/remove/{mod,desinstallation}.rs`, miroir d'`add/{mod,installation}.rs` :

- `remove::mod` lit les métadonnées, résout le fragment, oppose les quatre refus, rend
  `Planned` ;
- `remove::desinstallation::actions` parcourt le manifeste à l'envers et remplit le
  `Builder`, comme `add::installation::actions` le remplit à l'endroit.

Ordre de retrait inverse de celui d'`add::resoudre` : un fragment part après ceux qui
l'exigent — cas que le refus 3 rend d'ailleurs inatteignable pour l'instant, mais l'ordre
doit être juste le jour où la commande prendra plusieurs noms.

## 7. Ce que le rapport dit

Le bilan nomme, quand elles s'appliquent : la migration retirée et le fait que le schéma
garde ses tables ; les variables d'environnement laissées ; les fichiers `if_absent`
laissés, dont le fragment ne revendique pas la paternité ; les dépendances laissées parce
qu'un autre fragment les réclame ou parce que le squelette les déclare ; et l'avertissement
que le projet peut ne plus compiler,
avec l'invitation à lancer `cargo build`.

## 8. Tests

- **Unitaires, dans `--lib`** : chaque retrait (fichier, ancre, migration, dépendance,
  feature cargo, section, métadonnée), chacun des quatre refus, la règle d'union sur une
  dépendance partagée, l'ancre optionnelle absente, et l'idempotence d'un second `remove`.
- **`tests/integration_remove.rs`** : engendrer un projet, `add`, `remove`, **et
  compiler**. C'est le seul test qui prouve qu'un projet survit à la commande ; Docker
  requis, `--no-fail-fast` obligatoire.
- **L'aller-retour** `add → remove → add` doit rendre un projet identique, aux horodatages
  de migration près.
- **`integration_examples`** doit rester vert sans régénération : aucune template ne
  change.

## 9. Documentation

`docs/docs/cli/remove.md` et sa jumelle française, avec une transcription gardée par
`integration_docs` ; la liste des commandes dans les deux `AGENTS.md` ; `completions` ;
`CHANGELOG.md` ; et une ligne en §4.4 de la spec de référence, qui ne connaît aujourd'hui
que `git checkout`.

## 10. Écarté, et pourquoi

| Écarté | Raison |
|---|---|
| Cascade automatique des dépendants | `remove mail` emportant le sous-système d'authentification est une surprise de taille ; installer de trop est réparable, supprimer de trop détruit du code écrit à la main. |
| Interroger `seaql_migrations` | Une base joignable et la crate `migration` compilée pour désinstaller des fichiers ; `remove` échouerait hors ligne. |
| Scan textuel des références | rustc répond exactement là où un grep approxime, et l'heuristique serait à maintenir. |
| Avertir au lieu de refuser | C'est le troisième verdict que le contrôle `decimal` du doctor a précisément écarté : avertir là où le reste du CLI refuse rend le refus incohérent. |
| Une liste de suppressions à côté de `File` | Deux canaux parallèles pour une même notion ; la porte des conflits, les deux rendus et la restauration devraient chacun apprendre à regarder le second, et un futur `Effect` l'oublierait. |
| Une commande autonome hors du plan | §4.4 impose la séquence à toute commande modifiant un projet ; elle redupliquerait les quatre garde-fous. |
