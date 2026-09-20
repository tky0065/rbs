//! Les points d'insertion du projet généré, et de quoi y écrire.
//!
//! Le CLI ne réécrit jamais d'AST : il insère dans des ancres en commentaires. Ce module
//! ne connaît que des chaînes — l'écriture sur disque appartient à ses appelants.

use std::borrow::Cow;
use std::fmt;
use std::path::Path;

/// Un point d'insertion, et le fichier du projet qui le porte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Anchor {
    /// Nom tel qu'il paraît entre les chevrons : `features` pour `// <rbs:features>`.
    ///
    /// Emprunté pour les ancres du registre ; possédé pour celles du modèle d'une entité,
    /// dont le nom porte celui de l'entité — `relations:users`.
    pub name: Cow<'static, str>,
    /// Chemin du fichier porteur, relatif à la racine du projet.
    ///
    /// Emprunté pour les ancres du registre, dont le fichier est fixe ; possédé pour
    /// celles du modèle d'une feature, dont il dépend du nom de cette feature.
    pub file: Cow<'static, str>,
    /// Marqueur de commentaire du langage porteur : `//` en Rust, `#` en YAML.
    pub comment: &'static str,
    /// Le bloc se maintient trié, au lieu d'empiler dans l'ordre d'arrivée.
    ///
    /// Réservé aux blocs dont le contenu est une liste que rustfmt ordonne lui-même —
    /// les `pub mod` des features. Un projet qui installe `auth`, lequel entraîne
    /// `rate-limit`, puis engendre un CRUD nommé entre les deux, verrait sinon son
    /// propre `cargo fmt --check` échouer sur une ligne qu'il n'a pas écrite. Les
    /// autres ancres gardent l'ordre d'arrivée, qui y porte du sens : les migrations
    /// s'appliquent dans l'ordre où elles sont nées.
    pub sorted: bool,
    /// L'ancre peut légitimement manquer, son fichier porteur étant lui-même facultatif.
    ///
    /// `doctor` ne réclame pas une ancre optionnelle dont le fichier est absent : un
    /// projet SQLite n'a pas de compose, et n'a donc pas à passer pour incomplet.
    pub optional: bool,
    /// Motif de la ligne après laquelle le bloc se repose, quand l'ancre a disparu.
    ///
    /// Une ancre effacée ne laisse rien derrière elle : `rbs doctor --fix` n'a pas d'autre
    /// moyen de savoir où elle vivait que cette ligne d'accroche, déclarée ici et vérifiée
    /// contre la template qui la porte. Vide pour les ancres dont l'accroche dépend du
    /// contenu engendré, et que le diagnostic ne réclame pas.
    pub after: &'static str,
}

impl Anchor {
    /// Balise ouvrante, telle qu'elle est écrite dans le fichier.
    pub(crate) fn opening(&self) -> String {
        format!("{} <rbs:{}>", self.comment, self.name)
    }

    /// Balise fermante, telle qu'elle est écrite dans le fichier.
    pub(crate) fn closing(&self) -> String {
        format!("{} </rbs:{}>", self.comment, self.name)
    }

    /// Le bloc à recoller quand l'ancre a disparu, prêt à être collé tel quel.
    pub(crate) fn block(&self) -> String {
        format!("{}\n{}", self.opening(), self.closing())
    }

    /// Le pas d'indentation du langage porteur : quatre colonnes en Rust, deux ailleurs.
    ///
    /// Sert à reposer le bloc d'une ancre disparue sous une accroche qui ouvre un bloc.
    /// Le marqueur de commentaire ne suffisait pas à le dire : le TypeScript se commente
    /// comme le Rust et s'indente comme le YAML, et la table de routage de
    /// l'administration recevait son ancre deux colonnes trop loin.
    pub(crate) fn pas(&self) -> &'static str {
        if self.file.ends_with(".rs") {
            "    "
        } else {
            "  "
        }
    }

    /// La même ancre, dans un autre fichier.
    ///
    /// Sert aux ancres du modèle d'une feature : leur fichier n'est connu qu'à
    /// l'exécution, une fois le nom de la feature en main.
    pub(crate) fn in_file(&self, path: &str) -> Anchor {
        Anchor {
            file: Cow::Owned(path.to_string()),
            ..self.clone()
        }
    }

    /// La même ancre, dans le modèle de l'entité `table` et pour elle seule.
    ///
    /// Un fichier de modèle peut porter plusieurs entités — `src/auth/model.rs` en porte
    /// deux, nichées dans leurs modules — et l'ancre s'y répète autant de fois. Le nom de
    /// l'entité l'accompagne donc entre les chevrons, sans quoi une relation vers la
    /// seconde irait s'écrire dans la première, seule que le fichier rencontre.
    pub(crate) fn for_entity(&self, path: &str, table: &str) -> Anchor {
        Anchor {
            name: Cow::Owned(format!("{}:{table}", self.name)),
            file: Cow::Owned(path.to_string()),
            ..self.clone()
        }
    }
}

/// Déclaration des modules de feature, en tête de `main.rs`.
pub(crate) const FEATURES: Anchor = Anchor {
    name: Cow::Borrowed("features"),
    file: Cow::Borrowed("src/main.rs"),
    comment: "//",
    sorted: true,
    optional: false,
    after: "pub mod state;",
};

/// Déclaration des modules que `rbs add` installe, dans leur point de montage.
///
/// `src/` ne mêle plus le code du développeur et celui du CLI : les fragments s'y
/// déclarent, et `<rbs:features>` ne reçoit d'eux que le `pub mod modules;` qui ouvre ce
/// fichier. `auth` fait exception et reste une feature comme les siennes.
pub(crate) const MODULES: Anchor = Anchor {
    name: Cow::Borrowed("modules"),
    file: Cow::Borrowed("src/modules/mod.rs"),
    comment: "//",
    // Même raison que `FEATURES` : rustfmt trie les `pub mod`, et un fragment intercalé
    // entre deux autres ferait échouer le `cargo fmt --check` du développeur.
    sorted: true,
    // `add` pose ce fichier au premier fragment qui l'y vise, et pas avant : un projet
    // sans fragment n'a pas de répertoire vide à porter.
    optional: true,
    after: "",
};

