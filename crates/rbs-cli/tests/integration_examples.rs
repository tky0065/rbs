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

/// Ce qui a produit un exemple. Changer ces valeurs sans régénérer l'exemple fait
/// échouer la comparaison, ce qui est le comportement voulu.
struct Exemple {
    nom: &'static str,
    database_url: &'static str,
    /// Features installées par `rbs add`, dans l'ordre, avant le CRUD.
    features: &'static [&'static str],
    crud: &'static str,
    champs: &'static str,
    /// Ce qu'aucune commande ne produit : les fichiers que l'exemple retouche à la main.
    ///
    /// Ils sortent de la comparaison, faute de quoi elle signalerait l'édition
    /// elle-même. Ce qu'ils portent est vérifié à part — voir
    /// `les_editions_a_la_main_de_blog_auth_sont_en_place`, sans lequel cette liste
    /// serait une porte ouverte à la dérive qu'elle sert à déclarer.
    edite_a_la_main: &'static [&'static str],
}

const EXEMPLES: &[Exemple] = &[
    Exemple {
        nom: "hello-crud",
        database_url: "postgres://rbs:rbs@localhost:5432/hello_crud",
        features: &[],
        crud: "articles",
        champs: "title:string,body:text,published:bool",
        edite_a_la_main: &[],
    },
    Exemple {
        nom: "blog-auth",
        database_url: "postgres://rbs:rbs@localhost:5432/blog_auth",
        features: &["auth"],
        // `posts` plutôt qu'`articles`, que porte déjà `hello-crud` : ce qui distingue
        // cet exemple est la protection, pas la ressource. Le nom rend en prime l'ancre
        // `features` triée — elle empile les `mod` dans l'ordre d'installation, et
        // `mod auth; mod articles;` ferait broncher un `cargo fmt` dans le projet.
        crud: "posts",
        champs: "title:string,body:text,published:bool",
        edite_a_la_main: &[
            "src/posts/controller.rs",
            "src/posts/tests.rs",
            "src/auth/guard.rs",
        ],
    },
];

const REGENERER: &str = "examples/README.md donne la commande de régénération";

fn exemple(nom: &str) -> &'static Exemple {
    EXEMPLES
        .iter()
        .find(|exemple| exemple.nom == nom)
        .unwrap_or_else(|| panic!("`{nom}` doit figurer dans `EXEMPLES`"))
}

#[test]
fn hello_crud_est_celui_que_le_cli_produit_aujourd_hui() {
    verifier_non_derive(exemple("hello-crud"));
}

#[test]
fn blog_auth_est_celui_que_le_cli_produit_aujourd_hui() {
    verifier_non_derive(exemple("blog-auth"));
}

fn verifier_non_derive(exemple: &Exemple) {
    let parent = tempfile::TempDir::new().expect("répertoire temporaire créable");
    let frais = engendrer(parent.path(), exemple);

    let attendu = normaliser_empreinte(
        &common::empreinte(&common::depot().join("examples").join(exemple.nom)),
        exemple,
    );
    let obtenu = normaliser_empreinte(&common::empreinte(&frais), exemple);

    let ecarts = comparer(&attendu, &obtenu);

    assert!(
        ecarts.is_empty(),
        "`examples/{}` a dérivé de ce que le CLI produit :\n{}\n\n{REGENERER}",
        exemple.nom,
        ecarts.join("\n")
    );
}

/// Rejoue les commandes qui ont produit l'exemple.
fn engendrer(parent: &Path, exemple: &Exemple) -> PathBuf {
    let noyau = common::noyau();

    assert_cmd::Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent)
        .args([
            "new",
            exemple.nom,
            "--database-url",
            exemple.database_url,
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
            "--yes",
        ])
        .assert()
        .success();

    let racine = parent.join(exemple.nom);

    // `add` refuse d'écrire dans un working tree sale, et `rbs new` initialise le dépôt
    // sans rien commiter : sans ce commit, la première feature s'arrête avant d'écrire.
    if !exemple.features.is_empty() {
        common::commiter(&racine, "projet neuf");
    }

    for feature in exemple.features {
        assert_cmd::Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(&racine)
            .args(["add", feature, "--yes"])
            .assert()
            .success();
    }

    assert_cmd::Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(&racine)
        .args([
            "generate",
            "crud",
            exemple.crud,
            "--fields",
            exemple.champs,
            "--yes",
            "--force",
        ])
        .assert()
        .success();

    racine
}

