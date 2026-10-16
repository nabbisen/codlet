# RFC process

codlet's design decisions are recorded as RFCs under the workspace `rfcs/`
directory and governed by `rfcs/done/000-rfc-lifecycle-policy.md`.

The directory uses the 5-folder variant (RFC-035 §3.1): `proposed/` (under
review), `accepted/` (review complete; implementation may start), `done/`
(implemented), and `archive/` (withdrawn or superseded). `handoffs/` holds
companion implementation documents; a handoff has no lifecycle state of its own
and inherits that of its governing RFC. The folder an RFC lives in is the source
of truth for its state; each RFC's `Status` field mirrors its folder.

The index at `rfcs/README.md` lists every RFC by state, with the release each
shipped in, and is updated in the same change that moves an RFC. Consult it for
what is proposed, accepted, or implemented at any given time — in particular,
RFC-001 and RFC-002 define the project scope and crate architecture.
