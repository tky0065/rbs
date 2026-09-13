# Langue des réponses HTTP — plan d'implémentation

> **Pour les agents :** sous-skill requis : `superpowers:subagent-driven-development`.
> Les étapes se suivent en cases `- [ ]`.

**But :** tout ce que voit le client HTTP d'un projet engendré — `title`/`detail` des
corps problem+json, messages fournis par les gabarits, descriptions communes du document
OpenAPI — suit la langue du projet, `fr` (défaut) ou `en`.

**Architecture :** `rbs-core` gagne `rbs_core::Lang` et un global de processus
(`lang::current()`/`lang::set()`), posé par `Config::load()` et résolu paresseusement pour
`bin/openapi.rs`. `Error::into_response` et `CommonResponses` le lisent. Côté CLI, `lang`
entre dans les contextes de rendu de `new`, `add`, `generate` ; les gabarits choisissent
leurs chaînes par `{% if lang == "en" %}…{% else %}…{% endif %}`.

**Spec :** `docs/superpowers/specs/2026-09-13-langue-des-reponses-http-design.md` (fait
autorité ; la lire avant toute tâche).

## Contraintes globales

- `CLAUDE.md` s'applique : frontière noyau/généré, commentaires = *pourquoi*, `///` d'une à
  trois lignes sur tout item public de `rbs-core` (`#![warn(missing_docs)]`).
- Commits : Conventional Commits, sujet français à l'impératif, sans majuscule ni point
  final ; corps = pourquoi technique + intertitre `Vérifications :` avec commandes et
  résultats réels. **Jamais** de `Co-Authored-By`, `Claude-Session`, ni mention d'assistant
  ou de session ; aucun identifiant de tâche, aucun renvoi à `IMPROVE.md`/`TODO.md`.
- Ne pas toucher : `CHANGELOG*.md`, `IMPROVE.md`, `TODO.md`, `ROADMAP.md`,
  `crates/rbs-cli/templates/agents/*`.
- minijinja : variables `{@ @}`, blocs `{% %}`, `UndefinedBehavior::Strict` (toute
  template qui lit `lang` exige `lang` dans **chaque** contexte qui la rend), pas de
  `trim_blocks` : `{%- x %}` mange le retour à la ligne qui *précède* la balise.
- Un rendu `fr` doit rester **octet pour octet** celui d'avant, hors `config/default.toml`.
- Les fragments (`add`) ne passent pas par rustfmt : la branche `en` doit être écrite
  sous la forme que rustfmt produirait (le projet engendré a `cargo fmt --check` en CI).
  Les fichiers de `generate crud` passent par rustfmt.
- Scratchpad propre :
  `/private/tmp/claude-501/-Users-yacoubakone-dev-rs/cdf33876-6963-4c94-bb34-990887cbdcf7/scratchpad/langue-reponses/`.
- Tests Docker : `--no-fail-fast` avant le `--`, une commande par suite, en arrière-plan,
  sortie redirigée dans le scratchpad.

## Table de référence (spec)

| Statut | `title` en | `title` fr | `detail` en | `detail` fr |
|---|---|---|---|---|
| 400 | Bad Request | Requête invalide | message de l'appelant | message de l'appelant |
| 401 | Unauthorized | Authentification requise | — | — |
| 403 | Forbidden | Accès interdit | — | — |
| 404 | Not Found | Introuvable | `{r} not found` | `{r} introuvable` |
| 409 | Conflict | Conflit | message de l'appelant | message de l'appelant |
| 422 | Validation failed | Validation échouée | — | — |
| 500 | Internal Server Error | Erreur interne | `an internal error occurred` | `une erreur interne est survenue` |

`Domain` : `title = code`, `detail = message`, inchangés. `Display` d'`Error` : inchangé.

Descriptions communes OpenAPI (`NAMED` + `UNIVERSAL`) :

| Clé | fr (actuel) | en |
|---|---|---|
| BadRequest | requête mal formée | malformed request |
| Unauthorized | authentification requise | authentication required |
| Forbidden | accès interdit | access forbidden |
| NotFound | ressource introuvable | resource not found |
| Conflict | conflit avec l'état courant de la ressource | conflict with the current state of the resource |
| TooManyRequests | trop de requêtes | too many requests |
| 422 | échec de validation, détaillé par champ | validation failed, detailed per field |
| 500 | erreur interne | internal error |

Chaînes des gabarits visibles du client :

