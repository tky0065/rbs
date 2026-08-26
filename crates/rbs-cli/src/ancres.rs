//! Les points d'insertion du projet généré, et de quoi y écrire.
//!
//! Le CLI ne réécrit jamais d'AST : il insère dans des ancres en commentaires. Ce module
//! ne connaît que des chaînes — l'écriture sur disque appartient à ses appelants.

use std::fmt;

/// Un point d'insertion, et le fichier du projet qui le porte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ancre {
    /// Nom tel qu'il paraît entre les chevrons : `features` pour `// <rbs:features>`.
    pub nom: &'static str,
    /// Chemin du fichier porteur, relatif à la racine du projet.
    pub fichier: &'static str,
}

impl Ancre {
    /// Balise ouvrante, telle qu'elle est écrite dans le fichier.
    pub(crate) fn ouverture(&self) -> String {
        format!("// <rbs:{}>", self.nom)
    }

    /// Balise fermante, telle qu'elle est écrite dans le fichier.
    pub(crate) fn fermeture(&self) -> String {
        format!("// </rbs:{}>", self.nom)
    }
}

/// Déclaration des modules de feature, en tête de `main.rs`.
pub(crate) const FEATURES: Ancre = Ancre {
    nom: "features",
    fichier: "src/main.rs",
};

/// Montage des routes d'une feature dans le routeur.
pub(crate) const ROUTES: Ancre = Ancre {
    nom: "routes",
    fichier: "src/router.rs",
};

/// Enregistrement des chemins d'une feature dans le document OpenAPI.
pub(crate) const OPENAPI: Ancre = Ancre {
    nom: "openapi",
    fichier: "src/openapi.rs",
};

/// Déclaration des fichiers de migration.
///
/// Distincte de [`MIGRATIONS`] : Rust interdit un `mod` non-inline dans un bloc, et la
/// déclaration ne peut donc pas tenir dans le `vec!` du `Migrator`.
pub(crate) const MIGRATION_MODULES: Ancre = Ancre {
    nom: "migration_modules",
    fichier: "migration/src/lib.rs",
};

/// Inscription des migrations dans le `Migrator`.
pub(crate) const MIGRATIONS: Ancre = Ancre {
    nom: "migrations",
    fichier: "migration/src/lib.rs",
};

/// Les cinq points d'insertion du squelette.
///
/// La génération vise chaque ancre nommément ; cette liste est celle que `rbs doctor`
/// parcourra pour vérifier qu'un projet les porte toutes.
#[allow(dead_code)]
pub(crate) const ANCRES: [Ancre; 5] = [FEATURES, ROUTES, OPENAPI, MIGRATION_MODULES, MIGRATIONS];

/// Une ancre attendue que le fichier ne porte pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Absente {
    pub ancre: Ancre,
}

impl fmt::Display for Absente {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ancre {} introuvable dans {}",
            self.ancre.ouverture(),
            self.ancre.fichier
        )
    }
}

impl std::error::Error for Absente {}

/// Insère `lignes` dans `ancre`, juste avant sa balise fermante.
///
/// Une ligne déjà présente dans l'ancre n'est pas réécrite, et le contenu qui s'y trouve
/// déjà traverse l'insertion tel quel : le développeur a pu l'ordonner ou l'indenter à sa
/// façon, et rien ici ne le sait mieux que lui.
pub(crate) fn inserer(source: &str, ancre: Ancre, lignes: &[String]) -> Result<String, Absente> {
    let absente = || Absente { ancre };

    let (ouverture, _) = ligne_de(source, &ancre.ouverture()).ok_or_else(absente)?;
    let (fermeture, indentation) = ligne_de(source, &ancre.fermeture()).ok_or_else(absente)?;

    if fermeture < ouverture {
        return Err(absente());
    }

    let dedans = &source[ouverture..fermeture];
    let ajouts: String = lignes
        .iter()
        .filter(|ligne| !contient(dedans, ligne))
        .map(|ligne| format!("{indentation}{ligne}\n"))
        .collect();

    Ok(format!(
        "{}{ajouts}{}",
        &source[..fermeture],
        &source[fermeture..]
    ))
}

/// Début de la ligne ne portant que `balise`, et l'indentation de cette ligne.
///
/// La ligne doit ne porter qu'elle : une balise citée dans une chaîne — le bloc à recoller
/// qu'affiche le CLI, par exemple — n'ouvre pas une ancre.
fn ligne_de(source: &str, balise: &str) -> Option<(usize, String)> {
    let mut debut = 0;

    for ligne in source.split_inclusive('\n') {
        if ligne.trim() == balise {
            let indentation = ligne[..ligne.len() - ligne.trim_start().len()].to_string();
            return Some((debut, indentation));
        }
        debut += ligne.len();
    }

    None
}

