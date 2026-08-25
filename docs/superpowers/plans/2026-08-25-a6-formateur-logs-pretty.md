# Formateur de logs `pretty` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** doter `rbs-core` d'un `FormatEvent` maison qui rend une ligne de log lisible en développement, à la place du formateur `pretty` de `tracing-subscriber`, jugé trop verbeux.

**Architecture:** un module `logs` accueillant `pretty.rs` — `PrettyFormat` implémente `FormatEvent` et écrit `HH:MM:SS  NIVEAU  cible  message  champs` sur une seule ligne. Les champs portés par les spans parents sont repris après ceux de l'événement, sans quoi le `request_id` attaché au span par le futur middleware `request_id` n'apparaîtrait jamais. La couleur est décidée à la construction (`stdout().is_terminal()`) et forçable, ce qui la rend testable. Le module est un répertoire dès maintenant : le formateur `json` et la bascule `RBS_LOG_FORMAT` viendront s'y ajouter.

**Tech Stack:** `tracing`, `tracing-subscriber` (features `fmt`, `ansi`, `chrono`), `nu-ansi-term` (déjà transitif via `ansi`), `std::io::IsTerminal`.

**Spec:** `docs/superpowers/specs/2026-08-25-rbs-design.md` §5.2

## Global Constraints

- `#![warn(missing_docs)]` sur `rbs-core` : tout item public porte un `///` d'une à trois lignes.
- Un commentaire explique le *pourquoi*, jamais le *quoi*.
- `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --all --check` doivent rester propres.
- Un fichier de feature au-delà de ~200 lignes signale un découpage à faire.
- Dépendances déclarées dans `[workspace.dependencies]`, reprises par `.workspace = true`.

---

### Task 1 : formateur `pretty`

**Files:**
- Create: `crates/rbs-core/src/logs/mod.rs`
- Create: `crates/rbs-core/src/logs/pretty.rs` (implémentation + `#[cfg(test)] mod tests`)
- Create: `crates/rbs-core/examples/logs_pretty.rs`
- Modify: `crates/rbs-core/src/lib.rs` (déclaration du module `logs`)
- Modify: `crates/rbs-core/Cargo.toml`, `Cargo.toml` (racine : `nu-ansi-term`)

**Interfaces:**
- Consumes: rien des tâches antérieures.
- Produces: `rbs_core::logs::PrettyFormat`, avec `PrettyFormat::new() -> Self` (couleur = `std::io::stdout().is_terminal()`) et `PrettyFormat::with_ansi(bool) -> Self`. `impl<S, N> FormatEvent<S, N> for PrettyFormat where S: Subscriber + for<'a> LookupSpan<'a>, N: for<'a> FormatFields<'a> + 'static`. La tâche A7 consommera ces deux constructeurs pour la bascule `RBS_LOG_FORMAT`.

- [ ] **Step 1 : déclarer les dépendances**

Racine `Cargo.toml`, dans `[workspace.dependencies]` : `nu-ansi-term = "0.50.3"`.
`crates/rbs-core/Cargo.toml`, dans `[dependencies]` :

```toml
nu-ansi-term.workspace = true
tracing-subscriber = { workspace = true, features = ["fmt", "ansi", "chrono"] }
```

- [ ] **Step 2 : écrire les tests d'abord**

