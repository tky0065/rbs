mod erreur;

pub(crate) use erreur::{ErreurChamp, ErreurChamps, NatureErreur};
use erreur::{en_snake_case, suggestions_mot_cle};

/// Un des sept types de la grammaire `--fields`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeChamp {
    String,
    Int,
    Float,
    Bool,
    Uuid,
    Datetime,
    Text,
}

impl TypeChamp {
    pub(crate) const NOMS: [&'static str; 7] =
        ["string", "int", "float", "bool", "uuid", "datetime", "text"];

    pub(crate) fn analyser(mot: &str) -> Option<Self> {
        Some(match mot {
            "string" => Self::String,
            "int" => Self::Int,
            "float" => Self::Float,
            "bool" => Self::Bool,
            "uuid" => Self::Uuid,
            "datetime" => Self::Datetime,
            "text" => Self::Text,
            _ => return None,
        })
    }

    pub(crate) fn nom(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Uuid => "uuid",
            Self::Datetime => "datetime",
            Self::Text => "text",
        }
    }

    pub(crate) fn type_rust(self) -> &'static str {
        match self {
            Self::String | Self::Text => "String",
            Self::Int => "i32",
            Self::Float => "f64",
            Self::Bool => "bool",
            Self::Uuid => "Uuid",
            Self::Datetime => "DateTimeWithTimeZone",
        }
    }

    pub(crate) fn methode_migration(self) -> &'static str {
        match self {
            Self::String => "string()",
            Self::Int => "integer()",
            Self::Float => "double()",
            Self::Bool => "boolean()",
            Self::Uuid => "uuid()",
            Self::Datetime => "timestamp_with_time_zone()",
            Self::Text => "text()",
        }
    }

    /// SeaORM déduit la colonne du type Rust ; seul `text` doit être forcé, `String`
    /// donnant sinon un `varchar`.
    pub(crate) fn attribut_column_type(self) -> Option<&'static str> {
        match self {
            Self::Text => Some("Text"),
            _ => None,
        }
    }
}

/// Un champ déclaré dans `--fields`, une fois analysé et validé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Champ {
    pub nom: String,
    pub type_: TypeChamp,
    pub unique: bool,
    pub optionnel: bool,
    pub index: bool,
}

impl Champ {
    pub(crate) fn type_rust(&self) -> String {
        if self.optionnel {
            format!("Option<{}>", self.type_.type_rust())
        } else {
            self.type_.type_rust().to_string()
        }
    }
}

/// Analyse la chaîne `--fields`. Les fautes de tous les champs sont collectées en une
/// passe : l'utilisateur corrige sa ligne d'un coup plutôt qu'une faute par exécution.
pub(crate) fn analyser(entree: &str) -> Result<Vec<Champ>, ErreurChamps> {
    if entree.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut champs = Vec::new();
    let mut erreurs = Vec::new();

    for (rang, portion) in entree.split(',').enumerate() {
        match analyser_champ(rang + 1, portion.trim()) {
            Ok(champ) => champs.push(champ),
            Err(erreur) => erreurs.push(erreur),
        }
    }

    if erreurs.is_empty() {
        Ok(champs)
    } else {
        Err(ErreurChamps { erreurs })
    }
}

