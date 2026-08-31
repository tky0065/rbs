# Ergonomie du CLI : portée des drapeaux, sortie machine et annonce d'attente

Date : 2026-08-31
Portée : trois tâches `P2` du backlog ouvert, toutes sur l'ergonomie du binaire `rbs`.

Les trois arbitrages produit sont tranchés en amont et ne sont pas rouverts ici :

1. `--yes` et `--template-dir` cessent d'être globaux et descendent sur les seules
   sous-commandes qui les honorent ; rien n'est câblé ailleurs.
2. `rbs doctor` reçoit un drapeau booléen `--json`, pas un `--format text|json`.
3. `rbs doctor` annonce l'étape lente avant de bloquer, sans drapeau et sans réduire la
   couverture du diagnostic.

Ce document tranche ce qui restait ouvert : la forme du JSON, le lieu de l'annonce, et la
façon dont le rendu texte cesse d'être un bloc différé.

## 1. Portée des drapeaux

### Constat

`crates/rbs-cli/src/cli.rs` déclare les deux drapeaux sur la struct racine avec
`global = true`. `lib.rs` ne transmet `yes` qu'à `create_project` et `template_dir` qu'à
`New` et `Add`. Les six autres sous-commandes les acceptent et les jettent : `rbs generate
crud users --template-dir ./mes-templates` sort en 0 sans avoir lu une seule template du
répertoire nommé, et l'aide de chaque sous-commande annonce les deux drapeaux comme si
elles les honoraient.

`prompts.rs` est bien le seul module qui pose des questions : `inquire` n'est importé que
là, et `prompts::resolve` n'est appelé que par `create_project`. `--yes` n'a donc de sens
que sur `new`.

`template_dir` est lu par `templates::feature_names` et `templates::Source::feature`, tous
deux atteints depuis `new::create` et `add::plan_for`. `--template-dir` n'a donc de sens
que sur `new` et `add`.

### Décision

Les deux champs quittent `Cli` et deviennent des champs des variantes :

- `Commands::New` porte `template_dir` et `yes` ;
- `Commands::Add` porte `template_dir`.

clap refuse alors `rbs generate crud users --template-dir …` avec son
`error: unexpected argument`, et l'aide de chaque sous-commande cesse d'annoncer un
drapeau inopérant.

`Cli` n'a plus que `command` : la struct reste, `Cli::parse()` et les tests de cohérence ne
bougent pas.

### Conséquences documentaires

Huit pages anglaises et huit pages françaises de `docs/` recopient un bloc `--help` où les
deux options figurent, plus `crates/rbs-cli/README.md`. Deux passages de prose les
présentent explicitement comme globales (`cli/new.md`, `cli/doctor.md`). Tous suivent dans
le même commit, anglais et français.

## 2. `rbs doctor --json`

### Forme retenue

```json
{
  "sain": false,
  "checks": [
    { "name": "ancres", "status": "ok", "detail": "les 11 points d'insertion sont en place" },
    { "name": "base", "status": "echec", "detail": "rien ne répond sur localhost:5432",
      "remede": "lancez `docker compose up -d` à la racine du projet, ou corrigez l'URL du .env" }
  ]
}
```

**Les clés sont le schéma, les valeurs sont le contenu.** Le schéma suit la convention
extérieure là où il en existe une, le contenu reste dans la langue du CLI :

- `checks` et `status` sont les noms que `rbs-core` emploie déjà pour la seule autre
  sortie structurée du dépôt, le corps de `GET /health` (`crates/rbs-core/src/health.rs`).
  Un script qui lit les deux n'a pas deux vocabulaires à retenir.
- `name`, `detail` et `remede` reprennent la forme montrée à la validation. `remede` reste
  en français parce que la notion est propre à rbs : aucune convention extérieure ne la
  nomme, et son contenu est une phrase française.
- `remede` est omis quand le contrôle n'en porte pas, comme `ProblemDetails` omet ses
  champs vides (`crates/rbs-core/src/openapi.rs`). Un `null` obligerait chaque lecteur à le
  filtrer.

