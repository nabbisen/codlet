//! Unit tests for the `audit` module.
use super::*;
use crate::error::RedemptionFailReason;

#[test]
fn event_keys_are_stable() {
    let events: &[(&str, CodeAuthEvent)] = &[
        (
            "code.issue.succeeded",
            CodeAuthEvent::CodeIssued {
                code_id: CodeId::new("c1".into()),
                purpose: None,
            },
        ),
        (
            "code.redeem.succeeded",
            CodeAuthEvent::CodeRedeemed {
                code_id: CodeId::new("c1".into()),
                subject_id: SubjectId::new("s1".into()),
            },
        ),
        (
            "code.redeem.failed",
            CodeAuthEvent::RedemptionFailed {
                reason: RedemptionFailReason::Expired,
            },
        ),
        (
            "code.revoke.succeeded",
            CodeAuthEvent::CodeRevoked {
                code_id: CodeId::new("c1".into()),
                scope: None,
            },
        ),
        (
            "session.issue.succeeded",
            CodeAuthEvent::SessionIssued {
                session_id: SessionId::new("s1".into()),
                subject_id: SubjectId::new("u1".into()),
            },
        ),
        (
            "session.validate.failed",
            CodeAuthEvent::SessionValidateFailed,
        ),
        (
            "session.revoke.succeeded",
            CodeAuthEvent::SessionRevoked {
                session_id: SessionId::new("s1".into()),
            },
        ),
        (
            "form_token.consume.replay",
            CodeAuthEvent::FormTokenReplay {
                purpose: "logout".into(),
            },
        ),
        (
            "rate_limit.blocked",
            CodeAuthEvent::RateLimitHit {
                key_fingerprint: "fp1".into(),
                purpose: None,
            },
        ),
        (
            "key_provider.missing_version",
            CodeAuthEvent::KeyVersionMissing {
                version: crate::hashing::KeyVersion::new("v0"),
            },
        ),
    ];
    for (expected_key, event) in events {
        assert_eq!(event.key(), *expected_key, "key mismatch for {event:?}");
    }
}

#[test]
fn noop_sink_accepts_all_events() {
    let sink = NoopAuditSink;
    sink.record(CodeAuthEvent::SessionValidateFailed);
    sink.record(CodeAuthEvent::FormTokenReplay {
        purpose: "logout".into(),
    });
}

#[test]
fn collecting_sink_drains() {
    let sink = CollectingAuditSink::new();
    assert!(sink.is_empty());
    sink.record(CodeAuthEvent::SessionValidateFailed);
    sink.record(CodeAuthEvent::SessionRevoked {
        session_id: SessionId::new("s1".into()),
    });
    assert_eq!(sink.len(), 2);
    let drained = sink.drain();
    assert_eq!(drained.len(), 2);
    assert!(sink.is_empty());
    assert_eq!(drained[0].key(), "session.validate.failed");
    assert_eq!(drained[1].key(), "session.revoke.succeeded");
}

#[test]
fn events_contain_no_secrets_by_construction() {
    // Guard: every event variant's Debug output must not contain any of the
    // forbidden content listed in the module docs.
    let forbidden = ["secret", "hmac", "pepper", "cookie", "password"];
    let events = [
        CodeAuthEvent::CodeIssued {
            code_id: CodeId::new("c1".into()),
            purpose: None,
        },
        CodeAuthEvent::RedemptionFailed {
            reason: RedemptionFailReason::AlreadyUsed,
        },
        CodeAuthEvent::RateLimitHit {
            key_fingerprint: "fp".into(),
            purpose: Some("redeem".into()),
        },
    ];
    for ev in &events {
        let dbg = format!("{ev:?}");
        for word in forbidden {
            assert!(
                !dbg.to_lowercase().contains(word),
                "event debug contains forbidden word {word:?}: {dbg}"
            );
        }
    }
}
