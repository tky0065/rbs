//! Le watch de `rbs dev` : relancer le serveur, et le tuer pour de bon.
//!
//! Le point dur n'est ni le debounce ni le filtrage. C'est la coupure : `cargo run`
//! n'est pas le serveur, il en est le parent, et un `cargo run` tué seul laisse derrière
//! lui un binaire qui garde le port. Le geste qui emporte les deux n'est pas le même
//! d'une plateforme à l'autre — groupe de processus sous Unix, *Job Object* sous
//! Windows — et c'est ce que `SpawnOptions { grouped: true }` couvre.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use watchexec::command::{Command, Program, SpawnOptions};
use watchexec::error::RuntimeError;
use watchexec::filter::Filterer;
use watchexec::{Id, Watchexec};
use watchexec_events::{Event, Priority};
use watchexec_signals::Signal;

use super::Error;

/// Répertoires dont le contenu ne dit jamais rien du code du projet.
///
/// `target` est le seul qui compte vraiment : le surveiller ferait relancer le serveur
/// par la compilation même qu'il vient de déclencher.
const IGNORES: [&str; 5] = ["target", ".git", "node_modules", "storage", ".rbs"];

/// Extensions dont un changement justifie de relancer le serveur.
const SURVEILLEES: [&str; 4] = ["rs", "toml", "sql", "jinja"];

/// Le fichier sans extension qui compte quand même : il porte l'URL de la base.
const ENV: &str = ".env";

/// Temps laissé aux écritures d'un même enregistrement pour se regrouper.
const REGROUPEMENT: Duration = Duration::from_millis(100);

/// Signal envoyé au serveur avant de l'abattre, et délai qui lui est laissé.
///
/// Un serveur Axum qui reçoit `SIGTERM` ferme ses connexions ; passé le délai, le groupe
/// entier est tué sans autre forme de procès.
const GRACE: Duration = Duration::from_millis(500);

/// Vrai si un changement sur `path` doit relancer le serveur.
pub(crate) fn restarts(path: &Path, root: &Path) -> bool {
    if ignored(path, root) {
        return false;
    }

    let Some(name) = path.file_name() else {
        return false;
    };

    if name == ENV {
        return true;
    }

    path.extension().is_some_and(|extension| {
        SURVEILLEES
            .iter()
            .any(|surveillee| extension.eq_ignore_ascii_case(surveillee))
    })
}

/// Vrai si `path` tombe dans un répertoire dont rien ne doit remonter.
///
/// Le chemin est jugé relativement à la racine, faute de quoi un projet posé sous
/// `~/target/demo` serait muet de bout en bout. Un chemin étranger à la racine est jugé
/// tel quel plutôt qu'écarté : `check_dir` s'en sert pour décider quoi surveiller, et
/// écarter par défaut y couperait le watch en silence.
fn ignored(path: &Path, root: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);

    relative.components().any(|composant| {
        IGNORES
            .iter()
            .any(|ignore| composant.as_os_str() == *ignore)
    })
}

/// La commande du serveur, telle que le watch la relance.
///
/// `grouped` est la ligne qui porte le critère : sans elle, `Job::stop()` n'abat que
/// `cargo`, et le binaire qu'il a lancé garde le port jusqu'au prochain redémarrage.
pub(crate) fn command(program: Program) -> Arc<Command> {
    Arc::new(Command {
        program,
        options: SpawnOptions {
            grouped: true,
            ..Default::default()
        },
    })
}

/// Le serveur du projet.
///
/// Ni racine ni environnement ici : `Program::Exec` n'en porte pas, ils se posent sur la
/// commande au moment du spawn.
fn server() -> Program {
    Program::Exec {
        prog: "cargo".into(),
        args: vec!["run".into()],
    }
}

/// Surveille le projet et relance son serveur à chaque changement utile.
pub(crate) fn run(root: &Path, variables: &[(String, String)]) -> Result<(), Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|source| Error::Watch(source.to_string()))?;

    runtime.block_on(supervise(root.to_path_buf(), variables.to_vec()))
}