**Les trois statuts sont `ok`, `avertissement`, `echec`.** `ok` est imposé par l'usage
attendu — `select(.status != "ok")` — et c'est déjà le nom du constructeur `Check::ok`.
`avertissement` et `echec` sont les noms que le code donne aux deux autres états
(`State::Avertissement`, `State::Echec`) : inventer un troisième mot pour un état que le
dépôt nomme déjà deux fois créerait un vocabulaire de plus à tenir à jour. Tous trois sont
en ASCII, donc grepables sans précaution.

**Un en-tête récapitulatif : `sain`.** Un booléen, en tête, qui vaut exactement
`Report::succeeded()` — c'est-à-dire exactement le sens du code de sortie. Sans lui, un
script qui veut le verdict d'ensemble doit le recalculer en repassant sur le tableau et en
sachant qu'un avertissement n'y fait pas obstacle, règle qui n'est écrite nulle part dans
le JSON. Le mot est celui que le rendu texte imprime déjà (« le projet est sain »).

Le rendu est indenté (`to_string_pretty`) : un diagnostic se lit aussi à l'œil, et `jq`
analyse les deux formes indifféremment.

### Mise en œuvre

`State` et `Check` reçoivent `Serialize` avec les renommages ci-dessus. Le rapport JSON est
rendu par `doctor::json::report(&Report) -> String`, à côté de `doctor::render`.

`serde_json` passe de `[dev-dependencies]` à `[dependencies]` de `rbs-cli` : la version est
déjà épinglée au workspace.

### Ce que `--json` n'imprime pas

En JSON, `stdout` ne porte que le document. Les deux lignes de conclusion du mode texte
(`ui::success` « le projet est sain », `ui::warn` « le projet demande votre attention »)
sont tues : `sain` les remplace. L'annonce d'attente de la section 3 est tue également.

Les erreurs de la commande — hors d'un projet rbs, manifeste illisible — continuent de
partir sur `stderr` par `ui::error`, `stdout` restant vide. Le code de sortie garde
exactement le sens qu'il a aujourd'hui : 0 projet sain, 1 diagnostic en défaut ou commande
en échec.

## 3. L'annonce de l'étape lente

### Le vrai obstacle

`doctor::run` exécute les contrôles et rend un `Report` ; `doctor::render::report` en fait
un `Vec<String>` joint puis imprimé d'un bloc. Rien ne peut donc s'afficher *pendant* le
travail : au moment où la première ligne atteint le terminal, la compilation de la crate
`migration` est déjà finie. C'est cette structure qu'il faut lever, pas seulement ajouter
un `println!`.

### Décision : un puits, alimenté au fil des contrôles

`doctor::run` reçoit un puits et lui remet chaque constat au moment où il est fait :

```rust
pub(crate) trait Sortie {
    /// Les titres de tous les contrôles prévus, avant que le premier ne s'exécute.
    fn debut(&mut self, titres: &[&'static str]);
    /// Ce qu'un contrôle s'apprête à faire, quand cela va prendre du temps.
    fn annonce(&mut self, titre: &'static str, raison: &str);
    /// Le constat qui vient d'être fait.
    fn constat(&mut self, check: &Check);
}
```

`debut` existe pour une raison précise : la colonne des détails est alignée sur le titre le
plus long du rapport, largeur qu'un rendu au fil de l'eau ne peut plus découvrir après
coup. Elle se connaît pourtant sans exécuter quoi que ce soit — les titres des contrôles
sont des constantes, et la liste des contrôles à jouer se déduit du manifeste. `run`
construit donc son plan de contrôles d'abord, en annonce les titres, puis les exécute.

Deux implémentations :

- `render::Texte<W: Write>` écrit chaque ligne dès qu'elle est connue, et vide la sortie
  après une annonce ;
- `json::Muette` ne fait rien : le document est rendu à la fin, depuis le `Report`.

