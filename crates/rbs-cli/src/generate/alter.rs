//! `rbs generate migration` : une migration d'évolution du schéma.
//!
//! La migration de création écrit une table entière ; celle-ci en modifie une qui existe
//! déjà — une colonne de plus par champ de `--fields`, retirée dans l'ordre inverse à la
//! descente. La séquence est celle de `generate crud` : les champs, la table et le moteur
//! sont jugés avant le rendu, et le premier octet n'est écrit qu'une fois le plan entier.
//!
//! Le CLI ne réécrit pas d'AST : `model.rs` et `dto.rs` n'ont pas d'ancre, et les lignes
//! qui leur reviennent sont affichées plutôt qu'insérées.

use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::Codee;
use crate::git;
use crate::metadata;
use crate::plan;
use crate::template::Renderer;

use super::feature::Feature;
use super::fields::{Field, FieldType, to_pascal_case};
use super::{entities, fields, format, mount, name};

const TEMPLATE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/templates/migration/alter.rs.jinja"
));

/// Ce qu'il faut savoir pour écrire une migration d'évolution.
pub(crate) struct Options {
    /// Nom de la migration, en snake_case : celui de son module.
    pub name: String,
    /// Table à modifier, telle que le projet la déclare.
    pub table: String,
    /// Colonnes à ajouter, telles que `--fields` les donne.
    pub fields: String,
    /// Répertoire d'où la commande est lancée.
    pub directory: PathBuf,
    /// Écrit même si le projet porte des modifications non commitées.
    pub force: bool,
}

/// Un bloc que la commande affiche sans l'écrire : le fichier visé, et ses lignes.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Bloc {
    /// Fichier où coller, relatif à la racine du projet.
    pub fichier: String,
    /// Ce qu'il y a à coller, tel quel.
    pub lignes: Vec<String>,
}

/// Ce que la commande fera au projet, entièrement calculé et rien d'écrit.
#[derive(Debug)]
pub(crate) struct Planned {
    /// Le plan, à afficher puis à appliquer.
    pub plan: plan::Plan,
    /// Module de la migration écrite : `m20260916_101500_ajoute_statut`.
    pub module: String,
    /// Chemin du fichier de migration, relatif à la racine du projet.
    pub fichier: String,
    /// Les blocs que le développeur reporte lui-même dans son modèle et ses DTO.
    pub blocs: Vec<Bloc>,
    /// Ce que rustfmt n'a pas pu faire sur le rendu, s'il y a lieu.
    pub avertissement: Option<format::Avertissement>,
}

/// Ce qui peut empêcher d'écrire une migration d'évolution.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    /// La commande n'a pas été lancée dans un projet rbs.
    #[error("aucun projet rbs ici : `rbs generate` s'exécute dans un projet créé par `rbs new`")]
    PasUnProjet,

    /// Le manifeste du projet n'a pu être lu.
    #[error("{0}")]
    Metadata(#[from] metadata::Error),

    /// Un fichier du projet n'a pu être lu ou écrit.
    #[error(transparent)]
    Acces(#[from] crate::errors::Acces),

    /// Le projet porte des modifications non commitées, qu'une écriture rendrait
    /// indiscernables des siennes.
    #[error(transparent)]
    WorkingTreeSale(#[from] crate::errors::WorkingTreeSale),

    /// Le nom ne fait pas un module Rust.
    ///
    /// `DeriveMigrationName` tire de ce nom celui de la migration en base : un nom que
    /// rustc refuse ne se verrait qu'à la compilation, le fichier déjà écrit.
    #[error("{0}")]
    Nom(name::NameError),

    /// Les champs ne s'analysent pas.
    #[error("{0}")]
    Fields(fields::FieldsError),

    /// `--fields` ne déclare aucune colonne.
    ///
    /// Le parseur rend un vecteur vide sans se plaindre, et la migration rendue n'altère
    /// alors rien — tout en s'écrivant, en s'inscrivant aux deux ancres et en se comptant
    /// dans `migrate status`. C'est ce que rend `rbs migrate new`, à ceci près que
    /// l'utilisateur ne l'a pas demandé.
    #[error(
        "`--fields` ne déclare aucune colonne : la migration rendue n'altérerait rien. \
         Déclarez les colonnes à ajouter, ou ouvrez une migration vide à écrire à la main \
         par `rbs migrate new {name}`"
    )]
    ChampsVides {
        /// Nom de la migration, tel qu'il a été demandé.
        name: String,
    },

    /// Aucun module du projet ne déclare cette table.
    #[error(
        "aucune entité du projet ne déclare la table « {table} » — cherchée dans \
         src/*/model.rs, qui déclare : {connues}"
    )]
    TableSansModule {
        /// Table demandée en ligne de commande.
        table: String,
        /// Tables que le projet déclare, énumérées.
        connues: String,
    },

    /// La table porte déjà une colonne de ce nom.
    ///
    /// Sans ce refus, l'échec vient du moteur au `migrate up`, la migration déjà écrite
    /// et déjà inscrite aux deux ancres — et son message ne dit pas quoi en faire.
    #[error(
        "le champ `{champ}` nomme une colonne que la table « {table} » porte déjà : \
         {fichier} la déclare. Nommez-en une autre, ou modifiez celle qui existe par une \
         migration écrite à la main — `rbs migrate new` en ouvre une"
    )]
    ColonneDejaDeclaree {
        /// Nom du champ fautif, tel qu'il a été déclaré.
        champ: String,
        /// Table visée, telle qu'elle a été demandée.
        table: String,
        /// Fichier qui atteste la colonne, relatif à la racine du projet.
        fichier: String,
    },

    /// Une colonne ajoutée à une table peuplée doit admettre le nul.
    ///
    /// SQLite exige une valeur par défaut pour une colonne `NOT NULL` ajoutée ; les deux
    /// autres moteurs refusent l'ajout dès qu'une ligne existe. Le défaut n'est pas
    /// proposé : il vaudrait pour les lignes anciennes comme pour les nouvelles, et ce
    /// choix-là appartient au schéma, pas au CLI.
    #[error(
        "le champ `{champ}` n'est pas optionnel : une colonne ajoutée à une table qui \
         porte déjà des lignes n'a pas de valeur pour elles. Déclarez `{champ}:…:optional`, \
         ou donnez-lui sa valeur par une migration écrite à la main"
    )]
    ColonneObligatoire {
        /// Nom du champ fautif, tel qu'il a été déclaré.
        champ: String,
    },

    /// `unique` sur une colonne ajoutée : SQLite ne sait pas l'ajouter.
    ///
    /// Le refus tient sur les trois moteurs, PostgreSQL compris : une migration engendrée
    /// s'applique partout, et une règle est une règle.
    #[error(
        "`unique` sur le champ `{champ}` : SQLite refuse d'ajouter une colonne sous \
         contrainte d'unicité, et une migration engendrée doit s'appliquer sur les trois \
         moteurs. Retirez le modificateur, puis posez l'index unique par une migration \
         écrite à la main — `rbs migrate new` en ouvre une"
    )]
    UniqueSurColonneAjoutee {
        /// Nom du champ fautif, tel qu'il a été déclaré.
        champ: String,
    },

    /// `references` sur une colonne ajoutée : SQLite ne sait pas ajouter de clé étrangère.
    #[error(
        "le champ `{champ}` est une `references` : SQLite ne sait pas ajouter de clé \
         étrangère à une table existante, et une migration engendrée doit s'appliquer sur \
         les trois moteurs. Ajoutez la colonne en `uuid:optional`, puis posez la contrainte \
         par une migration écrite à la main — `rbs migrate new` en ouvre une"
    )]
    ReferenceInterdite {
        /// Nom du champ fautif, tel qu'il a été déclaré.
        champ: String,
    },

    /// Un champ `decimal` sur un projet SQLite, dont le pilote ne lie aucun décimal exact.
    #[error("{}", crate::errors::decimal_sous_sqlite(champ))]
    DecimalSousSqlite {
        /// Nom du champ fautif, tel qu'il a été déclaré.
        champ: String,
    },

    /// La template de la migration ne s'est pas rendue.
    #[error("{file} ne se rend pas : {source}")]
    Rendu {
        /// Fichier fautif.
        file: String,
        /// Cause du moteur de rendu.
        source: minijinja::Error,
    },

    /// Le plan de l'écriture n'a pu être calculé.
    #[error("{0}")]
    Plan(#[from] plan::Error),

    /// Le plan n'a pu être appliqué au projet.
    #[error("{0}")]
    Application(#[from] plan::application::Error),
}

// Une faute du manifeste se nomme ; seule son absence vaut « pas un projet rbs ».
crate::errors::depuis_la_racine!(Error);

impl Codee for Error {
    fn code(&self) -> &'static str {
        match self {
            Error::PasUnProjet => "pas_un_projet",
            Error::Metadata(_) => "manifeste_illisible",
            Error::Acces(_) => "fichier_inaccessible",
            Error::WorkingTreeSale(_) => "arbre_sale",
            Error::Nom(_) => "nom_invalide",
            Error::Fields(_) => "champs_invalides",
            Error::ChampsVides { .. } => "champs_vides",
            Error::TableSansModule { .. } => "table_sans_module",
            Error::ColonneDejaDeclaree { .. } => "colonne_deja_declaree",
            Error::ColonneObligatoire { .. } => "colonne_obligatoire",
            Error::UniqueSurColonneAjoutee { .. } => "unique_sur_colonne_ajoutee",
            Error::ReferenceInterdite { .. } => "reference_interdite",
            Error::DecimalSousSqlite { .. } => "decimal_sous_sqlite",
            Error::Rendu { .. } => "rendu_impossible",
            Error::Plan(erreur) => erreur.code(),
            Error::Application(erreur) => erreur.code(),
        }
    }

    /// Les refus de cette commande disent tous dans leur message le geste qui les lève :
    /// un modificateur à retirer, un `:optional` à écrire, une table à nommer autrement.
    /// Seule une ancre du plan a un remède qui tient en un bloc à coller.
    fn remede(&self) -> Option<String> {
        match self {
            Error::Plan(erreur) => erreur.remede(),
            _ => None,
        }
    }

    fn bloc(&self) -> Option<String> {
        match self {
            Error::Plan(erreur) => erreur.bloc(),
            _ => None,
        }
    }
}

