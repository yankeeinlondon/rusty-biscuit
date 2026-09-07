---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-09-03T22:47:24+01:00
spec: 2026-09-03-dmls-regression/spec.md
log: darkmatter/fixes/2026-09-03-dmls-regression/log.md
implemented: true
implemented_by: codex/default
next: 2026-09-03-dmls-regression/review-4.md
description: A **fix** review of `2026-09-03-dmls-regression/spec.md`
fix: 2026-09-03-dmls-regression/review-3.md
previous: 2026-09-03-dmls-regression/review-2.md
---

# Review 3: DMLS Regression

## Verdict

The fix is **not ready for production**. The implementation now closes most of
Review 2, including the passive manifest contract, typed staging and doctor
commands, mandatory WASI check, CI scope selection, and active documentation.
However, one supported Linux installation shape resolves the wrong Zed data
directory, and the two environment-backed acceptance gates that prove Zed can
package and continue loading the staged extension have not run successfully.

## Findings

### High — Linux Flatpak defaults do not match Zed's path authority

`HostDiscovery::capture` always obtains the Linux root from
`dirs::data_dir()`, and `default_paths` appends `zed`
(`zed-dmls-cli/src/lib.rs:68-145`). Zed's pinned `paths.rs` first honors
`FLATPAK_XDG_DATA_HOME` on Linux and only otherwise falls back to
`dirs::data_local_dir()`. The CLI therefore looks outside Zed's actual data
directory for Flatpak installations unless the user manually supplies
`--zed-data-dir` and `--zed-log`.

That breaks the default-path contract in specification section 4 and can make
`zed-dmls doctor --if-zed-present` silently return success because
`should_run_doctor` sees neither the real Zed data directory nor its DMLS
registration. It can also direct a user to register a staging location derived
from a different data root than the platform authority intended.

The current `platform_paths_use_native_data_roots_and_overrides` test is Level
1, but it injects an already-chosen root and therefore cannot detect discovery
drift (`src/lib.rs:635-664`). Add injected environment/directory discovery
coverage for ordinary XDG and Flatpak paths and derive the Zed data directory
with the same precedence as the pinned Zed source. This is a **high test-level
gap** because AC4/AC8 claim correct cross-platform default behavior while the
strongest relevant test bypasses the behavior that is wrong.

### High — The official Zed packaging gate has configuration coverage but no successful execution evidence

The Ubuntu lint producer is wired to download the digest-pinned
`zed-extension` binary, install `wasm32-wasip2`, and run `just zed-verify`.
The affected-scope tests prove those workflow strings and package selection,
but they do not execute the official packager or validate its emitted
`manifest.json` and archive. The implementation log explicitly states that the
Linux-only packager was not run on this host.

AC3 requires the Ubuntu companion gate to package the extension and validate
the result. Until a designated Ubuntu CI run completes this step, the strongest
evidence is Level-1 policy/source inspection, not the required companion
execution. A URL digest match and negative missing-tool check do not prove that
the pinned binary accepts these arguments, builds this extension, or emits the
artifact shape consumed by `zed-verify`. Record a green package-CI execution
for `dmls`; treat any argument or artifact mismatch as an implementation
failure rather than weakening the gate.

### High — The defining real-Zed worktree-removal scenario remains unverified

AC6 requires staging from worktree A, registering the printed stable path,
deleting worktree A, restarting Zed, and confirming that Zed does not log the
DMLS missing-manifest error. The implementation log explicitly records that
this exercise was not performed.

The Level-1 test named
`staging_does_not_mutate_registration_and_survives_source_removal` does not
model the claimed relationship: it creates an unrelated ordinary
`installed/dmls` directory containing a sentinel, never points that
registration at `staging_dir`, deletes the source, and checks only that the
staged files remain (`src/lib.rs:693-714`). It proves copy independence and
non-mutation, but not that the registration resolves to the stable copy or that
Zed loads it after restart.

