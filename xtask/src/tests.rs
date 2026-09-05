//! Unit tests for the `xtask` gate infrastructure (RFC-040 §3.1, §5.1).
use super::*;

#[test]
fn empty_corpus_is_an_error_not_an_empty_pass() {
    // RFC-040 §3.1: absence of evidence is never evidence of absence. Before
    // this fix, an empty corpus silently produced `Ok(vec![])`, which every
    // gate's `hits.is_empty()` check then read as "no violations found" —
    // the exact shape of the `core-deps` fail-open defect (RFC-036 §3.5),
    // reproduced here for `xtask`.
    let result = require_nonempty(Vec::new(), "some/nonexistent/path");
    assert!(
        result.is_err(),
        "an empty corpus must be an error, not an empty Ok(vec![])"
    );
}

#[test]
fn nonempty_corpus_passes_through_unchanged() {
    let sources = vec![("a.rs".to_string(), "fn main() {}".to_string())];
    let result = require_nonempty(sources.clone(), "irrelevant");
    assert_eq!(result, Ok(sources));
}

#[test]
fn assert_covers_fails_when_the_claimed_crate_is_absent() {
    let sources = vec![("crates/codlet-sqlx/src/lib.rs".to_string(), String::new())];
    assert!(
        assert_covers(&sources, "crates/codlet/src/").is_err(),
        "a corpus that only contains codlet-sqlx must not be reported as \
         covering codlet — `codlet-sqlx/src/` must not satisfy a \
         `codlet/src/` substring match"
    );
}

#[test]
fn assert_covers_passes_when_the_claimed_crate_is_present() {
    let sources = vec![("crates/codlet/src/lib.rs".to_string(), String::new())];
    assert!(assert_covers(&sources, "crates/codlet/src/").is_ok());
}

#[test]
fn real_library_sources_is_nonempty_and_covers_codlet() {
    // Sanity check against the real corpus, not a fixture: proves the
    // production path (not just the unit-testable helpers) actually holds
    // in this checkout.
    let sources = library_sources().expect("real crates/ corpus must not be empty");
    assert_covers(&sources, "crates/codlet/src/").expect("real crates/ corpus must include codlet");
}

// ── RFC-048 / no-interpolated-sql-values ─────────────────────────────────────

/// RFC-048 §4.2 / handoff §5.1: "the hard requirement is not catching the
/// bug — it is not firing on the legitimate four." Each of these is real
/// production shape, taken verbatim from the adapters.
#[test]
fn does_not_fire_on_the_four_legitimate_interpolations() {
    let legitimate = [
        // {t}: a configured table name, never a host-supplied value.
        ("d1_table.rs", "\"SELECT id FROM {t} WHERE lookup_key = ?\""),
        // ${param_idx}: PostgreSQL's numbered placeholder -- `$` sits
        // between `=` and `{`, so this is a positional marker, not a value.
        (
            "postgres_admin.rs",
            "where_parts.push(format!(\"scope = ${param_idx}\"));",
        ),
        // LIMIT {n}: a typed integer, never following `= `.
        (
            "admin_limit.rs",
            "let limit_clause = filter.limit.map(|n| format!(\"LIMIT {n}\"));",
        ),
        // WHERE {} / {where_clause}: joining pre-built, placeholder-bearing
        // fragments, never following `= `.
        (
            "admin_where.rs",
            "format!(\"WHERE {}\", where_parts.join(\" AND \"))",
        ),
    ];
    for (name, line) in legitimate {
        let sources = vec![(name.to_string(), line.to_string())];
        assert_eq!(
            check_no_interpolated_sql_values(&sources),
            Ok(()),
            "false positive on legitimate interpolation in {name}: {line:?}"
        );
    }
}

/// Two real, non-SQL false positives an earlier draft of this gate's
/// heuristic actually produced, before the SQL-clause-keyword requirement
/// was added (found by running the gate against the real tree, not
/// anticipated in advance) — pinned here so the fix cannot silently regress.
#[test]
fn does_not_fire_on_non_sql_format_strings_shaped_like_the_pattern() {
    let non_sql = [
        // cookie.rs's Set-Cookie header assembly.
        ("cookie.rs", "\"{}={}; Max-Age={}; Path={}; HttpOnly; {}\""),
        // A StoreError diagnostic message, not a query.
        (
            "code.rs",
            "format!(\"claim_code changed {changed} rows for id={id}\")",
        ),
    ];
    for (name, line) in non_sql {
        let sources = vec![(name.to_string(), line.to_string())];
        assert_eq!(
            check_no_interpolated_sql_values(&sources),
            Ok(()),
            "false positive on non-SQL text in {name}: {line:?}"
        );
    }
}

#[test]
fn fires_on_each_real_historical_site() {
    // The exact seven lines this gate found in the unfixed tree (RFC-048),
    // one per file/branch, verified by direct reading to be genuine
    // `claim_code` SQL, not a false positive -- see the RFC-048 review
    // request for the full accounting (7 lines, not the 6 the RFC's prose
    // estimated, all confirmed inside `claim_code`).
    let vulnerable = [
        "                   AND expires_at > ? AND purpose = {p:?} AND scope = {s:?}\"",
        "                   AND expires_at > ? AND purpose = {p:?}\"",
        "                   AND expires_at > ? AND scope = {s:?}\"",
        "            sql.push_str(&format!(\" AND purpose = {p:?}\"));",
        "            sql.push_str(&format!(\" AND scope = {s:?}\"));",
    ];
    for line in vulnerable {
        let sources = vec![("vulnerable.rs".to_string(), line.to_string())];
        assert!(
            check_no_interpolated_sql_values(&sources).is_err(),
            "gate did not fire on known-vulnerable shape: {line:?}"
        );
    }
}
