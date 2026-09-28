# PROVENANCE

Hand-edited from `workspace` (sibling directory `../workspace/`). Only `package-lock.json` was changed; every manifest is identical to the real fixture.

Date: 2026-09-26

## Exact edit

Added two entries under `packages`, matching how npm records a workspace member: `"node_modules/gamma": {"resolved": "packages/gamma", "link": true}` (inserted before `node_modules/hidden-tool`) and `"packages/gamma": {"version": "0.0.0"}` (appended after `packages/beta`).

## Expected result

The lockfile records a member (`packages/gamma`) that the root `workspaces` globs no longer match (no `packages/gamma/package.json` exists): a stale-extra membership drift. Lockfile members: `.tools/hidden`, `packages/alpha`, `packages/beta`, `packages/gamma`; manifest members: `.tools/hidden`, `packages/alpha`, `packages/beta`.
