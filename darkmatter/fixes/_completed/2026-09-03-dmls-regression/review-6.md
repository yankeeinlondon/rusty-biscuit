---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-03T23:46:26+01:00
spec: 2026-09-03-dmls-regression/spec.md
implemented: false
description: A **fix** review of `2026-09-03-dmls-regression/spec.md`
fix: 2026-09-03-dmls-regression/review-6.md
previous: 2026-09-03-dmls-regression/review-5.md
---

# Review 6: DMLS Regression

## Verdict

The fix is **not ready for production**. The repository implementation remains
coherent and its available macOS Level-1, lint, and CI-policy checks pass, but
the implementation of review 5 explicitly deferred all three production
acceptance findings. No production or test code changed afterward, and fresh
inspection found no new external evidence: the official Ubuntu packager has
still never completed, Linux and Windows Level-1 tests have not run, and the
genuine Zed registration still points at the deleted worktree rather than the
stable stage.

## Findings

### High — The official Zed packaging gate still has no successful execution

The intended fail-closed gate remains correctly wired. Package CI provisions
`wasm32-wasip2`, downloads the commit-pinned Linux x86_64 `zed-extension`
binary, verifies its SHA-256 digest, and invokes `just zed-verify`
(`.github/workflows/_package-ci.yml:451-519`). The local recipe checks the
emitted `manifest.json`, packaged `extension.toml`, and `extension.wasm`
(`darkmatter/justfile:330-370`).

There is still no successful end-to-end execution to validate that contract.
The public Actions API reports zero runs for `fix/dmls-regression`, and the
branch is not present on the remote. This review's `just zed-verify` invocation
again stopped at the intentional missing-`wasm32-wasip2` failure on macOS.
Consequently AC3 has configuration inspection, digest verification recorded in
the implementation log, and negative fail-closed evidence, but no proof that
the pinned official binary accepts the invocation and emits the asserted
artifact shape. Retain one successful designated Ubuntu companion run before
release.

### High — The required real-Zed worktree-removal scenario remains unverified

The Unix Level-1 test stages the extension, points a manufactured registration
at the stable stage, removes the manufactured source worktree, and proves that
doctor remains healthy (`zed-dmls-cli/src/lib.rs:769-809`). A Windows junction
equivalent exists at lines 812-855. This is sound Level-1 coverage of the
repository-owned filesystem contract.

It does not satisfy AC6's explicit real-editor acceptance exercise. A fresh
read-only doctor run against the genuine host state found that Zed's
`extensions/installed/dmls` symlink still points at the deleted
`/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/dmls/zed-dmls`
directory, while `/Users/ken/Library/Application Support/dmls/zed-dmls` is
absent. Thus the stable path has not been registered and no post-removal Zed
restart can have established that the editor reloads it without a new
missing-manifest error. Complete and record the real-Zed registration,
source-worktree deletion, restart, and log check before release.

### High — Linux and Windows Level-1 runtime evidence is still absent

AC1 and AC8 require the manifest contract and `zed-dmls-cli` tests to execute
on macOS, Linux, and Windows. The reusable package workflow declares the
blocking native matrix, but the unpublished branch still has no workflow run.
This review executed all 664 scoped tests on macOS; prior Windows evidence is a
cross-compile only. In particular, the Windows-only junction test at
`zed-dmls-cli/src/lib.rs:812-855` has compiled but has not run. Compile evidence
cannot validate Windows junction semantics, and the macOS run cannot validate
Linux filesystem and path behavior. Retain successful native Linux and Windows
Level-1 package results before release.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: manifest/crate regressions fail L1 on all OSes | Level 1 passes on macOS; prior non-vacuity mutations are recorded; no Linux/Windows execution | **High gap:** required native matrix evidence is absent. |
| AC2: mandatory `wasm32-wasip2` compile gate | Level-1 recipe/workflow inspection and observed fail-closed missing-target behavior | Satisfied structurally; successful extension compilation remains part of the missing AC3 Ubuntu execution. |
| AC3: official packager validates emitted artifacts | Level-1 policy/recipe inspection and recorded digest verification | **High gap:** no successful official-packager execution. |
| AC4: doctor diagnoses binary, registration, manifest, and log failures | Level 1 unit and subprocess tests on macOS, plus a read-only genuine-host diagnosis | Satisfied for the exercised platform; native matrix evidence remains under AC8. |
| AC5: stable, repeatable, rollback-safe staging | Level 1 filesystem and subprocess tests on macOS | Satisfied for macOS; cross-platform execution remains under AC8. |
| AC6: deleting a source worktree does not break what Zed loads | Level 1 manufactured symlink test on macOS; Windows test compile-only | **High gap:** no real-Zed registration/restart observation, and the genuine registration remains broken. |
| AC7: active documentation and recipe agree | Static inspection | Satisfied for active guidance. Historical records retain `wasm32-wasip1`, so the specification's literal unscoped grep remains incompatible with preserving history. |
| AC8: CLI tests pass on all OSes and output uses `TerminalRenderable` | Level 1 macOS execution plus prior Windows cross-compile | **High gap:** Linux and Windows native execution are absent. The output path uses `TerminalRenderable`. |
| AC9: extension-only changes schedule DMLS companion verification | Level 1 affected-scope policy tests | Satisfied; all 67 policy tests pass. |

No Level 2 or Level 3 test is required by this fix. It makes no terminal-
emulator rendering, terminal input encoding, or physical-keyboard claim. The
official packager and real-Zed restart are environment-backed acceptance checks
outside those terminal test tiers.

## Ergonomics and Performance

No additional production-blocking ergonomics or performance defect was found.
The typed discovery, bounded subprocess/log operations, allowlisted atomic
staging, rollback behavior, plain output, and `TerminalRenderable` report path
remain appropriate. Review 5 produced no implementation change to reassess.

## Verification Performed

- `just test dmls zed-dmls-cli`: **664 passed, 0 skipped** on macOS (Level 1).
- `just _lint dmls`: passed.
- `just _lint zed-dmls-cli`: passed.
- `python3 test_affected_scope.py` from `scripts/ci/`: **67 passed**.
- `just zed-verify`: failed closed with the documented provisioning command
  because `wasm32-wasip2` is absent; no package was produced.
- Public GitHub Actions query for `fix/dmls-regression`: **0 runs**; the branch
  endpoint returned 404.
- `zed-dmls doctor --plain` against the genuine host paths: found a
  version-compatible `dmls`, then correctly failed on the dangling deleted-
  worktree registration; the stable stage is absent.
- GitNexus compare-to-main analysis reported critical aggregate risk across 321
  files and 31 affected processes, but the available index belongs to another
  worktree and includes hundreds of unrelated mainline changes; it is not a
  reliable scope assessment for this fix. Direct inspection and scoped gates
  remain the review authority.
- `git diff --check`: passed before review metadata edits.

## Production Readiness

The fix is **not production ready**. Release requires a successful designated
Ubuntu `zed-verify` run, native Linux and Windows Level-1 package runs, and the
explicit real-Zed stable-registration/worktree-removal/restart result.
