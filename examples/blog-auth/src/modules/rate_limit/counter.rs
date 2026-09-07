// Le compteur vit dans le processus : le projet ne porte pas le fragment `redis`, et
// une file d'attente distribuée ne s'improvise pas sur une base de données.
//
// Conséquence à connaître avant de mettre plusieurs instances derrière un répartiteur :
// chacune compte pour elle, et la limite effective est multipliée par leur nombre.
// `rbs add redis` fait basculer ce fichier sur un compteur partagé.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Nombre d'entrées au-delà duquel les fenêtres échues sont balayées.
///
/// Le balayage est amorti sur les requêtes plutôt que confié à une tâche de fond : sans
/// lui, une adresse par requête suffirait à faire grossir la table indéfiniment.
const SEUIL_DE_BALAYAGE: usize = 10_000;

/// Le compteur à fenêtre fixe du projet, partagé par tous les handlers.
#[derive(Debug, Clone, Default)]
pub struct Counter {
    fenetres: Arc<Mutex<HashMap<String, Fenetre>>>,
}

/// Une fenêtre de comptage ouverte, et ce qu'elle a vu passer.
#[derive(Debug)]
struct Fenetre {
    count: u64,
    echeance: Instant,
}

impl Counter {
    /// Construit un compteur vide.
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self::default())
    }

    /// Compte une requête sur `key` et rend le total de la fenêtre en cours.
    pub async fn hit(&self, key: &str, window: Duration) -> anyhow::Result<u64> {
        let maintenant = Instant::now();

        // Un verrou empoisonné n'est pas une raison de refuser tout le trafic : la table
        // n'a pas d'invariant qu'une panique d'un autre thread aurait pu rompre.
        let mut fenetres = self
            .fenetres
            .lock()
            .unwrap_or_else(|empoisonne| empoisonne.into_inner());

        if fenetres.len() >= SEUIL_DE_BALAYAGE {
            fenetres.retain(|_, fenetre| fenetre.echeance > maintenant);
        }

        let fenetre = fenetres.entry(key.to_string()).or_insert(Fenetre {
            count: 0,
            echeance: maintenant + window,
        });

        if fenetre.echeance <= maintenant {
            *fenetre = Fenetre {
                count: 0,
                echeance: maintenant + window,
            };
        }

        fenetre.count += 1;

        Ok(fenetre.count)
    }
}
