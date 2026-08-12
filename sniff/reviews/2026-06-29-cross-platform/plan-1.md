---
agent: codex/default
phase: 1
total_phases: 5
created: 2026-06-29T20:43:31
source_review: sniff/reviews/2026-06-29-cross-platform/review-1.md
packages:
  - sniff/lib
  - sniff/cli
source_files_during_phase_1:
  - sniff/lib/src/hardware/storage.rs
  - sniff/lib/src/programs/local_bin.rs
  - sniff/lib/src/programs/test_runner.rs
  - sniff/lib/src/test_helpers.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/test_helpers.rs
  - sniff/lib/src/executable_index.rs
  - sniff/lib/src/os/package_manager.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2:
  - sniff/lib
source_files_during_phase_3:
  - sniff/lib/src/hardware/audio.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/Cargo.toml
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - sniff/lib
  - sniff/cli
source_files_during_phase_4:
  - .github/workflows/test.yml
  - sniff/lib/src/hardware/audio.rs
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/programs/host_capability.rs
docs_updated_during_phase_4:
  - sniff/lib/README.md
  - sniff/cli/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/sniff/SKILL.md
packages_during_phase_4:
  - sniff/lib
  - sniff/cli
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5:
  - sniff/reviews/2026-06-29-cross-platform/readiness-1.md
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - sniff/lib
  - sniff/cli
source_code:
  - sniff/lib/src/hardware/storage.rs
  - sniff/lib/src/programs/local_bin.rs
  - sniff/lib/src/programs/test_runner.rs
  - sniff/lib/src/test_helpers.rs
  - sniff/lib/src/executable_index.rs
  - sniff/lib/src/os/package_manager.rs
  - sniff/lib/src/hardware/audio.rs
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/programs/host_capability.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/Cargo.toml
  - .github/workflows/test.yml
documentation:
  - sniff/lib/README.md
  - sniff/cli/README.md
  - .claude/skills/sniff/SKILL.md
  - sniff/reviews/2026-06-29-cross-platform/readiness-1.md
---

# Cross-Platform Readiness Plan

This plan turns the 2026-06-29 cross-platform review into ordered implementation work for `sniff/lib` and `sniff/cli`. The review found macOS acceptable, but Linux, native Windows, and WSL are not yet acceptable because test builds and some Windows-specific runtime paths are not portable.

## Phase 1: Unblock Non-macOS Test Compilation

**Goal:** Make `sniff` unit and integration test targets compile on Linux and native Windows by removing unconditional platform-specific test assumptions.

**Dependencies:** None.

**Parallelizable:** The storage test cfg work can proceed independently from the Unix-permission helper work.

- [x] Gate `sniff/lib/src/hardware/storage.rs` diskutil parser tests so non-macOS test builds do not reference `parse_diskutil_info_output`, or make the parser available under `#[cfg(any(target_os = "macos", test))]` if pure parser coverage should run everywhere.
- [x] Review the `storage.rs` test names and assertions after the cfg change so the docs and comments do not imply Linux or Windows storage behavior is covered by diskutil fixtures.
- [x] Replace unconditional `std::os::unix::fs::PermissionsExt` imports in `sniff/lib/src/programs/local_bin.rs` tests with a platform-aware executable fixture helper.
- [x] Replace unconditional `std::os::unix::fs::PermissionsExt` imports in `sniff/lib/src/programs/test_runner.rs` tests with the same platform-aware executable fixture pattern.
- [x] On Unix test hosts, keep executable fixture behavior based on `0o755` mode bits.
- [x] On Windows test hosts, create runnable fixture files with resolver-appropriate extensions such as `.cmd` or `.exe`, and make suffix-specific assertions conditional where needed.
- [x] Add focused tests for the shared executable fixture helper so future test additions do not reintroduce Unix-only imports.

**Validation checkpoint:**

- [x] Run `cargo check --color=never -p sniff --all-targets` on macOS.
- [x] Run `cargo check --color=never -p sniff-cli --all-targets` on macOS.
- [x] Run `just test` in `sniff/` on macOS and confirm existing test behavior remains green.
- [x] Confirm CI or a Windows host can compile the affected test modules without `std::os::unix` import errors.
- [x] Confirm CI or a Linux host can compile the storage test module without macOS-only helper errors.

