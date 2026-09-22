//! Les transcripts marqués de la documentation rendent-ils encore ce que les pages
//! montrent ?
//!
//! Un bloc de transcript est un oracle : il dit ce qu'une commande rend. Un oracle qui
//! n'est jamais rejoué se périme sans bruit — quatre blocs ont ainsi vécu faux sur trois
//! axes à la fois, et la prose raisonnait sur leurs chiffres. `parite.mjs` ne voit que la
//! structure et les liens ; `integration_examples` ne couvre que le code d'`examples/`.
//! Ce test est le seul endroit d'où le mensonge d'une sortie citée est visible.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

mod common;

/// Un bloc de sortie gardé par son marqueur, et de quoi le rejouer.
struct Transcript {
    page: PathBuf,
    /// Ligne du marqueur, 1-based : c'est elle que l'échec nomme.
    ligne: usize,
    cmd: String,
    /// Commandes à jouer avant, séparées par ` && `.
    setup: Option<String>,
    /// Sous-répertoire du tmpdir où lancer `cmd`.
    dans: Option<String>,
    /// La commande exige un PostgreSQL joignable.
    base: bool,
    /// Le bloc est une portion de la sortie, non son intégralité.
    extrait: bool,
    /// L'invite que le bloc montre, quand la page en écrit une : `$ <cmd>`.
    ///
    /// Elle n'est pas de la sortie et ne peut pas être comparée à elle ; la garder à
    /// part permet de vérifier qu'elle dit bien la commande que le marqueur porte.
    invite: Option<String>,
    attendu: String,
}

const MARQUEUR: &str = "{/* rbs:transcript";

/// Les blocs gardés de `contenu`, dans l'ordre où la page les écrit.
fn extrait(page: &Path, contenu: &str) -> Vec<Transcript> {
    let lignes: Vec<&str> = contenu.lines().collect();
    let mut trouves = Vec::new();
    let mut rang = 0;

    while rang < lignes.len() {
        let Some(attributs) = marqueur(lignes[rang]) else {
            rang += 1;
            continue;
        };

        let ligne = rang + 1;
        rang += 1;

        // Docusaurus tolère une ligne vide entre un commentaire HTML et le bloc qu'il
        // annonce, et la relecture y gagne : le marqueur n'est pas collé au code.
        while rang < lignes.len() && lignes[rang].trim().is_empty() {
            rang += 1;
        }

        if rang >= lignes.len() || !lignes[rang].trim_start().starts_with("```") {
            panic!(
                "{}:{ligne} : le marqueur n'annonce aucun bloc",
                page.display()
            );
        }

        rang += 1;
        let debut = rang;
        while rang < lignes.len() && !lignes[rang].trim_start().starts_with("```") {
            rang += 1;
        }

        let mut corps = &lignes[debut..rang];
        rang += 1;

        let invite = corps
            .first()
            .filter(|premiere| premiere.starts_with("$ "))
            .map(|premiere| (*premiere).to_string());

        if invite.is_some() {
            corps = &corps[1..];
        }

        let mut attendu = String::new();
        for ligne in corps {
            attendu.push_str(ligne);
            attendu.push('\n');
        }

        trouves.push(Transcript {
            page: page.to_path_buf(),
            ligne,
            cmd: attribut(attributs, "cmd").unwrap_or_else(|| {
                panic!("{}:{ligne} : le marqueur n'a pas de `cmd`", page.display())
            }),
            setup: attribut(attributs, "setup"),
            dans: attribut(attributs, "dans"),
            base: attribut(attributs, "base").as_deref() == Some("oui"),
            extrait: attribut(attributs, "extrait").as_deref() == Some("oui"),
            invite,
            attendu,
        });
    }

    trouves
}

/// Les attributs d'une ligne de marqueur, ou `None` si ce n'en est pas une.
fn marqueur(ligne: &str) -> Option<&str> {
    let nu = ligne.trim();
    let reste = nu.strip_prefix(MARQUEUR)?;
    Some(reste.strip_suffix("*/}").unwrap_or(reste))
}

/// La valeur de `clé="…"` dans une ligne d'attributs.
///
/// Une petite boucle plutôt qu'une expression régulière : le dépôt n'a pas la dépendance,
/// et la forme reconnue tient en une ligne de grammaire.
fn attribut(attributs: &str, cle: &str) -> Option<String> {
    let mut reste = attributs;

    while let Some(position) = reste.find(&format!("{cle}=\"")) {
        let avant_conforme = reste[..position]
            .chars()
            .last()
            .is_none_or(char::is_whitespace);

        let apres = &reste[position + cle.len() + 2..];

        if avant_conforme {
            return apres.find('"').map(|fin| apres[..fin].to_string());
        }

        reste = apres;
    }

    None
}

/// Les pages du site, anglaises et françaises : une jumelle qui dérive est une jumelle
/// qui ment.
fn pages() -> Vec<PathBuf> {
    let docs = common::depot().join("docs");
    let mut trouvees = Vec::new();

    for racine in [
        docs.join("docs"),
        docs.join("i18n/fr/docusaurus-plugin-content-docs/current"),
    ] {
        collecte(&racine, &mut trouvees);
    }

    trouvees.sort();
    trouvees
}

fn collecte(repertoire: &Path, trouvees: &mut Vec<PathBuf>) {
    let entrees = std::fs::read_dir(repertoire).expect("répertoire de pages lisible");

    for entree in entrees {
        let chemin = entree.expect("entrée lisible").path();

        if chemin.is_dir() {
            collecte(&chemin, trouvees);
        } else if chemin.extension().is_some_and(|suffixe| suffixe == "md") {
            trouvees.push(chemin);
        }
    }
}

/// Tous les blocs gardés du site.
fn transcripts() -> Vec<Transcript> {
    pages()
        .iter()
        .flat_map(|page| {
            let contenu = std::fs::read_to_string(page).expect("page lisible");
            extrait(page, &contenu)
        })
        .collect()
}