Add a Level-1 symlink test on Unix (and an appropriate Windows link-resolution
test on Windows) that points `installed/dmls` at the stage and proves doctor
remains healthy after source removal. More importantly, complete the explicit
manual real-Zed restart exercise. This is not Level 2 or Level 3: no terminal
emulator or OS keyboard encoding is involved. It is a required real-editor
acceptance check that cannot be replaced by manufactured filesystem state.

### Medium — Doctor's version contract is accidentally coupled to the helper CLI version

`check_binary` compares `dmls --version` with
`env!("CARGO_PKG_VERSION")`, which is the version of `zed-dmls-cli`, not the
`dmls` package. Both happen to be `0.1.0` today. An independent helper release
or DMLS version bump will make a correct installation appear
version-incompatible.

Make the expected DMLS version an explicit build-time/generated contract or
read it from workspace metadata in a deterministic way, then add a test that
would fail if the two package versions drift unintentionally. The current fake
probe tests repeat the helper's own `CARGO_PKG_VERSION`, so they preserve rather
than detect this coupling.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: manifest/crate regressions fail L1 on all OSes | Level 1 passive contract; three negative mutations are recorded | Satisfied in implementation; the focused contract passed in this review. |
| AC2: mandatory `wasm32-wasip2` compile gate | Mandatory build/CI configuration; missing-target path was exercised | Satisfied structurally. This is a build gate, not L2. |
| AC3: official packager validates emitted artifacts | Level-1 workflow/policy inspection only; no successful Ubuntu companion run | **High gap.** Required companion execution is absent. |
| AC4: doctor diagnoses binary, registration, manifest, and logs | Level 1 hermetic unit/subprocess tests | Mostly appropriate, but **high gap** for Flatpak default discovery. |
| AC5: stable, repeatable, rollback-safe staging | Level 1 filesystem and subprocess tests | Appropriate and passing on macOS. |
| AC6: worktree deletion does not break Zed | Partial Level 1 copy-independence test; no linked-registration proof and no real-Zed restart | **High gap.** The explicit acceptance scenario is unverified. |
| AC7: active docs and recipe agree | Static inspection | Satisfied for active surfaces. The literal `git grep wasip1 darkmatter/` wording remains impossible because the specification preserves historical occurrences. |
| AC8: cross-platform CLI and `TerminalRenderable` output | Level 1 tests on macOS, Windows compile evidence, CI matrix configuration | Output architecture is correct, but Linux Flatpak discovery is broken and no three-OS test run was presented. |
| AC9: extension-only changes select DMLS companion verification | Level 1 affected-scope policy tests | Satisfied; all 67 policy tests passed. |

No Level 2 or Level 3 tests are required by this fix. It makes no terminal
rendering-fidelity or physical keyboard/mouse claim. The official Zed packager
and the manual real-editor restart are separate environment-backed acceptance
evidence.

## Ergonomics and Performance

The typed CLI and injected seams are generally ergonomic, and staging performs
bounded allowlisted I/O. The repeated `check-zed` inside both area `lint` and
`zed-verify` causes the weekly workflow to compile-check the extension twice;
this is low risk but can be removed after the required gates have distinct,
non-bypassable entry points. Correctness findings above take priority.

## Verification Performed

- `cargo nextest run -p zed-dmls-cli --no-fail-fast`: **18 passed, 0 skipped**
  on macOS (Level 1).
- Focused DMLS manifest contract: **1 passed**; 642 unrelated tests were
  filtered.
- `just _lint zed-dmls-cli`: passed without warnings.
- `python3 scripts/ci/test_affected_scope.py`: **67 passed**.
- `git diff --check`: passed.
- The pinned Zed `paths.rs` was inspected to compare Linux, macOS, Windows,
  Flatpak, and log-path precedence.
- GitNexus concept search found only the pre-existing indexed Zed surfaces.
  The requested worktree is not registered in the index; compare-to-main
  change detection used another checkout's stale graph and over-reported 321
  files, so it was not treated as authoritative evidence for this review.

## Production Readiness

The fix is **not production ready**. Resolve the Linux Flatpak path mismatch,
run the DMLS package's mandatory Ubuntu Zed packaging step successfully, and
complete the linked-registration plus real-Zed worktree-removal/restart proof
before changing `ready` to `true`.
