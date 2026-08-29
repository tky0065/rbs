//! Les points d'insertion du projet généré, et de quoi y écrire.
//!
//! Le CLI ne réécrit jamais d'AST : il insère dans des ancres en commentaires. Ce module
//! ne connaît que des chaînes — l'écriture sur disque appartient à ses appelants.

use std::fmt;

/// Un point d'insertion, et le fichier du projet qui le porte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Anchor {
    /// Nom tel qu'il paraît entre les chevrons : `features` pour `// <rbs:features>`.
    pub name: &'static str,
    /// Chemin du fichier porteur, relatif à la racine du projet.
    pub file: &'static str,
    /// Marqueur de commentaire du langage porteur : `//` en Rust, `#` en YAML.
    pub comment: &'static str,
    /// L'ancre peut légitimement manquer, son fichier porteur étant lui-même facultatif.
    ///
    /// `doctor` ne réclame pas une ancre optionnelle dont le fichier est absent : un
    /// projet SQLite n'a pas de compose, et n'a donc pas à passer pour incomplet.
    pub optional: bool,
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
}

/// Déclaration des modules de feature, en tête de `main.rs`.
pub(crate) const FEATURES: Anchor = Anchor {
    name: "features",
    file: "src/main.rs",
    comment: "//",
    optional: false,
};

/// Montage des routes d'une feature dans le routeur.
pub(crate) const ROUTES: Anchor = Anchor {
    name: "routes",
    file: "src/router.rs",
    comment: "//",
    optional: false,
};

/// Enregistrement des chemins d'une feature dans le document OpenAPI.
pub(crate) const OPENAPI: Anchor = Anchor {
    name: "openapi",
    file: "src/openapi.rs",
    comment: "//",
    optional: false,
};

/// Déclaration des fichiers de migration.
///
/// Distincte de [`MIGRATIONS`] : Rust interdit un `mod` non-inline dans un bloc, et la
/// déclaration ne peut donc pas tenir dans le `vec!` du `Migrator`.
pub(crate) const MIGRATION_MODULES: Anchor = Anchor {
    name: "migration_modules",
    file: "migration/src/lib.rs",
    comment: "//",
    optional: false,
};

/// Inscription des migrations dans le `Migrator`.
pub(crate) const MIGRATIONS: Anchor = Anchor {
    name: "migrations",
    file: "migration/src/lib.rs",
    comment: "//",
    optional: false,
};

/// Déclaration d'un champ partagé dans la struct `AppState`.
pub(crate) const STATE_CHAMPS: Anchor = Anchor {
    name: "state_champs",
    file: "src/state.rs",
    comment: "//",
    optional: false,
};

/// Initialisation de ce champ dans `AppState::new`.
///
/// Distincte de [`STATE_CHAMPS`] : un champ se déclare à un endroit et se construit à un
/// autre, et une ancre unique ne pourrait pas viser les deux.
pub(crate) const STATE_INIT: Anchor = Anchor {
    name: "state_init",
    file: "src/state.rs",
    comment: "//",
    optional: false,
};

/// Tâches de fond lancées au démarrage, l'état construit et le serveur pas encore lié.
///
/// Distincte de [`STATE_INIT`] : ce qui vit dans l'état est une valeur, ce qui vit ici est
/// une tâche, et une valeur ne peut pas se détacher elle-même.
pub(crate) const STARTUP: Anchor = Anchor {
    name: "startup",
    file: "src/main.rs",
    comment: "//",
    optional: false,
};

/// Déclaration des seeds dans le binaire qui les applique.
///
/// Seule ancre à vivre dans une invocation de `macro_rules!` : elle porte des
/// identifiants de module, que la macro déclare et enchaîne d'un même geste. Un `mod` non
/// inline ne s'écrit pas dans un bloc — c'est ce qui vaut deux ancres à la crate
/// `migration` — et la macro évite ici d'en poser une seconde.
pub(crate) const SEEDS: Anchor = Anchor {
    name: "seeds",
    file: "src/seeds/main.rs",
    comment: "//",
    optional: false,
};

