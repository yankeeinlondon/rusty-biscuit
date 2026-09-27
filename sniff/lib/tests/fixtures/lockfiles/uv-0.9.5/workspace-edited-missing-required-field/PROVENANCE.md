# Provenance: uv-0.9.5/workspace-edited-missing-required-field

Hand-edited from `workspace` (see `../workspace/PROVENANCE.md` for the tool, host, and commands). Date 2026-09-26.
Only `uv.lock` differs from `workspace`.

## Exact edit

Deleted the whole `[manifest]` table (header and `members` array).

## Expected result

Membership section absent while the manifest declares a workspace. Caution: a lock with no `[manifest]`
is VALID for a single project (see `single-project`), so the parser can only call this an error by
comparing with the manifest, or it must fall back to the `editable`/`virtual` package sources.

## Coordinator ruling (Phase 1, S3)

uv omits `[manifest]` exactly when the root is the only workspace member (see
`../root-only-workspace`). An absent `[manifest]` therefore means "the locked member set
is the root alone", not "membership data missing". Against this fixture's manifests, the
expected Sniff result is `mismatch` with `missing` = `.tools/hidden`, `packages/alpha`,
`packages/beta`, and `extra` = `[]`. It is not `parse_failed`.

## Superseded (review-1, 2026-09-27)

The Phase 1 ruling above read every absent `[manifest]` as "root alone". That
turned a removed membership record into a complete empty set. Sniff now reads an
absent `[manifest]` as root-only only when the root (`.`) is the document's sole
local package; this document keeps `alpha`, `beta`, `hidden-tool`, and
`local-lib`, so the expected result is `unverifiable` with `no_membership_data`,
`extra` = `[]`, `missing` = `[]`.
