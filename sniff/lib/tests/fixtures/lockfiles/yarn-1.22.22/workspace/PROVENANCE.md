# Provenance: yarn-1.22.22/workspace

- Tool: Yarn Classic 1.22.22 via `npx --yes yarn@1.22.22` (`npx --yes yarn@1.22.22 --version` printed `1.22.22`)
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 Darwin Kernel Version 27.2.0: Sun Sep 13 19:46:18 PDT 2026; root:xnu-13432.40.162~92/RELEASE_ARM64_T6041 arm64); Node v22.20.0
- Date: 2026-09-26
- Generated in `/tmp/lockfile-fixtures/yarn-1.22.22/workspace/`, then copied here.

## Setup

- `package.json` (name `fixture-root`, `private: true`) declares `workspaces: ["packages/*", ".tools/hidden"]` and
  depends on `local-lib` via `file:./local-lib` (not a member).
- `packages/alpha` depends on `@fixture/beta` with `^1.0.0` (Classic has no `workspace:` protocol; the range
  resolves to the workspace).

## Commands (in order)

1. Wrote the manifests by hand.
2. `COREPACK_ENABLE_STRICT=0 npx --yes yarn@1.22.22 install --non-interactive --no-progress </dev/null`
   -> "info No lockfile found." ... "success Saved lockfile."

## Trimmed

- `node_modules/`

## Expected membership

Declared by the manifest (root excluded, sorted):

```
.tools/hidden
packages/alpha
packages/beta
```

Recorded by the lockfile (root excluded, sorted):

```
(none)
```

Yarn Classic does NOT record workspace members in `yarn.lock`. The file contains only the header
(`# yarn lockfile v1`) and the non-member `"local-lib@file:./local-lib"` entry. A Classic lockfile therefore
cannot corroborate membership; a parser must treat it as "no membership data", not "zero members".
