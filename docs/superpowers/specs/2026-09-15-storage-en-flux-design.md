# Storage en flux — design

**Date :** 2026-09-15 · **Portée :** fragment `storage` et routes `/{module}/{id}/content`
qu'engendre `generate crud --with-upload`.

## Problème

Le trait `Storage` du fragment (`templates/features/storage/mod.rs.jinja:37,40`) prend et
rend des `Vec<u8>` :

- `get` charge l'objet entier en mémoire — `fs::read` côté fichiers, `body.collect()` puis
  `to_vec()` côté S3 (`s3.rs.jinja:80`), soit une copie de plus ;
- `put` reçoit un `Vec<u8>` que le contrôleur fabrique par `Bytes::to_vec()`
  (`feature/controller.rs.jinja:337`) : une copie du corps déjà en mémoire ;
- vingt GET concurrents d'un objet de 10 Mio tiennent 400 Mo résidents, et un objet déposé
  par une autre voie que l'API — donc hors de `DefaultBodyLimit` — n'est borné par rien.

## Décision

Lecture en flux, écriture en `Bytes` sans copie. L'envoi reste en un bloc : il est déjà
borné par la limite de corps de la route, et un `put` en flux vers S3 exigerait l'upload
multipart (parts ≥ 5 Mio), une complexité que rien ne réclame.

```rust
/// Le contenu d'un objet, morceau par morceau.
pub type ContentStream = futures_util::stream::BoxStream<'static, std::io::Result<bytes::Bytes>>;

/// Un objet lu : sa taille quand le backend la connaît, son contenu en flux.
pub struct Object {
    pub length: Option<u64>,
    pub body: ContentStream,
}

async fn put(&self, key: &str, content: bytes::Bytes) -> Result<(), StorageError>;
async fn get(&self, key: &str) -> Result<Object, StorageError>;
```

`ContentStream` et non `ByteStream` : le nom est pris par `aws_sdk_s3::primitives`, que
`s3.rs` importe.

### Backends

- **Fichiers** — `File::open` tranche `NotFound` avant tout octet envoyé ; la taille vient
  de `file.metadata()` ; le corps est un `tokio_util::io::ReaderStream` sur le fichier.
  `put` garde son `spawn_blocking(deposit)` (écriture atomique par fichier temporaire) :
  `Bytes` se déréférence en `&[u8]`.
- **S3** — `get_object` tranche `NoSuchKey` avant tout octet envoyé, comme aujourd'hui ;
  la taille vient de `content_length()` ; le corps est
  `ReaderStream::new(object.body.into_async_read())`. `put` passe
  `ByteStream::from(content)` sans copie.

### Routes CRUD

- `put_content` transmet le `Bytes` extrait tel quel au service, qui le passe au trait.
- `get_content` répond `Body::from_stream(object.body)`, `content-type:
  application/octet-stream`, et `content-length` quand `length` est connue. La couche de
  compression du squelette reste libre de retirer `content-length` quand elle compresse.
- `has_content` (HEAD) ne change pas : il passe par `exists`.

Une erreur au milieu du flux — fichier tronqué, connexion S3 coupée — ne peut plus devenir
un 500 : les en-têtes sont partis. La réponse s'interrompt, ce que le client voit comme un
corps incomplet face à `content-length`. C'est le prix du flux, et c'est ce que fait tout
serveur de fichiers.

### Dépendances engendrées

Le fragment `storage` ajoute `bytes`, `futures-util` et `tokio-util` (feature `io`), en
dernière version stable (`cargo add --dry-run`). `--with-upload` exige déjà `storage` :
le code CRUD peut compter sur les trois.

## Compatibilité

Code engendré : un projet existant garde son trait et ses routes, rien ne change chez lui
tant qu'il ne réengendre pas. La note `1.5.0` dit comment adopter le flux à la main
(signature du trait, deux backends, `get_content`). Aucun changement dans `rbs-core`.

## Preuves attendues

- Ronde des deux backends réécrite sur `Object` : longueur rendue, contenu recollé,
  `NotFound` sur une clé absente.
- Un objet de 1 Mio lu par le backend fichiers arrive en **plus d'un** morceau — la preuve
  que rien n'est plus chargé d'un bloc.
- Le test CRUD `the_content_round_trips_through_put_get_and_head` vérifie `content-length`.
- `integration_storage` (MinIO) et `integration_crud` verts, `integration_examples` vert
  après régénération de `file-drop`.
