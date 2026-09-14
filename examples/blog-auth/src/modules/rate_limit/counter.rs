// Le compteur vit dans le processus : le projet ne porte pas le fragment `redis`, et
// une file d'attente distribuée ne s'improvise pas sur une base de données.
//
// Conséquence à connaître avant de mettre plusieurs instances derrière un répartiteur :
// chacune compte pour elle, et la limite effective est multipliée par leur nombre.
// `rbs add redis` fait basculer ce fichier sur un compteur partagé.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Taille en deçà de laquelle la table n'est jamais balayée.
///
/// Le balayage est amorti sur les requêtes plutôt que confié à une tâche de fond : sans
/// lui, une adresse par requête suffirait à faire grossir la table indéfiniment.
pub(super) const SEUIL_DE_BALAYAGE: usize = 10_000;

/// Le compteur à fenêtre fixe du projet, partagé par tous les handlers.
#[derive(Debug, Clone, Default)]
pub struct Counter {
    table: Arc<Mutex<Table>>,
}

/// Les fenêtres ouvertes, et la taille à laquelle la table sera balayée.
#[derive(Debug)]
struct Table {
    fenetres: HashMap<String, Fenetre>,
    seuil: usize,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            fenetres: HashMap::new(),
            seuil: SEUIL_DE_BALAYAGE,
        }
    }
}

/// Une fenêtre de comptage ouverte, et ce qu'elle a vu passer.
#[derive(Debug)]
struct Fenetre {
    count: u64,
    echeance: Instant,
}

/// La taille à laquelle le prochain balayage aura lieu, après un balayage qui a laissé
/// `vivantes` fenêtres.
///
/// Rebalayer au même seuil ferait parcourir toute la table sous le verrou à chaque
/// requête dès que 10 000 clients sont actifs à la fois. En doublant, un balayage n'a
/// lieu qu'après autant d'insertions qu'il a gardé de fenêtres : son coût amorti reste
/// constant, et la table ne dépasse pas le double de ce que le dernier balayage a gardé.
pub(super) fn prochain_seuil(vivantes: usize) -> usize {
    SEUIL_DE_BALAYAGE.max(vivantes.saturating_mul(2))
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
        let mut table = self
            .table
            .lock()
            .unwrap_or_else(|empoisonne| empoisonne.into_inner());
        let Table { fenetres, seuil } = &mut *table;

        if fenetres.len() >= *seuil {
            fenetres.retain(|_, fenetre| fenetre.echeance > maintenant);
            *seuil = prochain_seuil(fenetres.len());
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
