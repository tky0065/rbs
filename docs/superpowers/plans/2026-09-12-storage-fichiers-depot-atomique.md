# Dépôt atomique du backend fichiers — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `FileStorage::put` écrit dans un temporaire à nom unique puis `rename`, la racine est créée à la construction, et `available()` n'écrit plus rien.

**Architecture:** Le dépôt est une fonction synchrone `deposit(path, content)` jouée en un seul `spawn_blocking` — quatre appels `std::fs` d'affilée n'ont aucun point de reprise, et un seul saut vers le pool bloquant vaut mieux que quatre. `new` rend `Result`, `build` propage par `?`. Trois tests sans base dans `tests.rs.jinja`, dont un de concurrence lecteur/écrivain qui voit rouge sur l'écriture en place.

**Tech Stack:** `std::fs`, `tokio::task::spawn_blocking`, `uuid` v4 (déjà au squelette), tests `#[tokio::test]`.

**Spec:** `docs/superpowers/specs/2026-09-12-storage-fichiers-depot-atomique-design.md`

## Global Constraints

- `files.rs` et `tests.rs` d'`examples/file-drop/src/modules/storage/` sont des copies du gabarit (aucune variable Jinja) ; `mod.rs` y est édité à la main et se reporte à la main.
- Le code des fragments n'est compilé par aucun test rapide : `cargo test` dans `examples/file-drop` est l'oracle rapide, `integration_storage` l'oracle Docker.
- `integration_examples` (19 passés) est l'oracle de non-dérive.
- Un commentaire dit le *pourquoi*, jamais le *quoi*. Documentation bilingue dans le même commit.
- Commit Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.

---

### Task 1 : les trois tests, rouges sur le code d'avant

**Files:**
- Modify: `crates/rbs-cli/templates/features/storage/tests.rs.jinja`
- Copy to: `examples/file-drop/src/modules/storage/tests.rs`

- [ ] **Step 1 : ajouter les tests** après `a_key_escaping_the_root_is_rejected`, et faire suivre les deux `FileStorage::new(...)` existants d'un `.expect("la racine doit se créer")`.

```rust
/// Les fichiers réguliers sous `dir`, à toute profondeur.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).expect("le répertoire doit se lire") {
        let path = entry.expect("l'entrée doit se lire").path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// Le dépôt passe par un fichier temporaire, qui ne doit pas lui survivre.
#[tokio::test]
async fn a_put_leaves_no_temporary_file_behind() {
    let root = root("temporaire");
    let storage = FileStorage::new(root.join("objets")).expect("la racine doit se créer");

    storage
        .put("dossier/objet.bin", b"charge utile".to_vec())
        .await
        .expect("le dépôt doit aboutir");

    assert_eq!(
        files_under(&root),
        vec![root.join("objets/dossier/objet.bin")],
        "seul l'objet doit rester sous la racine"
    );

    fs::remove_dir_all(&root).expect("le répertoire du test doit se nettoyer");
}

/// Un lecteur qui relit une clé pendant qu'on la remplace voit l'ancien objet ou le
/// nouveau, entiers — jamais un corps vide ni tronqué.
///
/// Sur une écriture en place, `fs::write` tronque le fichier avant de le remplir, et une
/// lecture concurrente tombe dans la fenêtre. Deux contenus d'un mébioctet l'élargissent
/// assez pour que le test la voie à chaque exécution.
#[tokio::test(flavor = "multi_thread")]
async fn a_put_on_an_existing_key_never_exposes_an_empty_object() {
    let root = root("remplacement");
    let storage = FileStorage::new(root.join("objets")).expect("la racine doit se créer");
    let key = "objet.bin";
    let (first, second) = (vec![b'a'; 1 << 20], vec![b'b'; 1 << 20]);

    storage
        .put(key, first.clone())
        .await
        .expect("le dépôt doit aboutir");

    let writer = {
        let storage = storage.clone();
        let (first, second) = (first.clone(), second.clone());
        tokio::spawn(async move {
            for round in 0..200 {
                let content = if round % 2 == 0 { &second } else { &first };
                storage
                    .put(key, content.clone())
                    .await
                    .expect("le dépôt doit aboutir");
            }
        })
    };

    let mut reads = 0;
    while !writer.is_finished() {
        let read = storage.get(key).await.expect("la relecture doit aboutir");
        assert!(
            read == first || read == second,
            "lecture n°{reads} : {} octets, ni l'un ni l'autre des contenus déposés",
            read.len()
        );
        reads += 1;
    }
    writer.await.expect("l'écrivain doit finir");
    assert!(reads > 0, "aucune lecture n'a eu lieu pendant les dépôts");

    fs::remove_dir_all(&root).expect("le répertoire du test doit se nettoyer");
}

/// La racine existe dès la construction, et la sonde constate sa présence sans la recréer.
///
/// Une sonde qui recréerait une racine disparue resterait verte sur un magasin qui vient
/// de perdre tous ses objets.
#[tokio::test]
async fn the_probe_reports_a_root_that_vanished() {
    let root = root("sonde");
    let objets = root.join("objets");
    let storage = FileStorage::new(objets.clone()).expect("la racine doit se créer");

    assert!(objets.is_dir(), "la racine doit exister dès la construction");
    assert!(storage.available().await, "la sonde doit être verte sur une racine présente");

    fs::remove_dir_all(&objets).expect("la racine doit se retirer");

    assert!(
        !storage.available().await,
        "une racine disparue doit rendre la sonde rouge"
    );
    assert!(!objets.exists(), "la sonde ne doit pas recréer la racine");

    fs::remove_dir_all(&root).expect("le répertoire du test doit se nettoyer");
}
```

