//! L'image PostgreSQL que le harnais démarre, et d'où sa version lui vient.
//!
//! Un seul point de décision pour les quatre démarreurs du dépôt : les trois des tests
//! d'intégration et celui du banc des générateurs. Une constante par fichier, comme il y en
//! avait trois, rend une matrice mensongère — un site oublié démarre la version qu'il a
//! écrite en dur pendant que la CI annonce en avoir éprouvé deux.
//!
//! Le fichier se partage par `#[path]` et non par un item public : `generate::bench` est un
//! module `#[cfg(test)]` de la bibliothèque, `tests/common` appartient à un autre crate, et
//! aucune visibilité ne relie ces deux mondes de compilation sans élargir l'API publique de
//! `rbs-cli` au bénéfice des seuls tests.

/// Nom et étiquette de l'image à démarrer.
///
/// La 18 par défaut : c'est ce que le `docker-compose.yml` engendré épingle, donc ce qu'un
/// projet rencontre. `RBS_TEST_PG=14` démarre le plancher que `rbs doctor` fait respecter,
/// la plus ancienne version encore corrigée côté sécurité.
///
/// La variable est lue **au démarrage du conteneur**, et non par `option_env!` qui la
/// figerait à la compilation : la matrice reconstruirait alors tout le harnais entre ses
/// deux branches, et un binaire de test déjà bâti mentirait sur la version qu'il démarre.
///
/// Les deux moitiés sont rendues en `String` : `GenericImage::new` réclame un seul type
/// pour son nom et son étiquette, et lui passer `&str` obligerait chaque appelant à figer
/// la version dans une liaison intermédiaire.
pub fn image() -> (String, String) {
    (
        "postgres".to_string(),
        std::env::var("RBS_TEST_PG").unwrap_or_else(|_| "18".to_string()),
    )
}
