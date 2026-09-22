//! Les chiffres que la prose affirme, comparés à ce qu'ils comptent.
//!
//! Un nombre écrit dans une page ne suit pas le code : le README a longtemps annoncé huit
//! jalons quand il y en avait treize, et une page comptait douze ancres au-dessus d'une
//! sortie qui en affichait quatorze. La prose ne garde donc que les chiffres qui disent
//! quelque chose au lecteur, et chacun d'eux porte un marqueur qui le désigne — la page
//! nomme ce qu'il compte, ce test le recompte à la source.
//!
//! Le marqueur précède immédiatement le nombre, en chiffres ou en toutes lettres :
//! `{/* rbs:chiffre ancres */}twenty-two` sur le site, `<!-- rbs:chiffre ancres -->22` dans
//! un README, que GitHub et crates.io rendent sans MDX.

use std::path::{Path, PathBuf};

use crate::anchors::ANCRES;

/// Ce qu'un marqueur peut compter, et le compte qu'en donne le code.
fn sources() -> Vec<(&'static str, usize)> {
    vec![
        ("ancres", ANCRES.len()),
        (
            "ancres-optionnelles",
            ANCRES.iter().filter(|anchor| anchor.optional).count(),
        ),
        ("fragments", crate::templates::feature_names(None).len()),
    ]
}

/// Les README publiés : mêmes lecteurs que le site, et la même tentation d'y compter.
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

/// L'ouverture et la fermeture d'un marqueur, selon ce que le fichier sait taire.
#[derive(Clone, Copy)]
enum Forme {
    Mdx,
    Html,
}

impl Forme {
    fn bornes(self) -> (&'static str, &'static str) {
        match self {
            Forme::Mdx => ("{/* rbs:chiffre ", "*/}"),
            Forme::Html => ("<!-- rbs:chiffre ", "-->"),
        }
    }
}

/// Un nombre désigné par un marqueur : où, ce qu'il compte, ce que la page en dit.
#[derive(Debug, PartialEq)]
struct Cite {
    ligne: usize,
    cle: String,
    valeur: Result<usize, String>,
}

/// Les nombres désignés dans `contenu`.
fn cites(contenu: &str, forme: Forme) -> Vec<Cite> {
    let (ouverture, fermeture) = forme.bornes();
    let mut trouves = Vec::new();

    for (rang, ligne) in contenu.lines().enumerate() {
        let mut reste = ligne;

        while let Some(debut) = reste.find(ouverture) {
            let apres = &reste[debut + ouverture.len()..];
            let Some(fin) = apres.find(fermeture) else {
                trouves.push(Cite {
                    ligne: rang + 1,
                    cle: apres.trim().to_string(),
                    valeur: Err(format!("marqueur non refermé par `{fermeture}`")),
                });
                break;
            };

            let suite = &apres[fin + fermeture.len()..];
            trouves.push(Cite {
                ligne: rang + 1,
                cle: apres[..fin].trim().to_string(),
                valeur: nombre(suite).ok_or_else(|| {
                    format!(
                        "aucun nombre lisible juste après le marqueur : « {} »",
                        debut_de(suite)
                    )
                }),
            });
            reste = suite;
        }
    }

    trouves
}

fn debut_de(texte: &str) -> String {
    texte.chars().take(24).collect()
}

/// Le nombre qui ouvre `texte`, en chiffres ou en toutes lettres, anglaises ou françaises.
///
/// Un nombre coupé par un retour à la ligne n'est pas lu : le marqueur et son nombre
/// tiennent sur une ligne, sans quoi une mise en page déplacerait la garde en silence.
fn nombre(texte: &str) -> Option<usize> {
    let longueur = texte
        .find(|c: char| !(c.is_alphanumeric() || c == '-'))
        .unwrap_or(texte.len());
    let mot = texte[..longueur].to_lowercase();

    if mot.is_empty() {
        return None;
    }

    if mot.chars().all(|c| c.is_ascii_digit()) {
        return mot.parse().ok();
    }

    // « vingt et un » s'écrit en trois mots : lu comme « vingt », il mentirait d'une unité.
    let reste = &texte[longueur..];
    let complet = match reste.split_whitespace().take(2).collect::<Vec<_>>()[..] {
        ["et", "un" | "une", ..] => format!("{mot}-et-un"),
        _ => mot,
    };

    numeraux()
        .into_iter()
        .find_map(|(ecrit, valeur)| (ecrit == complet).then_some(valeur))
}