/// Efface d'une sortie ce qui change d'une exécution à l'autre.
///
/// Appliquée des deux côtés de la comparaison : ce que la page écrit comme `…/demo` et ce
/// que la commande écrit comme chemin absolu se rejoignent sur `<tmp>/demo`.
fn normalise(sortie: &str, tmp: &Path) -> String {
    let mut texte = efface_ansi(sortie);

    // Le tmpdir de macOS est un lien symbolique : ce que la commande imprime est sa forme
    // canonique, ce que le test connaît est l'autre.
    let mut chemins = vec![tmp.to_string_lossy().into_owned()];
    if let Ok(canonique) = tmp.canonicalize() {
        chemins.push(canonique.to_string_lossy().into_owned());
    }
    chemins.sort_by_key(|chemin| std::cmp::Reverse(chemin.len()));

    for chemin in chemins {
        texte = texte.replace(&chemin, "<tmp>");
    }

    texte = texte.replace("…/", "<tmp>/");
    // Sous Windows, clap tire la ligne `Utilisation :` du nom réel de l'exécutable et écrit `rbs.exe`,
    // là où la page montre la commande telle qu'on la tape. Le suffixe tombe des deux
    // côtés de la comparaison, qui reste sensible à tout le reste de la ligne.
    texte = texte.replace("rbs.exe", "rbs");
    texte = unifie_separateurs(&texte);
    texte = masque_moteur(&texte);
    texte = masque_version(&texte);
    texte = common::masque_horodatage(&texte);
    texte = masque_duree(&texte);
    texte = masque_adresse(&texte);

    let mut rendu = String::with_capacity(texte.len());
    for ligne in texte.lines() {
        let nette = masque_progression(ligne.trim_end());
        if nette.is_empty() {
            continue;
        }
        rendu.push_str(&nette);
        rendu.push('\n');
    }

    rendu
}

/// `en attente de la base (…) ...` → `… …` : un point par tentative de connexion, dont le
/// nombre suit la vitesse à laquelle la machine refuse un port fermé.
///
/// Seule une suite de points précédée d'une espace et close par la fin de ligne est une
/// progression : un point collé au mot qui le précède clôt une phrase.
fn masque_progression(ligne: &str) -> String {
    let sans_points = ligne.trim_end_matches('.');
    if sans_points.len() < ligne.len() && sans_points.ends_with(' ') {
        format!("{sans_points}…")
    } else {
        ligne.to_string()
    }
}

/// Les chemins situés sous `<tmp>` reçoivent la barre oblique : Windows imprime
/// `<tmp>\demo` là où la page montre `<tmp>/demo`.
///
/// Seuls les mots commençant par `<tmp>` sont touchés. Un `\` ailleurs appartient à du
/// code cité — une chaîne Rust échappée, un chemin d'exemple — et doit rester comparé
/// tel quel, faute de quoi la normalisation masquerait une vraie dérive de la page.
fn unifie_separateurs(texte: &str) -> String {
    texte
        .split_inclusive(char::is_whitespace)
        .map(|mot| {
            if mot.starts_with("<tmp>") {
                mot.replace('\\', "/")
            } else {
                mot.to_string()
            }
        })
        .collect()
}

