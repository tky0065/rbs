//! L'écran d'administration : ce qui varie d'une table à l'autre, et rien d'autre.
//!
//! Le fragment `frontend-admin` dépose un écran de données pour une entité de
//! démonstration ; `rbs generate crud` en émettra un par table. Les deux sortent de la
//! même template — `client/src/admin/vues/Patron.vue.jinja`, dans le fragment — et c'est
//! ce module qui décrit ce que cette template attend. Une seule forme, deux producteurs :
//! un second gabarit divergerait du premier, et rien ne le dirait.
//!
//! Les lignes que les deux ancres de l'administration reçoivent se construisent ici aussi,
//! pour la même raison : la route et l'entrée de rail d'un écran engendré doivent être
//! celles de l'écran de démonstration, à l'entité près.

use serde::Serialize;

use crate::lang::Lang;

/// Une colonne de la table d'un écran.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Colonne {
    /// La propriété d'une ligne que la colonne affiche.
    pub cle: String,
    /// Ce que l'opérateur lit en tête de colonne.
    pub libelle: String,
    /// Un clic sur l'en-tête trie-t-il la table dessus ?
    pub triable: bool,
}

/// Les libellés d'un écran, dans la langue du projet.
///
/// Portés par l'écran lui-même et non par un fichier de libellés partagé : un écran
/// engendré s'ajoute alors sans qu'aucun fichier commun ne reçoive de ligne, et le
/// registre d'ancres n'en porte que deux pour toute l'administration.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Textes {
    pub filtre: String,
    pub lignes: String,
    pub aucune: String,
    pub chargement: String,
    pub erreur: String,
    pub page: String,
    pub sur: String,
    pub precedent: String,
    pub suivant: String,
}

/// Un écran d'administration, tel que la template le rend.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Ecran {
    /// Le titre de l'écran, et son libellé dans le rail.
    pub titre: String,
    /// La phrase sous le titre.
    pub sous_titre: String,
    /// Le segment de chemin sous `/admin`.
    pub segment: String,
    /// Le nom de la route, unique dans le routeur du projet.
    pub route: String,
    /// Le composant, sans son suffixe : `Demonstration` pour `vues/Demonstration.vue`.
    pub composant: String,
    /// La colonne qui identifie une ligne, et qui sert de clé au rendu de la table.
    pub cle: String,
    /// Les colonnes, dans l'ordre où elles paraissent.
    pub colonnes: Vec<Colonne>,
    /// Les lignes que l'écran montre, chacune déjà écrite en objet TypeScript.
    pub lignes: Vec<String>,
    /// Les libellés de l'écran.
    pub textes: Textes,
    /// Ce que l'ancre de la table de routage reçoit.
    pub montage: String,
    /// Ce que l'ancre du rail reçoit.
    pub rail: String,
}

impl Ecran {
    /// L'écran que le fragment pose à l'installation, pour son entité de démonstration.
    ///
    /// Un projet neuf n'a aucune table à administrer : sans cet écran, l'espace
    /// d'administration s'ouvrirait vide et muet, et son propriétaire n'aurait rien vu de
    /// ce que la commande de génération lui donnera. Ses lignes vivent dans le fichier —
    /// la démonstration ne suppose donc aucune route que le projet n'expose pas.
    pub(crate) fn demonstration(lang: Lang) -> Self {
        let titre = match lang {
            Lang::Fr => "Démonstration",
            Lang::En => "Demonstration",
        };
        let sous_titre = match lang {
            Lang::Fr => {
                "Cet écran est le patron dont sortent les écrans engendrés. Ses lignes vivent \
                 dans le fichier : aucune table ne les porte encore."
            }
            Lang::En => {
                "This screen is the one every generated screen comes from. Its rows live in the \
                 file: no table carries them yet."
            }
        };

        let colonnes = vec![
            colonne("reference", lang, "Référence", "Reference", true),
            colonne("libelle", lang, "Libellé", "Label", true),
            colonne("etat", lang, "État", "State", false),
            colonne("maj", lang, "Mise à jour", "Updated", true),
        ];

        let libelles: [(&str, &str); 11] = [
            ("Bordereau", "Slip"),
            ("Relevé", "Statement"),
            ("Inventaire", "Inventory"),
            ("Journal", "Journal"),
            ("Facture", "Invoice"),
            ("Carnet", "Ledger"),
            ("Registre", "Register"),
            ("Feuille", "Sheet"),
            ("Ruban", "Ribbon"),
            ("Listing", "Listing"),
            ("Bandeau", "Banner"),
        ];
        let etats = match lang {
            Lang::Fr => ["actif", "archivé", "brouillon"],
            Lang::En => ["active", "archived", "draft"],
        };

        // Onze lignes et cinq par page : la pagination a trois pages à montrer, et la
        // dernière n'est pas pleine. Une démonstration qui tiendrait sur une page ne
        // dirait rien des deux boutons qu'elle porte.
        let lignes = libelles
            .iter()
            .enumerate()
            .map(|(rang, (fr, en))| {
                let libelle = match lang {
                    Lang::Fr => fr,
                    Lang::En => en,
                };
                format!(
                    "{{ reference: 'DEM-{:03}', libelle: '{libelle}', etat: '{}', maj: '2026-{:02}-{:02}' }}",
                    rang + 1,
                    etats[rang % etats.len()],
                    (rang % 6) + 1,
                    ((rang * 3) % 27) + 1,
                )
            })
            .collect();

        let route = "admin-demonstration".to_string();
        let segment = "demonstration".to_string();
        let composant = "Demonstration".to_string();

        Self {
            montage: montage(&segment, &route, &composant),
            rail: rail(&route, titre),
            titre: titre.to_string(),
            sous_titre: sous_titre.to_string(),
            segment,
            route,
            composant,
            cle: "reference".to_string(),
            colonnes,
            lignes,
            textes: Textes::of(lang),
        }
    }
}

