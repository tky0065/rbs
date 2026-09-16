use super::*;

use super::super::files::FileStorage;

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
            .put(key, Bytes::from_static(b"charge utile"))
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
        .put("sous/../recu.txt", Bytes::from_static(b"charge utile"))
        .await
        .expect("`sous/../recu.txt` reste sous la racine");
    let (_, relu) = read(&storage, "recu.txt")
        .await
        .expect("la clé normalisée doit se relire");
    assert_eq!(relu, b"charge utile");

    fs::remove_dir_all(&root).expect("le répertoire du test doit se nettoyer");
}

/// Le dépôt passe par un fichier temporaire, qui ne doit pas lui survivre.
#[tokio::test]
async fn a_put_leaves_no_temporary_file_behind() {
    let root = root("temporaire");
    let storage = FileStorage::new(root.join("objets")).expect("la racine doit se créer");

    storage
        .put("dossier/objet.bin", Bytes::from_static(b"charge utile"))
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
    let (first, second) = (
        Bytes::from(vec![b'a'; 1 << 20]),
        Bytes::from(vec![b'b'; 1 << 20]),
    );

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
                    let (_, relu) = read(&storage, key)
                        .await
                        .expect("la relecture doit aboutir");
                    assert!(
                        relu == first || relu == second,
                        "lecture n°{reads} : {} octets, ni l'un ni l'autre des contenus déposés",
                        relu.len()
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

/// Un objet d'un mébioctet se lit en plusieurs morceaux : la preuve qu'il n'est plus
/// chargé d'un bloc avant de partir.
#[tokio::test]
async fn a_large_object_reads_back_in_several_chunks() {
    let root = root("flux");
    let storage = FileStorage::new(root.join("objets")).expect("la racine doit se créer");
    let content = Bytes::from(vec![b'x'; 1 << 20]);

    storage
        .put("gros.bin", content.clone())
        .await
        .expect("le dépôt doit aboutir");

    let object = storage
        .get("gros.bin")
        .await
        .expect("la lecture doit aboutir");
    assert_eq!(object.length, Some(1 << 20));
    let morceaux: Vec<Bytes> = object
        .body
        .try_collect()
        .await
        .expect("le flux doit se lire");

    assert!(
        morceaux.len() > 1,
        "l'objet est arrivé d'un seul bloc : il a été chargé en mémoire avant de partir"
    );
    assert_eq!(morceaux.concat(), content);

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
