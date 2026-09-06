---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-03T21:47:54+01:00
spec: 2026-09-03-dmls-regression/spec.md
implemented: true
implemented_by: codex/default
log: darkmatter/fixes/2026-09-03-dmls-regression/log.md
description: A **fix** review of `2026-09-03-dmls-regression/spec.md`
fix: 2026-09-03-dmls-regression/review-2.md
previous: 2026-09-03-dmls-regression/review-1.md
next: 2026-09-03-dmls-regression/review-3.md
---

# Review 2: DMLS Regression

## Verdict

The fix is **not ready for production**. Contrary to the implementation claim,
the current worktree still contains the specification and plan but none of the
implementation required by AC1–AC9. Review 1's production blockers therefore
remain open.

## Findings

### Critical — Review 1 was marked implemented without implementing the fix

There is still no `darkmatter/dmls/zed-dmls-cli/` package, `zed-dmls stage` or
`doctor` command, `zed_extension_contract.rs` test, official Zed packager
configuration, or `install-zed`, `zed-doctor`, `zed-package`, or `zed-verify`
recipe. The execution plan remains entirely unchecked. Repository and history
inspection found no implementation of these surfaces after Review 1.

The original failure mode is consequently unchanged: Zed can remain registered
to a disposable worktree, deleting that worktree can leave a dangling
registration, and the repository still has no typed command that diagnoses or
repairs the staging side of that condition. Implement the reviewed plan before
marking Review 2 implemented.

### High — The normal L1 gate still cannot detect an unloadable extension

`darkmatter/dmls/tests/zed_extension_contract.rs` is absent. The only current
Zed-related L1 test, `packaging_contract.rs`, compares release archive-name
strings. It does not parse `extension.toml`, verify its id/language mapping,
compare manifest and crate versions, validate `cdylib`/API dependency shape, or
check the extension lockfile and source entry point.

`just test dmls` passed 642 tests, with the archive-name contract as the only
Zed packaging test. That green run supplies no verification for AC1, and the
three required non-vacuity mutations have not been implemented or recorded.

### High — Zed's actual build and packaging paths are still non-blocking

`darkmatter/justfile` still checks `wasm32-wasip1`, omits `--locked`, and exits
zero when the target is absent. On this host, `just check-zed` printed a skip
for the missing `wasm32-wasip1` target and succeeded. The recipe remains under
`check`, not the area `lint` gate specified by AC2.

There is no pinned `zed-extension` binary or digest, no artifact validation, no
Ubuntu companion step in `_package-ci.yml`, and no latest-stable packaging
step. AC2 and AC3 therefore have no blocking verification. The official
packager belongs in companion CI verification as designed; it should not be
misclassified as L2.

### High — Stable staging and doctor behavior have no implementation or tests

AC4–AC6 and AC8 require observable CLI behavior: cross-platform stable paths,
allowlisted and rollback-safe staging, repeat updates, exact manual-registration
status, binary/version checks, registration/link/manifest diagnosis, bounded
log interpretation, and `TerminalRenderable` output. None of these commands or
tests exists.

Their strongest verification level is therefore **none**. Hermetic Level 1
tests are the appropriate automated level because these requirements do not
claim terminal-emulator rendering fidelity or physical input behavior. AC6
also requires the explicit manual real-Zed restart evidence from the spec; no
such evidence is recorded.

### High — Extension-only changes still do not schedule mandatory Zed verification

`zed-extension` is absent from the closed `runner-tools` vocabulary and from
DMLS package metadata, which still lists only `neovim`. The affected-scope
suite has no test proving that a change under the workspace-excluded
`darkmatter/dmls/zed-dmls/**` path selects DMLS and the companion gate.

The existing 65 affected-scope tests pass, but they do not exercise AC9. An
extension-only regression can therefore remain outside an authoritative Zed
build/package gate.

### Medium — Active documentation still prescribes the fragile workflow

The Zed guide still tells developers to register the extension directory from
the current checkout and explicitly says to reinstall when a worktree is
deleted. The extension README still says monorepo recipes intentionally do not
build it, and the DMLS README still directs users to install `zed-dmls/`
directly. The justfile continues to call `wasm32-wasip1` Zed's target while the
editor documentation says `wasm32-wasip2`.

AC7 is not satisfied: active code and documentation still disagree, and
`git grep wasip1 darkmatter/` returns the current justfile in addition to
historical records.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: manifest/crate-shape regressions fail L1 on all OSes | None; only archive-name strings are tested | **High gap.** Cross-platform Level 1 contract and non-vacuity tests are required. |
| AC2: mandatory `wasm32-wasip2` compile gate | None; current `wasip1` check skips successfully | **High gap.** This is a build/CI gate, not L2. |
| AC3: pinned official packager validates manifest and WASM | None | **High gap.** Mandatory companion verification is required. |
| AC4: doctor accurately diagnoses registration and historical logs | None; command absent | **High gap.** Hermetic Level 1 subprocess/filesystem coverage is required. |
| AC5: stable, repeatable staging without registration mutation | None; command absent | **High gap.** Hermetic Level 1 staging and exit-code coverage is required. |
| AC6: registered extension survives source-worktree removal | None; no Level 1 path-independence proof or manual real-Zed evidence | **High gap.** Both forms of evidence required by the spec are missing. |
| AC7: docs and recipe agree on target/install procedure | None; active guidance conflicts | Not satisfied. |
| AC8: cross-platform CLI and `TerminalRenderable` output | None; package absent | **High gap.** Level 1 on the OS matrix is appropriate. |
| AC9: extension-only changes select companion verification | None; unrelated scope-policy tests pass | **High gap.** A focused Level 1 policy test is required. |

Levels 2 and 3 are not required for this fix. It claims neither real-terminal
rendering fidelity nor OS keyboard, mouse, paste, or IME behavior. Zed's
headless packager is companion verification, while the one real-Zed restart is
an explicit manual acceptance exercise.

## Ergonomics and Performance

There is no new Rust implementation to review for ergonomics or performance.
The specified architecture remains appropriate: a small typed CLI with
injectable discovery/filesystem seams keeps OS behavior testable, while an
allowlisted sibling-directory swap bounds I/O and preserves the prior usable
stage on failure. Those benefits remain unimplemented.

## Verification Performed

- `just test dmls`: **642 passed, 0 skipped**. This verifies existing DMLS L1
  behavior, not AC1–AC9.
- `just check-zed`: exited **0** after skipping because
  `wasm32-wasip1` is absent, directly confirming the non-blocking wrong-target
  behavior.
- `python3 scripts/ci/test_affected_scope.py`: **65 passed**; source inspection
  confirmed none covers the excluded Zed extension path or companion gate.
- Inventory, source, and Git-history inspection confirmed every implementation
  surface named above is absent and the execution plan remains unchecked.
- GitNexus concept search found only the pre-existing archive-name contract and
  Zed extension binary-resolution symbols; it found no stage, doctor, manifest
  contract, or companion-verification implementation. The current worktree is
  not registered as an index, so the same-HEAD `feat-unifi` index was used for
  read-only graph corroboration.

## Production Readiness

The fix is **not production ready**. None of AC1–AC9 is satisfied, and every
user-observable requirement has either no test at all or no implementation to
test. A green DMLS suite must not be treated as evidence that Zed can load,
build, package, stage, or diagnose this extension.