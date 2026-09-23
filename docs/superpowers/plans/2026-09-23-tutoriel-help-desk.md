# Tutoriel « gestionnaire de tickets » — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** un tutoriel de bout en bout, fr et en, qui construit une application de tickets avec son frontend d'administration, adossé à un septième exemple versionné, `examples/help-desk`.

**Architecture :** l'exemple est engendré par le CLI (`new`, `add frontend-admin`, deux `generate crud`), puis retouché à la main en un seul point — l'auteur lu dans le jeton — côté Rust et côté Vue. Le harnais de non-dérive apprend à rejouer plusieurs CRUD. La page cite l'exemple par régions et rejoue les sorties du CLI par transcriptions.

**Tech Stack :** Rust (axum, SeaORM, utoipa), `rbs-core`, Vue 3 + TypeScript (vue-tsc, Vite), Docusaurus, `assert_cmd`.

**Spec :** `docs/superpowers/specs/2026-09-23-tutoriel-help-desk-design.md`

## Global Constraints

- Branche `docs/tutoriel-help-desk` ; ne jamais committer sur `main` ; ne pas créer d'autre branche.
- Commits Conventional Commits, sujet en français à l'impératif, sans majuscule ni point final ; corps avec un intertitre `Vérifications :` portant les commandes lancées et leur résultat réel.
- **Jamais** de ligne `Co-Authored-By`, `Claude-Session`, ni mention d'un assistant, d'une session, d'un identifiant de tâche, de `TODO.md`, `ROADMAP.md`, d'un plan ou d'un lot dans un message de commit.
- Aucun fichier sous `.github/workflows/` n'est modifié.
- Toute page de documentation modifiée en anglais l'est en français **dans le même commit**.
- Un commentaire de code explique le *pourquoi*, jamais le *quoi*.
- Les commandes de l'exemple se lancent avec `--lang fr`, et la base `postgres://rbs:rbs@localhost:5432/help_desk`.
- Champs exacts : tickets = `sujet:string,detail:text,statut:enum(ouvert,en_cours,resolu,ferme),priorite:enum(basse,normale,haute),auteur:references:users` ; commentaires = `corps:text,ticket:references:tickets:cascade,auteur:references:users`.
- Tests Docker du CLI : toujours `--no-fail-fast`, sortie redirigée vers le scratchpad (une sortie longue en arrière-plan est rognée).
- Shell zsh : `ls` est un alias d'eza — employer `command ls`.
- `$SCRATCH` désigne le scratchpad de la session (`export SCRATCH=<scratchpad>` avant la première commande) ; deux agents en parallèle n'y emploient jamais le même nom de fichier.

## Review Focus

1. **Un `auteur_id` étranger dans le corps** — un appelant qui envoie l'identifiant d'un *autre compte réel* doit voir le ticket créé à son propre nom, pas un 409/500 ni le nom d'autrui. Épinglé par `the_author_is_the_caller_whatever_the_body_claims` (tâche 3), qui emploie un compte réel pour que la clé étrangère ne masque pas le défaut.
2. **Un `PATCH` qui tente de réécrire l'auteur** — l'auteur d'un ticket existant ne doit pas bouger. Épinglé par `the_author_survives_an_update_that_names_another` (tâche 3).
3. **Un commentaire posté au nom d'autrui** — même règle que les tickets. Épinglé par `the_hand_edits_of_help_desk_are_in_place` (tâche 3), qui exige `user_uuid()` dans les deux contrôleurs et aucun `auteur_id` dans les DTO d'entrée des deux features.
4. **Un fichier en trop dans l'exemple après un build frontend** — `frontend/package-lock.json`, que `.gitignore` ne couvre pas, fait échouer la non-dérive. Épinglé par le passage d'`integration_examples` après nettoyage (tâche 4, étape 7).
5. **Une région citée qui disparaît** — `integration_examples` ignore les lignes `region`, seul `npm run build` sous `docs/` la voit. Épinglé par le build docs de la tâche 6.

---

## Carte des fichiers

| Fichier | Rôle | Tâche |
|---|---|---|
| `crates/rbs-cli/tests/integration_examples.rs` | `crud`/`champs` → `cruds` ; entrée `help-desk` ; tests `help_desk_is_what_the_cli_produces_today` et `the_hand_edits_of_help_desk_are_in_place` | 1, 2, 3 |
| `examples/help-desk/**` | le projet engendré | 2 |
| `examples/help-desk/src/{tickets,commentaires}/{dto,service,controller}.rs` | la retouche Rust | 3 |
| `examples/help-desk/src/tickets/tests/auteur.rs` (nouveau), `…/tests/mod.rs` | la preuve | 3 |
| `examples/help-desk/frontend/src/admin/vues/{Tickets,Commentaires}.vue` | la retouche Vue | 4 |
| `examples/help-desk/frontend/src/api/client.ts` | client typé régénéré | 4 |
| `examples/README.md`, `examples/README.fr.md` | tableau + section `### help-desk` | 5 |
| `docs/docs/tutorials/help-desk.md`, `docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/help-desk.md` | la page | 6 |
| `docs/docs/tutorials/typescript-client.md` + jumeau fr | « This is the last tutorial » | 6 |

---

### Task 1 : le harnais rejoue plusieurs CRUD

**Files :**
- Modify : `crates/rbs-cli/tests/integration_examples.rs:16-45` (struct `Exemple`), `:47-199` (six entrées), `:296-319` (fin de `generate`)

**Interfaces :**
- Produces : le champ `cruds: &'static [(&'static str, &'static str)]` d'`Exemple`, (nom de table, `--fields`), engendrés dans l'ordre.

