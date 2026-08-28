//! Le cache d'un projet réel, joué contre un Redis en conteneur.
//!
//! Les tests du fragment prouvent ses fonctions pures — encodage, motif de balayage,
//! refiltrage du préfixe. Celui-ci fait traverser `Cache::set`, `Cache::get` et
//! `Cache::invalider_prefixe` par un vrai serveur, seul endroit où l'expiration d'une clé
//! peut être attestée autrement que par une horloge simulée.

use assert_cmd::Command;
use tempfile::TempDir;
use testcontainers::GenericImage;
use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::SyncRunner;

mod common;

const IMAGE: (&str, &str) = ("redis", "8-alpine");

#[test]
#[ignore = "démarre Redis et compile un projet Axum + SeaORM complet : plusieurs minutes"]
fn le_cache_d_un_projet_genere_se_joue_contre_un_redis_reel() {
    let redis = GenericImage::new(IMAGE.0, IMAGE.1)
        .with_wait_for(WaitFor::log(LogWaitStrategy::stdout_or_stderr(
            "Ready to accept connections",
        )))
        .start()
        .expect("Redis doit démarrer — Docker est-il lancé ?");

    let port = redis
        .get_host_port_ipv4(6379.tcp())
        .expect("le port de Redis doit être publié");

    let parent = TempDir::new().expect("répertoire temporaire créable");
    let projet = common::projet(parent.path());

    rbs(&projet).args(["add", "redis"]).assert().success();

    // Le conteneur reçoit son port au démarrage : `config/default.toml` ne peut pas le
    // connaître, et c'est la surcharge par l'environnement qui le lui apprend.
    //
    // `-- --ignored` ne lance que les tests serveur du fragment : le squelette n'en porte
    // aucun autre, et ses tests de santé exigeraient une base de données.
    let sortie = Command::new("cargo")
        .current_dir(&projet)
        .env("CARGO_TARGET_DIR", common::cible())
        .env("RBS_CACHE__URL", format!("redis://127.0.0.1:{port}"))
        .args(["test", "--workspace", "--", "--ignored"])
        .output()
        .expect("cargo doit se lancer");

    let journal = format!(
        "{}{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    assert!(
        sortie.status.success(),
        "les tests du cache ont échoué :\n{journal}"
    );

    // `cargo test -- --ignored` sort en 0 même quand il ne filtre **aucun** test : sans
    // les deux lignes qui suivent, un fragment qui cesserait de livrer ses tests serveur
    // laisserait celui-ci au vert sans que rien n'ait joint Redis.
    for test in [
        "le_parcours_complet_se_joue_contre_un_serveur",
        "une_valeur_a_ttl_d_une_seconde_a_disparu_apres_l_attente",
        "un_prefixe_a_metacaractere_n_emporte_que_ce_qu_il_designe",
    ] {
        assert!(
            journal.contains(&format!("test cache::tests::{test} ... ok")),
            "`{test}` n'a pas été exécuté :\n{journal}"
        );
    }
}

/// Le binaire livré, lancé depuis `repertoire`.
fn rbs(repertoire: impl AsRef<std::path::Path>) -> Command {
    let mut commande = Command::cargo_bin("rbs").expect("le binaire rbs doit être compilé");
    commande.current_dir(repertoire);
    commande
}
