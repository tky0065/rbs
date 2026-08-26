# Parseur de champs — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transformer la chaîne `--fields "name:string,email:string:unique"` en un `Vec<Champ>` validé, projetant chaque type vers Rust et vers SeaORM.

**Architecture:** Fonction pure sur une chaîne, sans accès disque ni base. La grammaire est plate — deux séparateurs, aucune récursion — donc analyse manuelle par `split`, sans crate de parsing. La connaissance des sept types est concentrée sur `TypeChamp` ; les générateurs suivants consomment des chaînes déjà résolues.

**Tech Stack:** Rust 2024, `serde` (sérialisation manuelle pour minijinja), tests unitaires inline.

**Spec:** `docs/superpowers/specs/2026-08-26-d1-parseur-de-champs-design.md`

## Global Constraints

- Le code de `rbs-cli` est nommé **en français**. L'anglais est réservé à `rbs-core`, publiée sur crates.io.
- Visibilité `pub(crate)` : rien de tout ceci n'est une API publique.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` sont bloquants en CI.
- Un commentaire explique le *pourquoi*, jamais le *quoi*. Un commentaire qui paraphrase la ligne suivante se supprime.
- Commits en Conventional Commits, sujet en français à l'impératif, sans identifiant de tâche ni mention d'outil.
- Grammaire fermée : sept types (`string`, `int`, `float`, `bool`, `uuid`, `datetime`, `text`), trois modificateurs (`unique`, `optional`, `index`). Ne rien ajouter.
- `id`, `created_at`, `updated_at` sont posés par rbs et ne se déclarent jamais.

## Structure de fichiers

| Fichier | Responsabilité |
|---|---|
| `crates/rbs-cli/src/generate/mod.rs` | Déclare le module de la commande `rbs generate` |
| `crates/rbs-cli/src/generate/champs.rs` | `Champ`, `TypeChamp`, projections, `analyser`, validation |
| `crates/rbs-cli/src/generate/champs/erreur.rs` | `ErreurChamps`, `ErreurChamp`, `NatureErreur`, messages, suggestions |
| `crates/rbs-cli/src/main.rs` | Une ligne : `mod generate;` |
| `crates/rbs-cli/Cargo.toml` | `serde_json` en dev-dependency (tâche 5) |

Les tests vivent en `#[cfg(test)] mod tests` dans le fichier qu'ils couvrent, comme partout ailleurs dans le dépôt.

---

### Task 1: Modèle des champs et projections de types

**Files:**
- Create: `crates/rbs-cli/src/generate/mod.rs`
- Create: `crates/rbs-cli/src/generate/champs.rs`
- Modify: `crates/rbs-cli/src/main.rs` (ajouter `mod generate;` après `mod cli;`)

**Interfaces:**
- Consumes: rien.
- Produces: `TypeChamp` (`String`, `Int`, `Float`, `Bool`, `Uuid`, `Datetime`, `Text`), `TypeChamp::NOMS: [&str; 7]`, `TypeChamp::analyser(&str) -> Option<TypeChamp>`, `TypeChamp::nom(self) -> &'static str`, `TypeChamp::type_rust(self) -> &'static str`, `TypeChamp::methode_migration(self) -> &'static str`, `TypeChamp::attribut_column_type(self) -> Option<&'static str>`, `Champ { nom: String, type_: TypeChamp, unique: bool, optionnel: bool, index: bool }`, `Champ::type_rust(&self) -> String`.

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `crates/rbs-cli/src/generate/mod.rs` :

```rust
#![allow(dead_code)] // Le premier appelant est le générateur d'entité, tâche suivante.

pub(crate) mod champs;
```

