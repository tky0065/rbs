//! Mise en forme d'un plan, avant qu'il ne soit appliqué.
//!
//! Un fichier par ligne, et non une action par ligne : deux insertions de la même ligne
//! sont deux actions mais un seul changement, et c'est le fichier qui porte le statut
//! agrégé qui dit la vérité.

use super::{Fichier, Plan, Statut};
use crate::ui;

/// Rend le plan : la racine du projet en tête, puis un fichier par ligne.
///
/// La puce et le libellé se suffisent à eux-mêmes : la couleur ne porte jamais seule une
/// information, pour que la sortie reste lisible dans un `less`, un log ou une CI.
pub(crate) fn plan(plan: &Plan) -> String {
    let entete = format!("plan pour {}", plan.racine().display());
    let fichiers = plan.fichiers();

    if fichiers.is_empty() {
        return format!("{entete}\n\n  rien à faire");
    }

    let largeur = fichiers
        .iter()
        .map(|fichier| fichier.chemin.chars().count())
        .max()
        .unwrap_or(0);

    let lignes: Vec<String> = fichiers
        .iter()
        .map(|fichier| ligne(fichier, largeur))
        .collect();

    format!("{entete}\n\n{}\n\n  {}", lignes.join("\n"), pied(fichiers))
}

/// Ce qu'une ligne dit d'un fichier : sa puce, son chemin, ce qu'il adviendra de lui.
fn ligne(fichier: &Fichier, largeur: usize) -> String {
    let chemin = format!("{:largeur$}", fichier.chemin);

    let (puce, libelle) = match (fichier.statut, &fichier.avant) {
        (Statut::AFaire, None) => (ui::vert("+"), ui::vert("créé")),
        (Statut::AFaire, Some(_)) => (ui::vert("~"), ui::vert("modifié")),
        (Statut::DejaFait, _) => (ui::attenue("·"), ui::attenue("inchangé")),
        (Statut::Conflit, _) => (ui::rouge("!"), ui::rouge("conflit — relancer avec --force")),
    };

    format!("  {puce} {chemin}   {libelle}")
}