- [ ] **Step 1 : remplacer les deux champs de la struct**

Dans `struct Exemple`, remplacer :

```rust
    crud: &'static str,
    champs: &'static str,
```

par :

```rust
    /// Les tables de `rbs generate crud`, dans l'ordre : son nom, puis ses `--fields`.
    ///
    /// Une seule partout sauf dans `help-desk`, dont la seconde table référence la
    /// première : l'ordre est celui des clés étrangères. `role` et `with_upload`
    /// s'appliquent à chacune.
    cruds: &'static [(&'static str, &'static str)],
```

- [ ] **Step 2 : convertir les six entrées**

Chaque paire `crud: "X",` + `champs: "Y",` devient `cruds: &[("X", "Y")],`, en gardant les commentaires qui précèdent `champs` au-dessus de `cruds`. Exemple pour `hello-crud` :

```rust
        cruds: &[("articles", "title:string,body:text,published:bool")],
```

Pour `admin-console`, la chaîne multiligne à `\` passe telle quelle dans le tuple :

```rust
        cruds: &[(
            "incidents",
            "reference:string:unique,sujet:string,detail:text,\
             gravite:enum(basse,moyenne,haute),ouvert:bool,duree_minutes:int:optional,\
             echeance:date:optional,constate_le:datetime",
        )],