fn efface_ansi(texte: &str) -> String {
    let mut rendu = String::with_capacity(texte.len());
    let mut lettres = texte.chars().peekable();

    while let Some(lettre) = lettres.next() {
        if lettre == '\u{1b}' && lettres.peek() == Some(&'[') {
            lettres.next();
            for suite in lettres.by_ref() {
                if suite.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }

        rendu.push(lettre);
    }

    rendu
}

/// `postgres 18.6` → `<moteur>` : la version du serveur est celle de la machine.
fn masque_moteur(texte: &str) -> String {
    common::remplace_motif(texte, |lettres, debut| {
        if debut > 0 && lettres[debut - 1].is_ascii_alphanumeric() {
            return None;
        }

        let moteur = ["postgres ", "mysql ", "sqlite "].into_iter().find(|nom| {
            lettres[debut..]
                .iter()
                .zip(nom.chars())
                .filter(|(lettre, attendue)| **lettre == *attendue)
                .count()
                == nom.len()
        })?;

        let apres = debut + moteur.len();
        let mut rang = apres;
        while lettres
            .get(rang)
            .is_some_and(|lettre| lettre.is_ascii_digit() || *lettre == '.')
        {
            rang += 1;
        }

        // « postgres://… » ou « postgres répond » : sans chiffre derrière, ce n'est pas
        // une version, et la ligne se compare telle quelle.
        if rang == apres || !lettres[apres].is_ascii_digit() {
            return None;
        }

        Some((rang, "<moteur>".to_string()))
    })
}

/// `1.2.0` → `<version>` : le dépôt travaille toujours sur la version qui suit celle que
/// la documentation cite, et une page ne se réécrit pas à chaque montée de version.
fn masque_version(texte: &str) -> String {
    common::remplace_motif(texte, |lettres, debut| {
        let mut rang = debut;
        for point in 0..3 {
            let chiffres = compte_chiffres(lettres, rang);
            if chiffres == 0 {
                return None;
            }
            rang += chiffres;
            if point < 2 {
                if lettres.get(rang) != Some(&'.') {
                    return None;
                }
                rang += 1;
            }
        }

        if debut > 0 && (lettres[debut - 1].is_ascii_digit() || lettres[debut - 1] == '.') {
            return None;
        }
        if lettres.get(rang).is_some_and(|suite| *suite == '.') {
            return None;
        }

        Some((rang, "<version>".to_string()))
    })
}

/// `in 0.11s`, `en 1.2 s`, `in 1m 12s` → `<durée>`.
fn masque_duree(texte: &str) -> String {
    common::remplace_motif(texte, |lettres, debut| {
        let prefixe: String = lettres[debut..(debut + 3).min(lettres.len())]
            .iter()
            .collect();
        if prefixe != "in " && prefixe != "en " {
            return None;
        }
        if debut > 0 && !lettres[debut - 1].is_whitespace() {
            return None;
        }

        let mut rang = debut + 3;

        // Cargo passe à `1m 12s` puis à `1h 02m 03s` dès que la compilation s'allonge :
        // la durée est une suite de groupes, et seul le dernier porte les secondes.
        loop {
            let entiers = compte_chiffres(lettres, rang);
            if entiers == 0 {
                return None;
            }
            rang += entiers;

            if lettres.get(rang) == Some(&'.') {
                let decimales = compte_chiffres(lettres, rang + 1);
                if decimales == 0 {
                    return None;
                }
                rang += 1 + decimales;
            }

            while lettres.get(rang) == Some(&' ') {
                rang += 1;
            }

            // `h` et `m` ouvrent un groupe de plus ; `ms` clôt, et c'est ce qui les
            // sépare — sans quoi une milliseconde serait lue comme une minute.
            let heure = lettres.get(rang) == Some(&'h');
            let minute = lettres.get(rang) == Some(&'m') && lettres.get(rang + 1) != Some(&'s');
            if heure || minute {
                rang += 1;
                while lettres.get(rang) == Some(&' ') {
                    rang += 1;
                }
                continue;
            }

            if lettres.get(rang) == Some(&'m') {
                rang += 1;
            }
            if lettres.get(rang) != Some(&'s') {
                return None;
            }
            rang += 1;
            break;
        }

        if lettres.get(rang).is_some_and(char::is_ascii_alphanumeric) {
            return None;
        }

        Some((rang, format!("{prefixe}<durée>")))
    })
}

/// `répond sur localhost:5432` → `répond sur <adresse>`.
///
/// L'hôte et le port sont ceux de la machine qui rejoue : le serveur du test écoute sur
/// un port que Docker lui donne, quand la page montre celui qu'un lecteur aurait.
fn masque_adresse(texte: &str) -> String {
    const ANNONCE: &str = "répond sur ";
    let mut rendu = String::with_capacity(texte.len());

    for ligne in texte.split_inclusive('\n') {
        match ligne.find(ANNONCE) {
            Some(debut) => {
                rendu.push_str(&ligne[..debut + ANNONCE.len()]);
                rendu.push_str("<adresse>");
                if ligne.ends_with('\n') {
                    rendu.push('\n');
                }
            }
            None => rendu.push_str(ligne),
        }
    }

    rendu
}

fn compte_chiffres(lettres: &[char], debut: usize) -> usize {
    lettres[debut..]
        .iter()
        .take_while(|lettre| lettre.is_ascii_digit())
        .count()
}

// --- Rejouer -----------------------------------------------------------------------

/// Une commande citée, telle qu'elle se lance : le programme et ses arguments.
///
/// Un découpage aux blancs ne suffit pas — `--fields "title:string,body:text"` est un
/// seul argument — et le dépôt n'a pas de shell portable à qui déléguer.
fn decoupe(commande: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut courant = String::new();
    let mut entre_guillemets = false;
    let mut commence = false;

    for lettre in commande.chars() {
        match lettre {
            '"' => {
                entre_guillemets = !entre_guillemets;
                commence = true;
            }
            lettre if lettre.is_whitespace() && !entre_guillemets => {
                if commence {
                    arguments.push(std::mem::take(&mut courant));
                    commence = false;
                }
            }
            lettre => {
                courant.push(lettre);
                commence = true;
            }
        }
    }

    if commence {
        arguments.push(courant);
    }

    arguments
}

/// Lance `commande` dans `repertoire` et rend ce qu'elle a écrit, les deux sorties
/// réunies dans l'ordre où le terminal les aurait vues.
///
/// Le statut n'est pas exigé : une page montre aussi ce qu'un refus rend, et c'est
/// précisément la sortie qu'il faut comparer.
///
/// Les deux flux partagent un même fichier, et non deux tuyaux : `rbs doctor` écrit ses
/// verdicts sur la sortie standard pendant que cargo compile sur l'erreur, et deux
/// captures séparées rendraient un bloc que personne n'a jamais vu à l'écran.
fn lance(commande: &str, repertoire: &Path, base: Option<&str>) -> String {
    let mut arguments = decoupe(commande);
    assert!(!arguments.is_empty(), "commande vide");
    let programme = arguments.remove(0);

    // Même raison que `--core-path` : l'URL qu'une page montre est celle qu'un lecteur
    // écrirait, et le serveur du test écoute là où Docker l'a mis.
    if let Some(vivante) = base {
        for argument in &mut arguments {
            if argument.starts_with("postgres://") {
                *argument = vivante.to_string();
            }
        }
    }

    let executable = if programme == "rbs" {
        // L'utilisateur, lui, prend la crate publiée. Le test ne le peut pas : le dépôt
        // travaille sur une version que crates.io ne porte pas encore, et la résolution
        // échouerait avant la première ligne de sortie. La substitution appartient donc
        // au test, non au bloc.
        if arguments.first().is_some_and(|premier| premier == "new") {
            arguments.push("--core-path".to_string());
            arguments.push(
                common::noyau()
                    .to_str()
                    .expect("chemin du noyau représentable")
                    .to_string(),
            );
        }

        env!("CARGO_BIN_EXE_rbs").to_string()
    } else {
        programme.clone()
    };

    let journal = tempfile::NamedTempFile::new().expect("fichier de capture créable");
    let sortie = std::fs::File::create(journal.path()).expect("capture ouvrable");
    let erreur = sortie.try_clone().expect("capture duplicable");

    // Sans terminal, comme une CI : lancé depuis un shell, `cargo test` léguerait le sien,
    // et `rbs new` sans `--yes` attendrait une réponse que personne ne donnera.
    std::process::Command::new(&executable)
        .current_dir(repertoire)
        .args(&arguments)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(sortie))
        .stderr(std::process::Stdio::from(erreur))
        .status()
        .unwrap_or_else(|erreur| panic!("`{programme}` doit être lançable : {erreur}"));

    String::from_utf8_lossy(&std::fs::read(journal.path()).expect("capture lisible")).into_owned()
}

/// Rejoue un transcript dans un répertoire neuf et compare sa sortie au bloc.
fn compare_transcript(transcript: &Transcript, base: Option<&str>) {
    let situe = format!("{}:{}", transcript.page.display(), transcript.ligne);

    if let Some(invite) = &transcript.invite {
        assert_eq!(
            invite,
            &format!("$ {}", transcript.cmd),
            "{situe} : l'invite montrée n'est pas la commande que le marqueur porte"
        );
    }

    let tmp = tempfile::TempDir::new().expect("répertoire temporaire créable");
    let dans = transcript
        .dans
        .as_ref()
        .map(|sous| tmp.path().join(sous))
        .unwrap_or_else(|| tmp.path().to_path_buf());

    for commande in transcript
        .setup
        .iter()
        .flat_map(|decor| decor.split(" && "))
    {
        // Le décor se pose à la racine tant que le projet n'existe pas, puis dedans :
        // c'est l'ordre dans lequel une page l'écrit, `rbs new` puis ce qui suit.
        let ou = if dans.is_dir() {
            dans.as_path()
        } else {
            tmp.path()
        };
        lance(commande.trim(), ou, base);
    }

    let obtenu = normalise(&lance(&transcript.cmd, &dans, base), tmp.path());
    let attendu = normalise(&transcript.attendu, tmp.path());

    let conforme = if transcript.extrait {
        contient(&obtenu, &attendu)
    } else {
        obtenu == attendu
    };

    assert!(
        conforme,
        "{situe} : `{}` ne rend plus ce que la page montre.\n\n--- la page ---\n{attendu}\n--- la commande ---\n{obtenu}",
        transcript.cmd
    );
}

