# Auth : un jeton fermé rejoué ne ferme plus le compte — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rejouer un jeton de rafraîchissement *tourné* révoque le compte comme aujourd'hui ; rejouer un jeton *fermé* (logout, révocation, reset, change) ne rend qu'un 401.

**Architecture:** `refresh_tokens` gagne `replaced_at` : la rotation le pose, la fermeture pose `revoked_at`, une session est ouverte quand les deux sont nuls. Le dépôt scinde `consume` en `rotate` (rend `Rotation::{Done, Replayed, Closed}`) et `close` ; `refresh` ne révoque la famille que sur `Replayed`.

**Tech Stack:** SeaORM (`update_many` conditionnel, `Expr::value`), sea-orm-migration (`ColumnDef`), chrono, tests `#[ignore]` joints à la base, régénération de `examples/blog-auth` par diff.

**Spec:** `docs/superpowers/specs/2026-09-12-lot-secu-p2-design.md`, section 2.

## Global Constraints

- Fichiers sous `crates/rbs-cli/templates/features/auth/` sauf mention contraire.
- Toute date écrite ou comparée vient de Rust (`Utc::now().fixed_offset()`) liée en paramètre, jamais de `CURRENT_TIMESTAMP` (voir le commentaire en tête de `repository/refresh_token.rs.jinja`).
- `examples/blog-auth` se régénère **par diff** entre deux générations (recette : `examples/README.md` § blog-auth), jamais par écrasement ; `cargo test -p rbs-cli --test integration_examples` est l'oracle.
- Commits Conventional, sujet en français, sans identifiant de tâche ni ligne d'attribution.
- Passe lente : `cargo test -p rbs-cli --test integration_auth --no-fail-fast -- --ignored > $SCRATCHPAD/auth-lent-10.log 2>&1`.
- Ce plan précède `2026-09-12-auth-jeton-acces-revocable.md` et `2026-09-12-auth-email-normalise.md` sur la même branche.

---

### Task 1: Le schéma et le modèle

**Files:**
- Modify: `migration.rs.jinja:72-78` (colonne `replaced_at` après `revoked_at`, enum `RefreshTokens`)
- Modify: `model.rs.jinja:85-94` (champ `replaced_at`)

- [x] **Step 1: Migration** — commit 7a0a91c ; `\d refresh_tokens` sur le projet jetable liste `replaced_at | timestamp with time zone | nullable`

Après le bloc `RevokedAt` :

```rust
                    // Nul tant que le jeton n'a pas tourné. Distinct de `revoked_at` :
                    // un jeton tourné qui reparaît a été volé, un jeton fermé qui
                    // reparaît n'est qu'un client qui réessaie — et seul le premier
                    // justifie de fermer tout le compte.
                    .col(
                        ColumnDef::new(RefreshTokens::ReplacedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
```

et `ReplacedAt,` dans l'enum `RefreshTokens` après `RevokedAt,`.

- [x] **Step 2: Modèle** — commit 7a0a91c ; `cargo check --workspace --all-targets` vert sur le projet jetable — dans `model.rs.jinja`, module `refresh_token`, après `pub revoked_at: Option<DateTimeWithTimeZone>,` : `pub replaced_at: Option<DateTimeWithTimeZone>,`.

- [x] **Step 3: Commit** — 7a0a91c

```bash
git add crates/rbs-cli/templates/features/auth/
git commit -m "feat(auth): distingue en table un jeton tourné d'un jeton fermé"
```

---

### Task 2: Le dépôt — `rotate` et `close`

**Files:**
- Modify: `repository/refresh_token.rs.jinja` (remplace `consume` ; `open_sessions_of`, `revoke_sessions_of`, `revoke_session` filtrent aussi `ReplacedAt.is_null()`)
- Modify: `repository/mod.rs.jinja` si `consume` y est réexporté (vérifier par `grep -n consume repository/mod.rs.jinja`)

