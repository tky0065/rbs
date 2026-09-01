//! Opérateurs de filtrage et de tri, sans connaissance des colonnes.
//!
//! Les types de ce module ne nomment aucune colonne et n'en valident aucune : c'est le
//! `filter.rs` que le CLI engendre par feature qui les compose en un type dont chaque
//! champ vient de `--fields`, et qui seul sait traduire un nom en `Column`.

use serde::{Deserialize, Deserializer};
use utoipa::ToSchema;

/// Conditions portées sur une colonne scalaire ordonnée.
///
/// Se lit d'une valeur nue, qui vaut `eq`, ou d'un objet nommant ses opérateurs :
/// `{ "views": 10 }` et `{ "views": { "eq": 10 } }` disent la même chose.
#[derive(Debug, Clone, Default, PartialEq, Eq, ToSchema)]
pub struct Comparison<T> {
    /// Égalité stricte.
    pub eq: Option<T>,
    /// Strictement supérieur.
    pub gt: Option<T>,
    /// Supérieur ou égal.
    pub gte: Option<T>,
    /// Strictement inférieur.
    pub lt: Option<T>,
    /// Inférieur ou égal.
    pub lte: Option<T>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

/// Conditions portées sur une colonne textuelle.
///
/// Se lit d'une chaîne nue, qui vaut `eq`, ou d'un objet nommant ses opérateurs.
#[derive(Debug, Clone, Default, PartialEq, Eq, ToSchema)]
pub struct TextMatch {
    /// Égalité stricte.
    pub eq: Option<String>,
    /// Sous-chaîne, insensible à la casse.
    pub contains: Option<String>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

/// Une colonne de tri et son sens.
#[derive(Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct SortKey {
    /// Nom de la colonne, préfixe retiré.
    pub column: String,
    /// Vrai quand le nom reçu portait `-`.
    pub descending: bool,
}

/// Colonnes de tri, dans l'ordre où le client les a demandées.
///
/// Aucun nom n'est validé ici : seul le `filter.rs` engendré connaît les colonnes de son
/// entité, et c'est lui qui refuse celles qu'il ne sait pas traduire.
#[derive(Debug, Clone, Default, PartialEq, Eq, ToSchema)]
pub struct Sort(Vec<SortKey>);

impl Sort {
    /// Les colonnes demandées, dans l'ordre.
    pub fn keys(&self) -> &[SortKey] {
        &self.0
    }
}

/// Les deux formes qu'un opérateur accepte, avant d'être ramenées à une seule.
///
/// L'énumération reste privée : posée sur le type public, son `untagged` sortirait dans le
/// document OpenAPI, où la forme longue est la seule qui se décrive.
#[derive(Deserialize)]
#[serde(untagged)]
enum OneOf<T, O> {
    Bare(T),
    Operators(O),
}

#[derive(Deserialize)]
struct ComparisonOperators<T> {
    eq: Option<T>,
    gt: Option<T>,
    gte: Option<T>,
    lt: Option<T>,
    lte: Option<T>,
    is_null: Option<bool>,
}

#[derive(Deserialize)]
struct TextMatchOperators {
    eq: Option<String>,
    contains: Option<String>,
    is_null: Option<bool>,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Comparison<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match OneOf::<T, ComparisonOperators<T>>::deserialize(deserializer)? {
                OneOf::Bare(valeur) => Self {
                    eq: Some(valeur),
                    gt: None,
                    gte: None,
                    lt: None,
                    lte: None,
                    is_null: None,
                },
                OneOf::Operators(operateurs) => Self {
                    eq: operateurs.eq,
                    gt: operateurs.gt,
                    gte: operateurs.gte,
                    lt: operateurs.lt,
                    lte: operateurs.lte,
                    is_null: operateurs.is_null,
                },
            },
        )
    }
}

impl<'de> Deserialize<'de> for TextMatch {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match OneOf::<String, TextMatchOperators>::deserialize(deserializer)? {
                OneOf::Bare(valeur) => Self {
                    eq: Some(valeur),
                    contains: None,
                    is_null: None,
                },
                OneOf::Operators(operateurs) => Self {
                    eq: operateurs.eq,
                    contains: operateurs.contains,
                    is_null: operateurs.is_null,
                },
            },
        )
    }
}

impl<'de> Deserialize<'de> for Sort {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let colonnes = Vec::<String>::deserialize(deserializer)?;

        Ok(Self(
            colonnes
                .into_iter()
                .map(|colonne| match colonne.strip_prefix('-') {
                    Some(reste) => SortKey {
                        column: reste.to_owned(),
                        descending: true,
                    },
                    None => SortKey {
                        column: colonne,
                        descending: false,
                    },
                })
                .collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La forme courte est ce qu'un client écrit dans le cas courant : `"published": true`
    /// doit valoir `{ "eq": true }`, sans quoi le corps le plus fréquent serait le plus
    /// verbeux.
    #[test]
    fn a_bare_value_reads_as_an_equality() {
        let compare: Comparison<bool> = serde_json::from_str("true").expect("valeur nue lisible");

        assert_eq!(compare.eq, Some(true));
        assert_eq!(compare.gt, None);
    }

    #[test]
    fn an_object_names_its_operators() {
        let compare: Comparison<i32> =
            serde_json::from_str(r#"{"gte": 10, "lt": 100}"#).expect("objet lisible");

        assert_eq!(compare.gte, Some(10));
        assert_eq!(compare.lt, Some(100));
        assert_eq!(compare.eq, None);
    }

    #[test]
    fn a_bare_string_reads_as_an_equality_on_text() {
        let recherche: TextMatch = serde_json::from_str(r#""rust""#).expect("chaîne nue lisible");

        assert_eq!(recherche.eq.as_deref(), Some("rust"));
        assert_eq!(recherche.contains, None);
    }

    #[test]
    fn a_text_object_names_its_operators() {
        let recherche: TextMatch =
            serde_json::from_str(r#"{"contains": "rust"}"#).expect("objet lisible");

        assert_eq!(recherche.contains.as_deref(), Some("rust"));
        assert_eq!(recherche.eq, None);
    }

    /// Le préfixe `-` est la seule syntaxe de tri : elle tient dans une chaîne JSON, se
    /// lit sans documentation, et ne demande pas d'objet par colonne.
    #[test]
    fn the_minus_prefix_marks_a_descending_column() {
        let sort: Sort = serde_json::from_str(r#"["-views", "title"]"#).expect("tri lisible");

        assert_eq!(sort.keys().len(), 2);
        assert_eq!(sort.keys()[0].column, "views");
        assert!(sort.keys()[0].descending);
        assert_eq!(sort.keys()[1].column, "title");
        assert!(!sort.keys()[1].descending);
    }

    /// `Sort` ne connaît aucune colonne : c'est le `filter.rs` engendré qui refuse un nom
    /// inconnu, en les nommant tous. Ici, seule la syntaxe du préfixe est analysée.
    #[test]
    fn sort_keeps_a_column_it_knows_nothing_about() {
        let sort: Sort = serde_json::from_str(r#"["-inconnue"]"#).expect("tri lisible");

        assert_eq!(sort.keys()[0].column, "inconnue");
    }

    /// Un corps qui ne dit rien ne restreint rien : le filtre par défaut est celui que
    /// `list` emploie, et il doit rendre la liste entière.
    #[test]
    fn an_empty_filter_carries_no_condition() {
        let compare = Comparison::<i32>::default();

        assert_eq!(compare.eq, None);
        assert_eq!(compare.is_null, None);
        assert!(Sort::default().keys().is_empty());
    }
}
