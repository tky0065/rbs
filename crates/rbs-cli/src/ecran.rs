//! L'écran d'administration : ce qui varie d'une table à l'autre, et rien d'autre.
//!
//! Le fragment `frontend-admin` dépose un écran de données pour une entité de
//! démonstration ; `rbs generate crud` en émet un par table. Les deux sortent de la même
//! template — `client/src/admin/vues/Patron.vue.jinja`, dans le fragment — et c'est ce
//! module qui décrit ce que cette template attend. Une seule forme, deux producteurs :
//! un second gabarit divergerait du premier, et rien ne le dirait.
//!
//! Trois listes décrivent une table, chacune pour un usage :
//!
//! - [`Ecran::proprietes`] est la forme d'une ligne, une propriété par clé — ce que
//!   `interface Ligne` déclare, et ce que le détail montre ;
//! - [`Ecran::colonnes`] est le sous-ensemble que la table affiche, et son tri ;
//! - [`Ecran::champs`] est le sous-ensemble éditable, un contrôle par champ.
//!
//! Ce qui sépare les deux producteurs tient dans [`Ecran::api`] : `None` est l'écran de
//! démonstration, dont les lignes vivent dans le fichier ; `Some` porte les cinq méthodes
//! du client typé qu'un écran engendré interroge. Tout le reste de l'écran — filtre, tri,
//! pagination, formulaire, détail, rendu — ne connaît que `Ligne`, `Requete` et
//! `Formulaire`, et ne change pas d'une table à l'autre.
//!
//! Les lignes que les deux ancres de l'administration reçoivent se construisent ici aussi,
//! pour la même raison : la route et l'entrée de rail d'un écran engendré doivent être
//! celles de l'écran de démonstration, à l'entité près.

use serde::Serialize;

use crate::generate::feature::Feature;
use crate::generate::fields::{Field, FieldType};
use crate::lang::Lang;

/// Combien de lignes une page d'écran engendré porte.
///
/// Cinq sur la démonstration, dont les onze lignes doivent montrer trois pages ; vingt sur
/// une table réelle, où une page de cinq ferait paginer ce qui tiendrait à l'écran.
const TAILLE_ENGENDREE: u32 = 20;

/// Une propriété d'une ligne : ce que `interface Ligne` déclare, et ce que le détail montre.
///
/// Distincte de [`Colonne`] : une ligne porte son identifiant et ses horodatages, que la
/// table n'affiche pas tous, et le détail les montre tous.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Propriete {
    /// La clé de la propriété, telle que le corps de la ressource la nomme.
    pub cle: String,
    /// Ce que l'opérateur lit devant la valeur, dans le détail.
    pub libelle: String,
    /// Le type TypeScript de la valeur, sa nullité exceptée : `string`, `number`, ou
    /// l'union des valeurs d'une énumération.
    pub type_base: String,
    /// La colonne accepte l'absence de valeur : le type gagne alors `| null`.
    pub optionnel: bool,
}

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

/// Un champ du formulaire : ce qui saisit une valeur, et ce que cette valeur envoie.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Champ {
    /// La propriété que le champ écrit.
    pub cle: String,
    /// Ce que l'opérateur lit au-dessus du contrôle.
    pub libelle: String,
    /// Le contrôle qui saisit la valeur : `texte`, `nombre`, `decimal`, `booleen`,
    /// `date`, `instant` ou `liste`.
    pub controle: String,
    /// Les valeurs admises, pour le seul contrôle `liste`.
    pub valeurs: Vec<String>,
    /// Le type TypeScript de la valeur envoyée, sa nullité exceptée.
    pub type_base: String,
    /// Le champ peut rester vide, et envoie alors `null`.
    pub optionnel: bool,
}

