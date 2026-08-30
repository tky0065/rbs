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
    /// `the_hand_edits_of_blog_auth_are_in_place`, sans lequel cette liste
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
    Exemple {
        nom: "file-drop",
        database_url: "postgres://rbs:rbs@localhost:5432/file_drop",
        // `rbs add redis` inscrit `mod cache;`, non `mod redis;`. L'ancre empile dans
        // l'ordre d'installation et doit rester triée : `uploads` est le nom de
        // ressource qui la laisse close derrière `storage`.
        features: &["redis", "mail", "storage"],
        crud: "uploads",
        // `owner_email` finit par `_email` : le DTO généré gagne sa contrainte d'email
        // sans qu'on l'écrive, et le courriel a un destinataire qui vient du modèle.
        champs: "title:string,owner_email:string,content_type:string,size:int",
        // Les trois briques câblées, et les trois fragments dont la permission
        // `dead_code` tombe parce qu'un handler les appelle enfin. C'est le point de cet
        // exemple, et `the_hand_edits_of_file_drop_are_in_place` en répond.
        edite_a_la_main: &[
            "src/uploads/service.rs",
            "src/uploads/controller.rs",
            "src/uploads/repository.rs",
            "src/uploads/mod.rs",
            "src/cache/mod.rs",
            "src/mail/mod.rs",
            "src/mail/service.rs",
            "src/storage/mod.rs",
            "templates/mail/depot.html",
        ],
    },
    Exemple {
        nom: "newsletter-queue",
        database_url: "postgres://rbs:rbs@localhost:5432/newsletter_queue",
        // L'ancre `features` empile les `mod` dans l'ordre d'installation et doit rester
        // triée : `jobs` puis `mail`, et une ressource qui les suit — ce qui écarte
        // `newsletter` comme nom de ressource.
        features: &["jobs", "mail"],
        crud: "subscribers",
        // `email` seul suffit à la contrainte de validation du DTO : la règle porte sur
        // le nom exact autant que sur le suffixe `_email`.
        champs: "email:string:unique,name:string,confirmed:bool",
        // Ce que montre cet exemple et qu'aucun autre ne montre : un job enfilé dans la
        // transaction qui l'a motivé. `the_hand_edits_of_newsletter_queue_are_in_place`
        // en répond.
        edite_a_la_main: &[
            "src/jobs/mod.rs",
            "src/jobs/demo.rs",
            "src/jobs/newsletter.rs",
            "src/mail/mod.rs",
            "src/mail/service.rs",
            "src/openapi.rs",
            "src/subscribers/dto.rs",
            "src/subscribers/repository.rs",
            "src/subscribers/service.rs",
            "src/subscribers/controller.rs",
            "src/subscribers/mod.rs",
            "src/seeds/subscribers.rs",
            "templates/mail/newsletter.html",
        ],
    },
];

const REGENERER: &str = "examples/README.md donne la commande de régénération";

fn example(nom: &str) -> &'static Exemple {
    EXEMPLES
        .iter()
        .find(|example| example.nom == nom)
        .unwrap_or_else(|| panic!("`{nom}` doit figurer dans `EXEMPLES`"))
}

#[test]
fn hello_crud_is_what_the_cli_produces_today() {
    assert_no_drift(example("hello-crud"));
}

#[test]
fn blog_auth_is_what_the_cli_produces_today() {
    assert_no_drift(example("blog-auth"));
}

#[test]
fn file_drop_is_what_the_cli_produces_today() {
    assert_no_drift(example("file-drop"));
}

#[test]
fn newsletter_queue_is_what_the_cli_produces_today() {
    assert_no_drift(example("newsletter-queue"));
}

fn assert_no_drift(example: &Exemple) {
    let parent = tempfile::TempDir::new().expect("répertoire temporaire créable");
    let frais = generate(parent.path(), example);

    let attendu = normalize_fingerprint(
        &common::empreinte(&common::depot().join("examples").join(example.nom)),
        example,
    );
    let obtenu = normalize_fingerprint(&common::empreinte(&frais), example);

    let ecarts = compare(&attendu, &obtenu);

    assert!(
        ecarts.is_empty(),
        "`examples/{}` a dérivé de ce que le CLI produit :\n{}\n\n{REGENERER}",
        example.nom,
        ecarts.join("\n")
    );
}

