//! Arrêt gracieux du processus.
//!
//! Le signal vit dans le noyau et non dans le projet engendré parce qu'il n'a aucune
//! raison de varier d'un projet à l'autre : écouter Ctrl-C et SIGTERM, dire aux tâches de
//! fond de s'arrêter, et les attendre. Le `main` engendré le tire de [`CoreState`], le
//! worker de la file et le ticker du calendrier s'y abonnent par le même état.
//!
//! [`CoreState`]: crate::CoreState

use std::future::Future;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

/// Le signal d'arrêt du processus, et les tâches de fond qu'il doit attendre.
///
/// Clonable à coût nul, et tous les clones sont le même signal : celui que `main`
/// déclenche est celui que le worker écoute.
#[derive(Debug, Clone, Default)]
pub struct Shutdown {
    token: CancellationToken,
    tasks: TaskTracker,
}

impl Shutdown {
    /// Un signal que personne n'a encore demandé, sans tâche à attendre.
    pub fn new() -> Self {
        Self::default()
    }

    /// Demande l'arrêt. Idempotent : la seconde demande ne fait rien.
    pub fn request(&self) {
        self.token.cancel();
        // Fermé ici et non dans `wait` : un tracker ouvert attend des tâches qui n'ont
        // pas encore été détachées, et le ferait sans fin.
        self.tasks.close();
    }

    /// L'arrêt a-t-il été demandé ?
    pub fn is_requested(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Se résout à la demande d'arrêt — tout de suite si elle est déjà faite.
    ///
    /// Fait pour `tokio::select!` : l'abandonner n'a aucun effet, et le redemander au
    /// tour suivant n'en a pas davantage.
    pub fn requested(&self) -> impl Future<Output = ()> + Send + 'static {
        self.token.clone().cancelled_owned()
    }

    /// Détache une tâche de fond que [`Shutdown::wait`] attendra.
    ///
    /// La tâche reçoit le signal par un clone de `self` : c'est à elle de l'écouter et
    /// de rendre la main, rien ne l'interrompt de force.
    pub fn spawn<F>(&self, task: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tasks.spawn(task)
    }

    /// Demande l'arrêt, si ce n'est pas fait, puis attend les tâches détachées par
    /// [`Shutdown::spawn`], au plus `timeout`.
    ///
    /// Rend le nombre de tâches encore en cours à l'échéance — zéro quand l'arrêt est
    /// propre. Celles qui restent meurent avec le processus, et le dire est tout ce qui
    /// peut être fait : une tâche qui ignore le signal ne s'interrompt pas de l'extérieur.
    pub async fn wait(&self, timeout: Duration) -> usize {
        self.request();

        if tokio::time::timeout(timeout, self.tasks.wait())
            .await
            .is_err()
        {
            let restantes = self.tasks.len();
            tracing::warn!(
                restantes,
                timeout_secs = timeout.as_secs(),
                "arrêt sans attendre la fin des tâches de fond"
            );
            return restantes;
        }

        0
    }

    /// Se résout au premier Ctrl-C ou SIGTERM, et demande l'arrêt.
    ///
    /// Fait pour `axum::serve(...).with_graceful_shutdown(...)` : le serveur cesse
    /// d'accepter à cet instant, et les tâches de fond apprennent l'arrêt au même
    /// instant, sans attendre que les connexions en vol soient drainées.
    pub fn on_signal(&self) -> impl Future<Output = ()> + Send + 'static {
        let shutdown = self.clone();

        async move {
            signal().await;
            tracing::info!("arrêt demandé");
            shutdown.request();
        }
    }
}

/// Se résout au premier Ctrl-C ou, hors Windows, SIGTERM — celui que `docker stop`,
/// systemd et Kubernetes envoient avant de tuer.
///
/// Les gestionnaires ne se posent qu'au premier `poll`. S'ils ne peuvent pas l'être, le
/// futur ne se résout jamais : le processus s'arrête alors comme s'il n'écoutait rien,
/// ce qu'il dit une fois dans le journal.
pub async fn signal() {
    match Listeners::new() {
        Ok(listeners) => listeners.wait().await,
        Err(error) => {
            tracing::error!(%error, "signaux d'arrêt inécoutables");
            std::future::pending().await
        }
    }
}

/// Les gestionnaires posés, séparés de leur attente : un test peut ainsi se signaler
/// lui-même sans risquer que SIGTERM n'arrive avant qu'ils ne soient en place.
struct Listeners {
    #[cfg(unix)]
    terminate: tokio::signal::unix::Signal,
}

impl Listeners {
    fn new() -> std::io::Result<Self> {
        Ok(Self {
            #[cfg(unix)]
            terminate: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?,
        })
    }

    async fn wait(mut self) {
        #[cfg(unix)]
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = self.terminate.recv() => {}
            }
        }

        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn a_task_that_listens_finishes_before_wait_returns() {
        let shutdown = Shutdown::new();
        let fini = Arc::new(AtomicBool::new(false));
        let temoin = Arc::clone(&fini);
        let ecoute = shutdown.clone();
        shutdown.spawn(async move {
            ecoute.requested().await;
            // Le sommeil sépare « a reçu le signal » de « a fini » : c'est la seconde que
            // `wait` promet d'attendre.
            tokio::time::sleep(Duration::from_millis(20)).await;
            temoin.store(true, Ordering::SeqCst);
        });

        assert_eq!(shutdown.wait(Duration::from_secs(5)).await, 0);
        assert!(
            fini.load(Ordering::SeqCst),
            "wait a rendu la main avant la fin de la tâche"
        );
    }

    #[tokio::test]
    async fn a_task_that_ignores_the_signal_is_counted_at_the_deadline() {
        let shutdown = Shutdown::new();
        shutdown.spawn(std::future::pending::<()>());

        assert_eq!(shutdown.wait(Duration::from_millis(50)).await, 1);
    }

    #[tokio::test]
    async fn requested_resolves_at_once_after_the_request_and_on_every_clone() {
        let shutdown = Shutdown::new();
        let clone = shutdown.clone();
        assert!(!clone.is_requested());

        shutdown.request();
        shutdown.request();

        assert!(clone.is_requested());
        tokio::time::timeout(Duration::from_millis(50), clone.requested())
            .await
            .expect("le signal doit être résolu dès la demande");
    }

    /// Le processus de test se signale lui-même : les gestionnaires sont posés avant
    /// l'envoi, faute de quoi SIGTERM tuerait le binaire de test.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_listener_resolves_when_the_process_receives_sigterm() {
        let ecoute = Listeners::new().expect("les gestionnaires doivent se poser");

        std::process::Command::new("kill")
            .args(["-TERM", &std::process::id().to_string()])
            .status()
            .expect("kill doit se lancer");

        tokio::time::timeout(Duration::from_secs(5), ecoute.wait())
            .await
            .expect("SIGTERM doit résoudre l'écoute");
    }
}
