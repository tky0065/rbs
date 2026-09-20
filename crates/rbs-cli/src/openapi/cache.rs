//! Le **contrat** mémorisé, pour que trois commandes ne recompilent pas le même projet.
//!
//! `rbs routes`, `rbs openapi export` et `rbs generate client` passent toutes par le
//! binaire `openapi`, dont chaque lancement est une compilation complète en profil dev
//! d'un projet Axum + SeaORM + utoipa. Deux d'entre elles se lancent l'une après l'autre
//! dans la même minute, sur des sources qui n'ont pas bougé.
//!
//! Le module n'expose qu'[`au_travers`] : l'appelant donne ce qu'il ferait sans cache, et
//! ignore tout du condensat, de l'emplacement du fichier et de ce qui invalide. Aucune
//! panne du cache ne remonte — `target/` non inscriptible, document corrompu, source
//! illisible se soldent tous par une recompilation, jamais par un refus : un cache qui
//! fait échouer une commande est pire que pas de cache.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

/// Où le contrat mémorisé vit, relativement à la racine du projet.
///
/// Sous `target/` et non dans un répertoire à soi : git l'ignore déjà et `cargo clean`
/// l'efface déjà. Rien de nouveau à apprendre à nettoyer, et un projet qui part en
/// archive n'emporte pas un contrat périmé.
const REPERTOIRE: &str = "target/rbs";

/// Le contrat, tel que le binaire l'a imprimé.
const DOCUMENT: &str = "openapi.json";

/// Le condensat des sources qui ont produit le document voisin.
const CONDENSAT: &str = "openapi.sha256";

/// Ce que le condensat couvre : tout ce dont le contrat peut dépendre.
///
/// `src/**` porte les annotations `utoipa` et les DTO ; `migration/src/**` n'y entre pas
/// pour lui-même mais parce qu'un module de migration déclaré et absent fait échouer la
/// compilation, et qu'un cache qui rendrait un contrat là où cargo refuserait mentirait ;
/// `Cargo.lock` parce qu'une montée d'`utoipa` change le document sans toucher une ligne
/// du projet.
const SOURCES: [&str; 3] = ["src", "migration/src", "Cargo.lock"];

/// Rend le contrat mémorisé du projet enraciné en `root`, ou ce qu'`imprimer` produit.
///
/// Le condensat est calculé **avant** `imprimer`, et c'est sous celui-là que le document
/// est mémorisé : une source retouchée pendant la compilation donne alors un condensat
/// qui ne vaudra plus au coup suivant, et la commande recompilera. L'inverse — condenser
/// après — mémoriserait le contrat d'avant sous la clé d'après.
pub(crate) fn au_travers<E>(
    root: &Path,
    imprimer: impl FnOnce() -> Result<String, E>,
) -> Result<String, E> {
    let condensat = condensat(root);

    if let Some(condensat) = &condensat
        && let Some(document) = memorise(root, condensat)
    {
        return Ok(document);
    }

    let document = imprimer()?;

    if let Some(condensat) = &condensat {
        memoriser(root, condensat, &document);
    }

    Ok(document)
}

/// Le condensat des sources du projet, ou `None` si l'une d'elles ne se lit pas.
///
/// Chemin **et** contenu de chaque fichier : un fichier renommé change le contrat sans
/// changer un octet de code, et un fichier supprimé disparaît de la liste — les deux
/// invalident. Les entrées sont triées pour que l'ordre de `read_dir`, qui n'est celui
/// d'aucun système de fichiers en particulier, ne fasse pas varier le condensat.
fn condensat(root: &Path) -> Option<String> {
    let mut entrees: Vec<(String, Vec<u8>)> = Vec::new();

    for source in SOURCES {
        collecter(&root.join(source), root, &mut entrees)?;
    }

    entrees.sort_by(|(un, _), (autre, _)| un.cmp(autre));

    let mut hacheur = Sha256::new();
    for (chemin, contenu) in entrees {
        // Les longueurs avant les octets : sans elles, un fichier `ab` vide et un fichier
        // `a` portant `b` donneraient la même suite d'octets, donc le même condensat.
        hacheur.update((chemin.len() as u64).to_le_bytes());
        hacheur.update(chemin.as_bytes());
        hacheur.update((contenu.len() as u64).to_le_bytes());
        hacheur.update(&contenu);
    }

    // Rendu à la main : `sha2` 0.11 sort un `Array`, qui n'implémente pas `LowerHex`.
    let mut hexadecimal = String::with_capacity(Sha256::output_size() * 2);
    for octet in hacheur.finalize() {
        let _ = write!(hexadecimal, "{octet:02x}");
    }

    Some(hexadecimal)
}