/// Les nombres de 0 à 69 en toutes lettres, dans les deux langues du site.
fn numeraux() -> Vec<(String, usize)> {
    const EN: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const EN_DIZAINES: [&str; 5] = ["twenty", "thirty", "forty", "fifty", "sixty"];
    const FR: [&str; 20] = [
        "zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix",
        "onze", "douze", "treize", "quatorze", "quinze", "seize", "dix-sept", "dix-huit",
        "dix-neuf",
    ];
    const FR_DIZAINES: [&str; 5] = ["vingt", "trente", "quarante", "cinquante", "soixante"];

    let mut tous: Vec<(String, usize)> = Vec::new();

    for (valeur, (en, fr)) in EN.iter().zip(FR).enumerate() {
        tous.push((en.to_string(), valeur));
        tous.push((fr.to_string(), valeur));
    }
    tous.push(("une".to_string(), 1));

    for (rang, (en, fr)) in EN_DIZAINES.iter().zip(FR_DIZAINES).enumerate() {
        let dizaine = 20 + 10 * rang;
        tous.push((en.to_string(), dizaine));
        tous.push((fr.to_string(), dizaine));

        for unite in 1..10 {
            tous.push((format!("{en}-{}", EN[unite]), dizaine + unite));
            let fr_unite = if unite == 1 { "et-un" } else { FR[unite] };
            tous.push((format!("{fr}-{fr_unite}"), dizaine + unite));
        }
    }

    tous
}

fn depot() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Chaque page du site dans les deux langues, et les README, avec leur forme de marqueur.
fn fichiers() -> Vec<(PathBuf, Forme)> {
    let depot = depot();
    let mut trouves = Vec::new();

    for racine in [
        "docs/docs",
        "docs/i18n/fr/docusaurus-plugin-content-docs/current",
    ] {
        collecte(&depot.join(racine), &mut trouves);
    }

    let mut fichiers: Vec<(PathBuf, Forme)> =
        trouves.into_iter().map(|page| (page, Forme::Mdx)).collect();
    fichiers.extend(
        READMES
            .iter()
            .map(|readme| (depot.join(readme), Forme::Html)),
    );
    fichiers
}

fn collecte(repertoire: &Path, trouves: &mut Vec<PathBuf>) {
    let entrees = std::fs::read_dir(repertoire)
        .unwrap_or_else(|erreur| panic!("{} illisible : {erreur}", repertoire.display()));

    for entree in entrees {
        let chemin = entree.expect("entrée lisible").path();

        if chemin.is_dir() {
            collecte(&chemin, trouves);
        } else if chemin
            .extension()
            .is_some_and(|suffixe| suffixe == "md" || suffixe == "mdx")
        {
            trouves.push(chemin);
        }
    }
}

/// Les fautes de `cites`, relevées dans `fichier` contre `sources`.
fn confronte(fichier: &str, cites: &[Cite], sources: &[(&str, usize)]) -> Vec<String> {
    cites
        .iter()
        .filter_map(|cite| {
            let Some((_, attendu)) = sources.iter().find(|(cle, _)| *cle == cite.cle) else {
                let connues: Vec<&str> = sources.iter().map(|(cle, _)| *cle).collect();
                return Some(format!(
                    "{fichier}:{} : `{}` ne compte rien de connu — clés admises : {}",
                    cite.ligne,
                    cite.cle,
                    connues.join(", ")
                ));
            };

            match &cite.valeur {
                Err(raison) => Some(format!("{fichier}:{} : {raison}", cite.ligne)),
                Ok(valeur) if valeur != attendu => Some(format!(
                    "{fichier}:{} : la page compte {valeur} `{}`, la source en compte {attendu}",
                    cite.ligne, cite.cle
                )),
                Ok(_) => None,
            }
        })
        .collect()
}

