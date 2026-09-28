# Provenance: cargo-1.98.1/workspace-edited-missing

Hand-edited from `workspace` (see `../workspace/PROVENANCE.md` for the tool, host, and commands). Date 2026-09-26.
Only `Cargo.lock` differs from `workspace`.

## Exact edit

Deleted the `[[package]] name = "itoa" version = "0.1.0"` table (member `crates/beta`) and the `"itoa 0.1.0"` entry from alpha's `dependencies`.

## Expected result

Lockfile omits the current member `crates/beta` (crate `itoa` 0.1.0): stale lockfile, missing member. Note the registry `itoa 1.0.18` remains, so matching by name alone would wrongly find it.
