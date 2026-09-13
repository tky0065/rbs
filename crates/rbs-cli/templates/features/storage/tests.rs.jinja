use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::files::FileStorage;
use super::{Storage, StorageConfig, StorageError, build};

/// Un répertoire vide, propre à un test, sous la racine temporaire du système.
fn root(nom: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("storage-{}-{nom}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("le répertoire du test doit se créer");

    path
}

/// Ce que le trait promet, sans rien connaître du backend qui l'honore.
///
/// Écrite contre `&dyn Storage` pour être rejouable telle quelle contre S3 : deux
/// backends qui ne passeraient pas la même ronde n'abstrairaient rien.
async fn round(storage: &dyn Storage) {
    let key = "factures/2026/janvier.pdf";

    assert!(!storage.exists(key).await.expect("l'existence se consulte"));

    storage
        .put(key, b"%PDF-1.7".to_vec())
        .await
        .expect("le dépôt doit aboutir");

    assert!(storage.exists(key).await.expect("l'existence se consulte"));
    assert_eq!(
        storage.get(key).await.expect("la lecture doit aboutir"),
        b"%PDF-1.7"
    );

    storage
        .delete(key)
        .await
        .expect("la suppression doit aboutir");

    assert!(!storage.exists(key).await.expect("l'existence se consulte"));
    assert!(
        matches!(storage.get(key).await, Err(StorageError::NotFound(_))),
        "lire un objet supprimé doit rendre `NotFound`"
    );
}

#[tokio::test]
async fn the_file_backend_puts_gets_attests_then_deletes() {
    let root = root("ronde");

    round(&FileStorage::new(root.join("objets")).expect("la racine doit se créer")).await;

    fs::remove_dir_all(&root).expect("le répertoire du test doit se nettoyer");
}

/// Un nom d'objet vient souvent de l'utilisateur : `../` y remonterait hors de la racine.
#[tokio::test]
async fn a_key_escaping_the_root_is_rejected() {
    let root = root("traversee");
    let storage = FileStorage::new(root.join("depot/objets")).expect("la racine doit se créer");

    let witnesses = [root.join("depot/vole.txt"), root.join("vole.txt")];
    let absolute = witnesses[0]
        .to_str()
        .expect("chemin représentable")
        .to_owned();

    for key in [
        "../vole.txt",
        "../../vole.txt",
        "sous/../../../vole.txt",
        &absolute,
    ] {
        let error = storage
            .put(key, b"charge utile".to_vec())
            .await
            .expect_err("une clé sortant de la racine doit être refusée");

        assert!(
            matches!(error, StorageError::RejectedKey(_)),
            "`{key}` doit être refusée comme clé, et non échouer à l'écriture : {error}"
        );
    }

    for witness in &witnesses {
        assert!(
            !witness.exists(),
            "{} a été écrit hors de la racine",
            witness.display()
        );
    }

    // Le refus porte sur l'évasion, pas sur la présence d'un `..` : une clé qui redescend
    // sans sortir reste valide, sans quoi la normalisation serait une simple sous-chaîne.
    storage
        .put("sous/../recu.txt", b"charge utile".to_vec())
        .await
        .expect("`sous/../recu.txt` reste sous la racine");
    assert_eq!(
        storage
            .get("recu.txt")
            .await
            .expect("la clé normalisée doit se relire"),
        b"charge utile"
    );

    fs::remove_dir_all(&root).expect("le répertoire du test doit se nettoyer");
}

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

/// Des lecteurs qui relisent une clé pendant qu'on la remplace voient l'ancien objet ou
/// le nouveau, entiers — jamais un corps vide ni tronqué.
///
/// Sur une écriture en place, `fs::write` tronque le fichier avant de le remplir, et une
/// lecture concurrente tombe dans la fenêtre. Quatre lecteurs et des contenus d'un
/// mébioctet l'échantillonnent assez pour que le test la voie à chaque exécution.
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

    let readers: Vec<_> = (0..4)
        .map(|_| {
            let storage = storage.clone();
            let (first, second) = (first.clone(), second.clone());
            let writer = writer.abort_handle();
            tokio::spawn(async move {
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
                reads
            })
        })
        .collect();

    writer.await.expect("l'écrivain doit finir");
    for reader in readers {
        let reads = reader.await.expect("le lecteur doit finir");
        assert!(reads > 0, "aucune lecture n'a eu lieu pendant les dépôts");
    }

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

    assert!(
        objets.is_dir(),
        "la racine doit exister dès la construction"
    );
    assert!(
        storage.available().await,
        "la sonde doit être verte sur une racine présente"
    );

    fs::remove_dir_all(&objets).expect("la racine doit se retirer");

    assert!(
        !storage.available().await,
        "une racine disparue doit rendre la sonde rouge"
    );
    assert!(!objets.exists(), "la sonde ne doit pas recréer la racine");

    fs::remove_dir_all(&root).expect("le répertoire du test doit se nettoyer");
}

