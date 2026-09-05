//! Workspace automation entry point.
//!
//! `cargo run -p xtask -- <task>`
//!
//! Tasks:
//!   - `release-check` Static release gates (RFC-015 §9), scanning
//!     `crates/*/src`.
//!   - `self-test` Proves each gate can fail (RFC-040 §3.2): runs every gate's
//!     check logic against a fixture in `fixtures/` that deliberately violates
//!     the pattern that gate exists to catch, and fails if any gate does not
//!     fail on its own fixture.
//!
//! This binary intentionally avoids external dependencies for now.

use std::process::ExitCode;

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("release-check") => release_check(),
        Some("self-test") => self_test(),
        Some(other) => {
            eprintln!("unknown task: {other}");
            print_usage();
            ExitCode::FAILURE
        }
        None => {
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!("usage: cargo run -p xtask -- <task>");
    eprintln!("tasks:");
    eprintln!("  release-check   run static release gates (RFC-015)");
    eprintln!("  self-test       prove every gate can fail (RFC-040 §3.2)");
}

/// A named static release gate: returns `Ok(())` when the invariant holds, or
/// `Err(reason)` describing the violation.
type Gate = (&'static str, fn() -> Result<(), String>);

/// A gate's pure pattern-matching logic, operating on an explicit source
/// corpus rather than reading `crates/` itself — this is what makes a gate's
/// check callable against a self-test fixture (RFC-040 §3.2).
type SourcesCheck = fn(&[(String, String)]) -> Result<(), String>;

/// Static release gates. Each gate is added alongside the RFC that introduces
/// the pattern it guards, so the gate and the code it protects land together.
fn release_check() -> ExitCode {
    let gates: &[Gate] = &[
        ("no-fallback-key", gate_no_fallback_key),
        ("rng-no-silent-fallback", gate_rng_no_silent_fallback),
        ("no-debug-prints", gate_no_debug_prints),
        ("no-plaintext-in-store-ops", gate_no_plaintext_store),
        (
            "no-interpolated-sql-values",
            gate_no_interpolated_sql_values,
        ),
    ];

    let mut failed = 0usize;
    for (name, gate) in gates {
        match gate() {
            Ok(()) => println!("gate ok: {name}"),
            Err(why) => {
                eprintln!("gate FAILED: {name}: {why}");
                failed += 1;
            }
        }
    }

    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("{failed} gate(s) failed");
        ExitCode::FAILURE
    }
}

/// RFC-040 §3.2: each gate's fixture deliberately violates exactly the
/// pattern that gate exists to catch. A gate that passes its own violation
/// fixture cannot actually protect anything — this converts "observed
/// failing" (RFC-036 §3.4, previously a manual trial) into a standing,
/// automated property.
struct SelfTestCase {
    gate_name: &'static str,
    check: SourcesCheck,
    /// Filename under `xtask/fixtures/`.
    fixture_file: &'static str,
}

fn self_test() -> ExitCode {
    let cases: &[SelfTestCase] = &[
        SelfTestCase {
            gate_name: "no-fallback-key",
            check: check_no_fallback_key,
            fixture_file: "no_fallback_key.rs",
        },
        SelfTestCase {
            gate_name: "rng-no-silent-fallback",
            check: check_rng_no_silent_fallback,
            fixture_file: "rng_no_silent_fallback.rs",
        },
        SelfTestCase {
            gate_name: "no-debug-prints",
            check: check_no_debug_prints,
            fixture_file: "no_debug_prints.rs",
        },
        SelfTestCase {
            gate_name: "no-plaintext-in-store-ops",
            check: check_no_plaintext_store,
            fixture_file: "no_plaintext_in_store_ops.rs",
        },
        SelfTestCase {
            gate_name: "no-interpolated-sql-values",
            check: check_no_interpolated_sql_values,
            fixture_file: "no_interpolated_sql_values.rs",
        },
    ];

    let fixtures_dir = fixtures_dir();
    let mut failed: Vec<&str> = Vec::new();
    for case in cases {
        let path = fixtures_dir.join(case.fixture_file);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "self-test FAILED: gate {}: could not read fixture {}: {e}",
                    case.gate_name,
                    path.display()
                );
                failed.push(case.gate_name);
                continue;
            }
        };
        let sources = [(path.display().to_string(), content)];
        match (case.check)(&sources) {
            Ok(()) => {
                eprintln!(
                    "self-test FAILED: gate {} did not fail against its own violation \
                     fixture — it cannot detect the pattern it exists to catch",
                    case.gate_name
                );
                failed.push(case.gate_name);
            }
            Err(reason) => {
                println!(
                    "self-test ok: gate {} correctly failed on its fixture: {reason}",
                    case.gate_name
                );
            }
        }
    }

    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "self-test: {} gate(s) failed to catch their own fixture: {}",
            failed.len(),
            failed.join(", ")
        );
        ExitCode::FAILURE
    }
}

fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Collect `.rs` files under `crates/*/src`. Returns `(path, contents)`.
///
/// RFC-040 §3.1: a gate must fail when it cannot perform its check. Returning
/// an empty vector here let all four gates in `release_check` report `Ok(())`
/// simultaneously and silently if the corpus ever went missing (the `core-deps`
/// failure mode, RFC-036 §3.5, previously unfixed for `xtask`) — so an empty
/// result is now an error, not a value the caller could mistake for "checked,
/// no violations found".
fn library_sources() -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let crates = root.join("crates");
    visit(&crates, &mut out);
    require_nonempty(out, &crates.display().to_string())
}

/// Absence of evidence is never reported as evidence of absence (RFC-040
/// §3.1): a gate given zero sources to inspect must not conclude "no
/// violations found".
fn require_nonempty(
    sources: Vec<(String, String)>,
    corpus_desc: &str,
) -> Result<Vec<(String, String)>, String> {
    if sources.is_empty() {
        Err(format!(
            "no source files found under {corpus_desc}; a gate cannot verify \
             anything against an empty corpus"
        ))
    } else {
        Ok(sources)
    }
}

/// Each gate asserts its corpus covers the crate it claims to guard, not just
/// that the corpus is non-empty (RFC-040 §3.1) — a corpus that silently
/// dropped `codlet` but still contained other crates would otherwise pass
/// `require_nonempty` while checking nothing relevant.
fn assert_covers(sources: &[(String, String)], must_contain: &str) -> Result<(), String> {
    if sources.iter().any(|(path, _)| path.contains(must_contain)) {
        Ok(())
    } else {
        Err(format!(
            "corpus does not include {must_contain:?} — cannot verify the \
             invariant this gate guards"
        ))
    }
}

fn visit(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip target/ if it ever appears under a crate.
            if path.file_name().map(|n| n == "target").unwrap_or(false) {
                continue;
            }
            visit(&path, out);
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            if let Ok(s) = std::fs::read_to_string(&path) {
                out.push((path.display().to_string(), s));
            }
        }
    }
}

/// Lines that are pure comments or doc comments — gates ignore these so that
/// describing a banned pattern in prose does not trip the gate.
fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

// ── W-1 / no-fallback-key ─────────────────────────────────────────────────

/// W-1: no development fallback key may exist. Bans the source service's
/// sentinel and any obvious `*-change-in-production` style literal in code.
fn gate_no_fallback_key() -> Result<(), String> {
    let sources = library_sources()?;
    assert_covers(&sources, "crates/codlet/src/")?;
    check_no_fallback_key(&sources)
}

fn check_no_fallback_key(sources: &[(String, String)]) -> Result<(), String> {
    let needles = ["dev-pepper-change-in-production", "change-in-production"];
    let mut hits = Vec::new();
    for (path, src) in sources {
        for (i, line) in src.lines().enumerate() {
            if is_comment(line) {
                continue;
            }
            for n in needles {
                if line.contains(n) {
                    hits.push(format!("{path}:{}: contains {n:?}", i + 1));
                }
            }
        }
    }
    if hits.is_empty() {
        Ok(())
    } else {
        Err(hits.join("; "))
    }
}

