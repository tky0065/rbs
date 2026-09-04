//! Les transcripts marqués de la documentation rendent-ils encore ce que les pages
//! montrent ?
//!
//! Un bloc de transcript est un oracle : il dit ce qu'une commande rend. Un oracle qui
//! n'est jamais rejoué se périme sans bruit — quatre blocs ont ainsi vécu faux sur trois
//! axes à la fois, et la prose raisonnait sur leurs chiffres. `parite.mjs` ne voit que la
//! structure et les liens ; `integration_examples` ne couvre que le code d'`examples/`.
//! Ce test est le seul endroit d'où le mensonge d'une sortie citée est visible.

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
    // Sous Windows, clap tire l'`Usage:` du nom réel de l'exécutable et écrit `rbs.exe`,
    // là où la page montre la commande telle qu'on la tape. Le suffixe tombe des deux
    // côtés de la comparaison, qui reste sensible à tout le reste de la ligne.
    texte = texte.replace("rbs.exe", "rbs");
    texte = unifie_separateurs(&texte);
    texte = masque_moteur(&texte);
    texte = masque_version(&texte);
    texte = masque_horodatage(&texte);
    texte = masque_duree(&texte);
    texte = masque_adresse(&texte);

    let mut rendu = String::with_capacity(texte.len());
    for ligne in texte.lines() {
        let nette = ligne.trim_end();
        if nette.is_empty() {
            continue;
        }
        rendu.push_str(nette);
        rendu.push('\n');
    }

    rendu
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
    remplace_motif(texte, |lettres, debut| {
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
    remplace_motif(texte, |lettres, debut| {
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

/// `m20260902_122330` → `m<horodatage>` : le nom d'une migration porte l'instant où elle
/// a été créée.
fn masque_horodatage(texte: &str) -> String {
    remplace_motif(texte, |lettres, debut| {
        if lettres[debut] != 'm' || debut + 16 > lettres.len() {
            return None;
        }
        if !lettres[debut + 1..debut + 9]
            .iter()
            .all(char::is_ascii_digit)
        {
            return None;
        }
        if lettres[debut + 9] != '_' {
            return None;
        }
        if !lettres[debut + 10..debut + 16]
            .iter()
            .all(char::is_ascii_digit)
        {
            return None;
        }

        Some((debut + 16, "m<horodatage>".to_string()))
    })
}

/// `in 0.11s`, `en 1.2 s`, `in 1m 12s` → `<durée>`.
fn masque_duree(texte: &str) -> String {
    remplace_motif(texte, |lettres, debut| {
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

/// Balaye `texte` et remplace ce que `motif` reconnaît, de gauche à droite.
fn remplace_motif(
    texte: &str,
    motif: impl Fn(&[char], usize) -> Option<(usize, String)>,
) -> String {
    let lettres: Vec<char> = texte.chars().collect();
    let mut rendu = String::with_capacity(texte.len());
    let mut rang = 0;

    while rang < lettres.len() {
        match motif(&lettres, rang) {
            Some((fin, remplacement)) => {
                rendu.push_str(&remplacement);
                rang = fin;
            }
            None => {
                rendu.push(lettres[rang]);
                rang += 1;
            }
        }
    }

    rendu
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

    std::process::Command::new(&executable)
        .current_dir(repertoire)
        .args(&arguments)
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

    /// Windows imprime le chemin absolu du tmpdir avec des barres inverses, et son `Usage:`
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
            normalise("Usage: rbs.exe generate <COMMAND>\n", tmp),
            "Usage: rbs generate <COMMAND>\n"
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
