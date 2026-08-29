# Un compose dès la création, et un `--with` qui installe — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rbs new demo` écrit un `docker-compose.yml` aligné sur le `.env` du projet, et `--with` installe réellement les features qu'il nomme.

**Architecture:** Un seul compose par projet, engendré par le squelette avec la base seule, étendu par les fragments via une ancre en commentaire YAML `# <rbs:services>`. Les services de déploiement que `rbs add docker` y dépose portent `profiles: ["app"]`, ce qui laisse `rbs dev` monter les seuls services d'infrastructure. `rbs new` enchaîne le pipeline d'`add` sur chaque feature demandée, après l'écriture du squelette et avant `git init`.

**Tech Stack:** Rust 2024, minijinja (délimiteurs `{@ @}`), `include_dir`, `inquire`, `assert_cmd`, `testcontainers`, Docusaurus (docs bilingues).

**Spec:** `docs/superpowers/specs/2026-08-29-compose-et-features-a-la-creation-design.md`

## Global Constraints

- **Branche dédiée** : `compose-des-la-creation`. Jamais de commit sur `main`.
- **Conventional Commits**, sujet en français à l'impératif, sans majuscule ni point final. **Aucun identifiant de tâche**, aucun renvoi à ce plan, à `TODO.md` ou à un lot. **Jamais de `Co-Authored-By` ni de mention d'un assistant.** Corps portant le *pourquoi*, puis un intertitre `Vérifications :` avec les commandes lancées et leur résultat réel.
- **TDD** : le test est écrit et **vu échouer** avant l'implémentation. Un critère non prouvé se dit, il ne se coche pas.
- **Bloquant en CI** : `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check`.
- **`#![warn(missing_docs)]` sur `rbs-core`** — non touché ici. Dans `rbs-cli`, tout item `pub(crate)` nouveau porte un `///`.
- **Un commentaire explique le *pourquoi*, jamais le *quoi*.**
- **Délimiteurs minijinja alternatifs** : `{@ variable @}` pour les expressions, `{% %}` pour les blocs. `{{ }}` n'est pas interprété.
- **Moteur par défaut** : PostgreSQL. Image du compose : `postgres:18-alpine` (choix de défaut, non un plancher). Plancher de support : PostgreSQL 14, MySQL 8, SQLite 3.35.
- **Ports internes des conteneurs** : PostgreSQL 5432, MySQL 3306. Le port *publié* est celui de l'URL du projet.
- **Documentation bilingue dans le même commit**, `npm run parite` à 0 écart (24 paires).
- **Version visée** : `1.1.0` du workspace, avec `crates/rbs-cli/notes/1.1.0.md` obligatoire.

## Structure des fichiers

| Fichier | Responsabilité | Sort |
|---|---|---|
| `crates/rbs-cli/src/url.rs` | Décomposer une URL de connexion en parties, et dire si l'hôte est local | **créé** |
| `crates/rbs-cli/src/anchors.rs` | `Anchor` porte son marqueur de commentaire et son caractère optionnel ; `SERVICES` | modifié |
| `crates/rbs-cli/src/doctor/anchors.rs` | Ne réclame pas une ancre optionnelle dont le fichier est absent | modifié |
| `crates/rbs-cli/src/doctor/base.rs` | `host_and_port` délègue à `url.rs` | modifié |
| `crates/rbs-cli/src/database.rs` | `default_port` ; `compose_url` réservé à SQLite | modifié |
| `crates/rbs-cli/templates/project/docker-compose.yml.jinja` | Le compose du squelette : la base seule, et l'ancre | **créé** |
| `crates/rbs-cli/src/new.rs` | Filtre le compose, enrichit le contexte, installe les features | modifié |
| `crates/rbs-cli/src/prompts.rs` | Une seule liste de features, dérivée des fragments | modifié |
| `crates/rbs-cli/src/templates.rs` | `embedded_names` devient `pub(crate)` ; tests du compose déplacés | modifié |
| `crates/rbs-cli/src/manifest.rs` | `DeclaredFile` gagne `if_absent` | modifié |
| `crates/rbs-cli/src/plan/mod.rs` | `Builder::exists` | modifié |
| `crates/rbs-cli/src/add/installation.rs` | Respecte `if_absent` | modifié |
| `crates/rbs-cli/src/add/mod.rs` | Contexte enrichi depuis le `.env` du projet | modifié |
| `crates/rbs-cli/templates/features/docker/*` | Compose en repli + insertion sous profil `app` | modifié |
| `crates/rbs-cli/templates/features/{redis,mail}/feature.toml` | Déposent leur service | modifié |
| `crates/rbs-cli/src/dev/mod.rs` | Le compose ne dépend plus de la feature | modifié |
| `crates/rbs-cli/src/lib.rs` | Rend compte des features installées | modifié |
| `crates/rbs-cli/tests/integration_new.rs` | Le parcours nominal, compilé | modifié |

---

## Task 1 : `Anchor` porte son marqueur de commentaire

**Files:**
- Modify: `crates/rbs-cli/src/anchors.rs:8-32` (struct et `impl`), `:34-116` (les neuf constantes), `:192-210` (`groups`)
- Test: `crates/rbs-cli/src/anchors.rs` (module `tests` en fin de fichier)

**Interfaces:**
- Consumes: rien.
- Produces: `Anchor { name: &'static str, file: &'static str, comment: &'static str, optional: bool }`. `Anchor::opening()` rend `"{comment} <rbs:{name}>"`, `closing()` rend `"{comment} </rbs:{name}>"`.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans le module `tests` de `crates/rbs-cli/src/anchors.rs` :

```rust
    #[test]
    fn a_yaml_anchor_is_written_with_a_hash() {
        let compose = Anchor {
            name: "services",
            file: "docker-compose.yml",
            comment: "#",
            optional: true,
        };

        assert_eq!(compose.opening(), "# <rbs:services>");
        assert_eq!(compose.closing(), "# </rbs:services>");
        assert_eq!(compose.block(), "# <rbs:services>\n# </rbs:services>");
    }

    #[test]
    fn the_rust_anchors_keep_their_double_slash() {
        for anchor in ANCRES {
            if anchor.comment == "//" {
                assert_eq!(anchor.opening(), format!("// <rbs:{}>", anchor.name));
            }
        }
    }

    /// Un commentaire YAML qualifie le service qui le suit, comme `#[allow(…)]` qualifie
    /// le champ Rust qui le suit : les dédupliquer séparément laisserait l'un des deux
    /// orphelin.
    #[test]
    fn a_yaml_comment_stays_attached_to_the_line_below_it() {
        let compose = Anchor {
            name: "services",
            file: "docker-compose.yml",
            comment: "#",
            optional: true,
        };
        let source = "services:\n  # <rbs:services>\n  # </rbs:services>\n";
        let lines = vec![
            "# le cache du projet".to_string(),
            "redis:".to_string(),
        ];

        let apres = insert(source, compose, &lines).expect("l'ancre est présente");

        assert!(
            apres.contains("  # le cache du projet\n  redis:\n"),
            "le commentaire doit précéder son service :\n{apres}"
        );

        let deux_fois = insert(&apres, compose, &lines).expect("l'ancre est toujours là");
        assert_eq!(
            deux_fois.matches("redis:").count(),
            1,
            "une seconde insertion ne doit rien ajouter :\n{deux_fois}"
        );
    }
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib anchors::tests`
Expected : FAIL — `missing fields comment and optional in initializer of Anchor` (erreur de compilation).

- [ ] **Step 3 : ajouter les deux champs et les propager**

Dans `crates/rbs-cli/src/anchors.rs`, la struct :

```rust
pub(crate) struct Anchor {
    /// Nom tel qu'il paraît entre les chevrons : `features` pour `// <rbs:features>`.
    pub name: &'static str,
    /// Chemin du fichier porteur, relatif à la racine du projet.
    pub file: &'static str,
    /// Marqueur de commentaire du langage porteur : `//` en Rust, `#` en YAML.
    pub comment: &'static str,
    /// L'ancre peut légitimement manquer, son fichier porteur étant lui-même facultatif.
    ///
    /// `doctor` ne réclame pas une ancre optionnelle dont le fichier est absent : un
    /// projet SQLite n'a pas de compose, et n'a donc pas à passer pour incomplet.
    pub optional: bool,
}
```

Et l'`impl` :

```rust
    pub(crate) fn opening(&self) -> String {
        format!("{} <rbs:{}>", self.comment, self.name)
    }

    pub(crate) fn closing(&self) -> String {
        format!("{} </rbs:{}>", self.comment, self.name)
    }
```

Les neuf constantes existantes (`FEATURES`, `ROUTES`, `OPENAPI`, `MIGRATION_MODULES`, `MIGRATIONS`, `STATE_CHAMPS`, `STATE_INIT`, `STARTUP`, `SEEDS`) reçoivent chacune :

```rust
    comment: "//",
    optional: false,
```

Dans `groups` (`anchors.rs:192`), le prédicat :

```rust
        // `# ` : un commentaire YAML. `#[` et `//` : leurs homologues Rust. Les trois
        // qualifient la ligne suivante et ne valent pas pour eux-mêmes.
        let qualifie = matches!(line.trim_start().get(..2), Some("#[") | Some("//") | Some("# "));
```

- [ ] **Step 4 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib anchors::tests`
Expected : PASS, tous les tests du module.

- [ ] **Step 5 : la suite entière et les linters**

Run : `cargo test -p rbs-cli --lib && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/src/anchors.rs
git commit -F - <<'EOF'
refactor(ancres): fait porter à une ancre son marqueur de commentaire

Une ancre s'écrivait nécessairement `// <rbs:nom>`, ce qui la réservait aux
fichiers Rust. Le marqueur devient un champ, et le regroupement des lignes
insérées reconnaît le commentaire YAML au même titre que `//` et `#[` : un
commentaire qui qualifie la ligne suivante ne doit pas s'en détacher à la
déduplication.