**Interfaces:**
- Produces: `pub enum Rotation { Done, Replayed, Closed }`, `pub async fn rotate(db, id: Uuid) -> Result<Rotation>`, `pub async fn close(db, id: Uuid) -> Result<bool>`.

- [x] **Step 1: Remplacer `consume`** — commit 32c987e ; `cargo check --workspace --all-targets` vert sur le projet jetable, plus aucun `consume` sur `refresh_token`

```rust
/// Ce qu'une rotation a trouvé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// La ligne était ouverte : c'est cet appel qui l'a tournée.
    Done,
    /// La ligne avait déjà tourné : le jeton a servi deux fois.
    Replayed,
    /// La ligne avait été fermée — déconnexion, révocation, mot de passe changé.
    Closed,
}

/// Tourne une session, et dit si c'est bien cet appel qui l'a fait.
///
/// L'`UPDATE` porte sa propre condition plutôt que de suivre une lecture : deux
/// rafraîchissements simultanés du même jeton franchiraient tous deux la lecture avant
/// que l'un ait écrit, et repartiraient chacun avec une paire valide. La lecture qui suit
/// un `UPDATE` sans effet ne décide de rien : elle nomme l'état déjà écrit.
pub async fn rotate(db: &DatabaseConnection, id: Uuid) -> Result<Rotation> {
    let touchees = refresh_token::Entity::update_many()
        .col_expr(
            refresh_token::Column::ReplacedAt,
            Expr::value(Utc::now().fixed_offset()),
        )
        .filter(refresh_token::Column::Id.eq(id))
        .filter(refresh_token::Column::ReplacedAt.is_null())
        .filter(refresh_token::Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    if touchees.rows_affected == 1 {
        return Ok(Rotation::Done);
    }

    let ligne = refresh_token::Entity::find_by_id(id).one(db).await?;

    Ok(match ligne {
        Some(ligne) if ligne.replaced_at.is_some() => Rotation::Replayed,
        _ => Rotation::Closed,
    })
}

/// Ferme une session présentée, et dit si c'est bien cet appel qui l'a fait.
pub async fn close(db: &DatabaseConnection, id: Uuid) -> Result<bool> {
    let touchees = refresh_token::Entity::update_many()
        .col_expr(
            refresh_token::Column::RevokedAt,
            Expr::value(Utc::now().fixed_offset()),
        )
        .filter(refresh_token::Column::Id.eq(id))
        .filter(refresh_token::Column::ReplacedAt.is_null())
        .filter(refresh_token::Column::RevokedAt.is_null())
        .exec(db)
        .await?;

    Ok(touchees.rows_affected == 1)
}
```

