# Developer Handoff — RFC-055: A Configured Rate Limit With No Key Is a Silent Opt-Out

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-18
- **RFC:** [`../../accepted/055-a-configured-rate-limit-with-no-key-is-a-silent-opt-out.md`](../../accepted/055-a-configured-rate-limit-with-no-key-is-a-silent-opt-out.md)
- **Baseline:** `5c1c2f9`
- **Decision: option (b), fail closed.** Not (a), not (c).
- **Do this before RFC-054.** It is much smaller and it closes a security hole.

## 1. Purpose

A host that supplies a `RateLimitStore` and a `RateLimitPolicy` and then passes
`rate_key: None` currently gets no rate limiting, no error and no audit event.
Make that combination an error instead of a silent pass.

You found this. The reason it matters is the reason you gave: switching the
examples to a real limiter would have changed nothing.

## 2. The decision, precisely

**When `self.rate_limit_policy` is `Some` and `rate_key` is `None`, return an
error before doing any work.** Do not check the limiter, do not touch the store,
do not perform the lookup.

Not option (a) — no signature change, so `rate_key` stays `Option<&RateLimitKey>`.
Not option (c) — an audit line on a control that is not running is the fail-open
shape this project has corrected five times.

Error variant: reuse an existing `RedeemError` if one fits the meaning
"misconfigured, not a client problem". If none does, add one — and name it for
the misconfiguration, not for the rate limit, because the request was never
rate-limited. Whatever you choose, its `public` counterpart must **not** tell
the caller anything about server configuration; map it to the same opaque public
error an internal failure already uses.

If you think a different variant or shape is better, say so in the review
request before building it.

## 3. There are five sites, not three, and one has a different shape

My review of RFC-053 said three. I audited it properly for this handoff and
there are five, spanning `find` and `claim` in `crates/codlet/src/auth/code.rs`:

| Line | Shape | What it gates |
|---|---|---|
| 158 | `if let (Some(key), Some(rl_policy))` | the pre-lookup limit check |
| 196 | same | `record_failure` on invalid format |
| 248 | same | `record_failure` on non-redeemable |
| 313 | same | `record_failure` on a lost claim |
| ~297 | **`if let Some(key) = rate_key { if self.rate_limit_policy.is_some() { … } }`** | `clear_failures` after success |

**The one at ~297 will not be found by grepping for the tuple pattern.** It is
the success path that clears the counter. Miss it and failure counts accumulate
across successful redemptions, which turns a working limiter into one that locks
out legitimate users over time.

Once §2's guard rejects the bad combination at entry, these five can be
simplified — inside the method, a present policy now implies a present key.
Simplify them only if it is a clear improvement; correctness first.

## 4. Non-change scope

- No change to `RateLimitStore`, `RateLimitPolicy`, `RateLimitOutcome`, or
  `RateLimitKey`.
- No change to the unavailable-store policy. It is already explicit and correct.
- No change to `without_rate_limit`, which RFC-053 handled — a host that opts
  out has `rate_limit_policy: None`, so §2's guard must **not** fire for it.
  That is the case to get right: no policy and no key is a legitimate,
  deliberately-deprecated configuration; a policy without a key is the defect.
- **No change to `auth/session.rs:204`.** It matches the shape and is not an
  instance — there `None` *is* the configuration (no idle timeout), and
  `classify_session` already handles it. I verified this; leave it alone.
- No change to the form-token manager. It takes no equivalent optional control.

## 5. Required tests

1. Policy configured, `rate_key: None` → the new error, for both `find` and
   `claim`. **Assert no store access happened** — use a `RateLimitStore` or
   `CodeStore` double that panics or records calls, so the test proves the
   request was rejected before work rather than merely that it failed.
2. `without_rate_limit` (no policy, no key) → still works exactly as before.
   This is the regression that would break hosts and examples.
3. Policy configured with a real key → unchanged behaviour, including that
   `clear_failures` still runs after a successful redemption (the §3 ~297 site).
4. The public error surfaced for case 1 reveals nothing about configuration.

## 6. Required verification

| Check | Expectation |
|---|---|
| `cargo test --workspace` | green; count rises by the tests you add |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | clean |
| `grep -n 'rate_limit_policy' crates/codlet/src/auth/code.rs` | every site accounted for against §3's table |
| each example builds and runs | exit 0 — they pass real keys after RFC-053 |
| `cargo run -p xtask -- release-check` | 5 gates ok |

Re-run the `axum_login_logout` ten-wrong-guesses check from your RFC-053
evidence. It is the only end-to-end proof that the limiter is live, and this
change touches the path it exercises.

## 7. Acceptance criteria

Binding.

1. A configured policy with `rate_key: None` returns an error from both `find`
   and `claim`, before any store access.
2. `without_rate_limit` behaviour is unchanged.
3. All five §3 sites are accounted for, including the nested one at ~297.
4. The public error discloses no configuration detail.
5. `auth/session.rs` and the form-token manager are untouched.
6. Tests for all four §5 cases, including the no-store-access assertion.
7. All blocking CI jobs green; examples build and run.

## 8. Required evidence

`git diff --stat`; the guard quoted; the four tests quoted; the §3 table
reconciled against the final code; the re-run axum check; CI run URL.

## 9. Review request

`.git-exclude/review-request/055-rate-limit-silent-opt-out.md`; result returns
at `.git-exclude/reviewed/055-rate-limit-silent-opt-out.md`.
