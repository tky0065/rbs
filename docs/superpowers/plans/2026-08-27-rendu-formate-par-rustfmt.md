# Le rendu de `generate crud` traverse `cargo fmt` sans diff

## Le défaut

Les templates posent en dur des lignes dont la longueur dépend du nom de la feature :
`service.rs.jinja:10` écrit la signature de `list` repliée sur trois lignes, forme choisie
pour `articles`. Sa longueur vaut `95 + len(entite)`. `Tag` donne 98 colonnes — rustfmt la
rassemble ; `AdministrativeDocument` en donne 117 — rustfmt replie ce qui était sur une
ligne ailleurs. Le premier `cargo fmt --check` de l'utilisateur, que `rbs add ci` pose dans
son projet, échoue sur un fichier qu'il n'a pas touché.

Mesuré sur le rendu réel :

| Nom | Fichiers reformatés |
|---|---|
| `tag`, `post`, `note`, `user` | `service.rs` |
| `billet`, `articles` | aucun — d'où le trou dans les tests |
| `administrative_documents` | `service.rs`, `repository.rs`, `controller.rs` |

`examples/blog-auth` en porte la trace : son CRUD s'appelle `posts`, et `cargo fmt --check`
y produit un diff sur `posts/service.rs`. La CI ne l'a jamais vu — `ci.yml:35` ne formate que
les membres du workspace, dont les exemples ne font pas partie.

## La décision

Formater le rendu, plutôt que corriger chaque ligne fautive. La longueur d'un nom est un
continuum : conditionner les templates sur la largeur ne se prouve que pour les noms testés,
et réimplante la règle de rustfmt dans le Jinja. Formater rend la propriété vraie par
construction, en un seul point de passage — `commande.rs:182`, où les sept fichiers sont
collectés avant écriture.

Corollaire assumé : les tests `le_rendu_traverse_rustfmt_sans_diff` par module testent
désormais un invariant que le Jinja n'a plus à porter, et qui est faux pour les noms courts.
Ils cèdent la place à un test central, qui couvre en plus `dto`, `model`, `mod.rs` et la
migration — que personne ne testait.

rustfmt absent ou refusant le rendu : le fichier est écrit brut, avec un avertissement.
`rbs generate` ne doit pas cesser de fonctionner sur une toolchain sans le composant.

## Étapes

1. Test rouge central dans `commande.rs` : pour un éventail de longueurs de nom, chaque
   fichier écrit traverse rustfmt sans diff. Doit échouer sur `tag`, `post`,
   `administrative_documents`.
2. Test rouge dans `integration_examples.rs` : chaque exemple passe `cargo fmt --check`.
   Doit échouer sur `blog-auth`.
3. `generate/format.rs` : `formate(&str) -> String`, repli avec avertissement.
   Distinct de `banc::formate`, qui reste strict — un rendu que rustfmt refuse doit faire
   échouer un test, pas se replier en silence.
4. Le brancher dans `commande.rs::rendre`, sur les sept fichiers et la migration.
5. Retirer les tests par module devenus contradictoires.
6. Régénérer `examples/blog-auth`, en préservant les trois fichiers retouchés à la main que
   `integration_examples.rs:53` autorise.
7. ~~`cargo fmt --check` ajouté à la boucle des exemples dans `ci.yml`~~ — **écart assumé** :
   le garde-fou est le test de l'étape 2, que `ci.yml:41` (`cargo test --workspace`) exécute
   déjà sur les trois plateformes. Un step de plus ferait le même contrôle avec un message
   moins précis, et ne tournerait pas en local.

## Vérifications

- `cargo test -p rbs-cli --lib generate`
- `cargo test -p rbs-cli --test integration_examples`
- `cargo test -p rbs-cli --test integration_generate -- --ignored`
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --check` dans chacun des deux exemples

## Ce que la mise en œuvre a appris

- Le test central lit `Planifiee::fichiers` plutôt qu'une liste de noms écrite en dur : un
  fichier ajouté au rendu entre dans la vérification sans qu'on y pense.
- `rbs new` a été éprouvé aux deux extrêmes (`api`, `tres-long-nom-de-projet-administratif`) :
  aucun rendu reformaté. Le nom du projet ne paraît dans aucune ligne proche de 100 colonnes,
  contrairement au nom de la feature. Le squelette reste hors du formatage.
- `examples/blog-auth` n'a pas été régénéré mais formaté : le CLI écrivant désormais la
  sortie de rustfmt, `cargo fmt` sur l'exemple produit exactement ce qu'une génération
  fraîche produirait — ce que `blog_auth_est_celui_que_le_cli_produit_aujourd_hui` confirme.
- Aucune page de documentation ne cite `posts/service.rs` : le site n'est pas touché.