/// `ligne` figure-t-elle déjà dans le bloc, à l'indentation près ?
fn contient(bloc: &str, ligne: &str) -> bool {
    bloc.lines()
        .any(|existante| existante.trim() == ligne.trim())
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

    fn lignes(sources: &[&str]) -> Vec<String> {
        sources.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn l_insertion_se_place_juste_avant_la_balise_fermante() {
        let rendu = inserer(
            ROUTEUR,
            ROUTES,
            &lignes(&[".merge(crate::users::routes())"]),
        )
        .expect("l'ancre est présente");

        assert!(
            rendu.contains(
                "        // <rbs:routes>\n        \
                 .merge(crate::users::routes())\n        // </rbs:routes>"
            ),
            "insertion mal placée :\n{rendu}"
        );
    }

    #[test]
    fn l_indentation_est_celle_de_la_balise_fermante() {
        let rendu = inserer(
            ROUTEUR,
            ROUTES,
            &lignes(&[".merge(crate::users::routes())"]),
        )
        .expect("l'ancre est présente");

        let inseree = rendu
            .lines()
            .find(|ligne| ligne.contains("users::routes"))
            .expect("la ligne doit être insérée");

        assert_eq!(inseree, "        .merge(crate::users::routes())");
    }

    #[test]
    fn plusieurs_lignes_gardent_l_ordre_dans_lequel_elles_sont_donnees() {
        let rendu = inserer(
            ROUTEUR,
            ROUTES,
            &lignes(&["premiere()", "deuxieme()", "troisieme()"]),
        )
        .expect("l'ancre est présente");

        let rangs: Vec<usize> = ["premiere()", "deuxieme()", "troisieme()"]
            .iter()
            .map(|ligne| rendu.find(ligne).expect("ligne insérée"))
            .collect();

        assert!(rangs[0] < rangs[1] && rangs[1] < rangs[2], "{rendu}");
    }

    /// Le critère du lot : ce que le développeur a écrit dans l'ancre lui appartient.
    #[test]
    fn le_contenu_existant_n_est_ni_reordonne_ni_reformate() {
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

        let rendu = inserer(peuple, ROUTES, &lignes(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        let attendu = peuple.replace(
            "        // </rbs:routes>",
            "        .merge(crate::users::routes())\n        // </rbs:routes>",
        );
        assert_eq!(rendu, attendu, "le contenu existant a bougé");
    }

    #[test]
    fn une_ligne_deja_presente_n_est_pas_reinseree() {
        let une_fois = inserer(
            ROUTEUR,
            ROUTES,
            &lignes(&[".merge(crate::users::routes())"]),
        )
        .expect("l'ancre est présente");

        let deux_fois = inserer(
            &une_fois,
            ROUTES,
            &lignes(&[".merge(crate::users::routes())"]),
        )
        .expect("l'ancre est présente");

        assert_eq!(deux_fois, une_fois, "la seconde insertion a réécrit");
    }

    #[test]
    fn seules_les_lignes_absentes_sont_ajoutees() {
        let une_fois =
            inserer(ROUTEUR, ROUTES, &lignes(&["deja()"])).expect("l'ancre est présente");

        let rendu = inserer(&une_fois, ROUTES, &lignes(&["deja()", "nouvelle()"]))
            .expect("l'ancre est présente");

        assert_eq!(rendu.matches("deja()").count(), 1, "{rendu}");
        assert_eq!(rendu.matches("nouvelle()").count(), 1, "{rendu}");
    }

    #[test]
    fn une_ancre_absente_est_signalee_avec_son_fichier() {
        let erreur = inserer("fn main() {}\n", ROUTES, &lignes(&["peu importe"]))
            .expect_err("l'ancre est absente");

        assert_eq!(erreur.ancre, ROUTES);
        assert_eq!(
            erreur.to_string(),
            "ancre // <rbs:routes> introuvable dans src/router.rs"
        );
    }

    #[test]
    fn une_ancre_dont_la_fermeture_manque_est_signalee() {
        let tronque = "// <rbs:routes>\n";

        let erreur =
            inserer(tronque, ROUTES, &lignes(&["peu importe"])).expect_err("fermeture absente");

        assert_eq!(erreur.ancre, ROUTES);
    }

    /// Une occurrence citée dans du code — une chaîne, un message d'erreur — n'ouvre pas
    /// une ancre : seule une ligne qui ne porte qu'elle en est une.
    #[test]
    fn une_balise_citee_au_milieu_d_une_ligne_n_est_pas_une_ancre() {
        let cite = "let aide = \"ajoute // <rbs:routes> puis // </rbs:routes>\";\n";

        let erreur = inserer(cite, ROUTES, &lignes(&["peu importe"])).expect_err("aucune ancre");

        assert_eq!(erreur.ancre, ROUTES);
    }

    #[test]
    fn les_cinq_ancres_portent_des_noms_distincts() {
        for (rang, ancre) in ANCRES.iter().enumerate() {
            assert!(
                !ANCRES[..rang].iter().any(|autre| autre.nom == ancre.nom),
                "`{}` déclarée deux fois",
                ancre.nom
            );
        }
    }
}
