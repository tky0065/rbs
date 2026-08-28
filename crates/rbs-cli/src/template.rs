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

    /// Rend la template `source` avec `context`, et échoue si une variable manque.
    pub fn render<S: Serialize>(&self, source: &str, context: S) -> Result<String, Error> {
        self.environnement.render_str(source, context)
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
    fn a_template_containing_a_rust_format_renders_intact() {
        let source = r#"let ligne = format!("{{}}", valeur);"#;

        let rendered = Renderer::new()
            .render(source, context! {})
            .expect("le rendu doit réussir");

        assert_eq!(rendered, source);
    }

    #[test]
    fn the_trailing_newline_is_preserved() {
        let rendered = Renderer::new()
            .render("fn main() {}\n", context! {})
            .expect("le rendu doit réussir");

        assert_eq!(rendered, "fn main() {}\n");
    }

    #[test]
    fn a_github_actions_expression_survives_the_render_intact() {
        let source = "token: ${{ secrets.TOKEN }}";

        let rendered = Renderer::new()
            .render(source, context! {})
            .expect("le rendu doit réussir");

        assert_eq!(rendered, source);
    }

    #[test]
    fn a_missing_variable_fails_the_render() {
        let error = Renderer::new()
            .render("nom = {@ name @}", context! {})
            .expect_err("une variable absente ne doit pas rendre une chaîne vide");

        assert_eq!(error.kind(), ErrorKind::UndefinedError);
    }

    #[test]
    fn the_html_reserved_characters_are_not_escaped() {
        let rendered = Renderer::new()
            .render(
                "{@ borne @}",
                context! { borne => "T: Into<String> & Clone" },
            )
            .expect("le rendu doit réussir");

        assert_eq!(rendered, "T: Into<String> & Clone");
    }

    #[test]
    fn a_supplied_variable_is_substituted() {
        let rendered = Renderer::new()
            .render("name = \"{@ name @}\"", context! { name => "mon-api" })
            .expect("le rendu doit réussir");

        assert_eq!(rendered, "name = \"mon-api\"");
    }
}
