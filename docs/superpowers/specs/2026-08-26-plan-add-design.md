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
    fichiers: Vec<Fichier>,
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
    /// L'« après » diffère de l'origine : l'action produira un effet.
    AFaire,
    /// L'« après » égale l'origine : fichier identique, lignes déjà dans l'ancre,
    /// feature déjà inscrite. L'action est sans effet.
    DejaFait,
    /// Le fichier existait déjà avec un contenu que l'action n'a pas produit. Seul
    /// `--force` (E4) l'écrasera. Ne concerne que `Creer` : une insertion s'ajoute à ce
    /// qu'elle trouve, et un patch TOML ne remplace jamais un fichier entier.
    Conflit,
}
```

`PatchToml` ne porte qu'une variante : E3 y ajoutera l'ajout de dépendance et l'ajout
d'une feature à une dépendance existante. La surface de `Effet` reste stable pendant ce
temps — c'est la raison de l'enum imbriqué plutôt que de trois variantes à plat.

### La règle du statut

> Le statut décrit la relation de l'action au projet **tel qu'il a été trouvé**.

Il ne se décide jamais contre l'aperçu que les actions précédentes du plan ont projeté.
Deux conditions seraient sinon confondues : un conflit **utilisateur**, où le disque porte
du contenu étranger qu'un `--force` doit pouvoir arbitrer, et un conflit **interne au
plan**, où deux actions de la même commande se contredisent — que rien ne devrait franchir.
Sans la règle, `--force` finirait réclamé pour un fichier que le plan est seul à avoir
écrit.

La projection, elle, continue de partir de l'aperçu : une action **compose** avec ce que la
précédente produit, mais se **juge** contre l'origine.

### Deux vues du même plan

```rust
impl Plan {
    /// Ce que l'utilisateur lit : une ligne par action, dans l'ordre où elles ont été
    /// planifiées.
    pub fn actions(&self) -> &[Action];

    /// Ce que E6 écrit : un fichier par chemin touché, avec son contenu d'origine, son
    /// contenu final et le statut agrégé des actions qui le visent.
    pub fn fichiers(&self) -> &[Fichier];

    /// Racine du projet, à laquelle les chemins des fichiers sont relatifs.
    pub fn racine(&self) -> &Path;
}