fn analyser_champ(rang: usize, portion: &str) -> Result<Champ, ErreurChamp> {
    let erreur = |libelle: &str, nature| ErreurChamp {
        rang,
        libelle: libelle.to_string(),
        nature,
    };

    let mut parties = portion.split(':').map(str::trim);
    let nom = parties.next().unwrap_or_default();
    let type_brut = parties.next().unwrap_or_default();

    if nom.is_empty() || type_brut.is_empty() {
        return Err(erreur(portion, NatureErreur::FormeInvalide));
    }

    if !est_en_snake_case(nom) {
        // Une recasse qui rendrait le nom inchangé, ou toujours invalide — un nom
        // accentué, par exemple — vaut mieux ne pas être proposée du tout.
        let recasse = en_snake_case(nom);
        let suggestion = (recasse != nom && est_en_snake_case(&recasse)).then_some(recasse);

        return Err(erreur(nom, NatureErreur::PasEnSnakeCase { suggestion }));
    }

    if MOTS_CLES_RUST.contains(&nom) {
        return Err(erreur(
            nom,
            NatureErreur::MotCleRust {
                suggestions: suggestions_mot_cle(nom),
            },
        ));
    }

    if NOMS_POSES_PAR_RBS.contains(&nom) {
        return Err(erreur(nom, NatureErreur::NomReserve));
    }

    let Some(type_) = TypeChamp::analyser(type_brut) else {
        return Err(erreur(
            nom,
            NatureErreur::TypeInconnu {
                nom: type_brut.to_string(),
            },
        ));
    };

    let mut champ = Champ {
        nom: nom.to_string(),
        type_,
        unique: false,
        optionnel: false,
        index: false,
    };

    for modificateur in parties {
        let drapeau = match modificateur {
            "unique" => &mut champ.unique,
            "optional" => &mut champ.optionnel,
            "index" => &mut champ.index,
            inconnu => {
                return Err(erreur(
                    nom,
                    NatureErreur::ModificateurInconnu {
                        nom: inconnu.to_string(),
                    },
                ));
            }
        };

        if *drapeau {
            return Err(erreur(
                nom,
                NatureErreur::ModificateurEnDouble {
                    nom: modificateur.to_string(),
                },
            ));
        }

        *drapeau = true;
    }

    if champ.unique && champ.index {
        return Err(erreur(nom, NatureErreur::IndexRedondant));
    }

    Ok(champ)
}

/// Mots-clés stricts et réservés des éditions 2015 à 2024. Un champ ainsi nommé
/// produirait une entité que rustc refuse, quarante secondes plus tard.
const MOTS_CLES_RUST: [&str; 49] = [
    "as", "async", "await", "become", "box", "break", "const", "continue", "crate", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof", "unsafe",
    "unsized", "use", "virtual", "where", "while",
];

/// Posées par rbs sur toute entité : les redéclarer donnerait deux fois la colonne.
const NOMS_POSES_PAR_RBS: [&str; 3] = ["id", "created_at", "updated_at"];