Vérifications :
- cargo test -p rbs-cli --lib : N passed, 0 failed
- cargo clippy -p rbs-cli --all-targets -- -D warnings : propre
- cargo fmt --all --check : propre
EOF
```

---

## Task 2 : l'ancre `services`, optionnelle pour `doctor`

**Files:**
- Modify: `crates/rbs-cli/src/anchors.rs` (nouvelle constante, `ANCRES` de 9 à 10)
- Modify: `crates/rbs-cli/src/doctor/anchors.rs:16-48`
- Test: les modules `tests` des deux fichiers

**Interfaces:**
- Consumes: `Anchor` de la tâche 1.
- Produces: `anchors::SERVICES` ; `ANCRES: [Anchor; 10]`. `add::installation::anchor()` résout `anchor = "services"` sans changement, parcourant `ANCRES`.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `crates/rbs-cli/src/anchors.rs`, module `tests` :

```rust
    #[test]
    fn the_services_anchor_lives_in_the_compose_and_is_optional() {
        assert_eq!(SERVICES.file, "docker-compose.yml");
        assert_eq!(SERVICES.comment, "#");
        assert!(SERVICES.optional);
        assert!(ANCRES.contains(&SERVICES));
    }

    /// Une ancre optionnelle est l'exception : toutes les autres décrivent un fichier que
    /// le squelette écrit toujours, et leur absence est un défaut.
    #[test]
    fn only_the_services_anchor_is_optional() {
        let optionnelles: Vec<&str> = ANCRES
            .iter()
            .filter(|anchor| anchor.optional)
            .map(|anchor| anchor.name)
            .collect();

        assert_eq!(optionnelles, ["services"]);
    }
```

Dans `crates/rbs-cli/src/doctor/anchors.rs`, module `tests` :

```rust
    #[test]
    fn an_optional_anchor_whose_file_is_absent_is_not_missing() {
        let (_parent, root) = project();
        fs::remove_file(root.join("docker-compose.yml")).expect("le compose doit exister");

        let check = check(&root);

        assert_eq!(check.state, State::Ok, "{check:?}");
        assert!(
            check.detail.contains('9'),
            "seules les neuf ancres applicables comptent : {}",
            check.detail
        );
    }

    #[test]
    fn an_optional_anchor_removed_from_a_present_file_is_missing() {
        let (_parent, root) = project();
        remove(&root, "docker-compose.yml", "<rbs:services>");

        let check = check(&root);

        assert_eq!(check.state, State::Echec, "{check:?}");
        assert!(
            check.detail.contains("services manque dans docker-compose.yml"),
            "{}",
            check.detail
        );
    }