impl crate::errors::Classee for Error {
    fn sortie(&self) -> crate::errors::Sortie {
        use crate::errors::Sortie;

        match self {
            // Chacun de ces refus se lève en corrigeant la ligne de commande.
            Self::PasUnProjet
            | Self::WorkingTreeSale(_)
            | Self::Nom(_)
            | Self::Fields(_)
            | Self::ChampsVides { .. }
            | Self::TableSansModule { .. }
            | Self::ColonneDejaDeclaree { .. }
            | Self::ColonneObligatoire { .. }
            | Self::UniqueSurColonneAjoutee { .. }
            | Self::ReferenceInterdite { .. }
            | Self::DecimalSousSqlite { .. } => Sortie::Usage,
            Self::Acces(_) => Sortie::Environnement,
            Self::Rendu { .. } => Sortie::Faute,
            Self::Metadata(cause) => cause.sortie(),
            Self::Plan(cause) => cause.sortie(),
            Self::Application(cause) => cause.sortie(),
        }
    }
}

/// Rend la migration qui ajoute `fields` à la table `table`.
///
/// Séparée de [`plan_for`] : le balayage qui compare le rendu à ce que rustfmt écrirait
/// n'a pas de projet sur le disque, et n'en a pas besoin.
pub(crate) fn render(table: &str, fields: &[Field]) -> Result<String, minijinja::Error> {
    Renderer::new().render(
        TEMPLATE,
        minijinja::context! {
            iden => to_pascal_case(table),
            table => table,
            fields => fields,
        },
    )
}

/// Calcule ce que la commande ferait au projet, sans rien écrire.
///
/// L'horodatage est reçu et non lu de l'horloge : un test doit pouvoir viser un nom.
pub(crate) fn plan_for(options: &Options, timestamp: &str) -> Result<Planned, Error> {
    let metadata::Cible { root, metadonnees } = metadata::cible::<Error>(&options.directory)?;

    if !options.force {
        git::garde(&root)?;
    }

    name::validate_identifier(&options.name).map_err(Error::Nom)?;

    let mut champs = fields::parse(&options.fields).map_err(Error::Fields)?;

    // Le parseur rend un vecteur vide sans se plaindre : c'est ce dont `rbs migrate new` a
    // besoin, et non cette commande-ci, dont le rendu n'altérerait alors aucune table.
    if champs.is_empty() {
        return Err(Error::ChampsVides {
            name: options.name.clone(),
        });
    }

    // Avant tout rendu : chacun de ces refus décrit une migration qu'un des trois moteurs
    // n'appliquerait pas. SQLite ne sait ajouter ni clé étrangère, ni colonne unique, ni
    // colonne obligatoire à une table peuplée — et une migration engendrée s'applique
    // partout, ou n'est pas écrite.
    for champ in &champs {
        if champ.reference().is_some() {
            return Err(Error::ReferenceInterdite {
                champ: champ.name.clone(),
            });
        }
        if !champ.optional {
            return Err(Error::ColonneObligatoire {
                champ: champ.name.clone(),
            });
        }
        if champ.unique {
            return Err(Error::UniqueSurColonneAjoutee {
                champ: champ.name.clone(),
            });
        }
        if metadonnees.database == crate::database::Database::Sqlite
            && champ.column_type() == FieldType::Decimal
        {
            return Err(Error::DecimalSousSqlite {
                champ: champ.name.clone(),
            });
        }
    }

    // L'inventaire des entités, et non la présence d'un répertoire : la table `users` d'un
    // projet authentifié vit sous `src/auth/model.rs`, et non dans un `src/users/`.
    let inventaire = entities::scan(&root);
    let module = inventaire
        .iter()
        .find(|entite| entite.table == options.table)
        .map(|entite| {
            entite
                .file
                .trim_start_matches("src/")
                .trim_end_matches("/model.rs")
                .to_string()
        })
        .ok_or_else(|| Error::TableSansModule {
            table: options.table.clone(),
            connues: tables_connues(&inventaire),
        })?;

    // Les deux fichiers sont lus avant le rendu : le doublon se juge sur le modèle, et ce
    // refus-là doit tomber comme les autres, avant le premier octet rendu. Le bloc affiché
    // se calcule ensuite sur les mêmes sources — il ne doit porter que les imports qui
    // manquent réellement à ce module-ci, et la ligne d'import du modèle est à modifier
    // plutôt qu'à ajouter.
    let model_path = format!("src/{module}/model.rs");
    let dto_path = format!("src/{module}/dto.rs");
    let model_source = lire(&root, &model_path);
    let dto_source = lire(&root, &dto_path);

    let declarees = colonnes_declarees(&model_source, &options.table);
    if let Some(champ) = champs
        .iter()
        .find(|champ| declarees.contains(&champ.column_name()))
    {
        return Err(Error::ColonneDejaDeclaree {
            champ: champ.name.clone(),
            table: options.table.clone(),
            fichier: model_path,
        });
    }

    // Le trio que l'en-tête du bloc nomme est celui que le module porte : l'heuristique
    // singulière le manque dès qu'un CRUD a été engendré avec `--singular`, et elle ne
    // sert plus que de repli à un `dto.rs` qui ne dit rien.
    let entity = entite_des_dto(&dto_source).unwrap_or_else(|| {
        // Seul le nom de la table entre dans l'entité ; les champs n'y ont aucune part.
        Feature::fresh(&options.table, Vec::new()).entity()
    });

    // Les champs de cette commande-ci ne transitent par aucune `Feature` : c'est donc ici
    // qu'ils reçoivent l'entité dont le type de leur énumération est préfixé. Le bloc
    // affiché nomme ainsi le type que `generate crud` aurait écrit.
    for champ in &mut champs {
        champ.entity.clone_from(&entity);
    }

    let module_migration = format!("m{timestamp}_{}", options.name);
    let fichier = format!("migration/src/{module_migration}.rs");

    let mut contenu = render(&options.table, &champs).map_err(|source| Error::Rendu {
        file: fichier.clone(),
        source,
    })?;

    // Après le rendu et avant le plan : le plan porte le contenu exact qui sera écrit, et
    // c'est lui que `--dry-run` montre.
    let avertissement = format::format_batch(std::iter::once(&mut contenu));

    // Les deux fichiers s'écrivent ensemble ou pas du tout : une migration que le `lib.rs`
    // ne déclare pas est un module que cargo refuse, et le projet ne compilerait plus pour
    // une commande qui a pourtant échoué.
    let mut builder = plan::Builder::new(root);
    builder.create(&fichier, &contenu)?;
    for mount in mount::for_migration(&module_migration) {
        builder.insert(mount.anchor, &mount.lines)?;
    }

    // La même épingle que `generate crud`, et depuis la même source : voir
    // `generate::patches_decimal`.
    if champs
        .iter()
        .any(|champ| champ.column_type() == FieldType::Decimal)
    {
        for patch in super::patches_decimal() {
            builder.patch(patch)?;
        }
    }

    Ok(Planned {
        plan: builder.finir(),
        module: module_migration,
        fichier,
        blocs: vec![
            Bloc {
                fichier: model_path,
                lignes: bloc_du_modele(&champs, &model_source),
            },
            Bloc {
                fichier: dto_path,
                lignes: bloc_des_dto(&entity, &champs, &dto_source),
            },
        ],
        avertissement,
    })
}

