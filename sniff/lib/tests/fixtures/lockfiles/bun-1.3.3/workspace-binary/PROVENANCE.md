# Provenance: bun-1.3.3/workspace-binary

- Tool: Bun 1.3.3 (274e01c7)
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 Darwin Kernel Version 27.2.0: Sun Sep 13 19:46:18 PDT 2026; root:xnu-13432.40.162~92/RELEASE_ARM64_T6041 arm64); Node v22.20.0
- Date: 2026-09-26
- Generated in `/tmp/lockfile-fixtures/bun-1.3.3/workspace-binary/`, then copied here.

## Setup

Same manifests as `bun-1.3.3/workspace`, plus `bunfig.toml`:

```toml
[install]
saveTextLockfile = false
```

Bun 1.3.3 has no `--save-binary` or `--lockfile-format` flag (`bun install --help` lists only
`--save-text-lockfile`); the `bunfig.toml` setting is what makes it write `bun.lockb`. `bunfig.toml` is kept
because it is how this state arises.

## Commands (in order)

1. Copied `package.json`, `packages/`, `local-lib/`, `.tools/` from the `workspace` scratch project (before its
   `node_modules/` existed in this directory) and wrote `bunfig.toml`.
2. `bun install --no-progress </dev/null` -> wrote `bun.lockb` (2720 bytes) and no `bun.lock`.

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
packages/beta  (binary; see note)
```

`bun.lockb` begins with the signature `#!/usr/bin/env bun\nbun-lockfile-format-v0\n` followed by binary data.
Member paths and names appear only inside an unstructured string buffer (`strings` shows
`fixture-rootpackages/beta@fixture/betapackages/alpha.tools/hiddenhidden-tool...`). There is no documented
format; treat `bun.lockb` as "present but not parseable for membership".