/// Les lignes d'`attendu` paraissent-elles dans `obtenu`, dans l'ordre ?
fn contient(obtenu: &str, attendu: &str) -> bool {
    let mut lignes = obtenu.lines();
    attendu
        .lines()
        .all(|cherchee| lignes.any(|ligne| ligne == cherchee))
}

#[test]
fn the_marked_transcripts_still_render_what_the_docs_show() {
    let mut rejoues = 0;

    for transcript in transcripts().iter().filter(|garde| !garde.base) {
        compare_transcript(transcript, None);
        rejoues += 1;
    }

    // Une garde qui ne garde plus rien passe au vert sans rien prouver : c'est
    // exactement l'angle mort qui a laissé quatre blocs vivre périmés.
    assert!(rejoues > 0, "aucun bloc marqué n'a été rejoué");
}

#[test]
#[ignore = "démarre un PostgreSQL sous Docker et compile la crate migration d'un projet temporaire"]
fn the_marked_transcripts_that_need_a_database_still_render_what_the_docs_show() {
    let gardes: Vec<Transcript> = transcripts()
        .into_iter()
        .filter(|garde| garde.base)
        .collect();

    assert!(
        !gardes.is_empty(),
        "aucun bloc `base=\"oui\"` n'a été rejoué"
    );

    let postgres = common::start_postgres();
    let url = common::url_of(&postgres);

    for transcript in &gardes {
        compare_transcript(transcript, Some(&url));
    }
}

/// Les deux README annoncent la version du dépôt, et rien d'autre ne tenait cette ligne :
/// elle n'est dans aucun transcript, que `masque_version` effacerait de toute façon. La
/// vitrine a ainsi vécu deux versions en retard.
#[test]
fn the_readmes_announce_the_version_of_the_workspace() {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let annonce = format!("Version {}.", env!("CARGO_PKG_VERSION"));

    for readme in ["README.md", "README.fr.md"] {
        let contenu = std::fs::read_to_string(racine.join(readme))
            .unwrap_or_else(|erreur| panic!("{readme} illisible : {erreur}"));

        assert!(
            contenu.lines().any(|ligne| ligne.starts_with(&annonce)),
            "{readme} n'annonce pas « {annonce} », la version du workspace"
        );
    }
}

mod extraction {
    use super::*;

    #[test]
    fn a_marker_carries_its_command_and_the_block_that_follows() {
        let page = "\
avant\n\
{/* rbs:transcript cmd=\"rbs new demo\" */}\n\
```text\n\
✓ demo créé — 18 fichiers\n\
```\n";
        let trouves = extrait(Path::new("page.md"), page);

        assert_eq!(trouves.len(), 1);
        assert_eq!(trouves[0].cmd, "rbs new demo");
        assert_eq!(trouves[0].attendu, "✓ demo créé — 18 fichiers\n");
        assert_eq!(trouves[0].ligne, 2);
    }

    #[test]
    fn a_block_without_a_marker_is_not_guarded() {
        assert!(extrait(Path::new("page.md"), "```text\nsortie\n```\n").is_empty());
    }

    #[test]
    fn the_optional_attributes_default_to_the_cheapest_case() {
        let page = "{/* rbs:transcript cmd=\"rbs doctor\" base=\"oui\" extrait=\"oui\" dans=\"demo\" */}\n```text\n✓\n```\n";
        let trouve = &extrait(Path::new("page.md"), page)[0];

        assert!(trouve.base);
        assert!(trouve.extrait);
        assert_eq!(trouve.dans.as_deref(), Some("demo"));

        let sobre = "{/* rbs:transcript cmd=\"rbs doctor\" */}\n```text\n✓\n```\n";
        let sobre = &extrait(Path::new("page.md"), sobre)[0];

        assert!(!sobre.base);
        assert!(!sobre.extrait);
        assert_eq!(sobre.dans, None);
        assert_eq!(sobre.setup, None);
        assert_eq!(sobre.invite, None);
    }

    #[test]
    fn the_prompt_a_page_shows_is_kept_apart_from_the_output() {
        let page = "{/* rbs:transcript cmd=\"rbs new site --yes\" */}\n```text\n$ rbs new site --yes\n✓ site créé\n```\n";
        let trouve = &extrait(Path::new("page.md"), page)[0];

        assert_eq!(trouve.invite.as_deref(), Some("$ rbs new site --yes"));
        assert_eq!(trouve.attendu, "✓ site créé\n");
    }

    #[test]
    fn what_changes_between_two_runs_is_erased_before_the_comparison() {
        let tmp = Path::new("/var/folders/x/T/.tmpAbC");
        let sortie = "  ✓ base  postgres 18.6 répond\n    Finished `dev` profile in 0.11s\n  /var/folders/x/T/.tmpAbC/demo\n";

        assert_eq!(
            normalise(sortie, tmp),
            "  ✓ base  <moteur> répond\n    Finished `dev` profile in <durée>\n  <tmp>/demo\n"
        );
    }

    /// Un point par tentative de connexion : Windows refuse un port fermé plus lentement
    /// que Linux, et `en attente de la base … ...` y rendait un seul point.
    #[test]
    fn the_progress_dots_of_a_wait_are_masked_whatever_their_count() {
        let tmp = Path::new("/var/folders/x/T/.tmpAbC");

        for sortie in [
            "en attente de la base (127.0.0.1:1) .\n",
            "en attente de la base (127.0.0.1:1) ...\n",
        ] {
            assert_eq!(
                normalise(sortie, tmp),
                "en attente de la base (127.0.0.1:1) …\n",
                "progression non masquée : {sortie}"
            );
        }

        assert_eq!(
            normalise("une phrase qui finit.\nversion 1.\n", tmp),
            "une phrase qui finit.\nversion 1.\n",
            "un point qui clôt une phrase n'est pas une progression"
        );
    }