/// La boucle du watch, une fois le runtime en place.
async fn supervise(root: PathBuf, variables: Vec<(String, String)>) -> Result<(), Error> {
    let commande = command(server());
    let racine = root.clone();

    // Un identifiant stable, sinon chaque événement créerait un serveur de plus au lieu
    // de relancer celui qui tourne.
    let serveur = Id::default();

    let wx = Watchexec::new(move |mut action| {
        if action.signals().next().is_some() {
            action.quit_gracefully(Signal::Terminate, GRACE);
            return action;
        }

        let job = action.get_or_create_job(serveur, || commande.clone());

        let repertoire = racine.clone();
        let environnement = variables.clone();
        job.set_spawn_hook(move |command, _| {
            let commande = command.command_mut();
            commande.current_dir(&repertoire);
            for (cle, valeur) in &environnement {
                commande.env(cle, valeur);
            }
        });

        let touches: Vec<PathBuf> = action.paths().map(|(path, _)| path.to_path_buf()).collect();
        if touches.is_empty() {
            job.start();
        } else {
            crate::ui::info(&cause(&touches, &racine));
            job.restart_with_signal(Signal::Terminate, GRACE);
        }

        action
    })
    .map_err(|error| Error::Watch(error.to_string()))?;

    wx.config.pathset([root.clone()]);
    wx.config.throttle(REGROUPEMENT);
    wx.config.filterer(Sources { root });

    wx.send_event(Event::default(), Priority::Urgent)
        .await
        .map_err(|error| Error::Watch(error.to_string()))?;

    wx.main()
        .await
        .map_err(|error| Error::Watch(error.to_string()))?
        .map_err(|error| Error::Watch(error.to_string()))?;

    Ok(())
}

/// Ce que le watch accepte de regarder, et ce qu'il accepte d'en retenir.
#[derive(Debug)]
struct Sources {
    /// Racine du projet, seule référence des chemins jugés.
    root: PathBuf,
}

impl Filterer for Sources {
    fn check_dir(&self, path: &Path) -> Result<bool, RuntimeError> {
        Ok(!ignored(path, &self.root))
    }

    fn check_event(&self, event: &Event, _priority: Priority) -> Result<bool, RuntimeError> {
        // Un événement sans chemin est un signal ou un réveil interne : le filtre des
        // fichiers n'a rien à en dire, et l'écarter couperait l'arrêt propre.
        if event.paths().next().is_none() {
            return Ok(true);
        }

        Ok(event.paths().any(|(path, _)| restarts(path, &self.root)))
    }
}

