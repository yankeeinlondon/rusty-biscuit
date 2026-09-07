---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-03T23:11:24+01:00
spec: 2026-09-03-dmls-regression/spec.md
implemented: true
implemented_by: codex/default
log: darkmatter/fixes/2026-09-03-dmls-regression/log.md
description: A **fix** review of `2026-09-03-dmls-regression/spec.md`
fix: 2026-09-03-dmls-regression/review-4.md
previous: 2026-09-03-dmls-regression/review-3.md
next: 2026-09-03-dmls-regression/review-5.md
---

# Review 4: DMLS Regression

## Verdict

The fix is **not ready for production**. Review 3's two code findings are
closed: Linux now follows Zed's Flatpak/XDG path precedence, doctor derives
the expected server version from the `dmls` package, and the linked stable
registration survives source removal in the macOS Level-1 test. The scoped
Rust and CI-policy gates pass. Production evidence is still absent for the
official Ubuntu packager, the Windows/Linux L1 matrix, and the specification's
defining real-Zed restart exercise.

## Findings

### High — The official Zed packaging acceptance gate has still never completed

The implementation correctly provisions the digest-pinned `zed-extension`
binary and `wasm32-wasip2` target in the Ubuntu lint producer, then invokes
`just zed-verify` (`.github/workflows/_package-ci.yml:451-519`). The local
recipe also fails closed when its target is absent (`darkmatter/justfile:302-313`).

However, the public Actions API reports zero workflow runs for
`fix/dmls-regression`, and this review's `just zed-verify` invocation stopped
at the missing-target check. Consequently, AC3's strongest evidence remains
Level-1 workflow/policy inspection. It does not prove that the pinned official
binary accepts the configured arguments, compiles this extension, or emits
the `manifest.json`, archive, and `extension.wasm` shape asserted by
`zed-verify`. A successful designated Ubuntu companion run is required.

### High — The defining real-Zed worktree-removal scenario remains unverified

The replacement Unix test now models the repository-verifiable relationship
correctly: it links Zed's manufactured registration to the stable stage,
updates the stage, removes the source worktree, and proves doctor remains
healthy (`zed-dmls-cli/src/lib.rs:769-809`). This closes the Level-1 gap from
Review 3.

AC6 additionally requires registering that path in a real Zed instance,
removing worktree A, restarting Zed, and confirming the new log contains no
missing-manifest error. No such acceptance result is recorded. The read-only
doctor run in this review found no DMLS dev registration at its discovered Zed
data path, so it could not supply even current-host corroboration. This is a
real-editor acceptance gap, not Level 2 or Level 3, and manufactured filesystem
state cannot satisfy it.

### High — Cross-platform Level-1 execution is absent for the new host CLI

AC1 and AC8 require the manifest contract and `zed-dmls-cli` tests to pass on
macOS, Linux, and Windows. This review ran all 664 scoped tests on macOS. The
Windows target, including the junction-only test, compiles, but that test did
not execute; no Linux or Windows CI run exists for this branch. In particular,
the Windows junction behavior at `zed-dmls-cli/src/lib.rs:812-854` is currently
supported only by compile evidence.

This is a Level-1 matrix gap. Run the blocking native package-CI legs on all
three operating systems and retain their successful results before declaring
the cross-platform CLI production ready.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: manifest/crate regressions fail L1 on all OSes | Level 1 contract passes on macOS; no Linux/Windows execution | **High gap:** the required OS matrix has not run. |
| AC2: mandatory `wasm32-wasip2` compile gate | Fail-closed local build gate and mandatory CI configuration | Structurally satisfied; the missing-target branch produced the documented failure. |
| AC3: official packager validates emitted artifacts | Level-1 workflow/policy inspection only | **High gap:** no successful Ubuntu companion execution. |
| AC4: doctor diagnoses binary, registration, manifest, and logs | Level 1 unit and subprocess tests on macOS | Satisfied for the exercised platform; Flatpak/XDG discovery coverage now follows the pinned authority. |
| AC5: stable, repeatable, rollback-safe staging | Level 1 filesystem and subprocess tests on macOS | Satisfied on macOS; cross-platform execution remains part of AC8. |
| AC6: worktree deletion does not break Zed | Level 1 linked-registration test on macOS; Windows test compile-only; no real-Zed restart | **High gap:** repository modeling improved, but explicit editor acceptance is absent. |
| AC7: active docs and recipe agree | Static inspection | Satisfied for active guidance. Historical feature/fix records necessarily retain `wasip1`, so the literal unscoped grep criterion is self-contradictory. |
| AC8: cross-platform CLI and `TerminalRenderable` output | Level 1 macOS tests plus Windows cross-compile | **High gap:** no Linux/Windows test execution. The output path does use `TerminalRenderable`. |
| AC9: extension-only changes select DMLS companion verification | Level 1 affected-scope policy tests | Satisfied; all 67 policy tests passed. |

No Level 2 or Level 3 test is required. The fix makes no terminal-emulator
rendering or physical keyboard/mouse claim. The official packager and real-Zed
restart are separate environment-backed acceptance checks.

## Ergonomics and Performance

No production-blocking ergonomics or performance regression was found. The
typed path-discovery seams, allowlisted stage, bounded log read, and generated
DMLS-version contract are appropriately scoped. The duplicate
`let source = source(temp.path());` in
`invalid_manifest_does_not_mutate_existing_stage` is harmless test cleanup,
not a readiness issue.

## Verification Performed

- `just test dmls zed-dmls-cli`: **664 passed, 0 skipped** on macOS (Level 1).
- `just _lint dmls`: passed.
- `just _lint zed-dmls-cli`: passed.
- `python3 scripts/ci/test_affected_scope.py`: **67 passed**.
- `cargo check --color=never --locked -p zed-dmls-cli --target x86_64-pc-windows-gnu --tests`: passed; compile evidence only.
- `just zed-verify`: failed closed because `wasm32-wasip2` is not installed; no package was attempted.
- Public GitHub Actions query for `fix/dmls-regression`: **0 runs**.
- `cargo run --quiet -p zed-dmls-cli -- doctor --plain`: server version compatible, but no DMLS Zed registration was visible at the discovered data path.
- `git diff --check`: passed before the review metadata edits.

## Production Readiness

The fix is **not production ready**. A green designated Ubuntu packaging run,
green Linux and Windows Level-1 package runs, and the explicit real-Zed
worktree-removal/restart exercise are still required.
