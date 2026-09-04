//! Imprime le document OpenAPI du projet, sans démarrer de serveur.
//!
//! `rbs generate client` lit cette sortie. Elle sert aussi à figer le contrat en CI :
//! `cargo run --bin openapi > openapi.json` puis un `git diff` qui doit rester vide.

use utoipa::OpenApi;

fn main() -> Result<(), serde_json::Error> {
    // Le document passe par une liaison plutôt que d'être imprimé d'un trait : rustfmt
    // borne un appel de macro à 60 colonnes, et l'expression entière les dépasse pour tout
    // nom de projet — le fichier engendré échouerait au `cargo fmt --check` de sa propre CI.
    let document = blog_auth::openapi::ApiDoc::openapi().to_pretty_json()?;

    println!("{document}");

    Ok(())
}