/// La ligne qui annonce un redémarrage, d'après les chemins qui l'ont déclenché.
///
/// Sans elle, rien à l'écran ne distingue un redémarrage voulu d'un serveur qui vient de
/// mourir de lui-même : le serveur se taisait, repartait, et l'utilisateur ne savait pas
/// lequel des deux il regardait.
///
/// Le regroupement rend plusieurs chemins d'un coup — un `cargo fmt` en touche vingt — et
/// seul le premier est nommé, les autres comptés.
fn cause(touches: &[PathBuf], racine: &Path) -> String {
    let Some(premier) = touches.first() else {
        return "redémarrage".to_string();
    };

    // Le chemin relatif si la racine le porte, entier sinon : un chemin hors du projet
    // est assez inattendu pour mériter d'être lu en entier.
    let nom = premier
        .strip_prefix(racine)
        .unwrap_or(premier)
        .display()
        .to_string();

    match touches.len() {
        1 => format!("redémarrage : {nom}"),
        compte => {
            let pluriel = if compte > 2 { "s" } else { "" };
            format!("redémarrage : {nom} et {} autre{pluriel}", compte - 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::time::Instant;

    use tempfile::TempDir;
    use watchexec::job::start_job;

    use super::*;

    /// Un redémarrage doit dire ce qui l'a causé.
    ///
    /// Sans cela, rien ne distingue à l'écran un redémarrage voulu d'un serveur qui
    /// vient de mourir tout seul.
    #[test]
    fn the_restart_names_the_file_that_caused_it() {
        let racine = Path::new("/projets/demo");

        assert_eq!(
            cause(&[racine.join("src/main.rs")], racine),
            "redémarrage : src/main.rs"
        );
    }

    /// Le regroupement rend plusieurs chemins d'un coup : le premier est nommé, les
    /// autres comptés — les énumérer tous noierait la ligne à chaque `cargo fmt`.
    #[test]
    fn several_files_are_counted_after_the_first() {
        let racine = Path::new("/projets/demo");
        let touches = [
            racine.join("src/main.rs"),
            racine.join("src/router.rs"),
            racine.join("src/state.rs"),
        ];

        assert_eq!(
            cause(&touches, racine),
            "redémarrage : src/main.rs et 2 autres"
        );
    }

    /// Un chemin hors de la racine se dit en entier plutôt que d'être tu.
    #[test]
    fn a_path_outside_the_root_keeps_its_whole_form() {
        assert_eq!(
            cause(
                &[PathBuf::from("/ailleurs/x.rs")],
                Path::new("/projets/demo")
            ),
            "redémarrage : /ailleurs/x.rs"
        );
    }

    /// Rôle que ce binaire de test tient quand il se relance lui-même.
    const ROLE: &str = "RBS_DEV_ROLE";

    /// Fichier où le petit-fils dépose le port qu'il a obtenu.
    const FICHIER: &str = "RBS_DEV_PORT_FILE";

    /// Nom exact du test qui sert de processus d'essai.
    const ESSAI: &str = "dev::watch::tests::the_test_binary_stands_in_for_a_server";

    /// Au-delà, un rôle s'arrête de lui-même : une preuve qui échoue ne doit pas laisser
    /// de processus derrière elle.
    const SURVIE: Duration = Duration::from_secs(30);

    /// Délai laissé au port pour s'ouvrir, puis pour se libérer.
    const DELAI: Duration = Duration::from_secs(10);

    /// Le processus d'essai, dans l'un ou l'autre de ses deux rôles.
    ///
    /// Sans `RBS_DEV_ROLE`, ce test ne fait rien : c'est un point d'entrée que le test de
    /// coupure relance, non une vérification. Un binaire dédié serait parti dans le
    /// paquet publié, et aucun interpréteur commun aux trois plateformes ne sait ouvrir
    /// un port.
    #[test]
    fn the_test_binary_stands_in_for_a_server() {
        match std::env::var(ROLE).as_deref() {
            // Le parent tient le rôle de `cargo run` : il lance le serveur et attend.
            Ok("parent") => {
                let mut enfant = std::process::Command::new(
                    std::env::current_exe().expect("le binaire de test se nomme"),
                )
                .args([ESSAI, "--exact", "--test-threads=1"])
                .env(ROLE, "enfant")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("le rôle enfant se lance");

                std::thread::sleep(SURVIE);
                let _ = enfant.kill();
                let _ = enfant.wait();
            }

            // L'enfant tient le rôle du binaire compilé : c'est lui qui garde le port.
            Ok("enfant") => {
                let ecoute = TcpListener::bind("127.0.0.1:0").expect("un port est libre");
                let port = ecoute
                    .local_addr()
                    .expect("le socket porte son adresse")
                    .port();

                let chemin = PathBuf::from(std::env::var(FICHIER).expect("le fichier est nommé"));
                let provisoire = chemin.with_extension("partiel");
                let mut fichier = std::fs::File::create(&provisoire).expect("le témoin s'écrit");
                write!(fichier, "{port}").expect("le port s'écrit");
                drop(fichier);
                std::fs::rename(&provisoire, &chemin).expect("le témoin se publie");

                std::thread::sleep(SURVIE);
                drop(ecoute);
            }

            _ => {}
        }
    }

    /// Le port que le témoin finit par porter.
    fn port_announced(temoin: &Path) -> u16 {
        let echeance = Instant::now() + DELAI;

        loop {
            if let Ok(contenu) = std::fs::read_to_string(temoin)
                && let Ok(port) = contenu.trim().parse()
            {
                return port;
            }

            assert!(
                Instant::now() < echeance,
                "le processus d'essai n'a jamais annoncé de port"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Vrai si `constat` finit par se vérifier dans le délai imparti.
    fn eventually(constat: impl Fn() -> bool) -> bool {
        let echeance = Instant::now() + DELAI;

        loop {
            if constat() {
                return true;
            }

            if Instant::now() >= echeance {
                return false;
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Vrai si quelque chose répond sur le port.
    fn served(port: u16) -> bool {
        let adresse = format!("127.0.0.1:{port}")
            .parse()
            .expect("l'adresse est valide");

        TcpStream::connect_timeout(&adresse, Duration::from_millis(250)).is_ok()
    }

    /// Vrai si le port peut être repris — la seule forme de « libre » qui compte, un
    /// redémarrage se soldant sinon par une erreur de liaison.
    ///
    /// Le constat ne se fait pas par une connexion refusée : un serveur qui n'accepte
    /// jamais finit par saturer sa file d'attente, et cesse alors de répondre sans avoir
    /// pour autant relâché quoi que ce soit.
    fn free(port: u16) -> bool {
        TcpListener::bind(format!("127.0.0.1:{port}")).is_ok()
    }

    /// La racine d'un projet imaginaire, écrite comme la plateforme l'écrit.
    fn root() -> PathBuf {
        PathBuf::from(if cfg!(windows) {
            r"C:\projets\demo-api"
        } else {
            "/projets/demo-api"
        })
    }

    #[test]
    fn a_source_file_restarts_the_server_and_a_build_artifact_does_not() {
        let root = root();

        assert!(
            restarts(&root.join("src").join("main.rs"), &root),
            "une source modifiée ne relance pas le serveur"
        );
        assert!(
            !restarts(&root.join("target").join("debug").join("demo-api"), &root),
            "un artefact de compilation relance le serveur"
        );
        // Le cas qui boucle : la compilation écrit des `.rs` sous `target/`, et les
        // retenir ferait relancer le serveur par le redémarrage qui vient d'avoir lieu.
        assert!(
            !restarts(
                &root
                    .join("target")
                    .join("debug")
                    .join("build")
                    .join("demo")
                    .join("out")
                    .join("genere.rs"),
                &root
            ),
            "un .rs engendré sous target/ relance le serveur"
        );
    }

    #[test]
    fn a_project_living_under_a_directory_named_target_is_not_mute() {
        // Le chemin absolu porte `target`, la racine aussi : seul le relatif tranche.
        let root = root().join("target").join("demo-api");

        assert!(
            restarts(&root.join("src").join("main.rs"), &root),
            "le projet est muet parce que son chemin traverse un `target`"
        );
    }

    #[test]
    fn target_is_not_even_watched() {
        let root = root();
        let sources = Sources { root: root.clone() };

        assert!(
            !sources
                .check_dir(&root.join("target"))
                .expect("le filtre répond"),
            "target/ est surveillé"
        );
        assert!(
            sources
                .check_dir(&root.join("src"))
                .expect("le filtre répond"),
            "src/ n'est pas surveillé"
        );
    }

    #[test]
    fn the_child_server_dies_with_its_group_and_frees_the_port() {
        let repertoire = TempDir::new().expect("répertoire temporaire créable");
        let temoin = repertoire.path().join("port");

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("un runtime tokio se construit");

        runtime.block_on(async {
            let commande = command(Program::Exec {
                prog: std::env::current_exe().expect("le binaire de test se nomme"),
                args: vec![ESSAI.into(), "--exact".into(), "--test-threads=1".into()],
            });

            let (job, tache) = start_job(commande);
            let chemin = temoin.clone();
            job.set_spawn_hook(move |command, _| {
                // Le rôle d'essai est un binaire de test : sa sortie se mêlerait à celle
                // de la preuve en cours.
                command
                    .command_mut()
                    .env(ROLE, "parent")
                    .env(FICHIER, &chemin)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
            })
            .await;
            job.start().await;

            let port = port_announced(&temoin);
            assert!(
                eventually(|| served(port)),
                "le petit-fils n'a jamais servi le port {port}"
            );

            job.stop().await;

            assert!(
                eventually(|| free(port)),
                "le petit-fils a survécu à la coupure : {port} n'est pas libre pour le \
                 redémarrage suivant"
            );

            job.delete_now().await;
            tache.abort();
        });
    }
}