Créer `crates/rbs-cli/src/generate/champs.rs` avec les seuls tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_type_de_la_grammaire_est_reconnu() {
        let cas = [
            ("string", TypeChamp::String),
            ("int", TypeChamp::Int),
            ("float", TypeChamp::Float),
            ("bool", TypeChamp::Bool),
            ("uuid", TypeChamp::Uuid),
            ("datetime", TypeChamp::Datetime),
            ("text", TypeChamp::Text),
        ];

        for (mot, attendu) in cas {
            assert_eq!(TypeChamp::analyser(mot), Some(attendu), "type « {mot} »");
        }
    }

    #[test]
    fn un_type_hors_grammaire_n_est_pas_reconnu() {
        assert_eq!(TypeChamp::analyser("decimal"), None);
        assert_eq!(TypeChamp::analyser("String"), None);
        assert_eq!(TypeChamp::analyser(""), None);
    }

    #[test]
    fn nom_est_l_inverse_de_analyser() {
        for mot in TypeChamp::NOMS {
            let type_ = TypeChamp::analyser(mot).expect("NOMS ne liste que des types connus");
            assert_eq!(type_.nom(), mot);
        }
    }

    #[test]
    fn chaque_type_se_projette_vers_rust() {
        assert_eq!(TypeChamp::String.type_rust(), "String");
        assert_eq!(TypeChamp::Text.type_rust(), "String");
        assert_eq!(TypeChamp::Int.type_rust(), "i32");
        assert_eq!(TypeChamp::Float.type_rust(), "f64");
        assert_eq!(TypeChamp::Bool.type_rust(), "bool");
        assert_eq!(TypeChamp::Uuid.type_rust(), "Uuid");
        assert_eq!(TypeChamp::Datetime.type_rust(), "DateTimeWithTimeZone");
    }

    #[test]
    fn chaque_type_se_projette_vers_une_methode_de_migration() {
        assert_eq!(TypeChamp::String.methode_migration(), "string()");
        assert_eq!(TypeChamp::Text.methode_migration(), "text()");
        assert_eq!(TypeChamp::Int.methode_migration(), "integer()");
        assert_eq!(TypeChamp::Float.methode_migration(), "double()");
        assert_eq!(TypeChamp::Bool.methode_migration(), "boolean()");
        assert_eq!(TypeChamp::Uuid.methode_migration(), "uuid()");
        assert_eq!(
            TypeChamp::Datetime.methode_migration(),
            "timestamp_with_time_zone()"
        );
    }

    #[test]
    fn seul_text_porte_un_attribut_column_type() {
        assert_eq!(TypeChamp::Text.attribut_column_type(), Some("Text"));
        for mot in TypeChamp::NOMS {
            if mot == "text" {
                continue;
            }
            let type_ = TypeChamp::analyser(mot).expect("NOMS ne liste que des types connus");
            assert_eq!(type_.attribut_column_type(), None, "type « {mot} »");
        }
    }

    #[test]
    fn un_champ_optionnel_enveloppe_son_type_rust() {
        let obligatoire = Champ {
            nom: "titre".to_string(),
            type_: TypeChamp::String,
            unique: false,
            optionnel: false,
            index: false,
        };
        let optionnel = Champ {
            optionnel: true,
            ..obligatoire.clone()
        };

        assert_eq!(obligatoire.type_rust(), "String");
        assert_eq!(optionnel.type_rust(), "Option<String>");
    }
}
```

Ajouter `mod generate;` dans `crates/rbs-cli/src/main.rs`, en gardant l'ordre alphabétique :

```rust
mod cli;
mod generate;
mod metadata;
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli generate::champs`
Expected: FAIL à la compilation — `cannot find type TypeChamp in this scope`.

- [ ] **Step 3: Écrire l'implémentation minimale**

En tête de `crates/rbs-cli/src/generate/champs.rs`, avant le `mod tests` :

```rust
/// Un des sept types de la grammaire `--fields`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeChamp {
    String,
    Int,
    Float,
    Bool,
    Uuid,
    Datetime,
    Text,
}

impl TypeChamp {
    pub(crate) const NOMS: [&'static str; 7] = [
        "string", "int", "float", "bool", "uuid", "datetime", "text",
    ];

    pub(crate) fn analyser(mot: &str) -> Option<Self> {
        Some(match mot {
            "string" => Self::String,
            "int" => Self::Int,
            "float" => Self::Float,
            "bool" => Self::Bool,
            "uuid" => Self::Uuid,
            "datetime" => Self::Datetime,
            "text" => Self::Text,
            _ => return None,
        })
    }

    pub(crate) fn nom(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Uuid => "uuid",
            Self::Datetime => "datetime",
            Self::Text => "text",
        }
    }

    pub(crate) fn type_rust(self) -> &'static str {
        match self {
            Self::String | Self::Text => "String",
            Self::Int => "i32",
            Self::Float => "f64",
            Self::Bool => "bool",
            Self::Uuid => "Uuid",
            Self::Datetime => "DateTimeWithTimeZone",
        }
    }

    pub(crate) fn methode_migration(self) -> &'static str {
        match self {
            Self::String => "string()",
            Self::Int => "integer()",
            Self::Float => "double()",
            Self::Bool => "boolean()",
            Self::Uuid => "uuid()",
            Self::Datetime => "timestamp_with_time_zone()",
            Self::Text => "text()",
        }
    }

    /// SeaORM déduit la colonne du type Rust ; seul `text` doit être forcé, `String`
    /// donnant sinon un `varchar`.
    pub(crate) fn attribut_column_type(self) -> Option<&'static str> {
        match self {
            Self::Text => Some("Text"),
            _ => None,
        }
    }
}

/// Un champ déclaré dans `--fields`, une fois analysé et validé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Champ {
    pub nom: String,
    pub type_: TypeChamp,
    pub unique: bool,
    pub optionnel: bool,
    pub index: bool,
}

impl Champ {
    pub(crate) fn type_rust(&self) -> String {
        if self.optionnel {
            format!("Option<{}>", self.type_.type_rust())
        } else {
            self.type_.type_rust().to_string()
        }
    }
}
```

- [ ] **Step 4: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-cli generate::champs`
Expected: PASS — 7 tests.

- [ ] **Step 5: Vérifier le lint et le format**

Run: `cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected: aucune sortie.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/src/generate/ crates/rbs-cli/src/main.rs
git commit -m "feat(cli): décrit les sept types de champs et leurs projections

Chaque type se projette vers un type Rust, une méthode du constructeur de
migration SeaORM et, pour text seulement, un attribut de colonne. Cette
connaissance tient dans un seul fichier : ajouter un type sera une variante
et trois bras de match, sans toucher aux générateurs.

Vérifications :
- cargo test -p rbs-cli generate::champs → 7 passed
- cargo clippy -p rbs-cli --all-targets -- -D warnings → propre"
```

---

### Task 2: Erreurs, messages et suggestions

**Files:**
- Create: `crates/rbs-cli/src/generate/champs/erreur.rs`
- Modify: `crates/rbs-cli/src/generate/champs.rs` (ajouter `mod erreur;` et le ré-export en tête)

