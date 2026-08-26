//! Ce que les générateurs du lot partagent : le nom de la feature et ses champs.
//!
//! Une feature se nomme au pluriel — `users` — mais son entité, ses DTO et son service se
//! nomment au singulier — `User`, `CreateUser`. La dérivation est faite ici, une fois,
//! plutôt que dans chaque template.

use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

use super::champs::{Champ, en_pascal_case};

/// Une feature à générer, telle que la voient l'entité, les DTO et la migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Feature {
    /// Nom au pluriel, en snake_case : le module et la table le portent tel quel.
    pub nom: String,
    /// Champs déclarés dans `--fields`, sans `id` ni les horodatages.
    pub champs: Vec<Champ>,
}

impl Feature {
    pub(crate) fn nouvelle(nom: &str, champs: Vec<Champ>) -> Self {
        Self {
            nom: nom.to_string(),
            champs,
        }
    }

    /// Nom du module et de la table : le nom donné, inchangé.
    pub(crate) fn module(&self) -> &str {
        &self.nom
    }

    /// Nom de l'entité en PascalCase singulier : `blog_posts` donne `BlogPost`.
    pub(crate) fn entite(&self) -> String {
        en_pascal_case(&self.singulier())
    }

    /// Nom de l'enum `DeriveIden` de la migration : `blog_posts` donne `BlogPosts`.
    ///
    /// SeaORM tire le nom de la table de celui de l'enum, pas de sa variante `Table` :
    /// l'enum se nomme donc au pluriel, contrairement à l'entité.
    pub(crate) fn iden(&self) -> String {
        en_pascal_case(&self.nom)
    }

    /// Nom singulier en snake_case : `blog_posts` donne `blog_post`.
    pub(crate) fn singulier(&self) -> String {
        au_singulier(&self.nom)
    }
}

/// Sérialisé à la main, comme `Champ` : minijinja ne voit pas les méthodes Rust, et les
/// templates lisent `entite` comme elles lisent `module`.
impl Serialize for Feature {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut etat = serializer.serialize_struct("Feature", 6)?;
        etat.serialize_field("module", self.module())?;
        etat.serialize_field("table", self.module())?;
        etat.serialize_field("entite", &self.entite())?;
        etat.serialize_field("iden", &self.iden())?;
        etat.serialize_field("singulier", &self.singulier())?;
        etat.serialize_field("champs", &self.champs)?;
        etat.end()
    }
}

/// Singularise le dernier mot d'un nom en snake_case.
///
/// Quinze lignes d'anglais approximatif plutôt qu'une crate d'inflexion : ce qu'on
/// cherche est un nom de type Rust, que l'utilisateur relira et pourra corriger, pas une
/// forme grammaticale juste. Les cas irréguliers — `people`, `children` — sortent
/// inchangés, ce qui donne une entité `People` : lisible, et rectifiable à la main.
pub(crate) fn au_singulier(nom: &str) -> String {
    let (prefixe, dernier) = match nom.rfind('_') {
        Some(coupe) => nom.split_at(coupe + 1),
        None => ("", nom),
    };

    let singulier = if let Some(racine) = dernier.strip_suffix("ies") {
        // `ies` sur un mot d'une syllabe — `ties`, `pies` — ne vient pas d'un `y`.
        if racine.is_empty() {
            dernier.to_string()
        } else {
            format!("{racine}y")
        }
    } else if SIFFLANTES
        .iter()
        .any(|sifflante| dernier.ends_with(&format!("{sifflante}es")))
    {
        dernier[..dernier.len() - 2].to_string()
    } else if dernier.ends_with('s') && !FINALES_NON_PLURIELLES.iter().any(|f| dernier.ends_with(f))
    {
        dernier[..dernier.len() - 1].to_string()
    } else {
        dernier.to_string()
    };

    format!("{prefixe}{singulier}")
}

/// Consonnes après lesquelles le pluriel anglais s'écrit `es` et non `s`.
const SIFFLANTES: [&str; 5] = ["s", "x", "z", "ch", "sh"];

/// Terminaisons en `s` qui ne sont pas des marques de pluriel : `status`, `class`.
const FINALES_NON_PLURIELLES: [&str; 3] = ["ss", "us", "is"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_pluriel_regulier_perd_son_s() {
        assert_eq!(au_singulier("users"), "user");
        assert_eq!(au_singulier("articles"), "article");
        assert_eq!(au_singulier("comments"), "comment");
    }

    #[test]
    fn un_pluriel_en_ies_redevient_y() {
        assert_eq!(au_singulier("categories"), "category");
        assert_eq!(au_singulier("companies"), "company");
    }

    #[test]
    fn un_pluriel_en_es_apres_sifflante_perd_ses_deux_lettres() {
        assert_eq!(au_singulier("addresses"), "address");
        assert_eq!(au_singulier("boxes"), "box");
        assert_eq!(au_singulier("branches"), "branch");
        assert_eq!(au_singulier("dishes"), "dish");
    }

    #[test]
    fn un_nom_deja_singulier_traverse_intact() {
        assert_eq!(au_singulier("status"), "status");
        assert_eq!(au_singulier("person"), "person");
        assert_eq!(au_singulier("data"), "data");
    }

    #[test]
    fn seul_le_dernier_mot_est_singularise() {
        assert_eq!(au_singulier("blog_posts"), "blog_post");
        assert_eq!(au_singulier("users_categories"), "users_category");
    }

    #[test]
    fn l_entite_est_le_singulier_en_pascal_case() {
        let feature = Feature::nouvelle("blog_posts", Vec::new());

        assert_eq!(feature.entite(), "BlogPost");
        assert_eq!(feature.singulier(), "blog_post");
        assert_eq!(feature.module(), "blog_posts");
    }

    #[test]
    fn la_serialisation_expose_les_noms_derives_aux_templates() {
        let feature = Feature::nouvelle("users", Vec::new());
        let vue = serde_json::to_value(&feature).expect("la feature doit se sérialiser");

        assert_eq!(vue["module"], "users");
        assert_eq!(vue["table"], "users");
        assert_eq!(vue["entite"], "User");
        assert_eq!(vue["iden"], "Users");
        assert_eq!(vue["singulier"], "user");
        assert!(vue["champs"].is_array(), "les champs doivent être exposés");
    }
}