| Gabarit | fr (inchangé) | en |
|---|---|---|
| `features/auth/repository/user.rs.jinja:29` `ADRESSE_PRISE` | `cette adresse est déjà inscrite` | `this address is already registered` |
| `features/webhooks/service.rs.jinja:75-77` | `un motif d'événement ne peut pas être vide` | `an event pattern cannot be empty` |
| `features/webhooks/target.rs.jinja:30` | `une URL de webhook doit être en http ou en https` | `a webhook URL must use http or https` |
| `features/webhooks/target.rs.jinja:31` | `une URL de webhook doit être en https` | `a webhook URL must use https` |
| `features/webhooks/target.rs.jinja:32` | `l'hôte de l'URL n'est pas une adresse publique` | `the URL host is not a public address` |
| `features/webhooks/repository.rs.jinja:68` | `NotFound("abonnement")` | `NotFound("subscription")` |
| `features/rate-limit/mod.rs.jinja:129` | `trop de requêtes : réessayez plus tard` | `too many requests: try again later` |
| `feature/controller.rs.jinja:377` | `NotFound("contenu")` | `NotFound("content")` |
| `feature/service.rs.jinja:189` | `NotFound("contenu")` | `NotFound("content")` |
| `feature/repository.rs.jinja:152` | `cette valeur est déjà prise` | `this value is already taken` |
| `feature/filter.rs.jinja:48` | `colonne de tri inconnue « {inconnue} » — {@ colonnes @}` | `unknown sort column '{inconnue}' — {@ colonnes @}` |