`render::report(&Report) -> String` disparaît au profit de `Texte`, qui devient le seul
rendu texte : deux fonctions produisant les mêmes lignes seraient deux vérités. Les tests
de rendu écrivent dans un `Vec<u8>` au lieu de comparer une `String` — la forme des lignes
rendues, elle, ne change pas d'un caractère.

### Le plan de contrôles

`doctor::run` tenait la liste en deux morceaux : un `vec![]` de six appels évalués d'un
coup, puis une boucle sur `FEATURE_CHECKS`. Les deux deviennent une seule liste :

```rust
/// Un contrôle du diagnostic : son titre, connu avant de l'exécuter, et son exécution.
struct Controle {
    titre: &'static str,
    executer: fn(&Path, &Config, &mut dyn FnMut(&str)) -> Check,
}
```

Les contrôles qui n'emploient ni la configuration ni l'annonce l'ignorent par une
fermeture, comme `FEATURE_CHECKS` le fait déjà pour la configuration. Chaque `TITRE` de
module devient `pub(crate)` : le titre est écrit une fois, là où le contrôle vit.

### Où l'annonce est émise

Dans `doctor/base.rs`, juste avant `migrate::launch`. Pas dans le plan de contrôles : le
contrôle `base` sort avant d'appeler cargo dans cinq cas — `.env` illisible, URL absente,
pilote en désaccord avec l'URL, manifeste illisible, hôte injoignable — et annoncer une
compilation qui n'aura pas lieu serait un mensonge imprimé à chaque diagnostic d'un projet
dont la base est arrêtée.

Le texte, rendu comme le sketch validé :

```text
  ✓ ancres      les 11 points d'insertion sont en place
  … base        compilation de la crate migration, peut prendre
                une minute au premier lancement…
  ✓ base        postgres 18.6 répond sur localhost:5432
```

La ligne reste au rapport une fois le constat rendu : l'effacer supposerait un terminal, et
`rbs doctor > diagnostic.txt` doit garder la trace de ce qui a pris une minute. Le retour
à la ligne de la raison est indenté sur la colonne des détails, comme un remède l'est déjà
sous son constat.

### L'ordre d'arrivée

`std::io::Stdout` est tamponné par ligne : un `writeln!` part donc de lui-même. Le puits
texte appelle malgré tout `flush()` après une annonce — la garantie ne doit pas tenir au
choix de tampon d'un appelant qui envelopperait la sortie, et c'est précisément la
promesse de cette tâche. `crate::ui::waiting` prend déjà cette précaution pour l'attente de
la base dans `rbs dev`.

La preuve est prise sur le binaire, horodatée : la ligne `… base` doit atteindre le
terminal des dizaines de secondes avant la ligne `✓ base` qui la suit.

## Tests

| Ce qui est prouvé | Où |
|---|---|
| `--template-dir` refusé par clap sur `generate`, `migrate`, `seed`, `dev`, `doctor`, `upgrade` | `cli.rs`, unitaire |
| `--yes` refusé partout sauf `new` | `cli.rs`, unitaire |
| `--template-dir` toujours accepté par `new` et `add` | `cli.rs`, unitaire |
| Les trois statuts se distinguent dans le JSON | `doctor/json.rs`, unitaire |
| `remede` absent quand le contrôle n'en porte pas | `doctor/json.rs`, unitaire |
| `sain` suit `Report::succeeded()`, avertissement compris | `doctor/json.rs`, unitaire |
| Le rendu texte est inchangé, ligne à ligne | `doctor/render.rs`, unitaires existants réécrits sur `Texte` |
| L'annonce précède le constat du même contrôle | `doctor/render.rs`, unitaire sur le puits |
| `rbs doctor --json` rend un document que `jq` analyse, sans ligne parasite | `integration_doctor.rs` |
| `rbs doctor` annonce avant de compiler, sur une base joignable | `integration_doctor.rs`, `#[ignore]` |

## Hors périmètre

Aucun contrôle n'est ajouté, retiré ni déplacé ; le contrôle de version de migration reste
dans le diagnostic par défaut. `--json` n'est offert par aucune autre commande.
