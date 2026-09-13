# Langue des réponses HTTP engendrées

**Date** : 2026-09-13 · **Backlog** : `IMPROVE.md`, tâche 23 (P2, Medium)

## Problème

Une réponse d'erreur d'un projet engendré est bilingue : `rbs-core` rend un `title`
anglais (`"Not Found"`, `"Validation failed"`) et un `detail` français
(`"{ressource} introuvable"`, `"une erreur interne est survenue"`,
`crates/rbs-core/src/error.rs:104-144`), et les gabarits ajoutent leurs propres messages
français (`ADRESSE_PRISE`, `"trop de requêtes : réessayez plus tard"`, `NotFound("abonnement")`…).
`rbs new --lang` ne règle que la langue d'`AGENTS.md` (`cli.rs:47`). Un client d'un projet
`--lang en` reçoit donc des `detail` en français.

## Décision (validée le 2026-09-13)

**Tout ce que voit le client HTTP suit la langue du projet**, `fr` ou `en` : le `title` et
le `detail` des réponses `application/problem+json`, les messages que les gabarits
fournissent à `Error`, et les descriptions que le document OpenAPI expose. Un projet sans
réglage garde le comportement actuel (français).

Hors périmètre, et dit tel quel dans la documentation : les journaux (destinés au
développeur, restent en français), les commentaires du code engendré, et les courriels de
`mail`/`auth` (objets et gabarits HTML sous `templates/mail/`, que l'utilisateur édite) —
ces derniers sont un candidat backlog à part.

## Conception

### `rbs-core`

- Nouveau type public `rbs_core::Lang` (`Fr`, `En`), `#[non_exhaustive]`, `Default = Fr`,
  `Deserialize` en minuscules (`"fr"`, `"en"`).
- `ServerConfig` gagne `lang: Lang`, `#[serde(default)]`. `ServerConfig` étant déjà
  `#[non_exhaustive]`, l'ajout est additif (mineure, pas de rupture semver).
- Module `rbs_core::lang` : `pub fn current() -> Lang` et `pub fn set(Lang)`, sur un
  global de processus (`AtomicU8`, 0 = non résolu).
  - `Config::load()` appelle `lang::set(config.server.lang)` après extraction : le serveur,
    le worker et les tests engendrés passent tous par `load()`.
  - `current()`, quand rien n'a été posé, résout **une fois** `[server] lang` par la
    cascade de `config::section` (fichiers + `RBS_SERVER__LANG`), `Fr` sur toute erreur.
    C'est ce qui couvre `src/bin/openapi.rs`, qui rend le document sans charger la
    configuration complète (pas de `.env` requis en CI).
- `Error::into_response` lit `lang::current()` et délègue à une fonction pure
  `fn parts(&self, lang: Lang) -> (StatusCode, &str, Option<String>, Option<…>)`, testée
  dans les deux langues sans toucher au global.

| Statut | `title` en | `title` fr | `detail` en | `detail` fr |
|---|---|---|---|---|
| 400 | Bad Request | Requête invalide | message de l'appelant | message de l'appelant |
| 401 | Unauthorized | Authentification requise | — | — |
| 403 | Forbidden | Accès interdit | — | — |
| 404 | Not Found | Introuvable | `{r} not found` | `{r} introuvable` |
| 409 | Conflict | Conflit | message de l'appelant | message de l'appelant |
| 422 | Validation failed | Validation échouée | — | — |
| 500 | Internal Server Error | Erreur interne | `an internal error occurred` | `une erreur interne est survenue` |

`Error::Domain` garde `title = code` (identifiant machine, neutre) ; son `message` vient
de l'appelant, donc du gabarit. Les messages de validation restent les codes de
`validator` (`"email"`, `"length"`), neutres — aucun DTO engendré ne pose de `message`.

- `openapi::CommonResponses` : les six descriptions de `NAMED` existent dans les deux
  langues, choisies par `lang::current()` au moment où le document est construit.
- L'impl `Display` d'`Error` (`#[error("{0} introuvable")]`) sert aux journaux : inchangée.

### CLI et gabarits

- `--lang` : « Langue du projet : `AGENTS.md` et réponses HTTP. » La valeur continue de
  s'inscrire dans `[package.metadata.rbs] lang`.
- `rbs new` écrit `lang = "<fr|en>"` dans la table `[server]` de `config/default.toml`.
- `lang` entre dans le contexte de rendu de `new`, `add` et `generate` (lu dans les
  métadonnées pour `add`/`generate`). Chaque chaîne qui atteint le client — messages de
  `Error::Conflict`/`BadRequest`/`Domain`, noms passés à `NotFound` qui sont des mots
  (`"session"`, `"contenu"`, `"abonnement"`…) — se rend par
  `{% if lang == "en" %}…{% else %}…{% endif %}`. Les noms dérivés d'identifiants
  (`NotFound("{@ singular @}")`) restent tels quels.
- Les tests engendrés qui figent une chaîne la figent dans la langue du projet.
- Un projet existant (clé absente) reste en français ; `rbs upgrade` ne réécrit rien. La
  note de version dit comment basculer (`[server] lang = "en"` + les messages déjà
  engendrés restent à traduire à la main).
- `templates/agents/*.md.jinja` n'est pas touché par cette tâche : la ligne `--lang` y est
  réécrite par la tâche 26, dans le même lot.

### Documentation

`guides/errors.md`, `guides/configuration.md` (`[server] lang`), `cli/new.md` (`--lang`),
FR et EN dans le même commit ; les transcriptions `rbs:transcript` touchées suivent.
Le CHANGELOG est écrit par l'orchestrateur du lot, pas par cette tâche.

## Preuves attendues

1. `cargo test -p rbs-core` : `parts` dans les deux langues, `section` absente → `Fr`,
   `load()` pose la langue.
2. `cargo test -p rbs-cli --lib` : un rendu `en` de chaque fragment ne contient plus aucune
   des chaînes françaises visibles du client (liste figée dans le test), un rendu `fr` est
   identique à aujourd'hui hors `config/default.toml`.
3. Un test d'intégration lent engendre un projet `--lang en` portant `auth`, `rate-limit`,
   `webhooks` et un CRUD, lance ses tests (`--include-ignored`), et vérifie qu'une 404, une
   409, une 422 et une 429 rendent un corps entièrement anglais.
4. `integration_examples` vert (les quatre exemples, `--lang fr`, gagnent la ligne
   `lang = "fr"`), `integration_docs` vert, clippy/fmt verts.