`NotFound("session")` (`auth/service/session.rs.jinja:184`) est le même mot dans les deux
langues : inchangé. `NotFound("{@ singular @}")` : inchangé (dérivé d'un identifiant).

---

### Tâche 1 : le noyau — `Lang`, `[server] lang`, `Error::parts`, descriptions communes

**Fichiers :**
- Créer : `crates/rbs-core/src/lang.rs`
- Modifier : `crates/rbs-core/src/lib.rs`, `crates/rbs-core/src/config.rs`,
  `crates/rbs-core/src/error.rs`, `crates/rbs-core/src/openapi.rs`
- Littéraux de `ServerConfig` à compléter (`lang: Lang::Fr`) : `extract.rs:239`,
  `health.rs:280`, `state.rs:133`

**Interfaces produites :**
- `pub enum rbs_core::Lang { Fr, En }` — `#[non_exhaustive]`, `Default = Fr`,
  `Deserialize` en minuscules, `Clone, Copy, Debug, PartialEq, Eq`.
- `pub fn rbs_core::lang::current() -> Lang`, `pub fn rbs_core::lang::set(lang: Lang)`.
- `ServerConfig::lang: Lang` (`#[serde(default)]`).

- [ ] **Étape 1 : tests rouges de `lang.rs`.** Créer le module avec `Lang`, `set`, `current`
  réduits à `todo!()` et une fonction privée `fn resolve() -> Lang { todo!() }`, puis les
  tests (Jail de figment, qui sérialise les tests qui l'emploient) :

```rust
#[cfg(test)]
#[allow(clippy::result_large_err)]
mod tests {
    use super::*;
    use figment::Jail;

    #[test]
    fn a_language_deserialises_from_its_lowercase_name() {
        assert_eq!(serde_json::from_str::<Lang>("\"fr\"").unwrap(), Lang::Fr);
        assert_eq!(serde_json::from_str::<Lang>("\"en\"").unwrap(), Lang::En);
        assert!(serde_json::from_str::<Lang>("\"EN\"").is_err());
    }

    #[test]
    fn without_configuration_the_language_resolves_to_french() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            assert_eq!(resolve(), Lang::Fr);
            Ok(())
        });
    }

    #[test]
    fn the_language_is_read_from_the_server_section() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", "[server]\nlang = \"en\"\n")?;
            assert_eq!(resolve(), Lang::En);
            Ok(())
        });
    }

    #[test]
    fn the_environment_overrides_the_language() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.set_env("RBS_SERVER__LANG", "en");
            assert_eq!(resolve(), Lang::En);
            Ok(())
        });
    }

    /// `bin/openapi.rs` n'a pas à échouer sur une valeur que `Config::load` refusera.
    #[test]
    fn an_unknown_language_resolves_to_french() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            jail.set_env("RBS_SERVER__LANG", "de");
            assert_eq!(resolve(), Lang::Fr);
            Ok(())
        });
    }
}
```

- [ ] **Étape 2 :** `cargo test -p rbs-core lang::tests` → échec (`todo!`/panique). Noter
  la sortie.
- [ ] **Étape 3 : implémentation.**

```rust
//! Langue dans laquelle le projet parle à ses clients HTTP.
//!
//! Un global de processus plutôt qu'un champ de l'état : `Error::into_response` ne voit
//! pas l'état, et le document OpenAPI se construit hors de toute requête.

use std::sync::atomic::{AtomicU8, Ordering};

use serde::Deserialize;

/// Langue des corps d'erreur et des descriptions communes du document OpenAPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Lang {
    /// Français, la langue d'un projet qui n'en déclare aucune.
    #[default]
    Fr,
    /// Anglais.
    En,
}

/// `0` : rien n'est encore posé ni résolu.
static COURANTE: AtomicU8 = AtomicU8::new(0);

impl Lang {
    fn code(self) -> u8 {
        match self {
            Self::Fr => 1,
            Self::En => 2,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Fr),
            2 => Some(Self::En),
            _ => None,
        }
    }
}

/// Pose la langue du processus ; [`Config::load`](crate::Config::load) l'appelle.
pub fn set(lang: Lang) {
    COURANTE.store(lang.code(), Ordering::Relaxed);
}

/// La langue du processus : celle que [`set`] a posée, à défaut `[server] lang`.
pub fn current() -> Lang {
    if let Some(lang) = Lang::from_code(COURANTE.load(Ordering::Relaxed)) {
        return lang;
    }

    let resolue = resolve();
    // Un `set` survenu entre-temps l'emporte : il vient d'une configuration complète.
    match COURANTE.compare_exchange(0, resolue.code(), Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => resolue,
        Err(posee) => Lang::from_code(posee).unwrap_or(resolue),
    }
}

#[derive(Deserialize)]
struct Server {
    #[serde(default)]
    lang: Lang,
}

/// `[server] lang` par la cascade de configuration, le français sur toute erreur.
///
/// C'est le chemin de `bin/openapi.rs`, qui rend le document sans charger la
/// configuration complète : un `database.url` absent n'y est pas une faute.
fn resolve() -> Lang {
    crate::config::section::<Server>("server")
        .map(|server| server.lang)
        .unwrap_or_default()
}
```

  `lib.rs` : `/// Langue des réponses HTTP du projet.` + `pub mod lang;` (ordre
  alphabétique des modules) et `pub use lang::Lang;`.
- [ ] **Étape 4 :** `cargo test -p rbs-core lang::tests` → 5 passés.
- [ ] **Étape 5 : test rouge dans `config.rs`.** Ajouter à `ServerConfig` :

```rust
    /// Langue des réponses HTTP : `title` et `detail` des erreurs, descriptions communes
    /// du document OpenAPI.
    #[serde(default)]
    pub lang: crate::lang::Lang,
```

  compléter les trois littéraux (`extract.rs`, `health.rs`, `state.rs`) par
  `lang: crate::lang::Lang::Fr,`, puis ajouter aux tests de `config.rs` :

```rust
    #[test]
    fn without_a_language_the_server_speaks_french() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            test_secret(jail);
            jail.create_dir("config")?;
            jail.create_file("config/default.toml", DEFAULT_TOML)?;

            let config = Config::load().expect("la configuration doit se charger");

            assert_eq!(config.server.lang, crate::lang::Lang::Fr);
            Ok(())
        });
    }

    /// Le serveur, le worker et les tests engendrés passent tous par `load()` : c'est lui
    /// qui doit poser la langue que `Error::into_response` lira.
    #[test]
    fn load_sets_the_language_of_the_process() {
        Jail::expect_with(|jail| {
            jail.clear_env();
            test_secret(jail);
            jail.create_dir("config")?;
            jail.create_file(
                "config/default.toml",
                &format!("{DEFAULT_TOML}\n[server]\nlang = \"en\"\n"),
            )?;
            // …
            Ok(())
        });
    }
```

  Attention : `DEFAULT_TOML` porte déjà une table `[server]` — une seconde table
  `[server]` est un TOML invalide. Écrire le fichier en entier :
  `"[server]\nport = 8080\nlang = \"en\"\n\n[database]\nurl = \"postgres://localhost/app\"\n"`.
  Corps du test : `Config::load()` → `assert_eq!(config.server.lang, Lang::En)` et
  `assert_eq!(crate::lang::current(), Lang::En)` ; puis réécrire le fichier sans `lang`,
  recharger, et `assert_eq!(crate::lang::current(), Lang::Fr)` — le test rend le global
  dans l'état où il l'a trouvé et prouve les deux sens.
- [ ] **Étape 6 :** `cargo test -p rbs-core config::tests` → le second test échoue sur
  `current()` (rien ne pose encore la langue).
- [ ] **Étape 7 :** dans `Config::load`, juste avant `Ok(config)` :

```rust
        // Posée au terme d'un chargement réussi seulement : un chargement refusé ne laisse
        // aucune trace dans le processus.
        crate::lang::set(config.server.lang);
```

- [ ] **Étape 8 :** `cargo test -p rbs-core config::tests` → tout passe.
- [ ] **Étape 9 : tests rouges d'`Error::parts`.** Dans `error.rs`, déclarer

```rust
/// Statut, `title`, `detail` et détail par champ d'une réponse d'erreur.
type Parts = (StatusCode, &'static str, Option<String>, Option<BTreeMap<String, Vec<String>>>);

impl Error {
    /// La réponse d'erreur dans `lang`, sans effet de bord : ni journal, ni global lu.
    fn parts(&self, lang: Lang) -> Parts {
        todo!()
    }
}
```

  et ajouter deux tests qui parcourent la table de référence, un par langue, variante par
  variante (`NotFound("utilisateur")`, `BadRequest("x")`, `validation_errors()`,
  `Unauthorized`, `Forbidden`, `Conflict("x")`, `Domain{…}`, `Database(DbErr::Custom)`,
  `Internal(anyhow!)`), en vérifiant statut, `title` et `detail` :

```rust
    #[test]
    fn in_english_the_parts_follow_the_table() {
        let (status, title, detail, _) = Error::NotFound("user").parts(Lang::En);
        assert_eq!((status, title, detail.as_deref()), (StatusCode::NOT_FOUND, "Not Found", Some("user not found")));
        // … une ligne d'assertion par variante, valeurs de la table « en »
    }

    #[test]
    fn in_french_the_parts_follow_the_table() {
        let (status, title, detail, _) = Error::NotFound("utilisateur").parts(Lang::Fr);
        assert_eq!((status, title, detail.as_deref()), (StatusCode::NOT_FOUND, "Introuvable", Some("utilisateur introuvable")));
        // … une ligne d'assertion par variante, valeurs de la table « fr »
    }
```

  Écrire **toutes** les lignes (9 variantes × 2 langues), pas seulement `NotFound`.
- [ ] **Étape 10 :** `cargo test -p rbs-core error::tests` → les deux tests paniquent.
- [ ] **Étape 11 : implémentation.** `parts` porte le `match` actuel d'`into_response`, les
  littéraux choisis par langue (`match lang { Lang::En => …, Lang::Fr => … }` — le `match`
  est exhaustif dans la crate malgré `#[non_exhaustive]`). `into_response` devient :

```rust
    fn into_response(self) -> Response {
        let request_id = request_id::current();

        if matches!(self, Error::Database(_) | Error::Internal(_)) {
            // La source part au journal et nulle part ailleurs : le client n'obtient que le
            // request_id, qui suffit à retrouver cette ligne.
            tracing::error!(
                request_id = request_id.as_deref().unwrap_or("-"),
                error = %self,
                "erreur interne"
            );
        }

        let (status, title, detail, errors) = self.parts(lang::current());
        // … corps et en-tête inchangés
    }
```

  Les tests existants passant par `into_response` ne doivent plus rien affirmer de
  dépendant de la langue (le global est partagé entre tests parallèles) : retirer les
  assertions sur `title` `"Not Found"`, `"Validation failed"`, `"Internal Server Error"`,
  `"Bad Request"` et sur `detail` `"utilisateur introuvable"` — la table les prouve
  désormais par `parts`. Garder statut, content-type, `request_id`, non-divulgation, et
  les `detail` fournis par l'appelant (Conflict, BadRequest, Domain), qui ne dépendent pas
  de la langue.
- [ ] **Étape 12 :** `cargo test -p rbs-core error::tests` → tout passe.
- [ ] **Étape 13 : descriptions communes, test rouge.** Dans `openapi.rs`, remplacer les
  constantes `NAMED`/`UNIVERSAL` par `fn named(lang: Lang) -> [(&'static str, &'static str); 6]`
  et `fn universal(lang: Lang) -> [(&'static str, &'static str); 2]` (valeurs de la table),
  extraire le corps de `Modify::modify` dans `fn declare(openapi: &mut utoipa::openapi::OpenApi, lang: Lang)`
  (`complete` prend `lang`), et `modify` devient `declare(openapi, crate::lang::current())`.
  D'abord, avec `declare` en `todo!()`, ajouter :

```rust
    /// Le même handler, sans le modificateur : `declare` s'y applique à la main, dans la
    /// langue voulue, sans passer par le global que les tests parallèles partagent.
    #[derive(OpenApi)]
    #[openapi(paths(list_all))]
    struct Bare;

    fn declared(lang: Lang) -> Value {
        let mut openapi = Bare::openapi();
        declare(&mut openapi, lang);
        serde_json::to_value(openapi).expect("document sérialisable")
    }

    #[test]
    fn in_english_the_common_descriptions_are_english() {
        let doc = declared(Lang::En);
        assert_eq!(doc["components"]["responses"]["NotFound"]["description"], "resource not found");
        assert_eq!(doc["components"]["responses"]["TooManyRequests"]["description"], "too many requests");
        assert_eq!(doc["paths"]["/things"]["get"]["responses"]["422"]["description"], "validation failed, detailed per field");
        assert_eq!(doc["paths"]["/things"]["get"]["responses"]["500"]["description"], "internal error");
    }

    #[test]
    fn in_french_the_common_descriptions_are_unchanged() {
        let doc = declared(Lang::Fr);
        assert_eq!(doc["components"]["responses"]["NotFound"]["description"], "ressource introuvable");
        assert_eq!(doc["paths"]["/things"]["get"]["responses"]["422"]["description"], "échec de validation, détaillé par champ");
    }
```

- [ ] **Étape 14 :** `cargo test -p rbs-core openapi::tests` → échec ; puis implémenter →
  `cargo test -p rbs-core` entier vert (avec et sans `--features auth` :
  `cargo test -p rbs-core --all-features`).
- [ ] **Étape 15 :** `cargo fmt --all --check`, `cargo clippy -p rbs-core --all-targets --all-features -- -D warnings`,
  et si l'outil est présent `cargo semver-checks check-release -p rbs-core` (sinon le noter).
- [ ] **Étape 16 : commit** `feat(core): rend les réponses d'erreur dans la langue du projet`
  — corps : pourquoi (global de processus, `load()` pose, résolution paresseuse pour
  `bin/openapi.rs`, `parts` pure testée sans le global), puis `Vérifications :`.

---

### Tâche 2 : le CLI et les gabarits

**Fichiers :**
- Modifier : `crates/rbs-cli/src/cli.rs:47` (texte de `--lang`), `crates/rbs-cli/src/lang.rs`
  (docs du module et de l'enum), `crates/rbs-cli/templates/project/config/default.toml.jinja`,
  `crates/rbs-cli/src/add/mod.rs:316` (contexte), `crates/rbs-cli/src/generate/feature.rs`
  (champ `lang`, `speaking`, `Serialize`), `crates/rbs-cli/src/generate/command.rs:308`,
  `crates/rbs-cli/src/generate/filter.rs:39` (contexte), les 9 gabarits de la table,
  `crates/rbs-cli/src/templates.rs` (contexte de test `feature_context`, tests neufs),
  `crates/rbs-cli/src/add/installation.rs:412` si son contexte rend un gabarit qui lit
  `lang`.

**Interfaces :**
- Consomme : rien du noyau (les deux crates sont indépendantes).
- Produit : `Feature::lang: crate::lang::Lang` (défaut `Fr` dans `fresh`),
  `Feature::speaking(self, lang) -> Self`, champ sérialisé `lang` = `"fr"|"en"`.

- [ ] **Étape 1 : vérifier l'inventaire.** `grep -rn 'message = "' crates/rbs-cli/templates --include=*.jinja`
  (aucun DTO engendré ne doit poser de message de validation) et
  `grep -rn 'Error::\(Conflict\|BadRequest\|NotFound\|Domain\)' crates/rbs-cli/templates | grep -v agents` :
  toute chaîne-mot hors table ci-dessus → la signaler au rapport, et la traiter si elle
  atteint le client.
- [ ] **Étape 2 : tests rouges.**
  - `templates.rs` : `feature_context(installees)` devient `feature_context_in(installees, lang)`
    (l'ancien appelle le neuf avec `"fr"`) ; ajouter

```rust
    /// Les messages que les fragments rendent au client, dans leur version française.
    const MESSAGES_FRANCAIS: [&str; 7] = [
        "cette adresse est déjà inscrite",
        "un motif d'événement ne peut pas être vide",
        "une URL de webhook doit être en http ou en https",
        "une URL de webhook doit être en https",
        "l'hôte de l'URL n'est pas une adresse publique",
        "Error::NotFound(\"abonnement\")",
        "trop de requêtes : réessayez plus tard",
    ];

    #[test]
    fn in_english_no_fragment_hands_a_french_message_to_the_client() {
        for template in feature_templates() {
            let rendu = render_fragment(&template, feature_context_in(&TOUTES, "en"));
            for message in MESSAGES_FRANCAIS {
                assert!(!rendu.contains(message), "{} rend « {message} » en anglais", template.display());
            }
        }
    }

    /// Sans ce pendant, la liste pourrait ne plus rien désigner et le test anglais passer
    /// à vide.
    #[test]
    fn in_french_each_listed_message_is_still_rendered() {
        let rendus: String = feature_templates()
            .iter()
            .map(|template| render_fragment(template, feature_context_in(&TOUTES, "fr")))
            .collect();
        for message in MESSAGES_FRANCAIS {
            assert!(rendus.contains(message), "« {message} » n'est plus rendu par aucun fragment");
        }
    }
```

    `render_fragment` et `TOUTES` : reprendre l'aide et la liste de features que le test
    existant `each_feature_template_renders…` (ou équivalent, voir autour de
    `templates.rs:662`) emploie déjà ; ne pas en inventer d'autres.
  - `templates.rs`, squelette : un test qui rend `config/default.toml.jinja` avec
    `lang => "en"` puis le parse (`toml::from_str::<toml::Value>` ou `toml_edit`, selon
    ce que la crate emploie déjà) et affirme `server.lang == "en"`.
  - `generate/command.rs` (tests) : sur une feature `title:string:unique` `.uploading()`
    rendue par `render(&feature, true, true, Some("demo_api"))`, affirmer qu'en
    `.speaking(Lang::En)` aucun fichier ne contient `"contenu"`, `cette valeur est déjà prise`
    ni `colonne de tri inconnue`, et contient `"content"`, `this value is already taken`,
    `unknown sort column` ; et qu'en `Fr` les trois chaînes françaises y sont.
  - `generate/feature.rs` : le champ sérialisé `lang` vaut `"fr"` par défaut, `"en"` après
    `speaking(Lang::En)`.
- [ ] **Étape 3 :** `cargo test -p rbs-cli --lib` → les tests neufs échouent (et d'autres
  tombent en `Strict` dès qu'un gabarit lit `lang` sans contexte : c'est attendu).
- [ ] **Étape 4 : plomberie.**
  - `cli.rs` : `/// Langue du projet : `AGENTS.md` et réponses HTTP. À défaut, celle de l'environnement.`
  - `lang.rs` (CLI) : docs du module et de l'enum disent « langue du projet : son
    `AGENTS.md` et ses réponses HTTP ».
  - `default.toml.jinja` : `lang = "{@ lang @}"` sous `shutdown_timeout_secs = 30`, sans
    commentaire (la spec dit « la ligne `lang = "fr"` »).
  - `add/mod.rs` : `lang => metadonnees.lang.name(),` dans le contexte (lire le champ
    avant tout `move` de `metadonnees`).
  - `Feature` : champ + `speaking` + `state.serialize_field("lang", self.lang.name())?;`
    (compteur `serialize_struct` à 15) ; `command.rs` : `.speaking(metadonnees.lang)` à la
    construction.
  - `generate/filter.rs` : `lang => feature.lang.name(),`.
- [ ] **Étape 5 : gabarits.** Une ligne → `{% if lang == "en" %}"…"{% else %}"…"{% endif %}`
  en place du littéral. Seul cas multi-ligne, `features/webhooks/service.rs.jinja:74-78`,
  parce que la version anglaise tient sur une ligne et que rustfmt l'y ramènerait :

```
        if motif.trim().is_empty() {
{%- if lang == "en" %}
            return Err(Error::BadRequest("an event pattern cannot be empty".to_string()));
{%- else %}
            return Err(Error::BadRequest(
                "un motif d'événement ne peut pas être vide".to_string(),
            ));
{%- endif %}
        }
```

  Vérifier de même, pour chaque fragment, que la ligne anglaise reste sous 100 colonnes
  (`ADRESSE_PRISE`, les trois bras de `Refusal`, `NotFound("subscription")`, le message du
  429) et qu'elle garde la forme de sa version française.
- [ ] **Étape 6 :** `cargo test -p rbs-cli --lib` → tout vert ; `cargo clippy -p rbs-cli --all-targets -- -D warnings` ;
  `cargo fmt --all --check`.
- [ ] **Étape 7 : identité du rendu `fr`, par diff entre deux générations.** Construire
  le CLI d'avant dans un worktree jetable du scratchpad
  (`git worktree add <scratch>/base improve/lot-p2-doc-langue-exemple`, puis
  `cargo build -p rbs-cli` avec `CARGO_TARGET_DIR=<scratch>/target-base`), et générer avec
  les deux binaires, chacun dans son répertoire :

```bash
rbs new tout --yes --lang fr --database-url 'postgres://rbs:rbs@localhost:5432/tout' \
  --with audit,auth,ci,cors,docker,jobs,mail,observability,rate-limit,redis,scheduler,storage,webhooks
cd tout && git add -A && git commit -q -m neuf
rbs generate crud items --fields 'title:string:unique,size:int' --with-upload --force
```

  `diff -r -x .git -x .env -x target avant/tout apres/tout` ne doit montrer que la ligne
  `lang = "fr"` de `config/default.toml` et l'horodatage des noms de migration de
  `generate` (comparer ces deux fichiers-là par contenu). Consigner la sortie dans le
  commit. Retirer le worktree jetable (`git worktree remove`).
- [ ] **Étape 8 : commits.** `feat(cli): inscrit la langue du projet dans [server] et dans les contextes de rendu`
  puis `feat(templates): rend dans la langue du projet les messages destinés au client`
  (ou un seul commit si la séparation casse la compilation d'un des deux).

---

### Tâche 3 : les exemples

**Fichiers :** `examples/{hello-crud,blog-auth,file-drop,newsletter-queue}/config/default.toml`.

- [ ] **Étape 1 :** `cargo test -p rbs-cli --test integration_examples` → rouge sur les
  quatre `config/default.toml` (preuve que l'oracle voit le changement).
- [ ] **Étape 2 :** régénérer chaque exemple avec le CLI d'avant et celui d'après, par les
  commandes exactes d'`examples/README.md` (répertoires du scratchpad, jamais dans
  `examples/`), `diff -r` des deux générations ; reporter **ce diff seul** sur les
  fichiers versionnés — attendu : `lang = "fr"` après `shutdown_timeout_secs = 30`.
- [ ] **Étape 3 :** `cargo test -p rbs-cli --test integration_examples` → vert.
- [ ] **Étape 4 :** dans chaque exemple : `cargo check --all-targets` puis
  `cargo clippy --all-targets -- -D warnings` (le noyau a changé sous eux).
- [ ] **Étape 5 : commit** `chore(examples): inscrit lang = "fr" dans la configuration des quatre exemples`.

---

### Tâche 4 : la preuve de bout en bout

**Fichier :** créer `crates/rbs-cli/tests/integration_lang.rs`.

Un seul test `#[ignore]` : `an_english_project_answers_its_clients_in_english`.

- [ ] **Étape 1 : écrire le test.** Modèles à reprendre (copier, ces aides sont privées
  à leur fichier) : `project_with_webhooks_on`, `cargo_test_brut`, `rbs` de
  `integration_webhooks.rs:88-190` ; `Serveur`, `free_port`, `wait_for_listening`,
  `request`, `decode` d'`integration_auth.rs:1102-1257`. Déroulé :
  1. `common::start_postgres()`, `TempDir`, `rbs new demo-api --lang en --database-url <url> --core-path <noyau> --yes`,
     `common::commiter`, `rbs add webhooks` (entraîne jobs, auth, rate-limit, mail),
     `rbs generate crud articles --fields 'title:string:unique,body:text,published:bool' --force`.
  2. `let _cible = common::verrou(&common::cible());` avant le premier cargo ;
     `config/default.toml` contient `lang = "en"`.
  3. `rbs migrate up` ; `cargo fmt --all --check` dans le projet (les branches `en` des
     fragments ne passent pas par rustfmt).
  4. `cargo test --workspace -- --include-ignored` : succès, et les lignes
     `articles::tests::a_replayed_unique_value_returns_409 ... ok`,
     `articles::tests::an_unknown_id_returns_404 ... ok`,
     `modules::webhooks::tests::an_admin_subscribing_a_private_url_gets_400 ... ok`
     présentes (un filtre vide sort aussi en 0).
  5. `cargo run --quiet --bin openapi` : `components.responses.NotFound.description ==
     "resource not found"` et le 500 d'une opération `/articles…` vaut `"internal error"`
     — c'est la résolution paresseuse, sans `Config::load()`.
  6. `cargo build`, `Serveur::lancer(&racine, "demo-api", "info")`, puis :
     - `POST /auth/register` `{email, password}` → 201 ;
     - le même → 409 : `title == "Conflict"`, `detail == "this address is already registered"` ;
     - `POST /auth/register` avec `"email":"pas-un-email"` → 422 : `title == "Validation failed"` ;
     - `POST /auth/login` → 200, jeton d'accès ;
     - `GET /articles/<uuid v4 aléatoire>` avec le jeton → 404 : `title == "Not Found"`,
       `detail == "article not found"` ;
     - `POST /articles` deux fois le même `title` avec le jeton → 201 puis 409 :
       `detail == "this value is already taken"` ;
     - `POST /auth/login` avec un mauvais mot de passe, jusqu'à 10 fois, jusqu'au premier
       429 (limite 5/60 s) : `title == "too_many_requests"`,
       `detail == "too many requests: try again later"`.
     - Pour chacun des cinq corps d'erreur : `assert!(corps.to_string().is_ascii())` et
       aucun de `introuvable`, `déjà`, `requête`, `réessayez`, `échouée`, `interne`.
- [ ] **Étape 2 :** `cargo test -p rbs-cli --test integration_lang --no-fail-fast -- --include-ignored`
  en arrière-plan, sortie dans `<scratch>/integration_lang.log`, en notant nombre de tests
  et durée. Le rouge n'est pas rejoué : il coûterait une compilation complète de plus contre
  le CLI d'avant. Le dire tel quel au rapport ; les assertions exactes sur `detail` sont ce
  qui empêche ce test de passer à vide.
- [ ] **Étape 3 : commit** `test(cli): éprouve qu'un projet --lang en répond en anglais de bout en bout`.

---

### Tâche 5 : la documentation (EN et FR dans le même commit)

**Fichiers :** `docs/docs/guides/errors.md`, `docs/docs/guides/configuration.md`,
`docs/docs/cli/new.md`, `docs/docs/tutorials/auth.md`, `docs/docs/tutorials/storage.md`,
et leurs pendants sous `docs/i18n/fr/docusaurus-plugin-content-docs/current/`.

- [ ] `errors.md` : colonne `title` à deux langues (table de référence) ; une section « la
  langue du corps » : `[server] lang` (`fr` par défaut, `en`), ce qui suit la langue
  (`title`, `detail` des variantes du noyau, messages des gabarits, descriptions communes
  OpenAPI), ce qui n'en dépend pas (`Domain` : `code` ; les codes de `validator` ; les
  messages passés par l'appelant, que *vous* écrivez), ce qui reste en français (journaux,
  commentaires, courriels de `mail`/`auth`), et comment basculer un projet existant
  (`lang = "en"` sous `[server]`, les messages déjà engendrés restant à traduire à la
  main). La phrase du 500 cite les deux langues.
- [ ] `configuration.md` : ligne `server.lang` | `RBS_SERVER__LANG` | `fr` dans la table.
- [ ] `cli/new.md` : ligne `--lang` du bloc d'aide, recopiée de
  `cargo run -p rbs-cli --bin rbs -- new -h` (verbatim) ; ligne du tableau des options ;
  un paragraphe sous la section `AGENTS.md` (ne pas renommer le titre : son ancre est
  peut-être liée) disant que la même valeur s'inscrit sous `[server] lang` et décide de la
  langue des réponses, lien vers `errors.md`.
- [ ] Tutoriels : corps capturés sur un projet `fr` dont le `title` change —
  `auth.md` (401 → `Authentification requise`, 403 → `Accès interdit`), `storage.md`
  (422 → `Validation échouée`) ; recalculer chaque `content-length` en octets UTF-8
  (`printf '%s' '<corps>' | wc -c`).
- [ ] `cargo test -p rbs-cli --test integration_docs` (et la variante base si elle ne
  demande qu'un PostgreSQL : `-- --include-ignored`, en arrière-plan) → vert ; lancer le
  contrôle de parité s'il existe (`docs/` : voir `package.json`).
- [ ] Commit `docs: décrit la langue des réponses HTTP et son réglage [server] lang`.

---

### Tâche 6 : vérification finale

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace` (sortie dans le scratchpad)
- [ ] `cargo semver-checks check-release -p rbs-core` si disponible
- [ ] relire `git log --oneline improve/lot-p2-doc-langue-exemple..HEAD` : sujets et corps
  conformes, aucune attribution.
