//! Joignabilité de la base et version du serveur.
//!
//! Deux constats de nature différente. Le premier tient à une connexion TCP : immédiate,
//! elle n'exige rien du projet. Le second demande une requête, donc un client SQL — que
//! rbs n'embarque pas : il le demande au binaire de la crate `migration`, comme
//! `rbs migrate` le fait déjà.

use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::database::Database;
use crate::migrate;

use super::Check;

const TITRE: &str = "base";

/// Délai au-delà duquel l'hôte est tenu pour injoignable.
const DELAI: Duration = Duration::from_secs(3);

/// Vérifie que la base répond et qu'elle est assez récente.
pub(crate) fn check(root: &Path) -> Check {
    let variables = match migrate::project_variables(root) {
        Ok(variables) => variables,
        Err(error) => {
            return Check::failed(
                TITRE,
                error.to_string(),
                format!("renseignez {} dans le .env du projet", migrate::URL),
            );
        }
    };

    let url = match url(&variables) {
        Some(url) => url,
        None => {
            return Check::failed(
                TITRE,
                format!(
                    "{} n'est lisible ni dans le .env ni dans l'environnement",
                    migrate::URL
                ),
                format!("renseignez {} dans le .env du projet", migrate::URL),
            );
        }
    };

    // Un serveur qui répond ne prouve rien quand le pilote compilé ne sait pas parler son
    // protocole : l'écart se dit avant que le port soit sondé.
    if let Some(ecart) = ecart(root, &url) {
        return ecart;
    }

    let database = database_of(root);

    let ou = match joignable(database, &url) {
        Ok(ou) => ou,
        Err(echec) => return echec,
    };

    let (minimum, cause) = plancher(database);

    match version(root, &variables) {
        Ok(brut) => match parse_version(database, &brut) {
            Some(version) if version >= minimum => Check::ok(
                TITRE,
                format!("{database} {} répond sur {ou}", readable(version)),
            ),
            Some(version) => Check::failed(
                TITRE,
                format!(
                    "{database} {} sur {ou}, {} attendu au minimum",
                    readable(version),
                    readable(minimum)
                ),
                format!("{database} {} est exigée par {cause}", readable(minimum)),
            ),
            None => Check::failed(
                TITRE,
                format!("{ou} répond, mais sa version reste illisible : {brut}"),
                "vérifiez que `cargo run -p migration -- version` aboutit",
            ),
        },
        Err(detail) => Check::failed(
            TITRE,
            format!("{ou} répond, mais sa version reste inconnue : {detail}"),
            "vérifiez que `cargo run -p migration -- version` aboutit",
        ),
    }
}

/// L'écart entre le pilote que le projet compile et le moteur que son URL désigne.
///
/// Les deux valeurs sont nommées plutôt que leur conclusion : c'est l'une ou l'autre que
/// le lecteur aura à corriger, et « configuration invalide » le renverrait aux deux
/// fichiers pour savoir laquelle.
fn ecart(root: &Path, url: &str) -> Option<Check> {
    let compile = pilote(root)?;
    let vise = Database::TOUS
        .into_iter()
        .find(|moteur| moteur.accepte(url))?;

    if compile == vise {
        return None;
    }

    let scheme = crate::database::scheme_of(url)?;

    Some(Check::failed(
        TITRE,
        format!(
            "le manifeste compile `{}` et {} est une URL `{scheme}://`",
            compile.sea_orm_feature(),
            migrate::URL
        ),
        format!(
            "alignez les deux : la feature `{}` de sea-orm au manifeste, \
             ou une URL `{}://` dans le .env",
            vise.sea_orm_feature(),
            compile.schemes()[0]
        ),
    ))
}

/// Moteur que le manifeste compile, lu à la feature de `sea-orm`.
///
/// C'est elle que `sqlx` embarque, et donc le seul pilote dont le binaire disposera.
/// `[package.metadata.rbs].database` n'en est que le suivi, qu'une édition à la main
/// laisse derrière elle sans rien casser à la compilation.
fn pilote(root: &Path) -> Option<Database> {
    let source = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let manifest: toml_edit::DocumentMut = source.parse().ok()?;

    let features = manifest
        .get("dependencies")?
        .get("sea-orm")?
        .get("features")?
        .as_array()?;

    features
        .iter()
        .filter_map(|feature| feature.as_str())
        .find_map(|feature| {
            Database::TOUS
                .into_iter()
                .find(|moteur| moteur.sea_orm_feature() == feature)
        })
}

