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
use crate::generate::reference::Issue;
use crate::lang::Lang;

/// Combien de lignes une page porte : cinq sur la démonstration, dont les onze lignes
/// doivent montrer trois pages, vingt sur une table réelle, où une page de cinq ferait
/// paginer ce qui tiendrait à l'écran.
///
/// C'est, avec les quatre fonctions de source, la seule chose qui sépare les deux
/// producteurs — et la seule qui ne dépende pas d'un contrat.
const TAILLE_DEMONSTRATION: u32 = 5;
const TAILLE_ENGENDREE: u32 = 20;

/// Le composant de l'écran que le fragment dépose pour son entité de démonstration.
///
/// Une table qui porterait ce nom rendrait son écran sur le même fichier et sous la même
/// route : l'écran du fragment tomberait sous `--force`, sa route resterait pointée dessus,
/// et le rail porterait deux entrées sur un nom de route déclaré deux fois — le routeur n'en
/// monterait plus aucune. `generate crud` le refuse, et `--no-admin` lève le refus.
pub(crate) const DEMONSTRATION: &str = "Demonstration";

/// Où le socle attend le client typé, et où le shell d'administration l'importe.
///
/// C'est le couplage qu'ADR-0003 assume les yeux ouverts : la commande connaît désormais la
/// disposition des fichiers du socle. C'est aussi le répertoire que `generate client` vise
/// par défaut dès que le fragment `frontend` est posé, et un test des deux côtés le garde —
/// déplacer ce répertoire casse la génération, et c'est le prix annoncé.
pub(crate) const CLIENT: &str = "frontend/src/api";

/// L'écran patron, pris là où le fragment `frontend-admin` le dépose.
///
/// Le seul `include_str!` de la crate à le viser, et c'est ce qui rend structurelle la
/// promesse « une seule template, deux producteurs » : [`Ecran::rendre`] est la seule voie
/// par laquelle `rbs generate crud` atteint un écran, et elle ne peut rendre que ce
/// fichier-ci. Un second gabarit demanderait d'écrire un second `include_str!`, ce qu'un
/// test refuse.
const PATRON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/features/frontend-admin/client/src/admin/vues/Patron.vue.jinja"
));

/// Les deux fichiers génériques des références, là où le fragment les dépose.
///
/// `generate crud` les rend s'ils manquent — projet équipé du shell avant qu'ils
/// n'existent — et ne les écrase jamais : ils appartiennent au projet une fois posés.
pub(crate) const GENERIQUES: [(&str, &str); 2] = [
    (
        "frontend/src/admin/references/ChoixReference.vue",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/frontend-admin/client/src/admin/references/ChoixReference.vue.jinja"
        )),
    ),
    (
        "frontend/src/admin/references/libelles.ts",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/frontend-admin/client/src/admin/references/libelles.ts.jinja"
        )),
    ),
];

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
    /// Comment la valeur se lit : voir [`Colonne::rendu`].
    pub rendu: String,
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
    /// Comment la valeur se lit : `texte`, `date`, `instant` ou `reference`.
    ///
    /// Le contrat porte ses horodatages en ISO 8601, que personne ne lit — une table qui
    /// affiche `2026-09-20T16:24:47.272129Z` donne l'heure sans la dire. La template
    /// n'aurait pas pu le deviner : [`Propriete::type_base`] vaut `string` pour une date
    /// comme pour un texte, et une clé qui *ressemble* à un horodatage n'en est pas un.
    ///
    /// Trois valeurs et non les sept du contrôle : un booléen se dit déjà sans rien savoir
    /// de la colonne, et un nombre s'écrit comme il vient.
    pub rendu: String,
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
    /// Le nom du contrôle : la template s'en sert pour brancher la *forme* des deux
    /// expressions qui traversent le formulaire — ce qui part vers la source, et ce qui en
    /// revient.
    pub controle: String,
    /// Le composant d'interface qui rend le contrôle : `input`, `checkbox`, `select` ou
    /// `reference`.
    ///
    /// Distinct du nom : c'est lui que la template lit pour choisir le balisage, et lui que
    /// [`composants`] compte pour savoir quoi importer.
    pub composant: String,
    /// L'attribut `type` du champ de saisie ; vide hors du composant `input`.
    pub type_html: String,
    /// Les valeurs admises, pour le seul composant `select`.
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

