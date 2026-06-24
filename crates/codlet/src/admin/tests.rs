//! Unit tests for the `admin` module.
use super::*;

#[test]
fn code_meta_is_redeemable_logic() {
    let now = 1_000;
    let base = CodeMeta {
        id: CodeId::new("c1".into()),
        key_version: crate::hashing::KeyVersion::new("v1"),
        purpose: None,
        scope: None,
        grant: None,
        created_at: Some(now - 10),
        expires_at: now + 100,
        used_at: None,
        used_by: None,
        revoked_at: None,
    };
    assert!(base.is_redeemable_at(now));
    assert!(
        !CodeMeta {
            used_at: Some(now),
            ..base.clone()
        }
        .is_redeemable_at(now)
    );
    assert!(
        !CodeMeta {
            revoked_at: Some(now),
            ..base.clone()
        }
        .is_redeemable_at(now)
    );
    assert!(
        !CodeMeta {
            expires_at: now - 1,
            ..base
        }
        .is_redeemable_at(now)
    );
}

#[test]
fn code_list_filter_helpers() {
    let all = CodeListFilter::all();
    assert!(all.scope.is_none() && !all.active_only);
    let scoped = CodeListFilter::active_in_scope(ScopeKey::new("community-1"));
    assert!(scoped.active_only);
    assert_eq!(scoped.scope.unwrap().as_str(), "community-1");
}

#[test]
fn code_stats_total() {
    let s = CodeStats {
        active: 3,
        used: 10,
        revoked: 2,
        expired: 5,
    };
    assert_eq!(s.total(), 20);
}

#[test]
fn code_meta_contains_no_secrets() {
    // Verify the type has no field named plaintext / lookup_key / hmac.
    // This is enforced by the type definition but we assert via Debug.
    let m = CodeMeta {
        id: CodeId::new("c1".into()),
        key_version: crate::hashing::KeyVersion::new("v1"),
        purpose: Some("invite".into()),
        scope: Some("community-1".into()),
        grant: Some("role:member".into()),
        created_at: None,
        expires_at: 9_999_999,
        used_at: None,
        used_by: None,
        revoked_at: None,
    };
    let dbg = format!("{m:?}");
    let forbidden = ["lookup_key", "hmac", "plain_code", "secret", "pepper"];
    for word in forbidden {
        assert!(
            !dbg.to_lowercase().contains(word),
            "CodeMeta debug contains {word:?}: {dbg}"
        );
    }
}