/// Une configuration S3 dont rien n'est joignable : port fermé et identifiants faux.
fn s3_config() -> StorageConfig {
    StorageConfig {
        backend: "s3".to_owned(),
        root: PathBuf::from("./stockage"),
        bucket: "demo".to_owned(),
        region: "eu-west-3".to_owned(),
        endpoint: Some("http://127.0.0.1:1".to_owned()),
        access_key_id: "aucune".to_owned(),
        secret_access_key: "aucune".to_owned(),
        force_path_style: true,
    }
}

/// `AppState::new` est synchrone : le client doit se dériver d'une configuration résolue.
#[test]
fn the_s3_backend_builds_without_touching_the_network() {
    // Aucune boucle Tokio n'est installée sous un `#[test]` : le SDK ne pourrait attendre
    // aucune réponse. L'endpoint pointe de surcroît sur un port fermé, dont une tentative
    // de connexion rendrait une erreur au lieu du client attendu.
    assert!(
        tokio::runtime::Handle::try_current().is_err(),
        "ce test ne prouve rien s'il tourne sous une boucle Tokio"
    );

    // La première construction du processus paie l'initialisation du client HTTPS du SDK
    // — fournisseur de chiffrement et magasin de certificats du système, une centaine de
    // millisecondes de disque et de calcul. La mesure porte sur la suivante.
    let storage = build(s3_config()).expect("le client S3 doit se dériver de la configuration");

    let depart = Instant::now();
    build(s3_config()).expect("le client S3 doit se dériver de la configuration");
    let duree = depart.elapsed();

    assert!(
        format!("{storage:?}").contains("S3Storage"),
        "`backend = \"s3\"` doit construire le backend S3 : {storage:?}"
    );
    // Une erreur de connexion avalée en silence resterait invisible aux `expect`
    // ci-dessus : le budget la rattrape, aucun aller-retour réseau n'y tenant.
    assert!(
        duree < Duration::from_millis(20),
        "la construction a duré {duree:?} : quelque chose a été attendu du réseau"
    );
}

#[test]
fn an_unknown_backend_fails_naming_the_allowed_values() {
    let mut config = s3_config();
    config.backend = "chimere".to_owned();

    let error = build(config)
        .expect_err("`chimere` n'est pas un backend")
        .to_string();

    assert!(error.contains("chimere"), "{error}");
    assert!(
        error.contains("\"fs\"") && error.contains("\"s3\""),
        "le message ne nomme pas les valeurs admises : {error}"
    );
}

// Les deux tests qui suivent joignent le service que décrit la section `[storage]`, avec
// `backend = "s3"` : MinIO en développement, S3 en production. `cargo test -- --ignored`
// les lance, les `RBS_STORAGE__*` désignant le bucket et le service.

/// La ronde du trait, rejouée telle quelle contre S3.
///
/// C'est la même fonction que le backend fichiers traverse plus haut, appelée sans une
/// ligne de différence : un jeu réécrit pour S3 prouverait que S3 marche, jamais que le
/// trait abstrait.
#[tokio::test]
#[ignore = "joint le service S3 de la section [storage]"]
async fn the_s3_backend_passes_the_same_round_as_the_file_backend() {
    let storage = super::from_config().expect("la section [storage] doit être lisible");

    round(&*storage).await;
}

/// Un objet déposé par le trait, relu sans lui.
///
/// Le client est bâti ici, et non emprunté à `S3Storage` dont le champ est privé : une
/// relecture qui repasserait par le même client ne dirait rien de ce qui est réellement
/// arrivé dans le bucket.
#[tokio::test]
#[ignore = "joint le service S3 de la section [storage]"]
async fn an_object_put_by_the_trait_reads_back_through_the_s3_client() {
    let config = rbs_core::config::section::<StorageConfig>("storage")
        .expect("la section [storage] doit être lisible");
    let storage = build(config.clone()).expect("le backend doit se construire");

    let key = "hors-trait/recu.bin";
    storage
        .put(key, b"charge utile".to_vec())
        .await
        .expect("le dépôt doit aboutir");

    let identifiants = aws_sdk_s3::config::Credentials::new(
        config.access_key_id.clone(),
        config.secret_access_key.clone(),
        None,
        None,
        "test-hors-trait",
    );

    let mut client = aws_sdk_s3::Config::builder()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(config.region.clone()))
        .credentials_provider(identifiants)
        .force_path_style(config.force_path_style);

    if let Some(endpoint) = &config.endpoint {
        client = client.endpoint_url(endpoint);
    }

    let object = aws_sdk_s3::Client::from_conf(client.build())
        .get_object()
        .bucket(&config.bucket)
        .key(key)
        .send()
        .await
        .expect("l'objet doit être dans le bucket");

    let content = object
        .body
        .collect()
        .await
        .expect("le corps de l'objet doit se lire")
        .into_bytes();

    assert_eq!(content.as_ref(), b"charge utile");

    storage
        .delete(key)
        .await
        .expect("la suppression doit aboutir");
}
