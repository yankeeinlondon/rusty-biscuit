# Provenance: `yarn-3.8.7/workspace`

- **Tool:** Yarn 3.8.7 (corepack)
- **Host:** macOS 27.2 (arm64), Node v22.20.0
- **Date:** 2026-09-26
- **Generated in:** a scratch directory under `/tmp`, outside the repository

## Commands

```sh
COREPACK_ENABLE_DOWNLOAD_PROMPT=0 YARN_ENABLE_IMMUTABLE_INSTALLS=false corepack yarn@3.8.7 install
```

## Trimmed

`node_modules/` and any tool cache or install state were deleted after generation. No file content was edited.

## Expected membership

- **Manifest members** (layer-relative, root excluded, sorted): `.tools/hidden`, `packages/alpha`, `packages/beta`
- **Non-member local dependency:** `local-lib` (`link:./local-lib`), plus `portal-lib` (`portal:./portal-lib`)
- **Lockfile records:** entries whose `resolution` is `<name>@workspace:<path>`: `.` (root), `.tools/hidden`, `packages/alpha`, `packages/beta`
- **Lockfile version:** `__metadata.version: 6`