/// Moteur que le manifeste déclare, PostgreSQL à défaut de manifeste lisible.
fn database_of(root: &Path) -> Database {
    crate::metadata::read(&root.join("Cargo.toml"))
        .map(|metadata| metadata.database)
        .unwrap_or_default()
}

/// Dit où la base se trouve, ou pourquoi on ne l'atteint pas.
///
/// SQLite n'est pas sondé par le réseau : ce qui le rend joignable est un fichier
/// ouvrable, ou un répertoire où il puisse naître — `mode=rwc` le crée au démarrage.
fn joignable(database: Database, url: &str) -> Result<String, Check> {
    if database == Database::Sqlite {
        let Some(chemin) = chemin_sqlite(url) else {
            return Err(Check::failed(
                TITRE,
                format!("{} n'est pas une URL SQLite", migrate::URL),
                "attendu : sqlite://chemin/vers/base.db?mode=rwc",
            ));
        };

        let accueil = chemin
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty());
        if chemin.is_file() || accueil.is_none_or(Path::is_dir) {
            return Ok(chemin.display().to_string());
        }

        return Err(Check::failed(
            TITRE,
            format!(
                "{} n'existe pas et son répertoire non plus",
                chemin.display()
            ),
            "créez le répertoire, ou corrigez l'URL du .env",
        ));
    }

    let Some((hote, port)) = host_and_port(url) else {
        return Err(Check::failed(
            TITRE,
            format!("{} n'est pas une URL {database}", migrate::URL),
            "attendu : moteur://utilisateur:motdepasse@hote:port/base",
        ));
    };

    if !reachable(&hote, port) {
        return Err(Check::failed(
            TITRE,
            format!("rien ne répond sur {hote}:{port}"),
            format!("démarrez {database}, ou corrigez l'URL du .env"),
        ));
    }

    Ok(format!("{hote}:{port}"))
}

/// L'URL visée : celle du `.env`, ou celle que l'appelant a exportée.
///
/// `variables` a déjà été amputé de ce que l'environnement porte : l'y chercher d'abord,
/// puis dans l'environnement, couvre les deux provenances sans en préférer une à tort.
fn url(variables: &[(String, String)]) -> Option<String> {
    crate::dotenv::value(variables, migrate::URL)
        .map(str::to_string)
        .or_else(|| std::env::var(migrate::URL).ok())
}

/// Vrai si une connexion TCP aboutit dans le délai imparti.
pub(crate) fn reachable(hote: &str, port: u16) -> bool {
    let Ok(adresses) = (hote, port).to_socket_addrs() else {
        return false;
    };

    adresses
        .into_iter()
        .any(|adresse| TcpStream::connect_timeout(&adresse, DELAI).is_ok())
}

/// Demande sa version au binaire de la crate `migration`, telle qu'il la rend.
///
/// L'interprétation appartient à l'appelant : chaque moteur a sa forme, et seul lui sait
/// lequel il interroge.
fn version(root: &Path, variables: &[(String, String)]) -> Result<String, String> {
    let output = migrate::launch(root, "version", variables, true).map_err(|e| e.to_string())?;

    output
        .split_whitespace()
        .next_back()
        .map(str::to_owned)
        .ok_or_else(|| format!("réponse incomprise : {}", output.trim()))
}

/// Découpe une URL en hôte et port, quel que soit celui des deux moteurs à serveur.
pub(crate) fn host_and_port(url: &str) -> Option<(String, u16)> {
    crate::url::parse(url).map(|connexion| (connexion.host, connexion.port))
}

/// Une version comparable : majeure et mineure, quelle que soit la forme rendue.
type Version = (u32, u32);