impl Textes {
    /// Les libellés d'un écran dans `lang`.
    fn of(lang: Lang) -> Self {
        let dit = |fr: &str, en: &str| {
            match lang {
                Lang::Fr => fr,
                Lang::En => en,
            }
            .to_string()
        };

        Self {
            filtre: dit("Filtrer", "Filter"),
            lignes: dit("lignes", "rows"),
            aucune: dit("Aucune ligne ne correspond", "No row matches"),
            chargement: dit("Chargement…", "Loading…"),
            erreur: dit(
                "La source est injoignable.",
                "The source cannot be reached.",
            ),
            page: dit("Page", "Page"),
            sur: dit("sur", "of"),
            precedent: dit("Précédent", "Previous"),
            suivant: dit("Suivant", "Next"),
        }
    }
}

/// Une colonne, son libellé pris dans la langue du projet.
fn colonne(cle: &str, lang: Lang, fr: &str, en: &str, triable: bool) -> Colonne {
    Colonne {
        cle: cle.to_string(),
        libelle: match lang {
            Lang::Fr => fr,
            Lang::En => en,
        }
        .to_string(),
        triable,
    }
}

/// La route d'un écran, telle qu'elle s'écrit dans la table de routage.
///
/// Le composant est importé paresseusement : la table de routage est lue dès le premier
/// écran du socle, et un écran d'administration importé statiquement partirait dans le
/// morceau d'entrée, que télécharge le visiteur de l'accueil.
fn montage(segment: &str, route: &str, composant: &str) -> String {
    format!(
        "{{\n  path: '{segment}',\n  name: '{route}',\n  component: () => \
         import('./vues/{composant}.vue'),\n}},"
    )
}

