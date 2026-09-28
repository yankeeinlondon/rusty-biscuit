# PROVENANCE

- Tool: npm 6.14.18 (latest 6.x; run as `npx -y npm@6.14.18` under Node.js v22.20.0)
- Lockfile: `package-lock.json`, `lockfileVersion: 1`
- Host: macOS 27.2 (build 26B5091g), `Darwin 27.2.0 arm64` (Apple Silicon); Node.js v22.20.0 unless stated
- Date: 2026-09-26
- Generated in scratch `/tmp/lockfile-fixtures/npm-6.14.18/single-project/`, outside any repository

## Setup

npm 6 has no workspaces. Root `package.json` (name `fixture-root`) depends on `local-lib` through `file:./local-lib`; `local-lib/package.json` (name `local-lib`). This fixture is a real v1 sample for version-signature evidence, not a workspace.

## Commands (in order)

1. `npx -y npm@6.14.18 install --no-audit --no-fund`

## Trimmed

- `node_modules/` (root)

## Expected membership

Manifest-declared members: none (npm 6 has no workspace concept).

Lockfile members: none. The v1 lockfile has no `packages` section; its only dependency record is `dependencies["local-lib"]` with `"version": "file:local-lib"`, which is a local dependency, not a member.
