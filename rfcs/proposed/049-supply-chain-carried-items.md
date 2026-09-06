# RFC-049: Resolving RFC-039's Deferred Questions — Yanked Crates and Duplicate Versions

- **Status:** Proposed
- **Target milestone:** M7 preamble (carried from M5)
- **Primary crate(s):** workspace configuration, `xtask`
- **Source basis:** RFC-039 §8.1, which deferred both questions to "after one release cycle of warnings". Two release cycles have now passed (0.19.0, 0.20.0).

## 1. Summary

Update two yanked dev-tree dependencies. **Keep `bans.multiple-versions` at
`warn`** rather than moving it to `deny`, with the audit that justifies it
recorded. Extend the existing `core-deps` gate to assert that `codlet`'s
published dependency tree carries no duplicate versions.

Auditing this turned up a fact worth stating separately: **`codlet-sqlx` ships
two generations of the crypto stack.** §4.

## 2. Yanked crates — update

`chacha20 0.10.0` and `spin 0.9.8` are yanked. Both have non-yanked successors
(`0.10.2`, `0.9.9`) reachable by `cargo update -p chacha20 -p spin`, verified by
dry run. Neither appears in `codlet`'s published tree — both are dev/test-side,
reached through the SQLx and Workers test dependencies.

Low stakes, trivially fixed, and being on a yanked version is a standing hygiene
signal worth clearing rather than carrying indefinitely.

## 3. `bans.multiple-versions` stays `warn`

RFC-039 §3.4 set it to `warn` because the graph had not been audited, and §8.1
asked for the question to be revisited. **Audited now: 12 duplicate crates
workspace-wide.**

```
block-buffer  cpufeatures  crypto-common  digest  getrandom  hashbrown
matchit  rand  rand_core  r-efi  sha2  windows-sys
```

Moving to `deny` would require twelve `skip` entries on day one. RFC-039 §3.2
is explicit that a policy which fails on introduction gets disabled, and that an
allow-list padded to force a pass is the same thing more slowly. Twelve skips is
that, and it would be a gate that appears to run while asserting almost nothing.

**Decision: keep `warn`, and stop re-asking.** The duplicates are a property of
the dependency ecosystem, not of a decision codlet has made. Record the audit so
the next reviewer inherits the evidence rather than repeating it.

## 4. The finding this audit produced

The check I intended to propose — *the published dependency tree carries no
duplicate versions* — passes for `codlet` and **fails for `codlet-sqlx`**:

| Crate | Duplicates in its published normal tree |
|---|---|
| `codlet` | **none** |
| `codlet-sqlx` | `block-buffer`, `cpufeatures`, `crypto-common`, `digest`, `hashbrown`, `sha2` |

Five of those six are the cryptographic stack, and the cause is upstream:

```
sha2 v0.10.9
└── sqlx-core v0.9.0
    └── sqlx v0.9.0
        └── codlet-sqlx v0.20.0
```

`sqlx` 0.9 depends on the `sha2 0.10` / `digest 0.10` generation; codlet uses
`sha2 0.11` / `digest 0.11`. **A consumer of `codlet-sqlx` therefore compiles
two generations of SHA-2, digest, crypto-common and block-buffer.**

This is not a vulnerability. Both generations are current and maintained, and
codlet's own hashing uses only the newer one — the duplication is compile-time
weight, not a correctness or security defect. But it is worth stating plainly in
a security library's record rather than leaving it to be discovered: a published
crate builds two copies of its cryptographic primitives, and we do not control
the reason.

**Not fixable here.** It resolves when `sqlx` moves to the 0.11 generation.
Pinning codlet down to `sha2 0.10` to match would be the wrong direction.

## 5. Decision — extend `core-deps`, do not add a gate

`core-deps` already asserts a property of `cargo tree -p codlet -e normal`: that
no framework, database, or executor crate has entered. Extend the same gate to
assert that tree also carries **no duplicate versions**.

- It passes today for `codlet` — verified.
- It is scoped to `codlet` only, deliberately. Extending it to `codlet-sqlx`
  would fail on day one for a reason upstream, which is the ignore-list trap.
- It is an extension of an existing gate, not a sixth gate. The gate count is
  itself a cost.

What it protects: `codlet` is the runtime-neutral core, targets wasm with
`opt-level = "s"`, and RFC-002 makes its minimality a principle. A future
dependency change pulling two versions of a crypto primitive into it is exactly
the regression this catches, and nothing catches it today.

## 6. Non-goals

- No change to `deny.toml`'s licences, sources, or advisories sections.
- No attempt to deduplicate `codlet-sqlx` — §4.
- No pinning of `sha2`/`digest` to older generations.
- No sixth `xtask` gate.

## 7. Security considerations

Neither yanked crate is in a published tree. The duplicate crypto generations in
`codlet-sqlx` are a weight and hygiene matter, not a vulnerability — both
generations are maintained, and codlet's own HMAC path uses one of them.

The `core-deps` extension adds detection for a regression class currently
unguarded in the crate that most consumers depend on.

## 8. Alternatives considered

1. **Move to `deny` with twelve skips.** Rejected — §3. It is the
   switched-off-gate pattern RFC-039 §3.2 names.
2. **`skip-tree` on the SQLx and testcontainers subtrees.** Rejected: it silently
   excludes everything beneath them, including future additions, which is worse
   than a warning that is read.
3. **Apply the duplicate check to both published crates.** Rejected — fails on
   day one for an upstream reason (§4).
4. **Do nothing.** Defensible, and the outcome for §3. Rejected for §5 because
   the property that actually matters — `codlet`'s tree staying clean — is
   currently unasserted and would regress silently.

## 9. Acceptance criteria

- `chacha20` and `spin` updated; `cargo deny check advisories` reports no yanked
  warnings.
- `bans.multiple-versions` remains `warn`; `deny.toml`'s comment records the
  audit result and that the question is settled.
- `core-deps` additionally asserts no duplicate versions in `codlet`'s normal
  tree, and is **observed failing** against a deliberately introduced duplicate.
- `codlet-sqlx`'s duplicate crypto generations recorded in the docs, with the
  upstream cause and the condition that would resolve it.
- No change to any published crate's behaviour.
