use rbs_core::{Comparison, ComparisonSchema, Error, Result, Sort, TextMatch, TextMatchSchema};
use sea_orm::prelude::{DateTimeUtc, Uuid};
use sea_orm::{ColumnTrait, Condition, QueryFilter, QueryOrder, Select, Value};
use serde::Deserialize;
use utoipa::ToSchema;

use super::model::{Column, Entity};

// Toute colonne est filtrable, indexée ou non : un filtre sur une colonne sans index
// parcourt la table. Ajoutez « index » au champ dans `--fields` si la table grandit.
#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct SubscriberFilter {
    // Chaque condition est décrite par le schéma que le noyau en donne : utoipa ne sait
    // pas décrire un générique dont le paramètre n'implémente pas `ToSchema`, et un objet
    // libre ne nommerait aucun opérateur. Les opérateurs acceptés sont `eq`, `gt`, `gte`,
    // `lt`, `lte` et `is_null` ; une valeur nue vaut `eq`.
    #[schema(value_type = ComparisonSchema)]
    pub id: Option<Comparison<Uuid>>,
    #[schema(value_type = ComparisonSchema)]
    pub created_at: Option<Comparison<DateTimeUtc>>,
    #[schema(value_type = ComparisonSchema)]
    pub updated_at: Option<Comparison<DateTimeUtc>>,
    #[schema(value_type = TextMatchSchema)]
    pub email: Option<TextMatch>,
    #[schema(value_type = TextMatchSchema)]
    pub name: Option<TextMatch>,
    #[schema(value_type = ComparisonSchema)]
    pub confirmed: Option<Comparison<bool>>,
    /// Colonnes de tri, préfixées de `-` pour l'ordre décroissant.
    #[schema(value_type = Vec<String>)]
    pub sort: Option<Sort>,
}

/// Traduit un nom de colonne reçu du client en `Column`.
///
/// Le `match` est écrit à la génération : aucun nom venu de la requête n'atteint la base,
/// et un nom inconnu est refusé en nommant ceux qui sont acceptés.
fn column_of(name: &str) -> Result<Column> {
    Ok(match name {
        "id" => Column::Id,
        "created_at" => Column::CreatedAt,
        "updated_at" => Column::UpdatedAt,
        "email" => Column::Email,
        "name" => Column::Name,
        "confirmed" => Column::Confirmed,
        inconnue => {
            return Err(Error::BadRequest(format!(
                "colonne de tri inconnue « {inconnue} » — id, created_at, updated_at, email, name, confirmed"
            )));
        }
    })
}

/// Applique le filtre à la requête de liste.
///
/// Le tri par défaut reste l'`id` décroissant : c'est un UUIDv7, et la pagination en
/// dépend.
pub(super) fn apply(select: Select<Entity>, filtre: &SubscriberFilter) -> Result<Select<Entity>> {
    let conditions = Condition::all()
        .add(compare(Column::Id, filtre.id.as_ref()))
        .add(compare(Column::CreatedAt, filtre.created_at.as_ref()))
        .add(compare(Column::UpdatedAt, filtre.updated_at.as_ref()))
        .add(matches(Column::Email, filtre.email.as_ref()))
        .add(matches(Column::Name, filtre.name.as_ref()))
        .add(compare(Column::Confirmed, filtre.confirmed.as_ref()));

    let select = select.filter(conditions);

    let Some(sort) = filtre.sort.as_ref().filter(|sort| !sort.keys().is_empty()) else {
        return Ok(select.order_by_desc(Column::Id));
    };

    sort.keys().iter().try_fold(select, |select, key| {
        let colonne = column_of(&key.column)?;

        Ok(match key.descending {
            true => select.order_by_desc(colonne),
            false => select.order_by_asc(colonne),
        })
    })
}

/// Les conditions portées sur une colonne comparable, en ET entre elles.
fn compare<T: Into<Value> + Clone>(colonne: Column, compare: Option<&Comparison<T>>) -> Condition {
    let Some(compare) = compare else {
        return Condition::all();
    };

    null_condition(colonne, compare.is_null)
        .add_option(compare.eq.clone().map(|valeur| colonne.eq(valeur)))
        .add_option(compare.gt.clone().map(|valeur| colonne.gt(valeur)))
        .add_option(compare.gte.clone().map(|valeur| colonne.gte(valeur)))
        .add_option(compare.lt.clone().map(|valeur| colonne.lt(valeur)))
        .add_option(compare.lte.clone().map(|valeur| colonne.lte(valeur)))
}

/// Les conditions portées sur une colonne textuelle, en ET entre elles.
fn matches(colonne: Column, recherche: Option<&TextMatch>) -> Condition {
    let Some(recherche) = recherche else {
        return Condition::all();
    };

    // `contains` rend un LIKE '%…%' et échappe la valeur. La casse suit la collation du
    // moteur : PostgreSQL la distingue, MySQL l'ignore par défaut.
    null_condition(colonne, recherche.is_null)
        .add_option(recherche.eq.clone().map(|valeur| colonne.eq(valeur)))
        .add_option(recherche.contains.clone().map(|v| colonne.contains(v)))
}

fn null_condition(colonne: Column, is_null: Option<bool>) -> Condition {
    match is_null {
        Some(true) => Condition::all().add(colonne.is_null()),
        Some(false) => Condition::all().add(colonne.is_not_null()),
        None => Condition::all(),
    }
}
