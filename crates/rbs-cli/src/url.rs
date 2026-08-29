//! Décomposition d'une URL de connexion en ses parties.
//!
//! Un seul analyseur pour tout le CLI : `new` en tire les identifiants du compose qu'il
//! engendre, `dev` et `doctor` l'hôte et le port qu'ils sondent. Deux analyseurs
//! divergents feraient publier un port que l'application ne joint pas.

use crate::database::Database;

/// Une URL de connexion, décomposée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Connection {
    /// Utilisateur, vide si l'URL n'en porte pas.
    pub user: String,
    /// Mot de passe, vide si l'URL n'en porte pas.
    pub password: String,
    /// Hôte, tel qu'il est écrit.
    pub host: String,
    /// Port explicite, ou celui du moteur à défaut.
    pub port: u16,
    /// Nom de la base, sans la chaîne de requête.
    pub database: String,
}

impl Connection {
    /// L'hôte désigne-t-il la machine qui lance la commande ?
    ///
    /// C'est la question que pose `rbs new` avant d'engendrer un compose : monter une
    /// base locale pour un projet qui en interroge une distante serait pire que ne rien
    /// écrire.
    ///
    /// `new` ne l'appelle pas encore : cette tâche ne fait que rassembler la lecture,
    /// pas engendrer le compose qui s'en servira.
    #[allow(dead_code)]
    pub(crate) fn est_locale(&self) -> bool {
        matches!(self.host.as_str(), "localhost" | "127.0.0.1" | "::1")
    }
}

/// Décompose `url`, ou rend `None` si aucun moteur à serveur ne la reconnaît.
pub(crate) fn parse(url: &str) -> Option<Connection> {
    let (reste, moteur) = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))
        .map(|reste| (reste, Database::Postgres))
        .or_else(|| {
            url.strip_prefix("mysql://")
                .map(|reste| (reste, Database::Mysql))
        })?;

    // Le dernier `@` sépare : un mot de passe a le droit d'en contenir un.
    let (identifiants, apres) = match reste.rsplit_once('@') {
        Some((avant, apres)) => (avant, apres),
        None => ("", reste),
    };

    let (user, password) = match identifiants.split_once(':') {
        Some((user, password)) => (user, password),
        None => (identifiants, ""),
    };

    let autorite = apres
        .split(['/', '?'])
        .next()
        .filter(|autorite| !autorite.is_empty())?;

    let defaut = moteur.default_port()?;
    let (host, port) = match autorite.rsplit_once(':') {
        Some((host, port)) => (host, port.parse().ok()?),
        None => (autorite, defaut),
    };

    let database = apres
        .split_once('/')
        .map(|(_, apres_barre)| apres_barre)
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");

    Some(Connection {
        user: user.to_string(),
        password: password.to_string(),
        host: host.to_string(),
        port,
        database: database.to_string(),
    })
}

/// L'URL de la même base, vue de l'intérieur du compose.
///
/// L'hôte y est le service `db`, et le port celui que le conteneur écoute : celui que le
/// compose a publié ne concerne que la machine hôte.
///
/// Sans appelant hors des tests tant que `new` n'engendre pas encore de compose.
#[allow(dead_code)]
pub(crate) fn interne(connexion: &Connection, database: Database) -> String {
    let scheme = database.name();
    let port = database.default_port().unwrap_or(connexion.port);

    format!(
        "{scheme}://{}:{}@db:{port}/{}",
        connexion.user, connexion.password, connexion.database
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_postgres_url_yields_every_part() {
        let connexion = parse("postgres://rbs:secret@localhost:5432/demo").expect("URL valide");

        assert_eq!(connexion.user, "rbs");
        assert_eq!(connexion.password, "secret");
        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.port, 5432);
        assert_eq!(connexion.database, "demo");
    }

    /// `postgresql://` est ce que rendent pg_dump et la plupart des hébergeurs.
    #[test]
    fn the_long_postgres_scheme_is_accepted_too() {
        let connexion = parse("postgresql://rbs:secret@db.exemple:6543/prod").expect("URL valide");

        assert_eq!(connexion.host, "db.exemple");
        assert_eq!(connexion.port, 6543);
        assert_eq!(connexion.database, "prod");
    }

    #[test]
    fn a_missing_port_falls_back_to_the_engine_default() {
        assert_eq!(
            parse("postgres://localhost/demo").expect("URL valide").port,
            5432
        );
        assert_eq!(
            parse("mysql://localhost/demo").expect("URL valide").port,
            3306
        );
    }

    #[test]
    fn a_url_without_credentials_yields_empty_ones() {
        let connexion = parse("postgres://localhost:5432/demo").expect("URL valide");

        assert_eq!(connexion.user, "");
        assert_eq!(connexion.password, "");
    }

    /// Le dernier `@` sépare : un mot de passe a le droit d'en contenir un.
    #[test]
    fn an_at_sign_in_the_password_does_not_split_the_url() {
        let connexion = parse("postgres://rbs:p@ss@localhost:5432/demo").expect("URL valide");

        assert_eq!(connexion.user, "rbs");
        assert_eq!(connexion.password, "p@ss");
        assert_eq!(connexion.host, "localhost");
    }

    #[test]
    fn a_query_string_is_not_part_of_the_database_name() {
        let connexion =
            parse("postgres://rbs:rbs@localhost:5432/demo?sslmode=require").expect("URL valide");

        assert_eq!(connexion.database, "demo");
    }

    /// SQLite n'a ni hôte, ni port, ni identifiants : il n'y a rien à décomposer.
    #[test]
    fn a_serverless_url_is_not_a_connection() {
        assert!(parse("sqlite://demo.db?mode=rwc").is_none());
        assert!(parse("demo").is_none());
    }

    #[test]
    fn the_three_loopback_spellings_are_local() {
        for hote in ["localhost", "127.0.0.1", "::1"] {
            let url = format!("postgres://rbs:rbs@{hote}:5432/demo");
            assert!(parse(&url).expect("URL valide").est_locale(), "{hote}");
        }
    }

    #[test]
    fn a_remote_host_is_not_local() {
        let connexion = parse("postgres://rbs:rbs@db.prod.exemple:5432/demo").expect("URL valide");

        assert!(!connexion.est_locale());
    }

    /// Vue du compose, la base n'est plus sur l'hôte mais sur le service `db`, et le port
    /// est celui que le conteneur écoute — non celui qui a été publié.
    #[test]
    fn the_internal_url_targets_the_db_service_on_its_container_port() {
        let connexion = parse("postgres://rbs:secret@localhost:15432/demo").expect("URL valide");

        assert_eq!(
            interne(&connexion, Database::Postgres),
            "postgres://rbs:secret@db:5432/demo"
        );
    }
}
