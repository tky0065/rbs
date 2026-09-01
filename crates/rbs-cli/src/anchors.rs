//! Les points d'insertion du projet généré, et de quoi y écrire.
//!
//! Le CLI ne réécrit jamais d'AST : il insère dans des ancres en commentaires. Ce module
//! ne connaît que des chaînes — l'écriture sur disque appartient à ses appelants.

use std::borrow::Cow;
use std::fmt;
use std::path::Path;

/// Un point d'insertion, et le fichier du projet qui le porte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Anchor {
    /// Nom tel qu'il paraît entre les chevrons : `features` pour `// <rbs:features>`.
    ///
    /// Emprunté pour les ancres du registre ; possédé pour celles du modèle d'une entité,
    /// dont le nom porte celui de l'entité — `relations:users`.
    pub name: Cow<'static, str>,
    /// Chemin du fichier porteur, relatif à la racine du projet.
    ///
    /// Emprunté pour les ancres du registre, dont le fichier est fixe ; possédé pour
    /// celles du modèle d'une feature, dont il dépend du nom de cette feature.
    pub file: Cow<'static, str>,
    /// Marqueur de commentaire du langage porteur : `//` en Rust, `#` en YAML.
    pub comment: &'static str,
    /// Le bloc se maintient trié, au lieu d'empiler dans l'ordre d'arrivée.
    ///
    /// Réservé aux blocs dont le contenu est une liste que rustfmt ordonne lui-même —
    /// les `pub mod` des features. Un projet qui installe `auth`, lequel entraîne
    /// `rate-limit`, puis engendre un CRUD nommé entre les deux, verrait sinon son
    /// propre `cargo fmt --check` échouer sur une ligne qu'il n'a pas écrite. Les
    /// autres ancres gardent l'ordre d'arrivée, qui y porte du sens : les migrations
    /// s'appliquent dans l'ordre où elles sont nées.
    pub sorted: bool,
    /// L'ancre peut légitimement manquer, son fichier porteur étant lui-même facultatif.
    ///
    /// `doctor` ne réclame pas une ancre optionnelle dont le fichier est absent : un
    /// projet SQLite n'a pas de compose, et n'a donc pas à passer pour incomplet.
    pub optional: bool,
    /// Motif de la ligne après laquelle le bloc se repose, quand l'ancre a disparu.
    ///
    /// Une ancre effacée ne laisse rien derrière elle : `rbs doctor --fix` n'a pas d'autre
    /// moyen de savoir où elle vivait que cette ligne d'accroche, déclarée ici et vérifiée
    /// contre la template qui la porte. Vide pour les ancres dont l'accroche dépend du
    /// contenu engendré, et que le diagnostic ne réclame pas.
    pub after: &'static str,
}

impl Anchor {
    /// Balise ouvrante, telle qu'elle est écrite dans le fichier.
    pub(crate) fn opening(&self) -> String {
        format!("{} <rbs:{}>", self.comment, self.name)
    }

    /// Balise fermante, telle qu'elle est écrite dans le fichier.
    pub(crate) fn closing(&self) -> String {
        format!("{} </rbs:{}>", self.comment, self.name)
    }

    /// Le bloc à recoller quand l'ancre a disparu, prêt à être collé tel quel.
    pub(crate) fn block(&self) -> String {
        format!("{}\n{}", self.opening(), self.closing())
    }

    /// La même ancre, dans un autre fichier.
    ///
    /// Sert aux ancres du modèle d'une feature : leur fichier n'est connu qu'à
    /// l'exécution, une fois le nom de la feature en main.
    pub(crate) fn in_file(&self, path: &str) -> Anchor {
        Anchor {
            file: Cow::Owned(path.to_string()),
            ..self.clone()
        }
    }

    /// La même ancre, dans le modèle de l'entité `table` et pour elle seule.
    ///
    /// Un fichier de modèle peut porter plusieurs entités — `src/auth/model.rs` en porte
    /// deux, nichées dans leurs modules — et l'ancre s'y répète autant de fois. Le nom de
    /// l'entité l'accompagne donc entre les chevrons, sans quoi une relation vers la
    /// seconde irait s'écrire dans la première, seule que le fichier rencontre.
    pub(crate) fn for_entity(&self, path: &str, table: &str) -> Anchor {
        Anchor {
            name: Cow::Owned(format!("{}:{table}", self.name)),
            file: Cow::Owned(path.to_string()),
            ..self.clone()
        }
    }
}

/// Déclaration des modules de feature, en tête de `main.rs`.
pub(crate) const FEATURES: Anchor = Anchor {
    name: Cow::Borrowed("features"),
    file: Cow::Borrowed("src/main.rs"),
    comment: "//",
    sorted: true,
    optional: false,
    after: "pub mod state;",
};

