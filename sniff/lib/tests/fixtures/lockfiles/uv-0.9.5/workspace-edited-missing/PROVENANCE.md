# Provenance: uv-0.9.5/workspace-edited-missing

Hand-edited from `workspace` (see `../workspace/PROVENANCE.md` for the tool, host, and commands). Date 2026-09-26.
Only `uv.lock` differs from `workspace`.

## Exact edit

Removed `"beta"` from `[manifest].members`, deleted the `[[package]] name = "beta"` table, and deleted
alpha's `dependencies = [{ name = "beta" }]` array and its `[package.metadata] requires-dist` table so no
reference to `beta` remains.

## Expected result

Lockfile omits the current member `packages/beta` (package `beta`): stale lockfile, missing member.