// ── INV-3 / rng-no-silent-fallback ─────────────────────────────────────────

/// INV-3: RNG results must not be silently defaulted or swallowed. Bans
/// `unwrap_or_default()` and `.ok()` appearing on the same line as a
/// `fill_bytes`/`getrandom` call in non-comment code.
fn gate_rng_no_silent_fallback() -> Result<(), String> {
    let sources = library_sources()?;
    assert_covers(&sources, "crates/codlet/src/")?;
    check_rng_no_silent_fallback(&sources)
}

fn check_rng_no_silent_fallback(sources: &[(String, String)]) -> Result<(), String> {
    let mut hits = Vec::new();
    for (path, src) in sources {
        for (i, line) in src.lines().enumerate() {
            if is_comment(line) {
                continue;
            }
            let rng_call = line.contains("fill_bytes") || line.contains("getrandom");
            if rng_call && (line.contains("unwrap_or_default") || line.contains(".ok()")) {
                hits.push(format!("{path}:{}: RNG result defaulted/swallowed", i + 1));
            }
        }
    }
    if hits.is_empty() {
        Ok(())
    } else {
        Err(hits.join("; "))
    }
}

// ── no-debug-prints ─────────────────────────────────────────────────────────

/// No `println!`/`dbg!`/`eprintln!` in library code (they risk leaking
/// secrets and are not a logging interface). The xtask crate itself is exempt
/// because it is a CLI, not a library; `library_sources` only scans `crates/`.
fn gate_no_debug_prints() -> Result<(), String> {
    let sources = library_sources()?;
    assert_covers(&sources, "crates/codlet/src/")?;
    check_no_debug_prints(&sources)
}

fn check_no_debug_prints(sources: &[(String, String)]) -> Result<(), String> {
    let banned = ["println!", "eprintln!", "dbg!", "print!"];
    let mut hits = Vec::new();
    for (path, src) in sources {
        // Allow prints inside integration tests and example binaries:
        // tests never ship; example binaries are demonstration programs
        // that intentionally produce terminal output.
        if path.contains("/tests/") {
            continue;
        }
        for (i, line) in src.lines().enumerate() {
            if is_comment(line) {
                continue;
            }
            for b in banned {
                if line.contains(b) {
                    hits.push(format!("{path}:{}: contains {b}", i + 1));
                }
            }
        }
    }
    if hits.is_empty() {
        Ok(())
    } else {
        Err(hits.join("; "))
    }
}

// ── RFC-005/015 / no-plaintext-in-store-ops ─────────────────────────────────

/// RFC-005/015: No raw secret string (the bearer value) should appear in a
/// store-insertion call. Bans patterns like `insert(secret.expose())` in
/// library source that would persist the plaintext rather than the lookup key.
///
/// Heuristic: reject any non-comment line inside a store impl that calls both
/// `.expose()` and an insert/update/bind in the same line, which would
/// indicate the plaintext is being passed to the DB layer.
fn gate_no_plaintext_store() -> Result<(), String> {
    let sources = library_sources()?;
    assert_covers(&sources, "crates/codlet/src/")?;
    check_no_plaintext_store(&sources)
}

fn check_no_plaintext_store(sources: &[(String, String)]) -> Result<(), String> {
    let mut hits = Vec::new();
    for (path, src) in sources {
        if path.contains("/tests/") {
            continue;
        }
        for (i, line) in src.lines().enumerate() {
            if is_comment(line) {
                continue;
            }
            // Pattern: `.expose()` used directly in a bind/insert/execute call.
            if line.contains(".expose()") && (line.contains(".bind(") || line.contains("INSERT")) {
                hits.push(format!("{path}:{}: expose() in store call", i + 1));
            }
        }
    }
    if hits.is_empty() {
        Ok(())
    } else {
        Err(hits.join("; "))
    }
}

// ── RFC-048 / no-interpolated-sql-values ─────────────────────────────────────