```

> Ces deux tests supposent que `new::create` écrit le compose : ils ne passeront qu'après la tâche 4. Les écrire ici et les voir échouer sur l'absence du fichier est le rouge attendu ; la tâche 4 les fait passer. **Marquer les deux `#[ignore = "le squelette n'écrit pas encore de compose"]` à la fin de cette tâche, et retirer l'attribut à la tâche 4.**

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib anchors`
Expected : FAIL — `cannot find value SERVICES in this scope`.

- [ ] **Step 3 : déclarer l'ancre et la rendre optionnelle pour `doctor`**

Dans `crates/rbs-cli/src/anchors.rs`, après `SEEDS` :

```rust
/// Services que les fragments ajoutent au compose du projet.
///
/// Optionnelle : un projet SQLite, un projet visant une base distante et tout projet créé
/// avant la 1.1.0 n'ont pas de compose, et n'ont donc pas cette ancre à porter.
pub(crate) const SERVICES: Anchor = Anchor {
    name: "services",
    file: "docker-compose.yml",
    comment: "#",
    optional: true,
};
```

`ANCRES` passe à `[Anchor; 10]` et gagne `SERVICES` en dernière position.

Dans `crates/rbs-cli/src/doctor/anchors.rs`, `check` :

```rust
pub(crate) fn check(root: &Path) -> Check {
    // Une ancre optionnelle dont le fichier n'existe pas n'est pas applicable : la
    // réclamer ferait passer pour incomplet un projet qui ne l'est pas.
    let applicables: Vec<&Anchor> = ANCRES
        .iter()
        .filter(|anchor| !anchor.optional || root.join(anchor.file).exists())
        .collect();

    let absentes: Vec<&&Anchor> = applicables.iter().filter(|a| !present(root, a)).collect();

    if absentes.is_empty() {
        return Check::ok(
            TITRE,
            format!("les {} points d'insertion sont en place", applicables.len()),
        );
    }
```

Le reste de la fonction est inchangé.

- [ ] **Step 4 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib anchors`
Expected : PASS. Les deux tests de `doctor::anchors` marqués `#[ignore]` s'affichent en `ignored`.

- [ ] **Step 5 : la suite entière et les linters**

Run : `cargo test -p rbs-cli --lib && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/src/anchors.rs crates/rbs-cli/src/doctor/anchors.rs
git commit -F - <<'EOF'
feat(ancres): ouvre un point d'insertion dans le compose du projet

Les fragments n'avaient aucun endroit où déposer un service : chacun annonçait
une adresse dans config/default.toml en laissant à l'utilisateur le soin de
monter ce qui y répondrait.

L'ancre est optionnelle, seule de son espèce : un projet SQLite ou visant une
base distante n'a pas de compose, et le diagnostic ne doit pas le tenir pour
incomplet à ce titre.

Vérifications :
- cargo test -p rbs-cli --lib : N passed, 0 failed, 2 ignored
- cargo clippy -p rbs-cli --all-targets -- -D warnings : propre
EOF
```

---

## Task 3 : un seul analyseur d'URL, qui rend toutes les parties

**Files:**
- Create: `crates/rbs-cli/src/url.rs`
- Modify: `crates/rbs-cli/src/lib.rs` (déclaration `mod url;`)
- Modify: `crates/rbs-cli/src/doctor/base.rs:20-21` (les deux constantes de port partent), `:248-273` (`host_and_port` délègue)
- Modify: `crates/rbs-cli/src/database.rs` (`default_port`, `compose_url` réservé à SQLite)
- Test: `crates/rbs-cli/src/url.rs` (module `tests`)

**Interfaces:**
- Consumes: `Database` de `database.rs`.
- Produces:
  - `pub(crate) struct Connection { pub user: String, pub password: String, pub host: String, pub port: u16, pub database: String }`
  - `pub(crate) fn parse(url: &str) -> Option<Connection>`
  - `Connection::est_locale(&self) -> bool`
  - `pub(crate) fn interne(connexion: &Connection, database: Database) -> String`
  - `Database::default_port(self) -> Option<u16>`
  - `doctor::base::host_and_port` conserve sa signature `(&str) -> Option<(String, u16)>`.

- [ ] **Step 1 : écrire les tests qui échouent**

Créer `crates/rbs-cli/src/url.rs` avec, pour l'instant, seulement son module de tests et les déclarations qu'il exerce :

```rust
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
        assert_eq!(parse("postgres://localhost/demo").expect("URL valide").port, 5432);
        assert_eq!(parse("mysql://localhost/demo").expect("URL valide").port, 3306);
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
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib url::tests`
Expected : FAIL — `cannot find function parse in this scope` (le module n'a pas encore de corps).

- [ ] **Step 3 : écrire le module**

En tête de `crates/rbs-cli/src/url.rs` :

```rust
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
pub(crate) fn interne(connexion: &Connection, database: Database) -> String {
    let scheme = database.name();
    let port = database.default_port().unwrap_or(connexion.port);

    format!(
        "{scheme}://{}:{}@db:{port}/{}",
        connexion.user, connexion.password, connexion.database
    )
}
```

Dans `crates/rbs-cli/src/database.rs`, ajouter à l'`impl Database` :

```rust
    /// Port que le serveur du moteur écoute, ou `None` pour un moteur qui n'en a pas.
    pub fn default_port(self) -> Option<u16> {
        match self {
            Self::Postgres => Some(5432),
            Self::Mysql => Some(3306),
            Self::Sqlite => None,
        }
    }
```

Dans `crates/rbs-cli/src/lib.rs`, déclarer le module à sa place alphabétique parmi les `mod` privés :

```rust
mod url;
```

Dans `crates/rbs-cli/src/doctor/base.rs`, supprimer `const PORT_POSTGRES` et `const PORT_MYSQL` (lignes 20-21) et remplacer le corps de `host_and_port` :

```rust
/// Découpe une URL en hôte et port, quel que soit celui des deux moteurs à serveur.
pub(crate) fn host_and_port(url: &str) -> Option<(String, u16)> {
    crate::url::parse(url).map(|connexion| (connexion.host, connexion.port))
}
```

Les tests existants de `doctor::base` qui citaient `PORT_POSTGRES` et `PORT_MYSQL` (`base.rs:371`, `:387`, `:395`) reçoivent les valeurs littérales `5432` et `3306`.

- [ ] **Step 4 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib url::tests && cargo test -p rbs-cli --lib doctor::base`
Expected : PASS des deux modules — les tests existants de `doctor::base` prouvent que la délégation n'a rien changé.

- [ ] **Step 5 : la suite entière et les linters**

Run : `cargo test -p rbs-cli --lib && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/src/url.rs crates/rbs-cli/src/lib.rs \
        crates/rbs-cli/src/doctor/base.rs crates/rbs-cli/src/database.rs
git commit -F - <<'EOF'
refactor(url): rassemble la lecture d'une URL de connexion en un module

Le diagnostic n'en tirait que l'hôte et le port, seuls dont il avait besoin. Le
compose engendré a besoin des identifiants et du nom de la base, et les lire
ailleurs ferait publier un port que l'application ne joint pas le jour où les
deux lectures divergeraient.

`doctor::base::host_and_port` y délègue et garde sa signature : ses tests
existants prouvent que la délégation ne change rien.

Vérifications :
- cargo test -p rbs-cli --lib : N passed, 0 failed
- cargo clippy -p rbs-cli --all-targets -- -D warnings : propre
EOF
```

---

## Task 4 : le compose du squelette

**Files:**
- Create: `crates/rbs-cli/templates/project/docker-compose.yml.jinja`
- Modify: `crates/rbs-cli/src/new.rs:254-285` (`render` : contexte et filtre)
- Modify: `crates/rbs-cli/src/lib.rs:186-189` (les prochains pas affichés)
- Modify: `crates/rbs-cli/src/doctor/anchors.rs` (retirer les deux `#[ignore]` de la tâche 2)
- Test: `crates/rbs-cli/src/new.rs` (module `tests`), `crates/rbs-cli/src/templates.rs` (module `tests`)

**Interfaces:**
- Consumes: `url::parse`, `Connection::est_locale`, `Database::default_port` (tâche 3) ; `anchors::SERVICES` (tâche 2).
- Produces: le contexte de rendu du squelette gagne `database_user`, `database_password`, `database_name`, `database_port`. Le squelette compte 17 fichiers pour un moteur à serveur sur hôte local, 16 sinon.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `crates/rbs-cli/src/new.rs`, module `tests` :

```rust
    /// Le compose n'est utile que s'il évite un `docker run` tapé à la main : c'est le
    /// seul critère qui décide de son écriture.
    #[test]
    fn a_local_postgres_project_gets_a_compose() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:secret@localhost:5432/demo".to_string(),
                database: Database::Postgres,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let compose = fs::read_to_string(project.root.join("docker-compose.yml"))
            .expect("le compose doit être écrit");

        assert!(compose.contains("POSTGRES_USER: rbs"), "{compose}");
        assert!(compose.contains("POSTGRES_PASSWORD: secret"), "{compose}");
        assert!(compose.contains("POSTGRES_DB: demo"), "{compose}");
        assert!(compose.contains("- \"5432:5432\""), "{compose}");
        assert!(compose.contains("# <rbs:services>"), "{compose}");
        assert!(compose.contains("# </rbs:services>"), "{compose}");
        assert_eq!(project.files, 17);
    }

    /// Le port publié est celui du .env, non 5432 en dur : sans quoi `cargo run` sur
    /// l'hôte joindrait un port que le conteneur n'expose pas.
    #[test]
    fn the_published_port_is_the_one_the_project_will_dial() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@localhost:15432/demo".to_string(),
                database: Database::Postgres,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        let compose = fs::read_to_string(project.root.join("docker-compose.yml"))
            .expect("le compose doit être écrit");

        assert!(compose.contains("- \"15432:5432\""), "{compose}");
    }

    #[test]
    fn a_sqlite_project_gets_no_compose() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "sqlite://demo.db?mode=rwc".to_string(),
                database: Database::Sqlite,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        assert!(!project.root.join("docker-compose.yml").exists());
        assert_eq!(project.files, 16);
    }

    #[test]
    fn a_remote_database_gets_no_compose() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@db.prod.exemple:5432/demo".to_string(),
                database: Database::Postgres,
                features: Vec::new(),
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        assert!(!project.root.join("docker-compose.yml").exists());
        assert_eq!(project.files, 16);
    }
```

Dans `crates/rbs-cli/src/templates.rs`, module `tests`, **remplacer** `the_docker_compose_publishes_only_the_api_port` (`:733`) et **déplacer** `the_docker_compose_targets_the_latest_stable_postgres` (`:868`) :

```rust
    /// Renversement assumé de la décision inverse : le compose ne publiait pas 5432
    /// parce que l'API l'atteignait par le réseau du compose. Le compose du squelette
    /// sert `cargo run` sur l'hôte, qui ne l'atteint que par un port publié.
    #[test]
    fn the_project_compose_publishes_the_database_port() {
        let source = read(&Path::new(RACINE_PROJET).join("docker-compose.yml.jinja"));

        assert!(
            source.contains("{@ database_port @}:5432"),
            "le compose doit publier le port du .env :\n{source}"
        );
    }

    #[test]
    fn the_project_compose_targets_the_latest_stable_postgres() {
        // Le code généré ne réclame plus la 18 depuis que le modèle pose lui-même son
        // identifiant : c'est un choix de défaut pour un projet neuf, non une exigence.
        // Le test l'épingle pour que l'image ne vieillisse pas en silence.
        let source = read(&Path::new(RACINE_PROJET).join("docker-compose.yml.jinja"));

        assert!(
            source.contains("postgres:18"),
            "le compose ne vise pas PostgreSQL 18 :\n{source}"
        );
    }
```

avec, près de `RACINE_FEATURES` (`templates.rs:581`) :

```rust
    const RACINE_PROJET: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/project");
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib new::tests templates::tests`
Expected : FAIL — `le compose doit être écrit: No such file or directory`, et `assert_eq!(project.files, 17)` obtenant 16.

- [ ] **Step 3 : écrire la template**

Créer `crates/rbs-cli/templates/project/docker-compose.yml.jinja` :

```jinja
name: {@ project_name @}

services:
  db:
{%- if database == "postgres" %}
    image: postgres:18-alpine
    environment:
      POSTGRES_USER: {@ database_user @}
      POSTGRES_PASSWORD: {@ database_password @}
      POSTGRES_DB: {@ database_name @}
    # Le port publié est celui du .env : c'est ce qui rend `docker compose up -d` suivi
    # de `cargo run` vrai sans recopier une valeur d'un fichier à l'autre. Le conflit
    # avec un PostgreSQL déjà installé sur la machine se règle en changeant les deux.
    ports:
      - "{@ database_port @}:5432"
    # PostgreSQL 18 place ses données sous /var/lib/postgresql/18/docker : c'est le
    # répertoire parent qui se monte, et non le /var/lib/postgresql/data des versions
    # précédentes, qui ne persisterait rien.
    volumes:
      - pgdata:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U {@ database_user @} -d {@ database_name @}"]
{%- else %}
    image: mysql:8
    environment:
      MYSQL_ROOT_PASSWORD: {@ database_password @}
      MYSQL_DATABASE: {@ database_name @}
{%- if database_user != "root" %}
      MYSQL_USER: {@ database_user @}
      MYSQL_PASSWORD: {@ database_password @}
{%- endif %}
    ports:
      - "{@ database_port @}:3306"
    volumes:
      - mysqldata:/var/lib/mysql
    # `mysqladmin ping` répond avant que la base du projet existe : le healthcheck
    # interroge donc le schéma, faute de quoi `migrate` démarrerait trop tôt.
    healthcheck:
      test:
        ["CMD-SHELL", "mysql -uroot -p{@ database_password @} -e 'use {@ database_name @}' 2>/dev/null"]
{%- endif %}
      interval: 2s
      timeout: 3s
      retries: 30

  # <rbs:services>
  # </rbs:services>

volumes:
{%- if database == "postgres" %}
  pgdata:
{%- else %}
  mysqldata:
{%- endif %}
```

- [ ] **Step 4 : enrichir le contexte et filtrer le fichier**

Dans `crates/rbs-cli/src/new.rs`, `render` :

```rust
fn render(options: &Options, dependency: &str) -> Result<Vec<(PathBuf, String)>, Error> {
    let mut files = Source::fresh(options.template_dir.as_deref())
        .files()
        .map_err(Error::Templates)?;

    let connexion = crate::url::parse(&options.database_url);
    if !compose_utile(options, connexion.as_ref()) {
        files.retain(|file| file.destination != Path::new(COMPOSE));
    }

    let renderer = Renderer::new();
    let context = context! {
        project_name => options.name.as_str(),
        crate_name => crate_name(&options.name),
        rbs_core_dep => dependency,
        rbs_version => env!("CARGO_PKG_VERSION"),
        database_url => options.database_url.as_str(),
        database => options.database.name(),
        sea_orm_feature => options.database.sea_orm_feature(),
        database_url_par_defaut => options.database.default_url(&crate_name(&options.name)),
        database_user => connexion.as_ref().map(|c| c.user.clone()).unwrap_or_default(),
        database_password => connexion.as_ref().map(|c| c.password.clone()).unwrap_or_default(),
        database_name => connexion
            .as_ref()
            .map(|c| c.database.clone())
            .unwrap_or_else(|| crate_name(&options.name)),
        database_port => connexion.as_ref().map(|c| c.port).unwrap_or_default(),
    };
```

Le reste de la fonction est inchangé. Ajouter en tête du fichier, près de `FEATURES_CONNUES` :

```rust
/// Nom du compose à la racine du projet, tel que la template le rend et que `rbs dev` le
/// cherche.
const COMPOSE: &str = "docker-compose.yml";
```

et, près des autres fonctions privées :

```rust
/// Le compose n'est écrit que s'il éviterait un `docker run` tapé à la main.
///
/// SQLite n'a rien à monter. Une base distante non plus : engendrer un service local que
/// `rbs dev` monterait pendant que l'application en interroge un autre serait pire que de
/// ne rien écrire.
fn compose_utile(options: &Options, connexion: Option<&crate::url::Connection>) -> bool {
    options.database.a_un_serveur() && connexion.is_some_and(crate::url::Connection::est_locale)
}
```

Dans `crates/rbs-cli/src/lib.rs`, les prochains pas affichés après la création — la ligne du compose n'apparaît que si le projet en a un :

```rust
    let compose = project.root.join("docker-compose.yml").exists();
    let demarrage = if compose {
        "\n  docker compose up -d   # la base du .env, montée\n  cargo run              # ou `rbs dev`, qui enchaîne les deux"
    } else {
        "\n  cargo run          # la base visée est dans .env"
    };
    ui::info(&format!("\n  cd {name}{demarrage}"));
```

Retirer les deux `#[ignore]` posés à la tâche 2 dans `crates/rbs-cli/src/doctor/anchors.rs`.

- [ ] **Step 5 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib new::tests templates::tests doctor::anchors`
Expected : PASS, `0 ignored`.

- [ ] **Step 6 : vérifier que le compose est du YAML valide**

Run :
```bash
cargo run -p rbs-cli -- new /tmp/verif-compose/demo --yes \
  --database-url postgres://rbs:secret@localhost:5432/demo \
  && (cd /tmp/verif-compose/demo && docker compose config >/dev/null && echo COMPOSE-VALIDE)
```
Expected : `COMPOSE-VALIDE`. Un compose syntaxiquement invalide ne se voit pas autrement.

- [ ] **Step 7 : la suite entière et les linters**

Run : `cargo test -p rbs-cli && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS. Les tests d'intégration qui comptent 16 fichiers échoueront ici : **les corriger à 17** dans le même commit, en vérifiant chaque compte au cas par cas (un projet SQLite en compte toujours 16).

- [ ] **Step 8 : commit**

```bash
git add crates/rbs-cli/templates/project/docker-compose.yml.jinja \
        crates/rbs-cli/src/new.rs crates/rbs-cli/src/lib.rs \
        crates/rbs-cli/src/templates.rs crates/rbs-cli/src/doctor/anchors.rs \
        crates/rbs-cli/tests
git commit -F - <<'EOF'
feat(new): engendre le compose de la base avec le projet

Un projet neuf réclamait un `docker run -p 5432:5432` recopié depuis la
documentation, alors que le CLI venait d'écrire l'URL de cette base dans le
.env. Les identifiants, le nom de la base et le port publié en sont désormais
tirés : `docker compose up -d && cargo run` est vrai sans qu'une valeur soit
recopiée d'un fichier à l'autre.

Le port publié renverse une décision inverse, prise pour un compose de
déploiement où l'API atteignait la base par le réseau du compose. Ici
l'application tourne sur l'hôte, qui ne l'atteint que par un port publié.

Rien n'est écrit là où rien ne servirait : SQLite n'a pas de serveur, et une
URL distante serait doublée par un service local monté à tort.

Vérifications :
- cargo test -p rbs-cli : N passed, 0 failed
- docker compose config sur un projet engendré : sortie 0
- cargo clippy -p rbs-cli --all-targets -- -D warnings : propre
EOF
```

---

## Task 5 : `rbs add docker` insère au lieu de déposer

**Files:**
- Modify: `crates/rbs-cli/src/manifest.rs:42-47` (`DeclaredFile`)
- Modify: `crates/rbs-cli/src/plan/mod.rs` (`Builder::exists`)
- Modify: `crates/rbs-cli/src/add/installation.rs:79-95` (`actions`), `:205-228` (`a_deposer`)
- Modify: `crates/rbs-cli/src/add/mod.rs:177-190` (contexte)
- Modify: `crates/rbs-cli/templates/features/docker/feature.toml`, `crates/rbs-cli/templates/features/docker/docker-compose.yml.jinja`
- Test: modules `tests` de `plan/mod.rs`, `add/installation.rs`, `add/mod.rs`, `templates.rs`

**Interfaces:**
- Consumes: `anchors::SERVICES` (tâche 2), `url::parse` et `url::interne` (tâche 3), la template du squelette (tâche 4).
- Produces: `manifest::DeclaredFile { source, destination, if_absent: bool }` (`if_absent` par défaut `false`) ; `plan::Builder::exists(&self, path: &str) -> Result<bool, plan::Error>`. Le contexte de rendu d'un fragment gagne `database_user`, `database_password`, `database_name`, `database_port`, et `database_url_compose` est dérivé du `.env` du projet.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `crates/rbs-cli/src/plan/mod.rs`, module `tests` :

```rust
    #[test]
    fn a_file_written_on_disk_is_reported_as_existing() {
        let root = TempDir::new().expect("répertoire temporaire créable");
        fs::write(root.path().join("present.yml"), "services:\n").expect("écriture possible");
        let builder = Builder::new(root.path());

        assert!(builder.exists("present.yml").expect("lecture possible"));
        assert!(!builder.exists("absent.yml").expect("lecture possible"));
    }
```

Dans `crates/rbs-cli/src/add/mod.rs`, module `tests` — **remplacer** les tests du compose déposé (`:342` et `:363`) :

```rust
    /// Trois états, trois comportements. Le premier : le projet a son compose, `add
    /// docker` n'y ajoute que ce qui manque.
    #[test]
    fn adding_docker_to_a_project_with_a_compose_inserts_its_services() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert_eq!(
            compose.matches("image: postgres:18-alpine").count(),
            1,
            "le service de base ne doit pas être doublé :\n{compose}"
        );
        assert!(compose.contains("profiles: [\"app\"]"), "{compose}");
        assert!(compose.contains("command: [\"migration\", \"up\"]"), "{compose}");
        assert!(
            !planned.files.iter().any(|f| f == "docker-compose.yml"),
            "le compose n'est pas déposé mais inséré : {:?}",
            planned.files
        );
    }

    /// Le deuxième : un projet créé avant la 1.1.0 n'a pas de compose. Le fragment lui en
    /// écrit un entier, ancre comprise, sans quoi il n'aurait aucun moyen d'en obtenir un.
    #[test]
    fn adding_docker_to_a_project_without_a_compose_writes_the_whole_file() {
        let (_parent, root) = project();
        fs::remove_file(root.join("docker-compose.yml")).expect("le compose doit exister");

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(compose.contains("image: postgres:18-alpine"), "{compose}");
        assert!(compose.contains("# <rbs:services>"), "{compose}");
        assert_eq!(
            compose.matches("profiles: [\"app\"]").count(),
            2,
            "api et migrate, une fois chacun :\n{compose}"
        );
        assert!(planned.files.iter().any(|f| f == "docker-compose.yml"));
    }

    /// Le troisième : un compose réécrit à la main a perdu son ancre. Le CLI n'écrit
    /// rien et affiche le bloc à recoller — la convention du projet.
    #[test]
    fn adding_docker_to_a_compose_without_its_anchor_refuses_and_shows_the_block() {
        let (_parent, root) = project();
        fs::write(root.join("docker-compose.yml"), "services:\n  db:\n    image: postgres\n")
            .expect("écriture possible");

        let error = plan_for(&options(&root, "docker")).expect_err("l'ancre manque");

        let message = error.to_string();
        assert!(message.contains("docker-compose.yml"), "{message}");
        assert!(
            error.remedy().is_some_and(|r| r.contains("# <rbs:services>")),
            "le bloc à coller doit être affiché : {error:?}"
        );
    }

    /// Le compose interne ne peut pas garder `postgres:postgres` en dur : la base du
    /// projet a les identifiants de son .env, et `migrate` ne s'y connecterait pas.
    #[test]
    fn the_internal_url_carries_the_credentials_of_the_project() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "docker")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(
            compose.contains("postgres://rbs:rbs@db:5432/demo_api"),
            "{compose}"
        );
    }
