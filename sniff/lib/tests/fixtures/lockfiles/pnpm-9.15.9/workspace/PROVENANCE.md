# PROVENANCE

- Tool: pnpm 9.15.9 (latest 9.x at generation time; run as `npx -y pnpm@9.15.9`)
- Lockfile: `pnpm-lock.yaml`, `lockfileVersion: '9.0'`
- Host: macOS 27.2 (build 26B5091g), `Darwin 27.2.0 arm64` (Apple Silicon); Node.js v22.20.0 unless stated
- Date: 2026-09-26
- Generated in scratch `/tmp/lockfile-fixtures/pnpm-9.15.9/workspace/`, outside any repository

## Setup

The manifests were written by hand with a shell script before the tool ran: root `package.json` (name `fixture-root`), `packages/alpha/package.json` (`alpha`), `packages/beta/package.json` (`@fixture/beta`), `.tools/hidden/package.json` (`hidden-tool`), `local-lib/package.json` (`local-lib`, not a member). `pnpm-workspace.yaml` lists `packages/*` and `.tools/hidden`. The root depends on `local-lib` through `link:./local-lib`; `alpha` depends on `@fixture/beta` through `workspace:*`. No registry dependencies.

## Commands (in order)

1. `CI=1 npx -y pnpm@9.15.9 install`

## Trimmed

- `node_modules/` (root)

## Expected membership

Manifest-declared members: `.tools/hidden`, `packages/alpha`, `packages/beta`.

Lockfile members (`importers` keys, excluding `.`): `.tools/hidden`, `packages/alpha`, `packages/beta`.

The lockfile is byte-identical to the pnpm 10.32.1 `workspace` fixture: pnpm 9 and 10 both write `lockfileVersion: '9.0'`, so the lockfile alone cannot distinguish them.
