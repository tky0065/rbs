use super::*;

use std::time::Duration;

use super::super::queue;

#[test]
fn each_status_carries_the_value_the_column_holds() {
    // La requête de réservation compare ces chaînes : les renommer sans toucher à la
    // table ferait un dépilage qui ne trouve jamais rien, et aucun test de compilation
    // ne le verrait.
    assert_eq!(Status::Pending.as_str(), "pending");
    assert_eq!(Status::Running.as_str(), "running");
    assert_eq!(Status::Done.as_str(), "done");
    assert_eq!(Status::Failed.as_str(), "failed");
}

#[test]
fn the_payload_of_a_job_reads_back_into_its_type() {
    let job = Succeeds {
        marque: "ada".to_string(),
    };

    let payload = serde_json::to_value(&job).expect("le job est sérialisable");
    let relu: Succeeds = serde_json::from_value(payload).expect("le payload est relisible");

    assert_eq!(relu.marque, "ada");
}

#[tokio::test]
async fn an_unregistered_kind_is_reported_rather_than_silently_dropped() {
    let error = registry()
        .run(&detached_state(), "inconnu", serde_json::json!({}))
        .await
        .expect_err("un `kind` absent du registre ne peut pas s'exécuter");

    assert!(error.to_string().contains("inconnu"), "{error}");
}

#[test]
fn the_retry_delay_doubles_with_each_attempt() {
    let config = delai(30, 3600);

    assert_eq!(queue::retry_delay(&config, 1), Duration::from_secs(30));
    assert_eq!(queue::retry_delay(&config, 2), Duration::from_secs(60));
    assert_eq!(queue::retry_delay(&config, 5), Duration::from_secs(480));
}

#[test]
fn the_retry_delay_is_capped_and_saturates_rather_than_overflowing() {
    let config = delai(30, 3600);

    assert_eq!(queue::retry_delay(&config, 20), Duration::from_secs(3600));
    // 2^199 déborde un u64 : le calcul doit saturer, pas paniquer.
    assert_eq!(queue::retry_delay(&config, 200), Duration::from_secs(3600));
}

#[test]
fn a_zero_retry_delay_stays_zero() {
    assert_eq!(queue::retry_delay(&delai(0, 3600), 7), Duration::ZERO);
}