```

> `project()` dans ce module crée un projet avec l'URL `postgres://rbs:rbs@localhost:5432/demo_api` — la même que celle de `doctor/anchors.rs:65`. Vérifier que c'est le cas et l'aligner si besoin.

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib add:: plan::tests`
Expected : FAIL — `no method named exists found for struct Builder`, et le compose projeté contenant deux services `db`.

- [ ] **Step 3 : `if_absent` dans le manifeste et `Builder::exists`**

Dans `crates/rbs-cli/src/manifest.rs` :

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclaredFile {
    pub source: String,
    pub destination: String,
    /// Le fichier n'est déposé que si le projet ne le porte pas déjà.
    ///
    /// Réservé aux fichiers qu'un fragment étend d'ordinaire par une ancre : sans ce
    /// repli, un projet antérieur à l'ancre n'aurait aucun moyen d'obtenir le fichier.
    #[serde(default)]
    pub if_absent: bool,
}
```

Dans `crates/rbs-cli/src/plan/mod.rs`, sur `impl Builder` :

```rust
    /// Le projet porte-t-il déjà ce fichier, sur le disque ou par une action projetée ?
    pub fn exists(&self, path: &str) -> Result<bool, Error> {
        Ok(self.states(path)?.courant.is_some())
    }
```

Dans `crates/rbs-cli/src/add/installation.rs`, `a_deposer` rend le drapeau et `actions` le respecte :

```rust
fn a_deposer<'a>(fragment: &'a Fragment) -> Result<Vec<(String, &'a str, bool)>, Error> {
    if fragment.manifest.files.is_empty() {
        return Ok(fragment
            .templates
            .iter()
            .map(|template| {
                (
                    template.destination.to_string_lossy().into_owned(),
                    template.source.as_str(),
                    false,
                )
            })
            .collect());
    }

    fragment
        .manifest
        .files
        .iter()
        .map(|declare| {
            let source = template(fragment, &declare.source)?;
            Ok((declare.destination.clone(), source, declare.if_absent))
        })
        .collect()
}
```

et, dans `actions` :

```rust
    for (destination, source, if_absent) in a_deposer(fragment)? {
        if if_absent && builder.exists(&destination)? {
            continue;
        }

        let content = render(&renderer, fragment, source, &destination)?;

        builder.create(&destination, &content)?;
        deposes.push(destination);
    }
```

- [ ] **Step 4 : le contexte du fragment lit le `.env` du projet**

Dans `crates/rbs-cli/src/add/mod.rs`, entre la lecture du moteur et la construction du contexte :

```rust
    // L'URL du projet, non une valeur par défaut : le compose que le fragment engendre
    // doit se connecter à la base que le projet interroge, avec ses identifiants.
    let url = migrate::project_variables(&root)
        .ok()
        .and_then(|variables| {
            crate::dotenv::value(&variables, migrate::URL).map(str::to_string)
        })
        .unwrap_or_else(|| database.default_url(&crate_name));
    let connexion = crate::url::parse(&url);

    let context = context! {
        project_name => nom_projet.clone(),
        crate_name => crate_name.clone(),
        database => database.name(),
        database_a_un_serveur => database.a_un_serveur(),
        database_url_compose => match connexion.as_ref() {
            Some(connexion) => crate::url::interne(connexion, database),
            None => database.compose_url(&crate_name),
        },
        database_url_par_defaut => database.default_url(&crate_name),
        database_user => connexion.as_ref().map(|c| c.user.clone()).unwrap_or_default(),
        database_password => connexion.as_ref().map(|c| c.password.clone()).unwrap_or_default(),
        database_name => connexion
            .as_ref()
            .map(|c| c.database.clone())
            .unwrap_or_else(|| crate_name.clone()),
        database_port => connexion.as_ref().map(|c| c.port).unwrap_or_default(),
    };
```

Le module `tests` de `templates.rs:591` construit le même contexte pour prouver que toutes les templates se rendent : y ajouter les quatre variables avec des valeurs littérales (`"rbs"`, `"rbs"`, `"mon_api"`, `5432u16`).

- [ ] **Step 5 : le fragment `docker`**

`crates/rbs-cli/templates/features/docker/feature.toml` :

```toml
[feature]
description = "Dockerfile multi-étapes, .dockerignore et services de déploiement"

[[files]]
source      = "Dockerfile.jinja"
destination = "Dockerfile"

[[files]]
source      = ".dockerignore.jinja"
destination = ".dockerignore"

# Un projet créé avant la 1.1.0 n'a pas de compose : le fragment lui en écrit un entier,
# services de déploiement compris. Là où le projet en a un, l'insertion ci-dessous suffit,
# et sa déduplication rend l'opération sans effet sur le fichier tout juste écrit.
[[files]]
source      = "docker-compose.yml.jinja"
destination = "docker-compose.yml"
if_absent   = true

[[anchors]]
anchor  = "services"
content = """
# Les migrations tournent une fois, à part : l'API ne démarre pas sur un schéma absent,
# et n'a pas à porter cette responsabilité au démarrage.
migrate:
  profiles: ["app"]
  build: .
  command: ["migration", "up"]
  environment:
    RBS_DATABASE__URL: {@ database_url_compose @}
  restart: "no"
api:
  profiles: ["app"]
  build: .
  environment:
    # config/default.toml écoute sur 127.0.0.1, ce qui rend le conteneur injoignable
    # depuis l'hôte. L'environnement l'emporte sur le TOML.
    RBS_SERVER__HOST: 0.0.0.0
    RBS_DATABASE__URL: {@ database_url_compose @}
    RBS_LOG_FORMAT: json
    RUST_LOG: info
  ports:
    - "8080:8080"
  depends_on:
    migrate:
      condition: service_completed_successfully
"""
```

