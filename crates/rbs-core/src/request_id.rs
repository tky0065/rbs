//! Identifiant de corrélation de la requête courante.
//!
//! Ce module ne porte que le stockage : le middleware qui génère l'ULID, le reprend de
//! l'en-tête `x-request-id` et ouvre le scope vit ailleurs. Tout ce qui a besoin de
//! l'identifiant sans l'avoir reçu en paramètre — la réponse d'erreur, un log — le lit
//! par [`current`].

use std::future::Future;

tokio::task_local! {
    static REQUEST_ID: String;
}

/// Identifiant de la requête en cours, `None` en dehors d'une requête.
pub fn current() -> Option<String> {
    REQUEST_ID.try_with(String::clone).ok()
}

/// Exécute `future` en lui associant `id` comme identifiant de requête.
pub async fn scope<F>(id: String, future: F) -> F::Output
where
    F: Future,
{
    REQUEST_ID.scope(id, future).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn current_retourne_none_hors_requete() {
        assert_eq!(current(), None);
    }

    #[tokio::test]
    async fn current_retourne_l_identifiant_du_scope() {
        let vu = scope("01JQ3F8K2P".to_string(), async { current() }).await;

        assert_eq!(vu.as_deref(), Some("01JQ3F8K2P"));
    }
}