/// Montage des routes d'une feature dans le routeur.
pub(crate) const ROUTES: Anchor = Anchor {
    name: Cow::Borrowed("routes"),
    file: Cow::Borrowed("src/router.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: ".merge(health::routes())",
};

/// Middlewares qu'une feature empile sur le routeur.
///
/// Distincte de [`ROUTES`], bien qu'elles partagent leur fichier : une route se monte sur
/// le routeur, une couche l'enveloppe, et l'endroit où l'une se déclare est précisément
/// celui où l'autre n'aurait aucun effet.
pub(crate) const LAYERS: Anchor = Anchor {
    name: Cow::Borrowed("layers"),
    file: Cow::Borrowed("src/router.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: ".merge(docs)",
};

/// Enregistrement des chemins d'une feature dans le document OpenAPI.
pub(crate) const OPENAPI: Anchor = Anchor {
    name: Cow::Borrowed("openapi"),
    file: Cow::Borrowed("src/openapi.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "crate::health::controller::health,",
};

/// Déclaration des fichiers de migration.
///
/// Distincte de [`MIGRATIONS`] : Rust interdit un `mod` non-inline dans un bloc, et la
/// déclaration ne peut donc pas tenir dans le `vec!` du `Migrator`.
///
/// Triée, à la différence de [`MIGRATIONS`] : rustfmt ordonne les `mod`, et une même
/// commande pose plusieurs migrations sous un seul horodatage — `add webhooks` en écrit
/// trois. L'ordre d'exécution vit dans le `vec!`, que rustfmt laisse tel quel.
pub(crate) const MIGRATION_MODULES: Anchor = Anchor {
    name: Cow::Borrowed("migration_modules"),
    file: Cow::Borrowed("migration/src/lib.rs"),
    comment: "//",
    sorted: true,
    optional: false,
    after: "pub use sea_orm_migration::prelude::*;",
};

/// Inscription des migrations dans le `Migrator`.
pub(crate) const MIGRATIONS: Anchor = Anchor {
    name: Cow::Borrowed("migrations"),
    file: Cow::Borrowed("migration/src/lib.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "vec![",
};

/// Déclaration d'un champ partagé dans la struct `AppState`.
pub(crate) const STATE_CHAMPS: Anchor = Anchor {
    name: Cow::Borrowed("state_champs"),
    file: Cow::Borrowed("src/state.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "core: CoreState,",
};

/// Initialisation de ce champ dans `AppState::new`.
///
/// Distincte de [`STATE_CHAMPS`] : un champ se déclare à un endroit et se construit à un
/// autre, et une ancre unique ne pourrait pas viser les deux.
pub(crate) const STATE_INIT: Anchor = Anchor {
    name: Cow::Borrowed("state_init"),
    file: Cow::Borrowed("src/state.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    // Avant `core: CoreState::new(db, config)`, et non après : ce dernier engloutit
    // `config` par valeur, et un fragment posé par l'ancre peut avoir besoin d'en lire un
    // champ avant qu'il ne parte.
    after: "Ok(Self {",
};

/// Tâches de fond lancées au démarrage, l'état construit et le serveur pas encore lié.
///
/// Distincte de [`STATE_INIT`] : ce qui vit dans l'état est une valeur, ce qui vit ici est
/// une tâche, et une valeur ne peut pas se détacher elle-même.
pub(crate) const STARTUP: Anchor = Anchor {
    name: Cow::Borrowed("startup"),
    file: Cow::Borrowed("src/main.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "let state = state::AppState::new(db, config)?;",
};

/// Déclaration des seeds dans le binaire qui les applique.
///
/// Seule ancre à vivre dans une invocation de `macro_rules!` : elle porte des
/// identifiants de module, que la macro déclare et enchaîne d'un même geste. Un `mod` non
/// inline ne s'écrit pas dans un bloc — c'est ce qui vaut deux ancres à la crate
/// `migration` — et la macro évite ici d'en poser une seconde.
pub(crate) const SEEDS: Anchor = Anchor {
    name: Cow::Borrowed("seeds"),
    file: Cow::Borrowed("src/seeds/main.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "seeds! {",
};

/// Services que les fragments ajoutent au compose du projet.
///
/// Optionnelle : un projet SQLite, un projet visant une base distante, un projet dont
/// l'URL ne porte pas d'identifiants et tout projet créé avant la 1.1.0 n'ont pas de
/// compose, et n'ont donc pas cette ancre à porter.
pub(crate) const SERVICES: Anchor = Anchor {
    name: Cow::Borrowed("services"),
    file: Cow::Borrowed("docker-compose.yml"),
    comment: "#",
    sorted: false,
    optional: true,
    after: "services:",
};

/// Exclusions Git que les fragments ajoutent au `.gitignore` du projet.
///
/// Un fragment ne pouvait jusqu'ici rien y écrire : le squelette pose le fichier, et un
/// fragment qui le déposerait à son tour entrerait en conflit avec lui. Le premier besoin
/// est le frontend — un `node_modules/` non ignoré, ce sont des dizaines de milliers de
/// fichiers proposés au premier `git status` — mais la lacune n'a rien qui lui soit propre.
///
/// Optionnelle : le développeur peut avoir supprimé le fichier, et un projet sans
/// exclusions n'est pas un projet incomplet. Le bloc lui est alors montré.
///
/// Non triée : l'ordre d'un fichier d'exclusions porte du sens — une négation ne vaut que
/// sous le motif qu'elle rouvre.
pub(crate) const IGNORE: Anchor = Anchor {
    name: Cow::Borrowed("ignore"),
    file: Cow::Borrowed(".gitignore"),
    comment: "#",
    sorted: false,
    optional: true,
    after: ".env",
};

/// Sondes de santé que les fragments ajoutent au contrôle de `GET /health`.
///
/// Le noyau porte la mécanique du contrôle, jamais la façon de joindre un cache ou un
/// stockage : ces clients-là vivent dans le projet, et leur sonde s'inscrit ici.
///
/// Elle manque à tout projet engendré avant son arrivée : `doctor` la nomme alors, affiche
/// son bloc, et `--fix` la repose sous le `vec![` des sondes.
pub(crate) const HEALTH_PROBES: Anchor = Anchor {
    name: Cow::Borrowed("health_probes"),
    file: Cow::Borrowed("src/health/controller.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "vec![",
};

/// Inscription d'un job au registre que le worker de la file consulte.
///
/// Vit, comme [`JOB_MODULES`] et celle du compose, dans un fichier qu'un fragment dépose
/// plutôt que le squelette : `src/modules/jobs/mod.rs` n'existe que sur un projet qui a
/// installé la file. C'est ce qui la rend optionnelle — un projet sans file n'a pas à
/// passer pour incomplet.
///
/// Sans elle, `registry()` ne s'écrit qu'à la main : un fragment ne peut viser qu'une ancre
/// de ce registre, et le worker n'exécute que ce que `registry()` lui a déclaré.
pub(crate) const JOBS: Anchor = Anchor {
    name: Cow::Borrowed("jobs"),
    file: Cow::Borrowed("src/modules/jobs/mod.rs"),
    comment: "//",
    sorted: false,
    optional: true,
    // Une instruction entière et non un appel chaîné : une ancre posée au milieu d'un
    // enchaînement de `.register()` ne survit pas à rustfmt, qui la disloque — la balise
    // fermante finit à une autre indentation que l'ouvrante — dès qu'un fragment y ajoute
    // un second appel. La forme en instructions reste, elle, stable dans les deux états.
    after: "registre = registre.register::<demo::Log>();",
};

/// Déclaration du module d'un job engendré, dans le fichier que le fragment `jobs` dépose.
///
/// Partage son fichier avec [`JOBS`], sans partager son accroche : `pub mod worker;` est le
/// dernier `pub mod` que le squelette écrit, et rustfmt ne réordonne pas des `pub mod` à
/// travers une ligne de commentaire — l'accroche reste donc valable une fois le premier job
/// engendré posé sous elle, ce que la ligne d'un job donné ne pourrait pas garantir.
pub(crate) const JOB_MODULES: Anchor = Anchor {
    name: Cow::Borrowed("job_modules"),
    file: Cow::Borrowed("src/modules/jobs/mod.rs"),
    comment: "//",
    // rustfmt trie les `pub mod` de ce bloc, comme il trie ceux de `MODULES` : un second
    // job engendré dans le désordre ferait sinon échouer le `cargo fmt --check` du projet.
    sorted: true,
    optional: true,
    after: "pub mod worker;",
};

/// Échéance d'un job engendré, ajoutée au calendrier que le ticker consulte.
///
/// Sans elle, `schedules()` ne s'écrit qu'à la main. L'accroche est la ligne qui initialise
/// le vecteur, et non l'échéance de démonstration qui la suit : elle seule survit au
/// retrait de cette démonstration, que le développeur fait tôt.
pub(crate) const SCHEDULES: Anchor = Anchor {
    name: Cow::Borrowed("schedules"),
    file: Cow::Borrowed("src/modules/scheduler/mod.rs"),
    comment: "//",
    sorted: false,
    optional: true,
    after: "let mut calendrier = Vec::new();",
};

/// Méthodes que les fragments ajoutent à l'`impl HasAuth for AppState` du projet.
///
/// Le seul point d'insertion du registre qui vive **à l'intérieur** d'un bloc `impl`, et
/// non entre deux items. La raison est que le trait `HasAuth` ne peut être implémenté
/// qu'une fois : un fragment qui voudrait juger un justificatif de plus — une clé d'API —
/// n'a aucun autre endroit où poser sa méthode, et le CLI ne réécrit pas d'AST.
///
/// L'accroche est la ligne d'ouverture de l'`impl`, stable et unique dans le fichier. Le
/// bloc n'est pas trié : rustfmt ne réordonne pas les méthodes d'une implémentation.
pub(crate) const AUTH_IMPL: Anchor = Anchor {
    name: Cow::Borrowed("auth_impl"),
    file: Cow::Borrowed("src/auth/mod.rs"),
    comment: "//",
    sorted: false,
    // Le fichier est déposé par le fragment `auth` : un projet qui ne l'a pas installé n'a
    // pas ce fichier, et n'est pas incomplet pour autant.
    optional: true,
    after: "impl HasAuth for AppState {",
};

/// Les préfixes que le serveur de développement du client relaie au binaire.
///
/// La première des trois ancres du frontend, et la seule que le fragment `frontend` dépose
/// seul. `vite.config.ts` relayait une liste figée — la sonde de santé, l'interface et le
/// document OpenAPI — quand chaque `rbs generate crud` ajoute un préfixe de route : l'écran
/// d'administration que la commande engendre appelait `/articles` sur le port de Vite, qui
/// lui rendait l'application en guise de page de données.
///
/// En `//` et non en `#`, à la différence des deux autres ancres à vivre hors du Rust : un
/// `#` ouvre un commentaire en YAML et dans un fichier d'exclusions, jamais en TypeScript,
/// où il ne nomme qu'un membre privé de classe — posé dans un littéral de tableau, il
/// arrêterait la construction du client.
///
/// Optionnelle : son fichier est déposé par le fragment `frontend`, et un projet sans
/// client n'a pas de serveur de développement à configurer.
///
/// L'accroche est le dernier préfixe de la liste figée, qui ne paraît qu'une fois dans le
/// fichier — le tableau lui-même s'ouvre sur une ligne que `repose` indenterait d'un cran
/// de trop.
pub(crate) const VITE_PROXY: Anchor = Anchor {
    name: Cow::Borrowed("vite_proxy"),
    file: Cow::Borrowed("frontend/vite.config.ts"),
    comment: "//",
    sorted: false,
    optional: true,
    after: "'/api-docs',",
};

/// Les écrans que l'espace d'administration monte dans sa table de routage.
///
/// La deuxième des trois ancres du frontend, et la seule déclaration d'un écran : en
/// TypeScript, l'import qui donne le composant à la route *est* la déclaration du module,
/// là où Rust demande un `pub mod` distinct du montage. Le registre s'épargne ainsi la
/// troisième ancre que la symétrie avec [`MODULES`] aurait réclamée.
///
/// Optionnelle : son fichier est déposé par le fragment `frontend-admin`, et un projet
/// sans espace d'administration n'a pas de table de routage à porter.
///
/// L'accroche est l'ouverture du tableau des enfants, et non la route de la coquille qui
/// la précède : c'est sous cette ligne que les écrans se montent, et elle ne paraît
/// qu'une fois dans le fichier.
pub(crate) const ADMIN_ROUTES: Anchor = Anchor {
    name: Cow::Borrowed("admin_routes"),
    file: Cow::Borrowed("frontend/src/admin/montage.ts"),
    comment: "//",
    sorted: false,
    optional: true,
    after: "children: [",
};

/// Les entrées que les écrans inscrivent au rail de l'espace d'administration.
///
/// Dans un module TypeScript et non dans la coquille : le rail y est servi deux fois — à
/// demeure au-delà de la largeur d'un ordinateur, dans un panneau en deçà — et le
/// mécanisme d'ancres ne connaît que les commentaires `//` et `#`, jamais ceux d'un
/// `<template>`. Une entrée posée ici paraît donc aux deux endroits, par une seule ligne.
///
/// Optionnelle, pour la même raison que [`ADMIN_ROUTES`] : le fragment dépose son fichier.
pub(crate) const ADMIN_RAIL: Anchor = Anchor {
    name: Cow::Borrowed("admin_rail"),
    file: Cow::Borrowed("frontend/src/admin/rail.ts"),
    comment: "//",
    sorted: false,
    optional: true,
    after: "export const RAIL: EntreeDuRail[] = [",
};

/// Variantes de l'énumération `Relation` du modèle d'une entité.
///
/// Hors du registre statique : son fichier et son nom dépendent tous deux de l'entité
/// visée, et se fixent par [`Anchor::for_entity`] une fois celle-ci connue — l'ancre
/// écrite est `<rbs:relations:users>`, non `<rbs:relations>`.
pub(crate) const RELATIONS: Anchor = Anchor {
    name: Cow::Borrowed("relations"),
    file: Cow::Borrowed("src/{feature}/model.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "",
};

/// Implémentations de `Related` du modèle d'une entité.
///
/// Hors du registre statique, pour la même raison que [`RELATIONS`].
pub(crate) const RELATED: Anchor = Anchor {
    name: Cow::Borrowed("related"),
    file: Cow::Borrowed("src/{feature}/model.rs"),
    comment: "//",
    sorted: false,
    optional: false,
    after: "",
};

/// Les points d'insertion du squelette.
///
/// La génération vise chaque ancre nommément ; `rbs doctor` parcourt cette liste pour
/// vérifier qu'un projet les porte toutes.
pub(crate) const ANCRES: [Anchor; 21] = [
    FEATURES,
    MODULES,
    ROUTES,
    LAYERS,
    OPENAPI,
    MIGRATION_MODULES,
    MIGRATIONS,
    STATE_CHAMPS,
    STATE_INIT,
    STARTUP,
    SEEDS,
    SERVICES,
    IGNORE,
    HEALTH_PROBES,
    JOBS,
    JOB_MODULES,
    SCHEDULES,
    AUTH_IMPL,
    VITE_PROXY,
    ADMIN_ROUTES,
    ADMIN_RAIL,
];

/// Résout l'ancre `<rbs:features>` par repli, entre `src/lib.rs` et `src/main.rs`.
///
/// Elle vise `src/lib.rs`, que porte tout projet engendré depuis ce jalon : le binaire
/// principal et celui des seeds y puisent les modules de feature par un chemin de crate,
/// et non plus par `#[path]`. Un projet engendré plus tôt n'a pas de bibliothèque, et
/// l'ancre y reste dans `src/main.rs`, où elle a toujours vécu — sans ce repli, `generate`
/// et `doctor` cesseraient de fonctionner sur l'ensemble du parc existant.
pub(crate) fn resolve_features(root: &Path) -> Anchor {
    resolve(FEATURES, has_library(root))
}

/// La même ancre, celle des features visant la bibliothèque quand le projet en porte une.
///
/// La règle n'existe qu'ici : quatre appelants la réécrivaient, et une ancre ajoutée au
/// registre demandait de les visiter tous.
pub(crate) fn resolve(anchor: Anchor, with_library: bool) -> Anchor {
    if with_library && anchor.name == FEATURES.name {
        anchor.in_file("src/lib.rs")
    } else {
        anchor
    }
}

/// Les ancres du registre, celle des features résolue pour `root`.
///
/// Le disque n'est interrogé qu'une fois pour toutes, et non une fois par ancre.
pub(crate) fn resolved(root: &Path) -> Vec<Anchor> {
    let with_library = has_library(root);

    ANCRES
        .into_iter()
        .map(|anchor| resolve(anchor, with_library))
        .collect()
}

/// La seule question que la résolution des ancres pose au disque.
fn has_library(root: &Path) -> bool {
    root.join("src/lib.rs").exists()
}

/// Une ancre attendue que le fichier ne porte pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Missing {
    pub anchor: Anchor,
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ancre {} introuvable dans {}",
            self.anchor.opening(),
            self.anchor.file
        )
    }
}

impl std::error::Error for Missing {}

/// Une ancre présente, mais sous une ligne que l'insertion doit précéder.
///
/// Distincte de [`Missing`] : reposer l'ancre n'y changerait rien, c'est sa place qui
/// est en cause, et seul le développeur peut la déplacer — le CLI ne réordonne jamais un
/// fichier qu'il a déjà écrit.
///
/// Rendue boxée : elle porte le bloc tel qu'il est dans le fichier, et chaque `Result`
/// qui la relaie prendrait sa taille.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Misplaced {
    pub anchor: Anchor,
    /// La ligne que l'ancre aurait dû précéder, telle que le fragment la déclare.
    pub before: String,
    /// Le bloc tel qu'il est dans le fichier, balises comprises, indentation ôtée : c'est
    /// ce que le développeur doit remonter, et il peut porter ce que d'autres fragments y
    /// ont déjà posé.
    pub block: String,
}

impl fmt::Display for Misplaced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ancre {} placée sous `{}` dans {}, qu'elle doit précéder",
            self.anchor.opening(),
            self.before,
            self.anchor.file
        )
    }
}

impl std::error::Error for Misplaced {}

/// Vérifie que `anchor` précède, dans `source`, toute ligne commençant par `before`.
///
/// La comparaison se fait sur la ligne sans son indentation, en préfixe : `core:
/// CoreState::new(` reconnaît `core: CoreState::new(db, config),` quel que soit ce que le
/// développeur a passé au constructeur. Une ancre absente n'est pas jugée ici — c'est
/// l'insertion qui la signale, avec le bloc à coller — et une ligne absente non plus : un
/// `state.rs` réécrit sans elle n'a plus le problème que ce contrôle cherche.
pub(crate) fn precedes(source: &str, anchor: &Anchor, before: &str) -> Result<(), Box<Misplaced>> {
    let lines: Vec<&str> = source.lines().collect();
    let position = |balise: &str| lines.iter().position(|line| line.trim() == balise);

    let (Some(opening), Some(closing)) = (position(&anchor.opening()), position(&anchor.closing()))
    else {
        return Ok(());
    };
    let Some(ligne) = lines
        .iter()
        .position(|line| line.trim().starts_with(before))
    else {
        return Ok(());
    };

    if ligne > opening {
        return Ok(());
    }

    Err(Box::new(Misplaced {
        anchor: anchor.clone(),
        before: before.to_string(),
        block: lines[opening..=closing.max(opening)]
            .iter()
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join("\n"),
    }))
}

/// Ce qui empêche de reposer une ancre disparue.
///
/// Chaque variante est une raison de s'abstenir, jamais un échec de la commande : une
/// ancre reposée au mauvais endroit coûte plus cher qu'une ancre laissée absente — un
/// `<rbs:layers>` glissé sous `request_id` cesserait de voir l'identifiant de la requête,
/// et rien ne le dirait avant la lecture d'un journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cause {
    /// Le fichier porteur n'existe pas : il n'y a pas de ligne où s'accrocher.
    FichierAbsent,
    /// L'ancre ne déclare pas d'accroche, sa position dépendant du contenu engendré.
    SansAccroche,
    /// Une des deux balises est encore là.
    ///
    /// Reposer le bloc entier doublerait celle qui reste, et l'endroit d'une balise seule
    /// ne se déduit pas de l'autre : entre les deux, il y a le corps de l'ancre.
    Partielle,
    /// Aucune ligne du fichier ne porte l'accroche.
    Introuvable,
    /// Plusieurs lignes la portent, et rien ne désigne la bonne.
    Ambigue(usize),
}

impl Cause {
    /// Ce qui est dit à l'utilisateur, l'ancre en cause en main.
    pub(crate) fn raison(&self, anchor: &Anchor) -> String {
        match self {
            Self::FichierAbsent => format!("{} est introuvable", anchor.file),
            Self::SansAccroche => {
                format!(
                    "aucune ligne d'accroche n'est déclarée pour {}",
                    anchor.name
                )
            }
            Self::Partielle => format!(
                "{} ou {} est encore là : la balise restante ne dit pas où reposer l'autre",
                anchor.opening(),
                anchor.closing()
            ),
            Self::Introuvable => format!(
                "la ligne d'accroche `{}` est introuvable dans {}",
                anchor.after, anchor.file
            ),
            Self::Ambigue(fois) => format!(
                "la ligne d'accroche `{}` paraît {fois} fois dans {}",
                anchor.after, anchor.file
            ),
        }
    }
}

/// La fin de ligne du fichier hôte, que tout ce qui s'y écrit reprend.
///
/// Le premier saut de ligne décide, et lui seul : la lecture — `marks`, `line_of`,
/// `contains` — passe partout par `trim()`, qui absorbe le `\r`, si bien qu'un fichier
/// CRLF traverse l'insertion sans que rien ne s'en aperçoive. Sur un dépôt en
/// `core.autocrlf=true`, les lignes LF ainsi posées font échouer le `cargo fmt --check`
/// du workflow que le CLI vient lui-même d'engendrer.
///
/// Un fichier aux fins de ligne mélangées n'est pas normalisé : rbs suit la convention
/// qu'il trouve en tête, il n'en impose pas une.
fn eol(source: &str) -> &'static str {
    match source.find('\n') {
        Some(0) | None => "\n",
        Some(rang) if source.as_bytes()[rang - 1] == b'\r' => "\r\n",
        Some(_) => "\n",
    }
}

/// Repose le bloc de `anchor` dans `source`, sous la ligne d'accroche que l'ancre déclare.
///
/// Le bloc est vide : la réparation rend au projet un point d'insertion, elle ne devine
/// pas ce qu'il portait.
pub(crate) fn repose(source: &str, anchor: &Anchor) -> Result<String, Cause> {
    if marks(source, &anchor.opening()) || marks(source, &anchor.closing()) {
        return Err(Cause::Partielle);
    }

    if anchor.after.is_empty() {
        return Err(Cause::SansAccroche);
    }

    let accroches: Vec<usize> = source
        .lines()
        .enumerate()
        .filter(|(_, line)| line.trim() == anchor.after)
        .map(|(index, _)| index)
        .collect();

    let [accroche] = accroches[..] else {
        return Err(match accroches.len() {
            0 => Cause::Introuvable,
            fois => Cause::Ambigue(fois),
        });
    };

    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    let indentation = indentation(&lines[accroche], anchor.pas());

    lines.insert(accroche + 1, format!("{indentation}{}", anchor.closing()));
    lines.insert(accroche + 1, format!("{indentation}{}", anchor.opening()));

    // `lines()` a mangé les `\r` avec les `\n` : les rejoindre par la fin de ligne du
    // fichier la lui rend telle qu'elle était, au lieu de le convertir en LF entier.
    let saut = eol(source);
    let mut rendu = lines.join(saut);
    // `lines()` mange le saut final : le rendre au fichier qui en portait un évite un
    // diff d'une ligne sur un fichier que la réparation n'a fait qu'ouvrir.
    if source.ends_with('\n') {
        rendu.push_str(saut);
    }

    Ok(rendu)
}

/// L'indentation que prend le bloc reposé sous sa ligne d'accroche.
///
/// Une ligne qui ouvre un bloc — `vec![`, `seeds! {`, `services:` — indente d'un cran ce
/// qui la suit ; les autres la partagent. Le pas vient du langage porteur et lui seul
/// (voir `Anchor::pas`) : quatre colonnes en Rust, deux ailleurs, où poser le bloc à côté
/// ferait insérer un service hors de `services:`.
fn indentation(accroche: &str, pas: &str) -> String {
    let propre = accroche.trim();
    let courante = &accroche[..accroche.len() - accroche.trim_start().len()];

    if propre.ends_with(['[', '{', '(', ':']) {
        format!("{courante}{pas}")
    } else {
        courante.to_string()
    }
}

/// Insère `lines` dans `anchor`, juste avant sa balise fermante.
///
/// L'insertion est tout ou rien : `lines` forme une unité — la variante d'une relation et
/// son attribut, les trois lignes d'un `impl Related`, le bloc d'un service compose — et
/// n'est écrite que si l'ancre ne la porte pas déjà en entier, dans cet ordre et d'un
/// seul tenant. Dédupliquer ligne à ligne amputait un bloc de celles qu'un bloc voisin
/// avait déjà déposées : une accolade fermante, un `#[allow(…)]`, une clé `ports:`.
///
/// Le contenu déjà présent traverse l'insertion tel quel : le développeur a pu l'ordonner
/// ou l'indenter à sa façon, et rien ici ne le sait mieux que lui.
pub(crate) fn insert(source: &str, anchor: Anchor, lines: &[String]) -> Result<String, Missing> {
    // Fermeture appelée jusqu'à deux fois : sans le `.clone()`, `Missing { anchor }`
    // consommerait `anchor` dès le premier appel, empêchant le second.
    let absente = || Missing {
        anchor: anchor.clone(),
    };

    let (opening, _) = line_of(source, &anchor.opening()).ok_or_else(absente)?;
    let (closing, indentation) = line_of(source, &anchor.closing()).ok_or_else(absente)?;

    if closing < opening {
        return Err(absente());
    }

    if contains(&source[opening..closing], lines) {
        return Ok(source.to_string());
    }

    let saut = eol(source);

    if anchor.sorted {
        // Le bloc est réécrit entier plutôt que complété : la ligne nouvelle doit pouvoir
        // se glisser entre deux anciennes, ce qu'une insertion avant la balise fermante ne
        // permet pas.
        let debut = debut_du_corps(source, opening, closing);
        let mut corps: Vec<&str> = source[debut..closing]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .chain(lines.iter().map(String::as_str))
            .collect();
        corps.sort_unstable();
        corps.dedup();

        let bloc: String = corps
            .iter()
            .map(|line| format!("{indentation}{line}{saut}"))
            .collect();

        return Ok(format!("{}{bloc}{}", &source[..debut], &source[closing..]));
    }

    let ajouts: String = lines
        .iter()
        .map(|line| format!("{indentation}{line}{saut}"))
        .collect();

    Ok(format!(
        "{}{ajouts}{}",
        &source[..closing],
        &source[closing..]
    ))
}

/// Début du corps d'un bloc dont les balises commencent à `opening` et `closing`.
///
/// `opening` est le début de la ligne de balise ouvrante : le corps commence après le
/// saut de ligne qui la termine.
fn debut_du_corps(source: &str, opening: usize, closing: usize) -> usize {
    source[opening..closing]
        .find('\n')
        .map_or(closing, |fin| opening + fin + 1)
}

/// Rend `source` privé des `lines` que porte le bloc de `anchor`.
///
/// Pas de branche `sorted`, à la différence d'[`insert`] : retirer une ligne d'un bloc
/// trié le laisse trié. La comparaison se fait sur la ligne ébarbée, comme dans
/// [`contains`] — l'indentation appartient au fichier, pas à la déclaration du fragment.
///
/// Les lignes qui restent gardent leur terminaison d'origine telle quelle : le corps est
/// filtré, jamais reformé ligne à ligne, si bien qu'un fichier CRLF n'a pas besoin d'
/// [`eol`] pour le rester — seule une ligne entièrement retirée en perd la sienne.
pub(crate) fn retire(source: &str, anchor: &Anchor, lines: &[String]) -> Result<String, Missing> {
    let absente = || Missing {
        anchor: anchor.clone(),
    };

    let (opening, _) = line_of(source, &anchor.opening()).ok_or_else(absente)?;
    let (closing, _) = line_of(source, &anchor.closing()).ok_or_else(absente)?;

    if closing < opening {
        return Err(absente());
    }

    let debut = debut_du_corps(source, opening, closing);

    let a_retirer: Vec<&str> = lines.iter().map(|line| line.trim()).collect();
    let corps: String = source[debut..closing]
        .split_inclusive('\n')
        .filter(|line| !a_retirer.contains(&line.trim()))
        .collect();

    Ok(format!("{}{corps}{}", &source[..debut], &source[closing..]))
}

/// Ce que l'ancre contient, entre ses deux balises, ou `None` si elle est absente.
///
/// `rbs seed` s'en sert pour distinguer un projet sans seed déclaré d'un projet qui en a :
/// le premier n'a aucune raison de lancer cargo.
pub(crate) fn body(source: &str, anchor: Anchor) -> Option<&str> {
    let (opening, _) = line_of(source, &anchor.opening())?;
    let (closing, _) = line_of(source, &anchor.closing())?;

    if closing < opening {
        return None;
    }

    let apres_ouverture = opening + source[opening..].find('\n').map_or(0, |fin| fin + 1);

    Some(&source[apres_ouverture.min(closing)..closing])
}

/// `source` porte-t-elle une ligne ne portant que `balise` ?
///
/// Seule définition de « la balise est là ». `doctor` l'interroge pour annoncer une ancre
/// présente, `line_of` pour la situer et y écrire : les deux réponses divergeraient
/// autrement, et un projet où l'insertion échoue passerait pour sain.
pub(crate) fn marks(source: &str, balise: &str) -> bool {
    source.lines().any(|line| line.trim() == balise)
}

/// Début de la ligne ne portant que `balise`, et l'indentation de cette ligne.
///
/// La ligne doit ne porter qu'elle : une balise citée dans une chaîne — le bloc à recoller
/// qu'affiche le CLI, par exemple — n'ouvre pas une ancre.
fn line_of(source: &str, balise: &str) -> Option<(usize, String)> {
    let mut debut = 0;

    for line in source.split_inclusive('\n') {
        if marks(line, balise) {
            let indentation = line[..line.len() - line.trim_start().len()].to_string();
            return Some((debut, indentation));
        }
        debut += line.len();
    }

    None
}

/// `lines` figure-t-elle déjà dans le bloc, d'un seul tenant et à l'indentation près ?
///
/// La contiguïté est ce qui rend le prédicat sûr sur un bloc multiligne : deux blocs
/// voisins partagent volontiers une ligne — une accolade fermante, un `#[allow(…)]`, une
/// clé `ports:` — et chercher ces lignes séparément conclurait que le bloc entier est
/// déjà posé.
fn contains(block: &str, lines: &[String]) -> bool {
    if lines.is_empty() {
        return true;
    }

    let present: Vec<&str> = block.lines().map(str::trim).collect();
    let cherchees: Vec<&str> = lines.iter().map(|line| line.trim()).collect();

    present
        .windows(cherchees.len())
        .any(|fenetre| fenetre == cherchees.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIBLIOTHEQUE: &str = "\
pub mod state;
// <rbs:features>
pub mod articles;
// </rbs:features>
";

    const ROUTEUR: &str = "\
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        // <rbs:routes>
        // </rbs:routes>
        .merge(docs)
}
";

    fn lines(sources: &[&str]) -> Vec<String> {
        sources.iter().map(|s| (*s).to_string()).collect()
    }

    /// Rang du premier `\n` que ne précède pas un `\r`, ou `None` si le rendu est tout CRLF.
    fn lf_orphelin(rendu: &str) -> Option<usize> {
        rendu
            .match_indices('\n')
            .map(|(rang, _)| rang)
            .find(|&rang| rang == 0 || rendu.as_bytes()[rang - 1] != b'\r')
    }

    /// Le cas qui a motivé le tri : `add auth` entraîne `rate-limit`, et le CRUD
    /// engendré ensuite s'intercale alphabétiquement entre les deux.
    #[test]
    fn a_sorted_anchor_keeps_its_block_in_order() {
        let source = "// <rbs:features>\npub mod auth;\npub mod rate_limit;\n// </rbs:features>\n";

        let obtenu = insert(source, FEATURES, &["pub mod posts;".to_string()])
            .expect("l'ancre est présente");

        assert_eq!(
            obtenu,
            "// <rbs:features>\npub mod auth;\npub mod posts;\npub mod rate_limit;\n// </rbs:features>\n",
            "le bloc doit rester dans l'ordre que rustfmt impose : {obtenu}"
        );
    }

    /// `add webhooks` pose trois migrations sous un seul horodatage, dans l'ordre où il
    /// installe leurs fragments ; rustfmt, lui, trie les `mod`. Laissé dans l'ordre
    /// d'arrivée, le bloc rendait `cargo fmt --check` rouge, et la CI engendrée avec lui.
    #[test]
    fn the_migration_modules_of_one_command_stay_in_rustfmt_order() {
        let source = "// <rbs:migration_modules>\nmod m20260101_000000_create_users;\n// </rbs:migration_modules>\n";

        let obtenu = insert(
            source,
            MIGRATION_MODULES,
            &lines(&[
                "mod m20260913_130735_create_jobs;",
                "mod m20260913_130735_create_auth_tables;",
                "mod m20260913_130735_create_webhook_subscriptions;",
            ]),
        )
        .expect("l'ancre est présente");

        assert_eq!(
            obtenu,
            "// <rbs:migration_modules>\n\
             mod m20260101_000000_create_users;\n\
             mod m20260913_130735_create_auth_tables;\n\
             mod m20260913_130735_create_jobs;\n\
             mod m20260913_130735_create_webhook_subscriptions;\n\
             // </rbs:migration_modules>\n",
            "le bloc doit suivre l'ordre de rustfmt : {obtenu}"
        );
    }

    #[test]
    fn an_unsorted_anchor_still_stacks_in_arrival_order() {
        let source = "// <rbs:migrations>\nBox::new(m2_b::Migration),\n// </rbs:migrations>\n";

        let obtenu = insert(
            source,
            MIGRATIONS,
            &["Box::new(m1_a::Migration),".to_string()],
        )
        .expect("l'ancre est présente");

        assert!(
            obtenu.contains("Box::new(m2_b::Migration),\nBox::new(m1_a::Migration),"),
            "l'ordre des migrations est chronologique, pas alphabétique : {obtenu}"
        );
    }

    #[test]
    fn the_insertion_lands_just_before_the_closing_tag() {
        let rendered = insert(ROUTEUR, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        assert!(
            rendered.contains(
                "        // <rbs:routes>\n        \
                 .merge(crate::users::routes())\n        // </rbs:routes>"
            ),
            "insertion mal placée :\n{rendered}"
        );
    }

    #[test]
    fn the_indentation_is_that_of_the_closing_tag() {
        let rendered = insert(ROUTEUR, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        let inserted = rendered
            .lines()
            .find(|line| line.contains("users::routes"))
            .expect("la ligne doit être insérée");

        assert_eq!(inserted, "        .merge(crate::users::routes())");
    }

    #[test]
    fn several_lines_keep_the_order_they_are_given_in() {
        let rendered = insert(
            ROUTEUR,
            ROUTES,
            &lines(&["premiere()", "deuxieme()", "troisieme()"]),
        )
        .expect("l'ancre est présente");

        let rangs: Vec<usize> = ["premiere()", "deuxieme()", "troisieme()"]
            .iter()
            .map(|line| rendered.find(line).expect("ligne insérée"))
            .collect();

        assert!(rangs[0] < rangs[1] && rangs[1] < rangs[2], "{rendered}");
    }

    /// Le critère du lot : ce que le développeur a écrit dans l'ancre lui appartient.
    #[test]
    fn the_existing_content_is_neither_reordered_nor_reformatted() {
        let peuple = "\
pub fn router(state: AppState) -> Router {
    Router::new()
        // <rbs:routes>
            .merge(crate::zebres::routes())
        .merge(crate::abeilles::routes())
        // un commentaire du développeur
        // </rbs:routes>
}
";

        let rendered = insert(peuple, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        let expected = peuple.replace(
            "        // </rbs:routes>",
            "        .merge(crate::users::routes())\n        // </rbs:routes>",
        );
        assert_eq!(rendered, expected, "le contenu existant a bougé");
    }

    /// Le fichier hôte décide de la fin de ligne, pas le CLI.
    ///
    /// Sur un dépôt en `core.autocrlf=true`, une ligne LF posée au milieu d'un fichier
    /// CRLF fait échouer le `cargo fmt --check` du workflow que le CLI vient d'engendrer.
    #[test]
    fn the_insertion_follows_the_line_endings_of_the_host_file() {
        let hote = ROUTEUR.replace('\n', "\r\n");

        let rendered = insert(&hote, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        assert!(
            lf_orphelin(&rendered).is_none(),
            "fin de ligne LF au rang {:?} d'un fichier CRLF :\n{rendered:?}",
            lf_orphelin(&rendered)
        );
    }

    /// Le même contrat sur la branche triée, qui réécrit le bloc au lieu de le compléter.
    #[test]
    fn the_sorted_insertion_follows_the_line_endings_of_the_host_file() {
        let hote = BIBLIOTHEQUE.replace('\n', "\r\n");

        let rendered =
            insert(&hote, FEATURES, &lines(&["pub mod users;"])).expect("l'ancre est présente");

        assert!(
            lf_orphelin(&rendered).is_none(),
            "fin de ligne LF au rang {:?} d'un fichier CRLF :\n{rendered:?}",
            lf_orphelin(&rendered)
        );
    }

    /// `repose` reconstruit le fichier entier : sans la règle, il le convertit en LF.
    #[test]
    fn reposing_an_anchor_follows_the_line_endings_of_the_host_file() {
        let hote = ROUTEUR
            .replace("        // <rbs:routes>\n", "")
            .replace("        // </rbs:routes>\n", "")
            .replace('\n', "\r\n");

        let rendered = repose(&hote, &ROUTES).expect("la ligne d'accroche est là");

        assert!(
            lf_orphelin(&rendered).is_none(),
            "fin de ligne LF au rang {:?} d'un fichier CRLF :\n{rendered:?}",
            lf_orphelin(&rendered)
        );
    }

    #[test]
    fn an_already_present_line_is_not_reinserted() {
        let une_fois = insert(ROUTEUR, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        let deux_fois = insert(
            &une_fois,
            ROUTES,
            &lines(&[".merge(crate::users::routes())"]),
        )
        .expect("l'ancre est présente");

        assert_eq!(deux_fois, une_fois, "la seconde insertion a réécrit");
    }

    /// Le cœur de la règle : une séquence n'est réputée posée que si l'ancre la porte en
    /// entier. Une ligne commune à deux blocs ne suffit pas à tenir le second pour écrit.
    #[test]
    fn a_sequence_only_partly_present_is_written_whole() {
        let une_fois = insert(ROUTEUR, ROUTES, &lines(&["deja()"])).expect("l'ancre est présente");

        let rendered = insert(&une_fois, ROUTES, &lines(&["deja()", "nouvelle()"]))
            .expect("l'ancre est présente");

        assert_eq!(rendered.matches("deja()").count(), 2, "{rendered}");
        assert_eq!(rendered.matches("nouvelle()").count(), 1, "{rendered}");
    }

    /// Le défaut que la déduplication ligne à ligne laissait passer : les trois lignes
    /// d'un `impl Related` n'en qualifient aucune, et l'accolade fermante du premier bloc
    /// faisait passer celle du second pour déjà écrite — le fichier sortait avec un
    /// délimiteur non refermé.
    #[test]
    fn a_second_block_sharing_a_closing_brace_keeps_its_own() {
        let source = "\
// <rbs:related>
// </rbs:related>
";
        let premier = insert(
            source,
            RELATED,
            &lines(&[
                "impl Related<crate::profiles::model::Entity> for Entity {",
                "    fn to() -> RelationDef {",
                "        Relation::Profiles.def()",
                "    }",
                "}",
            ]),
        )
        .expect("l'ancre est présente");

        let second = insert(
            &premier,
            RELATED,
            &lines(&[
                "impl Related<crate::notes::model::Entity> for Entity {",
                "    fn to() -> RelationDef {",
                "        Relation::Notes.def()",
                "    }",
                "}",
            ]),
        )
        .expect("l'ancre est présente");

        assert_eq!(
            second.matches('{').count(),
            second.matches('}').count(),
            "les délimiteurs ne s'équilibrent plus :\n{second}"
        );
        assert_eq!(second.matches("impl Related").count(), 2, "{second}");
    }

    /// Deux services YAML qui ouvrent chacun un `ports:` : le second ne doit pas perdre
    /// sa clé sous prétexte qu'un premier fragment en a déjà posé une. Sans le
    /// qualificatif des clés nues dans `groups`, `ports:` — présent dans le bloc depuis le
    /// premier service — se filtrait comme groupe à lui seul, et les deux lignes de liste
    /// du second atterrissaient sans l'en-tête qui les rattache à leur service.
    #[test]
    fn a_bare_key_shared_by_two_services_is_not_dropped_from_the_second() {
        let compose = "\
services:
  # <rbs:services>
  # </rbs:services>
";

        let premier = insert(
            compose,
            SERVICES,
            &lines(&[
                "redis:",
                "  image: redis:8-alpine",
                "  ports:",
                "    - \"6379:6379\"",
            ]),
        )
        .expect("l'ancre est présente");

        let second = insert(
            &premier,
            SERVICES,
            &lines(&[
                "mailpit:",
                "  image: axllent/mailpit:latest",
                "  ports:",
                "    - \"1025:1025\"",
                "    - \"8025:8025\"",
            ]),
        )
        .expect("l'ancre est présente");

        assert!(
            second.contains(
                "  mailpit:\n    image: axllent/mailpit:latest\n    ports:\n      \
                 - \"1025:1025\"\n      - \"8025:8025\""
            ),
            "le second service a perdu sa clé ports: :\n{second}"
        );
    }

    #[test]
    fn a_missing_anchor_is_reported_with_its_file() {
        let error = insert("fn main() {}\n", ROUTES, &lines(&["peu importe"]))
            .expect_err("l'ancre est absente");

        assert_eq!(error.anchor, ROUTES);
        assert_eq!(
            error.to_string(),
            "ancre // <rbs:routes> introuvable dans src/router.rs"
        );
    }

    #[test]
    fn an_anchor_missing_its_closing_is_reported() {
        let tronque = "// <rbs:routes>\n";

        let error =
            insert(tronque, ROUTES, &lines(&["peu importe"])).expect_err("fermeture absente");

        assert_eq!(error.anchor, ROUTES);
    }

    /// La ligne visée s'en va, ses voisines restent.
    #[test]
    fn the_named_line_leaves_and_its_neighbours_stay() {
        let source =
            "// <rbs:routes>\n    .merge(a::routes())\n    .merge(b::routes())\n// </rbs:routes>\n";

        let apres =
            retire(source, &ROUTES, &[".merge(a::routes())".to_string()]).expect("l'ancre est là");

        assert!(!apres.contains("a::routes"));
        assert!(apres.contains("    .merge(b::routes())\n"));
    }

    /// Retirer une ligne absente ne change rien : le retrait est idempotent.
    #[test]
    fn removing_an_absent_line_changes_nothing() {
        let source = "// <rbs:routes>\n    .merge(b::routes())\n// </rbs:routes>\n";

        let apres =
            retire(source, &ROUTES, &[".merge(a::routes())".to_string()]).expect("l'ancre est là");

        assert_eq!(apres, source);
    }

    /// Rien hors de l'ancre n'est touché, fût-ce une ligne identique.
    #[test]
    fn an_identical_line_outside_the_anchor_survives() {
        let source =
            "    .merge(a::routes())\n// <rbs:routes>\n    .merge(a::routes())\n// </rbs:routes>\n";

        let apres =
            retire(source, &ROUTES, &[".merge(a::routes())".to_string()]).expect("l'ancre est là");

        assert_eq!(apres.matches("a::routes").count(), 1);
    }

    /// Un bloc déjà vide n'a rien à perdre : `debut` et `closing` coïncident, et le
    /// retrait ne panique pas sur une tranche vide.
    #[test]
    fn retiring_from_an_already_empty_anchor_changes_nothing() {
        let source = "// <rbs:routes>\n// </rbs:routes>\n";

        let apres =
            retire(source, &ROUTES, &[".merge(a::routes())".to_string()]).expect("l'ancre est là");

        assert_eq!(apres, source);
    }

    /// Une ancre absente est une faute, comme pour l'insertion.
    #[test]
    fn a_missing_anchor_is_reported() {
        retire(
            "pub fn router() {}\n",
            &ROUTES,
            &[".merge(a::routes())".to_string()],
        )
        .expect_err("l'ancre manque");
    }

    /// Le retrait ne touche qu'aux lignes qu'il ôte : les autres gardent la fin de ligne
    /// du fichier hôte, sans passage par `eol` — aucune ligne n'y est reconstruite.
    #[test]
    fn retiring_a_line_follows_the_line_endings_of_the_host_file() {
        let hote = "// <rbs:routes>\r\n    .merge(a::routes())\r\n    .merge(b::routes())\r\n// </rbs:routes>\r\n";

        let apres =
            retire(hote, &ROUTES, &[".merge(a::routes())".to_string()]).expect("l'ancre est là");

        assert!(
            lf_orphelin(&apres).is_none(),
            "fin de ligne LF au rang {:?} d'un fichier CRLF :\n{apres:?}",
            lf_orphelin(&apres)
        );
    }

    /// Une occurrence citée dans du code — une chaîne, un message d'erreur — n'ouvre pas
    /// une ancre : seule une ligne qui ne porte qu'elle en est une.
    #[test]
    fn a_tag_quoted_mid_line_is_not_an_anchor() {
        let cite = "let aide = \"ajoute // <rbs:routes> puis // </rbs:routes>\";\n";

        let error = insert(cite, ROUTES, &lines(&["peu importe"])).expect_err("aucune ancre");

        assert_eq!(error.anchor, ROUTES);
    }

    /// `src/state.rs` d'un projet d'avant 1.5.0 : l'ancre suit `core:`, qui a déjà
    /// consommé `config`.
    const ETAT_ANCIEN: &str = "impl AppState {\n    pub fn new(db: DatabaseConnection, config: Config) -> anyhow::Result<Self> {\n        Ok(Self {\n            core: CoreState::new(db, config),\n            // <rbs:state_init>\n            mail: crate::modules::mail::Mailer::from_config()?,\n            // </rbs:state_init>\n        })\n    }\n}\n";

    /// Le même, tel que le squelette le rend depuis 1.5.0.
    const ETAT_COURANT: &str = "impl AppState {\n    pub fn new(db: DatabaseConnection, config: Config) -> anyhow::Result<Self> {\n        Ok(Self {\n            // <rbs:state_init>\n            mail: crate::modules::mail::Mailer::from_config()?,\n            // </rbs:state_init>\n            core: CoreState::new(db, config),\n        })\n    }\n}\n";

    const CORE: &str = "core: CoreState::new(";

    #[test]
    fn an_anchor_below_the_line_it_must_precede_is_misplaced_with_its_block() {
        let error =
            precedes(ETAT_ANCIEN, &STATE_INIT, CORE).expect_err("l'ancre est sous la ligne");

        assert_eq!(error.anchor, STATE_INIT);
        assert_eq!(error.before, CORE);
        assert_eq!(
            error.block,
            "// <rbs:state_init>\nmail: crate::modules::mail::Mailer::from_config()?,\n// </rbs:state_init>"
        );
        assert_eq!(
            error.to_string(),
            "ancre // <rbs:state_init> placée sous `core: CoreState::new(` dans src/state.rs, \
             qu'elle doit précéder"
        );
    }

    #[test]
    fn an_anchor_above_the_line_is_in_its_place() {
        precedes(ETAT_COURANT, &STATE_INIT, CORE).expect("l'ancre précède la ligne");
    }

    /// Ni l'ancre ni la ligne absentes ne sont l'affaire de ce contrôle : la première est
    /// signalée par l'insertion, la seconde est un fichier réécrit que rien ici ne sait
    /// juger.
    #[test]
    fn a_missing_anchor_or_a_missing_line_is_not_a_misplacement() {
        precedes("fn main() {}\n", &STATE_INIT, CORE).expect("ni ancre ni ligne");

        let sans_ligne = ETAT_ANCIEN.replace("core: CoreState::new(db, config),", "core,");
        precedes(&sans_ligne, &STATE_INIT, CORE).expect("la ligne est absente");
    }

    /// Le prédicat qu'interroge `doctor` répond comme l'insertion : indentation tolérée,
    /// citation refusée.
    #[test]
    fn marks_accepts_an_indented_tag_and_refuses_a_quoted_one() {
        assert!(marks("    // <rbs:routes>\n", "// <rbs:routes>"));
        assert!(marks(ROUTEUR, "// <rbs:routes>"));
        assert!(!marks(
            "let aide = \"// <rbs:routes>\";\n",
            "// <rbs:routes>"
        ));
        assert!(!marks("", "// <rbs:routes>"));
    }

    #[test]
    fn an_untouched_anchor_has_an_empty_body() {
        let body = body(ROUTEUR, ROUTES).expect("l'ancre est présente");

        assert!(body.trim().is_empty(), "corps inattendu : {body:?}");
    }

    #[test]
    fn a_filled_anchor_gives_back_what_was_inserted() {
        let rempli = insert(ROUTEUR, ROUTES, &lines(&[".merge(crate::users::routes())"]))
            .expect("l'ancre est présente");

        let body = body(&rempli, ROUTES).expect("l'ancre est présente");

        assert!(body.contains(".merge(crate::users::routes())"), "{body:?}");
        assert!(
            !body.contains("<rbs:routes>"),
            "les balises ne font pas partie du corps : {body:?}"
        );
    }

    #[test]
    fn a_missing_anchor_has_no_body() {
        assert_eq!(body("fn main() {}\n", ROUTES), None);
    }

    #[test]
    fn the_block_to_paste_carries_both_tags_of_the_anchor() {
        assert_eq!(ROUTES.block(), "// <rbs:routes>\n// </rbs:routes>");
    }

    #[test]
    fn the_anchors_carry_distinct_names() {
        for (rang, anchor) in ANCRES.iter().enumerate() {
            assert!(
                !ANCRES[..rang].iter().any(|other| other.name == anchor.name),
                "`{}` déclarée deux fois",
                anchor.name
            );
        }
    }

    /// Deux fragments peuvent déclarer une même ligne — un attribut, le plus souvent —
    /// sans que le bloc de l'un rende celui de l'autre superflu. Dédupliquer ligne à
    /// ligne amputait le second de sa ligne commune et laissait le reste orphelin.
    #[test]
    fn the_block_is_written_whole_when_only_one_of_its_lines_is_already_there() {
        let source = "\
struct AppState {
    // <rbs:state_champs>
    #[allow(dead_code)]
    pub mail: Mailer,
    // </rbs:state_champs>
}
";

        let after = insert(
            source,
            STATE_CHAMPS,
            &[
                "#[allow(dead_code)]".to_string(),
                "pub storage: Arc<dyn Storage>,".to_string(),
            ],
        )
        .expect("l'ancre est présente");

        assert_eq!(
            after.matches("#[allow(dead_code)]").count(),
            2,
            "chaque champ porte le sien : {after}"
        );
        assert!(
            after.contains("pub storage: Arc<dyn Storage>,"),
            "le champ ne doit pas être laissé de côté : {after}"
        );
    }

    #[test]
    fn a_yaml_anchor_is_written_with_a_hash() {
        let compose = Anchor {
            name: Cow::Borrowed("services"),
            file: Cow::Borrowed("docker-compose.yml"),
            comment: "#",
            sorted: false,
            optional: true,
            after: "services:",
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
            name: Cow::Borrowed("services"),
            file: Cow::Borrowed("docker-compose.yml"),
            comment: "#",
            sorted: false,
            optional: true,
            after: "services:",
        };
        let source = "services:\n  # <rbs:services>\n  # </rbs:services>\n";
        let lines = vec!["# le cache du projet".to_string(), "redis:".to_string()];

        let apres = insert(source, compose.clone(), &lines).expect("l'ancre est présente");

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

    /// Le cas asymétrique, seul à distinguer les deux comportements : le commentaire est
    /// déjà dans l'ancre, la ligne qu'il qualifie ne l'est pas encore. Sans le
    /// groupement, le commentaire passerait pour posé et le service s'insérerait seul,
    /// sous un commentaire qui ne le concerne pas.
    #[test]
    fn a_yaml_comment_already_present_does_not_orphan_the_line_it_qualifies() {
        let compose = Anchor {
            name: Cow::Borrowed("services"),
            file: Cow::Borrowed("docker-compose.yml"),
            comment: "#",
            sorted: false,
            optional: true,
            after: "services:",
        };
        // Une autre feature a posé ce commentaire, et un service qui l'en sépare.
        let source = "services:\n  # <rbs:services>\n  # le cache du projet\n  memcached:\n  # </rbs:services>\n";
        let lines = vec!["# le cache du projet".to_string(), "redis:".to_string()];

        let apres = insert(source, compose, &lines).expect("l'ancre est présente");

        assert!(
            apres.contains("  # le cache du projet\n  redis:\n"),
            "le service doit arriver avec son propre commentaire :\n{apres}"
        );
        assert!(
            !apres.contains("  memcached:\n  redis:\n"),
            "le service ne doit pas s'insérer nu sous un commentaire étranger :\n{apres}"
        );
    }

    // `SERVICES` étant un `const`, clippy évalue `SERVICES.optional` à la compilation et
    // signale l'assertion comme triviale ; elle mord pourtant si quelqu'un change le
    // champ, ce que clippy ne voit pas.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn the_services_anchor_lives_in_the_compose_and_is_optional() {
        assert_eq!(SERVICES.file, "docker-compose.yml");
        assert_eq!(SERVICES.comment, "#");
        assert!(SERVICES.optional);
        assert!(ANCRES.contains(&SERVICES));
    }

    /// Les deux ancres du routeur partagent leur fichier et ne se confondent pas : une
    /// ligne montée dans `routes` n'enveloppe rien, une couche posée dans `layers`
    /// n'expose aucune route.
    #[test]
    fn the_router_carries_the_routes_and_the_layers_anchors_apart() {
        assert_eq!(LAYERS.file, ROUTES.file);
        assert_ne!(LAYERS.name, ROUTES.name);
        assert_eq!(LAYERS.opening(), "// <rbs:layers>");
        assert!(ANCRES.contains(&LAYERS));
    }

    /// Une ancre optionnelle est l'exception : les onze autres décrivent un fichier que le
    /// squelette écrit toujours et que rien n'invite à supprimer, et leur absence est un
    /// défaut. Neuf des dix qui le sont vivent dans un fichier qu'un fragment dépose — le
    /// point de montage des `modules`, le compose de `docker`, le registre de `jobs`, la
    /// liste de ses modules, le calendrier du `scheduler`, l'implémentation
    /// d'authentification, le relais du serveur de développement du client, la table de
    /// routage et le rail du shell d'administration — et manquent légitimement à qui n'a
    /// pas installé ce fragment.
    ///
    /// `ignore` est la seule à l'être pour une autre raison : le squelette écrit bien le
    /// fichier d'exclusions, mais celui-ci appartient au développeur, qui peut l'avoir
    /// supprimé — un dépôt dont le `.gitignore` vit à la racine d'un monorepo, par
    /// exemple. Réclamer l'ancre ferait passer ce projet pour incomplet.
    #[test]
    fn an_optional_anchor_is_either_deposited_by_a_fragment_or_the_developer_s_to_delete() {
        let optionnelles: Vec<&str> = ANCRES
            .iter()
            .filter(|anchor| anchor.optional)
            .map(|anchor| anchor.name.as_ref())
            .collect();

        assert_eq!(
            optionnelles,
            [
                "modules",
                "services",
                "ignore",
                "jobs",
                "job_modules",
                "schedules",
                "auth_impl",
                "vite_proxy",
                "admin_routes",
                "admin_rail"
            ]
        );
    }

    /// Les trois ancres du frontend, et trois seulement : une pour le relais du serveur de
    /// développement, deux pour l'espace d'administration — en TypeScript, l'import qui
    /// donne le composant à la route est la déclaration du module, et aucune quatrième
    /// n'est nécessaire pour la déclarer.
    ///
    /// Rien n'empêcherait d'en poser une de plus — pour un fichier de libellés partagé,
    /// pour un registre de modules — et c'est précisément ce que ce contrôle garde :
    /// chaque ancre est une condition de plus pour qu'un projet reste générable, et une
    /// ligne de plus dans ce que `doctor` parcourt.
    #[test]
    fn the_client_carries_three_anchors_and_three_only() {
        let frontend: Vec<&str> = ANCRES
            .iter()
            .filter(|anchor| anchor.file.starts_with("frontend/"))
            .map(|anchor| anchor.name.as_ref())
            .collect();

        assert_eq!(frontend, ["vite_proxy", "admin_routes", "admin_rail"]);
        assert_eq!(VITE_PROXY.file, "frontend/vite.config.ts");
        assert_eq!(ADMIN_ROUTES.file, "frontend/src/admin/montage.ts");
        assert_eq!(ADMIN_RAIL.file, "frontend/src/admin/rail.ts");

        // Dans un module TypeScript, et non dans le `<template>` de la coquille : le
        // mécanisme ne sait ouvrir une ancre que derrière `//` ou `#`. Les trois prennent
        // `//` : un `#` ouvre un commentaire en YAML et dans un fichier d'exclusions,
        // jamais en TypeScript, où le fichier cesserait de se construire.
        for anchor in [VITE_PROXY, ADMIN_ROUTES, ADMIN_RAIL] {
            assert_eq!(anchor.comment, "//");
            assert!(anchor.file.ends_with(".ts"), "{}", anchor.file);
            assert!(anchor.optional, "{}", anchor.name);
        }
    }

    /// Les deux seules ancres du registre à porter le marqueur `#`, et les deux seules à
    /// vivre hors d'un fichier `.rs` ou `.ts`.
    ///
    /// Le contrôle tient la règle dans les deux sens : un `#` posé dans un fichier que
    /// TypeScript construit n'y ouvre pas un commentaire, et le relais du client — la
    /// tentation, l'ancre ayant d'abord été écrite ainsi — arrêterait `npm run build`.
    #[test]
    fn only_the_compose_and_the_exclusions_carry_the_hash_marker() {
        let dieses: Vec<&str> = ANCRES
            .iter()
            .filter(|anchor| anchor.comment == "#")
            .map(|anchor| anchor.name.as_ref())
            .collect();

        assert_eq!(dieses, ["services", "ignore"]);
        for anchor in ANCRES {
            if anchor.file.ends_with(".ts") || anchor.file.ends_with(".rs") {
                assert_eq!(anchor.comment, "//", "{}", anchor.file);
            }
        }
    }

    /// La documentation nomme les vingt-et-une ancres, et aucune autre, dans les quatre
    /// pages qui en dressent la liste.
    ///
    /// C'est la promesse de compatibilité qui rend ce contrôle nécessaire : elle porte sur
    /// les noms d'ancres et leur syntaxe, et une page qui en oublierait une la rendrait
    /// fausse. Rien ne le signalait — le jalon du frontend a laissé « dix-huit ancres » sur
    /// cinq pages dans deux langues pendant tout un lot, sans qu'une suite ne bronche.
    ///
    /// Les deux ancres d'un modèle sont attendues en plus : leur nom porte la table, elles
    /// sortent du registre, et les pages le disent.
    #[test]
    fn the_documentation_names_every_anchor_and_no_other() {
        /// Les ancres d'un modèle, hors du registre parce que leur nom porte la table.
        const HORS_REGISTRE: [&str; 2] = ["relations:table", "related:table"];

        const PAGES: [&str; 4] = [
            "docs/docs/compatibility.md",
            "docs/docs/cli/doctor.md",
            "docs/i18n/fr/docusaurus-plugin-content-docs/current/compatibility.md",
            "docs/i18n/fr/docusaurus-plugin-content-docs/current/cli/doctor.md",
        ];

        let depot = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut attendues: Vec<&str> = ANCRES
            .iter()
            .map(|anchor| anchor.name.as_ref())
            .chain(HORS_REGISTRE)
            .collect();
        attendues.sort_unstable();

        for page in PAGES {
            let source = std::fs::read_to_string(depot.join(page))
                .unwrap_or_else(|faute| panic!("{page} doit se lire : {faute}"));

            let mut nommees: Vec<&str> = source
                .split("<rbs:")
                .skip(1)
                .filter_map(|reste| reste.split('>').next())
                .collect();
            nommees.sort_unstable();
            nommees.dedup();

            assert_eq!(
                nommees, attendues,
                "{page} ne nomme pas les mêmes ancres que le registre"
            );
        }
    }

    /// L'ancre des exclusions est la seule, avec celle du compose, à ne pas vivre dans du
    /// Rust : son marqueur de commentaire est celui de Git, et le bloc à coller doit sortir
    /// avec ce marqueur-là.
    // `IGNORE` étant un `const`, clippy évalue `.optional` à la compilation et signale
    // l'assertion comme triviale ; elle mord pourtant si quelqu'un change le champ.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn the_ignore_anchor_lives_in_the_exclusions_with_the_git_comment_marker() {
        assert_eq!(IGNORE.file, ".gitignore");
        assert_eq!(IGNORE.comment, "#");
        assert_eq!(IGNORE.opening(), "# <rbs:ignore>");
        assert_eq!(IGNORE.block(), "# <rbs:ignore>\n# </rbs:ignore>");
        assert!(IGNORE.optional);
        assert!(
            !IGNORE.sorted,
            "l'ordre d'un fichier d'exclusions porte du sens"
        );
        assert!(ANCRES.contains(&IGNORE));
    }

    /// Sans elle, un fragment ne peut pas inscrire de job : le worker n'exécute que ce que
    /// `registry()` connaît, et `add` refuse tout nom d'ancre hors du registre.
    // `JOBS` étant un `const`, clippy évalue `.optional` à la compilation et signale
    // l'assertion comme triviale ; elle mord pourtant si quelqu'un change le champ.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn the_jobs_anchor_lives_in_the_queue_registry_and_is_optional() {
        assert_eq!(JOBS.file, "src/modules/jobs/mod.rs");
        assert_eq!(JOBS.opening(), "// <rbs:jobs>");
        assert!(JOBS.optional);
        assert!(ANCRES.contains(&JOBS));
    }

    /// L'ancre vit dans un fichier que le fragment `auth` dépose : un projet sans `auth`
    /// n'a pas ce fichier, et `doctor` ne doit pas le tenir pour incomplet.
    // `AUTH_IMPL` étant un `const`, clippy évalue `.optional` et `.sorted` à la
    // compilation et signale les assertions comme triviales ; elles mordent pourtant si
    // quelqu'un change ces champs.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn the_auth_impl_anchor_is_optional_and_lives_in_the_auth_module() {
        assert!(ANCRES.contains(&AUTH_IMPL));
        assert!(AUTH_IMPL.optional);
        assert!(!AUTH_IMPL.sorted);
        assert_eq!(AUTH_IMPL.file, "src/auth/mod.rs");
        assert_eq!(AUTH_IMPL.comment, "//");
    }

    /// L'accroche d'une ancre effacée doit exister dans la template qui la porte, sans quoi
    /// `doctor --fix` n'a aucun endroit où la reposer.
    #[test]
    fn the_auth_impl_hook_is_a_line_of_its_own_template() {
        let gabarit = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/features/auth/mod.rs.jinja"
        ))
        .expect("le gabarit du fragment auth se lit");

        assert_eq!(
            gabarit
                .lines()
                .filter(|ligne| ligne.trim() == AUTH_IMPL.after)
                .count(),
            1,
            "l'accroche doit être présente une fois et une seule"
        );
        assert!(gabarit.contains(&AUTH_IMPL.opening()));
        assert!(gabarit.contains(&AUTH_IMPL.closing()));
    }

    /// L'ancre des modules vit dans un fichier que le squelette ne pose pas : la déclarer
    /// obligatoire ferait passer pour incomplet tout projet sans fragment.
    #[test]
    fn the_modules_anchor_is_optional_and_sorted() {
        let modules = ANCRES
            .into_iter()
            .find(|anchor| anchor.name == "modules")
            .expect("le registre porte l'ancre des modules");

        assert_eq!(modules.file, "src/modules/mod.rs");
        assert!(modules.optional, "son fichier n'existe pas sans fragment");
        assert!(
            modules.sorted,
            "rustfmt trie les `pub mod` du point de montage"
        );
    }

    /// L'accroche d'une ancre est vérifiée contre la template qui la porte, et celle-ci
    /// vit sous `features/` : le balayage du squelette ne la rencontre jamais.
    ///
    /// La ligne qui suit l'accroche doit être la balise ouvrante elle-même, et non
    /// seulement la précéder quelque part dans le fichier : c'est ce qui rend `doctor
    /// --fix` exact à l'octet quand il repose une ancre effacée.
    #[test]
    fn each_optional_anchor_of_the_registry_hooks_directly_under_its_line() {
        for (anchor, template) in [
            (JOBS, "jobs/mod.rs.jinja"),
            (JOB_MODULES, "jobs/mod.rs.jinja"),
            (SCHEDULES, "scheduler/mod.rs.jinja"),
            (VITE_PROXY, "frontend/client/vite.config.ts.jinja"),
        ] {
            let source = std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("templates/features/{template}")),
            )
            .unwrap_or_else(|_| panic!("la template {template} doit se lire"));

            assert_eq!(
                source.matches(&anchor.opening()).count(),
                1,
                "{} : {source}",
                anchor.name
            );
            assert_eq!(
                source.matches(&anchor.closing()).count(),
                1,
                "{} : {source}",
                anchor.name
            );
            assert_eq!(
                source.matches(anchor.after).count(),
                1,
                "{} : l'accroche doit paraître une fois exactement",
                anchor.name
            );

            let ligne_suivante = source
                .lines()
                .skip_while(|ligne| !ligne.contains(anchor.after))
                .nth(1)
                .unwrap_or_else(|| panic!("{} : aucune ligne après l'accroche", anchor.name));
            assert_eq!(
                ligne_suivante.trim(),
                anchor.opening(),
                "{} : la ligne qui suit l'accroche doit être la balise ouvrante",
                anchor.name
            );
        }
    }

    /// La sonde d'une dépendance vit dans le projet, et le fragment qui l'installe a
    /// besoin d'un endroit où l'inscrire : sans cette ancre, `/health` ne contrôlerait
    /// jamais que la base.
    // `HEALTH_PROBES` étant un `const`, clippy évalue `.sorted` à la compilation et
    // signale l'assertion comme triviale ; elle mord pourtant si quelqu'un change le
    // champ, ce que clippy ne voit pas.
    #[allow(clippy::assertions_on_constants)]
    #[test]
    fn the_health_probes_anchor_lives_in_the_health_controller() {
        assert_eq!(HEALTH_PROBES.file, "src/health/controller.rs");
        assert_eq!(HEALTH_PROBES.opening(), "// <rbs:health_probes>");
        assert!(!HEALTH_PROBES.sorted);
        assert!(ANCRES.contains(&HEALTH_PROBES));
    }

    #[test]
    fn an_anchor_can_be_rebound_to_a_computed_file() {
        let anchor = RELATIONS.in_file("src/posts/model.rs");

        assert_eq!(anchor.file, "src/posts/model.rs");
        assert_eq!(anchor.name, RELATIONS.name);
        assert_eq!(anchor.opening(), "// <rbs:relations>");
    }

    /// Le nom de l'entité rejoint celui de l'ancre : `src/auth/model.rs` porte deux
    /// paires, et une relation vers la seconde entité n'a que ce nom pour la viser.
    #[test]
    fn a_model_anchor_carries_the_name_of_its_entity() {
        let anchor = RELATIONS.for_entity("src/auth/model.rs", "refresh_tokens");

        assert_eq!(anchor.file, "src/auth/model.rs");
        assert_eq!(anchor.opening(), "// <rbs:relations:refresh_tokens>");
        assert_eq!(anchor.closing(), "// </rbs:relations:refresh_tokens>");
    }

    #[test]
    fn two_entities_of_one_file_get_two_distinct_anchors() {
        let users = RELATED.for_entity("src/auth/model.rs", "users");
        let tokens = RELATED.for_entity("src/auth/model.rs", "refresh_tokens");

        assert_eq!(users.file, tokens.file);
        assert_ne!(users.name, tokens.name);
    }

    // Les deux ancres du modèle ne rejoignent pas le registre statique : leur fichier
    // dépend des features du projet, que `doctor` énumère autrement.
    #[test]
    fn the_model_anchors_are_absent_from_the_static_registry() {
        for anchor in ANCRES {
            assert_ne!(anchor.name, "relations", "{:?}", anchor);
            assert_ne!(anchor.name, "related", "{:?}", anchor);
        }
    }

    #[test]
    fn the_features_anchor_resolves_to_the_library_when_it_exists() {
        let project = tempfile::TempDir::new().expect("répertoire temporaire créable");
        std::fs::create_dir_all(project.path().join("src")).expect("le répertoire se crée");
        std::fs::write(project.path().join("src/lib.rs"), "// bibliothèque")
            .expect("l'écriture aboutit");

        let anchor = resolve_features(project.path());

        assert_eq!(anchor.file, "src/lib.rs");
        assert_eq!(anchor.name, FEATURES.name);
    }

    #[test]
    fn the_features_anchor_falls_back_to_main_without_a_library() {
        let project = tempfile::TempDir::new().expect("répertoire temporaire créable");

        let anchor = resolve_features(project.path());

        assert_eq!(anchor.file, "src/main.rs");
    }

    /// Une ancre ajoutée au registre paraît dans la liste résolue sans que personne ait à
    /// toucher les appelants : c'est ce que la liste unique achète.
    #[test]
    fn the_resolved_registry_carries_every_anchor_of_the_registry() {
        let project = tempfile::TempDir::new().expect("répertoire temporaire créable");

        let resolues = resolved(project.path());

        assert_eq!(resolues.len(), ANCRES.len());
        for anchor in ANCRES {
            assert!(
                resolues.iter().any(|autre| autre.name == anchor.name),
                "`{}` manque à la liste résolue",
                anchor.name
            );
        }
    }

    #[test]
    fn the_resolved_registry_follows_the_fallback_of_the_features_anchor() {
        let project = tempfile::TempDir::new().expect("répertoire temporaire créable");
        std::fs::create_dir_all(project.path().join("src")).expect("le répertoire se crée");
        std::fs::write(project.path().join("src/lib.rs"), "// bibliothèque")
            .expect("l'écriture aboutit");

        let features = resolved(project.path())
            .into_iter()
            .find(|anchor| anchor.name == FEATURES.name)
            .expect("l'ancre des features est au registre");

        assert_eq!(features.file, "src/lib.rs");
    }
}
