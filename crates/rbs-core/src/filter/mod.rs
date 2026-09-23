//! Opérateurs de filtrage et de tri, sans connaissance des colonnes.
//!
//! Les types de ce module ne nomment aucune colonne et n'en valident aucune : c'est le
//! `filter.rs` que le CLI engendre par feature qui les compose en un type dont chaque
//! champ vient de `--fields`, et qui seul sait traduire un nom en `Column`.
//!
//! Aucun des types de ce module ne dérive `ToSchema` : le filtre engendré décrit ses
//! champs par `#[schema(value_type = ...)]`, en citant les schémas de [`schema`].

pub mod schema;

pub use schema::{
    BoolComparisonSchema, ComparisonSchema, DateComparisonSchema, DateTimeComparisonSchema,
    DecimalComparisonSchema, FloatComparisonSchema, IntComparisonSchema, OneOfSchema,
    TextMatchSchema, UuidComparisonSchema,
};

use serde::{Deserialize, Deserializer};

/// Conditions portées sur une colonne scalaire ordonnée.
///
/// Se lit d'une valeur nue, qui vaut `eq`, ou d'un objet nommant ses opérateurs :
/// `{ "views": 10 }` et `{ "views": { "eq": 10 } }` disent la même chose.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    /// Appartenance à l'une des valeurs citées. Une liste vide n'en accepte aucune.
    pub r#in: Option<Vec<T>>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

/// Conditions portées sur une colonne textuelle.
///
/// Se lit d'une chaîne nue, qui vaut `eq`, ou d'un objet nommant ses opérateurs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextMatch {
    /// Égalité stricte.
    pub eq: Option<String>,
    /// Sous-chaîne, cherchée par `LIKE '%…%'`.
    ///
    /// La casse suit la collation du moteur : PostgreSQL la distingue, MySQL l'ignore avec
    /// sa collation par défaut. `ILIKE` l'uniformiserait, mais il n'existe que sur
    /// PostgreSQL, et le CLI engendre aussi pour MySQL et SQLite.
    pub contains: Option<String>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

/// Conditions portées sur une colonne à valeurs énumérées.
///
/// Se lit d'une valeur nue, qui vaut `eq`, ou d'un objet nommant ses opérateurs :
/// `{ "status": "draft" }` et `{ "status": { "eq": "draft" } }` disent la même chose.
///
/// Une énumération ne s'ordonne pas : `in` y remplace les comparaisons de [`Comparison`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OneOf<T> {
    /// Égalité stricte.
    pub eq: Option<T>,
    /// Appartenance à l'une des valeurs citées. Une liste vide n'en accepte aucune.
    pub r#in: Option<Vec<T>>,
    /// `true` exige une colonne nulle, `false` une colonne renseignée.
    pub is_null: Option<bool>,
}

/// Une colonne de tri et son sens.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
enum Forme<T, O> {
    Bare(T),
    Operators(O),
}

#[derive(Deserialize)]
struct ComparisonInput<T> {
    eq: Option<T>,
    gt: Option<T>,
    gte: Option<T>,
    lt: Option<T>,
    lte: Option<T>,
    r#in: Option<Vec<T>>,
    is_null: Option<bool>,
}

#[derive(Deserialize)]
struct OneOfInput<T> {
    eq: Option<T>,
    r#in: Option<Vec<T>>,
    is_null: Option<bool>,
}