> Le profil `app` isole ces deux services : `docker compose up -d` nu monte l'infrastructure seule, ce que `rbs dev` attend. `docker compose --profile app up` monte l'ensemble.
>
> `depends_on: db` n'y figure pas : `migrate` et `api` ne connaissent pas le nom du service de base, qui n'existe pas dans un projet SQLite. La dépendance passe par `migrate`, qui échoue et bloque `api` si la base n'est pas prête.
>
> `add::installation::lines` retire les lignes vides : le bloc inséré n'en portera aucune. C'est délibéré — l'écrire avec des lignes vides donnerait un fichier différent de ce que le fragment déclare.

`crates/rbs-cli/templates/features/docker/docker-compose.yml.jinja` devient le repli, identique à la template du squelette (tâche 4) plus les deux services entre les balises de l'ancre, et la branche SQLite conservée pour le volume partagé. Le rendre depuis la template du squelette et y coller le contenu du `[[anchors]]` ci-dessus, indenté de deux espaces, entre `# <rbs:services>` et `# </rbs:services>`.

- [ ] **Step 6 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib`
Expected : PASS.

- [ ] **Step 7 : vérifier les trois états au compose réel**

Run :
```bash
rm -rf /tmp/verif-add && mkdir -p /tmp/verif-add && cd /tmp/verif-add
cargo run -p rbs-cli -- new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
cd demo && git add -A && git commit -qm base
cargo run -p rbs-cli -- add docker && docker compose config >/dev/null && echo ETAT-1-VALIDE
docker compose config --profiles
```
Expected : `ETAT-1-VALIDE`, et `--profiles` listant `app`. Répéter en supprimant le compose avant l'`add` (état 2), puis en le remplaçant par un fichier sans ancre (état 3 : le CLI n'écrit rien et affiche le bloc).

- [ ] **Step 8 : la suite entière et les linters**

Run : `cargo test -p rbs-cli && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS.

- [ ] **Step 9 : commit**

```bash
git add crates/rbs-cli/src/manifest.rs crates/rbs-cli/src/plan/mod.rs \
        crates/rbs-cli/src/add crates/rbs-cli/templates/features/docker \
        crates/rbs-cli/src/templates.rs
git commit -F - <<'EOF'
feat(add): fait de docker un fragment qui étend le compose du projet

Le fragment déposait un compose entier, ce qui écrasait celui que le projet
porte désormais. Il insère ses deux services de déploiement dans l'ancre du
fichier, sous un profil `app` : `docker compose up -d` nu monte
l'infrastructure seule, ce qui laisse `rbs dev` démarrer sans bâtir une image
d'API que le cargo watch de l'hôte rend inutile.

Le dépôt du fichier subsiste en repli, pour un projet antérieur à l'ancre qui
n'aurait aucun autre moyen d'en obtenir un.

L'URL interne du compose porte les identifiants du .env du projet, et non plus
un `postgres:postgres` en dur auquel la base engendrée refuserait la connexion.

Vérifications :
- cargo test -p rbs-cli : N passed, 0 failed
- docker compose config sur les trois états : sortie 0, profil `app` listé
- cargo clippy -p rbs-cli --all-targets -- -D warnings : propre
EOF
```

---

## Task 6 : `redis` et `mail` déposent leur service

**Files:**
- Modify: `crates/rbs-cli/templates/features/redis/feature.toml`
- Modify: `crates/rbs-cli/templates/features/mail/feature.toml`
- Test: `crates/rbs-cli/src/add/mod.rs` (module `tests`)

**Interfaces:**
- Consumes: l'ancre `services` (tâche 2), le compose du squelette (tâche 4), l'insertion sous ancre du fragment `docker` (tâche 5).
- Produces: rien de nouveau côté code.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `crates/rbs-cli/src/add/mod.rs`, module `tests` :

```rust
    /// Le fragment annonçait redis://127.0.0.1:6379 dans config/default.toml sans que
    /// rien y réponde. Le service le sert, et sans profil : c'est une dépendance de
    /// développement, que `rbs dev` doit monter.
    #[test]
    fn adding_redis_serves_the_url_its_config_announces() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "redis")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(compose.contains("redis:8-alpine"), "{compose}");
        assert!(compose.contains("- \"6379:6379\""), "{compose}");
        assert!(
            !compose.contains("profiles"),
            "un service de développement n'a pas de profil :\n{compose}"
        );
    }

    #[test]
    fn adding_mail_serves_the_smtp_port_its_config_announces() {
        let (_parent, root) = project();

        let planned = plan_for(&options(&root, "mail")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(compose.contains("axllent/mailpit"), "{compose}");
        assert!(compose.contains("- \"1025:1025\""), "{compose}");
        assert!(compose.contains("- \"8025:8025\""), "{compose}");
    }

    /// Deux fragments dans un même compose ne se marchent pas dessus : chacun a son
    /// service, et le fichier reste du YAML.
    #[test]
    fn two_fragments_share_the_same_anchor_without_colliding() {
        let (_parent, root) = project();

        apply(&root, "redis");
        let planned = plan_for(&options(&root, "mail")).expect("le plan doit se calculer");
        let compose = projected(&planned, "docker-compose.yml");

        assert!(compose.contains("redis:8-alpine"), "{compose}");
        assert!(compose.contains("axllent/mailpit"), "{compose}");
        assert_eq!(compose.matches("image: postgres:18-alpine").count(), 1, "{compose}");
    }
```

> `apply(&root, feature)` : le module `tests` d'`add` a déjà une fonction qui calcule puis applique un plan (celle qu'emploient les tests d'idempotence). L'employer ; si elle porte un autre nom, l'appeler par ce nom.

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib add::tests`
Expected : FAIL — `redis:8-alpine` absent du compose projeté.

- [ ] **Step 3 : déclarer les deux services**

Ajouter à `crates/rbs-cli/templates/features/redis/feature.toml` :

```toml
# Le port publié est celui que `config/default.toml` annonce ci-dessus : le cache tourne
# sur l'hôte comme le serveur, et n'a pas de réseau de compose pour l'atteindre.
[[anchors]]
anchor  = "services"
content = """
redis:
  image: redis:8-alpine
  ports:
    - "6379:6379"
  healthcheck:
    test: ["CMD", "redis-cli", "ping"]
    interval: 2s
    timeout: 3s
    retries: 30
"""
```

Ajouter à `crates/rbs-cli/templates/features/mail/feature.toml` :

```toml
# Mailpit et non un vrai SMTP : il accepte tout, n'envoie rien, et montre sur 8025 ce que
# le projet a cru envoyer. C'est le serveur que `smtp_port = 1025` désignait déjà.
[[anchors]]
anchor  = "services"
content = """
mailpit:
  image: axllent/mailpit:latest
  ports:
    - "1025:1025"
    - "8025:8025"
"""
```

- [ ] **Step 4 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib add::tests`
Expected : PASS.

- [ ] **Step 5 : vérifier au compose réel**

Run :
```bash
rm -rf /tmp/verif-services && mkdir -p /tmp/verif-services && cd /tmp/verif-services
cargo run -p rbs-cli -- new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
cd demo && git add -A && git commit -qm base
cargo run -p rbs-cli -- add redis && cargo run -p rbs-cli -- add mail
docker compose config >/dev/null && docker compose config --services
```
Expected : sortie 0, et `db`, `redis`, `mailpit` listés — sans `api` ni `migrate`, qui n'ont pas été installés.

- [ ] **Step 6 : la suite entière et les linters**

Run : `cargo test -p rbs-cli && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS.

- [ ] **Step 7 : commit**

```bash
git add crates/rbs-cli/templates/features/redis/feature.toml \
        crates/rbs-cli/templates/features/mail/feature.toml \
        crates/rbs-cli/src/add/mod.rs
git commit -F - <<'EOF'
feat(fragments): fait servir par le compose les adresses que redis et mail annoncent

Les deux fragments écrivaient une adresse dans config/default.toml —
redis://127.0.0.1:6379, smtp_port 1025 — en laissant à l'utilisateur le soin de
monter ce qui y répondrait, faute d'un fichier où déposer le service. Le compose
du projet est cet endroit.

Aucun des deux ne porte de profil : ce sont des dépendances de développement,
que `rbs dev` doit monter au même titre que la base.

Vérifications :
- cargo test -p rbs-cli : N passed, 0 failed
- docker compose config --services sur un projet portant les deux : db, redis,
  mailpit — sortie 0
EOF
```

---

## Task 7 : une seule liste de features, et un `--with` qui installe

**Files:**
- Modify: `crates/rbs-cli/src/templates.rs:136` (`embedded_names` devient `pub(crate)`, et une fonction publique la joint au `--template-dir`)
- Modify: `crates/rbs-cli/src/prompts.rs:11` (suppression de `FEATURES_DISPONIBLES`), `:155-160`
- Modify: `crates/rbs-cli/src/new.rs:24` (suppression de `FEATURES_CONNUES`), `:64-80` (`Error`), `:145-172` (`create`), `:206-222` (`validate_features`)
- Modify: `crates/rbs-cli/src/lib.rs:150-192`
- Test: modules `tests` de `prompts.rs`, `new.rs`

**Interfaces:**
- Consumes: `add::plan_for` et l'application d'un plan (inchangés) ; `templates::Source`.
- Produces:
  - `templates::feature_names(directory: Option<&Path>) -> Vec<String>` — les fragments disponibles, triés.
  - `new::Project` gagne `pub installed: Vec<InstalledFeature>` avec `pub struct InstalledFeature { pub name: String, pub files: usize, pub migration: bool }`.
  - `new::Error::FeatureAVenir` **disparaît**. `new::Error::Installation { feature: String, source: Box<add::Error> }` apparaît.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `crates/rbs-cli/src/prompts.rs`, module `tests` :

```rust
    /// Une liste écrite à la main se désynchronise : celle-ci se dérive des fragments
    /// que le binaire embarque.
    #[test]
    fn the_question_offers_every_embedded_feature() {
        let spy = Spy::default();

        resolve_with(&spy, Some("demo".into()), Some("postgres://x".into()),
                     Database::Postgres, None, false)
            .expect("les questions doivent aboutir");

        assert_eq!(
            spy.features_proposees(),
            crate::templates::feature_names(None),
            "la question doit proposer les sept fragments"
        );
    }
