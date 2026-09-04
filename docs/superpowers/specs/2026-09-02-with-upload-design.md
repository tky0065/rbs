# `rbs generate crud --with-upload`

**Tâche 78 d'`IMPROVE.md`.** Le fragment `storage` pose un trait à cinq méthodes dans
l'`AppState` (`templates/features/storage/mod.rs.jinja:35`, ancre `state_champs`), mais
aucune commande n'en tire de route : chaque projet recâble à la main le dépôt et la
relecture d'un contenu binaire. `examples/file-drop` le prouve — ses trois handlers de
contenu (`src/uploads/controller.rs:143-214`) sont écrits à la main et suivis par des
marqueurs `region:`.

## Ce qui est décidé

**Le drapeau engendre ce que `file-drop` porte à la main, paramétré par le nom de
l'entité.** Le dépôt a déjà tranché la forme de ces routes, en production dans un exemple
compilé en CI ; l'inventer une seconde fois serait ignorer la réponse qu'on a déjà.

### Le corps voyage brut, pas en multipart

L'énoncé de la tâche dit « route multipart ». **La spec s'en écarte.** `file-drop` dépose
en `application/octet-stream` et son commentaire dit pourquoi : « un corps binaire n'a pas
sa place dans un JSON, et le faire passer en base64 obligerait à charger deux fois le
fichier en mémoire ». Le multipart n'apporterait ici qu'un parseur de plus et une
dépendance (`axum` l'a derrière une feature), pour un seul fichier par ligne dont le nom
n'est pas demandé. `Bytes` en extracteur suffit.

### Aucune colonne n'est injectée

C'est le second écart, et il vient de la même lecture. `file-drop` **a** des colonnes
`content_type` et `size` — déclarées à la main dans son `--fields`
(`tests/integration_examples.rs:57-81`) — et son handler de relecture ne les lit jamais :
il écrit `("content-type", "application/octet-stream")` en dur
(`src/uploads/controller.rs:190`). Elles y sont décoratives.

Les injecter automatiquement coûterait plus qu'elles ne rapportent : `dto.rs.jinja` itère
sur `fields`, elles atterriraient donc dans `Create` et `Update` — un client y déclarerait
la taille d'un fichier qu'il n'a pas encore déposé. Les en exclure demanderait une notion
de « champ posé par le serveur » qui n'existe nulle part dans le générateur et qui
traverserait cinq templates.

Le drapeau ajoute donc **les routes, la garde et la clé**. Qui veut des métadonnées les
déclare dans `--fields`, comme `file-drop` le fait.

## La garde

`--with-upload` sur un projet sans le fragment `storage` **refuse avant tout écrit**, dans
`command::plan_for`. Sans lui, `state.storage()` n'existe pas et le projet engendré ne
compilerait plus — un échec de `rustc` sur du code que l'utilisateur n'a pas écrit.

Le patron est celui de `--role` : `validate_role` (`command.rs:345`) lit
`metadata::Metadata.features` (`metadata.rs:98`) et rend `Error::RoleSansAuth`
(`command.rs:145-152`), dont le message nomme la commande qui répare. La nouvelle variante
dit :

> `--with-upload` exige la feature `storage`, absente de ce projet : lancez
> `rbs add storage`, puis relancez la génération

## Les trois routes

Montées sur `/<module>/{id}/content`, après `/<module>/{id}` — aucun conflit, le segment
`content` étant littéral.

| Route | Corps | Réponses |
|---|---|---|
| `PUT /<module>/{id}/content` | `application/octet-stream` | 204, 404 si la ligne n'existe pas |
| `GET /<module>/{id}/content` | — | 200 `application/octet-stream`, 404 si rien n'est déposé |
| `HEAD /<module>/{id}/content` | — | 204 si un contenu existe, 404 sinon |

`HEAD` répond par `exists` et non par un `get` dont on jetterait le corps : les deux
backends du fragment savent y répondre sans lire l'objet.

**La taille** est bornée par un `DefaultBodyLimit::max(TAILLE_MAX)` posé **sur la seule
route de dépôt**, `TAILLE_MAX` étant une constante engendrée et commentée dans
`src/<module>/mod.rs`. Le poser sur le routeur entier relèverait aussi la limite des routes
JSON, qu'aucun besoin ne justifie. Aucun filtrage de type MIME : le code engendré est fait
pour être modifié, et une liste blanche devinée serait fausse pour tout le monde.

## La clé, dérivée et non stockée

```rust
/// Clé du contenu déposé pour `id`.
///
/// Le stockage est un magasin plat : c'est ce préfixe qui range les objets de cette
/// ressource, et rien d'autre ne les distingue.
fn content_key(id: Uuid) -> String {
    format!("<module>/{id}")
}
```

Reprise mot pour mot de `examples/file-drop/src/uploads/service.rs:22-28`. Une colonne qui
porterait la clé la dupliquerait sans jamais en diverger.

## Ce que chaque couche reçoit

La dépendance unidirectionnelle du projet tient : le contrôleur ne touche pas au stockage,
il le passe.

- **`controller.rs.jinja`** — trois handlers annotés `#[utoipa::path]`. `put_content` prend
  `content: Bytes`, `get_content` rend `impl IntoResponse` avec l'en-tête de type.
- **`service.rs.jinja`** — `content_key`, `put_content` (qui **lit la ligne d'abord** :
  sans elle le magasin accumulerait des objets qu'aucune ressource ne réclame),
  `get_content`, `has_content`. `delete` gagne le retrait du contenu, `Storage::delete`
  étant idempotent — une ressource créée sans contenu ne fait donc pas échouer sa
  suppression.
- **`mod.rs.jinja`** — les trois routes, la constante de taille, le `DefaultBodyLimit`.
- **`repository.rs.jinja`, `dto.rs.jinja`, `filter.rs.jinja`, `migration.rs.jinja`,
  `model.rs.jinja`** — inchangés.

Les erreurs du trait se traduisent comme dans l'exemple : `StorageError::NotFound` devient
`Error::NotFound("contenu")` — le seul cas qui vienne du client — et tout le reste un
`Error::Internal`.

## Ce que le générateur doit apprendre

1. `cli.rs:136-156` — `#[arg(long)] with_upload: bool`.
2. `lib.rs:93-106` — la struct d'options qui remplace le tuple positionnel, partagée avec
   `--soft-delete`.
3. `command.rs:24-39` — le champ sur `Options`, la garde `storage`, le report sur
   `Feature`.
4. `feature.rs:222-237` — la clé dans le `Serialize` manuel, `serialize_struct` incrémenté.
5. **`mount.rs:12`** — `const HANDLERS: [&str; 6]` devient conditionnel : les trois
   handlers de contenu doivent s'inscrire à l'ancre `openapi`, faute de quoi les routes
   existeraient sans figurer au document. C'est le point qu'on oublie, et qu'aucune
   compilation ne signale.
6. `tests_http.rs:36-56` — contexte reconstruit, à qui `with_upload` doit être passé
   explicitement (`UndefinedBehavior::Strict`, `template.rs:26`).

## Tests

**Unitaires** :

| Test | Ce qu'il prouve |
|---|---|
| `the_content_routes_are_mounted` | les trois `.route` et leurs méthodes |
| `the_upload_route_alone_raises_the_body_limit` | le `DefaultBodyLimit` n'est pas sur le routeur |
| `the_key_is_derived_from_the_id` | `content_key` et son préfixe de module |
| `putting_content_reads_the_row_first` | l'ordre lecture puis dépôt dans le service |
| `deleting_the_row_removes_its_content` | `delete` appelle `storage.delete` |
| `the_three_handlers_reach_the_openapi_anchor` | `mount::pour` pose neuf lignes à l'ancre `openapi` au lieu de six |
| `an_ordinary_crud_is_unchanged` | **témoin** : sans le drapeau, rendu identique |
| `upload_without_the_storage_feature_is_refused` | la garde, et le message qui nomme `rbs add storage` |

**Banc `#[ignore]`** : un projet `rbs new --with storage` puis
`generate crud … --with-upload` compile et passe `clippy -D warnings` — le seul test qui
prouve que le code engendré s'accorde vraiment au trait du fragment. Un aller-retour
dépôt → relecture s'ajoute aux scénarios HTTP engendrés, sur le backend `fs`, qui ne
demande aucun service.

**`examples/file-drop`** : ses trois handlers écrits à la main deviennent engendrés. Ses
marqueurs `region:` et son entrée `edite_a_la_main` d'`integration_examples.rs:230-235`
tombent, et la non-dérive octet à octet devient la preuve que l'engendré vaut l'écrit à la
main. C'est le meilleur test du lot, et il est gratuit.

## Documentation

- `CHANGELOG.md` et `CHANGELOG.fr.md`, `[Unreleased] / Added`.
- `examples/README.md` et `examples/README.fr.md` : la commande de régénération de
  `file-drop` gagne `--with-upload`, et la liste de ses éditions manuelles se réduit.
- Le guide du stockage et sa paire française.