```

- [ ] **Step 3 : boucler dans `generate`**

Remplacer le bloc qui va de `let mut args = vec![` jusqu'au `.success();` qui précède `racine` en fin de fonction par :

```rust
    for &(crud, champs) in example.cruds {
        let mut args = vec!["generate", "crud", crud, "--fields", champs, "--force"];
        if let Some(role) = example.role {
            args.extend(["--role", role]);
        }
        if example.with_upload {
            args.push("--with-upload");
        }

        assert_cmd::Command::cargo_bin("rbs")
            .expect("le binaire rbs doit être compilé")
            .current_dir(&racine)
            .args(args)
            .assert()
            .success();
    }
```

- [ ] **Step 4 : vérifier que rien d'autre ne lit `crud`/`champs`**

Run : `grep -n "\.crud\b\|\.champs\b\|crud:\|champs:" crates/rbs-cli/tests/integration_examples.rs`
Expected : aucune ligne.

- [ ] **Step 5 : les six exemples ne dérivent pas**

Run : `cargo test -p rbs-cli --test integration_examples --no-fail-fast > $SCRATCH/t1.log 2>&1; tail -20 $SCRATCH/t1.log`
Expected : `test result: ok`, tous les `*_is_what_the_cli_produces_today` verts. Puis `cargo fmt --all --check` et `cargo clippy -p rbs-cli --all-targets -- -D warnings` : verts.

- [ ] **Step 6 : commit**

```bash
git add crates/rbs-cli/tests/integration_examples.rs
git commit -F - <<'EOF'
test(examples): laisse un exemple engendrer plusieurs tables

Un exemple ne décrivait qu'un `generate crud`. Une table qui en référence
une autre demande deux appels, dans l'ordre des clés étrangères : le
harnais rejoue désormais une liste, que les six exemples existants
remplissent d'un seul élément.

Vérifications :
- cargo test -p rbs-cli --test integration_examples : <résultat réel>
- cargo clippy -p rbs-cli --all-targets -- -D warnings : <résultat réel>
EOF
```

---

### Task 2 : `examples/help-desk`, tel que le CLI le produit

**Files :**
- Create : `examples/help-desk/**`
- Modify : `crates/rbs-cli/tests/integration_examples.rs` (entrée `EXEMPLES` + test)

**Interfaces :**
- Consumes : `cruds` (tâche 1).
- Produces : un exemple non retouché, que la non-dérive valide ; les tâches 3 et 4 le retouchent.

- [ ] **Step 1 : écrire le test de non-dérive (rouge)**

Ajouter l'entrée à la fin d'`EXEMPLES` :

```rust
    Exemple {
        nom: "help-desk",
        database_url: "postgres://rbs:rbs@localhost:5432/help_desk",
        // `frontend-admin` tire `frontend` et `auth`, et par elle `mail` et `rate-limit`.
        features: &["frontend-admin"],
        // `commentaires` référence `tickets` : l'ordre est celui des clés étrangères, et
        // l'inverse est refusé par le CLI avant d'écrire quoi que ce soit.
        cruds: &[
            (
                "tickets",
                "sujet:string,detail:text,statut:enum(ouvert,en_cours,resolu,ferme),\
                 priorite:enum(basse,normale,haute),auteur:references:users",
            ),
            (
                "commentaires",
                "corps:text,ticket:references:tickets:cascade,auteur:references:users",
            ),
        ],
        role: None,
        with_upload: false,
        edite_a_la_main: &[],
        engendre_a_part: &[],
    },
```

et, à côté des autres :

```rust
#[test]
fn help_desk_is_what_the_cli_produces_today() {
    assert_no_drift(example("help-desk"));
}
```

Run : `cargo test -p rbs-cli --test integration_examples help_desk_is_what_the_cli_produces_today`
Expected : FAIL — `examples/help-desk` n'existe pas.

- [ ] **Step 2 : engendrer le projet, depuis la racine du dépôt**

```bash
cargo run -p rbs-cli --bin rbs -- new help-desk --yes \
  --core-path ./crates/rbs-core \
  --database-url 'postgres://rbs:rbs@localhost:5432/help_desk' \
  --lang fr
cd help-desk
git add -A && git commit -q -m 'avant frontend-admin'
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- add frontend-admin
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- generate crud tickets \
  --fields 'sujet:string,detail:text,statut:enum(ouvert,en_cours,resolu,ferme),priorite:enum(basse,normale,haute),auteur:references:users' \
  --force
cargo run --manifest-path ../Cargo.toml -p rbs-cli --bin rbs -- generate crud commentaires \
  --fields 'corps:text,ticket:references:tickets:cascade,auteur:references:users' \
  --force
cd .. && mv help-desk examples/help-desk
```

- [ ] **Step 3 : les trois éditions communes à tout exemple** (`examples/README.md`, section *Edits the CLI does not produce*)

```bash
rm -rf examples/help-desk/.git
```

Puis, dans `examples/help-desk/Cargo.toml` (et `migration/Cargo.toml` s'il nomme `rbs-core`), réécrire le `path` absolu de `rbs-core` en `path = "../../crates/rbs-core"` en gardant `default-features` et `features` intacts. Aucune région n'est encore à restaurer.

- [ ] **Step 4 : le test passe**

Run : `cargo test -p rbs-cli --test integration_examples --no-fail-fast > $SCRATCH/t2.log 2>&1; tail -20 $SCRATCH/t2.log`
Expected : les sept `*_is_what_the_cli_produces_today` verts, et `each_example_file_is_tracked_by_git` **rouge** tant que rien n'est ajouté à l'index — l'ajouter (`git add examples/help-desk`) puis relancer : vert.

- [ ] **Step 5 : l'exemple compile sans avertissement**

Run : `cd examples/help-desk && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected : vert. (Première compilation longue.)

- [ ] **Step 6 : commit**

```bash
git add examples/help-desk crates/rbs-cli/tests/integration_examples.rs
git commit -F - <<'EOF'
docs(examples): ajoute help-desk, un gestionnaire de tickets engendré

Deux tables — tickets et commentaires — sur un projet portant
frontend-admin : le seul exemple dont une table référence une autre
table engendrée, et non seulement celles d'auth.

Vérifications :
- cargo test -p rbs-cli --test integration_examples : <résultat réel>
- examples/help-desk : cargo clippy --workspace --all-targets -- -D warnings : <résultat réel>
EOF
```

---

### Task 3 : l'auteur pris dans le jeton, côté Rust

**Files :**
- Modify : `examples/help-desk/src/tickets/{dto,service,controller}.rs`, `examples/help-desk/src/commentaires/{dto,service,controller}.rs`, `examples/help-desk/src/tickets/tests/mod.rs`
- Create : `examples/help-desk/src/tickets/tests/auteur.rs`
- Modify : `crates/rbs-cli/tests/integration_examples.rs` (`edite_a_la_main` + test)

**Interfaces :**
- Consumes : l'exemple de la tâche 2 ; `rbs_core::Identity::user_uuid(&self) -> rbs_core::Result<Uuid>` ; `rbs_core::jwt::verify(token: &str, secret: &str) -> Result<Claims, JwtError>` ; dans `src/tickets/tests/mod.rs` : `application() -> Router`, `call(&Router, Request<Body>) -> (StatusCode, Value)`, `token(&DatabaseConnection, &str) -> String`.
- Produces : `service::create(db: &DatabaseConnection, auteur: Uuid, input: CreateX) -> Result<XResponse>` dans les deux features ; régions `entree` (dto), `create` (service et controller), `auteur` (test), citées par la tâche 6.

- [ ] **Step 1 : monter une base pour les tests de l'exemple**

Les tests `#[ignore]` de l'exemple ne tournent jamais dans la suite : il faut un PostgreSQL à la main.

```bash
docker run -d --rm --name help-desk-pg -e POSTGRES_USER=rbs -e POSTGRES_PASSWORD=rbs \
  -e POSTGRES_DB=help_desk -p 5432:5432 postgres:16
cd examples/help-desk && cargo run -p migration -- up
```

Expected : les migrations `create_tickets` puis `create_commentaires` appliquées. (Si le port 5432 est pris, arrêter le conteneur qui l'occupe plutôt que changer l'URL — elle est versionnée dans `.env`.)

- [ ] **Step 2 : écrire le test (rouge)**

Créer `examples/help-desk/src/tickets/tests/auteur.rs` :

```rust
use serde_json::json;

use super::*;

/// L'identifiant que porte `jeton` dans son `sub`.
fn sujet(jeton: &str) -> String {
    let config = rbs_core::Config::load().expect("configuration lisible");

    rbs_core::jwt::verify(jeton, &config.auth.secret)
        .expect("jeton lisible")
        .sub
}

// region: auteur
/// L'auteur d'un ticket est l'appelant, quoi que le corps prétende.
///
/// Le corps nomme un autre compte, réel : un identifiant inventé ferait refuser l'insertion
/// par la clé étrangère, et le test passerait sans rien prouver.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_author_is_the_caller_whatever_the_body_claims() {
    let api = application().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");
    let jeton = token(&db, "user").await;
    let autrui = sujet(&token(&db, "user").await);

    let requete = Request::builder()
        .method("POST")
        .uri("/tickets")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::from(
            json!({
                "sujet": "L'imprimante du deuxième",
                "detail": "Elle imprime tout en double.",
                "statut": "ouvert",
                "priorite": "haute",
                "auteur_id": autrui,
            })
            .to_string(),
        ))
        .expect("requête bien formée");

    let (status, body) = call(&api, requete).await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["auteur_id"], sujet(&jeton).as_str(), "{body}");
}
// endregion: auteur

/// Un `PATCH` ne réécrit pas l'auteur : le champ n'est plus dans le contrat de mise à jour.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn the_author_survives_an_update_that_names_another() {
    let api = application().await;
    let config = rbs_core::Config::load().expect("configuration lisible");
    let db = rbs_core::db::connect(&config.database)
        .await
        .expect("base joignable");
    let jeton = token(&db, "user").await;
    let autrui = sujet(&token(&db, "user").await);
    let envoi = |methode: &str, chemin: &str, corps: Value| {
        Request::builder()
            .method(methode)
            .uri(chemin)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {jeton}"))
            .body(Body::from(corps.to_string()))
            .expect("requête bien formée")
    };

    let (_, cree) = call(
        &api,
        envoi(
            "POST",
            "/tickets",
            json!({
                "sujet": "Le badge du parking",
                "detail": "Il ne s'ouvre plus.",
                "statut": "ouvert",
                "priorite": "normale",
            }),
        ),
    )
    .await;
    let chemin = format!("/tickets/{}", cree["id"].as_str().expect("id rendu"));
    let (status, body) = call(
        &api,
        envoi("PATCH", &chemin, json!({ "statut": "en_cours", "auteur_id": autrui })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["statut"], "en_cours", "{body}");
    assert_eq!(body["auteur_id"], sujet(&jeton).as_str(), "{body}");
}
```

Vérifier d'abord la méthode de mise à jour que le contrôleur engendré monte (`grep -n "patch\|put" examples/help-desk/src/tickets/mod.rs`) : si c'est `put`, remplacer `"PATCH"` par `"PUT"` dans le test.

Dans `examples/help-desk/src/tickets/tests/mod.rs`, ajouter `mod auteur;` en tête de la liste finale des modules (ordre alphabétique : `mod access;` puis `mod auteur;` puis `mod errors;`).

Run : `cd examples/help-desk && cargo test tickets::tests::auteur -- --include-ignored`
Expected : les deux tests FAIL — le premier sur `auteur_id` (le serveur a pris celui du corps), le second sur la création sans `auteur_id` (422) ou sur l'auteur réécrit.

- [ ] **Step 3 : le DTO**

Dans `src/tickets/dto.rs`, supprimer `pub auteur_id: Uuid,` de `CreateTicket` et `pub auteur_id: Option<Uuid>,` d'`UpdateTicket`, et encadrer les deux structs d'entrée :

```rust
// region: entree
#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateTicket {
    #[validate(length(max = 255))]
    pub sujet: String,
    pub detail: String,
    pub statut: TicketStatut,
    pub priorite: TicketPriorite,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdateTicket {
    #[validate(length(max = 255))]
    pub sujet: Option<String>,
    pub detail: Option<String>,
    pub statut: Option<TicketStatut>,
    pub priorite: Option<TicketPriorite>,
}
// endregion: entree
```

`TicketResponse` garde `auteur_id`.

- [ ] **Step 4 : le service**

Dans `src/tickets/service.rs`, remplacer la fonction `create` par :

```rust
// region: create
pub async fn create(
    db: &DatabaseConnection,
    auteur: Uuid,
    input: CreateTicket,
) -> Result<TicketResponse> {
    let ticket = ActiveModel {
        sujet: Set(input.sujet),
        detail: Set(input.detail),
        statut: Set(input.statut),
        priorite: Set(input.priorite),
        auteur_id: Set(auteur),
        ..Default::default()
    };

    Ok(repository::create(db, ticket).await?.into())
}
// endregion: create
```

et, dans `update`, supprimer le bloc :

```rust
    if let Some(auteur_id) = input.auteur_id {
        ticket.auteur_id = Set(auteur_id);
    }
```

- [ ] **Step 5 : le contrôleur**

Dans `src/tickets/controller.rs`, le corps de `create` devient (l'annotation `#[utoipa::path]` ne change pas ; la région enveloppe la fonction seule) :

```rust
// region: create
pub async fn create(
    State(state): State<AppState>,
    identite: Identity,
    ValidatedJson(input): ValidatedJson<CreateTicket>,
) -> Result<(StatusCode, Json<TicketResponse>)> {
    identite.require_role(Role::User)?;
    // L'auteur est l'appelant : lu dans le corps, il laisserait écrire au nom d'autrui.
    let auteur = identite.user_uuid()?;

    let ticket = service::create(state.core().db(), auteur, input).await?;

    Ok((StatusCode::CREATED, Json(ticket)))
}
// endregion: create
```

- [ ] **Step 6 : le test passe**

Run : `cd examples/help-desk && cargo test tickets::tests::auteur -- --include-ignored`
Expected : 2 passed.

- [ ] **Step 7 : le même geste sur `commentaires`**

- `src/commentaires/dto.rs` : retirer `pub auteur_id: Uuid,` de `CreateCommentaire` et `pub auteur_id: Option<Uuid>,` d'`UpdateCommentaire`. `ticket_id` reste.
- `src/commentaires/service.rs` :

```rust
pub async fn create(
    db: &DatabaseConnection,
    auteur: Uuid,
    input: CreateCommentaire,
) -> Result<CommentaireResponse> {
    let commentaire = ActiveModel {
        corps: Set(input.corps),
        ticket_id: Set(input.ticket_id),
        auteur_id: Set(auteur),
        ..Default::default()
    };

    Ok(repository::create(db, commentaire).await?.into())
}
```

et supprimer, dans `update`, le bloc `if let Some(auteur_id) = input.auteur_id { commentaire.auteur_id = Set(auteur_id); }`.
- `src/commentaires/controller.rs`, dans `create` :

```rust
    identite.require_role(Role::User)?;
    // L'auteur est l'appelant : lu dans le corps, il laisserait écrire au nom d'autrui.
    let auteur = identite.user_uuid()?;

    let commentaire = service::create(state.core().db(), auteur, input).await?;
```

- [ ] **Step 8 : toute la suite de l'exemple**

Run : `cd examples/help-desk && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace -- --include-ignored > $SCRATCH/t3.log 2>&1; tail -30 $SCRATCH/t3.log`
Expected : clippy vert ; tous les tests verts, ignorés compris.

- [ ] **Step 9 : déclarer et garder les retouches dans le harnais (rouge puis vert)**

Dans l'entrée `help-desk` :

```rust
        // L'auteur se lit dans le jeton, sur les deux tables : c'est la retouche que le
        // tutoriel enseigne, et `the_hand_edits_of_help_desk_are_in_place` en répond.
        edite_a_la_main: &[
            "src/tickets/dto.rs",
            "src/tickets/service.rs",
            "src/tickets/controller.rs",
            "src/tickets/tests/mod.rs",
            "src/tickets/tests/auteur.rs",
            "src/commentaires/dto.rs",
            "src/commentaires/service.rs",
            "src/commentaires/controller.rs",
        ],
```

et le test, après `the_hand_edits_of_event_hub_are_in_place` :

```rust
/// L'auteur est-il encore lu dans le jeton, sur les deux tables ?
///
/// Les fichiers retouchés sortent de la comparaison : sans ce test, un `auteur_id` revenu
/// dans un DTO d'entrée rouvrirait l'écriture au nom d'autrui, et le tutoriel qui cite
/// l'exemple décrirait une garde que plus rien ne tient.
#[test]
fn the_hand_edits_of_help_desk_are_in_place() {
    let racine = common::depot().join("examples").join("help-desk");
    let lire = |relatif: &str| {
        std::fs::read_to_string(racine.join(relatif))
            .unwrap_or_else(|erreur| panic!("{relatif} illisible : {erreur}"))
    };

    for feature in ["tickets", "commentaires"] {
        let controleur = lire(&format!("src/{feature}/controller.rs"));
        assert!(
            controleur.contains("identite.user_uuid()?"),
            "src/{feature}/controller.rs : l'auteur n'est plus lu dans le jeton"
        );

        let dto = lire(&format!("src/{feature}/dto.rs"));
        for bloc in dto.split("pub struct").skip(1) {
            let entree = bloc.starts_with(" Create") || bloc.starts_with(" Update");
            assert!(
                !(entree && bloc.contains("auteur_id")),
                "src/{feature}/dto.rs : un DTO d'entrée accepte de nouveau `auteur_id`"
            );
        }
    }

    let tests = lire("src/tickets/tests/auteur.rs");
    assert!(
        tests.contains("async fn the_author_is_the_caller_whatever_the_body_claims()"),
        "le test que le tutoriel cite a disparu"
    );
    assert!(
        lire("src/tickets/tests/mod.rs").contains("mod auteur;"),
        "`auteur.rs` n'est plus déclaré : ses tests ne compileraient plus"
    );
}
```

Vérifier le rouge : remettre temporairement `pub auteur_id: Uuid,` dans `CreateCommentaire`, lancer `cargo test -p rbs-cli --test integration_examples the_hand_edits_of_help_desk_are_in_place` → FAIL sur le message attendu ; annuler la modification temporaire ; relancer → PASS.

Run final : `cargo test -p rbs-cli --test integration_examples --no-fail-fast > $SCRATCH/t3b.log 2>&1; tail -20 $SCRATCH/t3b.log`
Expected : tout vert.

- [ ] **Step 10 : commit**

```bash
git add examples/help-desk crates/rbs-cli/tests/integration_examples.rs
git commit -F - <<'EOF'
docs(examples): lit l'auteur des tickets et commentaires dans le jeton

Tel qu'engendré, `auteur_id` venait du corps : tout compte connecté
écrivait au nom de qui il voulait. Le contrôleur lit désormais l'auteur
dans le jeton et le passe au service ; les DTO d'entrée ne l'acceptent
plus, ni à la création ni à la mise à jour.

Vérifications :
- examples/help-desk : cargo test --workspace -- --include-ignored (PostgreSQL 16 local) : <résultat réel>
- examples/help-desk : cargo clippy --workspace --all-targets -- -D warnings : <résultat réel>
- the_hand_edits_of_help_desk_are_in_place : rouge sur un `auteur_id` réintroduit, vert après : <résultat réel>
- cargo test -p rbs-cli --test integration_examples : <résultat réel>
EOF
```

---

### Task 4 : la retouche portée à l'écran

**Files :**
- Create : `examples/help-desk/frontend/src/api/client.ts` (engendré)
- Modify : `examples/help-desk/frontend/src/admin/vues/Tickets.vue`, `…/Commentaires.vue`
- Modify : `crates/rbs-cli/tests/integration_examples.rs` (`edite_a_la_main`, `engendre_a_part`)

**Interfaces :**
- Consumes : les DTO sans `auteur_id` (tâche 3).
- Produces : la région `corps` de `Tickets.vue`, citée par la tâche 6.

- [ ] **Step 1 : régénérer le client**

Run : `cd examples/help-desk && cargo run --manifest-path ../../Cargo.toml -p rbs-cli --bin rbs -- generate client --lang ts --out frontend/src/api --force`
Expected : `✓ client engendré — frontend/src/api/client.ts porte N opérations`. Noter la sortie exacte : la tâche 6 la transcrit.

Run : `grep -n "auteur_id" frontend/src/api/client.ts`
Expected : présent seulement dans `TicketResponse`, `CommentaireResponse` et les filtres — absent de `CreateTicket`, `UpdateTicket`, `CreateCommentaire`, `UpdateCommentaire`.

- [ ] **Step 2 : constater que rien n'échoue encore**

```bash
cd examples/help-desk/frontend && npm install && npm run typecheck
```

Expected : vert, bien que `corps()` envoie encore `auteur_id` — c'est le constat que la page rapporte (`corps()` n'a pas de type de retour déclaré). Si au contraire `vue-tsc` échoue, **s'arrêter** et le signaler : la spec et la page reposent sur ce silence.

- [ ] **Step 3 : retoucher `Tickets.vue`**

Retirer `auteur_id` de l'interface `Formulaire`, de `VIERGE`, de `corps()`, de `saisie()`, et supprimer le bloc de formulaire :

```vue
          <div class="flex flex-col gap-2">
            <Label for="champ-auteur_id">Auteur id</Label>
            <Input
              id="champ-auteur_id"
              ...
            />
          </div>
```

`Ligne`, `depuis()` et les deux entrées `{ cle: 'auteur_id', … }` des colonnes restent : la liste affiche l'auteur. `corps()` devient, avec sa région :

```ts
// region: corps
/**
 * Ce que la source reçoit, tiré de ce que le formulaire porte.
 *
 * L'auteur n'y est pas : le serveur le lit dans le jeton, et un champ envoyé en plus serait
 * jeté sans un mot — rien ne le typant ici, rien ne l'aurait signalé.
 */
function corps(formulaire: Formulaire) {
  return {
    sujet: formulaire.sujet,
    detail: formulaire.detail,
    statut: formulaire.statut as 'ouvert' | 'en_cours' | 'resolu' | 'ferme',
    priorite: formulaire.priorite as 'basse' | 'normale' | 'haute',
  }
}
// endregion: corps
```

- [ ] **Step 4 : même retouche sur `Commentaires.vue`**

Retirer `auteur_id` de `Formulaire`, `VIERGE`, `corps()`, `saisie()` et du formulaire ; garder `ticket_id` partout ; garder la colonne `auteur_id` de la liste. Dans le commentaire de `corps()`, la même phrase sur l'auteur.

- [ ] **Step 5 : le client se construit**

Run : `cd examples/help-desk/frontend && npm run typecheck && npm run build`
Expected : vert.

- [ ] **Step 6 : nettoyer et déclarer**

```bash
cd examples/help-desk/frontend && rm -rf node_modules dist package-lock.json
```

Dans l'entrée `help-desk` du harnais, ajouter à `edite_a_la_main` :

```rust
            "frontend/src/admin/vues/Tickets.vue",
            "frontend/src/admin/vues/Commentaires.vue",
```

et poser :

```rust
        // Le client typé que les écrans importent. Le rejeu ne lance pas la commande qui
        // l'écrit — elle compile le projet — et aucun job ne le régénère pour celui-ci :
        // la tâche de régénération d'`examples/README.md` le refait à la main.
        engendre_a_part: &["frontend/src/api/client.ts"],
```

Dans `the_hand_edits_of_help_desk_are_in_place`, ajouter :

```rust
    for ecran in ["Tickets", "Commentaires"] {
        let vue = lire(&format!("frontend/src/admin/vues/{ecran}.vue"));
        assert!(
            !vue.contains("champ-auteur_id"),
            "{ecran}.vue : le formulaire demande de nouveau un auteur que le serveur ignore"
        );
    }
```

- [ ] **Step 7 : non-dérive**

Run : `cargo test -p rbs-cli --test integration_examples --no-fail-fast > $SCRATCH/t4.log 2>&1; tail -20 $SCRATCH/t4.log`
Expected : vert. Un échec qui nomme `package-lock.json` signifie que l'étape 6 a été sautée.

- [ ] **Step 8 : commit**

```bash
git add examples/help-desk crates/rbs-cli/tests/integration_examples.rs
git commit -F - <<'EOF'
docs(examples): retire l'auteur des formulaires de help-desk

Le client régénéré n'accepte plus `auteur_id` en entrée, mais rien ne
l'a signalé : `corps()` n'a pas de type de retour, et le serveur jette
un champ inconnu. Les deux écrans cessent de demander un auteur qu'on
ne lit plus ; la liste continue de l'afficher.

Vérifications :
- npm run typecheck avant retouche : vert (le silence attendu)
- npm run typecheck && npm run build après retouche : <résultat réel>
- cargo test -p rbs-cli --test integration_examples : <résultat réel>
EOF
```

---

### Task 5 : le rejeu documenté dans `examples/README`

**Files :**
- Modify : `examples/README.md`, `examples/README.fr.md`

- [ ] **Step 1 : la ligne du tableau**, en fin de tableau, dans les deux langues.

EN :
```markdown
| `help-desk` | `frontend-admin` and two tables, `tickets` and `commentaires`, the second referencing the first: the project the end-to-end tutorial builds, author taken from the token rather than the body. |
```
FR :
```markdown
| `help-desk` | `frontend-admin` et deux tables, `tickets` et `commentaires`, la seconde référençant la première : le projet que construit le tutoriel de bout en bout, l'auteur lu dans le jeton plutôt que dans le corps. |
```

- [ ] **Step 2 : la section `### help-desk`**, sous *Regenerating* / *Régénérer*, après `### admin-console`. Contenu, dans chaque langue :
  1. le bloc de commandes exact de la tâche 2, étape 2, suivi de la régénération du client (tâche 4, étape 1) ;
  2. un paragraphe : l'ordre des deux `generate crud` est celui de la clé étrangère, l'inverse est refusé ;
  3. la liste des fichiers retouchés à la main — ceux d'`edite_a_la_main` — et ce que chacun porte (l'auteur lu dans le jeton ; les deux tests `auteur` ; les deux écrans sans champ auteur), et que `the_hand_edits_of_help_desk_are_in_place` les garde ;
  4. le rappel : après un `npm install` dans `frontend/`, supprimer `package-lock.json`, `node_modules/` et `dist/` ;
  5. le client n'est régénéré par aucun job CI pour cet exemple : il se refait par la commande ci-dessus.