/// Rejoue les commandes qui ont produit l'exemple.
fn generate(parent: &Path, example: &Exemple) -> PathBuf {
    let noyau = common::noyau();

    assert_cmd::Command::cargo_bin("rbs")
        .expect("le binaire rbs doit être compilé")
        .current_dir(parent)
        .args([
            "new",
            example.nom,
            "--database-url",
            example.database_url,
            "--core-path",
            noyau.to_str().expect("chemin du noyau représentable"),
            // Les exemples commités portent `lang = "fr"` : sans ce flag, la comparaison
            // dériverait selon la locale de la machine qui régénère la fixture.
            "--lang",
            "fr",
            "--yes",
        ])
        .assert()
        .success();

    let racine = parent.join(example.nom);

    for feature in example.features {
        // `add` refuse d'écrire dans un working tree sale. `rbs new` initialise le dépôt
        // sans rien commiter, et chaque feature laisse à son tour de quoi arrêter la
        // suivante : le commit se prend avant chacune, non une fois pour toutes.
        common::commiter(&racine, &format!("avant {feature}"));

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
            example.crud,
            "--fields",
            example.champs,
            "--yes",
            "--force",
        ])
        .assert()
        .success();

    racine
}

fn normalize_fingerprint(
    empreinte: &common::Empreinte,
    example: &Exemple,
) -> BTreeMap<PathBuf, String> {
    empreinte
        .iter()
        // `Cargo.lock` est écrit par cargo à la première compilation, pas par `rbs new` :
        // l'exemple le porte, une génération fraîche non. Il est versionné pour que la
        // CI compile l'exemple à dépendances figées, et reste hors de la comparaison.
        .filter(|(chemin, _)| chemin.file_name().is_none_or(|nom| nom != "Cargo.lock"))
        .filter(|(chemin, _)| {
            !example
                .edite_a_la_main
                .iter()
                .any(|edite| chemin.as_path() == Path::new(edite))
        })
        .map(|(chemin, contenu)| {
            (
                PathBuf::from(mask_timestamp(&chemin.to_string_lossy())),
                normalize(contenu),
            )
        })
        .collect()
}