    /// Cargo passe de `12.34s` à `1m 12s` dès qu'une compilation dépasse la minute, et
    /// celle de la crate `migration` la dépasse sur une machine froide. Une durée non
    /// masquée fait échouer le transcript de `doctor` sous les traits d'une dérive de la
    /// documentation, là où seule l'horloge a bougé.
    #[test]
    fn a_duration_above_the_minute_is_masked_like_the_others() {
        let tmp = Path::new("/var/folders/x/T/.tmpAbC");

        for sortie in [
            "    Finished `dev` profile in 1m 12s\n",
            "    Finished `dev` profile in 1h 02m 03s\n",
        ] {
            assert_eq!(
                normalise(sortie, tmp),
                "    Finished `dev` profile in <durée>\n",
                "durée non masquée : {sortie}"
            );
        }
    }

    #[test]
    fn the_ellipsis_a_page_writes_for_a_temporary_path_meets_the_real_one() {
        let tmp = Path::new("/var/folders/x/T/.tmpAbC");

        assert_eq!(
            normalise("plan pour …/demo\n", tmp),
            normalise("plan pour /var/folders/x/T/.tmpAbC/demo\n", tmp)
        );
    }

    /// Windows imprime le chemin absolu du tmpdir avec des barres inverses, et sa ligne `Utilisation :`
    /// avec le nom réel de l'exécutable. Les pages montrent l'un et l'autre sous la forme
    /// qu'on tape : sans cette unification, tout transcript citant un chemin absolu
    /// échouait sur cette seule plateforme, en accusant la page d'une dérive.
    #[test]
    fn a_windows_path_and_executable_meet_what_the_page_writes() {
        let tmp = Path::new("/var/folders/x/T/.tmpAbC");

        assert_eq!(
            normalise("plan pour <tmp>\\demo\\src\n", tmp),
            "plan pour <tmp>/demo/src\n"
        );
        assert_eq!(
            normalise("Utilisation : rbs.exe generate <COMMANDE>\n", tmp),
            "Utilisation : rbs generate <COMMANDE>\n"
        );
    }

    /// Un `\` hors d'un chemin du tmpdir appartient au contenu que la page cite : le
    /// masquer y ferait passer une vraie dérive pour une différence de plateforme.
    #[test]
    fn a_backslash_outside_the_tmpdir_is_left_alone() {
        let tmp = Path::new("/var/folders/x/T/.tmpAbC");

        assert_eq!(
            normalise("  let ligne = \"a\\nb\";\n", tmp),
            "  let ligne = \"a\\nb\";\n"
        );
    }

    #[test]
    fn a_migration_stamp_and_a_cli_version_are_erased() {
        let tmp = Path::new("/var/folders/x/T/.tmpAbC");

        assert_eq!(
            normalise("m20260902_122330_create_articles — rbs 1.2.0\n", tmp),
            "m<horodatage>_create_articles — rbs <version>\n"
        );
    }

    #[test]
    fn both_languages_are_walked() {
        let pages = pages();
        // Les séparateurs sont ramenés à `/` avant la comparaison : sous Windows un chemin
        // s'écrit `docs\docs\getting-started.md`, et le suffixe attendu ne s'y retrouvait
        // pas — l'assertion accusait le parcours des pages d'une absence qui n'était que
        // celle d'une barre oblique.
        let porte = |suffixe: &str| {
            pages
                .iter()
                .any(|page| page.to_string_lossy().replace('\\', "/").ends_with(suffixe))
        };

        assert!(porte("docs/docs/getting-started.md"));
        assert!(porte("current/getting-started.md"));
        assert!(porte("docs/docs/cli/new.md"));
        assert!(porte("current/cli/new.md"));
    }

    #[test]
    fn every_marker_of_the_site_parses() {
        for transcript in transcripts() {
            assert!(
                !transcript.cmd.is_empty(),
                "{}:{} : commande vide",
                transcript.page.display(),
                transcript.ligne
            );
        }
    }
}

// --- La garde des blocs de sortie --------------------------------------------------

/// Un bloc ```text de sortie est une transcription ou se déclare libre, avec sa raison.
///
/// Rejouer les blocs marqués ne suffisait pas : un bloc qu'on oubliait de marquer n'était
/// vu par personne, et seize guides sur vingt et un montraient ainsi des sorties écrites à
/// la main — dont un compte de fichiers faux. La garde rend l'oubli impossible ; les
/// blocs nus d'avant elle vivent dans `EXEMPTIONS`, que chaque migration vide.
const EXEMPTIONS: &str = "crates/rbs-cli/tests/transcriptions-exemptees.txt";

/// Les README qu'on lit sur GitHub et crates.io : ils montrent des sorties comme le site.
const READMES: [&str; 8] = [
    "README.md",
    "README.fr.md",
    "crates/rbs-cli/README.md",
    "crates/rbs-cli/README.fr.md",
    "crates/rbs-core/README.md",
    "crates/rbs-core/README.fr.md",
    "examples/README.md",
    "examples/README.fr.md",
];

/// Le commentaire qu'un fichier sait taire.
///
/// Une page du site est du MDX, où un commentaire HTML ne compile pas ; un README est
/// rendu par GitHub et crates.io, qui imprimeraient `{/* */}` en clair.
#[derive(Clone, Copy)]
enum Forme {
    Mdx,
    Html,
}

impl Forme {
    fn libre(self) -> &'static str {
        match self {
            Forme::Mdx => "{/* rbs:libre",
            Forme::Html => "<!-- rbs:libre",
        }
    }

    fn fermeture(self) -> &'static str {
        match self {
            Forme::Mdx => "*/}",
            Forme::Html => "-->",
        }
    }

    /// Seul le site est rejoué : un marqueur de transcription dans un README ne garderait
    /// rien, et le bloc qu'il annonce resterait nu.
    fn marque_un_bloc(self, ligne: &str) -> bool {
        (matches!(self, Forme::Mdx) && ligne.starts_with(MARQUEUR))
            || self.marqueur_libre(ligne).is_some()
    }

    /// Les attributs d'un marqueur `libre`, ou `None` si la ligne n'en est pas un.
    fn marqueur_libre(self, ligne: &str) -> Option<&str> {
        let reste = ligne.trim().strip_prefix(self.libre())?;
        if !reste.is_empty() && !reste.starts_with(char::is_whitespace) {
            return None;
        }
        Some(reste.strip_suffix(self.fermeture()).unwrap_or(reste))
    }
}