/// Les tables que le projet déclare, triées et dédupliquées.
///
/// L'ordre de `read_dir` dépend du système de fichiers : sans tri, le message changerait
/// d'une machine à l'autre.
fn tables_connues(inventaire: &[entities::Entity]) -> String {
    let mut tables: Vec<&str> = inventaire
        .iter()
        .map(|entite| entite.table.as_str())
        .collect();
    tables.sort_unstable();
    tables.dedup();

    if tables.is_empty() {
        "aucune".to_string()
    } else {
        tables.join(", ")
    }
}

/// Les colonnes que le `struct Model` de `table` déclare, relevées dans `source`.
///
/// Le relevé part de l'attribut `table_name` et non du début du fichier : un `model.rs`
/// en porte parfois plusieurs — `src/auth/model.rs` déclare `users`, `refresh_tokens` et
/// `one_time_tokens` —, et un doublon prononcé sur l'homonyme d'une table voisine
/// refuserait une colonne parfaitement légitime.
///
/// La lecture est textuelle, comme celle d'[`entities::scan`] : un modèle lourdement
/// réécrit y échappera, et le doublon retombera alors sur le moteur. Ce relevé sert à
/// refuser, jamais à autoriser.
fn colonnes_declarees(source: &str, table: &str) -> Vec<String> {
    let attribut = format!("table_name = \"{table}\"");
    let mut colonnes = Vec::new();
    let mut dans_la_structure = false;

    for ligne in source.lines() {
        let ligne = ligne.trim();

        if !dans_la_structure {
            dans_la_structure = ligne.contains(&attribut);
            continue;
        }

        // La structure ne porte que des attributs et des champs : sa première accolade
        // fermante en début de ligne est la sienne.
        if ligne == "}" {
            break;
        }

        if let Some(reste) = ligne.strip_prefix("pub ")
            && let Some((nom, _)) = reste.split_once(':')
        {
            colonnes.push(nom.trim().to_string());
        }
    }

    colonnes
}

/// Les imports que le gabarit du modèle ne pose que sous `{% if enum_types %}`.
///
/// Un module qui ne portait aucune énumération ne les a pas, et le type collé ne
/// compilerait pas sans eux.
const IMPORTS_ENUM_MODELE: [&str; 2] = [
    "use serde::{Deserialize, Serialize};",
    "use utoipa::ToSchema;",
];

/// Le contenu d'un fichier du projet, ou le vide s'il n'existe pas.
///
/// Un fichier absent n'arrête pas la commande : le bloc portera alors tous ses imports,
/// ce qui reste juste — c'est ce qu'un fichier vide réclame.
fn lire(root: &Path, chemin: &str) -> String {
    fs::read_to_string(root.join(chemin)).unwrap_or_default()
}

/// Ce que le modèle reçoit : les imports qui lui manquent, le type de chaque
/// énumération, puis les champs de `Model`.
///
/// Affiché et non inséré : `model.rs` n'a pas d'ancre, et le CLI ne réécrit pas d'AST. Le
/// type d'une énumération est écrit comme `generate crud` le rend, documentation comprise :
/// la migration pose le `CHECK` qui borne la colonne, et un modèle qui n'accorderait pas
/// décrirait un schéma que la base n'a pas.
fn bloc_du_modele(champs: &[Field], source: &str) -> Vec<String> {
    let mut lignes = Vec::new();
    let enumerations: Vec<&Field> = champs
        .iter()
        .filter(|champ| !champ.enum_variants().is_empty())
        .collect();

    if !enumerations.is_empty() {
        let manquants: Vec<&str> = IMPORTS_ENUM_MODELE
            .iter()
            .copied()
            .filter(|import| !source.contains(import))
            .collect();

        if !manquants.is_empty() {
            lignes.push("// aux imports, en tête du fichier".to_string());
            lignes.extend(manquants.iter().map(|import| (*import).to_string()));
            lignes.push(String::new());
        }
    }

    for champ in enumerations {
        lignes.push(format!(
            "/// Valeurs acceptées par la colonne « {} ».",
            champ.name
        ));
        lignes.push("///".to_string());
        lignes.push(
            "/// Une valeur de plus s'ajoute ici et dans le `CHECK` que porte une migration \
             nouvelle :"
                .to_string(),
        );
        lignes.push(
            "/// la base refuse d'elle-même celles qu'elle ne connaît pas. Plus longue que \
             toutes les"
                .to_string(),
        );
        lignes.push(
            "/// actuelles, elle demande en troisième lieu d'élargir le `StringLen::N` \
             ci-dessous, et"
                .to_string(),
        );
        lignes.push("/// avec lui le `string_len` de cette migration.".to_string());
        lignes.push("#[derive(".to_string());
        lignes.push(
            "    Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, \
             Serialize, ToSchema,"
                .to_string(),
        );
        lignes.push(")]".to_string());
        lignes.push(format!(
            "#[sea_orm(rs_type = \"String\", db_type = \"String(StringLen::N({}))\")]",
            champ.enum_length()
        ));
        lignes.push(format!("pub enum {} {{", champ.enum_type()));
        for cas in champ.enum_cases() {
            lignes.push(format!("    #[sea_orm(string_value = \"{}\")]", cas.value));
            lignes.push(format!("    #[serde(rename = \"{}\")]", cas.value));
            lignes.push(format!("    {},", cas.variant));
        }
        lignes.push("}".to_string());
        lignes.push(String::new());
    }

    lignes.push("// dans `struct Model`".to_string());
    for champ in champs {
        if let Some(attribut) = champ.column_type_attribute() {
            lignes.push(format!("    #[sea_orm(column_type = \"{attribut}\")]"));
        } else if champ.index {
            lignes.push("    #[sea_orm(indexed)]".to_string());
        }
        lignes.push(format!("    pub {}: {},", champ.name, champ.rust_type()));
    }

    lignes
}

/// L'import que le prélude ne donne pas : `Date` et `Decimal` n'y entrent que nommés.
///
/// Les bras sont écrits un par un, sans `_` : un type nouveau ne compile pas tant que
/// personne n'a tranché s'il demande un import.
fn import_du_type(champ: &Field) -> Option<String> {
    match champ.column_type() {
        FieldType::Date => Some("use sea_orm::prelude::Date;".to_string()),
        FieldType::Decimal => Some("use sea_orm::prelude::Decimal;".to_string()),
        FieldType::String
        | FieldType::Int
        | FieldType::Float
        | FieldType::Bool
        | FieldType::Uuid
        | FieldType::Datetime
        | FieldType::Text => None,
    }
}

/// Le nom que les DTO du module portent, lu dans `dto.rs`.
///
/// L'en-tête du bloc nomme trois structures : les redeviner par l'heuristique singulière
/// les manque dès qu'un CRUD a été engendré avec `--singular`, et le développeur cherche
/// alors des noms qui n'existent pas. `[package.metadata.rbs]` ne retient pas cette
/// forme-là ; le fichier des DTO, lui, la porte.
///
/// Les trois doivent concorder — c'est le trio que l'en-tête annonce —, faute de quoi le
/// fichier n'est pas celui d'un CRUD engendré et l'heuristique reprend la main.
fn entite_des_dto(source: &str) -> Option<String> {
    let entite = source.lines().find_map(|ligne| {
        ligne
            .trim()
            .strip_prefix("pub struct Create")?
            .split([' ', '{', '(', '<'])
            .next()
            .filter(|nom| !nom.is_empty())
            .map(str::to_string)
    })?;

    let complet = source.contains(&format!("pub struct Update{entite}"))
        && source.contains(&format!("pub struct {entite}Response"));

    complet.then_some(entite)
}