/// Ajoute à `entrees` le fichier `chemin`, ou tous ceux de l'arborescence qu'il ouvre.
///
/// Un chemin absent n'est pas une panne : un projet sans crate de migration n'a pas de
/// `migration/src`, et un projet jamais compilé pas de `Cargo.lock`. Leur apparition
/// ajoute des entrées, ce qui invalide — l'absence n'a donc pas à être notée.
fn collecter(chemin: &Path, root: &Path, entrees: &mut Vec<(String, Vec<u8>)>) -> Option<()> {
    let Ok(genre) = fs::symlink_metadata(chemin) else {
        return Some(());
    };

    if genre.is_file() {
        // Relatif, et à barres obliques : le condensat d'un même projet doit être le même
        // sous Windows et ailleurs, faute de quoi un cache déposé par l'un serait toujours
        // manqué par l'autre.
        let relatif = chemin
            .strip_prefix(root)
            .ok()?
            .to_string_lossy()
            .replace('\\', "/");
        entrees.push((relatif, fs::read(chemin).ok()?));
        return Some(());
    }

    if !genre.is_dir() {
        // Un lien symbolique n'est ni suivi ni condensé : le suivre exposerait la marche à
        // une boucle, et aucun projet engendré n'en porte dans ses sources.
        return Some(());
    }

    for entree in fs::read_dir(chemin).ok()? {
        collecter(&entree.ok()?.path(), root, entrees)?;
    }

    Some(())
}

/// Le document mémorisé sous ce condensat, quand il est là et qu'il est du JSON.
///
/// L'analyse n'est pas un luxe : le fichier vit dans `target/`, qu'un build interrompu, un
/// disque plein ou une main humaine peuvent laisser tronqué. Le rendre tel quel ferait
/// échouer l'appelant sur un contrat que la recompilation aurait réparé.
fn memorise(root: &Path, condensat: &str) -> Option<String> {
    let repertoire = root.join(REPERTOIRE);

    let atteste = fs::read_to_string(repertoire.join(CONDENSAT)).ok()?;
    if atteste.trim() != condensat {
        return None;
    }

    let document = fs::read_to_string(repertoire.join(DOCUMENT)).ok()?;
    serde_json::from_str::<serde_json::Value>(&document).ok()?;

    Some(document)
}

