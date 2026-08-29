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

    // L'autorité s'arrête au premier `/` ou `?`, et le dernier `@` qui la précède sépare
    // les identifiants : un `@` situé au-delà appartient au chemin.
    let delimiteur = reste.find(['/', '?']).unwrap_or(reste.len());
    let (identifiants, apres) = match reste[..delimiteur].rfind('@') {
        Some(fin) => (&reste[..fin], &reste[fin + 1..]),
        None => match premier_arobase_apres_identifiants(reste) {
            Some(fin) => (&reste[..fin], &reste[fin + 1..]),
            None => ("", reste),
        },
    };

    let (user, password) = match identifiants.split_once(':') {
        Some((user, password)) => (user, password),
        None => (identifiants, ""),
    };

    let fin_autorite = apres.find(['/', '?']).unwrap_or(apres.len());
    let autorite = &apres[..fin_autorite];
    let database = apres[fin_autorite..]
        .strip_prefix('/')
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");

    if autorite.is_empty() {
        return None;
    }

    let defaut = moteur.default_port()?;
    // Un hôte IPv6 s'écrit entre crochets, seule façon de distinguer les deux-points de
    // l'adresse de celui du port. Les crochets appartiennent à l'URL, pas à l'hôte : les
    // garder ferait échouer la résolution du nom autant que la comparaison à `::1`.
    let (host, port) = match autorite.strip_prefix('[') {
        Some(apres_crochet) => {
            let (adresse, apres) = apres_crochet.split_once(']')?;
            let port = match apres.strip_prefix(':') {
                Some(port) => port.parse().ok()?,
                None => defaut,
            };
            (adresse, port)
        }
        None => match autorite.rsplit_once(':') {
            Some((host, port)) => (host, port.parse().ok()?),
            None => (autorite, defaut),
        },
    };

    Some(Connection {
        user: user.to_string(),
        password: password.to_string(),
        host: host.to_string(),
        port,
        database: database.to_string(),
    })
}

/// Position du `@` qui suit un mot de passe portant un `/` ou un `?` non encodé.
///
/// La RFC veut ces deux caractères encodés, mais un mot de passe engendré en base64 en
/// porte un une fois sur deux, et refuser ces URL rendrait le CLI inutilisable avec la
/// moitié des bases hébergées. Deux garde-fous évitent de prendre pour des identifiants
/// un `@` qui n'en sépare aucun : c'est le **premier** `@` qui est retenu, non le dernier,
/// qui appartiendrait au nom de la base ; et ce qui le précède doit porter un `:`, faute
/// de quoi `postgres://localhost/de@mo` verrait son hôte tiré du nom de sa base.
///
/// Limite connue et délibérée : un mot de passe portant à la fois un `/` et un `@` non
/// encodés (`rbs:sec/r@ss@localhost/demo`) est mal découpé. Deux caractères réservés non
/// encodés dans un même mot de passe, c'est une URL qu'aucun outil ne lit correctement.
fn premier_arobase_apres_identifiants(reste: &str) -> Option<usize> {
    let position = reste.find('@')?;

    reste[..position].contains(':').then_some(position)
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

    /// Un nom de base portant un `@` faisait couper l'URL entière au dernier `@` :
    /// le mot de passe et l'hôte étaient alors tirés du chemin.
    #[test]
    fn an_at_sign_in_the_database_name_does_not_reach_the_credentials() {
        let connexion = parse("postgres://rbs:secret@localhost:5432/demo@x").expect("URL valide");

        assert_eq!(connexion.user, "rbs");
        assert_eq!(connexion.password, "secret");
        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.port, 5432);
        assert_eq!(connexion.database, "demo@x");
    }

    /// Les crochets appartiennent à l'URL et non à l'hôte : `doctor` les passe à la
    /// résolution de nom, qui les refuse, et la comparaison à la boucle locale échoue.
    #[test]
    fn a_bracketed_ipv6_host_loses_its_brackets() {
        let connexion = parse("postgres://rbs:secret@[::1]:5432/demo").expect("URL valide");

        assert_eq!(connexion.host, "::1");
        assert_eq!(connexion.port, 5432);
        assert!(connexion.est_locale());
    }

    #[test]
    fn a_bracketed_ipv6_host_without_a_port_falls_back_to_the_engine_default() {
        let connexion = parse("postgres://rbs:secret@[2001:db8::1]/demo").expect("URL valide");

        assert_eq!(connexion.host, "2001:db8::1");
        assert_eq!(connexion.port, 5432);
        assert!(!connexion.est_locale());
    }

    /// Un mot de passe engendré en base64 porte un `/` une fois sur deux. La RFC veut
    /// qu'il soit encodé ; refuser l'URL parce qu'il ne l'est pas rendrait le CLI
    /// inutilisable avec la moitié des bases hébergées.
    #[test]
    fn a_slash_in_the_password_does_not_end_the_authority() {
        let connexion = parse("postgres://rbs:sec/ret@localhost:5432/demo").expect("URL valide");

        assert_eq!(connexion.user, "rbs");
        assert_eq!(connexion.password, "sec/ret");
        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.port, 5432);
        assert_eq!(connexion.database, "demo");
    }

    #[test]
    fn a_question_mark_in_the_password_does_not_start_the_query() {
        let connexion = parse("postgres://rbs:sec?ret@localhost:5432/demo").expect("URL valide");

        assert_eq!(connexion.password, "sec?ret");
        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.database, "demo");
    }

    /// Aucun identifiant, un `@` dans le nom de la base : rien ne doit être pris pour des
    /// identifiants, et l'hôte ne doit pas être tiré du chemin.
    #[test]
    fn an_at_sign_in_the_database_of_a_url_without_credentials_stays_in_the_database() {
        let connexion = parse("postgres://localhost/de@mo").expect("URL valide");

        assert_eq!(connexion.user, "");
        assert_eq!(connexion.password, "");
        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.port, 5432);
        assert_eq!(connexion.database, "de@mo");
    }

    /// Les deux tolérances à la fois : un `/` non encodé dans le mot de passe et un `@`
    /// dans le nom de la base. Le repli doit retenir le premier `@`, non le dernier.
    #[test]
    fn a_slash_in_the_password_and_an_at_sign_in_the_database_both_land_right() {
        let connexion = parse("postgres://rbs:sec/ret@localhost:5432/demo@x").expect("URL valide");

        assert_eq!(connexion.user, "rbs");
        assert_eq!(connexion.password, "sec/ret");
        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.port, 5432);
        assert_eq!(connexion.database, "demo@x");
    }

    #[test]
    fn a_slash_in_the_password_survives_an_at_sign_in_the_query() {
        let connexion =
            parse("postgres://rbs:sec/ret@localhost:5432/demo?x=1@y").expect("URL valide");

        assert_eq!(connexion.password, "sec/ret");
        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.database, "demo");
    }
}
