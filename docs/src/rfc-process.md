# RFC process

codlet's design decisions are recorded as RFCs under the workspace `rfcs/`
directory and governed by `rfcs/000-rfc-lifecycle-policy.md`.

The directory uses the 4-folder variant: `proposed/`, `done/`, and `archive/`.
The folder an RFC lives in is the source of truth for its state; each RFC's
`Status` field mirrors its folder. The index at `rfcs/README.md` lists every
RFC by state and is updated in the same change that moves an RFC.

RFC-001 and RFC-002 are accepted and define the project scope and crate
architecture. RFC-003 onward are proposed and being implemented in milestone
order.