/// Ce que l'écran sait d'une colonne référence : comment chercher ses lignes et les nommer.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReferenceEcran {
    /// La colonne : `ticket_id`.
    pub cle: String,
    /// La méthode du client qui filtre la cible : `ticketsFilter`.
    pub methode: String,
    /// La colonne libellé de la cible, absente quand l'identifiant raccourci en tient lieu.
    pub libelle: Option<String>,
    /// La référence peut rester vide.
    pub optionnel: bool,
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
    /// Les conversions de temps que l'écran porte, pour la même raison.
    pub formats: Vec<String>,
    /// L'étiquette BCP 47 dans laquelle l'écran formate ses dates.
    pub locale: String,
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
    /// Les références qu'un sélecteur choisit et qu'un libellé nomme ; vide tant que
    /// [`Ecran::avec_references`] ne les a pas branchées, et toujours sur la démonstration.
    pub references: Vec<ReferenceEcran>,
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
            ("reference", "Référence", "Reference", Controle::Texte, true),
            ("libelle", "Libellé", "Label", Controle::Texte, true),
            ("etat", "État", "State", Controle::Liste, false),
            ("maj", "Mise à jour", "Updated", Controle::Date, true),
        ];
        let valeurs_de = |controle: Controle| match controle {
            Controle::Liste => etats.iter().map(|etat| (*etat).to_string()).collect(),
            _ => Vec::new(),
        };

        let proprietes: Vec<Propriete> = declarees
            .iter()
            .map(|(cle, fr, en, controle, _)| Propriete {
                cle: (*cle).to_string(),
                libelle: dans(lang, fr, en),
                type_base: controle.type_ts(&valeurs_de(*controle)),
                rendu: controle.rendu().to_string(),
                optionnel: false,
            })
            .collect();

        let colonnes: Vec<Colonne> = declarees
            .iter()
            .map(|(cle, fr, en, controle, triable)| Colonne {
                cle: (*cle).to_string(),
                libelle: dans(lang, fr, en),
                rendu: controle.rendu().to_string(),
                triable: *triable,
            })
            .collect();

        let champs: Vec<Champ> = declarees
            .iter()
            .map(|(cle, fr, en, controle, _)| {
                champ_de(
                    cle,
                    dans(lang, fr, en),
                    *controle,
                    valeurs_de(*controle),
                    false,
                )
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
        let composant = DEMONSTRATION.to_string();

        Self {
            montage: montage(&segment, &route, &composant),
            rail: rail(&route, titre),
            titre: titre.to_string(),
            sous_titre: sous_titre.to_string(),
            segment,
            route,
            composant,
            cle: "reference".to_string(),
            taille: TAILLE_DEMONSTRATION,
            filtrable: true,
            composants: composants(&champs, true),
            formats: formats(&colonnes, &proprietes, &champs),
            locale: lang.tag().to_string(),
            proprietes,
            colonnes,
            champs,
            lignes,
            api: None,
            references: Vec::new(),
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
            rendu: "texte".to_string(),
            optionnel: false,
        }];
        proprietes.extend(feature.fields.iter().map(propriete));
        // Les deux horodatages que le noyau tient lui-même : aucun champ ne les déclare,
        // donc aucun contrôle ne dit comment ils se lisent.
        for (cle, fr, en) in [
            ("created_at", "Créé le", "Created"),
            ("updated_at", "Mise à jour", "Updated"),
        ] {
            proprietes.push(Propriete {
                cle: cle.to_string(),
                libelle: dans(lang, fr, en),
                type_base: "string".to_string(),
                rendu: Controle::Instant.rendu().to_string(),
                optionnel: false,
            });
        }

        let mut colonnes: Vec<Colonne> = feature
            .fields
            .iter()
            .map(|field| Colonne {
                cle: field.column_name(),
                libelle: humanise(&field.column_name()),
                rendu: Controle::of(field).rendu().to_string(),
                triable: true,
            })
            .collect();
        colonnes.push(Colonne {
            cle: "updated_at".to_string(),
            libelle: dans(lang, "Mise à jour", "Updated"),
            rendu: Controle::Instant.rendu().to_string(),
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
            formats: formats(&colonnes, &proprietes, &champs),
            locale: lang.tag().to_string(),
            proprietes,
            colonnes,
            champs,
            lignes: Vec::new(),
            api: Some(api),
            references: Vec::new(),
            textes: Textes::of(lang),
        }
    }

    /// Branche les références que le plan a pu résoudre ; celles qui restent en texte
    /// gardent le champ de saisie et la colonne brute.
    ///
    /// Une colonne référence n'est plus triable : elle se lit par le libellé de la cible,
    /// et un tri sur l'identifiant sous-jacent rangerait les lignes dans un ordre que
    /// l'opérateur ne voit pas.
    pub(crate) fn avec_references(mut self, issues: &[Issue]) -> Self {
        for issue in issues {
            let Issue::Selecteur(fiche) = issue else {
                continue;
            };
            let optionnel = self
                .champs
                .iter()
                .find(|champ| champ.cle == fiche.cle)
                .is_some_and(|champ| champ.optionnel);

            for colonne in self.colonnes.iter_mut().filter(|c| c.cle == fiche.cle) {
                colonne.rendu = "reference".to_string();
                colonne.libelle = fiche.entete.clone();
                colonne.triable = false;
            }
            for propriete in self.proprietes.iter_mut().filter(|p| p.cle == fiche.cle) {
                propriete.rendu = "reference".to_string();
                propriete.libelle = fiche.entete.clone();
            }
            for champ in self.champs.iter_mut().filter(|c| c.cle == fiche.cle) {
                champ.composant = "reference".to_string();
                champ.libelle = fiche.entete.clone();
            }
            self.references.push(ReferenceEcran {
                cle: fiche.cle.clone(),
                methode: fiche.methode.clone(),
                libelle: fiche.libelle.clone(),
                optionnel,
            });
        }
        self.composants = composants(&self.champs, self.filtrable);
        self
    }

    /// Le fichier de l'écran, relatif à la racine du projet.
    pub(crate) fn fichier(&self) -> String {
        format!("frontend/src/admin/vues/{}.vue", self.composant)
    }

    /// L'écran rendu, prêt à écrire.
    ///
    /// La seule voie par laquelle `rbs generate crud` atteint un écran : elle ne sait rendre
    /// que [`PATRON`], et un second gabarit demanderait donc de toucher ce module-ci. C'est
    /// ce qui rend la promesse « une seule template, deux producteurs » structurelle plutôt
    /// que gardée par un test.
    pub(crate) fn rendre(&self) -> Result<String, minijinja::Error> {
        crate::template::Renderer::new().render(PATRON, minijinja::context! { ecran => self })
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

/// Les conversions de temps qu'un écran porte, parmi trois.
///
/// `date` et `instant` sont les deux formateurs que la lecture demande — la table et le
/// détail passant par la même fonction, une propriété suffit à en exiger un, même quand
/// aucune colonne ne l'affiche : les deux horodatages du noyau sont dans ce cas. `saisie`
/// est l'aller-retour du formulaire, qu'un seul champ instant réclame.
///
/// Calculées ici et non devinées dans la template, pour la raison qui vaut aux composants :
/// `noUnusedLocals` arrête la vérification des types sur un formateur construit et jamais
/// lu — une table sans colonne de temps n'en porte aucun.
fn formats(colonnes: &[Colonne], proprietes: &[Propriete], champs: &[Champ]) -> Vec<String> {
    let mut retenus = Vec::new();

    for rendu in ["date", "instant"] {
        let lu = colonnes.iter().any(|colonne| colonne.rendu == rendu)
            || proprietes.iter().any(|propriete| propriete.rendu == rendu);
        if lu {
            retenus.push(rendu.to_string());
        }
    }
    if champs.iter().any(|champ| champ.controle == "instant") {
        retenus.push("saisie".to_string());
    }

    retenus
}

/// Les composants d'interface qu'un écran importe, au-delà de ceux qu'il importe toujours.
///
/// Le bouton, la table, la boîte de dialogue et le panneau sont de tous les écrans — la
/// création, le détail et la suppression en dépendent quelle que soit la table. Les trois
/// autres suivent les composants que réclament les champs, et le champ de saisie suit
/// aussi le filtre.
fn composants(champs: &[Champ], filtrable: bool) -> Vec<String> {
    let mut retenus = Vec::new();

    if filtrable || champs.iter().any(|champ| champ.composant == "input") {
        retenus.push("input".to_string());
    }
    if !champs.is_empty() {
        retenus.push("label".to_string());
    }
    for composant in ["checkbox", "select"] {
        if champs.iter().any(|champ| champ.composant == composant) {
            retenus.push(composant.to_string());
        }
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
pub(crate) fn humanise(nom: &str) -> String {
    let espace = nom.replace('_', " ");
    let mut chars = espace.chars();

    match chars.next() {
        Some(premier) => premier.to_uppercase().collect::<String>() + chars.as_str(),
        None => espace,
    }
}

/// Le contrôle qui saisit un champ.
///
/// Une énumération se saisit dans une liste, un booléen dans une case, une date et un
/// instant dans les contrôles natifs qui les rendent — leur format de sortie est exactement
/// celui que le contrat attend, à l'instant près, dont le fuseau se repose à l'envoi. Un
/// texte long passe par le même champ qu'une chaîne courte : les quatorze composants figés
/// n'en portent pas d'autre, et en ajouter un ferait dépendre l'écran engendré de ce que le
/// socle n'a pas.
///
/// Une énumération, et elle seule, plutôt que sept chaînes nues : le contrôle décide de
/// quatre choses — le composant, l'attribut `type`, le type TypeScript, et la forme des
/// deux expressions que la template écrit — et sept comparaisons de chaînes dispersées les
/// auraient fait diverger à la première variante ajoutée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Controle {
    /// Une chaîne, un texte long, un identifiant, une référence.
    Texte,
    /// Un entier ou un flottant.
    Nombre,
    /// Un décimal, que le contrat porte en chaîne : un nombre JavaScript y perdrait les
    /// centimes, ce que la représentation choisie par le noyau évite précisément.
    Decimal,
    /// Un booléen.
    Booleen,
    /// Une date.
    Date,
    /// Un instant.
    Instant,
    /// Une valeur parmi celles qu'une énumération déclare.
    Liste,
}

impl Controle {
    /// Celui qui saisit `field`.
    fn of(field: &Field) -> Self {
        if !field.enum_variants().is_empty() {
            return Self::Liste;
        }
        if field.reference().is_some() {
            return Self::Texte;
        }

        match field.column_type() {
            FieldType::String | FieldType::Text | FieldType::Uuid => Self::Texte,
            FieldType::Int | FieldType::Float => Self::Nombre,
            FieldType::Decimal => Self::Decimal,
            FieldType::Bool => Self::Booleen,
            FieldType::Date => Self::Date,
            FieldType::Datetime => Self::Instant,
        }
    }

    /// Le nom que la template lit pour brancher la forme d'une expression.
    fn nom(self) -> &'static str {
        match self {
            Self::Texte => "texte",
            Self::Nombre => "nombre",
            Self::Decimal => "decimal",
            Self::Booleen => "booleen",
            Self::Date => "date",
            Self::Instant => "instant",
            Self::Liste => "liste",
        }
    }

    /// Comment une valeur ainsi saisie se relit dans la table et dans le détail.
    ///
    /// Les cinq contrôles qui ne portent pas de temps se lisent tels quels : voir
    /// [`Colonne::rendu`].
    fn rendu(self) -> &'static str {
        match self {
            Self::Date => "date",
            Self::Instant => "instant",
            _ => "texte",
        }
    }

    /// Le composant d'interface qui le rend.
    fn composant(self) -> &'static str {
        match self {
            Self::Booleen => "checkbox",
            Self::Liste => "select",
            _ => "input",
        }
    }

    /// L'attribut `type` du champ de saisie, vide hors du composant `input`.
    fn type_html(self) -> &'static str {
        match self {
            Self::Nombre => "number",
            Self::Date => "date",
            Self::Instant => "datetime-local",
            Self::Booleen | Self::Liste => "",
            Self::Texte | Self::Decimal => "text",
        }
    }

    /// Le type TypeScript de la valeur envoyée, sa nullité exceptée. `valeurs` ne sert
    /// qu'à la liste, dont le type *est* l'union de ce qu'elle déclare.
    fn type_ts(self, valeurs: &[String]) -> String {
        match self {
            Self::Nombre => "number".to_string(),
            Self::Booleen => "boolean".to_string(),
            Self::Liste => union(valeurs),
            Self::Texte | Self::Decimal | Self::Date | Self::Instant => "string".to_string(),
        }
    }
}