/// Plancher du moteur, et la cause qui le fixe.
///
/// Chacun a la sienne, et aucune n'est un chiffre repris d'ailleurs : un plancher sans
/// cause est une règle que personne ne saura mettre à jour.
fn plancher(database: Database) -> (Version, &'static str) {
    match database {
        Database::Postgres => (
            (14, 0),
            "le support communautaire, les versions antérieures ne recevant plus de correctif de sécurité",
        ),
        Database::Mysql => (
            (8, 0),
            "`FOR UPDATE SKIP LOCKED`, dont le dépilage des jobs dépend",
        ),
        Database::Sqlite => (
            (3, 35),
            "`UPDATE … RETURNING`, dont le dépilage des jobs dépend",
        ),
    }
}

/// Lit la version que `migration -- version` a rendue, selon le moteur interrogé.
fn parse_version(database: Database, brut: &str) -> Option<Version> {
    if database == Database::Postgres {
        // `server_version_num` est un entier compact : 170004 pour 17.4.
        let numero: u32 = brut.parse().ok()?;
        return Some((numero / 10_000, numero % 10_000));
    }

    // « 8.4.0 », mais aussi « 8.0.36-0ubuntu0.22.04.1 » : les paquets de distribution
    // suffixent la version d'une révision qui ne dit rien du moteur.
    let mut morceaux = brut.split(['.', '-']);

    Some((
        morceaux.next()?.parse().ok()?,
        morceaux.next()?.parse().ok()?,
    ))
}

/// Le chemin du fichier que désigne une URL SQLite.
fn chemin_sqlite(url: &str) -> Option<PathBuf> {
    let reste = url.strip_prefix("sqlite://")?;
    let chemin = reste.split('?').next().filter(|c| !c.is_empty())?;

    Some(PathBuf::from(chemin))
}

