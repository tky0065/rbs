use std::fmt;

use super::TypeChamp;

/// Toutes les fautes relevées dans une chaîne `--fields`, dans l'ordre des champs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ErreurChamps {
    pub erreurs: Vec<ErreurChamp>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ErreurChamp {
    /// Rang du champ dans la chaîne, à partir de 1.
    pub rang: usize,
    /// Le nom du champ, ou la portion brute quand le nom n'a pas pu être lu.
    pub libelle: String,
    pub nature: NatureErreur,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NatureErreur {
    FormeInvalide,
    PasEnSnakeCase { suggestion: Option<String> },
    MotCleRust { suggestions: Vec<String> },
    NomReserve,
    TypeInconnu { nom: String },
    ModificateurInconnu { nom: String },
    ModificateurEnDouble { nom: String },
    IndexRedondant,
}

impl NatureErreur {
    fn message(&self, libelle: &str) -> String {
        match self {
            Self::FormeInvalide => "forme attendue : « nom:type[:modificateur…] »".to_string(),
            Self::PasEnSnakeCase { .. } => {
                "le nom doit être en snake_case : minuscules ASCII, chiffres et souligné"
                    .to_string()
            }
            Self::MotCleRust { .. } => format!("« {libelle} » est un mot-clé Rust"),
            Self::NomReserve => format!("« {libelle} » ne se déclare pas"),
            Self::TypeInconnu { nom } => format!("type inconnu « {nom} »"),
            Self::ModificateurInconnu { nom } => format!("modificateur inconnu « {nom} »"),
            Self::ModificateurEnDouble { nom } => {
                format!("modificateur « {nom} » en double")
            }
            Self::IndexRedondant => {
                "« index » redondant : « unique » pose déjà un index".to_string()
            }
        }
    }

    fn indice(&self) -> Option<String> {
        match self {
            Self::FormeInvalide => Some("exemple : « email:string:unique »".to_string()),
            Self::PasEnSnakeCase { suggestion } => suggestion
                .as_ref()
                .map(|valeur| format!("essayez « {valeur} »")),
            Self::MotCleRust { suggestions } => {
                let liste: Vec<String> = suggestions.iter().map(|s| format!("« {s} »")).collect();
                Some(format!("essayez {}", liste.join(" ou ")))
            }
            Self::NomReserve => {
                Some("id, created_at et updated_at sont posés sur toute entité".to_string())
            }
            Self::TypeInconnu { .. } => Some(TypeChamp::NOMS.join(", ")),
            Self::ModificateurInconnu { .. } => Some("unique, optional, index".to_string()),
            Self::ModificateurEnDouble { .. } => None,
            Self::IndexRedondant => Some("retirez « index »".to_string()),
        }
    }
}

impl fmt::Display for ErreurChamps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut premier = true;
        for erreur in &self.erreurs {
            if !premier {
                writeln!(f)?;
            }
            premier = false;

            write!(
                f,
                "erreur : champ {} « {} » — {}",
                erreur.rang,
                erreur.libelle,
                erreur.nature.message(&erreur.libelle)
            )?;

            if let Some(indice) = erreur.nature.indice() {
                write!(f, "\n        → {indice}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ErreurChamps {}

/// Recasse un nom en snake_case sans chercher à interpréter les acronymes : la sortie
/// est une suggestion soumise à l'utilisateur, pas une correction appliquée d'office.
pub(crate) fn en_snake_case(nom: &str) -> String {
    let mut sortie = String::with_capacity(nom.len() + 4);

    for (rang, caractere) in nom.chars().enumerate() {
        if caractere.is_uppercase() {
            if rang > 0 && !sortie.ends_with('_') {
                sortie.push('_');
            }
            sortie.extend(caractere.to_lowercase());
        } else if caractere == '-' || caractere == ' ' {
            if !sortie.is_empty() && !sortie.ends_with('_') {
                sortie.push('_');
            }
        } else {
            sortie.push(caractere);
        }
    }

    sortie
}

/// Le suffixe `_` marche pour tout mot-clé ; les quatre alias devant lui sont ceux
/// qu'un développeur écrirait de lui-même.
pub(crate) fn suggestions_mot_cle(mot: &str) -> Vec<String> {
    let alias = match mot {
        "type" => Some("kind"),
        "ref" => Some("reference"),
        "match" => Some("matching"),
        "move" => Some("movement"),
        _ => None,
    };

    alias
        .map(str::to_string)
        .into_iter()
        .chain(std::iter::once(format!("{mot}_")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendu(nature: NatureErreur, libelle: &str) -> String {
        ErreurChamps {
            erreurs: vec![ErreurChamp {
                rang: 1,
                libelle: libelle.to_string(),
                nature,
            }],
        }
        .to_string()
    }

    #[test]
    fn une_forme_invalide_montre_la_forme_attendue() {
        let texte = rendu(NatureErreur::FormeInvalide, "titre");
        assert_eq!(
            texte,
            "erreur : champ 1 « titre » — forme attendue : « nom:type[:modificateur…] »\n\
             \x20       → exemple : « email:string:unique »"
        );
    }

    #[test]
    fn un_nom_mal_casse_suggere_sa_forme_snake_case() {
        let texte = rendu(
            NatureErreur::PasEnSnakeCase {
                suggestion: Some("title".to_string()),
            },
            "Title",
        );
        assert!(texte.contains("le nom doit être en snake_case"), "{texte}");
        assert!(texte.contains("→ essayez « title »"), "{texte}");
    }

    #[test]
    fn un_nom_sans_recasse_possible_n_a_pas_d_indice() {
        let texte = rendu(NatureErreur::PasEnSnakeCase { suggestion: None }, "prénom");

        assert!(
            texte.contains("minuscules ASCII, chiffres et souligné"),
            "{texte}"
        );
        assert!(!texte.contains("→"), "{texte}");
    }

    #[test]
    fn un_mot_cle_rust_suggere_ses_deux_replis() {
        let texte = rendu(
            NatureErreur::MotCleRust {
                suggestions: vec!["kind".to_string(), "type_".to_string()],
            },
            "type",
        );
        assert!(texte.contains("« type » est un mot-clé Rust"), "{texte}");
        assert!(texte.contains("→ essayez « kind » ou « type_ »"), "{texte}");
    }

    #[test]
    fn un_nom_reserve_rappelle_les_trois_colonnes_implicites() {
        let texte = rendu(NatureErreur::NomReserve, "id");
        assert!(texte.contains("« id » ne se déclare pas"), "{texte}");
        assert!(
            texte.contains("id, created_at et updated_at sont posés sur toute entité"),
            "{texte}"
        );
    }

    #[test]
    fn un_type_inconnu_liste_les_types_admis() {
        let texte = rendu(
            NatureErreur::TypeInconnu {
                nom: "decimal".to_string(),
            },
            "prix",
        );
        assert!(texte.contains("type inconnu « decimal »"), "{texte}");
        for mot in TypeChamp::NOMS {
            assert!(texte.contains(mot), "« {mot} » absent de : {texte}");
        }
    }

    #[test]
    fn un_modificateur_inconnu_liste_les_trois_admis() {
        let texte = rendu(
            NatureErreur::ModificateurInconnu {
                nom: "uniq".to_string(),
            },
            "name",
        );
        assert!(texte.contains("modificateur inconnu « uniq »"), "{texte}");
        assert!(texte.contains("unique, optional, index"), "{texte}");
    }

    #[test]
    fn un_modificateur_en_double_est_nomme() {
        let texte = rendu(
            NatureErreur::ModificateurEnDouble {
                nom: "unique".to_string(),
            },
            "email",
        );
        assert!(
            texte.contains("modificateur « unique » en double"),
            "{texte}"
        );
    }

    #[test]
    fn un_index_redondant_explique_pourquoi() {
        let texte = rendu(NatureErreur::IndexRedondant, "slug");
        assert!(
            texte.contains("« index » redondant : « unique » pose déjà un index"),
            "{texte}"
        );
        assert!(texte.contains("→ retirez « index »"), "{texte}");
    }

    #[test]
    fn plusieurs_erreurs_se_rendent_une_par_bloc_dans_l_ordre() {
        let texte = ErreurChamps {
            erreurs: vec![
                ErreurChamp {
                    rang: 1,
                    libelle: "Title".to_string(),
                    nature: NatureErreur::PasEnSnakeCase {
                        suggestion: Some("title".to_string()),
                    },
                },
                ErreurChamp {
                    rang: 2,
                    libelle: "type".to_string(),
                    nature: NatureErreur::MotCleRust {
                        suggestions: vec!["kind".to_string(), "type_".to_string()],
                    },
                },
            ],
        }
        .to_string();

        let lignes: Vec<&str> = texte.lines().collect();
        assert_eq!(lignes.len(), 4, "{texte}");
        assert!(
            lignes[0].starts_with("erreur : champ 1 « Title »"),
            "{texte}"
        );
        assert!(
            lignes[2].starts_with("erreur : champ 2 « type »"),
            "{texte}"
        );
    }

    #[test]
    fn en_snake_case_recasse_les_formes_usuelles() {
        assert_eq!(en_snake_case("Title"), "title");
        assert_eq!(en_snake_case("firstName"), "first_name");
        assert_eq!(en_snake_case("HTTPStatus"), "h_t_t_p_status");
        assert_eq!(en_snake_case("mon-champ"), "mon_champ");
        assert_eq!(en_snake_case("déjà_ok"), "déjà_ok");
    }

    #[test]
    fn un_mot_cle_courant_a_un_alias_avant_son_repli() {
        assert_eq!(suggestions_mot_cle("type"), vec!["kind", "type_"]);
        assert_eq!(suggestions_mot_cle("ref"), vec!["reference", "ref_"]);
        assert_eq!(suggestions_mot_cle("loop"), vec!["loop_"]);
    }
}