#[derive(Deserialize)]
struct TextMatchInput {
    eq: Option<String>,
    contains: Option<String>,
    is_null: Option<bool>,
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Comparison<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match Forme::<T, ComparisonInput<T>>::deserialize(deserializer)? {
                Forme::Bare(valeur) => Self {
                    eq: Some(valeur),
                    gt: None,
                    gte: None,
                    lt: None,
                    lte: None,
                    r#in: None,
                    is_null: None,
                },
                Forme::Operators(operateurs) => Self {
                    eq: operateurs.eq,
                    gt: operateurs.gt,
                    gte: operateurs.gte,
                    lt: operateurs.lt,
                    lte: operateurs.lte,
                    r#in: operateurs.r#in,
                    is_null: operateurs.is_null,
                },
            },
        )
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for OneOf<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match Forme::<T, OneOfInput<T>>::deserialize(deserializer)? {
                Forme::Bare(valeur) => Self {
                    eq: Some(valeur),
                    r#in: None,
                    is_null: None,
                },
                Forme::Operators(operateurs) => Self {
                    eq: operateurs.eq,
                    r#in: operateurs.r#in,
                    is_null: operateurs.is_null,
                },
            },
        )
    }
}

impl<'de> Deserialize<'de> for TextMatch {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(
            match Forme::<String, TextMatchInput>::deserialize(deserializer)? {
                Forme::Bare(valeur) => Self {
                    eq: Some(valeur),
                    contains: None,
                    is_null: None,
                },
                Forme::Operators(operateurs) => Self {
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

    /// `in` vaut sur une colonne comparable comme sur une énumération : c'est ce qui laisse
    /// un écran résoudre les identifiants d'une page entière en un seul appel.
    #[test]
    fn a_comparison_object_names_in() {
        let compare: Comparison<i32> =
            serde_json::from_str(r#"{"in": [1, 2]}"#).expect("objet lisible");

        assert_eq!(compare.r#in, Some(vec![1, 2]));
        assert_eq!(compare.eq, None);
    }

    /// Une liste vide reste une liste : le filtre engendré n'accepte alors aucune ligne,
    /// là où `None` n'en écarterait aucune.
    #[test]
    fn an_empty_in_list_on_a_comparison_stays_an_empty_list() {
        let compare: Comparison<i32> =
            serde_json::from_str(r#"{"in": []}"#).expect("objet lisible");

        assert_eq!(compare.r#in, Some(Vec::new()));
    }

    #[test]
    fn a_bare_comparison_value_carries_no_in_list() {
        let compare: Comparison<i32> = serde_json::from_str("7").expect("valeur nue lisible");

        assert_eq!(compare.r#in, None);
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

    /// La forme courte vaut sur une colonne à valeurs énumérées comme sur les autres :
    /// `{ "status": "draft" }` est ce qu'un client écrit, et doit valoir `eq`.
    #[test]
    fn a_bare_value_reads_as_an_equality_on_an_enumerated_column() {
        let choix: OneOf<String> = serde_json::from_str(r#""draft""#).expect("valeur nue lisible");

        assert_eq!(choix.eq.as_deref(), Some("draft"));
        assert_eq!(choix.r#in, None);
        assert_eq!(choix.is_null, None);
    }

    #[test]
    fn an_enumerated_object_names_its_operators() {
        let choix: OneOf<String> =
            serde_json::from_str(r#"{"in": ["draft", "published"]}"#).expect("objet lisible");

        assert_eq!(
            choix.r#in,
            Some(vec!["draft".to_owned(), "published".to_owned()])
        );
        assert_eq!(choix.eq, None);
    }

    /// Une liste vide est une liste, non l'absence de condition : le filtre engendré la
    /// traduit en `IN ()`, que sea-query écrit `1 = 2` — aucune ligne. La confondre avec
    /// `None` rendrait la liste entière là où le client n'accepte aucune valeur.
    #[test]
    fn an_empty_in_list_stays_an_empty_list() {
        let choix: OneOf<String> = serde_json::from_str(r#"{"in": []}"#).expect("objet lisible");

        assert_eq!(choix.r#in, Some(Vec::new()));
    }

    #[test]
    fn an_enumerated_column_reads_is_null() {
        let choix: OneOf<String> =
            serde_json::from_str(r#"{"is_null": true}"#).expect("objet lisible");

        assert_eq!(choix.is_null, Some(true));
        assert_eq!(choix.eq, None);
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
