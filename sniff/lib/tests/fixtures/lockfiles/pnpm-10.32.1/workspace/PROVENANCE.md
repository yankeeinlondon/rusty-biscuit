# PROVENANCE

- Tool: pnpm 10.32.1 (run as `npx -y pnpm@10.32.1`; the ambient `pnpm` outside the repository resolves to 10.34.4, so the version was pinned explicitly)
- Lockfile: `pnpm-lock.yaml`, `lockfileVersion: '9.0'`
- Host: macOS 27.2 (build 26B5091g), `Darwin 27.2.0 arm64` (Apple Silicon); Node.js v22.20.0 unless stated
- Date: 2026-09-26
- Generated in scratch `/tmp/lockfile-fixtures/pnpm-10.32.1/workspace/`, outside any repository

## Setup

The manifests were written by hand with a shell script before the tool ran: root `package.json` (name `fixture-root`), `packages/alpha/package.json` (`alpha`), `packages/beta/package.json` (`@fixture/beta`), `.tools/hidden/package.json` (`hidden-tool`), `local-lib/package.json` (`local-lib`, not a member). `pnpm-workspace.yaml` lists `packages/*` and `.tools/hidden` (explicitly, since globs may skip dot directories). The root depends on `local-lib` through `link:./local-lib`; `alpha` depends on `@fixture/beta` through `workspace:*`. No registry dependencies.

## Commands (in order)

1. `CI=1 npx -y pnpm@10.32.1 install --lockfile-only` (exploratory; its lockfile was deleted)
2. `rm pnpm-lock.yaml`
3. `CI=1 npx -y pnpm@10.32.1 install` (this run produced the committed lockfile, byte-identical to step 1's)

## Trimmed

- `node_modules/` (root; contained `.pnpm/`, `.pnpm-workspace-state-v1.json`, and the `local-lib` link)

## Expected membership

Manifest-declared members (`pnpm-workspace.yaml`):

- `.tools/hidden`
- `packages/alpha`
- `packages/beta`

Lockfile members (`importers` keys, excluding `.`, which is the root):

- `.tools/hidden`
- `packages/alpha`
- `packages/beta`

`local-lib` appears only as a dependency value (`version: link:local-lib`) under the root importer, never as an importer key.