/// Le contrat qu'un écran engendré interroge, méthode par méthode.
///
/// Les noms viennent de [`crate::client::ts::nom_de_methode`], celle-là même qui écrira le
/// client : un écran qui appellerait `api.articles_list` là où le client publie
/// `articlesList` ne se verrait qu'à la vérification des types, en ligne et ignorée par
/// défaut.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Api {
    /// Le type du corps d'une ressource, tel que le client typé le nomme.
    pub reponse: String,
    /// La méthode qui rend une page filtrée.
    pub liste: String,
    /// La méthode qui rend une ressource par son identifiant.
    pub lire: String,
    /// La méthode qui crée une ressource.
    pub creer: String,
    /// La méthode qui remplace une ressource.
    pub modifier: String,
    /// La méthode qui retire une ressource.
    pub supprimer: String,
    /// La colonne sur laquelle le filtre porte, vide quand la table n'en a aucune de
    /// textuelle — [`Ecran::filtrable`] le dit alors, et l'écran n'affiche pas de filtre.
    pub recherche: String,
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
    pub actions: String,
    pub creer: String,
    pub modifier: String,
    pub supprimer: String,
    pub detail: String,
    pub enregistrer: String,
    pub enregistrement: String,
    pub annuler: String,
    pub suppression_titre: String,
    pub suppression_detail: String,
    pub enregistree: String,
    pub supprimee: String,
    pub refus_enregistrement: String,
    pub refus_suppression: String,
    pub introuvable: String,
    pub vide: String,
    pub oui: String,
    pub non: String,
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
    /// La propriété qui identifie une ligne, et qui sert de clé au rendu de la table.
    pub cle: String,
    /// Combien de lignes une page porte.
    pub taille: u32,
    /// L'écran porte un champ de filtre : la source sait chercher un texte.
    pub filtrable: bool,
    /// Les composants d'interface que l'écran importe, au-delà des quatre qu'il importe
    /// toujours.
    ///
    /// Calculés ici et non devinés dans la template : `noUnusedLocals` est posé sur le
    /// projet engendré, et un composant importé sans être employé — la case à cocher sur
    /// une table qui n'a pas de booléen — arrêterait la vérification des types.
    pub composants: Vec<String>,
    /// La forme d'une ligne, une propriété par clé.
    pub proprietes: Vec<Propriete>,
    /// Les colonnes, dans l'ordre où elles paraissent.
    pub colonnes: Vec<Colonne>,
    /// Les champs du formulaire, dans l'ordre où ils se saisissent.
    pub champs: Vec<Champ>,
    /// Les lignes que l'écran montre, chacune déjà écrite en objet TypeScript — vides
    /// pour un écran engendré, qui tient les siennes de son API.
    pub lignes: Vec<String>,
    /// Le contrat que la source interroge, absent sur l'écran de démonstration.
    pub api: Option<Api>,
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

        let etats = match lang {
            Lang::Fr => ["actif", "archivé", "brouillon"],
            Lang::En => ["active", "archived", "draft"],
        };

        // Les quatre propriétés de la démonstration sont aussi ses quatre colonnes et ses
        // quatre champs : une entité sans identifiant serveur ni horodatage automatique,
        // ce qui est précisément ce qui la rend lisible dans le fichier.
        let declarees = [
            ("reference", "Référence", "Reference", "texte", true),
            ("libelle", "Libellé", "Label", "texte", true),
            ("etat", "État", "State", "liste", false),
            ("maj", "Mise à jour", "Updated", "date", true),
        ];

        let proprietes = declarees
            .iter()
            .map(|(cle, fr, en, controle, _)| Propriete {
                cle: (*cle).to_string(),
                libelle: dans(lang, fr, en),
                type_base: type_de_controle(controle, &etats),
                optionnel: false,
            })
            .collect();

        let colonnes = declarees
            .iter()
            .map(|(cle, fr, en, _, triable)| Colonne {
                cle: (*cle).to_string(),
                libelle: dans(lang, fr, en),
                triable: *triable,
            })
            .collect();

        let champs: Vec<Champ> = declarees
            .iter()
            .map(|(cle, fr, en, controle, _)| Champ {
                cle: (*cle).to_string(),
                libelle: dans(lang, fr, en),
                controle: (*controle).to_string(),
                valeurs: if *controle == "liste" {
                    etats.iter().map(|etat| (*etat).to_string()).collect()
                } else {
                    Vec::new()
                },
                type_base: type_de_controle(controle, &etats),
                optionnel: false,
            })
            .collect();

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
            taille: 5,
            filtrable: true,
            composants: composants(&champs, true),
            proprietes,
            colonnes,
            champs,
            lignes,
            api: None,
            textes: Textes::of(lang),
        }
    }

    /// L'écran qu'engendre `rbs generate crud` pour la table de `feature`.
    ///
    /// Le même patron que la démonstration, l'API à la place du tableau écrit dans le
    /// fichier. La table montre chaque champ déclaré puis la date de mise à jour ; le
    /// détail y ajoute l'identifiant et la date de création, que la table tairait pour ne
    /// pas noyer ce que l'opérateur est venu lire.
    pub(crate) fn pour(feature: &Feature, lang: Lang) -> Self {
        let module = feature.module().to_string();
        let entity = feature.entity();

        let titre = humanise(&module);
        let sous_titre = match lang {
            Lang::Fr => format!(
                "Les lignes de la table {module}, lues et écrites par les routes que ce projet \
                 expose."
            ),
            Lang::En => format!(
                "The rows of the {module} table, read and written through the routes this project \
                 exposes."
            ),
        };

        let mut proprietes = vec![Propriete {
            cle: "id".to_string(),
            libelle: dans(lang, "Identifiant", "Identifier"),
            type_base: "string".to_string(),
            optionnel: false,
        }];
        proprietes.extend(feature.fields.iter().map(propriete));
        for (cle, fr, en) in [
            ("created_at", "Créé le", "Created"),
            ("updated_at", "Mise à jour", "Updated"),
        ] {
            proprietes.push(Propriete {
                cle: cle.to_string(),
                libelle: dans(lang, fr, en),
                type_base: "string".to_string(),
                optionnel: false,
            });
        }

        let mut colonnes: Vec<Colonne> = feature
            .fields
            .iter()
            .map(|field| Colonne {
                cle: field.column_name(),
                libelle: humanise(&field.column_name()),
                triable: true,
            })
            .collect();
        colonnes.push(Colonne {
            cle: "updated_at".to_string(),
            libelle: dans(lang, "Mise à jour", "Updated"),
            triable: true,
        });

        let champs: Vec<Champ> = feature.fields.iter().map(champ).collect();

        // La première colonne textuelle, et elle seule : le filtre du projet conjugue ses
        // conditions par ET, et chercher un motif dans plusieurs colonnes demanderait un
        // OU que le contrat n'expose pas. Une colonne énumérée ou un identifiant sont
        // écartés — ils n'acceptent pas `contains`.
        let recherche = feature
            .fields
            .iter()
            .find(|field| {
                field.enum_variants().is_empty()
                    && field.reference().is_none()
                    && matches!(field.column_type(), FieldType::String | FieldType::Text)
            })
            .map(Field::column_name)
            .unwrap_or_default();

        let methode =
            |suffixe: &str| crate::client::ts::nom_de_methode(&format!("{module}_{suffixe}"));
        let api = Api {
            reponse: crate::client::ts::identifiant(&format!("{entity}Response")),
            liste: methode("filter"),
            lire: methode("find"),
            creer: methode("create"),
            modifier: methode("update"),
            supprimer: methode("delete"),
            recherche: recherche.clone(),
        };

        let segment = module.replace('_', "-");
        let route = format!("admin-{segment}");
        let composant = feature.iden();

        Self {
            montage: montage(&segment, &route, &composant),
            rail: rail(&route, &titre),
            titre,
            sous_titre,
            segment,
            route,
            composant,
            cle: "id".to_string(),
            taille: TAILLE_ENGENDREE,
            filtrable: !recherche.is_empty(),
            composants: composants(&champs, !recherche.is_empty()),
            proprietes,
            colonnes,
            champs,
            lignes: Vec::new(),
            api: Some(api),
            textes: Textes::of(lang),
        }
    }

    /// Le fichier de l'écran, relatif à la racine du projet.
    pub(crate) fn fichier(&self) -> String {
        format!("frontend/src/admin/vues/{}.vue", self.composant)
    }
}

