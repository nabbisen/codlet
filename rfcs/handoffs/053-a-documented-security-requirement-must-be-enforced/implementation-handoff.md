# Developer Handoff — RFC-053: A Documented Security Requirement Must Be Enforced, Not Described

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-17
- **RFC:** [`../../accepted/053-a-documented-security-requirement-must-be-enforced.md`](../../accepted/053-a-documented-security-requirement-must-be-enforced.md)
- **Baseline:** `3767721`
- **Do after RFC-052 and RFC-051.** No hard dependency; this one is larger and touches docs both of them leave alone.

## 1. Purpose

The published "Secure Configuration Guide" tells readers that 8-symbol codes
are safe without rate limiting. They are not, and the threat model says so.
Fix the claim, move the friction onto the opt-out, enforce the key-length
minimum that two documents assert and no code checks.

**§4 is the security fix. If you get one thing right, get that one.**

## 2. Change scope — indicative, not authoritative

`docs/src/adapter-matrix-and-config.md`, `docs/src/threat-model.md`,
`crates/codlet/src/auth/code.rs`, `crates/codlet/src/hashing.rs`,
`examples/axum_login_logout/`, `examples/sqlite_file/`,
`examples/sqlite_quickstart/`, `CHANGELOG.md`. §8 is binding.

## 3. Read this first

The arithmetic in the existing doc is correct and the conclusion drawn from it
is wrong. 39.6 bits is the cost of guessing **one specific** code; the endpoint
accepts **any** valid code, so the attacker's odds scale with how many are
outstanding:

P ≈ N × G / 31⁸ — with 10,000 codes outstanding and 10⁶ guesses (~30/s for nine
hours, trivially unthrottled) that is ~1.2%; at 100,000 outstanding and 10⁷
guesses it is a certainty.

Do not restate my numbers without checking them. If you get a different answer,
tell me — I would rather be corrected than have you reproduce my figure.

## 4. Required: the rate-limiting claim

In `docs/src/adapter-matrix-and-config.md`, the "Code policy" section
(~lines 78-96): delete "and does not require rate limiting to be safe — the
entropy is sufficient", and the matching comment in the code sample
("no rate-limit dependency").

Replace with the real reasoning: per-code entropy is not the relevant quantity,
because the attacker guesses against the whole outstanding population, so a
`RateLimitStore` is required for **every** code policy. Keep it to a short
paragraph in the existing voice.

Leave the `six_symbol` paragraph's framing alone — it was never wrong.

Then make `threat-model.md` the single statement of the requirement and have
the configuration guide cite it rather than restate it. Two documents stating
the same security requirement in their own words is how they came to disagree.

## 5. Required: the friction goes where it belongs

Mark `CodeAuth::without_rate_limit` (`auth/code.rs:451`) `#[deprecated]`, with
a note naming the threat model — the same mechanism `CodePolicy::six_symbol`
already uses, and RFC-003's established pattern in this codebase.

Do **not** remove it. Tests and single-process examples legitimately opt out;
removing it pushes hosts into hand-rolling a no-op `RateLimitStore`, which is
the same risk with none of the visibility.

The workspace builds with `-D warnings`, so every remaining call site needs
`#[allow(deprecated)]`. After §6 those should be only the three tests
(`rfc_047_code_lookup_reasons.rs:58`, `rfc_013_acceptance.rs:47` and `:433`).
**If you end up adding `#[allow(deprecated)]` to an example, you have skipped
§6** — the annotation count is the check on whether §6 was actually done.

## 6. Required: the examples must stop teaching the opt-out

`examples/axum_login_logout/src/main.rs:95`, `examples/sqlite_file/src/main.rs:76`,
and `examples/sqlite_quickstart/src/main.rs:52` all call
`without_rate_limit`. Examples are the most-copied code we publish.

Switch all three to `CodeAuth::new` with `MemRateLimitStore`
(`crates/codlet/src/mem.rs:15`) and a sensible `RateLimitPolicy`. No new
dependency.

`MemRateLimitStore` is in-process, so for the multi-instance examples add one
comment line saying a shared backend is needed in production and pointing at
the adapter matrix. Do not let that comment grow into a tutorial.

