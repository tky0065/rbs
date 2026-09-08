# `rbs generate crud` — fermé par défaut quand `auth` est là

Date : 2026-09-08
Statut : validé, prêt pour le plan d'implémentation

Un projet qui vient de faire `rbs add auth` obtient, au `generate crud` suivant, neuf
routes anonymes — `DELETE /posts/{id}` compris. La protection existe (`--role`,
`crates/rbs-cli/src/cli.rs:161`) mais il faut la connaître, la demander, et elle ne couvre
que les écritures. `examples/README.md:119` l'écrit sans détour à propos des éditions
manuelles de `blog-auth` : *« no command wires a guard onto a route you generated »*.

Le défaut est donc « ouvert » sur un projet qui a explicitement installé de quoi fermer.
Cette spec l'inverse.

## Ce qui est décidé

**La présence de `auth` dans `[package.metadata.rbs].features` bascule le rendu de
`generate crud`.** Les neuf routes — les six du CRUD, plus les trois de `--with-upload` —
portent alors l'extracteur `Identity` et une garde de rôle. Sans `auth`, le rendu est
inchangé, octet pour octet.

**Une seule forme pour les neuf handlers** : `identite: Identity` en paramètre,
`identite.require_role(Role::User)?` en tête de corps, `security(("bearer" = []))` et les
réponses 401 et 403 dans le `#[utoipa::path]`. Pas de régime distinct entre lectures et
écritures : le même geste ouvre ou durcit n'importe quelle route, au même endroit.

**Aucune option CLI nouvelle.** L'échappatoire est l'édition du fichier généré — ce code
est fait pour être modifié, et un `--public list,find` figerait au moment de la génération
une décision qui se révise ensuite. Retirer deux lignes du handler et deux lignes de
l'annotation rouvre une route.

**`--role X` survit, purement optionnel**, et ne fait plus que substituer le nom du rôle
**sur les écritures** — `create`, `update`, `delete`, `put_content` — les lectures restant
à `Role::User`. Aucune commande existante ne change de sens ; `rbs generate crud posts
--role Admin` produit ce qu'il produisait, plus les lectures fermées.

**Le garde de rôle devient hiérarchique**, sans quoi le défaut serait faux (voir plus bas).

## Le garde hiérarchique

`templates/features/auth/guard.rs.jinja:35` compare aujourd'hui par égalité :

```rust
if porte == expected { Ok(()) } else { Err(Error::Forbidden) }
```

Écrit tel quel, `require_role(Role::User)?` sur `create` rendrait **403 à un admin**.
Le garde passe donc à un seuil :

```rust
// Les variantes sont ordonnées du moins étendu au plus étendu : un Admin satisfait
// une exigence User. L'ordre de déclaration de l'enum porte cette hiérarchie — un
// rôle inséré entre deux variantes déplace le seuil de toutes les gardes du projet.
if porte >= minimum { Ok(()) } else { Err(Error::Forbidden) }
```

L'enum `Role` de `templates/features/auth/model.rs.jinja:9` gagne `PartialOrd, Ord` dans
son `derive`. Rien d'autre : `User` y est déjà déclaré avant `Admin`, et l'ordre dérivé
suit la déclaration. Le paramètre du trait est renommé `expected` → `minimum`, et le `///`
du trait dit « au moins ce rôle » plutôt que « ce rôle ».

**Le commentaire sur l'ordre de déclaration n'est pas décoratif.** C'est la seule chose
qui empêche un développeur d'ajouter `Moderator` en tête de l'enum et de donner, sans le
voir, un accès administrateur à toutes ses routes gardées. Il est écrit dans le fichier
que le développeur ouvre pour ajouter un rôle — le `///` de l'enum, pas ailleurs.

**Conséquence assumée** : le CLI ne réécrit jamais un fichier existant. Un projet généré
avant cette version garde son `guard.rs` à égalité stricte, et un projet neuf reçoit le
seuil. Les deux sémantiques coexistent selon la date du projet ; la note de version le dit,
et donne les quatre lignes à remplacer pour migrer. Aucune détection automatique n'est
prévue : `rbs upgrade` aligne le manifeste, il ne touche pas au code de l'utilisateur.

## Le contrôleur rendu

