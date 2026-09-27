# Provenance: bun-1.3.3/precedence-both

- Tools: Bun 1.3.3 (274e01c7) and Bun 1.1.38 (via `npx --yes bun@1.1.38`)
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 Darwin Kernel Version 27.2.0: Sun Sep 13 19:46:18 PDT 2026; root:xnu-13432.40.162~92/RELEASE_ARM64_T6041 arm64); Node v22.20.0
- Date: 2026-09-26
- Generated in `/tmp/lockfile-fixtures/bun-1.3.3/precedence-both/`, then copied here.

Both `bun.lock` and `bun.lockb` are real tool output for the same project.

## How this state arose

Bun 1.3.3 never produces both files itself:

- with an existing `bun.lockb`, `bun install --save-text-lockfile` writes `bun.lock` and DELETES `bun.lockb`;
- with an existing `bun.lock`, `saveTextLockfile = false` in `bunfig.toml` is ignored: `bun.lock` is left
  unchanged and no `bun.lockb` is written.

The two-file state arises when an older Bun that predates the text lockfile (1.1.x) runs `bun install` in a
project that already has `bun.lock`: it ignores `bun.lock` and writes `bun.lockb` beside it. (It can also arise
from merging branches that each committed a different lockfile.)

## Commands (in order)

1. Copied the manifests and `bun.lockb` from the `workspace-binary` scratch project.
2. `bun install --save-text-lockfile --no-progress </dev/null` (Bun 1.3.3) -> wrote `bun.lock`, removed `bun.lockb`.
3. Wrote `bunfig.toml` with `saveTextLockfile = false`; `bun install --no-progress </dev/null` (Bun 1.3.3) ->
   `bun.lock` unchanged (sha1 `30bff06a...`), no `bun.lockb`.
4. `rm -f bunfig.toml; rm -rf node_modules`
5. `npx --yes bun@1.1.38 install --no-progress </dev/null` -> wrote `bun.lockb` (2624 bytes, signature
   `bun-lockfile-format-v0`); `bun.lock` unchanged.
6. Observation only (in a throwaway copy, not this directory): Bun 1.3.3 `bun install` with both files present
   left both byte-identical and succeeded. Which file it read was not determined by this experiment.

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
packages/beta  (from bun.lock `workspaces` keys; bun.lockb is binary)
```