/// Un bloc ```text qui n'est ni une transcription ni libre.
#[derive(Clone, Debug, PartialEq)]
struct BlocNu {
    /// Ligne de la clôture ouvrante, 1-based.
    ligne: usize,
    /// Sa première ligne non vide : elle l'identifie dans `EXEMPTIONS` sans dépendre de la
    /// prose qui le précède, qu'une retouche décalerait.
    premiere: String,
}

#[derive(Default)]
struct Releve {
    /// Tous les blocs ```text vus, marqués ou non.
    vus: usize,
    nus: Vec<BlocNu>,
    fautes: Vec<String>,
}

/// Le langage d'une clôture ouvrante, ou `None` si la ligne n'en est pas une.
fn cloture(ligne: &str) -> Option<&str> {
    let reste = ligne.trim().strip_prefix("```")?;
    Some(reste.split_whitespace().next().unwrap_or(""))
}

/// Les blocs ```text de `contenu` qui ne sont ni une transcription ni libres, et les
/// marqueurs `libre` fautifs.
fn releve(fichier: &str, contenu: &str, forme: Forme) -> Releve {
    let lignes: Vec<&str> = contenu.lines().collect();
    let mut releve = Releve::default();
    let non_vide =
        |depuis: usize| (depuis..lignes.len()).find(|&rang| !lignes[rang].trim().is_empty());
    let mut rang = 0;

    while rang < lignes.len() {
        if let Some(attributs) = forme.marqueur_libre(lignes[rang]) {
            let ligne = rang + 1;

            // Une raison est ce qui distingue un bloc libre d'un bloc qu'on n'a pas voulu
            // rejouer : sans elle, le marqueur n'est qu'une exemption déguisée.
            if attribut(attributs, "raison").is_none_or(|raison| raison.trim().is_empty()) {
                releve.fautes.push(format!(
                    "{fichier}:{ligne} : un bloc libre dit pourquoi il n'est pas rejoué — `raison=\"…\"` manque ou est vide"
                ));
            }

            let annonce = non_vide(rang + 1).and_then(|suivante| cloture(lignes[suivante]));
            if annonce != Some("text") {
                releve.fautes.push(format!(
                    "{fichier}:{ligne} : le marqueur libre n'annonce aucun bloc ```text"
                ));
            }

            rang += 1;
            continue;
        }

        let Some(langage) = cloture(lignes[rang]) else {
            rang += 1;
            continue;
        };

        let ouverture = rang;
        rang += 1;
        while rang < lignes.len() && cloture(lignes[rang]).is_none() {
            rang += 1;
        }

        if langage == "text" {
            releve.vus += 1;

            // Docusaurus tolère une ligne vide entre le marqueur et le bloc : la garde
            // aussi, comme l'extracteur des transcriptions.
            let marque = (0..ouverture)
                .rev()
                .find(|&avant| !lignes[avant].trim().is_empty())
                .is_some_and(|avant| forme.marque_un_bloc(lignes[avant].trim()));

            if !marque {
                releve.nus.push(BlocNu {
                    ligne: ouverture + 1,
                    premiere: lignes[ouverture + 1..rang]
                        .iter()
                        .map(|ligne| ligne.trim())
                        .find(|ligne| !ligne.is_empty())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }

        rang += 1;
    }

    releve
}

/// Compare les blocs nus à la liste d'exemptions, dans les deux sens.
///
/// Une entrée vaut pour un bloc : deux blocs nus d'une même page ouverts par la même ligne
/// demandent deux entrées. Une entrée que plus aucun bloc ne consomme échoue aussi — sans
/// quoi la liste ne ferait que croître, et un bloc nu ajouté plus tard sous la même
/// première ligne s'y abriterait.
fn confronte(nus: &[(String, BlocNu)], exemptions: &str) -> Vec<String> {
    let mut fautes = Vec::new();
    let mut entrees: Vec<(usize, String, String)> = Vec::new();

    for (rang, ligne) in exemptions.lines().enumerate() {
        let nette = ligne.trim();
        if nette.is_empty() || nette.starts_with('#') {
            continue;
        }
        match nette.split_once('|') {
            Some((fichier, premiere)) => entrees.push((
                rang + 1,
                fichier.trim().to_string(),
                premiere.trim().to_string(),
            )),
            None => fautes.push(format!(
                "{EXEMPTIONS}:{} : entrée sans `|` entre le fichier et la première ligne du bloc",
                rang + 1
            )),
        }
    }

    for (fichier, bloc) in nus {
        let exemptee = entrees
            .iter()
            .position(|(_, exempte, premiere)| exempte == fichier && *premiere == bloc.premiere);

        match exemptee {
            Some(position) => {
                entrees.remove(position);
            }
            None => fautes.push(format!(
                "{fichier}:{} : bloc ```text nu — marquez-le `rbs:transcript` pour qu'il soit rejoué, ou `rbs:libre raison=\"…\"` s'il ne peut pas l'être",
                bloc.ligne
            )),
        }
    }

    for (ligne, fichier, premiere) in entrees {
        fautes.push(format!(
            "{EXEMPTIONS}:{ligne} : `{fichier} | {premiere}` ne correspond plus à aucun bloc nu — retirez l'entrée"
        ));
    }

    fautes
}

/// Les fichiers que la garde parcourt, chacun avec la forme de commentaire qu'il admet.
fn fichiers_gardes() -> Vec<(PathBuf, Forme)> {
    let depot = common::depot();
    let mut fichiers: Vec<(PathBuf, Forme)> =
        pages().into_iter().map(|page| (page, Forme::Mdx)).collect();
    fichiers.extend(
        READMES
            .iter()
            .map(|readme| (depot.join(readme), Forme::Html)),
    );
    fichiers
}

#[test]
fn every_text_block_is_a_transcript_or_declared_free() {
    let depot = common::depot();
    let mut nus = Vec::new();
    let mut fautes = Vec::new();
    let mut vus = 0;

    for (fichier, forme) in fichiers_gardes() {
        let contenu = std::fs::read_to_string(&fichier)
            .unwrap_or_else(|erreur| panic!("{} illisible : {erreur}", fichier.display()));
        let relatif = fichier
            .strip_prefix(&depot)
            .expect("fichier gardé sous le dépôt")
            .to_string_lossy()
            .replace('\\', "/");

        let trouve = releve(&relatif, &contenu, forme);
        vus += trouve.vus;
        fautes.extend(trouve.fautes);
        nus.extend(trouve.nus.into_iter().map(|bloc| (relatif.clone(), bloc)));
    }

    // Même angle mort que pour le rejeu : un parcours cassé ne verrait aucun bloc, et la
    // garde passerait au vert sans rien garder.
    assert!(vus > 0, "aucun bloc ```text n'a été vu");

    let exemptions = std::fs::read_to_string(depot.join(EXEMPTIONS))
        .unwrap_or_else(|erreur| panic!("{EXEMPTIONS} illisible : {erreur}"));
    fautes.extend(confronte(&nus, &exemptions));

    assert!(fautes.is_empty(), "\n{}\n", fautes.join("\n"));
}

mod garde {
    use super::*;

    fn nus(contenu: &str, forme: Forme) -> Vec<BlocNu> {
        let releve = releve("page.md", contenu, forme);
        assert!(releve.fautes.is_empty(), "{:?}", releve.fautes);
        releve.nus
    }

    #[test]
    fn a_bare_text_block_is_named_by_its_fence_line_and_first_line() {
        let page = "prose\n\n```text\n\n  ✓ demo créé\nsuite\n```\n";

        assert_eq!(
            nus(page, Forme::Mdx),
            vec![BlocNu {
                ligne: 3,
                premiere: "✓ demo créé".to_string()
            }]
        );
    }

    #[test]
    fn a_transcript_or_a_free_block_with_its_reason_is_not_bare() {
        let page = "\
{/* rbs:transcript cmd=\"rbs doctor\" */}\n\
\n\
```text\n\
✓\n\
```\n\
{/* rbs:libre raison=\"la sortie dépend du réseau\" */}\n\
```text\n\
✓\n\
```\n";

        assert!(nus(page, Forme::Mdx).is_empty());
    }

    #[test]
    fn only_text_blocks_are_guarded_and_their_content_is_not_read_as_markdown() {
        let page = "```bash\nrbs new demo\n```\n```rust\nlet x = \"```text\";\n```\n";

        assert!(nus(page, Forme::Mdx).is_empty());
    }

    #[test]
    fn a_free_block_without_a_reason_is_refused() {
        for marqueur in [
            "{/* rbs:libre */}",
            "{/* rbs:libre raison=\"\" */}",
            "{/* rbs:libre raison=\"  \" */}",
            "{/* rbs:libre reason=\"typo\" */}",
        ] {
            let page = format!("{marqueur}\n```text\n✓\n```\n");
            let releve = releve("page.md", &page, Forme::Mdx);

            assert_eq!(
                releve.fautes.len(),
                1,
                "{marqueur} devait être refusé : {:?}",
                releve.fautes
            );
            assert!(releve.fautes[0].starts_with("page.md:1 : "));
        }
    }

    #[test]
    fn a_free_marker_that_announces_no_text_block_is_refused() {
        let releve = releve(
            "page.md",
            "{/* rbs:libre raison=\"x\" */}\nprose\n```text\n✓\n```\n",
            Forme::Mdx,
        );

        assert!(
            releve
                .fautes
                .iter()
                .any(|faute| faute.starts_with("page.md:1 : ")),
            "{:?}",
            releve.fautes
        );
    }

    /// Un README s'affiche sur GitHub et crates.io, où `{/* */}` s'imprimerait en clair ;
    /// une page du site est du MDX, où un commentaire HTML ne compile pas.
    #[test]
    fn each_kind_of_file_takes_the_comment_it_can_hide() {
        let html = "<!-- rbs:libre raison=\"x\" -->\n```text\n✓\n```\n";
        let mdx = "{/* rbs:libre raison=\"x\" */}\n```text\n✓\n```\n";

        assert!(nus(html, Forme::Html).is_empty());
        assert!(nus(mdx, Forme::Mdx).is_empty());
        assert_eq!(nus(mdx, Forme::Html).len(), 1);
        assert_eq!(nus(html, Forme::Mdx).len(), 1);

        let transcript = "{/* rbs:transcript cmd=\"rbs doctor\" */}\n```text\n✓\n```\n";
        assert_eq!(
            nus(transcript, Forme::Html).len(),
            1,
            "un README ne rejoue rien : un marqueur de transcription n'y garde aucun bloc"
        );
    }

    fn nu(page: &str, ligne: usize, premiere: &str) -> (String, BlocNu) {
        (
            page.to_string(),
            BlocNu {
                ligne,
                premiere: premiere.to_string(),
            },
        )
    }

    #[test]
    fn a_bare_block_absent_from_the_exemptions_is_named_by_file_and_line() {
        let fautes = confronte(&[nu("docs/a.md", 12, "✓ demo")], "# rien\n");

        assert_eq!(fautes.len(), 1);
        assert!(fautes[0].starts_with("docs/a.md:12 : "), "{fautes:?}");
    }

    #[test]
    fn the_exemptions_list_exactly_the_bare_blocks() {
        let liste = "# en-tête\n\ndocs/a.md | ✓ demo\ndocs/a.md | ✓ demo\ndocs/b.md |\n";
        let blocs = [
            nu("docs/a.md", 3, "✓ demo"),
            nu("docs/a.md", 9, "✓ demo"),
            nu("docs/b.md", 1, ""),
        ];

        assert!(confronte(&blocs, liste).is_empty());

        // Un bloc de plus sous la même première ligne dépasse ce que la liste tolère.
        let mut plus = blocs.to_vec();
        plus.push(nu("docs/a.md", 20, "✓ demo"));
        let fautes = confronte(&plus, liste);
        assert_eq!(fautes.len(), 1);
        assert!(fautes[0].starts_with("docs/a.md:20 : "), "{fautes:?}");

        // Une entrée qui ne correspond plus à aucun bloc est une exemption fantôme.
        let fautes = confronte(&blocs[..2], liste);
        assert_eq!(fautes.len(), 1);
        assert!(fautes[0].contains("docs/b.md"), "{fautes:?}");
    }

    #[test]
    fn a_malformed_exemption_is_refused() {
        assert_eq!(confronte(&[], "docs/a.md sans séparateur\n").len(), 1);
    }
}

// --- Le tableau des codes d'erreur -------------------------------------------------

/// Les fichiers `.rs` de `racine`, récursivement.
fn collecte_sources(repertoire: &Path, trouvees: &mut Vec<PathBuf>) {
    let entrees = std::fs::read_dir(repertoire).expect("répertoire de sources lisible");

    for entree in entrees {
        let chemin = entree.expect("entrée lisible").path();

        if chemin.is_dir() {
            collecte_sources(&chemin, trouvees);
        } else if chemin.extension().is_some_and(|suffixe| suffixe == "rs") {
            trouvees.push(chemin);
        }
    }
}

/// Le corps de chaque `fn code(` de la crate, un par implémentation.
///
/// Les codes ne sont pas énumérables à l'exécution — ce sont des bras de `match` rendant
/// un `&'static str` — et c'est donc le texte des sources qu'on balaie, faute de mieux.
/// Le découpage se fait par comptage d'accolades depuis la signature ; la déclaration du
/// trait, qui finit sur `;`, rend un corps vide et se compte quand même. C'est leur
/// nombre qui dit qu'aucune implémentation n'a échappé au balayage : sans lui, un
/// découpage cassé rendrait zéro corps et le test passerait au vert.
fn corps_des_code(racine: &Path) -> Vec<String> {
    let mut fichiers = Vec::new();
    collecte_sources(racine, &mut fichiers);
    fichiers.sort();

    let mut corps = Vec::new();
    for fichier in fichiers {
        let texte = std::fs::read_to_string(&fichier).expect("source lisible");
        let octets = texte.as_bytes();
        let mut depuis = 0;

        while let Some(decalage) = texte[depuis..].find("fn code(") {
            let mut rang = depuis + decalage;
            while octets[rang] != b'{' && octets[rang] != b';' {
                rang += 1;
            }

            // La déclaration du trait n'a pas de corps : elle se compte, et n'apporte
            // aucun code.
            if octets[rang] == b';' {
                corps.push(String::new());
                depuis = rang + 1;
                continue;
            }

            let ouverture = rang;
            let mut profondeur = 0;
            loop {
                match octets[rang] {
                    b'{' => profondeur += 1,
                    b'}' => {
                        profondeur -= 1;
                        if profondeur == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                rang += 1;
            }

            corps.push(texte[ouverture..=rang].to_string());
            depuis = rang + 1;
        }
    }

    corps
}

/// Les littéraux d'un corps de `code()` qui ont la forme d'un code : snake_case ASCII.
///
/// Le corps d'un `fn code(&self) -> i32` n'en porte aucun, et n'en apporte donc aucun.
fn codes_du_corps(corps: &str) -> BTreeSet<String> {
    let mut trouves = BTreeSet::new();
    let mut reste = corps;

    while let Some(debut) = reste.find('"') {
        let apres = &reste[debut + 1..];
        let Some(fin) = apres.find('"') else {
            break;
        };

        let litteral = &apres[..fin];
        if !litteral.is_empty()
            && litteral.chars().all(|lettre| {
                lettre.is_ascii_lowercase() || lettre.is_ascii_digit() || lettre == '_'
            })
        {
            trouves.insert(litteral.to_string());
        }

        reste = &apres[fin + 1..];
    }

    trouves
}

/// Les codes que le tableau d'une page énumère, dans l'ordre où elle les écrit.
fn codes_du_tableau(page: &Path) -> Vec<String> {
    let contenu = std::fs::read_to_string(page).expect("page lisible");
    let mut lignes = contenu
        .lines()
        .skip_while(|ligne| !ligne.starts_with("| Code |"));

    lignes.next().expect("le tableau des codes a un en-tête");
    lignes.next().expect("le tableau des codes a un séparateur");

    lignes
        .take_while(|ligne| ligne.starts_with('|'))
        .map(|ligne| {
            ligne
                .split('`')
                .nth(1)
                .unwrap_or_else(|| panic!("ligne de tableau sans code : {ligne}"))
                .to_string()
        })
        .collect()
}

/// Le tableau des codes d'`agents.md` nomme exactement ce que le CLI peut rendre.
///
/// C'est le contrat sur lequel un agent branche : le message est français et peut changer
/// d'une relecture à l'autre, le `code` non. Rien ne le gardait — `parite.mjs` ne voit pas
/// les tableaux, et aucun test de `tests/` ne le lisait : toutes ses lignes étaient sans
/// garde, pas seulement les dernières ajoutées.
///
/// Le balayage se fait dans les deux sens. Un code rendu qu'on oublie d'inscrire laisse un
/// agent sans branche ; un code inscrit que plus rien ne rend l'envoie attendre une panne
/// qui n'arrivera pas.
#[test]
fn the_error_code_table_names_exactly_the_codes_the_cli_can_render() {
    let corps = corps_des_code(&common::depot().join("crates/rbs-cli/src"));
    assert_eq!(
        corps.len(),
        13,
        "treize `fn code(` sont attendus, la déclaration du trait et le code de sortie compris"
    );

    let mut sources = BTreeSet::new();
    for un in &corps {
        sources.extend(codes_du_corps(un));
    }
    assert!(
        sources.len() >= 50,
        "un balayage qui ne trouve presque rien passe au vert sans rien prouver : {} codes",
        sources.len()
    );

    let anglais = codes_du_tableau(&common::depot().join("docs/docs/guides/agents.md"));
    let francais = codes_du_tableau(
        &common::depot()
            .join("docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/agents.md"),
    );
    assert!(
        !anglais.is_empty(),
        "le tableau des codes n'a pas été trouvé dans la page anglaise"
    );
    assert_eq!(
        anglais, francais,
        "les deux tableaux doivent porter les mêmes codes, dans le même ordre"
    );

    let tableau: BTreeSet<String> = anglais.iter().cloned().collect();
    assert_eq!(
        tableau.len(),
        anglais.len(),
        "un code est inscrit deux fois au tableau"
    );

    let manquants: Vec<&String> = sources.difference(&tableau).collect();
    assert!(
        manquants.is_empty(),
        "codes qu'un `code()` rend et que le tableau n'inscrit pas : {manquants:?}"
    );

    let fantomes: Vec<&String> = tableau.difference(&sources).collect();
    assert!(
        fantomes.is_empty(),
        "codes inscrits au tableau que plus aucun `code()` ne rend : {fantomes:?}"
    );
}