`use std::path::{Path, PathBuf};` remplace l'import de `PathBuf` seul.

- [ ] **Step 2 : voir le rouge** — copier le gabarit sur `examples/file-drop/src/modules/storage/tests.rs`, puis `cd examples/file-drop && cargo test modules::storage::tests`. Attendu : erreur de compilation sur `.expect` (l'ancien `new` rend `Self`). Retirer provisoirement les `.expect` pour observer les trois échecs et **compter** les lectures fautives du test de remplacement sur l'ancien `put` ; consigner le chiffre dans le commit.

### Task 2 : le dépôt par temporaire, la racine à la construction, la sonde sans écriture

**Files:**
- Modify: `crates/rbs-cli/templates/features/storage/files.rs.jinja`
- Modify: `crates/rbs-cli/templates/features/storage/mod.rs.jinja:72,121`
- Copy/port to: `examples/file-drop/src/modules/storage/{files,mod}.rs`

**Interfaces:**
- Produces: `FileStorage::new(root: PathBuf) -> Result<Self, StorageError>` ; `Storage::available` inchangé de signature.

- [ ] **Step 1 : `files.rs.jinja`**

```rust
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs;
use uuid::Uuid;

use super::{Storage, StorageError, normalize};

/// Stockage sur le système de fichiers local, sous une racine unique.
#[derive(Debug, Clone)]
pub struct FileStorage {
    root: PathBuf,
}

impl FileStorage {
    /// Ouvre un stockage sous `root`, créée ici.
    ///
    /// Au démarrage plutôt qu'au premier dépôt : une racine qui ne se crée pas — droit
    /// absent, chemin sous un fichier — est une erreur de configuration, et elle se voit
    /// avant que la première requête ne la découvre en 500. C'est aussi ce qui laisse à
    /// `available` quelque chose à constater sans rien écrire.
    pub fn new(root: PathBuf) -> Result<Self, StorageError> {
        std::fs::create_dir_all(&root).map_err(|error| {
            unavailable(std::io::Error::new(
                error.kind(),
                format!("la racine {} ne se crée pas : {error}", root.display()),
            ))
        })?;

        Ok(Self { root })
    }

    /// Le chemin d'un objet, sa clé une fois ramenée sous la racine.
    fn path(&self, key: &str) -> Result<PathBuf, StorageError> {
        Ok(self.root.join(normalize(key)?))
    }
}

fn unavailable(error: std::io::Error) -> StorageError {
    StorageError::Unavailable(Box::new(error))
}

/// Écrit `content` dans un temporaire à côté de `path`, puis le renomme sur elle.
///
/// Même répertoire, donc même système de fichiers : le `rename` est atomique, et un
/// lecteur voit l'ancien objet ou le nouveau, jamais un fichier tronqué. Un UUID par
/// dépôt : deux dépôts concurrents sur la même clé n'écrivent jamais dans le même
/// temporaire, et le dernier renommé l'emporte.
fn deposit(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut partial = path.as_os_str().to_owned();
    partial.push(format!(".{}.partiel", Uuid::new_v4()));
    let partial = PathBuf::from(partial);

    let deposited = std::fs::File::create(&partial).and_then(|mut file| {
        file.write_all(content)?;
        // Le renommage peut atteindre le disque avant les données : sans cette
        // synchronisation, une coupure laisserait un fichier vide sous le nom final.
        file.sync_all()?;
        // Windows refuse de renommer un fichier encore ouvert.
        drop(file);
        std::fs::rename(&partial, path)
    });

    if deposited.is_err() {
        let _ = std::fs::remove_file(&partial);
    }

    deposited
}

#[async_trait]
impl Storage for FileStorage {
    async fn put(&self, key: &str, content: Vec<u8>) -> Result<(), StorageError> {
        let path = self.path(key)?;

        // Un seul saut vers le pool bloquant pour toute la séquence : elle n'a aucun
        // point où reprendre autrement que du début, et `tokio::fs` en ferait quatre.
        tokio::task::spawn_blocking(move || deposit(&path, &content))
            .await
            .map_err(|error| unavailable(std::io::Error::other(error)))?
            .map_err(unavailable)
    }

    // get / delete / exists inchangés

    // La racine est créée à la construction : la sonde constate qu'elle est toujours là,
    // sans rien écrire — la recréer masquerait précisément la perte qu'elle doit montrer.
    // Un droit d'écriture retiré ou un volume remonté en lecture seule ne se voient pas
    // ici : ils se voient au premier dépôt, en 500 dans le journal.
    async fn available(&self) -> bool {
        fs::metadata(&self.root)
            .await
            .is_ok_and(|metadata| metadata.is_dir())
    }
}
```

- [ ] **Step 2 : `mod.rs.jinja`** — `/// Racine du backend \`fs\`, créée au démarrage.` et `"fs" => Ok(Arc::new(files::FileStorage::new(config.root)?)),`.
- [ ] **Step 3 : reporter** — copier `files.rs.jinja` sur `examples/file-drop/src/modules/storage/files.rs`, appliquer les deux lignes de `mod.rs` à la main.
- [ ] **Step 4 : voir le vert** — `cd examples/file-drop && cargo test modules::storage::tests` (7 passés, 2 ignorés), puis `cargo clippy --all-targets -- -D warnings` et `cargo fmt --check` dans l'exemple.
- [ ] **Step 5 : non-dérive** — `cargo test -p rbs-cli --test integration_examples` (19 passés).

### Task 3 : documentation et changelog

**Files:**
- Modify: `docs/docs/guides/storage.md` (sous le listing de `files.rs`, et « Testing »)
- Modify: `docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/storage.md` (mêmes endroits)
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md` (`## [1.5.0]`, *Fixed* / *Corrigé*)

- [ ] **Step 1 : guide** — après le bloc `file=…/files.rs`, un paragraphe : le dépôt écrit un temporaire à côté de la cible et le renomme, un lecteur voit l'ancien objet ou le nouveau, la racine est créée au démarrage, et la sonde constate sans écrire. Dans « Testing », nommer les trois tests ajoutés.
- [ ] **Step 2 : changelog** — un item par langue.
- [ ] **Step 3 : vérifier** — `cargo test -p rbs-cli --test integration_docs -- --include-ignored` (14 passés), `cd docs && npm test` (16) et `npm run typecheck`.

### Task 4 : passe Docker et finitions

- [ ] `cargo test -p rbs-cli --test integration_storage -- --ignored --no-fail-fast > …/scratchpad/storage-t19.log 2>&1`, lire le fichier.
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p rbs-cli --lib`.
- [ ] Commit `fix(storage): dépose par fichier temporaire et rename, et crée la racine au démarrage`.