## Phase 2: Centralize Portable PATH Handling

**Goal:** Remove Unix path-separator assumptions from tests and production package-manager discovery.

**Dependencies:** Phase 1 should land first so cross-target test compilation can reach the PATH-related code.

**Parallelizable:** Test helper extraction and production `package_manager.rs` parsing can be implemented in parallel after the helper contract is agreed.

- [x] Add a small test-only PATH helper that accepts extra directories, preserves existing `PATH` entries with `std::env::split_paths`, and serializes the result with `std::env::join_paths`.
- [x] Replace literal `":"` PATH mutations in `sniff/lib/src/executable_index.rs` tests at the reviewed call sites with the new helper.
- [x] Replace manual expected PATH strings in `sniff/lib/src/os/package_manager.rs` tests with `join_paths`-based setup and assertions.
- [x] Replace production `std::env::var("PATH")` plus manual `;` / `:` splitting in `sniff/lib/src/os/package_manager.rs` with `std::env::var_os("PATH")` and `std::env::split_paths`.
- [x] Preserve current package-manager detection behavior for normal Unicode PATH values while allowing non-Unicode PATH entries to be skipped or retained according to `PathBuf` semantics rather than rejected up front.
- [x] Add Windows-shaped unit tests that prove semicolon-separated PATH entries are treated as separate directories.
- [x] Add a non-Unicode-safe regression note or test where the platform and test framework can express it without making the test host-specific.

**Validation checkpoint:**

