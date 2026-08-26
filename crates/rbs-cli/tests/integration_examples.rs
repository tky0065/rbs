//! L'exemple versionné est-il encore ce que le CLI produit aujourd'hui ?
//!
//! Les extraits de la documentation sont lus dans `examples/`, jamais écrits à la main.
//! Le jour où une template change, l'exemple commité ne bouge pas de lui-même et les
//! pages se mettent à montrer un code que le CLI ne produit plus. Rien, dans une
//! compilation, ne le signale : l'exemple compile toujours, il est simplement périmé.
//! Ce test est le seul endroit d'où ce mensonge est visible.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod common;

/// Les paramètres qui ont produit `examples/hello-crud`. Les changer ici sans régénérer
/// l'exemple fait échouer ce test, ce qui est le comportement voulu.
const NOM: &str = "hello-crud";
const DATABASE_URL: &str = "postgres://rbs:rbs@localhost:5432/hello_crud";
const FEATURE: &str = "articles";
const CHAMPS: &str = "title:string,body:text,published:bool";

const REGENERER: &str = "examples/README.md donne la commande de régénération";

#[test]
fn l_exemple_versionne_est_celui_que_le_cli_produit_aujourd_hui() {
    let parent = tempfile::TempDir::new().expect("répertoire temporaire créable");
    let frais = engendrer(parent.path());

    let attendu = normaliser_empreinte(&common::empreinte(
        &common::depot().join("examples").join(NOM),
    ));
    let obtenu = normaliser_empreinte(&common::empreinte(&frais));

    let ecarts = comparer(&attendu, &obtenu);

    assert!(
        ecarts.is_empty(),
        "`examples/{NOM}` a dérivé de ce que le CLI produit :\n{}\n\n{REGENERER}",
        ecarts.join("\n")
    );
}

/// Rejoue les commandes qui ont produit l'exemple.
fn engendrer(parent: &Path) -> PathBuf {
    let noyau = common::noyau();

    assert_cmd::Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent)
        .args([
            "new",
            NOM,
            "--database-url",
            DATABASE_URL,
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    let racine = parent.join(NOM);

    assert_cmd::Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&racine)
        .args([
            "generate", "crud", FEATURE, "--fields", CHAMPS, "--yes", "--force",
        ])
        .assert()
        .success();

    racine
}

fn normaliser_empreinte(empreinte: &common::Empreinte) -> BTreeMap<PathBuf, String> {
    empreinte
        .iter()
        // `Cargo.lock` est écrit par cargo à la première compilation, pas par `rbs new` :
        // l'exemple le porte, une génération fraîche non. Il est versionné pour que la
        // CI compile l'exemple à dépendances figées, et reste hors de la comparaison.
        .filter(|(chemin, _)| chemin.file_name().is_none_or(|nom| nom != "Cargo.lock"))
        .map(|(chemin, contenu)| {
            (
                PathBuf::from(masquer_horodatage(&chemin.to_string_lossy())),
                normaliser(contenu),
            )
        })
        .collect()
}