/// RFC-048: a value must never be interpolated into a SQL string via
/// `format!` — it must be bound as a parameter. `format!("{:?}")` on a `&str`
/// performs Rust escaping, not SQL escaping, which is exactly how a critical
/// SQL injection reached `claim_code`'s `purpose`/`scope` handling.
///
/// Heuristic: flags a line matching `<KEYWORD> <column> = {value}`, where
/// `KEYWORD` is one of SQL's `WHERE`/`AND`/`OR`/`SET` and `{value}` is a
/// `format!` placeholder sitting directly after the `=` — the shape of an
/// interpolated comparison value, which is where a bound parameter belongs
/// instead. The keyword requirement is what keeps this scoped to SQL clauses
/// rather than firing on any string containing `word = {value}`: an earlier
/// draft without it also matched `Set-Cookie` header assembly
/// (`"{}={}; Max-Age={}"`, `cookie.rs`) and `format!` diagnostic messages
/// (`"changed={changed} rows for id={id}"`, several `StoreError` messages) —
/// neither is SQL, and both were caught by running this gate against the
/// real tree before trusting the design (RFC-048 §2).
///
/// Checked against the four legitimate interpolations this project actually
/// uses (RFC-048 §4.2) rather than assumed safe by construction:
///
/// - `{t}` (a configured table name) never follows `= `, so it never matches;
/// - `LIMIT {n}` and `WHERE {}`/`{where_clause}` never follow `= ` either;
/// - PostgreSQL's `${param_idx}` numbered placeholder *does* follow `= `, but
///   the `$` sits between `=` and `{`, so the whitespace-skipping scan below
///   lands on `$`, not `{`, and does not match — not a special-cased
///   exclusion, a consequence of the literal shape of the two patterns.
fn gate_no_interpolated_sql_values() -> Result<(), String> {
    let sources = library_sources()?;
    assert_covers(&sources, "crates/codlet-sqlx/src/")?;
    assert_covers(&sources, "crates/codlet-worker/src/")?;
    check_no_interpolated_sql_values(&sources)
}

fn check_no_interpolated_sql_values(sources: &[(String, String)]) -> Result<(), String> {
    let mut hits = Vec::new();
    for (path, src) in sources {
        if path.contains("/tests/") {
            continue;
        }
        for (i, line) in src.lines().enumerate() {
            if is_comment(line) {
                continue;
            }
            if has_interpolated_sql_clause_value(line) {
                hits.push(format!(
                    "{path}:{}: value interpolated into a SQL clause after `=` \
                     — bind it as a parameter instead",
                    i + 1
                ));
            }
        }
    }
    if hits.is_empty() {
        Ok(())
    } else {
        Err(hits.join("; "))
    }
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_sql_clause_keyword(word: &str) -> bool {
    matches!(
        word.to_ascii_uppercase().as_str(),
        "AND" | "OR" | "WHERE" | "SET"
    )
}

/// True if `line` contains `<KEYWORD> <column> = {value}`: a SQL clause
/// keyword, then an identifier, then `=` immediately followed (skipping
/// spaces only) by `{` — an interpolated `format!` value where a bound
/// parameter belongs.
fn has_interpolated_sql_clause_value(line: &str) -> bool {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'=' {
            continue;
        }
        // The interpolated placeholder must immediately follow (skipping
        // only spaces) the `=`.
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] == b' ' {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'{' {
            continue;
        }
        // Walk back over the column identifier immediately before `=`.
        let mut k = i;
        while k > 0 && bytes[k - 1] == b' ' {
            k -= 1;
        }
        let ident_end = k;
        while k > 0 && is_ident_byte(bytes[k - 1]) {
            k -= 1;
        }
        if k == ident_end {
            // No identifier immediately before `=` — not a `column = value`
            // shape (e.g. cookie.rs's `"{}={}…"`).
            continue;
        }
        // Walk back over whitespace before the identifier, then read the
        // preceding word and check it is a SQL clause keyword.
        let mut m = k;
        while m > 0 && bytes[m - 1] == b' ' {
            m -= 1;
        }
        let word_end = m;
        while m > 0 && is_ident_byte(bytes[m - 1]) {
            m -= 1;
        }
        if is_sql_clause_keyword(&line[m..word_end]) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests;
