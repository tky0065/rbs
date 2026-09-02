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

    // La requête ne fait partie ni de l'autorité ni du nom de la base, et l'autorité
    // s'arrête au premier `/`. Chercher les identifiants avant ces deux découpes ferait
    // couper l'URL au `@` d'un nom de base, qui emporterait avec lui le mot de passe et
    // l'hôte.
    let reste = reste.split('?').next()?;
    let (autorite_complete, database) = match reste.split_once('/') {
        Some((autorite, database)) => (autorite, database),
        None => (reste, ""),
    };

    // Le dernier `@` de l'autorité sépare : un mot de passe a le droit d'en contenir un.
    // Un `/` ou un `?` non encodé, en revanche, met fin à l'autorité — la RFC les veut
    // encodés, et sqlx comme libpq les exigent de même. L'URL rend alors `None` plutôt
    // qu'un hôte deviné : un hôte faux part dans un compose et dans une sonde réseau,
    // un `None` ne va nulle part.
    let (identifiants, autorite) = match autorite_complete.rsplit_once('@') {
        Some((avant, apres)) => (avant, apres),
        None => ("", autorite_complete),
    };

    let (user, password) = match identifiants.split_once(':') {
        Some((user, password)) => (user, password),
        None => (identifiants, ""),
    };

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

    // `postgres://:5432/demo` a bien une autorité, mais pas d'hôte : la découpe au `:`
    // accepte une partie gauche vide. Un hôte vide partirait dans le compose et dans la
    // sonde de `doctor` sans désigner de machine, quand un `None` déclenche
    // l'avertissement sur l'URL que rbs n'a pas lue.
    if host.is_empty() {
        return None;
    }

    Some(Connection {
        user: user.to_string(),
        password: password.to_string(),
        host: host.to_string(),
        port,
        database: database.to_string(),
    })
}

