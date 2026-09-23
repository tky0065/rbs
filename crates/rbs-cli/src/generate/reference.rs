//! Ce qu'un écran engendré sait d'une référence : où chercher ses lignes, et comment les
//! nommer.
//!
//! Séparé de `ecran.rs`, qui ne lit pas le disque : la colonne libellé se relève dans le
//! `model.rs` de la cible, et la route de filtre dans les contrôleurs du projet.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use super::entities::{self, Entity};
use super::feature::Feature;
use super::fields::{Field, FieldType};

/// Ce qu'un écran engendré sait d'une référence, une fois sa cible retrouvée dans le
/// projet : où filtrer ses lignes, et comment les nommer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fiche {
    /// La colonne de la table engendrée : `ticket_id`.
    pub cle: String,
    /// L'en-tête, humanisé depuis le nom de la relation : `Ticket`.
    pub entete: String,
    /// La méthode du client qui filtre la cible : `ticketsFilter`.
    pub methode: String,
    /// La colonne libellé de la cible : `Some("sujet")`, `None` → identifiant raccourci.
    pub libelle: Option<String>,
}

/// Une issue par champ référence, dans l'ordre des champs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Issue {
    /// La cible expose une route de filtre : l'écran rend un sélecteur à recherche.
    Selecteur(Fiche),
    /// La cible n'expose aucune route de filtre : l'écran retombe sur une saisie
    /// d'identifiant brute.
    Texte,
}

/// Ce que le plan annonce d'une référence qui ne reçoit pas tout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Repli {
    /// Nom de la relation, tel que `--fields` l'a déclaré : `auteur`.
    pub relation: String,
    /// Table visée : `users`.
    pub cible: String,
    pub cause: Cause,
}

/// Ce qui a manqué à une référence pour recevoir tout ce que l'écran sait en rendre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cause {
    /// La cible n'expose pas `POST /<table>/filter` : la recherche du sélecteur n'a rien
    /// à interroger.
    SansRouteDeFiltre,
    /// La cible n'a pas de colonne textuelle exploitable : le sélecteur retombe sur
    /// l'identifiant.
    SansColonneTextuelle,
}

/// Une référence dont `label=` nomme une colonne que la cible ne porte pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LabelInconnu {
    relation: String,
    cible: String,
    colonne: String,
    connues: Vec<String>,
}

impl fmt::Display for LabelInconnu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "relation « {} » — « {} » n'est pas une colonne textuelle de « {} »\n        \
             → colonnes textuelles : {}",
            self.relation,
            self.colonne,
            self.cible,
            if self.connues.is_empty() {
                "aucune".to_string()
            } else {
                self.connues.join(", ")
            }
        )
    }
}

impl std::error::Error for LabelInconnu {}

/// Les colonnes `String` ou `Option<String>` que le `struct Model` de `table` déclare.
///
/// Même lecture textuelle que `alter::colonnes_declarees`, le type en plus : une
/// énumération ou une référence n'y ressemblent pas, et c'est ce qui les écarte.
pub(crate) fn colonnes_textuelles(source: &str, table: &str) -> Vec<String> {
    let attribut = format!("table_name = \"{table}\"");
    let mut colonnes = Vec::new();
    let mut dans_la_structure = false;

    for ligne in source.lines() {
        let ligne = ligne.trim();
        if !dans_la_structure {
            dans_la_structure = ligne.contains(&attribut);
            continue;
        }
        if ligne == "}" {
            break;
        }
        if let Some(reste) = ligne.strip_prefix("pub ")
            && let Some((nom, type_)) = reste.split_once(':')
        {
            let type_ = type_.trim().trim_end_matches(',');
            if type_ == "String" || type_ == "Option<String>" {
                colonnes.push(nom.trim().to_string());
            }
        }
    }

    colonnes
}

/// La cible expose-t-elle `POST /<table>/filter` ? Lu à son `operation_id`, que
/// `generate crud` et le fragment `auth` écrivent tous deux.
pub(crate) fn a_une_route_de_filtre(root: &Path, table: &str) -> bool {
    let cherche = format!("operation_id = \"{table}_filter\"");
    fichiers_rust(&root.join("src"))
        .iter()
        .any(|fichier| fs::read_to_string(fichier).is_ok_and(|contenu| contenu.contains(&cherche)))
}

/// Les fichiers `.rs` sous `dossier`, à toute profondeur.
///
/// Les erreurs de lecture d'une entrée sont ignorées : un répertoire momentanément
/// inaccessible ne doit pas faire échouer un simple relevé, plus prudent qu'une route de
/// filtre manquée à tort.
fn fichiers_rust(dossier: &Path) -> Vec<PathBuf> {
    let mut fichiers = Vec::new();

    let Ok(entries) = fs::read_dir(dossier) else {
        return fichiers;
    };

    for entry in entries.flatten() {
        let chemin = entry.path();
        if chemin.is_dir() {
            fichiers.extend(fichiers_rust(&chemin));
        } else if chemin
            .extension()
            .is_some_and(|extension| extension == "rs")
        {
            fichiers.push(chemin);
        }
    }

    fichiers
}

