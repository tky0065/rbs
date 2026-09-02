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

const MARQUEUR: &str = "<!-- rbs:transcript";

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
    Some(reste.strip_suffix("-->").unwrap_or(reste))
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
    texte = masque_moteur(&texte);
    texte = masque_version(&texte);
    texte = masque_horodatage(&texte);
    texte = masque_duree(&texte);

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

/// `in 0.11s`, `en 1.2 s` → `<durée>`.
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
        if lettres.get(rang) == Some(&'m') {
            rang += 1;
        }
        if lettres.get(rang) != Some(&'s') {
            return None;
        }
        rang += 1;

        if lettres.get(rang).is_some_and(char::is_ascii_alphanumeric) {
            return None;
        }

        Some((rang, format!("{}<durée>", &prefixe)))
    })
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

mod extraction {
    use super::*;

    #[test]
    fn a_marker_carries_its_command_and_the_block_that_follows() {
        let page = "\
avant\n\
<!-- rbs:transcript cmd=\"rbs new demo\" -->\n\
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
        let page = "<!-- rbs:transcript cmd=\"rbs doctor\" base=\"oui\" extrait=\"oui\" dans=\"demo\" -->\n```text\n✓\n```\n";
        let trouve = &extrait(Path::new("page.md"), page)[0];

        assert!(trouve.base);
        assert!(trouve.extrait);
        assert_eq!(trouve.dans.as_deref(), Some("demo"));

        let sobre = "<!-- rbs:transcript cmd=\"rbs doctor\" -->\n```text\n✓\n```\n";
        let sobre = &extrait(Path::new("page.md"), sobre)[0];

        assert!(!sobre.base);
        assert!(!sobre.extrait);
        assert_eq!(sobre.dans, None);
        assert_eq!(sobre.setup, None);
        assert_eq!(sobre.invite, None);
    }

    #[test]
    fn the_prompt_a_page_shows_is_kept_apart_from_the_output() {
        let page = "<!-- rbs:transcript cmd=\"rbs new site --yes\" -->\n```text\n$ rbs new site --yes\n✓ site créé\n```\n";
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

    #[test]
    fn the_ellipsis_a_page_writes_for_a_temporary_path_meets_the_real_one() {
        let tmp = Path::new("/var/folders/x/T/.tmpAbC");

        assert_eq!(
            normalise("plan pour …/demo\n", tmp),
            normalise("plan pour /var/folders/x/T/.tmpAbC/demo\n", tmp)
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
        let porte = |suffixe: &str| {
            pages
                .iter()
                .any(|page| page.to_string_lossy().ends_with(suffixe))
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