pub(crate) struct Fichier {
    pub chemin: String,
    /// `None` si le fichier n'existe pas encore.
    pub avant: Option<String>,
    pub apres: String,
    /// Un fichier dont une seule action est en conflit est en conflit ; un fichier dont
    /// toutes les actions sont sans effet est sans effet.
    pub statut: Statut,
}
```

`fichiers()` est l'agrégat de `actions()`, calculé au fil de la construction plutôt que
recalculé à la demande. La distinction règle le cas de `migration/src/lib.rs`, que deux
ancres visent : deux actions, un seul fichier, et la seconde insertion se calcule contre
l'aperçu produit par la première — jamais contre ce que le disque contient encore.

Le statut porté par `Fichier` est ce qui rend `fichiers()` sûr à consommer seul : sans lui,
E6 écrirait un fichier en conflit sans jamais voir le conflit, resté dans `actions()`.

### Ce que la construction refuse

```rust
pub(crate) enum Erreur {
    Acces { chemin: String, source: io::Error },
    DejaProjete { chemin: String },
    Ancre(ancres::Absente),
    Metadonnees(metadata::Erreur),
    ManifesteAbsent { chemin: String },
}
```

`DejaProjete` ferme la seule composition qui n'a pas de sens : `creer` sur un chemin qu'une
action précédente a déjà projeté. Un contenu complet ne compose pas avec un aperçu — il
l'effacerait, et le plan affirmerait deux choses inconciliables sur un même fichier. C'est
une erreur de programmation de l'appelant, pas une situation à absorber.

Chaque variante nomme son fichier **relativement à la racine**, comme `Action::chemin` :
l'emplacement complet du projet est porté une seule fois, par l'en-tête de l'affichage du
plan (E5). Une même commande imprimerait sinon un chemin absolu sur un échec et un nom nu
sur le suivant.

## 4. La construction

```rust
let mut constructeur = Constructeur::nouveau(racine);
constructeur.creer("Dockerfile", contenu)?;
constructeur.inserer(ancres::ROUTES, &lignes)?;
constructeur.patcher(PatchToml::InscrireFeature("docker".into()))?;
let plan = constructeur.finir();
```

Chaque méthode **lit** le fichier visé — ou reprend l'aperçu déjà projeté s'il a été
touché par une action précédente —, calcule l'« après » à partir de cet aperçu, puis en
déduit le statut contre l'origine. Aucune n'écrit.

Chacune lit et calcule *avant* de muter le constructeur : sur un chemin d'erreur, ni les
actions ni les fichiers n'ont bougé, et un plan abandonné en cours de route est sain
plutôt que tronqué. La lecture n'est pas une écriture : le critère « aucun effet de bord sur le
disque » porte sur l'état du répertoire, qui reste inchangé.

Une ancre absente fait échouer `inserer` : le plan ne se construit pas, et rien n'a été
écrit puisque rien ne s'écrit pendant la planification. E2 se charge du message et du code
de sortie.

## 5. La seule retouche à du code existant

`metadata::ajouter_feature` lit, modifie et écrit en une fois. `PatchToml::InscrireFeature`
a besoin de la partie « modifie » seule. La fonction se scinde :

```rust
/// Rend le manifeste avec `feature` inscrite, ou `None` si elle y est déjà.
/// `nom` ne désigne le fichier que dans les messages d'erreur : rien n'est lu ni écrit ici.
pub fn inscrire_feature(texte: &str, feature: &str, nom: &str) -> Result<Option<String>, Erreur>;

/// Inchangée pour ses appelants : lit, appelle `inscrire_feature`, écrit.
pub fn ajouter_feature(cargo_toml: &Path, feature: &str) -> Result<(), Erreur>;
```

Deux points que la première rédaction n'avait pas vus :

- `inscrire_feature` ne lisant plus le fichier, elle ne sait plus le nommer dans ses
  erreurs : d'où le paramètre `nom`, que le plan renseigne avec le chemin relatif.
- Le `None` du retour porte l'idempotence : « la feature est déjà là, aucun texte à
  écrire ». Rendre le texte inchangé aurait obligé chaque appelant à le comparer pour
  décider s'il doit écrire.

`generate` n'est pas touché et son comportement ne change pas. La scission rend au passage
inutile le `#![allow(dead_code)]` que portait `metadata.rs`.

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
| Vérification du working tree Git, arbitrage d'un `Conflit` par `--force` | E4 |
| Affichage du plan, en-tête portant la racine du projet, `--dry-run` | E5 |
| Écriture, rollback, migration de `generate` vers le plan | E6 |

L'en-tête d'affichage revient à E5, et c'est lui qui justifie que les chemins du module
soient relatifs de bout en bout : le plan nomme les fichiers, la racine se dit une fois.

Deux points relevés à la revue de ce module, qui engagent les tâches suivantes :

- **E2 hérite d'une variante d'erreur trop large, pas d'un texte à reformuler.** `inserer`
  sur un fichier qui n'existe pas rend aujourd'hui « ancre introuvable dans
  `migration/src/lib.rs` », pour un fichier absent. C'est le même défaut que celui corrigé
  ici entre un manifeste absent et une section manquante : il se répare en ajoutant une
  variante, non en récrivant une chaîne.
- **`AFaire` dit « cette cible diffère de son point de départ », pas « cette action changera
  quelque chose ».** Juger chaque action contre l'origine a ce corollaire : deux insertions
  de la même ligne sont toutes deux `AFaire`, alors que la seconde ne produira rien une fois
  composée. L'affichage d'E5 montrera donc deux lignes pour un seul changement, et devra
  décider s'il agrège par fichier ou nomme chaque action.