/// Le compte, par ce qui adviendra des fichiers.
///
/// Les conflits se comptent à part : sans `--force`, ils ne seront pas écrits, et les
/// ranger avec le reste ferait annoncer une écriture qui n'aura pas lieu.
fn pied(fichiers: &[Fichier]) -> String {
    let compter = |statut: Statut| fichiers.iter().filter(|f| f.statut == statut).count();

    let (a_ecrire, inchanges, conflits) = (
        compter(Statut::AFaire),
        compter(Statut::DejaFait),
        compter(Statut::Conflit),
    );

    let mut segments = Vec::new();
    if a_ecrire > 0 {
        let pluriel = if a_ecrire > 1 { "s" } else { "" };
        segments.push(format!("{a_ecrire} fichier{pluriel} à écrire"));
    }
    if inchanges > 0 {
        let pluriel = if inchanges > 1 { "s" } else { "" };
        segments.push(format!("{inchanges} inchangé{pluriel}"));
    }
    if conflits > 0 {
        segments.push(format!("{conflits} en conflit"));
    }

    segments.join(", ")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::{Fichier, Statut};
    use super::*;

    fn fichier(chemin: &str, avant: Option<&str>, statut: Statut) -> Fichier {
        Fichier {
            chemin: chemin.to_string(),
            avant: avant.map(str::to_string),
            apres: "peu importe".to_string(),
            statut,
        }
    }

    fn plan_de(fichiers: Vec<Fichier>) -> Plan {
        Plan {
            racine: PathBuf::from("/projets/demo-api"),
            actions: Vec::new(),
            fichiers,
        }
    }

    /// Colonne d'un libellé, comptée en caractères : `find` rend des octets, et les puces
    /// n'en occupent pas le même nombre.
    fn colonne(ligne: &str, libelle: &str) -> usize {
        let octets = ligne.find(libelle).expect("le libellé est présent");
        ligne[..octets].chars().count()
    }

    fn ligne_de<'a>(rendu: &'a str, chemin: &str) -> &'a str {
        rendu
            .lines()
            .find(|ligne| ligne.contains(chemin))
            .unwrap_or_else(|| panic!("aucune ligne pour `{chemin}` dans :\n{rendu}"))
    }

    #[test]
    fn l_en_tete_porte_la_racine_du_projet_une_seule_fois() {
        let rendu = plan(&plan_de(vec![fichier("Dockerfile", None, Statut::AFaire)]));

        assert!(
            rendu.starts_with("plan pour /projets/demo-api\n"),
            "{rendu}"
        );
        assert_eq!(rendu.matches("/projets/demo-api").count(), 1, "{rendu}");
    }

    #[test]
    fn un_fichier_absent_est_annonce_cree_et_un_fichier_present_modifie() {
        let rendu = plan(&plan_de(vec![
            fichier("Dockerfile", None, Statut::AFaire),
            fichier("Cargo.toml", Some("[package]\n"), Statut::AFaire),
        ]));

        assert!(ligne_de(&rendu, "Dockerfile").contains("créé"), "{rendu}");
        assert!(
            ligne_de(&rendu, "Cargo.toml").contains("modifié"),
            "{rendu}"
        );
    }

    #[test]
    fn un_fichier_deja_conforme_est_annonce_inchange() {
        let rendu = plan(&plan_de(vec![fichier(
            "src/router.rs",
            Some("déjà monté"),
            Statut::DejaFait,
        )]));

        assert!(
            ligne_de(&rendu, "src/router.rs").contains("inchangé"),
            "{rendu}"
        );
    }

    #[test]
    fn un_conflit_porte_son_remede_sur_sa_ligne() {
        let rendu = plan(&plan_de(vec![fichier(
            "src/main.rs",
            Some("écrit à la main"),
            Statut::Conflit,
        )]));

        let ligne = ligne_de(&rendu, "src/main.rs");
        assert!(ligne.contains("conflit"), "{ligne}");
        assert!(ligne.contains("--force"), "{ligne}");
    }

    #[test]
    fn les_libelles_sont_alignes_sur_le_plus_long_chemin() {
        let rendu = plan(&plan_de(vec![
            fichier("Dockerfile", None, Statut::AFaire),
            fichier("docker-compose.yml", None, Statut::AFaire),
            fichier("src/router.rs", Some("x"), Statut::DejaFait),
        ]));

        assert_eq!(
            colonne(ligne_de(&rendu, "Dockerfile"), "créé"),
            colonne(ligne_de(&rendu, "docker-compose.yml"), "créé"),
            "{rendu}"
        );
        assert_eq!(
            colonne(ligne_de(&rendu, "Dockerfile"), "créé"),
            colonne(ligne_de(&rendu, "src/router.rs"), "inchangé"),
            "{rendu}"
        );
    }

    #[test]
    fn le_pied_compte_les_fichiers_a_ecrire_et_les_inchanges() {
        let un = plan(&plan_de(vec![fichier("Dockerfile", None, Statut::AFaire)]));
        assert!(un.ends_with("1 fichier à écrire"), "{un}");

        let plusieurs = plan(&plan_de(vec![
            fichier("Dockerfile", None, Statut::AFaire),
            fichier("Cargo.toml", Some("x"), Statut::AFaire),
            fichier("src/router.rs", Some("x"), Statut::DejaFait),
        ]));
        assert!(
            plusieurs.ends_with("2 fichiers à écrire, 1 inchangé"),
            "{plusieurs}"
        );
    }

    #[test]
    fn un_conflit_ne_se_compte_pas_parmi_les_fichiers_a_ecrire() {
        let rendu = plan(&plan_de(vec![
            fichier("Dockerfile", None, Statut::AFaire),
            fichier("src/main.rs", Some("x"), Statut::Conflit),
        ]));

        assert!(
            rendu.ends_with("1 fichier à écrire, 1 en conflit"),
            "{rendu}"
        );
    }

    #[test]
    fn un_plan_vide_ne_ment_pas() {
        let rendu = plan(&plan_de(Vec::new()));

        assert!(rendu.contains("rien à faire"), "{rendu}");
        assert!(!rendu.contains("à écrire"), "{rendu}");
    }

    #[test]
    fn chaque_etat_se_distingue_sans_la_couleur() {
        let rendu = plan(&plan_de(vec![
            fichier("cree.txt", None, Statut::AFaire),
            fichier("modifie.txt", Some("x"), Statut::AFaire),
            fichier("inchange.txt", Some("x"), Statut::DejaFait),
            fichier("conflit.txt", Some("x"), Statut::Conflit),
        ]));

        assert!(
            !rendu.contains('\u{1b}'),
            "aucun code ANSI hors TTY :\n{rendu}"
        );

        let puces: Vec<char> = ["cree.txt", "modifie.txt", "inchange.txt", "conflit.txt"]
            .iter()
            .map(|chemin| {
                ligne_de(&rendu, chemin)
                    .trim_start()
                    .chars()
                    .next()
                    .expect("la ligne porte une puce")
            })
            .collect();

        let mut distinctes = puces.clone();
        distinctes.sort_unstable();
        distinctes.dedup();
        assert_eq!(distinctes.len(), puces.len(), "puces : {puces:?}");
    }
}