/// Mémorise le document sous son condensat, ou renonce sans rien dire.
///
/// Le document d'abord, le condensat ensuite : coupée entre les deux, l'écriture laisse un
/// condensat qui ne désigne plus rien de ce que `memorise` acceptera. L'ordre inverse
/// laisserait une clé neuve sur un document périmé, et le cache mentirait.
fn memoriser(root: &Path, condensat: &str, document: &str) {
    let repertoire = root.join(REPERTOIRE);

    if fs::create_dir_all(&repertoire).is_err() {
        return;
    }
    if fs::write(repertoire.join(DOCUMENT), document).is_err() {
        return;
    }

    let _ = fs::write(repertoire.join(CONDENSAT), condensat);
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::convert::Infallible;

    use super::*;
    use crate::fixtures;

    /// Un contrat minimal, que `memorise` doit accepter comme du JSON.
    const CONTRAT: &str = r#"{"openapi":"3.1.0","paths":{}}"#;

    /// Le faux binaire : il compte ses lancements et rend le texte qu'on lui donne.
    struct Binaire {
        lancements: Cell<usize>,
        texte: String,
    }

    impl Binaire {
        fn new(texte: &str) -> Self {
            Self {
                lancements: Cell::new(0),
                texte: texte.to_string(),
            }
        }

        fn imprimer(&self) -> Result<String, Infallible> {
            self.lancements.set(self.lancements.get() + 1);
            Ok(self.texte.clone())
        }
    }

    fn au_travers_de(root: &Path, binaire: &Binaire) -> String {
        au_travers(root, || binaire.imprimer()).expect("le faux binaire n'échoue pas")
    }

    #[test]
    fn unchanged_sources_answer_without_running_the_binary_again() {
        let (_tmp, root) = fixtures::project();
        let binaire = Binaire::new(CONTRAT);

        assert_eq!(au_travers_de(&root, &binaire), CONTRAT);
        assert_eq!(au_travers_de(&root, &binaire), CONTRAT);

        assert_eq!(binaire.lancements.get(), 1);
    }

    #[test]
    fn a_touched_source_file_invalidates_the_memorised_contract() {
        let (_tmp, root) = fixtures::project();
        let binaire = Binaire::new(CONTRAT);
        au_travers_de(&root, &binaire);

        fs::write(root.join("src/router.rs"), "// autre chose\n").expect("le routeur s'écrit");

        au_travers_de(&root, &binaire);

        assert_eq!(binaire.lancements.get(), 2);
    }

    #[test]
    fn a_deleted_source_file_invalidates_the_memorised_contract() {
        let (_tmp, root) = fixtures::project();
        let binaire = Binaire::new(CONTRAT);
        au_travers_de(&root, &binaire);

        fs::remove_file(root.join("src/router.rs")).expect("le routeur se supprime");

        au_travers_de(&root, &binaire);

        assert_eq!(binaire.lancements.get(), 2);
    }

    /// Le contenu ne bouge pas, le chemin si : sans le chemin dans le condensat, le cache
    /// rendrait le contrat d'une arborescence qui n'existe plus.
    #[test]
    fn a_renamed_source_file_invalidates_the_memorised_contract() {
        let (_tmp, root) = fixtures::project();
        let binaire = Binaire::new(CONTRAT);
        au_travers_de(&root, &binaire);

        fs::rename(root.join("src/router.rs"), root.join("src/routeur.rs"))
            .expect("le routeur se renomme");

        au_travers_de(&root, &binaire);

        assert_eq!(binaire.lancements.get(), 2);
    }

    #[test]
    fn a_touched_lock_file_invalidates_the_memorised_contract() {
        let (_tmp, root) = fixtures::project();
        fs::write(root.join("Cargo.lock"), "version = 4\n").expect("le verrou s'écrit");
        let binaire = Binaire::new(CONTRAT);
        au_travers_de(&root, &binaire);

        fs::write(root.join("Cargo.lock"), "version = 3\n").expect("le verrou se réécrit");

        au_travers_de(&root, &binaire);

        assert_eq!(binaire.lancements.get(), 2);
    }

    /// Le cas que la migration seule peut produire : une source ajoutée hors de `src/`.
    #[test]
    fn a_new_migration_module_invalidates_the_memorised_contract() {
        let (_tmp, root) = fixtures::project();
        let binaire = Binaire::new(CONTRAT);
        au_travers_de(&root, &binaire);

        fs::write(
            root.join("migration/src/m20260101_000000_articles.rs"),
            "// une migration\n",
        )
        .expect("le module de migration s'écrit");

        au_travers_de(&root, &binaire);

        assert_eq!(binaire.lancements.get(), 2);
    }

    #[test]
    fn a_corrupted_memorised_document_is_recompiled_rather_than_returned() {
        let (_tmp, root) = fixtures::project();
        let binaire = Binaire::new(CONTRAT);
        au_travers_de(&root, &binaire);

        fs::write(root.join(REPERTOIRE).join(DOCUMENT), "{ tronqué").expect("le cache s'écrit");

        assert_eq!(au_travers_de(&root, &binaire), CONTRAT);
        assert_eq!(binaire.lancements.get(), 2);
    }

    #[test]
    fn an_unreadable_digest_is_recompiled_rather_than_refused() {
        let (_tmp, root) = fixtures::project();
        let binaire = Binaire::new(CONTRAT);
        au_travers_de(&root, &binaire);

        fs::remove_file(root.join(REPERTOIRE).join(CONDENSAT)).expect("le condensat se supprime");

        assert_eq!(au_travers_de(&root, &binaire), CONTRAT);
        assert_eq!(binaire.lancements.get(), 2);
    }

    /// `target/` occupé par un fichier : `create_dir_all` échoue, et rien ne doit remonter.
    #[test]
    fn a_target_that_cannot_be_written_degrades_to_a_plain_recompilation() {
        let (_tmp, root) = fixtures::project();
        fs::write(root.join("target"), "pas un répertoire").expect("le leurre s'écrit");
        let binaire = Binaire::new(CONTRAT);

        assert_eq!(au_travers_de(&root, &binaire), CONTRAT);
        assert_eq!(au_travers_de(&root, &binaire), CONTRAT);

        assert_eq!(binaire.lancements.get(), 2);
    }

    /// Le condensat ne doit pas dépendre de l'ordre où le système de fichiers livre les
    /// entrées : deux calculs sur le même arbre rendent la même chaîne.
    #[test]
    fn the_digest_of_an_unchanged_tree_is_stable() {
        let (_tmp, root) = fixtures::project();

        assert_eq!(condensat(&root), condensat(&root));
        assert!(condensat(&root).is_some_and(|digest| digest.len() == 64));
    }

    /// Le cache n'a pas à voir passer un échec : l'appelant ne mémorise que ce qui a été
    /// imprimé, et un projet qui ne compile pas doit retenter à chaque fois.
    #[test]
    fn a_failing_binary_memorises_nothing() {
        let (_tmp, root) = fixtures::project();

        let echec: Result<String, &str> = au_travers(&root, || Err("le projet ne compile pas"));

        assert_eq!(echec, Err("le projet ne compile pas"));
        assert!(!root.join(REPERTOIRE).join(DOCUMENT).exists());
    }
}