/// Montage des routes d'une feature dans le routeur.
pub(crate) const ROUTES: Anchor = Anchor {
    name: Cow::Borrowed("routes"),
    file: Cow::Borrowed("src/router.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: ".merge(health::routes())",
};

/// Middlewares qu'une feature empile sur le routeur.
///
/// Distincte de [`ROUTES`], bien qu'elles partagent leur fichier : une route se monte sur
/// le routeur, une couche l'enveloppe, et l'endroit où l'une se déclare est précisément
/// celui où l'autre n'aurait aucun effet.
pub(crate) const LAYERS: Anchor = Anchor {
    name: Cow::Borrowed("layers"),
    file: Cow::Borrowed("src/router.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: ".merge(docs)",
};

/// Enregistrement des chemins d'une feature dans le document OpenAPI.
pub(crate) const OPENAPI: Anchor = Anchor {
    name: Cow::Borrowed("openapi"),
    file: Cow::Borrowed("src/openapi.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "crate::health::controller::health,",
};

/// Déclaration des fichiers de migration.
///
/// Distincte de [`MIGRATIONS`] : Rust interdit un `mod` non-inline dans un bloc, et la
/// déclaration ne peut donc pas tenir dans le `vec!` du `Migrator`.
pub(crate) const MIGRATION_MODULES: Anchor = Anchor {
    name: Cow::Borrowed("migration_modules"),
    file: Cow::Borrowed("migration/src/lib.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "pub use sea_orm_migration::prelude::*;",
};

/// Inscription des migrations dans le `Migrator`.
pub(crate) const MIGRATIONS: Anchor = Anchor {
    name: Cow::Borrowed("migrations"),
    file: Cow::Borrowed("migration/src/lib.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "vec![",
};

/// Déclaration d'un champ partagé dans la struct `AppState`.
pub(crate) const STATE_CHAMPS: Anchor = Anchor {
    name: Cow::Borrowed("state_champs"),
    file: Cow::Borrowed("src/state.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "core: CoreState,",
};

/// Initialisation de ce champ dans `AppState::new`.
///
/// Distincte de [`STATE_CHAMPS`] : un champ se déclare à un endroit et se construit à un
/// autre, et une ancre unique ne pourrait pas viser les deux.
pub(crate) const STATE_INIT: Anchor = Anchor {
    name: Cow::Borrowed("state_init"),
    file: Cow::Borrowed("src/state.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "core: CoreState::new(db, config),",
};

/// Tâches de fond lancées au démarrage, l'état construit et le serveur pas encore lié.
///
/// Distincte de [`STATE_INIT`] : ce qui vit dans l'état est une valeur, ce qui vit ici est
/// une tâche, et une valeur ne peut pas se détacher elle-même.
pub(crate) const STARTUP: Anchor = Anchor {
    name: Cow::Borrowed("startup"),
    file: Cow::Borrowed("src/main.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "let state = state::AppState::new(db, config)?;",
};

/// Déclaration des seeds dans le binaire qui les applique.
///
/// Seule ancre à vivre dans une invocation de `macro_rules!` : elle porte des
/// identifiants de module, que la macro déclare et enchaîne d'un même geste. Un `mod` non
/// inline ne s'écrit pas dans un bloc — c'est ce qui vaut deux ancres à la crate
/// `migration` — et la macro évite ici d'en poser une seconde.
pub(crate) const SEEDS: Anchor = Anchor {
    name: Cow::Borrowed("seeds"),
    file: Cow::Borrowed("src/seeds/main.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "seeds! {",
};

/// Services que les fragments ajoutent au compose du projet.
///
/// Optionnelle : un projet SQLite, un projet visant une base distante, un projet dont
/// l'URL ne porte pas d'identifiants et tout projet créé avant la 1.1.0 n'ont pas de
/// compose, et n'ont donc pas cette ancre à porter.
pub(crate) const SERVICES: Anchor = Anchor {
    name: Cow::Borrowed("services"),
    file: Cow::Borrowed("docker-compose.yml"),
    comment: "#",
    sorted: false,
    optional: true,
    after: "services:",
};

/// Variantes de l'énumération `Relation` du modèle d'une entité.
///
/// Hors du registre statique : son fichier et son nom dépendent tous deux de l'entité
/// visée, et se fixent par [`Anchor::for_entity`] une fois celle-ci connue — l'ancre
/// écrite est `<rbs:relations:users>`, non `<rbs:relations>`.
pub(crate) const RELATIONS: Anchor = Anchor {
    name: Cow::Borrowed("relations"),
    file: Cow::Borrowed("src/{feature}/model.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "",
};

/// Implémentations de `Related` du modèle d'une entité.
///
/// Hors du registre statique, pour la même raison que [`RELATIONS`].
pub(crate) const RELATED: Anchor = Anchor {
    name: Cow::Borrowed("related"),
    file: Cow::Borrowed("src/{feature}/model.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "",
};

/// Les points d'insertion du squelette.
///
/// La génération vise chaque ancre nommément ; `rbs doctor` parcourt cette liste pour
/// vérifier qu'un projet les porte toutes.
pub(crate) const ANCRES: [Anchor; 11] = [
    FEATURES,
    ROUTES,
    LAYERS,
    OPENAPI,
    MIGRATION_MODULES,
    MIGRATIONS,
    STATE_CHAMPS,
    STATE_INIT,
    STARTUP,
    SEEDS,
    SERVICES,
];

/// Résout l'ancre `<rbs:features>` par repli, entre `src/lib.rs` et `src/main.rs`.
///
/// Elle vise `src/lib.rs`, que porte tout projet engendré depuis ce jalon : le binaire
/// principal et celui des seeds y puisent les modules de feature par un chemin de crate,
/// et non plus par `#[path]`. Un projet engendré plus tôt n'a pas de bibliothèque, et
/// l'ancre y reste dans `src/main.rs`, où elle a toujours vécu — sans ce repli, `generate`
/// et `doctor` cesseraient de fonctionner sur l'ensemble du parc existant.
pub(crate) fn resolve_features(root: &Path) -> Anchor {
    resolve(FEATURES, has_library(root))
}

/// La même ancre, celle des features visant la bibliothèque quand le projet en porte une.
///
/// La règle n'existe qu'ici : quatre appelants la réécrivaient, et une ancre ajoutée au
/// registre demandait de les visiter tous.
pub(crate) fn resolve(anchor: Anchor, with_library: bool) -> Anchor {
    if with_library && anchor.name == FEATURES.name {
        anchor.in_file("src/lib.rs")
    } else {
        anchor
    }
}

/// Les ancres du registre, celle des features résolue pour `root`.
///
/// Le disque n'est interrogé qu'une fois pour les onze, et non une fois par ancre.
pub(crate) fn resolved(root: &Path) -> Vec<Anchor> {
    let with_library = has_library(root);

    ANCRES
        .into_iter()
        .map(|anchor| resolve(anchor, with_library))
        .collect()
}

/// La seule question que la résolution des ancres pose au disque.
fn has_library(root: &Path) -> bool {
    root.join("src/lib.rs").exists()
}

/// Une ancre attendue que le fichier ne porte pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Missing {
    pub anchor: Anchor,
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ancre {} introuvable dans {}",
            self.anchor.opening(),
            self.anchor.file
        )
    }
}

impl std::error::Error for Missing {}

/// Ce qui empêche de reposer une ancre disparue.
///
/// Chaque variante est une raison de s'abstenir, jamais un échec de la commande : une
/// ancre reposée au mauvais endroit coûte plus cher qu'une ancre laissée absente — un
/// `<rbs:layers>` glissé sous `request_id` cesserait de voir l'identifiant de la requête,
/// et rien ne le dirait avant la lecture d'un journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cause {
    /// Le fichier porteur n'existe pas : il n'y a pas de ligne où s'accrocher.
    FichierAbsent,
    /// L'ancre ne déclare pas d'accroche, sa position dépendant du contenu engendré.
    SansAccroche,
    /// Une des deux balises est encore là.
    ///
    /// Reposer le bloc entier doublerait celle qui reste, et l'endroit d'une balise seule
    /// ne se déduit pas de l'autre : entre les deux, il y a le corps de l'ancre.
    Partielle,
    /// Aucune ligne du fichier ne porte l'accroche.
    Introuvable,
    /// Plusieurs lignes la portent, et rien ne désigne la bonne.
    Ambigue(usize),
}

impl Cause {
    /// Ce qui est dit à l'utilisateur, l'ancre en cause en main.
    pub(crate) fn raison(&self, anchor: &Anchor) -> String {
        match self {
            Self::FichierAbsent => format!("{} est introuvable", anchor.file),
            Self::SansAccroche => {
                format!(
                    "aucune ligne d'accroche n'est déclarée pour {}",
                    anchor.name
                )
            }
            Self::Partielle => format!(
                "{} ou {} est encore là : la balise restante ne dit pas où reposer l'autre",
                anchor.opening(),
                anchor.closing()
            ),
            Self::Introuvable => format!(
                "la ligne d'accroche `{}` est introuvable dans {}",
                anchor.after, anchor.file
            ),
            Self::Ambigue(fois) => format!(
                "la ligne d'accroche `{}` paraît {fois} fois dans {}",
                anchor.after, anchor.file
            ),
        }
    }
}

/// Repose le bloc de `anchor` dans `source`, sous la ligne d'accroche que l'ancre déclare.
///
/// Le bloc est vide : la réparation rend au projet un point d'insertion, elle ne devine
/// pas ce qu'il portait.
pub(crate) fn repose(source: &str, anchor: &Anchor) -> Result<String, Cause> {
    if marks(source, &anchor.opening()) || marks(source, &anchor.closing()) {
        return Err(Cause::Partielle);
    }

    if anchor.after.is_empty() {
        return Err(Cause::SansAccroche);
    }

    let accroches: Vec<usize> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.trim() == anchor.after)
        .map(|(index, _)| index)
        .collect();

    let [accroche] = accroches[..] else {
        return Err(match accroches.len() {
            0 => Cause::Introuvable,
            fois => Cause::Ambigue(fois),
        });
    };

    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    let indentation = indentation(&lines[accroche], anchor.comment);

    lines.insert(accroche + 1, format!("{indentation}{}", anchor.closing()));
    lines.insert(accroche + 1, format!("{indentation}{}", anchor.opening()));

    let mut rendu = lines.join("\n");
    // `lines()` mange le saut final : le rendre au fichier qui en portait un évite un
    // diff d'une ligne sur un fichier que la réparation n'a fait qu'ouvrir.
    if source.ends_with('\n') {
        rendu.push('\n');
    }

    Ok(rendu)
}

/// L'indentation que prend le bloc reposé sous sa ligne d'accroche.
///
/// Une ligne qui ouvre un bloc — `vec![`, `seeds! {`, `services:` — indente d'un cran ce
/// qui la suit ; les autres la partagent. Le pas est celui du langage porteur, que le
/// marqueur de commentaire désigne : quatre colonnes en Rust, deux en YAML, où poser le
/// bloc à côté ferait insérer un service hors de `services:`.
fn indentation(accroche: &str, comment: &str) -> String {
    let propre = accroche.trim();
    let courante = &accroche[..accroche.len() - accroche.trim_start().len()];

    if propre.ends_with(['[', '{', '(', ':']) {
        let pas = if comment == "#" { "  " } else { "    " };
        format!("{courante}{pas}")
    } else {
        courante.to_string()
    }
}

/// Insère `lines` dans `anchor`, juste avant sa balise fermante.
///
/// L'insertion est tout ou rien : `lines` forme une unité — la variante d'une relation et
/// son attribut, les trois lignes d'un `impl Related`, le bloc d'un service compose — et
/// n'est écrite que si l'ancre ne la porte pas déjà en entier, dans cet ordre et d'un
/// seul tenant. Dédupliquer ligne à ligne amputait un bloc de celles qu'un bloc voisin
/// avait déjà déposées : une accolade fermante, un `#[allow(…)]`, une clé `ports:`.
///
/// Le contenu déjà présent traverse l'insertion tel quel : le développeur a pu l'ordonner
/// ou l'indenter à sa façon, et rien ici ne le sait mieux que lui.
pub(crate) fn insert(source: &str, anchor: Anchor, lines: &[String]) -> Result<String, Missing> {
    // Fermeture appelée jusqu'à deux fois : sans le `.clone()`, `Missing { anchor }`
    // consommerait `anchor` dès le premier appel, empêchant le second.
    let absente = || Missing {
        anchor: anchor.clone(),
    };

    let (opening, _) = line_of(source, &anchor.opening()).ok_or_else(absente)?;
    let (closing, indentation) = line_of(source, &anchor.closing()).ok_or_else(absente)?;

    if closing < opening {
        return Err(absente());
    }

    if contains(&source[opening..closing], lines) {
        return Ok(source.to_string());
    }

    if anchor.sorted {
        // Le bloc est réécrit entier plutôt que complété : la ligne nouvelle doit pouvoir
        // se glisser entre deux anciennes, ce qu'une insertion avant la balise fermante ne
        // permet pas.
        // `opening` et `closing` sont les débuts des deux lignes de balise : le corps
        // commence après le saut de ligne de la première.
        let debut = source[opening..closing]
            .find('\n')
            .map_or(closing, |fin| opening + fin + 1);
        let mut corps: Vec<&str> = source[debut..closing]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .chain(lines.iter().map(String::as_str))
            .collect();
        corps.sort_unstable();
        corps.dedup();

        let bloc: String = corps
            .iter()
            .map(|line| format!("{indentation}{line}\n"))
            .collect();

        return Ok(format!("{}{bloc}{}", &source[..debut], &source[closing..]));
    }

    let ajouts: String = lines
        .iter()
        .map(|line| format!("{indentation}{line}\n"))
        .collect();

    Ok(format!(
        "{}{ajouts}{}",
        &source[..closing],
        &source[closing..]
    ))
}

/// Ce que l'ancre contient, entre ses deux balises, ou `None` si elle est absente.
///
/// `rbs seed` s'en sert pour distinguer un projet sans seed déclaré d'un projet qui en a :
/// le premier n'a aucune raison de lancer cargo.
pub(crate) fn body(source: &str, anchor: Anchor) -> Option<&str> {
    let (opening, _) = line_of(source, &anchor.opening())?;
    let (closing, _) = line_of(source, &anchor.closing())?;

    if closing < opening {
        return None;
    }

    let apres_ouverture = opening + source[opening..].find('\n').map_or(0, |fin| fin + 1);

    Some(&source[apres_ouverture.min(closing)..closing])
}

/// `source` porte-t-elle une ligne ne portant que `balise` ?
///
/// Seule définition de « la balise est là ». `doctor` l'interroge pour annoncer une ancre
/// présente, `line_of` pour la situer et y écrire : les deux réponses divergeraient
/// autrement, et un projet où l'insertion échoue passerait pour sain.
pub(crate) fn marks(source: &str, balise: &str) -> bool {
    source.lines().any(|line| line.trim() == balise)
}

/// Début de la ligne ne portant que `balise`, et l'indentation de cette ligne.
///
/// La ligne doit ne porter qu'elle : une balise citée dans une chaîne — le bloc à recoller
/// qu'affiche le CLI, par exemple — n'ouvre pas une ancre.
fn line_of(source: &str, balise: &str) -> Option<(usize, String)> {
    let mut debut = 0;

    for line in source.split_inclusive('\n') {
        if marks(line, balise) {
            let indentation = line[..line.len() - line.trim_start().len()].to_string();
            return Some((debut, indentation));
        }
        debut += line.len();
    }

    None
}

/// `lines` figure-t-elle déjà dans le bloc, d'un seul tenant et à l'indentation près ?
///
/// La contiguïté est ce qui rend le prédicat sûr sur un bloc multiligne : deux blocs
/// voisins partagent volontiers une ligne — une accolade fermante, un `#[allow(…)]`, une
/// clé `ports:` — et chercher ces lignes séparément conclurait que le bloc entier est
/// déjà posé.
fn contains(block: &str, lines: &[String]) -> bool {
    if lines.is_empty() {
        return true;
    }

    let present: Vec<&str> = block.lines().map(str::trim).collect();
    let cherchees: Vec<&str> = lines.iter().map(|line| line.trim()).collect();

    present
        .windows(cherchees.len())
        .any(|fenetre| fenetre == cherchees.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTEUR: &str = "\
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        // <rbs:routes>
        // </rbs:routes>
        .merge(docs)
}
";

    fn lines(sources: &[&str]) -> Vec<String> {
        sources.iter().map(|s| (*s).to_string()).collect()
    }

    /// Le cas qui a motivé le tri : `add auth` entraîne `rate-limit`, et le CRUD
    /// engendré ensuite s'intercale alphabétiquement entre les deux.
    #[test]
    fn a_sorted_anchor_keeps_its_block_in_order() {
        let source = "// <rbs:features>\npub mod auth;\npub mod rate_limit;\n// </rbs:features>\n";

        let obtenu = insert(source, FEATURES, &["pub mod posts;".to_string()])
            .expect("l'ancre est présente");

        assert_eq!(
            obtenu,
            "// <rbs:features>\npub mod auth;\npub mod posts;\npub mod rate_limit;\n// </rbs:features>\n",
            "le bloc doit rester dans l'ordre que rustfmt impose : {obtenu}"
        );
    }

    #[test]
    fn an_unsorted_anchor_still_stacks_in_arrival_order() {
        let source = "// <rbs:migrations>\nBox::new(m2_b::Migration),\n// </rbs:migrations>\n";

        let obtenu = insert(
            source,
            MIGRATIONS,
            &["Box::new(m1_a::Migration),".to_string()],
        )
        .expect("l'ancre est présente");

        assert!(
            obtenu.contains("Box::new(m2_b::Migration),\nBox::new(m1_a::Migration),"),
            "l'ordre des migrations est chronologique, pas alphabétique : {obtenu}"
        );
    }

    #[test]
    fn the_insertion_lands_just_before_the_closing_tag() {
        let rendered = insert(ROUTEUR, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        assert!(
            rendered.contains(
                "        // <rbs:routes>\n        \
                 .merge(crate::users::routes())\n        // </rbs:routes>"
            ),
            "insertion mal placée :\n{rendered}"
        );
    }

    #[test]
    fn the_indentation_is_that_of_the_closing_tag() {
        let rendered = insert(ROUTEUR, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        let inserted = rendered
            .lines()
            .find(|line| line.contains("users::routes"))
            .expect("la ligne doit être insérée");

        assert_eq!(inserted, "        .merge(crate::users::routes())");
    }

    #[test]
    fn several_lines_keep_the_order_they_are_given_in() {
        let rendered = insert(
            ROUTEUR,
            ROUTES,
            &lines(&["premiere()", "deuxieme()", "troisieme()"]),
        )
        .expect("l'ancre est présente");

        let rangs: Vec<usize> = ["premiere()", "deuxieme()", "troisieme()"]
            .iter()
            .map(|line| rendered.find(line).expect("ligne insérée"))
            .collect();

        assert!(rangs[0] < rangs[1] && rangs[1] < rangs[2], "{rendered}");
    }

    /// Le critère du lot : ce que le développeur a écrit dans l'ancre lui appartient.
    #[test]
    fn the_existing_content_is_neither_reordered_nor_reformatted() {
        let peuple = "\
pub fn router(state: AppState) -> Router {
    Router::new()
        // <rbs:routes>
            .merge(crate::zebres::routes())
        .merge(crate::abeilles::routes())
        // un commentaire du développeur
        // </rbs:routes>
}
";

        let rendered = insert(peuple, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        let expected = peuple.replace(
            "        // </rbs:routes>",
            "        .merge(crate::users::routes())\n        // </rbs:routes>",
        );
        assert_eq!(rendered, expected, "le contenu existant a bougé");
    }

    #[test]
    fn an_already_present_line_is_not_reinserted() {
        let une_fois = insert(ROUTEUR, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        let deux_fois = insert(
            &une_fois,
            ROUTES,
            &lines(&[".merge(crate::users::routes())"]),
        )
        .expect("l'ancre est présente");

        assert_eq!(deux_fois, une_fois, "la seconde insertion a réécrit");
    }

    /// Le cœur de la règle : une séquence n'est réputée posée que si l'ancre la porte en
    /// entier. Une ligne commune à deux blocs ne suffit pas à tenir le second pour écrit.
    #[test]
    fn a_sequence_only_partly_present_is_written_whole() {
        let une_fois = insert(ROUTEUR, ROUTES, &lines(&["deja()"])).expect("l'ancre est présente");

        let rendered = insert(&une_fois, ROUTES, &lines(&["deja()", "nouvelle()"]))
            .expect("l'ancre est présente");

        assert_eq!(rendered.matches("deja()").count(), 2, "{rendered}");
        assert_eq!(rendered.matches("nouvelle()").count(), 1, "{rendered}");
    }

    /// Le défaut que la déduplication ligne à ligne laissait passer : les trois lignes
    /// d'un `impl Related` n'en qualifient aucune, et l'accolade fermante du premier bloc
    /// faisait passer celle du second pour déjà écrite — le fichier sortait avec un
    /// délimiteur non refermé.
    #[test]
    fn a_second_block_sharing_a_closing_brace_keeps_its_own() {
        let source = "\
// <rbs:related>
// </rbs:related>
";
        let premier = insert(
            source,
            RELATED,
            &lines(&[
                "impl Related<crate::profiles::model::Entity> for Entity {",
                "    fn to() -> RelationDef {",
                "        Relation::Profiles.def()",
                "    }",
                "}",
            ]),
        )
        .expect("l'ancre est présente");

        let second = insert(
            &premier,
            RELATED,
            &lines(&[
                "impl Related<crate::notes::model::Entity> for Entity {",
                "    fn to() -> RelationDef {",
                "        Relation::Notes.def()",
                "    }",
                "}",
            ]),
        )
        .expect("l'ancre est présente");

        assert_eq!(
            second.matches('{').count(),
            second.matches('}').count(),
            "les délimiteurs ne s'équilibrent plus :\n{second}"
        );
        assert_eq!(second.matches("impl Related").count(), 2, "{second}");
    }

    /// Deux services YAML qui ouvrent chacun un `ports:` : le second ne doit pas perdre
    /// sa clé sous prétexte qu'un premier fragment en a déjà posé une. Sans le
    /// qualificatif des clés nues dans `groups`, `ports:` — présent dans le bloc depuis le
    /// premier service — se filtrait comme groupe à lui seul, et les deux lignes de liste
    /// du second atterrissaient sans l'en-tête qui les rattache à leur service.
    #[test]
    fn a_bare_key_shared_by_two_services_is_not_dropped_from_the_second() {
        let compose = "\
services:
  # <rbs:services>
  # </rbs:services>
";

        let premier = insert(
            compose,
            SERVICES,
            &lines(&[
                "redis:",
                "  image: redis:8-alpine",
                "  ports:",
                "    - \"6379:6379\"",
            ]),
        )
        .expect("l'ancre est présente");

        let second = insert(
            &premier,
            SERVICES,
            &lines(&[
                "mailpit:",
                "  image: axllent/mailpit:latest",
                "  ports:",
                "    - \"1025:1025\"",
                "    - \"8025:8025\"",
            ]),
        )
        .expect("l'ancre est présente");

        assert!(
            second.contains(
                "  mailpit:\n    image: axllent/mailpit:latest\n    ports:\n      \
                 - \"1025:1025\"\n      - \"8025:8025\""
            ),
            "le second service a perdu sa clé ports: :\n{second}"
        );
    }

    #[test]
    fn a_missing_anchor_is_reported_with_its_file() {
        let error = insert("fn main() {}\n", ROUTES, &lines(&["peu importe"]))
            .expect_err("l'ancre est absente");

        assert_eq!(error.anchor, ROUTES);
        assert_eq!(
            error.to_string(),
            "ancre // <rbs:routes> introuvable dans src/router.rs"
        );
    }

    #[test]
    fn an_anchor_missing_its_closing_is_reported() {
        let tronque = "// <rbs:routes>\n";

        let error =
            insert(tronque, ROUTES, &lines(&["peu importe"])).expect_err("fermeture absente");

        assert_eq!(error.anchor, ROUTES);
    }

    /// Une occurrence citée dans du code — une chaîne, un message d'erreur — n'ouvre pas
    /// une ancre : seule une ligne qui ne porte qu'elle en est une.
    #[test]
    fn a_tag_quoted_mid_line_is_not_an_anchor() {
        let cite = "let aide = \"ajoute // <rbs:routes> puis // </rbs:routes>\";\n";

        let error = insert(cite, ROUTES, &lines(&["peu importe"])).expect_err("aucune ancre");

        assert_eq!(error.anchor, ROUTES);
    }

    /// Le prédicat qu'interroge `doctor` répond comme l'insertion : indentation tolérée,
    /// citation refusée.
    #[test]
    fn marks_accepts_an_indented_tag_and_refuses_a_quoted_one() {
        assert!(marks("    // <rbs:routes>\n", "// <rbs:routes>"));
        assert!(marks(ROUTEUR, "// <rbs:routes>"));
        assert!(!marks(
            "let aide = \"// <rbs:routes>\";\n",
            "// <rbs:routes>"
        ));
        assert!(!marks("", "// <rbs:routes>"));
    }

    #[test]
    fn an_untouched_anchor_has_an_empty_body() {
        let body = body(ROUTEUR, ROUTES).expect("l'ancre est présente");

        assert!(body.trim().is_empty(), "corps inattendu : {body:?}");
    }

    #[test]
    fn a_filled_anchor_gives_back_what_was_inserted() {
        let rempli = insert(ROUTEUR, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        let body = body(&rempli, ROUTES).expect("l'ancre est présente");

        assert!(body.contains(".merge(crate::users::routes())"), "{body:?}");
        assert!(
            !body.contains("<rbs:routes>"),
            "les balises ne font pas partie du corps : {body:?}"
        );
    }

    #[test]
    fn a_missing_anchor_has_no_body() {
        assert_eq!(body("fn main() {}\n", ROUTES), None);
    }

    #[test]
    fn the_block_to_paste_carries_both_tags_of_the_anchor() {
        assert_eq!(ROUTES.block(), "// <rbs:routes>\n// </rbs:routes>");
    }

    #[test]
    fn the_anchors_carry_distinct_names() {
        for (rang, anchor) in ANCRES.iter().enumerate() {
            assert!(
                !ANCRES[..rang].iter().any(|other| other.name == anchor.name),
                "`{}` déclarée deux fois",
                anchor.name
            );
        }
    }

    /// Deux fragments peuvent déclarer une même ligne — un attribut, le plus souvent —
    /// sans que le bloc de l'un rende celui de l'autre superflu. Dédupliquer ligne à
    /// ligne amputait le second de sa ligne commune et laissait le reste orphelin.
    #[test]
    fn the_block_is_written_whole_when_only_one_of_its_lines_is_already_there() {
        let source = "\
struct AppState {
    // <rbs:state_champs>
    #[allow(dead_code)]
    pub mail: Mailer,
    // </rbs:state_champs>
}
";

        let after = insert(
            source,
            STATE_CHAMPS,
            &[
                "#[allow(dead_code)]".to_string(),
                "pub storage: Arc<dyn Storage>,".to_string(),
            ],
        )
        .expect("l'ancre est présente");

        assert_eq!(
            after.matches("#[allow(dead_code)]").count(),
            2,
            "chaque champ porte le sien : {after}"
        );
        assert!(
            after.contains("pub storage: Arc<dyn Storage>,"),
            "le champ ne doit pas être laissé de côté : {after}"
        );
    }

    #[test]
    fn a_yaml_anchor_is_written_with_a_hash() {
        let compose = Anchor {
            name: Cow::Borrowed("services"),
            file: Cow::Borrowed("docker-compose.yml"),
            comment: "#",
            sorted: false,
            optional: true,
            after: "services:",
        };

        assert_eq!(compose.opening(), "# <rbs:services>");
        assert_eq!(compose.closing(), "# </rbs:services>");
        assert_eq!(compose.block(), "# <rbs:services>\n# </rbs:services>");
    }

    #[test]
    fn the_rust_anchors_keep_their_double_slash() {
        for anchor in ANCRES {
            if anchor.comment == "//" {
                assert_eq!(anchor.opening(), format!("// <rbs:{}>", anchor.name));
            }
        }
    }

    /// Un commentaire YAML qualifie le service qui le suit, comme `#[allow(…)]` qualifie
    /// le champ Rust qui le suit : les dédupliquer séparément laisserait l'un des deux
    /// orphelin.
    #[test]
    fn a_yaml_comment_stays_attached_to_the_line_below_it() {
        let compose = Anchor {
            name: Cow::Borrowed("services"),
            file: Cow::Borrowed("docker-compose.yml"),
            comment: "#",
            sorted: false,
            optional: true,
            after: "services:",
        };
        let source = "services:\n  # <rbs:services>\n  # </rbs:services>\n";
        let lines = vec!["# le cache du projet".to_string(), "redis:".to_string()];

        let apres = insert(source, compose.clone(), &lines).expect("l'ancre est présente");

        assert!(
            apres.contains("  # le cache du projet\n  redis:\n"),
            "le commentaire doit précéder son service :\n{apres}"
        );

        let deux_fois = insert(&apres, compose, &lines).expect("l'ancre est toujours là");
        assert_eq!(
            deux_fois.matches("redis:").count(),
            1,
            "une seconde insertion ne doit rien ajouter :\n{deux_fois}"
        );
    }

    /// Le cas asymétrique, seul à distinguer les deux comportements : le commentaire est
    /// déjà dans l'ancre, la ligne qu'il qualifie ne l'est pas encore. Sans le
    /// groupement, le commentaire passerait pour posé et le service s'insérerait seul,
    /// sous un commentaire qui ne le concerne pas.
    #[test]
    fn a_yaml_comment_already_present_does_not_orphan_the_line_it_qualifies() {
        let compose = Anchor {
            name: Cow::Borrowed("services"),
            file: Cow::Borrowed("docker-compose.yml"),
            comment: "#",
            sorted: false,
            optional: true,
            after: "services:",
        };
        // Une autre feature a posé ce commentaire, et un service qui l'en sépare.
        let source = "services:\n  # <rbs:services>\n  # le cache du projet\n  memcached:\n  # </rbs:services>\n";
        let lines = vec!["# le cache du projet".to_string(), "redis:".to_string()];

        let apres = insert(source, compose, &lines).expect("l'ancre est présente");

        assert!(
            apres.contains("  # le cache du projet\n  redis:\n"),
            "le service doit arriver avec son propre commentaire :\n{apres}"
        );
        assert!(
            !apres.contains("  memcached:\n  redis:\n"),
            "le service ne doit pas s'insérer nu sous un commentaire étranger :\n{apres}"
        );
    }

    // `SERVICES` étant un `const`, clippy évalue `SERVICES.optional` à la compilation et
    // signale l'assertion comme triviale ; elle mord pourtant si quelqu'un change le
    // champ, ce que clippy ne voit pas.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn the_services_anchor_lives_in_the_compose_and_is_optional() {
        assert_eq!(SERVICES.file, "docker-compose.yml");
        assert_eq!(SERVICES.comment, "#");
        assert!(SERVICES.optional);
        assert!(ANCRES.contains(&SERVICES));
    }

    /// Les deux ancres du routeur partagent leur fichier et ne se confondent pas : une
    /// ligne montée dans `routes` n'enveloppe rien, une couche posée dans `layers`
    /// n'expose aucune route.
    #[test]
    fn the_router_carries_the_routes_and_the_layers_anchors_apart() {
        assert_eq!(LAYERS.file, ROUTES.file);
        assert_ne!(LAYERS.name, ROUTES.name);
        assert_eq!(LAYERS.opening(), "// <rbs:layers>");
        assert!(ANCRES.contains(&LAYERS));
    }

    /// Une ancre optionnelle est l'exception : toutes les autres décrivent un fichier que
    /// le squelette écrit toujours, et leur absence est un défaut.
    #[test]
    fn only_the_services_anchor_is_optional() {
        let optionnelles: Vec<&str> = ANCRES
            .iter()
            .filter(|anchor| anchor.optional)
            .map(|anchor| anchor.name.as_ref())
            .collect();

        assert_eq!(optionnelles, ["services"]);
    }

    #[test]
    fn an_anchor_can_be_rebound_to_a_computed_file() {
        let anchor = RELATIONS.in_file("src/posts/model.rs");

        assert_eq!(anchor.file, "src/posts/model.rs");
        assert_eq!(anchor.name, RELATIONS.name);
        assert_eq!(anchor.opening(), "// <rbs:relations>");
    }

    /// Le nom de l'entité rejoint celui de l'ancre : `src/auth/model.rs` porte deux
    /// paires, et une relation vers la seconde entité n'a que ce nom pour la viser.
    #[test]
    fn a_model_anchor_carries_the_name_of_its_entity() {
        let anchor = RELATIONS.for_entity("src/auth/model.rs", "refresh_tokens");

        assert_eq!(anchor.file, "src/auth/model.rs");
        assert_eq!(anchor.opening(), "// <rbs:relations:refresh_tokens>");
        assert_eq!(anchor.closing(), "// </rbs:relations:refresh_tokens>");
    }

    #[test]
    fn two_entities_of_one_file_get_two_distinct_anchors() {
        let users = RELATED.for_entity("src/auth/model.rs", "users");
        let tokens = RELATED.for_entity("src/auth/model.rs", "refresh_tokens");

        assert_eq!(users.file, tokens.file);
        assert_ne!(users.name, tokens.name);
    }

    // Les deux ancres du modèle ne rejoignent pas le registre statique : leur fichier
    // dépend des features du projet, que `doctor` énumère autrement.
    #[test]
    fn the_model_anchors_are_absent_from_the_static_registry() {
        for anchor in ANCRES {
            assert_ne!(anchor.name, "relations", "{:?}", anchor);
            assert_ne!(anchor.name, "related", "{:?}", anchor);
        }
    }

    #[test]
    fn the_features_anchor_resolves_to_the_library_when_it_exists() {
        let project = tempfile::TempDir::new().expect("répertoire temporaire créable");
        std::fs::create_dir_all(project.path().join("src")).expect("le répertoire se crée");
        std::fs::write(project.path().join("src/lib.rs"), "// bibliothèque")
            .expect("l'écriture aboutit");

        let anchor = resolve_features(project.path());

        assert_eq!(anchor.file, "src/lib.rs");
        assert_eq!(anchor.name, FEATURES.name);
    }

    #[test]
    fn the_features_anchor_falls_back_to_main_without_a_library() {
        let project = tempfile::TempDir::new().expect("répertoire temporaire créable");

        let anchor = resolve_features(project.path());

        assert_eq!(anchor.file, "src/main.rs");
    }

    /// Une ancre ajoutée au registre paraît dans la liste résolue sans que personne ait à
    /// toucher les appelants : c'est ce que la liste unique achète.
    #[test]
    fn the_resolved_registry_carries_every_anchor_of_the_registry() {
        let project = tempfile::TempDir::new().expect("répertoire temporaire créable");

        let resolues = resolved(project.path());

        assert_eq!(resolues.len(), ANCRES.len());
        for anchor in ANCRES {
            assert!(
                resolues.iter().any(|autre| autre.name == anchor.name),
                "`{}` manque à la liste résolue",
                anchor.name
            );
        }
    }

    #[test]
    fn the_resolved_registry_follows_the_fallback_of_the_features_anchor() {
        let project = tempfile::TempDir::new().expect("répertoire temporaire créable");
        std::fs::create_dir_all(project.path().join("src")).expect("le répertoire se crée");
        std::fs::write(project.path().join("src/lib.rs"), "// bibliothèque")
            .expect("l'écriture aboutit");

        let features = resolved(project.path())
            .into_iter()
            .find(|anchor| anchor.name == FEATURES.name)
            .expect("l'ancre des features est au registre");

        assert_eq!(features.file, "src/lib.rs");
    }
}