- [ ] **Step 3 : parité**

Run : `cd docs && npm run parite`
Expected : aucune divergence signalée sur `examples/README*.md` (si l'instrument ne les couvre pas, relire les deux sections côte à côte, paragraphe par paragraphe).

- [ ] **Step 4 : commit**

```bash
git add examples/README.md examples/README.fr.md
git commit -m "docs(examples): donne la recette de régénération de help-desk" -m "Vérifications :
- npm run parite : <résultat réel>"
```

---

### Task 6 : la page du tutoriel, fr et en

**Files :**
- Create : `docs/docs/tutorials/help-desk.md`, `docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/help-desk.md`
- Modify : `docs/docs/tutorials/typescript-client.md:8`, `docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/typescript-client.md:8`

**Interfaces :**
- Consumes : les régions `entree` (`src/tickets/dto.rs`), `create` (`src/tickets/service.rs`, `src/tickets/controller.rs`), `auteur` (`src/tickets/tests/auteur.rs`), `corps` (`frontend/src/admin/vues/Tickets.vue`) ; la sortie de `generate client` notée à la tâche 4.

Modèle de ton et de forme : `docs/docs/tutorials/auth.md` — le lire en entier avant d'écrire. Mêmes intertitres de premier niveau (`## What you need`, `## 1. …`, `## Verify`, `## What was installed`, `## Going further`), phrases qui disent le *pourquoi*, pas de liste à puces là où un paragraphe suffit.

- [ ] **Step 1 : front matter et intro (EN)**

```markdown
---
sidebar_position: 10
title: Building a help desk
---

# Building a help desk
```

Intro, deux paragraphes : ce qu'on construit (tickets, commentaires, un espace d'administration servi par le binaire) ; ce qui distingue cette page — un projet neuf, `help-desk`, et non `demo`, qui n'a ni `auth` ni frontend ; la seule ligne écrite à la main est celle qui corrige une faille, et la page la montre. Puis la phrase qui renvoie à l'exemple : tout extrait vient de [`examples/help-desk`](https://github.com/tky0065/rbs/tree/main/examples/help-desk), compilé en CI.

- [ ] **Step 2 : `## What you need`** — Rust, Node 20+, Docker pour PostgreSQL et Mailpit, comme dans [Setting up](./setup.md).

- [ ] **Step 3 : `## 1. Create the project`**

````markdown
```bash
rbs new help-desk --lang fr
cd help-desk
git add -A && git commit -m 'projet neuf'
rbs add frontend-admin
```

{/* rbs:transcript cmd="rbs add frontend-admin" setup="rbs new help-desk --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/help_desk && git add -A && git -c user.email=rbs@example.com -c user.name=rbs commit -q -m init" dans="help-desk" extrait="oui" */}
```text
<sortie réelle : lancer la commande, coller la tête (description, fragments tirés, plan) et la dernière ligne, couper le milieu comme le fait guides/frontend.md>
```
````

Prose : pourquoi le commit (`add` refuse un arbre sale) ; ce que `frontend-admin` tire (`frontend`, `auth`, `mail`, `rate-limit`) et pourquoi (le shell exige un compte).

- [ ] **Step 4 : `## 2. Model the tickets`** — la commande `generate crud tickets` et son `rbs:transcript` (setup = celui de l'étape 3 suivi de `&& rbs add frontend-admin`), `extrait="oui"` si la sortie dépasse ~30 lignes. Prose : les deux `enum` (liste déroulante à l'écran, `CHECK` en base), `auteur:references:users` (une table d'`auth`, inventoriée par le CLI), l'écran `Tickets.vue` écrit et monté dans le rail et le routage, parce que le projet porte `frontend-admin`.

- [ ] **Step 5 : `## 3. Hang comments on them`** — d'abord la commande dans le mauvais ordre, sur un projet sans `tickets` :

````markdown
{/* rbs:transcript cmd="rbs generate crud commentaires --fields corps:text,ticket:references:tickets:cascade,auteur:references:users --dry-run" setup="rbs new help-desk --yes --lang fr --database-url postgres://rbs:secret@localhost:5432/help_desk --with auth" dans="help-desk" */}
```text
<sortie réelle — attendu : erreur : relation « ticket » — « tickets » est introuvable dans ce projet …>
```
````

puis la vraie commande, avec son transcript (setup : étape 4 + `&& rbs generate crud tickets --fields …`). Prose : `cascade` (supprimer un ticket emporte ses commentaires) ; `auteur` sans politique, donc `Restrict` ; l'ordre des migrations garanti par l'horodatage.

- [ ] **Step 6 : `## 4. Take the author from the token`**

Le défaut en premier : tel qu'engendré, `CreateTicket` porte `auteur_id`, donc tout compte connecté ouvre un ticket au nom d'un autre. Puis les trois retouches, chacune citée :

````markdown
```rust file=examples/help-desk/src/tickets/dto.rs region=entree
```

```rust file=examples/help-desk/src/tickets/controller.rs region=create
```

```rust file=examples/help-desk/src/tickets/service.rs region=create
```
````

Prose entre chaque : le DTO perd le champ à la création *et* à la mise à jour ; seul le contrôleur connaît le jeton ; le service reçoit l'auteur sans savoir d'où il vient — c'est la dépendance `controller → service → repository` qui le veut. Un paragraphe : le même geste sur `commentaires`, où `ticket_id` reste dans le corps parce que choisir le ticket commenté est légitime.

- [ ] **Step 7 : `## 5. Carry the change to the screen`**

La commande `rbs generate client --lang ts --out frontend/src/api` et son `rbs:transcript` (setup = étape 5 complet ; la sortie ne dépend pas des retouches). Puis le constat, franc : `npm run typecheck` passe encore. Pourquoi : `corps()` n'a pas de type de retour déclaré, TypeScript ne contrôle donc pas ses propriétés en trop, et le serveur jette un champ inconnu. L'écran afficherait un champ « auteur » que plus personne ne lit. Puis l'extrait :

````markdown
```ts file=examples/help-desk/frontend/src/admin/vues/Tickets.vue region=corps
```
````

La leçon en une phrase : le client typé suit le contrat, mais un écran ne signale un champ disparu que là où il type ce qu'il envoie. Puis la limite assumée : dans `Commentaires.vue`, `ticket_id` se saisit encore comme un UUID ; renvoi à [Frontend](../guides/frontend.md).

- [ ] **Step 8 : `## 6. Run it`**

```bash
docker compose up -d
make migrate
make dev
```

Prose : `make dev` lance le binaire et le serveur Vite ; l'écran vit sur `http://localhost:5173`. Les sorties de serveur vivant, s'il en est cité, en `rbs:libre` avec la raison employée par `auth.md`.

- [ ] **Step 9 : `## Verify`** — dans le navigateur : créer un compte (en développement, `login_requires_verification` vaut `false`, cf. `auth.md`), se connecter, ouvrir **Tickets**, créer un ticket : le formulaire ne demande pas d'auteur, la liste affiche le vôtre. Puis la preuve automatisée :

````markdown
```bash
cargo test --workspace -- --include-ignored
```

```rust file=examples/help-desk/src/tickets/tests/auteur.rs region=auteur
```
````

Prose : le corps nomme un autre compte réel, parce qu'un identifiant inventé serait refusé par la clé étrangère et le test passerait sans rien prouver.

- [ ] **Step 10 : `## What was installed`** et **`## Going further`** — trois sous-sections courtes pointant vers les fichiers déjà cités ; puis les liens : [Relations](../guides/relations.md), [Frontend](../guides/frontend.md), [Authentication](../guides/auth.md), et le sélecteur de ticket comme prochaine retouche naturelle.

- [ ] **Step 11 : la page française**

`docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials/help-desk.md`, `title: Construire un gestionnaire de tickets`, même `sidebar_position: 10`, mêmes blocs `rbs:transcript` et `file=` à l'identique (les sorties sont déjà en français), prose traduite paragraphe pour paragraphe. Lire `docs/i18n/fr/…/tutorials/auth.md` pour le vocabulaire (« jeton », « retouche », « engendré »).

- [ ] **Step 12 : « le dernier tutoriel »**

EN, `typescript-client.md:8` : remplacer `This is the last tutorial. It picks up` par `It picks up`, et ajouter en fin de page, dans `## Going further`, une phrase qui renvoie à [Building a help desk](./help-desk.md) comme suite naturelle. FR, même geste : `C'est le dernier tutoriel. Il reprend` → `Il reprend`, et le renvoi vers [Construire un gestionnaire de tickets](./help-desk.md).

- [ ] **Step 13 : transcriptions**

Run : `cargo test -p rbs-cli --test integration_docs --no-fail-fast > $SCRATCH/t6.log 2>&1; tail -40 $SCRATCH/t6.log`
Expected : vert. Un écart signale une sortie collée à la main — la remplacer par celle que le test affiche.

- [ ] **Step 14 : site, régions, liens, parité**

Run : `cd docs && npm ci && npm run build > $SCRATCH/t6b.log 2>&1; tail -20 $SCRATCH/t6b.log && npm run parite`
Expected : build vert dans les deux langues (une région manquante ou un lien cassé l'arrête) ; parité sans divergence sur `tutorials/help-desk.md` et `tutorials/typescript-client.md`.

- [ ] **Step 15 : commit**

```bash
git add docs/docs/tutorials docs/i18n/fr/docusaurus-plugin-content-docs/current/tutorials
git commit -F - <<'EOF'
docs(tutorials): construit un gestionnaire de tickets de bout en bout

Aucun tutoriel ne touchait au frontend ni ne montrait où le code engendré
se modifie. Celui-ci part d'un projet neuf, pose frontend-admin et deux
tables liées, puis corrige la seule faille de l'engendré — l'auteur lu
dans le corps — du contrôleur jusqu'à l'écran.

Vérifications :
- cargo test -p rbs-cli --test integration_docs : <résultat réel>
- npm run build (docs, fr et en) : <résultat réel>
- npm run parite : <résultat réel>
EOF
```

---

### Task 7 : passe finale

- [ ] **Step 1 : workspace**

Run : `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p rbs-cli --lib > $SCRATCH/t7.log 2>&1; tail -10 $SCRATCH/t7.log`
Expected : vert.

- [ ] **Step 2 : l'exemple, une dernière fois**

Run : `cd examples/help-desk && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace -- --include-ignored`
Expected : vert (conteneur `help-desk-pg` encore monté, sinon le remonter selon la tâche 3, étape 1). Puis `docker stop help-desk-pg`.

- [ ] **Step 3 : l'arbre est propre**

Run : `git status --short && command ls examples/help-desk/frontend`
Expected : rien à committer ; ni `node_modules`, ni `dist`, ni `package-lock.json`.