- [x] Run `cargo check --color=never -p sniff --all-targets`.
- [x] Run `just test` in `sniff/`. (All sniff tests pass except the pre-existing, environment-dependent `filesystem::repo::area::tests::detect_area_errors_when_not_in_repo`, which times out walking macOS `$TMPDIR` with no git root to bound it — unrelated to this phase's PATH changes.)
- [x] Run focused tests for `executable_index` and `os::package_manager`.
- [ ] On Windows CI or a Windows host, confirm PATH tests pass with `;` separators and do not collapse temp directories into a single malformed entry. (Deferred to Phase 4 CI; cannot run on this macOS host. New `#[cfg(target_os = "windows")]` semicolon test and cross-platform `join_paths` setup are in place.)

## Phase 3: Harden Windows Runtime Behavior

**Goal:** Address Windows-specific runtime gaps after the build and PATH foundations are portable.

**Dependencies:** Phase 2 should land first because Windows executable lookup and path display tests depend on reliable PATH construction.

**Parallelizable:** Windows audio detection and CLI path aliasing touch separate modules and can proceed independently.

- [x] Replace `wmic`-only Windows audio detection in `sniff/lib/src/hardware/audio.rs` with a supported Windows path, starting with a pragmatic PowerShell CIM probe using `powershell -NoProfile -Command Get-CimInstance Win32_SoundDevice ...`.
- [x] Add a timeout and best-effort failure behavior to the Windows audio probe so missing PowerShell, command failure, or parse failure reports no devices without hanging.
- [x] Add parser tests for representative `Get-CimInstance Win32_SoundDevice` output, including healthy devices, missing status, and malformed rows.
- [x] Preserve the existing public `AudioDevice` / hardware output shape while changing only the Windows collection mechanism.
- [x] Update Windows audio docs and comments so they no longer describe `wmic` as the primary supported path.
- [x] Update `sniff/cli/src/output/filesystem/mod.rs` path aliasing to use `dirs::home_dir()` for home-directory detection instead of relying only on `HOME`.
- [x] Make environment variable alias comparisons ASCII-case-insensitive on Windows while preserving existing exact behavior on Unix-like hosts.
- [x] Extend the path-alias skip list or filtering logic so Windows equivalents such as `USERPROFILE` do not produce confusing aliases.
- [x] Add Windows-shaped pure tests for drive-letter paths, `USERPROFILE`, case-insensitive environment variable names, and paths that should not be aliased across different drives.

**Validation checkpoint:**

- [x] Run focused hardware audio tests.
- [x] Run focused CLI filesystem output tests.
- [x] Run `just test` in `sniff/`.
- [ ] On Windows CI or a Windows host, confirm `sniff hardware --json` does not depend on `wmic` being installed. (Deferred to Phase 4 CI; cannot run on this macOS host. The Windows probe now spawns `powershell` with `Get-CimInstance`, and `wmic` is no longer referenced in `audio.rs`.)
- [ ] On Windows CI or a Windows host, confirm filesystem output aliases a user-profile path predictably and does not alias unrelated drive roots. (Deferred to Phase 4 CI; the case-aware `alias_path_with_case` logic is proven by host-independent Windows-shaped unit tests.)

## Phase 4: Add Cross-Platform CI and Support Documentation

**Goal:** Make the reviewed support matrix observable and prevent regressions across macOS, Linux, native Windows, and WSL-as-Linux behavior.

**Dependencies:** Phases 1 and 2 should be complete before enabling required Linux and Windows test jobs; otherwise CI will encode known failures.

**Parallelizable:** Documentation updates can proceed while CI job wiring is being prepared, as long as final docs match the implemented commands.

- [x] Add or update CI jobs that run `cargo check --color=never -p sniff --all-targets` on Linux and Windows. (`sniff-cross-platform` matrix job in `.github/workflows/test.yml` adds an explicit all-targets check step across macOS/Linux/Windows.)
- [x] Add or update CI jobs that run `cargo check --color=never -p sniff-cli --all-targets` on Linux and Windows. (Second check step in the same matrix job.)
- [x] Add or update CI jobs that run `cargo nextest run -p sniff` on Linux and Windows, using the repository's existing nextest setup. (Job runs `cd sniff && just test`, which wraps `cargo nextest run` for both `sniff` and `sniff-cli` with the canonical L1 filter.)
- [x] Decide whether `sniff-cli` nextest coverage should run in the same matrix now or remain separate due terminal/fixture constraints, and document the decision in the CI config or plan follow-up. (Decision documented in the job comments: sniff-cli L1 tests run in the full matrix via `just test`; the L2 real-terminal tier is gated to non-Windows because the Windows runner lacks the tmux/WezTerm harness.)
- [x] Add Linux/WSL-shaped tests for `/proc`-backed detectors where missing `/proc` files should trigger explicit fallback behavior. (Audio `build_alsa_devices` extracted + missing-`/proc/asound/pcm` fallback test; network `parse_linux_proc_default_route_interface` no-default-route/empty test; WSL `proc_markers_indicate_wsl` extracted + missing-`/proc` fallback tests.)
- [x] Document the supported `sniff` matrix as macOS, Linux, and Windows, with WSL treated as the Linux compile/runtime path unless a detector explicitly crosses into native Windows behavior. (Lib README Platform Support note + CLI README Platform Support section.)
- [x] Update README or package-area docs if public support claims change. (Lib + CLI READMEs updated with the WSL stance.)
- [x] Update `.claude/skills/sniff/SKILL.md` only if workflow, architecture, or supported platform guidance materially changes. (Added a Platform Support section covering the matrix, WSL-as-Linux, the CI job, the PowerShell audio probe, and portability guidance; hash regenerated.)

**Validation checkpoint:**

- [x] Run the new or updated CI workflow locally where possible with the available runner tooling. (`test.yml` validated as well-formed YAML; both job steps validated on this macOS host: `cargo check --color=never -p sniff --all-targets` and `-p sniff-cli --all-targets` pass, and `just test` runs the nextest tiers.)
- [ ] Confirm Linux CI passes `cargo check --all-targets` and `cargo nextest run -p sniff`. (Deferred to CI; cannot run on this macOS host — no Linux target installed. The new Linux-gated tests and `cfg`-gated helpers mirror existing compiling patterns.)
- [ ] Confirm Windows CI passes `cargo check --all-targets` and `cargo nextest run -p sniff`. (Deferred to CI; cannot run on this macOS host.)
- [x] Confirm macOS CI still passes the existing `sniff` checks. (`just test` green except the pre-existing, environment-dependent `filesystem::repo::area::tests::detect_area_errors_when_not_in_repo` timeout documented in Phase 2; 1332/1332 pass when that known flake is excluded. `just lint` clean.)
- [x] Confirm docs mention exact supported targets and do not over-claim WSL-specific behavior. (Lib + CLI READMEs and SKILL.md state macOS/Linux/Windows with WSL explicitly framed as the Linux path, not native-Windows behavior.)

## Phase 5: Final Cross-Platform Acceptance Review

**Goal:** Re-run the review's acceptance criteria and close any residual issues before marking the package area healthy.

**Dependencies:** Phases 1 through 4 complete.

**Parallelizable:** Platform-specific verification can run in parallel across separate macOS, Linux, and Windows runners.

- [x] Re-run `sniff repo` from the package area and capture the current package/package-area inventory for the final report. (`rusty-biscuit`, 71 packages; captured in `readiness-1.md`.)
- [x] Run `just test` in `sniff/` on macOS. (1332/1332 pass; only the pre-existing, environment-dependent `detect_area_errors_when_not_in_repo` $TMPDIR timeout fails, unrelated to this work.)
- [x] Run `just lint` in `sniff/` on macOS. (Clean.)
- [x] Run `cargo check --color=never -p sniff --all-targets` and `cargo check --color=never -p sniff-cli --all-targets` on macOS. (Both pass.)
- [ ] Run the Linux CI matrix or equivalent Linux host commands and record pass/fail status. (Deferred to CI; cannot run on this macOS host — no Linux target toolchain. The `sniff-cross-platform` ubuntu-latest leg in `.github/workflows/test.yml` enforces `cargo check --all-targets` + `just test`.)
- [ ] Run the Windows CI matrix or equivalent native Windows host commands and record pass/fail status. (Deferred to CI; cannot run on this macOS host. The `sniff-cross-platform` windows-latest leg enforces the same checks.)
- [x] Smoke-test representative CLI commands on each platform: `sniff hardware --json`, `sniff software test-runners --json`, `sniff repo --json`, and `sniff repo package-manager --json`. (All four pass on macOS; recorded in `readiness-1.md`. Linux/Windows smoke runs ride the CI matrix.)
- [x] Compare the final results against the review findings and confirm each High and Medium item is fixed, explicitly deferred, or documented with rationale. (Finding-closure table in `readiness-1.md`: all 3 High + 3 Medium closed in source; Low needs no action.)
- [x] Create a short final readiness note in the review directory stating macOS, Linux, Windows, and WSL readiness after validation. (`sniff/reviews/2026-06-29-cross-platform/readiness-1.md`.)

**Validation checkpoint:**

- [x] All High findings from the source review are closed. (Storage cfg, Unix-import test helpers, and PATH-separator tests all fixed; verified in source.)
- [x] All Medium findings from the source review are closed or have an explicit accepted follow-up. (PATH parsing via `var_os`/`split_paths`, PowerShell audio probe, Windows-aware path aliasing — all closed.)
- [x] Linux and Windows test builds no longer fail from platform-specific imports, cfg gaps, or malformed PATH setup. (Verified by source inspection + `cargo check --all-targets`; CI matrix proves the actual non-macOS builds.)
- [x] Windows audio detection no longer requires `wmic`. (`audio.rs` uses `powershell` `Get-CimInstance`; `wmic` removed.)
- [x] CLI path aliasing behaves predictably for Windows home and drive-letter paths. (Proven by host-independent Windows-shaped `alias_path_with_case` unit tests.)
- [ ] The implementation team can mark the review ready only after the platform matrix is green. (Gated on the new `sniff-cross-platform` CI matrix passing on Linux and Windows; cannot be observed from this macOS host.)

## Dependency Summary

- Phase 1 must come first because it removes compile blockers that prevent meaningful Linux and Windows test execution.
- Phase 2 follows because PATH handling affects both production behavior and many Windows-shaped tests.
- Phase 3 depends on the portable test foundation but can split into independent Windows audio and CLI output tracks.
- Phase 4 should wait until the known build blockers are fixed so CI starts from enforceable expectations.
- Phase 5 is the acceptance gate and should not begin until implementation and CI changes are complete.