/// Les points d'insertion du squelette.
///
/// La génération vise chaque ancre nommément ; `rbs doctor` parcourt cette liste pour
/// vérifier qu'un projet les porte toutes.
pub(crate) const ANCRES: [Anchor; 9] = [
    FEATURES,
    ROUTES,
    OPENAPI,
    MIGRATION_MODULES,
    MIGRATIONS,
    STATE_CHAMPS,
    STATE_INIT,
    STARTUP,
    SEEDS,
];

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

/// Insère `lines` dans `anchor`, juste avant sa balise fermante.
///
/// Une ligne déjà présente dans l'ancre n'est pas réécrite, et le contenu qui s'y trouve
/// déjà traverse l'insertion tel quel : le développeur a pu l'ordonner ou l'indenter à sa
/// façon, et rien ici ne le sait mieux que lui.
pub(crate) fn insert(source: &str, anchor: Anchor, lines: &[String]) -> Result<String, Missing> {
    let absente = || Missing { anchor };

    let (opening, _) = line_of(source, &anchor.opening()).ok_or_else(absente)?;
    let (closing, indentation) = line_of(source, &anchor.closing()).ok_or_else(absente)?;

    if closing < opening {
        return Err(absente());
    }

    let dedans = &source[opening..closing];
    let ajouts: String = groups(lines)
        .into_iter()
        .filter(|groupe| !groupe.iter().all(|line| contains(dedans, line)))
        .flatten()
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

/// Découpe les lignes à insérer en groupes indivisibles.
///
/// Une ligne d'attribut ou de commentaire ne vaut pas pour elle-même : elle qualifie la
/// ligne qui la suit. Dédupliquer sans elle amputait un champ de son `#[allow(…)]` dès
/// qu'un autre fragment en avait posé un, et laissait le champ orphelin.
///
/// Les lignes autonomes — les chemins OpenAPI d'une feature — forment chacune leur
/// groupe, et restent donc dédupliquées une à une.
fn groups(lines: &[String]) -> Vec<Vec<&String>> {
    let mut groups = Vec::new();
    let mut courant = Vec::new();

    for line in lines {
        // `# ` : un commentaire YAML. `#[` et `//` : leurs homologues Rust. Les trois
        // qualifient la ligne suivante et ne valent pas pour eux-mêmes.
        let qualifie = matches!(
            line.trim_start().get(..2),
            Some("#[") | Some("//") | Some("# ")
        );
        courant.push(line);

        if !qualifie {
            groups.push(std::mem::take(&mut courant));
        }
    }

    if !courant.is_empty() {
        groups.push(courant);
    }

    groups
}

/// Début de la ligne ne portant que `balise`, et l'indentation de cette ligne.
///
/// La ligne doit ne porter qu'elle : une balise citée dans une chaîne — le bloc à recoller
/// qu'affiche le CLI, par exemple — n'ouvre pas une ancre.
fn line_of(source: &str, balise: &str) -> Option<(usize, String)> {
    let mut debut = 0;

    for line in source.split_inclusive('\n') {
        if line.trim() == balise {
            let indentation = line[..line.len() - line.trim_start().len()].to_string();
            return Some((debut, indentation));
        }
        debut += line.len();
    }

    None
}

/// `line` figure-t-elle déjà dans le bloc, à l'indentation près ?
fn contains(block: &str, line: &str) -> bool {
    block
        .lines()
        .any(|existante| existante.trim() == line.trim())
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

    #[test]
    fn only_the_missing_lines_are_added() {
        let une_fois = insert(ROUTEUR, ROUTES, &lines(&["deja()"])).expect("l'ancre est présente");

        let rendered = insert(&une_fois, ROUTES, &lines(&["deja()", "nouvelle()"]))
            .expect("l'ancre est présente");

        assert_eq!(rendered.matches("deja()").count(), 1, "{rendered}");
        assert_eq!(rendered.matches("nouvelle()").count(), 1, "{rendered}");
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
            name: "services",
            file: "docker-compose.yml",
            comment: "#",
            optional: true,
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
            name: "services",
            file: "docker-compose.yml",
            comment: "#",
            optional: true,
        };
        let source = "services:\n  # <rbs:services>\n  # </rbs:services>\n";
        let lines = vec!["# le cache du projet".to_string(), "redis:".to_string()];

        let apres = insert(source, compose, &lines).expect("l'ancre est présente");

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
            name: "services",
            file: "docker-compose.yml",
            comment: "#",
            optional: true,
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
}
