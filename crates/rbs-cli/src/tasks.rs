//! Le `Makefile` du projet engendré, rendu hors de la création.
//!
//! `rbs new` le pose avec les vingt autres fichiers du squelette, dans la passe qui les
//! rend tous. `rbs upgrade` doit pouvoir le poser seul, sur un projet engendré avant que
//! ce fichier existe : le rendu vit donc ici plutôt que dans `new::render`, qui réclame
//! pour son contexte une URL de base, un moteur et un compose dont un fichier de tâches
//! n'a que faire. Deux variables lui suffisent, et un test les tient à jour.

use std::path::Path;

use crate::lang::Lang;
use crate::template::Renderer;
use crate::templates::Source;

/// Nom du fichier, à la racine du projet.
pub(crate) const FICHIER: &str = "Makefile";

/// Ce qui peut empêcher de rendre le fichier de tâches.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// Le squelette embarqué ne porte pas de template pour ce fichier.
    #[error("le squelette ne porte pas de template pour {FICHIER}")]
    Absente,

    /// Les templates du squelette n'ont pas pu être lues.
    #[error("les templates du squelette sont illisibles : {0}")]
    Templates(#[source] std::io::Error),

    /// La template ne s'est pas rendue.
    #[error("le {FICHIER} ne se rend pas : {0}")]
    Rendu(#[source] minijinja::Error),
}

/// Rend le `Makefile` du squelette pour un projet déjà nommé.
///
/// La template embarquée, et jamais celle d'un `--template-dir` : la commande qui appelle
/// cette fonction met un projet à niveau sur la version du CLI, et un squelette de
/// substitution n'a rien à voir avec cette version-là.
pub(crate) fn render(project_name: &str, lang: Lang) -> Result<String, Error> {
    let files = Source::fresh(None).files().map_err(Error::Templates)?;

    let template = files
        .iter()
        .find(|file| file.destination == Path::new(FICHIER))
        .ok_or(Error::Absente)?;

    Renderer::new()
        .render(
            &template.source,
            minijinja::context! {
                project_name => project_name,
                lang => lang.name(),
            },
        )
        .map_err(Error::Rendu)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le rendu isolé n'a que deux variables à fournir, et le moteur est strict : une
    /// variable ajoutée à la template sans l'être ici fait tomber ce test, non la mise à
    /// niveau d'un projet chez un utilisateur.
    #[test]
    fn the_task_file_renders_from_the_project_name_and_the_language_alone() {
        let rendu = render("demo", Lang::Fr).expect("le fichier de tâches doit se rendre");

        assert!(rendu.contains("Raccourcis de demo"), "{rendu}");
        assert!(rendu.contains("\n.DEFAULT_GOAL := help\n"), "{rendu}");
    }

    /// Les noms de cibles ne suivent pas la langue, seules leurs descriptions le font.
    #[test]
    fn only_the_descriptions_of_the_rendered_task_file_follow_the_language() {
        let fr = render("demo", Lang::Fr).expect("le fichier de tâches fr doit se rendre");
        let en = render("demo", Lang::En).expect("le fichier de tâches en doit se rendre");

        assert!(fr.contains("## affiche cette liste"), "{fr}");
        assert!(en.contains("## list these shortcuts"), "{en}");

        for cible in ["dev:", "back:", "migrate:", "seed:", "openapi:"] {
            assert!(fr.contains(cible), "{cible} absent de :\n{fr}");
            assert!(en.contains(cible), "{cible} absent de :\n{en}");
        }
    }

    /// Le fichier que ce module nomme est celui où vit l'ancre des fragments : deux noms
    /// distincts ici poseraient un `Makefile` sans ancre, ou une ancre sans fichier.
    #[test]
    fn the_file_named_here_is_the_one_the_fragment_anchor_lives_in() {
        assert_eq!(FICHIER, crate::anchors::MAKE.file);
    }
}
