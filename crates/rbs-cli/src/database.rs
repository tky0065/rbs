//! Ce qui varie d'un moteur de base à l'autre, rassemblé en un seul endroit.
//!
//! Sans cette table, chaque commande qui doit distinguer les moteurs redécoupe son propre
//! `match` sur une chaîne, et l'ajout d'un quatrième moteur devient une chasse au
//! `postgres` à travers les templates. `doctor` et le dépilage des jobs lisent d'ici.

use std::fmt;

/// Moteur de base de données sur lequel tourne un projet rbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Database {
    /// PostgreSQL, le défaut historique — et celui des projets créés avant ce choix.
    #[default]
    Postgres,
    /// MySQL 8, dont `FOR UPDATE SKIP LOCKED` est contemporain.
    Mysql,
    /// SQLite, sans serveur : ni compose, ni attente au démarrage.
    Sqlite,
}

impl Database {
    /// Les trois moteurs, dans l'ordre où les messages d'erreur les énumèrent.
    pub const TOUS: [Self; 3] = [Self::Postgres, Self::Mysql, Self::Sqlite];

    /// Nom du moteur, tel qu'il s'écrit au flag et dans `[package.metadata.rbs]`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::Mysql => "mysql",
            Self::Sqlite => "sqlite",
        }
    }

    /// Feature `sea-orm` — et `sea-orm-migration` — qui porte ce moteur.
    pub fn sea_orm_feature(self) -> &'static str {
        match self {
            Self::Postgres => "sqlx-postgres",
            Self::Mysql => "sqlx-mysql",
            Self::Sqlite => "sqlx-sqlite",
        }
    }

    /// Schémas d'URL que ce moteur reconnaît comme siens.
    pub fn schemes(self) -> &'static [&'static str] {
        match self {
            // SeaORM accepte les deux, et `postgresql://` est ce que rendent pg_dump et
            // la plupart des hébergeurs : n'en refuser qu'un piégerait un copier-coller.
            Self::Postgres => &["postgres", "postgresql"],
            Self::Mysql => &["mysql"],
            Self::Sqlite => &["sqlite"],
        }
    }

    /// URL de connexion proposée à défaut de `--database-url`.
    pub fn default_url(self, crate_name: &str) -> String {
        match self {
            Self::Postgres => format!("postgres://postgres:postgres@localhost:5432/{crate_name}"),
            Self::Mysql => format!("mysql://root:root@localhost:3306/{crate_name}"),
            // `mode=rwc` crée le fichier au premier démarrage : sans lui, SQLite exige
            // qu'il préexiste et `rbs migrate up` échoue sur un projet neuf.
            Self::Sqlite => format!("sqlite://{crate_name}.db?mode=rwc"),
        }
    }

    /// URL de connexion vue de l'intérieur du compose.
    ///
    /// L'hôte y est le service `db` et non `localhost`, et SQLite y pointe le volume que
    /// `migrate` et `api` se partagent.
    pub fn compose_url(self, crate_name: &str) -> String {
        match self {
            Self::Postgres => format!("postgres://postgres:postgres@db:5432/{crate_name}"),
            Self::Mysql => format!("mysql://root:root@db:3306/{crate_name}"),
            Self::Sqlite => format!("sqlite:///data/{crate_name}.db?mode=rwc"),
        }
    }

    /// Port que le serveur du moteur écoute, ou `None` pour un moteur qui n'en a pas.
    pub fn default_port(self) -> Option<u16> {
        match self {
            Self::Postgres => Some(5432),
            Self::Mysql => Some(3306),
            Self::Sqlite => None,
        }
    }

    /// Le moteur a-t-il un serveur à monter et à attendre ?
    ///
    /// Répond pour le service `db` du compose comme pour l'attente de `rbs dev`.
    pub fn a_un_serveur(self) -> bool {
        !matches!(self, Self::Sqlite)
    }

    /// Le moteur reconnaît-il cette URL comme sienne ?
    ///
    /// Une URL sans schéma est refusée : `--database-url mabase` ne désigne rien.
    pub fn accepte(self, url: &str) -> bool {
        scheme_of(url).is_some_and(|scheme| self.schemes().contains(&scheme))
    }

    /// Relit un moteur écrit dans `[package.metadata.rbs]`.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|engine| engine.name() == name)
    }
}

