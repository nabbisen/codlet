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
        // ("no-fallback-key",  gate_no_fallback_key),   // RFC-004
        // ("rng-fail-closed",  gate_rng_fail_closed),    // RFC-003/020
        // ("cookie-attrs",     gate_cookie_attributes),  // RFC-006
        // ("never-double-proceed", gate_never_double_proceed), // RFC-007
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
        println!("release-check: no gates registered yet (Phase 0 skeleton)");
    }

    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("{failed} gate(s) failed");
        ExitCode::FAILURE
    }
}
