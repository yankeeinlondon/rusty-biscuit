# Provenance: `bun-1.2.0/workspace`

- **Tool:** Bun 1.2.0 (`npx -y bun@1.2.0`)
- **Host:** macOS 27.2 (arm64), Node v22.20.0
- **Date:** 2026-09-26
- **Generated in:** a scratch directory under `/tmp`, outside the repository

## Commands

```sh
npx -y bun@1.2.0 install --save-text-lockfile
```

## Trimmed

`node_modules/` and any tool cache or install state were deleted after generation. No file content was edited.

## Expected membership

- **Manifest members** (layer-relative, root excluded, sorted): `.tools/hidden`, `packages/alpha`, `packages/beta`
- **Non-member local dependency:** `local-lib` (`file:./local-lib` from the root)
- **Lockfile records:** `workspaces` keys `""`, `.tools/hidden`, `packages/alpha`, `packages/beta`
- **Lockfile version:** `"lockfileVersion": 1` with no `configVersion` key (Bun 1.3 adds `configVersion`)