/// Le champ de formulaire d'une propriété, quel que soit son producteur.
fn champ_de(
    cle: &str,
    libelle: String,
    controle: Controle,
    valeurs: Vec<String>,
    optionnel: bool,
) -> Champ {
    Champ {
        cle: cle.to_string(),
        libelle,
        controle: controle.nom().to_string(),
        composant: controle.composant().to_string(),
        type_html: controle.type_html().to_string(),
        type_base: controle.type_ts(&valeurs),
        valeurs,
        optionnel,
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
    let controle = Controle::of(field);

    Propriete {
        cle: field.column_name(),
        libelle: humanise(&field.column_name()),
        type_base: controle.type_ts(field.enum_variants()),
        rendu: controle.rendu().to_string(),
        optionnel: field.optional,
    }
}

/// Le champ de formulaire qu'un champ déclaré reçoit.
fn champ(field: &Field) -> Champ {
    champ_de(
        &field.column_name(),
        humanise(&field.column_name()),
        Controle::of(field),
        field.enum_variants().to_vec(),
        field.optional,
    )
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
    use crate::generate::reference::{Fiche, Issue};
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

    /// Ce qu'un écran d'administration porte et que rien d'autre ne porte : la table de
    /// ses colonnes. Sert de signature pour compter les gabarits d'écran du dépôt.
    const SIGNATURE: &str = "const COLONNES: readonly Colonne[] = [";

    /// Le fragment rend exactement la template que [`Ecran::rendre`] rend.
    ///
    /// C'est le contrôle central de la conception : une seule forme, deux producteurs. Le
    /// fragment atteint sa template par l'arborescence qu'`include_dir` embarque, la
    /// commande par un `include_str!` — deux chemins de lecture, qui doivent aboutir aux
    /// mêmes octets. Sans ce test, rien ne signalerait qu'ils ont cessé de le faire.
    #[test]
    fn the_fragment_renders_the_very_template_the_command_renders() {
        let source =
            crate::templates::Source::feature(None, "frontend-admin").expect("le fragment s'ouvre");
        let (manifeste, fichiers) = source.manifest_and_files().expect("le fragment se lit");
        let manifeste = crate::manifest::read(
            &manifeste.expect("le fragment porte un manifeste"),
            "frontend-admin/feature.toml",
        )
        .expect("le manifeste embarqué est valide");

        let depose = crate::add::installation::a_deposer("frontend-admin", &manifeste, &fichiers)
            .expect("les templates du fragment sont là");
        let (_, template, _) = depose
            .iter()
            .find(|(destination, _, _)| destination == "frontend/src/admin/vues/Demonstration.vue")
            .expect("le fragment dépose l'écran de démonstration");

        assert_eq!(
            *template, PATRON,
            "le fragment et la commande ne rendent plus le même fichier"
        );

        // Et personne ne dicte plus le répertoire du client : c'est celui que la commande
        // vise d'elle-même dès que le socle est posé, et le shell s'en remet à elle.
        assert!(
            !manifeste
                .feature
                .next_steps
                .iter()
                .any(|geste| geste.contains("--out")),
            "le shell dicte encore un répertoire de client : {:?}",
            manifeste.feature.next_steps
        );

        // Le geste appartient au socle, qui porte le module instanciant le client. Sans
        // `--out` : le répertoire qu'il obtiendra est `CLIENT`, et c'est le défaut de la
        // commande qui le lui donne.
        let socle = crate::templates::Source::feature(None, "frontend").expect("le socle s'ouvre");
        let (socle, _) = socle.manifest_and_files().expect("le socle se lit");
        let socle = crate::manifest::read(
            &socle.expect("le socle porte un manifeste"),
            "frontend/feature.toml",
        )
        .expect("le manifeste embarqué est valide");
        assert!(
            socle
                .feature
                .next_steps
                .iter()
                .any(|geste| geste == "rbs generate client --lang ts"),
            "le socle ne dit plus comment engendrer le client qu'il importe : {:?}",
            socle.feature.next_steps
        );
    }

    /// Un seul gabarit d'écran dans tout le dépôt.
    ///
    /// Le balayage est récursif et porte sur le contenu, non sur un nom de fichier ni sur
    /// un répertoire : une copie posée sous `templates/feature/tests/` et rebaptisée
    /// `ecran.jinja` serait exactement la divergence que la conception refuse, et un
    /// contrôle qui ne regarderait qu'un répertoire ou qu'une extension la laisserait
    /// passer.
    #[test]
    fn the_repository_carries_exactly_one_screen_template() {
        let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
        let mut gabarits = Vec::new();
        let mut a_visiter = vec![racine.clone()];

        while let Some(repertoire) = a_visiter.pop() {
            for entree in std::fs::read_dir(&repertoire).expect("le répertoire se lit") {
                let chemin = entree.expect("l'entrée se lit").path();
                if chemin.is_dir() {
                    a_visiter.push(chemin);
                } else if std::fs::read_to_string(&chemin)
                    .is_ok_and(|contenu| contenu.contains(SIGNATURE))
                {
                    gabarits.push(
                        chemin
                            .strip_prefix(&racine)
                            .expect("sous la racine")
                            .to_path_buf(),
                    );
                }
            }
        }

        assert_eq!(
            gabarits,
            [Path::new(CHEMIN)
                .strip_prefix("templates")
                .expect("le chemin part de templates/")],
            "un seul gabarit d'écran doit vivre dans le dépôt"
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
        let rendu = rendu(&Ecran::pour(&feature(), Lang::Fr));

        assert!(rendu.contains("  title: string\n"), "{rendu}");
        assert!(
            rendu.contains("{ cle: 'title', libelle: 'Title', rendu: 'texte', triable: true },"),
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

    /// Les trois colonnes de temps se lisent formatées, et les autres telles quelles.
    ///
    /// La table rendait `2026-09-20T16:24:47.272129Z` : l'ISO du contrat, que personne ne
    /// lit de l'œil. Les deux horodatages du noyau en sont, bien qu'aucun champ ne les
    /// déclare.
    #[test]
    fn a_column_carrying_time_is_read_formatted_and_the_others_as_they_come() {
        let ecran = Ecran::pour(&feature(), Lang::Fr);

        for (cle, attendu) in [
            ("title", "texte"),
            ("vues", "texte"),
            ("paru", "date"),
            ("vu", "instant"),
            ("updated_at", "instant"),
        ] {
            let colonne = ecran
                .colonnes
                .iter()
                .find(|colonne| colonne.cle == cle)
                .unwrap_or_else(|| panic!("la colonne `{cle}` doit être là"));
            assert_eq!(colonne.rendu, attendu, "{colonne:?}");
        }
        let cree = ecran
            .proprietes
            .iter()
            .find(|propriete| propriete.cle == "created_at")
            .expect("le détail montre la date de création");
        assert_eq!(cree.rendu, "instant", "{cree:?}");

        let rendu = rendu(&ecran);
        for temoin in [
            "const INSTANT = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'short', timeStyle: \
             'short' })",
            "const JOUR = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'short', timeZone: 'UTC' })",
            "{{ afficher(ligne[colonne.cle], colonne.rendu) }}",
            "{{ afficher(detaillee[propriete.cle], propriete.rendu) }}",
        ] {
            assert!(rendu.contains(temoin), "`{temoin}` manque :\n{rendu}");
        }
        assert!(
            !rendu.contains("{{ afficher(ligne[colonne.cle]) }}"),
            "la table rend encore l'ISO brut :\n{rendu}"
        );
    }

    /// La locale se fige à la génération, comme les libellés qu'elle accompagne.
    #[test]
    fn the_locale_of_the_dates_follows_the_language_of_the_project() {
        let anglais = rendu(&Ecran::pour(&feature(), Lang::En));

        assert!(
            anglais.contains("new Intl.DateTimeFormat('en-US',"),
            "{anglais}"
        );
        assert!(!anglais.contains("'fr-FR'"), "{anglais}");
    }

    /// Un écran ne porte que les conversions qu'il emploie.
    ///
    /// `noUnusedLocals` et `noUnusedParameters` sont posés sur le projet engendré : un
    /// formateur construit et jamais lu arrêterait la vérification des types. La
    /// démonstration en est le témoin — sa seule colonne de temps est un jour, et aucun de
    /// ses champs ne se saisit en instant.
    #[test]
    fn a_screen_carries_only_the_time_conversions_it_uses() {
        let demonstration = Ecran::demonstration(Lang::Fr);
        assert_eq!(demonstration.formats, ["date"]);

        let rendu = rendu(&demonstration);
        assert!(
            rendu.contains("const JOUR = new Intl.DateTimeFormat("),
            "{rendu}"
        );
        for absent in [
            "const INSTANT",
            "function pourControle",
            "rendu === 'instant'",
        ] {
            assert!(!rendu.contains(absent), "`{absent}` est de trop :\n{rendu}");
        }
    }

    /// Un instant se saisit à l'heure de l'opérateur, et n'est plus décalé par l'édition.
    ///
    /// `datetime-local` ne porte pas de fuseau : l'ISO du contrat, simplement tronqué,
    /// entrait décalé dans le contrôle et ressortait décalé une seconde fois — ouvrir puis
    /// enregistrer une ligne sans y toucher déplaçait son horodatage.
    #[test]
    fn an_instant_makes_the_round_trip_of_the_form_without_moving() {
        let ecran = Ecran::pour(&feature(), Lang::Fr);
        assert!(
            ecran.formats.contains(&"saisie".to_string()),
            "{:?}",
            ecran.formats
        );

        let rendu = rendu(&ecran);
        assert!(rendu.contains("    vu: pourControle(ligne.vu),"), "{rendu}");
        assert!(
            rendu.contains("instant.getTime() - instant.getTimezoneOffset() * 60_000"),
            "{rendu}"
        );
        assert!(
            !rendu.contains("ligne.vu.slice(0, 16)"),
            "l'instant entre encore tronqué dans le contrôle :\n{rendu}"
        );
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

    fn avec_ticket() -> Ecran {
        let feature = Feature::fresh(
            "commentaires",
            fields::parse("corps:text,ticket:references:tickets,auteur:references:users:optional")
                .expect("valide"),
        );
        let issues = vec![
            Issue::Selecteur(Fiche {
                cle: "ticket_id".into(),
                entete: "Ticket".into(),
                methode: "ticketsFilter".into(),
                libelle: Some("sujet".into()),
            }),
            Issue::Selecteur(Fiche {
                cle: "auteur_id".into(),
                entete: "Auteur".into(),
                methode: "usersFilter".into(),
                libelle: Some("email".into()),
            }),
        ];
        Ecran::pour(&feature, Lang::Fr).avec_references(&issues)
    }

    #[test]
    fn a_reference_column_is_read_by_its_label_and_is_not_sortable() {
        let ecran = avec_ticket();
        let colonne = ecran
            .colonnes
            .iter()
            .find(|c| c.cle == "ticket_id")
            .expect("colonne");

        assert_eq!(colonne.rendu, "reference");
        assert_eq!(colonne.libelle, "Ticket");
        assert!(!colonne.triable);
    }

    #[test]
    fn a_reference_field_is_chosen_not_typed() {
        let rendu = rendu(&avec_ticket());

        assert!(
            rendu.contains("import ChoixReference from '@/admin/references/ChoixReference.vue'"),
            "{rendu}"
        );
        assert!(
            rendu.contains(":chercher=\"REFERENCES.ticket_id.chercher\""),
            "{rendu}"
        );
        assert!(rendu.contains("api.ticketsFilter("), "{rendu}");
        assert!(
            rendu.contains("sujet: motif === '' ? undefined : { contains: motif }"),
            "{rendu}"
        );
        assert!(rendu.contains("id: { in: ids }"), "{rendu}");
    }

    #[test]
    fn an_optional_reference_offers_to_clear_it() {
        let rendu = rendu(&avec_ticket());

        let auteur = rendu
            .split("id=\"champ-auteur_id\"")
            .nth(1)
            .and_then(|suite| suite.split("/>").next())
            .expect("le sélecteur de l'auteur est rendu");
        let ticket = rendu
            .split("id=\"champ-ticket_id\"")
            .nth(1)
            .and_then(|suite| suite.split("/>").next())
            .expect("le sélecteur du ticket est rendu");

        assert!(
            auteur.contains("optionnel"),
            "l'auteur est facultatif : {auteur}"
        );
        assert!(
            !ticket.contains("optionnel"),
            "le ticket est requis : {ticket}"
        );
    }

    /// Relire une seule ligne — le détail — ne doit pas faire oublier les libellés des
    /// autres lignes de la page : `nommer` fusionne ce qu'il résout, il ne remplace pas.
    #[test]
    fn naming_rows_merges_the_labels_already_resolved() {
        let rendu = rendu(&avec_ticket());

        let nommer = rendu
            .split("async function nommer(")
            .nth(1)
            .and_then(|suite| suite.split("\n}\n").next())
            .expect("nommer est rendue");
        assert!(
            nommer.contains("new Map([...(libelles.value[cle] ?? []), ...nouvelles])"),
            "{nommer}"
        );
    }

    /// Le libellé qu'on vient de choisir reste affiché, même quand la page courante ne
    /// nomme pas la ligne choisie : sinon l'identifiant raccourci l'écraserait aussitôt.
    #[test]
    fn the_selector_keeps_the_label_it_has_just_chosen() {
        let (_, selecteur) = GENERIQUES[0];
        let rendu = Renderer::new()
            .render(selecteur, context! { lang => "fr" })
            .expect("le sélecteur se rend");

        let repos = rendu
            .split("function repos(): string {")
            .nth(1)
            .and_then(|suite| suite.split("\n}\n").next())
            .expect("repos est rendue");
        assert!(
            repos.contains("dernier !== null && dernier.cle === props.modelValue"),
            "{repos}"
        );
        assert!(rendu.contains("dernier = entree"), "{rendu}");
        // Déclaré avant le premier appel de `repos`, qui l'initialise : une zone morte
        // temporelle ferait planter le composant à son montage.
        let declaration = rendu.find("let dernier").expect("dernier est déclaré");
        let premier_appel = rendu
            .find("ref(repos())")
            .expect("repos initialise la saisie");
        assert!(declaration < premier_appel, "{rendu}");
    }

    fn selecteur_rendu() -> String {
        Renderer::new()
            .render(GENERIQUES[0].1, context! { lang => "fr" })
            .expect("le sélecteur se rend")
    }

    fn corps<'a>(rendu: &'a str, ouverture: &str) -> &'a str {
        rendu
            .split(ouverture)
            .nth(1)
            .and_then(|suite| suite.split("\n}\n").next())
            .unwrap_or_else(|| panic!("{ouverture} est rendue :\n{rendu}"))
    }

    /// Jamais un libellé faux : une source qui ignore `in` rend des lignes quelconques, et
    /// seules celles qu'on a demandées reçoivent un libellé.
    #[test]
    fn the_labels_keep_only_the_identifiers_they_asked_for() {
        let rendu = Renderer::new()
            .render(GENERIQUES[1].1, context! { lang => "fr" })
            .expect("les libellés se rendent");

        let resoudre = corps(&rendu, "export async function resoudre(");
        let garde = resoudre
            .find("if (voulus.has(entree.cle)) {")
            .expect("les réponses sont filtrées par les identifiants demandés");
        let pose = resoudre
            .find("libelles.set(entree.cle, entree.libelle)")
            .expect("les libellés sont posés");
        assert!(garde < pose, "{resoudre}");
        assert_eq!(
            resoudre.matches("libelles.set(").count(),
            1,
            "un libellé se pose hors de la garde :\n{resoudre}"
        );
    }

    /// Un 403 dit que la recherche est réservée, et non qu'elle est en panne : l'opérateur
    /// qui n'est pas admin sait alors pourquoi la liste reste vide.
    #[test]
    fn the_selector_tells_a_refusal_from_a_failure() {
        let rendu = selecteur_rendu();

        let chercher = corps(
            &rendu,
            "async function chercher(motif: string): Promise<void> {",
        );
        assert!(
            chercher
                .contains("faute.value = refusee(cause) ? TEXTES.reserve : TEXTES.indisponible"),
            "{chercher}"
        );
        assert!(
            rendu.contains("reserve: 'Réservé aux administrateurs'"),
            "{rendu}"
        );
    }

    /// Sans colonne libellé, la recherche n'a rien où chercher le motif : le paramètre
    /// reste, mais nommé pour que `noUnusedParameters` ne l'arrête pas.
    #[test]
    fn a_reference_without_a_label_column_reads_its_shortened_identifier() {
        let mut ecran = avec_ticket();
        ecran.references[1].libelle = None;
        let rendu = rendu(&ecran);

        let auteur = rendu
            .split("  auteur_id: {")
            .nth(1)
            .and_then(|suite| suite.split("\n  },").next())
            .expect("la référence de l'auteur est rendue");
        assert!(auteur.contains("async (_motif: string)"), "{auteur}");
        assert!(!auteur.contains("motif ==="), "{auteur}");
        assert!(auteur.contains("libelle: courte(ligne.id)"), "{auteur}");
        assert!(
            rendu.contains("async (motif: string)"),
            "le ticket cherche : {rendu}"
        );
    }

    #[test]
    fn a_reference_left_as_text_keeps_the_text_input() {
        let feature = Feature::fresh(
            "sessions",
            fields::parse("jeton:references:refresh_tokens").expect("valide"),
        );
        let issues = vec![Issue::Texte];
        let rendu = rendu(&Ecran::pour(&feature, Lang::Fr).avec_references(&issues));

        assert!(!rendu.contains("ChoixReference"), "{rendu}");
        assert!(rendu.contains("id=\"champ-jeton_id\""), "{rendu}");
    }

    /// Le rendu de la démonstration, figé avant que le patron n'apprenne les références.
    ///
    /// Figé dans la crate et non lu dans `examples/` : l'exemple se régénère avec le
    /// patron, et comparer le patron à sa propre sortie ne prouverait rien. Ce fichier-ci
    /// ne bouge que si l'on décide que la démonstration doit bouger.
    #[test]
    fn the_demonstration_screen_does_not_move_by_a_byte() {
        assert_eq!(
            rendu(&Ecran::demonstration(Lang::Fr)),
            include_str!("ecran/demonstration.fr.vue")
        );
    }

    /// Le fragment dépose les deux fichiers génériques que la commande dépose quand ils
    /// manquent, depuis les mêmes templates : deux sources divergeraient.
    #[test]
    fn the_fragment_lays_the_very_generic_files_the_command_lays() {
        let source =
            crate::templates::Source::feature(None, "frontend-admin").expect("le fragment s'ouvre");
        let (manifeste, fichiers) = source.manifest_and_files().expect("le fragment se lit");
        let manifeste = crate::manifest::read(
            &manifeste.expect("le fragment porte un manifeste"),
            "frontend-admin/feature.toml",
        )
        .expect("le manifeste embarqué est valide");
        let depose = crate::add::installation::a_deposer("frontend-admin", &manifeste, &fichiers)
            .expect("les templates du fragment sont là");

        for (destination, template) in GENERIQUES {
            let (_, deposee, _) = depose
                .iter()
                .find(|(chemin, _, _)| chemin == destination)
                .unwrap_or_else(|| panic!("le fragment ne dépose pas {destination}"));
            assert_eq!(*deposee, template, "{destination} diverge");
        }
    }

    /// Le sélecteur parle la langue du projet, et ses deux fichiers se rendent dans l'une
    /// comme dans l'autre.
    #[test]
    fn the_generic_files_speak_the_language_of_the_project() {
        for (lang, temoin) in [(Lang::Fr, "Aucune correspondance"), (Lang::En, "No match")] {
            let (_, selecteur) = GENERIQUES[0];
            let rendu = Renderer::new()
                .render(selecteur, context! { lang => lang.name() })
                .expect("le sélecteur se rend");
            assert!(rendu.contains(temoin), "{lang:?} : {rendu}");

            Renderer::new()
                .render(GENERIQUES[1].1, context! { lang => lang.name() })
                .expect("les libellés se rendent");
        }
    }
}