**Interfaces:**
- Consumes: `TypeChamp::NOMS` de la tâche 1.
- Produces: `ErreurChamps { erreurs: Vec<ErreurChamp> }` implémentant `Display` et `std::error::Error` ; `ErreurChamp { rang: usize, libelle: String, nature: NatureErreur }` ; `NatureErreur` à huit variantes ; `en_snake_case(&str) -> String` ; `suggestions_mot_cle(&str) -> Vec<String>`.

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `crates/rbs-cli/src/generate/champs/erreur.rs` avec les seuls tests :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn rendu(nature: NatureErreur, libelle: &str) -> String {
        ErreurChamps {
            erreurs: vec![ErreurChamp {
                rang: 1,
                libelle: libelle.to_string(),
                nature,
            }],
        }
        .to_string()
    }

    #[test]
    fn une_forme_invalide_montre_la_forme_attendue() {
        let texte = rendu(NatureErreur::FormeInvalide, "titre");
        assert_eq!(
            texte,
            "erreur : champ 1 « titre » — forme attendue : « nom:type[:modificateur…] »\n\
             \x20       → exemple : « email:string:unique »"
        );
    }

    #[test]
    fn un_nom_mal_casse_suggere_sa_forme_snake_case() {
        let texte = rendu(
            NatureErreur::PasEnSnakeCase {
                suggestion: Some("title".to_string()),
            },
            "Title",
        );
        assert!(texte.contains("le nom doit être en snake_case"), "{texte}");
        assert!(texte.contains("→ essayez « title »"), "{texte}");
    }

    #[test]
    fn un_nom_sans_recasse_possible_n_a_pas_d_indice() {
        let texte = rendu(NatureErreur::PasEnSnakeCase { suggestion: None }, "prénom");

        assert!(
            texte.contains("minuscules ASCII, chiffres et souligné"),
            "{texte}"
        );
        assert!(!texte.contains("→"), "{texte}");
    }

    #[test]
    fn un_mot_cle_rust_suggere_ses_deux_replis() {
        let texte = rendu(
            NatureErreur::MotCleRust {
                suggestions: vec!["kind".to_string(), "type_".to_string()],
            },
            "type",
        );
        assert!(texte.contains("« type » est un mot-clé Rust"), "{texte}");
        assert!(texte.contains("→ essayez « kind » ou « type_ »"), "{texte}");
    }

    #[test]
    fn un_nom_reserve_rappelle_les_trois_colonnes_implicites() {
        let texte = rendu(NatureErreur::NomReserve, "id");
        assert!(texte.contains("« id » ne se déclare pas"), "{texte}");
        assert!(
            texte.contains("id, created_at et updated_at sont posés sur toute entité"),
            "{texte}"
        );
    }

    #[test]
    fn un_type_inconnu_liste_les_types_admis() {
        let texte = rendu(
            NatureErreur::TypeInconnu {
                nom: "decimal".to_string(),
            },
            "prix",
        );
        assert!(texte.contains("type inconnu « decimal »"), "{texte}");
        for mot in TypeChamp::NOMS {
            assert!(texte.contains(mot), "« {mot} » absent de : {texte}");
        }
    }

    #[test]
    fn un_modificateur_inconnu_liste_les_trois_admis() {
        let texte = rendu(
            NatureErreur::ModificateurInconnu {
                nom: "uniq".to_string(),
            },
            "name",
        );
        assert!(texte.contains("modificateur inconnu « uniq »"), "{texte}");
        assert!(texte.contains("unique, optional, index"), "{texte}");
    }

    #[test]
    fn un_modificateur_en_double_est_nomme() {
        let texte = rendu(
            NatureErreur::ModificateurEnDouble {
                nom: "unique".to_string(),
            },
            "email",
        );
        assert!(
            texte.contains("modificateur « unique » en double"),
            "{texte}"
        );
    }

    #[test]
    fn un_index_redondant_explique_pourquoi() {
        let texte = rendu(NatureErreur::IndexRedondant, "slug");
        assert!(
            texte.contains("« index » redondant : « unique » pose déjà un index"),
            "{texte}"
        );
        assert!(texte.contains("→ retirez « index »"), "{texte}");
    }

    #[test]
    fn plusieurs_erreurs_se_rendent_une_par_bloc_dans_l_ordre() {
        let texte = ErreurChamps {
            erreurs: vec![
                ErreurChamp {
                    rang: 1,
                    libelle: "Title".to_string(),
                    nature: NatureErreur::PasEnSnakeCase {
                        suggestion: Some("title".to_string()),
                    },
                },
                ErreurChamp {
                    rang: 2,
                    libelle: "type".to_string(),
                    nature: NatureErreur::MotCleRust {
                        suggestions: vec!["kind".to_string(), "type_".to_string()],
                    },
                },
            ],
        }
        .to_string();

        let lignes: Vec<&str> = texte.lines().collect();
        assert_eq!(lignes.len(), 4, "{texte}");
        assert!(lignes[0].starts_with("erreur : champ 1 « Title »"), "{texte}");
        assert!(lignes[2].starts_with("erreur : champ 2 « type »"), "{texte}");
    }

    #[test]
    fn en_snake_case_recasse_les_formes_usuelles() {
        assert_eq!(en_snake_case("Title"), "title");
        assert_eq!(en_snake_case("firstName"), "first_name");
        assert_eq!(en_snake_case("HTTPStatus"), "h_t_t_p_status");
        assert_eq!(en_snake_case("mon-champ"), "mon_champ");
        assert_eq!(en_snake_case("déjà_ok"), "déjà_ok");
    }

    #[test]
    fn un_mot_cle_courant_a_un_alias_avant_son_repli() {
        assert_eq!(suggestions_mot_cle("type"), vec!["kind", "type_"]);
        assert_eq!(suggestions_mot_cle("ref"), vec!["reference", "ref_"]);
        assert_eq!(suggestions_mot_cle("loop"), vec!["loop_"]);
    }
}
```

Note sur `HTTPStatus` → `h_t_t_p_status` : la conversion est mécanique et n'essaie pas de deviner les acronymes. C'est une suggestion, pas une correction automatique.

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli generate::champs::erreur`
Expected: FAIL à la compilation — `cannot find type NatureErreur in this scope`.

