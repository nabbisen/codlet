# Follow-Up Handoff — RFC-055: A Misconfiguration Is Not a Transient Failure

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-18
- **Governing RFCs:** [`../../done/055-…`](../../done/055-a-configured-rate-limit-with-no-key-is-a-silent-opt-out.md), [`../../done/053-…`](../../done/053-a-documented-security-requirement-must-be-enforced.md)
- **Review results:** `.git-exclude/reviewed/055-rate-limit-silent-opt-out.md` §2; `.git-exclude/reviewed/053-followup-public-minimum.md` §2
- **Size:** small. Two unrelated items, bundled so you get one handoff.

## 1. Item 1 — a dedicated error variant

You asked whether `RedeemError::Internal` was the right variant for the RFC-055
guard. It is not, and the reason is firmer than the one you gave.

`PublicRedemptionError::TemporarilyUnavailable`'s own doc
(`crates/codlet/src/error.rs:130`) says "A **transient** problem prevented the
check", and its `Display` is "service temporarily unavailable". A missing
`rate_key` is permanent until code changes. So today's mapping makes that doc
comment false and invites a retry that cannot succeed — on top of an operator
being unable to tell a misconfiguration from a store outage without reading
`cause`.

The tell was in your own diff: you had to **widen** `Internal`'s doc to admit
configuration errors. A variant whose meaning must be stretched is the wrong
variant.

Three parts, all required:

1. **Add a variant** — `RedeemError::Misconfigured { cause, public }`, or a
   better name you can defend — and use it in `require_rate_key_if_limited`.
2. **Keep the public mapping as `TemporarilyUnavailable`.** From the end
   user's side the service genuinely is unavailable, and it discloses nothing.
   **Do not add a public variant**; that surface is rendered to end users and
   does not need to distinguish causes.
3. **Revert `Internal`'s doc comment** to its original meaning ("a transient
   store or key failure"), and **amend `TemporarilyUnavailable`'s** so it no
   longer asserts the cause is necessarily transient.

Part 3 matters as much as part 1. Leaving that line is another comment that is
false about the code beneath it, which is the defect class this week has been
about.

`RedeemError::public()` matches on every variant, so the compiler will find the
sites. Check whether anything matches `RedeemError` exhaustively outside the
crate's own tests, and say so if adding a variant is a breaking change for a
host — it is acceptable, but it should be named in the CHANGELOG.

The RFC-055 tests assert `RedeemError::Internal`; update them to the new
variant without weakening what they check. The no-store-access assertions and
the `public()` disclosure test must survive unchanged in substance.

## 2. Item 2 — delete the dead callback example

`examples/sqlite_quickstart/src/main.rs`'s `callback_flow_example` is
`#[allow(dead_code)]`, nothing calls it, and it demonstrates
`redeem_with_callback` — deprecated, experimental, and carrying a documented
interim state where a claim is won with no subject yet.

**Delete the function**, and the `#[allow(deprecated)]` and `#[allow(dead_code)]`
with it.

You were right that it collides with RFC-013 §10.4 item 5, and right not to
touch the RFC. **I have amended RFC-013** — the checklist item now claims one
supported path rather than two, with a dated note explaining why. You do not
need to edit any RFC.

Afterwards `grep -rn 'allow(deprecated)' examples/` must return nothing.

**Do not remove `redeem_with_callback` itself.** Whether it survives the M7
freeze is recorded for the M7 API audit; the candidate decision is removal, but
that is a separate call.

## 3. Non-change scope

- No change to the RFC-055 guard's placement or condition — only its variant.
- No change to `MIN_HMAC_KEY_BYTES` or anything else from the RFC-053
  follow-up, which is approved as committed.
- No change to `redeem_with_callback`, including its missing `record_failure`
  on a lost claim. Recorded, deprecated, not yours.
- No change to the other four examples.

## 4. Required verification

| Check | Expectation |
|---|---|
| `cargo test --workspace` | green; 250 or higher, no test lost |
| `grep -rn 'allow(deprecated)' examples/` | no hits |
| `grep -rn 'callback_flow_example' examples/` | no hits |
| `grep -n 'transient' crates/codlet/src/error.rs crates/codlet/src/auth/error.rs` | reconciled against §1.3 |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | clean |
| `cargo build` for all five examples | exit 0 |
| `sqlite_quickstart` runs | exit 0, unchanged behaviour |

## 5. Acceptance criteria

1. The guard returns a dedicated misconfiguration variant, not `Internal`.
2. The public mapping is still `TemporarilyUnavailable`; no public variant
   added.
3. `Internal`'s doc is restored and `TemporarilyUnavailable`'s no longer claims
   the cause is transient.
4. RFC-055's five tests still pass, asserting the new variant, with the
   no-store-access and disclosure checks intact.
5. `callback_flow_example` is gone; no `allow(deprecated)` in `examples/`.
6. `redeem_with_callback` itself is unchanged.
7. CHANGELOG notes the new variant, and whether it is breaking for callers
   matching exhaustively.
8. All blocking CI jobs green.

## 6. Review request

`.git-exclude/review-request/055-followup-misconfigured-variant.md`; result
returns at `.git-exclude/reviewed/055-followup-misconfigured-variant.md`.