```rust
#[utoipa::path(
    get,
    path = "/posts",
    tag = "posts",
    operation_id = "posts_list",
    security(("bearer" = [])),
    params(…),
    responses(
        (status = 200, description = "page de posts", body = Page<PostResponse>),
        (status = 400, …),
        (status = 401, description = "jeton absent ou invalide", …),
        (status = 403, description = "rôle insuffisant", …)
    )
)]
pub async fn list(
    State(state): State<AppState>,
    identite: Identity,
    pagination: Pagination,
) -> Result<Json<Page<PostResponse>>> {
    identite.require_role(Role::User)?;

    Ok(Json(service::list(state.core().db(), &pagination).await?))
}
```

`Identity` implémente `FromRequestParts` : il se place avant tout extracteur qui consomme
le corps — `ValidatedJson`, `Json`, `Bytes` — et l'ordre retenu est celui qu'emploie déjà
le rendu sous `--role`, juste après `State`.

**Le mode d'emploi est un bandeau en tête du fichier, pas un commentaire par handler.**
Répété neuf fois, le même paragraphe alourdirait un contrôleur déjà proche des 200 lignes
et paraphraserait une ligne qui se lit seule. En tête de `controller.rs`, une fois :

```rust
//! Toutes les routes exigent un jeton : `Identity` rend 401 sans jeton valide, et
//! `require_role` 403 en deçà du rôle nommé. Sur une route à durcir, montez le rôle ;
//! sur une route à ouvrir au public, retirez le paramètre `identite`, l'appel à
//! `require_role`, l'entrée `security` et les réponses 401 et 403 de son annotation.
```

Le commentaire de `filter` qui justifie aujourd'hui son absence de garde
(`templates/feature/controller.rs.jinja:60`) disparaît : la route est gardée comme les
autres.

## Les tests générés

`templates/feature/tests.rs.jinja` produit une dizaine de tests HTTP `#[ignore]` qui
appellent le routeur sans en-tête. Tout fermer les ferait tous répondre 401 : la couverture
générée s'effondre si le harnais ne suit pas.

**Le harnais forge son jeton.** `rbs_core::jwt::sign` est public
(`crates/rbs-core/src/jwt.rs:58`) et `Identity` ne vérifie que la signature, jamais
l'existence du compte (`crates/rbs-core/src/extract.rs:39`) : aucune ligne en base n'est
requise. Sous `auth`, le fichier gagne

```rust
/// Signe un jeton portant `role`, sans passer par la base : l'extracteur `Identity`
/// ne vérifie que la signature, et le compte n'a pas à exister pour qu'elle tienne.
fn token(role: &str) -> String { … }
```

et chaque requête existante le présente en `Authorization: Bearer`. Les tests continuent
d'exercer le cycle complet — création, lecture, mise à jour, suppression, filtre, 404, 400
— comme sur un projet sans `auth`. Le rôle signé est celui que le contrôleur exige : `user`
par défaut, celui de `--role` quand il est passé.

**Un test s'ajoute** : la même requête sans en-tête rend 401. **Le renoncement disparaît** :
le bandeau de `tests.rs.jinja:7-10` — « les écritures ne sont pas exercées ici, obtenez un
jeton par `POST /auth/login` » — n'a plus lieu d'être, la template sachant désormais en
émettre un.

L'alternative — enregistrer un compte par la route, le promouvoir en base, se connecter,
ce que `examples/blog-auth/src/posts/tests.rs` fait à la main — est écartée : elle couple
le fichier de tests d'une feature quelconque au schéma du fragment `auth`, et laisse en
base des lignes qu'aucun test ne nettoie.

## `auth` installé après coup

Un projet qui fait `rbs add auth` alors que `posts` existe déjà garde un `posts` grand
ouvert. Le CLI ne réécrit pas les fichiers existants, et cette spec ne commence pas.

Mais le silence est ici un trou de sécurité : le développeur croit avoir fermé son API.
`add auth` liste donc, en fin de sortie, les modules CRUD déjà présents — ceux que
`[package.metadata.rbs].features` porte hors des fragments connus — avec la phrase disant
qu'ils restent publics et les quatre lignes à ajouter par handler. Rien n'est écrit, tout
est dit.

