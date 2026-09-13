# Le backend fichiers de `storage` dépose par fichier temporaire et `rename`

Date : 2026-09-12
Portée : `crates/rbs-cli/templates/features/storage/{files,mod,tests}.rs.jinja`,
`examples/file-drop/src/modules/storage/`, guide `storage` en deux langues, `CHANGELOG` en
deux langues.
Hors portée : le backend S3 (un `PutObject` est déjà atomique côté service), un
`fsync` du répertoire parent après le `rename`, le nettoyage au démarrage des temporaires
qu'un crash aurait laissés, une clé qui désigne un répertoire existant (`get` rend alors
`Unavailable`, comme avant).

## Le problème

`files.rs.jinja:40` écrit l'objet en place : `fs::write(&path, content)` ouvre le fichier
final en le tronquant, puis y copie le corps. Entre les deux, un `get` concurrent lit un
fichier vide ou tronqué et le sert en 200 ; un crash au milieu laisse ce fichier tronqué
sous le nom final, où `exists` le voit comme un objet complet. Un second `put` sur une clé
existante expose donc d'abord un objet vide à quiconque lit pendant l'écriture.

`available()` (`:71`) fait `create_dir_all(&self.root)` à chaque `GET /health`. Le
commentaire prétend qu'un volume démonté ou un droit retiré « échouent tous ici » : c'est
faux, `create_dir_all` sur un répertoire existant réussit sans rien vérifier, et une racine
disparue est recréée en silence — la sonde masque précisément la panne qu'elle devait
montrer.

## Le dépôt

`put` écrit dans un fichier temporaire **du même répertoire** que la cible, puis le
renomme sur elle :

```
<parent>/<nom>.<uuid v4>.partiel  →  <parent>/<nom>
```

- Même répertoire, donc même système de fichiers : `rename(2)` est atomique, et un lecteur
  voit soit l'ancien objet complet, soit le nouveau complet, jamais un intermédiaire.
- Un UUID v4 par dépôt : deux `put` concurrents sur la même clé écrivent chacun leur
  temporaire, le dernier `rename` gagne, aucun des deux ne lit ou ne tronque l'autre. Le
  générateur `v4` est déjà une dépendance du squelette (`uuid = { features = ["v4",
  "v7"] }`), aucune dépendance à ajouter au fragment.
- Le contenu est **synchronisé** (`sync_all`) avant le `rename` : sans cela, un arrêt
  brutal de la machine peut laisser un fichier vide sous le nom final, le renommage ayant
  atteint le disque avant les données. Un `put` acquitté vaut dépôt, comme S3 le promet ;
  c'est le prix d'un `fsync` par dépôt.
- Si l'écriture ou le renommage échoue, le temporaire est retiré, sans que cette
  suppression puisse masquer l'erreur d'origine.
- Une clé à sous-répertoires (`factures/2026/janvier.pdf`) crée toujours son parent avant ;
  le temporaire y vit aussi, ce qui garantit le même système de fichiers même quand `root`
  chevauche un point de montage.

### Rejeté

- **Un répertoire `.partiels/` sous la racine.** Un seul endroit à balayer après un crash,
  mais un nom que `normalize` laisse passer comme clé — un objet `.partiels/x` entrerait en
  collision avec l'espace des temporaires — et un `rename` entre deux répertoires, qui
  reste atomique mais impose que la racine ne chevauche aucun point de montage.
- **Un nom fixe `<nom>.partiel`.** C'est la lettre du backlog, mais deux `put` concurrents
  sur la même clé se partageraient le fichier : le second tronquerait ce que le premier
  écrit, et le premier renommerait un mélange.
- **Un verrou en mémoire par clé.** Il ne protège que le processus qui le tient, et rien
  n'empêche deux instances de servir la même racine.

## La racine et la sonde

`FileStorage::new` crée la racine (`create_dir_all`) et rend `Result<Self, StorageError>` :
une racine qui ne se crée pas — droit absent, chemin sous un fichier — est une erreur de
**démarrage**, nommant le chemin, et non un 500 au premier dépôt. `build` la propage par
`?`, et `AppState::new` échoue là où il échoue déjà pour une base injoignable.

`available()` se réduit à `fs::metadata(&self.root)` : la racine existe encore et est un
répertoire. Elle n'écrit rien.

Ce que la sonde voit : une racine supprimée, un volume dont le point de montage a disparu,
un chemin devenu un fichier. Ce qu'elle ne voit pas, et que son commentaire dit : un droit
d'écriture retiré, un volume remonté en lecture seule — ces pannes se voient au premier
`put`, en 500 dans le journal.

### Rejeté

- **Une sonde qui écrit et efface un fichier témoin.** Elle verrait le remontage en lecture
  seule, mais `/health` est interrogé toutes les quelques secondes par l'orchestrateur, sur
  une route souvent sans authentification : c'est une écriture par sonde dans le magasin
  de données, et un disque plein rendrait la sonde rouge alors que les lectures servent
  encore.
- **Garder `create_dir_all` dans la sonde.** C'est le bug : une racine disparue est
  recréée vide, et `/health` reste vert sur un magasin qui a perdu tous ses objets.
- **`permissions().readonly()`** en plus de `metadata` : sur Unix ce n'est que le bit
  d'écriture du propriétaire, ni les ACL ni un remontage. Une garantie partielle qu'il
  faudrait documenter comme telle n'en est pas une.

## Tests livrés (`tests.rs.jinja`)

Trois tests sans base ni service, sur le backend fichiers :

1. `a_put_leaves_no_temporary_file_behind` — après un dépôt sous une clé à
   sous-répertoires, la racine ne contient que l'objet, aucun `.partiel`.
2. `a_put_on_an_existing_key_never_exposes_an_empty_object` — un lecteur relit la clé en
   boucle pendant qu'un écrivain la remplace en alternant deux contenus d'un mébioctet :
   chaque lecture rend l'un des deux, entier. Sur le code d'avant, `fs::write` tronque
   avant d'écrire et le lecteur attrape un corps vide ou partiel — le rouge est observé
   avant le vert, et sa fréquence est consignée dans le plan.
3. `the_probe_reports_a_root_that_vanished` — la racine existe dès la construction, la
   sonde répond vrai ; la racine retirée sous le stockage vivant, la sonde répond faux et
   ne la recrée pas.

Les deux tests existants suivent le nouveau `new` (`.expect(...)`).

## Documentation

- `mod.rs.jinja` : `root` est « créée au démarrage », et non « au premier dépôt » ;
  `build` propage l'erreur de `new`. `examples/file-drop/src/modules/storage/mod.rs` est
  édité à la main (régions, `dead_code`) : les deux lignes s'y reportent à la main ;
  `files.rs` et `tests.rs` y sont des copies du gabarit.
- Guide `storage` (en, fr) : le paragraphe sous le listing intégral de `files.rs` dit ce
  que le fichier temporaire et le `rename` garantissent, et la section *Testing* nomme les
  trois tests ajoutés.
- `CHANGELOG` 1.5.0, *Fixed* / *Corrigé*, un item par langue.