```

> `Spy` doit mémoriser la liste reçue par `features` en plus de compter l'appel : lui ajouter un `RefCell<Vec<String>>` et l'accesseur `features_proposees`.

Dans `crates/rbs-cli/src/new.rs`, module `tests` :

```rust
    #[test]
    fn an_unknown_feature_is_refused_before_anything_is_written() {
        let parent = TempDir::new().expect("répertoire temporaire créable");

        let error = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                database: Database::Postgres,
                features: vec!["graphql".to_string()],
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect_err("`graphql` n'est pas une feature");

        let message = error.to_string();
        assert!(message.contains("graphql"), "{message}");
        assert!(message.contains("jobs"), "la liste doit être complète : {message}");
        assert!(!parent.path().join("demo").exists(), "rien ne doit être écrit");
    }

    /// `jobs` était refusé par une liste qui l'avait oublié, alors que `rbs add jobs`
    /// fonctionnait.
    #[test]
    fn every_embedded_feature_is_accepted_by_name() {
        for feature in crate::templates::feature_names(None) {
            let parent = TempDir::new().expect("répertoire temporaire créable");

            create(
                &Options {
                    name: "demo".to_string(),
                    database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                    database: Database::Postgres,
                    features: vec![feature.clone()],
                    core_path: None,
                    template_dir: None,
                },
                parent.path(),
            )
            .unwrap_or_else(|error| panic!("`{feature}` doit s'installer : {error}"));
        }
    }

    #[test]
    fn a_requested_feature_is_actually_installed() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let project = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                database: Database::Postgres,
                features: vec!["auth".to_string()],
                core_path: None,
                template_dir: None,
            },
            parent.path(),
        )
        .expect("le projet doit se créer");

        assert!(project.root.join("src/auth/service.rs").is_file());

        let main = fs::read_to_string(project.root.join("src/main.rs")).expect("main lisible");
        assert!(main.contains("mod auth;"), "{main}");

        let manifest = fs::read_to_string(project.root.join("Cargo.toml")).expect("manifeste");
        assert!(manifest.contains("\"auth\""), "{manifest}");

        assert_eq!(project.installed.len(), 1);
        assert_eq!(project.installed[0].name, "auth");
        assert!(project.installed[0].migration);
    }

    /// L'ordre de frappe ne doit pas décider du contenu : deux `--with` équivalents
    /// produisent deux projets identiques.
    #[test]
    fn the_install_order_does_not_depend_on_the_typing_order() {
        let rendu = |features: Vec<String>| {
            let parent = TempDir::new().expect("répertoire temporaire créable");
            let project = create(
                &Options {
                    name: "demo".to_string(),
                    database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                    database: Database::Postgres,
                    features,
                    core_path: None,
                    template_dir: None,
                },
                parent.path(),
            )
            .expect("le projet doit se créer");

            let main = fs::read_to_string(project.root.join("src/main.rs")).expect("main");
            let compose = fs::read_to_string(project.root.join("docker-compose.yml"))
                .expect("compose");
            (main, compose)
        };

        assert_eq!(
            rendu(vec!["redis".into(), "mail".into()]),
            rendu(vec!["mail".into(), "redis".into()])
        );
    }

    #[test]
    fn a_failed_installation_leaves_no_project_behind() {
        let parent = TempDir::new().expect("répertoire temporaire créable");
        let directory = TempDir::new().expect("répertoire temporaire créable");
        // Un fragment vide : son manifeste est illisible, l'installation échoue.
        fs::create_dir(directory.path().join("cassee")).expect("répertoire créable");
        fs::write(directory.path().join("cassee/feature.toml"), "pas du toml [")
            .expect("écriture possible");

        let error = create(
            &Options {
                name: "demo".to_string(),
                database_url: "postgres://rbs:rbs@localhost:5432/demo".to_string(),
                database: Database::Postgres,
                features: vec!["cassee".to_string()],
                core_path: None,
                template_dir: Some(directory.path().to_path_buf()),
            },
            parent.path(),
        )
        .expect_err("le fragment est cassé");

        assert!(error.to_string().contains("cassee"), "{error}");
        assert!(
            !parent.path().join("demo").exists(),
            "le projet à moitié installé ne doit pas subsister"
        );
    }
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib new::tests prompts::tests`
Expected : FAIL — `` `auth` ne s'installe pas à la création `` sur les tests d'installation, et `cannot find function feature_names`.

- [ ] **Step 3 : exposer la liste unique**

Dans `crates/rbs-cli/src/templates.rs`, sous `names_on_disk` :

```rust
/// Les features installables, celles du `--template-dir` s'il en désigne un.
///
/// Une seule liste pour la question de `rbs new`, la validation de `--with` et le message
/// qui énumère les features connues : trois listes écrites à la main avaient divergé, et
/// `jobs` manquait à celle qui décidait.
pub(crate) fn feature_names(directory: Option<&Path>) -> Vec<String> {
    match directory {
        Some(directory) => names_on_disk(directory),
        None => embedded_names(),
    }
}
```

> Si `Source::feature` combine déjà les deux provenances autrement, reprendre exactement sa règle plutôt que d'en inventer une seconde.

Dans `crates/rbs-cli/src/prompts.rs`, supprimer `FEATURES_DISPONIBLES` et passer la liste en paramètre de `resolve` / `resolve_with` :

```rust
pub fn resolve(
    name: Option<String>,
    database_url: Option<String>,
    database: Database,
    features: Option<Vec<String>>,
    disponibles: &[String],
    yes: bool,
) -> Result<ProjectOptions, PromptError> {
    resolve_with(&Interactive, name, database_url, database, features, disponibles, yes)
}
```

et, dans `resolve_with` :

```rust
    let features = match features {
        Some(features) => features,
        None if yes => Vec::new(),
        None => questions.features(disponibles)?,
    };
```

Le trait `Questions::features` prend `&[String]` au lieu de `&[&str]`, et l'implémentation `Interactive` :

```rust
    fn features(&self, disponibles: &[String]) -> Result<Vec<String>, PromptError> {
        MultiSelect::new("Features à installer ?", disponibles.to_vec())
            .with_help_message(
                "espace pour cocher, entrée pour valider — `rbs add` en ajoute plus tard",
            )
            .prompt()
            .map_err(translate)
    }
```

- [ ] **Step 4 : `new` valide, puis installe**

Dans `crates/rbs-cli/src/new.rs`, supprimer `FEATURES_CONNUES` et la variante `Error::FeatureAVenir`, et ajouter :

```rust
    /// Une feature demandée n'a pas pu être installée.
    #[error("`{feature}` n'a pas pu être installée : {source}")]
    Installation {
        /// Feature en cause.
        feature: String,
        /// Cause remontée par l'installation.
        source: Box<crate::add::Error>,
    },
```

`Project` gagne :

```rust
/// Une feature posée par la création, et ce qu'elle a écrit.
#[derive(Debug)]
pub struct InstalledFeature {
    /// Nom de la feature, tel que `rbs add` l'accepte.
    pub name: String,
    /// Nombre de fichiers que le fragment a déposés.
    pub files: usize,
    /// Le fragment a posé une migration.
    pub migration: bool,
}
```

`validate_features` :

```rust
/// Une feature que rbs ne connaît pas est refusée avant qu'un fichier soit écrit — comme
/// le nom du projet et l'URL le sont déjà.
fn validate_features(features: &[String], disponibles: &[String]) -> Result<(), Error> {
    for feature in features {
        if !disponibles.contains(feature) {
            return Err(Error::FeatureInconnue {
                feature: feature.clone(),
                known: disponibles.join(", "),
            });
        }
    }

    Ok(())
}
```

`create` :

```rust
pub fn create(options: &Options, parent: &Path) -> Result<Project, Error> {
    let disponibles = crate::templates::feature_names(options.template_dir.as_deref());

    validate_name(&options.name)?;
    validate_features(&options.features, &disponibles)?;
    validate_database(options.database, &options.database_url)?;

    let root = parent.join(&options.name);
    if root.exists() {
        return Err(Error::RepertoireOccupe {
            path: root.display().to_string(),
        });
    }

    let dependency = core_dependency(options.core_path.as_deref(), options.database)?;
    let rendus = render(options, &dependency)?;

    write(&root, &rendus).map_err(|(path, source)| {
        let _ = fs::remove_dir_all(&root);
        Error::Ecriture { path, source }
    })?;

    // L'ordre est celui de la liste dérivée, non celui de la frappe : les insertions dans
    // le Migrator et dans le compose suivent l'ordre d'installation, et deux `--with`
    // équivalents doivent rendre deux projets identiques.
    let demandees: Vec<&String> = disponibles
        .iter()
        .filter(|feature| options.features.contains(feature))
        .collect();

    let mut installed = Vec::new();
    for feature in demandees {
        match install(&root, feature, options.template_dir.as_deref()) {
            Ok(pose) => installed.push(pose),
            Err(source) => {
                // Le répertoire n'existait pas avant la commande : le retirer entièrement
                // ne peut rien emporter qui lui préexistait.
                let _ = fs::remove_dir_all(&root);
                return Err(Error::Installation {
                    feature: feature.clone(),
                    source: Box::new(source),
                });
            }
        }
    }

    Ok(Project {
        depot_git: git_init(&root),
        files: rendus.len(),
        installed,
        root,
    })
}

/// Pose une feature dans le projet tout juste créé, par le pipeline de `rbs add`.
fn install(
    root: &Path,
    feature: &str,
    template_dir: Option<&Path>,
) -> Result<InstalledFeature, crate::add::Error> {
    let planned = crate::add::plan_for(&crate::add::Options {
        root: root.to_path_buf(),
        feature: feature.to_string(),
        force: false,
        template_dir: template_dir.map(Path::to_path_buf),
    })?;

    let migration = planned
        .files
        .iter()
        .any(|file| file.starts_with("migration/src/"));
    let files = planned.files.len();

    crate::add::apply(root, planned)?;

    Ok(InstalledFeature {
        name: feature.to_string(),
        files,
        migration,
    })
}
```

> `add::Options` et la fonction qui applique un plan portent peut-être d'autres noms : reprendre exactement ceux de `crates/rbs-cli/src/add/mod.rs` et de `crates/rbs-cli/src/lib.rs:194-230`, où la commande `add` les emploie déjà. `git init` reste après la boucle : un dépôt initialisé ensuite a le projet complet dans son premier `git add`.

- [ ] **Step 5 : la commande rend compte**

Dans `crates/rbs-cli/src/lib.rs`, après le `ui::success` du nombre de fichiers :

```rust
    for pose in &project.installed {
        let migration = if pose.migration { ", 1 migration" } else { "" };
        ui::info(&format!(
            "  + {:<8} {}{migration}",
            pose.name,
            ui::files(pose.files)
        ));
    }
