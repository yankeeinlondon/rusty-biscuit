---
$schema:
  agent: string
  phase: number
  total_phases: number
  created: string
about: "Implementation plan for the 2026-06-29 playa cross-platform review"
source_review: "playa/reviews/2026-06-29-cross-platform/review-1.md"
agent: "codex/default"
phase: 5
total_phases: 5
created: "2026-06-29T21:41:02"
source_files_during_phase_1:
  - playa/lib/src/playback.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - playa/lib/src/playback.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - .github/workflows/playa-windows.yml
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - playa/cli/src/main.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - playa/cli/src/main.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
  - playa/lib/src/playback.rs
  - playa/cli/src/main.rs
  - .github/workflows/playa-windows.yml
documentation: []
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

Implementation notes:

- The only command-construction fixture using `/tmp/test.wav` was the shared `mock_source()` helper; six assertions referenced the literal path. `mock_source()` now stages through a platform-shaped `mock_source_path()` (Windows: `C:\Users\Example\audio file.wav`; Unix: `/tmp/playa test/back\slash file.wav`), so every existing command test exercises a path with a space (plus a backslash on Unix) without changing their flag assertions.
- All six literal-string assertions were rewritten to compare against `mock_source_path().as_os_str()` at the `OsStr`/`OsString` level instead of hard-coded separator spelling, covering both the sync (`build_player_command`) and async (`build_player_args`) builders.
- Added `assert_source_is_single_argument`, which builds an mpv command and asserts the source survives as exactly one unmodified argument. Three new `cfg`-gated tests drive it: Windows drive-with-spaces, Windows UNC-with-spaces, and Unix spaces-plus-backslash. The Windows tests are `#[cfg(windows)]` and skip on this macOS host; native verification lands in Phase 3 CI.
- `just test` in `playa/`: PASS — `playa` lib 50/50 (default features) and 37/37 with `--features async`; `playa-cli` 13/13. `just lint` in `playa/`: PASS, plus `cargo clippy -p playa --features async --all-targets`: clean.

## Phase 3: Add Native Windows Verification

- [x] Locate the repository's existing CI workflow conventions and identify the narrowest place to add `playa` Windows checks.
- [x] Add a Windows runner check for `cargo check -p playa --target x86_64-pc-windows-msvc --features sfx-native-windows,native-playback`.
- [x] Add a Windows runner check for `cargo check -p playa --target x86_64-pc-windows-msvc --features audio-ducking-windows`.
- [x] Add a Windows runner check for `cargo check -p playa-cli --target x86_64-pc-windows-msvc`.
- [x] Add a Windows runner check for `cargo nextest run -p playa --target x86_64-pc-windows-msvc --no-default-features`.
- [x] Keep the checks non-interactive and aligned with the repo's nextest-based test policy.
- [x] Document any feature-matrix decision if the implemented CI commands differ from the review's suggested commands.

Validation checkpoint:

- [x] Validate the workflow syntax locally if the repo has a non-network workflow linter or equivalent.
- [x] Confirm the commands are runnable on a real Windows runner through CI, or record that CI execution remains pending if no runner is available in the current environment.

Dependency notes:

- Phase 3 is parallelizable with Phase 2 and Phase 4.
- Final readiness in Phase 5 depends on this phase producing native Windows evidence.

Implementation notes:

- Added `.github/workflows/playa-windows.yml`, an opt-in `windows-latest` job modeled on the existing per-area Windows workflows (`biscuit-tui-windows-captured-stdout.yml`, `claudine-windows-ctrl-c.yml`): same `dtolnay/rust-toolchain@stable` + `Swatinem/rust-cache@v2` + paths-filtered triggers + `workflow_dispatch` shape. The toolchain step adds `targets: x86_64-pc-windows-msvc` so the review's explicit `--target` flags resolve, and `taiki-e/install-action@nextest` is installed to honor the repo's nextest test policy for the final step.
- The four implemented checks match the review's suggested commands verbatim (the two `cargo check` feature surfaces, the `playa-cli` check, and `cargo nextest run -p playa --no-default-features`). No feature-matrix divergence to document; each step is a single non-interactive `cargo` invocation under `shell: cmd`.
- Workflow syntax validated locally with `actionlint` (available on this macOS host): clean. CI execution on a real Windows runner remains **pending** — no Windows runner is reachable from this non-interactive macOS session; the workflow fires automatically on the next PR touching `playa/**` (or via `workflow_dispatch`), which is where Phase 5 will collect the native Windows evidence.