- [ ] **Step 3: Écrire l'implémentation minimale**

En tête de `crates/rbs-cli/src/generate/champs/erreur.rs`, avant le `mod tests` :

```rust
use std::fmt;

use super::TypeChamp;

/// Toutes les fautes relevées dans une chaîne `--fields`, dans l'ordre des champs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ErreurChamps {
    pub erreurs: Vec<ErreurChamp>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ErreurChamp {
    /// Rang du champ dans la chaîne, à partir de 1.
    pub rang: usize,
    /// Le nom du champ, ou la portion brute quand le nom n'a pas pu être lu.
    pub libelle: String,
    pub nature: NatureErreur,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NatureErreur {
    FormeInvalide,
    PasEnSnakeCase { suggestion: Option<String> },
    MotCleRust { suggestions: Vec<String> },
    NomReserve,
    TypeInconnu { nom: String },
    ModificateurInconnu { nom: String },
    ModificateurEnDouble { nom: String },
    IndexRedondant,
}

impl NatureErreur {
    fn message(&self, libelle: &str) -> String {
        match self {
            Self::FormeInvalide => {
                "forme attendue : « nom:type[:modificateur…] »".to_string()
            }
            Self::PasEnSnakeCase { .. } => {
                "le nom doit être en snake_case : minuscules ASCII, chiffres et souligné"
                    .to_string()
            }
            Self::MotCleRust { .. } => format!("« {libelle} » est un mot-clé Rust"),
            Self::NomReserve => format!("« {libelle} » ne se déclare pas"),
            Self::TypeInconnu { nom } => format!("type inconnu « {nom} »"),
            Self::ModificateurInconnu { nom } => format!("modificateur inconnu « {nom} »"),
            Self::ModificateurEnDouble { nom } => {
                format!("modificateur « {nom} » en double")
            }
            Self::IndexRedondant => {
                "« index » redondant : « unique » pose déjà un index".to_string()
            }
        }
    }

    fn indice(&self) -> Option<String> {
        match self {
            Self::FormeInvalide => Some("exemple : « email:string:unique »".to_string()),
            Self::PasEnSnakeCase { suggestion } => suggestion
                .as_ref()
                .map(|valeur| format!("essayez « {valeur} »")),
            Self::MotCleRust { suggestions } => {
                let liste: Vec<String> =
                    suggestions.iter().map(|s| format!("« {s} »")).collect();
                Some(format!("essayez {}", liste.join(" ou ")))
            }
            Self::NomReserve => {
                Some("id, created_at et updated_at sont posés sur toute entité".to_string())
            }
            Self::TypeInconnu { .. } => Some(TypeChamp::NOMS.join(", ")),
            Self::ModificateurInconnu { .. } => Some("unique, optional, index".to_string()),
            Self::ModificateurEnDouble { .. } => None,
            Self::IndexRedondant => Some("retirez « index »".to_string()),
        }
    }
}

impl fmt::Display for ErreurChamps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut premier = true;
        for erreur in &self.erreurs {
            if !premier {
                writeln!(f)?;
            }
            premier = false;

            write!(
                f,
                "erreur : champ {} « {} » — {}",
                erreur.rang,
                erreur.libelle,
                erreur.nature.message(&erreur.libelle)
            )?;

            if let Some(indice) = erreur.nature.indice() {
                write!(f, "\n        → {indice}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ErreurChamps {}

/// Recasse un nom en snake_case sans chercher à interpréter les acronymes : la sortie
/// est une suggestion soumise à l'utilisateur, pas une correction appliquée d'office.
pub(crate) fn en_snake_case(nom: &str) -> String {
    let mut sortie = String::with_capacity(nom.len() + 4);

    for (rang, caractere) in nom.chars().enumerate() {
        if caractere.is_uppercase() {
            if rang > 0 && !sortie.ends_with('_') {
                sortie.push('_');
            }
            sortie.extend(caractere.to_lowercase());
        } else if caractere == '-' || caractere == ' ' {
            if !sortie.is_empty() && !sortie.ends_with('_') {
                sortie.push('_');
            }
        } else {
            sortie.push(caractere);
        }
    }

    sortie
}

/// Le suffixe `_` marche pour tout mot-clé ; les quatre alias devant lui sont ceux
/// qu'un développeur écrirait de lui-même.
pub(crate) fn suggestions_mot_cle(mot: &str) -> Vec<String> {
    let alias = match mot {
        "type" => Some("kind"),
        "ref" => Some("reference"),
        "match" => Some("matching"),
        "move" => Some("movement"),
        _ => None,
    };

    alias
        .map(str::to_string)
        .into_iter()
        .chain(std::iter::once(format!("{mot}_")))
        .collect()
}
```