/// Ce que la ligne `use super::model::…` des DTO réclame d'un type d'énumération.
enum ImportDuModele {
    /// Le fichier porte déjà la ligne : elle se **remplace** par celle-ci, le type de
    /// l'énumération l'ayant rejointe. L'ajouter telle quelle la déclarerait deux fois.
    Remplacer {
        /// La ligne telle que le fichier la porte.
        ancienne: String,
        /// La même, le type joint.
        nouvelle: String,
    },
    /// Le fichier n'en porte aucune — les DTO du fragment `auth` n'en ont pas : la ligne
    /// s'ajoute entière, avec les autres imports qui manquent.
    Ajouter(String),
}

/// La ligne d'import telle que le gabarit l'écrit : noms triés, accolades seulement à
/// plusieurs — faute de quoi le premier `cargo fmt` du projet la réécrirait.
fn ligne_du_modele(noms: &[String]) -> String {
    if noms.len() > 1 {
        format!("use super::model::{{{}}};", noms.join(", "))
    } else {
        format!("use super::model::{};", noms[0])
    }
}

/// L'import du modèle que les types d'énumération collés réclament aux DTO.
///
/// `None` quand rien ne s'y ajoute : aucune énumération, ou toutes déjà importées.
fn import_du_modele(champs: &[Field], source: &str) -> Option<ImportDuModele> {
    let mut types: Vec<String> = champs
        .iter()
        .filter(|champ| !champ.enum_variants().is_empty())
        .map(Field::enum_type)
        .collect();

    if types.is_empty() {
        return None;
    }

    // Un fichier sans ligne à modifier n'est pas un fichier sans besoin : se taire
    // livrerait le champ collé sans dire d'où vient son type.
    let Some(ancienne) = source
        .lines()
        .find(|ligne| ligne.trim_start().starts_with("use super::model::"))
        .map(|ligne| ligne.trim().to_string())
    else {
        types.sort();
        types.dedup();

        return Some(ImportDuModele::Ajouter(ligne_du_modele(&types)));
    };

    let dedans = ancienne
        .trim_start_matches("use super::model::")
        .trim_end_matches(';');
    let mut noms: Vec<String> = dedans
        .trim_start_matches('{')
        .trim_end_matches('}')
        .split(',')
        .map(|nom| nom.trim().to_string())
        .filter(|nom| !nom.is_empty())
        .collect();

    let avant = noms.len();
    for type_ in types {
        if !noms.contains(&type_) {
            noms.push(type_);
        }
    }

    if noms.len() == avant {
        return None;
    }

    noms.sort();
    let nouvelle = ligne_du_modele(&noms);

    Some(ImportDuModele::Remplacer { ancienne, nouvelle })
}

/// Ce que les DTO reçoivent : les imports qui leur manquent, la ligne d'import du modèle —
/// à reprendre, ou à ajouter entière —, puis la même ligne de champ dans les trois
/// structures.
///
/// Toute colonne ajoutée étant optionnelle — c'est ce que cette commande exige —, `Create`,
/// `Update` et la réponse portent le même `Option<T>` ; un champ obligatoire les aurait
/// distingués.
fn bloc_des_dto(entity: &str, champs: &[Field], source: &str) -> Vec<String> {
    let mut lignes = Vec::new();

    let mut imports: Vec<String> = Vec::new();
    for import in champs.iter().filter_map(import_du_type) {
        if !source.contains(&import) && !imports.contains(&import) {
            imports.push(import);
        }
    }

    let modele = import_du_modele(champs, source);

    // Les deux cas se distinguent à l'en-tête sous lequel la ligne tombe, et aucun des
    // deux ne se tait : un fichier sans ligne à remplacer reçoit l'import entier, parmi
    // ceux qui s'ajoutent.
    if let Some(ImportDuModele::Ajouter(ligne)) = &modele {
        imports.push(ligne.clone());
    }

    if !imports.is_empty() {
        lignes.push("// aux imports, en tête du fichier".to_string());
        lignes.append(&mut imports);
        lignes.push(String::new());
    }

    if let Some(ImportDuModele::Remplacer { ancienne, nouvelle }) = &modele {
        lignes.push(format!("// remplacez `{ancienne}` par :"));
        lignes.push(nouvelle.clone());
        lignes.push(String::new());
    }

    lignes.push(format!(
        "// dans `Create{entity}`, `Update{entity}` et `{entity}Response`"
    ));

    for champ in champs {
        if let Some(attribut) = schema_format(champ) {
            lignes.push(attribut);
        }

        let validations = champ.validations();
        if !validations.is_empty() {
            lignes.push(format!("    #[validate({})]", validations.join(", ")));
        }

        lignes.push(format!("    pub {}: {},", champ.name, champ.rust_type()));
    }

    lignes
}