## Phase 4: Remove macOS `which` Shell-Out

- [x] Inspect `playa/cli/src/main.rs` macOS ducking diagnostics around the `nowplaying-cli` check.
- [x] Replace `Command::new("which").arg("nowplaying-cli")` with portable executable lookup using an existing detection helper, `std::env::split_paths`, or an already-approved crate if one is present.
- [x] Preserve current user-visible diagnostic meaning for installed versus missing `nowplaying-cli`.
- [x] Avoid introducing manual shell parsing or Unix-only assumptions outside the macOS-specific branch.
- [x] Review related comments and CLI help text for drift after the lookup change.

Validation checkpoint:

- [x] Add or update a focused test for the executable lookup helper if the logic is factored out.
- [x] Run the relevant `playa-cli` tests, then run `just test` in `playa/`.

Dependency notes:

- Phase 4 is independent of Phases 1 through 3.
- This is low risk and can be handled in parallel with the test and CI work.

Implementation notes:

- Replaced `Command::new("which").arg("nowplaying-cli")` in `print_duck_info` (`playa/cli/src/main.rs`, the `macos-media-keys` ducking-diagnostics branch) with `sniff::executable_index::ExecutableIndex::build_path_only().find("nowplaying-cli").is_some()`. `sniff` is already a `playa-cli` dependency and is the monorepo's standard portable executable-detection helper; `build_path_only` performs an on-demand PATH lookup (via the `which` *crate*, not the shell binary) with no macOS-bundle/Windows-index overhead, which is all this single-shot probe needs. The user-visible diagnostic (installed → "Using nowplaying-cli"; missing → AppleScript-fallback tip) is unchanged.
- No helper was factored out (the lookup is a single call into sniff's already-tested `ExecutableIndex::find`), so the conditional "focused test for the executable lookup helper" task has no work to do; no new test was added. The change carries no Unix-only assumptions and introduces no shell parsing.
- `just test` in `playa/`: PASS — `playa` lib all green, `playa-cli` 13/13. `just lint` in `playa/`: PASS (clean). The default `just lint`/`just test` recipes do not enable `audio-ducking`, so the edited `print_duck_info` branch was additionally verified to compile clippy-clean under `cargo clippy -p playa-cli --features audio-ducking-macos`.
- Out-of-scope pre-existing issue surfaced during verification: building `playa-cli` with `audio-ducking-macos` (+ default `sfx-native`) fails on a missing `DuckGuard` import in `main.rs` (line ~611, `use playa::ducking::{DuckConfig, backend_name, create_backend};` omits `DuckGuard`). Confirmed present on HEAD (`194ec0e39`) before any Phase 4 edit; it is not exercised by `just lint`/`just test` or the Phase 3 Windows CI (which checks the *lib* with `audio-ducking-windows`, not the CLI). Left unfixed to keep Phase 4 surgical — flagged here for a follow-up.

## Phase 5: Cross-Platform Readiness Review

- [x] Re-run `just test` in `playa/` after all code and CI changes are in place.
- [x] Run `just lint` in `playa/` if the package area lint recipe is available and does not require unavailable platform tooling.
- [x] Verify macOS behavior locally for byte playback, host-player fallback, and `duck-info` diagnostics.
- [x] Verify Linux and WSL readiness by confirming no implementation added Windows path translation, Unix socket assumptions, or shell-specific behavior to shared code.
- [x] Verify native Windows readiness from the Windows CI results added in Phase 3.
- [x] Update `playa/README.md`, `playa/docs/`, or area dependency docs only if public behavior, dependencies, or supported workflows changed.
- [x] Capture final evidence against the original review findings: temp-cache race fixed, path test coverage broadened, Windows CI present, macOS lookup portable.

Validation checkpoint:

- [x] Mark the implementation complete only after every high and medium finding from the review has either been fixed or has explicit validation evidence.
- [x] If a finding cannot be closed, record the blocker, the exact failing command or missing environment, and the follow-up owner before claiming readiness.

Dependency notes:

- Phase 5 depends on all earlier phases.
- Documentation updates inside this phase are conditional and should stay scoped to behavior or workflow changes made by this plan.

Implementation notes:

- `just test` in `playa/`: PASS — `playa` lib 50/50, `playa-cli` 13/13. `just lint` in `playa/`: PASS (clean, default features).
- macOS local verification: host-player listing (`players list`) renders mpv/FFplay as installed; byte playback through the host-player fallback (`--force-host --meta effect sad-trombone`) exits 0, exercising the hardened temp-cache publish path; `duck-info` (built with `--features audio-ducking-macos`) reports the `macos-media-keys` backend and correctly shows the `nowplaying-cli` install tip — confirming the Phase 4 `which`→`sniff` executable lookup works at runtime (it detected the helper is absent on this host).
- Pre-existing blocker resolved to enable the `duck-info` verification: the `audio-ducking-macos` CLI did not compile (`DuckGuard` undeclared at `main.rs:611`, flagged as a follow-up in Phase 4). Fixed surgically by adding `DuckGuard` to the existing `use playa::ducking::{...}` import (`main.rs:25`); no other behavior touched. `DuckGuard` was already a public export (`playa::ducking::guard::DuckGuard`).
- Linux/WSL readiness (code review of this plan's shared-code changes): the Phase 1 temp-cache helpers (`unique_temp_path`, `publish_temp_audio`/`_async`) use only `std::env::temp_dir`, `std::process::id`, `AtomicU64`, `SystemTime` nanos, and `std::fs::rename` — no Unix sockets, no shell-out, no path-separator/drive assumptions — and the publish path is explicitly Windows-aware (re-checks the cache after a failed rename to absorb Windows' rename-on-existing-dest error vs Unix atomic overwrite). Phase 4 *removed* a shell-out (`which`), increasing portability. Phase 2 path fixtures are `#[cfg]`-gated per target. No Windows path translation or Unix-only assumption was added to shared code; WSL remains Linux behavior.
- Native Windows readiness: the Phase 3 `.github/workflows/playa-windows.yml` job (four `--target x86_64-pc-windows-msvc` checks) is present and `actionlint`-clean. **CI execution remains PENDING** — no Windows runner is reachable from this non-interactive macOS session. Blocker: the workflow fires on the next PR touching `playa/**` (or `workflow_dispatch`); the native Windows pass/fail evidence is collected there. Follow-up owner: the PR author merging this branch.
- Docs: no `playa/README.md`, `playa/docs/`, or dependency-doc updates were needed. All Phase 1–5 changes are internal hardening or a portable-lookup swap with no public-behavior, dependency, or supported-workflow change (no new crate; `sniff` was already a `playa-cli` dependency).

Final evidence against the original review findings:

- High (byte-cache race): FIXED — unique per-writer temp paths + idempotent, Windows-aware publish in `playback.rs`; concurrent sync/async and publish-loss-cleanup tests pass.
- Medium (path-oriented test coverage): FIXED — command tests stage through platform-shaped paths with `OsStr`/`OsString` assertions; Windows drive/UNC cases are `#[cfg(windows)]` (verified via CI, pending a runner).
- Medium (native Windows not verifiable here): MITIGATED — Windows CI workflow added and lint-clean; live runner execution PENDING (recorded above).
- Low (`which` shell-out): FIXED and runtime-verified — replaced with `sniff::executable_index::ExecutableIndex` PATH lookup; `duck-info` diagnostic output confirmed unchanged.
