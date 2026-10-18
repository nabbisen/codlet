# Follow-Up Handoff — RFC-058: An Unset Version Variable Must Fail In CI

- **From:** architect (high-capability model)
- **To:** dev team (mid-capability model)
- **Date:** 2026-10-19
- **Governing RFC:** [`../../done/058-the-postgresql-adapter-is-tested-only-on-an-eol-server.md`](../../done/058-the-postgresql-adapter-is-tested-only-on-an-eol-server.md)
- **Review result:** `.git-exclude/reviewed/058-postgres-version-range.md` §3 and §4
- **Baseline:** `7b08aa7`
- **Size:** two lines of logic and a flag removal.

## 1. Why

RFC-058 is approved and merged. One hole survives, and it is the same shape as
the defect the RFC closed: `requested_pg_major()` defaults to
`DEFAULT_PG_MAJOR` when the variable is unset, so if
`env: CODLET_TEST_PG_MAJOR: ${{ matrix.pg }}` were ever dropped from the
workflow, **both matrix legs would request 15, pull `15-alpine`, assert
15 == 15, and pass** — in jobs still named "on PostgreSQL 15" and "on
PostgreSQL 18".

The version assertion cannot catch it, because the image and the expectation
both come from the same defaulted value. It closes "image ≠ request"; this
closes "request ≠ intent".

## 2. The change

Keep the default for local runs; make an unset variable fatal in CI:



Word the panic so it says what to do: the variable must be set per matrix leg,
and the default exists only for local runs.

**Confirm GitHub Actions sets `CI=true`** rather than taking my word. The entire
value of this change is that it fires, so a wrong environment-variable name
would make it decorative — which is worse than not doing it, because it would
look like a guard.

If `CI` turns out to be unreliable, an explicit marker the workflow sets itself
is just as good. Say which you used.

## 3. Then drop `--show-output`

Both PostgreSQL jobs pass `-- --show-output` so the version `println!` survives
in the log. Once §2 is in, the machine checks what that line existed for a human
to check, and the flag's cost — the whole job's passing output in the log — is
no longer worth paying. Reading CI logs is how several real findings were made;
keep them readable.

**Order matters: do §2 first.** Removing the flag before the guard exists would
make the hole worse, since the printed line is currently the only signal.

Keep the named test `postgres_server_major_matches_requested`. Its name in the
output is the human-legible signal and costs nothing.

## 4. Demonstrate it, as you did last time

Delete the `env:` mapping from one job on a branch, push, and confirm that
leg now **fails** rather than silently testing 15. Then restore it. Quote the
failure.

That is the same trial you ran for the tag mismatch, against the failure mode
that one could not reach. A pull request is pre-authorised.

## 5. Non-change scope

- No change to `DEFAULT_PG_MAJOR`, the 15–18 range, the tag format, or
  `assert_server_major`.
- No change to the matrix values or the job names.
- No change to the documentation — it is approved as written.
- `std::mem::forget` on the containers stays as it is; it is parked on RFC-057's
  open Ryuk question and needs Docker to settle.

## 6. Acceptance criteria

1. An unset `CODLET_TEST_PG_MAJOR` fails in CI and still defaults locally.
2. The environment marker used is confirmed, not assumed, and named in the
   review request.
3. `--show-output` is gone from both PostgreSQL job commands.
4. The §4 trial was run and its failure output is quoted.
5. `cargo test -p codlet-sqlx --no-default-features --features postgres-test`
   still works locally with no variable set (it will skip on a machine without
   Docker; that is fine).
6. All blocking CI jobs green, both legs of both PostgreSQL jobs.

## 7. Required evidence

The changed match arms quoted; which environment marker and how you confirmed
it; the §4 trial's failure output; the two job commands after the flag removal;
CI run URL showing all four PostgreSQL legs green.

## 8. Review request

`.git-exclude/review-request/058-followup-unset-variable.md`; result returns at
`.git-exclude/reviewed/058-followup-unset-variable.md`.