/// Le schéma d'une URL de connexion, ou `None` si elle n'en porte pas.
pub fn scheme_of(url: &str) -> Option<&str> {
    let scheme = url.split_once("://").map(|(scheme, _)| scheme)?;

    (!scheme.is_empty()).then_some(scheme)
}

impl fmt::Display for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_engine_carries_its_sea_orm_feature() {
        assert_eq!(Database::Postgres.sea_orm_feature(), "sqlx-postgres");
        assert_eq!(Database::Mysql.sea_orm_feature(), "sqlx-mysql");
        assert_eq!(Database::Sqlite.sea_orm_feature(), "sqlx-sqlite");
    }

    #[test]
    fn a_name_reads_back_to_its_engine_and_an_unknown_one_to_nothing() {
        for engine in [Database::Postgres, Database::Mysql, Database::Sqlite] {
            assert_eq!(Database::from_name(engine.name()), Some(engine));
        }

        assert_eq!(Database::from_name("oracle"), None);
    }

    #[test]
    fn the_default_engine_is_postgres() {
        assert_eq!(Database::default(), Database::Postgres);
    }

    #[test]
    fn the_default_url_names_the_project_and_the_engine() {
        assert_eq!(
            Database::Postgres.default_url("mon_api"),
            "postgres://postgres:postgres@localhost:5432/mon_api"
        );
        assert_eq!(
            Database::Mysql.default_url("mon_api"),
            "mysql://root:root@localhost:3306/mon_api"
        );
        assert_eq!(
            Database::Sqlite.default_url("mon_api"),
            "sqlite://mon_api.db?mode=rwc"
        );
    }

    #[test]
    fn an_engine_accepts_its_own_schemes_and_rejects_the_others() {
        assert!(Database::Postgres.accepte("postgres://postgres@localhost/api"));
        assert!(Database::Postgres.accepte("postgresql://postgres@localhost/api"));
        assert!(!Database::Postgres.accepte("mysql://root@localhost/api"));

        assert!(Database::Mysql.accepte("mysql://root@localhost:3306/api"));
        assert!(!Database::Mysql.accepte("postgres://postgres@localhost/api"));

        assert!(Database::Sqlite.accepte("sqlite://api.db?mode=rwc"));
        assert!(!Database::Sqlite.accepte("postgres://postgres@localhost/api"));
    }

    // L'URL vient d'un flag : `--database-url mabase` est une faute plausible, et la
    // laisser passer ferait échouer le projet à la connexion plutôt qu'à la création.
    #[test]
    fn an_url_without_a_scheme_belongs_to_no_engine() {
        for engine in [Database::Postgres, Database::Mysql, Database::Sqlite] {
            assert!(!engine.accepte("mabase"));
            assert!(!engine.accepte(""));
        }
    }

    // L'URL du compose n'est pas celle du `.env` : l'hôte y est le service, et le
    // fichier SQLite vit sur le volume que `migrate` et `api` se partagent.
    #[test]
    fn the_compose_url_names_the_service_rather_than_localhost() {
        assert_eq!(
            Database::Postgres.compose_url("mon_api"),
            "postgres://postgres:postgres@db:5432/mon_api"
        );
        assert_eq!(
            Database::Mysql.compose_url("mon_api"),
            "mysql://root:root@db:3306/mon_api"
        );
        assert_eq!(
            Database::Sqlite.compose_url("mon_api"),
            "sqlite:///data/mon_api.db?mode=rwc"
        );

        for engine in Database::TOUS {
            assert!(
                !engine.compose_url("mon_api").contains("localhost"),
                "{engine} pointe localhost depuis le compose"
            );
        }
    }

    #[test]
    fn only_sqlite_has_no_server_to_wait_for() {
        assert!(Database::Postgres.a_un_serveur());
        assert!(Database::Mysql.a_un_serveur());
        assert!(!Database::Sqlite.a_un_serveur());
    }
}
