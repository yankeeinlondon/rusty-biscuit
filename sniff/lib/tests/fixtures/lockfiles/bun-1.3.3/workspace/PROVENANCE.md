# Provenance: bun-1.3.3/workspace

- Tool: Bun 1.3.3 (274e01c7), installed at `~/.bun/bin/bun`
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 Darwin Kernel Version 27.2.0: Sun Sep 13 19:46:18 PDT 2026; root:xnu-13432.40.162~92/RELEASE_ARM64_T6041 arm64); Node v22.20.0
- Date: 2026-09-26
- Generated in `/tmp/lockfile-fixtures/bun-1.3.3/workspace/`, then copied here.

## Setup

- `package.json` (name `fixture-root`, `private: true`) declares `workspaces: ["packages/*", ".tools/hidden"]` and
  depends on `local-lib` via `file:./local-lib` (not a member).
- `packages/alpha` depends on `@fixture/beta` with `workspace:*`.

## Commands (in order)

1. Wrote the manifests by hand.
2. `bun install --no-progress </dev/null` -> "Saved lockfile".

## Trimmed

- `node_modules/`
- `packages/alpha/node_modules/` (install output for alpha's `@fixture/beta` workspace dependency)

## Expected membership

Declared by the manifest (root excluded, sorted):

```
.tools/hidden
packages/alpha
packages/beta
```

Recorded by the lockfile (root excluded, sorted):

```
.tools/hidden
packages/alpha
packages/beta
```

Membership lives in the top-level `workspaces` object: keys are member paths, `""` is the root. `bun.lock` is
JSONC as written by Bun itself (trailing commas after the last member of every object and array).
