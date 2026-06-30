---
$schema:
  agent: string
  phase: number
  total_phases: number
  created: string
about: "Implementation plan for the 2026-06-29 playa cross-platform review"
source_review: "playa/reviews/2026-06-29-cross-platform/review-1.md"
agent: "codex/default"
phase: 1
total_phases: 5
created: "2026-06-29T21:41:02"
source_files_during_phase_1:
  - playa/lib/src/playback.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - playa
---

# Cross-Platform Hardening Plan

## Goal

Bring the `playa` package area to cross-platform readiness by fixing the byte-playback temp-cache race, strengthening path-oriented tests, adding native Windows verification, and removing a small macOS-only portability assumption.

## Success Criteria

- Concurrent byte playback from in-memory audio bytes cannot race on a shared deterministic temp file.
- Host-player command construction remains path-safe for platform-shaped paths, including spaces and Windows-style paths.
- Native Windows build and test coverage runs on a real Windows runner.
- macOS ducking diagnostics no longer shell out to `which`.
- The review readiness gap can be closed with macOS, Linux, WSL, and native Windows acceptance evidence.

## Phase 1: Fix Byte Playback Temp-Cache Publishing

- [x] Inspect `playa/lib/src/playback.rs` byte-cache code paths for sync and async playback and identify the shared helper boundary for cache path creation.
- [x] Replace deterministic `{hash}.tmp` writes with a unique writer temp path in the same cache directory, using process id plus a monotonic or random suffix, or `tempfile::NamedTempFile::new_in`.
- [x] Publish the unique temp file to the deterministic `{hash}.audio` cache path with idempotent behavior when another writer wins the race.
- [x] Ensure the temp file is flushed and closed before the cache file is exposed to host players.
- [x] Delete the unique temp file on publish loss, publish failure, or early error without deleting a valid existing `{hash}.audio` file.
- [x] Keep sync and async byte playback on the same cache-publication semantics so they cannot diverge.
- [x] Review adjacent `///` and inline comments in `playback.rs`; update or remove any comment that still describes deterministic temp-file behavior.

Validation checkpoint:

- [x] Add focused tests that call the byte-cache path concurrently from multiple threads with identical bytes and assert every caller receives an existing, readable cache file.
- [x] Add a test for the "cache file already exists" path to verify a losing writer cleans up its unique temp file and returns the existing cache path.
- [x] Run `just test` in `playa/` and record the result in the implementation notes.

Dependency notes:

- Phase 1 blocks final readiness and should be completed before broader validation.
- The test work in this phase can run in parallel with implementation after the cache helper boundary is identified.

Implementation notes:

- Shared boundary identified at `write_temp_audio` (sync) and `write_temp_audio_async` (async); both now stage into a process-unique path from `unique_temp_path` (pid + monotonic `AtomicU64` counter + nanosecond stamp) and publish through `publish_temp_audio` / `publish_temp_audio_async` with identical race semantics. No new crate dependency was added (the pid-plus-suffix option was chosen over `tempfile`).
- Staging writes via `File::create` + `write_all` + `sync_all` before the file is closed and renamed, so host players never see a partially written cache file. Publication pre-checks the cache path and re-checks after a failed rename, so a losing writer removes only its own unique temp and never deletes a valid existing `{hash}.audio`.
- `just test` in `playa/`: PASS — playa-cli 13/13; `playa` lib `playback` module 38/38 (with `--features async`), including the new concurrent sync, concurrent async, and publish-loss-cleanup tests. `just lint` in `playa/`: PASS (clean, including `cargo clippy --features async --all-targets`).

## Phase 2: Broaden Path-Oriented Command Tests

- [x] Inventory command-construction tests in `playa/lib/src/playback.rs` that currently use `/tmp/test.wav`.
- [x] Introduce platform-shaped path fixtures for command tests while preserving assertions at the `OsStr` or `OsString` level.
- [x] On Windows builds, cover a drive-prefixed path with spaces such as `C:\Users\Example\audio file.wav`.
- [x] On Windows builds, cover a UNC-shaped path such as `\\server\share\audio file.wav`.
- [x] On Unix builds, cover paths with spaces and backslash characters as ordinary filename characters.
- [x] Update any brittle string assertions so tests verify argument boundaries rather than separator spelling.

