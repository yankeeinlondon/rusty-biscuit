---
kind: feature
name: publish
date: 2026-10-28
status: draft-spec
depends-on:
  - 2026-11-06-release-path-repair
next_spec: 2026-11-06-ubuntu-archive-disk
related:
  - 2026-10-03-binary-releases
  - 2026-11-06-windows-wsl-test-failures
reviewed: false
implemented: false
---

# Publish installable binaries

## Why

Nothing is installable today. `2026-10-03-binary-releases` chose the tooling and
channels, but it closed without a pipeline being built: there is no `dist`
config and no release workflow.

## Outcome

When a CLI is tagged, its archives are attached to its GitHub Release, and a user
can install it with:

| Channel | Example |
|---|---|
| GitHub Release archive | direct download |
| Shell / PowerShell installer | `curl -LsSf <release>/…-installer.sh \| sh` |
| cargo-binstall | `cargo binstall --git https://github.com/yankeeinlondon/rusty-biscuit biscuit-hash-cli` |
| Homebrew (own tap) | `brew install yankeeinlondon/tap/bh` |

## Scope

- Adopt `dist` with `precise-builds`, opting in only approved CLIs, with a
  stripped, LTO release profile.
- Prove the whole chain on `bh` first, then widen to `md`, `dmls`, `sniff`, `wt`,
  `hug`, `claudine`, and `bf`.
- Use `docs/deployment.yaml` as the one source of what is approved for which
  channel. Generate `dist` opt-ins from it, or check them against it.
- Fix each published crate's metadata. `repository` is unset on seven of the
  eight first-set CLIs, and `darkmatter-cli` points at the old `dockhand` repo.
- Set `git_release_enable = false` if `dist` owns the GitHub Release.
- Verify every channel with a real install on each target OS.

## Decisions already made

- Early releases use macOS and Linux evidence, and may ship before Windows and
  WSL2 runs are event-driven.
- Windows binaries ship only once `2026-11-06-windows-wsl-test-failures` has
  Windows green. This is a condition on the Windows target, not a prerequisite
  for this spec.

## Needs from Ken

Create the `yankeeinlondon/homebrew-tap` repository and a token for it.

## Spikes needed

- Does `dist` 0.34 handle release-plz's `{package}-v{version}` tags in this
  workspace? The earlier spec's claim was made against 0.33, without a run.
- Does binstall find the assets from that tag format, or does each crate need an
  explicit `pkg-url`?
- Re-measure binary sizes and the cmake/ALSA build requirements; the earlier
  figures date from Oct 3.

## Not in scope

npm, PyPI, and crates.io: `2026-11-06-publish-more-channels`.