Dans `crates/rbs-core/src/logs/pretty.rs`, un `mod tests` avec un `MakeWriter` de test et cinq cas :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct Tampon(Arc<Mutex<Vec<u8>>>);

    impl Tampon {
        fn contenu(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl io::Write for Tampon {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Tampon {
        type Writer = Tampon;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn capture(format: PrettyFormat, emettre: impl FnOnce()) -> String {
        let tampon = Tampon::default();
        let abonne = tracing_subscriber::fmt()
            .event_format(format)
            .with_writer(tampon.clone())
            .finish();
        tracing::subscriber::with_default(abonne, emettre);
        tampon.contenu()
    }

    #[test]
    fn aucune_couleur_quand_la_sortie_n_est_pas_un_tty() {
        let sortie = capture(PrettyFormat::new(), || tracing::info!("bonjour"));
        assert!(!sortie.contains('\u{1b}'), "sortie colorée hors TTY : {sortie:?}");
    }

    #[test]
    fn les_couleurs_sont_presentes_quand_elles_sont_forcees() {
        let sortie = capture(PrettyFormat::with_ansi(true), || tracing::info!("bonjour"));
        assert!(sortie.contains('\u{1b}'), "aucune séquence ANSI : {sortie:?}");
    }

    #[test]
    fn la_ligne_porte_le_niveau_la_cible_le_message_puis_les_champs() {
        let sortie = capture(PrettyFormat::with_ansi(false), || {
            tracing::warn!(actives = 18, max = 20, "pool proche de la saturation")
        });
        let niveau = sortie.find("WARN").expect("niveau absent");
        let cible = sortie.find("rbs_core::logs::pretty").expect("cible absente");
        let message = sortie.find("pool proche de la saturation").expect("message absent");
        let champs = sortie.find("actives=18").expect("champs absents");
        assert!(niveau < cible && cible < message && message < champs, "ordre inattendu : {sortie:?}");
        assert!(sortie.contains("max=20"));
    }

    #[test]
    fn les_champs_d_un_span_parent_sont_repris_apres_ceux_de_l_evenement() {
        let sortie = capture(PrettyFormat::with_ansi(false), || {
            let span = tracing::info_span!("requete", request_id = "01JQ3F8K2P");
            let _entree = span.enter();
            tracing::error!(status = 422, "requête refusée");
        });
        let champ_evenement = sortie.find("status=422").expect("champ d'événement absent");
        let champ_span = sortie.find("request_id=01JQ3F8K2P").expect("champ de span absent");
        assert!(champ_evenement < champ_span, "ordre inattendu : {sortie:?}");
    }

    #[test]
    fn les_cinq_niveaux_sont_rendus_avec_leur_libelle() {
        let sortie = capture(PrettyFormat::with_ansi(false), || {
            tracing::trace!("t");
            tracing::debug!("d");
            tracing::info!("i");
            tracing::warn!("w");
            tracing::error!("e");
        });
        for niveau in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] {
            assert!(sortie.contains(niveau), "niveau {niveau} absent : {sortie:?}");
        }
    }
}
```

Note : les trois premiers niveaux sont filtrés par défaut si aucun filtre n'est posé — `tracing_subscriber::fmt()` sans `EnvFilter` laisse passer tout ce que le `max_level` autorise ; si `trace!`/`debug!` n'apparaissent pas, ajouter `.with_max_level(tracing::Level::TRACE)` au constructeur de `capture`.

- [ ] **Step 3 : lancer les tests, les voir échouer**

Run: `cargo test -p rbs-core logs`
Expected: échec de compilation, `PrettyFormat` inexistant.

- [ ] **Step 4 : implémenter**

`crates/rbs-core/src/logs/mod.rs` :

```rust
//! Formateurs de logs du runtime.

mod pretty;

pub use pretty::PrettyFormat;
```

`crates/rbs-core/src/logs/pretty.rs` — points structurants :

- `pub struct PrettyFormat { ansi: bool, horodatage: ChronoLocal }`, `ChronoLocal::new("%H:%M:%S".to_owned())`.
- `new()` délègue à `with_ansi(std::io::stdout().is_terminal())`.
- Un visiteur `ChampsEvenement { message: String, champs: String }` implémentant `tracing::field::Visit` : `record_str` écrit `clé=valeur` sans guillemets, `record_debug` route le champ `message` vers `message` et le reste vers `champs`.
- `format_event` écrit, dans l'ordre : horodatage, deux espaces, niveau padé à 5 **puis** coloré (padder avant de colorer, les séquences ANSI comptant dans la largeur de `{:<5}`), deux espaces, cible padée à 18 puis grisée, deux espaces, message, puis — s'il y en a — deux espaces et le bloc de champs grisé.
- Les champs des spans parents sont lus via `ctx.event_scope()` et `span.extensions().get::<FormattedFields<N>>()`, parcourus de la feuille vers la racine, et concaténés **après** ceux de l'événement.
- Couleurs : `TRACE` `DarkGray`, `DEBUG` `Blue`, `INFO` `Green`, `WARN` `Yellow`, `ERROR` `Red` ; cible et champs `dimmed`. Quand `ansi` est faux, aucun style n'est appliqué.

`crates/rbs-core/src/lib.rs` : ajouter la déclaration du module.

```rust
/// Formateurs de logs du runtime.
pub mod logs;
```

- [ ] **Step 5 : lancer les tests, les voir passer**

Run: `cargo test -p rbs-core logs`
Expected: 5 passed.

- [ ] **Step 6 : écrire l'exemple d'inspection visuelle**

`crates/rbs-core/examples/logs_pretty.rs` : un `main` qui installe un subscriber `PrettyFormat::new()` avec `max_level = TRACE`, puis émet un événement par niveau — dont un avec des champs et un depuis un span portant un `request_id` — pour montrer les cinq couleurs et l'alignement.

- [ ] **Step 7 : vérifier l'ensemble**

Run: `cargo run -p rbs-core --example logs_pretty`
Expected: cinq lignes colorées et alignées ; la sortie est soumise au user pour validation, ce critère étant subjectif.

Run: `cargo clippy --workspace --all-targets -- -D warnings` puis `cargo fmt --all --check`
Expected: aucun warning, aucune divergence de format.

- [ ] **Step 8 : commit**

```bash
git add Cargo.toml crates/rbs-core/Cargo.toml crates/rbs-core/src/lib.rs \
        crates/rbs-core/src/logs crates/rbs-core/examples/logs_pretty.rs
git commit
```

Message : `feat(core): rend les logs lisibles avec un formateur pretty maison`, corps portant le *pourquoi* et un intertitre `Vérifications :` avec les commandes et leur résultat réel.

## Reste hors de portée

Le critère « capture dans les docs » de la tâche dépend du site Docusaurus, qui n'existe pas encore. La case reste donc décochée avec une annotation `PARTIEL`, décision prise avec le user avant le démarrage.