impl Textes {
    /// Les libellés d'un écran dans `lang`.
    ///
    /// Aucun ne porte d'apostrophe : la template les écrit entre apostrophes, comme tout
    /// le client, et un test le tient.
    fn of(lang: Lang) -> Self {
        let dit = |fr: &str, en: &str| dans(lang, fr, en);

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
            actions: dit("Actions", "Actions"),
            creer: dit("Nouvelle ligne", "New row"),
            modifier: dit("Modifier", "Edit"),
            supprimer: dit("Supprimer", "Delete"),
            detail: dit("Détail", "Details"),
            enregistrer: dit("Enregistrer", "Save"),
            enregistrement: dit("Enregistrement…", "Saving…"),
            annuler: dit("Annuler", "Cancel"),
            suppression_titre: dit("Supprimer cette ligne", "Delete this row"),
            suppression_detail: dit(
                "La ligne part de la table, et le geste ne se reprend pas.",
                "The row leaves the table, and the action cannot be undone.",
            ),
            enregistree: dit("Ligne enregistrée.", "Row saved."),
            supprimee: dit("Ligne supprimée.", "Row deleted."),
            refus_enregistrement: dit("Enregistrement refusé.", "The row was not saved."),
            refus_suppression: dit("Suppression refusée.", "The row was not deleted."),
            introuvable: dit("Ligne introuvable.", "Row not found."),
            vide: dit("—", "—"),
            oui: dit("Oui", "Yes"),
            non: dit("Non", "No"),
        }
    }
}