```

et l'appel à `prompts::resolve` reçoit la liste :

```rust
    let disponibles = templates::feature_names(template_dir.as_deref());
    let options = prompts::resolve(
        Some(name),
        database_url,
        database,
        features,
        &disponibles,
        yes,
    )?;
```

- [ ] **Step 6 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib`
Expected : PASS.

- [ ] **Step 7 : vérifier que le projet engendré compile**

Run :
```bash
rm -rf /tmp/verif-with && mkdir -p /tmp/verif-with && cd /tmp/verif-with
cargo run -p rbs-cli -- new demo --yes --with auth,redis \
  --database-url postgres://rbs:secret@localhost:5432/demo
cd demo && cargo build && docker compose config --services
```
Expected : `Finished`, et `db`, `redis` listés.

- [ ] **Step 8 : la suite entière et les linters**

Run : `cargo test -p rbs-cli && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS. Les tests d'intégration qui prouvaient le refus de `--with` échouent ici : **les récrire** pour prouver l'installation, dans le même commit.

- [ ] **Step 9 : commit**

```bash
git add crates/rbs-cli/src crates/rbs-cli/tests
git commit -F - <<'EOF'
feat(new): installe les features que la création demande

La question « Features à installer ? » proposait des choix dont aucun
n'aboutissait : la validation refusait toute feature, connue ou non. Elle les
installe désormais, par le pipeline de `rbs add` et non par un second chemin
d'écriture.

Trois listes de features cohabitaient et aucune n'était juste — celle qui
décidait avait oublié `jobs`, si bien que `--with jobs` répondait « n'est pas
une feature rbs » quand `rbs add jobs` fonctionnait. Elles se dérivent
désormais des fragments embarqués.

L'ordre d'installation est celui de cette liste et non celui de la frappe : les
insertions dans le Migrator et dans le compose le suivent, et deux `--with`
équivalents doivent rendre deux projets identiques. L'échec d'une installation
retire toute la racine, qui n'existait pas avant la commande.

Vérifications :
- cargo test -p rbs-cli : N passed, 0 failed
- `new demo --with auth,redis` puis cargo build du projet : Finished
- cargo clippy -p rbs-cli --all-targets -- -D warnings : propre
EOF
```

---

## Task 8 : `rbs dev` monte le compose sans condition de feature

**Files:**
- Modify: `crates/rbs-cli/src/dev/mod.rs:102-116` (`remedy`), `:147-180` (`plan_with`, suppression de `declares_docker`)
- Modify: `crates/rbs-cli/src/doctor/base.rs` (le conseil nomme le compose du projet)
- Test: modules `tests` de `dev/mod.rs` et `doctor/base.rs`

**Interfaces:**
- Consumes: le compose du squelette (tâche 4).
- Produces: rien de nouveau ; `dev::plan` conserve sa signature.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `crates/rbs-cli/src/dev/mod.rs`, module `tests` — **supprimer** le test qui prouve qu'un projet sans la feature ne sonde pas le disque (sa prémisse n'existe plus) et écrire :

```rust
    /// Le compose n'est plus conditionné à `[package.metadata.rbs] features` : le
    /// squelette l'écrit, et un projet neuf doit démarrer sans `rbs add docker`.
    #[test]
    fn a_fresh_project_mounts_its_compose_without_the_docker_feature() {
        let (_parent, root) = project();
        assert!(
            !crate::metadata::read(&root.join("Cargo.toml"))
                .expect("manifeste lisible")
                .features
                .iter()
                .any(|f| f == "docker"),
            "le projet de ce test ne doit pas porter la feature"
        );

        let steps = plan(&root).expect("le plan doit se calculer");

        assert!(
            matches!(steps.first(), Some(Step::Compose(_))),
            "{steps:?}"
        );
    }

    #[test]
    fn a_project_without_a_compose_starts_at_the_database() {
        let (_parent, root) = project();
        fs::remove_file(root.join("docker-compose.yml")).expect("le compose doit exister");

        let steps = plan(&root).expect("le plan doit se calculer");

        assert!(
            !steps.iter().any(|step| matches!(step, Step::Compose(_))),
            "{steps:?}"
        );
    }
