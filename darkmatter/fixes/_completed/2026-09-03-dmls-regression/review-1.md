---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-03T21:43:32+01:00
spec: 2026-09-03-dmls-regression/spec.md
implemented: true
description: A **fix** review of `2026-09-03-dmls-regression/spec.md`
fix: 2026-09-03-dmls-regression/review-1.md
next: 2026-09-03-dmls-regression/review-2.md
---

# Review 1: DMLS Regression

## Verdict

The fix is **not ready for production**. The repository contains the reviewed
specification and execution plan, but none of the specified implementation.
The original failure mode therefore remains possible and invisible to the
normal green DMLS test gate.

## Findings

### Critical — The fix has not been implemented

The proposed `zed-dmls-cli` package and its `zed-dmls stage` / `doctor`
commands do not exist. Neither do the `install-zed`, `zed-doctor`,
`zed-package`, or `zed-verify` recipes. There is no stable, checkout-independent
extension staging path, no rollback-safe staging operation, and no diagnostic
path for absent registrations, dangling links, invalid manifests, binary
compatibility, or relevant Zed log errors.

Consequently, deleting the worktree that owns a registered dev-extension path
can still break DMLS in Zed, and the repository still provides no command that
identifies the cause or directs the developer to a stable registration target.
Implement phases 1, 4, and 5 of the plan, including the cross-platform injected
filesystem and host-discovery tests, before treating the incident as fixed.

### High — The normal L1 gate still cannot detect an unloadable Zed extension

`darkmatter/dmls/tests/zed_extension_contract.rs` is absent. The existing
`packaging_contract.rs` only compares release archive-name strings in the
justfile and extension source; it does not parse `extension.toml` or validate
the crate shape. The current `just test dmls` run passed all 642 tests while
none of the manifest invariants in AC1 were checked.

Add the specified passive cross-platform contract test and perform the three
non-vacuity mutations. Until then, a missing manifest, changed extension id,
missing language-server mapping, mismatched version, invalid crate type, or
missing API dependency can pass the normal L1 gate on every OS.

### High — WASI compilation and official Zed packaging remain optional or absent

`check-zed` still targets `wasm32-wasip1`, omits `--locked`, and exits zero when
the target is missing. It is folded into `check`, not the Darkmatter `lint`
recipe required by the specification. There are no `zed-package` or
`zed-verify` recipes, no pinned `zed-extension` supply-chain record, no
digest/artifact validation, and no mandatory package-CI or latest-stable
companion step.

This leaves AC2 and AC3 unimplemented: neither Zed's actual
`wasm32-wasip2` build target nor its own manifest/build/package path is a
blocking production gate. Implement phases 2 and 3 and retain the intended
classification as companion verification rather than L2.

### High — Every user-observable staging and doctor behavior has no verification

AC4 through AC6 describe observable command and editor outcomes: actionable
doctor failures, a stable staged copy, repeat updates, and survival after the
source worktree is removed. Their strongest automated verification is
**none**, because the CLI and tests do not exist. The explicit real-host check
for removing worktree A and restarting Zed is also not recorded.

These behaviors require the specified Level 1 hermetic CLI/filesystem tests;
AC6 additionally requires the explicit manual real-Zed evidence allowed by the
specification. With neither form present, each requirement has a test-level gap
of high severity and cannot contribute to a production-ready verdict.

### High — CI scope cannot select a companion verification that does not exist

`zed-extension` is absent from the `runner-tools` vocabulary and DMLS package
policy. The affected-scope tests contain no assertion that changes below the
workspace-excluded `darkmatter/dmls/zed-dmls/**` path select DMLS plus the Zed
companion verification. AC9 therefore has no Level 1 policy proof, and an
extension-only regression is not guaranteed to schedule any authoritative Zed
verification.

### Medium — Documentation still prescribes the failure-prone workflow