/// Les composants d'interface qu'un écran importe, au-delà de ceux qu'il importe toujours.
///
/// Le bouton, la table, la boîte de dialogue et le panneau sont de tous les écrans — la
/// création, le détail et la suppression en dépendent quelle que soit la table. Les
/// quatre autres suivent les contrôles du formulaire, et le champ suit aussi le filtre.
fn composants(champs: &[Champ], filtrable: bool) -> Vec<String> {
    let porte = |controle: &str| champs.iter().any(|champ| champ.controle == controle);

    let mut retenus = Vec::new();
    if filtrable
        || porte("texte")
        || porte("nombre")
        || porte("decimal")
        || porte("date")
        || porte("instant")
    {
        retenus.push("input".to_string());
    }
    if !champs.is_empty() {
        retenus.push("label".to_string());
    }
    if porte("booleen") {
        retenus.push("checkbox".to_string());
    }
    if porte("liste") {
        retenus.push("select".to_string());
    }

    retenus
}

/// Le texte de `lang`, entre les deux que le projet connaît.
fn dans(lang: Lang, fr: &str, en: &str) -> String {
    match lang {
        Lang::Fr => fr,
        Lang::En => en,
    }
    .to_string()
}

/// Ce que l'opérateur lit d'un nom de colonne : `published_at` devient `Published at`.
///
/// Non traduit, et c'est délibéré : le nom vient de `--fields`, donc du développeur, et
/// une traduction inventerait un mot qu'il n'a pas écrit.
fn humanise(nom: &str) -> String {
    let espace = nom.replace('_', " ");
    let mut chars = espace.chars();

    match chars.next() {
        Some(premier) => premier.to_uppercase().collect::<String>() + chars.as_str(),
        None => espace,
    }
}