```

- [ ] **Step 2 : lancer les tests pour les voir échouer**

Run : `cargo test -p rbs-cli --lib dev::`
Expected : FAIL — le premier `Step` est `Database`, la feature n'étant pas déclarée.

- [ ] **Step 3 : retirer la condition**

Dans `crates/rbs-cli/src/dev/mod.rs`, `plan_with` :

```rust
fn plan_with(root: &Path, exists: impl Fn(&Path) -> bool) -> Result<Vec<Step>, Error> {
    let mut steps = Vec::new();

    // Le compose n'est plus la marque d'une feature : le squelette l'écrit pour tout
    // projet dont la base a un serveur à monter. Sa présence est le seul critère.
    let compose = root.join(COMPOSE);
    if exists(&compose) {
        steps.push(Step::Compose(compose));
    }
```

Supprimer `fn declares_docker`. Dans `Error::remedy` :

```rust
            Self::Injoignable { .. } => Some(format!(
                "démarrez-la — `docker compose up -d` à la racine du projet — ou corrigez \
                 {} dans le .env du projet",
                migrate::URL
            )),
```

Dans `crates/rbs-cli/src/doctor/base.rs`, le conseil du contrôle « base joignable » nomme le compose quand le projet en porte un :

```rust
    if !reachable(&hote, port) {
        let remedy = if root.join("docker-compose.yml").is_file() {
            "lancez `docker compose up -d` à la racine du projet, ou corrigez l'URL du .env"
                .to_string()
        } else {
            format!("démarrez {database}, ou corrigez l'URL du .env")
        };

        return Err(Check::failed(
            TITRE,
            format!("rien ne répond sur {hote}:{port}"),
            remedy,
        ));
    }
```

> Si la fonction ne reçoit pas `root`, le lui passer — l'appelant `doctor::run` l'a.

- [ ] **Step 4 : lancer les tests pour les voir passer**

Run : `cargo test -p rbs-cli --lib dev:: doctor::`
Expected : PASS.

- [ ] **Step 5 : vérifier le parcours réel**

Run :
```bash
rm -rf /tmp/verif-dev && mkdir -p /tmp/verif-dev && cd /tmp/verif-dev
cargo run -p rbs-cli -- new demo --yes --database-url postgres://rbs:secret@localhost:5432/demo
cd demo && cargo run -p rbs-cli -- add docker
docker compose ps --services  # avant : rien ne tourne
timeout 180 cargo run -p rbs-cli -- dev &
sleep 60 && docker compose ps --services && curl -sf localhost:8080/health && echo SANTE-OK
```
Expected : `docker compose ps --services` liste `db` **et non** `api` — le profil `app` a bien tenu le build de l'image à l'écart —, et `SANTE-OK`. Arrêter avec `docker compose down -v`.

- [ ] **Step 6 : la suite entière et les linters**

Run : `cargo test -p rbs-cli && cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS.

- [ ] **Step 7 : commit**

```bash
git add crates/rbs-cli/src/dev crates/rbs-cli/src/doctor
git commit -F - <<'EOF'
fix(dev): monte le compose du projet, qui n'est plus la marque d'une feature

`rbs dev` ne cherchait le compose que si le manifeste déclarait la feature
docker, et montait alors un fichier de déploiement qui ne publiait pas le port
de la base — celui-là même que la commande attendait ensuite sur l'hôte, où
elle ne pouvait donc jamais l'obtenir. Le squelette écrit désormais un compose
dont le port est publié, et sa présence devient le seul critère.

Le test qui prouvait qu'un projet sans la feature ne sondait pas le disque perd
sa prémisse et est supprimé plutôt que retourné en silence.

Vérifications :
- cargo test -p rbs-cli : N passed, 0 failed
- `rbs dev` sur un projet neuf : db monté, api absent (profil app), /health 200
EOF
```

---

## Task 9 : le parcours nominal, prouvé de bout en bout

**Files:**
- Modify: `crates/rbs-cli/tests/integration_new.rs`
- Test: le fichier lui-même

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces: rien.

> Ces tests compilent un projet engendré et démarrent un PostgreSQL en conteneur : ils sont lents et exigent Docker. C'est le seul test qui prouve réellement que rbs fonctionne.

- [ ] **Step 1 : écrire le test qui échoue**

Dans `crates/rbs-cli/tests/integration_new.rs`, en suivant les conventions du fichier (`assert_cmd`, `testcontainers`, `--core-path` vers la crate locale) :

```rust
/// Le parcours que la documentation enseigne, joué en entier : créer, monter la base par
/// le compose engendré, migrer, compiler, interroger /health. Aucune valeur n'est recopiée
/// d'un fichier à l'autre — c'est ce que le compose engendré doit garantir.
#[test]
fn the_generated_compose_serves_the_project_it_was_generated_for() {
    let parent = TempDir::new().expect("répertoire temporaire créable");
    let port = port_libre();
    let url = format!("postgres://rbs:secret@localhost:{port}/demo");

    rbs(&parent)
        .args(["new", "demo", "--yes", "--database-url", &url])
        .args(["--core-path", core_path()])
        .assert()
        .success();

    let root = parent.path().join("demo");

    // Le compose engendré, et lui seul : aucun `docker run` ni variable d'environnement.
    compose(&root, &["up", "-d", "--wait"]).assert().success();

    rbs(&root).args(["migrate", "up"]).assert().success();

    Command::new("cargo")
        .current_dir(&root)
        .args(["build"])
        .assert()
        .success();

    compose(&root, &["down", "-v"]).assert().success();
}

/// Deux fragments posés à la création cohabitent, et le projet compile.
#[test]
fn a_project_created_with_two_features_compiles() {
    let parent = TempDir::new().expect("répertoire temporaire créable");

    rbs(&parent)
        .args(["new", "demo", "--yes", "--with", "auth,redis"])
        .args(["--database-url", "postgres://rbs:secret@localhost:5432/demo"])
        .args(["--core-path", core_path()])
        .assert()
        .success()
        .stdout(predicate::str::contains("+ auth"))
        .stdout(predicate::str::contains("+ redis"));

    let root = parent.path().join("demo");

    assert!(root.join("src/auth/service.rs").is_file());
    assert!(root.join("src/cache/mod.rs").is_file());

    let compose = fs::read_to_string(root.join("docker-compose.yml")).expect("compose lisible");
    assert!(compose.contains("redis:8-alpine"), "{compose}");

    Command::new("docker")
        .current_dir(&root)
        .args(["compose", "config"])
        .assert()
        .success();

    Command::new("cargo")
        .current_dir(&root)
        .args(["build"])
        .assert()
        .success();
}
```

> `port_libre()`, `rbs()`, `compose()` et `core_path()` : réutiliser les helpers du fichier ou de `tests/common/mod.rs`. `port_libre` doit lier un port éphémère puis le relâcher, pour que deux exécutions concurrentes ne se disputent pas 5432 — c'est aussi ce qui prouve que le port publié suit bien l'URL.

- [ ] **Step 2 : lancer le test pour le voir échouer**

Run : `cargo test -p rbs-cli --test integration_new -- --nocapture`
Expected : FAIL — le premier test sur `docker compose up` (aucun compose avant la tâche 4) si l'on rejoue le plan depuis le début ; à ce stade du plan, il échoue sur les helpers manquants.

- [ ] **Step 3 : ajouter les helpers manquants et faire passer**

Écrire dans le fichier de test (ou `tests/common/mod.rs`) ce qui manque, sans toucher au code de production : les deux comportements doivent déjà être vrais après les tâches 4 à 7.

- [ ] **Step 4 : lancer le test pour le voir passer**

Run : `cargo test -p rbs-cli --test integration_new -- --nocapture`
Expected : PASS. Noter la durée : ces tests compilent un projet.

- [ ] **Step 5 : la suite entière**

Run : `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/tests
git commit -F - <<'EOF'
test(new): joue le parcours nominal sur le compose engendré

Les tests unitaires prouvent la forme du fichier, non qu'il serve. Celui-ci
monte la base par le seul compose engendré, applique les migrations et compile
le projet : aucune valeur n'est recopiée d'un fichier à l'autre, ce qui est
précisément la promesse.

Le port est tiré au sort à chaque exécution, ce qui interdit à deux exécutions
concurrentes de se disputer 5432 et prouve au passage que le port publié suit
l'URL du projet.

Vérifications :
- cargo test -p rbs-cli --test integration_new : N passed, 0 failed
- cargo test --workspace : N passed, 0 failed
EOF
```

---

## Task 10 : la documentation, la note de migration et la 1.1.0

**Files:**
- Modify: `docs/docs/getting-started.md` + `docs/i18n/fr/docusaurus-plugin-content-docs/current/getting-started.md`
- Modify: `docs/docs/cli/new.md`, `cli/add.md`, `cli/dev.md` + leurs trois homologues français
- Modify: `docs/docs/guides/cache.md`, `guides/mail.md` + leurs deux homologues français
- Modify: `README.md`, `README.fr.md` (si leur parcours de démarrage montre le `docker run`)
- Create: `crates/rbs-cli/notes/1.1.0.md`
- Modify: `Cargo.toml` (workspace en `1.1.0`), `CHANGELOG.md` et son homologue français

**Interfaces:**
- Consumes: le binaire recompilé de toutes les tâches précédentes.
- Produces: rien de code.

- [ ] **Step 1 : recompiler et capturer les sorties**

Run :
```bash
cargo build -p rbs-cli --release
rm -rf /tmp/captures && mkdir -p /tmp/captures && cd /tmp/captures
../../target/release/rbs-cli new demo --yes \
  --database-url postgres://rbs:secret@localhost:5432/demo 2>&1 | tee new.txt
cd demo && cat docker-compose.yml | tee ../compose.txt
docker compose up -d --wait 2>&1 | tee ../up.txt
../../../target/release/rbs-cli doctor 2>&1 | tee ../doctor.txt
```

Toutes les sorties des pages viennent de ces fichiers. **Aucun extrait écrit à la main.**

- [ ] **Step 2 : récrire les pages anglaises**

- `getting-started.md` : la section « Starting a database » et son `docker run -p 5432:5432` sont **supprimées**. Le compose engendré les remplace, avec la sortie de `up.txt`. Le compte « 16 fichiers » passe à 17 dans le bloc de `new.txt`.
- `cli/new.md` : la section « `--with` in this version » est supprimée et remplacée par ce que `--with` fait — la liste des sept features, l'ordre d'installation dérivé et non frappé, le message de refus d'un nom inconnu. Le tableau des flags décrit `--with` comme installant. Ajouter une section sur le compose engendré et ses deux cas d'abstention (SQLite, hôte distant).
- `cli/add.md` : `add docker` insère au lieu de déposer ; les trois états et leur comportement ; `add redis` et `add mail` déposent un service.
- `cli/dev.md` : le compose n'est plus conditionné à la feature ; les profils, et pourquoi `rbs dev` ne bâtit pas l'image de l'API.
- `guides/cache.md`, `guides/mail.md` : le service est monté par le compose du projet ; retirer les instructions de montage manuel.

- [ ] **Step 3 : porter les six pages en français**

Chaque page modifiée en anglais l'est aussi en français, dans le même commit. Les blocs de terminal sont identiques — les sorties du CLI sont en français dans les deux versions.

- [ ] **Step 4 : mesurer la parité**

Run : `cd docs && npm run parite && npm run clear && npm run build`
Expected : « 24 paires », « 0 écart », sortie 0, puis deux `[SUCCESS]`.

> `parite.mjs` ne mesure ni les tableaux ni le dernier commit des paires racine : relire à l'œil les tableaux modifiés de `cli/new.md` et `cli/add.md`.

- [ ] **Step 5 : la note de migration et la version**

Créer `crates/rbs-cli/notes/1.1.0.md` :

```markdown
# rbs 1.1.0

## `--with` installe au lieu de refuser

`rbs new mon-api --with auth` posait la feature ? Non : la commande échouait avec
« `auth` ne s'installe pas à la création », code de sortie 1. Elle l'installe désormais.
Un script qui comptait sur cet échec change de comportement.

`--with jobs` était refusé par une liste qui l'avait oublié. Les sept fragments du binaire
sont acceptés.

## Le compose du projet

`rbs new` écrit un `docker-compose.yml` portant la base du projet, avec les identifiants et
le port de son `.env`. `docker compose up -d` puis `cargo run` suffisent.

Rien n'est écrit pour un projet SQLite, ni pour un projet dont l'URL vise un hôte distant :
il n'y aurait rien à monter, ou le service monté doublerait à tort la base visée.

**Un projet créé avant la 1.1.0 n'a pas ce fichier, et `rbs upgrade` ne le lui ajoutera
pas** — cette commande n'écrit que dans un manifeste. `rbs add docker` lui en écrit un
entier, services de déploiement compris.

## Le compose de `rbs add docker`

Le fragment déposait un fichier ; il insère désormais ses services `api` et `migrate` dans
l'ancre `# <rbs:services>` du compose du projet, sous le profil `app` :

- `docker compose up -d` monte l'infrastructure seule — c'est ce que fait `rbs dev` ;
- `docker compose --profile app up` monte l'ensemble, image de l'API comprise.

Un compose réécrit à la main qui a perdu son ancre n'est pas touché : le CLI affiche le
bloc à recoller.
```

Porter `Cargo.toml` du workspace en `1.1.0`. Ajouter l'entrée datée au `CHANGELOG.md` et à son homologue français.

- [ ] **Step 6 : vérifier la complétude des notes**

Run : `cargo test -p rbs-cli --lib notes::`
Expected : PASS — le contrôle de complétude trouve `1.1.0.md` pour le saut depuis la version publiée.

- [ ] **Step 7 : la suite entière**

Run : `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected : PASS.

- [ ] **Step 8 : commit**

```bash
git add docs README.md README.fr.md CHANGELOG.md CHANGELOG.fr.md \
        Cargo.toml Cargo.lock crates/rbs-cli/notes/1.1.0.md
git commit -F - <<'EOF'
docs: remet le parcours de démarrage sur le compose engendré

La page d'accueil enseignait un `docker run -p 5432:5432` dont chaque valeur
était recopiée depuis le .env que le CLI venait d'écrire. Elle montre désormais
le fichier que la création engendre.

La page de `new` décrivait un `--with` qui refusait tout ; celle de `dev`,
un compose conditionné à une feature. Les deux disent ce que les commandes
font.

Toutes les sorties sont capturées sur le binaire recompilé.

Vérifications :
- npm run parite : 24 paires, 0 écart, sortie 0
- npm run clear && npm run build : deux [SUCCESS]
- cargo test --workspace : N passed, 0 failed
EOF
```

---

## Auto-relecture

**Couverture de la spec.** §3 → tâche 4. §3.1 → tâches 3 et 4. §3.2 → tâche 4 (test `the_published_port_is_the_one_the_project_will_dial`). §3.3 → tâche 4 (tests SQLite et hôte distant). §3.4 → tâche 4 (test de l'image). §3.5 → tâche 4. §4.1 → tâches 1 et 2. §4.2 → tâche 1. §4.3 → tâche 5. §4.4 → tâche 5 (trois tests, un par état). §4.5 → tâche 6. §5.1 → tâche 7. §5.2 → tâche 7 (ordre, atomicité, `git init`). §5.3 → tâche 7. §6 → tâche 8. §7.1 → tâches 4, 5 et 8, chaque test nommé à l'endroit où il change. §7.2 → tâches 4, 5, 6, 9. §7.3 → tâche 10. §7.4 → tâche 10. §8 — hors périmètre : aucune tâche, ce qui est correct.

**Trous connus, signalés et non masqués :**

- La tâche 2 pose deux tests que seule la tâche 4 fait passer. Ils sont marqués `#[ignore]` avec sa raison, et l'attribut est retiré à la tâche 4, step 4. Une tâche qui laisse un test ignoré derrière elle est une dette, pas un état stable : c'est le seul endroit du plan où elle est acceptée, et elle est refermée par la tâche suivante.
- Les noms exacts de `add::Options`, de la fonction d'application d'un plan et du helper `apply` des tests ne sont pas figés ici : le plan dit de reprendre ceux du code plutôt que d'en inventer. C'est une lecture de deux minutes à la tâche 7, et l'inventer serait la vraie erreur.
- Le nombre de fichiers des tests d'intégration existants n'est pas énuméré : ils sont corrigés au cas par cas à la tâche 4, step 7, un projet SQLite en comptant toujours 16.

**Cohérence des types.** `Connection` (tâche 3) est employée par `compose_utile` (tâche 4) et par le contexte d'`add` (tâche 5) sous le même nom de champs. `Anchor` gagne `comment` et `optional` à la tâche 1, et les deux sont lus à la tâche 2. `DeclaredFile::if_absent` (tâche 5) est lu par `a_deposer` dans la même tâche. `InstalledFeature` (tâche 7) est lu par `lib.rs` dans la même tâche. `templates::feature_names` (tâche 7) est appelée par `prompts`, `new` et `lib` — trois appelants, une signature.