- [x] **Step 2: Les trois autres requêtes** — commit 32c987e ; `cargo test --lib -- --include-ignored auth::tests` : 44 passed, 0 failed (projet jetable, PostgreSQL) — ajouter `.filter(refresh_token::Column::ReplacedAt.is_null())` à `revoke_sessions_of`, `open_sessions_of`, `revoke_session`. Mettre à jour le commentaire de `revoke_sessions_of` (la phrase « distinguer les chaînes demanderait une colonne de famille, qu'un projet déjà migré ne recevrait jamais » reste vraie : la granularité reste le compte).

- [x] **Step 3: Commit** — regroupé avec la tâche 3 dans 32c987e (le service ne compile pas encore ; le commit suivant le répare — regrouper les deux si l'on préfère un arbre compilable à chaque commit : **préférer regrouper** avec la tâche 3).

---

### Task 3: Le service et les tests

**Files:**
- Modify: `service/session.rs.jinja:61-119` (`refresh`, `logout`)
- Modify: `tests/session.rs.jinja` (test neuf), `tests/password.rs.jinja:125-179` (commentaire périmé)

- [x] **Step 1: Test rouge dans `tests/session.rs.jinja`** — rouge observé sur le dépôt pré-correctif (`left: 401, right: 200`, « la session sœur est tombée sur le rejeu d'un jeton fermé »), vert après : 44 passed, après `a_revoked_refresh_returns_401` :

```rust
/// Un jeton fermé par `logout` puis rejoué n'est pas un jeton volé : c'est un client qui
/// réessaie. Le traiter comme un rejeu fermerait toutes les sessions du compte à la
/// demande de qui tient un jeton mort — trente jours durant.
#[tokio::test]
#[ignore = "joint la base du projet"]
async fn a_refresh_closed_by_logout_when_replayed_leaves_the_other_sessions_open() {
    let api = application().await;
    let email = fresh_email();
    register(&api, &email).await;

    let (_, fermee) = authenticate(&api, &email, PASSWORD).await;
    let (_, vivante) = authenticate(&api, &email, PASSWORD).await;

    let (status, _) = logout(&api, &refresh_for(&fermee)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (rejeu, body) = refresh(&api, &refresh_for(&fermee)).await;
    assert_eq!(rejeu, StatusCode::UNAUTHORIZED, "{body}");

    let (status, body) = refresh(&api, &refresh_for(&vivante)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "la session sœur est tombée sur le rejeu d'un jeton fermé : {body}"
    );
}
```

- [x] **Step 2: `refresh` et `logout`** — commit 32c987e ; `replaying_a_refresh_closes_the_other_sessions_of_the_account` et le test neuf verts

```rust
use super::super::repository::{self, ADRESSE_PRISE, refresh_token::Rotation};
// ...
    // Rien ici ne relit les deux colonnes : c'est `rotate` qui porte la condition, et
    // elle seule peut la porter sans laisser passer deux rafraîchissements concurrents.
    match repository::refresh_token::rotate(db, session.id).await? {
        Rotation::Done => {}
        // La ligne avait tourné : ce jeton a servi deux fois. L'un de ses deux porteurs
        // n'est pas le titulaire du compte, et rien ne dit lequel — un jeton volé et joué
        // avant la rotation légitime laisserait sinon le voleur avec une paire valide,
        // renouvelée indéfiniment. Tout le compte se reconnecte.
        Rotation::Replayed => {
            let fermees = repository::revoke_sessions_of(db, session.user_id).await?;

            // Ni l'adresse ni le jeton : le journal ne porte pas ce que la réponse tait,
            // et l'identifiant du compte suffit à retrouver ce qui s'est passé.
            tracing::warn!(
                user_id = %session.user_id,
                sessions_revoquees = fermees,
                "jeton de rafraîchissement rejoué : les sessions du compte sont révoquées"
            );

            return Err(Error::Unauthorized);
        }
        // Fermée par une déconnexion ou une révocation : un client qui réessaie, pas un
        // jeton qui circule. Le même 401 qu'un jeton inconnu, et rien d'autre.
        Rotation::Closed => return Err(Error::Unauthorized),
    }
```

`logout` : `if !repository::refresh_token::close(db, session.id).await? {` — le commentaire au-dessus se raccourcit : « La session ferme la ligne présentée, et elle seule : les autres appareils du même compte gardent la leur. Un jeton déjà fermé ne l'est pas deux fois. »

Vérifier comment `repository/mod.rs.jinja` réexporte (`pub use refresh_token::{...}`) et aligner les chemins.

- [x] **Step 3: Commentaire périmé** — commit 32c987e dans `tests/password.rs.jinja` (`changing_the_password_returns_a_usable_pair_and_closes_the_others`) : le paragraphe « L'ordre compte : rejouer le jeton de `premiere` arme la défense anti-rejeu… » est faux désormais — un jeton fermé rejoué ne ferme rien. Le remplacer par : « La nouvelle paire d'abord, puis l'ancienne : l'ordre n'importe plus depuis qu'un jeton fermé rejoué ne ferme rien d'autre, mais lire le succès avant le refus est ce qu'un lecteur attend. »

- [x] **Step 4: Régénérer `examples/blog-auth` par diff** — diff 8c71ad2 → 32c987e appliqué sans `.rej`, `migration/src/lib.rs` inchangé ; `cargo clippy --all-targets -- -D warnings` vert dans l'exemple — générer deux fois (avant/après les templates, ou `git stash` des templates) dans le scratchpad, `diff -ru` des deux sorties, appliquer le diff sur `examples/blog-auth` (`patch -p1`). Puis, dans `examples/blog-auth` : `cargo clippy --all-targets -- -D warnings` (nécessite le noyau : `--core-path`, déjà relatif dans son `Cargo.toml`).

- [x] **Step 5: Vérifier** — `integration_examples` 19 passed ; `auth-lent-10.log` : 8 passed, 0 failed (dont le banc SQLite, 191 s) ; `fmt --check` et `clippy --workspace` verts — `cargo test -p rbs-cli --test integration_examples` → vert ; passe lente `integration_auth` (voir les contraintes) → tous verts, dont le banc SQLite ; `cargo fmt --all --check` ; `cargo clippy --workspace --all-targets -- -D warnings`.

- [x] **Step 6: Commit** — 32c987e

```bash
git add crates/rbs-cli/templates/features/auth/ examples/blog-auth/
git commit -m "fix(auth): ne révoque le compte que sur le rejeu d'un jeton tourné, jamais d'un jeton fermé"
```

Corps : le pourquoi (un attaquant expulsé rejouait son jeton mort à chaque reconnexion de la victime), puis `Vérifications :` avec les commandes et leurs comptes réels.

---

### Task 4: Documentation et notes de version

**Files:**
- Modify: `docs/docs/guides/auth.md:132-136` et l'équivalent FR (`docs/i18n/fr/docusaurus-plugin-content-docs/current/guides/auth.md`)
- Modify: `CHANGELOG.md`, `CHANGELOG.fr.md`

- [x] **Step 1: Guide** — commit 2183d2b (EN + FR) — remplacer le paragraphe « Refreshing **rotates** the pair … which is why the two operations share their repository call. » par :

```markdown
Refreshing **rotates** the pair: the token presented is marked replaced in the same
conditional `UPDATE` that reads it, so two concurrent refreshes cannot both win. A
replaced token presented again has been used twice — one of its two holders is not the
account owner — and every session of the account is closed. Logging out, revoking a
session, resetting or changing the password **close** a token instead, in a separate
column: a closed token presented again gets a 401 and nothing else, because a client
retrying a logout is not a stolen token circulating.
```

FR : même sens, même emplacement.

- [x] **Step 2: CHANGELOG** — commit 2183d2b, entrée 1.5.0 créée (EN `### Fixed`, FR `### Corrigé`) — sous `## [1.5.0] — 2026-09-12` (créer l'entrée si absente), `### Fixed` :

```markdown
- **A refresh token closed by logout, replayed, no longer closes the whole account.**
  `refresh_tokens` gains `replaced_at`: rotation sets it, closing (`logout`,
  `DELETE /auth/sessions`, password reset or change) sets `revoked_at`, and only a
  *replaced* token presented again triggers the family revocation. An attacker thrown out
  by a reset could otherwise log the victim out at will for thirty days by replaying a
  dead token. **Projects generated before 1.5.0:** the migration only changes for a fresh
  `rbs add auth`; run `ALTER TABLE refresh_tokens ADD COLUMN replaced_at timestamptz NULL;`
  (`datetime NULL` on MySQL, `TEXT NULL` on SQLite) and copy the new `rotate`/`close`
  pair from the fragment.
```

- [x] **Step 3: `cargo test -p rbs-cli --test integration_docs`** → 13 passed; 0 failed; 1 ignored (commit 2183d2b). Commit :

```bash
git commit -am "docs(auth): sépare rotation et fermeture dans le cycle des jetons"
```