/// Le contrôle qui saisit un champ, et le type TypeScript de ce qu'il envoie.
///
/// Une énumération se saisit dans une liste, un booléen dans une case, une date et un
/// instant dans les contrôles natifs qui les rendent — leur format de sortie est
/// exactement celui que le contrat attend, à l'instant près, dont le fuseau se repose à
/// l'envoi. Un texte long passe par le même champ qu'une chaîne courte : les quatorze
/// composants figés n'en portent pas d'autre, et en ajouter un ferait dépendre l'écran
/// engendré de ce que le socle n'a pas.
fn controle(field: &Field) -> (&'static str, String) {
    if !field.enum_variants().is_empty() {
        return ("liste", union(field.enum_variants()));
    }
    if field.reference().is_some() {
        return ("texte", "string".to_string());
    }

    match field.column_type() {
        FieldType::String | FieldType::Text | FieldType::Uuid => ("texte", "string".to_string()),
        FieldType::Int | FieldType::Float => ("nombre", "number".to_string()),
        // Le contrat porte le décimal en chaîne : un nombre JavaScript y perdrait les
        // centimes, ce que la représentation choisie par le noyau évite précisément.
        FieldType::Decimal => ("decimal", "string".to_string()),
        FieldType::Bool => ("booleen", "boolean".to_string()),
        FieldType::Date => ("date", "string".to_string()),
        FieldType::Datetime => ("instant", "string".to_string()),
    }
}

/// Le type TypeScript d'un contrôle de la démonstration, dont les champs n'ont pas de
/// `Field` derrière eux.
fn type_de_controle(controle: &str, valeurs: &[&str]) -> String {
    match controle {
        "liste" => union(valeurs),
        "nombre" => "number".to_string(),
        "booleen" => "boolean".to_string(),
        _ => "string".to_string(),
    }
}