/// L'attribut que le document OpenAPI réclame d'un type qu'utoipa ne reconnaît pas à son
/// nom écrit : `DateTimeWithTimeZone` est un alias, et un décimal doit dire une chaîne —
/// un nombre y passerait par le flottant d'un client JavaScript, qui perdrait les centimes.
///
/// Les bras sont écrits un par un, sans `_` : un type nouveau ne compile pas tant que
/// personne n'a tranché s'il porte un format explicite.
fn schema_format(champ: &Field) -> Option<String> {
    match champ.column_type() {
        FieldType::Datetime => {
            Some("    #[schema(value_type = Option<String>, format = DateTime)]".to_string())
        }
        FieldType::Decimal => {
            Some("    #[schema(value_type = Option<String>, format = \"decimal\")]".to_string())
        }
        FieldType::String
        | FieldType::Int
        | FieldType::Float
        | FieldType::Bool
        | FieldType::Uuid
        | FieldType::Date
        | FieldType::Text => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::*;
    use crate::generate::bench;
    use crate::generate::feature::Feature;

    const HORODATAGE: &str = "20260916_101500";

    /// Empreinte récursive d'un répertoire : chemin relatif -> contenu.
    ///
    /// `.git` reste dehors : lire l'état du working tree rafraîchit l'index, et cette
    /// écriture-là n'est pas celle qu'un plan non appliqué promet d'éviter.
    fn empreinte(root: &Path) -> BTreeMap<PathBuf, String> {
        let mut vus = BTreeMap::new();
        let mut a_parcourir = vec![root.to_path_buf()];

        while let Some(directory) = a_parcourir.pop() {
            for entree in fs::read_dir(&directory).expect("le répertoire se lit") {
                let path = entree.expect("l'entrée se lit").path();

                if path.file_name().is_some_and(|nom| nom == ".git") {
                    continue;
                }

                if path.is_dir() {
                    a_parcourir.push(path);
                    continue;
                }

                let relatif = path
                    .strip_prefix(root)
                    .expect("le chemin est sous la racine")
                    .to_path_buf();
                vus.insert(relatif, fs::read_to_string(&path).unwrap_or_default());
            }
        }

        vus
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{} illisible : {error}", path.display()))
    }

    /// Écrit un CRUD dans le projet, pour que la table visée ait un module.
    fn crud(root: &Path, table: &str, champs: &str) {
        crud_avec(root, table, champs, None);
    }

    /// Le même CRUD, sa forme singulière imposée : ce que `--singular` passe.
    fn crud_singulier(root: &Path, table: &str, champs: &str, singular: &str) {
        crud_avec(root, table, champs, Some(singular.to_string()));
    }

    fn crud_avec(root: &Path, table: &str, champs: &str, singular: Option<String>) {
        let planned = crate::generate::command::plan_for(&crate::generate::command::Options {
            name: table.to_string(),
            fields: Some(champs.to_string()),
            complete: true,
            directory: root.to_path_buf(),
            force: false,
            has_many: Vec::new(),
            role: None,
            soft_delete: false,
            with_upload: false,
            cursor: false,
            singular,
        })
        .expect("le CRUD du test doit se planifier");

        crate::plan::application::apply(&planned.plan, false).expect("le CRUD du test s'écrit");
    }

    /// Un projet neuf portant la table `articles`.
    fn projet() -> (TempDir, PathBuf) {
        let (parent, root) = crate::fixtures::project();
        crud(&root, "articles", "titre:string");

        (parent, root)
    }

    fn options(root: &Path, name: &str, table: &str, champs: &str) -> Options {
        Options {
            name: name.to_string(),
            table: table.to_string(),
            fields: champs.to_string(),
            directory: root.to_path_buf(),
            force: false,
        }
    }

    /// Planifie puis applique, comme la commande le fait.
    fn run(options: &Options) -> Result<Planned, Error> {
        let planned = plan_for(options, HORODATAGE)?;
        crate::plan::application::apply(&planned.plan, options.force)?;

        Ok(planned)
    }

    /// Le rendu débarrassé de ses blancs : ce qui s'y vérifie est la projection du type
    /// sur sa méthode, non la ligne où elle tombe.
    fn sans_blancs(rendu: &str) -> String {
        rendu.split_whitespace().collect()
    }

    /// Le rendu seul, sans projet sur le disque.
    fn rendu(table: &str, champs: &str) -> String {
        let champs = fields::parse(champs).expect("les champs du test doivent être valides");

        render(table, &champs).expect("la migration doit se rendre")
    }

    #[test]
    fn the_module_carries_the_timestamp_and_the_given_name() {
        let (_parent, root) = projet();

        let planned = run(&options(
            &root,
            "ajoute_statut",
            "articles",
            "statut:enum(draft,published):optional",
        ))
        .expect("la migration doit s'écrire");

        assert_eq!(planned.module, "m20260916_101500_ajoute_statut");
        assert_eq!(
            planned.fichier,
            "migration/src/m20260916_101500_ajoute_statut.rs"
        );
        assert!(root.join(&planned.fichier).is_file());
    }

    /// Une migration que le `lib.rs` ne déclare pas est un module que rien n'appelle, et
    /// que cargo refuse : les deux ancres se remplissent ensemble.
    #[test]
    fn the_migration_is_declared_then_recorded_in_the_two_anchors() {
        let (_parent, root) = projet();

        run(&options(
            &root,
            "ajoute_statut",
            "articles",
            "statut:string:optional",
        ))
        .expect("la migration doit s'écrire");

        let lib = read(&root.join("migration/src/lib.rs"));
        assert!(
            lib.contains("mod m20260916_101500_ajoute_statut;"),
            "le module n'est pas déclaré :\n{lib}"
        );
        assert!(
            lib.contains("Box::new(m20260916_101500_ajoute_statut::Migration),"),
            "la migration n'est pas inscrite au Migrator :\n{lib}"
        );
    }

    /// Les dix types de la grammaire que `--add-column` accepte — `references` étant le
    /// onzième, et refusé.
    #[test]
    fn each_type_projects_to_its_column_method() {
        let rendu = rendu(
            "samples",
            "title:string:optional,quantity:int:optional,ratio:float:optional,\
             price:decimal:optional,active:bool:optional,owner:uuid:optional,\
             published_at:datetime:optional,due:date:optional,body:text:optional,\
             statut:enum(draft,published):optional",
        );
        let compact = sans_blancs(&rendu);

        for attendu in [
            "ColumnDef::new(Samples::Title).string()",
            "ColumnDef::new(Samples::Quantity).integer()",
            "ColumnDef::new(Samples::Ratio).double()",
            "ColumnDef::new(Samples::Price).decimal_len(19,4)",
            "ColumnDef::new(Samples::Active).boolean()",
            "ColumnDef::new(Samples::Owner).uuid()",
            "ColumnDef::new(Samples::PublishedAt).timestamp_with_time_zone()",
            "ColumnDef::new(Samples::Due).date()",
            "ColumnDef::new(Samples::Body).text()",
            "ColumnDef::new(Samples::Statut).string_len(9)",
        ] {
            assert!(
                compact.contains(attendu),
                "« {attendu} » absent de :\n{rendu}"
            );
        }
    }

    /// Une colonne ajoutée est toujours nullable : la table porte déjà des lignes, qui
    /// n'ont pas de valeur pour elle.
    #[test]
    fn every_added_column_is_nullable() {
        let rendu = rendu("articles", "statut:string:optional,vues:int:optional");

        assert_eq!(
            rendu.matches(".null()").count(),
            2,
            "chaque colonne ajoutée doit admettre le nul :\n{rendu}"
        );
        assert!(
            !rendu.contains(".not_null()"),
            "une colonne ajoutée ne peut pas être obligatoire :\n{rendu}"
        );
    }

    /// Chaque champ a son propre `alter_table` : SQLite n'accepte qu'une option d'ALTER
    /// par instruction, et sea-query y renonce plutôt que de la plier.
    #[test]
    fn each_field_gets_its_own_alter_statement() {
        let rendu = rendu("articles", "statut:string:optional,vues:int:optional");

        assert_eq!(
            rendu.matches(".alter_table(").count(),
            4,
            "deux ajouts à la montée, deux retraits à la descente :\n{rendu}"
        );
    }

    /// La descente retire les colonnes dans l'ordre inverse de leur ajout.
    #[test]
    fn the_down_drops_the_columns_in_reverse_order() {
        let rendu = rendu("articles", "statut:string:optional,vues:int:optional");
        let descente = rendu
            .split_once("async fn down")
            .expect("la migration doit déclarer sa descente")
            .1;

        let statut = descente
            .find("drop_column(Articles::Statut)")
            .expect("la colonne statut doit être retirée");
        let vues = descente
            .find("drop_column(Articles::Vues)")
            .expect("la colonne vues doit être retirée");

        assert!(
            vues < statut,
            "la descente doit défaire la montée à l'envers :\n{rendu}"
        );
    }

    /// Le fichier déclare son propre `Iden`, comme la migration de création : la table et
    /// les seules colonnes qu'il touche.
    #[test]
    fn the_iden_enum_declares_the_table_and_only_the_added_columns() {
        let rendu = rendu("blog_posts", "statut:string:optional,vues:int:optional");

        assert!(rendu.contains("enum BlogPosts {"), "{rendu}");
        for variante in ["Table,", "Statut,", "Vues,"] {
            assert!(
                rendu.contains(variante),
                "variante {variante} absente :\n{rendu}"
            );
        }
        assert!(
            !rendu.contains("CreatedAt,") && !rendu.contains("    Id,"),
            "l'Iden ne déclare que ce que la migration nomme :\n{rendu}"
        );
    }

    /// La colonne d'une énumération est une chaîne bornée sous un `CHECK` : c'est la base,
    /// et non l'application, qui refuse une valeur étrangère.
    #[test]
    fn an_enum_column_is_a_bounded_string_under_a_check() {
        let rendu = rendu("articles", "statut:enum(draft,published):optional");

        assert!(
            sans_blancs(&rendu).contains(
                "ColumnDef::new(Articles::Statut).string_len(9).null().check(Expr::col(Articles::Statut).is_in([\"draft\",\"published\"]))"
            ),
            "colonne ou contrainte absente :\n{rendu}"
        );
    }

    /// Un champ `index` reçoit son index nommé, que la descente retire avant la colonne :
    /// SQLite refuse de retirer une colonne indexée.
    #[test]
    fn an_indexed_column_gets_its_index_and_drops_it_before_the_column() {
        let rendu = rendu("articles", "slug:string:optional:index");

        assert!(
            rendu.contains(r#".name("idx_articles_slug")"#),
            "index nommé absent :\n{rendu}"
        );

        let descente = rendu
            .split_once("async fn down")
            .expect("la migration doit déclarer sa descente")
            .1;
        let index = descente
            .find("drop_index")
            .expect("l'index doit être retiré à la descente");
        let colonne = descente
            .find("drop_column")
            .expect("la colonne doit être retirée");

        assert!(
            index < colonne,
            "SQLite refuse de retirer une colonne encore indexée :\n{rendu}"
        );
    }

    /// Un champ sans modificateur ne crée aucun index.
    #[test]
    fn a_field_without_a_modifier_creates_no_index() {
        let rendu = rendu("articles", "statut:string:optional");

        assert!(
            !rendu.contains("create_index"),
            "index créé sans avoir été demandé :\n{rendu}"
        );
    }

    /// Les deux balayages : le nom de la table à champ figé, puis le champ sur une table
    /// déjà longue — c'est leur somme qui décide de la mise en forme, et un seul axe
    /// laisserait la moitié des gardes non éprouvée.
    #[test]
    fn the_render_is_already_what_rustfmt_would_write() {
        let table_longue = "a".repeat(33) + "e";

        for (libelle, divergentes) in [
            (
                "table, champ scalaire",
                bench::longueurs_divergentes(|table| rendu(table, "statut:string:optional")),
            ),
            (
                "champ scalaire, table longue",
                bench::longueurs_divergentes(|champ| {
                    rendu(&table_longue, &format!("{champ}:string:optional"))
                }),
            ),
            (
                "table, champ indexé",
                bench::longueurs_divergentes(|table| rendu(table, "statut:string:optional:index")),
            ),
            (
                "champ indexé, table longue",
                bench::longueurs_divergentes(|champ| {
                    rendu(&table_longue, &format!("{champ}:string:optional:index"))
                }),
            ),
            // Les formes ramassées de la descente — l'appel entier sur la ligne de
            // `manager` — ne s'atteignent qu'à table *et* colonne courtes : les deux axes
            // ci-dessus figent l'un ou l'autre trop long pour jamais y tomber.
            (
                "table courte, champ d'une lettre",
                bench::longueurs_divergentes(|table| rendu(table, "a:string:optional")),
            ),
            (
                "table courte, champ d'une lettre indexé",
                bench::longueurs_divergentes(|table| rendu(table, "a:string:optional:index")),
            ),
            (
                "table, champ énuméré",
                bench::longueurs_divergentes(|table| {
                    rendu(table, "statut:enum(draft,published):optional")
                }),
            ),
            (
                "champ énuméré, table longue",
                bench::longueurs_divergentes(|champ| {
                    rendu(
                        &table_longue,
                        &format!("{champ}:enum(draft,published):optional"),
                    )
                }),
            ),
        ] {
            assert_eq!(
                divergentes,
                Vec::<usize>::new(),
                "le rendu diverge de rustfmt à ces longueurs ({libelle})"
            );
        }
    }

    /// Le refus est prononcé avant tout rendu : une colonne obligatoire n'a pas de valeur
    /// pour les lignes déjà là, et le message doit nommer le champ et le remède.
    #[test]
    fn a_required_column_is_refused_naming_the_field_and_its_remedy() {
        let (_parent, root) = projet();
        let avant = empreinte(&root);

        let error = run(&options(
            &root,
            "ajoute_statut",
            "articles",
            "statut:string",
        ))
        .expect_err("une colonne obligatoire est refusée");

        let message = error.to_string();
        assert!(
            message.contains("statut"),
            "le refus doit nommer le champ : {message}"
        );
        assert!(
            message.contains("optional"),
            "le refus doit donner le remède : {message}"
        );
        assert_eq!(empreinte(&root), avant, "rien ne doit avoir été écrit");
    }

    /// Le message nomme ce qui a été cherché, et les tables que le projet déclare.
    #[test]
    fn a_table_without_a_module_is_refused_naming_what_was_searched() {
        let (_parent, root) = projet();
        let avant = empreinte(&root);

        let error = run(&options(
            &root,
            "ajoute_statut",
            "factures",
            "statut:string:optional",
        ))
        .expect_err("une table sans module est refusée");

        let message = error.to_string();
        assert!(
            message.contains("factures"),
            "le refus doit nommer la table demandée : {message}"
        );
        assert!(
            message.contains("src/*/model.rs"),
            "le refus doit nommer ce qui a été cherché : {message}"
        );
        assert!(
            message.contains("articles"),
            "le refus doit nommer les tables connues : {message}"
        );
        assert_eq!(empreinte(&root), avant, "rien ne doit avoir été écrit");
    }

    /// Sans champ, la migration rendue n'altère rien — mais elle s'écrit, s'inscrit aux
    /// deux ancres et se compte dans `migrate status`. C'est ce que rend `rbs migrate
    /// new`, à ceci près que l'utilisateur ne l'a pas demandé.
    #[test]
    fn an_empty_fields_string_is_refused_rather_than_written_as_an_empty_migration() {
        let (_parent, root) = projet();
        let avant = empreinte(&root);

        let error = run(&options(&root, "ajoute_statut", "articles", ""))
            .expect_err("une migration sans colonne est refusée");

        let message = error.to_string();
        assert!(
            message.contains("--fields"),
            "le refus doit nommer ce qui manque : {message}"
        );
        assert!(
            message.contains("rbs migrate new ajoute_statut"),
            "le refus doit donner le remède, la commande toute faite : {message}"
        );
        assert_eq!(empreinte(&root), avant, "rien ne doit avoir été écrit");
    }

    /// Une colonne que l'entité déclare déjà est refusée au plan. Sans ce refus, le
    /// moteur la rejette au `migrate up` — la migration déjà écrite, déjà inscrite aux
    /// deux ancres, et sans remède affiché.
    #[test]
    fn a_column_the_entity_already_declares_is_refused_before_anything_is_written() {
        let (_parent, root) = projet();
        let avant = empreinte(&root);

        let error = run(&options(
            &root,
            "ajoute_titre",
            "articles",
            "titre:string:optional",
        ))
        .expect_err("une colonne déjà déclarée est refusée");

        let message = error.to_string();
        assert!(
            message.contains("titre"),
            "le refus doit nommer le champ : {message}"
        );
        assert!(
            message.contains("articles"),
            "le refus doit nommer la table : {message}"
        );
        assert!(
            message.contains("src/articles/model.rs"),
            "le refus doit nommer le fichier qui atteste : {message}"
        );
        assert_eq!(empreinte(&root), avant, "rien ne doit avoir été écrit");
    }

    /// Le doublon se cherche dans le `struct Model` de la table visée, et non dans tout
    /// le fichier : `src/auth/model.rs` porte trois entités, et `email` n'appartient
    /// qu'à `users`. Un refus prononcé sur l'homonyme d'une table voisine interdirait une
    /// colonne parfaitement légitime.
    #[test]
    fn the_duplicate_is_sought_in_the_targeted_table_alone() {
        let (_parent, root) = crate::fixtures::Project::new().features(&["auth"]).create();

        run(&options(
            &root,
            "ajoute_email_au_jeton",
            "refresh_tokens",
            "email:string:optional",
        ))
        .expect("`email` n'appartient pas à refresh_tokens");

        let error = run(&options(
            &root,
            "ajoute_email",
            "users",
            "email:string:optional",
        ))
        .expect_err("`users` porte déjà `email`");

        assert!(
            matches!(error, Error::ColonneDejaDeclaree { .. }),
            "le refus doit être celui du doublon : {error}"
        );
    }

    /// SQLite refuse d'ajouter une colonne sous contrainte d'unicité : le refus tient sur
    /// les trois moteurs, une migration engendrée devant s'appliquer partout.
    #[test]
    fn unique_on_an_added_column_is_refused_on_every_engine() {
        let (_parent, root) = projet();
        let avant = empreinte(&root);

        let error = run(&options(
            &root,
            "ajoute_slug",
            "articles",
            "slug:string:optional:unique",
        ))
        .expect_err("`unique` est refusé sur une colonne ajoutée");

        let message = error.to_string();
        assert!(
            message.contains("slug") && message.contains("SQLite"),
            "le refus doit nommer le champ et le moteur : {message}"
        );
        assert_eq!(empreinte(&root), avant, "rien ne doit avoir été écrit");
    }

    /// SQLite ne sait pas ajouter de clé étrangère à une table existante.
    #[test]
    fn a_reference_is_refused_naming_the_fallback() {
        let (_parent, root) = projet();
        crud(&root, "users", "email:string:unique");
        let avant = empreinte(&root);

        let error = run(&options(
            &root,
            "ajoute_auteur",
            "articles",
            "author:references:users:optional",
        ))
        .expect_err("une référence est refusée sur une colonne ajoutée");

        let message = error.to_string();
        assert!(
            message.contains("author") && message.contains("uuid"),
            "le refus doit nommer le champ et le repli : {message}"
        );
        assert_eq!(empreinte(&root), avant, "rien ne doit avoir été écrit");
    }

    /// Le refus du décimal est celui de `generate crud`, au mot près : sqlx-sqlite ne lie
    /// aucun décimal exact, quel que soit le chemin par lequel la colonne arrive.
    #[test]
    fn a_decimal_under_sqlite_is_refused_as_generate_crud_refuses_it() {
        let (_parent, root) = crate::fixtures::Project::new()
            .database(crate::database::Database::Sqlite)
            .url("sqlite://demo_api.db?mode=rwc")
            .create();
        crud(&root, "articles", "titre:string");
        let avant = empreinte(&root);

        let error = run(&options(
            &root,
            "ajoute_prix",
            "articles",
            "prix:decimal:optional",
        ))
        .expect_err("SQLite ne porte pas de décimal exact");

        assert_eq!(
            error.to_string(),
            crate::errors::decimal_sous_sqlite("prix"),
            "les deux commandes doivent refuser dans les mêmes termes"
        );
        assert_eq!(empreinte(&root), avant, "rien ne doit avoir été écrit");
    }

    /// Le manifeste reçoit de quoi porter le type, par les mêmes actions de plan que
    /// `generate crud` : `sea_orm::prelude::Decimal` n'existe que sous `with-rust_decimal`.
    #[test]
    fn a_decimal_column_adds_what_the_manifest_needs() {
        let (_parent, root) = projet();

        run(&options(
            &root,
            "ajoute_prix",
            "articles",
            "prix:decimal:optional",
        ))
        .expect("un décimal se génère sous PostgreSQL");

        let manifest = read(&root.join("Cargo.toml"));
        assert!(
            manifest.contains(r#"rust_decimal = { version = "1.43", features = ["serde-str"] }"#),
            "la dépendance au décimal manque :\n{manifest}"
        );
        assert!(
            manifest.contains("with-rust_decimal"),
            "la feature de sea-orm manque :\n{manifest}"
        );
    }

    /// Les deux commandes épinglent le décimal à la même ligne de manifeste.
    ///
    /// `generate crud` crée la colonne, `generate migration` l'ajoute : une épingle qui
    /// divergerait ferait dépendre le manifeste de celle qui l'a touché en dernier. Le
    /// test compare les deux projets plutôt que la constante, qui ne prouverait que
    /// d'elle-même.
    #[test]
    fn both_commands_pin_the_decimal_to_the_same_manifest_line() {
        let epingle = |root: &Path| {
            read(&root.join("Cargo.toml"))
                .lines()
                .find(|ligne| ligne.starts_with("rust_decimal"))
                .unwrap_or_else(|| panic!("l'épingle du décimal manque à {}", root.display()))
                .to_string()
        };

        let (_par_crud, par_crud) = crate::fixtures::project();
        crud(&par_crud, "orders", "price:decimal");

        let (_par_migration, par_migration) = projet();
        run(&options(
            &par_migration,
            "ajoute_prix",
            "articles",
            "prix:decimal:optional",
        ))
        .expect("la migration doit s'écrire");

        assert_eq!(epingle(&par_crud), epingle(&par_migration));
    }

    /// Une migration sans champ décimal ne touche pas au manifeste.
    #[test]
    fn a_migration_without_a_decimal_leaves_the_manifest_alone() {
        let (_parent, root) = projet();
        let avant = read(&root.join("Cargo.toml"));

        run(&options(
            &root,
            "ajoute_statut",
            "articles",
            "statut:string:optional",
        ))
        .expect("la migration doit s'écrire");

        assert_eq!(
            read(&root.join("Cargo.toml")),
            avant,
            "le manifeste ne bouge que pour un type qui l'exige"
        );
    }

    /// Les fautes de la grammaire restent celles du parseur : cette commande n'en écrit
    /// pas de seconde édition.
    #[test]
    fn the_faults_of_the_grammar_stay_the_parsers() {
        let (_parent, root) = projet();

        let error = run(&options(
            &root,
            "ajoute_statut",
            "articles",
            "statut:couleur:optional",
        ))
        .expect_err("un type inconnu est refusé");

        assert!(
            matches!(error, Error::Fields(_)),
            "le refus doit venir du parseur : {error}"
        );
        assert!(error.to_string().contains("couleur"), "{error}");
    }

    /// `model.rs` et `dto.rs` n'ont pas d'ancre : leurs lignes s'affichent, et rien ne les
    /// écrit.
    #[test]
    fn the_blocks_to_paste_name_the_model_and_the_dto_without_touching_them() {
        let (_parent, root) = projet();
        let model_avant = read(&root.join("src/articles/model.rs"));
        let dto_avant = read(&root.join("src/articles/dto.rs"));

        let planned = run(&options(
            &root,
            "ajoute_vues",
            "articles",
            "vues:int:optional",
        ))
        .expect("la migration doit s'écrire");

        let fichiers: Vec<&str> = planned
            .blocs
            .iter()
            .map(|bloc| bloc.fichier.as_str())
            .collect();
        assert_eq!(fichiers, ["src/articles/model.rs", "src/articles/dto.rs"]);

        for bloc in &planned.blocs {
            assert!(
                bloc.lignes.iter().any(|ligne| ligne.contains("pub vues:")),
                "le bloc de {} ne porte pas la colonne :\n{:#?}",
                bloc.fichier,
                bloc.lignes
            );
        }

        assert_eq!(read(&root.join("src/articles/model.rs")), model_avant);
        assert_eq!(read(&root.join("src/articles/dto.rs")), dto_avant);
    }

    /// Pour une énumération, le bloc porte aussi le type `DeriveActiveEnum` à coller, tel
    /// que `generate crud` le rend : deux rendus voisins qui divergeraient donneraient un
    /// modèle que la migration ne décrit plus.
    #[test]
    fn the_block_carries_the_derive_active_enum_as_generate_crud_renders_it() {
        let (_parent, root) = projet();

        let planned = run(&options(
            &root,
            "ajoute_statut",
            "articles",
            "statut:enum(draft,published):optional",
        ))
        .expect("la migration doit s'écrire");

        let bloc = planned
            .blocs
            .iter()
            .find(|bloc| bloc.fichier == "src/articles/model.rs")
            .expect("le modèle doit recevoir un bloc");
        let colle = bloc.lignes.join("\n");

        let champs = fields::parse("statut:enum(draft,published)").expect("champs valides");
        let modele = crate::generate::entity::render(&Feature::fresh("articles", champs))
            .expect("l'entité doit se rendre");

        // La déclaration **entière**, de sa documentation à son accolade fermante : le
        // commentaire dit où s'ajoute une valeur de plus, et un bloc qui l'abrégerait
        // laisserait le lecteur sans cette consigne-là.
        let debut = modele
            .find("/// Valeurs acceptées par la colonne « statut ».")
            .expect("le modèle engendré documente l'énumération");
        let fin = debut
            + modele[debut..]
                .find("\n}")
                .expect("la déclaration se ferme")
            + 2;
        let declaration = &modele[debut..fin];

        assert!(
            declaration.contains("DeriveActiveEnum")
                && declaration.contains("pub enum ArticleStatut {"),
            "l'extraction doit couvrir la déclaration entière :\n{declaration}"
        );
        for ligne in declaration.lines().map(str::trim).filter(|l| !l.is_empty()) {
            assert!(
                colle.contains(ligne),
                "« {ligne} » manque au bloc, que `generate crud` écrit pourtant :\n{colle}"
            );
        }
    }

    /// Les lignes du bloc visant `fichier`, réunies.
    fn bloc(planned: &Planned, fichier: &str) -> String {
        planned
            .blocs
            .iter()
            .find(|bloc| bloc.fichier == fichier)
            .unwrap_or_else(|| panic!("aucun bloc pour {fichier}"))
            .lignes
            .join("\n")
    }

    /// Un module qui ne portait aucune énumération n'a pas les imports que le type collé
    /// réclame : le gabarit du modèle ne les pose que sous `{% if enum_types %}`. Sans eux,
    /// le développeur qui suit la consigne à la lettre récolte une erreur de compilation.
    #[test]
    fn the_model_block_carries_the_imports_the_module_lacked() {
        let (_parent, root) = projet();

        let planned = run(&options(
            &root,
            "ajoute_statut",
            "articles",
            "statut:enum(draft,published):optional",
        ))
        .expect("la migration doit s'écrire");

        let colle = bloc(&planned, "src/articles/model.rs");
        assert!(
            colle.contains("use serde::{Deserialize, Serialize};"),
            "l'import de serde manque :\n{colle}"
        );
        assert!(
            colle.contains("use utoipa::ToSchema;"),
            "l'import d'utoipa manque :\n{colle}"
        );
    }

    /// Le même module, une énumération déjà là : les imports y sont, et les répéter
    /// donnerait un doublon que rustc refuse.
    #[test]
    fn the_model_block_leaves_out_the_imports_the_module_already_has() {
        let (_parent, root) = crate::fixtures::project();
        crud(&root, "factures", "statut:enum(draft,sent)");

        let planned = run(&options(
            &root,
            "ajoute_etat",
            "factures",
            "etat:enum(neuf,ancien):optional",
        ))
        .expect("la migration doit s'écrire");

        let colle = bloc(&planned, "src/factures/model.rs");
        assert!(
            !colle.contains("use serde::"),
            "import déjà présent, proposé une seconde fois :\n{colle}"
        );
        assert!(
            !colle.contains("use utoipa::"),
            "import déjà présent, proposé une seconde fois :\n{colle}"
        );
        assert!(colle.contains("pub enum FactureEtat {"), "{colle}");
    }

    /// Le trio que l'en-tête du bloc nomme est celui que le module porte, et non celui
    /// que l'heuristique singulière redevine : un CRUD engendré avec `--singular` porte
    /// d'autres noms, et l'en-tête désignerait alors trois structures qui n'existent pas.
    #[test]
    fn the_dto_block_names_the_structs_the_module_really_carries() {
        let (_parent, root) = crate::fixtures::project();
        crud_singulier(&root, "news", "titre:string", "news_item");

        let planned = run(&options(&root, "ajoute_vues", "news", "vues:int:optional"))
            .expect("la migration doit s'écrire");

        let colle = bloc(&planned, "src/news/dto.rs");
        assert!(
            colle.contains("// dans `CreateNewsItem`, `UpdateNewsItem` et `NewsItemResponse`"),
            "l'en-tête doit nommer les structures du module :\n{colle}"
        );
    }

    /// Un `dto.rs` qui ne nomme aucun trio — réécrit à la main, ou absent — ne dit rien :
    /// l'heuristique reprend alors la main, et c'est le meilleur nom qui reste.
    #[test]
    fn the_dto_block_falls_back_to_the_heuristic_when_the_file_names_no_trio() {
        let (_parent, root) = projet();
        fs::write(
            root.join("src/articles/dto.rs"),
            "// ce fichier ne déclare plus rien que la commande sache lire\n",
        )
        .expect("le DTO du test doit s'écrire");

        let planned = run(&options(
            &root,
            "ajoute_vues",
            "articles",
            "vues:int:optional",
        ))
        .expect("la migration doit s'écrire");

        let colle = bloc(&planned, "src/articles/dto.rs");
        assert!(
            colle.contains("// dans `CreateArticle`, `UpdateArticle` et `ArticleResponse`"),
            "sans trio lisible, l'en-tête reste celui de l'heuristique :\n{colle}"
        );
    }

    /// Un `dto.rs` sans ligne `use super::model::` n'a rien à remplacer — ceux du
    /// fragment `auth` n'en portent pas, et ce sont les tables que le refus de la
    /// commande cite en exemple. Se taire livrerait un `Option<UserStatut>` sans dire d'où
    /// vient le type.
    #[test]
    fn the_dto_block_gives_the_whole_import_when_the_file_has_no_line_to_edit() {
        let (_parent, root) = crate::fixtures::Project::new().features(&["auth"]).create();

        let planned = run(&options(
            &root,
            "ajoute_statut",
            "users",
            "statut:enum(actif,suspendu):optional",
        ))
        .expect("la migration doit s'écrire");

        let colle = bloc(&planned, "src/auth/dto.rs");
        assert!(
            colle.contains("use super::model::UserStatut;"),
            "le bloc doit donner l'import à ajouter :\n{colle}"
        );
        assert!(
            !colle.contains("remplacez"),
            "le fichier ne porte aucune ligne d'import du modèle : rien à remplacer :\n{colle}"
        );
    }

    /// L'import du modèle dans les DTO est une ligne à **modifier** : le fichier en porte
    /// déjà une, et le type de l'énumération s'y joint. L'ajouter telle quelle la
    /// déclarerait deux fois.
    #[test]
    fn the_dto_block_says_the_model_import_is_an_edit() {
        let (_parent, root) = projet();

        let planned = run(&options(
            &root,
            "ajoute_statut",
            "articles",
            "statut:enum(draft,published):optional",
        ))
        .expect("la migration doit s'écrire");

        let colle = bloc(&planned, "src/articles/dto.rs");
        assert!(
            colle.contains("// remplacez `use super::model::Model;` par :"),
            "le bloc doit dire que la ligne se remplace :\n{colle}"
        );
        assert!(
            colle.contains("use super::model::{ArticleStatut, Model};"),
            "le bloc doit donner la ligne complète :\n{colle}"
        );
    }

    /// Un `decimal` demande aux DTO un import que le prélude ne donne pas nommément.
    #[test]
    fn the_dto_block_carries_the_prelude_import_a_decimal_needs() {
        let (_parent, root) = projet();

        let planned = run(&options(
            &root,
            "ajoute_prix",
            "articles",
            "prix:decimal:optional",
        ))
        .expect("la migration doit s'écrire");

        let colle = bloc(&planned, "src/articles/dto.rs");
        assert!(
            colle.contains("use sea_orm::prelude::Decimal;"),
            "l'import du décimal manque :\n{colle}"
        );
        assert!(
            !colle.contains("remplacez"),
            "sans énumération, la ligne d'import du modèle n'a pas à changer :\n{colle}"
        );
    }

    /// Le plan se calcule entièrement sans écrire : c'est ce que `--dry-run` montre.
    #[test]
    fn planning_writes_nothing() {
        let (_parent, root) = projet();
        let avant = empreinte(&root);

        plan_for(
            &options(&root, "ajoute_statut", "articles", "statut:string:optional"),
            HORODATAGE,
        )
        .expect("le plan doit se calculer");

        assert_eq!(empreinte(&root), avant, "un plan n'écrit rien");
    }

    /// Un nom qui ne fait pas un module Rust est refusé avant toute écriture :
    /// `DeriveMigrationName` en tire le nom de la migration en base.
    #[test]
    fn a_name_that_is_not_a_rust_identifier_is_refused() {
        let (_parent, root) = projet();
        let avant = empreinte(&root);

        let error = run(&options(
            &root,
            "ajoute-statut",
            "articles",
            "statut:string:optional",
        ))
        .expect_err("le tiret est refusé");

        assert!(matches!(error, Error::Nom(_)), "{error}");
        assert_eq!(empreinte(&root), avant, "rien ne doit avoir été écrit");
    }

    /// Sous `--json`, un refus se décide sur un code stable plutôt que sur un message
    /// français, et chaque code est en snake_case ASCII.
    #[test]
    fn each_refusal_carries_a_stable_snake_case_code() {
        let codes = [
            Error::PasUnProjet.code(),
            Error::TableSansModule {
                table: "factures".to_string(),
                connues: "articles".to_string(),
            }
            .code(),
            Error::ChampsVides {
                name: "ajoute_statut".to_string(),
            }
            .code(),
            Error::ColonneDejaDeclaree {
                champ: "titre".to_string(),
                table: "articles".to_string(),
                fichier: "src/articles/model.rs".to_string(),
            }
            .code(),
            Error::ColonneObligatoire {
                champ: "statut".to_string(),
            }
            .code(),
            Error::UniqueSurColonneAjoutee {
                champ: "slug".to_string(),
            }
            .code(),
            Error::ReferenceInterdite {
                champ: "author".to_string(),
            }
            .code(),
            Error::DecimalSousSqlite {
                champ: "prix".to_string(),
            }
            .code(),
        ];

        for code in codes {
            assert!(
                !code.is_empty() && code.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{code:?}"
            );
        }
    }
}
