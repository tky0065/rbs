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
    /// Les tables de `rbs generate crud`, dans l'ordre : son nom, puis ses `--fields`.
    ///
    /// Une seule partout sauf dans `help-desk`, dont la seconde table référence la
    /// première : l'ordre est celui des clés étrangères. `role` et `with_upload`
    /// s'appliquent à chacune.
    cruds: &'static [(&'static str, &'static str)],
    /// `--role` sur `generate crud` : le rôle qu'exigent les écritures.
    ///
    /// Seul `blog-auth` le porte — c'est le seul exemple sous `auth`, et l'option refuse
    /// ailleurs.
    role: Option<&'static str>,
    /// `--with-upload` sur `generate crud` : engendre les trois routes de contenu binaire.
    ///
    /// Seul `file-drop` le porte — les autres exemples n'ont rien à déposer.
    with_upload: bool,
    /// Ce qu'aucune commande ne produit : les fichiers que l'exemple retouche à la main.
    ///
    /// Ils sortent de la comparaison, faute de quoi elle signalerait l'édition
    /// elle-même. Ce qu'ils portent est vérifié à part — voir
    /// `the_hand_edits_of_blog_auth_are_in_place`, sans lequel cette liste
    /// serait une porte ouverte à la dérive qu'elle sert à déclarer.
    edite_a_la_main: &'static [&'static str],
    /// Ce qu'une *autre* commande produit : les fichiers qu'un second appel dépose.
    ///
    /// Distinct d'`edite_a_la_main` — rien ici n'est écrit à la main — et hors de la
    /// comparaison pour la même raison : la génération de référence ne lance pas cette
    /// commande-là. `the_typescript_client_of_hello_crud_is_in_place` en répond.
    engendre_a_part: &'static [&'static str],
}