Validation checkpoint:

- [x] Run focused `playback.rs` unit tests for command construction.
- [x] Run `just test` in `playa/` after the test updates.

Dependency notes:

- Phase 2 can start after Phase 1's cache helper shape is stable.
- Phase 2 is parallelizable with Phase 3 because it does not depend on CI wiring.

## Phase 3: Add Native Windows Verification

- [ ] Locate the repository's existing CI workflow conventions and identify the narrowest place to add `playa` Windows checks.
- [ ] Add a Windows runner check for `cargo check -p playa --target x86_64-pc-windows-msvc --features sfx-native-windows,native-playback`.
- [ ] Add a Windows runner check for `cargo check -p playa --target x86_64-pc-windows-msvc --features audio-ducking-windows`.
- [ ] Add a Windows runner check for `cargo check -p playa-cli --target x86_64-pc-windows-msvc`.
- [ ] Add a Windows runner check for `cargo nextest run -p playa --target x86_64-pc-windows-msvc --no-default-features`.
- [ ] Keep the checks non-interactive and aligned with the repo's nextest-based test policy.
- [ ] Document any feature-matrix decision if the implemented CI commands differ from the review's suggested commands.

Validation checkpoint:

- [ ] Validate the workflow syntax locally if the repo has a non-network workflow linter or equivalent.
- [ ] Confirm the commands are runnable on a real Windows runner through CI, or record that CI execution remains pending if no runner is available in the current environment.

Dependency notes:

- Phase 3 is parallelizable with Phase 2 and Phase 4.
- Final readiness in Phase 5 depends on this phase producing native Windows evidence.

## Phase 4: Remove macOS `which` Shell-Out

- [ ] Inspect `playa/cli/src/main.rs` macOS ducking diagnostics around the `nowplaying-cli` check.
- [ ] Replace `Command::new("which").arg("nowplaying-cli")` with portable executable lookup using an existing detection helper, `std::env::split_paths`, or an already-approved crate if one is present.
- [ ] Preserve current user-visible diagnostic meaning for installed versus missing `nowplaying-cli`.
- [ ] Avoid introducing manual shell parsing or Unix-only assumptions outside the macOS-specific branch.
- [ ] Review related comments and CLI help text for drift after the lookup change.

Validation checkpoint:

- [ ] Add or update a focused test for the executable lookup helper if the logic is factored out.
- [ ] Run the relevant `playa-cli` tests, then run `just test` in `playa/`.

Dependency notes:

- Phase 4 is independent of Phases 1 through 3.
- This is low risk and can be handled in parallel with the test and CI work.

## Phase 5: Cross-Platform Readiness Review

- [ ] Re-run `just test` in `playa/` after all code and CI changes are in place.
- [ ] Run `just lint` in `playa/` if the package area lint recipe is available and does not require unavailable platform tooling.
- [ ] Verify macOS behavior locally for byte playback, host-player fallback, and `duck-info` diagnostics.
- [ ] Verify Linux and WSL readiness by confirming no implementation added Windows path translation, Unix socket assumptions, or shell-specific behavior to shared code.
- [ ] Verify native Windows readiness from the Windows CI results added in Phase 3.
- [ ] Update `playa/README.md`, `playa/docs/`, or area dependency docs only if public behavior, dependencies, or supported workflows changed.
- [ ] Capture final evidence against the original review findings: temp-cache race fixed, path test coverage broadened, Windows CI present, macOS lookup portable.

Validation checkpoint:

- [ ] Mark the implementation complete only after every high and medium finding from the review has either been fixed or has explicit validation evidence.
- [ ] If a finding cannot be closed, record the blocker, the exact failing command or missing environment, and the follow-up owner before claiming readiness.

Dependency notes:

- Phase 5 depends on all earlier phases.
- Documentation updates inside this phase are conditional and should stay scoped to behavior or workflow changes made by this plan.
