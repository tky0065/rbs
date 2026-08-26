use minijinja::syntax::SyntaxConfig;
use minijinja::{AutoEscape, Environment, Error, UndefinedBehavior};
use serde::Serialize;

/// Rend les templates de génération avec les réglages propres à rbs.
pub struct Renderer {
    environnement: Environment<'static>,
}

impl Renderer {
    /// Construit un moteur de rendu.
    pub fn new() -> Self {
        // Jinja et Rust se disputent `{{ }}` : une template contenant `format!("{{}}")`
        // ne se rend pas. Les autres candidats butent chacun sur un langage que rbs
        // génère — `${ }` sur les `${VAR}` de docker-compose et les `${{ secrets.X }}`
        // de GitHub Actions, `[[ ]]` sur les `[[bin]]` des Cargo.toml, `<< >>` sur les
        // heredocs `<<'EOF'`. Blocs et commentaires gardent leurs délimiteurs : `{% %}`
        // et `{# #}` n'apparaissent dans aucun des formats produits.
        let syntaxe = SyntaxConfig::builder()
            .variable_delimiters("{@", "@}")
            .build()
            .expect("délimiteurs constants et non ambigus");

        let mut environnement = Environment::new();
        environnement.set_syntax(syntaxe);
        // Une variable oubliée doit arrêter la génération plutôt que laisser un trou
        // silencieux dans un fichier que l'utilisateur ne relira pas.
        environnement.set_undefined_behavior(UndefinedBehavior::Strict);
        // On produit du Rust et du TOML, où `&` et `<` sont des caractères ordinaires.
        environnement.set_auto_escape_callback(|_| AutoEscape::None);
        // minijinja retire le retour à la ligne final, utile pour un gabarit HTML,
        // fâcheux pour un fichier source : rustfmt et Git le réclament tous les deux.
        environnement.set_keep_trailing_newline(true);

        Self { environnement }
    }

    /// Rend la template `source` avec `contexte`, et échoue si une variable manque.
    pub fn rendre<S: Serialize>(&self, source: &str, contexte: S) -> Result<String, Error> {
        self.environnement.render_str(source, contexte)
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minijinja::{ErrorKind, context};

    #[test]
    fn une_template_contenant_un_format_rust_se_rend_intacte() {
        let source = r#"let ligne = format!("{{}}", valeur);"#;

        let rendu = Renderer::new()
            .rendre(source, context! {})
            .expect("le rendu doit réussir");

        assert_eq!(rendu, source);
    }

    #[test]
    fn le_retour_a_la_ligne_final_est_preserve() {
        let rendu = Renderer::new()
            .rendre("fn main() {}\n", context! {})
            .expect("le rendu doit réussir");

        assert_eq!(rendu, "fn main() {}\n");
    }

    #[test]
    fn une_expression_github_actions_traverse_le_rendu_intacte() {
        let source = "token: ${{ secrets.TOKEN }}";

        let rendu = Renderer::new()
            .rendre(source, context! {})
            .expect("le rendu doit réussir");

        assert_eq!(rendu, source);
    }

    #[test]
    fn une_variable_non_fournie_fait_echouer_le_rendu() {
        let erreur = Renderer::new()
            .rendre("nom = {@ nom @}", context! {})
            .expect_err("une variable absente ne doit pas rendre une chaîne vide");

        assert_eq!(erreur.kind(), ErrorKind::UndefinedError);
    }

    #[test]
    fn les_caracteres_reserves_du_html_ne_sont_pas_echappes() {
        let rendu = Renderer::new()
            .rendre(
                "{@ borne @}",
                context! { borne => "T: Into<String> & Clone" },
            )
            .expect("le rendu doit réussir");

        assert_eq!(rendu, "T: Into<String> & Clone");
    }

    #[test]
    fn une_variable_fournie_est_substituee() {
        let rendu = Renderer::new()
            .rendre("name = \"{@ nom @}\"", context! { nom => "mon-api" })
            .expect("le rendu doit réussir");

        assert_eq!(rendu, "name = \"mon-api\"");
    }
}