/// Quatre différences sont attendues entre l'exemple du dépôt et une génération fraîche,
/// et aucune ne trahit une dérive des templates.
fn normalize(contenu: &str) -> String {
    contenu
        .lines()
        // L'exemple porte les marqueurs que la documentation cite ; ils n'ont rien à
        // faire dans les templates, donc rien à faire dans la comparaison.
        .filter(|ligne| !is_marker(ligne))
        .map(|ligne| mask_timestamp(&mask_core_path(&mask_secret(ligne))))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_marker(ligne: &str) -> bool {
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
fn mask_core_path(ligne: &str) -> String {
    const CLE: &str = "path = ";

    if !ligne.trim_start().starts_with("rbs-core") {
        return ligne.to_string();
    }

    let Some(apres_cle) = ligne.find(CLE).map(|debut| debut + CLE.len()) else {
        return ligne.to_string();
    };

    // Windows canonicalise `--core-path` en chemin UNC (`\\?\D:\…`), que `toml_edit`
    // écrit en chaîne littérale — guillemets simples — pour n'avoir pas à échapper ses
    // antislashs. Les deux délimiteurs mènent donc au même `<NOYAU>`, sans quoi la
    // comparaison passe sur trois plateformes et tombe sur la quatrième.
    let Some(delimiteur) = ligne[apres_cle..]
        .chars()
        .next()
        .filter(|mark| *mark == '"' || *mark == '\'')
    else {
        return ligne.to_string();
    };

    let ouverture = apres_cle + delimiteur.len_utf8();
    let Some(fermeture) = ligne[ouverture..]
        .find(delimiteur)
        .map(|fin| ouverture + fin + delimiteur.len_utf8())
    else {
        return ligne.to_string();
    };

    format!("{}\"<NOYAU>\"{}", &ligne[..apres_cle], &ligne[fermeture..])
}

/// Neutralise un secret tiré à l'installation, que deux générations ne partagent jamais.
///
/// Seule la forme tirée — soixante-quatre hexadécimaux — est masquée : le placeholder
/// de `.env.example` reste comparé caractère par caractère, et une template qui cesserait
/// d'y déclarer la variable serait toujours signalée.
fn mask_secret(ligne: &str) -> String {
    const CLE: &str = "RBS_AUTH__SECRET=";

    let Some(valeur) = ligne.strip_prefix(CLE) else {
        return ligne.to_string();
    };

    if valeur.len() != 64
        || !valeur
            .chars()
            .all(|lettre| lettre.is_ascii_hexdigit() && !lettre.is_uppercase())
    {
        return ligne.to_string();
    }

    format!("{CLE}<SECRET>")
}

/// Remplace `m20260826_205243` par `m<STAMP>` : le nom d'une migration porte la date et
/// l'heure de sa création, qui diffèrent nécessairement d'une génération à l'autre.
fn mask_timestamp(texte: &str) -> String {
    let lettres: Vec<char> = texte.chars().collect();
    let mut output = String::with_capacity(texte.len());
    let mut i = 0;

    // `m` + AAAAMMJJ + `_` + HHMMSS, soit seize caractères.
    let horodatage_en = |start: usize| {
        start + 16 <= lettres.len()
            && lettres[start] == 'm'
            && lettres[start + 1..start + 9]
                .iter()
                .all(char::is_ascii_digit)
            && lettres[start + 9] == '_'
            && lettres[start + 10..start + 16]
                .iter()
                .all(char::is_ascii_digit)
    };

    while i < lettres.len() {
        if horodatage_en(i) {
            output.push_str("m<STAMP>");
            i += 16;
        } else {
            output.push(lettres[i]);
            i += 1;
        }
    }

    output
}

/// Ne montre que ce qui diffère : déverser deux projets entiers noierait l'écart.
fn compare(attendu: &BTreeMap<PathBuf, String>, obtenu: &BTreeMap<PathBuf, String>) -> Vec<String> {
    let mut ecarts = Vec::new();

    for (chemin, contenu) in attendu {
        match obtenu.get(chemin) {
            None => ecarts.push(format!("  - {} n'est plus produit", chemin.display())),
            Some(frais) if frais != contenu => {
                ecarts.push(format!(
                    "  ~ {} : {}",
                    chemin.display(),
                    first_difference(contenu, frais)
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

fn first_difference(attendu: &str, obtenu: &str) -> String {
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

/// Sans ce garde-fou, `mask_timestamp` pourrait ne rien masquer sans que le test de
/// non-dérive n'en souffre : il comparerait deux textes également non masqués, et
/// laisserait passer une dérive le jour où les horodatages coïncideraient.
#[test]
fn a_migration_timestamp_is_properly_masked() {
    assert_eq!(
        mask_timestamp("m20260826_205243_create_articles.rs"),
        "m<STAMP>_create_articles.rs"
    );
    assert_eq!(mask_timestamp("marge_20260826"), "marge_20260826");
    assert_eq!(mask_timestamp("m2026_court"), "m2026_court");
}

#[test]
fn the_region_markers_are_ignored() {
    assert!(is_marker("// region: routeur"));
    assert!(is_marker("    // endregion: routeur"));
    assert!(!is_marker("// la région parisienne"));
    assert!(!is_marker("let region = 1;"));
}

#[test]
fn the_core_path_is_neutralised() {
    let absolu = "rbs-core = { path = \"/Users/x/rs/crates/rbs-core\" }";
    let relatif = "rbs-core = { path = \"../../crates/rbs-core\" }";

    assert_eq!(normalize(absolu), normalize(relatif));
}

/// Les deux façons dont un chemin s'écrit en TOML mènent au même masque.
///
/// La ligne citée est celle qu'un runner `windows-latest` a réellement produite : le
/// chemin y est canonicalisé en UNC, et `toml_edit` l'écrit en chaîne littérale plutôt
/// que d'échapper chacun de ses antislashs. Un masquage qui ne connaissait que les
/// guillemets doubles rendait la comparaison verte sur Linux et macOS, rouge sur Windows.
#[test]
fn the_core_path_is_neutralised_whatever_its_quotes() {
    let unc = r"rbs-core = { path = '\\?\D:\a\rbs\rbs\crates\rbs-core' }";
    let relatif = "rbs-core = { path = \"../../crates/rbs-core\" }";

    assert_eq!(normalize(unc), normalize(relatif));
    assert!(!normalize(unc).contains("D:"), "{}", normalize(unc));
}

/// Le masque ne mange pas ce qui suit le chemin, quel que soit son délimiteur.
#[test]
fn the_features_survive_a_path_in_single_quotes() {
    let unc = r#"rbs-core = { path = '\\?\D:\a\rbs\crates\rbs-core' , features = ["auth"] }"#;

    assert_eq!(
        normalize(unc),
        "rbs-core = { path = \"<NOYAU>\" , features = [\"auth\"] }"
    );
}

/// Le masquage s'arrête au chemin : une feature perdue reste une dérive visible.
///
/// La ligne était autrefois remplacée en entier, du temps où elle ne portait que le
/// chemin. `blog-auth` est le premier exemple où elle porte aussi des features — les
/// effacer avec le chemin rendrait le test aveugle à un `add auth` qui ne les
/// installerait plus.
#[test]
fn the_core_features_stay_compared() {
    let avec = "rbs-core = { path = \"/Users/x/rs/crates/rbs-core\" , features = [\"auth\"] }";
    let sans = "rbs-core = { path = \"../../crates/rbs-core\" }";

    assert_ne!(normalize(avec), normalize(sans));
    assert!(normalize(avec).contains("features = [\"auth\"]"));
    assert!(!normalize(avec).contains("/Users/x"));
}

/// Deux secrets tirés se confondent, mais pas le placeholder de `.env.example`.
///
/// Masquer la ligne entière laisserait passer une template qui cesserait de déclarer la
/// variable, ou qui publierait son secret dans le fichier versionné.
#[test]
fn a_drawn_secret_is_neutralised_but_the_published_placeholder_is_not() {
    let premier =
        "RBS_AUTH__SECRET=afac42334b295f8e48e9aef0c0de0c0ad4a15780bf54910d993f99ec78c0b72a";
    let second =
        "RBS_AUTH__SECRET=0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0";
    let exemple = "RBS_AUTH__SECRET=changez-moi-par-un-secret-tire-au-hasard-de-32-octets-au-moins";

    assert_eq!(normalize(premier), normalize(second));
    assert_ne!(normalize(premier), normalize(exemple));
    assert_eq!(normalize(exemple), exemple);
}

/// Vérifie que la comparaison voit une dérive de contenu, et pas seulement de nom de
/// fichier : un test de non-dérive qui ne détecte rien est pire qu'aucun test.
#[test]
fn a_content_difference_is_reported() {
    let mut attendu = BTreeMap::new();
    attendu.insert(PathBuf::from("src/main.rs"), "fn main() {}".to_string());

    let mut obtenu = BTreeMap::new();
    obtenu.insert(PathBuf::from("src/main.rs"), "fn main() { () }".to_string());

    let ecarts = compare(&attendu, &obtenu);

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
fn each_example_file_is_tracked_by_git() {
    let mut non_suivis = Vec::new();

    for example in EXEMPLES {
        let racine = common::depot().join("examples").join(example.nom);

        non_suivis.extend(
            common::empreinte(&racine)
                .keys()
                .filter(|relatif| !is_tracked(&racine.join(relatif)))
                .map(|relatif| format!("  - {}/{}", example.nom, relatif.display())),
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
fn the_hand_edits_of_blog_auth_are_in_place() {
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

fn is_tracked(chemin: &Path) -> bool {
    std::process::Command::new("git")
        .args(["ls-files", "--error-unmatch"])
        .arg(chemin)
        .current_dir(common::depot())
        .output()
        .expect("git doit être lançable")
        .status
        .success()
}

/// Un exemple que `cargo fmt` reformate est un exemple qui ment.
///
/// `rbs add ci` pose un `cargo fmt --check` dans le projet de l'utilisateur : le code que
/// le CLI vient d'écrire doit y passer. La CI du dépôt ne le voyait pas — son `cargo fmt
/// --all --check` ne couvre que les membres du workspace, dont les exemples ne font pas
/// partie.
#[test]
fn each_example_passes_cargo_fmt() {
    for example in EXEMPLES {
        let racine = common::depot().join("examples").join(example.nom);

        let output = std::process::Command::new("cargo")
            .args(["fmt", "--check"])
            .current_dir(&racine)
            .output()
            .expect("cargo fmt doit être lançable");

        assert!(
            output.status.success(),
            "`cargo fmt --check` reformate `examples/{}` :\n{}\n\n{REGENERER}",
            example.nom,
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

/// Ce que `file-drop` ajoute à ce que le CLI produit, et que la comparaison exclut.
///
/// Sans ce test, `edite_a_la_main` serait une liste de neuf chemins hors de toute
/// surveillance : le câblage pourrait disparaître sans que rien ne le dise, et c'est
/// pourtant lui qui distingue cet exemple des deux autres.
#[test]
fn the_hand_edits_of_file_drop_are_in_place() {
    let racine = common::depot().join("examples").join("file-drop");
    let lire = |relatif: &str| {
        std::fs::read_to_string(racine.join(relatif))
            .unwrap_or_else(|erreur| panic!("{relatif} illisible : {erreur}"))
    };

    // Les trois fragments portent chacun une permission `dead_code` que leur commentaire
    // dit de retirer au premier appel. C'est ce retrait, et non le câblage lui-même, qui
    // fait de `clippy -D warnings` la preuve que les briques sont appelées : une seule
    // d'entre elles remise ferait passer un câblage disparu.
    for module in ["cache", "mail", "storage"] {
        let source = lire(&format!("src/{module}/mod.rs"));
        assert!(
            !source.contains("#![allow(dead_code)]"),
            "src/{module}/mod.rs : la permission de module tombe avec le premier appel, \
             et c'est ce que cet exemple montre"
        );
    }

    let service = lire("src/uploads/service.rs");
    // Les appels sont cherchés sans leur récepteur : rustfmt coupe une chaîne de méthodes
    // dès qu'elle dépasse, et `storage.put(` se retrouve sur deux lignes.
    for (brique, appel) in [
        ("le cache", ".invalidate_prefix(CACHE)"),
        ("le stockage", ".put(&content_key(id), content)"),
        ("le courriel", ".send_template("),
    ] {
        assert!(
            service.contains(appel),
            "src/uploads/service.rs n'appelle plus {brique} : « {appel} » absent"
        );
    }

    // La lecture du cache autant que son invalidation : un service qui n'écrirait que
    // dans le cache sans jamais le relire passerait les assertions ci-dessus.
    assert!(
        service.contains("cache.get::<u64>(&key)") && service.contains("cache.set(&key, &total)"),
        "le total doit être lu du cache et y être écrit :\n{service}"
    );

    // Les trois écritures invalident, et non une seule : chercher la simple présence de
    // l'appel laisserait passer deux routes sur trois servant un total périmé.
    assert_eq!(
        service.matches(".invalidate_prefix(CACHE)").count(),
        3,
        "la création, la mise à jour et la suppression doivent toutes trois invalider :\n{service}"
    );

    let controller = lire("src/uploads/controller.rs");
    for handler in ["put_content", "get_content", "head_content"] {
        assert!(
            controller.contains(&format!("pub async fn {handler}(")),
            "src/uploads/controller.rs : le handler `{handler}` a disparu"
        );
    }

    // Le gabarit ajouté à la main, et le lien dans son attribut : c'est là qu'une
    // variable mal nommée rend un lien vide sans que le corps le montre.
    let gabarit = lire("templates/mail/depot.html");
    assert!(
        gabarit.contains(r#"<a href="{{ link }}">"#),
        "templates/mail/depot.html : le href doit porter la variable du contexte"
    );
}

/// Ce que `newsletter-queue` porte et qu'aucune commande n'écrit.
///
/// Onze de ses fichiers sortent de la comparaison octet à octet, qui signalerait l'édition
/// elle-même. Sans ce test, ces onze chemins ne seraient sous aucune surveillance et le
/// câblage pourrait disparaître en silence.
#[test]
fn the_hand_edits_of_newsletter_queue_are_in_place() {
    let racine = common::depot().join("examples").join("newsletter-queue");
    let lire = |relatif: &str| {
        std::fs::read_to_string(racine.join(relatif))
            .unwrap_or_else(|erreur| panic!("{relatif} illisible : {erreur}"))
    };

    // Les deux fragments livrent une brique et aucune route, et chacun porte une
    // permission `dead_code` que son commentaire dit de retirer au premier appel. C'est
    // ce retrait qui fait de `clippy -D warnings` la preuve du câblage.
    for module in ["jobs", "mail"] {
        let source = lire(&format!("src/{module}/mod.rs"));
        assert!(
            !source.contains("#![allow(dead_code)]"),
            "src/{module}/mod.rs : la permission de module tombe avec le premier appel, \
             et c'est ce que cet exemple montre"
        );
    }

    // Le job de démonstration part avec son inscription : le laisser inscrit ferait passer
    // un exemple dont le registre ne porterait aucun job à lui.
    assert!(
        !racine.join("src/jobs/demo.rs").exists(),
        "src/jobs/demo.rs : le job d'exemple s'efface devant celui du projet"
    );
    let jobs = lire("src/jobs/mod.rs");
    assert!(
        jobs.contains("register::<newsletter::SendNewsletter>()") && !jobs.contains("demo::Log"),
        "src/jobs/mod.rs : le registre doit porter `SendNewsletter` et lui seul :\n{jobs}"
    );

    // Le job attend l'envoi au lieu de le détacher : c'est ce que le réessai exige, et
    // toute la différence que cet exemple sert à montrer.
    let newsletter = lire("src/jobs/newsletter.rs");
    assert!(
        newsletter.contains("impl Job for SendNewsletter")
            && newsletter.contains(".send_template(")
            && !newsletter.contains(".send_detached("),
        "src/jobs/newsletter.rs : le job rend l'échec au worker, il ne détache pas l'envoi :\n\
         {newsletter}"
    );

    // `send_detached` reste offert, et n'a plus que sa propre permission : la permission
    // de module retirée ci-dessus la rendait invisible.
    let mailer = lire("src/mail/service.rs");
    assert!(
        mailer.contains("#[allow(dead_code)]\n    pub fn send_detached"),
        "src/mail/service.rs : la fonction est conservée, sous une permission qui ne vaut \
         que pour elle"
    );

    // Le cœur de l'exemple. `jobs::enqueue` reçoit la transaction et non `db` : sur `db`,
    // les lettres survivraient au rollback qui les annule, et l'exemple montrerait
    // exactement ce que la file en base sert à éviter.
    let service = lire("src/subscribers/service.rs");
    for (raison, extrait) in [
        ("la transaction n'est pas ouverte", "db.begin().await?"),
        (
            "la lecture ne partage pas la transaction",
            "repository::confirmed(&transaction)",
        ),
        (
            "l'enfilage ne la partage pas",
            "jobs::enqueue(\n            &transaction,",
        ),
        ("rien ne la commite", "transaction.commit().await?"),
    ] {
        assert!(
            service.contains(extrait),
            "src/subscribers/service.rs : {raison} — « {extrait} » absent :\n{service}"
        );
    }

    // La lecture est générique sur la connexion : c'est ce qui lui permet de recevoir une
    // transaction, qui n'est pas un `DatabaseConnection`.
    let repository = lire("src/subscribers/repository.rs");
    assert!(
        repository.contains("pub async fn confirmed<C: ConnectionTrait>(db: &C)")
            && repository.contains("Column::Confirmed.eq(true)"),
        "src/subscribers/repository.rs : la porte des confirmés doit accepter une \
         transaction et filtrer :\n{repository}"
    );

    // `202` et non `200`, et la route déclarée à OpenAPI : une route montée mais absente
    // du document est une route que personne ne trouve.
    let controller = lire("src/subscribers/controller.rs");
    assert!(
        controller.contains("pub async fn broadcast(")
            && controller.contains("StatusCode::ACCEPTED"),
        "src/subscribers/controller.rs : la diffusion accuse réception, elle ne rend pas 200"
    );
    assert!(
        lire("src/openapi.rs").contains("crate::subscribers::controller::broadcast,"),
        "src/openapi.rs : la route de diffusion doit figurer au document"
    );

    // Avant `/subscribers/{id}`, faute de quoi `broadcast` serait lu comme un identifiant.
    let monte = lire("src/subscribers/mod.rs");
    let (Some(diffusion), Some(par_id)) = (
        monte.find("\"/subscribers/broadcast\""),
        monte.find("\"/subscribers/{id}\""),
    ) else {
        panic!("src/subscribers/mod.rs : les deux routes doivent être montées :\n{monte}");
    };
    assert!(
        diffusion < par_id,
        "src/subscribers/mod.rs : `/subscribers/broadcast` se monte avant `/subscribers/{{id}}`"
    );

    // Un abonné non confirmé, sans quoi le filtre de `confirmed` ne se verrait pas : quatre
    // lignes insérées, trois lettres enfilées.
    let seed = lire("src/seeds/subscribers.rs");
    assert_eq!(
        seed.matches("true").count(),
        3,
        "src/seeds/subscribers.rs : trois abonnés confirmés sur quatre, pour que le filtre \
         se voie :\n{seed}"
    );

    // Le gabarit ajouté à la main : celui que le fragment livre annonce un compte ouvert,
    // ce qu'aucune lettre ne peut réemployer.
    let gabarit = lire("templates/mail/newsletter.html");
    assert!(
        gabarit.contains("{{ name }}") && gabarit.contains("{{ body }}"),
        "templates/mail/newsletter.html : les deux variables du contexte doivent y être"
    );
}