/// Rend une version lisible : `(18, 1)` devient `18.1`.
fn readable(version: Version) -> String {
    format!("{}.{}", version.0, version.1)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::super::State;
    use super::*;

    /// Un projet visant `url`, sans passer par le binaire ni par cargo.
    fn project(url: &str) -> (TempDir, PathBuf) {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = crate::new::create(
            &crate::new::Options {
                name: "demo-api".to_string(),
                database_url: url.to_string(),
                database: Default::default(),
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        (parent, project.root)
    }

    #[test]
    fn the_host_and_the_port_read_from_the_url() {
        assert_eq!(
            host_and_port("postgres://rbs:rbs@localhost:55433/demo"),
            Some(("localhost".to_string(), 55433))
        );
    }

    #[test]
    fn sans_port_celui_de_postgresql_est_supposé() {
        assert_eq!(
            host_and_port("postgres://rbs:rbs@db.interne/demo"),
            Some(("db.interne".to_string(), 5432))
        );
    }

    #[test]
    fn an_at_sign_in_the_password_does_not_shift_the_host() {
        assert_eq!(
            host_and_port("postgres://rbs:p@ss@localhost:5432/demo"),
            Some(("localhost".to_string(), 5432))
        );
    }

    #[test]
    fn a_url_without_credentials_stays_readable() {
        assert_eq!(
            host_and_port("postgres://localhost/demo"),
            Some(("localhost".to_string(), 5432))
        );
    }

    #[test]
    fn a_mysql_url_reads_back_with_its_own_default_port() {
        assert_eq!(
            host_and_port("mysql://root:root@db.interne/demo"),
            Some(("db.interne".to_string(), 3306))
        );
    }

    /// SQLite n'a ni hôte ni port : le sonder par le réseau n'a pas de sens.
    #[test]
    fn a_sqlite_url_has_neither_host_nor_port() {
        assert_eq!(host_and_port("sqlite://demo.db?mode=rwc"), None);
    }

    #[test]
    fn the_version_number_renders_as_major_minor() {
        assert_eq!(readable((18, 1)), "18.1");
        assert_eq!(readable((3, 45)), "3.45");
    }

    /// Chaque moteur dit sa version dans sa propre forme.
    #[test]
    fn each_engine_version_reads_back_in_its_own_shape() {
        assert_eq!(
            parse_version(Database::Postgres, "170004"),
            Some((17, 4)),
            "`server_version_num` est un entier compact"
        );
        // Les paquets de distribution suffixent la version d'un numéro de révision.
        assert_eq!(
            parse_version(Database::Mysql, "8.0.36-0ubuntu0.22.04.1"),
            Some((8, 0))
        );
        assert_eq!(parse_version(Database::Mysql, "8.4.0"), Some((8, 4)));
        assert_eq!(parse_version(Database::Sqlite, "3.45.1"), Some((3, 45)));
        assert_eq!(parse_version(Database::Sqlite, "n'importe quoi"), None);
    }

    /// Chaque plancher a une cause, et elle est vérifiable.
    #[test]
    fn each_engine_carries_its_own_floor() {
        assert_eq!(plancher(Database::Postgres).0, (14, 0));
        // `FOR UPDATE SKIP LOCKED`, dont le dépilage des jobs dépend.
        assert_eq!(plancher(Database::Mysql).0, (8, 0));
        // `UPDATE … RETURNING`, dont le dépilage des jobs dépend.
        assert_eq!(plancher(Database::Sqlite).0, (3, 35));

        for moteur in Database::TOUS {
            assert!(
                !plancher(moteur).1.is_empty(),
                "{moteur} n'a pas de cause à son plancher"
            );
        }
    }

    #[test]
    fn a_sqlite_url_yields_the_file_it_designates() {
        assert_eq!(
            chemin_sqlite("sqlite:///var/lib/demo.db?mode=rwc"),
            Some(PathBuf::from("/var/lib/demo.db"))
        );
        assert_eq!(
            chemin_sqlite("sqlite://demo.db"),
            Some(PathBuf::from("demo.db"))
        );
        assert_eq!(chemin_sqlite("postgres://localhost/demo"), None);
    }

    /// Réécrit le `.env` du projet pour viser `url`.
    ///
    /// `new::create` refuse une URL étrangère au moteur : la contradiction ne peut
    /// naître que d'une édition postérieure, et c'est celle-là que le test rejoue.
    fn viser(root: &Path, url: &str) {
        std::fs::write(
            root.join(".env"),
            format!("RBS_ENV=development\n{}={url}\n", migrate::URL),
        )
        .expect("écriture du .env");
    }

    #[test]
    fn the_compiled_driver_is_read_from_the_sea_orm_feature() {
        let (_parent, root) = project("postgres://rbs:rbs@127.0.0.1:1/demo");

        assert_eq!(pilote(&root), Some(Database::Postgres));
    }

    #[test]
    fn a_driver_at_odds_with_the_url_names_both_values() {
        let (_parent, root) = project("postgres://rbs:rbs@127.0.0.1:1/demo");
        viser(&root, "mysql://root:root@127.0.0.1:1/demo");

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{}", check.detail);
        assert!(check.detail.contains("sqlx-postgres"), "{}", check.detail);
        assert!(check.detail.contains("mysql://"), "{}", check.detail);
    }

    /// Le remède nomme les deux corrections possibles, sans choisir pour le lecteur.
    #[test]
    fn the_remedy_offers_either_side_of_the_gap() {
        let (_parent, root) = project("postgres://rbs:rbs@127.0.0.1:1/demo");
        viser(&root, "mysql://root:root@127.0.0.1:1/demo");

        let remedy = check(&root).remedy.expect("un échec porte son remède");

        assert!(remedy.contains("sqlx-mysql"), "{remedy}");
        assert!(remedy.contains("postgres://"), "{remedy}");
    }

    /// Une URL en accord avec le pilote laisse le diagnostic aller jusqu'à la connexion.
    #[test]
    fn a_url_matching_the_driver_raises_no_gap() {
        let (_parent, root) = project("postgres://rbs:rbs@127.0.0.1:1/demo");

        assert!(ecart(&root, "postgresql://rbs@127.0.0.1:1/demo").is_none());
    }

    #[test]
    fn an_unreachable_database_is_reported_with_its_host_and_port() {
        // Port 1 : réservé, rien n'y écoute — le refus est immédiat et déterministe.
        let (_parent, root) = project("postgres://rbs:rbs@127.0.0.1:1/demo");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains("127.0.0.1:1"));
        assert!(check.remedy.is_some());
    }

    #[test]
    fn a_url_missing_from_env_is_reported_without_attempting_a_connection() {
        let (_parent, root) = project("postgres://rbs:rbs@127.0.0.1:1/demo");
        std::fs::write(root.join(".env"), "RBS_ENV=development\n").expect("écriture du .env");

        let check = check(&root);

        assert_eq!(check.state, State::Echec);
        assert!(check.detail.contains(migrate::URL));
    }
}