#[test]
fn every_figure_the_prose_cites_matches_what_it_counts() {
    let sources = sources();
    let mut fautes = Vec::new();
    let mut vues: Vec<&str> = Vec::new();

    for (fichier, forme) in fichiers() {
        let contenu = std::fs::read_to_string(&fichier)
            .unwrap_or_else(|erreur| panic!("{} illisible : {erreur}", fichier.display()));
        let trouves = cites(&contenu, forme);

        vues.extend(
            sources
                .iter()
                .filter(|(cle, _)| trouves.iter().any(|cite| cite.cle == *cle))
                .map(|(cle, _)| *cle),
        );
        let depot = depot();
        let relatif = fichier.strip_prefix(&depot).unwrap_or(&fichier);
        fautes.extend(confronte(
            &relatif.display().to_string(),
            &trouves,
            &sources,
        ));
    }

    // Une source que plus aucune page ne cite est une garde morte : un parcours cassé ne
    // verrait aucun marqueur, et le test passerait au vert sans rien comparer.
    for (cle, _) in &sources {
        if !vues.contains(cle) {
            fautes.push(format!(
                "aucune page ne cite `{cle}` : retirez la source, ou citez-la"
            ));
        }
    }

    assert!(fautes.is_empty(), "\n{}\n", fautes.join("\n"));
}

#[test]
fn a_figure_is_read_in_digits_or_in_words_of_either_language() {
    assert_eq!(nombre("22 anchors"), Some(22));
    assert_eq!(nombre("twenty-two in all"), Some(22));
    assert_eq!(nombre("Sixteen are shipped"), Some(16));
    assert_eq!(nombre("vingt-deux ancres"), Some(22));
    assert_eq!(nombre("Onze sont optionnelles"), Some(11));
    assert_eq!(nombre("dix-sept"), Some(17));
    assert_eq!(nombre("vingt et un fragments"), Some(21));
    assert_eq!(nombre("twenty"), Some(20));
    assert_eq!(nombre(" twenty-two"), None);
    assert_eq!(nombre("many"), None);
}

#[test]
fn a_marker_designates_the_figure_right_after_it() {
    let page = "Il y en a {/* rbs:chiffre ancres */}vingt-deux, dont \
                {/* rbs:chiffre ancres-optionnelles */}onze.\n\
                Rien ici : {/* rbs:chiffre fragments */}\nseize.";

    let trouves = cites(page, Forme::Mdx);

    assert_eq!(
        trouves
            .iter()
            .map(|c| (c.ligne, c.cle.as_str()))
            .collect::<Vec<_>>(),
        [(1, "ancres"), (1, "ancres-optionnelles"), (2, "fragments")]
    );
    assert_eq!(trouves[0].valeur, Ok(22));
    assert_eq!(trouves[1].valeur, Ok(11));
    assert!(
        trouves[2].valeur.is_err(),
        "un nombre renvoyé à la ligne n'est pas lu"
    );
}

#[test]
fn a_wrong_figure_or_an_unknown_key_is_named_with_its_line() {
    let sources = [("ancres", 22)];
    let trouves = cites(
        "<!-- rbs:chiffre ancres -->21 ancres, <!-- rbs:chiffre jalons -->treize jalons",
        Forme::Html,
    );

    let fautes = confronte("README.md", &trouves, &sources);

    assert_eq!(fautes.len(), 2, "{fautes:#?}");
    assert!(
        fautes[0].contains("README.md:1") && fautes[0].contains("21") && fautes[0].contains("22")
    );
    assert!(fautes[1].contains("`jalons`"));
}