/// L'union TypeScript des valeurs d'une énumération : `'draft' | 'published'`.
fn union<S: AsRef<str>>(valeurs: &[S]) -> String {
    valeurs
        .iter()
        .map(|valeur| format!("'{}'", valeur.as_ref()))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// La propriété qu'un champ déclaré ajoute à une ligne.
fn propriete(field: &Field) -> Propriete {
    let (_, type_base) = controle(field);

    Propriete {
        cle: field.column_name(),
        libelle: humanise(&field.column_name()),
        type_base,
        optionnel: field.optional,
    }
}

/// Le champ de formulaire qu'un champ déclaré reçoit.
fn champ(field: &Field) -> Champ {
    let (controle, type_base) = controle(field);

    Champ {
        cle: field.column_name(),
        libelle: humanise(&field.column_name()),
        controle: controle.to_string(),
        valeurs: field.enum_variants().to_vec(),
        type_base,
        optionnel: field.optional,
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
    use crate::generate::fields;
    use crate::template::Renderer;

    /// Où vit l'écran patron, dans le fragment qui le dépose.
    ///
    /// Le fragment le rend pour son entité de démonstration ; `rbs generate crud` rend le
    /// même fichier pour chaque table. Il n'a donc pas de copie à prendre sous
    /// `templates/feature/`, d'où sortent les autres templates de la génération : deux
    /// exemplaires de cette template-ci divergeraient, et la divergence est précisément
    /// ce que la conception voulait éviter.
    const CHEMIN: &str = "templates/features/frontend-admin/client/src/admin/vues/Patron.vue.jinja";

    const PATRON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/templates/features/frontend-admin/client/src/admin/vues/Patron.vue.jinja"
    ));

    /// Une feature à plusieurs types de champs, pour exercer chaque contrôle.
    fn feature() -> Feature {
        let champs = fields::parse(
            "title:string,resume:text,vues:int,prix:decimal,publie:bool,paru:date,vu:datetime,\
             statut:enum(draft,published),note:string:optional",
        )
        .expect("les champs du test doivent s'analyser");

        Feature::fresh("articles", champs)
    }

    /// Rend l'écran patron pour `ecran`, comme le fragment et la commande le font.
    fn rendu(ecran: &Ecran) -> String {
        Renderer::new()
            .render(PATRON, context! { ecran => ecran })
            .unwrap_or_else(|faute| panic!("l'écran patron doit se rendre : {faute}"))
    }

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

        // Et la commande de génération n'en a pas pris de copie sous `templates/feature/`,
        // d'où sortent ses autres templates : deux exemplaires divergeraient, et la
        // divergence est le défaut même que la conception voulait éviter.
        let copies: Vec<_> = std::fs::read_dir(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/feature"
        )))
        .expect("le répertoire des templates de feature se lit")
        .filter_map(Result::ok)
        .map(|entree| entree.file_name().to_string_lossy().into_owned())
        .filter(|nom| nom.contains(".vue"))
        .collect();
        assert!(
            copies.is_empty(),
            "une seconde template d'écran vit sous templates/feature/ : {copies:?}"
        );

        // La commande lit bien celle du fragment, et non une chaîne à elle.
        let commande = std::fs::read_to_string(Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/generate/command.rs"
        )))
        .expect("la commande se lit");
        assert!(commande.contains(CHEMIN), "{commande}");
    }

    /// Le patron n'a rien de l'entité qu'il montre : c'est ce qui lui permet d'en servir
    /// deux.
    ///
    /// Sans ce contrôle, un écran de démonstration écrit en dur passerait toutes les
    /// autres vérifications — il se rend, il se compile, il s'affiche — et la commande de
    /// génération n'aurait plus qu'à s'en faire une copie.
    #[test]
    fn the_same_template_renders_a_screen_for_another_entity() {
        let rendu = rendu(&Ecran::pour(&feature(), Lang::Fr));

        assert!(rendu.contains("  title: string\n"), "{rendu}");
        assert!(
            rendu.contains("{ cle: 'title', libelle: 'Title', triable: true },"),
            "{rendu}"
        );
        assert!(
            !rendu.contains("DEM-001"),
            "l'entité de démonstration est écrite en dur dans le patron :\n{rendu}"
        );
    }

    /// Les deux régimes de source sortent du même fichier : la démonstration lit son
    /// tableau, l'écran engendré appelle le client typé.
    #[test]
    fn the_two_producers_differ_only_by_their_source() {
        let demonstration = rendu(&Ecran::demonstration(Lang::Fr));
        let engendre = rendu(&Ecran::pour(&feature(), Lang::Fr));

        assert!(
            demonstration.contains("const SOURCE: Ligne[] = [") && !demonstration.contains("api."),
            "la démonstration doit tenir ses lignes dans le fichier :\n{demonstration}"
        );
        for appel in [
            "api.articlesFilter(",
            "api.articlesFind(",
            "api.articlesCreate(",
            "api.articlesUpdate(",
            "api.articlesDelete(",
        ] {
            assert!(
                engendre.contains(appel),
                "`{appel}` manque à l'écran engendré :\n{engendre}"
            );
        }
        assert!(
            !engendre.contains("const SOURCE"),
            "l'écran engendré porte un tableau écrit dans le fichier :\n{engendre}"
        );

        // Et la part commune est bien commune : ce qui suit la source ne change pas.
        for commun in [
            "async function charger(): Promise<void> {",
            "function trier(cle: keyof Ligne): void {",
            "const pages = computed(() => Math.max(1, Math.ceil(total.value / TAILLE)))",
        ] {
            assert!(demonstration.contains(commun), "{demonstration}");
            assert!(engendre.contains(commun), "{engendre}");
        }
    }

    /// Chaque contrôle du formulaire se rend, et le type envoyé est celui du contrat.
    #[test]
    fn each_control_of_the_form_is_rendered_for_its_column_type() {
        let rendu = rendu(&Ecran::pour(&feature(), Lang::Fr));

        for temoin in [
            // Une chaîne et un texte long : le même champ.
            r#"id="champ-title""#,
            r#"id="champ-resume""#,
            // Un entier : un champ numérique, converti au départ.
            r#"type="number""#,
            // Un booléen : une case.
            "<Checkbox",
            // Une date et un instant : les contrôles natifs.
            r#"type="date""#,
            r#"type="datetime-local""#,
            // Une énumération : une liste, et le transtypage vers son union.
            "<Select",
            "'draft' | 'published'",
        ] {
            assert!(
                rendu.contains(temoin),
                "`{temoin}` manque au formulaire engendré :\n{rendu}"
            );
        }
    }

    /// Une colonne facultative accepte le vide, et l'envoie en `null`.
    #[test]
    fn an_optional_column_sends_null_when_it_is_left_empty() {
        let ecran = Ecran::pour(&feature(), Lang::Fr);

        let note = ecran
            .champs
            .iter()
            .find(|champ| champ.cle == "note")
            .expect("le champ facultatif du test doit être là");
        assert!(note.optionnel, "{note:?}");

        let rendu = rendu(&ecran);
        assert!(rendu.contains("  note: string | null\n"), "{rendu}");
    }

    /// Le filtre porte sur la première colonne textuelle ; sans aucune, il disparaît.
    #[test]
    fn the_filter_targets_the_first_textual_column_or_disappears() {
        let avec = Ecran::pour(&feature(), Lang::Fr);
        assert!(avec.filtrable);
        assert_eq!(
            avec.api
                .as_ref()
                .expect("un écran engendré porte son api")
                .recherche,
            "title"
        );
        assert!(rendu(&avec).contains("title: motif === '' ? undefined : { contains: motif }"));

        let champs = fields::parse("vues:int,publie:bool").expect("les champs doivent s'analyser");
        let sans = Ecran::pour(&Feature::fresh("compteurs", champs), Lang::Fr);

        assert!(!sans.filtrable);
        let rendu = rendu(&sans);
        assert!(
            !rendu.contains("const filtre = ref"),
            "un filtre qui ne filtre rien serait un mensonge :\n{rendu}"
        );
        assert!(!rendu.contains("TEXTES.filtre"), "{rendu}");
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

    /// L'écran engendré se monte par le même chemin, à l'entité près — et un nom composé
    /// devient un segment à tirets, un nom de route et un composant sans surprise.
    #[test]
    fn a_generated_screen_mounts_itself_the_same_way() {
        let champs = fields::parse("title:string").expect("les champs doivent s'analyser");
        let ecran = Ecran::pour(&Feature::fresh("blog_posts", champs), Lang::Fr);

        assert_eq!(ecran.segment, "blog-posts");
        assert_eq!(ecran.route, "admin-blog-posts");
        assert_eq!(ecran.composant, "BlogPosts");
        assert_eq!(ecran.fichier(), "frontend/src/admin/vues/BlogPosts.vue");
        assert_eq!(
            ecran.montage,
            "{\n  path: 'blog-posts',\n  name: 'admin-blog-posts',\n  component: () => \
             import('./vues/BlogPosts.vue'),\n},"
        );
        assert_eq!(
            ecran.rail,
            "{ route: 'admin-blog-posts', libelle: 'Blog posts' },"
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
            for ecran in [Ecran::demonstration(lang), Ecran::pour(&feature(), lang)] {
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
                    textes.actions.as_str(),
                    textes.creer.as_str(),
                    textes.modifier.as_str(),
                    textes.supprimer.as_str(),
                    textes.detail.as_str(),
                    textes.enregistrer.as_str(),
                    textes.enregistrement.as_str(),
                    textes.annuler.as_str(),
                    textes.suppression_titre.as_str(),
                    textes.suppression_detail.as_str(),
                    textes.enregistree.as_str(),
                    textes.supprimee.as_str(),
                    textes.refus_enregistrement.as_str(),
                    textes.refus_suppression.as_str(),
                    textes.introuvable.as_str(),
                    textes.vide.as_str(),
                    textes.oui.as_str(),
                    textes.non.as_str(),
                ]
                .into_iter()
                .chain(
                    ecran
                        .colonnes
                        .iter()
                        .map(|colonne| colonne.libelle.as_str()),
                )
                .chain(
                    ecran
                        .proprietes
                        .iter()
                        .map(|propriete| propriete.libelle.as_str()),
                )
                .chain(ecran.champs.iter().flat_map(|champ| {
                    std::iter::once(champ.libelle.as_str())
                        .chain(champ.valeurs.iter().map(String::as_str))
                }))
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
}