fn est_en_snake_case(nom: &str) -> bool {
    let Some(premier) = nom.chars().next() else {
        return false;
    };

    premier.is_ascii_lowercase()
        && !nom.ends_with('_')
        && nom
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_type_de_la_grammaire_est_reconnu() {
        let cas = [
            ("string", TypeChamp::String),
            ("int", TypeChamp::Int),
            ("float", TypeChamp::Float),
            ("bool", TypeChamp::Bool),
            ("uuid", TypeChamp::Uuid),
            ("datetime", TypeChamp::Datetime),
            ("text", TypeChamp::Text),
        ];

        for (mot, attendu) in cas {
            assert_eq!(TypeChamp::analyser(mot), Some(attendu), "type « {mot} »");
        }
    }

    #[test]
    fn un_type_hors_grammaire_n_est_pas_reconnu() {
        assert_eq!(TypeChamp::analyser("decimal"), None);
        assert_eq!(TypeChamp::analyser("String"), None);
        assert_eq!(TypeChamp::analyser(""), None);
    }

    #[test]
    fn nom_est_l_inverse_de_analyser() {
        for mot in TypeChamp::NOMS {
            let type_ = TypeChamp::analyser(mot).expect("NOMS ne liste que des types connus");
            assert_eq!(type_.nom(), mot);
        }
    }

    #[test]
    fn chaque_type_se_projette_vers_rust() {
        assert_eq!(TypeChamp::String.type_rust(), "String");
        assert_eq!(TypeChamp::Text.type_rust(), "String");
        assert_eq!(TypeChamp::Int.type_rust(), "i32");
        assert_eq!(TypeChamp::Float.type_rust(), "f64");
        assert_eq!(TypeChamp::Bool.type_rust(), "bool");
        assert_eq!(TypeChamp::Uuid.type_rust(), "Uuid");
        assert_eq!(TypeChamp::Datetime.type_rust(), "DateTimeWithTimeZone");
    }

    #[test]
    fn chaque_type_se_projette_vers_une_methode_de_migration() {
        assert_eq!(TypeChamp::String.methode_migration(), "string()");
        assert_eq!(TypeChamp::Text.methode_migration(), "text()");
        assert_eq!(TypeChamp::Int.methode_migration(), "integer()");
        assert_eq!(TypeChamp::Float.methode_migration(), "double()");
        assert_eq!(TypeChamp::Bool.methode_migration(), "boolean()");
        assert_eq!(TypeChamp::Uuid.methode_migration(), "uuid()");
        assert_eq!(
            TypeChamp::Datetime.methode_migration(),
            "timestamp_with_time_zone()"
        );
    }

    #[test]
    fn seul_text_porte_un_attribut_column_type() {
        assert_eq!(TypeChamp::Text.attribut_column_type(), Some("Text"));
        for mot in TypeChamp::NOMS {
            if mot == "text" {
                continue;
            }
            let type_ = TypeChamp::analyser(mot).expect("NOMS ne liste que des types connus");
            assert_eq!(type_.attribut_column_type(), None, "type « {mot} »");
        }
    }

    #[test]
    fn un_champ_optionnel_enveloppe_son_type_rust() {
        let obligatoire = Champ {
            nom: "titre".to_string(),
            type_: TypeChamp::String,
            unique: false,
            optionnel: false,
            index: false,
        };
        let optionnel = Champ {
            optionnel: true,
            ..obligatoire.clone()
        };

        assert_eq!(obligatoire.type_rust(), "String");
        assert_eq!(optionnel.type_rust(), "Option<String>");
    }

    fn champs(entree: &str) -> Vec<Champ> {
        analyser(entree).expect("la chaîne doit être valide")
    }

    #[test]
    fn une_chaine_vide_ne_declare_aucun_champ() {
        assert_eq!(analyser(""), Ok(Vec::new()));
        assert_eq!(analyser("   "), Ok(Vec::new()));
    }

    #[test]
    fn un_champ_sans_modificateur_a_ses_trois_drapeaux_baisses() {
        let champs = champs("titre:string");

        assert_eq!(champs.len(), 1);
        assert_eq!(champs[0].nom, "titre");
        assert_eq!(champs[0].type_, TypeChamp::String);
        assert!(!champs[0].unique);
        assert!(!champs[0].optionnel);
        assert!(!champs[0].index);
    }

    #[test]
    fn chaque_modificateur_leve_son_drapeau() {
        assert!(champs("email:string:unique")[0].unique);
        assert!(champs("bio:text:optional")[0].optionnel);
        assert!(champs("slug:string:index")[0].index);
    }

    #[test]
    fn l_ordre_des_modificateurs_est_libre() {
        assert_eq!(
            champs("email:string:unique:optional"),
            champs("email:string:optional:unique")
        );
    }

    #[test]
    fn les_espaces_autour_des_separateurs_sont_ignores() {
        assert_eq!(
            champs(" titre : string , email : string : unique "),
            champs("titre:string,email:string:unique")
        );
    }

    #[test]
    fn les_champs_gardent_leur_ordre_de_declaration() {
        let champs = champs("un:string,deux:int,trois:bool");
        let noms: Vec<&str> = champs.iter().map(|champ| champ.nom.as_str()).collect();

        assert_eq!(noms, ["un", "deux", "trois"]);
    }

    #[test]
    fn un_champ_sans_type_est_une_forme_invalide() {
        let erreur = analyser("titre").expect_err("un champ sans type est refusé");

        assert_eq!(erreur.erreurs.len(), 1);
        assert_eq!(erreur.erreurs[0].rang, 1);
        assert_eq!(erreur.erreurs[0].libelle, "titre");
        assert_eq!(erreur.erreurs[0].nature, NatureErreur::FormeInvalide);
    }

    #[test]
    fn une_virgule_finale_est_une_forme_invalide() {
        let erreur = analyser("titre:string,").expect_err("la virgule finale est refusée");

        assert_eq!(erreur.erreurs.len(), 1);
        assert_eq!(erreur.erreurs[0].rang, 2);
        assert_eq!(erreur.erreurs[0].nature, NatureErreur::FormeInvalide);
    }

    #[test]
    fn un_type_hors_grammaire_est_signale_sur_son_champ() {
        let erreur = analyser("prix:decimal").expect_err("decimal n'est pas dans la grammaire");

        assert_eq!(erreur.erreurs[0].libelle, "prix");
        assert_eq!(
            erreur.erreurs[0].nature,
            NatureErreur::TypeInconnu {
                nom: "decimal".to_string()
            }
        );
    }

    fn nature(entree: &str) -> NatureErreur {
        let mut erreur = analyser(entree).expect_err("la chaîne doit être refusée");
        assert_eq!(erreur.erreurs.len(), 1, "une seule faute attendue");
        erreur.erreurs.remove(0).nature
    }

    #[test]
    fn un_nom_hors_snake_case_est_refuse_avec_sa_recasse() {
        assert_eq!(
            nature("Title:string"),
            NatureErreur::PasEnSnakeCase {
                suggestion: Some("title".to_string())
            }
        );
        assert_eq!(
            nature("firstName:string"),
            NatureErreur::PasEnSnakeCase {
                suggestion: Some("first_name".to_string())
            }
        );
    }

    #[test]
    fn un_nom_accentue_est_refuse_sans_suggestion_trompeuse() {
        assert_eq!(
            nature("prénom:string"),
            NatureErreur::PasEnSnakeCase { suggestion: None }
        );
    }

    #[test]
    fn un_nom_a_souligne_final_ou_a_chiffre_initial_est_refuse() {
        assert!(matches!(
            nature("titre_:string"),
            NatureErreur::PasEnSnakeCase { .. }
        ));
        assert!(matches!(
            nature("1titre:string"),
            NatureErreur::PasEnSnakeCase { .. }
        ));
    }

    #[test]
    fn un_nom_a_chiffre_ou_souligne_interne_est_accepte() {
        let champs = champs("adresse_ligne_2:string");
        assert_eq!(champs[0].nom, "adresse_ligne_2");
    }

    #[test]
    fn un_mot_cle_rust_est_refuse_avant_la_compilation() {
        assert_eq!(
            nature("type:string"),
            NatureErreur::MotCleRust {
                suggestions: vec!["kind".to_string(), "type_".to_string()]
            }
        );
        assert!(matches!(
            nature("match:string"),
            NatureErreur::MotCleRust { .. }
        ));
        assert!(matches!(
            nature("async:bool"),
            NatureErreur::MotCleRust { .. }
        ));
        assert!(matches!(
            nature("box:string"),
            NatureErreur::MotCleRust { .. }
        ));
    }

    #[test]
    fn les_trois_colonnes_posees_par_rbs_sont_refusees() {
        for nom in ["id", "created_at", "updated_at"] {
            assert_eq!(
                nature(&format!("{nom}:string")),
                NatureErreur::NomReserve,
                "nom « {nom} »"
            );
        }
    }

    #[test]
    fn un_modificateur_en_double_est_refuse() {
        assert_eq!(
            nature("email:string:unique:unique"),
            NatureErreur::ModificateurEnDouble {
                nom: "unique".to_string()
            }
        );
    }

    #[test]
    fn unique_avec_index_est_refuse_comme_redondant() {
        assert_eq!(
            nature("slug:string:unique:index"),
            NatureErreur::IndexRedondant
        );
        assert_eq!(
            nature("slug:string:index:unique"),
            NatureErreur::IndexRedondant
        );
    }

    #[test]
    fn un_unique_sur_du_texte_passe_sans_commentaire() {
        assert!(champs("bio:text:unique")[0].unique);
        assert!(champs("actif:bool:index")[0].index);
    }

    #[test]
    fn toutes_les_fautes_de_la_chaine_remontent_dans_l_ordre() {
        let erreur =
            analyser("Title:string,type:text,prix:decimal").expect_err("trois fautes attendues");

        assert_eq!(erreur.erreurs.len(), 3);
        assert_eq!(erreur.erreurs[0].rang, 1);
        assert!(matches!(
            erreur.erreurs[0].nature,
            NatureErreur::PasEnSnakeCase { .. }
        ));
        assert_eq!(erreur.erreurs[1].rang, 2);
        assert!(matches!(
            erreur.erreurs[1].nature,
            NatureErreur::MotCleRust { .. }
        ));
        assert_eq!(erreur.erreurs[2].rang, 3);
        assert!(matches!(
            erreur.erreurs[2].nature,
            NatureErreur::TypeInconnu { .. }
        ));
    }

    #[test]
    fn un_champ_portant_deux_fautes_ne_remonte_que_la_premiere() {
        let erreur = analyser("Type:decimal").expect_err("deux fautes, une seule remontée");

        assert_eq!(erreur.erreurs.len(), 1);
        assert!(matches!(
            erreur.erreurs[0].nature,
            NatureErreur::PasEnSnakeCase { .. }
        ));
    }
}