Dans `crates/rbs-cli/src/generate/champs.rs`, ajouter en tête :

```rust
mod erreur;

pub(crate) use erreur::{ErreurChamp, ErreurChamps, NatureErreur};
```

- [ ] **Step 4: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-cli generate::champs`
Expected: PASS — 19 tests (7 de la tâche 1, 12 ici).

- [ ] **Step 5: Vérifier le lint et le format**

Run: `cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected: aucune sortie.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/src/generate/
git commit -m "feat(cli): rend les fautes de la chaîne de champs avec leur correction

Une faute de frappe dans --fields ne se voyait qu'à la compilation du projet
généré, sous un message de rustc sans rapport avec ce que l'utilisateur a
écrit. Chaque nature de faute porte maintenant son diagnostic et, quand la
correction est calculable, la valeur à essayer.

Vérifications :
- cargo test -p rbs-cli generate::champs → 19 passed
- cargo clippy -p rbs-cli --all-targets -- -D warnings → propre"
```

---

### Task 3: Analyse nominale de la chaîne

**Files:**
- Modify: `crates/rbs-cli/src/generate/champs.rs`

**Interfaces:**
- Consumes: `Champ`, `TypeChamp` (tâche 1), `ErreurChamps`, `ErreurChamp`, `NatureErreur` (tâche 2).
- Produces: `analyser(entree: &str) -> Result<Vec<Champ>, ErreurChamps>`.

Cette tâche traite le chemin passant. Les refus de noms, de types et de modificateurs incohérents sont la tâche 4 : ici, seule la variante `FormeInvalide` est produite, et un type inconnu remonte `TypeInconnu`.

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans le `mod tests` de `crates/rbs-cli/src/generate/champs.rs` :

```rust
    fn champs(entree: &str) -> Vec<Champ> {
        analyser(entree).expect("la chaîne doit être valide")
    }

    #[test]
    fn une_chaine_vide_ne_declare_aucun_champ() {
        assert_eq!(analyser(""), Ok(Vec::new()));
        assert_eq!(analyser("   "), Ok(Vec::new()));
    }

    #[test]
    fn un_champ_sans_modificateur_a_ses_trois_drapeaux_baisses() {
        let champs = champs("titre:string");

        assert_eq!(champs.len(), 1);
        assert_eq!(champs[0].nom, "titre");
        assert_eq!(champs[0].type_, TypeChamp::String);
        assert!(!champs[0].unique);
        assert!(!champs[0].optionnel);
        assert!(!champs[0].index);
    }

    #[test]
    fn chaque_modificateur_leve_son_drapeau() {
        assert!(champs("email:string:unique")[0].unique);
        assert!(champs("bio:text:optional")[0].optionnel);
        assert!(champs("slug:string:index")[0].index);
    }

    #[test]
    fn l_ordre_des_modificateurs_est_libre() {
        assert_eq!(
            champs("email:string:unique:optional"),
            champs("email:string:optional:unique")
        );
    }

    #[test]
    fn les_espaces_autour_des_separateurs_sont_ignores() {
        assert_eq!(
            champs(" titre : string , email : string : unique "),
            champs("titre:string,email:string:unique")
        );
    }

    #[test]
    fn les_champs_gardent_leur_ordre_de_declaration() {
        let noms: Vec<&str> = champs("un:string,deux:int,trois:bool")
            .iter()
            .map(|champ| champ.nom.as_str())
            .collect();

        assert_eq!(noms, ["un", "deux", "trois"]);
    }

    #[test]
    fn un_champ_sans_type_est_une_forme_invalide() {
        let erreur = analyser("titre").expect_err("un champ sans type est refusé");

        assert_eq!(erreur.erreurs.len(), 1);
        assert_eq!(erreur.erreurs[0].rang, 1);
        assert_eq!(erreur.erreurs[0].libelle, "titre");
        assert_eq!(erreur.erreurs[0].nature, NatureErreur::FormeInvalide);
    }

    #[test]
    fn une_virgule_finale_est_une_forme_invalide() {
        let erreur = analyser("titre:string,").expect_err("la virgule finale est refusée");

        assert_eq!(erreur.erreurs.len(), 1);
        assert_eq!(erreur.erreurs[0].rang, 2);
        assert_eq!(erreur.erreurs[0].nature, NatureErreur::FormeInvalide);
    }

    #[test]
    fn un_type_hors_grammaire_est_signale_sur_son_champ() {
        let erreur = analyser("prix:decimal").expect_err("decimal n'est pas dans la grammaire");

        assert_eq!(erreur.erreurs[0].libelle, "prix");
        assert_eq!(
            erreur.erreurs[0].nature,
            NatureErreur::TypeInconnu {
                nom: "decimal".to_string()
            }
        );
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli generate::champs`
Expected: FAIL à la compilation — `cannot find function analyser in this scope`.

- [ ] **Step 3: Écrire l'implémentation minimale**

Ajouter dans `crates/rbs-cli/src/generate/champs.rs`, après `impl Champ` :

```rust
/// Analyse la chaîne `--fields`. Les fautes de tous les champs sont collectées en une
/// passe : l'utilisateur corrige sa ligne d'un coup plutôt qu'une faute par exécution.
pub(crate) fn analyser(entree: &str) -> Result<Vec<Champ>, ErreurChamps> {
    if entree.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut champs = Vec::new();
    let mut erreurs = Vec::new();

    for (rang, portion) in entree.split(',').enumerate() {
        match analyser_champ(rang + 1, portion.trim()) {
            Ok(champ) => champs.push(champ),
            Err(erreur) => erreurs.push(erreur),
        }
    }

    if erreurs.is_empty() {
        Ok(champs)
    } else {
        Err(ErreurChamps { erreurs })
    }
}

fn analyser_champ(rang: usize, portion: &str) -> Result<Champ, ErreurChamp> {
    let erreur = |libelle: &str, nature| ErreurChamp {
        rang,
        libelle: libelle.to_string(),
        nature,
    };

    let mut parties = portion.split(':').map(str::trim);
    let nom = parties.next().unwrap_or_default();
    let type_brut = parties.next().unwrap_or_default();

    if nom.is_empty() || type_brut.is_empty() {
        return Err(erreur(portion, NatureErreur::FormeInvalide));
    }

    let Some(type_) = TypeChamp::analyser(type_brut) else {
        return Err(erreur(
            nom,
            NatureErreur::TypeInconnu {
                nom: type_brut.to_string(),
            },
        ));
    };

    let mut champ = Champ {
        nom: nom.to_string(),
        type_,
        unique: false,
        optionnel: false,
        index: false,
    };

    for modificateur in parties {
        match modificateur {
            "unique" => champ.unique = true,
            "optional" => champ.optionnel = true,
            "index" => champ.index = true,
            inconnu => {
                return Err(erreur(
                    nom,
                    NatureErreur::ModificateurInconnu {
                        nom: inconnu.to_string(),
                    },
                ));
            }
        }
    }

    Ok(champ)
}
```

- [ ] **Step 4: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-cli generate::champs`
Expected: PASS — 28 tests.

- [ ] **Step 5: Vérifier le lint et le format**

Run: `cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected: aucune sortie.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/src/generate/
git commit -m "feat(cli): analyse la chaîne de champs de generate crud

La grammaire est plate — deux séparateurs, aucune récursion — donc découpage
manuel plutôt qu'un combinateur, qui ne rendrait pas les messages meilleurs
pour une dépendance de plus.

Les fautes sont collectées sur tous les champs avant d'être rendues : une
chaîne portant trois erreurs se corrige en une fois.

Vérifications :
- cargo test -p rbs-cli generate::champs → 28 passed
- cargo clippy -p rbs-cli --all-targets -- -D warnings → propre"
```

---

### Task 4: Validation des noms et des modificateurs

**Files:**
- Modify: `crates/rbs-cli/src/generate/champs.rs`
- Modify: `crates/rbs-cli/src/generate/champs/erreur.rs` (rien à changer si les tâches précédentes sont faites ; vérifier seulement que `en_snake_case` et `suggestions_mot_cle` sont visibles depuis le parent)

**Interfaces:**
- Consumes: tout ce qui précède.
- Produces: aucune nouvelle signature publique — `analyser` refuse désormais les noms invalides, les mots-clés Rust, les noms imposés, les modificateurs en double et `unique` avec `index`.

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans le `mod tests` de `crates/rbs-cli/src/generate/champs.rs` :

```rust
    fn nature(entree: &str) -> NatureErreur {
        let mut erreur = analyser(entree).expect_err("la chaîne doit être refusée");
        assert_eq!(erreur.erreurs.len(), 1, "une seule faute attendue");
        erreur.erreurs.remove(0).nature
    }

    #[test]
    fn un_nom_hors_snake_case_est_refuse_avec_sa_recasse() {
        assert_eq!(
            nature("Title:string"),
            NatureErreur::PasEnSnakeCase {
                suggestion: Some("title".to_string())
            }
        );
        assert_eq!(
            nature("firstName:string"),
            NatureErreur::PasEnSnakeCase {
                suggestion: Some("first_name".to_string())
            }
        );
    }

    #[test]
    fn un_nom_accentue_est_refuse_sans_suggestion_trompeuse() {
        assert_eq!(
            nature("prénom:string"),
            NatureErreur::PasEnSnakeCase { suggestion: None }
        );
    }

    #[test]
    fn un_nom_a_souligne_final_ou_a_chiffre_initial_est_refuse() {
        assert!(matches!(
            nature("titre_:string"),
            NatureErreur::PasEnSnakeCase { .. }
        ));
        assert!(matches!(
            nature("1titre:string"),
            NatureErreur::PasEnSnakeCase { .. }
        ));
    }

    #[test]
    fn un_nom_a_chiffre_ou_souligne_interne_est_accepte() {
        let champs = champs("adresse_ligne_2:string");
        assert_eq!(champs[0].nom, "adresse_ligne_2");
    }

    #[test]
    fn un_mot_cle_rust_est_refuse_avant_la_compilation() {
        assert_eq!(
            nature("type:string"),
            NatureErreur::MotCleRust {
                suggestions: vec!["kind".to_string(), "type_".to_string()]
            }
        );
        assert!(matches!(
            nature("match:string"),
            NatureErreur::MotCleRust { .. }
        ));
        assert!(matches!(
            nature("async:bool"),
            NatureErreur::MotCleRust { .. }
        ));
        assert!(matches!(
            nature("box:string"),
            NatureErreur::MotCleRust { .. }
        ));
    }

    #[test]
    fn les_trois_colonnes_posees_par_rbs_sont_refusees() {
        for nom in ["id", "created_at", "updated_at"] {
            assert_eq!(
                nature(&format!("{nom}:string")),
                NatureErreur::NomReserve,
                "nom « {nom} »"
            );
        }
    }

    #[test]
    fn un_modificateur_en_double_est_refuse() {
        assert_eq!(
            nature("email:string:unique:unique"),
            NatureErreur::ModificateurEnDouble {
                nom: "unique".to_string()
            }
        );
    }

    #[test]
    fn unique_avec_index_est_refuse_comme_redondant() {
        assert_eq!(nature("slug:string:unique:index"), NatureErreur::IndexRedondant);
        assert_eq!(nature("slug:string:index:unique"), NatureErreur::IndexRedondant);
    }

    #[test]
    fn un_unique_sur_du_texte_passe_sans_commentaire() {
        assert!(champs("bio:text:unique")[0].unique);
        assert!(champs("actif:bool:index")[0].index);
    }

    #[test]
    fn toutes_les_fautes_de_la_chaine_remontent_dans_l_ordre() {
        let erreur = analyser("Title:string,type:text,prix:decimal")
            .expect_err("trois fautes attendues");

        assert_eq!(erreur.erreurs.len(), 3);
        assert_eq!(erreur.erreurs[0].rang, 1);
        assert!(matches!(
            erreur.erreurs[0].nature,
            NatureErreur::PasEnSnakeCase { .. }
        ));
        assert_eq!(erreur.erreurs[1].rang, 2);
        assert!(matches!(
            erreur.erreurs[1].nature,
            NatureErreur::MotCleRust { .. }
        ));
        assert_eq!(erreur.erreurs[2].rang, 3);
        assert!(matches!(
            erreur.erreurs[2].nature,
            NatureErreur::TypeInconnu { .. }
        ));
    }

    #[test]
    fn un_champ_portant_deux_fautes_ne_remonte_que_la_premiere() {
        let erreur = analyser("Type:decimal").expect_err("deux fautes, une seule remontée");

        assert_eq!(erreur.erreurs.len(), 1);
        assert!(matches!(
            erreur.erreurs[0].nature,
            NatureErreur::PasEnSnakeCase { .. }
        ));
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli generate::champs`
Expected: FAIL — `un_nom_hors_snake_case_est_refuse_avec_sa_recasse` panique sur `expect_err`, la chaîne étant acceptée.

- [ ] **Step 3: Écrire l'implémentation minimale**

Dans `crates/rbs-cli/src/generate/champs.rs`, compléter l'import du module d'erreurs :

```rust
pub(crate) use erreur::{ErreurChamp, ErreurChamps, NatureErreur};
use erreur::{en_snake_case, suggestions_mot_cle};
```

Ajouter les deux tables et le prédicat, après `analyser_champ` :

```rust
/// Mots-clés stricts et réservés des éditions 2015 à 2024. Un champ ainsi nommé
/// produirait une entité que rustc refuse, quarante secondes plus tard.
const MOTS_CLES_RUST: [&str; 49] = [
    "as", "async", "await", "become", "box", "break", "const", "continue", "crate", "do",
    "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
    "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv",
    "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true", "try",
    "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while",
];

/// Posées par rbs sur toute entité : les redéclarer donnerait deux fois la colonne.
const NOMS_POSES_PAR_RBS: [&str; 3] = ["id", "created_at", "updated_at"];

fn est_en_snake_case(nom: &str) -> bool {
    let Some(premier) = nom.chars().next() else {
        return false;
    };

    premier.is_ascii_lowercase()
        && !nom.ends_with('_')
        && nom
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
```

Insérer les quatre contrôles dans `analyser_champ`, entre le contrôle de forme et la résolution du type :

```rust
    if nom.is_empty() || type_brut.is_empty() {
        return Err(erreur(portion, NatureErreur::FormeInvalide));
    }

    if !est_en_snake_case(nom) {
        // Une recasse qui rendrait le nom inchangé, ou toujours invalide — un nom
        // accentué, par exemple — vaut mieux ne pas être proposée du tout.
        let recasse = en_snake_case(nom);
        let suggestion = (recasse != nom && est_en_snake_case(&recasse)).then_some(recasse);

        return Err(erreur(nom, NatureErreur::PasEnSnakeCase { suggestion }));
    }

    if MOTS_CLES_RUST.contains(&nom) {
        return Err(erreur(
            nom,
            NatureErreur::MotCleRust {
                suggestions: suggestions_mot_cle(nom),
            },
        ));
    }

    if NOMS_POSES_PAR_RBS.contains(&nom) {
        return Err(erreur(nom, NatureErreur::NomReserve));
    }
```

Remplacer la boucle des modificateurs par sa version détectant les doublons :

```rust
    for modificateur in parties {
        let drapeau = match modificateur {
            "unique" => &mut champ.unique,
            "optional" => &mut champ.optionnel,
            "index" => &mut champ.index,
            inconnu => {
                return Err(erreur(
                    nom,
                    NatureErreur::ModificateurInconnu {
                        nom: inconnu.to_string(),
                    },
                ));
            }
        };

        if *drapeau {
            return Err(erreur(
                nom,
                NatureErreur::ModificateurEnDouble {
                    nom: modificateur.to_string(),
                },
            ));
        }

        *drapeau = true;
    }

    if champ.unique && champ.index {
        return Err(erreur(nom, NatureErreur::IndexRedondant));
    }
```

- [ ] **Step 4: Lancer les tests pour les voir passer**

Run: `cargo test -p rbs-cli generate::champs`
Expected: PASS — 39 tests.

- [ ] **Step 5: Vérifier le lint et le format**

Run: `cargo clippy -p rbs-cli --all-targets -- -D warnings && cargo fmt --all --check`
Expected: aucune sortie.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/src/generate/
git commit -m "feat(cli): refuse les noms de champs que rustc ou rbs rejetteraient

Un champ nommé « type » compile en une entité que rustc refuse ; un champ
nommé « id » double une colonne que rbs pose déjà. Les deux se voyaient
après quarante secondes de compilation du projet généré, sous un message
sans rapport avec la ligne écrite.

unique avec index est refusé du même mouvement : un index unique est déjà un
index, et la migration en aurait posé deux sur une colonne.

Vérifications :
- cargo test -p rbs-cli generate::champs → 39 passed
- cargo clippy -p rbs-cli --all-targets -- -D warnings → propre"
```

---

### Task 5: Sérialisation pour les templates

**Files:**
- Modify: `crates/rbs-cli/src/generate/champs.rs`
- Modify: `crates/rbs-cli/Cargo.toml` (ajouter `serde_json.workspace = true` sous `[dev-dependencies]`)

**Interfaces:**
- Consumes: `Champ` et ses projections.
- Produces: `impl Serialize for Champ`, exposant les clés `nom`, `type`, `unique`, `optionnel`, `index`, `type_rust`, `methode_migration`, `attribut_column_type`.

- [ ] **Step 1: Écrire le test qui échoue**

Ajouter `serde_json.workspace = true` sous `[dev-dependencies]` de `crates/rbs-cli/Cargo.toml`, puis ajouter dans le `mod tests` de `champs.rs` :

```rust
    #[test]
    fn un_champ_se_serialise_avec_ses_projections() {
        let champ = &champs("bio:text:optional")[0];
        let json = serde_json::to_value(champ).expect("Champ est sérialisable");

        assert_eq!(json["nom"], "bio");
        assert_eq!(json["type"], "text");
        assert_eq!(json["unique"], false);
        assert_eq!(json["optionnel"], true);
        assert_eq!(json["index"], false);
        assert_eq!(json["type_rust"], "Option<String>");
        assert_eq!(json["methode_migration"], "text()");
        assert_eq!(json["attribut_column_type"], "Text");
    }

    #[test]
    fn un_type_sans_attribut_de_colonne_serialise_null() {
        let champ = &champs("titre:string")[0];
        let json = serde_json::to_value(champ).expect("Champ est sérialisable");

        assert_eq!(json["type_rust"], "String");
        assert!(json["attribut_column_type"].is_null());
    }
```

- [ ] **Step 2: Lancer les tests pour les voir échouer**

Run: `cargo test -p rbs-cli generate::champs`
Expected: FAIL à la compilation — `the trait bound Champ: Serialize is not satisfied`.

- [ ] **Step 3: Écrire l'implémentation minimale**

En tête de `crates/rbs-cli/src/generate/champs.rs` :

```rust
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
```

Et après `impl Champ` :

```rust
/// Sérialisé à la main : minijinja ne voit pas les méthodes Rust, or les templates
/// doivent lire `type_rust` comme elles lisent `nom`. Sans cela, chaque générateur
/// reconstruirait sa propre structure de vue.
impl Serialize for Champ {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut etat = serializer.serialize_struct("Champ", 8)?;
        etat.serialize_field("nom", &self.nom)?;
        etat.serialize_field("type", self.type_.nom())?;
        etat.serialize_field("unique", &self.unique)?;
        etat.serialize_field("optionnel", &self.optionnel)?;
        etat.serialize_field("index", &self.index)?;
        etat.serialize_field("type_rust", &self.type_rust())?;
        etat.serialize_field("methode_migration", self.type_.methode_migration())?;
        etat.serialize_field(
            "attribut_column_type",
            &self.type_.attribut_column_type(),
        )?;
        etat.end()
    }
}
```

- [ ] **Step 4: Lancer la suite complète**

Run: `cargo test -p rbs-cli generate::champs`
Expected: PASS — 41 tests.

Run: `cargo test --workspace`
Expected: PASS — aucune régression sur les tests existants.

- [ ] **Step 5: Vérifier le lint et le format**

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: aucune sortie.

- [ ] **Step 6: Commit**

```bash
git add crates/rbs-cli/src/generate/ crates/rbs-cli/Cargo.toml
git commit -m "feat(cli): expose les projections d'un champ aux templates

minijinja ne voit pas les méthodes Rust : sans sérialisation explicite, une
template ne peut pas lire le type Rust d'un champ, et chaque générateur du
lot devrait reconstruire sa propre structure de vue.

Vérifications :
- cargo test -p rbs-cli generate::champs → 41 passed
- cargo test --workspace → aucune régression
- cargo clippy --workspace --all-targets -- -D warnings → propre"
```

---

## Preuve du critère de la tâche

Le critère est : « Tests : chaque type et modificateur, plus les messages d'erreur de syntaxe. »

À la fin de la tâche 5, la ligne à consigner dans `TODO.md` est produite par :

```bash
cargo test -p rbs-cli generate::champs
```

Elle ne se coche que si la sortie couvre bien les trois obligations : les sept types (tâche 1), les trois modificateurs (tâches 3 et 4), et les huit natures d'erreur avec leur rendu (tâche 2).
