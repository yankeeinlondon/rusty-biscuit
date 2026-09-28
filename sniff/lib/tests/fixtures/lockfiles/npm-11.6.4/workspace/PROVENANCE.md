# PROVENANCE

- Tool: npm 11.6.4 (installed; Node.js v22.20.0)
- Lockfile: `package-lock.json`, `lockfileVersion: 3`
- Host: macOS 27.2 (build 26B5091g), `Darwin 27.2.0 arm64` (Apple Silicon); Node.js v22.20.0 unless stated
- Date: 2026-09-26
- Generated in scratch `/tmp/lockfile-fixtures/npm-11.6.4/workspace/`, outside any repository

## Setup

The manifests were written by hand with a shell script before the tool ran: root `package.json` (name `fixture-root`), `packages/alpha/package.json` (`alpha`), `packages/beta/package.json` (`@fixture/beta`), `.tools/hidden/package.json` (`hidden-tool`), `local-lib/package.json` (`local-lib`, not a member). The root `package.json` declares `"workspaces": ["packages/*", ".tools/hidden"]` and depends on `local-lib` through `file:./local-lib`; `alpha` depends on `@fixture/beta` at `0.0.0` (satisfied by the workspace). No registry dependencies.

## Commands (in order)

1. `npm install --no-audit --no-fund`

## Trimmed

- `node_modules/` (root)

## Expected membership

Manifest-declared members (root `package.json` `workspaces`: `packages/*`, `.tools/hidden`):

- `.tools/hidden`
- `packages/alpha`
- `packages/beta`

Lockfile members: keys of `packages` that are neither `""` (the root) nor prefixed `node_modules/`, *and* that match the root entry's `packages[""].workspaces` globs:

- `.tools/hidden`
- `packages/alpha`
- `packages/beta`

Caution: `local-lib` (a `file:` directory dependency, not a member) is recorded exactly like a member, as `packages["local-lib"]` plus a `packages["node_modules/local-lib"]` link entry with `"resolved": "local-lib", "link": true`. The lockfile alone cannot tell it apart from a workspace member; only the `workspaces` globs (copied into `packages[""].workspaces`) can.