/// L'entrée de rail d'un écran, telle qu'elle s'écrit dans la liste des entrées.
fn rail(route: &str, libelle: &str) -> String {
    format!("{{ route: '{route}', libelle: '{libelle}' }},")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use minijinja::context;

    use super::*;
    use crate::template::Renderer;

    /// Où vit l'écran patron, dans le fragment qui le dépose.
    ///
    /// Le fragment le rend pour son entité de démonstration ; `rbs generate crud` rendra
    /// le même fichier pour chaque table. Il n'a donc pas de copie à prendre sous
    /// `templates/feature/`, d'où sortent les autres templates de la génération : deux
    /// exemplaires de cette template-ci divergeraient, et la divergence est précisément
    /// ce que la conception voulait éviter.
    const CHEMIN: &str = "templates/features/frontend-admin/client/src/admin/vues/Patron.vue.jinja";

    const PATRON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/templates/features/frontend-admin/client/src/admin/vues/Patron.vue.jinja"
    ));

    /// La template embarquée est bien le fichier que le fragment dépose, et non une copie.
    #[test]
    fn the_embedded_pattern_screen_is_the_one_the_fragment_lays_down() {
        let sur_disque =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(CHEMIN))
                .expect("l'écran patron doit se lire dans le fragment");

        assert_eq!(sur_disque, PATRON);

        let manifeste = std::fs::read_to_string(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/frontend-admin/feature.toml"
        )))
        .expect("le manifeste du shell doit se lire");
        assert!(
            manifeste.contains("client/src/admin/vues/Patron.vue.jinja"),
            "le fragment ne dépose pas l'écran patron :\n{manifeste}"
        );
    }

    /// Le patron n'a rien de l'entité qu'il montre : c'est ce qui lui permet d'en servir
    /// deux.
    ///
    /// Sans ce contrôle, un écran de démonstration écrit en dur passerait toutes les
    /// autres vérifications — il se rend, il se compile, il s'affiche — et la commande de
    /// génération n'aurait plus qu'à s'en faire une copie.
    #[test]
    fn the_same_template_renders_a_screen_for_another_entity() {
        let autre = Ecran {
            titre: "Commandes".to_string(),
            sous_titre: "Les commandes du service.".to_string(),
            segment: "commandes".to_string(),
            route: "admin-commandes".to_string(),
            composant: "Commandes".to_string(),
            cle: "numero".to_string(),
            colonnes: vec![colonne("numero", Lang::Fr, "Numéro", "Number", true)],
            lignes: vec!["{ numero: 'C-1' }".to_string()],
            textes: Textes::of(Lang::Fr),
            montage: montage("commandes", "admin-commandes", "Commandes"),
            rail: rail("admin-commandes", "Commandes"),
        };

        let rendu = Renderer::new()
            .render(PATRON, context! { ecran => autre })
            .expect("l'écran patron doit se rendre pour une autre entité");

        assert!(rendu.contains("  numero: string\n"), "{rendu}");
        assert!(
            rendu.contains("{ cle: 'numero', libelle: 'Numéro', triable: true },"),
            "{rendu}"
        );
        assert!(rendu.contains("ligne.numero"), "{rendu}");
        assert!(
            !rendu.contains("DEM-001"),
            "l'entité de démonstration est écrite en dur dans le patron :\n{rendu}"
        );
    }

    /// Les deux ancres reçoivent de quoi monter l'écran, et rien de plus.
    #[test]
    fn the_demonstration_screen_mounts_itself_by_route_and_rail() {
        let ecran = Ecran::demonstration(Lang::Fr);

        assert_eq!(
            ecran.montage,
            "{\n  path: 'demonstration',\n  name: 'admin-demonstration',\n  component: () => \
             import('./vues/Demonstration.vue'),\n},"
        );
        assert_eq!(
            ecran.rail,
            "{ route: 'admin-demonstration', libelle: 'Démonstration' },"
        );
    }

    /// Un libellé portant une apostrophe fermerait la chaîne TypeScript qui le porte.
    ///
    /// La template écrit chaque libellé entre apostrophes, comme tout le client : une
    /// seule apostrophe dans « n'a pas » couperait le fichier en deux, et seule la
    /// vérification des types — en ligne, et ignorée par défaut — le dirait.
    #[test]
    fn no_label_closes_the_typescript_string_that_carries_it() {
        for lang in [Lang::Fr, Lang::En] {
            let ecran = Ecran::demonstration(lang);
            let textes = &ecran.textes;

            let libelles: Vec<&str> = [
                ecran.titre.as_str(),
                ecran.sous_titre.as_str(),
                textes.filtre.as_str(),
                textes.lignes.as_str(),
                textes.aucune.as_str(),
                textes.chargement.as_str(),
                textes.erreur.as_str(),
                textes.page.as_str(),
                textes.sur.as_str(),
                textes.precedent.as_str(),
                textes.suivant.as_str(),
            ]
            .into_iter()
            .chain(
                ecran
                    .colonnes
                    .iter()
                    .map(|colonne| colonne.libelle.as_str()),
            )
            .chain(ecran.lignes.iter().flat_map(|ligne| {
                // Les lignes portent leurs propres apostrophes, qui délimitent les
                // valeurs : seules celles des valeurs sont en cause.
                ligne.split('\'').skip(1).step_by(2)
            }))
            .collect();

            for libelle in libelles {
                assert!(
                    !libelle.contains('\''),
                    "`{libelle}` porte une apostrophe, qui fermerait sa chaîne"
                );
            }
        }
    }
}
