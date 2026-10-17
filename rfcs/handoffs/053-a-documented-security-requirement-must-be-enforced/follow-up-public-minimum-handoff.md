# Follow-Up Handoff — RFC-053: Publish the Key-Length Minimum

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-18
- **Governing RFC:** [`../../done/053-a-documented-security-requirement-must-be-enforced.md`](../../done/053-a-documented-security-requirement-must-be-enforced.md)
- **Review result:** `.git-exclude/reviewed/053-enforce-documented-security-requirements.md` §7
- **Size:** small. Two items.

## 1. Purpose

RFC-053 is approved and merged. Two loose ends, both arising from its own §4.3
reasoning.

## 2. Item 1: make the minimum public

`MIN_HMAC_KEY_BYTES` is a private const in `crates/codlet/src/hashing.rs:172`.
Make it `pub` and re-export it wherever `hashing`'s other public items are
re-exported, so it is reachable as `codlet::hashing::MIN_HMAC_KEY_BYTES`.

Why: the `KeyProvider` trait docs now state "every key must be ≥ 32 bytes" as a
contract for custom implementations, and RFC-053 §4.3 deliberately left those
implementations unenforced — the docs are their only safeguard. A custom
implementor who can write `key.len() >= codlet::hashing::MIN_HMAC_KEY_BYTES`
has a contract they can check; one who reads "32" in prose has a number to
hardcode and to get wrong later.

Its rustdoc should say what it is for: the minimum `StaticKeyProvider` enforces
and the one custom providers are expected to honour. Reference it from the
`KeyProvider` contract section you added, so the two cannot drift.

Do **not** add a public validation helper function. The constant is the
contract; a helper is a second place for the rule to live.

## 3. Item 2: the quickstart still demonstrates a deprecated API

`examples/sqlite_quickstart/src/main.rs:148` calls `redeem_with_callback` under
`#[allow(deprecated)]`. You correctly left it alone as out of scope for
RFC-053 — but it is the same defect class §4.4 just fixed: an example teaching
a path the library tells everyone else not to take.

Switch it to the non-deprecated equivalent. If the callback form has no direct
replacement that fits the example, say so in the review request and leave it —
I would rather know the API has a gap than have the example contorted to hide
one.

Afterwards, `grep -rn 'allow(deprecated)' examples/` should return nothing.

## 4. Non-change scope

- Nothing else from RFC-053. It is approved as committed.
- No change to `StaticKeyProvider::new`'s validation, the error message, or the
  32-byte value itself.
- No change to `SecretHasher::new` — RFC-053 §4.3 settled that.
- Do not revisit the frozen test vectors. Your NUL-padding analysis is correct
  and the comment explaining it should stay exactly as written.

## 5. Required verification

| Check | Expectation |
|---|---|
| `grep -rn 'MIN_HMAC_KEY_BYTES' crates/codlet/src/` | declared `pub`, referenced from the `KeyProvider` contract docs |
| a scratch crate referencing `codlet::hashing::MIN_HMAC_KEY_BYTES` | compiles |
| `grep -rn 'allow(deprecated)' examples/` | no hits, or one justified exception named in the review request |
| `cargo test --workspace` | 242 passed, 0 failed, 3 ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | clean |
| `cargo build` for each example | exit 0 |

## 6. Acceptance criteria

1. `codlet::hashing::MIN_HMAC_KEY_BYTES` is public, documented, and cited from
   the `KeyProvider` contract section.
2. No public validation helper was added.
3. `examples/sqlite_quickstart` uses no deprecated API, or the exception is
   named and justified.
4. Test count unchanged at 242; all blocking CI jobs green.

## 7. Review request

`.git-exclude/review-request/053-followup-public-minimum.md`; result returns at
`.git-exclude/reviewed/053-followup-public-minimum.md`.