const EXEMPLES: &[Exemple] = &[
    Exemple {
        nom: "hello-crud",
        database_url: "postgres://rbs:rbs@localhost:5432/hello_crud",
        features: &[],
        cruds: &[("articles", "title:string,body:text,published:bool")],
        role: None,
        with_upload: false,
        edite_a_la_main: &[],
        engendre_a_part: &["clients/ts/client.ts"],
    },
    Exemple {
        nom: "blog-auth",
        database_url: "postgres://rbs:rbs@localhost:5432/blog_auth",
        features: &["auth"],
        // `posts` plutôt qu'`articles`, que porte déjà `hello-crud` : ce qui distingue
        // cet exemple est la protection, pas la ressource. Le nom rend en prime l'ancre
        // `features` triée — elle empile les `mod` dans l'ordre d'installation, et
        // `mod auth; mod articles;` ferait broncher un `cargo fmt` dans le projet.
        cruds: &[("posts", "title:string,body:text,published:bool")],
        // Les lectures restent au seuil que `generate crud` pose seul, les écritures
        // montent : l'exemple porte les deux régimes, et sa promesse — seul un admin
        // écrit — n'a plus à être tenue à la main.
        role: Some("admin"),
        with_upload: false,
        // Le contrôleur et la garde en sont sortis : `generate crud` pose lui-même le
        // `require_role` sous `auth`, et le `#[allow(dead_code)]` que la template porte
        // reste ici tel quel — le retirer coûtait la surveillance du fichier entier pour
        // une ligne dont aucune route ne dépend.
        edite_a_la_main: &["src/posts/tests/access.rs"],
        engendre_a_part: &[],
    },
    Exemple {
        nom: "file-drop",
        database_url: "postgres://rbs:rbs@localhost:5432/file_drop",
        // `rbs add redis` inscrit `mod cache;`, non `mod redis;`, dans l'ancre
        // `modules` — distincte de `features`, qui ne porte plus que `uploads`.
        features: &["redis", "mail", "storage"],
        // `owner_email` finit par `_email` : le DTO généré gagne sa contrainte d'email
        // sans qu'on l'écrive, et le courriel a un destinataire qui vient du modèle.
        cruds: &[(
            "uploads",
            "title:string,owner_email:string,content_type:string,size:int",
        )],
        role: None,
        // Les trois routes de contenu binaire, sans quoi elles resteraient écrites à la
        // main alors que c'est précisément ce que ce drapeau produit.
        with_upload: true,
        // Les trois briques câblées, et les trois fragments dont la permission
        // `dead_code` tombe parce qu'un handler les appelle enfin. C'est le point de cet
        // exemple, et `the_hand_edits_of_file_drop_are_in_place` en répond.
        // `mod.rs` en est sorti : le drapeau engendre désormais sa route et sa borne de
        // taille à l'identique, marqueurs de région compris.
        edite_a_la_main: &[
            "src/uploads/service.rs",
            "src/uploads/controller.rs",
            "src/uploads/repository.rs",
            "src/modules/cache/mod.rs",
            "src/modules/mail/mod.rs",
            "src/modules/mail/service.rs",
            "src/modules/storage/mod.rs",
            "templates/mail/depot.html",
        ],
        engendre_a_part: &[],
    },
    Exemple {
        nom: "newsletter-queue",
        database_url: "postgres://rbs:rbs@localhost:5432/newsletter_queue",
        // Les trois fragments s'installent dans l'ancre `modules`, triée indépendamment
        // de l'ordre d'arrivée des `add` : `jobs`, `mail`, `observability` y figurent
        // alphabétiquement quel que soit l'ordre choisi ici.
        features: &["jobs", "mail", "observability"],
        // `email` seul suffit à la contrainte de validation du DTO : la règle porte sur
        // le nom exact autant que sur le suffixe `_email`.
        cruds: &[(
            "subscribers",
            "email:string:unique,name:string,confirmed:bool",
        )],
        role: None,
        with_upload: false,
        // Ce que montre cet exemple et qu'aucun autre ne montre : un job enfilé dans la
        // transaction qui l'a motivé. `the_hand_edits_of_newsletter_queue_are_in_place`
        // en répond.
        edite_a_la_main: &[
            "src/modules/jobs/mod.rs",
            "src/modules/jobs/demo.rs",
            "src/modules/jobs/newsletter.rs",
            "src/modules/mail/mod.rs",
            "src/modules/mail/service.rs",
            "src/openapi.rs",
            "src/subscribers/dto.rs",
            "src/subscribers/repository.rs",
            "src/subscribers/service.rs",
            "src/subscribers/controller.rs",
            "src/subscribers/mod.rs",
            "src/seeds/subscribers.rs",
            "templates/mail/newsletter.html",
            "prometheus.yml",
        ],
        engendre_a_part: &[],
    },
    Exemple {
        nom: "event-hub",
        database_url: "postgres://rbs:rbs@localhost:5432/event_hub",
        // `webhooks` tire `jobs` et `auth`, et par elle `mail` et `rate-limit` : les cinq
        // descendent d'un seul plan, dont les trois migrations portent le même horodatage.
        // C'est le seul exemple où l'ancre `migration_modules` doit trier des noms nés dans
        // la même seconde.
        features: &[
            "webhooks",
            "scheduler",
            "audit",
            "cors",
            "docker",
            "ci",
            "api-keys",
        ],
        cruds: &[("orders", "reference:string,amount:int")],
        role: None,
        with_upload: false,
        // La création d'une commande trace et émet dans sa propre transaction : c'est ce
        // que ce projet montre, et `the_hand_edits_of_event_hub_are_in_place` en répond.
        edite_a_la_main: &[
            "src/orders/repository.rs",
            "src/orders/service.rs",
            "src/orders/controller.rs",
        ],
        engendre_a_part: &[],
    },
    Exemple {
        nom: "admin-console",
        database_url: "postgres://rbs:rbs@localhost:5432/admin_console",
        // `frontend-admin` tire `frontend` et `auth`, et par elle `mail` et `rate-limit` :
        // cinq fragments descendent d'un seul plan. `cors` vient avant, pour que la couche
        // qu'il pose enveloppe celle de la limite de débit — c'est aussi ce qui rend le
        // serveur de développement de Vite joignable depuis son propre port.
        features: &["cors", "frontend-admin"],
        // Huit colonnes, et huit contrôles à couvrir : une chaîne, un texte long, une
        // énumération, un booléen, un entier facultatif, une date facultative et un
        // instant. C'est le seul endroit du dépôt où le formulaire engendré est compilé
        // pour de bon, et une seule forme non couverte ici ne l'est nulle part.
        cruds: &[(
            "incidents",
            "reference:string:unique,sujet:string,detail:text,\
             gravite:enum(basse,moyenne,haute),ouvert:bool,duree_minutes:int:optional,\
             echeance:date:optional,constate_le:datetime",
        )],
        role: None,
        with_upload: false,
        edite_a_la_main: &[],
        // Le client typé que le shell importe. Le rejeu ne lance pas la commande qui
        // l'écrit — elle compile le projet — et c'est le job `admin-console · frontend`
        // qui répond de lui, en le régénérant puis en exigeant qu'il n'ait pas bougé.
        engendre_a_part: &["frontend/src/api/client.ts"],
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

#[test]
fn admin_console_is_what_the_cli_produces_today() {
    assert_no_drift(example("admin-console"));
}

#[test]
fn event_hub_is_what_the_cli_produces_today() {
    assert_no_drift(example("event-hub"));
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
            .args(["add", feature])
            .assert()
            .success();
    }

    for &(crud, champs) in example.cruds {
        let mut args = vec!["generate", "crud", crud, "--fields", champs, "--force"];
        if let Some(role) = example.role {
            args.extend(["--role", role]);
        }
        if example.with_upload {
            args.push("--with-upload");
        }

        assert_cmd::Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(&racine)
            .args(args)
            .assert()
            .success();
    }

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
        .filter(|(chemin, _)| {
            !example
                .engendre_a_part
                .iter()
                .any(|engendre| chemin.as_path() == Path::new(engendre))
        })
        .map(|(chemin, contenu)| {
            (
                PathBuf::from(mask_timestamp(&chemin.to_string_lossy())),
                sort_migration_modules(&normalize(contenu)),
            )
        })
        .collect()
}

/// Cinq différences sont attendues entre l'exemple du dépôt et une génération fraîche, et
/// aucune ne trahit une dérive des templates : les quatre que masque cette fonction, plus
/// l'ordre des `mod` de migration, que [`sort_migration_modules`] neutralise à son tour
/// dans `normalize_fingerprint`.
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

/// Les deux syntaxes de commentaire que reconnaît `docs/plugins/remark-code-from-file.js` :
/// `//` pour le Rust, `#` pour le TOML et le YAML. N'en admettre qu'une ici ferait passer
/// pour une dérive un marqueur posé dans un `config/default.toml` que la documentation
/// cite.
fn is_marker(ligne: &str) -> bool {
    let nu = ligne.trim_start();
    let Some(reste) = nu.strip_prefix("//").or_else(|| nu.strip_prefix('#')) else {
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
/// Une clé par `[[env]] secret = true` du catalogue : le fragment `auth` en déclare deux,
/// le secret de signature et le mot de passe du compte d'administration, et les deux
/// atterrissent dans le `.env` versionné de trois exemples.
///
/// Seule la forme tirée — soixante-quatre hexadécimaux — est masquée : le placeholder
/// de `.env.example` reste comparé caractère par caractère, et une template qui cesserait
/// d'y déclarer la variable serait toujours signalée.
fn mask_secret(ligne: &str) -> String {
    const CLES: [&str; 2] = ["RBS_AUTH__SECRET=", "ADMIN_PASSWORD="];

    for cle in CLES {
        let Some(valeur) = ligne.strip_prefix(cle) else {
            continue;
        };

        if valeur.len() != 64
            || !valeur
                .chars()
                .all(|lettre| lettre.is_ascii_hexdigit() && !lettre.is_uppercase())
        {
            return ligne.to_string();
        }

        return format!("{cle}<SECRET>");
    }

    ligne.to_string()
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

/// Neutralise l'ordre des lignes `mod m<STAMP>_…;` de l'ancre `migration_modules`, que
/// rustfmt trie déjà par nom complet — donc par horodatage d'abord. Deux générations ne
/// tombent jamais dans les mêmes secondes : quand deux commandes partagent la leur,
/// `create_audit_log` passe devant `create_schedules`, une minute d'écart plus tard il
/// passe derrière. L'horodatage étant déjà masqué par [`mask_timestamp`], l'ordre qu'il
/// dicte doit l'être aussi, sans quoi la comparaison verrait une dérive là où seule
/// l'horloge a tourné différemment entre deux régénérations. L'ordre d'exécution, lui, vit
/// dans le `vec!` du `Migrator` — anchor `migrations`, non triée — et reste comparé tel
/// quel ; `each_example_passes_cargo_fmt` répond du tri de l'exemple committé.
fn sort_migration_modules(contenu: &str) -> String {
    const PREFIXE: &str = "mod m<STAMP>_";

    let lignes: Vec<&str> = contenu.lines().collect();
    let mut resultat: Vec<&str> = Vec::with_capacity(lignes.len());
    let mut i = 0;

    while i < lignes.len() {
        if lignes[i].starts_with(PREFIXE) {
            let debut = i;
            while i < lignes.len() && lignes[i].starts_with(PREFIXE) {
                i += 1;
            }
            let mut groupe = lignes[debut..i].to_vec();
            groupe.sort_unstable();
            resultat.extend(groupe);
        } else {
            resultat.push(lignes[i]);
            i += 1;
        }
    }

    resultat.join("\n")
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

/// L'ancre `migration_modules` trie ses `mod` par nom complet, donc par horodatage
/// d'abord : trois migrations nées à trois secondes distinctes ne se trient pas comme
/// trois migrations nées à la même seconde, bien que ce ne soit là qu'un effet de
/// l'horloge. La comparaison doit voir les deux comme identiques. L'ordre d'exécution du
/// `Migrator`, lui, ne dépend d'aucune horloge : une inversion y reste une dérive.
#[test]
fn the_order_of_migration_modules_is_masked_with_their_timestamp() {
    let committed = "\
mod m20260913_131801_create_auth_tables;
mod m20260913_131808_create_schedules;
mod m20260913_131820_create_audit_log;

Box::new(m20260913_131801_create_auth_tables::Migration),
Box::new(m20260913_131820_create_audit_log::Migration),
";
    let fresh = "\
mod m20260913_131801_create_audit_log;
mod m20260913_131801_create_auth_tables;
mod m20260913_131801_create_schedules;

Box::new(m20260913_131801_create_auth_tables::Migration),
Box::new(m20260913_131801_create_audit_log::Migration),
";

    assert_eq!(
        sort_migration_modules(&normalize(committed)),
        sort_migration_modules(&normalize(fresh))
    );

    let execution_inversee = "\
mod m20260913_131801_create_audit_log;
mod m20260913_131801_create_auth_tables;
mod m20260913_131801_create_schedules;

Box::new(m20260913_131820_create_audit_log::Migration),
Box::new(m20260913_131801_create_auth_tables::Migration),
";

    assert_ne!(
        sort_migration_modules(&normalize(fresh)),
        sort_migration_modules(&normalize(execution_inversee))
    );
}

#[test]
fn the_region_markers_are_ignored() {
    assert!(is_marker("// region: routeur"));
    assert!(is_marker("    // endregion: routeur"));
    assert!(is_marker("# region: metriques"));
    assert!(is_marker("  # endregion: metriques"));
    assert!(!is_marker("// la région parisienne"));
    assert!(!is_marker("# le port de la region"));
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

    // Le mot de passe du compte d'administration est tiré de la même façon, et atterrit
    // dans le même fichier versionné : sans lui, la comparaison signalerait une dérive à
    // chaque génération.
    let mot_de_passe =
        "ADMIN_PASSWORD=1f3c9a7e5b2d8064af1e3c5970b2d846e1c3a597f0b2d8461f3c9a7e5b2d8064";
    let autre = "ADMIN_PASSWORD=0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a69788796a5b4c3d2e1f0";
    let repere = "ADMIN_PASSWORD=changez-moi-ce-mot-de-passe-est-publie-dans-git";

    assert_eq!(normalize(mot_de_passe), normalize(autre));
    assert_eq!(normalize(repere), repere);

    // L'adresse, elle, se déduit du projet : elle doit rester comparée telle quelle.
    let adresse = "ADMIN_EMAIL=admin@blog-auth.test";
    assert_eq!(normalize(adresse), adresse);
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

/// Le seul fichier exclu de la comparaison porte-t-il encore ce pour quoi il l'est ?
///
/// `generate crud` pose désormais la garde lui-même : de l'inventaire d'hier il ne reste
/// que le 403, qu'aucune commande n'engendre — le `tests/` généré n'inscrit qu'un
/// compte `admin`, et ne sait donc rien refuser à un rôle trop court. Ce test sorti de
/// la comparaison, rien ne verrait ce refus disparaître au fil d'une régénération.
#[test]
fn the_hand_edits_of_blog_auth_are_in_place() {
    let racine = common::depot().join("examples").join("blog-auth");
    let tests = std::fs::read_to_string(racine.join("src/posts/tests/access.rs"))
        .expect("src/posts/tests/access.rs lisible");

    assert!(
        tests.contains("async fn a_non_admin_write_returns_403()"),
        "`a_non_admin_write_returns_403` a disparu de l'exemple"
    );
    assert!(
        tests.contains(r#"token(&db, "user")"#),
        "le 403 ne prouve le seuil que si la requête présente un jeton d'un rôle plus court"
    );
}

/// L'exemple montre-t-il encore les deux régimes que sa promesse annonce ?
///
/// Le contrôleur est rentré dans la comparaison, qui le confronte à une génération
/// fraîche : elle rattraperait une template ayant changé, mais non un `--role admin`
/// retiré d'`EXEMPLES` — les deux côtés régénéreraient alors des écritures au seuil par
/// défaut, sans un mot. Or c'est ce seuil relevé que le README promet et que la
/// documentation cite.
#[test]
fn blog_auth_carries_the_two_regimes_of_the_guard() {
    let racine = common::depot().join("examples").join("blog-auth");
    let controller = std::fs::read_to_string(racine.join("src/posts/controller.rs"))
        .expect("src/posts/controller.rs lisible");

    assert_eq!(
        controller.matches("require_role(Role::Admin)").count(),
        3,
        "les trois écritures doivent exiger `admin`"
    );
    assert_eq!(
        controller.matches("require_role(Role::User)").count(),
        3,
        "les trois lectures — `list`, `filter` et `find` — restent au seuil par défaut"
    );
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
        let source = lire(&format!("src/modules/{module}/mod.rs"));
        assert!(
            !source.contains("#![allow(dead_code)]"),
            "src/modules/{module}/mod.rs : la permission de module tombe avec le premier \
             appel, et c'est ce que cet exemple montre"
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
/// Quatorze de ses fichiers sortent de la comparaison octet à octet, qui signalerait
/// l'édition elle-même. Sans ce test, ces quatorze chemins ne seraient sous aucune
/// surveillance et le câblage pourrait disparaître en silence.
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
        let source = lire(&format!("src/modules/{module}/mod.rs"));
        assert!(
            !source.contains("#![allow(dead_code)]"),
            "src/modules/{module}/mod.rs : la permission de module tombe avec le premier \
             appel, et c'est ce que cet exemple montre"
        );
    }

    // Le job de démonstration part avec son inscription : le laisser inscrit ferait passer
    // un exemple dont le registre ne porterait aucun job à lui.
    assert!(
        !racine.join("src/modules/jobs/demo.rs").exists(),
        "src/modules/jobs/demo.rs : le job d'exemple s'efface devant celui du projet"
    );
    let jobs = lire("src/modules/jobs/mod.rs");
    assert!(
        jobs.contains("register::<newsletter::SendNewsletter>()") && !jobs.contains("demo::Log"),
        "src/modules/jobs/mod.rs : le registre doit porter `SendNewsletter` et lui seul :\n{jobs}"
    );

    // Le job attend l'envoi au lieu de le détacher : c'est ce que le réessai exige, et
    // toute la différence que cet exemple sert à montrer.
    let newsletter = lire("src/modules/jobs/newsletter.rs");
    assert!(
        newsletter.contains("impl Job for SendNewsletter")
            && newsletter.contains(".send_template(")
            && !newsletter.contains(".send_detached("),
        "src/modules/jobs/newsletter.rs : le job rend l'échec au worker, il ne détache pas \
         l'envoi :\n{newsletter}"
    );

    // `send_detached` reste offert, et n'a plus que sa propre permission : la permission
    // de module retirée ci-dessus la rendait invisible.
    let mailer = lire("src/modules/mail/service.rs");
    assert!(
        mailer.contains("#[allow(dead_code)]\n    pub fn send_detached"),
        "src/modules/mail/service.rs : la fonction est conservée, sous une permission qui \
         ne vaut que pour elle"
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

    // La configuration Prometheus vise le second listener, et non l'API : deux littéraux
    // `9090` qui dériveraient l'un de l'autre feraient d'un exemple compilé un exemple
    // faux, que rien d'autre ne relèverait.
    let prometheus = lire("prometheus.yml");
    let port = lire("config/default.toml")
        .lines()
        .find_map(|ligne| ligne.strip_prefix("metrics_port = ").map(str::to_owned))
        .expect("config/default.toml doit porter `metrics_port`");
    assert!(
        prometheus.contains(&format!("\"localhost:{port}\"")),
        "prometheus.yml : la cible doit être le port {port} de `[observability]` :\n{prometheus}"
    );
}

/// Ce que `event-hub` porte et qu'aucune commande n'écrit.
///
/// Les trois fichiers de `orders` sortent de la comparaison octet à octet. Sans ce test, la
/// transaction que deux guides citent pourrait se défaire en silence : l'exemple
/// compilerait encore, et les pages montreraient un contrat que plus rien ne tient.
#[test]
fn the_hand_edits_of_event_hub_are_in_place() {
    let racine = common::depot().join("examples").join("event-hub");
    let lire = |relatif: &str| {
        std::fs::read_to_string(racine.join(relatif))
            .unwrap_or_else(|erreur| panic!("{relatif} illisible : {erreur}"))
    };

    // Dans cet ordre, et non seulement présents : une trace ou une émission posée après le
    // commit survivrait au rollback qu'elle doit suivre, et c'est ce que les guides réfutent.
    let service = lire("src/orders/service.rs");
    let mut depuis = 0;
    for (raison, extrait) in [
        ("la transaction n'est pas ouverte", "db.begin().await?"),
        (
            "la commande ne s'écrit pas dans la transaction",
            "repository::create(&transaction, order)",
        ),
        (
            "la trace ne partage pas la transaction",
            "audit::record(\n        &transaction,",
        ),
        (
            "l'événement ne partage pas la transaction",
            "webhooks::emit(&transaction, \"order.created\", &order)",
        ),
        ("rien ne la commite", "transaction.commit().await?"),
    ] {
        let Some(position) = service[depuis..].find(extrait) else {
            panic!(
                "src/orders/service.rs : {raison} — « {extrait} » absent ou hors d'ordre :\n{service}"
            );
        };
        depuis += position + extrait.len();
    }

    // Générique sur la connexion : une transaction n'est pas un `DatabaseConnection`.
    let repository = lire("src/orders/repository.rs");
    assert!(
        repository.contains("pub async fn create<C: ConnectionTrait>(db: &C, order: ActiveModel)"),
        "src/orders/repository.rs : la création doit accepter une transaction :\n{repository}"
    );

    // L'acteur vient du jeton : sans lui, le journal ne dirait pas qui a créé la commande.
    let controller = lire("src/orders/controller.rs");
    assert!(
        controller.contains("service::create(state.core().db(), input, &identite.user_id)"),
        "src/orders/controller.rs : l'identité doit descendre jusqu'au journal :\n{controller}"
    );
}

/// Ce que `rbs generate client` a déposé dans `admin-console`, et que la comparaison exclut.
///
/// Le shell d'administration importe ce fichier, et l'écran engendré y prend le corps de
/// la ressource : un client qui cesserait de publier une des cinq méthodes du CRUD ferait
/// échouer la vérification des types, mais seulement dans le job qui construit le
/// frontend. Ce test-ci répond du fichier versionné sans installer Node.
///
/// Il n'est pas régénéré ici — la commande compile le projet, ce qu'un test rapide ne peut
/// pas faire. Le job `admin-console · frontend` le régénère et exige qu'il n'ait pas bougé.
#[test]
fn the_typescript_client_of_admin_console_is_in_place() {
    let client = std::fs::read_to_string(
        common::depot()
            .join("examples/admin-console")
            .join("frontend/src/api/client.ts"),
    )
    .expect("le client versionné doit être lisible");

    // Les cinq méthodes que l'écran engendré appelle, et celles du shell.
    for methode in [
        "incidentsFilter(",
        "incidentsFind(",
        "incidentsCreate(",
        "incidentsUpdate(",
        "incidentsDelete(",
        "authLogin(",
        "authMe(",
        "authListSessions(",
        "health(",
    ] {
        assert!(
            client.contains(methode),
            "{methode} absente du client versionné de admin-console"
        );
    }

    // Le composant de l'énumération se déclare en alias de type : rendu en `interface`, il
    // n'était pas du TypeScript, et le fichier entier cessait de s'analyser.
    assert!(
        client.contains(r#"export type IncidentGravite = "basse" | "moyenne" | "haute""#),
        "{client}"
    );

    // Et le type que l'écran importe nommément.
    assert!(
        client.contains("export interface IncidentResponse {"),
        "{client}"
    );
}

/// Ce que `rbs generate client` a déposé dans `hello-crud`, et que la comparaison exclut.
///
/// La documentation cite ce fichier plutôt qu'un extrait écrit à la main : sans ce test,
/// `engendre_a_part` le sortirait de toute surveillance, et une page du site pourrait
/// montrer un client que la commande ne produit plus.
///
/// Il n'est pas régénéré ici — la commande compile le projet pour lire son document, ce
/// qu'un test rapide ne peut pas faire. `integration_client` s'en charge sur un projet
/// jetable ; ce test-ci répond du fichier versionné.
#[test]
fn the_typescript_client_of_hello_crud_is_in_place() {
    let client = std::fs::read_to_string(
        common::depot()
            .join("examples/hello-crud")
            .join("clients/ts/client.ts"),
    )
    .expect("le client versionné doit être lisible");

    // Une méthode par opération du CRUD, plus les deux sondes : c'est ce que le document
    // porte.
    for methode in [
        "articlesList(",
        "articlesCreate(",
        "articlesFind(",
        "articlesUpdate(",
        "articlesDelete(",
        "health(",
        "healthLive(",
    ] {
        assert!(
            client.contains(methode),
            "{methode} absente du client versionné"
        );
    }

    assert!(client.contains("export class ApiClient"), "{client}");

    // Le paramètre interne accepte un objet fermé : un `Record<string, unknown>` y
    // refuserait toute interface de query, et le client ne passerait plus `tsc --strict`.
    assert!(client.contains("query?: object;"), "{client}");
}