If any example genuinely cannot take a rate limiter, keep the opt-out there,
annotate it, comment that it is not a production configuration, and **say which
one and why** in the review request.

## 7. Required: enforce 32 bytes, and the two minor doc items

### 7.1 Key length

`StaticKeyProvider::new` (`hashing.rs:179`) currently rejects only an empty
`active_key`. Reject any key under 32 bytes — the active key **and** every
entry in `previous`. The existing `KeyError::InvalidKeyMaterial` is the right
error if its message can state the requirement; if it cannot, say so rather
than inventing a variant unilaterally.

A test for each position: short active key, short previous key, and a 32-byte
key accepted. `threat-model.md:97` changes from "≥ 16 bytes" to 32.

State the minimum in the `KeyProvider` trait docs (`hashing.rs:140`) as a
contract for custom implementations. We are not enforcing it there —
`SecretHasher::new` is infallible and RFC-053 §4.3 decided not to change that
— so the docs are the whole mechanism for custom providers. Word it as a
requirement, not a suggestion.

### 7.2 Two corrections carried from the sweep

- `adapter-matrix-and-config.md:24` says D1 stores "timestamps as REAL (f64)".
  The migration declares every timestamp `INTEGER`
  (`crates/codlet-worker/migrations/0001_initial.sql:17`). The `f64` is real
  but it is the D1 bridge's row type (`d1/session.rs:41`,
  `d1/code.rs:49`), cast to `u64` on read. Correct the row to say the columns
  are `INTEGER` and the binding goes through `f64`, which is exact for Unix
  seconds. One clause, not a paragraph.
- `docs/src/introduction.md` still calls tutorials and API references
  "forthcoming". Reword to match what exists.

## 8. Acceptance criteria

Binding; §2 yields to these.

1. No document claims any code policy is safe without rate limiting.
2. `threat-model.md` and `adapter-matrix-and-config.md` agree, with the threat
   model as the single statement.
3. `without_rate_limit` is `#[deprecated]`; workspace builds under
   `-D warnings`; the only `#[allow(deprecated)]` sites are the three tests.
4. All three examples use a real `RateLimitStore`, or the exception is named
   and justified.
5. `StaticKeyProvider::new` rejects keys under 32 bytes in both positions, with
   a test for each.
6. `threat-model.md` says 32; the `KeyProvider` trait docs state the minimum.
7. §7.2's two corrections applied.
8. All blocking CI jobs green; examples still build and run.

## 9. Required verification

| Check | Expectation |
|---|---|
| `grep -rn 'does not require rate limiting' docs/` | no hits |
| `grep -rn 'no rate-limit dependency' docs/` | no hits |
| `grep -rn '16 bytes' docs/src/threat-model.md` | no hits |
| `grep -rn 'REAL' docs/src/adapter-matrix-and-config.md` | no hits, or only in a corrected clause |
| `grep -rn 'allow(deprecated)' crates/ examples/` | three hits, all in tests |
| `grep -rn 'without_rate_limit' examples/` | no hits (or the justified exception) |
| `cargo test --workspace` | green |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | clean |
| each example builds | `cargo build` in each example dir |

## 10. Prohibited shortcuts

- Do not soften §4 into "rate limiting is recommended". The threat model says
  required; that is the decision.
- Do not remove `without_rate_limit`.
- Do not add validation to `SecretHasher::new` — RFC-053 §4.3.
- Do not pick 16 bytes because it is the laxer of the two documented numbers.
- Do not silence the new deprecation warning with a crate-level
  `#![allow(deprecated)]`. Per-site or nothing.

## 11. Required evidence

`git diff --stat`; each §9 check with exit status; the rewritten rate-limiting
paragraph quoted in full so I can read it without a checkout; the three key-length
tests quoted; one example's diff; CI run URL.

If you recompute §3's arithmetic and disagree with me, put that in the review
request before doing the doc rewrite.

## 12. Review request

`.git-exclude/review-request/053-enforce-documented-security-requirements.md`;
result returns at
`.git-exclude/reviewed/053-enforce-documented-security-requirements.md`.