/// Les colonnes textuelles de `cible`, qu'elle soit la feature en cours de génération —
/// auto-référence, pas encore sur le disque — ou une entité déjà présente dans le projet.
fn colonnes_de_la_cible(
    root: &Path,
    feature: &Feature,
    entities: &[Entity],
    cible: &str,
) -> Vec<String> {
    if cible == feature.module() {
        feature
            .fields
            .iter()
            .filter(|field| {
                field.reference().is_none()
                    && field.enum_variants().is_empty()
                    && matches!(field.column_type(), FieldType::String | FieldType::Text)
            })
            .map(Field::column_name)
            .collect()
    } else {
        match entities::find(entities, cible) {
            Some(entity) => colonnes_textuelles(
                &fs::read_to_string(root.join(&entity.file)).unwrap_or_default(),
                cible,
            ),
            None => Vec::new(),
        }
    }
}

/// Refuse un `label=<colonne>` qui ne désigne pas une colonne textuelle de sa cible.
///
/// Appelée avant le rendu, que l'écran soit engendré ou non : sans ce garde, une colonne
/// mal orthographiée n'échouerait que le jour où le projet recevrait le shell
/// d'administration.
pub(crate) fn verifier_les_labels(
    root: &Path,
    feature: &Feature,
    entities: &[Entity],
) -> Result<(), LabelInconnu> {
    for field in &feature.fields {
        let Some(reference) = field.reference() else {
            continue;
        };
        let Some(colonne) = &reference.label else {
            continue;
        };

        let connues = colonnes_de_la_cible(root, feature, entities, &reference.target);
        if !connues.contains(colonne) {
            return Err(LabelInconnu {
                relation: field.relation_name().to_string(),
                cible: reference.target.clone(),
                colonne: colonne.clone(),
                connues,
            });
        }
    }

    Ok(())
}

/// Une fiche par champ référence, et un repli pour chacune qui ne reçoit pas tout ce
/// qu'un écran sait en rendre.
pub(crate) fn fiches(
    root: &Path,
    feature: &Feature,
    entities: &[Entity],
) -> (Vec<Issue>, Vec<Repli>) {
    let mut issues = Vec::new();
    let mut replis = Vec::new();

    for field in &feature.fields {
        let Some(reference) = field.reference() else {
            continue;
        };
        let cible = &reference.target;
        let relation = field.relation_name().to_string();
        let auto_reference = cible == feature.module();

        if !auto_reference && !a_une_route_de_filtre(root, cible) {
            issues.push(Issue::Texte);
            replis.push(Repli {
                relation,
                cible: cible.clone(),
                cause: Cause::SansRouteDeFiltre,
            });
            continue;
        }

        let colonnes = colonnes_de_la_cible(root, feature, entities, cible);
        let libelle = reference
            .label
            .clone()
            .or_else(|| colonnes.first().cloned());
        if libelle.is_none() {
            replis.push(Repli {
                relation,
                cible: cible.clone(),
                cause: Cause::SansColonneTextuelle,
            });
        }

        issues.push(Issue::Selecteur(Fiche {
            cle: field.column_name(),
            entete: crate::ecran::humanise(field.relation_name()),
            methode: crate::client::ts::nom_de_methode(&format!("{cible}_filter")),
            libelle,
        }));
    }

    (issues, replis)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODELE: &str = r#"
#[sea_orm(table_name = "tickets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub sujet: String,
    pub resume: Option<String>,
    pub statut: TicketStatut,
    pub auteur_id: Uuid,
}
"#;

    #[test]
    fn the_textual_columns_are_the_strings_of_the_model_in_order() {
        assert_eq!(colonnes_textuelles(MODELE, "tickets"), ["sujet", "resume"]);
    }

    #[test]
    fn a_model_file_holding_several_tables_is_read_for_the_right_one() {
        let source = format!(
            "{MODELE}\n#[sea_orm(table_name = \"autres\")]\npub struct Model {{\n    pub nom: String,\n}}\n"
        );

        assert_eq!(colonnes_textuelles(&source, "autres"), ["nom"]);
    }

    #[test]
    fn a_filter_route_is_found_by_its_operation_id() {
        let racine = tempfile::TempDir::new().expect("répertoire");
        std::fs::create_dir_all(racine.path().join("src/tickets")).expect("dossier");
        std::fs::write(
            racine.path().join("src/tickets/controller.rs"),
            "    operation_id = \"tickets_filter\",\n",
        )
        .expect("écriture");

        assert!(a_une_route_de_filtre(racine.path(), "tickets"));
        assert!(!a_une_route_de_filtre(racine.path(), "refresh_tokens"));
    }
}