/// Trois différences sont attendues entre l'exemple du dépôt et une génération fraîche,
/// et aucune ne trahit une dérive des templates.
fn normaliser(contenu: &str) -> String {
    contenu
        .lines()
        // L'exemple porte les marqueurs que la documentation cite ; ils n'ont rien à
        // faire dans les templates, donc rien à faire dans la comparaison.
        .filter(|ligne| !est_marqueur(ligne))
        // `--core-path` est canonicalisé en chemin absolu, que l'exemple versionné rend
        // relatif pour rester portable.
        .map(|ligne| {
            if ligne.trim_start().starts_with("rbs-core") {
                "rbs-core = <NOYAU>".to_string()
            } else {
                masquer_horodatage(ligne)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn est_marqueur(ligne: &str) -> bool {
    let nu = ligne.trim_start();
    let Some(reste) = nu.strip_prefix("//") else {
        return false;
    };
    let reste = reste.trim_start();
    reste.starts_with("region:") || reste.starts_with("endregion:")
}

/// Remplace `m20260826_205243` par `m<STAMP>` : le nom d'une migration porte la date et
/// l'heure de sa création, qui diffèrent nécessairement d'une génération à l'autre.
fn masquer_horodatage(texte: &str) -> String {
    let lettres: Vec<char> = texte.chars().collect();
    let mut sortie = String::with_capacity(texte.len());
    let mut i = 0;

    // `m` + AAAAMMJJ + `_` + HHMMSS, soit seize caractères.
    let horodatage_en = |depart: usize| {
        depart + 16 <= lettres.len()
            && lettres[depart] == 'm'
            && lettres[depart + 1..depart + 9]
                .iter()
                .all(char::is_ascii_digit)
            && lettres[depart + 9] == '_'
            && lettres[depart + 10..depart + 16]
                .iter()
                .all(char::is_ascii_digit)
    };

    while i < lettres.len() {
        if horodatage_en(i) {
            sortie.push_str("m<STAMP>");
            i += 16;
        } else {
            sortie.push(lettres[i]);
            i += 1;
        }
    }

    sortie
}

/// Ne montre que ce qui diffère : déverser deux projets entiers noierait l'écart.
fn comparer(
    attendu: &BTreeMap<PathBuf, String>,
    obtenu: &BTreeMap<PathBuf, String>,
) -> Vec<String> {
    let mut ecarts = Vec::new();

    for (chemin, contenu) in attendu {
        match obtenu.get(chemin) {
            None => ecarts.push(format!("  - {} n'est plus produit", chemin.display())),
            Some(frais) if frais != contenu => {
                ecarts.push(format!(
                    "  ~ {} : {}",
                    chemin.display(),
                    premiere_difference(contenu, frais)
                ));
            }
            Some(_) => {}
        }
    }

    for chemin in obtenu.keys() {
        if !attendu.contains_key(chemin) {
            ecarts.push(format!(
                "  + {} est produit mais absent de l'exemple",
                chemin.display()
            ));
        }
    }

    ecarts
}

fn premiere_difference(attendu: &str, obtenu: &str) -> String {
    for (rang, (a, o)) in attendu.lines().zip(obtenu.lines()).enumerate() {
        if a != o {
            return format!(
                "ligne {}, « {} » contre « {} »",
                rang + 1,
                a.trim(),
                o.trim()
            );
        }
    }

    format!(
        "{} lignes contre {}",
        attendu.lines().count(),
        obtenu.lines().count()
    )
}

/// Sans ce garde-fou, `masquer_horodatage` pourrait ne rien masquer sans que le test de
/// non-dérive n'en souffre : il comparerait deux textes également non masqués, et
/// laisserait passer une dérive le jour où les horodatages coïncideraient.
#[test]
fn l_horodatage_d_une_migration_est_bien_masque() {
    assert_eq!(
        masquer_horodatage("m20260826_205243_create_articles.rs"),
        "m<STAMP>_create_articles.rs"
    );
    assert_eq!(masquer_horodatage("marge_20260826"), "marge_20260826");
    assert_eq!(masquer_horodatage("m2026_court"), "m2026_court");
}

#[test]
fn les_marqueurs_de_region_sont_ignores() {
    assert!(est_marqueur("// region: routeur"));
    assert!(est_marqueur("    // endregion: routeur"));
    assert!(!est_marqueur("// la région parisienne"));
    assert!(!est_marqueur("let region = 1;"));
}

#[test]
fn le_chemin_du_noyau_est_neutralise() {
    let absolu = "rbs-core = { path = \"/Users/x/rs/crates/rbs-core\" }";
    let relatif = "rbs-core = { path = \"../../crates/rbs-core\" }";

    assert_eq!(normaliser(absolu), normaliser(relatif));
}

/// Vérifie que la comparaison voit une dérive de contenu, et pas seulement de nom de
/// fichier : un test de non-dérive qui ne détecte rien est pire qu'aucun test.
#[test]
fn une_difference_de_contenu_est_signalee() {
    let mut attendu = BTreeMap::new();
    attendu.insert(PathBuf::from("src/main.rs"), "fn main() {}".to_string());

    let mut obtenu = BTreeMap::new();
    obtenu.insert(PathBuf::from("src/main.rs"), "fn main() { () }".to_string());

    let ecarts = comparer(&attendu, &obtenu);

    assert_eq!(ecarts.len(), 1, "{ecarts:?}");
    assert!(ecarts[0].contains("src/main.rs"), "{ecarts:?}");
}

/// Un clone neuf reproduit-il l'exemple tel quel ?
///
/// Le test de non-dérive compare l'exemple versionné à une génération fraîche. Encore
/// faut-il que « versionné » soit vrai de chaque fichier : le `.gitignore` que `rbs new`
/// écrit dans le projet ignore `.env`, que le CLI produit pourtant. Sur une machine de
/// développement le fichier traîne depuis la génération et la comparaison passe ; sur un
/// checkout de CI il n'existe pas et elle échoue. Rien, dans la comparaison elle-même, ne
/// distingue les deux situations.
///
/// Ce test ne relève que les fichiers **présents et non suivis** : sur un checkout où le
/// fichier manque déjà, il n'a rien à voir et c'est la comparaison qui tombe. C'est voulu.
/// Il garde la machine qui engendre l'exemple, seul endroit où l'oubli s'introduit.
#[test]
fn chaque_fichier_de_l_exemple_est_suivi_par_git() {
    let racine = common::depot().join("examples").join(NOM);

    let non_suivis: Vec<String> = common::empreinte(&racine)
        .keys()
        .filter(|relatif| !est_suivi(&racine.join(relatif)))
        .map(|relatif| format!("  - {}", relatif.display()))
        .collect();

    assert!(
        non_suivis.is_empty(),
        "ces fichiers manqueraient à un clone neuf, où la comparaison échouerait :\n{}\n\n\
         les suivre par `git add -f`, le `.gitignore` du projet généré ne devant pas bouger",
        non_suivis.join("\n")
    );
}

fn est_suivi(chemin: &Path) -> bool {
    std::process::Command::new("git")
        .args(["ls-files", "--error-unmatch"])
        .arg(chemin)
        .current_dir(common::depot())
        .output()
        .expect("git doit être lançable")
        .status
        .success()
}