/// L'URL de la même base, vue de l'intérieur du compose — identifiants non substitués.
///
/// L'hôte y est le service `db`, et le port celui que le conteneur écoute : celui que le
/// compose a publié ne concerne que la machine hôte.
///
/// Les identifiants y sont *nommés* et non écrits : ce sont les mêmes clés que celles du
/// service `db`, et c'est Compose qui les substitue depuis le `.env` du projet. Le compose
/// est versionné ; une URL complète y publierait le mot de passe de la base, que les
/// variables du service ont justement cessé de porter.
///
/// `user` ne sert qu'à MySQL, dont l'image ne crée `MYSQL_USER` que pour un compte autre
/// que `root`. SQLite n'a pas d'identifiants à nommer : l'appelant garde son URL de volume.
pub(crate) fn interne(database: Database, user: &str) -> Option<String> {
    let scheme = database.name();
    let port = database.default_port()?;

    Some(match database {
        Database::Postgres => format!(
            "{scheme}://${{POSTGRES_USER}}:${{POSTGRES_PASSWORD}}@db:{port}/${{POSTGRES_DB}}"
        ),
        Database::Mysql if user == "root" => {
            format!("{scheme}://root:${{MYSQL_ROOT_PASSWORD}}@db:{port}/${{MYSQL_DATABASE}}")
        }
        Database::Mysql => {
            format!("{scheme}://${{MYSQL_USER}}:${{MYSQL_PASSWORD}}@db:{port}/${{MYSQL_DATABASE}}")
        }
        Database::Sqlite => return None,
    })
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
    /// est celui que le conteneur écoute — non celui qui a été publié. Les identifiants,
    /// eux, sont ceux que Compose interpole depuis le `.env`.
    #[test]
    fn the_internal_url_targets_the_db_service_on_its_container_port() {
        assert_eq!(
            interne(Database::Postgres, "rbs").as_deref(),
            Some("postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@db:5432/${POSTGRES_DB}")
        );
    }

    /// Le mot de passe du projet ne traverse pas cette URL : elle part dans un compose
    /// versionné, où il serait publié avec lui.
    #[test]
    fn the_internal_url_never_carries_a_password() {
        for (database, user) in [
            (Database::Postgres, "rbs"),
            (Database::Mysql, "root"),
            (Database::Mysql, "app"),
        ] {
            let url = interne(database, user).expect("le moteur a un serveur");
            assert!(!url.contains("secret"), "{url}");
        }
    }

    /// L'image MySQL ne crée `MYSQL_USER` que pour un compte autre que `root` : sous
    /// `root`, l'URL doit nommer `MYSQL_ROOT_PASSWORD`, seule clé que le `.env` porte.
    #[test]
    fn a_mysql_url_names_the_account_the_image_will_have_created() {
        assert_eq!(
            interne(Database::Mysql, "root").as_deref(),
            Some("mysql://root:${MYSQL_ROOT_PASSWORD}@db:3306/${MYSQL_DATABASE}")
        );
        assert_eq!(
            interne(Database::Mysql, "app").as_deref(),
            Some("mysql://${MYSQL_USER}:${MYSQL_PASSWORD}@db:3306/${MYSQL_DATABASE}")
        );
    }

    /// SQLite n'a ni identifiants ni service à nommer : l'appelant garde son URL de volume.
    #[test]
    fn a_serverless_engine_has_no_credentials_to_name() {
        assert_eq!(interne(Database::Sqlite, ""), None);
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

    /// La RFC veut `/` et `?` encodés dans un mot de passe, et sqlx comme libpq l'exigent
    /// de même : une URL qui ne les encode pas ne connecterait de toute façon pas. Le
    /// refus est franc plutôt que deviné — un hôte faux partirait dans un compose et dans
    /// une sonde réseau, là où un `None` ne va nulle part.
    #[test]
    fn an_unencoded_separator_in_the_password_is_refused_rather_than_guessed() {
        assert!(parse("postgres://rbs:sec/ret@localhost:5432/demo").is_none());
        assert!(parse("postgres://rbs:sec?ret@localhost:5432/demo").is_none());
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

    /// Une autorité portant un port explicite ressemble à un couple utilisateur/mot de
    /// passe, et c'est cette ressemblance qui a fait échouer trois découpes successives.
    #[test]
    fn an_explicit_port_is_not_mistaken_for_credentials() {
        let connexion = parse("postgres://localhost:5432/de@mo").expect("URL valide");

        assert_eq!(connexion.user, "");
        assert_eq!(connexion.password, "");
        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.port, 5432);
        assert_eq!(connexion.database, "de@mo");
    }

    #[test]
    fn a_bracketed_ipv6_authority_without_credentials_keeps_its_host() {
        let connexion = parse("postgres://[::1]:5432/de@mo").expect("URL valide");

        assert_eq!(connexion.host, "::1");
        assert_eq!(connexion.port, 5432);
        assert_eq!(connexion.database, "de@mo");
    }

    #[test]
    fn a_colon_in_the_database_name_is_not_a_credentials_separator() {
        let connexion = parse("postgres://localhost/de:mo@x").expect("URL valide");

        assert_eq!(connexion.user, "");
        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.database, "de:mo@x");
    }

    /// Une autorité réduite à son port n'a pas d'hôte : la décomposer rendrait un hôte
    /// vide, que le compose publierait et que la sonde de `doctor` interrogerait. C'est
    /// exactement le cas que guette l'avertissement de `rbs new` sur une URL non lue.
    #[test]
    fn an_authority_without_a_host_is_not_understood() {
        assert!(parse("postgres://:5432/demo").is_none());
        assert!(parse("postgres://rbs:secret@:5432/demo").is_none());
    }

    /// Des crochets vides ne portent pas davantage d'adresse : la branche IPv6 refuse au
    /// même titre que l'autre.
    #[test]
    fn an_empty_bracketed_host_is_not_understood() {
        assert!(parse("postgres://[]:5432/demo").is_none());
        assert!(parse("postgres://[]/demo").is_none());
    }

    #[test]
    fn an_at_sign_in_the_query_does_not_reach_the_authority() {
        let connexion = parse("postgres://localhost:5432/demo?a=b@c").expect("URL valide");

        assert_eq!(connexion.host, "localhost");
        assert_eq!(connexion.database, "demo");
    }
}