The editor documentation still tells users to select the extension directory
inside the current checkout and warns them to reinstall after a worktree is
removed. The extension README says the crate is intentionally not built by
monorepo recipes, while `check-zed` still documents `wasm32-wasip1` as Zed's
target. `git grep wasip1 darkmatter/` therefore still returns active justfile
guidance, contrary to AC7.

Update all documentation only after the commands and their verified exit/output
contracts exist, so the docs describe real behavior rather than anticipated
behavior.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: manifest and crate-shape regressions fail `just test` on macOS, Linux, and Windows | None; the specified Level 1 contract test is absent | **High gap.** Level 1 is appropriate and required. |
| AC2: mandatory `wasm32-wasip2` compile gate with non-mutating missing-target failure | None; the existing recipe checks `wasm32-wasip1` and missing targets skip successfully | **High gap.** This is a build/CI gate, not L2. |
| AC3: pinned official packager validates manifest and WASM artifact | None | **High gap.** Companion CI verification is required; L2 is not applicable. |
| AC4: doctor reports broken registration accurately and accepts a repaired install despite historical logs | None; the command does not exist | **High gap.** Hermetic Level 1 subprocess/filesystem tests are appropriate. |
| AC5: install stages and refreshes a stable extension copy without changing Zed registration | None; the command does not exist | **High gap.** Hermetic Level 1 staging and exit-code tests are appropriate. |
| AC6: removing the source worktree does not break the registered extension | None; no automated staging proof or recorded manual real-Zed check | **High gap.** Level 1 proves path independence; the spec also requires one manual real-host check. |
| AC7: docs and justfile agree on `wasm32-wasip2` and stable installation | None; active justfile text still uses `wasm32-wasip1` and docs retain checkout-bound installation | Not satisfied. |
| AC8: cross-platform CLI behavior and `TerminalRenderable` output | None; the package does not exist | **High gap.** Level 1 cross-platform tests are appropriate because no terminal rendering fidelity or physical input behavior is claimed. |
| AC9: extension-only changes select DMLS and mandatory Zed verification | None | **High gap.** Level 1 CI scope-policy tests are appropriate and required. |

Levels 2 and 3 are not required by this specification. It claims no terminal
emulator rendering fidelity, glyph geometry, scrolling, or physical keyboard,
mouse, paste, or IME behavior. The official Zed packager is a headless
companion gate, while the one real-Zed restart exercise is explicitly manual.

## Ergonomics and Performance

There is no new implementation to assess for Rust ergonomics or runtime
performance. The proposed architecture is reasonable: a typed Rust CLI with
injectable discovery/filesystem seams is preferable to encoding cross-platform
path and link behavior in shell, and allowlisted sibling staging bounds both
I/O and failure scope. Those advantages remain prospective until implemented.

## Verification Performed

- `just test dmls`: **642 passed; 0 skipped**. This demonstrates that the
  existing Level 1 suite is green while the regression requirements remain
  unverified.
- Repository inventory checks confirmed that
  `dmls/tests/zed_extension_contract.rs` and `dmls/zed-dmls-cli/` are absent.
- Recipe and source searches confirmed that only existing `check-zed` is
  present; the stage, doctor, install, package, and verification surfaces are
  absent outside the specification and plan.
- Source inspection confirmed that `check-zed` still uses
  `wasm32-wasip1` and treats a missing target as a successful skip.
- Git history for the fix directory and proposed implementation paths contains
  only the planning commit, and every plan task/checkpoint remains unchecked.

GitNexus could not provide graph evidence for this worktree: it has no index
registered for this path, and the same-HEAD `feat-unifi` index could not be
opened because its LadybugDB shadow pages require read-write recovery. This did
not limit the conclusion because no production symbols were added or changed
for the fix; the review establishes absence directly from the clean worktree,
workspace/recipe inventory, source, tests, and history.

## Production Readiness

None of AC1–AC9 is satisfied. The fix should remain out of production until the
planned implementation exists and every acceptance criterion has the
verification shown above. In particular, a green current DMLS suite must not
be interpreted as evidence that Zed can load, build, package, stage, or diagnose
the extension.
