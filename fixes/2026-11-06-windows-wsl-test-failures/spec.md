---
kind: fix
name: windows-wsl-test-failures
date: 2026-11-06
status: draft-spec
depends-on:
  - 2026-11-06-ubuntu-archive-disk
next_spec: 2026-11-06-legible-ci-names
reviewed: false
implemented: false
---

# Clear the Windows and WSL2 test failures

## Why

The Nov 6 nightly had 141 failures in about 19,900 Windows tests, plus one
compile error. No confirmed product bug has been found, but this red hides any
real one and blocks Windows binaries. WSL2's state is unknown until
`2026-11-06-ubuntu-archive-disk` lands.

## Known failures

| Tests | Package | Apparent cause | Verdict so far |
|---|---|---|---|
| compile | `biscuit-visualized` | E0284 in `embed_render_tests.rs:450`; Windows only | Test code |
| 24 | `tree-hugger-cli` `index_corruption` | Expects `RUNNER~1`, output has `runneradmin` | Test bug |
| 96 | `tree-hugger-cli` `ownership` | Symlink targets spelled with `/` do not resolve | Probably test bug |
| 6 | `worktree` remove lifecycle | Compares two spellings of one path | Test bug |
| 2 | `claudine-cli` | Short vs long path, reversed | Test bug |
| 4 | `darkmatter` `kind_semantic_checks` | Expects `/`; path appears to use `\` | Probably test bug |
| 1 | `model_id` `ui` | `trybuild` offline, no `proc-macro2` | Test environment |
| 3 | `worktree`, `git-wt-cli` | Runner Git at `ucrt64\bin\git.exe`, not recognized | **Real, small product gap** |
| 1 | `messenger` | Timing wait expires; intermittent | Probably flaky |
| 4 | `darkmatter-cli` | Intermittent; cause unknown | **Unexplained** |

These verdicts come from logs and test code. None has been rerun on Windows.

## Outcome

- Windows and WSL2 are green across consecutive runs on `main`.
- One shared helper compares paths in canonical form, per the `os` skill's
  `windows-paths.md`, in place of per-test workarounds.
- `wt` handles the `ucrt64` layout, or is shown to be correct without it.

## Scope

- Each row above, plus whatever WSL2 reveals.
- A test showing a product bug gets a product fix; never loosen an assertion to
  get to green.
- Record new OS facts in the `os` skill.

## Spike needed

Reproduce the unexplained `darkmatter-cli` and `messenger` failures on
`$BUILD_WIN`. That host may not have an 8.3 short-name temp directory, so a pass
there does not clear a `RUNNER~1` failure.
