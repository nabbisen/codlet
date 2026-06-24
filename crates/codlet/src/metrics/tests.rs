//! Unit tests for the `metrics` module.
use super::*;

#[test]
fn noop_metrics_is_zero_cost() {
    let m = NoopMetrics;
    // Should compile to nothing; just verify it doesn't panic.
    for _ in 0..1000 {
        m.increment(counter::CODE_ISSUED, None);
        m.increment(counter::SESSION_VALIDATE, Some(Outcome::Miss));
    }
}

#[test]
fn outcome_labels_are_stable() {
    assert_eq!(Outcome::Success.label(), "success");
    assert_eq!(Outcome::Miss.label(), "miss");
    assert_eq!(Outcome::RateLimited.label(), "rate_limited");
    assert_eq!(Outcome::Invalid.label(), "invalid");
    assert_eq!(Outcome::Replay.label(), "replay");
    assert_eq!(Outcome::Error.label(), "error");
}

#[test]
fn capturing_metrics_records_and_drains() {
    let m = CapturingMetrics::new();
    m.increment(counter::CODE_ISSUED, None);
    m.increment(counter::CODE_CLAIM_WON, Some(Outcome::Success));
    m.increment(counter::SESSION_VALIDATE, Some(Outcome::Miss));
    assert_eq!(m.count(counter::CODE_ISSUED), 1);
    assert_eq!(m.count(counter::SESSION_VALIDATE), 1);
    let all = m.drain();
    assert_eq!(all.len(), 3);
    assert!(m.drain().is_empty());
}

#[test]
fn metric_names_contain_no_secret_vocabulary() {
    // Guard: counter names must not contain words that suggest they carry
    // sensitive data. Metric label names are logged and exported.
    let forbidden = [
        "secret",
        "key",
        "hmac",
        "pepper",
        "code_value",
        "subject_id",
    ];
    for (name, _) in [
        (counter::CODE_ISSUED, ()),
        (counter::CODE_REDEEM_ATTEMPT, ()),
        (counter::CODE_CLAIM_WON, ()),
        (counter::CODE_CLAIM_LOST, ()),
        (counter::FORM_TOKEN_CONSUME, ()),
        (counter::SESSION_ISSUED, ()),
        (counter::SESSION_VALIDATE, ()),
        (counter::RATE_LIMIT_BLOCKED, ()),
    ] {
        for word in forbidden {
            assert!(
                !name.contains(word),
                "counter {name:?} contains sensitive word {word:?}"
            );
        }
    }
}