fn normaliser_empreinte(
    empreinte: &common::Empreinte,
    exemple: &Exemple,
) -> BTreeMap<PathBuf, String> {
    empreinte
        .iter()
        // `Cargo.lock` est écrit par cargo à la première compilation, pas par `rbs new` :
        // l'exemple le porte, une génération fraîche non. Il est versionné pour que la
        // CI compile l'exemple à dépendances figées, et reste hors de la comparaison.
        .filter(|(chemin, _)| chemin.file_name().is_none_or(|nom| nom != "Cargo.lock"))
        .filter(|(chemin, _)| {
            !exemple
                .edite_a_la_main
                .iter()
                .any(|edite| chemin.as_path() == Path::new(edite))
        })
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
        .map(|ligne| masquer_horodatage(&masquer_chemin_du_noyau(ligne)))
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

/// Neutralise le chemin que `--core-path` canonicalise en absolu, et que l'exemple
/// versionné rend relatif pour rester portable.
///
/// Seule la valeur de `path` est masquée, et non la ligne entière : depuis qu'une feature
/// installe `rbs-core` avec les siennes, cette ligne porte davantage que le chemin, et
/// tout en effacer laisserait passer un `add` qui cesserait d'ajouter sa feature.
fn masquer_chemin_du_noyau(ligne: &str) -> String {
    const CHEMIN: &str = "path = \"";

    if !ligne.trim_start().starts_with("rbs-core") {
        return ligne.to_string();
    }

    let Some(ouverture) = ligne.find(CHEMIN).map(|debut| debut + CHEMIN.len()) else {
        return ligne.to_string();
    };
    let Some(fermeture) = ligne[ouverture..].find('"').map(|fin| ouverture + fin) else {
        return ligne.to_string();
    };

    format!("{}<NOYAU>{}", &ligne[..ouverture], &ligne[fermeture..])
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

/// Le masquage s'arrête au chemin : une feature perdue reste une dérive visible.
///
/// La ligne était autrefois remplacée en entier, du temps où elle ne portait que le
/// chemin. `blog-auth` est le premier exemple où elle porte aussi des features — les
/// effacer avec le chemin rendrait le test aveugle à un `add auth` qui ne les
/// installerait plus.
#[test]
fn les_features_du_noyau_restent_comparees() {
    let avec = "rbs-core = { path = \"/Users/x/rs/crates/rbs-core\" , features = [\"auth\"] }";
    let sans = "rbs-core = { path = \"../../crates/rbs-core\" }";

    assert_ne!(normaliser(avec), normaliser(sans));
    assert!(normaliser(avec).contains("features = [\"auth\"]"));
    assert!(!normaliser(avec).contains("/Users/x"));
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
fn chaque_fichier_des_exemples_est_suivi_par_git() {
    let mut non_suivis = Vec::new();

    for exemple in EXEMPLES {
        let racine = common::depot().join("examples").join(exemple.nom);

        non_suivis.extend(
            common::empreinte(&racine)
                .keys()
                .filter(|relatif| !est_suivi(&racine.join(relatif)))
                .map(|relatif| format!("  - {}/{}", exemple.nom, relatif.display())),
        );
    }

    assert!(
        non_suivis.is_empty(),
        "ces fichiers manqueraient à un clone neuf, où la comparaison échouerait :\n{}\n\n\
         les suivre par `git add -f`, le `.gitignore` du projet généré ne devant pas bouger",
        non_suivis.join("\n")
    );
}

/// Les trois fichiers exclus de la comparaison portent-ils encore ce pour quoi ils le sont ?
///
/// `blog-auth` existe pour montrer une ressource protégée. Ses trois éditions à la main
/// sortent de la comparaison de non-dérive, qui signalerait sinon l'édition elle-même —
/// et rien, alors, ne verrait un `require_role` disparu au fil d'une régénération.
/// C'est exactement le mensonge que le fichier voisin sert à empêcher.
#[test]
fn les_editions_a_la_main_de_blog_auth_sont_en_place() {
    let racine = common::depot().join("examples").join("blog-auth");
    let lire = |relatif: &str| {
        std::fs::read_to_string(racine.join(relatif))
            .unwrap_or_else(|erreur| panic!("{relatif} illisible : {erreur}"))
    };

    let controller = lire("src/posts/controller.rs");
    assert_eq!(
        controller.matches("require_role(Role::Admin)").count(),
        3,
        "les trois mutations doivent porter la garde"
    );
    assert!(
        !controller.contains("pub async fn list(identite"),
        "la lecture reste publique : c'est ce qui distingue le 401 de l'extracteur du 403 de la garde"
    );

    let guard = lire("src/auth/guard.rs");
    assert!(
        !guard.contains("#[allow(dead_code)]"),
        "la garde a un appelant ici — le fragment prescrit lui-même de retirer cette ligne"
    );

    let tests = lire("src/posts/tests.rs");
    for nom in [
        "sans_jeton_la_creation_rend_401",
        "un_user_ne_peut_pas_creer_403",
    ] {
        assert!(tests.contains(nom), "`{nom}` a disparu de l'exemple");
    }
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
