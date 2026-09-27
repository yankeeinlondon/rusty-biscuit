# Provenance: `pnpm-8.15.9/workspace`

- **Tool:** pnpm 8.15.9 (`npx -y pnpm@8`)
- **Host:** macOS 27.2 (arm64), Node v22.20.0
- **Date:** 2026-09-26
- **Generated in:** a scratch directory under `/tmp`, outside the repository

## Commands

```sh
CI=1 npx -y pnpm@8 install --lockfile-only
```

## Trimmed

`node_modules/` and any tool cache or install state were deleted after generation. No file content was edited.

## Expected membership

- **Manifest members** (layer-relative, root excluded, sorted): `.tools/hidden`, `packages/alpha`, `packages/beta`
- **Non-member local dependency:** `local-lib` (`link:./local-lib` from the root)
- **Lockfile records:** `importers` keys `.`, `.tools/hidden`, `packages/alpha`, `packages/beta`
- **Lockfile version:** `lockfileVersion: '6.0'`
