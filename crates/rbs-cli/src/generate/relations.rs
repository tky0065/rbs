//! Confrontation des cibles déclarées dans `--fields` à ce que le projet contient.
//!
//! Séparé du parseur, qui reste pur : une chaîne s'analyse sans projet, une cible ne se
//! juge que contre un inventaire.

use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::Path;

use super::entities::{self, Entity};
use super::feature::{Feature, to_singular};
use super::fields::{Field, RelationView, to_pascal_case};

/// Une cible qu'aucune entité du projet ne porte.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UnknownTarget {
    /// Nom de la relation fautive : `author`.
    pub relation: String,
    /// Cible écrite : `writers`.
    pub target: String,
    /// Tables connues, triées.
    pub known: Vec<String>,
}

impl fmt::Display for UnknownTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Le préfixe « erreur : » appartient à la couche d'affichage, qui le pose déjà :
        // le porter ici aussi le double aux yeux de l'utilisateur.
        write!(
            f,
            "relation « {} » — « {} » est introuvable dans ce projet\n        \
             → entités connues : {}",
            self.relation,
            self.target,
            self.known.join(", ")
        )
    }
}

impl std::error::Error for UnknownTarget {}

/// Deux relations dont le nom se singularise en une seule variante.
///
/// Le modèle émet une variante `Relation` par référence : deux relations qui en visent
/// la même donnent un `enum` que rustc refuse, loin de la commande qui l'a écrit.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DuplicateVariant {
    /// La variante que les deux relations réclament : `Author`.
    pub variant: String,
    /// La relation qui l'a réservée la première : `author`.
    pub first: String,
    /// Celle qui la réclame ensuite : `authors`.
    pub second: String,
}

impl fmt::Display for DuplicateVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Le préfixe « erreur : » appartient à la couche d'affichage, qui le pose déjà :
        // le porter ici aussi le double aux yeux de l'utilisateur.
        write!(
            f,
            "relations « {} » et « {} » — toutes deux nommeraient la variante `{}`\n        \
             → `enum Relation` ne peut pas la déclarer deux fois : renommez l'une des deux",
            self.first, self.second, self.variant
        )
    }
}

impl std::error::Error for DuplicateVariant {}