## Ce qui ne change pas

- **Un projet sans `auth` rend exactement ce qu'il rendait.** C'est un critère d'acceptation,
  pas une intention : `hello-crud`, `file-drop` et `newsletter-queue` doivent rester
  identiques octet pour octet sous `integration_examples`.
- **`rbs-core` n'est pas touché.** Le garde vit dans le fragment, donc dans le projet ; le
  schéma `bearer` du document OpenAPI est déjà déclaré par le noyau
  (`crates/rbs-core/src/openapi.rs:62`).
- **Aucune ancre nouvelle.** Les quatorze de `anchors.rs` suffisent.
- **`rbs doctor` n'audite pas les routes ouvertes.** Une route publique est une décision
  légitime ; un diagnostic qui la signale crierait au loup à chaque projet.

## Portée des modifications

| Fichier | Ce qui change |
|---|---|
| `templates/feature/controller.rs.jinja` | garde sur les neuf handlers, bandeau de tête, OpenAPI |
| `templates/feature/tests.rs.jinja` | harnais `token()`, en-tête sur les requêtes, test 401, bandeau retiré |
| `templates/features/auth/guard.rs.jinja` | seuil `>=`, `expected` → `minimum`, doc du trait |
| `templates/features/auth/model.rs.jinja` | `PartialOrd, Ord` sur `Role`, `///` sur l'ordre |
| `src/generate/feature.rs` | le contexte de rendu porte la présence de `auth`, pas seulement `role` |
| `src/generate/command.rs` | lit `features` pour alimenter ce contexte |
| `src/add/` | la sortie de `add auth` liste les CRUD déjà présents |
| `src/generate/controller.rs`, `tests_http.rs` | tests de rendu, dans les deux régimes |
| `examples/blog-auth/` | régénéré : le contrôleur et l'essentiel des tests cessent d'être manuels |
| `crates/rbs-cli/tests/integration_examples.rs` | `the_hand_edits_of_blog_auth_are_in_place` réduit à ce qui reste manuel |
| `examples/README.md`, `examples/README.fr.md` | la liste des éditions de `blog-auth`, et la phrase de la ligne 119 |
| `docs/` | pages `generate crud` et `add auth`, en anglais et en français |

## Versions

`rbs-cli` en **mineure** : aucune signature de commande ne bouge, seul le rendu par défaut
change. `rbs-core` n'est pas republié.

La note de version porte deux points, et pas un de moins :

1. sur un projet portant `auth`, `generate crud` rend désormais des routes fermées — ce
   qu'un pipeline qui compare un rendu attendu verra passer au rouge ;
2. `require_role` compare un seuil dans les projets neufs et une égalité dans les anciens,
   avec les quatre lignes à remplacer pour migrer.

## Critères d'acceptation

- Sur un projet portant `auth`, les neuf routes rendues portent `Identity`,
  `require_role`, `security` et les réponses 401 et 403.
- Sur un projet sans `auth`, le rendu est inchangé — les trois exemples sans `auth`
  repassent `integration_examples` sans modification.
- `--role Admin` nomme `Admin` sur les quatre écritures et `User` sur les cinq lectures.
- Un admin traverse une garde `require_role(Role::User)` ; un `user` bute sur une garde
  `require_role(Role::Admin)` en 403.
- Les tests générés d'un projet sous `auth` passent contre une vraie base, avec
  `--ignored`, sans édition manuelle — c'est le cycle complet qui doit passer, pas un
  fichier réduit aux 401.
- Une requête sans jeton sur une route générée rend 401 ; le test qui l'affirme est
  généré, non écrit à la main.
- `add auth` sur un projet portant déjà un CRUD nomme ce module dans sa sortie.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`
  passent, sur le dépôt comme sur les quatre exemples régénérés.

## Hors périmètre

Pas de permissions fines ni de scopes — le rôle reste le seul axe. Pas de politique par
champ ni de filtrage des réponses selon l'appelant. Pas de réécriture des CRUD déjà
générés, ni de commande qui les fermerait après coup. Pas d'option `--public`, et pas de
propriétaire de ligne : « seul l'auteur modifie son post » demande une colonne `owner_id`
et une décision de modèle que ce défaut ne sait pas prendre à la place du développeur.
