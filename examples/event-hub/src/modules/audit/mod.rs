pub mod model;
pub mod repository;

#[cfg(test)]
mod tests;

// Réexportée pour que le projet écrive `audit::record(&transaction, entry)` : tant
// qu'aucun service ne le fait, le compilateur la tient pour inutile.
#[allow(unused_imports)]
pub use repository::record;

/// Les trois actions du CRUD, nommées une fois pour ne pas les réécrire à chaque appel.
///
/// Ce sont des constantes et non un enum : l'ensemble est ouvert, et un `login` ou un
/// `export` sont des actions légitimes qu'un enum fermé forcerait à contourner.
pub const CREATE: &str = "create";
pub const UPDATE: &str = "update";
pub const DELETE: &str = "delete";

/// Une écriture à inscrire au journal.
#[derive(Debug, Clone)]
pub struct Entry {
    pub action: String,
    pub entity: String,
    pub entity_id: String,
    pub actor_id: Option<String>,
    pub changes: serde_json::Value,
}

impl Entry {
    /// Les trois champs sans lesquels une ligne de journal ne veut rien dire.
    pub fn new(
        action: impl Into<String>,
        entity: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> Self {
        Self {
            action: action.into(),
            entity: entity.into(),
            entity_id: entity_id.into(),
            actor_id: None,
            changes: serde_json::Value::Null,
        }
    }

    /// L'auteur de l'écriture. Sous `auth`, c'est `identity.user_id`.
    pub fn actor(mut self, actor_id: impl Into<String>) -> Self {
        self.actor_id = Some(actor_id.into());
        self
    }

    /// Ce qui a changé, sous la forme que le projet décide.
    pub fn changes(mut self, changes: serde_json::Value) -> Self {
        self.changes = changes;
        self
    }
}