/// Ce que la résolution des références peut relever, toutes fautes confondues.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ResolveError {
    UnknownTarget(UnknownTarget),
    DuplicateVariant(DuplicateVariant),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTarget(error) => error.fmt(f),
            Self::DuplicateVariant(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ResolveError {}

/// Résout chaque référence contre l'inventaire, et pose sa vue pour les templates.
///
/// `generated_table` rejoint les cibles admises : elle n'est pas encore sur le disque,
/// et une entité qui se référence elle-même — un arbre — est légitime.
pub(crate) fn resolve(
    fields: &mut [Field],
    entities: &[Entity],
    generated_table: &str,
) -> Result<(), Vec<ResolveError>> {
    let mut known = entities::tables(entities);
    if !known.iter().any(|table| table == generated_table) {
        known.push(generated_table.to_string());
        known.sort();
    }

    let mut errors = Vec::new();
    let mut variants: Vec<(String, String)> = Vec::new();

    for field in fields.iter_mut() {
        let Some(reference) = field.reference().cloned() else {
            continue;
        };

        // La variante se réserve avant la résolution de la cible : la collision tient au
        // seul nom de la relation, et se signale même si la cible est par ailleurs bonne.
        let variant = to_pascal_case(&to_singular(field.relation_name()));
        if let Some((_, first)) = variants.iter().find(|(seen, _)| *seen == variant) {
            errors.push(ResolveError::DuplicateVariant(DuplicateVariant {
                variant,
                first: first.clone(),
                second: field.relation_name().to_string(),
            }));
            continue;
        }
        variants.push((variant.clone(), field.relation_name().to_string()));

        let entity_path = if reference.target == generated_table {
            // L'entité se référence elle-même : son module n'existe pas encore, et
            // `Entity` la désigne depuis son propre fichier.
            "Entity".to_string()
        } else {
            match entities::find(entities, &reference.target) {
                Some(entity) => format!("{}::Entity", entity.module_path),
                None => {
                    errors.push(ResolveError::UnknownTarget(UnknownTarget {
                        relation: field.relation_name().to_string(),
                        target: reference.target.clone(),
                        known: known.clone(),
                    }));
                    continue;
                }
            }
        };

        // `Entity` désigne l'entité locale : sa colonne est `Column::Id`, sans chemin.
        let target_column_path = if entity_path == "Entity" {
            "Column::Id".to_string()
        } else {
            entity_path.replace("::Entity", "::Column::Id")
        };

        field.set_relation(RelationView {
            name: field.relation_name().to_string(),
            variant,
            target: reference.target.clone(),
            entity_path,
            target_column_path,
            target_iden: to_pascal_case(&reference.target),
            on_delete: reference.on_delete.action().to_string(),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Une cible existe dans le projet, mais aucune migration n'y crée sa table.
///
/// Distincte d'`UnknownTarget` : ici l'entité est réelle, seule sa table manque. Une clé
/// étrangère qui la viserait échouerait à l'application des migrations, loin de la
/// commande qui l'a posée.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TargetWithoutMigration {
    /// Nom de la relation fautive : `author`.
    pub relation: String,
    /// Cible dépourvue de migration : `users`.
    pub target: String,
}

impl fmt::Display for TargetWithoutMigration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Le préfixe « erreur : » appartient à la couche d'affichage, qui le pose déjà :
        // le porter ici aussi le double aux yeux de l'utilisateur.
        write!(
            f,
            "relation « {} » — « {} » n'a pas de migration dans ce projet\n        \
             → une clé étrangère la viserait avant qu'aucune migration ne crée sa table : \
             écrivez sa migration avec `rbs migrate new`",
            self.relation, self.target,
        )
    }
}

impl std::error::Error for TargetWithoutMigration {}

/// Vérifie que chaque cible résolue a réellement une table en projet.
///
/// `entities::scan` reconnaît une cible à la présence de son `model.rs`, pas de sa
/// migration : `rbs generate feature` écrit l'un sans l'autre. Sans ce second passage,
/// une référence vers une telle cible produirait une contrainte de clé étrangère qu'aucune
/// migration ne satisferait.
///
/// Une auto-référence n'est jamais concernée : sa migration s'écrit dans le même plan, pas
/// encore sur le disque au moment de cette vérification.
pub(crate) fn ensure_migrations_exist(
    fields: &[Field],
    root: &Path,
) -> Result<(), Vec<TargetWithoutMigration>> {
    let errors: Vec<TargetWithoutMigration> = fields
        .iter()
        .filter_map(|field| field.relation())
        .filter(|view| view.entity_path != "Entity")
        .filter(|view| !entities::has_migration(root, &view.target))
        .map(|view| TargetWithoutMigration {
            relation: view.name.clone(),
            target: view.target.clone(),
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Ce que le côté inverse d'une relation ajoute au modèle de sa cible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Inverse {
    /// Fichier du modèle cible, relatif à la racine : `src/auth/model.rs`.
    pub file: String,
    /// Table de l'entité visée : `users`.
    ///
    /// Elle qualifie les deux ancres du modèle — un fichier peut en porter plusieurs, et
    /// `src/auth/model.rs` en porte deux.
    pub entity: String,
    /// Lignes à insérer dans `<rbs:relations:…>` : la variante `has_many`, ou le
    /// commentaire qui dit pourquoi elle n'y est pas.
    pub variant: Vec<String>,
    /// Lignes de l'`impl Related`, à insérer dans `<rbs:related:…>`. Vides quand la cible
    /// est visée plusieurs fois : la variante `has_many` ne s'écrit pas non plus.
    pub related: Vec<String>,
}

/// Calcule, pour chaque référence, ce qu'il faut écrire dans le modèle de sa cible.
///
/// Déclarer `author:references:users` sur `posts` implique que `users` a des `posts` :
/// la relation n'est écrite qu'une fois, et son inverse en découle. Une auto-référence
/// est exclue — ses deux côtés vivent déjà dans le même fichier.
///
/// Une cible visée plusieurs fois ne reçoit qu'un commentaire, une fois : sa variante
/// `has_many` exigerait l'`impl Related` que l'ambiguïté interdit au côté portant.
pub(crate) fn inverses(fields: &[Field], feature: &Feature, entities: &[Entity]) -> Vec<Inverse> {
    let own_entity = format!("crate::{}::model::Entity", feature.module());
    let variant = to_pascal_case(feature.module());
    let ambiguous = feature.ambiguous_targets();

    let mut produced = Vec::new();
    let mut commented = HashSet::new();

    for field in fields {
        let Some(view) = field.relation() else {
            continue;
        };
        if view.target == feature.module() {
            continue;
        }
        let Some(target) = entities::find(entities, &view.target) else {
            continue;
        };

        if let Some(concurrent) = ambiguous.iter().find(|other| other.target == view.target) {
            // Une seule fois par table : le commentaire parle des relations concurrentes
            // au pluriel, et le répéter à chacune d'elles le dirait deux fois.
            if commented.insert(view.target.clone()) {
                produced.push(Inverse {
                    file: target.file.clone(),
                    entity: target.table.clone(),
                    variant: concurrent.inverse_comment(feature.module()),
                    related: Vec::new(),
                });
            }
            continue;
        }

        produced.push(Inverse {
            file: target.file.clone(),
            entity: target.table.clone(),
            // Sans indentation propre : l'ancre `<rbs:relations:…>` vit dans le corps de
            // l'énumération, et `anchors::insert` préfixe déjà chaque ligne de celle de
            // sa balise fermante. L'ajouter ici la doublerait.
            variant: vec![
                format!(r#"#[sea_orm(has_many = "{own_entity}")]"#),
                format!("{variant},"),
            ],
            // Trois lignes plutôt qu'une : rustfmt éclate un corps de fonction, et un
            // projet fraîchement engendré échouerait son premier `cargo fmt --check` sur
            // une ligne que le CLI a lui-même écrite.
            related: related_impl(&own_entity, &variant),
        });
    }

    produced
}

/// L'`impl Related` d'un côté inverse, à la mise en forme de rustfmt.
pub(crate) fn related_impl(entity_path: &str, variant: &str) -> Vec<String> {
    vec![
        format!("impl Related<{entity_path}> for Entity {{"),
        "    fn to() -> RelationDef {".to_string(),
        format!("        Relation::{variant}.def()"),
        "    }".to_string(),
        "}".to_string(),
    ]
}

/// Une variante déjà présente dans le fichier cible, mais visant une autre entité.
///
/// `anchors::insert` ne dédoublonne qu'une séquence identique dans son entier : deux
/// relations distinctes qui produiraient la même variante s'y inséreraient donc toutes
/// deux, ce que `rustc` refuse comme identifiant dupliqué (`E0428`). C'est ce cas précis
/// que ce contrôle referme, avant qu'aucune écriture n'ait lieu.
///
/// La comparaison ignore l'indentation, comme `anchors::insert` le fait lui-même pour
/// décider si une séquence est déjà posée : sans quoi une relation déjà écrite, retrouvée
/// indentée par l'ancre, ne serait plus reconnue comme identique et se verrait refusée à
/// tort au second passage.
pub(crate) fn homonymous_conflict(existing: &str, inverse: &Inverse) -> bool {
    // Un inverse sans `impl Related` n'écrit qu'un commentaire : il ne déclare aucun
    // identifiant, donc n'entre en conflit avec aucun.
    if inverse.related.is_empty() {
        return false;
    }

    let Some(variant_line) = inverse.variant.last() else {
        return false;
    };
    let already_written = inverse.variant.iter().all(|line| {
        existing
            .lines()
            .any(|existing_line| existing_line.trim() == line.trim())
    });

    !already_written
        && existing
            .lines()
            .any(|line| line.trim() == variant_line.trim())
}

/// Vérifie que `child` porte bien une colonne référençant `parent`.
///
/// Sans cette vérification, `--has-many` écrirait une variante que SeaORM rejetterait
/// quarante secondes plus tard, à la compilation.
///
/// Le chemin attendu vient du `module_path` que le scan a relevé, et n'est pas reconstruit
/// depuis le nom de la table : une entité nichée — `users`, sous `crate::auth::model::user`
/// — ne vit pas au chemin que son nom laisserait deviner, et l'attendre là déclarait
/// l'enfant sans clé alors qu'il la portait.
pub(crate) fn child_references(child: &Entity, parent: &Entity, root: &Path) -> bool {
    let Ok(source) = fs::read_to_string(root.join(&child.file)) else {
        return false;
    };
    let expected = format!(r#"belongs_to = "{}::Entity""#, parent.module_path);

    source.contains(&expected)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::generate::entities::Entity;
    use crate::generate::fields;

    fn inventory() -> Vec<Entity> {
        vec![
            Entity {
                table: "users".to_string(),
                module_path: "crate::auth::model::user".to_string(),
                file: "src/auth/model.rs".to_string(),
            },
            Entity {
                table: "tags".to_string(),
                module_path: "crate::tags::model".to_string(),
                file: "src/tags/model.rs".to_string(),
            },
        ]
    }

    /// Une entité de feature plate, telle que le scan la relèverait.
    fn parent(table: &str) -> Entity {
        Entity {
            table: table.to_string(),
            module_path: format!("crate::{table}::model"),
            file: format!("src/{table}/model.rs"),
        }
    }

    fn resolved(input: &str, generated: &str) -> Vec<fields::Field> {
        let mut parsed = fields::parse(input).expect("la chaîne doit être acceptée");
        resolve(&mut parsed, &inventory(), generated).expect("les cibles doivent se résoudre");
        parsed
    }

    #[test]
    fn a_target_in_a_nested_module_resolves_to_its_full_path() {
        let fields = resolved("author:references:users", "posts");
        let view = fields[0].relation().expect("la vue de relation est posée");

        assert_eq!(view.entity_path, "crate::auth::model::user::Entity");
        assert_eq!(
            view.target_column_path,
            "crate::auth::model::user::Column::Id"
        );
        assert_eq!(view.variant, "Author");
        assert_eq!(view.target_iden, "Users");
        assert_eq!(view.on_delete, "Restrict");
    }

    // Un arbre : l'entité en cours de génération n'est pas encore sur le disque, et
    // doit pourtant être une cible valable.
    #[test]
    fn the_entity_being_generated_is_a_valid_target() {
        let fields = resolved("parent:references:posts:optional", "posts");
        let view = fields[0].relation().expect("la vue de relation est posée");

        assert_eq!(view.entity_path, "Entity");
        assert_eq!(view.target_column_path, "Column::Id");
        assert_eq!(view.target_iden, "Posts");
    }

    fn unknown_target(error: &ResolveError) -> &UnknownTarget {
        match error {
            ResolveError::UnknownTarget(target) => target,
            other => panic!("cible inconnue attendue : {other:?}"),
        }
    }

    #[test]
    fn an_unknown_target_is_rejected_and_names_the_known_tables() {
        let mut parsed = fields::parse("author:references:writers").expect("acceptée");
        let errors = resolve(&mut parsed, &inventory(), "posts")
            .expect_err("une cible inconnue doit être refusée");

        assert_eq!(errors.len(), 1);
        let error = unknown_target(&errors[0]);
        assert_eq!(error.target, "writers");
        assert_eq!(error.relation, "author");
        assert_eq!(error.known, ["posts", "tags", "users"]);
    }

    #[test]
    fn every_unknown_target_is_collected_in_one_pass() {
        let mut parsed = fields::parse("a:references:x,b:references:y").expect("acceptée");
        let errors = resolve(&mut parsed, &inventory(), "posts").expect_err("refusée");

        assert_eq!(errors.len(), 2, "{errors:?}");
    }

    #[test]
    fn the_message_names_the_relation_the_target_and_the_known_tables() {
        let error = UnknownTarget {
            relation: "author".to_string(),
            target: "writers".to_string(),
            known: vec!["posts".to_string(), "users".to_string()],
        };
        let text = error.to_string();

        assert!(text.contains("« writers » est introuvable"), "{text}");
        assert!(text.contains("author"), "{text}");
        assert!(text.contains("posts, users"), "{text}");
    }

    /// `to_singular` retire le `s` final : `author` et `authors` visent la même variante
    /// `Author`, que le modèle déclarerait deux fois.
    #[test]
    fn two_relations_singularising_alike_are_rejected() {
        let mut parsed =
            fields::parse("author:references:users,authors:references:users").expect("acceptée");
        let errors =
            resolve(&mut parsed, &inventory(), "posts").expect_err("la variante en double");

        assert_eq!(errors.len(), 1, "{errors:?}");
        let ResolveError::DuplicateVariant(collision) = &errors[0] else {
            panic!("collision de variante attendue : {errors:?}");
        };
        assert_eq!(collision.variant, "Author");
        assert_eq!(collision.first, "author");
        assert_eq!(collision.second, "authors");
    }

    #[test]
    fn the_variant_collision_names_both_relations_and_the_variant() {
        let text = DuplicateVariant {
            variant: "Author".to_string(),
            first: "author".to_string(),
            second: "authors".to_string(),
        }
        .to_string();

        assert!(text.contains("« author »"), "{text}");
        assert!(text.contains("« authors »"), "{text}");
        assert!(text.contains("`Author`"), "{text}");
    }

    /// Deux relations vers la même table restent légitimes tant que leurs variantes
    /// diffèrent : c'est le cas que `ambiguous_targets` sait déjà traiter.
    #[test]
    fn two_relations_to_one_table_with_distinct_variants_pass() {
        let fields = resolved("author:references:users,reviewer:references:users", "posts");

        assert_eq!(fields[0].relation().expect("posée").variant, "Author");
        assert_eq!(fields[1].relation().expect("posée").variant, "Reviewer");
    }

    #[test]
    fn a_scalar_field_is_left_untouched() {
        let fields = resolved("title:string", "posts");

        assert!(fields[0].relation().is_none());
    }

    #[test]
    fn a_reference_produces_the_has_many_side_in_the_target_model() {
        let mut fields = fields::parse("author:references:users").expect("acceptée");
        resolve(&mut fields, &inventory(), "posts").expect("cibles résolues");
        let feature = Feature::fresh("posts", fields.clone());
        let produced = inverses(&fields, &feature, &inventory());

        assert_eq!(produced.len(), 1);
        assert_eq!(produced[0].file, "src/auth/model.rs");
        assert!(
            produced[0]
                .variant
                .join("\n")
                .contains(r#"#[sea_orm(has_many = "crate::posts::model::Entity")]"#),
            "{:?}",
            produced[0].variant
        );
        assert!(
            produced[0].variant.join("\n").contains("Posts,"),
            "{:?}",
            produced[0].variant
        );
        assert!(
            produced[0]
                .related
                .join("\n")
                .contains("impl Related<crate::posts::model::Entity> for Entity {"),
            "{:?}",
            produced[0].related
        );
    }

    // Une auto-référence a déjà ses deux côtés dans le même fichier : l'inverse y serait
    // une seconde variante homonyme.
    #[test]
    fn a_self_reference_produces_no_inverse() {
        let mut fields = fields::parse("parent:references:posts:optional").expect("acceptée");
        resolve(&mut fields, &[], "posts").expect("cibles résolues");
        let feature = Feature::fresh("posts", fields.clone());

        assert!(inverses(&fields, &feature, &[]).is_empty());
    }

    /// La symétrie qu'exige `EntityTrait::has_many<R>`, qui réclame `R: Related<Self>` :
    /// le côté portant n'écrit pas son `impl Related` quand deux relations se disputent
    /// la cible, et la variante `has_many` d'en face, qui l'exigerait, ne s'écrit pas non
    /// plus. Il ne reste qu'un commentaire, une fois pour les deux relations.
    #[test]
    fn two_references_to_one_target_leave_it_a_comment_instead_of_a_has_many() {
        let mut fields =
            fields::parse("author:references:users,reviewer:references:users").expect("acceptée");
        resolve(&mut fields, &inventory(), "posts").expect("cibles résolues");
        let feature = Feature::fresh("posts", fields.clone());
        let produced = inverses(&fields, &feature, &inventory());

        assert_eq!(produced.len(), 1, "{produced:?}");
        assert_eq!(produced[0].file, "src/auth/model.rs");
        assert_eq!(produced[0].entity, "users");
        assert!(
            produced[0].related.is_empty(),
            "aucun `impl Related` ne peut être posé : {produced:?}"
        );
        assert!(
            produced[0]
                .variant
                .iter()
                .all(|line| line.starts_with("//")),
            "seul un commentaire est écrit : {produced:?}"
        );
        assert!(
            produced[0]
                .variant
                .join(" ")
                .contains("`Author`, `Reviewer`"),
            "le commentaire doit nommer les relations concurrentes : {produced:?}"
        );
    }

    /// L'entité visée accompagne le fichier : `src/auth/model.rs` porte deux entités, et
    /// le fichier seul ne dirait pas laquelle reçoit la variante.
    #[test]
    fn an_inverse_names_the_entity_it_targets() {
        let mut fields = fields::parse("author:references:users").expect("acceptée");
        resolve(&mut fields, &inventory(), "posts").expect("cibles résolues");
        let feature = Feature::fresh("posts", fields.clone());
        let produced = inverses(&fields, &feature, &inventory());

        assert_eq!(produced[0].entity, "users");
    }

    /// Le `impl Related` est écrit à la mise en forme de rustfmt : un projet fraîchement
    /// engendré doit passer son premier `cargo fmt --check`.
    #[test]
    fn the_related_impl_is_written_the_way_rustfmt_would() {
        assert_eq!(
            related_impl("crate::posts::model::Entity", "Posts"),
            [
                "impl Related<crate::posts::model::Entity> for Entity {",
                "    fn to() -> RelationDef {",
                "        Relation::Posts.def()",
                "    }",
                "}",
            ]
        );
    }

    /// Une migration qui crée `iden`, dans un fichier dont le nom ne la nomme pas.
    fn migration(root: &Path, iden: &str) {
        fs::create_dir_all(root.join("migration/src")).expect("le répertoire se crée");
        fs::write(
            root.join("migration/src/m20260826_143000_create_tables.rs"),
            format!(
                "manager.create_table(Table::create().table({iden}::Table).to_owned());\n\
                 \n#[derive(DeriveIden)]\nenum {iden} {{\n    Table,\n}}\n"
            ),
        )
        .expect("l'écriture aboutit");
    }

    /// Le trou trouvé en relecture d'une tâche antérieure : `entities::scan` reconnaît
    /// une cible à son `model.rs`, pas à sa migration.
    #[test]
    fn a_target_without_a_migration_is_refused() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        migration(root.path(), "Tags");

        let fields = resolved("author:references:users", "posts");
        let errors =
            ensure_migrations_exist(&fields, root.path()).expect_err("users n'a pas de migration");

        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].target, "users");
        assert_eq!(errors[0].relation, "author");
    }

    #[test]
    fn a_target_with_its_migration_is_accepted() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");
        migration(root.path(), "Users");

        let fields = resolved("author:references:users", "posts");

        assert!(ensure_migrations_exist(&fields, root.path()).is_ok());
    }

    // L'auto-référence vise l'entité en cours de génération : sa migration n'est pas
    // encore sur le disque au moment de cette vérification, et ne doit pas être exigée.
    #[test]
    fn a_self_reference_does_not_need_an_existing_migration() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");

        let fields = resolved("parent:references:posts:optional", "posts");

        assert!(ensure_migrations_exist(&fields, root.path()).is_ok());
    }

    #[test]
    fn a_scalar_field_needs_no_migration_check() {
        let root = TempDir::new().expect("le répertoire temporaire se crée");

        let fields = resolved("title:string", "posts");

        assert!(ensure_migrations_exist(&fields, root.path()).is_ok());
    }

    #[test]
    fn a_variant_absent_from_the_target_is_not_a_conflict() {
        let inverse = Inverse {
            file: "src/users/model.rs".to_string(),
            entity: "users".to_string(),
            variant: vec![
                r#"    #[sea_orm(has_many = "crate::posts::model::Entity")]"#.to_string(),
                "    Posts,".to_string(),
            ],
            related: related_impl("crate::posts::model::Entity", "Posts"),
        };

        assert!(!homonymous_conflict(
            "    // <rbs:relations:users>\n",
            &inverse
        ));
    }

    #[test]
    fn the_exact_same_block_already_written_is_not_a_conflict() {
        let inverse = Inverse {
            file: "src/users/model.rs".to_string(),
            entity: "users".to_string(),
            variant: vec![
                r#"    #[sea_orm(has_many = "crate::posts::model::Entity")]"#.to_string(),
                "    Posts,".to_string(),
            ],
            related: related_impl("crate::posts::model::Entity", "Posts"),
        };
        let existing = format!(
            "    // <rbs:relations:users>\n{}\n    // </rbs:relations:users>\n",
            inverse.variant.join("\n")
        );

        assert!(!homonymous_conflict(&existing, &inverse));
    }

    // Le cas que `anchors::insert` ne voit pas : le nom de la variante est déjà pris,
    // mais par une relation qui vise une autre entité.
    #[test]
    fn a_variant_already_taken_by_another_target_is_a_conflict() {
        let inverse = Inverse {
            file: "src/users/model.rs".to_string(),
            entity: "users".to_string(),
            variant: vec![
                r#"    #[sea_orm(has_many = "crate::posts::model::Entity")]"#.to_string(),
                "    Posts,".to_string(),
            ],
            related: related_impl("crate::posts::model::Entity", "Posts"),
        };
        let existing = "    // <rbs:relations:users>\n    \
             #[sea_orm(has_many = \"crate::somewhere::model::Entity\")]\n    Posts,\n    \
             // </rbs:relations:users>\n";

        assert!(homonymous_conflict(existing, &inverse));
    }

    #[test]
    fn a_child_referencing_the_table_is_a_valid_has_many() {
        let root = TempDir::new().expect("le répertoire se crée");
        fs::create_dir_all(root.path().join("src/comments")).expect("le répertoire se crée");
        fs::write(
            root.path().join("src/comments/model.rs"),
            "#[sea_orm(belongs_to = \"crate::posts::model::Entity\", from = \"Column::PostId\", to = \"crate::posts::model::Column::Id\")]\npub struct Model {}\n",
        )
        .expect("l'écriture aboutit");
        let child = Entity {
            table: "comments".to_string(),
            module_path: "crate::comments::model".to_string(),
            file: "src/comments/model.rs".to_string(),
        };

        assert!(child_references(&child, &parent("posts"), root.path()));
    }

    /// Le chemin attendu vient du `module_path` du parent, non de son nom de table :
    /// `users` vit sous `crate::auth::model::user`, et l'attendre sous
    /// `crate::users::model` déclarait l'enfant sans clé alors qu'il la portait.
    #[test]
    fn a_child_referencing_a_nested_parent_is_a_valid_has_many() {
        let root = TempDir::new().expect("le répertoire se crée");
        fs::create_dir_all(root.path().join("src/posts")).expect("le répertoire se crée");
        fs::write(
            root.path().join("src/posts/model.rs"),
            "#[sea_orm(belongs_to = \"crate::auth::model::user::Entity\", from = \"Column::AuthorId\", to = \"crate::auth::model::user::Column::Id\")]\npub struct Model {}\n",
        )
        .expect("l'écriture aboutit");
        let child = Entity {
            table: "posts".to_string(),
            module_path: "crate::posts::model".to_string(),
            file: "src/posts/model.rs".to_string(),
        };
        let users = Entity {
            table: "users".to_string(),
            module_path: "crate::auth::model::user".to_string(),
            file: "src/auth/model.rs".to_string(),
        };

        assert!(child_references(&child, &users, root.path()));
    }

    #[test]
    fn a_child_without_a_key_towards_us_is_not_a_valid_has_many() {
        let root = TempDir::new().expect("le répertoire se crée");
        fs::create_dir_all(root.path().join("src/comments")).expect("le répertoire se crée");
        fs::write(
            root.path().join("src/comments/model.rs"),
            "pub struct Model {}\n",
        )
        .expect("l'écriture aboutit");
        let child = Entity {
            table: "comments".to_string(),
            module_path: "crate::comments::model".to_string(),
            file: "src/comments/model.rs".to_string(),
        };

        assert!(!child_references(&child, &parent("posts"), root.path()));
    }
}
