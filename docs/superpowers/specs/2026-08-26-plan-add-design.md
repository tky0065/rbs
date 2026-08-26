# Le modèle de plan de `rbs add`

Date : 2026-08-26
Statut : validé, prêt pour le plan d'implémentation
Portée : lot E, tâche E1. Fige le format que E2 à E6 consomment.

## 1. Le problème

Le §4.4 de la spec impose à toute commande qui modifie un projet existant la séquence
`lire → planifier → vérifier → afficher → appliquer`. `rbs generate` la suit aujourd'hui,
mais sans réifier l'étape de planification : `generate/commande.rs` rend les fichiers en
mémoire, calcule les insertions, puis écrit en boucle. Le plan n'existe nulle part comme
valeur.

Trois tâches du lot E ont besoin qu'il existe :

| Tâche | Ce qu'elle demande au plan |
|---|---|
| E4 · working tree | Savoir quels fichiers seront touchés avant d'en toucher un |
| E5 · `--dry-run` | Afficher exactement ce que l'exécution ferait, sans l'exécuter |
| E6 · atomicité | Restaurer les fichiers déjà écrits quand une action échoue |

Sans plan réifié, chacune de ces trois tâches reconstruit sa propre vue partielle de
l'opération, et le `--dry-run` finit par mentir : il décrit une intention, pas un résultat.

## 2. L'invariant

> Un plan est une liste d'actions ; chaque action vise un fichier et connaît son contenu
> **avant** et son contenu **après**. Planifier, c'est calculer les « après » sans rien écrire.

Cet énoncé porte à lui seul les garanties du lot :

- **`--dry-run` ne peut pas mentir** : il affiche les « après » que l'application écrira,
  pas une description de ce qu'elle tentera.
- **Le rollback est gratuit** : le plan a lu les « avant » pendant la planification. E6
  n'a rien à sauvegarder au moment d'écrire.
- **L'idempotence est une propriété du plan**, pas de l'application : une action dont
  l'« après » égale l'« avant » se marque `DejaFait` à la planification.

## 3. Les types

Module `crates/rbs-cli/src/plan/`, neutre : il ne connaît ni `add` ni `generate`, seulement
des actions. C'est ce qui permettra à E6 d'y faire passer `generate` sans que le module
change.

```rust
pub(crate) struct Plan {
    racine: PathBuf,
    actions: Vec<Action>,
}

pub(crate) struct Action {
    /// Chemin du fichier visé, relatif à la racine du projet.
    pub chemin: String,
    pub effet: Effet,
    pub statut: Statut,
}

pub(crate) enum Effet {
    Creer { contenu: String },
    Inserer { ancre: Ancre, lignes: Vec<String> },
    PatcherToml { patch: PatchToml },
}

pub(crate) enum PatchToml {
    /// Inscrit une feature dans `[package.metadata.rbs]`.
    InscrireFeature(String),
}

pub(crate) enum Statut {
    /// L'« après » diffère de l'« avant » : l'action produira un effet.
    AFaire,
    /// L'« après » égale l'« avant » : fichier identique, lignes déjà dans l'ancre,
    /// feature déjà inscrite. L'action est sans effet.
    DejaFait,
    /// Le fichier existe avec un contenu que l'action n'a pas produit. Seul `--force`
    /// (E4) l'écrasera. Ne concerne que `Creer` : une insertion s'ajoute à ce qu'elle
    /// trouve, et un patch TOML ne remplace jamais un fichier entier.
    Conflit,
}
```

`PatchToml` ne porte qu'une variante : E3 y ajoutera l'ajout de dépendance et l'ajout
d'une feature à une dépendance existante. La surface de `Effet` reste stable pendant ce
temps — c'est la raison de l'enum imbriqué plutôt que de trois variantes à plat.

### Deux vues du même plan

```rust
impl Plan {
    /// Ce que l'utilisateur lit : une ligne par action, dans l'ordre où elles ont été
    /// planifiées.
    pub fn actions(&self) -> &[Action];

    /// Ce que E6 écrit : un fichier par chemin touché, avec son contenu d'origine et son
    /// contenu final.
    pub fn fichiers(&self) -> Vec<Fichier>;
}

pub(crate) struct Fichier {
    pub chemin: String,
    /// `None` si le fichier n'existe pas encore.
    pub avant: Option<String>,
    pub apres: String,
}
```

`fichiers()` est l'agrégat de `actions()`. La distinction règle le cas de
`migration/src/lib.rs`, que deux ancres visent : deux actions, un seul fichier, et la
seconde insertion se calcule contre l'aperçu produit par la première — jamais contre ce
que le disque contient encore.

## 4. La construction

```rust
let mut constructeur = Constructeur::nouveau(racine);
constructeur.creer("Dockerfile", contenu)?;
constructeur.inserer(ancres::ROUTES, &lignes)?;
constructeur.patcher(PatchToml::InscrireFeature("docker".into()))?;
let plan = constructeur.finir();
```

Chaque méthode **lit** le fichier visé — ou reprend l'aperçu déjà projeté s'il a été
touché par une action précédente —, calcule l'« après » et en déduit le statut. Aucune
n'écrit. La lecture n'est pas une écriture : le critère « aucun effet de bord sur le
disque » porte sur l'état du répertoire, qui reste inchangé.

Une ancre absente fait échouer `inserer` : le plan ne se construit pas, et rien n'a été
écrit puisque rien ne s'écrit pendant la planification. E2 se charge du message et du code
de sortie.

## 5. La seule retouche à du code existant

`metadata::ajouter_feature` (`crates/rbs-cli/src/metadata.rs:97`) lit, modifie et écrit
en une fois. `PatchToml::InscrireFeature` a besoin de la partie « modifie » seule. La
fonction se scinde :

```rust
/// Rend le texte du `Cargo.toml` avec `feature` inscrite. Ne lit ni n'écrit rien.
pub(crate) fn inscrire_feature(texte: &str, feature: &str) -> Result<String, Erreur>;

/// Inchangée pour ses appelants : lit, appelle `inscrire_feature`, écrit.
pub(crate) fn ajouter_feature(cargo_toml: &Path, feature: &str) -> Result<(), Erreur>;
```

`generate` n'est pas touché et son comportement ne change pas.

## 6. Tests

Le critère de la tâche — « construction d'un plan sans effet de bord sur le disque » — se
prouve par une empreinte du répertoire temporaire prise avant et après la planification,
comparée fichier par fichier. C'est plus fort qu'un test qui vérifie l'absence d'un
fichier attendu : il attrape aussi une écriture qu'on n'aurait pas prévue.

S'y ajoutent les statuts, qui sont la partie du modèle dont E2, E5 et E6 dépendent :

- fichier absent → `AFaire` ; fichier présent et identique → `DejaFait` ; fichier présent
  et différent → `Conflit`
- lignes déjà présentes dans l'ancre → `DejaFait`
- feature déjà inscrite dans `metadata.rbs` → `DejaFait`
- deux insertions visant le même fichier → un seul `Fichier`, la seconde chaînée sur la
  première
- ancre absente → erreur, et le répertoire reste intact

## 7. Hors périmètre

| Ce qui n'est pas dans E1 | Où |
|---|---|
| Message d'erreur et code de sortie sur ancre absente | E2 |
| Ajout de dépendance, feature d'une dépendance | E3 |
| Vérification du working tree Git, `--force` | E4 |
| Affichage du plan, `--dry-run` | E5 |
| Écriture, rollback, migration de `generate` vers le plan | E6 |
