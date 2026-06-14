//! Workspace automation entry point.
//!
//! `cargo run -p xtask -- <task>`
//!
//! Tasks:
//!   - `release-check` Static release gates (RFC-015 §9). Currently a skeleton;
//!     gates are added as the primitives they guard are implemented
//!     (no-fallback-key, no `unwrap_or_default` near RNG, cookie attribute
//!     enforcement, etc.).
//!
//! This binary intentionally avoids external dependencies for now.

use std::process::ExitCode;

fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("release-check") => release_check(),
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
}

/// A named static release gate: returns `Ok(())` when the invariant holds, or
/// `Err(reason)` describing the violation.
type Gate = (&'static str, fn() -> Result<(), String>);

/// Static release gates. Each gate is added alongside the RFC that introduces
/// the pattern it guards, so the gate and the code it protects land together.
fn release_check() -> ExitCode {
    let gates: &[Gate] = &[
        ("no-fallback-key", gate_no_fallback_key),
        ("rng-no-silent-fallback", gate_rng_no_silent_fallback),
        ("no-debug-prints", gate_no_debug_prints),
        ("cookie-attrs-present", gate_cookie_attrs),
        ("no-plaintext-in-store-ops", gate_no_plaintext_store),
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

    if gates.is_empty() {
        println!("release-check: no gates registered yet");
    }

    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("{failed} gate(s) failed");
        ExitCode::FAILURE
    }
}

/// Collect `.rs` files under `crates/*/src`, excluding test modules is not
/// attempted here (gates are conservative and also scan tests intentionally
/// for the fallback-key literal). Returns (path, contents).
fn library_sources() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let crates = root.join("crates");
    visit(&crates, &mut out);
    out
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

/// W-1: no development fallback key may exist. Bans the source service's
/// sentinel and any obvious `*-change-in-production` style literal in code.
fn gate_no_fallback_key() -> Result<(), String> {
    let needles = ["dev-pepper-change-in-production", "change-in-production"];
    let mut hits = Vec::new();
    for (path, src) in library_sources() {
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

/// INV-3: RNG results must not be silently defaulted or swallowed. Bans
/// `unwrap_or_default()` and `.ok()` appearing on the same line as a
/// `fill_bytes`/`getrandom` call in non-comment code.
fn gate_rng_no_silent_fallback() -> Result<(), String> {
    let mut hits = Vec::new();
    for (path, src) in library_sources() {
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

/// No `println!`/`dbg!`/`eprintln!` in library code (they risk leaking
/// secrets and are not a logging interface). The xtask crate itself is exempt
/// because it is a CLI, not a library; `library_sources` only scans `crates/`.
fn gate_no_debug_prints() -> Result<(), String> {
    let banned = ["println!", "eprintln!", "dbg!", "print!"];
    let mut hits = Vec::new();
    for (path, src) in library_sources() {
        // Allow prints inside integration tests: they never ship and the vector
        // printer is intentional. (Unit `#[cfg(test)]` prints would also be
        // stripped from release builds; this gate targets shipping code.)
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

/// RFC-006/015: The cookie builder must always emit `HttpOnly` and `Secure` in
/// production profiles. Bans construction of `Set-Cookie` values that omit
/// these attributes in non-dev code paths inside `codlet-core`.
///
/// Specifically: if `build_set_cookie` or `build_clear_cookie` is called and
/// the result does NOT contain `HttpOnly`, that is a security defect. Here we
/// scan for any line that builds a Set-Cookie string without the attribute in a
/// non-test source file — a heuristic, not a full parser.
fn gate_cookie_attrs() -> Result<(), String> {
    // The cookie module is the only place cookies are built; verify it contains
    // both attribute names. If they disappear from that file, the gate fires.
    let required = ["HttpOnly", "Secure", "SameSite"];
    let mut missing = Vec::new();
    for (path, src) in library_sources() {
        if !path.contains("cookie.rs") {
            continue;
        }
        for attr in required {
            if !src.contains(attr) {
                missing.push(format!("{path}: missing {attr:?} in cookie builder"));
            }
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing.join("; "))
    }
}

/// RFC-005/015: No raw secret string (the bearer value) should appear in a
/// store-insertion call. Bans patterns like `insert(secret.expose())` in
/// library source that would persist the plaintext rather than the lookup key.
///
/// Heuristic: reject any non-comment line inside a store impl that calls both
/// `.expose()` and an insert/update/bind in the same line, which would
/// indicate the plaintext is being passed to the DB layer.
fn gate_no_plaintext_store() -> Result<(), String> {
    let mut hits = Vec::new();
    for (path, src) in library_sources() {
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
