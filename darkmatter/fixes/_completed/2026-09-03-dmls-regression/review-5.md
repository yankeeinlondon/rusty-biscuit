---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-03T23:25:51+01:00
spec: 2026-09-03-dmls-regression/spec.md
implemented: true
implemented_by: codex/default
log: darkmatter/fixes/2026-09-03-dmls-regression/log.md
description: A **fix** review of `2026-09-03-dmls-regression/spec.md`
fix: 2026-09-03-dmls-regression/review-5.md
previous: 2026-09-03-dmls-regression/review-4.md
next: 2026-09-03-dmls-regression/review-6.md
---

# Review 5: DMLS Regression

## Verdict

The fix is **not ready for production**. The implementation remains coherent
and the current macOS Level-1, lint, cross-compile, and CI-policy checks pass,
but review 4's three high-severity acceptance gaps were deferred rather than
closed. There is still no successful run of the official Ubuntu packager, no
native Linux or Windows Level-1 execution for the new host CLI, and no real-Zed
restart result proving that the staged registration survives source-worktree
removal.

## Findings

### High — The official Zed packaging gate still has no successful execution

The implementation provisions the pinned `zed-extension` binary, verifies its
SHA-256 digest, installs `wasm32-wasip2`, and invokes `just zed-verify` in the
Ubuntu lint producer (`.github/workflows/_package-ci.yml:451-519`). The local
recipe validates `manifest.json`, the packaged `extension.toml`, and
`extension.wasm` (`darkmatter/justfile:330-370`). These are the intended
contracts.

They have nevertheless not run end to end. The public Actions API currently
reports zero runs for `fix/dmls-regression`, and this review's local
`just check-zed` stopped at its intentional missing-target failure. Therefore
AC3 still has only Level-1 configuration inspection and negative provisioning
evidence. That cannot establish that the pinned official binary accepts the
arguments, builds this extension, or emits the artifact shape consumed by the
verification recipe. Retain one successful designated Ubuntu companion run
before release.

### High — The required real-Zed worktree-removal scenario remains unverified

The Unix Level-1 test now stages the extension, links a manufactured
registration to the stable stage, removes the manufactured source worktree,
and proves that doctor remains healthy
(`zed-dmls-cli/src/lib.rs:769-809`). A corresponding Windows junction test is
present at lines 812-855.

AC6 requires the same relationship through a real Zed registration: stage from
worktree A, register the stable directory, delete worktree A, restart Zed, and
verify that no new missing-manifest error appears. No result for that exercise
is recorded. The read-only doctor invocation in this review found no DMLS dev
registration in its discovered Zed data directory, so it could not provide
current-host corroboration. Manufactured filesystem state is appropriate
Level-1 coverage, but it is not evidence for Zed's actual registration and
reload behavior.

### High — Linux and Windows Level-1 runtime evidence is still absent

AC1 and AC8 explicitly require the manifest contract and `zed-dmls-cli` tests
to execute on macOS, Linux, and Windows. The reusable workflow declares a
blocking native matrix (`.github/workflows/_package-ci.yml:198-252`), but there
is no branch run from which to retain Linux or Windows results. This review
executed all 664 scoped tests on macOS and cross-compiled the CLI tests for
`x86_64-pc-windows-gnu`; the Windows-only junction test compiled but did not
run. Compile evidence cannot validate Windows junction behavior, and neither
it nor the macOS run validates Linux filesystem behavior. Run the native
package-CI Level-1 legs on both platforms before release.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: manifest/crate regressions fail L1 on all OSes | Level 1 passes on macOS; prior non-vacuity mutations are recorded; no Linux/Windows execution | **High gap:** required native matrix evidence is absent. |
| AC2: mandatory `wasm32-wasip2` compile gate | Level-1 recipe/workflow inspection plus observed fail-closed missing-target behavior | Satisfied structurally; successful extension compilation is folded into the missing AC3 Ubuntu run. |
| AC3: official packager validates emitted artifacts | Level-1 policy/recipe inspection and digest evidence | **High gap:** no successful official-packager execution. |
| AC4: doctor diagnoses binary, registration, manifest, and log failures | Level 1 unit and subprocess tests on macOS | Satisfied for the exercised platform; native matrix evidence remains under AC8. |
| AC5: stable, repeatable, rollback-safe staging | Level 1 filesystem and subprocess tests on macOS | Satisfied for macOS; cross-platform execution remains under AC8. |
| AC6: deleting a source worktree does not break what Zed loads | Level 1 manufactured symlink test on macOS; Windows test compile-only | **High gap:** no real-Zed registration/restart observation. |
| AC7: active documentation and recipe agree | Static inspection | Satisfied for active guidance. Historical records retain `wasm32-wasip1`, so the specification's literal unscoped grep cannot be empty without rewriting history. |
| AC8: CLI tests pass on all OSes and output uses `TerminalRenderable` | Level 1 macOS execution plus Windows cross-compile | **High gap:** Linux and Windows native execution are absent. The output path does use `TerminalRenderable`. |
| AC9: extension-only changes schedule DMLS companion verification | Level 1 affected-scope policy tests | Satisfied; all 67 policy tests pass. |

No Level 2 or Level 3 test is required by this fix. It makes no terminal-
emulator rendering or physical-input claim. The official packager and real-Zed
restart are environment-backed acceptance checks outside those terminal test
tiers.

## Ergonomics and Performance

No additional production-blocking ergonomics or performance defect was found.
The typed path discovery, bounded process/log operations, allowlisted staging,
rollback path, plain output mode, and `TerminalRenderable` report construction
remain appropriately scoped. No implementation changes have landed since
review 4 that would alter that assessment.

## Verification Performed

- `just test dmls zed-dmls-cli`: **664 passed, 0 skipped** on macOS (Level 1).
- `just _lint dmls`: passed.
- `just _lint zed-dmls-cli`: passed.
- `python3 test_affected_scope.py` from `scripts/ci/`: **67 passed**.
- `cargo check --color=never --locked -p zed-dmls-cli --target x86_64-pc-windows-gnu --tests`: passed; compile evidence only.
- `just check-zed`: failed closed with the documented provisioning command because `wasm32-wasip2` is absent.
- Public GitHub Actions query for `fix/dmls-regression`: **0 runs**.
- `cargo run --quiet -p zed-dmls-cli -- doctor --plain`: found a version-compatible `dmls`, but no registration at the discovered Zed data directory.
- Both modified workflow documents parse as YAML.
- `git diff --check`: passed before review metadata edits.

## Production Readiness

The fix is **not production ready**. Production readiness requires a successful
designated Ubuntu `zed-verify` run, native Linux and Windows Level-1 package
runs, and the explicit real-Zed worktree-removal/restart acceptance result.
